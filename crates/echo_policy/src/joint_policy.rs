use crate::value_state::Config;
use crate::{CostModel, InternalScorer, RerollPolicySolver, RerollPolicySolverError, SolverLimits};

#[derive(Debug)]
pub enum JointPolicySolverError {
    Reroll(RerollPolicySolverError),
    InvalidFrequencyWeight { weight: f64 },
    InvalidTolerance { tolerance: f64 },
    PolicyNotDerived,
    FailedToConvergeWithinMaxIter,
    NumericalFailure,
}

impl From<RerollPolicySolverError> for JointPolicySolverError {
    fn from(error: RerollPolicySolverError) -> Self {
        Self::Reroll(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointDecision {
    Upgrade,
    Abandon,
    Keep,
    Reroll { lock_mask: u16 },
}

#[derive(Clone, Copy, Debug)]
pub struct ExpectedJointCost {
    success_probability: f64,
    tuner_per_success: f64,
    exp_per_success: f64,
    frequency_per_success: f64,
}
impl ExpectedJointCost {
    /// Probability of success before abandoning a freshly started echo.
    pub fn success_probability(&self) -> f64 {
        self.success_probability
    }
    pub fn echo_per_success(&self) -> f64 {
        1.0 / self.success_probability
    }
    pub fn tuner_per_success(&self) -> f64 {
        self.tuner_per_success
    }
    /// Premium Sealed Tubes (5000 raw EXP per tube), including refund effects.
    pub fn exp_per_success(&self) -> f64 {
        self.exp_per_success
    }
    pub fn frequency_per_success(&self) -> f64 {
        self.frequency_per_success
    }
}

#[derive(Clone, Copy, Default)]
struct Outcome {
    success: f64,
    tuner: f64,
    exp: f64,
    frequency: f64,
}
impl Outcome {
    fn accumulate(&mut self, other: Self, p: f64) {
        self.success += p * other.success;
        self.tuner += p * other.tuner;
        self.exp += p * other.exp;
        self.frequency += p * other.frequency;
    }
}
struct Layer {
    configs: Vec<Config>,
    values: Vec<f64>,
    outcomes: Vec<Outcome>,
    upgrade: Vec<bool>,
}

/// Joint upgrade/reroll policy with fixed resource weights and unlimited
/// repeated attempts. A large frequency weight expresses scarcity, but does
/// not enforce a hard inventory limit. `UpgradePolicySolver` remains the
/// separate, faster solver for the mode in which rerolling is disabled.
///
/// Scores/targets use internal substat units. Query echoes contain actual
/// in-game numeric values. A retained echo is always fully upgraded/tuned.
/// All-zero-scoring types are compressed, but individual useful stat scores
/// are retained because two echoes with the same total can have different
/// optimal locks.
pub struct JointPolicySolver {
    reroll: RerollPolicySolver,
    cost_model: CostModel,
    normalized_cost: CostModel,
    cost_scale: f64,
    frequency_weight: f64,
    layers: Vec<Layer>,
    lambda: f64,
    derived: bool,
}

impl JointPolicySolver {
    pub fn new<S: InternalScorer>(
        scorer: &S,
        cost_model: CostModel,
        frequency_weight: f64,
        target_score: u16,
    ) -> Result<Self, JointPolicySolverError> {
        Self::with_limits(
            scorer,
            cost_model,
            frequency_weight,
            target_score,
            SolverLimits::default(),
        )
    }

    pub fn with_limits<S: InternalScorer>(
        scorer: &S,
        cost_model: CostModel,
        frequency_weight: f64,
        target_score: u16,
        limits: SolverLimits,
    ) -> Result<Self, JointPolicySolverError> {
        Self::validate_weight(frequency_weight)?;
        let mut reroll = RerollPolicySolver::with_scorer_and_limits(scorer, limits)?;
        reroll.set_target(target_score)?;
        Self::from_reroll(reroll, cost_model, frequency_weight)
    }

    /// Reuse a precomputed reroll table. An unsolved table with a target set is
    /// also accepted; `derive_policy` will solve it once before the joint search.
    pub fn from_reroll(
        reroll: RerollPolicySolver,
        cost_model: CostModel,
        frequency_weight: f64,
    ) -> Result<Self, JointPolicySolverError> {
        Self::validate_weight(frequency_weight)?;
        if !reroll.is_target_set() {
            return Err(RerollPolicySolverError::TargetNotSet.into());
        }
        let mut layers = Vec::new();
        let total: usize = (0..5)
            .map(|n| reroll.space.count(reroll.space.min_count(n), n))
            .sum();
        reroll.space.check_count(total)?;
        for n in 0..5 {
            let configs = reroll.space.configs(reroll.space.min_count(n), n);
            let len = configs.len();
            layers.push(Layer {
                configs,
                values: vec![0.0; len],
                outcomes: vec![Outcome::default(); len],
                upgrade: vec![false; len],
            });
        }
        let (normalized_cost, cost_scale) = cost_model.normalized();
        Ok(Self {
            reroll,
            cost_model,
            normalized_cost,
            cost_scale,
            frequency_weight,
            layers,
            lambda: 0.0,
            derived: false,
        })
    }

    fn validate_weight(weight: f64) -> Result<(), JointPolicySolverError> {
        if !weight.is_finite() || weight < 0.0 {
            Err(JointPolicySolverError::InvalidFrequencyWeight { weight })
        } else {
            Ok(())
        }
    }

    pub fn cost_model(&self) -> &CostModel {
        &self.cost_model
    }
    pub fn frequency_weight(&self) -> f64 {
        self.frequency_weight
    }
    pub fn reroll_solver(&self) -> &RerollPolicySolver {
        &self.reroll
    }
    pub fn is_policy_derived(&self) -> bool {
        self.derived
    }

    /// Changing resource weights preserves the precomputed reroll policy.
    pub fn update_cost_model(
        &mut self,
        cost_model: CostModel,
        frequency_weight: f64,
    ) -> Result<(), JointPolicySolverError> {
        Self::validate_weight(frequency_weight)?;
        self.cost_model = cost_model;
        (self.normalized_cost, self.cost_scale) = cost_model.normalized();
        self.frequency_weight = frequency_weight;
        self.derived = false;
        Ok(())
    }

    pub fn update_target_score(&mut self, target_score: u16) -> Result<(), JointPolicySolverError> {
        self.reroll.set_target(target_score)?;
        self.derived = false;
        Ok(())
    }

    /// Dinkelbach search over success probability / weighted net attempt cost.
    /// The residual is measured relative to the minimum cost of an attempt,
    /// making tolerance independent of a common scaling of resource weights.
    pub fn derive_policy(
        &mut self,
        tol: f64,
        max_iter: usize,
    ) -> Result<(), JointPolicySolverError> {
        if !tol.is_finite() || tol <= 0.0 {
            return Err(JointPolicySolverError::InvalidTolerance { tolerance: tol });
        }
        self.derived = false;
        if max_iter == 0 {
            return Err(JointPolicySolverError::FailedToConvergeWithinMaxIter);
        }
        if !self.reroll.is_policy_derived() {
            self.reroll.solve()?;
        }
        // Seed with a feasible upgrade-only policy. This avoids selecting
        // arbitrarily expensive rerolls at lambda=0 when their weight is huge.
        self.evaluate(0.0, false);
        let first = self.layers[0].outcomes[0];
        let mut lambda = first.success / self.attempt_cost(first);
        if !lambda.is_finite() || lambda <= 0.0 {
            // An extraordinarily rare direct success can underflow this seed
            // even though accumulating useful rolls makes rerolling practical.
            // Try the feasible policy that upgrades fully and then rerolls.
            self.evaluate(0.0, true);
            let first = self.layers[0].outcomes[0];
            lambda = first.success / self.attempt_cost(first);
        }
        if !lambda.is_finite() || lambda <= 0.0 {
            return Err(JointPolicySolverError::NumericalFailure);
        }
        for _ in 0..max_iter {
            self.evaluate(lambda, true);
            let total = self.layers[0].outcomes[0];
            let cost = self.attempt_cost(total);
            let next = total.success / cost;
            if !cost.is_finite() || !next.is_finite() || next <= 0.0 {
                return Err(JointPolicySolverError::NumericalFailure);
            }
            let gap = (next - lambda).abs() / next
                * (cost / self.normalized_cost.weighted_reveal_cost(0));
            if gap <= tol.max(64.0 * f64::EPSILON) {
                self.lambda = lambda;
                self.derived = true;
                return Ok(());
            }
            lambda = next;
        }
        Err(JointPolicySolverError::FailedToConvergeWithinMaxIter)
    }

    fn attempt_cost(&self, total: Outcome) -> f64 {
        let frequency = if total.frequency == 0.0 || self.frequency_weight == 0.0 {
            0.0
        } else {
            let ratio = self.frequency_weight / self.cost_scale;
            if ratio.is_finite() {
                total.frequency * ratio
            } else {
                (total.frequency * self.frequency_weight) / self.cost_scale
            }
        };
        self.normalized_cost
            .weighted_resources(1.0, total.tuner, total.exp)
            + frequency
    }

    fn terminal(&self, index: usize, lambda: f64, allow_reroll: bool) -> (f64, Outcome) {
        let frequency = self.reroll.values[index];
        if frequency == 0.0 {
            return (
                1.0,
                Outcome {
                    success: 1.0,
                    ..Outcome::default()
                },
            );
        }
        if !allow_reroll {
            return (0.0, Outcome::default());
        }
        let penalty = if self.frequency_weight == 0.0 {
            0.0
        } else {
            let ratio = self.frequency_weight / self.cost_scale;
            let unit = if ratio.is_finite() {
                lambda * ratio
            } else {
                (lambda * self.frequency_weight) / self.cost_scale
            };
            unit * frequency
        };
        // Prefer abandonment on a tie to avoid spending scarce resources for
        // no improvement. Rejected rerolls keep the baseline state, so the
        // optimal restart option is exactly min(restart_cost, weight*R(s)).
        if penalty < 1.0 {
            (
                1.0 - penalty,
                Outcome {
                    success: 1.0,
                    frequency,
                    ..Outcome::default()
                },
            )
        } else {
            (0.0, Outcome::default())
        }
    }

    fn next(&self, code: u64, slots: usize, lambda: f64, allow_reroll: bool) -> (f64, Outcome) {
        if slots == 5 {
            self.terminal(self.reroll.index(code), lambda, allow_reroll)
        } else {
            let i = self
                .reroll
                .space
                .rank(code, self.reroll.space.min_count(slots), slots);
            (self.layers[slots].values[i], self.layers[slots].outcomes[i])
        }
    }

    fn evaluate(&mut self, lambda: f64, allow_reroll: bool) {
        for n in (0..5).rev() {
            for j in 0..self.layers[n].configs.len() {
                let c = self.layers[n].configs[j];
                let mut expected = 0.0;
                let mut outcome = Outcome::default();
                let remaining = (13 - n) as f64;
                let zero_count = self.reroll.space.zeros - (n - c.count);
                if zero_count > 0 {
                    let (v, o) = self.next(c.code, n + 1, lambda, allow_reroll);
                    let p = zero_count as f64 / remaining;
                    expected += p * v;
                    outcome.accumulate(o, p);
                }
                for i in 0..self.reroll.space.types.len() {
                    if self.reroll.space.digit(c.code, i) != 0 {
                        continue;
                    }
                    for (d, &(_, probability)) in self.reroll.space.pmfs[i].iter().enumerate() {
                        let code = c.code + (d + 1) as u64 * self.reroll.space.strides[i];
                        let (v, o) = self.next(code, n + 1, lambda, allow_reroll);
                        let p = probability / remaining;
                        expected += p * v;
                        outcome.accumulate(o, p);
                    }
                }
                let advantage = expected - lambda * self.normalized_cost.weighted_reveal_cost(n);
                // The root starts an attempt; choosing to abandon before
                // paying for one would make its success/cost ratio undefined.
                let upgrade = n == 0 || advantage > 0.0;
                if upgrade {
                    outcome.tuner += self.cost_model.tuner_cost();
                    outcome.exp += self.cost_model.exp_cost(n);
                } else {
                    outcome = Outcome::default();
                }
                self.layers[n].values[j] = if upgrade { advantage } else { 0.0 };
                self.layers[n].outcomes[j] = outcome;
                self.layers[n].upgrade[j] = upgrade;
            }
        }
    }

    fn check_policy(&self) -> Result<(), JointPolicySolverError> {
        if self.derived {
            Ok(())
        } else {
            Err(JointPolicySolverError::PolicyNotDerived)
        }
    }

    pub fn get_decision(
        &self,
        echo: &[(usize, u16)],
    ) -> Result<JointDecision, JointPolicySolverError> {
        self.check_policy()?;
        let code = self.reroll.space.echo_code(echo, false)?;
        if echo.len() == 5 {
            let index = self.reroll.index(code);
            if self.reroll.values[index] == 0.0 {
                return Ok(JointDecision::Keep);
            }
            let (_, outcome) = self.terminal(index, self.lambda, true);
            if outcome.success > 0.0 {
                Ok(JointDecision::Reroll {
                    lock_mask: self
                        .reroll
                        .best_lock_choices_for_echo(echo)?
                        .expect("nonterminal reroll state has a best lock"),
                })
            } else {
                Ok(JointDecision::Abandon)
            }
        } else {
            let i =
                self.reroll
                    .space
                    .rank(code, self.reroll.space.min_count(echo.len()), echo.len());
            Ok(if self.layers[echo.len()].upgrade[i] {
                JointDecision::Upgrade
            } else {
                JointDecision::Abandon
            })
        }
    }

    /// Conditional success probability before abandoning this particular echo.
    /// Includes eventual success through rerolling when that is recommended.
    pub fn get_success_probability(
        &self,
        echo: &[(usize, u16)],
    ) -> Result<f64, JointPolicySolverError> {
        self.check_policy()?;
        let code = self.reroll.space.echo_code(echo, false)?;
        Ok(self
            .next(code, echo.len(), self.lambda, true)
            .1
            .success
            .clamp(0.0, 1.0))
    }

    pub fn calculate_expected_resources(
        &self,
    ) -> Result<ExpectedJointCost, JointPolicySolverError> {
        self.check_policy()?;
        let total = self.layers[0].outcomes[0];
        Ok(ExpectedJointCost {
            success_probability: total.success,
            tuner_per_success: total.tuner / total.success
                + self.cost_model.success_additional_tuner_cost(),
            exp_per_success: total.exp / total.success
                + self.cost_model.success_additional_exp_cost(),
            frequency_per_success: total.frequency / total.success,
        })
    }

    pub fn weighted_expected_cost(&self) -> Result<f64, JointPolicySolverError> {
        self.check_policy()?;
        let total = self.layers[0].outcomes[0];
        Ok((self.attempt_cost(total) / total.success
            + self.normalized_cost.weighted_success_additional_cost())
            * self.cost_scale)
    }
}
