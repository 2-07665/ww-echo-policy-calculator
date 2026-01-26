mod solver_cost;
mod solver_mask;

use crate::CostModel;
use crate::SCORE_MULTIPLIER;
use crate::data::{MAX_SELECTED_TYPES, TOTAL_BUFF_TYPES};

use solver_cost::ExpectedCostState;
pub use solver_cost::ExpectedResourceCost;
use solver_mask::{
    MASK_ALL, MASKS, MaskCache, NUM_OF_MASKS, best_case_remaining_score, mask_to_cache_index,
    used_slots,
};

const CONTINUE_VALUE_MULTIPLIER: f64 = 1000.0;

#[derive(Debug)]
pub enum SolverError {
    TargetScoreImpossible,
    NonFiniteLambda,
    LambdaNotBracketed,
    PolicyNotDerived,
    ExpectedResourcesNotComputed,
    LambdaNotFoundWithinMaxIter,
    InvalidScore,
}

pub struct PolicySolver {
    score_pmfs: Vec<Vec<(u16, f64)>>,
    target_score: u16,
    cost_model: CostModel,
    lambda: Option<f64>,

    pmf_len: [usize; TOTAL_BUFF_TYPES],
    max_possible_score: u16,
    caches: Vec<MaskCache>,
    touched_cache: Vec<usize>,
    expected_cost_cache: Option<Vec<Option<Vec<ExpectedCostState>>>>,
}

impl PolicySolver {
    pub fn new(
        score_pmfs: Vec<Vec<(u16, f64)>>,
        target_score_raw: f64,
        cost_model: CostModel,
    ) -> Result<Self, SolverError> {
        let mut buff_min_score = [0u16; TOTAL_BUFF_TYPES];
        let mut buff_max_score = [0u16; TOTAL_BUFF_TYPES];

        for (buff_idx, buff_pmfs) in score_pmfs.iter().enumerate() {
            let (min_s, max_s) = buff_pmfs
                .iter()
                .map(|(s, _)| *s)
                .fold((u16::MAX, u16::MIN), |(mn, mx), s| (mn.min(s), mx.max(s)));
            buff_min_score[buff_idx] = min_s;
            buff_max_score[buff_idx] = max_s;
        }

        let pmf_len = std::array::from_fn(|i| score_pmfs[i].len());

        let max_possible_score = best_case_remaining_score(0u16, &buff_max_score);
        let target_score = (target_score_raw * SCORE_MULTIPLIER).round() as u16;
        if target_score > max_possible_score {
            return Err(SolverError::TargetScoreImpossible);
        }

        let mut caches: Vec<MaskCache> = Vec::with_capacity(NUM_OF_MASKS);

        for &mask in MASKS.iter() {
            let mut mask_min_score: u16 = 0;
            let mut mask_max_score: u16 = 0;

            for buff_idx in 0..TOTAL_BUFF_TYPES {
                if (mask & (1u16 << buff_idx)) == 0 {
                    continue;
                }
                mask_min_score += buff_min_score[buff_idx];
                mask_max_score += buff_max_score[buff_idx];
            }

            let best_case_remaining_score = best_case_remaining_score(mask, &buff_max_score);

            caches.push(MaskCache::new(
                mask_min_score,
                mask_max_score,
                best_case_remaining_score,
            ));
        }

        Ok(Self {
            score_pmfs,
            target_score,
            cost_model,
            lambda: None,

            pmf_len,
            max_possible_score,
            caches,
            touched_cache: Vec::with_capacity(NUM_OF_MASKS),
            expected_cost_cache: None,
        })
    }

    pub fn cost_model(&self) -> &CostModel {
        &self.cost_model
    }

    pub fn update_target_score(&mut self, target_score_raw: f64) -> Result<(), SolverError> {
        let target_score = (target_score_raw * SCORE_MULTIPLIER).round() as u16;
        if target_score > self.max_possible_score {
            return Err(SolverError::TargetScoreImpossible);
        }
        self.lambda = None;
        self.clear_caches();
        self.target_score = target_score;
        Ok(())
    }

    pub fn is_policy_derived(&self) -> bool {
        return !self.lambda.is_none();
    }

    pub fn get_decision(&self, mask: u16, score: u16) -> Result<bool, SolverError> {
        if !self.is_policy_derived() {
            return Err(SolverError::PolicyNotDerived);
        };

        if mask == 0u16 {
            return Ok(true);
        };

        if used_slots(mask) >= MAX_SELECTED_TYPES {
            return Ok(false);
        }

        let cache_idx = mask_to_cache_index(mask);
        Ok(self.caches[cache_idx].get_decision(score).unwrap_or(false))
    }

    pub fn get_success_prob(&self, mask: u16, score: u16) -> Result<f64, SolverError> {
        if !self.get_decision(mask, score)? {
            return Ok(0.0);
        }

        if score >= self.target_score {
            return Ok(1.0);
        }

        let cache = self
            .expected_cost_cache
            .as_ref()
            .ok_or(SolverError::ExpectedResourcesNotComputed)?;
        let score_key = score.min(self.target_score) as usize;
        let cache_idx = mask_to_cache_index(mask);
        let prob = cache[cache_idx].as_ref().unwrap()[score_key].success_prob;
        if prob.is_nan() {
            return Err(SolverError::InvalidScore);
        }
        Ok(prob)
    }

    pub fn weighted_expected_cost(&self) -> Result<f64, SolverError> {
        self.lambda
            .map(|lam| {
                CONTINUE_VALUE_MULTIPLIER / lam + self.cost_model.weighted_success_additional_cost()
            })
            .ok_or(SolverError::PolicyNotDerived)
    }

    pub fn derive_policy_at_lambda(&mut self, lambda: f64) {
        self.lambda = Some(lambda);
        self.clear_caches();
        self.value_rec(0u16, 0u16);
    }

    pub fn lambda_search(
        &mut self,
        mut lo: f64,
        mut hi: f64,
        tol: f64,
        max_iter: usize,
    ) -> Result<f64, SolverError> {
        if !(lo.is_finite() && hi.is_finite() && tol.is_finite()) {
            return Err(SolverError::NonFiniteLambda);
        }
        if lo < 0.0 {
            lo = 0.0;
        }
        if hi <= lo {
            hi = lo + 1.0;
        }

        let mut fa = self.root_advantage(lo);
        if !(fa > 0.0) {
            return Err(SolverError::LambdaNotBracketed);
        }
        let mut fb = self.root_advantage(hi);
        let mut expand = 0usize;
        while fb > 0.0 && expand < 80 {
            hi *= 2.0;
            fb = self.root_advantage(hi);
            expand += 1;
        }
        if !(fb < 0.0) {
            return Err(SolverError::LambdaNotBracketed);
        }

        let mut a = lo;
        let mut b = hi;
        let mut scale_a = 1.0f64;
        let mut scale_b = 1.0f64;

        for _ in 0..max_iter {
            let fa_s = fa * scale_a;
            let fb_s = fb * scale_b;
            let denom = fb_s - fa_s;

            let c = if denom.abs() <= f64::EPSILON * (fa_s.abs() + fb_s.abs() + 1.0) {
                0.5 * (a + b)
            } else {
                (a * fb_s - b * fa_s) / denom
            };

            let fc = self.root_advantage(c);
            if fc.abs() <= tol {
                return Ok(c);
            }

            if fc > 0.0 {
                a = c;
                fa = fc;
                scale_a = 1.0;
                scale_b *= 0.5;
            } else {
                b = c;
                fb = fc;
                scale_b = 1.0;
                scale_a *= 0.5;
            }

            if (b - a).abs() <= tol * (1.0 + c.abs()) {
                let c = 0.5 * (a + b);
                self.derive_policy_at_lambda(c);
                return Ok(c);
            }
        }
        Err(SolverError::LambdaNotFoundWithinMaxIter)
    }

    fn clear_caches(&mut self) {
        for &idx in self.touched_cache.iter() {
            self.caches[idx].clear_touched();
        }
        self.touched_cache.clear();
        self.expected_cost_cache = None;
    }

    fn cache_set(&mut self, mask: u16, score: u16, continue_value: f64, decision: bool) {
        let cache_idx = mask_to_cache_index(mask);
        if self.caches[cache_idx].touched.is_empty() {
            self.touched_cache.push(cache_idx);
        }
        self.caches[cache_idx].set(score, continue_value, decision);
    }

    fn root_advantage(&mut self, lambda: f64) -> f64 {
        self.lambda = Some(lambda);
        self.clear_caches();

        let mut total = 0.0f64;
        let mut remaining = MASK_ALL;
        while remaining != 0 {
            let lsb = remaining & remaining.wrapping_neg();
            let idx = lsb.trailing_zeros() as usize;
            remaining ^= lsb;
            let next_mask = 1u16 << idx;

            for j in 0..self.pmf_len[idx] {
                let (delta, prob) = self.score_pmfs[idx][j];
                total += prob * self.value_rec(next_mask, delta);
            }
        }

        let expected = total / TOTAL_BUFF_TYPES as f64;
        expected - lambda * self.cost_model.weighted_reveal_cost(0)
    }

    fn value_rec(&mut self, mask: u16, score: u16) -> f64 {
        let used_slots = used_slots(mask);
        if used_slots >= MAX_SELECTED_TYPES {
            return if score >= self.target_score {
                1.0 * CONTINUE_VALUE_MULTIPLIER
            } else {
                0.0
            };
        }

        let cache_idx = mask_to_cache_index(mask);
        let score = if score >= self.target_score {
            let min_score = self.caches[cache_idx].min_score();
            if self.target_score < min_score {
                min_score
            } else {
                self.target_score
            }
        } else {
            score
        };

        {
            let cache_value = self.caches[cache_idx].get_value(score);
            if !cache_value.is_nan() {
                return cache_value;
            }
        }

        if score + self.caches[cache_idx].best_case_remaining_score < self.target_score {
            self.cache_set(mask, score, 0.0, false);
            return 0.0;
        }

        let remaining_types = TOTAL_BUFF_TYPES - used_slots;
        let mut total: f64 = 0.0;
        let mut remaining = MASK_ALL ^ mask;
        while remaining != 0 {
            let lsb = remaining & remaining.wrapping_neg();
            let idx = lsb.trailing_zeros() as usize;
            remaining ^= lsb;
            let next_mask = mask | (1u16 << idx);

            for j in 0..self.pmf_len[idx] {
                let (delta, prob) = self.score_pmfs[idx][j];
                total += prob * self.value_rec(next_mask, score + delta);
            }
        }

        let expected = total / (remaining_types as f64);
        let advantage = expected
            - self.lambda.expect("lambda should not be empty")
                * self.cost_model.weighted_reveal_cost(used_slots);
        let decision = advantage >= 0.0;
        let continue_value = if decision { advantage } else { 0.0 };

        self.cache_set(mask, score, continue_value, decision);

        continue_value
    }

    pub fn expected_resources(&mut self) -> Result<ExpectedResourceCost, SolverError> {
        if !self.is_policy_derived() {
            return Err(SolverError::PolicyNotDerived);
        }

        let mut memo: Vec<Option<Vec<ExpectedCostState>>> = Vec::with_capacity(NUM_OF_MASKS);

        for &mask in MASKS.iter() {
            if mask == 0u16 {
                memo.push(Some(vec![ExpectedCostState::default(); 1]));
                continue;
            }

            let cache_idx = mask_to_cache_index(mask);
            // If the policy is derived and the cut_off_score is still none, then the decision for this mask is always abandon.
            let cut_off_score = self.caches[cache_idx].cut_off_score();
            match cut_off_score {
                None => memo.push(None),
                Some(cut_off_s) => {
                    if cut_off_s < self.target_score {
                        let size = (self.target_score - cut_off_s + 1) as usize;
                        memo.push(Some(vec![ExpectedCostState::default(); size]));
                    } else {
                        memo.push(None);
                    }
                }
            }
        }

        let mut total = ExpectedCostState::failed_state();
        let mut remaining = MASK_ALL;
        while remaining != 0 {
            let lsb = remaining & remaining.wrapping_neg();
            let idx = lsb.trailing_zeros() as usize;
            remaining ^= lsb;
            let next_mask = 1u16 << idx;

            for &(delta, prob) in self.score_pmfs[idx].iter() {
                let next_state = self.expected_resources_rec(&mut memo, next_mask, delta);

                total.success_prob += prob * next_state.success_prob;
                total.tuner += prob * next_state.tuner;
                total.exp += prob * next_state.exp;
            }
        }

        let scale = 1.0 / TOTAL_BUFF_TYPES as f64;
        total.success_prob *= scale;
        total.tuner *= scale;
        total.exp *= scale;

        total.tuner += self.cost_model.tuner_cost();
        total.exp += self.cost_model.exp_cost(0);

        memo[0].as_deref_mut().unwrap()[0] = total;

        if total.success_prob <= 0.0 {
            return Err(SolverError::TargetScoreImpossible);
        }
        self.expected_cost_cache = Some(memo);

        Ok(ExpectedResourceCost {
            success_prob: total.success_prob,
            tuner_per_succ: total.tuner / total.success_prob
                + self.cost_model.success_additional_tuner_cost(),
            exp_per_succ: total.exp / total.success_prob
                + self.cost_model.success_additional_exp_cost(),
        })
    }

    fn expected_resources_rec(
        &self,
        memo: &mut [Option<Vec<ExpectedCostState>>],
        mask: u16,
        score: u16,
    ) -> ExpectedCostState {
        let used_slots = used_slots(mask);
        if used_slots >= MAX_SELECTED_TYPES {
            return ExpectedCostState {
                success_prob: if score >= self.target_score { 1.0 } else { 0.0 },
                ..Default::default()
            };
        }

        let cache_idx = mask_to_cache_index(mask);
        let cut_off_score = self.caches[cache_idx].cut_off_score();
        let score_key: usize;
        match cut_off_score {
            None => {
                return ExpectedCostState::failed_state();
            }
            Some(cut_off_s) => {
                if score < cut_off_s {
                    return ExpectedCostState::failed_state();
                }
                if score >= self.target_score {
                    return ExpectedCostState::always_success_state(&self.cost_model, used_slots);
                }
                // target_score > score >= cut_off_s
                score_key = (score - cut_off_s) as usize;
                let state = memo[cache_idx].as_deref().unwrap()[score_key];
                if !state.success_prob.is_nan() {
                    return state;
                }
            }
        }

        let remaining_types = TOTAL_BUFF_TYPES - used_slots;
        let mut total = ExpectedCostState::failed_state();
        let mut remaining = MASK_ALL ^ mask;
        while remaining != 0 {
            let lsb = remaining & remaining.wrapping_neg();
            let idx = lsb.trailing_zeros() as usize;
            remaining ^= lsb;
            let next_mask = mask | (1u16 << idx);

            for &(delta, prob) in self.score_pmfs[idx].iter() {
                let next_state = self.expected_resources_rec(memo, next_mask, score + delta);

                total.success_prob += prob * next_state.success_prob;
                total.tuner += prob * next_state.tuner;
                total.exp += prob * next_state.exp;
            }
        }

        let scale = 1.0 / remaining_types as f64;
        total.success_prob *= scale;
        total.tuner *= scale;
        total.exp *= scale;

        total.tuner += self.cost_model.tuner_cost();
        total.exp += self.cost_model.exp_cost(used_slots);

        memo[cache_idx].as_deref_mut().unwrap()[score_key] = total;
        total
    }
}
