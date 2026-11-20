//! The invariant warp's calibration evidence (REQ-CHART-044; R-71, R-133; chart_reference §2.1): `K_max` and `γ_K`, the
//! constants register's proposed entries, measured against what the `(L_z, E)` and `(L_z, K)` charts show at them.
//! - `invariant_warp_evidence`: the measurement, `fixtures/gates/invariant_warp/measurement.toml`, is this test's,
//!   within a relative 10⁻⁶, and the renders beside it are the PNGs it draws. Regenerate both with
//!   `PRIN_WRITE_INVARIANT_WARP=1`. The coverage shares and the unchecked root's count, which a platform's `powf` or
//!   `sqrt` moves (3204 on macOS, 3192 on CI's Linux), are not compared as numbers: the file records the platform
//!   that wrote it, and both it and the measurement must show what each is evidence of (`check_claims`).
//!
//! **What it measures**, at the register's `K_max` and `γ_K`, through the kernel's own chart decode (natively; R-133:
//! decode-time fields, no integrated field):
//! - `|U|` over decoded configurations, the frozen part's potential, at the golden IC and its percentiles over
//!   `(z_α, z_β, z_μ₁, z_μ₂)` uniform on `[−1, 1]⁴` and `[−2, 2]⁴`, and the row `t = (|U|/K_max)^{1/γ_K}` where each
//!   reaches the escape energy `E = 0`: the chart spans `E ∈ [U, U + K_max]`;
//! - the rows below `K = 0.01·K_max` and `0.1·K_max` on a 512-row chart at `γ_K = 1, 2, 3`: the resolution near the apex,
//!   where `K` meets `K_min = L_z²/2I`;
//! - the outermost interior column's margin `(K* − K_min)/K* = 1 − (1 − 2/(n − 1))²`, at `n = 512`: the resolution at
//!   the parabola's sides, `γ_K`-free;
//! - the rows' spacing in `K` at the top, `γ_K·K_max/(n − 1)`, the price of crowding them toward the apex;
//! - the share of the feasible region `|L_z| ≤ √(2K)`, `K ≤ K_max`, cut into 512 × 512 cells of the `(L_z, K)` plane,
//!   that a 512 × 512 chart reaches, at `γ_K = 1, 2, 3`, overall and below `0.1·K_max`;
//! - the construction's infeasible pixels on a 512 × 512 grid with its edges, at f64 and at f32, at six frozen
//!   configurations: 0, and the count a warp with an unchecked rounded root would give at f32.
//!
//! **The renders** (8-bit grey PNGs): `coverage_lz_k.png`, the `(L_z, K)` plane, `L_z ∈ [−√(2K_max), √(2K_max)]`
//! across, `K ∈ [0, K_max]` up, each of a 256 × 256 chart's pixels plotted where its decode lands (its momenta's `L_z`,
//! its descriptor's `K₀`), the parabola `|L_z| = √(2K)` in grey; `coverage_lz_e.png`, the same on `(L_z, E)` at the
//! golden IC, `E = K₀ + V₀`, with `E = 0` in grey; `fields_st.png`, the chart's own square `(s, t)`, `t` up, three
//! panels: the decode's feasibility (white where `Feasible`), `K₀/K_max`, and `L_z` mapped from `[−√(2K_max),
//! √(2K_max)]` to black–white.

use std::path::{Path, PathBuf};

use kernel::chart::invariant::{self, Invariant, Targets};
use kernel::chart::{InvariantLE, InvariantLK};
use kernel::decode::invariant::{infeasible, Outcome};
use kernel::decode::{self as dec, Decoded};
use kernel::generated::links::Defaults;
use validation::decode_f32_eps::SplitMix;
use validation::{negative_control, GAMMA_K, K_MAX};

const DIR: &str = "fixtures/gates/invariant_warp";
const MEASUREMENT: &str = "measurement.toml";

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DIR)
}

/// The frozen configurations the infeasibility count runs at, `(z_α, z_β, z_μ₁, z_μ₂)`.
const FROZEN: [[f64; 4]; 6] = [
    [0.0, 0.0, 0.0, 0.0],
    [0.3, -0.5, 0.7, -0.4],
    [1.8, 0.4, -1.0, 1.2],
    [-1.5, 1.0, 0.2, 0.9],
    [-0.25, -1.75, 1.5, 1.5],
    [-1e-30, 0.6, -2.0, 0.1],
];

fn chart(z: [f64; 4], k_max: f64, gamma_k: f64) -> Invariant<f64> {
    Invariant {
        z_cfg: [z[0], z[1]],
        z_mass: [z[2], z[3]],
        k_max,
        gamma_k,
    }
}

fn narrow(c: &Invariant<f64>) -> Invariant<f32> {
    Invariant {
        z_cfg: c.z_cfg.map(|x| x as f32),
        z_mass: c.z_mass.map(|x| x as f32),
        k_max: c.k_max as f32,
        gamma_k: c.gamma_k as f32,
    }
}

/// `V₀`, the frozen configuration's potential `U`, at f64.
fn potential(z: [f64; 4]) -> f64 {
    invariant::decode::<Defaults, f64, _>(&InvariantLK(chart(z, 1.0, 2.0)), 0.5, 0.0)
        .0
        .descriptor
        .V_0
}

/// The `q`-quantile of sorted `v`.
fn quantile(v: &[f64], q: f64) -> f64 {
    v[((q * v.len() as f64).ceil() as usize).clamp(1, v.len()) - 1]
}

/// `|U|` at `n` configurations uniform on `[−h, h]⁴`, sorted.
fn potentials(seed: u64, h: f32, n: usize) -> Vec<f64> {
    let mut g = SplitMix::new(seed);
    let mut out: Vec<f64> = (0..n)
        .map(|_| -potential([0; 4].map(|_| f64::from(g.uniform(-h, h)))))
        .collect();
    out.sort_by(f64::total_cmp);
    out
}

/// The `(s, t)` grid with `n` points per side, its edges included.
fn grid(n: usize) -> impl Iterator<Item = (f64, f64)> {
    let at = move |i: usize| i as f64 / (n - 1) as f64;
    (0..n).flat_map(move |j| (0..n).map(move |i| (at(i), at(j))))
}

/// The rows of an `n`-row chart with `K < f·K_max` at `γ`.
fn rows_below(n: usize, gamma: f64, f: f64) -> f64 {
    (0..n)
        .filter(|&j| (j as f64 / (n - 1) as f64).powf(gamma) < f)
        .count() as f64
}

/// The share of the feasible region's cells (`cells` × `cells` over `L_z ∈ [−√2, √2]`, `K ∈ [0, 1]`, `K_max = 1`)
/// that an `n` × `n` chart at `γ` reaches: overall, and below `K = 0.1`.
fn coverage(n: usize, gamma: f64, cells: usize) -> (f64, f64) {
    let span = 2f64.sqrt();
    let cell = |lz: f64, k: f64| {
        let i = (((lz + span) / (2.0 * span)) * cells as f64).floor() as usize;
        let j = (k * cells as f64).floor() as usize;
        (i.min(cells - 1), j.min(cells - 1))
    };
    let mut hit = vec![false; cells * cells];
    for (s, t) in grid(n) {
        let w = invariant::warp(1.0, gamma, s, t);
        let (i, j) = cell(w.lz, w.k);
        hit[j * cells + i] = true;
    }
    // A cell is feasible where its centre is: |L_z| ≤ √(2K).
    let (mut all, mut low, mut got, mut got_low) = (0, 0, 0, 0);
    for j in 0..cells {
        for i in 0..cells {
            let lz = -span + (i as f64 + 0.5) / cells as f64 * 2.0 * span;
            let k = (j as f64 + 0.5) / cells as f64;
            if lz * lz <= 2.0 * k {
                all += 1;
                let h = hit[j * cells + i];
                got += usize::from(h);
                if k < 0.1 {
                    low += 1;
                    got_low += usize::from(h);
                }
            }
        }
    }
    (got as f64 / all as f64, got_low as f64 / low as f64)
}

/// §2.1's warp with an unchecked rounded root, `L_max = √(2K)`, at f32.
fn rounded_root(k_max: f32, gamma_k: f32, s: f32, t: f32) -> Targets<f32> {
    let k = k_max * t.powf(gamma_k);
    let l_max = (k + k).sqrt();
    Targets {
        k,
        l_max,
        lz: (s + s - 1.0) * l_max,
    }
}

/// The construction's infeasible pixels over the six frozen configurations' `n` × `n` grids: through the chart's decode
/// at f64 and at f32, and of an unchecked rounded root's targets at f32.
fn infeasible_counts(n: usize, k_max: f64, gamma_k: f64) -> [f64; 3] {
    let mut out = [0.0; 3];
    for z in FROZEN {
        let c = chart(z, k_max, gamma_k);
        let c32 = narrow(&c);
        for (s, t) in grid(n) {
            let (s32, t32) = (s as f32, t as f32);
            let d64 = invariant::decode::<Defaults, f64, _>(&InvariantLE(c), s, t).1;
            let d32 = invariant::decode::<Defaults, f32, _>(&InvariantLE(c32), s32, t32).1;
            let r = rounded_root(c32.k_max, c32.gamma_k, s32, t32);
            out[0] += f64::from(u8::from(d64 == Outcome::Infeasible));
            out[1] += f64::from(u8::from(d32 == Outcome::Infeasible));
            out[2] += f64::from(u8::from(infeasible(1.0, r.k, r.lz)));
        }
    }
    out
}

/// The measurement: `(key, value, what)`.
fn measure() -> Vec<(&'static str, f64, &'static str)> {
    let (k_max, gamma_k) = (K_MAX.number(), GAMMA_K.number());
    let escape = |u: f64| (u / k_max).powf(1.0 / gamma_k);
    let golden = -potential([0.0; 4]);
    let u1 = potentials(11, 1.0, 20_000);
    let u2 = potentials(12, 2.0, 20_000);
    let (c1, c1_low) = coverage(512, 1.0, 512);
    let (c2, c2_low) = coverage(512, 2.0, 512);
    let (c3, c3_low) = coverage(512, 3.0, 512);
    let [inf64, inf32, rounded] = infeasible_counts(512, k_max, gamma_k);
    let edge = 1.0 - (1.0 - 2.0 / 511.0f64).powi(2);
    vec![
        (
            "k_max",
            k_max,
            "the proposed K_max (constants register, provisional until M2)",
        ),
        (
            "gamma_k",
            gamma_k,
            "the proposed γ_K (constants register, provisional until M2)",
        ),
        ("u_golden", golden, "|U| at the golden IC, z = 0"),
        (
            "u_h1_p50",
            quantile(&u1, 0.5),
            "|U|, median, z uniform on [−1, 1]⁴ (20 000, seed 11)",
        ),
        (
            "u_h1_p90",
            quantile(&u1, 0.9),
            "|U|, 90th percentile, [−1, 1]⁴",
        ),
        (
            "u_h1_p99",
            quantile(&u1, 0.99),
            "|U|, 99th percentile, [−1, 1]⁴",
        ),
        (
            "u_h2_p50",
            quantile(&u2, 0.5),
            "|U|, median, z uniform on [−2, 2]⁴ (20 000, seed 12)",
        ),
        (
            "u_h2_p90",
            quantile(&u2, 0.9),
            "|U|, 90th percentile, [−2, 2]⁴",
        ),
        (
            "u_h2_p99",
            quantile(&u2, 0.99),
            "|U|, 99th percentile, [−2, 2]⁴",
        ),
        (
            "u_h1_above_k_max",
            u1.iter().filter(|&&u| u > k_max).count() as f64,
            "configurations of [−1, 1]⁴ whose escape energy lies above the chart",
        ),
        (
            "u_h2_above_k_max",
            u2.iter().filter(|&&u| u > k_max).count() as f64,
            "configurations of [−2, 2]⁴ whose escape energy lies above the chart",
        ),
        (
            "t_escape_golden",
            escape(golden),
            "the row t where E = 0 at the golden IC",
        ),
        (
            "t_escape_h1_p50",
            escape(quantile(&u1, 0.5)),
            "the row t where E = 0 at [−1, 1]⁴'s median |U|",
        ),
        (
            "t_escape_h1_p99",
            escape(quantile(&u1, 0.99)),
            "the row t where E = 0 at [−1, 1]⁴'s 99th percentile",
        ),
        (
            "t_escape_h2_p99",
            escape(quantile(&u2, 0.99)),
            "the row t where E = 0 at [−2, 2]⁴'s 99th percentile",
        ),
        (
            "rows_below_0p01_gamma1",
            rows_below(512, 1.0, 0.01),
            "rows of 512 with K < 0.01·K_max, γ_K = 1",
        ),
        (
            "rows_below_0p01_gamma2",
            rows_below(512, 2.0, 0.01),
            "rows of 512 with K < 0.01·K_max, γ_K = 2",
        ),
        (
            "rows_below_0p01_gamma3",
            rows_below(512, 3.0, 0.01),
            "rows of 512 with K < 0.01·K_max, γ_K = 3",
        ),
        (
            "rows_below_0p1_gamma1",
            rows_below(512, 1.0, 0.1),
            "rows of 512 with K < 0.1·K_max, γ_K = 1",
        ),
        (
            "rows_below_0p1_gamma2",
            rows_below(512, 2.0, 0.1),
            "rows of 512 with K < 0.1·K_max, γ_K = 2",
        ),
        (
            "rows_below_0p1_gamma3",
            rows_below(512, 3.0, 0.1),
            "rows of 512 with K < 0.1·K_max, γ_K = 3",
        ),
        (
            "dk_top_gamma1",
            k_max / 511.0,
            "K's row spacing at the top of 512 rows, γ_K = 1",
        ),
        ("dk_top_gamma2", 2.0 * k_max / 511.0, "the same, γ_K = 2"),
        ("dk_top_gamma3", 3.0 * k_max / 511.0, "the same, γ_K = 3"),
        (
            "edge_margin_512",
            edge,
            "(K* − K_min)/K* at the outermost interior column of 512, any γ_K",
        ),
        (
            "coverage_gamma1",
            c1,
            "share of the feasible region's 512² cells a 512² chart reaches, γ_K = 1",
        ),
        ("coverage_gamma2", c2, "the same, γ_K = 2"),
        ("coverage_gamma3", c3, "the same, γ_K = 3"),
        (
            "coverage_low_gamma1",
            c1_low,
            "the same below K = 0.1·K_max, γ_K = 1",
        ),
        (
            "coverage_low_gamma2",
            c2_low,
            "the same below K = 0.1·K_max, γ_K = 2",
        ),
        (
            "coverage_low_gamma3",
            c3_low,
            "the same below K = 0.1·K_max, γ_K = 3",
        ),
        (
            "infeasible_f64",
            inf64,
            "infeasible pixels, six configurations' 512² grids with edges, f64 decode",
        ),
        ("infeasible_f32", inf32, "the same, f32 decode"),
        (
            "infeasible_rounded_root_f32",
            rounded,
            "the same for an unchecked rounded √(2K) at f32: why L_max is stepped",
        ),
    ]
}

/// The measurement file's text for `values`.
fn measurement_text(values: &[(&str, f64, &str)]) -> String {
    let mut s = String::from(
        "# The invariant warp's calibration evidence (REQ-CHART-044; R-71, R-133): K_max and γ_K, proposed in the\n\
         # constants register, provisional until the M2 gate. Measured by crates/validation/tests/invariant_warp.rs,\n\
         # invariant_warp_evidence, through the kernel's chart decode natively; regenerate with\n\
         # PRIN_WRITE_INVARIANT_WARP=1, which also redraws the renders beside it. Compared at a relative 1e-6, except\n\
         # the coverage shares and the unchecked root's count, which a platform's powf or sqrt moves: as written on\n\
         # macOS, and checked only for what they evidence (the unchecked root infeasible somewhere; coverage falling\n\
         # with γ_K overall and rising below 0.1·K_max).\n",
    );
    for (k, v, what) in values {
        s.push_str(&format!("{k} = {v:e}  # {what}\n"));
    }
    s
}

/// The keys a platform's `powf` or `sqrt` moves, one ulp taking a pixel across a cell's edge or a rounded root across
/// the exact one (3204 on macOS against 3192 on CI's Linux): they are checked for what they evidence
/// ([`check_claims`]), not compared as numbers.
fn platform_dependent(key: &str) -> bool {
    key.starts_with("coverage_") || key == "infeasible_rounded_root_f32"
}

/// What the platform-dependent keys are evidence of, in `get`'s values from `source`: an unchecked rounded root is
/// infeasible somewhere (why `L_max` is stepped down); each coverage share is a share; a larger `γ_K` reaches less of
/// the feasible region overall and more of it below `0.1·K_max` than `γ_K = 1`.
fn check_claims(get: &dyn Fn(&str) -> f64, source: &str) {
    assert!(
        get("infeasible_rounded_root_f32") > 0.0,
        "{source}: an unchecked rounded root is never infeasible"
    );
    for k in [
        "coverage_gamma1",
        "coverage_gamma2",
        "coverage_gamma3",
        "coverage_low_gamma1",
        "coverage_low_gamma2",
        "coverage_low_gamma3",
    ] {
        let v = get(k);
        assert!(
            v > 0.0 && v <= 1.0,
            "{source}: `{k}` = {v:e} is not a share"
        );
    }
    assert!(
        get("coverage_gamma1") > get("coverage_gamma2")
            && get("coverage_gamma2") > get("coverage_gamma3"),
        "{source}: the coverage does not fall with γ_K"
    );
    assert!(
        get("coverage_low_gamma2") > get("coverage_low_gamma1")
            && get("coverage_low_gamma3") > get("coverage_low_gamma1"),
        "{source}: a larger γ_K does not reach more below 0.1·K_max"
    );
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

/// Each key of `text` holds its measured value within a relative 10⁻⁶, but the [`platform_dependent`] ones, which
/// both `text` and the measurement must hold to [`check_claims`].
fn check_measurement(text: &str, values: &[(&str, f64, &str)]) {
    for (k, v, _) in values {
        let held = held(text, k);
        assert!(
            platform_dependent(k) || (held - v).abs() <= 1e-6 * v.abs(),
            "{MEASUREMENT}: `{k}` holds {held:e}, measured {v:e}"
        );
    }
    if values.iter().any(|(k, _, _)| platform_dependent(k)) {
        check_claims(&|k| held(text, k), MEASUREMENT);
        let measured = |k: &str| {
            values
                .iter()
                .find(|(key, _, _)| *key == k)
                .map(|(_, v, _)| *v)
                .unwrap_or_else(|| panic!("no measured `{k}`"))
        };
        check_claims(&measured, "the measurement");
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

/// `L_z(p) = Σ (rᵢ × pᵢ)_z` of a decode.
fn lz_of(d: &Decoded<f32>) -> f64 {
    (0..3)
        .map(|i| f64::from(d.ic.r[i][0] * d.ic.p[i][1] - d.ic.r[i][1] * d.ic.p[i][0]))
        .sum()
}

/// The three renders, from the f32 decodes of a 256 × 256 chart at the golden IC under the register's warp.
fn renders() -> [(&'static str, Image); 3] {
    let (k_max, gamma_k) = (K_MAX.number(), GAMMA_K.number());
    let c = narrow(&chart([0.0; 4], k_max, gamma_k));
    let span = (2.0 * k_max).sqrt();
    let n = 256;
    let u = f64::from(
        invariant::decode::<Defaults, f32, _>(&InvariantLE(c), 0.5, 0.0)
            .0
            .descriptor
            .V_0,
    );
    let (mut lk, mut le) = (Image::new(512, 512, 0), Image::new(512, 512, 0));
    let mut fields = Image::new(3 * n + 16, n, 0);
    for y in 0..512 {
        let k = y as f64 / 511.0 * k_max;
        let edge = (2.0 * k).sqrt();
        for x in [-edge, edge] {
            lk.plot((x + span) / (2.0 * span), k / k_max, 128);
            le.plot((x + span) / (2.0 * span), k / k_max, 128);
        }
    }
    for x in 0..512 {
        le.plot(x as f64 / 511.0, -u / k_max, 128);
    }
    for (j, i, (s, t)) in grid(n).enumerate().map(|(q, p)| (q / n, q % n, p)) {
        let (d, outcome) =
            invariant::decode::<Defaults, f32, _>(&InvariantLE(c), s as f32, t as f32);
        let k0 = f64::from(d.descriptor.K_0);
        let e0 = f64::from(dec::descriptor::energy(&d.descriptor));
        let lz = lz_of(&d);
        let x = (lz + span) / (2.0 * span);
        lk.plot(x, k0 / k_max, 255);
        le.plot(x, (e0 - u) / k_max, 255);
        let row = n - 1 - j;
        let at = |panel: usize| row * fields.w + panel * (n + 8) + i;
        let ok = outcome == Outcome::Feasible;
        fields.px[at(0)] = if ok { 255 } else { 0 };
        fields.px[at(1)] = (k0 / k_max * 255.0).round().clamp(0.0, 255.0) as u8;
        fields.px[at(2)] = (x * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    [
        ("coverage_lz_k.png", lk),
        ("coverage_lz_e.png", le),
        ("fields_st.png", fields),
    ]
}

#[test]
fn invariant_warp_evidence() {
    let dir = dir();
    let values = measure();
    let images = renders();
    if std::env::var_os("PRIN_WRITE_INVARIANT_WARP").is_some() {
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
        eprintln!("invariant_warp_evidence: {k} = {v:e}");
    }
}

negative_control!(
    invariant_warp_evidence,
    "a measurement file holding another K_max must fail",
    expected = "`k_max` holds 2e0, measured",
    check_measurement("k_max = 2e0\n", &[("k_max", 1.0, "")])
);
