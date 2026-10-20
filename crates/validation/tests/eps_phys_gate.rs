//! The `encode-eps-phys` gate (TASK-M2-16; REQ-ENC-024, REQ-ENC-004, PIT-3): its draws, the clamp-flag agreement, the
//! verdict at and past the threshold, the rows, the refusals, and the runner on `fixtures/gates/encode-eps-phys/`, where
//! the mismatched encode fails.

use std::path::{Path, PathBuf};

use validation::eps_phys::{
    agrees, latents, points, rows, verdict, EncodeKind, EpsPhysGate, Input, Row, NAME,
};
use validation::gate::{self, GateConfig, Verdict};
use validation::negative_control;
use validation::roundtrip::{keys, Sides};

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn input(per_chart: u32, fuzz: u32, encode: EncodeKind) -> Input {
    Input {
        description: "a test input".to_owned(),
        seed: 5,
        per_chart,
        fuzz,
        span: 6.0,
        encode,
    }
}

/// The gate's configuration, as `fixtures/gates/encode-eps-phys/gate.json` gives it.
fn config() -> GateConfig {
    gate::config(&root(), NAME).expect("the gate's gate.json")
}

// ── The draws ────────────────────────────────────────────────────────────────────────────────────────────────────

/// `points` are `per_chart` points in `[0, 1]²` and `latents` `fuzz` vectors in `[−span, span]⁸`, both taking values
/// across their ranges, the same from one seed and another from another.
fn check_draws(i: &Input, p: &[(f64, f64)], z: &[[f64; 8]]) {
    assert_eq!(p.len(), i.per_chart as usize, "the point count");
    assert_eq!(z.len(), i.fuzz as usize, "the latent count");
    assert!(
        p.iter()
            .all(|&(u, v)| (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)),
        "a point outside [0, 1]²"
    );
    assert!(
        z.iter().flatten().all(|x| x.abs() <= i.span),
        "a latent outside the span"
    );
    let spread =
        |xs: Vec<f64>, lo: f64, hi: f64| xs.iter().any(|&x| x < lo) && xs.iter().any(|&x| x > hi);
    assert!(
        spread(p.iter().map(|q| q.0).collect(), 0.1, 0.9)
            && spread(p.iter().map(|q| q.1).collect(), 0.1, 0.9)
            && spread(
                z.iter().flatten().copied().collect(),
                -0.8 * i.span,
                0.8 * i.span
            ),
        "the draws do not cover their ranges"
    );
}

#[test]
fn eps_phys_gate_draws() {
    let i = input(512, 512, EncodeKind::Defaults);
    check_draws(&i, &points(&i), &latents(&i));
    assert_eq!(points(&i), points(&i), "one seed, one list");
    let other = Input {
        seed: 6,
        ..i.clone()
    };
    assert_ne!(
        points(&i),
        points(&other),
        "another seed draws another list"
    );
    assert_ne!(
        latents(&i),
        latents(&other),
        "another seed draws other latents"
    );
}

negative_control!(
    eps_phys_gate_draws,
    "a list missing its last point must fail",
    expected = "the point count",
    {
        let i = input(512, 512, EncodeKind::Defaults);
        let mut p = points(&i);
        p.pop();
        check_draws(&i, &p, &latents(&i))
    }
);

// ── The clamp-flag agreement ─────────────────────────────────────────────────────────────────────────────────────

/// `agrees` at each case: a past control needs its bit, an inside one must not have it, and a control in the band
/// may have it or not.
fn check_agrees(agree: fn(Sides, u32) -> bool) {
    let interior = Sides {
        inside: 0xff,
        past: 0,
    };
    assert!(agree(interior, 0), "an interior sample with no bits");
    assert!(!agree(interior, 1 << 3), "an interior sample with a bit");
    let past = Sides {
        inside: 0xff & !(1 << 2),
        past: 1 << 2,
    };
    assert!(agree(past, 1 << 2), "a past control with its bit");
    assert!(!agree(past, 0), "a past control without its bit");
    assert!(
        !agree(past, (1 << 2) | 1),
        "a past control's bit with an inside one's"
    );
    let band = Sides {
        inside: 0xff & !(1 << 6),
        past: 0,
    };
    assert!(
        agree(band, 0) && agree(band, 1 << 6),
        "a control in the band, with or without its bit"
    );
}

#[test]
fn eps_phys_gate_agrees() {
    check_agrees(agrees);
}

negative_control!(
    eps_phys_gate_agrees,
    "an agreement that ignores the past controls must fail",
    expected = "a past control without its bit",
    check_agrees(|s, b| (s.inside & b) == 0)
);

// ── The verdict ──────────────────────────────────────────────────────────────────────────────────────────────────

fn row(worst: f64, interior: usize, mismatches: usize) -> Row {
    Row {
        source: "test".to_owned(),
        bits: 64,
        interior,
        clamped: 0,
        band: 0,
        mismatches,
        worst: (worst, 0),
        median: 0.0,
        p99: 0.0,
    }
}

/// The verdict passes at the threshold and fails past it, on a row with no interior sample, on a mismatch, and on a
/// NaN.
fn check_verdict(judge: fn(&[Row], f64) -> Verdict) {
    let k = 64.0f64;
    assert_eq!(
        judge(&[row(k, 10, 0)], k),
        Verdict::Pass,
        "at the threshold"
    );
    assert_eq!(
        judge(&[row(k.next_up(), 10, 0)], k),
        Verdict::Fail,
        "one ulp past the threshold"
    );
    assert_eq!(
        judge(&[row(1.0, 0, 0)], k),
        Verdict::Fail,
        "no interior sample"
    );
    assert_eq!(
        judge(&[row(1.0, 1, 0)], k),
        Verdict::Pass,
        "one interior sample"
    );
    assert_eq!(judge(&[row(1.0, 10, 1)], k), Verdict::Fail, "one mismatch");
    assert_eq!(
        judge(&[row(f64::NAN, 10, 0)], k),
        Verdict::Fail,
        "a NaN worst"
    );
    assert_eq!(
        judge(&[row(1.0, 10, 0), row(k + 1.0, 10, 0)], k),
        Verdict::Fail,
        "one failing row among passing ones"
    );
}

#[test]
fn eps_phys_gate_verdict() {
    check_verdict(verdict);
}

negative_control!(
    eps_phys_gate_verdict,
    "a verdict that admits a residual past the threshold must fail",
    expected = "one ulp past the threshold",
    check_verdict(|rows, k| verdict(rows, 2.0 * k))
);

// ── The rows ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// `rows` on `i`: each registered chart at f64 then f32, in the registry's order, then the 8-D interior, each row's
/// samples all counted; the ternary mass plot's corners clamped; and, under the defaults, every row within 64 `ε` with
/// no mismatch, while the mismatched encode is far past it.
fn check_rows(i: &Input, got: &[Row]) {
    let mut want: Vec<(String, u32)> = keys()
        .into_iter()
        .flat_map(|k| [(k.to_owned(), 64), (k.to_owned(), 32)])
        .collect();
    want.extend([
        ("latent z (8-D)".to_owned(), 64),
        ("latent z (8-D)".to_owned(), 32),
    ]);
    let have: Vec<(String, u32)> = got.iter().map(|r| (r.source.clone(), r.bits)).collect();
    assert_eq!(have, want, "the rows and their order");
    for r in got {
        let n = if r.source.starts_with("latent z") {
            i.fuzz
        } else {
            i.per_chart
        } as usize;
        assert_eq!(
            r.interior + r.clamped + r.band,
            n,
            "{} f{}: the samples counted",
            r.source,
            r.bits
        );
        assert!(
            r.median <= r.p99 && r.p99 <= r.worst.0,
            "{} f{}: the median, p99 and worst out of order",
            r.source,
            r.bits
        );
    }
    let simplex = got
        .iter()
        .find(|r| r.source == "mass_simplex")
        .expect("listed");
    assert!(
        simplex.clamped > 0,
        "the ternary plot's corners are not clamped"
    );
    match i.encode {
        EncodeKind::Defaults => assert!(
            got.iter().all(|r| r.mismatches == 0 && r.worst.0 <= 64.0),
            "a row past 64 ε or with a mismatch under the defaults"
        ),
        EncodeKind::Mismatched => assert!(
            got.iter().all(|r| r.worst.0 > 1e3),
            "a row within 1000 ε under the mismatched encode"
        ),
    }
}

#[test]
fn eps_phys_gate_rows() {
    for kind in [EncodeKind::Defaults, EncodeKind::Mismatched] {
        let i = input(256, 1024, kind);
        check_rows(&i, &rows(&i));
    }
}

negative_control!(
    eps_phys_gate_rows,
    "rows without the 8-D interior must fail",
    expected = "the rows and their order",
    {
        let i = input(256, 0, EncodeKind::Defaults);
        check_rows(&input(256, 1024, EncodeKind::Defaults), &rows(&i))
    }
);

// ── Judging ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// A judge, as the gate's.
type Judge = fn(&GateConfig, &str, &Input) -> Result<validation::gate::GateReport, String>;

/// The judge refuses a threshold other than the register's, an input with no points, and a span that is not positive;
/// it passes the defaults and fails the mismatched encode, with a margin line on the pass and the failing rows named
/// on the fail.
fn check_judge(judge: Judge) {
    let c = config();
    let small = input(256, 1024, EncodeKind::Defaults);
    let mut other = c.clone();
    other.threshold.value = 63.0;
    let refused = judge(&other, "t", &small).expect_err("a threshold of 63 accepted");
    assert!(refused.contains("eps_phys"), "the refusal: {refused}");
    assert!(
        judge(&c, "t", &input(0, 16, EncodeKind::Defaults)).is_err(),
        "no points accepted"
    );
    for span in [0.0, -1.0, f64::NAN] {
        assert!(
            judge(
                &c,
                "t",
                &Input {
                    span,
                    ..small.clone()
                }
            )
            .is_err(),
            "the span {span} accepted"
        );
    }
    let pass = judge(&c, "t", &small).expect("judged");
    assert_eq!(
        pass.verdict,
        Verdict::Pass,
        "the defaults: {:?}",
        pass.negative_results
    );
    assert!(
        pass.scatter.iter().any(|l| l.starts_with("margin")),
        "the pass gives no margin"
    );
    let fail = judge(
        &c,
        "t",
        &Input {
            encode: EncodeKind::Mismatched,
            ..small
        },
    )
    .expect("judged");
    assert_eq!(fail.verdict, Verdict::Fail, "the mismatched encode passes");
    assert!(
        fail.negative_results[0].starts_with("failing rows: latent f64"),
        "the fail does not name its rows: {:?}",
        fail.negative_results
    );
}

#[test]
fn eps_phys_gate_judge() {
    check_judge(EpsPhysGate::judge);
}

negative_control!(
    eps_phys_gate_judge,
    "a judge that reads any threshold as the register's accepts 63, so the check must fail",
    expected = "a threshold of 63 accepted",
    check_judge(|c, n, i| {
        let mut c = c.clone();
        c.threshold.value = ledger::constants::EPS_PHYS.number();
        EpsPhysGate::judge(&c, n, i)
    })
);

// ── The fixtures ─────────────────────────────────────────────────────────────────────────────────────────────────

/// A copy of the gate's fixtures under a scratch root named `name`, with `plan/requirements.yaml`: `gate.json` and
/// `mismatched.json` as checked in, and `interior.json` with its draws cut to 2048 points per chart and 2¹⁵ latents, so
/// the runner's test runs in a debug build (`cargo xtask gate encode-eps-phys` runs the full input). `edit` then edits
/// the copy's gate directory.
fn fixture_copy(name: &str, edit: fn(&Path)) -> PathBuf {
    let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let dir = copy.join("fixtures/gates").join(NAME);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(copy.join("plan")).unwrap();
    std::fs::copy(
        root().join("plan/requirements.yaml"),
        copy.join("plan/requirements.yaml"),
    )
    .unwrap();
    for file in ["gate.json", "interior.json", "mismatched.json"] {
        std::fs::copy(
            root().join("fixtures/gates").join(NAME).join(file),
            dir.join(file),
        )
        .unwrap();
    }
    let interior = dir.join("interior.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&interior).unwrap()).unwrap();
    value["per_chart"] = 2048.into();
    value["fuzz"] = 32768.into();
    std::fs::write(&interior, value.to_string()).unwrap();
    edit(&dir);
    copy
}

/// The runner on `root`'s `fixtures/gates/encode-eps-phys/` gives every input its expected outcome: `interior.json`
/// passes and `mismatched.json` fails.
fn check_fixtures(root: &Path) {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("eps_phys_gate");
    let result = gate::run(root, NAME, &out);
    assert!(
        result.is_ok(),
        "the gate's fixtures do not give their expected outcomes: {result:?}"
    );
}

#[test]
fn eps_phys_gate_fixtures() {
    check_fixtures(&fixture_copy("eps_phys_fixtures", |_| {}));
}

negative_control!(
    eps_phys_gate_fixtures,
    "the mismatched fixture expected to pass must fail the run",
    expected = "do not give their expected outcomes",
    check_fixtures(&fixture_copy("eps_phys_flipped", |dir| {
        let negative = dir.join("mismatched.json");
        let text = std::fs::read_to_string(&negative)
            .unwrap()
            .replace(r#""expected": "fail""#, r#""expected": "pass""#);
        std::fs::write(&negative, text).unwrap();
    }))
);
