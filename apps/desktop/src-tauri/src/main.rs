#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tauri::State;
use echo_policy::{
    bits_to_mask, mask_to_bits, CostModel, FixedScorer, LinearScorer, RerollPolicySolver, Scorer,
    UpgradePolicySolver, SCORE_MULTIPLIER,
};

const NUM_BUFFS: usize = 13;
const MAX_SELECTED_TYPES: usize = 5;
const DEFAULT_TARGET_SCORE: f64 = 60.0;
const DEFAULT_EXP_REFUND_RATIO: f64 = 0.66;
const DEFAULT_SCORER_TYPE: &str = "linear";

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

const DEFAULT_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    100.0, 100.0, 70.0, 0.0, 0.0, 30.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0,
];

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
    buff_values: Vec<f64>,
    total_score: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComputeRerollPolicyRequest {
    #[serde(default)]
    buff_weights: HashMap<String, f64>,
    target_score: f64,
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
    max_selected_types: usize,
    default_target_score: f64,
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
    target_score: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RerollRecommendationResponse {
    valid: bool,
    reason: Option<String>,
    baseline_score: f64,
    candidate_score: Option<f64>,
    recommended_lock_choices: Vec<RerollChoiceResponse>,
    accept_candidate: Option<bool>,
}

struct SolverSession {
    solver: UpgradePolicySolver,
    target_score: f64,
    weights: [f64; NUM_BUFFS],
    scorer_type: String,
}

struct RerollSession {
    solver: RerollPolicySolver,
    weights: [f64; NUM_BUFFS],
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

fn buff_index(buff_name: &str) -> Option<usize> {
    BUFF_TYPES.iter().position(|name| *name == buff_name)
}

fn build_default_weight_map() -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    for (index, buff_name) in BUFF_TYPES.iter().enumerate() {
        out.insert((*buff_name).to_string(), DEFAULT_BUFF_WEIGHTS[index]);
    }
    out
}

fn build_weight_array(input: &HashMap<String, f64>) -> Result<[f64; NUM_BUFFS], String> {
    let mut weights = DEFAULT_BUFF_WEIGHTS;

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

fn fixed_score_from_selected(
    weights: &[f64; NUM_BUFFS],
    buff_names: &[String],
) -> Result<f64, String> {
    let mut total = 0.0;
    for buff_name in buff_names {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in selection: {buff_name}"))?;
        total += weights[index];
    }
    Ok(total)
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

fn score_from_selected_buffs(
    scorer_type: &str,
    weights: &[f64; NUM_BUFFS],
    buff_names: &[String],
    buff_values: &[f64],
) -> Result<u16, String> {
    if buff_names.len() != buff_values.len() {
        return Err("buffNames and buffValues length mismatch".to_string());
    }

    let mut sum: u32 = 0;
    match scorer_type {
        "linear" => {
            let scorer = LinearScorer::new(*weights)
                .map_err(|err| format!("Invalid linear scorer: {err:?}"))?;
            for (name, value) in buff_names.iter().zip(buff_values.iter()) {
                let index = buff_index(name)
                    .ok_or_else(|| format!("Unknown buff name in selection: {name}"))?;
                let per_buff = (scorer.buff_score(index, *value) * SCORE_MULTIPLIER).round();
                if !per_buff.is_finite() || per_buff < 0.0 {
                    return Err(format!("Invalid per-buff score for {name}"));
                }
                sum = sum.saturating_add(per_buff as u32);
            }
        }
        "fixed" => {
            let scorer = FixedScorer::new(*weights)
                .map_err(|err| format!("Invalid fixed scorer: {err:?}"))?;
            for (name, value) in buff_names.iter().zip(buff_values.iter()) {
                let index = buff_index(name)
                    .ok_or_else(|| format!("Unknown buff name in selection: {name}"))?;
                let per_buff = (scorer.buff_score(index, *value) * SCORE_MULTIPLIER).round();
                if !per_buff.is_finite() || per_buff < 0.0 {
                    return Err(format!("Invalid per-buff score for {name}"));
                }
                sum = sum.saturating_add(per_buff as u32);
            }
        }
        _ => return Err(format!("Unsupported scorer type in session: {scorer_type}")),
    }

    if sum > u16::MAX as u32 {
        return Err("Computed score exceeds u16 range".to_string());
    }

    Ok(sum as u16)
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
        default_buff_weights: build_default_weight_map(),
        max_selected_types: MAX_SELECTED_TYPES,
        default_target_score: DEFAULT_TARGET_SCORE,
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
    if !payload.target_score.is_finite() || payload.target_score < 0.0 {
        return Err("targetScore must be a non-negative finite number".to_string());
    }
    if payload.lambda_tolerance <= 0.0 || !payload.lambda_tolerance.is_finite() {
        return Err("lambdaTolerance must be a positive finite number".to_string());
    }
    if payload.lambda_max_iter == 0 {
        return Err("lambdaMaxIter must be greater than 0".to_string());
    }

    let weights = build_weight_array(&payload.buff_weights)?;

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

    let scorer_type = payload.scorer_type.trim().to_ascii_lowercase();
    let mut solver = match scorer_type.as_str() {
        "linear" => {
            let scorer = LinearScorer::new(weights)
                .map_err(|err| format!("Invalid linear scorer: {err:?}"))?;
            UpgradePolicySolver::new(
                &scorer,
                payload.blend_data,
                payload.target_score,
                cost_model,
            )
            .map_err(|err| format!("Failed to create solver: {err:?}"))?
        }
        "fixed" => {
            let scorer = FixedScorer::new(weights)
                .map_err(|err| format!("Invalid fixed scorer: {err:?}"))?;
            UpgradePolicySolver::new(
                &scorer,
                payload.blend_data,
                payload.target_score,
                cost_model,
            )
            .map_err(|err| format!("Failed to create solver: {err:?}"))?
        }
        _ => {
            return Err(format!(
                "Unsupported scorerType '{}'. Use 'linear' or 'fixed'.",
                payload.scorer_type
            ));
        }
    };

    let start = Instant::now();
    let lambda_star = solver
        .lambda_search(payload.lambda_tolerance, payload.lambda_max_iter)
        .map_err(|err| format!("Failed during lambda search: {err:?}"))?;
    let expected = solver
        .calculate_expected_resources()
        .map_err(|err| format!("Failed to compute expected resources: {err:?}"))?;
    let expected_cost_per_success = solver
        .weighted_expected_cost()
        .map_err(|err| format!("Failed to compute weighted expected cost: {err:?}"))?;
    let compute_seconds = start.elapsed().as_secs_f64();

    let summary = PolicySummary {
        target_score: payload.target_score,
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

    let mut current_upgrade = state
        .current_upgrade
        .lock()
        .map_err(|_| "Failed to lock current upgrade solver".to_string())?;
    *current_upgrade = Some(SolverSession {
        solver,
        target_score: payload.target_score,
        weights,
        scorer_type,
    });

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
        score_from_selected_buffs(
            &session.scorer_type,
            &session.weights,
            &payload.buff_names,
            &payload.buff_values,
        )?
    } else {
        (payload.total_score * SCORE_MULTIPLIER)
            .round()
            .clamp(0.0, u16::MAX as f64) as u16
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
    if !payload.target_score.is_finite() || payload.target_score < 0.0 {
        return Err("targetScore must be a non-negative finite number".to_string());
    }

    let weights = build_weight_array(&payload.buff_weights)?;

    let mut solver = RerollPolicySolver::new(weights)
        .map_err(|err| format!("Failed to create reroll solver: {err:?}"))?;
    solver
        .set_target(payload.target_score)
        .map_err(|err| format!("Failed to set reroll target: {err:?}"))?;
    solver
        .derive_policy(1e-4, 200)
        .map_err(|err| format!("Failed to derive reroll policy: {err:?}"))?;

    let mut current_reroll = state
        .current_reroll
        .lock()
        .map_err(|_| "Failed to lock current reroll solver".to_string())?;
    *current_reroll = Some(RerollSession { solver, weights });

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
            baseline_score: 0.0,
            candidate_score: None,
            recommended_lock_choices: Vec::new(),
            accept_candidate: None,
        });
    }

    let baseline_mask = build_full_mask(&payload.baseline_buff_names)?;
    let baseline_score = fixed_score_from_selected(&session.weights, &payload.baseline_buff_names)?;

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
        let score = fixed_score_from_selected(&session.weights, &payload.candidate_buff_names)?;
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
            compute_policy,
            policy_suggestion,
            compute_reroll_policy,
            query_reroll_recommendation
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
