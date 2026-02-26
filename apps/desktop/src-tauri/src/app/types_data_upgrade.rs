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
