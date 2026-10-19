use echo_app::*;

fn fixed(mode: Mode) -> SolveRequest {
    SolveRequest {
        config: Config {
            mode,
            scoring: Scoring::Fixed,
            weights: [1., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.],
            target: 2.,
            ..Config::default()
        },
        echo: vec![],
        candidate: None,
    }
}
const FULL: [(usize, u16); 5] = [(0, 63), (2, 64), (3, 81), (4, 64), (5, 30)];
#[test]
fn all_modes_and_unit_conversion() {
    for mode in [Mode::Upgrade, Mode::Joint] {
        let r = Service::default().solve(fixed(mode)).unwrap();
        assert_eq!(r.preview.target_internal, 2);
        assert_eq!(r.decision, "upgrade");
        let resources = r.resources.unwrap();
        assert!(resources.tuners >= 50.);
        assert!(resources.echoes >= 1.);
    }
    let mut request = fixed(Mode::Reroll);
    request.echo = FULL.to_vec();
    let r = Service::default().solve(request).unwrap();
    assert_eq!(r.decision, "reroll");
    assert!(r.frequencies.unwrap() > 0.);
    assert!(r.locks[0].types.contains(&0));
    assert!(r.resources.is_none());
}
#[test]
fn actual_values_change_reroll_cost_and_candidate_decision() {
    let mut req = fixed(Mode::Reroll);
    req.config.scoring = Scoring::Linear;
    req.config.target = 80.;
    req.echo = vec![(0, 63), (1, 126), (2, 64), (3, 81), (4, 64)];
    req.candidate = Some(vec![(0, 105), (1, 210), (2, 64), (3, 81), (4, 64)]);
    let r = Service::default().solve(req).unwrap();
    assert_eq!(r.preview.target_internal, 8000);
    assert_eq!(r.preview.score, 60.);
    assert!(r.frequencies.unwrap() > 0.);
    let c = r.comparison.unwrap();
    assert!(c.accept);
    assert_eq!(c.candidate_cost, 0.);
}
#[test]
fn main_score_and_fixed_targets_use_correct_units() {
    let mut req = fixed(Mode::Upgrade);
    assert_eq!(preview(&req).unwrap().target_internal, 2);
    req.config.scoring = Scoring::McBoostAssistant;
    req.config.target = 95.;
    let p = preview(&req).unwrap();
    assert_eq!(p.main_score, 50.);
    assert_eq!(p.target_internal, 4500);
    req.config.scoring = Scoring::Linear;
    req.config.main_score = 2.;
    req.config.target = 60.;
    let p = preview(&req).unwrap();
    assert_eq!(p.main_score, 50.);
    assert_eq!(p.target_internal, 1000);
}
#[test]
fn target_zero_still_counts_finished_product_resources() {
    for mode in [Mode::Upgrade, Mode::Joint] {
        let mut req = fixed(mode);
        req.config.target = 0.;
        let r = Service::default().solve(req).unwrap().resources.unwrap();
        assert!((r.echoes - 1.).abs() < 1e-8);
        assert!((r.tuners - 50.).abs() < 1e-8);
        assert!((r.tubes - 28.6).abs() < 1e-8);
        assert_eq!(r.frequencies, 0.);
    }
}
#[test]
fn cache_is_keyed_by_full_configuration_and_never_an_implicit_session() {
    let mut service = Service::default();
    let a = fixed(Mode::Upgrade);
    assert!(!service.solve(a.clone()).unwrap().cached);
    let mut b = a.clone();
    b.echo = vec![(0, 63)];
    assert!(service.solve(b.clone()).unwrap().cached);
    b.config.target = 1.;
    assert!(!service.solve(b.clone()).unwrap().cached);
    assert!(!service.solve(a).unwrap().cached);
    b.config.target = 3.;
    assert!(service.solve(b).is_err());
    // Validation failures do not discard an already solved policy.
    assert!(service.solve(fixed(Mode::Upgrade)).unwrap().cached);
}
#[test]
fn validates_echoes_and_rejects_impossible_targets() {
    let mut service = Service::default();
    let mut req = fixed(Mode::Reroll);
    assert_eq!(
        service.solve(req.clone()).err().unwrap().code,
        "invalid_input"
    );
    req.echo = FULL.to_vec();
    req.echo[0].1 = 64;
    assert!(service.solve(req.clone()).is_err());
    req.echo = FULL.to_vec();
    req.echo[1] = req.echo[0];
    assert!(service.solve(req.clone()).is_err());
    req.echo = FULL.to_vec();
    req.echo[0].0 = 13;
    assert!(service.solve(req.clone()).is_err());
    req.echo = FULL.to_vec();
    req.config.target = 3.;
    assert_eq!(service.solve(req).err().unwrap().code, "impossible_target");
}
#[test]
fn refuses_fractional_fixed_weights_and_resource_errors() {
    let mut req = fixed(Mode::Joint);
    req.config.weights[0] = 0.5;
    assert!(Service::default().solve(req).is_err());
    let mut req = fixed(Mode::Joint);
    req.config.costs = [0.; 4];
    assert!(Service::default().solve(req).is_err());
    let mut req = fixed(Mode::Joint);
    req.config.target = f64::NAN;
    assert!(Service::default().solve(req).is_err());
}
#[test]
fn full_upgrade_keep_is_not_confused_with_abandon() {
    let mut req = fixed(Mode::Upgrade);
    req.echo = FULL.to_vec();
    assert_eq!(
        Service::default().solve(req.clone()).unwrap().decision,
        "abandon"
    );
    req.echo[1] = (1, 126);
    assert_eq!(Service::default().solve(req).unwrap().decision, "keep");
}
#[test]
fn state_limits_are_reported_without_approximation() {
    let mut req = fixed(Mode::Reroll);
    req.echo = FULL.to_vec();
    assert_eq!(
        service_with_limits(1, 1).solve(req).err().unwrap().code,
        "state_limit"
    );
}

#[test]
fn standalone_reroll_does_not_require_upgrade_resource_weights() {
    let mut req = fixed(Mode::Reroll);
    req.echo = FULL.to_vec();
    req.config.costs = [0.; 4];
    assert!(Service::default().solve(req).is_ok());
}

#[test]
fn joint_hands_off_to_reroll_without_returning_lock_details() {
    let mut req = fixed(Mode::Joint);
    req.echo = FULL.to_vec();
    req.config.costs[3] = 0.01;
    let r = Service::default().solve(req.clone()).unwrap();
    assert_eq!(r.decision, "reroll");
    assert!(r.locks.is_empty());
    assert!(r.lock_mask.is_none());
    assert!(r.frequencies.is_none());
    assert!(r.comparison.is_none());
    req.candidate = Some(FULL.to_vec());
    assert_eq!(
        Service::default().solve(req).err().unwrap().code,
        "invalid_input"
    );
}

#[test]
fn measured_exp_cost_matches_weighted_solver_objective() {
    for mode in [Mode::Upgrade, Mode::Joint] {
        let mut req = fixed(mode);
        req.config.costs = [0., 0., 1., 10.];
        let resources = Service::default().solve(req).unwrap().resources.unwrap();
        assert!(
            (resources.weighted - (resources.tubes + 10. * resources.frequencies)).abs() < 1e-6
        );
        assert!(resources.tubes >= 28.6);
    }
    assert!(
        serde_json::to_value(Config::default())
            .unwrap()
            .get("refund")
            .is_none()
    );
}
