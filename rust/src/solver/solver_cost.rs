use crate::{CostModel, data::MAX_SELECTED_TYPES};

#[derive(Debug)]
pub struct ExpectedResourceCost {
    pub success_prob: f64,
    pub tuner_per_succ: f64,
    pub exp_per_succ: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ExpectedCostState {
    pub success_prob: f64,
    pub tuner: f64,
    pub exp: f64,
}

impl Default for ExpectedCostState {
    fn default() -> Self {
        Self {
            success_prob: f64::NAN,
            tuner: 0.0,
            exp: 0.0,
        }
    }
}

impl ExpectedCostState {
    pub fn failed_state() -> Self {
        Self {
            success_prob: 0.0,
            tuner: 0.0,
            exp: 0.0,
        }
    }

    pub fn always_success_state(cost_model: &CostModel, used_slots: usize) -> Self {
        let tuner = (MAX_SELECTED_TYPES - used_slots) as f64 * cost_model.tuner_cost();
        let exp: f64 = cost_model.full_upgrade_exp_cost(used_slots);

        Self {
            success_prob: 1.0,
            tuner,
            exp,
        }
    }
}
