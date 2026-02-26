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
