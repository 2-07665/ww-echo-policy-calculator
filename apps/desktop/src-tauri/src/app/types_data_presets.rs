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
