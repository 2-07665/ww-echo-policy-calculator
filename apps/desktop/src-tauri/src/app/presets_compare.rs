fn resolved_variants_equal(
    left: &ScorerPresetResolvedVariantItem,
    right: &ScorerPresetResolvedVariantItem,
) -> bool {
    left.variant_name == right.variant_name
        && left.preset_intro == right.preset_intro
        && option_f64_bits_equal(left.main_buff_score, right.main_buff_score)
        && option_f64_bits_equal(left.normalized_max_score, right.normalized_max_score)
        && BUFF_TYPES.iter().all(|buff_name| {
            let lhs = *left.weights.get(*buff_name).unwrap_or(&0.0);
            let rhs = *right.weights.get(*buff_name).unwrap_or(&0.0);
            f64_bits_equal(lhs, rhs)
        })
}

fn resolved_presets_equal(left: &ScorerPresetResolvedItem, right: &ScorerPresetResolvedItem) -> bool {
    left.preset_name == right.preset_name
        && left.variants.len() == right.variants.len()
        && left
            .variants
            .iter()
            .zip(right.variants.iter())
            .all(|(lhs, rhs)| resolved_variants_equal(lhs, rhs))
}

fn f64_bits_equal(left: f64, right: f64) -> bool {
    left.to_bits() == right.to_bits()
}

fn cost_weights_equal(left: &CostWeightsOutput, right: &CostWeightsOutput) -> bool {
    f64_bits_equal(left.w_echo, right.w_echo)
        && f64_bits_equal(left.w_tuner, right.w_tuner)
        && f64_bits_equal(left.w_exp, right.w_exp)
}

fn f64_weight_arrays_equal(left: &[f64; NUM_BUFFS], right: &[f64; NUM_BUFFS]) -> bool {
    left.iter()
        .zip(right.iter())
        .all(|(lhs, rhs)| f64_bits_equal(*lhs, *rhs))
}

fn scorer_configs_equal(left: &UpgradeScorerConfig, right: &UpgradeScorerConfig) -> bool {
    match (left, right) {
        (
            UpgradeScorerConfig::LinearDefault {
                weights: lw,
                main_buff_score: lmain,
                normalized_max_score: lnorm,
            },
            UpgradeScorerConfig::LinearDefault {
                weights: rw,
                main_buff_score: rmain,
                normalized_max_score: rnorm,
            },
        ) => {
            f64_weight_arrays_equal(lw, rw)
                && f64_bits_equal(*lmain, *rmain)
                && f64_bits_equal(*lnorm, *rnorm)
        }
        (
            UpgradeScorerConfig::WuwaEchoTool {
                weights: lw,
                main_buff_score: lmain,
                normalized_max_score: lnorm,
            },
            UpgradeScorerConfig::WuwaEchoTool {
                weights: rw,
                main_buff_score: rmain,
                normalized_max_score: rnorm,
            },
        ) => {
            f64_weight_arrays_equal(lw, rw)
                && f64_bits_equal(*lmain, *rmain)
                && f64_bits_equal(*lnorm, *rnorm)
        }
        (
            UpgradeScorerConfig::McBoostAssistant { weights: lw },
            UpgradeScorerConfig::McBoostAssistant { weights: rw },
        ) => f64_weight_arrays_equal(lw, rw),
        (
            UpgradeScorerConfig::QQBot {
                qq_bot_weights: lw,
                main_buff_score: lmain,
                normalized_max_score: lnorm,
            },
            UpgradeScorerConfig::QQBot {
                qq_bot_weights: rw,
                main_buff_score: rmain,
                normalized_max_score: rnorm,
            },
        ) => {
            f64_weight_arrays_equal(lw, rw)
                && f64_bits_equal(*lmain, *rmain)
                && f64_bits_equal(*lnorm, *rnorm)
        }
        (
            UpgradeScorerConfig::Fixed { weights: lw },
            UpgradeScorerConfig::Fixed { weights: rw },
        ) => lw == rw,
        _ => false,
    }
}

