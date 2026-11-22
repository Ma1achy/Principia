//! The reversibility diagnostic on the free-fall brake orbits (TASK-M3-29; integrator contract Part 6).
//!
//! - `free_fall_fixtures`: each committed orbit is Hristov et al.'s row in their convention: `T*` recomputed from
//!   `(x, y, T)` as `T|E|^{3/2}` agrees with the published `T*` to `1e-12` relative. Control: `x` and `y` swapped.
//! - `reversibility_refuses_step_irreversible`: the diagnostic refuses every logH pair whose profile has
//!   `step_reversible` false (RK4 and Euler, in both time modes), and so does the reversibility test, naming that flag.
//!   Control: the same assertion on logH-TTL × KDK.
//! - `reversibility_binds_run_reversible_only`: the reversibility test refuses every step-reversible pair but
//!   logH-TTL × KDK, the one `run_reversible` occupant, and binds that one. Control: the same refusal asserted of
//!   logH-TTL × KDK.
//! - `reversibility_xi_brake_orbits`: logH-TTL × KDK at the fixed step reads `ξ ≤ 1e-9` and `|t₂|/T ≤ 1e-9` on row 3
//!   (the criteria in `validation::reversibility`'s note, written before the first run), and every orbit's reading is
//!   printed. Rows 1 and 4 fail the criterion: the first run's finding, recorded as the gate's expected failures with
//!   their measurement, never judged pass here. Control: logH-TTL × RK4 through the unguarded measurement, at the same
//!   step, on row 3.
//! - `reversibility_xi_needs_fixed_step`: the same criteria on row 3. Control: the adaptive schedule, the same stepper
//!   with each step sized from the state at its start.
//! - `reversibility_xi_gate`: the gate on its fixtures, each input its expected outcome. Control: the adaptive control
//!   relabelled `pass`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::regularisation::logh::LhStepper;
use kernel::regularisation::pairs::LogHMode;
use validation::gate::{self, Verdict};
use validation::negative_control;
use validation::reversibility::{
    bind, diagnose, energy, judge, measure, orbits, table, BrakeOrbit, Reading, Refused, Schedule,
    NAME, STEPS_PER_PERIOD, XI_MAX,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn all() -> Vec<BrakeOrbit> {
    let o = orbits(&root()).expect("the brake orbits");
    assert_eq!(o.len(), 3, "three brake orbits are committed");
    o
}

/// Row 3, the orbit whose reading the criterion judges pass (the first run's finding: rows 1 and 4 fail it).
fn row3() -> BrakeOrbit {
    all()
        .into_iter()
        .find(|o| o.name == "hristov_sol80_row3")
        .expect("row 3")
}

// ── The fixtures ─────────────────────────────────────────────────────────────────────────────────────────────────

/// `T*` recomputed from `orbit`'s start (through `edit`) and `T`, against the published one.
fn check_fixture(orbit: &BrakeOrbit, edit: fn(&mut kernel::occupant::State<f64>)) {
    let mut s = orbit.start().unwrap();
    edit(&mut s);
    assert!(
        s.p.iter().flatten().all(|&x| x == 0.0),
        "a brake orbit starts at rest"
    );
    let t = orbit.period().unwrap();
    let recomputed = t * energy(&s).abs().powf(1.5);
    let published = orbit.scaled_period().unwrap();
    let rel = (recomputed - published).abs() / published;
    println!(
        "{}: T {t:.15}, T* recomputed {recomputed:.15}, published {published:.15}, relative {rel:.2e}",
        orbit.name
    );
    assert!(
        rel <= 1e-12,
        "{}'s T* does not match the published one: {rel:.3e}",
        orbit.name
    );
}

#[test]
fn free_fall_fixtures() {
    for o in all() {
        check_fixture(&o, |_| {});
    }
}

negative_control!(
    free_fall_fixtures,
    "the third body's x and y swapped",
    expected = "T* does not match the published one",
    check_fixture(&all()[0], |s| s.r[2] = [s.r[2][1], s.r[2][0]])
);

// ── The guard ────────────────────────────────────────────────────────────────────────────────────────────────────

/// `diagnose` on logH × `S` in `mode` is refused as not step-reversible.
fn check_refused_step<S: LhStepper>(name: &str, mode: LogHMode) {
    let o = &all()[0];
    let (start, period) = (o.start().unwrap(), o.period().unwrap());
    let got = diagnose::<S>(mode, &start, period, 64);
    assert!(
        matches!(got, Err(Refused::NotStepReversible)),
        "{name} was not refused by the diagnostic"
    );
    let got = bind::<S>(mode, &start, period, 64);
    assert!(
        matches!(got, Err(Refused::NotStepReversible)),
        "{name} was not refused by the reversibility test for step_reversible"
    );
}

#[test]
fn reversibility_refuses_step_irreversible() {
    check_refused_step::<Rk4>("logH-TTL × RK4", LogHMode::Ttl);
    check_refused_step::<Euler>("logH-TTL × Euler", LogHMode::Ttl);
    check_refused_step::<Rk4>("logH × RK4", LogHMode::LogH);
    check_refused_step::<Euler>("logH × Euler", LogHMode::LogH);
}

negative_control!(
    reversibility_refuses_step_irreversible,
    "the refusal asserted of the step-reversible logH-TTL × KDK",
    expected = "was not refused by the diagnostic",
    check_refused_step::<Kdk>("logH-TTL × KDK", LogHMode::Ttl)
);

/// `bind` on logH × `S` in `mode` is refused as not run-reversible.
fn check_refused_run<S: LhStepper>(name: &str, mode: LogHMode) {
    let o = &all()[0];
    let got = bind::<S>(mode, &o.start().unwrap(), o.period().unwrap(), 64);
    assert!(
        matches!(got, Err(Refused::NotRunReversible)),
        "{name} was not refused by the reversibility test"
    );
}

#[test]
fn reversibility_binds_run_reversible_only() {
    check_refused_run::<Kdk>("logH × KDK", LogHMode::LogH);
    check_refused_run::<Yoshida4>("logH × Yoshida-4", LogHMode::LogH);
    check_refused_run::<Yoshida6>("logH × Yoshida-6", LogHMode::LogH);
    check_refused_run::<Yoshida4>("logH-TTL × Yoshida-4", LogHMode::Ttl);
    check_refused_run::<Yoshida6>("logH-TTL × Yoshida-6", LogHMode::Ttl);
    let o = &all()[0];
    assert!(
        bind::<Kdk>(LogHMode::Ttl, &o.start().unwrap(), o.period().unwrap(), 64).is_ok(),
        "logH-TTL × KDK, the run-reversible occupant, was refused"
    );
}

negative_control!(
    reversibility_binds_run_reversible_only,
    "the refusal asserted of logH-TTL × KDK, the run-reversible occupant",
    expected = "was not refused by the reversibility test",
    check_refused_run::<Kdk>("logH-TTL × KDK", LogHMode::Ttl)
);

// ── ξ on the brake orbits ────────────────────────────────────────────────────────────────────────────────────────

/// `reading` on `name` judged against [`XI_MAX`]: panics on a fail.
fn check_reading(name: &str, reading: &Reading) {
    for line in table(reading) {
        println!("{name}: {line}");
    }
    let (verdict, lines) = judge(reading, XI_MAX);
    for line in &lines {
        println!("{name}: - {line}");
    }
    assert_eq!(
        verdict,
        Verdict::Pass,
        "{name} reads not time-symmetric: {lines:?}"
    );
}

/// The reading of logH-TTL × `S` on `orbit` under `schedule`, through the unguarded measurement.
fn reading<S: LhStepper>(orbit: &BrakeOrbit, schedule: Schedule) -> Reading {
    measure::<S>(
        LogHMode::Ttl,
        &orbit.start().unwrap(),
        orbit.period().unwrap(),
        STEPS_PER_PERIOD,
        schedule,
    )
}

#[test]
fn reversibility_xi_brake_orbits() {
    for o in all() {
        let r = bind::<Kdk>(
            LogHMode::Ttl,
            &o.start().unwrap(),
            o.period().unwrap(),
            STEPS_PER_PERIOD,
        )
        .expect("logH-TTL × KDK binds");
        for line in table(&r) {
            println!("{} logH-TTL × KDK: {line}", o.name);
        }
        println!(
            "{} logH-TTL × KDK: verdict {:?}",
            o.name,
            judge(&r, XI_MAX).0
        );
        // Reported beside, never judged: the non-reversible pairs and the step-reversible, not run-reversible one,
        // through the unguarded measurement at the same step, and the adaptive schedule.
        for (what, r) in [
            ("logH-TTL × RK4", reading::<Rk4>(&o, Schedule::Fixed)),
            ("logH-TTL × Euler", reading::<Euler>(&o, Schedule::Fixed)),
            (
                "logH-TTL × Yoshida-4",
                reading::<Yoshida4>(&o, Schedule::Fixed),
            ),
            (
                "logH-TTL × KDK adaptive",
                reading::<Kdk>(&o, Schedule::Adaptive),
            ),
        ] {
            for line in table(&r) {
                println!("{} {what} (reported): {line}", o.name);
            }
        }
    }
    let o = row3();
    let r = bind::<Kdk>(
        LogHMode::Ttl,
        &o.start().unwrap(),
        o.period().unwrap(),
        STEPS_PER_PERIOD,
    )
    .expect("logH-TTL × KDK binds");
    check_reading(&format!("{} logH-TTL × KDK", o.name), &r);
}

negative_control!(
    reversibility_xi_brake_orbits,
    "logH-TTL × RK4, not step-reversible, through the unguarded measurement at the same step",
    expected = "reads not time-symmetric",
    check_reading("logH-TTL × RK4", &reading::<Rk4>(&row3(), Schedule::Fixed))
);

#[test]
fn reversibility_xi_needs_fixed_step() {
    check_reading(
        "logH-TTL × KDK fixed",
        &reading::<Kdk>(&row3(), Schedule::Fixed),
    );
}

negative_control!(
    reversibility_xi_needs_fixed_step,
    "the adaptive schedule: each step sized from the state at its start",
    expected = "reads not time-symmetric",
    check_reading(
        "logH-TTL × KDK adaptive",
        &reading::<Kdk>(&row3(), Schedule::Adaptive)
    )
);

// ── The gate ─────────────────────────────────────────────────────────────────────────────────────────────────────

fn out(name: &str) -> PathBuf {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "reversibility-{name}-{}-{call}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The gate run on a copy of its fixtures, with `relabel` applied to each input file's text; panics unless every input
/// gives its expected outcome. The threshold is the module's.
fn check_gate(relabel: fn(&str, String) -> String) {
    assert_eq!(
        gate::config(&root(), NAME).unwrap().threshold.value,
        XI_MAX,
        "gate.json's threshold is the module's XI_MAX"
    );
    let copy = out("root");
    let dir = copy.join("fixtures/gates").join(NAME);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(copy.join("docs/history")).unwrap();
    std::fs::copy(
        root().join("docs/history/requirements.yaml"),
        copy.join("docs/history/requirements.yaml"),
    )
    .unwrap();
    for entry in std::fs::read_dir(root().join("fixtures/gates").join(NAME)).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(dir.join(&name), relabel(&name, text)).unwrap();
    }
    gate::run(&copy, NAME, &out("reports")).expect("each input its expected outcome");
}

#[test]
fn reversibility_xi_gate() {
    check_gate(|_, text| text);
}

negative_control!(
    reversibility_xi_gate,
    "the adaptive control relabelled `pass`",
    expected = "each input its expected outcome",
    check_gate(|name, text| if name == "control_adaptive.json" {
        text.replace("\"expected\": \"fail\"", "\"expected\": \"pass\"")
    } else {
        text
    })
);
