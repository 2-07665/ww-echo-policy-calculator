fn build_default_linear_scorer(
    weights: [f64; NUM_BUFFS],
    main_buff_score: f64,
    normalized_max_score: f64,
) -> Result<LinearScorer, String> {
    if (main_buff_score - DEFAULT_LINEAR_MAIN_BUFF_SCORE).abs() <= 1e-12
        && (normalized_max_score - DEFAULT_LINEAR_NORMALIZED_MAX_SCORE).abs() <= 1e-12
    {
        LinearScorer::default(weights).map_err(|err| format!("Invalid linear scorer: {err:?}"))
    } else {
        LinearScorer::new(weights, main_buff_score, normalized_max_score)
            .map_err(|err| format!("Invalid linear scorer: {err:?}"))
    }
}

fn build_wuwa_echo_tool_scorer(
    weights: [f64; NUM_BUFFS],
    main_buff_score: f64,
    normalized_max_score: f64,
) -> Result<LinearScorer, String> {
    LinearScorer::new(weights, main_buff_score, normalized_max_score)
        .map_err(|err| format!("Invalid Wuwa Echo Tool scorer: {err:?}"))
}

fn build_qq_bot_scorer(
    qq_bot_weights: [f64; NUM_BUFFS],
    main_buff_score: f64,
) -> Result<LinearScorer, String> {
    LinearScorer::qq_bot_scorer(qq_bot_weights, main_buff_score)
        .map_err(|err| format!("Invalid QQ Bot scorer: {err:?}"))
}

fn build_mc_boost_assistant_scorer(weights: [f64; NUM_BUFFS]) -> Result<LinearScorer, String> {
    LinearScorer::mc_boost_assistant_scorer(weights)
        .map_err(|err| format!("Invalid MC Boost Assistant scorer: {err:?}"))
}

fn build_indexed_echo(
    buff_names: &[String],
    buff_values: &[u16],
) -> Result<Vec<(usize, u16)>, String> {
    if buff_names.len() != buff_values.len() {
        return Err("buffNames and buffValues length mismatch".to_string());
    }

    let mut indexed = Vec::with_capacity(buff_names.len());
    for (buff_name, &buff_value) in buff_names.iter().zip(buff_values.iter()) {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in selection: {buff_name}"))?;
        indexed.push((index, buff_value));
    }
    Ok(indexed)
}

fn score_from_selected_buffs_for_solver(
    scorer: &UpgradeScorer,
    buff_names: &[String],
    buff_values: &[u16],
) -> Result<u16, String> {
    if buff_names.is_empty() {
        return Ok(0);
    }

    let indexed = build_indexed_echo(buff_names, buff_values)?;
    match scorer {
        UpgradeScorer::Linear(linear) => linear
            .echo_score_internal(&indexed)
            .map_err(|err| format!("Failed to compute internal score: {err:?}")),
        UpgradeScorer::Fixed(fixed) => fixed
            .echo_score_internal(&indexed)
            .map_err(|err| format!("Failed to compute internal score: {err:?}")),
    }
}

