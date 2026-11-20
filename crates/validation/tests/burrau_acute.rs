//! The acute-angle Burrau charts' calibration evidence (REQ-CHART-045, REQ-CHART-046; R-71, R-133; chart_reference
//! §4.5): `θ_min` and `θ_max`, and `Φ_θ,L_z`'s `K` and `λ_L`, the constants register's proposed entries, measured
//! against what `Φ_θ,K` (the `θ × K` strip) and `Φ_θ,L_z` show at them.
//! - `burrau_acute_evidence`: the measurement, `fixtures/gates/burrau_acute/measurement.toml`, is this test's, within a
//!   relative 10⁻⁶, and the renders beside it are the PNGs it draws. Regenerate both with `PRIN_WRITE_BURRAU_ACUTE=1`.
//!   The bound share, a pixel count a platform's `sin`, `cos` or `powf` can move across `E = 0`, is checked within
//!   10⁻³ (a few pixels of 256²).
//!
//! **What it measures**, at the register's defaults (`Φ_θ,K` under the warp's `K_max` and `γ_K`, REQ-CHART-044),
//! through the decode stage's CPU dispatch, `engine::decode_dispatch::decode_only` (TASK-M2-06; R-133: decode-time
//! fields, no integrated field), and the same source at f32 for the f32 counts:
//! - `ν`, the lightest mass and `|U|` at `θ_min` and `θ_max` (`U` the configuration's potential, `I = 1`), and the
//!   row `t = (|U|/K_max)^{1/γ_K}` where `Φ_θ,K` reaches the escape energy `E = 0` at each: the strip's span in `E`;
//! - the share of the `θ × K` strip's 256² pixels that are bound, `E < 0`;
//! - on `Φ_θ,L_z`, the largest and least `E` (every system bound where the largest is negative), `2K/|U|` at both ends
//!   of the `θ` range, and the largest `|L_z(p)|/√(2K)` (the band reaching the feasible edge);
//! - the construction's infeasible pixels and the non-finite decodes on 257² grids with their edges, both charts, at
//!   f64 and f32: 0.
//!
//! **The renders** (8-bit grey PNGs, each panel 256 × 256, `u` across, `v` up): `theta_k_fields.png`, the `θ × K`
//! strip: bound (white), unbound (grey) or infeasible or non-finite (black); `E` from the strip's least to its largest,
//! black to white; the lightest mass, 0 to 1/3; `theta_lz_fields.png`, `Φ_θ,L_z`, the same first panel, `L_z(p)/√(2K)`
//! from −1 to 1, and `E` from its least to its largest; `theta_lz_coverage.png`, each `Φ_θ,L_z` pixel plotted at its
//! decode's `L_z(p)` across (`±1.1·√(2K)`) and its `θ` up, 256 × 256, with the feasible edges `L_z = ±√(2K)` in grey
//! one pixel outside them.

use std::path::{Path, PathBuf};

use engine::decode_dispatch::decode_only;
use kernel::chart::burrau_acute::{decode_theta_k, decode_theta_lz};
use kernel::chart::{BurrauThetaK, BurrauThetaLz, ChartDecode};
use kernel::decode::descriptor;
use kernel::decode::invariant::Outcome;
use kernel::generated::links::Defaults;
use kernel::stage::decode::DecodeOut;
use validation::decode_f32_eps::physics_key;
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, GAMMA_K, K_MAX,
};

const DIR: &str = "fixtures/gates/burrau_acute";
const MEASUREMENT: &str = "measurement.toml";

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DIR)
}

/// `Φ_θ,K` at the register's defaults.
fn theta_k() -> BurrauThetaK<f64> {
    BurrauThetaK {
        theta_range: [BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number()],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    }
}

/// `Φ_θ,L_z` at the register's defaults.
fn theta_lz() -> BurrauThetaLz<f64> {
    BurrauThetaLz {
        theta_range: [BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number()],
        k: BURRAU_THETA_LZ_K.number(),
        lz_fraction: BURRAU_THETA_LZ_FRACTION.number(),
    }
}

/// The decode stage's CPU dispatch at `(u, v)` (`decode_only`, the physics kernel, the default links).
fn dispatch<C: ChartDecode>(chart: &C, u: f64, v: f64) -> DecodeOut<f64>
where
    f64: From<C::Param>,
{
    decode_only::<C, Defaults>(chart, [u, v], &physics_key()).expect("decode_only decodes")
}

/// `E = K₀ + V₀` of a decode.
fn energy(d: &DecodeOut<f64>) -> f64 {
    descriptor::energy(&d.descriptor)
}

/// `L_z(p) = Σ (rᵢ × pᵢ)_z` of a decode.
fn lz_of(d: &DecodeOut<f64>) -> f64 {
    (0..3)
        .map(|i| d.ic.r[i][0] * d.ic.p[i][1] - d.ic.r[i][1] * d.ic.p[i][0])
        .sum()
}

/// Whether every number of a decode's `(m, r, p)` is finite.
fn finite(d: &DecodeOut<f64>) -> bool {
    d.ic.m
        .iter()
        .chain(d.ic.r.iter().flatten())
        .chain(d.ic.p.iter().flatten())
        .all(|x| x.is_finite())
}

/// The `(u, v)` grid with `n` points per side, its edges included, row by row from `v = 0`.
fn grid(n: usize) -> impl Iterator<Item = (f64, f64)> {
    let at = move |i: usize| i as f64 / (n - 1) as f64;
    (0..n).flat_map(move |j| (0..n).map(move |i| (at(i), at(j))))
}

/// The infeasible pixels and the non-finite decodes of both charts on the 257² grid, at f64 (through the dispatch) and
/// f32 (the same source): `[θK f64, θK f32, θL_z f64, θL_z f32, non-finite]`.
fn counts() -> [f64; 5] {
    let (k, lz) = (theta_k(), theta_lz());
    let k32 = BurrauThetaK {
        theta_range: k.theta_range.map(|x| x as f32),
        k_max: k.k_max as f32,
        gamma_k: k.gamma_k as f32,
    };
    let lz32 = BurrauThetaLz {
        theta_range: lz.theta_range.map(|x| x as f32),
        k: lz.k as f32,
        lz_fraction: lz.lz_fraction as f32,
    };
    let mut c = [0.0; 5];
    for (u, v) in grid(257) {
        let (u32_, v32) = (u as f32, v as f32);
        c[0] += f64::from(u8::from(decode_theta_k(&k, u, v).1 != Outcome::Feasible));
        c[1] += f64::from(u8::from(
            decode_theta_k(&k32, u32_, v32).1 != Outcome::Feasible,
        ));
        c[2] += f64::from(u8::from(decode_theta_lz(&lz, u, v).1 != Outcome::Feasible));
        c[3] += f64::from(u8::from(
            decode_theta_lz(&lz32, u32_, v32).1 != Outcome::Feasible,
        ));
        let fine = finite(&dispatch(&k, u, v)) && finite(&dispatch(&lz, u, v));
        c[4] += f64::from(u8::from(!fine));
    }
    c
}

/// The measurement: each key, its value and what it is.
fn measure() -> Vec<(&'static str, f64, &'static str)> {
    let (k, lz) = (theta_k(), theta_lz());
    let at = |u: f64| dispatch(&k, u, 0.0);
    let (lo, hi) = (at(0.0), at(1.0));
    let light = |d: &DecodeOut<f64>| d.ic.m.iter().copied().fold(f64::INFINITY, f64::min);
    let u_abs = |d: &DecodeOut<f64>| -d.descriptor.V_0;
    let t_escape = |d: &DecodeOut<f64>| (u_abs(d) / k.k_max).powf(1.0 / k.gamma_k);
    let classical = BurrauThetaK {
        theta_range: [0.75f64.atan(); 2],
        ..k
    };
    let n = 256;
    let bound = grid(n)
        .filter(|&(u, v)| energy(&dispatch(&k, u, v)) < 0.0)
        .count() as f64
        / (n * n) as f64;
    let (mut e_lo, mut e_hi, mut span) = (f64::INFINITY, f64::NEG_INFINITY, 0.0f64);
    let root = (2.0 * lz.k).sqrt();
    for (u, v) in grid(n) {
        let d = dispatch(&lz, u, v);
        let e = energy(&d);
        e_lo = e_lo.min(e);
        e_hi = e_hi.max(e);
        span = span.max(lz_of(&d).abs() / root);
    }
    let c = counts();
    vec![
        (
            "theta_min",
            k.theta_range[0],
            "the proposed θ_min, rad (constants register, provisional until M2)",
        ),
        (
            "theta_max",
            k.theta_range[1],
            "the proposed θ_max, rad (constants register, provisional until M2)",
        ),
        (
            "k",
            lz.k,
            "the proposed K of Φ_θ,L_z (constants register, provisional until M2)",
        ),
        (
            "lz_fraction",
            lz.lz_fraction,
            "the proposed λ_L of Φ_θ,L_z (constants register, provisional until M2)",
        ),
        ("nu_theta_min", k.nu(0.0), "ν at θ_min"),
        ("nu_theta_max", k.nu(1.0), "ν at θ_max"),
        (
            "m_light_theta_min",
            light(&lo),
            "the lightest mass at θ_min",
        ),
        (
            "m_light_theta_max",
            light(&hi),
            "the lightest mass at θ_max",
        ),
        ("u_theta_min", u_abs(&lo), "|U| at θ_min, I = 1"),
        ("u_theta_max", u_abs(&hi), "|U| at θ_max, I = 1"),
        (
            "u_classical",
            u_abs(&dispatch(&classical, 0.0, 0.0)),
            "|U| at the classical θ = arctan(3/4)",
        ),
        (
            "t_escape_theta_min",
            t_escape(&lo),
            "the row v where E = 0 on Φ_θ,K at θ_min",
        ),
        (
            "t_escape_theta_max",
            t_escape(&hi),
            "the row v where E = 0 on Φ_θ,K at θ_max",
        ),
        (
            "bound_share_theta_k",
            bound,
            "share of Φ_θ,K's 256² pixels with E < 0",
        ),
        (
            "e_max_theta_lz",
            e_hi,
            "the largest E over Φ_θ,L_z's 256² pixels",
        ),
        (
            "e_min_theta_lz",
            e_lo,
            "the least E over Φ_θ,L_z's 256² pixels",
        ),
        (
            "two_k_over_u_theta_min",
            2.0 * lz.k / u_abs(&lo),
            "2K/|U| on Φ_θ,L_z at θ_min",
        ),
        (
            "two_k_over_u_theta_max",
            2.0 * lz.k / u_abs(&hi),
            "2K/|U| on Φ_θ,L_z at θ_max",
        ),
        (
            "lz_span",
            span,
            "the largest |L_z(p)|/√(2K) over Φ_θ,L_z's 256² pixels",
        ),
        (
            "infeasible_theta_k_f64",
            c[0],
            "infeasible pixels of Φ_θ,K, 257² with edges, f64",
        ),
        ("infeasible_theta_k_f32", c[1], "the same, f32"),
        (
            "infeasible_theta_lz_f64",
            c[2],
            "infeasible pixels of Φ_θ,L_z, 257² with edges, f64",
        ),
        ("infeasible_theta_lz_f32", c[3], "the same, f32"),
        (
            "nonfinite_f64",
            c[4],
            "non-finite decodes, both charts, 257² with edges, f64",
        ),
    ]
}

/// The measurement file's text for `values`.
fn measurement_text(values: &[(&str, f64, &str)]) -> String {
    let mut s = String::from(
        "# The acute-angle Burrau charts' calibration evidence (REQ-CHART-045, REQ-CHART-046; R-71, R-133): θ_min,\n\
         # θ_max, and Φ_θ,L_z's K and λ_L, proposed in the constants register, provisional until the M2 gate.\n\
         # Measured by crates/validation/tests/burrau_acute.rs, burrau_acute_evidence, through the decode stage's CPU\n\
         # dispatch (decode_only) and the same source at f32; regenerate with PRIN_WRITE_BURRAU_ACUTE=1, which also\n\
         # redraws the renders beside it. Compared at a relative 1e-6, except the bound share, a pixel count, at an\n\
         # absolute 1e-3.\n",
    );
    for (k, v, what) in values {
        s.push_str(&format!("{k} = {v:e}  # {what}\n"));
    }
    s
}

/// The number `text` holds at `k`.
fn held(text: &str, k: &str) -> f64 {
    let line = text
        .lines()
        .find(|l| l.split('=').next().is_some_and(|key| key.trim() == k))
        .unwrap_or_else(|| panic!("{MEASUREMENT}: no `{k}`"));
    line.split('=')
        .nth(1)
        .and_then(|r| r.split('#').next())
        .map(str::trim)
        .and_then(|r| r.parse().ok())
        .unwrap_or_else(|| panic!("{MEASUREMENT}: `{k}` holds no number"))
}

/// Each key of `text` holds its measured value within a relative 10⁻⁶, the bound share within an absolute 10⁻³; and
/// the measurement shows what it is evidence of: every `Φ_θ,L_z` system bound, the band reaching `√(2K)` and no
/// further, `Φ_θ,K`'s `E = 0` row inside the strip at both ends, and no infeasible or non-finite pixel.
fn check_measurement(text: &str, values: &[(&str, f64, &str)]) {
    for (k, v, _) in values {
        let held = held(text, k);
        let tol = if *k == "bound_share_theta_k" {
            1e-3
        } else {
            1e-6 * v.abs()
        };
        assert!(
            (held - v).abs() <= tol,
            "{MEASUREMENT}: `{k}` holds {held:e}, measured {v:e}"
        );
    }
    let get = |k: &str| {
        values
            .iter()
            .find(|(key, _, _)| *key == k)
            .map(|(_, v, _)| *v)
            .unwrap_or_else(|| panic!("no measured `{k}`"))
    };
    assert!(get("e_max_theta_lz") < 0.0, "a Φ_θ,L_z system is unbound");
    let span = get("lz_span");
    assert!(
        (span - 1.0).abs() <= 1e-12,
        "the L_z band reaches {span} of √(2K)"
    );
    for k in ["t_escape_theta_min", "t_escape_theta_max"] {
        assert!(get(k) > 0.0 && get(k) < 1.0, "`{k}` is outside the strip");
    }
    for k in [
        "infeasible_theta_k_f64",
        "infeasible_theta_k_f32",
        "infeasible_theta_lz_f64",
        "infeasible_theta_lz_f32",
        "nonfinite_f64",
    ] {
        assert_eq!(get(k), 0.0, "`{k}`");
    }
}

// ── The renders ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A grey image, `w` × `h`, row 0 at the top.
struct Image {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Image {
    fn new(w: usize, h: usize, fill: u8) -> Self {
        Image {
            w,
            h,
            px: vec![fill; w * h],
        }
    }

    /// Sets the pixel at `(x, y)`, `y` up from the bottom, both in `[0, 1]`, if inside.
    fn plot(&mut self, x: f64, y: f64, v: u8) {
        if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) {
            let i = ((x * (self.w - 1) as f64).round()) as usize;
            let j = self.h - 1 - ((y * (self.h - 1) as f64).round()) as usize;
            self.px[j * self.w + i] = v;
        }
    }

    fn write(&self, path: &Path) {
        let file = std::fs::File::create(path).expect("the render writes");
        let mut e = png::Encoder::new(std::io::BufWriter::new(file), self.w as u32, self.h as u32);
        e.set_color(png::ColorType::Grayscale);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().expect("the PNG header");
        w.write_image_data(&self.px).expect("the PNG data");
    }
}

/// `x` mapped from `[lo, hi]` to a grey level.
fn grey(x: f64, lo: f64, hi: f64) -> u8 {
    ((x - lo) / (hi - lo) * 255.0).round().clamp(0.0, 255.0) as u8
}

/// The first panel's level: white for a bound decode, grey for an unbound one, black for an infeasible or non-finite.
fn state(d: &DecodeOut<f64>, feasible: bool) -> u8 {
    if !(feasible && finite(d)) {
        0
    } else if energy(d) < 0.0 {
        255
    } else {
        128
    }
}

/// A three-panel image: `n` × `n` panels side by side, 8 pixels apart; `levels(u, v)` gives a pixel's three levels.
fn panels(n: usize, levels: impl Fn(f64, f64) -> [u8; 3]) -> Image {
    let mut img = Image::new(3 * n + 16, n, 0);
    for (q, (u, v)) in grid(n).enumerate() {
        let (j, i) = (q / n, q % n);
        let row = n - 1 - j;
        for (panel, level) in levels(u, v).into_iter().enumerate() {
            img.px[row * img.w + panel * (n + 8) + i] = level;
        }
    }
    img
}

/// The three renders, from the f64 dispatch at the register's defaults, 256 × 256 per panel.
fn renders() -> [(&'static str, Image); 3] {
    let (k, lz) = (theta_k(), theta_lz());
    let n = 256;
    let range = |f: &dyn Fn(f64, f64) -> f64| {
        grid(n)
            .map(|(u, v)| f(u, v))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| {
                (a.min(x), b.max(x))
            })
    };
    let (ek_lo, ek_hi) = range(&|u, v| energy(&dispatch(&k, u, v)));
    let (el_lo, el_hi) = range(&|u, v| energy(&dispatch(&lz, u, v)));
    let third = 1.0 / 3.0;
    let k_fields = panels(n, |u, v| {
        let d = dispatch(&k, u, v);
        let feasible = decode_theta_k(&k, u, v).1 == Outcome::Feasible;
        let light = d.ic.m.iter().copied().fold(f64::INFINITY, f64::min);
        [
            state(&d, feasible),
            grey(energy(&d), ek_lo, ek_hi),
            grey(light, 0.0, third),
        ]
    });
    let root = (2.0 * lz.k).sqrt();
    let lz_fields = panels(n, |u, v| {
        let d = dispatch(&lz, u, v);
        let feasible = decode_theta_lz(&lz, u, v).1 == Outcome::Feasible;
        [
            state(&d, feasible),
            grey(lz_of(&d) / root, -1.0, 1.0),
            grey(energy(&d), el_lo, el_hi),
        ]
    });
    let mut coverage = Image::new(n, n, 0);
    let x = |l: f64| (l / (1.1 * root) + 1.0) / 2.0;
    for (u, v) in grid(n) {
        coverage.plot(x(lz_of(&dispatch(&lz, u, v))), u, 255);
    }
    for y in 0..n {
        for edge in [-1.0, 1.0] {
            coverage.plot(
                x(edge * root * (1.0 + 1.0 / n as f64)),
                y as f64 / (n - 1) as f64,
                128,
            );
        }
    }
    [
        ("theta_k_fields.png", k_fields),
        ("theta_lz_fields.png", lz_fields),
        ("theta_lz_coverage.png", coverage),
    ]
}

#[test]
fn burrau_acute_evidence() {
    let dir = dir();
    let values = measure();
    let images = renders();
    if std::env::var_os("PRIN_WRITE_BURRAU_ACUTE").is_some() {
        std::fs::create_dir_all(&dir).expect("the evidence directory");
        std::fs::write(dir.join(MEASUREMENT), measurement_text(&values))
            .expect("the measurement writes");
        for (name, image) in &images {
            image.write(&dir.join(name));
        }
    }
    let text = std::fs::read_to_string(dir.join(MEASUREMENT)).expect("the measurement reads");
    check_measurement(&text, &values);
    for (name, image) in &images {
        let file = std::fs::File::open(dir.join(name)).expect("the render reads");
        let info = png::Decoder::new(std::io::BufReader::new(file))
            .read_info()
            .expect("the render is a PNG")
            .info()
            .clone();
        assert_eq!(
            (info.width as usize, info.height as usize),
            (image.w, image.h),
            "{name}'s size"
        );
    }
    for (k, v, _) in &values {
        eprintln!("burrau_acute_evidence: {k} = {v:e}");
    }
}

negative_control!(
    burrau_acute_evidence,
    "a measurement file holding another θ_max must fail",
    expected = "`theta_max` holds 1e0, measured",
    check_measurement(
        "theta_max = 1e0\n",
        &[("theta_max", 0.7853981256484985, "")]
    )
);
