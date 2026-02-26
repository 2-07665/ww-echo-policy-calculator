fn build_upgrade_scorer_config_from_inputs(
    scorer_type: &str,
    buff_weights: &HashMap<String, f64>,
    main_buff_score: Option<f64>,
    normalized_max_score: Option<f64>,
) -> Result<UpgradeScorerConfig, String> {
    match scorer_type {
        SCORER_TYPE_LINEAR_DEFAULT => {
            let weights = build_weight_array_f64(buff_weights, DEFAULT_LINEAR_BUFF_WEIGHTS)?;
            let main_buff_score = main_buff_score.unwrap_or(DEFAULT_LINEAR_MAIN_BUFF_SCORE);
            let normalized_max_score =
                normalized_max_score.unwrap_or(DEFAULT_LINEAR_NORMALIZED_MAX_SCORE);
            Ok(UpgradeScorerConfig::LinearDefault {
                weights,
                main_buff_score,
                normalized_max_score,
            })
        }
        SCORER_TYPE_WUWA_ECHO_TOOL => {
            let weights = build_weight_array_f64(buff_weights, DEFAULT_WUWA_ECHO_TOOL_BUFF_WEIGHTS)?;
            let main_buff_score =
                main_buff_score.unwrap_or(DEFAULT_WUWA_ECHO_TOOL_MAIN_BUFF_SCORE);
            let normalized_max_score =
                normalized_max_score.unwrap_or(DEFAULT_WUWA_ECHO_TOOL_NORMALIZED_MAX_SCORE);
            Ok(UpgradeScorerConfig::WuwaEchoTool {
                weights,
                main_buff_score,
                normalized_max_score,
            })
        }
        SCORER_TYPE_MC_BOOST_ASSISTANT => {
            let weights =
                build_weight_array_f64(buff_weights, DEFAULT_MC_BOOST_ASSISTANT_BUFF_WEIGHTS)?;
            Ok(UpgradeScorerConfig::McBoostAssistant { weights })
        }
        SCORER_TYPE_QQ_BOT => {
            let qq_bot_weights = build_weight_array_f64(buff_weights, DEFAULT_QQ_BOT_BUFF_WEIGHTS)?;
            let main_buff_score = main_buff_score.unwrap_or(DEFAULT_QQ_BOT_MAIN_BUFF_SCORE);
            let normalized_max_score =
                normalized_max_score.unwrap_or(DEFAULT_QQ_BOT_NORMALIZED_MAX_SCORE);
            if !normalized_max_score.is_finite() || normalized_max_score <= 0.0 {
                return Err("normalizedMaxScore must be a positive finite number".to_string());
            }
            Ok(UpgradeScorerConfig::QQBot {
                qq_bot_weights,
                main_buff_score,
                normalized_max_score,
            })
        }
        SCORER_TYPE_FIXED => {
            let weights =
                build_weight_array_u16_from_f64(buff_weights, DEFAULT_FIXED_BUFF_WEIGHTS)?;
            Ok(UpgradeScorerConfig::Fixed { weights })
        }
        _ => unreachable!(),
    }
}

fn build_upgrade_scorer(config: &UpgradeScorerConfig) -> Result<UpgradeScorer, String> {
    match config {
        UpgradeScorerConfig::LinearDefault {
            weights,
            main_buff_score,
            normalized_max_score,
        } => Ok(UpgradeScorer::Linear(build_default_linear_scorer(
            *weights,
            *main_buff_score,
            *normalized_max_score,
        )?)),
        UpgradeScorerConfig::WuwaEchoTool {
            weights,
            main_buff_score,
            normalized_max_score,
        } => Ok(UpgradeScorer::Linear(build_wuwa_echo_tool_scorer(
            *weights,
            *main_buff_score,
            *normalized_max_score,
        )?)),
        UpgradeScorerConfig::McBoostAssistant { weights } => Ok(UpgradeScorer::Linear(
            build_mc_boost_assistant_scorer(*weights)?,
        )),
        UpgradeScorerConfig::QQBot {
            qq_bot_weights,
            main_buff_score,
            ..
        } => Ok(UpgradeScorer::Linear(build_qq_bot_scorer(
            *qq_bot_weights,
            *main_buff_score,
        )?)),
        UpgradeScorerConfig::Fixed { weights } => {
            let scorer = FixedScorer::new(*weights)
                .map_err(|err| format!("Invalid fixed scorer: {err:?}"))?;
            Ok(UpgradeScorer::Fixed(scorer))
        }
    }
}

fn build_upgrade_solver(
    scorer: &UpgradeScorer,
    blend_data: bool,
    target_score_display: f64,
    cost_model: CostModel,
) -> Result<UpgradePolicySolver, String> {
    match scorer {
        UpgradeScorer::Linear(linear) => {
            UpgradePolicySolver::new(linear, blend_data, target_score_display, cost_model)
                .map_err(|err| format!("Failed to create solver: {err:?}"))
        }
        UpgradeScorer::Fixed(fixed) => {
            UpgradePolicySolver::new(fixed, blend_data, target_score_display, cost_model)
                .map_err(|err| format!("Failed to create solver: {err:?}"))
        }
    }
}

fn resolve_target_scores(
    scorer_config: &UpgradeScorerConfig,
    scorer: &UpgradeScorer,
    raw_target_score: f64,
) -> Result<(f64, f64), String> {
    match scorer_config {
        UpgradeScorerConfig::Fixed { .. } => {
            let target_score = parse_u16_from_f64(raw_target_score, "targetScore")?;
            let summary_target = f64::from(target_score);
            Ok((summary_target, summary_target / SCORE_MULTIPLIER))
        }
        UpgradeScorerConfig::QQBot {
            normalized_max_score,
            ..
        } => {
            if !raw_target_score.is_finite() || raw_target_score < 0.0 {
                return Err("targetScore must be a non-negative finite number".to_string());
            }
            let score_scale = *normalized_max_score / DEFAULT_QQ_BOT_NORMALIZED_MAX_SCORE;
            let target_on_solver_scale = raw_target_score / score_scale;
            let main_score = match scorer {
                UpgradeScorer::Linear(linear) => linear.main_buff_score(),
                UpgradeScorer::Fixed(_) => unreachable!(),
            };
            Ok((
                raw_target_score,
                (target_on_solver_scale - main_score).max(0.0),
            ))
        }
        UpgradeScorerConfig::LinearDefault { .. }
        | UpgradeScorerConfig::WuwaEchoTool { .. }
        | UpgradeScorerConfig::McBoostAssistant { .. } => {
            if !raw_target_score.is_finite() || raw_target_score < 0.0 {
                return Err("targetScore must be a non-negative finite number".to_string());
            }
            let main_score = match scorer {
                UpgradeScorer::Linear(linear) => linear.main_buff_score(),
                UpgradeScorer::Fixed(_) => unreachable!(),
            };
            Ok((raw_target_score, (raw_target_score - main_score).max(0.0)))
        }
    }
}

fn can_reuse_upgrade_solver(
    session: &SolverSession,
    scorer: &UpgradeScorerConfig,
    blend_data: bool,
    cost_weights: &CostWeightsOutput,
    exp_refund_ratio: f64,
) -> bool {
    scorer_configs_equal(&session.scorer_config, scorer)
        && session.blend_data == blend_data
        && cost_weights_equal(&session.cost_weights, cost_weights)
        && f64_bits_equal(session.exp_refund_ratio, exp_refund_ratio)
}

