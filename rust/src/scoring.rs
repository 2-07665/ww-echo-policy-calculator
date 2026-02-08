use std::collections::BTreeMap;

use crate::data::{BUFF_MAX_VALUES, BUFF_TYPES, NUM_BUFFS, NUM_ECHO_SLOTS};

const BLEND_GROUP_CRIT: [usize; 2] = [0, 1];
const BLEND_GROUP_MAIN: [usize; 9] = [2, 3, 4, 7, 8, 9, 10, 11, 12];

pub const SCORE_MULTIPLIER: f64 = 100.0;
const FIXED_SCORER_SCORE_LIMIT: f64 = u16::MAX as f64 / SCORE_MULTIPLIER;

pub trait Scorer {
    fn buff_score(&self, buff_index: usize, buff_value: f64) -> f64;

    fn echo_score(&self, echo: &[(usize, f64)]) -> f64 {
        let mut sum: f64 = 0.0;
        for &(buff_index, buff_value) in echo {
            if !(0..NUM_BUFFS).contains(&buff_index) {
                continue;
            }
            sum += self.buff_score(buff_index, buff_value);
        }
        sum
    }

    fn build_score_pmfs(&self, blend_data: bool) -> Vec<Vec<(u16, f64)>>;
}

#[derive(Debug)]
pub enum ScorerError {
    NegativeWeight { index: usize, weight: f64 },
    AllWeightsZero,
    FixedScorerWeightsTooLarge { sum: f64, limit: f64 },
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
    weights: [f64; NUM_BUFFS],
}

impl FixedScorer {
    pub fn new(weights: [f64; NUM_BUFFS]) -> Result<Self, ScorerError> {
        validate_weights(&weights)?;
        let top_weights_sum = top_weights_sum(&weights);
        if top_weights_sum > FIXED_SCORER_SCORE_LIMIT {
            return Err(ScorerError::FixedScorerWeightsTooLarge {
                sum: top_weights_sum,
                limit: FIXED_SCORER_SCORE_LIMIT,
            });
        }
        Ok(Self { weights })
    }
}

impl Scorer for FixedScorer {
    fn buff_score(&self, buff_index: usize, _buff_value: f64) -> f64 {
        self.weights[buff_index]
    }

    fn build_score_pmfs(&self, blend_data: bool) -> Vec<Vec<(u16, f64)>> {
        build_score_pmfs(self, blend_data)
    }
}

pub struct LinearScorer {
    weights: [f64; NUM_BUFFS],
    top_weights_sum: f64,
}

impl LinearScorer {
    pub fn new(weights: [f64; NUM_BUFFS]) -> Result<Self, ScorerError> {
        validate_weights(&weights)?;
        let top_weights_sum = top_weights_sum(&weights);
        Ok(Self {
            weights,
            top_weights_sum,
        })
    }
}

impl Scorer for LinearScorer {
    fn buff_score(&self, buff_index: usize, buff_value: f64) -> f64 {
        let weight = self.weights[buff_index];
        if buff_value <= 0.0 {
            return 0.0;
        }
        let buff_max_value = BUFF_MAX_VALUES[buff_index];
        let ratio = if buff_value > buff_max_value {
            1.0
        } else {
            buff_value / buff_max_value
        };
        SCORE_MULTIPLIER * weight / self.top_weights_sum * ratio
    }

    fn build_score_pmfs(&self, blend_data: bool) -> Vec<Vec<(u16, f64)>> {
        build_score_pmfs(self, blend_data)
    }
}

pub fn build_score_pmfs<S: Scorer>(scorer: &S, blend_data: bool) -> Vec<Vec<(u16, f64)>> {
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

fn build_score_pmfs_from_histograms<S: Scorer>(
    scorer: &S,
    histograms: &[&[(u16, u32)]],
) -> Vec<Vec<(u16, f64)>> {
    let mut score_pmfs: Vec<Vec<(u16, f64)>> = Vec::with_capacity(NUM_BUFFS);
    for (buff_index, histogram) in histograms.iter().enumerate() {
        let total_counts: f64 = histogram.iter().map(|&(_, c)| c as f64).sum();
        let mut map: BTreeMap<u16, f64> = BTreeMap::new();
        for &(buff_value, count) in histogram.iter() {
            let score_float = scorer.buff_score(buff_index, buff_value as f64);
            let bucket_int = (score_float * SCORE_MULTIPLIER).round() as u16;
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
