use crate::data::NUM_ECHO_SLOTS;

const ECHO_COST: f64 = 1.0;

const TUNER_COST: f64 = 10.0;
const TUNER_REFUND_RATIO: f64 = 0.3;

// EXP costs are in "Premium Sealed Tubes", where 1 tube = 5000 raw EXP.
const EXP_PER_TUBE: f64 = 5000.0;
const EXP_COST_BY_LEVEL: [f64; NUM_ECHO_SLOTS] = [
    4400.0 / EXP_PER_TUBE,
    16500.0 / EXP_PER_TUBE,
    39600.0 / EXP_PER_TUBE,
    79100.0 / EXP_PER_TUBE,
    142600.0 / EXP_PER_TUBE,
];
const EXP_INCREMENTAL_COSTS: [f64; NUM_ECHO_SLOTS] = [
    4400.0 / EXP_PER_TUBE,
    12100.0 / EXP_PER_TUBE,
    23100.0 / EXP_PER_TUBE,
    39500.0 / EXP_PER_TUBE,
    63500.0 / EXP_PER_TUBE,
];
// The ideal refund ratio is 0.75.
const EXP_REFUND_RATIO_DEFAULT: f64 = 0.66;
const EXP_REFUND_RATIO_MAX: f64 = 0.75;

// Shell credit cost not considered.
// Each (raw) Echo EXP requires 0.1 Shell Credit.
// Each tune attempt requires 2000 Shell Credit.

#[derive(Debug)]
pub enum CostModelError {
    NegativeWeight { field: &'static str, value: f64 },
    AllWeightsZero,
    InvalidExpRefundRatio { value: f64 },
}

pub struct CostModel {
    weight_echo: f64,
    weight_tuner: f64,
    weight_exp: f64,
    exp_refund_ratio: f64,

    // Cached costs
    reveal_cost_cached: [f64; NUM_ECHO_SLOTS],
}

impl CostModel {
    /// Create a cost model with validation.
    pub fn new(
        weight_echo: f64,
        weight_tuner: f64,
        weight_exp: f64,
        exp_refund_ratio: f64,
    ) -> Result<Self, CostModelError> {
        Self::validate_weights(weight_echo, weight_tuner, weight_exp, exp_refund_ratio)?;
        Ok(Self::build_cached(
            weight_echo,
            weight_tuner,
            weight_exp,
            exp_refund_ratio,
        ))
    }

    /// Validate the weights
    ///
    /// Constraints enforced:
    /// - weights are finite and >= 0
    /// - exp_refund_ratio is finite and in [0, 0.75]
    /// - not all weights are zero
    fn validate_weights(
        weight_echo: f64,
        weight_tuner: f64,
        weight_exp: f64,
        exp_refund_ratio: f64,
    ) -> Result<(), CostModelError> {
        if !weight_echo.is_finite() || weight_echo < 0.0 {
            return Err(CostModelError::NegativeWeight {
                field: "weight_echo",
                value: weight_echo,
            });
        }
        if !weight_tuner.is_finite() || weight_tuner < 0.0 {
            return Err(CostModelError::NegativeWeight {
                field: "weight_tuner",
                value: weight_tuner,
            });
        }
        if !weight_exp.is_finite() || weight_exp < 0.0 {
            return Err(CostModelError::NegativeWeight {
                field: "weight_exp",
                value: weight_exp,
            });
        }

        if !exp_refund_ratio.is_finite()
            || !(0.0..=EXP_REFUND_RATIO_MAX).contains(&exp_refund_ratio)
        {
            return Err(CostModelError::InvalidExpRefundRatio {
                value: exp_refund_ratio,
            });
        }

        if weight_echo == 0.0 && weight_tuner == 0.0 && weight_exp == 0.0 {
            return Err(CostModelError::AllWeightsZero);
        }

        Ok(())
    }

    /// Build a cost model from the weights (without validation).
    fn build_cached(
        weight_echo: f64,
        weight_tuner: f64,
        weight_exp: f64,
        exp_refund_ratio: f64,
    ) -> Self {
        let weighted_echo_cost = weight_echo * ECHO_COST;
        let weighted_tuner_cost = weight_tuner * (1.0 - TUNER_REFUND_RATIO) * TUNER_COST;
        let weighted_exp_factor = weight_exp * (1.0 - exp_refund_ratio);

        let mut reveal_cost_cached = [0.0; NUM_ECHO_SLOTS];
        for (slot, cost) in reveal_cost_cached.iter_mut().enumerate() {
            let base = weighted_tuner_cost + weighted_exp_factor * EXP_INCREMENTAL_COSTS[slot];
            *cost = if slot == 0 {
                base + weighted_echo_cost
            } else {
                base
            };
        }

        Self {
            weight_echo,
            weight_tuner,
            weight_exp,
            exp_refund_ratio,
            reveal_cost_cached,
        }
    }

    /// Create a cost model with only weight_tuner=1.0
    pub fn tuner_only() -> Self {
        Self::build_cached(0.0, 1.0, 0.0, EXP_REFUND_RATIO_DEFAULT)
    }

    /// Validate new weights and update the cost model.
    pub fn update_weights(
        &mut self,
        new_weight_echo: Option<f64>,
        new_weight_tuner: Option<f64>,
        new_weight_exp: Option<f64>,
        new_exp_refund_ratio: Option<f64>,
    ) -> Result<(), CostModelError> {
        let weight_echo = new_weight_echo.unwrap_or(self.weight_echo);
        let weight_tuner = new_weight_tuner.unwrap_or(self.weight_tuner);
        let weight_exp = new_weight_exp.unwrap_or(self.weight_exp);
        let exp_refund_ratio = new_exp_refund_ratio.unwrap_or(self.exp_refund_ratio);

        Self::validate_weights(weight_echo, weight_tuner, weight_exp, exp_refund_ratio)?;
        *self = Self::build_cached(weight_echo, weight_tuner, weight_exp, exp_refund_ratio);
        Ok(())
    }

    pub fn tuner_cost(&self) -> f64 {
        (1.0 - TUNER_REFUND_RATIO) * TUNER_COST
    }

    pub fn exp_cost(&self, slot: usize) -> f64 {
        (1.0 - self.exp_refund_ratio) * EXP_INCREMENTAL_COSTS[slot]
    }

    /// Calculate the exp cost for a full upgrade starting from current_slot
    ///
    /// Must ensure `current_slot` is in 0..=5
    pub fn full_upgrade_exp_cost(&self, current_slot: usize) -> f64 {
        let exp_now = if current_slot == 0 {
            0.0
        } else {
            EXP_COST_BY_LEVEL[current_slot - 1]
        };
        (1.0 - self.exp_refund_ratio) * (EXP_COST_BY_LEVEL[NUM_ECHO_SLOTS - 1] - exp_now)
    }

    /// The weighted cost to reveal `slot`.
    pub fn weighted_reveal_cost(&self, slot: usize) -> f64 {
        self.reveal_cost_cached[slot]
    }

    /// The additional tuner cost for an echo that is kept.
    pub fn success_additional_tuner_cost(&self) -> f64 {
        TUNER_COST * TUNER_REFUND_RATIO * (NUM_ECHO_SLOTS as f64)
    }

    /// The additional exp cost for an echo that is kept.
    pub fn success_additional_exp_cost(&self) -> f64 {
        self.exp_refund_ratio * EXP_COST_BY_LEVEL[NUM_ECHO_SLOTS - 1]
    }

    /// The weighted additional cost for an echo that is kept.
    pub fn weighted_success_additional_cost(&self) -> f64 {
        self.weight_tuner * self.success_additional_tuner_cost()
            + self.weight_exp * self.success_additional_exp_cost()
    }
}
