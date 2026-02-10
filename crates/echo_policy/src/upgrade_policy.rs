use crate::CostModel;
use crate::data::{NUM_BUFFS, NUM_ECHO_SLOTS};
use crate::mask::{
    MASK_ALL, NUM_PARTIAL_MASKS, PARTIAL_MASKS, calculate_num_filled_slots,
    is_valid_external_full_mask, is_valid_external_partial_mask, partial_mask_to_index,
};
use crate::scoring::{SCORE_MULTIPLIER, Scorer};

const DP_VALUE_MULTIPLIER: f64 = 1000.0;

fn best_case_remaining_score(mask: u16, buff_max_score: &[u16; NUM_BUFFS]) -> u16 {
    let num_filled_slots = calculate_num_filled_slots(mask);
    if num_filled_slots >= NUM_ECHO_SLOTS {
        return 0;
    }
    let num_remaining_slots = NUM_ECHO_SLOTS - num_filled_slots;
    let mut top_scores = [0u16; NUM_ECHO_SLOTS];
    for (buff_index, &score) in buff_max_score.iter().enumerate() {
        if (mask & (1u16 << buff_index)) != 0 {
            continue;
        }
        if score <= top_scores[num_remaining_slots - 1] {
            continue;
        }
        let mut j = num_remaining_slots - 1;
        while j > 0 && score > top_scores[j - 1] {
            top_scores[j] = top_scores[j - 1];
            j -= 1;
        }
        top_scores[j] = score;
    }
    top_scores[..num_remaining_slots].iter().sum()
}

struct MaskCache {
    dp: Vec<f64>,
    touched: Vec<usize>,

    min_score: u16,
    best_case_remaining_score: u16,
    cut_off_score: Option<u16>,
}

impl MaskCache {
    fn new(min_score: u16, max_score: u16, best_case_remaining_score: u16) -> Self {
        let size = (max_score - min_score + 1) as usize;

        Self {
            dp: vec![f64::NAN; size],
            touched: Vec::new(),

            min_score,
            best_case_remaining_score,
            cut_off_score: None,
        }
    }

    fn min_score(&self) -> u16 {
        self.min_score
    }

    fn get_decision(&self, score: u16) -> Option<bool> {
        self.cut_off_score.map(|s| score >= s)
    }

    /// This do not check whether `score` is valid.
    /// If the input `score` is not valid, the resulting index is out of bound!
    fn score_to_index(&self, score: u16) -> usize {
        (score - self.min_score) as usize
    }

    /// Get the dp value for a score.
    ///
    /// Output is NAN if the dp value has not been set.
    fn dp(&self, score: u16) -> f64 {
        self.dp[self.score_to_index(score)]
    }

    fn set_cache(&mut self, score: u16, dp: f64, decision: bool) {
        let index = self.score_to_index(score);
        if self.dp[index].is_nan() {
            self.touched.push(index);
        }
        self.dp[index] = dp;
        if decision {
            self.cut_off_score = Some(self.cut_off_score.map_or(score, |s| s.min(score)));
        }
    }

    fn clear_touched(&mut self) {
        for &index in self.touched.iter() {
            self.dp[index] = f64::NAN;
        }
        self.touched.clear();
        self.cut_off_score = None;
    }
}

pub struct ExpectedUpgradeCost {
    success_probability: f64,
    tuner_per_success: f64,
    exp_per_success: f64,
}

impl ExpectedUpgradeCost {
    pub fn success_probability(&self) -> f64 {
        self.success_probability
    }

    pub fn echo_per_success(&self) -> f64 {
        1.0 / self.success_probability
    }

    pub fn tuner_per_success(&self) -> f64 {
        self.tuner_per_success
    }

    pub fn exp_per_success(&self) -> f64 {
        self.exp_per_success
    }
}

#[derive(Clone, Copy)]
struct ExpectedUpgradeCostState {
    success_probability: f64,
    tuner: f64,
    exp: f64,
}

impl Default for ExpectedUpgradeCostState {
    fn default() -> Self {
        Self {
            success_probability: f64::NAN,
            tuner: 0.0,
            exp: 0.0,
        }
    }
}

impl ExpectedUpgradeCostState {
    fn failed_state() -> Self {
        Self {
            success_probability: 0.0,
            tuner: 0.0,
            exp: 0.0,
        }
    }

    fn guaranteed_success_state(cost_model: &CostModel, num_filled_slots: usize) -> Self {
        let tuner = (NUM_ECHO_SLOTS - num_filled_slots) as f64 * cost_model.tuner_cost();
        let exp = cost_model.full_upgrade_exp_cost(num_filled_slots);

        Self {
            success_probability: 1.0,
            tuner,
            exp,
        }
    }
}

enum ExpectedCostCache {
    NotComputed,
    Computed(Vec<ExpectedCostCacheEntry>),
}

enum ExpectedCostCacheEntry {
    Abandon,
    Reachable {
        cut_off_score: u16,
        states: Vec<ExpectedUpgradeCostState>,
    },
}

#[derive(Debug)]
pub enum UpgradePolicySolverError {
    ExpectedResourcesNotComputed,
    InvalidMask {
        mask: u16,
    },
    InvalidScorePmfCount {
        count: usize,
    },
    InvalidScorePmfEmpty {
        buff_index: usize,
    },
    InvalidScorePmfProbability {
        buff_index: usize,
        probability: f64,
    },
    InvalidScorePmfNotNormalized {
        buff_index: usize,
        probability_sum: f64,
    },
    ScoreRangeOverflow {
        max_score_sum: u32,
    },
    InvalidScore,
    InvalidTolerance {
        tolerance: f64,
    },
    LambdaNotBracketed,
    LambdaNotFoundWithinMaxIter,
    PolicyNotDerived,
    TargetScoreImpossible {
        max_possible_score: u16,
        target_score: u16,
    },
}

pub struct UpgradePolicySolver {
    score_pmfs: Vec<Vec<(u16, f64)>>,
    target_score: u16,
    cost_model: CostModel,
    lambda: f64,
    is_policy_derived: bool,

    pmf_len: [usize; NUM_BUFFS],
    max_possible_score: u16,
    caches: Vec<MaskCache>,
    touched_cache: Vec<usize>,
    expected_cost_cache: ExpectedCostCache,
}

impl UpgradePolicySolver {
    pub fn cost_model(&self) -> &CostModel {
        &self.cost_model
    }

    pub fn is_policy_derived(&self) -> bool {
        self.is_policy_derived
    }

    pub fn get_decision(&self, mask: u16, score: u16) -> Result<bool, UpgradePolicySolverError> {
        if !self.is_policy_derived() {
            return Err(UpgradePolicySolverError::PolicyNotDerived);
        }

        if is_valid_external_partial_mask(mask) {
            if mask == 0 {
                return Ok(true);
            }
            let cache_index = partial_mask_to_index(mask);
            return Ok(self.caches[cache_index]
                .get_decision(score)
                .unwrap_or(false));
        }

        if is_valid_external_full_mask(mask) {
            return Ok(false);
        }

        Err(UpgradePolicySolverError::InvalidMask { mask })
    }

    /// This is the probability of reaching target_score by strictly following the policy.
    pub fn get_success_probability(
        &self,
        mask: u16,
        score: u16,
    ) -> Result<f64, UpgradePolicySolverError> {
        if !is_valid_external_partial_mask(mask) && !is_valid_external_full_mask(mask) {
            return Err(UpgradePolicySolverError::InvalidMask { mask });
        }
        if score >= self.target_score {
            return Ok(1.0);
        }
        if !self.get_decision(mask, score)? {
            return Ok(0.0);
        }

        let cache = match &self.expected_cost_cache {
            ExpectedCostCache::NotComputed => {
                return Err(UpgradePolicySolverError::ExpectedResourcesNotComputed);
            }
            ExpectedCostCache::Computed(cache) => cache,
        };
        let cache_index = partial_mask_to_index(mask);
        let probability = match &cache[cache_index] {
            ExpectedCostCacheEntry::Abandon => 0.0,
            ExpectedCostCacheEntry::Reachable {
                cut_off_score,
                states,
            } => {
                if score < *cut_off_score {
                    return Ok(0.0);
                }
                let score_key = (score - *cut_off_score) as usize;
                match states.get(score_key) {
                    Some(state) => state.success_probability,
                    None => {
                        return Err(UpgradePolicySolverError::InvalidScore);
                    }
                }
            }
        };
        if probability.is_nan() {
            return Err(UpgradePolicySolverError::InvalidScore);
        }
        Ok(probability)
    }

    pub fn weighted_expected_cost(&self) -> Result<f64, UpgradePolicySolverError> {
        if !self.is_policy_derived() {
            return Err(UpgradePolicySolverError::PolicyNotDerived);
        }
        Ok(DP_VALUE_MULTIPLIER / self.lambda + self.cost_model.weighted_success_additional_cost())
    }
}

impl UpgradePolicySolver {
    pub fn new<S: Scorer>(
        scorer: &S,
        blend_data: bool,
        target_score_raw: f64,
        cost_model: CostModel,
    ) -> Result<Self, UpgradePolicySolverError> {
        if target_score_raw.is_nan() || target_score_raw.is_infinite() {
            return Err(UpgradePolicySolverError::InvalidScore);
        }

        let score_pmfs = scorer.build_score_pmfs(blend_data);
        if score_pmfs.len() != NUM_BUFFS {
            return Err(UpgradePolicySolverError::InvalidScorePmfCount {
                count: score_pmfs.len(),
            });
        }

        let mut buff_min_score = [0u16; NUM_BUFFS];
        let mut buff_max_score = [0u16; NUM_BUFFS];
        let mut pmf_len = [0usize; NUM_BUFFS];
        let mut top_max_scores = [0u16; NUM_ECHO_SLOTS];
        const PMF_SUM_TOL: f64 = 1e-9;

        for buff_index in 0..NUM_BUFFS {
            let buff_pmf = &score_pmfs[buff_index];
            if buff_pmf.is_empty() {
                return Err(UpgradePolicySolverError::InvalidScorePmfEmpty { buff_index });
            }

            pmf_len[buff_index] = buff_pmf.len();

            let mut min_score = u16::MAX;
            let mut max_score = u16::MIN;
            let mut probability_sum: f64 = 0.0;
            for &(_, probability) in buff_pmf.iter() {
                if !probability.is_finite() || probability < 0.0 {
                    return Err(UpgradePolicySolverError::InvalidScorePmfProbability {
                        buff_index,
                        probability,
                    });
                }
                probability_sum += probability;
            }
            if (probability_sum - 1.0).abs() > PMF_SUM_TOL {
                return Err(UpgradePolicySolverError::InvalidScorePmfNotNormalized {
                    buff_index,
                    probability_sum,
                });
            }

            for &(score, _) in buff_pmf.iter() {
                min_score = min_score.min(score);
                max_score = max_score.max(score);
            }
            buff_min_score[buff_index] = min_score;
            buff_max_score[buff_index] = max_score;

            if max_score > top_max_scores[NUM_ECHO_SLOTS - 1] {
                let mut j = NUM_ECHO_SLOTS - 1;
                while j > 0 && max_score > top_max_scores[j - 1] {
                    top_max_scores[j] = top_max_scores[j - 1];
                    j -= 1;
                }
                top_max_scores[j] = max_score;
            }
        }

        let max_score_sum: u32 = top_max_scores.into_iter().map(u32::from).sum();
        if max_score_sum > u16::MAX as u32 {
            return Err(UpgradePolicySolverError::ScoreRangeOverflow { max_score_sum });
        }

        let max_possible_score = best_case_remaining_score(0u16, &buff_max_score);
        let target_score = if target_score_raw <= 0.0 {
            0
        } else {
            (target_score_raw * SCORE_MULTIPLIER).round() as u16
        };
        if target_score > max_possible_score {
            return Err(UpgradePolicySolverError::TargetScoreImpossible {
                max_possible_score,
                target_score,
            });
        }

        let mut caches: Vec<MaskCache> = Vec::with_capacity(NUM_PARTIAL_MASKS);

        for &mask in PARTIAL_MASKS.iter() {
            let mut mask_min_score: u16 = 0;
            let mut mask_max_score: u16 = 0;

            for buff_index in 0..NUM_BUFFS {
                if (mask & (1u16 << buff_index)) == 0 {
                    continue;
                }
                mask_min_score += buff_min_score[buff_index];
                mask_max_score += buff_max_score[buff_index];
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
            lambda: 0.0,
            is_policy_derived: false,

            pmf_len,
            max_possible_score,
            caches,
            touched_cache: Vec::new(),
            expected_cost_cache: ExpectedCostCache::NotComputed,
        })
    }

    pub fn update_target_score(
        &mut self,
        new_target_score_raw: f64,
    ) -> Result<(), UpgradePolicySolverError> {
        if new_target_score_raw.is_nan() | new_target_score_raw.is_infinite() {
            return Err(UpgradePolicySolverError::InvalidScore);
        }
        let new_target_score = if new_target_score_raw <= 0.0 {
            0
        } else {
            (new_target_score_raw * SCORE_MULTIPLIER).round() as u16
        };
        if new_target_score > self.max_possible_score {
            return Err(UpgradePolicySolverError::TargetScoreImpossible {
                max_possible_score: self.max_possible_score,
                target_score: new_target_score,
            });
        }
        self.clear_caches();
        self.target_score = new_target_score;
        Ok(())
    }
}

impl UpgradePolicySolver {
    fn clear_caches(&mut self) {
        self.lambda = 0.0;
        self.is_policy_derived = false;
        for &index in self.touched_cache.iter() {
            self.caches[index].clear_touched();
        }
        self.touched_cache.clear();
        self.expected_cost_cache = ExpectedCostCache::NotComputed;
    }

    fn set_cache(&mut self, mask: u16, score: u16, dp: f64, decision: bool) {
        let cache_index = partial_mask_to_index(mask);
        if self.caches[cache_index].touched.is_empty() {
            self.touched_cache.push(cache_index);
        }
        self.caches[cache_index].set_cache(score, dp, decision);
    }

    pub fn derive_policy_at_lambda(&mut self, lambda: f64) {
        self.clear_caches();
        self.lambda = lambda;
        self.is_policy_derived = true;
        self.value_rec(0u16, 0u16);
    }

    pub fn lambda_search(
        &mut self,
        tol: f64,
        max_iter: usize,
    ) -> Result<f64, UpgradePolicySolverError> {
        if tol.is_nan() || tol.is_infinite() || tol <= 0.0 {
            return Err(UpgradePolicySolverError::InvalidTolerance { tolerance: tol });
        }

        let lo = 0.0;
        let mut hi = 1.0;

        let mut fa = self.root_advantage(lo);
        if fa < 0.0 {
            return Err(UpgradePolicySolverError::LambdaNotBracketed);
        }
        let mut fb = self.root_advantage(hi);
        let mut expand_count: usize = 0;
        while fb > 0.0 && expand_count < 80 {
            hi *= 2.0;
            fb = self.root_advantage(hi);
            expand_count += 1;
        }
        if fb > 0.0 {
            return Err(UpgradePolicySolverError::LambdaNotBracketed);
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
                self.root_advantage(c);
                return Ok(c);
            }
        }
        Err(UpgradePolicySolverError::LambdaNotFoundWithinMaxIter)
    }

    fn root_advantage(&mut self, lambda: f64) -> f64 {
        self.clear_caches();
        self.lambda = lambda;
        self.is_policy_derived = true;

        let mut total: f64 = 0.0;
        let mut remaining_buffs = MASK_ALL;
        while remaining_buffs != 0 {
            let lsb = remaining_buffs & remaining_buffs.wrapping_neg();
            let index = lsb.trailing_zeros() as usize;
            remaining_buffs ^= lsb;
            let next_mask = 1u16 << index;

            for j in 0..self.pmf_len[index] {
                let (delta, probability) = self.score_pmfs[index][j];
                total += probability * self.value_rec(next_mask, delta);
            }
        }

        let expected = total / NUM_BUFFS as f64;
        expected - lambda * self.cost_model.weighted_reveal_cost(0)
    }

    fn value_rec(&mut self, mask: u16, score: u16) -> f64 {
        let num_filled_slots = calculate_num_filled_slots(mask);
        if num_filled_slots >= NUM_ECHO_SLOTS {
            return if score >= self.target_score {
                1.0 * DP_VALUE_MULTIPLIER
            } else {
                0.0
            };
        }

        let cache_index = partial_mask_to_index(mask);

        // Clamp score to up to target_score (but still above min_score for the mask).
        let score = if score >= self.target_score {
            self.caches[cache_index].min_score().max(self.target_score)
        } else {
            score
        };

        let dp_cache = self.caches[cache_index].dp(score);
        if !dp_cache.is_nan() {
            return dp_cache;
        }

        if score + self.caches[cache_index].best_case_remaining_score < self.target_score {
            self.set_cache(mask, score, 0.0, false);
            return 0.0;
        }

        let num_remaining_buffs = NUM_BUFFS - num_filled_slots;
        let mut total: f64 = 0.0;
        let mut remaining_buffs = MASK_ALL ^ mask;
        while remaining_buffs != 0 {
            let lsb = remaining_buffs & remaining_buffs.wrapping_neg();
            let idx = lsb.trailing_zeros() as usize;
            remaining_buffs ^= lsb;
            let next_mask = mask | (1u16 << idx);

            for j in 0..self.pmf_len[idx] {
                let (delta, probability) = self.score_pmfs[idx][j];
                total += probability * self.value_rec(next_mask, score + delta);
            }
        }

        let expected = total / (num_remaining_buffs as f64);
        let advantage =
            expected - self.lambda * self.cost_model.weighted_reveal_cost(num_filled_slots);
        let decision = advantage >= 0.0;
        let dp = if decision { advantage } else { 0.0 };
        self.set_cache(mask, score, dp, decision);

        dp
    }

    pub fn calculate_expected_resources(
        &mut self,
    ) -> Result<ExpectedUpgradeCost, UpgradePolicySolverError> {
        if !self.is_policy_derived {
            return Err(UpgradePolicySolverError::PolicyNotDerived);
        }

        let mut memo: Vec<ExpectedCostCacheEntry> = Vec::with_capacity(NUM_PARTIAL_MASKS);

        for &mask in PARTIAL_MASKS.iter() {
            if mask == 0u16 {
                memo.push(ExpectedCostCacheEntry::Reachable {
                    cut_off_score: 0,
                    states: vec![ExpectedUpgradeCostState::default(); 1],
                });
                continue;
            }

            let cache_index = partial_mask_to_index(mask);
            // If the policy is derived and the cut_off_score is still none,
            // then the decision for this mask is always abandon.
            let cut_off_score = self.caches[cache_index].cut_off_score;
            match cut_off_score {
                None => memo.push(ExpectedCostCacheEntry::Abandon),
                Some(cut_off_s) => {
                    if cut_off_s < self.target_score {
                        let size = (self.target_score - cut_off_s + 1) as usize;
                        memo.push(ExpectedCostCacheEntry::Reachable {
                            cut_off_score: cut_off_s,
                            states: vec![ExpectedUpgradeCostState::default(); size],
                        });
                    } else {
                        // For cut_off_s >= target_score, we never index memoized states:
                        // score < cut_off_s fails immediately, and score >= target_score
                        // returns guaranteed success. Keep cut_off_score for decision logic,
                        // but use an empty state vector to avoid unused allocation.
                        memo.push(ExpectedCostCacheEntry::Reachable {
                            cut_off_score: cut_off_s,
                            states: Vec::new(),
                        });
                    }
                }
            }
        }

        let mut total = ExpectedUpgradeCostState::failed_state();
        let mut remaining_buffs = MASK_ALL;
        while remaining_buffs != 0 {
            let lsb = remaining_buffs & remaining_buffs.wrapping_neg();
            let index = lsb.trailing_zeros() as usize;
            remaining_buffs ^= lsb;
            let next_mask = 1u16 << index;

            for j in 0..self.pmf_len[index] {
                let (delta, probability) = self.score_pmfs[index][j];
                let next_state = self.expected_resources_rec(&mut memo, next_mask, delta);

                total.success_probability += probability * next_state.success_probability;
                total.tuner += probability * next_state.tuner;
                total.exp += probability * next_state.exp;
            }
        }

        let scale = 1.0 / NUM_BUFFS as f64;
        total.success_probability *= scale;
        total.tuner *= scale;
        total.exp *= scale;

        total.tuner += self.cost_model.tuner_cost();
        total.exp += self.cost_model.exp_cost(0);

        match &mut memo[0] {
            ExpectedCostCacheEntry::Reachable { states, .. } => {
                states[0] = total;
            }
            ExpectedCostCacheEntry::Abandon => unreachable!("root state must be reachable"),
        }

        self.expected_cost_cache = ExpectedCostCache::Computed(memo);

        Ok(ExpectedUpgradeCost {
            success_probability: total.success_probability,
            tuner_per_success: total.tuner / total.success_probability
                + self.cost_model.success_additional_tuner_cost(),
            exp_per_success: total.exp / total.success_probability
                + self.cost_model.success_additional_exp_cost(),
        })
    }

    fn expected_resources_rec(
        &self,
        memo: &mut [ExpectedCostCacheEntry],
        mask: u16,
        score: u16,
    ) -> ExpectedUpgradeCostState {
        let num_filled_slots = calculate_num_filled_slots(mask);
        if num_filled_slots >= NUM_ECHO_SLOTS {
            return ExpectedUpgradeCostState {
                success_probability: if score >= self.target_score { 1.0 } else { 0.0 },
                ..Default::default()
            };
        }

        let cache_index = partial_mask_to_index(mask);
        let score_key = match &memo[cache_index] {
            ExpectedCostCacheEntry::Abandon => {
                return ExpectedUpgradeCostState::failed_state();
            }
            ExpectedCostCacheEntry::Reachable {
                cut_off_score,
                states,
            } => {
                if score < *cut_off_score {
                    return ExpectedUpgradeCostState::failed_state();
                }
                if score >= self.target_score {
                    return ExpectedUpgradeCostState::guaranteed_success_state(
                        &self.cost_model,
                        num_filled_slots,
                    );
                }
                // Memo indexing path: cut_off_score <= score < target_score.
                let score_key = (score - *cut_off_score) as usize;
                let state = states[score_key];
                if !state.success_probability.is_nan() {
                    return state;
                }
                score_key
            }
        };

        let num_remaining_buffs = NUM_BUFFS - num_filled_slots;
        let mut total = ExpectedUpgradeCostState::failed_state();
        let mut remaining_buffs = MASK_ALL ^ mask;
        while remaining_buffs != 0 {
            let lsb = remaining_buffs & remaining_buffs.wrapping_neg();
            let index = lsb.trailing_zeros() as usize;
            remaining_buffs ^= lsb;
            let next_mask = mask | (1u16 << index);

            for j in 0..self.pmf_len[index] {
                let (delta, probability) = self.score_pmfs[index][j];
                let next_state = self.expected_resources_rec(memo, next_mask, score + delta);

                total.success_probability += probability * next_state.success_probability;
                total.tuner += probability * next_state.tuner;
                total.exp += probability * next_state.exp;
            }
        }

        let scale = 1.0 / num_remaining_buffs as f64;
        total.success_probability *= scale;
        total.tuner *= scale;
        total.exp *= scale;

        total.tuner += self.cost_model.tuner_cost();
        total.exp += self.cost_model.exp_cost(num_filled_slots);

        match &mut memo[cache_index] {
            ExpectedCostCacheEntry::Reachable {
                cut_off_score: _,
                states,
            } => {
                states[score_key] = total;
            }
            ExpectedCostCacheEntry::Abandon => unreachable!("state was reachable above"),
        }
        total
    }
}
