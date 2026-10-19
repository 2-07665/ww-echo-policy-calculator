//! Quotas are intentionally process-local. Only explicitly trusted peers may forward IPs.
use axum::http::HeaderMap;
use echo_app::Mode;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    net::IpAddr,
    time::{Duration, Instant},
};

pub fn number(name: &str, default: usize) -> usize {
    std::env::var(name)
        .map(|v| {
            v.parse()
                .unwrap_or_else(|_| panic!("{name} must be a nonnegative integer"))
        })
        .unwrap_or(default)
}
pub fn digest(key: &str) -> String {
    format!("{:x}", Sha256::digest(key.as_bytes()))
}
pub fn generate_key() {
    let key = format!(
        "echo_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    println!(
        "Access key (save privately; shown only here):\n{key}\n\nServer configuration:\nECHO_ADMIN_KEY_HASHES={}",
        digest(&key)
    );
}
#[derive(Clone)]
pub struct Access {
    pub hashes: Vec<String>,
    pub proxies: Vec<IpAddr>,
    pub hourly: [usize; 3],
    pub rpm: usize,
    pub identities: usize,
}
impl Access {
    pub fn from_env() -> Self {
        let hashes: Vec<String> = std::env::var("ECHO_ADMIN_KEY_HASHES")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_ascii_lowercase())
            .collect();
        assert!(
            hashes
                .iter()
                .all(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())),
            "ECHO_ADMIN_KEY_HASHES requires comma-separated SHA-256 digests"
        );
        let proxies = std::env::var("ECHO_TRUSTED_PROXIES")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                normalize(
                    s.trim()
                        .parse()
                        .expect("ECHO_TRUSTED_PROXIES requires IP addresses"),
                )
            })
            .collect();
        Self {
            hashes,
            proxies,
            hourly: [
                number("ECHO_QUOTA_UPGRADE", 60),
                number("ECHO_QUOTA_REROLL", 12),
                number("ECHO_QUOTA_JOINT", 4),
            ],
            rpm: number("ECHO_REQUESTS_PER_MINUTE", 240),
            identities: number("ECHO_MAX_IDENTITIES", 4096),
        }
    }
    pub fn admin(&self, headers: &HeaderMap) -> bool {
        let Some(key) = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
        else {
            return false;
        };
        if key.len() > 256 {
            return false;
        }
        let hash = digest(key);
        self.hashes.iter().any(|h| {
            h.bytes()
                .zip(hash.bytes())
                .fold(0u8, |v, (a, b)| v | (a ^ b))
                == 0
        })
    }
    pub fn ip(&self, peer: IpAddr, headers: &HeaderMap) -> IpAddr {
        let mut current = normalize(peer);
        if !self.proxies.contains(&current) {
            return current;
        }
        if let Some(chain) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
            for hop in chain.rsplit(',') {
                if !self.proxies.contains(&current) {
                    break;
                }
                match hop.trim().parse() {
                    Ok(ip) => current = normalize(ip),
                    Err(_) => return normalize(peer),
                }
            }
        }
        current
    }
}
fn normalize(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v) => v.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        _ => ip,
    }
}
pub fn quota_ip(ip: IpAddr) -> IpAddr {
    // A single IPv6 allocation cannot get fresh quota by rotating interface addresses.
    match ip {
        IpAddr::V6(v) => IpAddr::V6((u128::from(v) & (!0u128 << 64)).into()),
        _ => ip,
    }
}
struct Counter {
    start: Instant,
    used: usize,
}
impl Counter {
    fn new(now: Instant) -> Self {
        Self {
            start: now,
            used: 0,
        }
    }
    fn refresh(&mut self, now: Instant, window: Duration) {
        if now.duration_since(self.start) >= window {
            *self = Self::new(now);
        }
    }
}
struct Usage {
    minute: Counter,
    hour: Counter,
    modes: [usize; 3],
    last: Instant,
}
#[derive(Default)]
pub struct Quotas {
    users: HashMap<IpAddr, Usage>,
}
impl Quotas {
    /// A failed admission does not consume a cold-build allowance. Failed admitted builds do.
    pub fn charge(
        &mut self,
        access: &Access,
        ip: IpAddr,
        mode: Option<Mode>,
        now: Instant,
    ) -> Result<(), u64> {
        self.users
            .retain(|_, u| now.duration_since(u.last) < Duration::from_secs(3600));
        let ip = quota_ip(ip);
        if !self.users.contains_key(&ip) && self.users.len() >= access.identities {
            return Err(60);
        }
        let u = self.users.entry(ip).or_insert_with(|| Usage {
            minute: Counter::new(now),
            hour: Counter::new(now),
            modes: [0; 3],
            last: now,
        });
        u.last = now;
        u.minute.refresh(now, Duration::from_secs(60));
        if now.duration_since(u.hour.start) >= Duration::from_secs(3600) {
            u.hour = Counter::new(now);
            u.modes = [0; 3];
        }
        let index = mode.map(|m| match m {
            Mode::Upgrade => 0,
            Mode::Reroll => 1,
            Mode::Joint => 2,
        });
        if u.minute.used >= access.rpm {
            return Err(60 - now.duration_since(u.minute.start).as_secs());
        }
        if let Some(i) = index {
            if u.modes[i] >= access.hourly[i] {
                return Err(3600 - now.duration_since(u.hour.start).as_secs());
            }
            u.modes[i] += 1;
        }
        u.minute.used += 1;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_and_key_are_not_spoofable() {
        let mut a = Access::from_env();
        a.hashes = vec![digest("secret")];
        a.proxies = vec!["127.0.0.1".parse().unwrap()];
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(a.ip("9.8.7.6".parse().unwrap(), &h).to_string(), "9.8.7.6");
        assert_eq!(
            a.ip("127.0.0.1".parse().unwrap(), &h).to_string(),
            "5.6.7.8"
        );
        assert!(!a.admin(&h));
        h.insert("authorization", "Bearer secret".parse().unwrap());
        assert!(a.admin(&h));
        h.insert("authorization", "Bearer wrong".parse().unwrap());
        assert!(!a.admin(&h));
    }
    #[test]
    fn request_and_identity_limits_remain_bounded() {
        let mut a = Access::from_env();
        a.rpm = 1;
        a.identities = 1;
        let mut q = Quotas::default();
        let now = Instant::now();
        let ip = "192.0.2.1".parse().unwrap();
        assert!(q.charge(&a, ip, None, now).is_ok());
        assert!(q.charge(&a, ip, None, now).is_err());
        assert!(
            q.charge(&a, "192.0.2.2".parse().unwrap(), None, now)
                .is_err()
        );
        assert_eq!(q.users.len(), 1);
        assert!(
            q.charge(&a, ip, None, now + Duration::from_secs(60))
                .is_ok()
        );
        assert!(
            q.charge(
                &a,
                "192.0.2.2".parse().unwrap(),
                None,
                now + Duration::from_secs(3660)
            )
            .is_ok()
        );
        assert_eq!(q.users.len(), 1);
    }
    #[test]
    fn quota_windows_modes_and_ipv6_prefix() {
        let mut a = Access::from_env();
        a.hourly = [1, 1, 0];
        a.rpm = 10;
        let mut q = Quotas::default();
        let now = Instant::now();
        let ip = "2001:db8::1".parse().unwrap();
        assert!(q.charge(&a, ip, Some(Mode::Upgrade), now).is_ok());
        assert!(
            q.charge(&a, "2001:db8::2".parse().unwrap(), Some(Mode::Upgrade), now)
                .is_err()
        );
        assert!(q.charge(&a, ip, None, now).is_ok());
        assert!(q.charge(&a, ip, Some(Mode::Reroll), now).is_ok());
        assert!(q.charge(&a, ip, Some(Mode::Joint), now).is_err());
        assert!(
            q.charge(&a, ip, Some(Mode::Upgrade), now + Duration::from_secs(3600))
                .is_ok()
        );
    }
}
