//! Run each case in a fresh process, e.g. `profile joint 5`.
use echo_app::{Config, Mode, Service, SolveRequest};
use std::time::Instant;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mode = match args.get(1).map(String::as_str) {
        Some("upgrade") => Mode::Upgrade,
        Some("reroll") => Mode::Reroll,
        Some("joint") => Mode::Joint,
        _ => panic!("usage: profile upgrade|reroll|joint [nonzero weights: 5..13]"),
    };
    let mut config = Config {
        mode,
        ..Config::default()
    };
    let count: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
    for i in [3, 6, 7, 9, 10, 11, 12, 4]
        .into_iter()
        .take(count.saturating_sub(5))
    {
        config.weights[i] = 30.;
    }
    let request = SolveRequest {
        config,
        echo: if mode == Mode::Reroll {
            [0, 1, 2, 5, 8]
                .map(|i| (i, echo_app::metadata::BUFF_VALUE_OPTIONS[i][0]))
                .to_vec()
        } else {
            vec![]
        },
        candidate: None,
    };
    let mut service = Service::default();
    let start = Instant::now();
    let result = service.solve(request.clone());
    let cold_ms = start.elapsed().as_secs_f64() * 1000.;
    let warm = Instant::now();
    if result.is_ok() {
        for _ in 0..100 {
            assert!(service.solve(request.clone()).unwrap().cached);
        }
    }
    let warm_mean_ms = result.is_ok().then(|| warm.elapsed().as_secs_f64() * 10.);
    let memory = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    println!(
        "{}",
        serde_json::json!({
            "mode":mode, "weights":args.get(2), "cold_ms":cold_ms,
            "warm_mean_ms":warm_mean_ms,
            "error":result.err(), "retained_bytes":service.allocated_bytes(), "memory":memory.lines().filter(|s| s.starts_with("VmRSS:") || s.starts_with("VmHWM:")).collect::<Vec<_>>()
        })
    );
}
