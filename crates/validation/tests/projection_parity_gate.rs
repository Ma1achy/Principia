//! The `projection-parity` gate (TASK-M3-05; REQ-INT-030, REQ-INT-110): the runner on `fixtures/gates/projection-parity/`,
//! where the default passes and both controls fail; the written order against hand-worked values; the shared states'
//! classes; the verdict at and past the threshold; and the inputs refused.

use std::path::{Path, PathBuf};

use kernel::occupant::State;
use validation::decode_f32_eps::SplitMix;
use validation::gate::{self, Gate, GateConfig, Verdict};
use validation::negative_control;
use validation::projection_parity::{
    jacobi_round_trip, measure, shared_state, written_order, Control, Fuzz, Input,
    ProjectionParityGate, CLASSES, NAME,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn config() -> GateConfig {
    gate::config(&root(), NAME).expect("the gate's gate.json")
}

fn input(control: Control, states: u32) -> Input {
    Input {
        description: "a test input".to_owned(),
        fuzz: Fuzz { seed: 3, states },
        dt: 1e-3,
        control,
    }
}

#[test]
fn projection_parity_runs_its_fixtures() {
    let out = std::env::temp_dir().join(format!("projection-parity-{}", std::process::id()));
    let result = gate::run(&root(), NAME, &out);
    let _ = std::fs::remove_dir_all(&out);
    result.expect("each input gives its expected outcome");
}

/// Checks a measurement of the unperturbed gate: nothing differs, every class judged, five occupants on each state.
fn check_clean(states: u32, m: &validation::projection_parity::Measured) {
    assert_eq!(m.failures(), 0, "{:?}", m.first);
    assert_eq!(m.steps, 5 * states);
    assert_eq!(m.regions(), CLASSES.len() as u32);
    assert!(m.first.is_none());
}

#[test]
fn projection_parity_clean_on_every_class() {
    check_clean(64, &measure(&input(Control::None, 64)));
}

negative_control!(
    projection_parity_clean_on_every_class,
    "the Jacobi round trip inside the step",
    expected = "assertion",
    check_clean(64, &measure(&input(Control::Jacobi, 64)))
);

#[test]
fn projection_parity_counts_regions_and_differences() {
    let m = measure(&input(Control::Reordered, 3));
    assert_eq!(m.regions(), 3, "three states reach three classes");
    assert_eq!(m.per_class[3], (0, 0));
    assert_eq!(m.failures(), m.repeat_differences + m.order_differences);
    let judged: u32 = m.per_class.iter().map(|c| c.0).sum();
    assert_eq!(judged, m.steps);
    let bad: u32 = m.per_class.iter().map(|c| c.1).sum();
    assert!(bad > 0 && bad <= m.steps);
    assert!(m.order_differences > 0);
    assert_eq!(m.repeat_differences, 0);
    assert!(m.first.is_some_and(|f| f.contains(" state ")));
}

negative_control!(
    projection_parity_counts_regions_and_differences,
    "no state judged reaches no region",
    expected = "three states reach three classes",
    assert_eq!(
        measure(&input(Control::None, 1)).regions(),
        3,
        "three states reach three classes"
    )
);

/// A state whose projection is worked by hand: `M = 1`, `R_com = (1, 2)`, `P_com = (4, 8)`.
fn worked() -> State<f32> {
    State {
        m: [0.5, 0.25, 0.25],
        r: [[1.0, 2.0], [2.0, 3.0], [0.0, 1.0]],
        p: [[2.0, 4.0], [1.0, 2.0], [1.0, 2.0]],
    }
}

fn check_written(order: fn(&State<f32>) -> State<f32>) {
    let s = order(&worked());
    assert_eq!(s.r, [[0.0, 0.0], [1.0, 1.0], [-1.0, -1.0]]);
    assert_eq!(s.p, [[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]]);
    assert_eq!(s.m, worked().m);
}

#[test]
fn projection_parity_written_order_worked() {
    check_written(written_order);
    // Unequal masses tell the mass weights apart.
    let s = written_order(&State {
        m: [0.5, 0.25, 0.25],
        r: [[0.0, 0.0], [4.0, 0.0], [0.0, 8.0]],
        p: [[1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
    });
    assert_eq!(s.r, [[-1.0, -2.0], [3.0, -2.0], [-1.0, 6.0]]);
    assert_eq!(s.p, [[0.5, -0.5], [-0.25, -0.25], [-0.25, 0.75]]);
}

negative_control!(
    projection_parity_written_order_worked,
    "the identity is no projection",
    expected = "assertion",
    check_written(|s| *s)
);

#[test]
fn projection_parity_jacobi_round_trip_is_near_the_identity() {
    let s = jacobi_round_trip(&worked());
    for i in 0..3 {
        for k in 0..2 {
            assert!((s.r[i][k] - worked().r[i][k]).abs() < 1e-5, "r[{i}][{k}]");
        }
    }
    assert_eq!(s.p, worked().p);
    assert_eq!(s.m, worked().m);
}

negative_control!(
    projection_parity_jacobi_round_trip_is_near_the_identity,
    "a state far from the worked one",
    expected = "far",
    assert!((worked().r[0][0] - 9.0).abs() < 1e-5, "far")
);

#[test]
fn projection_parity_classes_are_what_they_name() {
    let mut rng = SplitMix::new(11);
    for _ in 0..200 {
        for class in 0..CLASSES.len() {
            let s = shared_state(&mut rng, class);
            let total = s.m[0] + s.m[1] + s.m[2];
            assert!((total - 1.0).abs() < 1e-5, "the masses sum to 1");
            // A raw mass on [0.05, 1], over a total at most 2.05 and at least 0.15.
            assert!(s.m.iter().all(|&m| m > 0.02 && m < 0.91), "{:?}", s.m);
            let sep = (s.r[1][0] - s.r[0][0]).hypot(s.r[1][1] - s.r[0][1]);
            let big_p = s.p.iter().flatten().fold(0f32, |a, x| a.max(x.abs()));
            let far = s.r.iter().flatten().fold(0f32, |a, x| a.max(x.abs()));
            match class {
                1 => assert!((0.9e-3..1.1e-2).contains(&sep), "the close pair at {sep}"),
                2 => assert!(big_p <= 20.0, "high momentum {big_p}"),
                3 => assert!(far <= 101.0, "far off at {far}"),
                _ => assert!(far <= 1.0 && big_p <= 1.0, "generic"),
            }
        }
    }
    // The classes reach past the generic box, over the draws.
    let top = |class, read: fn(&State<f32>) -> f32| {
        let mut rng = SplitMix::new(5);
        (0..64).fold(0f32, |a, _| a.max(read(&shared_state(&mut rng, class))))
    };
    assert!(
        top(2, |s| s
            .p
            .iter()
            .flatten()
            .fold(0f32, |a, x| a.max(x.abs())))
            > 5.0
    );
    assert!(
        top(3, |s| s
            .r
            .iter()
            .flatten()
            .fold(0f32, |a, x| a.max(x.abs())))
            > 20.0
    );
}

negative_control!(
    projection_parity_classes_are_what_they_name,
    "the generic class read as a close pair",
    expected = "the close pair",
    {
        let s = shared_state(&mut SplitMix::new(1), 0);
        let sep = (s.r[1][0] - s.r[0][0]).hypot(s.r[1][1] - s.r[0][1]);
        assert!(sep < 1.1e-2, "the close pair at {sep}");
    }
);

#[test]
fn projection_parity_verdict_at_the_threshold() {
    let i = input(Control::Reordered, 8);
    let failures = f64::from(measure(&i).failures());
    let mut c = config();
    assert_eq!(c.threshold.value, 0.0, "bit-identical");
    assert_eq!(
        ProjectionParityGate::judge(&c, "t", &i).verdict,
        Verdict::Fail
    );
    c.threshold.value = failures;
    assert_eq!(
        ProjectionParityGate::judge(&c, "t", &i).verdict,
        Verdict::Pass
    );
    c.threshold.value = failures - 1.0;
    assert_eq!(
        ProjectionParityGate::judge(&c, "t", &i).verdict,
        Verdict::Fail
    );
}

negative_control!(
    projection_parity_verdict_at_the_threshold,
    "a clean input is no failure",
    expected = "assertion",
    assert_eq!(
        ProjectionParityGate::judge(&config(), "t", &input(Control::None, 4)).verdict,
        Verdict::Fail
    )
);

#[test]
fn projection_parity_refuses_bad_inputs() {
    let run = |dt: serde_json::Value, states: u32| {
        let v = serde_json::json!({
            "description": "t", "fuzz": { "seed": 1, "states": states }, "dt": dt
        });
        ProjectionParityGate.run(&config(), "t", &v)
    };
    assert!(run(0.0.into(), 4).is_err(), "dt 0");
    assert!(run((-1.0).into(), 4).is_err(), "dt < 0");
    assert!(run(1e-3.into(), 0).is_err(), "no states");
    assert!(run(1e-3.into(), 1).is_ok(), "one state");
    let unknown = serde_json::json!({
        "description": "t", "fuzz": { "seed": 1, "states": 1 }, "dt": 1e-3, "extra": 1
    });
    let err = ProjectionParityGate.run(&config(), "t", &unknown).err();
    assert!(err.is_some_and(|e| e.contains("not a projection-parity input")));
    assert_eq!(ProjectionParityGate.name(), NAME);
}

negative_control!(
    projection_parity_refuses_bad_inputs,
    "a good input is not refused",
    expected = "refused",
    {
        let v = serde_json::json!({
            "description": "t", "fuzz": { "seed": 1, "states": 1 }, "dt": 1e-3
        });
        assert!(
            ProjectionParityGate.run(&config(), "t", &v).is_err(),
            "refused"
        );
    }
);
