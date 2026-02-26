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

