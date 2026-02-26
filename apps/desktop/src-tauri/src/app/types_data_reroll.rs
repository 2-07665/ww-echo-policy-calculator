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
