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
