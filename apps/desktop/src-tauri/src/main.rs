#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::Instant;

use echo_policy::{
    CostModel, FixedScorer, InternalScorer, LinearScorer, RerollPolicySolver, SCORE_MULTIPLIER,
    UpgradePolicySolver, bits_to_mask, mask_to_bits,
};
use serde::{Deserialize, Serialize};
use tauri::State;

const NUM_BUFFS: usize = 13;
const MAX_SELECTED_TYPES: usize = 5;
const DEFAULT_TARGET_SCORE: f64 = 60.0;
const DEFAULT_FIXED_TARGET_SCORE: u16 = 7;
const DEFAULT_EXP_REFUND_RATIO: f64 = 0.66;
const DEFAULT_SCORER_TYPE: &str = "linear_default";

const SCORER_TYPE_LINEAR_DEFAULT: &str = "linear_default";
const SCORER_TYPE_MC_BOOST_ASSISTANT: &str = "mc_boost_assistant";
const SCORER_TYPE_QQ_BOT: &str = "qq_bot";
const SCORER_TYPE_FIXED: &str = "fixed";

const DEFAULT_LINEAR_MAIN_BUFF_SCORE: f64 = 0.0;
const DEFAULT_LINEAR_NORMALIZED_MAX_SCORE: f64 = 100.0;
const DEFAULT_QQ_BOT_MAIN_BUFF_SCORE: f64 = 0.0;
const DEFAULT_QQ_BOT_NORMALIZED_MAX_SCORE: f64 = 50.0;

const BUFF_TYPES: [&str; NUM_BUFFS] = [
    "Crit_Rate",
    "Crit_Damage",
    "Attack",
    "Defence",
    "HP",
    "Attack_Flat",
    "Defence_Flat",
    "HP_Flat",
    "ER",
    "Basic_Attack_Damage",
    "Heavy_Attack_Damage",
    "Skill_Damage",
    "Ult_Damage",
];

const BUFF_LABELS: [&str; NUM_BUFFS] = [
    "暴击",
    "暴击伤害",
    "攻击百分比",
    "防御百分比",
    "生命百分比",
    "攻击",
    "防御",
    "生命",
    "共鸣效率",
    "普攻伤害加成",
    "重击伤害加成",
    "共鸣技能伤害加成",
    "共鸣解放伤害加成",
];

const BUFF_TYPE_MAX_VALUES: [f64; NUM_BUFFS] = [
    105.0, 210.0, 116.0, 147.0, 116.0, 60.0, 70.0, 580.0, 124.0, 116.0, 116.0, 116.0, 116.0,
];

const DEFAULT_LINEAR_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    100.0, 100.0, 70.0, 0.0, 0.0, 30.0, 0.0, 0.0, 30.0, 40.0, 0.0, 0.0, 0.0,
];

const DEFAULT_MC_BOOST_ASSISTANT_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    1.0, 0.9, 0.0, 0.0, 0.7, 0.0, 0.0, 0.3, 0.25, 0.4, 0.0, 0.12, 0.32,
];

const DEFAULT_QQ_BOT_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    2.0, 1.0, 1.1, 0.0, 0.0, 0.1, 0.0, 0.0, 0.2, 0.0, 0.0, 0.0, 0.91,
];

const DEFAULT_FIXED_BUFF_WEIGHTS: [u16; NUM_BUFFS] = [3, 3, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0];

const VALUES_CRIT_RATE: [u16; 8] = [63, 69, 75, 81, 87, 93, 99, 105];
const VALUES_CRIT_DAMAGE: [u16; 8] = [126, 138, 150, 162, 174, 186, 198, 210];
const VALUES_ATTACK: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];
const VALUES_DEFENCE: [u16; 8] = [81, 90, 100, 109, 118, 128, 138, 147];
const VALUES_HP: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];
const VALUES_ATTACK_FLAT: [u16; 4] = [30, 40, 50, 60];
const VALUES_DEFENCE_FLAT: [u16; 4] = [40, 50, 60, 70];
const VALUES_HP_FLAT: [u16; 8] = [320, 360, 390, 430, 470, 510, 540, 580];
const VALUES_ER: [u16; 8] = [68, 76, 84, 92, 100, 108, 116, 124];
const VALUES_BASIC: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];
const VALUES_HEAVY: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];
const VALUES_SKILL: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];
const VALUES_ULT: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];

const BUFF_VALUE_OPTIONS: [&[u16]; NUM_BUFFS] = [
    &VALUES_CRIT_RATE,
    &VALUES_CRIT_DAMAGE,
    &VALUES_ATTACK,
    &VALUES_DEFENCE,
    &VALUES_HP,
    &VALUES_ATTACK_FLAT,
    &VALUES_DEFENCE_FLAT,
    &VALUES_HP_FLAT,
    &VALUES_ER,
    &VALUES_BASIC,
    &VALUES_HEAVY,
    &VALUES_SKILL,
    &VALUES_ULT,
];

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CostWeightsInput {
    #[serde(default)]
    w_echo: f64,
    #[serde(default, alias = "wDkq")]
    w_tuner: f64,
    #[serde(default)]
    w_exp: f64,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
struct CostWeightsOutput {
    w_echo: f64,
    w_tuner: f64,
    w_exp: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComputePolicyRequest {
    #[serde(default)]
    buff_weights: HashMap<String, f64>,
    target_score: f64,
    #[serde(default = "default_scorer_type")]
    scorer_type: String,
    #[serde(default)]
    main_buff_score: Option<f64>,
    #[serde(default)]
    normalized_max_score: Option<f64>,
    #[serde(default)]
    cost_weights: CostWeightsInput,
    exp_refund_ratio: Option<f64>,
    #[serde(default)]
    blend_data: bool,
    #[serde(default = "default_lambda_tolerance")]
    lambda_tolerance: f64,
    #[serde(default = "default_lambda_max_iter")]
    lambda_max_iter: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PolicySuggestionRequest {
    #[serde(default)]
    buff_names: Vec<String>,
    #[serde(default)]
    buff_values: Vec<u16>,
    total_score: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpgradeScorePreviewRequest {
    #[serde(default)]
    buff_weights: HashMap<String, f64>,
    #[serde(default = "default_scorer_type")]
    scorer_type: String,
    #[serde(default)]
    main_buff_score: Option<f64>,
    #[serde(default)]
    normalized_max_score: Option<f64>,
    #[serde(default)]
    buff_names: Vec<String>,
    #[serde(default)]
    buff_values: Vec<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComputeRerollPolicyRequest {
    #[serde(default)]
    buff_weights: HashMap<String, u16>,
    target_score: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueryRerollRecommendationRequest {
    #[serde(default)]
    baseline_buff_names: Vec<String>,
    #[serde(default)]
    candidate_buff_names: Vec<String>,
    #[serde(default = "default_reroll_top_k")]
    top_k: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapResponse {
    buff_types: Vec<String>,
    buff_labels: BTreeMap<String, String>,
    buff_type_max_values: Vec<f64>,
    buff_value_options: BTreeMap<String, Vec<u16>>,
    default_buff_weights: BTreeMap<String, f64>,
    default_linear_buff_weights: BTreeMap<String, f64>,
    default_mc_boost_assistant_buff_weights: BTreeMap<String, f64>,
    default_qq_bot_buff_weights: BTreeMap<String, f64>,
    default_fixed_buff_weights: BTreeMap<String, u16>,
    max_selected_types: usize,
    default_target_score: f64,
    default_fixed_target_score: u16,
    default_linear_main_buff_score: f64,
    default_linear_normalized_max_score: f64,
    default_qq_bot_main_buff_score: f64,
    default_qq_bot_normalized_max_score: f64,
    default_cost_weights: CostWeightsOutput,
    default_exp_refund_ratio: f64,
    default_scorer_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PolicySummary {
    target_score: f64,
    lambda_star: f64,
    expected_cost_per_success: f64,
    compute_seconds: f64,
    success_probability: f64,
    echo_per_success: f64,
    tuner_per_success: f64,
    exp_per_success: f64,
    cost_weights: CostWeightsOutput,
    exp_refund_ratio: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ComputePolicyResponse {
    summary: PolicySummary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PolicySuggestionResponse {
    suggestion: String,
    stage: usize,
    target_score: f64,
    success_probability: f64,
    mask_bits: Vec<u8>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpgradeScorePreviewResponse {
    contributions: Vec<f64>,
    main_contribution: f64,
    total_score: f64,
    max_score: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RerollChoiceResponse {
    lock_mask_bits: Vec<u8>,
    lock_slot_indices: Vec<usize>,
    expected_cost: f64,
    regret: f64,
    success_probability: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ComputeRerollPolicyResponse {
    target_score: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RerollRecommendationResponse {
    valid: bool,
    reason: Option<String>,
    baseline_score: u16,
    candidate_score: Option<u16>,
    recommended_lock_choices: Vec<RerollChoiceResponse>,
    accept_candidate: Option<bool>,
}

#[derive(Clone, Copy)]
enum UpgradeScorerConfig {
    LinearDefault {
        weights: [f64; NUM_BUFFS],
        main_buff_score: f64,
        normalized_max_score: f64,
    },
    McBoostAssistant {
        weights: [f64; NUM_BUFFS],
    },
    QQBot {
        qq_bot_weights: [f64; NUM_BUFFS],
        main_buff_score: f64,
        normalized_max_score: f64,
    },
    Fixed {
        weights: [u16; NUM_BUFFS],
    },
}

enum UpgradeScorer {
    Linear(LinearScorer),
    Fixed(FixedScorer),
}

struct SolverSession {
    solver: UpgradePolicySolver,
    target_score: f64,
    scorer_config: UpgradeScorerConfig,
    query_scorer: UpgradeScorer,
    blend_data: bool,
    cost_weights: CostWeightsOutput,
    exp_refund_ratio: f64,
}

struct RerollSession {
    solver: RerollPolicySolver,
    weights: [u16; NUM_BUFFS],
    scorer: FixedScorer,
}

struct AppState {
    current_upgrade: Mutex<Option<SolverSession>>,
    current_reroll: Mutex<Option<RerollSession>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            current_upgrade: Mutex::new(None),
            current_reroll: Mutex::new(None),
        }
    }
}

fn default_lambda_tolerance() -> f64 {
    1e-6
}

fn default_lambda_max_iter() -> usize {
    120
}

fn default_reroll_top_k() -> usize {
    3
}

fn default_cost_weights() -> CostWeightsOutput {
    CostWeightsOutput {
        w_echo: 0.0,
        w_tuner: 1.0,
        w_exp: 0.0,
    }
}

fn default_scorer_type() -> String {
    DEFAULT_SCORER_TYPE.to_string()
}

fn parse_scorer_type(raw: &str) -> Result<&'static str, String> {
    let lowered = raw.trim().to_ascii_lowercase();
    match lowered.as_str() {
        "linear" | SCORER_TYPE_LINEAR_DEFAULT => Ok(SCORER_TYPE_LINEAR_DEFAULT),
        SCORER_TYPE_MC_BOOST_ASSISTANT => Ok(SCORER_TYPE_MC_BOOST_ASSISTANT),
        SCORER_TYPE_QQ_BOT => Ok(SCORER_TYPE_QQ_BOT),
        SCORER_TYPE_FIXED => Ok(SCORER_TYPE_FIXED),
        _ => Err(format!(
            "Unsupported scorerType '{}'. Use 'linear_default', 'mc_boost_assistant', 'qq_bot', or 'fixed'.",
            raw
        )),
    }
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
        (UpgradeScorerConfig::Fixed { weights: lw }, UpgradeScorerConfig::Fixed { weights: rw }) => {
            lw == rw
        }
        _ => false,
    }
}

fn build_upgrade_scorer_config_from_inputs(
    scorer_type: &str,
    buff_weights: &HashMap<String, f64>,
    main_buff_score: Option<f64>,
    normalized_max_score: Option<f64>,
) -> Result<UpgradeScorerConfig, String> {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => {
            let weights = build_weight_array_f64(buff_weights, DEFAULT_LINEAR_BUFF_WEIGHTS)?;
            let main_buff_score = main_buff_score.unwrap_or(DEFAULT_LINEAR_MAIN_BUFF_SCORE);
            let normalized_max_score =
                normalized_max_score.unwrap_or(DEFAULT_LINEAR_NORMALIZED_MAX_SCORE);
            Ok(UpgradeScorerConfig::LinearDefault {
                weights,
                main_buff_score,
                normalized_max_score,
            })
        }
        SCORER_TYPE_MC_BOOST_ASSISTANT => {
            let weights =
                build_weight_array_f64(buff_weights, DEFAULT_MC_BOOST_ASSISTANT_BUFF_WEIGHTS)?;
            Ok(UpgradeScorerConfig::McBoostAssistant { weights })
        }
        SCORER_TYPE_QQ_BOT => {
            let qq_bot_weights = build_weight_array_f64(buff_weights, DEFAULT_QQ_BOT_BUFF_WEIGHTS)?;
            let main_buff_score = main_buff_score.unwrap_or(DEFAULT_QQ_BOT_MAIN_BUFF_SCORE);
            let normalized_max_score =
                normalized_max_score.unwrap_or(DEFAULT_QQ_BOT_NORMALIZED_MAX_SCORE);
            if !normalized_max_score.is_finite() || normalized_max_score <= 0.0 {
                return Err("normalizedMaxScore must be a positive finite number".to_string());
            }
            Ok(UpgradeScorerConfig::QQBot {
                qq_bot_weights,
                main_buff_score,
                normalized_max_score,
            })
        }
        SCORER_TYPE_FIXED => {
            let weights = build_weight_array_u16_from_f64(buff_weights, DEFAULT_FIXED_BUFF_WEIGHTS)?;
            Ok(UpgradeScorerConfig::Fixed { weights })
        }
        _ => unreachable!(),
    }
}

fn build_upgrade_scorer(config: &UpgradeScorerConfig) -> Result<UpgradeScorer, String> {
    match config {
        UpgradeScorerConfig::LinearDefault {
            weights,
            main_buff_score,
            normalized_max_score,
        } => Ok(UpgradeScorer::Linear(build_default_linear_scorer(
            *weights,
            *main_buff_score,
            *normalized_max_score,
        )?)),
        UpgradeScorerConfig::McBoostAssistant { weights } => Ok(UpgradeScorer::Linear(
            build_mc_boost_assistant_scorer(*weights)?,
        )),
        UpgradeScorerConfig::QQBot {
            qq_bot_weights,
            main_buff_score,
            ..
        } => Ok(UpgradeScorer::Linear(build_qq_bot_scorer(
            *qq_bot_weights,
            *main_buff_score,
        )?)),
        UpgradeScorerConfig::Fixed { weights } => {
            let scorer =
                FixedScorer::new(*weights).map_err(|err| format!("Invalid fixed scorer: {err:?}"))?;
            Ok(UpgradeScorer::Fixed(scorer))
        }
    }
}

fn build_upgrade_solver(
    scorer: &UpgradeScorer,
    blend_data: bool,
    target_score_display: f64,
    cost_model: CostModel,
) -> Result<UpgradePolicySolver, String> {
    match scorer {
        UpgradeScorer::Linear(linear) => UpgradePolicySolver::new(
            linear,
            blend_data,
            target_score_display,
            cost_model,
        )
        .map_err(|err| format!("Failed to create solver: {err:?}")),
        UpgradeScorer::Fixed(fixed) => UpgradePolicySolver::new(
            fixed,
            blend_data,
            target_score_display,
            cost_model,
        )
        .map_err(|err| format!("Failed to create solver: {err:?}")),
    }
}

fn resolve_target_scores(
    scorer_config: &UpgradeScorerConfig,
    scorer: &UpgradeScorer,
    raw_target_score: f64,
) -> Result<(f64, f64), String> {
    match scorer_config {
        UpgradeScorerConfig::Fixed { .. } => {
            let target_score = parse_u16_from_f64(raw_target_score, "targetScore")?;
            let summary_target = f64::from(target_score);
            Ok((summary_target, summary_target / SCORE_MULTIPLIER))
        }
        UpgradeScorerConfig::QQBot {
            normalized_max_score,
            ..
        } => {
            if !raw_target_score.is_finite() || raw_target_score < 0.0 {
                return Err("targetScore must be a non-negative finite number".to_string());
            }
            let score_scale = *normalized_max_score / DEFAULT_QQ_BOT_NORMALIZED_MAX_SCORE;
            let target_on_solver_scale = raw_target_score / score_scale;
            let main_score = match scorer {
                UpgradeScorer::Linear(linear) => linear.main_buff_score(),
                UpgradeScorer::Fixed(_) => unreachable!(),
            };
            Ok((raw_target_score, (target_on_solver_scale - main_score).max(0.0)))
        }
        UpgradeScorerConfig::LinearDefault { .. } | UpgradeScorerConfig::McBoostAssistant { .. } => {
            if !raw_target_score.is_finite() || raw_target_score < 0.0 {
                return Err("targetScore must be a non-negative finite number".to_string());
            }
            let main_score = match scorer {
                UpgradeScorer::Linear(linear) => linear.main_buff_score(),
                UpgradeScorer::Fixed(_) => unreachable!(),
            };
            Ok((raw_target_score, (raw_target_score - main_score).max(0.0)))
        }
    }
}

fn can_reuse_upgrade_solver(
    session: &SolverSession,
    scorer: &UpgradeScorerConfig,
    blend_data: bool,
    cost_weights: &CostWeightsOutput,
    exp_refund_ratio: f64,
) -> bool {
    scorer_configs_equal(&session.scorer_config, scorer)
        && session.blend_data == blend_data
        && cost_weights_equal(&session.cost_weights, cost_weights)
        && f64_bits_equal(session.exp_refund_ratio, exp_refund_ratio)
}

fn buff_index(buff_name: &str) -> Option<usize> {
    BUFF_TYPES.iter().position(|name| *name == buff_name)
}

fn parse_u16_from_f64(value: f64, field: &str) -> Result<u16, String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{field} must be a non-negative finite number"));
    }
    if value > u16::MAX as f64 {
        return Err(format!("{field} must be <= {}", u16::MAX));
    }
    if value.fract().abs() > f64::EPSILON {
        return Err(format!("{field} must be an integer"));
    }
    Ok(value as u16)
}

fn build_default_weight_map_f64(weights: &[f64; NUM_BUFFS]) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    for (index, buff_name) in BUFF_TYPES.iter().enumerate() {
        out.insert((*buff_name).to_string(), weights[index]);
    }
    out
}

fn build_default_weight_map_u16(weights: &[u16; NUM_BUFFS]) -> BTreeMap<String, u16> {
    let mut out = BTreeMap::new();
    for (index, buff_name) in BUFF_TYPES.iter().enumerate() {
        out.insert((*buff_name).to_string(), weights[index]);
    }
    out
}

fn build_weight_array_f64(
    input: &HashMap<String, f64>,
    defaults: [f64; NUM_BUFFS],
) -> Result<[f64; NUM_BUFFS], String> {
    let mut weights = defaults;

    for (buff_name, value) in input {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in weights: {buff_name}"))?;
        if !value.is_finite() || *value < 0.0 {
            return Err(format!("Invalid weight for {buff_name}: {value}"));
        }
        weights[index] = *value;
    }

    Ok(weights)
}

fn build_weight_array_u16(
    input: &HashMap<String, u16>,
    defaults: [u16; NUM_BUFFS],
) -> Result<[u16; NUM_BUFFS], String> {
    let mut weights = defaults;

    for (buff_name, value) in input {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in weights: {buff_name}"))?;
        weights[index] = *value;
    }

    Ok(weights)
}

fn build_weight_array_u16_from_f64(
    input: &HashMap<String, f64>,
    defaults: [u16; NUM_BUFFS],
) -> Result<[u16; NUM_BUFFS], String> {
    let mut weights = defaults;

    for (buff_name, value) in input {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in weights: {buff_name}"))?;
        weights[index] = parse_u16_from_f64(*value, &format!("weight[{buff_name}]"))?;
    }

    Ok(weights)
}

fn build_mask(buff_names: &[String]) -> Result<u16, String> {
    if buff_names.len() > MAX_SELECTED_TYPES {
        return Err(format!(
            "Too many selected buffs: {}, max is {MAX_SELECTED_TYPES}",
            buff_names.len()
        ));
    }

    let mut bits = [0u8; NUM_BUFFS];
    for buff_name in buff_names {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in selection: {buff_name}"))?;
        if bits[index] == 1 {
            return Err(format!("Duplicate buff in selection: {buff_name}"));
        }
        bits[index] = 1;
    }

    Ok(bits_to_mask(&bits))
}

fn build_full_mask(buff_names: &[String]) -> Result<u16, String> {
    if buff_names.len() != MAX_SELECTED_TYPES {
        return Err(format!(
            "Exactly {MAX_SELECTED_TYPES} buff types are required, got {}",
            buff_names.len()
        ));
    }
    let mask = build_mask(buff_names)?;
    if mask.count_ones() as usize != MAX_SELECTED_TYPES {
        return Err("Buff selections must be unique and fully filled".to_string());
    }
    Ok(mask)
}

fn fixed_score_from_selected(scorer: &FixedScorer, buff_names: &[String]) -> Result<u16, String> {
    let zero_values = vec![0u16; buff_names.len()];
    let indexed = build_indexed_echo(buff_names, &zero_values)?;
    scorer
        .echo_score_display(&indexed)
        .map_err(|err| format!("Failed to compute fixed display score: {err:?}"))
}

fn lock_slot_indices_from_mask(lock_mask: u16, baseline_buff_names: &[String]) -> Vec<usize> {
    let mut slots = Vec::new();
    for (slot_idx, buff_name) in baseline_buff_names.iter().enumerate() {
        if let Some(buff_idx) = buff_index(buff_name) {
            if (lock_mask & (1u16 << buff_idx)) != 0 {
                slots.push(slot_idx + 1);
            }
        }
    }
    slots
}

fn build_default_linear_scorer(
    weights: [f64; NUM_BUFFS],
    main_buff_score: f64,
    normalized_max_score: f64,
) -> Result<LinearScorer, String> {
    if (main_buff_score - DEFAULT_LINEAR_MAIN_BUFF_SCORE).abs() <= 1e-12
        && (normalized_max_score - DEFAULT_LINEAR_NORMALIZED_MAX_SCORE).abs() <= 1e-12
    {
        LinearScorer::default(weights).map_err(|err| format!("Invalid linear scorer: {err:?}"))
    } else {
        LinearScorer::new(weights, main_buff_score, normalized_max_score)
            .map_err(|err| format!("Invalid linear scorer: {err:?}"))
    }
}

fn build_qq_bot_scorer(
    qq_bot_weights: [f64; NUM_BUFFS],
    main_buff_score: f64,
) -> Result<LinearScorer, String> {
    LinearScorer::qq_bot_scorer(qq_bot_weights, main_buff_score)
        .map_err(|err| format!("Invalid QQ Bot scorer: {err:?}"))
}

fn build_mc_boost_assistant_scorer(weights: [f64; NUM_BUFFS]) -> Result<LinearScorer, String> {
    LinearScorer::mc_boost_assistant_scorer(weights)
        .map_err(|err| format!("Invalid MC Boost Assistant scorer: {err:?}"))
}

fn build_indexed_echo(
    buff_names: &[String],
    buff_values: &[u16],
) -> Result<Vec<(usize, u16)>, String> {
    if buff_names.len() != buff_values.len() {
        return Err("buffNames and buffValues length mismatch".to_string());
    }

    let mut indexed = Vec::with_capacity(buff_names.len());
    for (buff_name, &buff_value) in buff_names.iter().zip(buff_values.iter()) {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in selection: {buff_name}"))?;
        indexed.push((index, buff_value));
    }
    Ok(indexed)
}

fn score_from_selected_buffs_for_solver(
    scorer: &UpgradeScorer,
    buff_names: &[String],
    buff_values: &[u16],
) -> Result<u16, String> {
    if buff_names.is_empty() {
        return Ok(0);
    }

    let indexed = build_indexed_echo(buff_names, buff_values)?;
    match scorer {
        UpgradeScorer::Linear(linear) => linear
            .echo_score_internal(&indexed)
            .map_err(|err| format!("Failed to compute internal score: {err:?}")),
        UpgradeScorer::Fixed(fixed) => fixed
            .echo_score_internal(&indexed)
            .map_err(|err| format!("Failed to compute internal score: {err:?}")),
    }
}

#[tauri::command]
fn preview_upgrade_score(
    payload: UpgradeScorePreviewRequest,
) -> Result<UpgradeScorePreviewResponse, String> {
    let scorer_type = parse_scorer_type(&payload.scorer_type)?;
    let scorer_config = build_upgrade_scorer_config_from_inputs(
        scorer_type,
        &payload.buff_weights,
        payload.main_buff_score,
        payload.normalized_max_score,
    )?;
    let scorer = build_upgrade_scorer(&scorer_config)?;

    let indexed = build_indexed_echo(&payload.buff_names, &payload.buff_values)?;
    match scorer {
        UpgradeScorer::Linear(linear) => {
            let contributions = indexed
                .iter()
                .map(|&(buff_index, buff_value)| linear.buff_score_display(buff_index, buff_value))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| format!("Failed to compute display contribution: {err:?}"))?;
            let total_score = linear
                .echo_score_display(&indexed)
                .map_err(|err| format!("Failed to compute display score: {err:?}"))?;
            Ok(UpgradeScorePreviewResponse {
                contributions,
                main_contribution: linear.main_buff_score(),
                total_score,
                max_score: linear.normalized_max_score(),
            })
        }
        UpgradeScorer::Fixed(fixed) => {
            let contributions = indexed
                .iter()
                .map(|&(buff_index, buff_value)| fixed.buff_score_display(buff_index, buff_value))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| format!("Failed to compute display contribution: {err:?}"))?
                .into_iter()
                .map(f64::from)
                .collect();
            let total_score = f64::from(
                fixed
                    .echo_score_display(&indexed)
                    .map_err(|err| format!("Failed to compute display score: {err:?}"))?,
            );
            Ok(UpgradeScorePreviewResponse {
                contributions,
                main_contribution: 0.0,
                total_score,
                max_score: f64::from(fixed.max_score()),
            })
        }
    }
}

#[tauri::command]
fn bootstrap() -> BootstrapResponse {
    let mut buff_labels = BTreeMap::new();
    let mut value_options = BTreeMap::new();

    for (index, buff_name) in BUFF_TYPES.iter().enumerate() {
        buff_labels.insert((*buff_name).to_string(), BUFF_LABELS[index].to_string());
        value_options.insert((*buff_name).to_string(), BUFF_VALUE_OPTIONS[index].to_vec());
    }

    BootstrapResponse {
        buff_types: BUFF_TYPES.iter().map(|name| (*name).to_string()).collect(),
        buff_labels,
        buff_type_max_values: BUFF_TYPE_MAX_VALUES.to_vec(),
        buff_value_options: value_options,
        default_buff_weights: build_default_weight_map_f64(&DEFAULT_LINEAR_BUFF_WEIGHTS),
        default_linear_buff_weights: build_default_weight_map_f64(&DEFAULT_LINEAR_BUFF_WEIGHTS),
        default_mc_boost_assistant_buff_weights: build_default_weight_map_f64(
            &DEFAULT_MC_BOOST_ASSISTANT_BUFF_WEIGHTS,
        ),
        default_qq_bot_buff_weights: build_default_weight_map_f64(&DEFAULT_QQ_BOT_BUFF_WEIGHTS),
        default_fixed_buff_weights: build_default_weight_map_u16(&DEFAULT_FIXED_BUFF_WEIGHTS),
        max_selected_types: MAX_SELECTED_TYPES,
        default_target_score: DEFAULT_TARGET_SCORE,
        default_fixed_target_score: DEFAULT_FIXED_TARGET_SCORE,
        default_linear_main_buff_score: DEFAULT_LINEAR_MAIN_BUFF_SCORE,
        default_linear_normalized_max_score: DEFAULT_LINEAR_NORMALIZED_MAX_SCORE,
        default_qq_bot_main_buff_score: DEFAULT_QQ_BOT_MAIN_BUFF_SCORE,
        default_qq_bot_normalized_max_score: DEFAULT_QQ_BOT_NORMALIZED_MAX_SCORE,
        default_cost_weights: default_cost_weights(),
        default_exp_refund_ratio: DEFAULT_EXP_REFUND_RATIO,
        default_scorer_type: DEFAULT_SCORER_TYPE.to_string(),
    }
}

#[tauri::command]
fn compute_policy(
    state: State<'_, AppState>,
    payload: ComputePolicyRequest,
) -> Result<ComputePolicyResponse, String> {
    if payload.lambda_tolerance <= 0.0 || !payload.lambda_tolerance.is_finite() {
        return Err("lambdaTolerance must be a positive finite number".to_string());
    }
    if payload.lambda_max_iter == 0 {
        return Err("lambdaMaxIter must be greater than 0".to_string());
    }

    let exp_refund_ratio = payload.exp_refund_ratio.unwrap_or(DEFAULT_EXP_REFUND_RATIO);
    let cost_weights = CostWeightsOutput {
        w_echo: payload.cost_weights.w_echo,
        w_tuner: payload.cost_weights.w_tuner,
        w_exp: payload.cost_weights.w_exp,
    };

    let cost_model = CostModel::new(
        cost_weights.w_echo,
        cost_weights.w_tuner,
        cost_weights.w_exp,
        exp_refund_ratio,
    )
    .map_err(|err| format!("Invalid cost model: {err:?}"))?;
    let scorer_type = parse_scorer_type(&payload.scorer_type)?;
    let scorer_config = build_upgrade_scorer_config_from_inputs(
        scorer_type,
        &payload.buff_weights,
        payload.main_buff_score,
        payload.normalized_max_score,
    )?;
    let scorer = build_upgrade_scorer(&scorer_config)?;
    let (summary_target_score, solver_target_score) =
        resolve_target_scores(&scorer_config, &scorer, payload.target_score)?;

    let mut current_upgrade = state
        .current_upgrade
        .lock()
        .map_err(|_| "Failed to lock current upgrade solver".to_string())?;

    let reuse_existing = current_upgrade.as_ref().is_some_and(|session| {
        can_reuse_upgrade_solver(
            session,
            &scorer_config,
            payload.blend_data,
            &cost_weights,
            exp_refund_ratio,
        )
    });

    if reuse_existing {
        let session = current_upgrade.as_mut().expect("checked above");
        session
            .solver
            .update_target_score(solver_target_score)
            .map_err(|err| format!("Failed to update target score: {err:?}"))?;
        session.target_score = summary_target_score;
    } else {
        let solver =
            build_upgrade_solver(&scorer, payload.blend_data, solver_target_score, cost_model)?;
        *current_upgrade = Some(SolverSession {
            solver,
            target_score: summary_target_score,
            scorer_config,
            query_scorer: scorer,
            blend_data: payload.blend_data,
            cost_weights,
            exp_refund_ratio,
        });
    }

    let session = current_upgrade.as_mut().expect("session is initialized");
    let start = Instant::now();
    let lambda_star = session
        .solver
        .lambda_search(payload.lambda_tolerance, payload.lambda_max_iter)
        .map_err(|err| format!("Failed during lambda search: {err:?}"))?;
    let expected = session
        .solver
        .calculate_expected_resources()
        .map_err(|err| format!("Failed to compute expected resources: {err:?}"))?;
    let expected_cost_per_success = session
        .solver
        .weighted_expected_cost()
        .map_err(|err| format!("Failed to compute weighted expected cost: {err:?}"))?;
    let compute_seconds = start.elapsed().as_secs_f64();

    let summary = PolicySummary {
        target_score: summary_target_score,
        lambda_star,
        expected_cost_per_success,
        compute_seconds,
        success_probability: expected.success_probability(),
        echo_per_success: expected.echo_per_success(),
        tuner_per_success: expected.tuner_per_success(),
        exp_per_success: expected.exp_per_success(),
        cost_weights,
        exp_refund_ratio,
    };

    Ok(ComputePolicyResponse { summary })
}

#[tauri::command]
fn policy_suggestion(
    state: State<'_, AppState>,
    payload: PolicySuggestionRequest,
) -> Result<PolicySuggestionResponse, String> {
    if !payload.total_score.is_finite() || payload.total_score < 0.0 {
        return Err("totalScore must be a non-negative finite number".to_string());
    }
    if !payload.buff_names.is_empty() && payload.buff_values.len() != payload.buff_names.len() {
        return Err("buffNames and buffValues must have the same length".to_string());
    }

    let current_upgrade = state
        .current_upgrade
        .lock()
        .map_err(|_| "Failed to lock current upgrade solver".to_string())?;
    let session = current_upgrade.as_ref().ok_or_else(|| {
        "No computed upgrade policy in memory. Please compute policy first.".to_string()
    })?;

    let mask = build_mask(&payload.buff_names)?;
    let score_scaled = if !payload.buff_names.is_empty() {
        score_from_selected_buffs_for_solver(
            &session.query_scorer,
            &payload.buff_names,
            &payload.buff_values,
        )?
    } else {
        0
    };

    let decision = if payload.buff_names.is_empty() {
        true
    } else {
        session
            .solver
            .get_decision(mask, score_scaled)
            .map_err(|err| format!("Failed to query suggestion: {err:?}"))?
    };
    let success_probability = session
        .solver
        .get_success_probability(mask, score_scaled)
        .map_err(|err| format!("Failed to query success probability: {err:?}"))?;

    Ok(PolicySuggestionResponse {
        suggestion: if decision {
            "Continue".to_string()
        } else {
            "Abandon".to_string()
        },
        stage: payload.buff_names.len(),
        target_score: session.target_score,
        success_probability,
        mask_bits: mask_to_bits(mask).to_vec(),
    })
}

#[tauri::command]
fn compute_reroll_policy(
    state: State<'_, AppState>,
    payload: ComputeRerollPolicyRequest,
) -> Result<ComputeRerollPolicyResponse, String> {
    let weights = build_weight_array_u16(&payload.buff_weights, DEFAULT_FIXED_BUFF_WEIGHTS)?;

    let mut current_reroll = state
        .current_reroll
        .lock()
        .map_err(|_| "Failed to lock current reroll solver".to_string())?;

    let reuse_existing = current_reroll
        .as_ref()
        .is_some_and(|session| session.weights == weights);

    if reuse_existing {
        let session = current_reroll.as_mut().expect("checked above");
        session
            .solver
            .set_target(payload.target_score)
            .map_err(|err| format!("Failed to set reroll target: {err:?}"))?;
        session
            .solver
            .derive_policy(1e-4, 200)
            .map_err(|err| format!("Failed to derive reroll policy: {err:?}"))?;
    } else {
        let mut solver = RerollPolicySolver::new(weights)
            .map_err(|err| format!("Failed to create reroll solver: {err:?}"))?;
        solver
            .set_target(payload.target_score)
            .map_err(|err| format!("Failed to set reroll target: {err:?}"))?;
        solver
            .derive_policy(1e-4, 200)
            .map_err(|err| format!("Failed to derive reroll policy: {err:?}"))?;
        let scorer = FixedScorer::new(weights)
            .map_err(|err| format!("Invalid fixed scorer: {err:?}"))?;
        *current_reroll = Some(RerollSession {
            solver,
            weights,
            scorer,
        });
    }

    Ok(ComputeRerollPolicyResponse {
        target_score: payload.target_score,
    })
}

#[tauri::command]
fn query_reroll_recommendation(
    state: State<'_, AppState>,
    payload: QueryRerollRecommendationRequest,
) -> Result<RerollRecommendationResponse, String> {
    let current_reroll = state
        .current_reroll
        .lock()
        .map_err(|_| "Failed to lock current reroll solver".to_string())?;
    let session = current_reroll.as_ref().ok_or_else(|| {
        "No computed reroll policy in memory. Please compute reroll policy first.".to_string()
    })?;

    let baseline_filled = payload.baseline_buff_names.len() == MAX_SELECTED_TYPES
        && payload
            .baseline_buff_names
            .iter()
            .all(|name| !name.is_empty());
    let candidate_filled = payload.candidate_buff_names.len() == MAX_SELECTED_TYPES
        && payload
            .candidate_buff_names
            .iter()
            .all(|name| !name.is_empty());

    if !baseline_filled {
        return Ok(RerollRecommendationResponse {
            valid: false,
            reason: Some("Baseline must have 5 buff types.".to_string()),
            baseline_score: 0,
            candidate_score: None,
            recommended_lock_choices: Vec::new(),
            accept_candidate: None,
        });
    }

    let baseline_mask = build_full_mask(&payload.baseline_buff_names)?;
    let baseline_score = fixed_score_from_selected(&session.scorer, &payload.baseline_buff_names)?;

    let default_top_k = default_reroll_top_k();
    let top_k = if payload.top_k == 0 {
        default_top_k
    } else {
        payload.top_k.min(default_top_k)
    };
    let choices = session
        .solver
        .lock_choices(baseline_mask, top_k)
        .map_err(|err| format!("Failed to query lock choices: {err:?}"))?;
    let recommended_lock_choices = choices
        .into_iter()
        .map(|choice| RerollChoiceResponse {
            lock_mask_bits: mask_to_bits(choice.lock_mask).to_vec(),
            lock_slot_indices: lock_slot_indices_from_mask(
                choice.lock_mask,
                &payload.baseline_buff_names,
            ),
            expected_cost: choice.expected_cost,
            regret: choice.regret,
            success_probability: choice.success_probability,
        })
        .collect();

    let (candidate_score, accept_candidate) = if candidate_filled {
        let candidate_mask = build_full_mask(&payload.candidate_buff_names)?;
        let score = fixed_score_from_selected(&session.scorer, &payload.candidate_buff_names)?;
        let accept = session
            .solver
            .should_accept(baseline_mask, candidate_mask)
            .map_err(|err| format!("Failed to compare baseline and candidate: {err:?}"))?;
        (Some(score), Some(accept))
    } else {
        (None, None)
    };

    Ok(RerollRecommendationResponse {
        valid: true,
        reason: None,
        baseline_score,
        candidate_score,
        recommended_lock_choices,
        accept_candidate,
    })
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            preview_upgrade_score,
            compute_policy,
            policy_suggestion,
            compute_reroll_policy,
            query_reroll_recommendation
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
