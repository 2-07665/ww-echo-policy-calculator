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

