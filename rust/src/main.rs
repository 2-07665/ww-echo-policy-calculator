use ww_echo_policy::CostModel;
use ww_echo_policy::PolicySolver;
use ww_echo_policy::{LinearScorer, Scorer};

use std::time::Instant;

fn main() {
    const BUFF_WEIGHTS: [f64; 13] = [
        1000.0, // Crit. Rate
        1000.0, // Crit. DMG
        0.0,    // ATK%
        0.0,    // DEF%
        443.0,  // HP%
        0.0,    // ATK
        0.0,    // DEF
        150.0,  // HP
        0.0,    // Energy Regen
        277.0,  // Basic Attack DMG Bonus
        0.0,    // Heavy Attack DMG Bonus
        51.0,   // Resonance Skill DMG Bonus
        224.0,  // Resonance Liberation DMG Bonus
    ];

    let pmfs = LinearScorer::new(BUFF_WEIGHTS)
        .unwrap()
        .build_score_pmfs(false);

    let cost_model = CostModel::new(10.0, 1.0, 0.5, 0.66).unwrap();

    let target_score: f64 = 72.0;

    let mut solver = PolicySolver::new(pmfs, target_score, cost_model).unwrap();

    let start = Instant::now();
    let lambda = solver
        .lambda_search(0.0, 1.0, 1e-6, 100)
        .expect("lambda search failed");
    let elapsed = start.elapsed();

    println!("target_score={:.2}", target_score);
    println!("lambda_star={lambda:.6}");
    println!(
        "lambda search completed in {:.3} milliseconds",
        elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "weighted expected cost per success={:.2}",
        solver.weighted_expected_cost().unwrap()
    );

    let start = Instant::now();
    let expected_cost = solver.expected_resources().unwrap();
    let elapsed = start.elapsed();
    println!(
        "expected resource computation completed in {:.3} milliseconds",
        elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "success probability={:.4}%",
        expected_cost.success_prob * 100.0
    );
    println!(
        "expected resource cost per success: echo={:.2} tuner={:.2} exp={:.2}",
        1.0 / expected_cost.success_prob,
        expected_cost.tuner_per_succ,
        expected_cost.exp_per_succ
    );
}
