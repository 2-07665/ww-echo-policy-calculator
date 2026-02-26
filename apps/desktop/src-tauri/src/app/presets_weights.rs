fn default_fixed_weights_f64() -> [f64; NUM_BUFFS] {
    let mut out = [0.0; NUM_BUFFS];
    for index in 0..NUM_BUFFS {
        out[index] = f64::from(DEFAULT_FIXED_BUFF_WEIGHTS[index]);
    }
    out
}

fn default_weights_for_scorer_f64(scorer_type: &str) -> [f64; NUM_BUFFS] {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => DEFAULT_LINEAR_BUFF_WEIGHTS,
        SCORER_TYPE_WUWA_ECHO_TOOL => DEFAULT_WUWA_ECHO_TOOL_BUFF_WEIGHTS,
        SCORER_TYPE_MC_BOOST_ASSISTANT => DEFAULT_MC_BOOST_ASSISTANT_BUFF_WEIGHTS,
        SCORER_TYPE_QQ_BOT => DEFAULT_QQ_BOT_BUFF_WEIGHTS,
        SCORER_TYPE_FIXED => default_fixed_weights_f64(),
        _ => unreachable!(),
    }
}

fn normalized_main_buff_score(value: Option<f64>, default_value: f64) -> Result<f64, String> {
    let raw = value.unwrap_or(default_value);
    if !raw.is_finite() {
        return Err("mainBuffScore must be a finite number".to_string());
    }
    Ok(raw.max(0.0))
}

fn normalized_max_score(value: Option<f64>, default_value: f64) -> Result<f64, String> {
    let raw = value.unwrap_or(default_value);
    if !raw.is_finite() {
        return Err("normalizedMaxScore must be a finite number".to_string());
    }
    Ok(raw.max(MIN_NORMALIZED_MAX_SCORE))
}

fn normalize_fixed_weight(value: f64, buff_name: &str) -> Result<f64, String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!(
            "Invalid fixed scorer weight for {buff_name}: {value}"
        ));
    }
    if value > u16::MAX as f64 {
        return Err(format!(
            "Fixed scorer weight for {buff_name} must be <= {}",
            u16::MAX
        ));
    }
    Ok(value.round())
}

fn normalize_preset_name(raw_name: &str) -> Result<String, String> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return Err("presetName cannot be empty".to_string());
    }
    if trimmed == SCORER_PRESET_NAME_CUSTOM {
        return Err(format!(
            "'{}' is reserved for built-in defaults",
            SCORER_PRESET_NAME_CUSTOM
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_preset_variant_name(raw_name: &str) -> Result<String, String> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return Err("variantName cannot be empty".to_string());
    }
    Ok(trimmed.to_string())
}

fn normalize_preset_intro(raw_intro: Option<String>) -> Option<String> {
    raw_intro.and_then(|intro| {
        let trimmed = intro.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn default_main_buff_score_for_scorer(scorer_type: &str) -> Option<f64> {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => Some(DEFAULT_LINEAR_MAIN_BUFF_SCORE),
        SCORER_TYPE_WUWA_ECHO_TOOL => Some(DEFAULT_WUWA_ECHO_TOOL_MAIN_BUFF_SCORE),
        SCORER_TYPE_MC_BOOST_ASSISTANT => None,
        SCORER_TYPE_QQ_BOT => Some(DEFAULT_QQ_BOT_MAIN_BUFF_SCORE),
        SCORER_TYPE_FIXED => None,
        _ => unreachable!(),
    }
}

fn default_normalized_max_score_for_scorer(scorer_type: &str) -> Option<f64> {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => Some(DEFAULT_LINEAR_NORMALIZED_MAX_SCORE),
        SCORER_TYPE_WUWA_ECHO_TOOL => Some(DEFAULT_WUWA_ECHO_TOOL_NORMALIZED_MAX_SCORE),
        SCORER_TYPE_MC_BOOST_ASSISTANT => None,
        SCORER_TYPE_QQ_BOT => None,
        SCORER_TYPE_FIXED => None,
        _ => unreachable!(),
    }
}

fn btree_weights_to_hash_map(weights: &BTreeMap<String, f64>) -> HashMap<String, f64> {
    weights
        .iter()
        .map(|(name, value)| (name.clone(), *value))
        .collect()
}

fn normalize_preset_variant_values_for_scorer(
    scorer_type: &str,
    raw_weights: &HashMap<String, f64>,
    raw_main_buff_score: Option<f64>,
    raw_normalized_max_score: Option<f64>,
    raw_preset_intro: Option<String>,
) -> Result<(BTreeMap<String, f64>, Option<f64>, Option<f64>, Option<String>), String> {
    let mut weights =
        build_weight_array_f64(raw_weights, default_weights_for_scorer_f64(scorer_type))?;

    if scorer_type == SCORER_TYPE_FIXED {
        for (index, buff_name) in BUFF_TYPES.iter().enumerate() {
            weights[index] = normalize_fixed_weight(weights[index], buff_name)?;
        }
    }

    let (main_buff_score, normalized_max_score) = match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => (
            Some(normalized_main_buff_score(
                raw_main_buff_score,
                DEFAULT_LINEAR_MAIN_BUFF_SCORE,
            )?),
            Some(normalized_max_score(
                raw_normalized_max_score,
                DEFAULT_LINEAR_NORMALIZED_MAX_SCORE,
            )?),
        ),
        SCORER_TYPE_WUWA_ECHO_TOOL => (
            Some(normalized_main_buff_score(
                raw_main_buff_score,
                DEFAULT_WUWA_ECHO_TOOL_MAIN_BUFF_SCORE,
            )?),
            Some(normalized_max_score(
                raw_normalized_max_score,
                DEFAULT_WUWA_ECHO_TOOL_NORMALIZED_MAX_SCORE,
            )?),
        ),
        SCORER_TYPE_MC_BOOST_ASSISTANT => (None, None),
        SCORER_TYPE_QQ_BOT => (
            Some(normalized_main_buff_score(
                raw_main_buff_score,
                DEFAULT_QQ_BOT_MAIN_BUFF_SCORE,
            )?),
            None,
        ),
        SCORER_TYPE_FIXED => (None, None),
        _ => unreachable!(),
    };

    Ok((
        build_default_weight_map_f64(&weights),
        main_buff_score,
        normalized_max_score,
        normalize_preset_intro(raw_preset_intro),
    ))
}

