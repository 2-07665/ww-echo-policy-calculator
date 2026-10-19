//! Transport-independent application API. Every query carries its complete configuration;
//! the single-entry cache never acts as a caller's implicit session.
pub mod metadata;

use echo_policy::*;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub type Echo = Vec<(usize, u16)>;
pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
}
impl AppError {
    pub fn input(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_input",
            message: message.into(),
        }
    }
    pub fn busy() -> Self {
        Self {
            code: "busy",
            message: "求解器正在处理另一项计算，请稍后重试。".into(),
        }
    }
}
fn solver_error(error: impl std::fmt::Debug) -> AppError {
    let detail = format!("{error:?}");
    let (code, hint) = if detail.contains("TooLarge") || detail.contains("Overflow") {
        (
            "state_limit",
            "精确状态空间超过服务限制。请减少非零评分词条，或使用固定评分；服务管理员也可调整规模限制。",
        )
    } else if detail.contains("Impossible") {
        (
            "impossible_target",
            "当前评分规则无法达到这个目标，请降低目标分数。",
        )
    } else {
        ("solver_error", "未能完成求解，请检查配置。")
    };
    AppError {
        code,
        message: format!("{hint}（{detail}）"),
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Upgrade,
    Reroll,
    Joint,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Scoring {
    Linear,
    Fixed,
    WuwaEchoTool,
    McBoostAssistant,
    QqBot,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub mode: Mode,
    pub scoring: Scoring,
    pub weights: [f64; 13],
    pub main_score: f64,
    pub max_score: f64,
    pub target: f64,
    /// Echo, tuner, premium sealed tube, frequency relative values.
    pub costs: [f64; 4],
    pub refund: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Joint,
            scoring: Scoring::Linear,
            weights: [100., 100., 70., 0., 0., 36., 0., 0., 40., 0., 0., 0., 0.],
            main_score: 0.,
            max_score: 100.,
            target: 70.,
            costs: [0., 1., 0., 1000.],
            refund: 0.66,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolveRequest {
    pub config: Config,
    pub echo: Echo,
    #[serde(default)]
    pub candidate: Option<Echo>,
}
#[derive(Serialize)]
pub struct Preview {
    pub score: f64,
    pub main_score: f64,
    pub maximum: f64,
    pub contributions: Vec<f64>,
    pub target_internal: u16,
}
#[derive(Serialize)]
pub struct Resources {
    pub echoes: f64,
    pub tuners: f64,
    pub tubes: f64,
    pub frequencies: f64,
    pub weighted: f64,
    pub attempt_probability: f64,
}
#[derive(Serialize)]
pub struct Lock {
    pub mask: u16,
    pub types: Vec<usize>,
    pub cost: u8,
    pub expected: f64,
    pub probability: f64,
    pub regret: f64,
}
#[derive(Serialize)]
pub struct Comparison {
    pub accept: bool,
    pub current_cost: f64,
    pub candidate_cost: f64,
    pub candidate_score: f64,
}
#[derive(Serialize)]
pub struct SolveResponse {
    pub mode: Mode,
    pub preview: Preview,
    pub decision: &'static str,
    pub lock_mask: Option<u16>,
    pub resources: Option<Resources>,
    pub current_probability: Option<f64>,
    pub frequencies: Option<f64>,
    pub locks: Vec<Lock>,
    pub comparison: Option<Comparison>,
    pub cached: bool,
    pub elapsed_ms: u128,
}

enum Scorer {
    Linear(LinearScorer),
    Fixed(FixedScorer),
}
impl InternalScorer for Scorer {
    fn buff_score_internal(&self, index: usize, value: u16) -> Result<u16, ScorerError> {
        match self {
            Self::Linear(s) => s.buff_score_internal(index, value),
            Self::Fixed(s) => s.buff_score_internal(index, value),
        }
    }
}
impl Scorer {
    fn scale(&self) -> f64 {
        match self {
            Self::Fixed(_) => 1.,
            _ => SCORE_MULTIPLIER,
        }
    }
    fn main(&self) -> f64 {
        match self {
            Self::Linear(s) => s.main_buff_score(),
            _ => 0.,
        }
    }
    fn maximum(&self) -> f64 {
        let mut scores: Vec<_> = metadata::BUFF_TYPE_MAX_VALUES
            .iter()
            .enumerate()
            .map(|(i, v)| self.buff_score_internal(i, *v as u16).unwrap() as u32)
            .collect();
        scores.sort_unstable_by(|a, b| b.cmp(a));
        scores[..5].iter().sum::<u32>() as f64 / self.scale() + self.main()
    }
    fn target(&self, target: f64) -> AppResult<u16> {
        let score = ((target - self.main()).max(0.) * self.scale()).round();
        if !target.is_finite() || target < 0. || score > u16::MAX as f64 {
            return Err(AppError::input(
                "目标分数必须非负，且内部副词条目标不能超过 65535。",
            ));
        }
        if matches!(self, Self::Fixed(_)) && target.fract() != 0. {
            return Err(AppError::input("固定评分的目标必须为整数。"));
        }
        Ok(score as u16)
    }
    fn preview(&self, echo: &Echo, target: f64) -> AppResult<Preview> {
        validate_echo(echo, false)?;
        let contributions: Vec<_> = echo
            .iter()
            .map(|&(i, v)| {
                self.buff_score_internal(i, v)
                    .map(|s| s as f64 / self.scale())
            })
            .collect::<Result<_, _>>()
            .map_err(solver_error)?;
        Ok(Preview {
            score: self.main() + contributions.iter().sum::<f64>(),
            main_score: self.main(),
            maximum: self.maximum(),
            contributions,
            target_internal: self.target(target)?,
        })
    }
}
fn build_scorer(c: &Config) -> AppResult<Scorer> {
    if c.weights.iter().any(|v| !v.is_finite() || *v < 0.)
        || !c.main_score.is_finite()
        || !c.max_score.is_finite()
    {
        return Err(AppError::input("评分参数必须是有限非负数。"));
    }
    let scorer = match c.scoring {
        Scoring::Fixed => {
            if c.weights
                .iter()
                .any(|w| w.fract() != 0. || *w > u16::MAX as f64)
            {
                return Err(AppError::input("固定评分权重必须是 0～65535 的整数。"));
            }
            Scorer::Fixed(FixedScorer::new(c.weights.map(|v| v as u16)).map_err(solver_error)?)
        }
        Scoring::Linear | Scoring::WuwaEchoTool => Scorer::Linear(
            LinearScorer::new(c.weights, c.main_score, c.max_score).map_err(solver_error)?,
        ),
        Scoring::McBoostAssistant => Scorer::Linear(
            LinearScorer::mc_boost_assistant_scorer(c.weights).map_err(solver_error)?,
        ),
        Scoring::QqBot => Scorer::Linear(
            LinearScorer::qq_bot_scorer(c.weights, c.main_score).map_err(solver_error)?,
        ),
    };
    // Bound the score axis used by the upgrade DP as well as the value-state limits.
    if (scorer.maximum() - scorer.main()) * scorer.scale() > 20_000. {
        return Err(AppError::input(
            "应用支持最多 20000 内部副词条分数，请降低归一化满分或固定权重。",
        ));
    }
    scorer.target(c.target)?;
    Ok(scorer)
}
pub fn validate_echo(echo: &[(usize, u16)], full: bool) -> AppResult<()> {
    if echo.len() > 5 || (full && echo.len() != 5) {
        return Err(AppError::input(
            "重抽需要完整的 5 条词条；强化状态允许 0～5 条。",
        ));
    }
    let mut mask = 0u16;
    for &(i, v) in echo {
        if i >= 13 || !metadata::BUFF_VALUE_OPTIONS[i].contains(&v) {
            return Err(AppError::input("词条类型或数值档位无效。"));
        }
        if mask & (1 << i) != 0 {
            return Err(AppError::input("同一声骸不能包含重复词条。"));
        }
        mask |= 1 << i;
    }
    Ok(())
}
pub fn preview(request: &SolveRequest) -> AppResult<Preview> {
    build_scorer(&request.config)?.preview(&request.echo, request.config.target)
}

/// Validate without allocating a solver, for server admission before charging quota.
pub fn validate_request(request: &SolveRequest) -> AppResult<()> {
    prepare(request).map(|_| ())
}
fn prepare(request: &SolveRequest) -> AppResult<(Scorer, Preview, CostModel)> {
    let c = &request.config;
    validate_echo(&request.echo, c.mode == Mode::Reroll)?;
    if let Some(candidate) = &request.candidate {
        if c.mode != Mode::Reroll {
            return Err(AppError::input("仅重抽模式支持候选比较。"));
        }
        validate_echo(&request.echo, true)?;
        validate_echo(candidate, true)?;
    }
    let scorer = build_scorer(c)?;
    let preview = scorer.preview(&request.echo, c.target)?;
    if c.mode != Mode::Reroll && c.costs.iter().any(|v| !v.is_finite() || *v < 0.) {
        return Err(AppError::input("资源权重必须是有限非负数。"));
    }
    let cost = if c.mode == Mode::Reroll {
        CostModel::tuner_only()
    } else {
        CostModel::new(c.costs[0], c.costs[1], c.costs[2], c.refund).map_err(solver_error)?
    };
    if preview.target_internal as f64 > (scorer.maximum() - scorer.main()) * scorer.scale() + 0.5 {
        return Err(AppError {
            code: "impossible_target",
            message: "当前评分规则无法达到这个目标，请降低目标分数。".into(),
        });
    }
    Ok((scorer, preview, cost))
}

enum Policy {
    Upgrade(UpgradePolicySolver),
    Reroll(RerollPolicySolver),
    Joint(JointPolicySolver),
}
struct Cached {
    config: Config,
    policy: Policy,
}
pub struct Service {
    cache: Option<Cached>,
    limits: SolverLimits,
}
impl Default for Service {
    fn default() -> Self {
        Self::new(SolverLimits {
            max_states: 750_000,
            max_transitions: 24_000_000,
        })
    }
}
impl Service {
    /// Bytes retained by the policy (allocator overhead is not included).
    pub fn allocated_bytes(&self) -> usize {
        self.cache.as_ref().map_or(0, |c| match &c.policy {
            Policy::Upgrade(s) => s.allocated_bytes(),
            Policy::Reroll(s) => s.allocated_bytes(),
            Policy::Joint(s) => s.allocated_bytes(),
        })
    }
    pub fn new(limits: SolverLimits) -> Self {
        Self {
            cache: None,
            limits,
        }
    }
    pub fn solve(&mut self, request: SolveRequest) -> AppResult<SolveResponse> {
        let started = Instant::now();
        let c = &request.config;
        let (scorer, preview, cost) = prepare(&request)?;
        let cached = self.cache.as_ref().is_some_and(|s| s.config == *c);
        if !cached {
            // Drop the old table before building the next one to bound peak memory.
            self.cache = None;
            let policy = match c.mode {
                Mode::Upgrade => {
                    let mut s = UpgradePolicySolver::new(
                        &scorer,
                        preview.target_internal as f64 / SCORE_MULTIPLIER,
                        cost,
                    )
                    .map_err(solver_error)?;
                    s.lambda_search(1e-9, 100).map_err(solver_error)?;
                    Policy::Upgrade(s)
                }
                Mode::Reroll => {
                    let mut s = RerollPolicySolver::with_scorer_and_limits(&scorer, self.limits)
                        .map_err(solver_error)?;
                    s.set_target(preview.target_internal)
                        .map_err(solver_error)?;
                    s.solve().map_err(solver_error)?;
                    Policy::Reroll(s)
                }
                Mode::Joint => {
                    let mut s = JointPolicySolver::with_limits(
                        &scorer,
                        cost,
                        c.costs[3],
                        preview.target_internal,
                        self.limits,
                    )
                    .map_err(solver_error)?;
                    s.derive_policy(1e-9, 100).map_err(solver_error)?;
                    Policy::Joint(s)
                }
            };
            self.cache = Some(Cached {
                config: c.clone(),
                policy,
            });
        }
        let echo = &request.echo;
        let score = scorer.echo_score_internal(echo).map_err(solver_error)?;
        let mut result = SolveResponse {
            mode: c.mode,
            preview,
            decision: "",
            lock_mask: None,
            resources: None,
            current_probability: None,
            frequencies: None,
            locks: vec![],
            comparison: None,
            cached,
            elapsed_ms: 0,
        };
        let reroll = match &mut self.cache.as_mut().unwrap().policy {
            Policy::Upgrade(s) => {
                let mask = echo.iter().fold(0, |m, (i, _)| m | 1 << i);
                result.decision = if echo.len() == 5 {
                    if score >= result.preview.target_internal {
                        "keep"
                    } else {
                        "abandon"
                    }
                } else if s.get_decision(mask, score).map_err(solver_error)? {
                    "upgrade"
                } else {
                    "abandon"
                };
                result.current_probability = Some(
                    s.get_success_probability(mask, score)
                        .map_err(solver_error)?,
                );
                let r = s.calculate_expected_resources().map_err(solver_error)?;
                result.resources = Some(Resources {
                    echoes: r.echo_per_success(),
                    tuners: r.tuner_per_success(),
                    tubes: r.exp_per_success(),
                    frequencies: 0.,
                    weighted: s.weighted_expected_cost().map_err(solver_error)?,
                    attempt_probability: r.success_probability(),
                });
                None
            }
            Policy::Reroll(s) => {
                result.decision = if score >= result.preview.target_internal {
                    "keep"
                } else {
                    "reroll"
                };
                Some(&*s)
            }
            Policy::Joint(s) => {
                result.decision = match s.get_decision(echo).map_err(solver_error)? {
                    JointDecision::Upgrade => "upgrade",
                    JointDecision::Abandon => "abandon",
                    JointDecision::Keep => "keep",
                    JointDecision::Reroll { .. } => "reroll",
                };
                result.current_probability =
                    Some(s.get_success_probability(echo).map_err(solver_error)?);
                let r = s.calculate_expected_resources().map_err(solver_error)?;
                result.resources = Some(Resources {
                    echoes: r.echo_per_success(),
                    tuners: r.tuner_per_success(),
                    tubes: r.exp_per_success(),
                    frequencies: r.frequency_per_success(),
                    weighted: s.weighted_expected_cost().map_err(solver_error)?,
                    attempt_probability: r.success_probability(),
                });
                // Joint hands off to reroll mode; avoid querying unused lock alternatives.
                None
            }
        };
        if let Some(s) = reroll.filter(|_| echo.len() == 5) {
            result.frequencies = Some(s.expected_lock_cost_for_echo(echo).map_err(solver_error)?);
            result.locks = s
                .lock_choices_for_echo(echo, 5)
                .map_err(solver_error)?
                .into_iter()
                .map(|l| Lock {
                    mask: l.lock_mask,
                    types: (0..13).filter(|i| l.lock_mask & (1 << i) != 0).collect(),
                    cost: match l.lock_mask.count_ones() {
                        0..=2 => 1,
                        3 => 2,
                        _ => 3,
                    },
                    expected: l.expected_cost,
                    probability: l.success_probability,
                    regret: l.regret,
                })
                .collect();
            result.lock_mask = s.best_lock_choices_for_echo(echo).map_err(solver_error)?;
            if let Some(candidate) = &request.candidate {
                result.comparison = Some(Comparison {
                    accept: s
                        .should_accept_echo(echo, candidate)
                        .map_err(solver_error)?,
                    current_cost: result.frequencies.unwrap(),
                    candidate_cost: s
                        .expected_lock_cost_for_echo(candidate)
                        .map_err(solver_error)?,
                    candidate_score: scorer.preview(candidate, c.target)?.score,
                });
            }
        }
        // JSON cannot represent infinities; fail explicitly instead of sending null metrics.
        let mut metrics = vec![];
        if let Some(r) = &result.resources {
            metrics.extend([
                r.echoes,
                r.tuners,
                r.tubes,
                r.frequencies,
                r.weighted,
                r.attempt_probability,
            ]);
        }
        metrics.extend(result.frequencies);
        metrics.extend(result.current_probability);
        for l in &result.locks {
            metrics.extend([l.expected, l.probability, l.regret]);
        }
        if let Some(r) = &result.comparison {
            metrics.extend([r.current_cost, r.candidate_cost]);
        }
        if metrics.iter().any(|v| !v.is_finite()) {
            return Err(AppError {
                code: "numerical_failure",
                message: "计算结果超出有限数值范围，请调整目标或权重。".into(),
            });
        }
        result.elapsed_ms = started.elapsed().as_millis();
        Ok(result)
    }
}

pub fn bootstrap() -> serde_json::Value {
    use metadata::*;
    serde_json::json!({"defaults":Config::default(),"stats":(0..13).map(|i|serde_json::json!({
        "id":BUFF_TYPES[i],"label":BUFF_LABELS[i],"values":BUFF_VALUE_OPTIONS[i],"percent":!(5..=7).contains(&i)
    })).collect::<Vec<_>>(),"presets": {
        "linear":serde_json::from_str::<serde_json::Value>(include_str!("../presets/linear_default.json")).unwrap(),
        "fixed":serde_json::from_str::<serde_json::Value>(include_str!("../presets/fixed.json")).unwrap(),
        "wuwa_echo_tool":serde_json::from_str::<serde_json::Value>(include_str!("../presets/wuwa_echo_tool.json")).unwrap(),
        "mc_boost_assistant":serde_json::from_str::<serde_json::Value>(include_str!("../presets/mc_boost_assistant.json")).unwrap(),
        "qq_bot":serde_json::from_str::<serde_json::Value>(include_str!("../presets/qq_bot.json")).unwrap()
    }})
}

pub fn service_with_limits(states: usize, transitions: usize) -> Service {
    Service::new(SolverLimits {
        max_states: states,
        max_transitions: transitions,
    })
}
