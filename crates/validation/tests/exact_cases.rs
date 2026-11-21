//! The exact-cases gate (TASK-M3-28; REQ-VAL-047, REQ-VAL-130): Lagrange's triangle rigid at every sampled `t`.
//!
//! - `exact_cases_gate`: the gate on its fixtures, every input its expected outcome (the occupants pass, the displaced
//!   start and the Euler stepper fail). Its control: the displaced start relabelled `pass`.
//! - `exact_cases_lagrange_default_occupant`: Lagrange at masses (0.2, 0.3, 0.5), one turn by Heggie (Eq. 22) × RK4,
//!   rigid at every one of its 121 boundaries within the gate's threshold; its control, body 0 displaced by `10⁻⁶`.
//! - `exact_cases_rigidity_sees_a_pulse`: a homographic pulse, the triangle equilateral but larger than the closed form,
//!   fails; its control judges by the equilateral residual alone, which every homographic motion keeps at 0.
//! - `exact_cases_every_sample`: an excursion at one boundary in mid-run fails though the end is clean; its control
//!   reads the last sample alone.
//!
//! **Pass criteria**, written in the gate before its first run (`validation::gate::lagrange`'s note): the march lands on
//! every boundary of one turn, still marching, and at every one the rigidity residual (each side against the closed
//! form's) and the shape residual (equilateral for Lagrange, collinear for Euler) are within the threshold.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use kernel::regularisation::az::End;
use validation::gate::lagrange::{
    self, equilateral, judge, rigidity, run, sides, Case, Input, Run, Sample, NAME,
};
use validation::gate::{self, Verdict};
use validation::ground_truth::tiers;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh scratch directory: one per call, so a test and its control, which run in one process, never share one.
fn out(name: &str) -> PathBuf {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("exact-cases-{name}-{}-{call}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn threshold() -> f64 {
    gate::config(&root(), NAME).unwrap().threshold.value
}

// ── The runner on the fixtures ───────────────────────────────────────────────────────────────────────────────────

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
fn exact_cases_gate() {
    check_gate(|_, text| text);
}

negative_control!(
    exact_cases_gate,
    "the displaced start relabelled `pass`",
    expected = "each input its expected outcome",
    check_gate(|name, text| if name == "control_displaced.json" {
        text.replace("\"expected\": \"fail\"", "\"expected\": \"pass\"")
    } else {
        text
    })
);

// ── Lagrange under the shipping occupant ─────────────────────────────────────────────────────────────────────────

fn lagrange_input(displace: Option<lagrange::Displace>) -> Input {
    Input {
        description: "Lagrange at (0.2, 0.3, 0.5), one turn".to_owned(),
        case: Case::Lagrange,
        masses: Some([0.2, 0.3, 0.5]),
        stepper: "rk4".to_owned(),
        regularisation: "heggie-eq22".to_owned(),
        eta: 0.01,
        displace,
        control: false,
    }
}

fn check_lagrange(displace: Option<lagrange::Displace>) {
    let r = run(&lagrange_input(displace)).unwrap();
    for line in r.table() {
        println!("{line}");
    }
    let (verdict, lines) = judge(&r, threshold());
    assert_eq!(
        verdict,
        Verdict::Pass,
        "Lagrange's triangle is not rigid at every sample: {lines:?}"
    );
}

#[test]
fn exact_cases_lagrange_default_occupant() {
    check_lagrange(None);
}

negative_control!(
    exact_cases_lagrange_default_occupant,
    "body 0 displaced by 1e-6 from the closed form's start",
    expected = "Lagrange's triangle is not rigid at every sample",
    check_lagrange(Some(lagrange::Displace {
        body: 0,
        by: [1e-6, 0.0]
    }))
);

// ── Synthetic runs: the residuals and the sampling ───────────────────────────────────────────────────────────────

/// A run of `n` samples of Lagrange's closed form at masses (0.2, 0.3, 0.5), each sample's positions those of the closed
/// form at its time, sample `k` mapped by `f(k, r)`, judged as `residual` reads each sample.
fn synthetic(
    n: u32,
    f: impl Fn(u32, [[f64; 2]; 3]) -> [[f64; 2]; 3],
    residual: impl Fn(&[[f64; 2]; 3], &[f64; 3]) -> (f64, f64),
) -> Run {
    let rig = tiers::lagrange([0.2, 0.3, 0.5]);
    let closed = sides(&rig.start.r);
    let grid = validation::gate::figure_eight::grid(rig.period(), n).unwrap();
    let samples = (1..=n)
        .map(|k| {
            let t = grid.time(k);
            let r = f(k, rig.at(t).r);
            let (rigidity, shape) = residual(&r, &closed);
            Sample {
                index: k,
                t,
                rigidity,
                shape,
                closed_form: 0.0,
                reference_rigidity: 0.0,
                reference_shape: 0.0,
            }
        })
        .collect();
    Run {
        case: Case::Lagrange,
        named: lagrange_input(None).named().unwrap(),
        grid,
        horizon: rig.period(),
        samples,
        end: End::Marching,
        boundary: n,
        steps: n,
    }
}

/// The gate's residuals.
fn gate_residuals(r: &[[f64; 2]; 3], closed: &[f64; 3]) -> (f64, f64) {
    (rigidity(r, closed), equilateral(r))
}

/// A sample's residuals, `(rigidity, shape)`, from its positions and the closed form's sides.
type Residuals = fn(&[[f64; 2]; 3], &[f64; 3]) -> (f64, f64);

fn check_pulse(residual: Residuals) {
    // Every side 10⁻⁶ longer than the closed form's: still equilateral, so homographic, but not the circular solution.
    let pulse = synthetic(
        16,
        |_, r| r.map(|v| [v[0] * (1.0 + 1e-6), v[1] * (1.0 + 1e-6)]),
        residual,
    );
    assert!(
        pulse.samples.iter().all(|s| equilateral_of(s) < 1e-12),
        "the pulse is not equilateral"
    );
    assert_eq!(
        judge(&pulse, threshold()).0,
        Verdict::Fail,
        "a homographic pulse 1e-6 off the closed form's size passes"
    );
    let clean = synthetic(16, |_, r| r, residual);
    assert_eq!(
        judge(&clean, threshold()).0,
        Verdict::Pass,
        "the closed form itself fails"
    );
}

/// A sample's shape residual is the equilateral one here.
fn equilateral_of(s: &Sample) -> f64 {
    s.shape
}

#[test]
fn exact_cases_rigidity_sees_a_pulse() {
    check_pulse(gate_residuals);
}

negative_control!(
    exact_cases_rigidity_sees_a_pulse,
    "the equilateral residual alone, which every homographic motion keeps at 0",
    expected = "a homographic pulse 1e-6 off the closed form's size passes",
    check_pulse(|r, _| (0.0, equilateral(r)))
);

fn check_every(read: fn(&Run) -> Run) {
    // Boundary 8 of 16 alone has body 0 displaced by 10⁻⁷; the end is clean.
    let excursion = synthetic(
        16,
        |k, mut r| {
            if k == 8 {
                r[0][0] += 1e-7;
            }
            r
        },
        gate_residuals,
    );
    assert!(excursion.samples.last().unwrap().rigidity < 1e-13);
    assert_eq!(
        judge(&read(&excursion), threshold()).0,
        Verdict::Fail,
        "a mid-run excursion at boundary 8 is not seen"
    );
}

#[test]
fn exact_cases_every_sample() {
    check_every(Run::clone);
}

negative_control!(
    exact_cases_every_sample,
    "the last sample alone read, as a check at T alone would",
    expected = "a mid-run excursion at boundary 8 is not seen",
    check_every(|r| {
        let mut last = r.clone();
        last.samples = last
            .samples
            .iter()
            .map(|s| Sample {
                rigidity: r.samples.last().unwrap().rigidity,
                shape: r.samples.last().unwrap().shape,
                ..*s
            })
            .collect();
        last
    })
);
