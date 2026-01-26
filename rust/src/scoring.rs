use std::collections::BTreeMap;

use crate::data::{BUFF_TYPES, MAX_SELECTED_TYPES, TOTAL_BUFF_TYPES};

const BLEND_GROUP_CRIT: [usize; 2] = [0, 1];
const BLEND_GROUP_MAIN: [usize; 9] = [2, 3, 4, 7, 8, 9, 10, 11, 12];

pub const SCORE_MULTIPLIER: f64 = 100.0;
const FIXED_SCORER_TOP_WEIGHT_SUM_MAX: f64 = 60000.0 / SCORE_MULTIPLIER;

pub trait Scorer {
    fn score(&self, buff_type_index: usize, raw_value: f64) -> f64;

    fn build_score_pmfs(&self, blend_data: bool) -> Vec<Vec<(u16, f64)>>;
}

#[derive(Debug)]
pub enum ScorerError {
    NegativeWeight { index: usize, value: f64 },
    AllWeightsZero,
    FixedScorerTopWeightsTooLarge { sum: f64, max: f64 },
}

fn validate_weights(weights: &[f64; TOTAL_BUFF_TYPES]) -> Result<(), ScorerError> {
    let mut any_positive = false;
    for (i, &w) in weights.iter().enumerate() {
        if !w.is_finite() || w < 0.0 {
            return Err(ScorerError::NegativeWeight { index: i, value: w });
        }
        if w > 0.0 {
            any_positive = true;
        }
    }
    if !any_positive {
        return Err(ScorerError::AllWeightsZero);
    }
    Ok(())
}

fn top_weight_sum(weights: &[f64; TOTAL_BUFF_TYPES]) -> f64 {
    let mut top_weights: [f64; MAX_SELECTED_TYPES] = [0.0; MAX_SELECTED_TYPES];
    for &w in weights.iter() {
        if w <= top_weights[MAX_SELECTED_TYPES - 1] {
            continue;
        }
        let mut j: usize = MAX_SELECTED_TYPES - 1;
        while j > 0 && w > top_weights[j - 1] {
            top_weights[j] = top_weights[j - 1];
            j -= 1;
        }
        top_weights[j] = w;
    }
    top_weights.into_iter().sum()
}

pub struct FixedScorer {
    weights: [f64; TOTAL_BUFF_TYPES],
}

impl FixedScorer {
    pub fn new(weights: [f64; TOTAL_BUFF_TYPES]) -> Result<Self, ScorerError> {
        validate_weights(&weights)?;
        let top_weight_sum = top_weight_sum(&weights);
        if top_weight_sum > FIXED_SCORER_TOP_WEIGHT_SUM_MAX {
            return Err(ScorerError::FixedScorerTopWeightsTooLarge {
                sum: top_weight_sum,
                max: FIXED_SCORER_TOP_WEIGHT_SUM_MAX,
            });
        }
        Ok(Self { weights })
    }
}

impl Scorer for FixedScorer {
    fn score(&self, buff_type_index: usize, _raw_value: f64) -> f64 {
        self.weights[buff_type_index]
    }

    fn build_score_pmfs(&self, blend_data: bool) -> Vec<Vec<(u16, f64)>> {
        build_score_prob_mass_functions(self, blend_data)
    }
}

pub struct LinearScorer {
    weights: [f64; TOTAL_BUFF_TYPES],
    max_weight_sum: f64,
    max_values: [f64; TOTAL_BUFF_TYPES],
}

impl LinearScorer {
    pub fn new(weights: [f64; TOTAL_BUFF_TYPES]) -> Result<Self, ScorerError> {
        validate_weights(&weights)?;
        let max_weight_sum = top_weight_sum(&weights);

        let max_values = BUFF_TYPES.map(|buff| buff.max_value as f64);

        Ok(Self {
            weights,
            max_weight_sum,
            max_values,
        })
    }
}

impl Scorer for LinearScorer {
    fn score(&self, buff_type_index: usize, raw_value: f64) -> f64 {
        let w = self.weights[buff_type_index];
        if raw_value <= 0.0 {
            return 0.0;
        }
        let ratio = if raw_value > self.max_values[buff_type_index] {
            1.0
        } else {
            raw_value / self.max_values[buff_type_index]
        };
        SCORE_MULTIPLIER * w / self.max_weight_sum * ratio
    }

    fn build_score_pmfs(&self, blend_data: bool) -> Vec<Vec<(u16, f64)>> {
        build_score_prob_mass_functions(self, blend_data)
    }
}

pub fn build_score_prob_mass_functions<S: Scorer>(
    scorer: &S,
    blend_data: bool,
) -> Vec<Vec<(u16, f64)>> {
    if blend_data {
        let blended_storage = build_blended_histograms();
        let histograms: Vec<&[(u16, u32)]> =
            blended_storage.iter().map(|hist| hist.as_slice()).collect();
        build_score_prob_mass_functions_from_histograms(scorer, &histograms)
    } else {
        let histograms: Vec<&[(u16, u32)]> = BUFF_TYPES.iter().map(|buff| buff.histogram).collect();
        build_score_prob_mass_functions_from_histograms(scorer, &histograms)
    }
}

fn build_score_prob_mass_functions_from_histograms<S: Scorer>(
    scorer: &S,
    histograms: &[&[(u16, u32)]],
) -> Vec<Vec<(u16, f64)>> {
    let mut score_pmfs: Vec<Vec<(u16, f64)>> = Vec::with_capacity(TOTAL_BUFF_TYPES);
    for (i, hist) in histograms.iter().enumerate() {
        let total: f64 = hist.iter().map(|&(_, c)| c as f64).sum();
        let mut map: BTreeMap<u16, f64> = BTreeMap::new();
        for &(raw_value, count) in hist.iter() {
            let score_float = scorer.score(i, raw_value as f64);
            let bucket_int = (score_float * SCORE_MULTIPLIER).round() as u16;
            *map.entry(bucket_int).or_insert(0.0) += count as f64 / total;
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

    for &idx in group.iter() {
        for (pos, &(_, count)) in BUFF_TYPES[idx].histogram.iter().enumerate() {
            counts[pos] += count;
        }
    }

    for &idx in group.iter() {
        for (pos, (_, count)) in blended[idx].iter_mut().enumerate() {
            *count = counts[pos];
        }
    }
}
