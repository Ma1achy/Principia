//! QA tests for TASK-M2-11's calibrations (REQ-CHART-045, REQ-CHART-046; R-71, R-133; chart_reference §4.2, §4.5),
//! written from the requirements, not from the implementation. They check that the proposals are registered as
//! proposals and that the evidence in `fixtures/gates/burrau_acute/` supports them; they confirm nothing (only the
//! human does, at the M2 gate).
//! - `qa_m211_calibration_register`: `burrau_theta_min`, `burrau_theta_max` cite REQ-CHART-045 and
//!   `burrau_theta_lz_k`, `burrau_theta_lz_fraction` REQ-CHART-046 as calibrations, provisional until M2; the range is
//!   inside `(0, π/4]` at f64 and at f32; `λ_L ∈ (0, 1]`, `K > 0`; and the measurement file states the same four
//!   numbers.
//! - `qa_m211_calibration_physics`: the measurement's `ν`, lightest mass, `|U|` (at `I = 1`) and the `E = 0` row of the
//!   `θ × K` strip, recomputed *here* from §4.2's triangle and masses, by direct pairwise potential and inertia; the
//!   stated rationale (`θ_min` is the acute angle of the Euclid plane's `ν_max`; `θ_max` the fold's end); and that
//!   `K < |U|` at every `θ` of the range, so every `Φ_θ,L_z` system is bound, with the measured `E` extremes
//!   `K − |U|`; and the bound share, a 256² pixel count, within one pixel row of the converged bound area (pitfalls
//!   §3).
//! - `qa_m211_calibration_renders`: the checked-in PNGs show what the requirement asks of them at the proposed defaults:
//!   the `θ × K` strip's bound/unbound boundary at `v* = (|U(θ(u))|/K_max)^{1/γ_K}` recomputed here, column by column,
//!   with no infeasible pixel; `Φ_θ,L_z` feasible and bound on every pixel, its `L_z` panel the linear sweep
//!   `(2v − 1)·λ_L`; and the coverage plot inside `|L_z| ≤ √(2IK)`.
//!
//! **Tolerances.** The measurement file states its own: relative `1e-6` (absolute `1e-3` for the bound share); the
//! physics recomputation is held to that `1e-6`. The renders are 8-bit: a level within 2 of the expected grey, and a
//! boundary pixel within 2 rows of `v*` is not judged (the render's rounding of `(u, v)` to a pixel centre).

use std::f64::consts::FRAC_PI_4;
use std::path::PathBuf;

use ledger::constants::{Citation, ConstantBuilder, Value};
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, EUCLID_NU_MAX, GAMMA_K, K_MAX,
};

const REL: f64 = 1e-6;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/gates/burrau_acute")
}

/// The measurement file's `key`, parsed from `key = value  # …`.
fn held(text: &str, key: &str) -> f64 {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .find_map(|l| {
            let (k, rest) = l.split_once('=')?;
            (k.trim() == key).then(|| {
                rest.split('#')
                    .next()
                    .unwrap()
                    .trim()
                    .parse::<f64>()
                    .unwrap()
            })
        })
        .unwrap_or_else(|| panic!("the measurement holds no `{key}`"))
}

fn measurement() -> String {
    std::fs::read_to_string(dir().join("measurement.toml")).expect("the measurement reads")
}

fn close(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() <= REL * want.abs().max(1e-12),
        "{what}: {got} vs {want}"
    );
}

// ── qa_m211_calibration_register ─────────────────────────────────────────────────────────────────────────────────

fn check_entry(k: &ConstantBuilder, requirement: &str) {
    assert_eq!(
        k.provisional,
        Some("M2"),
        "`{}` is not provisional until M2",
        k.name
    );
    assert_eq!(
        k.citation,
        Some(Citation::Calibration(match requirement {
            "REQ-CHART-045" => "REQ-CHART-045",
            _ => "REQ-CHART-046",
        })),
        "`{}` does not cite {requirement}",
        k.name
    );
    assert!(
        matches!(k.value, Some(Value::Exact(_))),
        "`{}` holds no proposed number",
        k.name
    );
}

fn check_register(entries: [(&ConstantBuilder, &str); 4], text: &str) {
    for (k, req) in entries {
        check_entry(k, req);
    }
    let (lo, hi) = (BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number());
    for (l, h, w) in [(lo, hi, "f64"), (lo as f32 as f64, hi as f32 as f64, "f32")] {
        assert!(
            l > 0.0 && l < h && h <= FRAC_PI_4,
            "the θ range at {w} is not inside (0, π/4]: [{l}, {h}]"
        );
    }
    let (k, f) = (
        BURRAU_THETA_LZ_K.number(),
        BURRAU_THETA_LZ_FRACTION.number(),
    );
    assert!(k > 0.0, "K = {k}");
    assert!(f > 0.0 && f <= 1.0, "λ_L = {f} is outside (0, 1]");
    close(held(text, "theta_min"), lo, "the measurement's θ_min");
    close(held(text, "theta_max"), hi, "the measurement's θ_max");
    close(held(text, "k"), k, "the measurement's K");
    close(held(text, "lz_fraction"), f, "the measurement's λ_L");
}

#[test]
fn qa_m211_calibration_register() {
    check_register(
        [
            (&BURRAU_THETA_MIN, "REQ-CHART-045"),
            (&BURRAU_THETA_MAX, "REQ-CHART-045"),
            (&BURRAU_THETA_LZ_K, "REQ-CHART-046"),
            (&BURRAU_THETA_LZ_FRACTION, "REQ-CHART-046"),
        ],
        &measurement(),
    );
}

negative_control!(
    qa_m211_calibration_register,
    "the warp's K_max, which cites REQ-CHART-044, is not REQ-CHART-046's",
    expected = "does not cite REQ-CHART-046",
    check_register(
        [
            (&BURRAU_THETA_MIN, "REQ-CHART-045"),
            (&BURRAU_THETA_MAX, "REQ-CHART-045"),
            (&K_MAX, "REQ-CHART-046"),
            (&BURRAU_THETA_LZ_FRACTION, "REQ-CHART-046"),
        ],
        &measurement()
    )
);

// ── Physics, recomputed here ─────────────────────────────────────────────────────────────────────────────────────

/// `ν(θ) = sec θ − tan θ`.
fn nu(theta: f64) -> f64 {
    1.0 / theta.cos() - theta.tan()
}

/// §4.2's system at `ν`: masses `(c, b, a)/(a + b + c)` and the triangle `r₀ = 0`, `r₁ = (a, 0)`, `r₂ = (0, b)`.
fn system(nu: f64) -> ([f64; 3], [[f64; 2]; 3]) {
    let (a, b, c) = (1.0 - nu * nu, 2.0 * nu, 1.0 + nu * nu);
    let p = a + b + c;
    ([c / p, b / p, a / p], [[0.0, 0.0], [a, 0.0], [0.0, b]])
}

/// `|U|` at `I = 1` (G = 1): `Σ mᵢmⱼ/rᵢⱼ` times `√I` of the raw triangle about its CoM (`U ∝ 1/ℓ`, `I ∝ ℓ²`).
fn u_abs(theta: f64) -> f64 {
    let (m, r) = system(nu(theta));
    let com = [0, 1].map(|x| (0..3).map(|i| m[i] * r[i][x]).sum::<f64>());
    let inertia: f64 = (0..3)
        .map(|i| m[i] * ((r[i][0] - com[0]).powi(2) + (r[i][1] - com[1]).powi(2)))
        .sum();
    let mut u = 0.0;
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let d = ((r[i][0] - r[j][0]).powi(2) + (r[i][1] - r[j][1]).powi(2)).sqrt();
        u += m[i] * m[j] / d;
    }
    u * inertia.sqrt()
}

/// The `θ × K` strip's `E = 0` row at `θ`: `K_max·v^γ_K = |U|`.
fn v_escape(theta: f64) -> f64 {
    (u_abs(theta) / K_MAX.number()).powf(1.0 / GAMMA_K.number())
}

fn check_physics(text: &str, u_of: fn(f64) -> f64) {
    let (lo, hi) = (BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number());
    close(held(text, "nu_theta_min"), nu(lo), "ν(θ_min)");
    close(held(text, "nu_theta_max"), nu(hi), "ν(θ_max)");
    let light = |t: f64| system(nu(t)).0.into_iter().fold(f64::INFINITY, f64::min);
    close(
        held(text, "m_light_theta_min"),
        light(lo),
        "the lightest mass at θ_min",
    );
    close(
        held(text, "m_light_theta_max"),
        light(hi),
        "the lightest mass at θ_max",
    );
    close(held(text, "u_theta_min"), u_of(lo), "|U| at θ_min");
    close(held(text, "u_theta_max"), u_of(hi), "|U| at θ_max");
    close(
        held(text, "u_classical"),
        u_of(0.75f64.atan()),
        "|U| at arctan(3/4)",
    );
    close(
        held(text, "t_escape_theta_min"),
        v_escape(lo),
        "the E = 0 row at θ_min",
    );
    close(
        held(text, "t_escape_theta_max"),
        v_escape(hi),
        "the E = 0 row at θ_max",
    );
    // The stated rationale: θ_min is the Euclid plane's ν_max's acute angle (to f32), θ_max the fold's end (to f32,
    // from below).
    let nu_max = EUCLID_NU_MAX.number();
    let theta_euclid = ((1.0 - nu_max * nu_max) / (2.0 * nu_max)).atan();
    assert!(
        (lo - theta_euclid).abs() <= f32::EPSILON as f64 * theta_euclid,
        "θ_min {lo} is not the acute angle of ν_max = {nu_max}, {theta_euclid}"
    );
    assert!(
        hi <= FRAC_PI_4 && FRAC_PI_4 - hi <= f32::EPSILON as f64,
        "θ_max {hi} is not the fold's end π/4"
    );
    // K below |U| at every θ of the range: every Φ_θ,L_z system bound; E extremes K − |U|.
    let k = BURRAU_THETA_LZ_K.number();
    let us: Vec<f64> = (0..=4096)
        .map(|i| u_of(lo + (hi - lo) * i as f64 / 4096.0))
        .collect();
    let (umin, umax) = us
        .iter()
        .fold((f64::INFINITY, 0.0f64), |(a, b), &x| (a.min(x), b.max(x)));
    assert!(
        k < umin,
        "K = {k} is not below |U| = {umin} somewhere on the range"
    );
    close(
        held(text, "e_max_theta_lz"),
        k - umin,
        "Φ_θ,L_z's largest E",
    );
    close(held(text, "e_min_theta_lz"), k - umax, "Φ_θ,L_z's least E");
    close(
        held(text, "two_k_over_u_theta_min"),
        2.0 * k / u_of(lo),
        "2K/|U| at θ_min",
    );
    close(
        held(text, "two_k_over_u_theta_max"),
        2.0 * k / u_of(hi),
        "2K/|U| at θ_max",
    );
    // The bound share, a 256² pixel count, against its converged value (pitfalls §3: a quantity quoted from one
    // sampling must agree with the refined one): the area under `min(v*(θ(u)), 1)`, by a 65536-point midpoint rule.
    // A column's count is within one pixel of `v*`, so the two agree within `1/256`.
    let fine = 65536;
    let area: f64 = (0..fine)
        .map(|i| v_escape(lo + (hi - lo) * (i as f64 + 0.5) / fine as f64).min(1.0))
        .sum::<f64>()
        / fine as f64;
    let share = held(text, "bound_share_theta_k");
    assert!(
        (share - area).abs() <= 1.0 / 256.0,
        "the bound share {share} does not converge to the strip's bound area {area}"
    );
    for key in [
        "infeasible_theta_k_f64",
        "infeasible_theta_k_f32",
        "infeasible_theta_lz_f64",
        "infeasible_theta_lz_f32",
        "nonfinite_f64",
    ] {
        assert_eq!(held(text, key), 0.0, "`{key}` is not 0");
    }
}

#[test]
fn qa_m211_calibration_physics() {
    check_physics(&measurement(), u_abs);
}

negative_control!(
    qa_m211_calibration_physics,
    "a potential without the I = 1 scale gauge is not the measurement's |U|",
    expected = "|U| at θ_min",
    check_physics(&measurement(), |t| u_abs(t) / 2.0)
);

// ── qa_m211_calibration_renders ──────────────────────────────────────────────────────────────────────────────────

struct Gray {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

fn read_png(name: &str) -> Gray {
    let file = std::fs::File::open(dir().join(name)).expect("the render reads");
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("a PNG");
    let mut buf = vec![0; reader.output_buffer_size().expect("a size")];
    let info = reader.next_frame(&mut buf).expect("a frame");
    assert_eq!(info.color_type, png::ColorType::Grayscale, "{name} is grey");
    assert_eq!(info.bit_depth, png::BitDepth::Eight, "{name} is 8-bit");
    buf.truncate(info.buffer_size());
    Gray {
        w: info.width as usize,
        h: info.height as usize,
        px: buf,
    }
}

const N: usize = 256;

/// The level of panel `panel`'s pixel `(i, j)`, `i` across (`u`), `j` up (`v`).
fn level(img: &Gray, panel: usize, i: usize, j: usize) -> u8 {
    img.px[(img.h - 1 - j) * img.w + panel * (N + 8) + i]
}

fn at(i: usize) -> f64 {
    i as f64 / (N - 1) as f64
}

/// The `θ × K` render's first panel against `v*(θ(u))`: white below, grey above, never black.
fn check_theta_k_render(img: &Gray, theta_range: [f64; 2]) {
    assert_eq!((img.w, img.h), (3 * N + 16, N), "theta_k_fields.png's size");
    let [lo, hi] = theta_range;
    for i in 0..N {
        let v_star = v_escape(lo + (hi - lo) * at(i));
        for j in 0..N {
            let l = level(img, 0, i, j);
            assert_ne!(
                l, 0,
                "the θ × K strip shows an infeasible pixel at ({i}, {j})"
            );
            if (at(j) - v_star).abs() <= 2.0 / (N - 1) as f64 {
                continue;
            }
            let want = if at(j) < v_star { 255 } else { 128 };
            assert_eq!(
                l, want,
                "the θ × K strip's bound/unbound state at column {i}, row {j} (v* = {v_star})"
            );
        }
    }
}

#[test]
fn qa_m211_calibration_renders() {
    let range = [BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number()];
    check_theta_k_render(&read_png("theta_k_fields.png"), range);

    let lz = read_png("theta_lz_fields.png");
    assert_eq!((lz.w, lz.h), (3 * N + 16, N), "theta_lz_fields.png's size");
    let f = BURRAU_THETA_LZ_FRACTION.number();
    for i in 0..N {
        for j in 0..N {
            assert_eq!(
                level(&lz, 0, i, j),
                255,
                "Φ_θ,L_z is not feasible and bound at ({i}, {j})"
            );
            let want = ((2.0 * at(j) - 1.0) * f + 1.0) / 2.0 * 255.0;
            assert!(
                (level(&lz, 1, i, j) as f64 - want).abs() <= 2.0,
                "Φ_θ,L_z's L_z panel at ({i}, {j}) is not (2v − 1)·λ_L"
            );
        }
    }

    // The coverage plot: L_z(p) across at ±1.1·√(2K); every plotted (white) pixel inside |L_z| ≤ √(2IK).
    let cov = read_png("theta_lz_coverage.png");
    assert_eq!((cov.w, cov.h), (N, N), "theta_lz_coverage.png's size");
    let x = |l: f64| (l / 1.1 + 1.0) / 2.0 * (N - 1) as f64;
    let (left, right) = (x(-1.0).floor() as usize, x(1.0).ceil() as usize);
    let mut white = 0;
    for row in 0..N {
        for col in 0..N {
            if cov.px[row * N + col] == 255 {
                white += 1;
                assert!(
                    (left..=right).contains(&col),
                    "a coverage pixel at column {col} lies outside |L_z| ≤ √(2IK) ({left}..={right})"
                );
            }
        }
    }
    // λ_L = 1: the band reaches both feasible edges on every θ row.
    for row in 0..N {
        let line = &cov.px[row * N..(row + 1) * N];
        assert!(
            line[left..=left + 1].contains(&255) && line[right - 1..=right].contains(&255),
            "row {row} of the coverage does not reach both edges"
        );
    }
    assert!(white > 0);
}

negative_control!(
    qa_m211_calibration_renders,
    "the θ × K render read against another θ range (θ_max = π/8) is not the render at the proposed defaults",
    expected = "bound/unbound state",
    check_theta_k_render(
        &read_png("theta_k_fields.png"),
        [BURRAU_THETA_MIN.number(), FRAC_PI_4 / 2.0]
    )
);
