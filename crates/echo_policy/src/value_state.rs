//! Shared, exact state compression for value-aware solvers.
use std::collections::BTreeMap;

use crate::data::{BUFF_TYPES, NUM_BUFFS, NUM_ECHO_SLOTS};
use crate::{InternalScorer, RerollPolicySolverError as Error};

/// Allocation guards for exact value-aware solvers. Raising these limits can
/// require substantial memory; no approximation is used when a limit is hit.
#[derive(Clone, Copy, Debug)]
pub struct SolverLimits {
    /// Maximum full-state count, lock-configuration count, or (for the joint
    /// solver) total partial-state count. Each is checked separately.
    pub max_states: usize,
    /// Maximum number of full-state/lock-configuration memberships. Stored
    /// as compact u32 indices; this is not a dense outcome transition matrix.
    pub max_transitions: usize,
}

impl Default for SolverLimits {
    fn default() -> Self {
        Self {
            max_states: 2_500_000,
            max_transitions: 80_000_000,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Config {
    pub code: u64,
    pub score: u16,
    pub count: usize,
    pub probability: f64,
}

pub(crate) struct ValueSpace {
    pub types: Vec<usize>,
    pub pmfs: Vec<Vec<(u16, f64)>>,
    pub strides: Vec<u64>,
    pub zeros: usize,
    pub max_score: u16,
    pub limits: SolverLimits,
    suffix: Vec<[usize; NUM_ECHO_SLOTS + 1]>,
    raw_scores: Vec<Vec<(u16, u16)>>,
}

pub(crate) fn choose(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    (0..k).fold(1.0, |v, i| v * (n - i) as f64 / (i + 1) as f64)
}

impl ValueSpace {
    pub fn new<S: InternalScorer>(scorer: &S, limits: SolverLimits) -> Result<Self, Error> {
        let all = scorer.build_score_pmfs();
        if all.len() != NUM_BUFFS {
            return Err(Error::InvalidScorePmf { buff_index: None });
        }
        let mut types = Vec::new();
        let mut pmfs = Vec::new();
        let mut maxima = Vec::new();
        let mut raw_scores = Vec::new();
        for (i, pmf) in all.into_iter().enumerate() {
            let mut merged = BTreeMap::new();
            let mut sum = 0.0;
            for (score, p) in pmf {
                if !p.is_finite() || p < 0.0 {
                    return Err(Error::InvalidScorePmf {
                        buff_index: Some(i),
                    });
                }
                sum += p;
                if p > 0.0 {
                    *merged.entry(score).or_insert(0.0) += p;
                }
            }
            if !sum.is_finite() || (sum - 1.0).abs() > 1e-9 || merged.is_empty() {
                return Err(Error::InvalidScorePmf {
                    buff_index: Some(i),
                });
            }
            let pmf: Vec<_> = merged.into_iter().map(|(s, p)| (s, p / sum)).collect();
            let max = pmf.last().unwrap().0;
            maxima.push(u32::from(max));
            if max > 0 {
                types.push(i);
                pmfs.push(pmf);
            }
            let mut raw = Vec::new();
            for &(value, _) in BUFF_TYPES[i].histogram {
                raw.push((value, scorer.buff_score_internal(i, value)?));
            }
            raw_scores.push(raw);
        }
        maxima.sort_unstable_by(|a, b| b.cmp(a));
        let max_score: u32 = maxima.iter().take(NUM_ECHO_SLOTS).sum();
        if max_score > u32::from(u16::MAX) {
            return Err(Error::ScoreRangeOverflow { max_score });
        }
        let mut strides = Vec::new();
        let mut stride = 1u64;
        for pmf in &pmfs {
            strides.push(stride);
            stride = stride
                .checked_mul((pmf.len() + 1) as u64)
                .ok_or(Error::StateEncodingOverflow)?;
        }
        let mut suffix = vec![[0usize; NUM_ECHO_SLOTS + 1]; pmfs.len() + 1];
        suffix[pmfs.len()][0] = 1;
        for i in (0..pmfs.len()).rev() {
            suffix[i][0] = 1;
            for k in 1..=NUM_ECHO_SLOTS {
                suffix[i][k] = suffix[i + 1][k]
                    .saturating_add(suffix[i + 1][k - 1].saturating_mul(pmfs[i].len()));
            }
        }
        let space = Self {
            zeros: NUM_BUFFS - types.len(),
            types,
            pmfs,
            strides,
            max_score: max_score as u16,
            limits,
            suffix,
            raw_scores,
        };
        space.check_count(space.count(space.min_count(5), 5))?;
        Ok(space)
    }

    pub fn check_count(&self, count: usize) -> Result<(), Error> {
        if count > self.limits.max_states || count >= u32::MAX as usize {
            Err(Error::StateSpaceTooLarge {
                states: count,
                limit: self.limits.max_states.min(u32::MAX as usize - 1),
            })
        } else {
            Ok(())
        }
    }

    pub fn min_count(&self, slots: usize) -> usize {
        slots.saturating_sub(self.zeros)
    }
    pub fn count(&self, min: usize, max: usize) -> usize {
        self.suffix[0][min..=max]
            .iter()
            .copied()
            .fold(0, usize::saturating_add)
    }

    fn suffix_count(&self, i: usize, used: usize, min: usize, max: usize) -> usize {
        if used > max {
            return 0;
        }
        self.suffix[i][min.saturating_sub(used)..=max - used]
            .iter()
            .sum()
    }

    // Rank only admissible configurations, without a dense mixed-radix lookup
    // table (which would contain billions of unused entries for many scorers).
    pub fn rank(&self, code: u64, min: usize, max: usize) -> usize {
        let mut rank = 0;
        let mut used = 0;
        for i in 0..self.types.len() {
            let d = self.digit(code, i);
            if d > 0 {
                rank += self.suffix_count(i + 1, used, min, max);
                rank += (d - 1) * self.suffix_count(i + 1, used + 1, min, max);
                used += 1;
            }
        }
        rank
    }

    pub fn digit(&self, code: u64, i: usize) -> usize {
        ((code / self.strides[i]) % (self.pmfs[i].len() + 1) as u64) as usize
    }

    pub fn configs(&self, min: usize, max: usize) -> Vec<Config> {
        let mut result = Vec::with_capacity(self.count(min, max));
        self.enumerate(
            0,
            min,
            max,
            Config {
                code: 0,
                score: 0,
                count: 0,
                probability: 1.0,
            },
            &mut result,
        );
        result
    }

    fn enumerate(&self, i: usize, min: usize, max: usize, c: Config, out: &mut Vec<Config>) {
        if c.count > max || c.count + self.types.len() - i < min {
            return;
        }
        if i == self.types.len() {
            out.push(c);
            return;
        }
        self.enumerate(i + 1, min, max, c, out);
        if c.count < max {
            for (j, &(s, p)) in self.pmfs[i].iter().enumerate() {
                self.enumerate(
                    i + 1,
                    min,
                    max,
                    Config {
                        code: c.code + (j + 1) as u64 * self.strides[i],
                        score: c.score + s,
                        count: c.count + 1,
                        probability: c.probability * p,
                    },
                    out,
                );
            }
        }
    }

    pub fn subsets(&self, code: u64, out: &mut Vec<u64>) {
        out.clear();
        out.push(0);
        let mut count = 0;
        for i in 0..self.types.len() {
            let d = self.digit(code, i);
            if d > 0 {
                count += 1;
                for j in 0..out.len() {
                    out.push(out[j] + d as u64 * self.strides[i]);
                }
            }
        }
        if count == NUM_ECHO_SLOTS {
            out.pop();
        }
    }

    pub fn mask(&self, code: u64) -> u16 {
        self.types.iter().enumerate().fold(0, |mask, (i, &t)| {
            if self.digit(code, i) > 0 {
                mask | (1 << t)
            } else {
                mask
            }
        })
    }

    pub fn echo_code(&self, echo: &[(usize, u16)], full: bool) -> Result<u64, Error> {
        if echo.len() > NUM_ECHO_SLOTS || (full && echo.len() != NUM_ECHO_SLOTS) {
            return Err(Error::InvalidEcho);
        }
        let mut mask = 0u16;
        let mut code = 0;
        for &(t, value) in echo {
            if t >= NUM_BUFFS || mask & (1 << t) != 0 {
                return Err(Error::InvalidEcho);
            }
            mask |= 1 << t;
            let score = self.raw_scores[t]
                .iter()
                .find(|&&(v, _)| v == value)
                .map(|&(_, s)| s)
                .ok_or(Error::InvalidEcho)?;
            if let Some(i) = self.types.iter().position(|&j| j == t) {
                let d = self.pmfs[i]
                    .iter()
                    .position(|&(s, _)| s == score)
                    .ok_or(Error::InvalidEcho)?;
                code += (d + 1) as u64 * self.strides[i];
            } else if score != 0 {
                return Err(Error::InvalidEcho);
            }
        }
        Ok(code)
    }

    pub fn mask_code(&self, mask: u16) -> Result<u64, Error> {
        if mask >> NUM_BUFFS != 0 || mask.count_ones() as usize != NUM_ECHO_SLOTS {
            return Err(Error::InvalidMask { mask });
        }
        let mut code = 0;
        for (i, &t) in self.types.iter().enumerate() {
            if mask & (1 << t) != 0 {
                if self.pmfs[i].len() != 1 {
                    return Err(Error::ValuesRequired);
                }
                code += self.strides[i];
            }
        }
        Ok(code)
    }
}
