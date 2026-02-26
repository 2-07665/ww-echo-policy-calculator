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
