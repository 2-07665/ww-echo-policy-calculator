use std::{cmp::Ordering, collections::BinaryHeap};

use crate::data::{NUM_BUFFS, NUM_ECHO_SLOTS};
use crate::value_state::{Config, SolverLimits, ValueSpace, choose};
use crate::{FixedScorer, InternalScorer, ScorerError};

fn lock_cost(k: usize) -> f64 {
    match k {
        0..=2 => 1.0,
        3 => 2.0,
        4 => 3.0,
        _ => unreachable!(),
    }
}

/// Invalid configuration/query, unavailable policy, allocation limit or numeric failure.
#[derive(Debug)]
pub enum RerollPolicySolverError {
    PolicyNotDerived,
    FailedtoConvergeWithinMaxIter,
    AllWeightsZero,
    TopWeightsTooLarge {
        sum: u32,
    },
    InvalidMask {
        mask: u16,
    },
    InvalidTolerance {
        tolerance: f64,
    },
    TargetScoreImpossible {
        target_score: u16,
        max_score: u16,
    },
    TargetNotSet,
    Scorer(ScorerError),
    InvalidScorePmf {
        buff_index: Option<usize>,
    },
    ScoreRangeOverflow {
        max_score: u32,
    },
    InvalidEcho,
    /// Use the corresponding `_for_echo` method to supply numeric values.
    ValuesRequired,
    StateSpaceTooLarge {
        states: usize,
        limit: usize,
    },
    TransitionSpaceTooLarge {
        transitions: usize,
        limit: usize,
    },
    StateEncodingOverflow,
    NumericalFailure,
}

impl From<ScorerError> for RerollPolicySolverError {
    fn from(err: ScorerError) -> Self {
        match err {
            ScorerError::AllWeightsZero => Self::AllWeightsZero,
            ScorerError::FixedScorerTopWeightsTooLarge { sum } => Self::TopWeightsTooLarge { sum },
            other => Self::Scorer(other),
        }
    }
}

/// One retained lock option, evaluated for a specific baseline echo.
#[derive(Clone, Debug)]
pub struct LockChoice {
    /// Types to lock; their current numeric values are preserved.
    pub lock_mask: u16,
    /// Expected total frequency resource consumption when taking this action
    /// once, then following the optimal policy (including rejecting results).
    pub expected_cost: f64,
    /// Extra expected cost relative to the first (best) returned lock option.
    pub regret: f64,
    /// Probability of immediately reaching the target on this reroll.
    pub success_probability: f64,
}

struct Action {
    code: u64,
    probability: f64,
    success_mass: f64,
    start: usize,
    end: usize,
    cost: f64,
}

#[derive(Clone, Copy)]
struct Entry {
    value: f64,
    action: usize,
    version: usize,
}
impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Entry {}
impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .value
            .total_cmp(&self.value)
            .then_with(|| other.action.cmp(&self.action))
            .then(self.version.cmp(&other.version))
    }
}
impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Exact reroll solver for additive, nonnegative `InternalScorer` scores.
/// Numeric outcomes with equal scores are merged; permanently zero-scoring
/// types are interchangeable. Results may be rejected without another charge.
/// Costs are expected frequency resource units, not a finite inventory budget.
/// Targets are internal substat scores (exclude the main-stat score).
///
/// `new(weights)` and mask queries remain available for type-only scoring.
/// Use `with_scorer` and `_for_echo` queries for value-dependent scoring.
pub struct RerollPolicySolver {
    pub(crate) space: ValueSpace,
    pub(crate) states: Vec<Config>,
    pub(crate) values: Vec<f64>,
    pub(crate) target_score: Option<u16>,
    actions: Vec<Action>,
    neighbors: Vec<u32>,
    best_locks: Vec<u16>,
    policy_derived: bool,
}

impl RerollPolicySolver {
    /// Retained allocation capacity, excluding allocator overhead and temporary solve buffers.
    pub fn allocated_bytes(&self) -> usize {
        size_of::<Self>()
            + self.space.heap_bytes()
            + self.states.capacity() * size_of::<Config>()
            + self.values.capacity() * size_of::<f64>()
            + self.actions.capacity() * size_of::<Action>()
            + self.neighbors.capacity() * size_of::<u32>()
            + self.best_locks.capacity() * size_of::<u16>()
    }

    /// Construct a type-only solver from internal integer type weights. Set a target before solving.
    pub fn new(weights: [u16; NUM_BUFFS]) -> Result<Self, RerollPolicySolverError> {
        Self::with_scorer(&FixedScorer::new(weights)?)
    }

    /// Construct a value-aware solver with default allocation limits; the scorer is not retained.
    pub fn with_scorer<S: InternalScorer>(scorer: &S) -> Result<Self, RerollPolicySolverError> {
        Self::with_scorer_and_limits(scorer, SolverLimits::default())
    }

    /// Construct compressed states and lock memberships with explicit allocation limits.
    /// Extremely small joint probabilities return `NumericalFailure` rather than
    /// allowing subnormal rounding to corrupt expected costs.
    pub fn with_scorer_and_limits<S: InternalScorer>(
        scorer: &S,
        limits: SolverLimits,
    ) -> Result<Self, RerollPolicySolverError> {
        let space = ValueSpace::new(scorer, limits)?;
        space.check_count(space.count(0, 4))?;
        let states = space.configs(space.min_count(5), 5);
        // Subnormal joint probabilities lose relative precision before they
        // underflow to zero. Dividing by these masses can materially corrupt
        // lock thresholds, so reject them rather than report inaccurate costs.
        if states.iter().any(|s| {
            let mass = choose(space.zeros, NUM_ECHO_SLOTS - s.count)
                / choose(NUM_BUFFS, NUM_ECHO_SLOTS)
                * s.probability;
            !mass.is_normal()
        }) {
            return Err(RerollPolicySolverError::NumericalFailure);
        }
        let transitions = states
            .iter()
            .map(|s| (1usize << s.count) - usize::from(s.count == 5))
            .sum::<usize>();
        if transitions > limits.max_transitions {
            return Err(RerollPolicySolverError::TransitionSpaceTooLarge {
                transitions,
                limit: limits.max_transitions,
            });
        }
        let mut actions: Vec<_> = space
            .configs(0, 4)
            .into_iter()
            .map(|s| Action {
                code: s.code,
                probability: choose(NUM_BUFFS - s.count, NUM_ECHO_SLOTS - s.count)
                    / choose(NUM_BUFFS, NUM_ECHO_SLOTS)
                    * s.probability,
                success_mass: 0.0,
                start: 0,
                end: 0,
                cost: lock_cost(s.count),
            })
            .collect();
        if actions.iter().any(|a| !a.probability.is_normal()) {
            return Err(RerollPolicySolverError::NumericalFailure);
        }
        let mut subsets = Vec::with_capacity(32);
        for s in &states {
            space.subsets(s.code, &mut subsets);
            for &code in &subsets {
                actions[space.rank(code, 0, 4)].end += 1;
            }
        }
        let mut offset = 0;
        for a in &mut actions {
            let len = a.end;
            a.start = offset;
            a.end = offset;
            offset += len;
        }
        let mut neighbors = vec![0u32; transitions];
        for (i, s) in states.iter().enumerate() {
            space.subsets(s.code, &mut subsets);
            for &code in &subsets {
                let a = &mut actions[space.rank(code, 0, 4)];
                neighbors[a.end] = i as u32;
                a.end += 1;
            }
        }
        let count = states.len();
        Ok(Self {
            space,
            states,
            values: vec![f64::INFINITY; count],
            target_score: None,
            actions,
            neighbors,
            best_locks: vec![0; count],
            policy_derived: false,
        })
    }

    /// Whether an internal-score target has been set.
    pub fn is_target_set(&self) -> bool {
        self.target_score.is_some()
    }
    /// Whether queries can use the derived policy for the current target.
    pub fn is_policy_derived(&self) -> bool {
        self.policy_derived
    }
    /// Number of compressed full-echo states, excluding lock configurations.
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Set an internal substat target, invalidating the derived policy on success.
    /// An invalid target leaves the existing configuration unchanged.
    pub fn set_target(&mut self, target_score: u16) -> Result<(), RerollPolicySolverError> {
        if target_score > self.space.max_score {
            return Err(RerollPolicySolverError::TargetScoreImpossible {
                target_score,
                max_score: self.space.max_score,
            });
        }
        self.target_score = Some(target_score);
        self.policy_derived = false;
        Ok(())
    }

    /// Compatibility entry point. The new finite label-setting algorithm has
    /// no convergence iterations: any positive iteration budget runs to
    /// completion. `tol` is still validated, but does not approximate scores.
    pub fn derive_policy(
        &mut self,
        tol: f64,
        max_iter: usize,
    ) -> Result<(), RerollPolicySolverError> {
        if !self.is_target_set() {
            return Err(RerollPolicySolverError::TargetNotSet);
        }
        if !tol.is_finite() || tol <= 0.0 {
            return Err(RerollPolicySolverError::InvalidTolerance { tolerance: tol });
        }
        if max_iter == 0 {
            self.policy_derived = false;
            return Err(RerollPolicySolverError::FailedtoConvergeWithinMaxIter);
        }
        self.solve()
    }

    /// Solve without value iteration. For a lock configuration L, its optimal
    /// threshold r satisfies E[(r - V(Y))_+ | L] = cost(L). Settling thresholds
    /// in ascending order visits each state once; higher-valued outcomes can
    /// always be rejected and therefore cannot affect an earlier threshold.
    pub fn solve(&mut self) -> Result<(), RerollPolicySolverError> {
        let target = self
            .target_score
            .ok_or(RerollPolicySolverError::TargetNotSet)?;
        self.policy_derived = false;
        self.values.fill(f64::INFINITY);
        let mut mass = vec![0.0; self.actions.len()];
        let mut sum = vec![0.0; self.actions.len()];
        let mut left: Vec<_> = self.actions.iter().map(|a| a.end - a.start).collect();
        let mut version = vec![0usize; self.actions.len()];
        let mut subsets = Vec::with_capacity(32);
        let mut remaining = self.states.len();
        for (i, s) in self.states.iter().enumerate() {
            if s.score >= target {
                self.values[i] = 0.0;
                remaining -= 1;
                let p = self.state_mass(i);
                self.space.subsets(s.code, &mut subsets);
                for &code in &subsets {
                    let a = self.space.rank(code, 0, 4);
                    mass[a] += p;
                    left[a] -= 1;
                }
            }
        }
        let mut heap = BinaryHeap::new();
        for (i, a) in self.actions.iter_mut().enumerate() {
            a.success_mass = mass[i];
            if mass[i] > 0.0 && left[i] > 0 {
                heap.push(Entry {
                    value: a.cost * a.probability / mass[i],
                    action: i,
                    version: 0,
                });
            }
        }
        let mut dirty = vec![false; self.actions.len()];
        let mut touched = Vec::new();
        while remaining > 0 {
            let entry = loop {
                let e = heap
                    .pop()
                    .ok_or(RerollPolicySolverError::NumericalFailure)?;
                if version[e.action] == e.version && left[e.action] > 0 {
                    break e;
                }
            };
            if !entry.value.is_finite() {
                return Err(RerollPolicySolverError::NumericalFailure);
            }
            let action = &self.actions[entry.action];
            let lock_mask = self.space.mask(action.code);
            for edge in action.start..action.end {
                let i = self.neighbors[edge] as usize;
                if self.values[i].is_finite() {
                    continue;
                }
                self.values[i] = entry.value;
                self.best_locks[i] = lock_mask;
                remaining -= 1;
                let p = self.state_mass(i);
                self.space.subsets(self.states[i].code, &mut subsets);
                for &code in &subsets {
                    let a = self.space.rank(code, 0, 4);
                    mass[a] += p;
                    sum[a] += p * entry.value;
                    left[a] -= 1;
                    if !dirty[a] {
                        dirty[a] = true;
                        touched.push(a);
                    }
                }
            }
            for a in touched.drain(..) {
                dirty[a] = false;
                version[a] += 1;
                if mass[a] > 0.0 && left[a] > 0 {
                    let value =
                        (self.actions[a].cost * self.actions[a].probability + sum[a]) / mass[a];
                    heap.push(Entry {
                        value: value.max(entry.value),
                        action: a,
                        version: version[a],
                    });
                }
            }
        }
        self.policy_derived = true;
        Ok(())
    }

    fn state_mass(&self, i: usize) -> f64 {
        let s = &self.states[i];
        choose(self.space.zeros, 5 - s.count) / choose(NUM_BUFFS, 5) * s.probability
    }

    fn check_policy(&self) -> Result<(), RerollPolicySolverError> {
        if self.policy_derived {
            Ok(())
        } else {
            Err(RerollPolicySolverError::PolicyNotDerived)
        }
    }

    pub(crate) fn index(&self, code: u64) -> usize {
        self.space.rank(code, self.space.min_count(5), 5)
    }
    fn mask_index(&self, mask: u16) -> Result<usize, RerollPolicySolverError> {
        self.check_policy()?;
        Ok(self.index(self.space.mask_code(mask)?))
    }
    fn echo_index(&self, echo: &[(usize, u16)]) -> Result<usize, RerollPolicySolverError> {
        self.check_policy()?;
        Ok(self.index(self.space.echo_code(echo, true)?))
    }
    fn best_lock(&self, index: usize) -> Option<u16> {
        if self.values[index] == 0.0 {
            None
        } else {
            Some(self.best_locks[index])
        }
    }
    fn choices(&self, index: usize, top_k: usize) -> Vec<LockChoice> {
        if self.values[index] == 0.0 {
            return Vec::new();
        }
        let baseline = self.values[index];
        let mut subsets = Vec::with_capacity(32);
        self.space.subsets(self.states[index].code, &mut subsets);
        let mut choices = Vec::with_capacity(subsets.len());
        for code in subsets {
            let a = &self.actions[self.space.rank(code, 0, 4)];
            let expected = a.cost
                + self.neighbors[a.start..a.end]
                    .iter()
                    .map(|&i| {
                        self.state_mass(i as usize) / a.probability
                            * baseline.min(self.values[i as usize])
                    })
                    .sum::<f64>();
            choices.push(LockChoice {
                lock_mask: self.space.mask(code),
                expected_cost: expected,
                regret: 0.0,
                success_probability: (a.success_mass / a.probability).clamp(0.0, 1.0),
            });
        }
        choices.sort_by(|a, b| {
            a.expected_cost
                .total_cmp(&b.expected_cost)
                .then(a.lock_mask.cmp(&b.lock_mask))
        });
        let best = choices[0].expected_cost;
        for choice in &mut choices {
            choice.regret = choice.expected_cost - best;
        }
        if top_k > 0 {
            choices.truncate(top_k);
        }
        choices
    }
    fn best_success(&self, index: usize) -> Option<f64> {
        let mask = self.best_lock(index)?;
        let mut code = 0;
        for (i, &t) in self.space.types.iter().enumerate() {
            if mask & (1 << t) != 0 {
                code += self.space.digit(self.states[index].code, i) as u64 * self.space.strides[i];
            }
        }
        let a = &self.actions[self.space.rank(code, 0, 4)];
        Some((a.success_mass / a.probability).clamp(0.0, 1.0))
    }

    /// Expected total frequency units to reach the target from a full type mask.
    /// Returns `ValuesRequired` when a present type has multiple score buckets.
    pub fn expected_lock_cost(&self, mask: u16) -> Result<f64, RerollPolicySolverError> {
        Ok(self.values[self.mask_index(mask)?])
    }
    /// Return one optimal lock mask, or `None` if already successful. Requires a full mask.
    pub fn best_lock_choices(&self, mask: u16) -> Result<Option<u16>, RerollPolicySolverError> {
        Ok(self.best_lock(self.mask_index(mask)?))
    }
    /// Rank lock options by expected total cost; `top_k = 0` returns all retained options.
    /// Successful states return an empty vector. Permanently zero-score locks are omitted.
    pub fn lock_choices(
        &self,
        mask: u16,
        top_k: usize,
    ) -> Result<Vec<LockChoice>, RerollPolicySolverError> {
        Ok(self.choices(self.mask_index(mask)?, top_k))
    }
    /// Probability the recommended action succeeds on the next reroll; `None` if already successful.
    pub fn best_lock_success_probability(
        &self,
        mask: u16,
    ) -> Result<Option<f64>, RerollPolicySolverError> {
        Ok(self.best_success(self.mask_index(mask)?))
    }
    /// Accept a candidate if its optimal remaining cost is no greater than the baseline’s.
    /// Both masks must be full. Does not check compatibility with a previously selected lock.
    pub fn should_accept(
        &self,
        baseline_mask: u16,
        candidate_mask: u16,
    ) -> Result<bool, RerollPolicySolverError> {
        Ok(self.expected_lock_cost(candidate_mask)? <= self.expected_lock_cost(baseline_mask)?)
    }

    /// Echo entries use actual in-game substat values in the same units as
    /// `InternalScorer::buff_score_internal`, not already-computed scores.
    /// Requires five distinct types with numeric values from the built-in legal histograms.
    /// Returns zero when already successful; otherwise includes repeated rerolls.
    pub fn expected_lock_cost_for_echo(
        &self,
        echo: &[(usize, u16)],
    ) -> Result<f64, RerollPolicySolverError> {
        Ok(self.values[self.echo_index(echo)?])
    }
    /// Value-aware optimal lock mask, or `None` for an already successful five-stat echo.
    pub fn best_lock_choices_for_echo(
        &self,
        echo: &[(usize, u16)],
    ) -> Result<Option<u16>, RerollPolicySolverError> {
        Ok(self.best_lock(self.echo_index(echo)?))
    }
    /// Value-aware counterpart of [`Self::lock_choices`]; `top_k = 0` returns all retained options.
    pub fn lock_choices_for_echo(
        &self,
        echo: &[(usize, u16)],
        top_k: usize,
    ) -> Result<Vec<LockChoice>, RerollPolicySolverError> {
        Ok(self.choices(self.echo_index(echo)?, top_k))
    }
    /// Value-aware one-reroll success probability for the recommended lock, or `None` if successful.
    pub fn best_lock_success_probability_for_echo(
        &self,
        echo: &[(usize, u16)],
    ) -> Result<Option<f64>, RerollPolicySolverError> {
        Ok(self.best_success(self.echo_index(echo)?))
    }
    /// Value-aware candidate comparison by optimal remaining frequency cost, accepting ties.
    /// Validates each echo, but not compatibility with a previously selected lock.
    pub fn should_accept_echo(
        &self,
        baseline: &[(usize, u16)],
        candidate: &[(usize, u16)],
    ) -> Result<bool, RerollPolicySolverError> {
        Ok(self.expected_lock_cost_for_echo(candidate)?
            <= self.expected_lock_cost_for_echo(baseline)?)
    }
}
