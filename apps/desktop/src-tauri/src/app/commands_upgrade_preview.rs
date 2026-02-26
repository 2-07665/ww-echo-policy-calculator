#[tauri::command]
fn preview_upgrade_score(
    payload: UpgradeScorePreviewRequest,
) -> Result<UpgradeScorePreviewResponse, String> {
    let scorer_type = parse_scorer_type(&payload.scorer_type)?;
    let scorer_config = build_upgrade_scorer_config_from_inputs(
        scorer_type,
        &payload.buff_weights,
        payload.main_buff_score,
        payload.normalized_max_score,
    )?;
    let scorer = build_upgrade_scorer(&scorer_config)?;

    let indexed = build_indexed_echo(&payload.buff_names, &payload.buff_values)?;
    match scorer {
        UpgradeScorer::Linear(linear) => {
            let contributions = indexed
                .iter()
                .map(|&(buff_index, buff_value)| linear.buff_score_display(buff_index, buff_value))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| format!("Failed to compute display contribution: {err:?}"))?;
            let total_score = linear
                .echo_score_display(&indexed)
                .map_err(|err| format!("Failed to compute display score: {err:?}"))?;
            Ok(UpgradeScorePreviewResponse {
                contributions,
                main_contribution: linear.main_buff_score(),
                total_score,
                max_score: linear.normalized_max_score(),
            })
        }
        UpgradeScorer::Fixed(fixed) => {
            let contributions = indexed
                .iter()
                .map(|&(buff_index, buff_value)| fixed.buff_score_display(buff_index, buff_value))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| format!("Failed to compute display contribution: {err:?}"))?
                .into_iter()
                .map(f64::from)
                .collect();
            let total_score = f64::from(
                fixed
                    .echo_score_display(&indexed)
                    .map_err(|err| format!("Failed to compute display score: {err:?}"))?,
            );
            Ok(UpgradeScorePreviewResponse {
                contributions,
                main_contribution: 0.0,
                total_score,
                max_score: f64::from(fixed.max_score()),
            })
        }
    }
}

