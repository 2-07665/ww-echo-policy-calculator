//! Bounded asynchronous jobs, a shared LRU policy cache, and one cold-build lane.
use crate::access::{Access, Quotas, number, quota_ip};
use echo_app::{AppError, Service, SolveRequest};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, watch};

type Policy = Arc<Mutex<Service>>;
type Built = Result<Policy, AppError>;
type BuildReceiver = watch::Receiver<Option<Built>>;
#[derive(Clone)]
pub struct Settings {
    pub cache_bytes: usize,
    pub cache_entries: usize,
    pub queue: usize,
    pub jobs: usize,
    pub per_ip: usize,
    pub states: usize,
    pub transitions: usize,
}
impl Settings {
    pub fn from_env() -> Self {
        let s = Self {
            cache_bytes: number("ECHO_CACHE_MIB", 512)
                .checked_mul(1024 * 1024)
                .expect("cache budget overflow"),
            cache_entries: number("ECHO_CACHE_ENTRIES", 32),
            queue: number("ECHO_MAX_PENDING_BUILDS", 8),
            jobs: number("ECHO_MAX_JOBS", 64),
            per_ip: number("ECHO_MAX_JOBS_PER_IP", 2),
            states: number("ECHO_MAX_STATES", 750_000),
            transitions: number("ECHO_MAX_TRANSITIONS", 24_000_000),
        };
        assert!(
            s.cache_bytes > 0
                && s.cache_entries > 0
                && s.queue > 0
                && s.jobs > 0
                && s.per_ip > 0
                && s.states > 0
                && s.transitions > 0,
            "queue, job and solver limits must be positive"
        );
        s
    }
}
struct Cached {
    policy: Policy,
    bytes: usize,
    used: Instant,
}
struct Job {
    owner: IpAddr,
    key: String,
    status: &'static str,
    result: Option<Value>,
    error: Option<AppError>,
    finished: Option<Instant>,
}
#[derive(Default)]
struct Inner {
    active: Option<String>,
    cache: HashMap<String, Cached>,
    pending: HashMap<String, BuildReceiver>,
    jobs: HashMap<String, Job>,
    quotas: Quotas,
}
pub struct Scheduler {
    inner: Mutex<Inner>,
    pub access: Access,
    settings: Settings,
    build: Arc<Semaphore>,
    query: Arc<Semaphore>,
}
#[derive(Debug)]
pub struct Rejection {
    pub code: &'static str,
    pub message: &'static str,
    pub retry: u64,
}
fn busy() -> Rejection {
    Rejection {
        code: "busy",
        message: "服务队列已满，请稍后重试。",
        retry: 5,
    }
}
impl Scheduler {
    pub fn new(settings: Settings, access: Access) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::default(),
            access,
            settings,
            build: Arc::new(Semaphore::new(1)),
            query: Arc::new(Semaphore::new(2)),
        })
    }
    pub fn charge_request(&self, ip: IpAddr, admin: bool) -> Result<(), Rejection> {
        if admin {
            return Ok(());
        }
        self.inner
            .lock()
            .unwrap()
            .quotas
            .charge(&self.access, ip, None, Instant::now())
            .map_err(|retry| Rejection {
                code: "quota_exceeded",
                message: "请求过于频繁，请稍后重试。",
                retry,
            })
    }
    pub fn submit(
        self: &Arc<Self>,
        request: SolveRequest,
        ip: IpAddr,
        admin: bool,
    ) -> Result<Value, Rejection> {
        let key = serde_json::to_string(&request.config).expect("validated configuration");
        let now = Instant::now();
        let mut inner = self.inner.lock().unwrap();
        inner.jobs.retain(|_, j| {
            j.finished
                .is_none_or(|t| now.duration_since(t) < Duration::from_secs(60))
        });
        inner.cache.retain(|_, c| {
            Arc::strong_count(&c.policy) > 1
                || now.duration_since(c.used) < Duration::from_secs(1800)
        });
        // Check capacity without evicting a result until this request passes admission.
        let oldest_finished = inner
            .jobs
            .iter()
            .filter_map(|(id, j)| j.finished.map(|t| (id.clone(), t)))
            .min_by_key(|(_, t)| *t)
            .map(|(id, _)| id);
        if (inner.jobs.len() >= self.settings.jobs && oldest_finished.is_none())
            || (!admin
                && inner
                    .jobs
                    .values()
                    .filter(|j| quota_ip(j.owner) == quota_ip(ip) && j.finished.is_none())
                    .count()
                    >= self.settings.per_ip)
        {
            return Err(busy());
        }
        let cached = inner.cache.get(&key).map(|c| c.policy.clone());
        let pending = inner.pending.get(&key).cloned();
        let cold = cached.is_none() && pending.is_none();
        if cold && inner.pending.len() >= self.settings.queue {
            return Err(busy());
        }
        if !admin {
            inner
                .quotas
                .charge(&self.access, ip, cold.then_some(request.config.mode), now)
                .map_err(|retry| Rejection {
                    code: "quota_exceeded",
                    message: "已达到访问配额，请稍后重试或使用访问密钥。",
                    retry,
                })?;
        }
        if inner.jobs.len() >= self.settings.jobs {
            inner.jobs.remove(&oldest_finished.unwrap());
        }
        if let Some(c) = inner.cache.get_mut(&key) {
            c.used = now;
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        inner.jobs.insert(
            id.clone(),
            Job {
                owner: ip,
                key: key.clone(),
                status: if cached.is_some() {
                    "querying"
                } else {
                    "queued"
                },
                result: None,
                error: None,
                finished: None,
            },
        );
        let receiver = if cold {
            let (tx, rx) = watch::channel(None);
            inner.pending.insert(key.clone(), rx.clone());
            let this = self.clone();
            let initial = request.clone();
            tokio::spawn(async move {
                this.build_policy(key, initial, tx).await;
            });
            Some(rx)
        } else {
            pending
        };
        drop(inner);
        let this = self.clone();
        let job_id = id.clone();
        let was_cached = cached.is_some();
        tokio::spawn(async move {
            let policy = match cached {
                Some(p) => Ok(p),
                None => {
                    let mut rx = receiver.unwrap();
                    loop {
                        if let Some(result) = rx.borrow_and_update().clone() {
                            break result;
                        }
                        if rx.changed().await.is_err() {
                            break Err(internal_error());
                        }
                    }
                }
            };
            let result = match policy {
                Ok(policy) => {
                    this.inner
                        .lock()
                        .unwrap()
                        .jobs
                        .get_mut(&job_id)
                        .unwrap()
                        .status = "querying";
                    let permit = this.query.clone().acquire_owned().await.unwrap();
                    tokio::task::spawn_blocking(move || {
                        let _permit = permit;
                        let mut result = policy
                            .lock()
                            .map_err(|_| internal_error())?
                            .solve(request)?;
                        result.cached = was_cached;
                        result.elapsed_ms = now.elapsed().as_millis();
                        serde_json::to_value(result).map_err(|_| internal_error())
                    })
                    .await
                    .unwrap_or_else(|_| Err(internal_error()))
                }
                Err(e) => Err(e),
            };
            let mut inner = this.inner.lock().unwrap();
            let job = inner.jobs.get_mut(&job_id).unwrap();
            match result {
                Ok(v) => {
                    job.result = Some(v);
                    job.status = "done";
                }
                Err(e) => {
                    job.error = Some(e);
                    job.status = "failed";
                }
            }
            job.finished = Some(Instant::now());
        });
        Ok(json!({"id":id,"status":"queued"}))
    }
    async fn build_policy(
        self: Arc<Self>,
        key: String,
        request: SolveRequest,
        tx: watch::Sender<Option<Built>>,
    ) {
        let _permit = self.build.clone().acquire_owned().await.unwrap();
        self.inner.lock().unwrap().active = Some(key.clone());
        // poll() reports this shared build state to every waiter.
        let settings = self.settings.clone();
        let mut result = tokio::task::spawn_blocking(move || {
            // This task is independent of HTTP connections.
            let mut service = echo_app::service_with_limits(settings.states, settings.transitions);
            service.solve(request)?;
            Ok(Arc::new(Mutex::new(service)))
        })
        .await
        .unwrap_or_else(|_| Err(internal_error()));
        let mut inner = self.inner.lock().unwrap();
        if let Ok(policy) = &result {
            // Account vector capacity plus 25% headroom. Temporary build buffers are separate.
            let bytes = policy.lock().unwrap().allocated_bytes().saturating_mul(5) / 4;
            while bytes <= self.settings.cache_bytes
                && (inner
                    .cache
                    .values()
                    .map(|c| c.bytes)
                    .sum::<usize>()
                    .saturating_add(bytes)
                    > self.settings.cache_bytes
                    || inner.cache.len() >= self.settings.cache_entries)
            {
                let oldest = inner
                    .cache
                    .iter()
                    .filter(|(_, c)| Arc::strong_count(&c.policy) == 1)
                    .min_by_key(|(_, c)| c.used)
                    .map(|(k, _)| k.clone());
                if let Some(k) = oldest {
                    inner.cache.remove(&k);
                } else {
                    break;
                }
            }
            if inner.cache.len() < self.settings.cache_entries
                && inner
                    .cache
                    .values()
                    .map(|c| c.bytes)
                    .sum::<usize>()
                    .saturating_add(bytes)
                    <= self.settings.cache_bytes
            {
                inner.cache.insert(
                    key.clone(),
                    Cached {
                        policy: policy.clone(),
                        bytes,
                        used: Instant::now(),
                    },
                );
            } else {
                result = Err(AppError {
                    code: "cache_capacity",
                    message: "策略超过缓存预算或缓存正在使用，请稍后重试或简化评分。".into(),
                });
            }
        }
        // Publish before removing pending so a new caller can never create a duplicate build.
        let _ = tx.send(Some(result));
        inner.pending.remove(&key);
        inner.active = None;
    }
    pub fn poll(&self, id: &str, ip: IpAddr) -> Option<Value> {
        let inner = self.inner.lock().unwrap();
        let j = inner.jobs.get(id).filter(|j| {
            j.owner == ip
                && j.finished
                    .is_none_or(|t| t.elapsed() < Duration::from_secs(60))
        })?;
        let status = if j.status == "queued" && inner.active.as_ref() == Some(&j.key) {
            "building"
        } else {
            j.status
        };
        Some(json!({"id":id,"status":status,"result":j.result,"error":j.error}))
    }
}
fn internal_error() -> AppError {
    AppError {
        code: "worker_failed",
        message: "求解任务异常结束，请重试。".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn server() -> Arc<Scheduler> {
        let mut a = Access::from_env();
        a.hourly = [1, 1, 1];
        a.rpm = 1000;
        Scheduler::new(Settings::from_env(), a)
    }
    fn req(target: f64) -> SolveRequest {
        SolveRequest {
            config: echo_app::Config {
                mode: echo_app::Mode::Upgrade,
                scoring: echo_app::Scoring::Fixed,
                weights: [1.; 13],
                target,
                ..Default::default()
            },
            echo: vec![],
            candidate: None,
        }
    }
    async fn done(s: &Scheduler, id: &str, ip: IpAddr) -> Value {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let j = s.poll(id, ip).unwrap();
                if j["status"] == "done" || j["status"] == "failed" {
                    return j;
                }
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .unwrap()
    }
    #[tokio::test]
    async fn shares_builds_preserves_owners_and_does_not_charge_cached_queries() {
        let s = server();
        let a = "1.1.1.1".parse().unwrap();
        let b = "2.2.2.2".parse().unwrap();
        let j1 = s.submit(req(3.), a, false).unwrap();
        let j2 = s.submit(req(3.), b, false).unwrap();
        assert_eq!(s.inner.lock().unwrap().pending.len(), 1);
        let id = j1["id"].as_str().unwrap();
        assert!(s.poll(id, b).is_none());
        assert_eq!(done(&s, id, a).await["status"], "done");
        assert_eq!(
            done(&s, j2["id"].as_str().unwrap(), b).await["result"]["decision"],
            "upgrade"
        );
        let cached = s.submit(req(3.), a, false).unwrap();
        assert_eq!(
            done(&s, cached["id"].as_str().unwrap(), a).await["result"]["cached"],
            true
        );
        assert_eq!(
            s.submit(req(4.), a, false).unwrap_err().code,
            "quota_exceeded"
        );
        let admin = s.submit(req(4.), a, true).unwrap();
        assert_eq!(
            done(&s, admin["id"].as_str().unwrap(), a).await["status"],
            "done"
        );
    }
    #[tokio::test]
    async fn bounded_queue_even_for_admin_and_cached_queries_continue_during_build() {
        let mut settings = Settings::from_env();
        settings.queue = 1;
        let s = Scheduler::new(settings, Access::from_env());
        let ip = "1.1.1.1".parse().unwrap();
        let first = s.submit(req(3.), ip, true).unwrap();
        done(&s, first["id"].as_str().unwrap(), ip).await;
        let held = s.build.acquire().await.unwrap();
        s.submit(req(4.), ip, true).unwrap();
        assert_eq!(s.submit(req(5.), ip, true).unwrap_err().code, "busy");
        let warm = s.submit(req(3.), ip, true).unwrap();
        assert_eq!(
            done(&s, warm["id"].as_str().unwrap(), ip).await["result"]["cached"],
            true
        );
        drop(held);
    }
    #[tokio::test]
    async fn ipv6_prefix_shares_pending_limit_but_not_result_access() {
        let s = server();
        let held = s.build.acquire().await.unwrap();
        let a = "2001:db8::1".parse().unwrap();
        let b = "2001:db8::2".parse().unwrap();
        let first = s.submit(req(3.), a, false).unwrap();
        s.submit(req(3.), b, false).unwrap();
        assert_eq!(
            s.submit(req(3.), "2001:db8::3".parse().unwrap(), false)
                .unwrap_err()
                .code,
            "busy"
        );
        assert!(s.poll(first["id"].as_str().unwrap(), b).is_none());
        assert!(
            s.submit(req(3.), "2001:db8:0:1::1".parse().unwrap(), false)
                .is_ok()
        );
        drop(held);
    }
    #[tokio::test]
    async fn rejected_admission_does_not_evict_another_result() {
        let mut settings = Settings::from_env();
        settings.jobs = 1;
        let mut access = Access::from_env();
        access.hourly = [0; 3];
        let s = Scheduler::new(settings, access);
        let a = "192.0.2.1".parse().unwrap();
        let b = "192.0.2.2".parse().unwrap();
        let first = s.submit(req(3.), a, true).unwrap();
        let id = first["id"].as_str().unwrap();
        assert_eq!(done(&s, id, a).await["status"], "done");
        assert_eq!(
            s.submit(req(4.), b, false).unwrap_err().code,
            "quota_exceeded"
        );
        assert_eq!(s.poll(id, a).unwrap()["status"], "done");
        let second = s.submit(req(4.), b, true).unwrap();
        assert!(s.poll(id, a).is_none());
        assert_eq!(
            done(&s, second["id"].as_str().unwrap(), b).await["status"],
            "done"
        );
    }
    #[tokio::test]
    async fn pinned_entries_and_byte_budget_are_enforced() {
        let mut settings = Settings::from_env();
        settings.cache_entries = 1;
        let s = Scheduler::new(settings.clone(), Access::from_env());
        let ip = "1.1.1.1".parse().unwrap();
        let first = s.submit(req(3.), ip, true).unwrap();
        done(&s, first["id"].as_str().unwrap(), ip).await;
        let pinned = s
            .inner
            .lock()
            .unwrap()
            .cache
            .values()
            .next()
            .unwrap()
            .policy
            .clone();
        let second = s.submit(req(4.), ip, true).unwrap();
        assert_eq!(
            done(&s, second["id"].as_str().unwrap(), ip).await["error"]["code"],
            "cache_capacity"
        );
        assert_eq!(s.inner.lock().unwrap().cache.len(), 1);
        drop(pinned);
        let second = s.submit(req(4.), ip, true).unwrap();
        assert_eq!(
            done(&s, second["id"].as_str().unwrap(), ip).await["status"],
            "done"
        );
        settings.cache_bytes = 1;
        let tiny = Scheduler::new(settings, Access::from_env());
        let job = tiny.submit(req(3.), ip, true).unwrap();
        assert_eq!(
            done(&tiny, job["id"].as_str().unwrap(), ip).await["error"]["code"],
            "cache_capacity"
        );
        assert!(tiny.inner.lock().unwrap().cache.is_empty());
    }
    #[tokio::test]
    async fn evicts_idle_policy_and_recovers_from_failed_build() {
        let mut settings = Settings::from_env();
        settings.cache_entries = 1;
        let s = Scheduler::new(settings, Access::from_env());
        let ip = "1.1.1.1".parse().unwrap();
        for target in [3., 4.] {
            let j = s.submit(req(target), ip, true).unwrap();
            assert_eq!(
                done(&s, j["id"].as_str().unwrap(), ip).await["status"],
                "done"
            );
        }
        assert_eq!(s.inner.lock().unwrap().cache.len(), 1);
        let mut bad = req(3.);
        bad.config.mode = echo_app::Mode::Joint;
        // Reject at solver construction, then verify the next independent job still completes.
        bad.config.costs[3] = -1.;
        let j = s.submit(bad, ip, true).unwrap();
        assert_eq!(
            done(&s, j["id"].as_str().unwrap(), ip).await["status"],
            "failed"
        );
        let j = s.submit(req(3.), ip, true).unwrap();
        assert_eq!(
            done(&s, j["id"].as_str().unwrap(), ip).await["status"],
            "done"
        );
        assert!(s.inner.lock().unwrap().pending.is_empty());
    }
}
