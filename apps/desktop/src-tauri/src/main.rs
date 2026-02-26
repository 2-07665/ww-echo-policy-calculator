#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use echo_policy::{
    CostModel, FixedScorer, InternalScorer, LinearScorer, RerollPolicySolver, SCORE_MULTIPLIER,
    UpgradePolicySolver, bits_to_mask, mask_to_bits,
};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};

const NUM_BUFFS: usize = 13;
const MAX_SELECTED_TYPES: usize = 5;
const DEFAULT_TARGET_SCORE: f64 = 60.0;
const DEFAULT_FIXED_TARGET_SCORE: u16 = 7;
const DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE: f64 = 95.0;
const DEFAULT_QQ_BOT_TARGET_SCORE: f64 = 35.0;
const DEFAULT_EXP_REFUND_RATIO: f64 = 0.66;
const DEFAULT_SCORER_TYPE: &str = "linear_default";

const SCORER_TYPE_LINEAR_DEFAULT: &str = "linear_default";
const SCORER_TYPE_WUWA_ECHO_TOOL: &str = "wuwa_echo_tool";
const SCORER_TYPE_MC_BOOST_ASSISTANT: &str = "mc_boost_assistant";
const SCORER_TYPE_QQ_BOT: &str = "qq_bot";
const SCORER_TYPE_FIXED: &str = "fixed";
const SCORER_PRESET_DIR: &str = "scorer-presets";
const SCORER_PRESET_NAME_CUSTOM: &str = "自定义";
const SCORER_PRESET_VARIANT_NAME_DEFAULT: &str = "默认";
const DEFAULT_LINEAR_PRESETS_JSON: &str = include_str!("../default-presets/linear_default.json");
const DEFAULT_WUWA_ECHO_TOOL_PRESETS_JSON: &str =
    include_str!("../default-presets/wuwa_echo_tool.json");
const DEFAULT_MC_BOOST_ASSISTANT_PRESETS_JSON: &str =
    include_str!("../default-presets/mc_boost_assistant.json");
const DEFAULT_QQ_BOT_PRESETS_JSON: &str = include_str!("../default-presets/qq_bot.json");
const DEFAULT_FIXED_PRESETS_JSON: &str = include_str!("../default-presets/fixed.json");

const DEFAULT_LINEAR_MAIN_BUFF_SCORE: f64 = 0.0;
const DEFAULT_LINEAR_NORMALIZED_MAX_SCORE: f64 = 100.0;
const DEFAULT_WUWA_ECHO_TOOL_MAIN_BUFF_SCORE: f64 = 0.0;
const DEFAULT_WUWA_ECHO_TOOL_NORMALIZED_MAX_SCORE: f64 = 100.0;
const DEFAULT_WUWA_ECHO_TOOL_TARGET_SCORE: f64 = 60.0;
const DEFAULT_QQ_BOT_MAIN_BUFF_SCORE: f64 = 0.0;
const DEFAULT_QQ_BOT_NORMALIZED_MAX_SCORE: f64 = 50.0;
const MIN_NORMALIZED_MAX_SCORE: f64 = 0.01;
const DEFAULT_OCR_UDP_PORT: u16 = 9999;
const OCR_UDP_EVENT_FILL_ENTRIES: &str = "ocr_udp_fill_entries";
const OCR_UDP_EVENT_LISTENER_STATUS: &str = "ocr_udp_listener_status";
const OCR_UDP_PACKET_BUFFER_SIZE: usize = 16 * 1024;
const OCR_UDP_READ_TIMEOUT_MS: u64 = 300;

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
    100.0, 100.0, 70.0, 0.0, 0.0, 36.0, 0.0, 0.0, 40.0, 0.0, 0.0, 0.0, 0.0,
];

const DEFAULT_WUWA_ECHO_TOOL_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    100.0, 100.0, 70.0, 0.0, 0.0, 36.0, 0.0, 0.0, 40.0, 0.0, 0.0, 0.0, 0.0,
];

const DEFAULT_MC_BOOST_ASSISTANT_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    1.0, 1.0, 0.7, 0.0, 0.0, 0.36, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0,
];

const DEFAULT_QQ_BOT_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    2.0, 1.0, 1.1, 0.0, 0.0, 0.1, 0.0, 0.0, 0.2, 0.0, 0.0, 0.0, 0.0,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartOcrUdpListenerRequest {
    port: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OcrUdpPayload {
    buff_entries: Vec<OcrUdpBuffEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OcrUdpBuffEntry {
    buff_name: String,
    buff_value: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadScorerPresetsRequest {
    #[serde(default = "default_scorer_type")]
    scorer_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveScorerPresetRequest {
    #[serde(default = "default_scorer_type")]
    scorer_type: String,
    preset_name: String,
    #[serde(default)]
    variant_name: Option<String>,
    #[serde(default)]
    weights: HashMap<String, f64>,
    #[serde(default)]
    main_buff_score: Option<f64>,
    #[serde(default)]
    normalized_max_score: Option<f64>,
    #[serde(default)]
    preset_intro: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveScorerPresetVariantRequest {
    #[serde(default = "default_scorer_type")]
    scorer_type: String,
    preset_name: String,
    variant_name: String,
    #[serde(default)]
    weights: HashMap<String, f64>,
    #[serde(default)]
    main_buff_score: Option<f64>,
    #[serde(default)]
    normalized_max_score: Option<f64>,
    #[serde(default)]
    preset_intro: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteScorerPresetRequest {
    #[serde(default = "default_scorer_type")]
    scorer_type: String,
    preset_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteScorerPresetVariantRequest {
    #[serde(default = "default_scorer_type")]
    scorer_type: String,
    preset_name: String,
    variant_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScorerPresetResponseVariantItem {
    variant_name: String,
    weights: BTreeMap<String, f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    main_buff_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    normalized_max_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preset_intro: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScorerPresetResponseItem {
    preset_name: String,
    variants: Vec<ScorerPresetResponseVariantItem>,
    built_in: bool,
    user_defined: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadScorerPresetsResponse {
    presets: Vec<ScorerPresetResponseItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveScorerPresetResponse {
    saved_preset_name: String,
    saved_variant_name: String,
    presets: Vec<ScorerPresetResponseItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteScorerPresetResponse {
    deleted_preset_name: String,
    presets: Vec<ScorerPresetResponseItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveScorerPresetVariantResponse {
    saved_preset_name: String,
    saved_variant_name: String,
    presets: Vec<ScorerPresetResponseItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteScorerPresetVariantResponse {
    deleted_preset_name: String,
    deleted_variant_name: String,
    presets: Vec<ScorerPresetResponseItem>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ScorerPresetFile {
    #[serde(default)]
    presets: Vec<ScorerPresetFileItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ScorerPresetFileItem {
    preset_name: String,
    #[serde(default)]
    variants: Vec<ScorerPresetVariantFileItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ScorerPresetVariantFileItem {
    variant_name: String,
    #[serde(default)]
    weights: BTreeMap<String, f64>,
    #[serde(default)]
    main_buff_score: Option<f64>,
    #[serde(default)]
    normalized_max_score: Option<f64>,
    #[serde(default)]
    preset_intro: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ScorerPresetRawFile {
    #[serde(default)]
    presets: Vec<ScorerPresetRawItem>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ScorerPresetRawItem {
    Grouped(ScorerPresetFileItem),
    Legacy(ScorerPresetLegacyFileItem),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScorerPresetLegacyFileItem {
    preset_name: String,
    #[serde(default)]
    weights: BTreeMap<String, f64>,
    #[serde(default)]
    main_buff_score: Option<f64>,
    #[serde(default)]
    normalized_max_score: Option<f64>,
    #[serde(default)]
    preset_intro: Option<String>,
}

#[derive(Debug, Clone)]
struct ScorerPresetResolvedItem {
    preset_name: String,
    variants: Vec<ScorerPresetResolvedVariantItem>,
}

#[derive(Debug, Clone)]
struct ScorerPresetResolvedVariantItem {
    variant_name: String,
    weights: BTreeMap<String, f64>,
    main_buff_score: Option<f64>,
    normalized_max_score: Option<f64>,
    preset_intro: Option<String>,
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
    default_wuwa_echo_tool_buff_weights: BTreeMap<String, f64>,
    default_mc_boost_assistant_buff_weights: BTreeMap<String, f64>,
    default_qq_bot_buff_weights: BTreeMap<String, f64>,
    default_fixed_buff_weights: BTreeMap<String, u16>,
    max_selected_types: usize,
    default_target_score: f64,
    default_fixed_target_score: u16,
    default_linear_main_buff_score: f64,
    default_linear_normalized_max_score: f64,
    default_wuwa_echo_tool_target_score: f64,
    default_mc_boost_assistant_target_score: f64,
    default_qq_bot_target_score: f64,
    default_wuwa_echo_tool_main_buff_score: f64,
    default_wuwa_echo_tool_normalized_max_score: f64,
    default_qq_bot_main_buff_score: f64,
    default_qq_bot_normalized_max_score: f64,
    default_cost_weights: CostWeightsOutput,
    default_exp_refund_ratio: f64,
    default_scorer_type: String,
    default_ocr_udp_port: u16,
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

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OcrListenerStatusResponse {
    listening: bool,
    port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OcrFillEntriesEvent {
    buff_names: Vec<String>,
    buff_values: Vec<u16>,
}

#[derive(Clone, Copy)]
enum UpgradeScorerConfig {
    LinearDefault {
        weights: [f64; NUM_BUFFS],
        main_buff_score: f64,
        normalized_max_score: f64,
    },
    WuwaEchoTool {
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

struct OcrUdpListenerSession {
    port: u16,
    stop_flag: Arc<AtomicBool>,
    join_handle: JoinHandle<()>,
}

#[derive(Default)]
struct OcrUdpListenerState {
    session: Option<OcrUdpListenerSession>,
    last_error: Option<String>,
}

struct AppState {
    current_upgrade: Mutex<Option<SolverSession>>,
    current_reroll: Mutex<Option<RerollSession>>,
    ocr_udp_listener: Mutex<OcrUdpListenerState>,
}

impl AppState {
    fn new() -> Self {
        Self {
            current_upgrade: Mutex::new(None),
            current_reroll: Mutex::new(None),
            ocr_udp_listener: Mutex::new(OcrUdpListenerState::default()),
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
        SCORER_TYPE_WUWA_ECHO_TOOL => Ok(SCORER_TYPE_WUWA_ECHO_TOOL),
        SCORER_TYPE_MC_BOOST_ASSISTANT => Ok(SCORER_TYPE_MC_BOOST_ASSISTANT),
        SCORER_TYPE_QQ_BOT => Ok(SCORER_TYPE_QQ_BOT),
        SCORER_TYPE_FIXED => Ok(SCORER_TYPE_FIXED),
        _ => Err(format!(
            "Unsupported scorerType '{}'. Use 'linear_default', 'wuwa_echo_tool', 'mc_boost_assistant', 'qq_bot', or 'fixed'.",
            raw
        )),
    }
}

fn scorer_preset_file_name(scorer_type: &str) -> &'static str {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => "linear_default.json",
        SCORER_TYPE_WUWA_ECHO_TOOL => "wuwa_echo_tool.json",
        SCORER_TYPE_MC_BOOST_ASSISTANT => "mc_boost_assistant.json",
        SCORER_TYPE_QQ_BOT => "qq_bot.json",
        SCORER_TYPE_FIXED => "fixed.json",
        _ => unreachable!(),
    }
}

fn built_in_preset_source_name(scorer_type: &str) -> &'static str {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => "default-presets/linear_default.json",
        SCORER_TYPE_WUWA_ECHO_TOOL => "default-presets/wuwa_echo_tool.json",
        SCORER_TYPE_MC_BOOST_ASSISTANT => "default-presets/mc_boost_assistant.json",
        SCORER_TYPE_QQ_BOT => "default-presets/qq_bot.json",
        SCORER_TYPE_FIXED => "default-presets/fixed.json",
        _ => unreachable!(),
    }
}

fn built_in_preset_json(scorer_type: &str) -> &'static str {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => DEFAULT_LINEAR_PRESETS_JSON,
        SCORER_TYPE_WUWA_ECHO_TOOL => DEFAULT_WUWA_ECHO_TOOL_PRESETS_JSON,
        SCORER_TYPE_MC_BOOST_ASSISTANT => DEFAULT_MC_BOOST_ASSISTANT_PRESETS_JSON,
        SCORER_TYPE_QQ_BOT => DEFAULT_QQ_BOT_PRESETS_JSON,
        SCORER_TYPE_FIXED => DEFAULT_FIXED_PRESETS_JSON,
        _ => unreachable!(),
    }
}

fn scorer_preset_file_path(app: &tauri::AppHandle, scorer_type: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config directory: {err}"))?
        .join(SCORER_PRESET_DIR);
    fs::create_dir_all(&dir).map_err(|err| {
        format!(
            "Failed to create preset directory '{}': {err}",
            dir.display()
        )
    })?;
    Ok(dir.join(scorer_preset_file_name(scorer_type)))
}

fn parse_scorer_preset_file_content(
    content: &str,
    source_name: &str,
) -> Result<ScorerPresetFile, String> {
    let raw_file: ScorerPresetRawFile = serde_json::from_str(content)
        .map_err(|err| format!("Failed to parse preset file '{source_name}': {err}"))?;
    let presets = raw_file
        .presets
        .into_iter()
        .map(|item| match item {
            ScorerPresetRawItem::Grouped(grouped) => grouped,
            ScorerPresetRawItem::Legacy(legacy) => ScorerPresetFileItem {
                preset_name: legacy.preset_name,
                variants: vec![ScorerPresetVariantFileItem {
                    variant_name: SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string(),
                    weights: legacy.weights,
                    main_buff_score: legacy.main_buff_score,
                    normalized_max_score: legacy.normalized_max_score,
                    preset_intro: legacy.preset_intro,
                }],
            },
        })
        .collect();
    Ok(ScorerPresetFile { presets })
}

fn read_scorer_preset_file(path: &Path) -> Result<ScorerPresetFile, String> {
    match fs::read_to_string(path) {
        Ok(content) => parse_scorer_preset_file_content(&content, &path.display().to_string()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(ScorerPresetFile::default()),
        Err(err) => Err(format!(
            "Failed to read preset file '{}': {err}",
            path.display()
        )),
    }
}

fn read_built_in_scorer_presets(scorer_type: &str) -> Result<Vec<ScorerPresetFileItem>, String> {
    let source_name = built_in_preset_source_name(scorer_type);
    let content = built_in_preset_json(scorer_type);
    let file = parse_scorer_preset_file_content(content, source_name)?;
    Ok(normalize_loaded_preset_groups(scorer_type, file.presets))
}

fn write_scorer_preset_file(path: &Path, file: &ScorerPresetFile) -> Result<(), String> {
    let content = serde_json::to_string_pretty(file)
        .map_err(|err| format!("Failed to serialize presets: {err}"))?;
    fs::write(path, content)
        .map_err(|err| format!("Failed to write preset file '{}': {err}", path.display()))
}

fn default_fixed_weights_f64() -> [f64; NUM_BUFFS] {
    let mut out = [0.0; NUM_BUFFS];
    for index in 0..NUM_BUFFS {
        out[index] = f64::from(DEFAULT_FIXED_BUFF_WEIGHTS[index]);
    }
    out
}

fn default_weights_for_scorer_f64(scorer_type: &str) -> [f64; NUM_BUFFS] {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => DEFAULT_LINEAR_BUFF_WEIGHTS,
        SCORER_TYPE_WUWA_ECHO_TOOL => DEFAULT_WUWA_ECHO_TOOL_BUFF_WEIGHTS,
        SCORER_TYPE_MC_BOOST_ASSISTANT => DEFAULT_MC_BOOST_ASSISTANT_BUFF_WEIGHTS,
        SCORER_TYPE_QQ_BOT => DEFAULT_QQ_BOT_BUFF_WEIGHTS,
        SCORER_TYPE_FIXED => default_fixed_weights_f64(),
        _ => unreachable!(),
    }
}

fn normalized_main_buff_score(value: Option<f64>, default_value: f64) -> Result<f64, String> {
    let raw = value.unwrap_or(default_value);
    if !raw.is_finite() {
        return Err("mainBuffScore must be a finite number".to_string());
    }
    Ok(raw.max(0.0))
}

fn normalized_max_score(value: Option<f64>, default_value: f64) -> Result<f64, String> {
    let raw = value.unwrap_or(default_value);
    if !raw.is_finite() {
        return Err("normalizedMaxScore must be a finite number".to_string());
    }
    Ok(raw.max(MIN_NORMALIZED_MAX_SCORE))
}

fn normalize_fixed_weight(value: f64, buff_name: &str) -> Result<f64, String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!(
            "Invalid fixed scorer weight for {buff_name}: {value}"
        ));
    }
    if value > u16::MAX as f64 {
        return Err(format!(
            "Fixed scorer weight for {buff_name} must be <= {}",
            u16::MAX
        ));
    }
    Ok(value.round())
}

fn normalize_preset_name(raw_name: &str) -> Result<String, String> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return Err("presetName cannot be empty".to_string());
    }
    if trimmed == SCORER_PRESET_NAME_CUSTOM {
        return Err(format!(
            "'{}' is reserved for built-in defaults",
            SCORER_PRESET_NAME_CUSTOM
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_preset_variant_name(raw_name: &str) -> Result<String, String> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return Err("variantName cannot be empty".to_string());
    }
    Ok(trimmed.to_string())
}

fn normalize_preset_intro(raw_intro: Option<String>) -> Option<String> {
    raw_intro.and_then(|intro| {
        let trimmed = intro.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn default_main_buff_score_for_scorer(scorer_type: &str) -> Option<f64> {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => Some(DEFAULT_LINEAR_MAIN_BUFF_SCORE),
        SCORER_TYPE_WUWA_ECHO_TOOL => Some(DEFAULT_WUWA_ECHO_TOOL_MAIN_BUFF_SCORE),
        SCORER_TYPE_MC_BOOST_ASSISTANT => None,
        SCORER_TYPE_QQ_BOT => Some(DEFAULT_QQ_BOT_MAIN_BUFF_SCORE),
        SCORER_TYPE_FIXED => None,
        _ => unreachable!(),
    }
}

fn default_normalized_max_score_for_scorer(scorer_type: &str) -> Option<f64> {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => Some(DEFAULT_LINEAR_NORMALIZED_MAX_SCORE),
        SCORER_TYPE_WUWA_ECHO_TOOL => Some(DEFAULT_WUWA_ECHO_TOOL_NORMALIZED_MAX_SCORE),
        SCORER_TYPE_MC_BOOST_ASSISTANT => None,
        SCORER_TYPE_QQ_BOT => None,
        SCORER_TYPE_FIXED => None,
        _ => unreachable!(),
    }
}

fn btree_weights_to_hash_map(weights: &BTreeMap<String, f64>) -> HashMap<String, f64> {
    weights
        .iter()
        .map(|(name, value)| (name.clone(), *value))
        .collect()
}

fn normalize_preset_variant_values_for_scorer(
    scorer_type: &str,
    raw_weights: &HashMap<String, f64>,
    raw_main_buff_score: Option<f64>,
    raw_normalized_max_score: Option<f64>,
    raw_preset_intro: Option<String>,
) -> Result<(BTreeMap<String, f64>, Option<f64>, Option<f64>, Option<String>), String> {
    let mut weights =
        build_weight_array_f64(raw_weights, default_weights_for_scorer_f64(scorer_type))?;

    if scorer_type == SCORER_TYPE_FIXED {
        for (index, buff_name) in BUFF_TYPES.iter().enumerate() {
            weights[index] = normalize_fixed_weight(weights[index], buff_name)?;
        }
    }

    let (main_buff_score, normalized_max_score) = match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => (
            Some(normalized_main_buff_score(
                raw_main_buff_score,
                DEFAULT_LINEAR_MAIN_BUFF_SCORE,
            )?),
            Some(normalized_max_score(
                raw_normalized_max_score,
                DEFAULT_LINEAR_NORMALIZED_MAX_SCORE,
            )?),
        ),
        SCORER_TYPE_WUWA_ECHO_TOOL => (
            Some(normalized_main_buff_score(
                raw_main_buff_score,
                DEFAULT_WUWA_ECHO_TOOL_MAIN_BUFF_SCORE,
            )?),
            Some(normalized_max_score(
                raw_normalized_max_score,
                DEFAULT_WUWA_ECHO_TOOL_NORMALIZED_MAX_SCORE,
            )?),
        ),
        SCORER_TYPE_MC_BOOST_ASSISTANT => (None, None),
        SCORER_TYPE_QQ_BOT => (
            Some(normalized_main_buff_score(
                raw_main_buff_score,
                DEFAULT_QQ_BOT_MAIN_BUFF_SCORE,
            )?),
            None,
        ),
        SCORER_TYPE_FIXED => (None, None),
        _ => unreachable!(),
    };

    Ok((
        build_default_weight_map_f64(&weights),
        main_buff_score,
        normalized_max_score,
        normalize_preset_intro(raw_preset_intro),
    ))
}

fn option_f64_bits_equal(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(lhs), Some(rhs)) => f64_bits_equal(lhs, rhs),
        (None, None) => true,
        _ => false,
    }
}

fn resolve_variant_from_base(
    scorer_type: &str,
    base_variant: &ScorerPresetResolvedVariantItem,
    raw_variant: &ScorerPresetVariantFileItem,
) -> Result<ScorerPresetResolvedVariantItem, String> {
    let variant_name = normalize_preset_variant_name(&raw_variant.variant_name)?;
    let mut merged_weights = btree_weights_to_hash_map(&base_variant.weights);
    for (buff_name, value) in &raw_variant.weights {
        merged_weights.insert(buff_name.clone(), *value);
    }
    let raw_main_buff_score = raw_variant.main_buff_score.or(base_variant.main_buff_score);
    let raw_normalized_max_score = raw_variant
        .normalized_max_score
        .or(base_variant.normalized_max_score);
    let raw_preset_intro = raw_variant
        .preset_intro
        .clone()
        .or(base_variant.preset_intro.clone());
    let (weights, main_buff_score, normalized_max_score, preset_intro) =
        normalize_preset_variant_values_for_scorer(
            scorer_type,
            &merged_weights,
            raw_main_buff_score,
            raw_normalized_max_score,
            raw_preset_intro,
        )?;
    Ok(ScorerPresetResolvedVariantItem {
        variant_name,
        weights,
        main_buff_score,
        normalized_max_score,
        preset_intro,
    })
}

fn build_variant_override_from_base(
    base_variant: &ScorerPresetResolvedVariantItem,
    variant: &ScorerPresetResolvedVariantItem,
) -> ScorerPresetVariantFileItem {
    let mut weights = BTreeMap::new();
    for buff_name in BUFF_TYPES {
        let base_value = *base_variant.weights.get(buff_name).unwrap_or(&0.0);
        let variant_value = *variant.weights.get(buff_name).unwrap_or(&0.0);
        if !f64_bits_equal(base_value, variant_value) {
            weights.insert(buff_name.to_string(), variant_value);
        }
    }

    let main_buff_score = if option_f64_bits_equal(base_variant.main_buff_score, variant.main_buff_score)
    {
        None
    } else {
        variant.main_buff_score
    };
    let normalized_max_score = if option_f64_bits_equal(
        base_variant.normalized_max_score,
        variant.normalized_max_score,
    ) {
        None
    } else {
        variant.normalized_max_score
    };
    let preset_intro = if base_variant.preset_intro == variant.preset_intro {
        None
    } else {
        variant.preset_intro.clone()
    };

    ScorerPresetVariantFileItem {
        variant_name: variant.variant_name.clone(),
        weights,
        main_buff_score,
        normalized_max_score,
        preset_intro,
    }
}

fn resolved_variant_to_file_full(variant: &ScorerPresetResolvedVariantItem) -> ScorerPresetVariantFileItem {
    ScorerPresetVariantFileItem {
        variant_name: variant.variant_name.clone(),
        weights: variant.weights.clone(),
        main_buff_score: variant.main_buff_score,
        normalized_max_score: variant.normalized_max_score,
        preset_intro: variant.preset_intro.clone(),
    }
}

fn resolve_preset_group_for_scorer(
    scorer_type: &str,
    preset: &ScorerPresetFileItem,
) -> Result<ScorerPresetResolvedItem, String> {
    let preset_name = normalize_preset_name(&preset.preset_name)?;
    let base_raw_variant = preset
        .variants
        .first()
        .ok_or_else(|| format!("Preset '{preset_name}' must contain at least one variant"))?;
    let default_variant_name = if base_raw_variant.variant_name.trim().is_empty() {
        SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string()
    } else {
        base_raw_variant.variant_name.clone()
    };
    let (weights, main_buff_score, normalized_max_score, preset_intro) =
        normalize_preset_variant_values_for_scorer(
            scorer_type,
            &btree_weights_to_hash_map(&base_raw_variant.weights),
            base_raw_variant
                .main_buff_score
                .or(default_main_buff_score_for_scorer(scorer_type)),
            base_raw_variant
                .normalized_max_score
                .or(default_normalized_max_score_for_scorer(scorer_type)),
            base_raw_variant.preset_intro.clone(),
        )?;
    let base_variant = ScorerPresetResolvedVariantItem {
        variant_name: normalize_preset_variant_name(&default_variant_name)?,
        weights,
        main_buff_score,
        normalized_max_score,
        preset_intro,
    };

    let mut variants = vec![base_variant.clone()];
    for raw_variant in preset.variants.iter().skip(1) {
        match resolve_variant_from_base(scorer_type, &base_variant, raw_variant) {
            Ok(variant) => {
                if !variants
                    .iter()
                    .any(|existing| existing.variant_name == variant.variant_name)
                {
                    variants.push(variant);
                }
            }
            Err(err) => {
                eprintln!(
                    "Skipping invalid variant '{}' in preset '{}': {err}",
                    raw_variant.variant_name, preset_name
                );
            }
        }
    }

    Ok(ScorerPresetResolvedItem {
        preset_name,
        variants,
    })
}

fn normalize_loaded_preset_groups(
    scorer_type: &str,
    items: Vec<ScorerPresetFileItem>,
) -> Vec<ScorerPresetFileItem> {
    let mut out = Vec::new();

    for item in items {
        match resolve_preset_group_for_scorer(scorer_type, &item) {
            Ok(resolved) => {
                if out
                    .iter()
                    .any(|existing: &ScorerPresetFileItem| existing.preset_name == resolved.preset_name)
                {
                    continue;
                }

                let Some(base_variant) = resolved.variants.first() else {
                    eprintln!(
                        "Skipping invalid preset '{}': no normalized variants",
                        resolved.preset_name
                    );
                    continue;
                };
                let mut normalized_variants = vec![resolved_variant_to_file_full(base_variant)];
                for variant in resolved.variants.iter().skip(1) {
                    normalized_variants.push(build_variant_override_from_base(base_variant, variant));
                }

                out.push(ScorerPresetFileItem {
                    preset_name: resolved.preset_name,
                    variants: normalized_variants,
                });
            }
            Err(err) => {
                eprintln!("Skipping invalid preset '{}': {err}", item.preset_name);
            }
        }
    }

    out
}

fn resolve_preset_groups_for_scorer(
    scorer_type: &str,
    groups: &[ScorerPresetFileItem],
) -> Vec<ScorerPresetResolvedItem> {
    groups
        .iter()
        .filter_map(|group| match resolve_preset_group_for_scorer(scorer_type, group) {
            Ok(resolved) => Some(resolved),
            Err(err) => {
                eprintln!("Skipping invalid preset '{}': {err}", group.preset_name);
                None
            }
        })
        .collect()
}

fn preset_item_to_response(
    item: ScorerPresetResolvedItem,
    built_in: bool,
    user_defined: bool,
) -> ScorerPresetResponseItem {
    ScorerPresetResponseItem {
        preset_name: item.preset_name,
        variants: item
            .variants
            .into_iter()
            .map(|variant| ScorerPresetResponseVariantItem {
                variant_name: variant.variant_name,
                weights: variant.weights,
                main_buff_score: variant.main_buff_score,
                normalized_max_score: variant.normalized_max_score,
                preset_intro: variant.preset_intro,
            })
            .collect(),
        built_in,
        user_defined,
    }
}

fn build_resolved_variant_from_payload(
    scorer_type: &str,
    variant_name: &str,
    weights: &HashMap<String, f64>,
    main_buff_score: Option<f64>,
    normalized_max_score: Option<f64>,
    preset_intro: Option<String>,
) -> Result<ScorerPresetResolvedVariantItem, String> {
    let variant_name = normalize_preset_variant_name(variant_name)?;
    let (weights, main_buff_score, normalized_max_score, preset_intro) =
        normalize_preset_variant_values_for_scorer(
            scorer_type,
            weights,
            main_buff_score,
            normalized_max_score,
            preset_intro,
        )?;
    Ok(ScorerPresetResolvedVariantItem {
        variant_name,
        weights,
        main_buff_score,
        normalized_max_score,
        preset_intro,
    })
}

fn find_preset_group_index(groups: &[ScorerPresetFileItem], preset_name: &str) -> Option<usize> {
    groups
        .iter()
        .position(|item| item.preset_name.as_str() == preset_name)
}

fn find_resolved_preset<'a>(
    presets: &'a [ScorerPresetResolvedItem],
    preset_name: &str,
) -> Option<&'a ScorerPresetResolvedItem> {
    presets
        .iter()
        .find(|item| item.preset_name.as_str() == preset_name)
}

fn find_resolved_variant<'a>(
    preset: &'a ScorerPresetResolvedItem,
    variant_name: &str,
) -> Option<&'a ScorerPresetResolvedVariantItem> {
    preset
        .variants
        .iter()
        .find(|item| item.variant_name.as_str() == variant_name)
}

fn build_merged_preset_response(
    scorer_type: &str,
    built_in_items: &[ScorerPresetFileItem],
    user_items: &[ScorerPresetFileItem],
) -> Vec<ScorerPresetResponseItem> {
    let built_in_resolved = resolve_preset_groups_for_scorer(scorer_type, built_in_items);
    let user_resolved = resolve_preset_groups_for_scorer(scorer_type, user_items);

    let mut out = Vec::new();
    for user_preset in &user_resolved {
        if built_in_resolved
            .iter()
            .any(|item| item.preset_name == user_preset.preset_name)
        {
            out.push(preset_item_to_response(user_preset.clone(), true, true));
        } else {
            out.push(preset_item_to_response(user_preset.clone(), false, true));
        }
    }

    for built_in_preset in &built_in_resolved {
        if !user_resolved
            .iter()
            .any(|item| item.preset_name == built_in_preset.preset_name)
        {
            out.push(preset_item_to_response(built_in_preset.clone(), true, false));
        }
    }

    out
}

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
        SCORER_TYPE_WUWA_ECHO_TOOL => {
            let weights = build_weight_array_f64(buff_weights, DEFAULT_WUWA_ECHO_TOOL_BUFF_WEIGHTS)?;
            let main_buff_score =
                main_buff_score.unwrap_or(DEFAULT_WUWA_ECHO_TOOL_MAIN_BUFF_SCORE);
            let normalized_max_score =
                normalized_max_score.unwrap_or(DEFAULT_WUWA_ECHO_TOOL_NORMALIZED_MAX_SCORE);
            Ok(UpgradeScorerConfig::WuwaEchoTool {
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
            let weights =
                build_weight_array_u16_from_f64(buff_weights, DEFAULT_FIXED_BUFF_WEIGHTS)?;
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
        UpgradeScorerConfig::WuwaEchoTool {
            weights,
            main_buff_score,
            normalized_max_score,
        } => Ok(UpgradeScorer::Linear(build_wuwa_echo_tool_scorer(
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
            let scorer = FixedScorer::new(*weights)
                .map_err(|err| format!("Invalid fixed scorer: {err:?}"))?;
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
        UpgradeScorer::Linear(linear) => {
            UpgradePolicySolver::new(linear, blend_data, target_score_display, cost_model)
                .map_err(|err| format!("Failed to create solver: {err:?}"))
        }
        UpgradeScorer::Fixed(fixed) => {
            UpgradePolicySolver::new(fixed, blend_data, target_score_display, cost_model)
                .map_err(|err| format!("Failed to create solver: {err:?}"))
        }
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
            Ok((
                raw_target_score,
                (target_on_solver_scale - main_score).max(0.0),
            ))
        }
        UpgradeScorerConfig::LinearDefault { .. }
        | UpgradeScorerConfig::WuwaEchoTool { .. }
        | UpgradeScorerConfig::McBoostAssistant { .. } => {
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

fn parse_ocr_udp_payload(raw_message: &str) -> Result<OcrFillEntriesEvent, String> {
    let payload: OcrUdpPayload =
        serde_json::from_str(raw_message).map_err(|err| format!("Invalid JSON payload: {err}"))?;
    if payload.buff_entries.is_empty() {
        return Err("buffEntries cannot be empty".to_string());
    }
    if payload.buff_entries.len() > MAX_SELECTED_TYPES {
        return Err(format!(
            "Too many buffEntries: {}, max is {MAX_SELECTED_TYPES}",
            payload.buff_entries.len()
        ));
    }

    let mut seen = [false; NUM_BUFFS];
    let mut buff_names = Vec::with_capacity(payload.buff_entries.len());
    let mut buff_values = Vec::with_capacity(payload.buff_entries.len());

    for (entry_idx, entry) in payload.buff_entries.iter().enumerate() {
        let buff_name = entry.buff_name.trim();
        let buff_idx = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff in buffEntries[{entry_idx}]: {buff_name}"))?;
        if seen[buff_idx] {
            return Err(format!(
                "Duplicate buff in buffEntries: {}",
                BUFF_TYPES[buff_idx]
            ));
        }
        if !BUFF_VALUE_OPTIONS[buff_idx].contains(&entry.buff_value) {
            return Err(format!(
                "Invalid value {} for buff {}",
                entry.buff_value, BUFF_TYPES[buff_idx]
            ));
        }

        seen[buff_idx] = true;
        buff_names.push(BUFF_TYPES[buff_idx].to_string());
        buff_values.push(entry.buff_value);
    }

    Ok(OcrFillEntriesEvent {
        buff_names,
        buff_values,
    })
}

fn ocr_listener_status_snapshot(state: &OcrUdpListenerState) -> OcrListenerStatusResponse {
    OcrListenerStatusResponse {
        listening: state.session.is_some(),
        port: state.session.as_ref().map(|session| session.port),
        last_error: state.last_error.clone(),
    }
}

fn emit_ocr_listener_status_event(app: &tauri::AppHandle, status: &OcrListenerStatusResponse) {
    if let Err(err) = app.emit(OCR_UDP_EVENT_LISTENER_STATUS, status.clone()) {
        eprintln!("Failed to emit OCR listener status event: {err}");
    }
}

fn stop_ocr_udp_session(session: OcrUdpListenerSession) -> Result<(), String> {
    session.stop_flag.store(true, Ordering::Relaxed);
    session
        .join_handle
        .join()
        .map_err(|_| "OCR UDP listener thread panicked".to_string())
}

fn run_ocr_udp_listener_loop(app: tauri::AppHandle, socket: UdpSocket, stop_flag: Arc<AtomicBool>) {
    let mut buffer = [0u8; OCR_UDP_PACKET_BUFFER_SIZE];
    while !stop_flag.load(Ordering::Relaxed) {
        match socket.recv_from(&mut buffer) {
            Ok((size, source)) => {
                let message = match std::str::from_utf8(&buffer[..size]) {
                    Ok(text) => text,
                    Err(err) => {
                        eprintln!("Ignoring OCR UDP packet from {source}: invalid UTF-8 ({err})");
                        continue;
                    }
                };
                match parse_ocr_udp_payload(message) {
                    Ok(fill_event) => {
                        if let Err(err) = app.emit(OCR_UDP_EVENT_FILL_ENTRIES, fill_event) {
                            eprintln!("Failed to emit OCR fill event: {err}");
                        }
                    }
                    Err(err) => {
                        eprintln!("Ignoring OCR UDP packet from {source}: {err}");
                    }
                }
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut => {}
            Err(err) => {
                eprintln!("OCR UDP listener receive error: {err}");
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
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

fn build_wuwa_echo_tool_scorer(
    weights: [f64; NUM_BUFFS],
    main_buff_score: f64,
    normalized_max_score: f64,
) -> Result<LinearScorer, String> {
    LinearScorer::new(weights, main_buff_score, normalized_max_score)
        .map_err(|err| format!("Invalid Wuwa Echo Tool scorer: {err:?}"))
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
        default_wuwa_echo_tool_buff_weights: build_default_weight_map_f64(
            &DEFAULT_WUWA_ECHO_TOOL_BUFF_WEIGHTS,
        ),
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
        default_wuwa_echo_tool_target_score: DEFAULT_WUWA_ECHO_TOOL_TARGET_SCORE,
        default_mc_boost_assistant_target_score: DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE,
        default_qq_bot_target_score: DEFAULT_QQ_BOT_TARGET_SCORE,
        default_wuwa_echo_tool_main_buff_score: DEFAULT_WUWA_ECHO_TOOL_MAIN_BUFF_SCORE,
        default_wuwa_echo_tool_normalized_max_score: DEFAULT_WUWA_ECHO_TOOL_NORMALIZED_MAX_SCORE,
        default_qq_bot_main_buff_score: DEFAULT_QQ_BOT_MAIN_BUFF_SCORE,
        default_qq_bot_normalized_max_score: DEFAULT_QQ_BOT_NORMALIZED_MAX_SCORE,
        default_cost_weights: default_cost_weights(),
        default_exp_refund_ratio: DEFAULT_EXP_REFUND_RATIO,
        default_scorer_type: DEFAULT_SCORER_TYPE.to_string(),
        default_ocr_udp_port: DEFAULT_OCR_UDP_PORT,
    }
}

#[tauri::command]
fn get_ocr_udp_listener_status(
    state: State<'_, AppState>,
) -> Result<OcrListenerStatusResponse, String> {
    let listener = state
        .ocr_udp_listener
        .lock()
        .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
    Ok(ocr_listener_status_snapshot(&listener))
}

#[tauri::command]
fn start_ocr_udp_listener(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    payload: StartOcrUdpListenerRequest,
) -> Result<OcrListenerStatusResponse, String> {
    if payload.port == 0 {
        return Err("port must be between 1 and 65535".to_string());
    }

    {
        let listener = state
            .ocr_udp_listener
            .lock()
            .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
        if let Some(session) = listener.session.as_ref()
            && session.port == payload.port
        {
            let status = ocr_listener_status_snapshot(&listener);
            emit_ocr_listener_status_event(&app, &status);
            return Ok(status);
        }
    }

    let socket = UdpSocket::bind(("127.0.0.1", payload.port))
        .map_err(|err| format!("Failed to bind UDP port {}: {err}", payload.port))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(OCR_UDP_READ_TIMEOUT_MS)))
        .map_err(|err| format!("Failed to configure UDP socket timeout: {err}"))?;

    let previous_session = {
        let mut listener = state
            .ocr_udp_listener
            .lock()
            .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
        listener.session.take()
    };
    if let Some(session) = previous_session {
        stop_ocr_udp_session(session)?;
    }

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_for_thread = Arc::clone(&stop_flag);
    let app_for_thread = app.clone();
    let listener_thread = thread::spawn(move || {
        run_ocr_udp_listener_loop(app_for_thread, socket, stop_flag_for_thread)
    });

    let status = {
        let mut listener = state
            .ocr_udp_listener
            .lock()
            .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
        listener.last_error = None;
        listener.session = Some(OcrUdpListenerSession {
            port: payload.port,
            stop_flag,
            join_handle: listener_thread,
        });
        ocr_listener_status_snapshot(&listener)
    };
    emit_ocr_listener_status_event(&app, &status);
    Ok(status)
}

#[tauri::command]
fn stop_ocr_udp_listener(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<OcrListenerStatusResponse, String> {
    let previous_session = {
        let mut listener = state
            .ocr_udp_listener
            .lock()
            .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
        listener.session.take()
    };

    let stop_error = if let Some(session) = previous_session {
        stop_ocr_udp_session(session).err()
    } else {
        None
    };

    let status = {
        let mut listener = state
            .ocr_udp_listener
            .lock()
            .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
        listener.last_error = stop_error;
        ocr_listener_status_snapshot(&listener)
    };
    emit_ocr_listener_status_event(&app, &status);
    Ok(status)
}

#[tauri::command]
fn load_scorer_presets(
    app: tauri::AppHandle,
    payload: LoadScorerPresetsRequest,
) -> Result<LoadScorerPresetsResponse, String> {
    let scorer_type = parse_scorer_type(&payload.scorer_type)?;
    let file_path = scorer_preset_file_path(&app, scorer_type)?;
    let built_in_items = read_built_in_scorer_presets(scorer_type)?;
    let file = read_scorer_preset_file(&file_path)?;
    let user_items = normalize_loaded_preset_groups(scorer_type, file.presets);
    let presets = build_merged_preset_response(scorer_type, &built_in_items, &user_items);
    Ok(LoadScorerPresetsResponse { presets })
}

#[tauri::command]
fn save_scorer_preset(
    app: tauri::AppHandle,
    payload: SaveScorerPresetRequest,
) -> Result<SaveScorerPresetResponse, String> {
    let scorer_type = parse_scorer_type(&payload.scorer_type)?;
    let preset_name = normalize_preset_name(&payload.preset_name)?;
    let file_path = scorer_preset_file_path(&app, scorer_type)?;
    let built_in_items = read_built_in_scorer_presets(scorer_type)?;
    let file = read_scorer_preset_file(&file_path)?;
    let mut user_items = normalize_loaded_preset_groups(scorer_type, file.presets);

    let bundled_exists = find_preset_group_index(&built_in_items, &preset_name).is_some();
    let user_index = find_preset_group_index(&user_items, &preset_name);
    if bundled_exists && user_index.is_none() {
        return Err(format!(
            "Bundled preset '{}' is read-only. Save it using a new preset name.",
            preset_name
        ));
    }

    let user_resolved = resolve_preset_groups_for_scorer(scorer_type, &user_items);
    let fallback_intro = find_resolved_preset(&user_resolved, &preset_name)
        .and_then(|preset| preset.variants.first())
        .and_then(|variant| variant.preset_intro.clone());

    let requested_variant_name = payload
        .variant_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(normalize_preset_variant_name)
        .transpose()?;

    let base_variant_name = if let Some(index) = user_index {
        user_items[index]
            .variants
            .first()
            .map(|variant| variant.variant_name.clone())
            .unwrap_or_else(|| SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string())
    } else {
        requested_variant_name.unwrap_or_else(|| SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string())
    };

    let resolved_base_variant = build_resolved_variant_from_payload(
        scorer_type,
        &base_variant_name,
        &payload.weights,
        payload.main_buff_score,
        payload.normalized_max_score,
        payload.preset_intro.or(fallback_intro),
    )?;
    let serialized_base_variant = resolved_variant_to_file_full(&resolved_base_variant);

    if let Some(existing_index) = user_index {
        if user_items[existing_index].variants.is_empty() {
            user_items[existing_index]
                .variants
                .push(serialized_base_variant.clone());
        } else {
            user_items[existing_index].variants[0] = serialized_base_variant.clone();
        }
    } else {
        user_items.push(ScorerPresetFileItem {
            preset_name: preset_name.clone(),
            variants: vec![serialized_base_variant],
        });
    }

    user_items = normalize_loaded_preset_groups(scorer_type, user_items);

    write_scorer_preset_file(
        &file_path,
        &ScorerPresetFile {
            presets: user_items.clone(),
        },
    )?;

    let presets = build_merged_preset_response(scorer_type, &built_in_items, &user_items);
    Ok(SaveScorerPresetResponse {
        saved_preset_name: preset_name,
        saved_variant_name: resolved_base_variant.variant_name,
        presets,
    })
}

#[tauri::command]
fn save_scorer_preset_variant(
    app: tauri::AppHandle,
    payload: SaveScorerPresetVariantRequest,
) -> Result<SaveScorerPresetVariantResponse, String> {
    let scorer_type = parse_scorer_type(&payload.scorer_type)?;
    let preset_name = normalize_preset_name(&payload.preset_name)?;
    let variant_name = normalize_preset_variant_name(&payload.variant_name)?;
    let file_path = scorer_preset_file_path(&app, scorer_type)?;
    let built_in_items = read_built_in_scorer_presets(scorer_type)?;
    let file = read_scorer_preset_file(&file_path)?;
    let mut user_items = normalize_loaded_preset_groups(scorer_type, file.presets);
    let user_index = find_preset_group_index(&user_items, &preset_name).ok_or_else(|| {
        if find_preset_group_index(&built_in_items, &preset_name).is_some() {
            format!(
                "Bundled preset '{}' is read-only. Save as a new preset first.",
                preset_name
            )
        } else {
            format!("Preset '{preset_name}' does not exist")
        }
    })?;
    let user_resolved = resolve_preset_groups_for_scorer(scorer_type, &user_items);
    let source_preset = find_resolved_preset(&user_resolved, &preset_name)
        .ok_or_else(|| format!("Preset '{preset_name}' does not exist"))?;
    let default_variant_name = source_preset
        .variants
        .first()
        .map(|variant| variant.variant_name.clone())
        .unwrap_or_else(|| SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string());

    if variant_name == default_variant_name {
        return Err(format!(
            "Variant '{}' is the default variant. Use preset save to update it.",
            variant_name
        ));
    }

    let current_user_preset = find_resolved_preset(&user_resolved, &preset_name)
        .ok_or_else(|| format!("Preset '{preset_name}' failed to load"))?;
    let base_variant = current_user_preset
        .variants
        .first()
        .ok_or_else(|| format!("Preset '{preset_name}' has no base variant"))?;
    let fallback_intro = find_resolved_variant(current_user_preset, &variant_name)
        .and_then(|variant| variant.preset_intro.clone());

    let resolved_variant = build_resolved_variant_from_payload(
        scorer_type,
        &variant_name,
        &payload.weights,
        payload.main_buff_score,
        payload.normalized_max_score,
        payload.preset_intro.or(fallback_intro),
    )?;
    let serialized_variant = build_variant_override_from_base(base_variant, &resolved_variant);

    if let Some(existing_index) = user_items[user_index]
        .variants
        .iter()
        .position(|variant| variant.variant_name == variant_name)
    {
        if existing_index == 0 {
            return Err(format!(
                "Variant '{}' is the default variant. Use preset save to update it.",
                variant_name
            ));
        }
        user_items[user_index].variants[existing_index] = serialized_variant;
    } else {
        user_items[user_index].variants.push(serialized_variant);
    }

    user_items = normalize_loaded_preset_groups(scorer_type, user_items);
    write_scorer_preset_file(
        &file_path,
        &ScorerPresetFile {
            presets: user_items.clone(),
        },
    )?;

    let presets = build_merged_preset_response(scorer_type, &built_in_items, &user_items);
    Ok(SaveScorerPresetVariantResponse {
        saved_preset_name: preset_name,
        saved_variant_name: variant_name,
        presets,
    })
}

#[tauri::command]
fn delete_scorer_preset(
    app: tauri::AppHandle,
    payload: DeleteScorerPresetRequest,
) -> Result<DeleteScorerPresetResponse, String> {
    let scorer_type = parse_scorer_type(&payload.scorer_type)?;
    let preset_name = normalize_preset_name(&payload.preset_name)?;
    let file_path = scorer_preset_file_path(&app, scorer_type)?;
    let built_in_items = read_built_in_scorer_presets(scorer_type)?;
    let file = read_scorer_preset_file(&file_path)?;
    let mut user_items = normalize_loaded_preset_groups(scorer_type, file.presets);

    let Some(existing_index) = find_preset_group_index(&user_items, &preset_name) else {
        if built_in_items
            .iter()
            .any(|item| item.preset_name == preset_name)
        {
            return Err(format!("Bundled preset '{preset_name}' cannot be deleted"));
        }
        return Err(format!("Preset '{preset_name}' does not exist"));
    };

    user_items.remove(existing_index);
    user_items = normalize_loaded_preset_groups(scorer_type, user_items);
    write_scorer_preset_file(
        &file_path,
        &ScorerPresetFile {
            presets: user_items.clone(),
        },
    )?;

    let presets = build_merged_preset_response(scorer_type, &built_in_items, &user_items);
    Ok(DeleteScorerPresetResponse {
        deleted_preset_name: preset_name,
        presets,
    })
}

#[tauri::command]
fn delete_scorer_preset_variant(
    app: tauri::AppHandle,
    payload: DeleteScorerPresetVariantRequest,
) -> Result<DeleteScorerPresetVariantResponse, String> {
    let scorer_type = parse_scorer_type(&payload.scorer_type)?;
    let preset_name = normalize_preset_name(&payload.preset_name)?;
    let variant_name = normalize_preset_variant_name(&payload.variant_name)?;
    let file_path = scorer_preset_file_path(&app, scorer_type)?;
    let built_in_items = read_built_in_scorer_presets(scorer_type)?;
    let built_in_resolved = resolve_preset_groups_for_scorer(scorer_type, &built_in_items);
    let file = read_scorer_preset_file(&file_path)?;
    let mut user_items = normalize_loaded_preset_groups(scorer_type, file.presets);

    let user_index = find_preset_group_index(&user_items, &preset_name);
    let bundled_has_variant = find_resolved_preset(&built_in_resolved, &preset_name)
        .and_then(|preset| find_resolved_variant(preset, &variant_name))
        .is_some();

    let Some(user_index) = user_index else {
        if bundled_has_variant {
            return Err(format!(
                "Bundled variant '{} / {}' cannot be deleted",
                preset_name, variant_name
            ));
        }
        return Err(format!(
            "Preset variant '{} / {}' does not exist",
            preset_name, variant_name
        ));
    };

    let resolved_user_preset = resolve_preset_group_for_scorer(scorer_type, &user_items[user_index])?;
    let default_variant_name = resolved_user_preset
        .variants
        .first()
        .map(|variant| variant.variant_name.clone())
        .unwrap_or_else(|| SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string());
    if variant_name == default_variant_name {
        return Err(format!(
            "Default variant '{}' cannot be deleted",
            variant_name
        ));
    }

    let Some(variant_index) = user_items[user_index]
        .variants
        .iter()
        .position(|variant| variant.variant_name == variant_name)
    else {
        if bundled_has_variant {
            return Err(format!(
                "Bundled variant '{} / {}' cannot be deleted",
                preset_name, variant_name
            ));
        }
        return Err(format!(
            "Preset variant '{} / {}' does not exist",
            preset_name, variant_name
        ));
    };

    if variant_index == 0 {
        return Err(format!(
            "Default variant '{}' cannot be deleted",
            variant_name
        ));
    }

    user_items[user_index].variants.remove(variant_index);
    user_items = normalize_loaded_preset_groups(scorer_type, user_items);

    if let Some(bundled_preset) = find_resolved_preset(&built_in_resolved, &preset_name)
        && let Some(new_user_index) = find_preset_group_index(&user_items, &preset_name)
    {
        let resolved_user_after =
            resolve_preset_group_for_scorer(scorer_type, &user_items[new_user_index])?;
        if resolved_presets_equal(&resolved_user_after, bundled_preset) {
            user_items.remove(new_user_index);
        }
    }

    write_scorer_preset_file(
        &file_path,
        &ScorerPresetFile {
            presets: user_items.clone(),
        },
    )?;

    let presets = build_merged_preset_response(scorer_type, &built_in_items, &user_items);
    Ok(DeleteScorerPresetVariantResponse {
        deleted_preset_name: preset_name,
        deleted_variant_name: variant_name,
        presets,
    })
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
        let session = current_upgrade
            .as_mut()
            .ok_or_else(|| "Upgrade solver session was not initialized".to_string())?;
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

    let session = current_upgrade
        .as_mut()
        .ok_or_else(|| "Upgrade solver session was not initialized".to_string())?;
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
        let session = current_reroll
            .as_mut()
            .ok_or_else(|| "Reroll solver session was not initialized".to_string())?;
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
        let scorer =
            FixedScorer::new(weights).map_err(|err| format!("Invalid fixed scorer: {err:?}"))?;
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
            get_ocr_udp_listener_status,
            start_ocr_udp_listener,
            stop_ocr_udp_listener,
            load_scorer_presets,
            save_scorer_preset,
            save_scorer_preset_variant,
            delete_scorer_preset,
            delete_scorer_preset_variant,
            preview_upgrade_score,
            compute_policy,
            policy_suggestion,
            compute_reroll_policy,
            query_reroll_recommendation
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
