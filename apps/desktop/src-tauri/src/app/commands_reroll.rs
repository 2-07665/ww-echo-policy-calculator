fn configure_and_derive_reroll_policy(
    solver: &mut RerollPolicySolver,
    target_score: u16,
) -> Result<(), String> {
    solver
        .set_target(target_score)
        .map_err(|err| format!("Failed to set reroll target: {err:?}"))?;
    solver
        .derive_policy(1e-4, 200)
        .map_err(|err| format!("Failed to derive reroll policy: {err:?}"))?;
    Ok(())
}

#[tauri::command]
fn compute_reroll_policy(
    state: State<'_, AppState>,
    payload: ComputeRerollPolicyRequest,
) -> Result<ComputeRerollPolicyResponse, String> {
    let weights = build_weight_array_u16(&payload.buff_weights, DEFAULT_FIXED_BUFF_WEIGHTS)?;

    let mut current_reroll = state
        .current_reroll
        .lock()
        .map_err(|_| "Failed to lock current reroll solver".to_string())?;

    let reuse_existing = current_reroll
        .as_ref()
        .is_some_and(|session| session.weights == weights);

    if reuse_existing {
        let session = current_reroll
            .as_mut()
            .ok_or_else(|| "Reroll solver session was not initialized".to_string())?;
        configure_and_derive_reroll_policy(&mut session.solver, payload.target_score)?;
    } else {
        let mut solver = RerollPolicySolver::new(weights)
            .map_err(|err| format!("Failed to create reroll solver: {err:?}"))?;
        configure_and_derive_reroll_policy(&mut solver, payload.target_score)?;
        let scorer =
            FixedScorer::new(weights).map_err(|err| format!("Invalid fixed scorer: {err:?}"))?;
        *current_reroll = Some(RerollSession {
            solver,
            weights,
            scorer,
        });
    }

    Ok(ComputeRerollPolicyResponse {
        target_score: payload.target_score,
    })
}

#[tauri::command]
fn query_reroll_recommendation(
    state: State<'_, AppState>,
    payload: QueryRerollRecommendationRequest,
) -> Result<RerollRecommendationResponse, String> {
    let current_reroll = state
        .current_reroll
        .lock()
        .map_err(|_| "Failed to lock current reroll solver".to_string())?;
    let session = current_reroll.as_ref().ok_or_else(|| {
        "No computed reroll policy in memory. Please compute reroll policy first.".to_string()
    })?;

    let baseline_filled = payload.baseline_buff_names.len() == MAX_SELECTED_TYPES
        && payload
            .baseline_buff_names
            .iter()
            .all(|name| !name.is_empty());
    let candidate_filled = payload.candidate_buff_names.len() == MAX_SELECTED_TYPES
        && payload
            .candidate_buff_names
            .iter()
            .all(|name| !name.is_empty());

    if !baseline_filled {
        return Ok(RerollRecommendationResponse {
            valid: false,
            reason: Some("Baseline must have 5 buff types.".to_string()),
            baseline_score: 0,
            candidate_score: None,
            recommended_lock_choices: Vec::new(),
            accept_candidate: None,
        });
    }

    let baseline_mask = build_full_mask(&payload.baseline_buff_names)?;
    let baseline_score = fixed_score_from_selected(&session.scorer, &payload.baseline_buff_names)?;

    let default_top_k = default_reroll_top_k();
    let top_k = if payload.top_k == 0 {
        default_top_k
    } else {
        payload.top_k.min(default_top_k)
    };
    let choices = session
        .solver
        .lock_choices(baseline_mask, top_k)
        .map_err(|err| format!("Failed to query lock choices: {err:?}"))?;
    let recommended_lock_choices = choices
        .into_iter()
        .map(|choice| RerollChoiceResponse {
            lock_mask_bits: mask_to_bits(choice.lock_mask).to_vec(),
            lock_slot_indices: lock_slot_indices_from_mask(
                choice.lock_mask,
                &payload.baseline_buff_names,
            ),
            expected_cost: choice.expected_cost,
            regret: choice.regret,
            success_probability: choice.success_probability,
        })
        .collect();

    let (candidate_score, accept_candidate) = if candidate_filled {
        let candidate_mask = build_full_mask(&payload.candidate_buff_names)?;
        let score = fixed_score_from_selected(&session.scorer, &payload.candidate_buff_names)?;
        let accept = session
            .solver
            .should_accept(baseline_mask, candidate_mask)
            .map_err(|err| format!("Failed to compare baseline and candidate: {err:?}"))?;
        (Some(score), Some(accept))
    } else {
        (None, None)
    };

    Ok(RerollRecommendationResponse {
        valid: true,
        reason: None,
        baseline_score,
        candidate_score,
        recommended_lock_choices,
        accept_candidate,
    })
}
