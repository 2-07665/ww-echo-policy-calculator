use std::io::{self, Write};

use echo_policy::{CostModel, LinearScorer, UpgradePolicySolver};

const BUFF_LABELS: [&str; 13] = [
    "暴击",
    "暴击伤害",
    "攻击百分比",
    "防御百分比",
    "生命百分比",
    "攻击",
    "防御",
    "生命",
    "共鸣效率",
    "普攻伤害加成",
    "重击伤害加成",
    "共鸣技能伤害加成",
    "共鸣解放伤害加成",
];

const LAMBDA_TOLERANCE: f64 = 1e-6;
const LAMBDA_MAX_ITER: usize = 100;
const EXP_REFUND_RATIO_MAX: f64 = 0.75;
const EXP_REFUND_RATIO_DEFAULT: f64 = 0.66;
const QQ_BOT_MAIN_BUFF_SCORE_DEFAULT: f64 = 14.25;
const DIVIDER: &str = "========================================";

#[derive(Clone, Copy)]
enum ScorerChoice {
    Default,
    QqBot,
    McBoostAssistant,
}

impl ScorerChoice {
    fn label(self) -> &'static str {
        match self {
            Self::Default => "自定义",
            Self::QqBot => "QQ机器人",
            Self::McBoostAssistant => "漂泊者强化助手",
        }
    }
}

enum CostModelChoice {
    TunerOnly,
    Custom {
        weight_echo: f64,
        weight_tuner: f64,
        weight_exp: f64,
        exp_refund_ratio: f64,
    },
}

impl CostModelChoice {
    fn build(&self) -> Result<CostModel, String> {
        match self {
            Self::TunerOnly => Ok(CostModel::tuner_only()),
            Self::Custom {
                weight_echo,
                weight_tuner,
                weight_exp,
                exp_refund_ratio,
            } => CostModel::new(
                *weight_echo,
                *weight_tuner,
                *weight_exp,
                *exp_refund_ratio,
            )
            .map_err(|err| format!("invalid custom cost model: {err:?}")),
        }
    }

    fn describe(&self) -> String {
        match self {
            Self::TunerOnly => "tuner_only".to_string(),
            Self::Custom {
                weight_echo,
                weight_tuner,
                weight_exp,
                exp_refund_ratio,
            } => format!(
                "custom (echo={weight_echo:.4}, tuner={weight_tuner:.4}, exp={weight_exp:.4}, exp_refund_ratio={exp_refund_ratio:.4})"
            ),
        }
    }
}

fn main() {
    if let Err(message) = run() {
        eprintln!("发生错误: {message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    print_banner();

    print_section("Step 1/4 评分模型");
    let choice = prompt_scorer_choice().map_err(|err| err.to_string())?;
    println!();

    print_section("Step 2/4 词条权重");
    let weights = prompt_weights().map_err(|err| err.to_string())?;
    let qq_main_buff_score = if matches!(choice, ScorerChoice::QqBot) {
        Some(
            prompt_f64_in_range(
                "QQ Bot 主词条分",
                0.0,
                f64::MAX,
                Some(QQ_BOT_MAIN_BUFF_SCORE_DEFAULT),
                Some("默认 14.25"),
            )
            .map_err(|err| err.to_string())?,
        )
    } else {
        None
    };
    println!();

    print_section("Step 3/4 资源成本模型");
    let cost_model_choice = prompt_cost_model_choice().map_err(|err| err.to_string())?;
    println!();

    let scorer = build_scorer(choice, weights, qq_main_buff_score)?;
    let cost_model = cost_model_choice.build()?;

    print_section("Step 4/4 目标分数");
    let target_score = prompt_target_score().map_err(|err| err.to_string())?;
    let solver_target_score = resolve_solver_target_score(&scorer, target_score);
    println!();

    print_section("开始计算");
    println!("评分模型: {}", choice.label());
    println!("成本模型: {}", cost_model_choice.describe());
    println!("目标分数: {target_score:.2}");
    println!();

    let mut solver = UpgradePolicySolver::new(&scorer, false, solver_target_score, cost_model)
        .map_err(|err| format!("failed to build upgrade policy solver: {err:?}"))?;
    let lambda = solver
        .lambda_search(LAMBDA_TOLERANCE, LAMBDA_MAX_ITER)
        .map_err(|err| format!("lambda_search failed: {err:?}"))?;
    let weighted_expected_cost = solver
        .weighted_expected_cost()
        .map_err(|err| format!("failed to read weighted expected cost: {err:?}"))?;
    let expected_cost = solver
        .calculate_expected_resources()
        .map_err(|err| format!("failed to calculate expected resources: {err:?}"))?;

    print_section("计算结果");
    println!("评分模型: {}", choice.label());
    println!("成本模型: {}", cost_model_choice.describe());
    println!("lambda: {lambda:.8}");
    println!("期望加权资源消耗: {weighted_expected_cost:.2}");
    println!(
        "成功率: {:.4}%",
        expected_cost.success_probability() * 100.0
    );
    println!();
    println!("期望资源消耗:");
    println!("  声骸胚子: {:.2}", expected_cost.echo_per_success());
    println!("  调谐器: {:.2}", expected_cost.tuner_per_success());
    println!("  金密音筒: {:.2}", expected_cost.exp_per_success());
    println!("{DIVIDER}");

    Ok(())
}

fn prompt_scorer_choice() -> io::Result<ScorerChoice> {
    loop {
        println!("请选择评分模型预设:");
        println!("  1. 自定义  (默认)");
        println!("  2. QQ机器人");
        println!("  3. 漂泊者强化助手");
        let input = prompt_line("选择", Some("输入 1/2/3"))?;
        match input.trim() {
            "" | "1" => return Ok(ScorerChoice::Default),
            "2" => return Ok(ScorerChoice::QqBot),
            "3" => return Ok(ScorerChoice::McBoostAssistant),
            _ => {
                println!("请输入 1、2 或 3。");
                println!();
            }
        }
    }
}

fn prompt_weights() -> io::Result<[f64; 13]> {
    let mut weights = [0.0; 13];
    println!("请输入各副词条权重 (默认 0，至少一个大于 0)。");
    for (index, weight) in weights.iter_mut().enumerate() {
        *weight = prompt_nonnegative_f64(&format!("{:>2}. {}", index + 1, BUFF_LABELS[index]), None)?;
    }
    if !weights.iter().any(|&weight| weight > 0.0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "至少需要一个大于 0 的词条权重。",
        ));
    }
    println!();
    Ok(weights)
}

fn prompt_cost_model_choice() -> io::Result<CostModelChoice> {
    loop {
        println!("请选择资源成本模型:");
        println!("  1. 仅调谐器  (默认)");
        println!("  2. 自定义");
        let input = prompt_line("选择", Some("输入 1/2"))?;
        match input.trim() {
            "" | "1" => {
                println!();
                return Ok(CostModelChoice::TunerOnly);
            }
            "2" => {
                println!();
                println!("请输入自定义成本权重 (默认 0):");
                println!("留空按 0 处理。");
                let weight_echo = prompt_nonnegative_f64("  声骸胚子权重", None)?;
                let weight_tuner = prompt_nonnegative_f64("  调谐器权重", None)?;
                let weight_exp = prompt_nonnegative_f64("  金密音筒权重", None)?;
                let exp_refund_ratio = prompt_f64_in_range(
                    "  经验值返还比例",
                    0.0,
                    EXP_REFUND_RATIO_MAX,
                    Some(EXP_REFUND_RATIO_DEFAULT),
                    Some("默认 0.66，上限 0.75"),
                )?;
                println!();
                return Ok(CostModelChoice::Custom {
                    weight_echo,
                    weight_tuner,
                    weight_exp,
                    exp_refund_ratio,
                });
            }
            _ => {
                println!("请输入 1 或 2。");
                println!();
            }
        }
    }
}

fn prompt_target_score() -> io::Result<f64> {
    prompt_nonnegative_f64("目标分数", None)
}

fn resolve_solver_target_score(scorer: &LinearScorer, display_target_score: f64) -> f64 {
    (display_target_score - scorer.main_buff_score()).max(0.0)
}

fn build_scorer(
    choice: ScorerChoice,
    weights: [f64; 13],
    qq_main_buff_score: Option<f64>,
) -> Result<LinearScorer, String> {
    match choice {
        ScorerChoice::Default => LinearScorer::default(weights)
            .map_err(|err| format!("invalid Default scorer weights: {err:?}")),
        ScorerChoice::QqBot => {
            let main_buff_score = qq_main_buff_score
                .ok_or_else(|| "missing QQ Bot main buff score".to_string())?;
            LinearScorer::qq_bot_scorer(weights, main_buff_score)
                .map_err(|err| format!("invalid QQ Bot scorer configuration: {err:?}"))
        }
        ScorerChoice::McBoostAssistant => LinearScorer::mc_boost_assistant_scorer(weights)
            .map_err(|err| format!("invalid MC_Boost_assistant scorer weights: {err:?}")),
    }
}

fn prompt_nonnegative_f64(prompt: &str, hint: Option<&str>) -> io::Result<f64> {
    loop {
        let input = prompt_line(prompt, hint)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(0.0);
        }
        match trimmed.parse::<f64>() {
            Ok(value) if value.is_finite() && value >= 0.0 => return Ok(value),
            _ => {
                println!("请输入大于等于 0 的有限数字。");
                println!();
            }
        }
    }
}

fn prompt_f64_in_range(
    prompt: &str,
    min: f64,
    max: f64,
    default: Option<f64>,
    hint: Option<&str>,
) -> io::Result<f64> {
    loop {
        let input = prompt_line(prompt, hint)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            if let Some(value) = default {
                return Ok(value);
            }
        }
        match trimmed.parse::<f64>() {
            Ok(value) if value.is_finite() && (min..=max).contains(&value) => return Ok(value),
            _ => {
                println!("请输入 [{min}, {max}] 范围内的有限数字。");
                println!();
            }
        }
    }
}

fn print_banner() {
    println!("{DIVIDER}");
    println!("鸣潮声骸强化策略计算器 CLI");
    println!("说明: 输入项可留空使用默认值。");
    println!("{DIVIDER}");
    println!();
}

fn print_section(title: &str) {
    println!("{DIVIDER}");
    println!("{title}");
    println!("{DIVIDER}");
}

fn prompt_line(prompt: &str, hint: Option<&str>) -> io::Result<String> {
    match hint {
        Some(hint) => print!("{prompt} ({hint}): "),
        None => print!("{prompt}: "),
    }
    io::stdout().flush()?;

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer)
}
