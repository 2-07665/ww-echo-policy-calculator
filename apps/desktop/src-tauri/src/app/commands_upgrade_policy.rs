#[tauri::command]
fn compute_policy(
    state: State<'_, AppState>,
    payload: ComputePolicyRequest,
) -> Result<ComputePolicyResponse, String> {
    if payload.lambda_tolerance <= 0.0 || !payload.lambda_tolerance.is_finite() {
        return Err("lambdaTolerance must be a positive finite number".to_string());
    }
    if payload.lambda_max_iter == 0 {
        return Err("lambdaMaxIter must be greater than 0".to_string());
    }

    let exp_refund_ratio = payload.exp_refund_ratio.unwrap_or(DEFAULT_EXP_REFUND_RATIO);
    let cost_weights = CostWeightsOutput {
        w_echo: payload.cost_weights.w_echo,
        w_tuner: payload.cost_weights.w_tuner,
        w_exp: payload.cost_weights.w_exp,
    };

    let cost_model = CostModel::new(
        cost_weights.w_echo,
        cost_weights.w_tuner,
        cost_weights.w_exp,
        exp_refund_ratio,
    )
    .map_err(|err| format!("Invalid cost model: {err:?}"))?;
    let scorer_type = parse_scorer_type(&payload.scorer_type)?;
    let scorer_config = build_upgrade_scorer_config_from_inputs(
        scorer_type,
        &payload.buff_weights,
        payload.main_buff_score,
        payload.normalized_max_score,
    )?;
    let scorer = build_upgrade_scorer(&scorer_config)?;
    let (summary_target_score, solver_target_score) =
        resolve_target_scores(&scorer_config, &scorer, payload.target_score)?;

    let mut current_upgrade = state
        .current_upgrade
        .lock()
        .map_err(|_| "Failed to lock current upgrade solver".to_string())?;

    let reuse_existing = current_upgrade.as_ref().is_some_and(|session| {
        can_reuse_upgrade_solver(
            session,
            &scorer_config,
            payload.blend_data,
            &cost_weights,
            exp_refund_ratio,
        )
    });

    if reuse_existing {
        let session = current_upgrade
            .as_mut()
            .ok_or_else(|| "Upgrade solver session was not initialized".to_string())?;
        session
            .solver
            .update_target_score(solver_target_score)
            .map_err(|err| format!("Failed to update target score: {err:?}"))?;
        session.target_score = summary_target_score;
    } else {
        let solver =
            build_upgrade_solver(&scorer, payload.blend_data, solver_target_score, cost_model)?;
        *current_upgrade = Some(SolverSession {
            solver,
            target_score: summary_target_score,
            scorer_config,
            query_scorer: scorer,
            blend_data: payload.blend_data,
            cost_weights,
            exp_refund_ratio,
        });
    }

    let session = current_upgrade
        .as_mut()
        .ok_or_else(|| "Upgrade solver session was not initialized".to_string())?;
    let start = Instant::now();
    let lambda_star = session
        .solver
        .lambda_search(payload.lambda_tolerance, payload.lambda_max_iter)
        .map_err(|err| format!("Failed during lambda search: {err:?}"))?;
    let expected = session
        .solver
        .calculate_expected_resources()
        .map_err(|err| format!("Failed to compute expected resources: {err:?}"))?;
    let expected_cost_per_success = session
        .solver
        .weighted_expected_cost()
        .map_err(|err| format!("Failed to compute weighted expected cost: {err:?}"))?;
    let compute_seconds = start.elapsed().as_secs_f64();

    let summary = PolicySummary {
        target_score: summary_target_score,
        lambda_star,
        expected_cost_per_success,
        compute_seconds,
        success_probability: expected.success_probability(),
        echo_per_success: expected.echo_per_success(),
        tuner_per_success: expected.tuner_per_success(),
        exp_per_success: expected.exp_per_success(),
        cost_weights,
        exp_refund_ratio,
    };

    Ok(ComputePolicyResponse { summary })
}

#[tauri::command]
fn policy_suggestion(
    state: State<'_, AppState>,
    payload: PolicySuggestionRequest,
) -> Result<PolicySuggestionResponse, String> {
    if !payload.buff_names.is_empty() && payload.buff_values.len() != payload.buff_names.len() {
        return Err("buffNames and buffValues must have the same length".to_string());
    }

    let current_upgrade = state
        .current_upgrade
        .lock()
        .map_err(|_| "Failed to lock current upgrade solver".to_string())?;
    let session = current_upgrade.as_ref().ok_or_else(|| {
        "No computed upgrade policy in memory. Please compute policy first.".to_string()
    })?;

    let mask = build_mask(&payload.buff_names)?;
    let score_scaled = if !payload.buff_names.is_empty() {
        score_from_selected_buffs_for_solver(
            &session.query_scorer,
            &payload.buff_names,
            &payload.buff_values,
        )?
    } else {
        0
    };

    let decision = if payload.buff_names.is_empty() {
        true
    } else {
        session
            .solver
            .get_decision(mask, score_scaled)
            .map_err(|err| format!("Failed to query suggestion: {err:?}"))?
    };
    let success_probability = session
        .solver
        .get_success_probability(mask, score_scaled)
        .map_err(|err| format!("Failed to query success probability: {err:?}"))?;

    Ok(PolicySuggestionResponse {
        suggestion: if decision {
            "Continue".to_string()
        } else {
            "Abandon".to_string()
        },
        stage: payload.buff_names.len(),
        target_score: session.target_score,
        success_probability,
        mask_bits: mask_to_bits(mask).to_vec(),
    })
}

