use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

use echo_policy::{CostModel, LinearScorer, UpgradePolicySolver};
use serde::Deserialize;

const LAMBDA_TOLERANCE: f64 = 1e-6;
const LAMBDA_MAX_ITER: usize = 100;
const EXP_REFUND_RATIO_MAX: f64 = 0.75;
const EXP_REFUND_RATIO_DEFAULT: f64 = 0.66;
const QQ_BOT_MAIN_BUFF_SCORE_DEFAULT: f64 = 14.25;

const BUFF_KEYS: [&str; 13] = [
    "Crit_Rate",
    "Crit_Damage",
    "Attack",
    "Defence",
    "HP",
    "Attack_Flat",
    "Defence_Flat",
    "HP_Flat",
    "ER",
    "Basic_Attack_Damage",
    "Heavy_Attack_Damage",
    "Skill_Damage",
    "Ult_Damage",
];

const RESULT_COLUMNS: [&str; 7] = [
    "targetScore",
    "lambda",
    "weightedExpectedCost",
    "successProbability",
    "echoPerSuccess",
    "tunerPerSuccess",
    "expPerSuccess",
];

#[derive(Deserialize)]
struct SweepConfig {
    #[serde(default)]
    blend_data: bool,
    #[serde(default = "default_lambda_tolerance")]
    lambda_tolerance: f64,
    #[serde(default = "default_lambda_max_iter")]
    lambda_max_iter: usize,
    scorer: ScorerConfig,
    cost_model: CostModelConfig,
    scan: ScanConfig,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ScorerConfig {
    Default {
        weights: WeightConfig,
    },
    QqBot {
        weights: WeightConfig,
        #[serde(default = "default_qq_main_buff_score")]
        main_buff_score: f64,
    },
    McBoostAssistant {
        weights: WeightConfig,
    },
}

impl ScorerConfig {
    fn build(&self) -> Result<LinearScorer, String> {
        match self {
            Self::Default { weights } => LinearScorer::default(resolve_weights(weights)?)
                .map_err(|err| format!("invalid Default scorer weights: {err:?}")),
            Self::QqBot {
                weights,
                main_buff_score,
            } => LinearScorer::qq_bot_scorer(resolve_weights(weights)?, *main_buff_score)
                .map_err(|err| format!("invalid QQ Bot scorer configuration: {err:?}")),
            Self::McBoostAssistant { weights } => {
                LinearScorer::mc_boost_assistant_scorer(resolve_weights(weights)?)
                    .map_err(|err| format!("invalid MC Boost Assistant scorer weights: {err:?}"))
            }
        }
    }

    fn resolve_solver_target_score(
        &self,
        scorer: &LinearScorer,
        display_target_score: f64,
    ) -> f64 {
        match self {
            Self::Default { .. } | Self::McBoostAssistant { .. } => {
                (display_target_score - scorer.main_buff_score()).max(0.0)
            }
            Self::QqBot { .. } => {
                let score_scale = scorer.normalized_max_score() / 50.0;
                let target_on_solver_scale = display_target_score / score_scale;
                (target_on_solver_scale - scorer.main_buff_score()).max(0.0)
            }
        }
    }
}

#[derive(Deserialize)]
struct CostModelConfig {
    weight_echo: f64,
    weight_tuner: f64,
    weight_exp: f64,
    #[serde(default = "default_exp_refund_ratio")]
    exp_refund_ratio: f64,
}

impl CostModelConfig {
    fn build(&self) -> Result<CostModel, String> {
        if !(0.0..=EXP_REFUND_RATIO_MAX).contains(&self.exp_refund_ratio) {
            return Err(format!(
                "exp_refund_ratio must be in [0, {EXP_REFUND_RATIO_MAX}]"
            ));
        }
        CostModel::new(
            self.weight_echo,
            self.weight_tuner,
            self.weight_exp,
            self.exp_refund_ratio,
        )
        .map_err(|err| format!("invalid cost model: {err:?}"))
    }

}

#[derive(Deserialize)]
struct ScanConfig {
    start: f64,
    end: f64,
    step: f64,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum WeightConfig {
    Array([f64; 13]),
    Map(BTreeMap<String, f64>),
}

struct SweepRow {
    target_score: f64,
    lambda: f64,
    weighted_expected_cost: f64,
    success_probability: f64,
    echo_per_success: f64,
    tuner_per_success: f64,
    exp_per_success: f64,
}

fn default_lambda_tolerance() -> f64 {
    LAMBDA_TOLERANCE
}

fn default_lambda_max_iter() -> usize {
    LAMBDA_MAX_ITER
}

fn default_exp_refund_ratio() -> f64 {
    EXP_REFUND_RATIO_DEFAULT
}

fn default_qq_main_buff_score() -> f64 {
    QQ_BOT_MAIN_BUFF_SCORE_DEFAULT
}

fn main() {
    let exit_code = match run() {
        Ok(()) => 0,
        Err(RunError::Usage(message)) => {
            println!("{message}");
            0
        }
        Err(RunError::Execution(message)) => {
        eprintln!("error: {message}");
            1
        }
    };
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

enum RunError {
    Usage(String),
    Execution(String),
}

fn run() -> Result<(), RunError> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "target_score_sweep".to_string());
    let config_path = args
        .next()
        .ok_or_else(|| RunError::Usage(format!("usage: {program} <config.json> [output.wl]")))?;
    let output_path = args.next();
    if args.next().is_some() {
        return Err(RunError::Usage(format!(
            "usage: {program} <config.json> [output.wl]"
        )));
    }

    let config_text = fs::read_to_string(&config_path)
        .map_err(|err| RunError::Execution(format!("failed to read config {config_path}: {err}")))?;
    let config: SweepConfig = serde_json::from_str(&config_text)
        .map_err(|err| RunError::Execution(format!("failed to parse config {config_path}: {err}")))?;

    validate_scan_config(&config.scan).map_err(RunError::Execution)?;

    let scorer = config.scorer.build().map_err(RunError::Execution)?;
    let cost_model = config.cost_model.build().map_err(RunError::Execution)?;
    let target_scores = build_target_scores(&config.scan).map_err(RunError::Execution)?;
    if target_scores.is_empty() {
        return Err(RunError::Execution("scan produced no target scores".to_string()));
    }

    let first_solver_target = config
        .scorer
        .resolve_solver_target_score(&scorer, target_scores[0]);
    let mut solver = UpgradePolicySolver::new(
        &scorer,
        config.blend_data,
        first_solver_target,
        cost_model,
    )
    .map_err(|err| RunError::Execution(format!("failed to build upgrade policy solver: {err:?}")))?;

    let mut rows = Vec::with_capacity(target_scores.len());
    for (index, target_score) in target_scores.into_iter().enumerate() {
        let solver_target_score = config
            .scorer
            .resolve_solver_target_score(&scorer, target_score);
        if index > 0 {
            solver
                .update_target_score(solver_target_score)
                .map_err(|err| {
                    RunError::Execution(format!(
                        "failed to update target score {target_score}: {err:?}"
                    ))
                })?;
        }

        let lambda = solver
            .lambda_search(config.lambda_tolerance, config.lambda_max_iter)
            .map_err(|err| {
                RunError::Execution(format!(
                    "lambda_search failed for target_score={target_score}: {err:?}"
                ))
            })?;
        let weighted_expected_cost = solver
            .weighted_expected_cost()
            .map_err(|err| {
                RunError::Execution(format!(
                    "failed to read weighted expected cost for target_score={target_score}: {err:?}"
                ))
            })?;
        let expected_cost = solver
            .calculate_expected_resources()
            .map_err(|err| {
                RunError::Execution(format!(
                    "failed to calculate expected resources for target_score={target_score}: {err:?}"
                ))
            })?;

        rows.push(SweepRow {
            target_score,
            lambda,
            weighted_expected_cost,
            success_probability: expected_cost.success_probability(),
            echo_per_success: expected_cost.echo_per_success(),
            tuner_per_success: expected_cost.tuner_per_success(),
            exp_per_success: expected_cost.exp_per_success(),
        });
    }

    let output = format_wolfram_output(&config, &rows);
    match output_path {
        Some(path) => {
            fs::write(&path, output).map_err(|err| {
                RunError::Execution(format!(
                    "failed to write output {}: {err}",
                    Path::new(&path).display()
                ))
            })?;
            eprintln!(
                "wrote {} rows to {}",
                rows.len(),
                Path::new(&path).display()
            );
        }
        None => {
            print!("{output}");
        }
    }

    Ok(())
}

fn validate_scan_config(scan: &ScanConfig) -> Result<(), String> {
    if !scan.start.is_finite() || !scan.end.is_finite() || !scan.step.is_finite() {
        return Err("scan.start, scan.end, and scan.step must be finite numbers".to_string());
    }
    if scan.step <= 0.0 {
        return Err("scan.step must be > 0".to_string());
    }
    if scan.end < scan.start {
        return Err("scan.end must be >= scan.start".to_string());
    }
    if scan.step > scan.end - scan.start && scan.start < scan.end {
        return Ok(());
    }
    Ok(())
}

fn build_target_scores(scan: &ScanConfig) -> Result<Vec<f64>, String> {
    let mut values = Vec::new();
    let mut current = scan.start;
    let epsilon = scan.step.abs() * 1e-9 + 1e-12;
    let max_iters = 1_000_000usize;

    for _ in 0..max_iters {
        if current > scan.end + epsilon {
            break;
        }
        values.push(round_display_score(current));
        current += scan.step;
    }

    if values.is_empty() {
        return Err("scan produced no points".to_string());
    }
    Ok(values)
}

fn round_display_score(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn resolve_weights(weights: &WeightConfig) -> Result<[f64; 13], String> {
    match weights {
        WeightConfig::Array(array) => Ok(*array),
        WeightConfig::Map(map) => {
            let mut resolved = [0.0; 13];
            for (index, key) in BUFF_KEYS.iter().enumerate() {
                resolved[index] = *map
                    .get(*key)
                    .ok_or_else(|| format!("missing weight for key `{key}`"))?;
            }
            Ok(resolved)
        }
    }
}

fn format_wolfram_output(config: &SweepConfig, rows: &[SweepRow]) -> String {
    let mut out = String::new();
    out.push_str("<|");
    out.push_str("\"config\" -> ");
    out.push_str(&format_config(config));
    out.push_str(",\n\"results\" -> {\n");

    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str("  ");
        out.push_str(&format_result_row(row));
    }

    out.push_str("\n}|>\n");
    out
}

fn format_config(config: &SweepConfig) -> String {
    let scorer = match &config.scorer {
        ScorerConfig::Default { weights } => format!(
            "<|\"type\" -> \"default\", \"weights\" -> {}|>",
            format_weights(weights)
        ),
        ScorerConfig::QqBot {
            weights,
            main_buff_score,
        } => format!(
            "<|\"type\" -> \"qq_bot\", \"mainBuffScore\" -> {}, \"weights\" -> {}|>",
            format_number(*main_buff_score),
            format_weights(weights)
        ),
        ScorerConfig::McBoostAssistant { weights } => format!(
            "<|\"type\" -> \"mc_boost_assistant\", \"weights\" -> {}|>",
            format_weights(weights)
        ),
    };

    let cost_model = format!(
        "<|\"weightEcho\" -> {}, \"weightTuner\" -> {}, \"weightExp\" -> {}, \"expRefundRatio\" -> {}|>",
        format_number(config.cost_model.weight_echo),
        format_number(config.cost_model.weight_tuner),
        format_number(config.cost_model.weight_exp),
        format_number(config.cost_model.exp_refund_ratio)
    );

    format!(
        "<|\"blendData\" -> {}, \"lambdaTolerance\" -> {}, \"lambdaMaxIter\" -> {}, \"resultColumns\" -> {}, \"scorer\" -> {}, \"costModel\" -> {}, \"scan\" -> <|\"start\" -> {}, \"end\" -> {}, \"step\" -> {}|>|>",
        if config.blend_data { "True" } else { "False" },
        format_number(config.lambda_tolerance),
        config.lambda_max_iter,
        format_result_columns(),
        scorer,
        cost_model,
        format_number(config.scan.start),
        format_number(config.scan.end),
        format_number(config.scan.step)
    )
}

fn format_result_columns() -> String {
    let mut out = String::from("{");
    for (index, column) in RESULT_COLUMNS.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push('"');
        out.push_str(column);
        out.push('"');
    }
    out.push('}');
    out
}

fn format_weights(weights: &WeightConfig) -> String {
    let resolved = match resolve_weights(weights) {
        Ok(weights) => weights,
        Err(_) => return "$Failed".to_string(),
    };

    let mut out = String::from("<|");
    for (index, key) in BUFF_KEYS.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push('"');
        out.push_str(key);
        out.push_str("\" -> ");
        out.push_str(&format_number(resolved[index]));
    }
    out.push_str("|>");
    out
}

fn format_result_row(row: &SweepRow) -> String {
    format!(
        "{{{}, {}, {}, {}, {}, {}, {}}}",
        format_number(row.target_score),
        format_number(row.lambda),
        format_number(row.weighted_expected_cost),
        format_number(row.success_probability),
        format_number(row.echo_per_success),
        format_number(row.tuner_per_success),
        format_number(row.exp_per_success)
    )
}

fn format_number(value: f64) -> String {
    let rounded = if value == 0.0 { 0.0 } else { value };
    let mut text = format!("{rounded:.12}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.push('0');
    }
    text
}
