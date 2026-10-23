//! The `encode-eps-phys` gate (TASK-M2-16; REQ-ENC-024, REQ-ENC-004, PIT-3): its draws, the clamp-flag agreement, a
//! row's counts, the verdict at and past the threshold, the rows, the refusals, and the runner on
//! `fixtures/gates/encode-eps-phys/`, where the mismatched encode fails.

use std::path::{Path, PathBuf};

use kernel::decode::{self, Decoded, Latent};
use kernel::generated::links::Defaults;
use validation::eps_phys::{
    agrees, chart_row, latents, points, rows, samples, verdict, worst_row, EncodeKind, EpsPhysGate,
    Input, Row, NAME,
};
use validation::gate::{self, GateConfig, RegionCount, Verdict};
use validation::negative_control;
use validation::roundtrip::{keys, margins, Registered, Sides, INSIDE, PAST};

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

/// `worst_row` is the row with the largest worst, the first of equals, and none of no rows; `samples` counts every
/// row's interior, clamped and band samples.
fn check_worst_row(pick: fn(&[Row]) -> Option<&Row>) {
    let named = |source: &str, worst: f64| Row {
        source: source.to_owned(),
        ..row(worst, 1, 0)
    };
    assert!(pick(&[]).is_none(), "a worst row of no rows");
    let tied = [
        named("a", 1.0),
        named("b", 3.0),
        named("c", 3.0),
        named("d", 2.0),
    ];
    assert_eq!(
        pick(&tied).map(|r| r.source.as_str()),
        Some("b"),
        "the first of equals"
    );
    let zeros = [named("a", 0.0), named("b", 0.0)];
    assert_eq!(
        pick(&zeros).map(|r| r.source.as_str()),
        Some("a"),
        "the first of zeros"
    );
    let rising = [named("a", 1.0), named("b", 1.0f64.next_up())];
    assert_eq!(
        pick(&rising).map(|r| r.source.as_str()),
        Some("b"),
        "one ulp larger"
    );
    let counted = [
        Row {
            clamped: 2,
            band: 3,
            ..row(1.0, 5, 0)
        },
        Row {
            clamped: 7,
            band: 11,
            ..row(1.0, 13, 0)
        },
    ];
    assert_eq!(samples(&counted), 41, "the samples counted");
}

#[test]
fn eps_phys_gate_worst_row() {
    check_worst_row(worst_row);
}

negative_control!(
    eps_phys_gate_worst_row,
    "a pick that takes the last of equals must fail",
    expected = "the first of equals",
    check_worst_row(
        |rows| rows
            .iter()
            .rev()
            .reduce(|a, r| if r.worst.0 > a.worst.0 { r } else { a })
    )
);

// ── The rows ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// `rows` on `i`: each registered chart at f64 then f32, in the registry's order, then the 8-D interior if `i` draws
/// any latents, each row's samples all counted; the ternary mass plot's corners clamped; and, under the defaults,
/// every row within 64 `ε` with no mismatch, while the mismatched encode is far past it.
fn check_rows(i: &Input, got: &[Row]) {
    let mut want: Vec<(String, u32)> = keys()
        .into_iter()
        .flat_map(|k| [(k.to_owned(), 64), (k.to_owned(), 32)])
        .collect();
    if i.fuzz > 0 {
        want.extend([
            ("latent z (8-D)".to_owned(), 64),
            ("latent z (8-D)".to_owned(), 32),
        ]);
    }
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
    let i = input(256, 0, EncodeKind::Defaults);
    check_rows(&i, &rows(&i));
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

// ── A row's counts ───────────────────────────────────────────────────────────────────────────────────────────────

/// A chart that decodes every `(u, v)` to the one latent `z`: each of its samples lies on the same side of every clamp.
struct Fixed(&'static str, [f64; 8]);

impl Registered for Fixed {
    fn key(&self) -> &'static str {
        self.0
    }

    fn type_name(&self) -> &'static str {
        "Fixed"
    }

    fn decode64(&self, _: f64, _: f64) -> Decoded<f64> {
        decode::decode::<Defaults, f64>(&Latent::new(self.1))
    }

    fn decode32(&self, _: f32, _: f32) -> Decoded<f32> {
        decode::decode::<Defaults, f32>(&Latent::new(self.1.map(|x| x as f32)))
    }
}

/// `z[6]`'s margin, the first ratio's, at `z` with `z[6] = t`, every other control at the centre.
fn mass_margin(t: f64) -> f64 {
    let mut z = [0.0; 8];
    z[6] = t;
    margins(&decode::decode::<Defaults, f64>(&Latent::new(z)).ic)[6]
}

/// The latent with `z[6]` where its margin is 1, in the band between [`PAST`] and [`INSIDE`], by bisection.
fn band_latent() -> [f64; 8] {
    let (mut lo, mut hi) = (0.0f64, 1e3f64);
    assert!(
        mass_margin(lo) > 1.0 && mass_margin(hi) < 1.0,
        "the bracket"
    );
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if mass_margin(mid) > 1.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut z = [0.0; 8];
    z[6] = lo;
    z
}

/// A row's function, as `chart_row::<f64>`.
type RowFn = fn(&dyn Registered, &Input) -> Row;

/// `row` counts each sample on its side: 16 samples of the centre all interior, its worst at the first of them and
/// equal to its median and p99; 16 in the band all counted there; 16 past the first mass clamp all clamped, with their
/// flags set; and the mismatched encode, whose flags disagree at the clamp, counts every one a mismatch.
fn check_counts(row: RowFn) {
    let z = band_latent();
    let m = margins(&decode::decode::<Defaults, f64>(&Latent::new(z)).ic);
    let s = Sides::of(m);
    assert!(
        PAST < m[6] && m[6] < INSIDE && s.inside == 0xff & !(1 << 6) && s.past == 0,
        "the band latent is not in the band: {m:?}"
    );
    let defaults = input(16, 0, EncodeKind::Defaults);
    let centre = row(&Fixed("centre", [0.0; 8]), &defaults);
    assert_eq!(
        (
            centre.interior,
            centre.clamped,
            centre.band,
            centre.mismatches
        ),
        (16, 0, 0, 0),
        "the centre's counts"
    );
    assert_eq!(centre.worst.1, 0, "the worst's index, the first of equals");
    assert!(
        centre.worst.0 == centre.median && centre.median == centre.p99,
        "the centre's worst, median and p99 differ: {centre:?}"
    );
    let band = row(&Fixed("band", z), &defaults);
    assert_eq!(
        (band.interior, band.clamped, band.band, band.mismatches),
        (0, 0, 16, 0),
        "the band's counts"
    );
    let mut past = [0.0; 8];
    past[6] = 1e3;
    let clamped = row(&Fixed("past", past), &defaults);
    assert_eq!(
        (
            clamped.interior,
            clamped.clamped,
            clamped.band,
            clamped.mismatches
        ),
        (0, 16, 0, 0),
        "the clamped counts"
    );
    let swapped = row(&Fixed("past", past), &input(16, 0, EncodeKind::Mismatched));
    assert_eq!(swapped.mismatches, 16, "the mismatch count");
}

#[test]
fn eps_phys_gate_counts() {
    check_counts(chart_row::<f64>);
}

negative_control!(
    eps_phys_gate_counts,
    "a row that drops its band count must fail",
    expected = "the band's counts",
    check_counts(|c, i| Row {
        band: 0,
        ..chart_row::<f64>(c, i)
    })
);

negative_control!(
    eps_phys_gate_counts_worst,
    "a row whose worst is the last of equals must fail",
    expected = "the worst's index",
    check_counts(|c, i| {
        let r = chart_row::<f64>(c, i);
        let n = r.interior.saturating_sub(1);
        Row {
            worst: (r.worst.0, n),
            ..r
        }
    })
);

negative_control!(
    eps_phys_gate_counts_mismatches,
    "a row that drops its mismatches must fail",
    expected = "the mismatch count",
    check_counts(|c, i| Row {
        mismatches: 0,
        ..chart_row::<f64>(c, i)
    })
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
    let measured = rows(&small);
    let top = worst_row(&measured).expect("rows");
    let margin = format!(
        "margin {:.1}× (k = 64 over the worst, {:.2}, {} f{})",
        64.0 / top.worst.0,
        top.worst.0,
        top.source,
        top.bits
    );
    assert!(
        pass.scatter.contains(&margin),
        "the pass gives no margin {margin}: {:?}",
        pass.scatter
    );
    assert_eq!(
        pass.regions.as_ref().map(|r| r.count.clone()),
        Some(RegionCount::Recorded(28 * 256 + 2 * 1024)),
        "the pass's sample count"
    );
    assert_eq!(
        pass.negative_results,
        [format!(
            "0 of {} samples beyond ε_phys or against their clamp flags",
            28 * 256 + 2 * 1024
        )],
        "the pass's negative results"
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
    assert_eq!(
        fail.negative_results[0].matches(" interior, ").count(),
        30,
        "the fail names every row: {:?}",
        fail.negative_results
    );
    assert!(
        fail.negative_results.len() == 2 && fail.negative_results[1].contains("PIT-3"),
        "the fail's PIT-3 note: {:?}",
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
