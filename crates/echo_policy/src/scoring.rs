use std::collections::BTreeMap;

use crate::data::{BUFF_FIXED_VALUE_INDEX, BUFF_MAX_VALUES, BUFF_TYPES, NUM_BUFFS, NUM_ECHO_SLOTS};

const BLEND_GROUP_CRIT: [usize; 2] = [0, 1];
const BLEND_GROUP_MAIN: [usize; 9] = [2, 3, 4, 7, 8, 9, 10, 11, 12];

pub const SCORE_MULTIPLIER: f64 = 100.0;
const MAX_DISPLAY_SCORE: f64 = u16::MAX as f64 / SCORE_MULTIPLIER;

pub fn convert_display_to_internal(score_display: f64) -> u16 {
    (score_display * SCORE_MULTIPLIER).round() as u16
}

fn is_valid_buff(buff_index: usize, buff_value: u16) -> Result<(), ScorerError> {
    if !(0..NUM_BUFFS).contains(&buff_index) {
        return Err(ScorerError::InvalidBuffIndex {
            buff_index,
            buff_value,
        });
    } else if buff_value > BUFF_MAX_VALUES[buff_index] {
        return Err(ScorerError::InvalidBuffValue {
            buff_index,
            buff_value,
        });
    }
    Ok(())
}

#[derive(Debug)]
pub enum ScorerError {
    NegativeWeight { index: usize, weight: f64 },
    AllWeightsZero,
    InvalidBuffIndex { buff_index: usize, buff_value: u16 },
    InvalidBuffValue { buff_index: usize, buff_value: u16 },
    InvalidMainBuffScore { main_buff_score: f64 },
    InvalidNormalizedMaxScore { normalized_max_score: f64 },
    InvalidUnnormalizedMaxScore { unnormalized_max_score: f64 },
    InvalidEcho,
    FixedScorerTopWeightsTooLarge { sum: u32 },
}

pub trait InternalScorer {
    fn buff_score_internal(&self, buff_index: usize, buff_value: u16) -> Result<u16, ScorerError>;

    fn echo_score_internal(&self, echo: &[(usize, u16)]) -> Result<u16, ScorerError> {
        if echo.len() > NUM_ECHO_SLOTS {
            return Err(ScorerError::InvalidEcho);
        }

        let mut seen_mask: u16 = 0;
        let mut sum: u16 = 0;
        for &(buff_index, buff_value) in echo.iter() {
            if buff_index < NUM_BUFFS {
                let bit = 1u16 << buff_index;
                if (seen_mask & bit) != 0 {
                    return Err(ScorerError::InvalidEcho);
                }
                seen_mask |= bit;
            }
            sum += self.buff_score_internal(buff_index, buff_value)?;
        }
        Ok(sum)
    }

    fn build_score_pmfs(&self, blend_data: bool) -> Vec<Vec<(u16, f64)>> {
        build_score_pmfs(self, blend_data)
    }
}

fn validate_weights(weights: &[f64; NUM_BUFFS]) -> Result<(), ScorerError> {
    let mut any_positive = false;
    for (index, &weight) in weights.iter().enumerate() {
        if !weight.is_finite() || weight < 0.0 {
            return Err(ScorerError::NegativeWeight { index, weight });
        }
        if weight > 0.0 {
            any_positive = true;
        }
    }
    if !any_positive {
        return Err(ScorerError::AllWeightsZero);
    }
    Ok(())
}

fn validate_fixed_scorer_weights(weights: &[u16; NUM_BUFFS]) -> Result<u16, ScorerError> {
    let mut any_positive = false;
    for &weight in weights.iter() {
        if weight > 0 {
            any_positive = true;
        }
    }
    if !any_positive {
        return Err(ScorerError::AllWeightsZero);
    }

    let sum = fixed_scorer_top_weights_sum(weights);
    if sum > u16::MAX as u32 {
        return Err(ScorerError::FixedScorerTopWeightsTooLarge { sum });
    }
    Ok(sum as u16)
}

fn fixed_scorer_top_weights_sum(weights: &[u16; NUM_BUFFS]) -> u32 {
    let mut top_weights: [u16; NUM_ECHO_SLOTS] = [0; NUM_ECHO_SLOTS];
    for &weight in weights.iter() {
        if weight <= top_weights[NUM_ECHO_SLOTS - 1] {
            continue;
        }
        let mut j: usize = NUM_ECHO_SLOTS - 1;
        while j > 0 && weight > top_weights[j - 1] {
            top_weights[j] = top_weights[j - 1];
            j -= 1;
        }
        top_weights[j] = weight;
    }
    top_weights.into_iter().map(|w| w as u32).sum()
}

/// Calculate the sum of the highest weights.
fn top_weights_sum(weights: &[f64; NUM_BUFFS]) -> f64 {
    let mut top_weights: [f64; NUM_ECHO_SLOTS] = [0.0; NUM_ECHO_SLOTS];
    for &weight in weights.iter() {
        if weight <= top_weights[NUM_ECHO_SLOTS - 1] {
            continue;
        }
        let mut j: usize = NUM_ECHO_SLOTS - 1;
        while j > 0 && weight > top_weights[j - 1] {
            top_weights[j] = top_weights[j - 1];
            j -= 1;
        }
        top_weights[j] = weight;
    }
    top_weights.into_iter().sum()
}

pub struct FixedScorer {
    weights: [u16; NUM_BUFFS],
    max_score: u16,
}

impl FixedScorer {
    // NOTE: reroll_policy's `From<ScorerError>` assumes `FixedScorer::new`
    // only returns `AllWeightsZero` and `FixedScorerTopWeightsTooLarge`.
    // If new error paths are added here, update that mapping accordingly.
    pub fn new(weights: [u16; NUM_BUFFS]) -> Result<Self, ScorerError> {
        let max_score = validate_fixed_scorer_weights(&weights)?;
        Ok(Self { weights, max_score })
    }

    pub fn build_from_buff_selection() -> Result<Self, ScorerError> {
        todo!()
    }
}

impl FixedScorer {
    pub fn max_score(&self) -> u16 {
        self.max_score
    }
}

impl FixedScorer {
    pub fn buff_score_display(
        &self,
        buff_index: usize,
        buff_value: u16,
    ) -> Result<u16, ScorerError> {
        self.buff_score_internal(buff_index, buff_value)
    }

    pub fn echo_score_display(&self, echo: &[(usize, u16)]) -> Result<u16, ScorerError> {
        self.echo_score_internal(echo)
    }
}

impl InternalScorer for FixedScorer {
    fn buff_score_internal(&self, buff_index: usize, buff_value: u16) -> Result<u16, ScorerError> {
        is_valid_buff(buff_index, buff_value)?;
        Ok(self.weights[buff_index])
    }
}

pub struct LinearScorer {
    weights: [f64; NUM_BUFFS],
    unnormalized_max_score: f64,
    normalized_main_buff_score: f64,
    normalized_max_score: f64,
}

impl LinearScorer {
    pub fn new(
        weights: [f64; NUM_BUFFS],
        main_buff_score: f64,
        normalized_max_score: f64,
    ) -> Result<Self, ScorerError> {
        validate_weights(&weights)?;
        if main_buff_score.is_infinite() || main_buff_score.is_nan() || main_buff_score < 0.0 {
            return Err(ScorerError::InvalidMainBuffScore { main_buff_score });
        }
        if normalized_max_score.is_infinite()
            || normalized_max_score.is_nan()
            || normalized_max_score <= 0.0
            || normalized_max_score > MAX_DISPLAY_SCORE
        {
            return Err(ScorerError::InvalidNormalizedMaxScore {
                normalized_max_score,
            });
        }

        let unnormalized_max_score = top_weights_sum(&weights) + main_buff_score;
        if !unnormalized_max_score.is_finite() || unnormalized_max_score <= 0.0 {
            return Err(ScorerError::InvalidUnnormalizedMaxScore {
                unnormalized_max_score,
            });
        }
        let normalized_main_buff_score =
            main_buff_score / unnormalized_max_score * normalized_max_score;

        Ok(Self {
            weights,
            unnormalized_max_score,
            normalized_main_buff_score,
            normalized_max_score,
        })
    }

    pub fn default(weights: [f64; NUM_BUFFS]) -> Result<Self, ScorerError> {
        Self::new(weights, 0.0, 100.0)
    }

    pub fn qq_bot_scorer(
        qq_bot_weights: [f64; NUM_BUFFS],
        main_buff_score: f64,
    ) -> Result<Self, ScorerError> {
        let mut weights = [0.0f64; NUM_BUFFS];
        for (index, weight) in weights.iter_mut().enumerate() {
            if BUFF_FIXED_VALUE_INDEX.contains(&index) {
                *weight = qq_bot_weights[index] * BUFF_MAX_VALUES[index] as f64;
            } else {
                *weight = 0.1 * qq_bot_weights[index] * BUFF_MAX_VALUES[index] as f64;
            }
        }
        Self::new(weights, main_buff_score, 50.0)
    }

    pub fn mc_boost_assistant_scorer(weights: [f64; NUM_BUFFS]) -> Result<Self, ScorerError> {
        validate_weights(&weights)?;
        let unnormalized_max_score = 12.0 / 7.0 * top_weights_sum(&weights);
        Ok(Self {
            weights,
            unnormalized_max_score,
            normalized_main_buff_score: 50.0,
            normalized_max_score: 120.0,
        })
    }
}

impl LinearScorer {
    pub fn main_buff_score(&self) -> f64 {
        self.normalized_main_buff_score
    }

    pub fn normalized_max_score(&self) -> f64 {
        self.normalized_max_score
    }
}

impl LinearScorer {
    pub fn buff_score_display(
        &self,
        buff_index: usize,
        buff_value: u16,
    ) -> Result<f64, ScorerError> {
        is_valid_buff(buff_index, buff_value)?;
        let weight = self.weights[buff_index];
        let ratio: f64 = buff_value as f64 / BUFF_MAX_VALUES[buff_index] as f64;
        Ok(self.normalized_max_score * weight * ratio / self.unnormalized_max_score)
    }

    pub fn echo_score_display(&self, echo: &[(usize, u16)]) -> Result<f64, ScorerError> {
        let mut sum: f64 = self.normalized_main_buff_score;
        for &(buff_index, buff_value) in echo.iter() {
            sum += self.buff_score_display(buff_index, buff_value)?;
        }
        Ok(sum)
    }
}

impl InternalScorer for LinearScorer {
    fn buff_score_internal(&self, buff_index: usize, buff_value: u16) -> Result<u16, ScorerError> {
        let score_display = self.buff_score_display(buff_index, buff_value)?;
        Ok(convert_display_to_internal(score_display))
    }
}

pub fn build_score_pmfs<S: InternalScorer + ?Sized>(
    scorer: &S,
    blend_data: bool,
) -> Vec<Vec<(u16, f64)>> {
    if blend_data {
        let blended_storage = build_blended_histograms();
        let histograms: Vec<&[(u16, u32)]> = blended_storage
            .iter()
            .map(|histogram| histogram.as_slice())
            .collect();
        build_score_pmfs_from_histograms(scorer, &histograms)
    } else {
        let histograms: Vec<&[(u16, u32)]> = BUFF_TYPES.iter().map(|buff| buff.histogram).collect();
        build_score_pmfs_from_histograms(scorer, &histograms)
    }
}

fn build_score_pmfs_from_histograms<S: InternalScorer + ?Sized>(
    scorer: &S,
    histograms: &[&[(u16, u32)]],
) -> Vec<Vec<(u16, f64)>> {
    let mut score_pmfs: Vec<Vec<(u16, f64)>> = Vec::with_capacity(NUM_BUFFS);
    for (buff_index, histogram) in histograms.iter().enumerate() {
        let total_counts: f64 = histogram.iter().map(|&(_, c)| c as f64).sum();
        let mut map: BTreeMap<u16, f64> = BTreeMap::new();
        for &(buff_value, count) in histogram.iter() {
            let bucket_int = scorer
                .buff_score_internal(buff_index, buff_value)
                .expect("built-in buff histogram should be scored correctly");
            *map.entry(bucket_int).or_insert(0.0) += count as f64 / total_counts;
        }
        score_pmfs.push(map.into_iter().collect());
    }
    score_pmfs
}

fn build_blended_histograms() -> Vec<Vec<(u16, u32)>> {
    let mut blended: Vec<Vec<(u16, u32)>> = BUFF_TYPES
        .iter()
        .map(|buff| buff.histogram.to_vec())
        .collect();
    blend_group(&mut blended, &BLEND_GROUP_CRIT);
    blend_group(&mut blended, &BLEND_GROUP_MAIN);
    blended
}

fn blend_group(blended: &mut [Vec<(u16, u32)>], group: &[usize]) {
    let len = BUFF_TYPES[group[0]].histogram.len();
    let mut counts: Vec<u32> = vec![0; len];

    for &buff_index in group.iter() {
        for (value_index, &(_, count)) in BUFF_TYPES[buff_index].histogram.iter().enumerate() {
            counts[value_index] += count;
        }
    }

    for &buff_index in group.iter() {
        for (value_index, (_, count)) in blended[buff_index].iter_mut().enumerate() {
            *count = counts[value_index];
        }
    }
}
