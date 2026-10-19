use crate::data::NUM_ECHO_SLOTS;

const ECHO_COST: f64 = 1.0;

const TUNER_COST: f64 = 10.0;
const TUNER_REFUND_RATIO: f64 = 0.3;

// EXP costs are in "Premium Sealed Tubes", where 1 tube = 5000 raw EXP.
const EXP_PER_TUBE: f64 = 5000.0;
// Cumulative raw EXP spent/refunded at +0, +5, +10, +15, +20, +25.
// Measured recycle rewards are rounded by level, so one refund ratio is insufficient.
const EXP_COST_BY_LEVEL: [f64; NUM_ECHO_SLOTS + 1] =
    [0.0, 4500.0, 16500.0, 40000.0, 79500.0, 143000.0];
const EXP_REFUND_BY_LEVEL: [f64; NUM_ECHO_SLOTS + 1] =
    [0.0, 3000.0, 12000.0, 30000.0, 59500.0, 106500.0];

fn net_exp_at_level(slot: usize) -> f64 {
    (EXP_COST_BY_LEVEL[slot] - EXP_REFUND_BY_LEVEL[slot]) / EXP_PER_TUBE
}
fn net_exp_increment(slot: usize) -> f64 {
    net_exp_at_level(slot + 1) - net_exp_at_level(slot)
}

// Shell credit cost not considered.
// Each (raw) Echo EXP requires 0.1 Shell Credit.
// Each tune attempt requires 2000 Shell Credit.

/// Invalid resource weights.
/// `NegativeWeight` also covers non-finite weights.
#[derive(Debug)]
pub enum CostModelError {
    NegativeWeight { field: &'static str, value: f64 },
    AllWeightsZero,
}

/// Relative costs for echoes, tuners and EXP tubes, with decomposition refunds.
/// One EXP tube is 5000 raw EXP. Frequency weight belongs to `JointPolicySolver`.
/// This type is `Copy`; mutating another copy does not update an existing solver.
#[derive(Clone, Copy)]
pub struct CostModel {
    weight_echo: f64,
    weight_tuner: f64,
    weight_exp: f64,

    // Cached costs
    reveal_cost_cached: [f64; NUM_ECHO_SLOTS],
}

impl CostModel {
    pub(crate) fn normalized(&self) -> (Self, f64) {
        let scale = self.weight_echo.max(self.weight_tuner).max(self.weight_exp);
        (
            Self::build_cached(
                self.weight_echo / scale,
                self.weight_tuner / scale,
                self.weight_exp / scale,
            ),
            scale,
        )
    }

    pub(crate) fn weighted_resources(&self, echo: f64, tuner: f64, exp: f64) -> f64 {
        self.weight_echo * echo + self.weight_tuner * tuner + self.weight_exp * exp
    }

    /// Create a cost model with validation.
    /// Weights must be finite, nonnegative and not all zero.
    /// These weights express relative value, not inventory limits.
    /// EXP costs and recycle rewards use the fixed measured level tables.
    pub fn new(
        weight_echo: f64,
        weight_tuner: f64,
        weight_exp: f64,
    ) -> Result<Self, CostModelError> {
        Self::validate_weights(weight_echo, weight_tuner, weight_exp)?;
        Ok(Self::build_cached(weight_echo, weight_tuner, weight_exp))
    }

    /// Validate the weights
    ///
    /// Constraints enforced:
    /// - weights are finite and >= 0
    /// - not all weights are zero
    fn validate_weights(
        weight_echo: f64,
        weight_tuner: f64,
        weight_exp: f64,
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

        if weight_echo == 0.0 && weight_tuner == 0.0 && weight_exp == 0.0 {
            return Err(CostModelError::AllWeightsZero);
        }

        Ok(())
    }

    /// Build a cost model from the weights (without validation).
    fn build_cached(weight_echo: f64, weight_tuner: f64, weight_exp: f64) -> Self {
        let weighted_echo_cost = weight_echo * ECHO_COST;
        let weighted_tuner_cost = weight_tuner * (1.0 - TUNER_REFUND_RATIO) * TUNER_COST;

        let mut reveal_cost_cached = [0.0; NUM_ECHO_SLOTS];
        for (slot, cost) in reveal_cost_cached.iter_mut().enumerate() {
            let base = weighted_tuner_cost + weight_exp * net_exp_increment(slot);
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
            reveal_cost_cached,
        }
    }

    /// Create a cost model with only weight_tuner=1.0
    pub fn tuner_only() -> Self {
        Self::build_cached(0.0, 1.0, 0.0)
    }

    /// Validate new weights and update the cost model.
    /// `None` preserves the previous field. Validation failure leaves the model unchanged.
    pub fn update_weights(
        &mut self,
        new_weight_echo: Option<f64>,
        new_weight_tuner: Option<f64>,
        new_weight_exp: Option<f64>,
    ) -> Result<(), CostModelError> {
        let weight_echo = new_weight_echo.unwrap_or(self.weight_echo);
        let weight_tuner = new_weight_tuner.unwrap_or(self.weight_tuner);
        let weight_exp = new_weight_exp.unwrap_or(self.weight_exp);

        Self::validate_weights(weight_echo, weight_tuner, weight_exp)?;
        *self = Self::build_cached(weight_echo, weight_tuner, weight_exp);
        Ok(())
    }

    /// Unweighted net tuner cost per reveal: 7 after the modeled 30% refund.
    pub fn tuner_cost(&self) -> f64 {
        (1.0 - TUNER_REFUND_RATIO) * TUNER_COST
    }

    /// Unweighted net EXP increment in tubes for reveal index `slot` (`0..5`).
    ///
    /// # Panics
    /// Panics when `slot >= 5`.
    pub fn exp_cost(&self, slot: usize) -> f64 {
        net_exp_increment(slot)
    }

    /// Calculate the exp cost for a full upgrade starting from current_slot
    ///
    /// Must ensure `current_slot` is in 0..=5
    pub fn full_upgrade_exp_cost(&self, current_slot: usize) -> f64 {
        net_exp_at_level(NUM_ECHO_SLOTS) - net_exp_at_level(current_slot)
    }

    /// The weighted cost to reveal `slot`.
    /// `slot` is zero-based (`0..5`); the first reveal includes the echo cost.
    ///
    /// # Panics
    /// Panics when `slot >= 5`.
    pub fn weighted_reveal_cost(&self, slot: usize) -> f64 {
        self.reveal_cost_cached[slot]
    }

    /// The additional tuner cost for an echo that is kept.
    pub fn success_additional_tuner_cost(&self) -> f64 {
        TUNER_COST * TUNER_REFUND_RATIO * (NUM_ECHO_SLOTS as f64)
    }

    /// The additional exp cost for an echo that is kept.
    pub fn success_additional_exp_cost(&self) -> f64 {
        EXP_REFUND_BY_LEVEL[NUM_ECHO_SLOTS] / EXP_PER_TUBE
    }

    /// The weighted additional cost for an echo that is kept.
    pub fn weighted_success_additional_cost(&self) -> f64 {
        self.weight_tuner * self.success_additional_tuner_cost()
            + self.weight_exp * self.success_additional_exp_cost()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }
    #[test]
    fn measured_recycling_at_each_stop_and_kept_product() {
        let model = CostModel::new(2., 3., 5.).unwrap();
        // Independent raw observations: net spending after recycling at each node.
        let spent = [4500., 16500., 40000., 79500., 143000.];
        let returned = [3000., 12000., 30000., 59500., 106500.];
        let mut net = 0.;
        for slot in 0..5 {
            net += model.exp_cost(slot);
            close(net * 5000., spent[slot] - returned[slot]);
            close(net + model.full_upgrade_exp_cost(slot + 1), 7.3);
            let weighted = (0..=slot)
                .map(|s| model.weighted_reveal_cost(s))
                .sum::<f64>();
            close(weighted, 2. + 3. * 7. * (slot + 1) as f64 + 5. * net);
        }
        close(
            model.full_upgrade_exp_cost(0) + model.success_additional_exp_cost(),
            28.6,
        );
        close(model.success_additional_exp_cost(), 21.3);
        // Discard at +10, discard at +20, then keep a fully leveled Echo.
        let exp = model.exp_cost(0) * 3.
            + model.exp_cost(1) * 3.
            + model.exp_cost(2) * 2.
            + model.exp_cost(3) * 2.
            + model.exp_cost(4)
            + model.success_additional_exp_cost();
        close(exp * 5000., 4500. + 20000. + 143000.);
    }
    #[test]
    fn normalization_and_weight_updates_preserve_measured_costs() {
        let mut model = CostModel::new(2., 3., 5.).unwrap();
        let (normalized, scale) = model.normalized();
        for slot in 0..5 {
            close(
                normalized.weighted_reveal_cost(slot) * scale,
                model.weighted_reveal_cost(slot),
            );
        }
        close(
            normalized.weighted_success_additional_cost() * scale,
            model.weighted_success_additional_cost(),
        );
        let before = model.weighted_reveal_cost(0);
        assert!(model.update_weights(None, None, Some(f64::NAN)).is_err());
        close(model.weighted_reveal_cost(0), before);
        model.update_weights(Some(0.), Some(0.), Some(1.)).unwrap();
        for slot in 0..5 {
            close(model.weighted_reveal_cost(slot), model.exp_cost(slot));
        }
    }
}
