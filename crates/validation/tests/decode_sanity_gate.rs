//! The `decode-sanity` gate (TASK-M2-19; REQ-ENC-027, R-71): the shift, a row's worsts and counts, the verdict at and
//! past `k`, the rows, the refusals, and the runner on `fixtures/gates/decode-sanity/`, where the shifted input fails.
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use kernel::decode::Ic;
use validation::decode_sanity::{fails, row, rows, shifted, DecodeSanityGate, Input, Row, NAME};
use validation::gate::{self, Gate, GateConfig, RegionCount, Verdict};
use validation::negative_control;

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn input(per_chart: u32, fuzz: u32, shift: f64) -> Input {
    Input {
        description: "a test input".to_owned(),
        seed: 5,
        per_chart,
        fuzz,
        span: 16.0,
        shift,
    }
}

fn config() -> GateConfig {
    gate::config(&root(), NAME).expect("the gate's gate.json")
}

/// A state on the section with `‖Σmᵢrᵢ‖ = 1` over `R_x = 4` and `‖Σpᵢ‖ = 0.5` over `P_x = 1`.
fn off() -> Ic<f64> {
    Ic {
        m: [0.5, 0.25, 0.25],
        r: [[0.0, 0.0], [4.0, 0.0], [0.0, 0.0]],
        p: [[0.5, 0.0], [0.0, 0.0], [0.0, 0.0]],
    }
}

/// `row` over `xs` at f64 and f32: the worsts in each precision's epsilon, NaN read as `+∞`, the samples and the
/// masses not positive counted, the shift moving every position along `x` only.
fn check_row(shifted: fn(&Ic<f64>, f64) -> Ic<f64>) {
    let shift = 0.75;
    let moved = shifted(&off(), shift);
    assert_eq!(
        moved.r,
        [[shift, 0.0], [4.0 + shift, 0.0], [shift, 0.0]],
        "the shift"
    );
    assert_eq!((moved.m, moved.p), (off().m, off().p));
    let r = row::<f64>("s", 0.0, [off(), off()].into_iter());
    assert_eq!(
        (r.source.as_str(), r.bits, r.samples, r.nonpositive),
        ("s", 64, 2, 0)
    );
    assert_eq!(
        (r.com, r.momentum),
        (0.25 / f64::EPSILON, 0.5 / f64::EPSILON),
        "the worsts"
    );
    let narrow = Ic {
        m: off().m.map(|v| v as f32),
        r: off().r.map(|v| v.map(|x| x as f32)),
        p: off().p.map(|v| v.map(|x| x as f32)),
    };
    let r = row::<f32>("s", 0.0, [narrow].into_iter());
    assert_eq!(
        (r.bits, r.com),
        (32, 0.25 / f64::from(f32::EPSILON)),
        "f32's epsilon"
    );
    let mut bad = off();
    bad.m[1] = 0.0;
    bad.p[0][0] = f64::NAN;
    let r = row::<f64>("s", 0.0, [off(), bad].into_iter());
    assert_eq!(
        (r.nonpositive, r.momentum),
        (1, f64::INFINITY),
        "a NaN or a zero mass is dropped"
    );
}

#[test]
fn decode_sanity_gate_row() {
    check_row(shifted);
}

negative_control!(
    decode_sanity_gate_row,
    "a shift along both axes is not the gate's",
    expected = "the shift",
    check_row(|x, s| Ic {
        r: x.r.map(|r| [r[0] + s, r[1] + s]),
        ..*x
    })
);

fn made(com: f64, momentum: f64, samples: usize, nonpositive: usize) -> Row {
    Row {
        source: "s".to_owned(),
        bits: 64,
        samples,
        com,
        momentum,
        nonpositive,
    }
}

/// `fails` at `k`'s edge: each worst at `k` passes, just past fails; no samples or a mass not positive fails.
fn check_fails(k: f64) {
    assert!(!fails(&made(16.0, 16.0, 4, 0), k), "a worst at k fails");
    assert!(
        fails(&made(16.0f64.next_up(), 1.0, 4, 0), k),
        "the CoM past k passes"
    );
    assert!(
        fails(&made(1.0, 16.0f64.next_up(), 4, 0), k),
        "Σp past k passes"
    );
    assert!(fails(&made(1.0, 1.0, 0, 0), k), "no samples passes");
    assert!(
        fails(&made(1.0, 1.0, 4, 1), k),
        "a mass not positive passes"
    );
}

#[test]
fn decode_sanity_gate_fails() {
    check_fails(16.0);
}

negative_control!(
    decode_sanity_gate_fails,
    "below the worst, k fails it",
    expected = "a worst at k fails",
    check_fails(15.0)
);

/// `rows`: each registered chart at f64 then f32, then the latent draws, each with its sample count, and the measured
/// worsts within `k`.
fn check_rows(i: &Input, k: f64) {
    let got = rows(i);
    assert_eq!(got.len(), 30, "the rows");
    for (n, r) in got.iter().enumerate() {
        assert_eq!(
            r.bits,
            if n % 2 == 0 { 64 } else { 32 },
            "row {n}'s precision"
        );
        let want = if n >= 28 { i.fuzz } else { i.per_chart } as usize;
        assert_eq!(r.samples, want, "{}'s samples", r.source);
        assert!(!fails(r, k), "{} f{} fails: {r:?}", r.source, r.bits);
    }
    assert_eq!(got[28].source, "latent z (8-D)");
}

#[test]
fn decode_sanity_gate_rows() {
    check_rows(&input(64, 256, 0.0), 16.0);
}

negative_control!(
    decode_sanity_gate_rows,
    "the shifted draws fail",
    expected = "fails",
    check_rows(&input(64, 256, 0.1), 16.0)
);

type Judge = fn(&GateConfig, &str, &Input) -> Result<validation::gate::GateReport, String>;

/// The judge refuses a threshold other than the register's, an input with no samples and a span that is not
/// positive; it passes the unshifted draws with a margin line and fails the shifted ones, naming every row.
fn check_judge(judge: Judge) {
    let c = config();
    let small = input(64, 256, 0.0);
    let mut other = c.clone();
    other.threshold.value = 15.0;
    let refused = judge(&other, "t", &small).expect_err("a threshold of 15 accepted");
    assert!(refused.contains("decode_sanity"), "the refusal: {refused}");
    assert!(
        judge(&c, "t", &input(0, 16, 0.0)).is_err(),
        "no points accepted"
    );
    assert!(
        judge(&c, "t", &input(16, 0, 0.0)).is_err(),
        "no latents accepted"
    );
    for span in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let i = Input {
            span,
            ..small.clone()
        };
        assert!(judge(&c, "t", &i).is_err(), "the span {span} accepted");
    }
    let pass = judge(&c, "t", &small).expect("judged");
    assert_eq!(pass.verdict, Verdict::Pass, "{:?}", pass.negative_results);
    let worst = rows(&small)
        .iter()
        .map(|r| r.com.max(r.momentum))
        .fold(0.0, f64::max);
    let margin = format!(
        "margin {:.1}× (k = 16 over the worst, {worst:.2})",
        16.0 / worst
    );
    assert!(
        pass.scatter.contains(&margin),
        "no margin {margin}: {:?}",
        pass.scatter
    );
    let n = 28 * 64 + 2 * 256;
    assert_eq!(
        pass.regions.map(|r| r.count),
        Some(RegionCount::Recorded(n))
    );
    assert_eq!(
        pass.negative_results,
        [format!(
            "0 of {n} decoded samples past 16·ε or with a mass not positive"
        )]
    );
    let fail = judge(
        &c,
        "t",
        &Input {
            shift: 0.1,
            ..small
        },
    )
    .expect("judged");
    assert_eq!(fail.verdict, Verdict::Fail, "the shifted draws pass");
    assert!(
        fail.negative_results[0].starts_with("failing rows: latent f64"),
        "{:?}",
        fail.negative_results
    );
    assert_eq!(
        fail.negative_results[0].matches(" (CoM ").count(),
        30,
        "every row named"
    );
}

#[test]
fn decode_sanity_gate_judge() {
    check_judge(DecodeSanityGate::judge);
}

negative_control!(
    decode_sanity_gate_judge,
    "a judge that reads any threshold as the register's accepts 15",
    expected = "a threshold of 15 accepted",
    check_judge(|c, n, i| {
        let mut c = c.clone();
        c.threshold.value = ledger::constants::DECODE_SANITY.number();
        DecodeSanityGate::judge(&c, n, i)
    })
);

/// A copy of the gate's fixtures under a scratch root `name`, with `plan/requirements.yaml`, `fuzz.json` cut to 1024
/// points per chart and 2¹⁴ latents for a debug build; `edit` then edits the copy's gate directory.
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
    for file in ["gate.json", "fuzz.json", "shifted.json"] {
        std::fs::copy(
            root().join("fixtures/gates").join(NAME).join(file),
            dir.join(file),
        )
        .unwrap();
    }
    let fuzz = dir.join("fuzz.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&fuzz).unwrap()).unwrap();
    value["per_chart"] = 1024.into();
    value["fuzz"] = 16384.into();
    std::fs::write(&fuzz, value.to_string()).unwrap();
    edit(&dir);
    copy
}

/// The runner on `root`'s fixtures gives every input its expected outcome, and the gate refuses a fixture that is no
/// input, naming the gate.
fn check_fixtures(root: &Path) {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(NAME);
    let result = gate::run(root, NAME, &out);
    assert!(
        result.is_ok(),
        "the gate's fixtures do not give their expected outcomes: {result:?}"
    );
    let refused = DecodeSanityGate
        .run(&config(), "x", &serde_json::json!({ "seed": 1 }))
        .expect_err("a fixture that is no input is judged");
    assert!(
        refused.contains("is not a decode-sanity input"),
        "{refused}"
    );
    assert_eq!(DecodeSanityGate.name(), "decode-sanity");
}

#[test]
fn decode_sanity_gate_fixtures() {
    check_fixtures(&fixture_copy("decode_sanity_fixtures", |_| {}));
}

negative_control!(
    decode_sanity_gate_fixtures,
    "the shifted fixture expected to pass must fail the run",
    expected = "do not give their expected outcomes",
    check_fixtures(&fixture_copy("decode_sanity_flipped", |dir| {
        let negative = dir.join("shifted.json");
        let text = std::fs::read_to_string(&negative)
            .unwrap()
            .replace(r#""expected": "fail""#, r#""expected": "pass""#);
        std::fs::write(&negative, text).unwrap();
    }))
);
