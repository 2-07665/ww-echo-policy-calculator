use rayon::prelude::*;

use crate::data::{NUM_BUFFS, NUM_ECHO_SLOTS};
use crate::mask::{
    FULL_MASK_SPACE, FULL_MASKS, NUM_FULL_MASKS, calculate_num_filled_slots, full_mask_to_index,
    is_valid_external_full_mask,
};
use crate::{FixedScorer, Scorer, ScorerError};

const MAX_LOCK_SIZE: usize = NUM_ECHO_SLOTS - 1;

#[inline(always)]
fn lock_cost(k: usize) -> f64 {
    match k {
        0..=2 => 1.0,
        3 => 2.0,
        4 => 3.0,
        _ => f64::INFINITY,
    }
}

#[derive(Debug)]
pub enum RerollPolicySolverError {
    PolicyNotDerived,
    FailedtoConvergeWithinMaxIter,
    InvalidWeights,
    InvalidMask { mask: u16 },
    InvalidTolerance { tolerance: f64 },
    TargetScoreImpossible { target_score: f64, max_score: f64 },
    TargetNotSet,
}

impl From<ScorerError> for RerollPolicySolverError {
    fn from(_: ScorerError) -> Self {
        RerollPolicySolverError::InvalidWeights
    }
}

#[derive(Clone)]
pub struct LockChoice {
    pub lock_mask: u16,
    pub expected_cost: f64,
    pub regret: f64,
    pub success_probability: f64,
}

pub struct RerollPolicySolver {
    scores: [f64; NUM_FULL_MASKS],
    max_score: f64,
    lock_sets: Vec<Vec<u16>>,
    transitions: Vec<Vec<usize>>,

    target_score: Option<f64>,
    success: [bool; NUM_FULL_MASKS],
    success_count: usize,
    policy_derived: bool,
    dp: [f64; NUM_FULL_MASKS],
    action_cache: Vec<Vec<LockChoice>>,
    best_lock_cache: [Option<u16>; NUM_FULL_MASKS],
    lock_success_probability_cache: Vec<f64>,
}

impl RerollPolicySolver {
    pub fn is_target_set(&self) -> bool {
        self.target_score.is_some()
    }

    pub fn is_policy_derived(&self) -> bool {
        self.policy_derived
    }
}

impl RerollPolicySolver {
    pub fn best_lock_choices(&self, mask: u16) -> Result<Option<u16>, RerollPolicySolverError> {
        if !self.is_policy_derived() {
            return Err(RerollPolicySolverError::PolicyNotDerived);
        }

        if !is_valid_external_full_mask(mask) {
            return Err(RerollPolicySolverError::InvalidMask { mask });
        }
        let index = full_mask_to_index(mask);
        if self.success[index] {
            return Ok(None);
        }

        Ok(self.best_lock_cache[index])
    }

    pub fn lock_choices(
        &self,
        mask: u16,
        top_k: usize,
    ) -> Result<Vec<LockChoice>, RerollPolicySolverError> {
        if !self.is_policy_derived() {
            return Err(RerollPolicySolverError::PolicyNotDerived);
        }
        if !is_valid_external_full_mask(mask) {
            return Err(RerollPolicySolverError::InvalidMask { mask });
        }

        let index = full_mask_to_index(mask);
        let choices = &self.action_cache[index];
        let keep = if top_k == 0 || top_k > choices.len() {
            choices.len()
        } else {
            top_k
        };
        Ok(choices[..keep].to_vec())
    }

    pub fn expected_lock_cost(&self, mask: u16) -> Result<f64, RerollPolicySolverError> {
        if !self.is_policy_derived() {
            return Err(RerollPolicySolverError::PolicyNotDerived);
        }
        if !is_valid_external_full_mask(mask) {
            return Err(RerollPolicySolverError::InvalidMask { mask });
        }
        Ok(self.dp[full_mask_to_index(mask)])
    }

    pub fn best_lock_success_probability(
        &self,
        mask: u16,
    ) -> Result<Option<f64>, RerollPolicySolverError> {
        if !self.is_policy_derived() {
            return Err(RerollPolicySolverError::PolicyNotDerived);
        }
        if !is_valid_external_full_mask(mask) {
            return Err(RerollPolicySolverError::InvalidMask { mask });
        }

        let index = full_mask_to_index(mask);
        if self.success[index] {
            return Ok(None);
        }

        Ok(self.action_cache[index]
            .first()
            .map(|choice| choice.success_probability))
    }

    pub fn should_accept(
        &self,
        baseline_mask: u16,
        candidate_mask: u16,
    ) -> Result<bool, RerollPolicySolverError> {
        if !self.policy_derived {
            return Err(RerollPolicySolverError::PolicyNotDerived);
        }
        if !is_valid_external_full_mask(baseline_mask) {
            return Err(RerollPolicySolverError::InvalidMask {
                mask: baseline_mask,
            });
        }
        if !is_valid_external_full_mask(candidate_mask) {
            return Err(RerollPolicySolverError::InvalidMask {
                mask: candidate_mask,
            });
        }
        let baseline_dp = self.dp[full_mask_to_index(baseline_mask)];
        let candidate_dp = self.dp[full_mask_to_index(candidate_mask)];
        Ok(candidate_dp <= baseline_dp)
    }
}

impl RerollPolicySolver {
    pub fn new(weights: [f64; NUM_BUFFS]) -> Result<Self, RerollPolicySolverError> {
        let scorer = FixedScorer::new(weights)?;
        let mut scores = [0.0f64; NUM_FULL_MASKS];
        let mut max_score: f64 = 0.0;

        let mut lock_sets = Vec::with_capacity(NUM_FULL_MASKS);
        let mut transitions = vec![Vec::new(); FULL_MASK_SPACE + 1];
        let mut positive_weight_mask: u16 = 0;
        for (buff_index, &weight) in weights.iter().enumerate() {
            if weight > 0.0 {
                positive_weight_mask |= 1u16 << buff_index;
            }
        }

        for (index, &mask) in FULL_MASKS.iter().enumerate() {
            let mut sum: f64 = 0.0;
            for buff_index in 0..NUM_BUFFS {
                if (mask & (1u16 << buff_index)) != 0 {
                    sum += scorer.buff_score(buff_index, 0.0);
                }
            }
            scores[index] = sum;
            if sum > max_score {
                max_score = sum;
            }

            let mut subsets = Vec::<u16>::with_capacity(1 << NUM_ECHO_SLOTS);
            let mut sub = mask;
            loop {
                if calculate_num_filled_slots(sub) <= MAX_LOCK_SIZE
                    && (sub & !positive_weight_mask) == 0
                {
                    subsets.push(sub);
                    transitions[sub as usize].push(index);
                }
                if sub == 0 {
                    break;
                }
                sub = (sub - 1) & mask;
            }
            lock_sets.push(subsets);
        }

        Ok(Self {
            scores,
            max_score,
            lock_sets,
            transitions,

            target_score: None,
            success: [false; NUM_FULL_MASKS],
            success_count: 0,
            policy_derived: false,
            dp: [0.0; NUM_FULL_MASKS],
            action_cache: vec![Vec::new(); NUM_FULL_MASKS],
            best_lock_cache: [None; NUM_FULL_MASKS],
            lock_success_probability_cache: vec![0.0; FULL_MASK_SPACE + 1],
        })
    }

    pub fn set_target(&mut self, target_score: f64) -> Result<(), RerollPolicySolverError> {
        if target_score.is_nan() || target_score > self.max_score {
            return Err(RerollPolicySolverError::TargetScoreImpossible {
                target_score,
                max_score: self.max_score,
            });
        }
        self.target_score = Some(target_score);
        self.policy_derived = false;
        self.best_lock_cache = [None; NUM_FULL_MASKS];
        for choices in self.action_cache.iter_mut() {
            choices.clear();
        }
        self.lock_success_probability_cache.fill(0.0);

        self.success = [false; NUM_FULL_MASKS];
        let mut success_count: usize = 0;
        for (index, &score) in self.scores.iter().enumerate() {
            if score >= target_score {
                self.success[index] = true;
                success_count += 1;
            }
        }
        self.success_count = success_count;
        Ok(())
    }
}

impl RerollPolicySolver {
    #[inline(always)]
    fn action_value(&self, baseline_dp: f64, lock_mask: u16) -> f64 {
        let k = calculate_num_filled_slots(lock_mask);
        let candidates = &self.transitions[lock_mask as usize];
        let mut total: f64 = 0.0;
        for &candidate_index in candidates.iter() {
            let candidate_dp = self.dp[candidate_index];
            total += if baseline_dp < candidate_dp {
                baseline_dp
            } else {
                candidate_dp
            };
        }
        let expected = total / candidates.len() as f64;
        lock_cost(k) + expected
    }

    fn build_lock_success_probability_cache(&mut self) {
        self.lock_success_probability_cache = (0..=FULL_MASK_SPACE)
            .into_par_iter()
            .map(|lock_mask| {
                let candidates = &self.transitions[lock_mask];
                if candidates.is_empty() {
                    return 0.0;
                }
                let success_count = candidates
                    .iter()
                    .filter(|&&candidate_index| self.success[candidate_index])
                    .count();
                success_count as f64 / candidates.len() as f64
            })
            .collect();
    }

    fn build_action_cache(&mut self) {
        self.build_lock_success_probability_cache();
        let action_cache: Vec<Vec<LockChoice>> = (0..NUM_FULL_MASKS)
            .into_par_iter()
            .map(|index| {
                if self.success[index] {
                    return Vec::new();
                }
                let baseline_dp = self.dp[index];
                let mut choices = Vec::with_capacity(self.lock_sets[index].len());
                for &lock_mask in self.lock_sets[index].iter() {
                    choices.push(LockChoice {
                        lock_mask,
                        expected_cost: self.action_value(baseline_dp, lock_mask),
                        regret: 0.0,
                        success_probability: self.lock_success_probability_cache
                            [lock_mask as usize],
                    });
                }
                choices.sort_by(|lhs, rhs| lhs.expected_cost.total_cmp(&rhs.expected_cost));
                let best = choices[0].expected_cost;
                for choice in choices.iter_mut() {
                    choice.regret = choice.expected_cost - best;
                }
                choices
            })
            .collect();

        let mut best_lock_cache = [None; NUM_FULL_MASKS];
        for (index, choices) in action_cache.iter().enumerate() {
            best_lock_cache[index] = choices.first().map(|choice| choice.lock_mask);
        }

        self.action_cache = action_cache;
        self.best_lock_cache = best_lock_cache;
    }

    pub fn derive_policy(
        &mut self,
        tol: f64,
        max_iter: usize,
    ) -> Result<(), RerollPolicySolverError> {
        if !self.is_target_set() {
            return Err(RerollPolicySolverError::TargetNotSet);
        }
        if tol.is_nan() || tol.is_infinite() || tol <= 0.0 {
            return Err(RerollPolicySolverError::InvalidTolerance { tolerance: tol });
        }
        self.policy_derived = false;
        self.best_lock_cache = [None; NUM_FULL_MASKS];
        for choices in self.action_cache.iter_mut() {
            choices.clear();
        }
        self.lock_success_probability_cache.fill(0.0);

        let p_success_all: f64 = self.success_count as f64 / NUM_FULL_MASKS as f64;
        let init_value = lock_cost(0) / p_success_all;

        for (index, dp) in self.dp.iter_mut().enumerate() {
            *dp = if self.success[index] { 0.0 } else { init_value };
        }

        let mut next = self.dp;

        for _ in 0..max_iter {
            let max_delta = next
                .par_iter_mut()
                .enumerate()
                .map(|(index, value)| {
                    if self.success[index] {
                        return 0.0;
                    }

                    let baseline_dp = self.dp[index];
                    let mut best = f64::INFINITY;
                    for &lock_mask in self.lock_sets[index].iter() {
                        let dp = self.action_value(baseline_dp, lock_mask);
                        if dp < best {
                            best = dp;
                        }
                    }
                    *value = best;
                    (best - self.dp[index]).abs()
                })
                .reduce(|| 0.0, f64::max);
            self.dp = next;
            if max_delta <= tol {
                self.build_action_cache();
                self.policy_derived = true;
                return Ok(());
            }
        }

        Err(RerollPolicySolverError::FailedtoConvergeWithinMaxIter)
    }
}
