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
