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
