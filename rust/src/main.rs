#[allow(dead_code)]
fn policy_exmaple() {
    use std::time::Instant;

    use ww_echo_policy::{CostModel, LinearScorer, Scorer, UpgradePolicySolver};
    use ww_echo_policy::{bits_to_mask, mask_to_bits};

    const BUFF_WEIGHTS: [f64; 13] = [
        388.0,  // Crit. Rate
        878.0,  // Crit. DMG
        219.0,  // ATK%
        643.0,  // DEF%
        0.0,    // HP%
        181.0,  // ATK
        226.0,  // DEF
        0.0,    // HP
        878.0,  // Energy Regen
        80.0,   // Basic Attack DMG Bonus
        78.0,   // Heavy Attack DMG Bonus
        48.0,   // Resonance Skill DMG Bonus
        1000.0, // Resonance Liberation DMG Bonus
    ];
    let scorer = LinearScorer::new(BUFF_WEIGHTS).unwrap();
    let score_pmfs = scorer.build_score_pmfs(false);
    for score_pmf in score_pmfs.iter() {
        println!("{:?}\n", score_pmf);
    }
    let cost_model = CostModel::tuner_only();
    let target_score_raw = 50.89;
    let mut solver =
        UpgradePolicySolver::new(&scorer, false, target_score_raw, cost_model).unwrap();

    let start = Instant::now();
    let lambda = solver.lambda_search(1e-6, 100).unwrap();
    let elapsed = start.elapsed();

    println!("target_score={:.2}", target_score_raw);
    println!(
        "lambda search completed in {:.3} milliseconds",
        elapsed.as_secs_f64() * 1000.0
    );
    println!("lambda_star={lambda:.6}");
    println!(
        "weighted expected cost per success={:.2}\n",
        solver.weighted_expected_cost().unwrap()
    );

    let start = Instant::now();
    let expected_cost = solver.calculate_expected_resources().unwrap();
    let elapsed = start.elapsed();
    println!(
        "expected resource computation completed in {:.3} milliseconds",
        elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "success probability={:.4}%",
        expected_cost.success_probability() * 100.0
    );
    println!(
        "expected resource cost per success: echo={:.2} tuner={:.2} exp={:.2}\n",
        expected_cost.echo_per_success(),
        expected_cost.tuner_per_success(),
        expected_cost.exp_per_success()
    );

    let test_state_mask: u16 = bits_to_mask(&[0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    let test_score: u16 = 1391 + 1457;
    println!(
        "state={:?}, score={}",
        mask_to_bits(test_state_mask),
        test_score
    );
    println!(
        "success probability={:.4}%",
        solver
            .get_success_probability(test_state_mask, test_score)
            .unwrap()
            * 100.0
    )
}

fn reroll_exmaple() {
    use std::time::Instant;

    use ww_echo_policy::RerollPolicySolver;
    use ww_echo_policy::{bits_to_mask, mask_to_bits};

    const BUFF_WEIGHTS: [f64; 13] = [
        3.0, // Crit. Rate
        3.0, // Crit. DMG
        1.0, // ATK%
        0.0, // DEF%
        0.0, // HP%
        0.0, // ATK
        0.0, // DEF
        0.0, // HP
        1.0, // Energy Regen
        1.0, // Basic Attack DMG Bonus
        0.0, // Heavy Attack DMG Bonus
        0.0, // Resonance Skill DMG Bonus
        0.0, // Resonance Liberation DMG Bonus
    ];

    let mut solver = RerollPolicySolver::new(BUFF_WEIGHTS).unwrap();
    let target_score = 7.0;
    solver.set_target(target_score).unwrap();
    let start = Instant::now();
    solver.derive_policy(1e-5, 100).unwrap();
    let elapsed = start.elapsed();

    let starting_state_mask = bits_to_mask(&[1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0]);
    let best_lock_mask = solver.best_lock_choices(starting_state_mask).unwrap();

    println!("target_score={:.1}", target_score);
    println!(
        "reroll policy computed in {:.3} milliseconds",
        elapsed.as_secs_f64() * 1000.0
    );
    println!("starting_state");
    println!("{:?}", mask_to_bits(starting_state_mask));
    println!(
        "expected lock cost={:.2?}",
        solver.expected_lock_cost(starting_state_mask).unwrap()
    );
    println!("best lock choice");
    println!("{:?}", mask_to_bits(best_lock_mask.unwrap()));

    let new_state_mask = bits_to_mask(&[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1]);
    println!("should accept?\n{:?}", mask_to_bits(new_state_mask));
    println!(
        "{}",
        solver
            .should_accept(starting_state_mask, new_state_mask)
            .unwrap()
    );
}

fn main() {
    // policy_exmaple();

    reroll_exmaple();
}
