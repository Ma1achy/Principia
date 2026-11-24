//! The Burrau anchor smoke tests (TASK-M3-27), blocking: SP-1 run to its own horizon, SP-2 against Portegies Zwart &
//! Boekholt 2018's converged reference, and PL-8a, the f64 run against the convergence reference to `t ≈ 60`.
//!
//! - `burrau_smoke_gate`: `cargo xtask gate burrau-smoke`'s runner on its fixtures, every input its expected outcome
//!   (SP-1 under the default occupant passes, Yoshida-4 unregularised fails). Control: the Yoshida-4 input relabelled
//!   `pass`.
//! - `burrau_smoke_sp1`: SP-1 at `t_max = 65` of S&P's units: binary plus the lightest body escaping, read from the
//!   march's own end state, and the escape label's criterion on the march to 100; `t_max` outside 60–65 refused.
//!   Control: Yoshida-4 unregularised in the default occupant's place (it fails on the label arm: escape at 28.2).
//! - `burrau_smoke_sp1_structural`: SP-1's structural arm alone, the label set aside. Control: the heaviest body read
//!   as the escaper.
//! - `burrau_smoke_sp1_label_body`: the escape label is scored with its body counted. Control: the label naming body 1
//!   (the right class, the wrong body).
//! - `burrau_smoke_sp2_unperturbed`: SP-2's unperturbed run on the convergence reference, against Table 2 at 100, the
//!   start at 200 after the reversal (reversibility) and Table 2 at 200 (trajectory). Control: no velocity reversal,
//!   which must fail the reversibility check.
//! - `burrau_smoke_sp2_perturbed`: the same for the perturbed run (`x₃ + 10⁻¹⁰`, exact). Control: the perturbation
//!   dropped, which must fail at `t = 100`: the `10⁻¹⁰` is resolved.
//! - `burrau_smoke_table2`: Table 2's reader: 12 rows read, each run back at its start at 200 to `10⁻¹²`. Control: a
//!   missing row refused.
//! - `burrau_smoke_pl8a`: the f64 run does not part from the reference before f64's horizon (52), and the f32 run parts
//!   in the window before it. Control: the f32 run judged in the f64 run's place.
//!
//! **Pass criteria**, each written in `validation::gate::burrau` before the run it judges (its note says which number
//! came from where). SP-1's class and escaper, the label window `[50, 100]` (unchanged) and PL-8a's horizon of 52 are
//! the spec's; SP-2's three bounds are the ladder's own disagreement, with no tuned number; PL-8a's parting threshold,
//! `10⁻²` of the system's size, is a proposal (`--propose` prints each decade's parting time).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::gate::burrau::{
    self, pl8a, sp1, sp2, table2, Sp2Options, Sp2Run, Width, NAME, PL8A_LADDER, SP1_T_MAX,
    SP2_LADDER,
};
use validation::gate::{self};
use validation::ground_truth::DefaultOccupant;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh scratch directory: one per call.
fn out(name: &str) -> PathBuf {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("burrau-smoke-{name}-{}-{call}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

// ── The gate's runner ────────────────────────────────────────────────────────────────────────────────────────────

fn check_gate(relabel: fn(&str, String) -> String) {
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
fn burrau_smoke_gate() {
    check_gate(|_, text| text);
}

negative_control!(
    burrau_smoke_gate,
    "the Yoshida-4 control relabelled `pass`",
    expected = "each input its expected outcome",
    check_gate(|name, text| if name == "control_yoshida4.json" {
        text.replace("\"expected\": \"fail\"", "\"expected\": \"pass\"")
    } else {
        text
    })
);

// ── SP-1 ─────────────────────────────────────────────────────────────────────────────────────────────────────────

fn check_sp1(run: Result<burrau::Sp1, String>) {
    let r = run.expect("SP-1 runs");
    for line in r.lines() {
        println!("sp1: {line}");
    }
    if let Err(e) = r.verdict() {
        panic!("{e}");
    }
    for t in [59.9, 65.1] {
        let refused = sp1::<DefaultOccupant>("Heggie × RK4", t);
        assert!(
            refused.as_ref().is_err_and(|e| e.contains("refused")),
            "SP-1 at t_max = {t}, outside 60–65, is not refused"
        );
    }
}

#[test]
fn burrau_smoke_sp1() {
    check_sp1(sp1::<DefaultOccupant>("Heggie × RK4", SP1_T_MAX));
}

negative_control!(
    burrau_smoke_sp1,
    "Yoshida-4 unregularised does not resolve Burrau's encounters",
    expected = "SP-1 under Yoshida-4",
    check_sp1(sp1::<kernel::occupant::yoshida4::Yoshida4>(
        "Yoshida-4 unregularised",
        SP1_T_MAX
    ))
);

/// SP-1's structural arm alone, the label's verdict set aside: the class and escaper at `t_max`, with `escaper` in the
/// run's own read's place where given (the control's single change).
fn check_sp1_structural(escaper: Option<Option<usize>>) {
    let mut r = sp1::<DefaultOccupant>("Heggie × RK4", SP1_T_MAX).expect("SP-1 runs");
    r.label = Ok(());
    if let Some(e) = escaper {
        r.outcome.escaper = e;
    }
    if let Err(e) = r.verdict() {
        panic!("{e}");
    }
}

#[test]
fn burrau_smoke_sp1_structural() {
    check_sp1_structural(None);
}

negative_control!(
    burrau_smoke_sp1_structural,
    "the heaviest body read as the escaper",
    expected = "is not binary plus the lightest body escaping",
    check_sp1_structural(Some(Some(2)))
);

/// SP-1's label arm on the escaping body: the label is scored with its body counted, so an escape naming any body but
/// the lightest is wrong even though its class is right. `body`, where given, is written into both marches' labels in
/// place of the run's own (the control's single change).
fn check_sp1_label_body(body: Option<u32>) {
    let mut r = sp1::<DefaultOccupant>("Heggie × RK4", SP1_T_MAX).expect("SP-1 runs");
    if let Some(b) = body {
        r.stopped.packed_a = kernel::payload::set_detail(r.stopped.packed_a, b);
        r.live.packed_a = kernel::payload::set_detail(r.live.packed_a, b);
    }
    let start = validation::ground_truth::tiers::burrau();
    let verdict = validation::ground_truth::tiers::burrau_live(
        &r.stopped,
        &r.live,
        r.dt_macro,
        validation::ground_truth::burrau_time_scale(),
        start.m,
    );
    if let Err(e) = verdict {
        panic!("{e}");
    }
}

#[test]
fn burrau_smoke_sp1_label_body() {
    check_sp1_label_body(None);
}

negative_control!(
    burrau_smoke_sp1_label_body,
    "the escape label naming body 1, not the lightest: the right class with the wrong body",
    expected = "does not end in the lightest body's escape",
    check_sp1_label_body(Some(1))
);

// ── SP-2 ─────────────────────────────────────────────────────────────────────────────────────────────────────────

fn table() -> String {
    burrau::table2_text(&root()).expect("Table 2")
}

fn check_sp2(run: Sp2Run, options: Sp2Options) {
    let t0 = std::time::Instant::now();
    let r = sp2(run, &SP2_LADDER, options, &table()).expect("SP-2 runs and its ladder converges");
    for line in r.lines() {
        println!("sp2: {line}");
    }
    println!("sp2: {:?}", t0.elapsed());
    if let Err(e) = r.verdict() {
        panic!("{e}");
    }
}

#[test]
fn burrau_smoke_sp2_unperturbed() {
    check_sp2(Sp2Run::Unperturbed, Sp2Options::of(Sp2Run::Unperturbed));
}

negative_control!(
    burrau_smoke_sp2_unperturbed,
    "the t = 200 state without the velocity reversal at t = 100 is not back at the start",
    expected = "reversibility",
    check_sp2(
        Sp2Run::Unperturbed,
        Sp2Options {
            reverse: false,
            ..Sp2Options::of(Sp2Run::Unperturbed)
        }
    )
);

#[test]
fn burrau_smoke_sp2_perturbed() {
    check_sp2(Sp2Run::Perturbed, Sp2Options::of(Sp2Run::Perturbed));
}

negative_control!(
    burrau_smoke_sp2_perturbed,
    "the perturbed run without its 1e-10 perturbation does not reach the perturbed Table 2 at t = 100",
    expected = "t = 100 against Table 2",
    check_sp2(
        Sp2Run::Perturbed,
        Sp2Options {
            nudge: None,
            ..Sp2Options::of(Sp2Run::Perturbed)
        }
    )
);

fn check_table2(text: &str) {
    let rows = table2(text).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(rows.len(), 12, "Table 2 has 12 rows");
    let start = validation::ground_truth::tiers::burrau();
    for r in rows.iter().filter(|r| r.t == 200.0) {
        let mut x0 = start.r[r.body];
        if r.run == Sp2Run::Perturbed && r.body == 0 {
            x0[0] = 1.000_000_000_1;
        }
        let off = (r.r[0] - x0[0]).abs().max((r.r[1] - x0[1]).abs());
        assert!(
            off <= 1e-12,
            "Table 2's {:?} run at 200, body {}: {off:e} from the start",
            r.run,
            r.body
        );
    }
}

#[test]
fn burrau_smoke_table2() {
    check_table2(&table());
}

negative_control!(
    burrau_smoke_table2,
    "Table 2 with a row missing is refused",
    expected = "one expected",
    check_table2(
        &table()
            .lines()
            .filter(|l| !l.starts_with("perturbed,200,4,"))
            .collect::<Vec<_>>()
            .join("\n")
    )
);

// ── PL-8a ────────────────────────────────────────────────────────────────────────────────────────────────────────

fn check_pl8a(judged: Width) {
    let t0 = std::time::Instant::now();
    let r = pl8a(&PL8A_LADDER, judged).expect("PL-8a runs and its reference converges");
    for line in r.lines() {
        println!("pl8a: {line}");
    }
    println!("pl8a: {:?}", t0.elapsed());
    if let Err(e) = r.verdict() {
        panic!("{e}");
    }
}

#[test]
fn burrau_smoke_pl8a() {
    check_pl8a(Width::F64);
}

negative_control!(
    burrau_smoke_pl8a,
    "the f32 run judged in the f64 run's place parts before f64's horizon",
    expected = "before f64's horizon",
    check_pl8a(Width::F32)
);
