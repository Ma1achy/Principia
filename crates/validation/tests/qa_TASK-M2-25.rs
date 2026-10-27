//! QA tests for TASK-M2-25, written from the requirements it closes, not from the implementation: the fragment decode
//! and encode generated from the one source (R-116), the DECODE preset and the agreement preset, on the GPU, through the
//! checked-in snippets alone. The test that needs the built kernel's decode stage is `qa_TASK-M2-25_gpu`.
//!
//! - REQ-COL-057 (`qa_m225_chart_z_is_the_definition`): `ctx.chart.z` as colour_composition §3's table defines it, row
//!   by row: on the latent slice `z₀ + (2s − 1)q₁ + (2t − 1)q₂` at `ctx.chart.slice_uv = (s, t)` and `z_latent` true;
//!   on the shape sphere config `0`, momentum and mass the chart's held `z_mom`, `z_mass`; on `(L_z, E)` and `(L_z, K)`
//!   the frozen `z_cfg`, `z_mass` and momentum `0`; on the ternary mass, Euclid and `θ × K` charts `0`; `z_latent`
//!   false off the latent chart. Control: `q₁` and `q₂` exchanged in the expectation.
//! - REQ-RENDER-027, REQ-TOOL-029, REQ-COL-057 (`qa_m225_decode_preset_panels`): the DECODE preset's colour view on
//!   every M2 chart, read as the preset documents its panels: the `z` panel is the invalid hatch exactly where the
//!   chart's `Φ` produces no latent (§3, "§6's DECODE does") and not the hatch on the latent chart; the mass panel is a
//!   point of the simplex, `Σm = 1`, each mass the f64 `decodeOnly()`'s at the pixel's chart point within REQ-DEC-043's
//!   factor; the positions panel, its `squash` inverted, has `CoM = 0` and `I = 1` with the panel's masses. Control:
//!   the latent chart's `z` panel expected hatched.
//! - REQ-TOOL-029, R-437 (`qa_m225_decode_off_lattice_and_pole`): every chart's fragment decode at fuzzed chart points
//!   (depth-20 cells, a fixed seed) and at the square's centre `(½, ½)` (the shape sphere's `n₀` pole, where its
//!   inverse's `atan2(q, p)` has both operands 0): each of the 27 values within REQ-DEC-043's factor of the f64
//!   `decodeOnly()`, the tag equal, and `Σm = 1`, `CoM = 0`, `I = 1`; at the centre, where R-437 asks the GPU to agree
//!   with the CPU, the CPU's f32 decode of the same source (the pole is a near-coincident pair on the shape sphere at
//!   the harness's parameters, outside REQ-DEC-043's measured domain). Control: the reference taken at the mirrored
//!   point `(1 − u, v)`, and at the centre the CPU's decode at `(¼, ½)`.
//! - REQ-COL-006, R-436 (`qa_m225_agreement_tests_each_operand_on_its_bits`): the agreement preset's own WGSL
//!   comparison, on the GPU, given each of its four operands (`K`, `V` of the fragment decode, `K₀`, `V₀` of the
//!   payload) non-finite in turn by its bits (`+∞`, `−∞`, a quiet NaN, a negative NaN, a signalling NaN): the sample
//!   is not compared and draws the invalid hatch; the largest finite, a subnormal and `−0` in each place are compared.
//!   Control: the preset's WGSL with `V₀`'s bit test removed.
//! - REQ-COL-006 (`qa_m225_fragment_encode_of_fragment_decode`): the fragment recomputes `encode(decode(z))` from the
//!   generated snippets alone, on the latent chart's interior: the fragment encode's `z`, decoded at f64, is the
//!   decoded state of the CPU's f32 encode of the same `(m, r, p)` in physical units (inverse-encode Part 4: never a
//!   `z` residual), within REQ-DEC-043's factor; `α`, `β` and `λ` within it too and the notices word equal; and both
//!   recover `ctx.chart.z`'s system. Control: the fragment's `z` read one component off.
//! - REQ-SYS-085 (`qa_m225_read_side_shape_is_the_translation`): `read_side.wgsl`'s `shape()` is `shape.wgsl`'s
//!   translated function, byte for byte, and no `.wgsl` file under `crates/`, `xtask/` or `fixtures/` nor any string
//!   in `crates/ledger/src/gen/read.rs` names a dd_decoder §3 shape formula (`mu_rho`, `mu_lambda`, `m01`). Controls: a
//!   `mu_rho` formula planted in a `.wgsl` text and in `read.rs`'s text are each found.
//! - REQ-COL-006, R-436 (`qa_m225_generated_snippets_have_no_infinite_literal`): the generated decode and encode
//!   write no infinite literal (no float literal past f32's range, no `inf`) and no `1/0` (no division by a literal
//!   zero); the decode's `−∞` is its bits through a bitcast. Controls: `-1f / 0f` and `-1e39f` planted in the decode.
//!
//! **Tolerance:** REQ-DEC-043's calibrated f32 decode factor, `fixtures/gates/decode-f32-eps/gate.json`'s threshold
//! (R-133; provisional until the M2 gate, R-71), in f32 `eps` scaled by `max(|x|, 1)`; on a quantity formed from the
//! decoded values (a sum, a colour's inverse) it is scaled by that quantity's own terms, stated at each use.
//!
//! **Domain** (pitfalls §10): the decode and encode alone, their inputs held fixed; each chart at the harness's fixed
//! parameters (`validation::decode_preset::uniforms`), inside its square.

use std::path::{Path, PathBuf};

use render::bind::ViewOutput;
use render::presets::{self, agreement, decode, DecodeChart};
use validation::decode_preset::{self as dp, Scene, State};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// REQ-DEC-043's factor, in f32 eps.
fn factor() -> f64 {
    validation::gate::config(&root(), "decode-f32-eps")
        .expect("decode-f32-eps's gate.json")
        .threshold
        .value
}

const EPS: f64 = f32::EPSILON as f64;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// `|a − b|` within the factor of `max(|b|, 1)`.
fn within(a: f64, b: f64, f: f64) -> bool {
    (a - b).abs() <= f * EPS * b.abs().max(1.0)
}

fn f32w(w: u32) -> f64 {
    f64::from(f32::from_bits(w))
}

// ── REQ-COL-057: ctx.chart.z per colour_composition §3's table ──────────────────────────────────────────────────────

/// The latent chart's frame, as the harness fixes it (`validation::decode_preset::uniforms`).
const Z0: [f32; 8] = [0.3, -0.2, 0.1, -0.4, 0.25, 0.05, 0.2, -0.35];
const Q1: [f32; 8] = [0.5, 0.2, -0.3, 0.1, 0.0, 0.2, 0.4, -0.1];
const Q2: [f32; 8] = [-0.2, 0.5, 0.2, 0.3, -0.25, 0.1, -0.3, 0.3];
/// The shape sphere's held momentum and mass controls.
const SHAPE_Z_MOM: [f32; 4] = [0.2, -0.1, 0.3, 0.15];
const SHAPE_Z_MASS: [f32; 2] = [0.3, -0.2];
/// The invariant charts' frozen config and mass.
const INV_Z_CFG: [f32; 2] = [0.4, -0.3];
const INV_Z_MASS: [f32; 2] = [0.2, -0.5];

/// The table's `z` on `chart` at `(s, t)`, and its `z_latent`; `swap` exchanges `q₁` and `q₂` (the control).
fn defined_z(chart: DecodeChart, st: [f64; 2], swap: bool) -> ([f64; 8], bool) {
    let mut z = [0.0f64; 8];
    match chart {
        DecodeChart::Latent => {
            let (a, b) = if swap { (Q2, Q1) } else { (Q1, Q2) };
            for k in 0..8 {
                z[k] = f64::from(Z0[k])
                    + (2.0 * st[0] - 1.0) * f64::from(a[k])
                    + (2.0 * st[1] - 1.0) * f64::from(b[k]);
            }
            return (z, true);
        }
        DecodeChart::ShapeSphere => {
            for k in 0..4 {
                z[2 + k] = f64::from(SHAPE_Z_MOM[k]);
            }
            z[6] = f64::from(SHAPE_Z_MASS[0]);
            z[7] = f64::from(SHAPE_Z_MASS[1]);
        }
        DecodeChart::InvariantLE | DecodeChart::InvariantLK => {
            z[0] = f64::from(INV_Z_CFG[0]);
            z[1] = f64::from(INV_Z_CFG[1]);
            z[6] = f64::from(INV_Z_MASS[0]);
            z[7] = f64::from(INV_Z_MASS[1]);
        }
        DecodeChart::MassSimplex | DecodeChart::EuclidPlane | DecodeChart::BurrauThetaK => {}
    }
    (z, false)
}

fn check_chart_z(swap: bool) {
    use kernel::chart::Latent;
    use kernel::stage::decode::UniformChart;
    // The constants above are the harness's: its latent uniform is the frame built from them.
    let frame = Latent {
        z0: Z0,
        q1: Q1,
        q2: Q2,
    }
    .uniform();
    assert_eq!(
        dp::uniforms(DecodeChart::Latent).chart,
        frame,
        "the harness's latent frame is not this test's"
    );
    let gpu = gpu();
    let f = factor();
    for chart in DecodeChart::ALL {
        let scene = Scene::lattice(chart, [5, 3], 12).unwrap();
        let probe = scene.probe(gpu.device(), gpu.queue()).unwrap();
        for w in &probe {
            let st = [f32w(w[37]), f32w(w[38])];
            let (want, latent) = defined_z(chart, st, swap);
            assert_eq!(w[24] != 0, latent, "{chart:?}: z_latent is not the table's");
            for k in 0..8 {
                let got = f32w(w[16 + k]);
                // The latent's three-term sum: the factor on its terms' scale.
                let scale = if latent {
                    (f64::from(Z0[k].abs()) + f64::from(Q1[k].abs()) + f64::from(Q2[k].abs()))
                        .max(1.0)
                } else {
                    1.0
                };
                assert!(
                    (got - want[k]).abs() <= f * EPS * scale,
                    "{chart:?} at {st:?}: ctx.chart.z[{k}] is {got}, colour_composition §3 defines {}",
                    want[k]
                );
            }
        }
    }
}

#[test]
fn qa_m225_chart_z_is_the_definition() {
    check_chart_z(false);
}

negative_control!(
    qa_m225_chart_z_is_the_definition,
    "the latent slice with q1 and q2 exchanged must not match",
    expected = "colour_composition §3 defines",
    check_chart_z(true)
);

// ── REQ-RENDER-027, REQ-TOOL-029: the DECODE preset's panels ────────────────────────────────────────────────────────

/// `squash`'s inverse: `d = 2c − 1 = x/(1 + |x|)`, so `x = d/(1 − |d|)`.
fn unsquash(c: f64) -> f64 {
    let d = 2.0 * c - 1.0;
    d / (1.0 - d.abs())
}

fn colour_at(image: &render::headless::Image, x: u32, y: u32) -> [f64; 3] {
    let w = image.words(x, y);
    [f32w(w[0]), f32w(w[1]), f32w(w[2])]
}

/// The DECODE preset's panels on every chart; `latent_hatched` expects the latent chart's `z` panel hatched too.
fn check_panels(latent_hatched: bool) {
    let gpu = gpu();
    let f = factor();
    let preset = decode::preset();
    for chart in DecodeChart::ALL {
        let scene = dp::golden(chart).unwrap();
        let image = scene.draw(gpu.device(), gpu.queue(), &preset).unwrap();
        let (width, height) = scene.size();
        let latent = chart == DecodeChart::Latent;
        let (mut z_px, mut m_px, mut r_px) = (0, 0, 0);
        for y in 0..height {
            for x in 0..width {
                let t = scene.tile_uv(x, y);
                let c = colour_at(&image, x, y);
                let hatch =
                    render::present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]);
                let is_hatch = (0..3).all(|k| (c[k] - hatch[k]).abs() <= 1e-5);
                let uv = scene.pixel_point(x, y);
                let r = dp::reference(chart, &scene.uniforms, uv).unwrap();
                if t[1] >= 2.0 / 3.0 {
                    z_px += 1;
                    let want_hatch = !latent || latent_hatched;
                    assert_eq!(
                        is_hatch, want_hatch,
                        "{chart:?} pixel ({x}, {y}): the z panel's hatch is {is_hatch}, z_latent is {latent}"
                    );
                } else if t[1] >= 1.0 / 3.0 {
                    m_px += 1;
                    let sum: f64 = c.iter().sum();
                    assert!(
                        (sum - 1.0).abs() <= 3.0 * f * EPS,
                        "{chart:?} pixel ({x}, {y}): the mass panel {c:?} is no point of the simplex"
                    );
                    for (k, (shown, want)) in c.iter().zip(r.out.ic.m).enumerate() {
                        assert!(
                            within(*shown, want, f),
                            "{chart:?} pixel ({x}, {y}): m{k} {shown} is not decodeOnly's {want}"
                        );
                    }
                } else {
                    r_px += 1;
                    // The positions panel shows one axis of the three bodies; the masses are the f64 decode's (the
                    // mass panel holds them to it above). CoM and I are checked on the axis pair below.
                    let axis = usize::from(t[0] >= 0.5);
                    let pos = c.map(unsquash);
                    for (b, (shown, body)) in pos.iter().zip(r.out.ic.r).enumerate() {
                        // The inverse amplifies the colour's rounding by (1 + |x|)²: the factor on that.
                        let tol = f * EPS * (1.0 + shown.abs()).powi(2);
                        let want = body[axis];
                        assert!(
                            (shown - want).abs() <= tol,
                            "{chart:?} pixel ({x}, {y}): body {b}'s position on axis {axis}, {shown}, is not decodeOnly's {want}"
                        );
                    }
                    let m = r.out.ic.m;
                    let com: f64 = (0..3).map(|b| m[b] * pos[b]).sum();
                    let scale: f64 = (0..3).map(|b| m[b] * (1.0 + pos[b].abs()).powi(2)).sum();
                    assert!(
                        com.abs() <= f * EPS * scale,
                        "{chart:?} pixel ({x}, {y}): the shown positions' CoM on axis {axis} is {com}"
                    );
                }
            }
        }
        assert!(
            z_px > 0 && m_px > 0 && r_px > 0,
            "{chart:?}: a panel has no pixel ({z_px}, {m_px}, {r_px})"
        );
        // I = 1 over the tile: each quad's positions panel, both axes, at its own pixels' decode.
        let grid = scene.context.grid;
        for q in 0..grid.quad_count() {
            let mut axes: [Option<([f64; 3], [f64; 3])>; 2] = [None, None];
            for y in 0..height {
                for x in 0..width {
                    let t = scene.tile_uv(x, y);
                    if grid.cell(x, y).quad != q || t[1] >= 1.0 / 3.0 {
                        continue;
                    }
                    let axis = usize::from(t[0] >= 0.5);
                    if axes[axis].is_none() {
                        let r =
                            dp::reference(chart, &scene.uniforms, scene.pixel_point(x, y)).unwrap();
                        axes[axis] = Some((colour_at(&image, x, y).map(unsquash), r.out.ic.m));
                    }
                }
            }
            let (Some((xs, m)), Some((ys, _))) = (axes[0], axes[1]) else {
                panic!("{chart:?} quad {q}: the positions panel lacks an axis");
            };
            let inertia: f64 = (0..3).map(|b| m[b] * (xs[b] * xs[b] + ys[b] * ys[b])).sum();
            let scale: f64 = (0..3)
                .map(|b| m[b] * ((1.0 + xs[b].abs()).powi(3) + (1.0 + ys[b].abs()).powi(3)))
                .sum();
            assert!(
                (inertia - 1.0).abs() <= 2.0 * f * EPS * scale,
                "{chart:?} quad {q}: the shown positions' I is {inertia}, not 1"
            );
        }
    }
}

#[test]
fn qa_m225_decode_preset_panels() {
    check_panels(false);
}

negative_control!(
    qa_m225_decode_preset_panels,
    "the latent chart's z panel held to the hatch must fail",
    expected = "the z panel's hatch is false",
    check_panels(true)
);

// ── REQ-TOOL-029, R-437: off the lattice and at the centre pole ─────────────────────────────────────────────────────

/// A fixed-seed generator, so the points do not hang on the implementation's.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// One probe word set against its f64 reference; `mirror` takes the reference at `(1 − u, v)`.
fn check_probe(chart: DecodeChart, scene: &Scene, w: &[u32], mirror: bool, f: f64) {
    let uv = [f32::from_bits(w[37]), f32::from_bits(w[38])];
    let at = if mirror { [1.0 - uv[0], uv[1]] } else { uv };
    let r = dp::reference(chart, &scene.uniforms, at).unwrap();
    assert_eq!(
        w[15],
        dp::tag(&r.out),
        "{chart:?} at {uv:?}: the tag differs from decodeOnly's"
    );
    let cpu = dp::values(&r.out);
    let frag: Vec<f64> = (0..15).chain(25..37).map(|k| f32w(w[k])).collect();
    for k in 0..27 {
        let e = validation::decode_f32_eps::error(frag[k], cpu[k]);
        assert!(
            e <= f,
            "{chart:?} at {uv:?}: {} is {} eps from decodeOnly, over REQ-DEC-043's {f}",
            validation::decode_f32_eps::QUANTITIES[k],
            e
        );
    }
    let m = [frag[0], frag[1], frag[2]];
    let rr = [0, 1, 2].map(|b| [frag[3 + 2 * b], frag[4 + 2 * b]]);
    assert!(
        (m.iter().sum::<f64>() - 1.0).abs() <= 3.0 * f * EPS,
        "{chart:?} at {uv:?}: Σm ≠ 1"
    );
    // Each axis's mass-weighted coordinates, mᵢ·rᵢ.
    let weighted = [0, 1].map(|a| [0, 1, 2].map(|b| m[b] * rr[b][a]));
    for (a, w) in weighted.iter().enumerate() {
        let com: f64 = w.iter().sum();
        let scale: f64 = w.iter().map(|x| x.abs()).sum::<f64>().max(1.0);
        assert!(
            com.abs() <= f * EPS * scale,
            "{chart:?} at {uv:?}: CoM_{a} = {com}"
        );
    }
    let inertia: f64 = (0..3)
        .map(|b| m[b] * (rr[b][0] * rr[b][0] + rr[b][1] * rr[b][1]))
        .sum();
    assert!(
        (inertia - 1.0).abs() <= 3.0 * f * EPS,
        "{chart:?} at {uv:?}: I = {inertia}"
    );
}

fn check_off_lattice(mirror: bool) {
    let gpu = gpu();
    let f = factor();
    let mut rng = Rng(0x4D32_2500_5EED);
    const DEPTH: u32 = 20;
    for chart in DecodeChart::ALL {
        let cells: Vec<[u64; 2]> = (0..36)
            .map(|_| [rng.next() % (1 << DEPTH), rng.next() % (1 << DEPTH)])
            .collect();
        let scene = Scene::new(chart, [6, 6], 1, DEPTH, &cells).unwrap();
        for w in scene.probe(gpu.device(), gpu.queue()).unwrap() {
            check_probe(chart, &scene, &w, mirror, f);
        }
        // The square's centre, the depth-0 cell's: exactly (½, ½), the shape sphere's n₀ pole, where both operands of
        // its inverse's atan2 are 0. R-437 asks the GPU to agree with the CPU there: the CPU's f32 decode of the same
        // source, since at these parameters n₀ is a near-coincident pair (ρ̃ ~ 5·10⁻⁸), where V ∝ 1/ρ and no f32 decode
        // can meet the f64 one within REQ-DEC-043's factor (whose domain, the golden IC and fuzzed interiors, excludes
        // it). Both are held to the factor of the f32 CPU value.
        let centre = Scene::new(chart, [1, 1], 1, 0, &[[0, 0]]).unwrap();
        let w = centre.probe(gpu.device(), gpu.queue()).unwrap()[0];
        assert_eq!([f32::from_bits(w[37]), f32::from_bits(w[38])], [0.5, 0.5]);
        let at = if mirror { [0.25, 0.5] } else { [0.5, 0.5] };
        let (cpu, tag) = cpu32(chart, &centre.uniforms, at);
        assert_eq!(
            w[15], tag,
            "{chart:?} at the centre: the tag differs from the CPU's f32 decode's"
        );
        for (k, x) in (0..15).chain(25..37).enumerate() {
            let e = validation::decode_f32_eps::error(f32w(w[x]), cpu[k]);
            assert!(
                e <= f,
                "{chart:?} at the centre: {} is {e} eps from the CPU's f32 decode, over REQ-DEC-043's {f}",
                validation::decode_f32_eps::QUANTITIES[k]
            );
        }
    }
}

/// The CPU's f32 decode of `chart` at `uv` (`kernel::stage::decode::sample_with`, toward the uniform's energy target,
/// as the decode stage runs it): the 27 values in REQ-DEC-043's order, widened, and the tag word.
fn cpu32(
    chart: DecodeChart,
    u: &kernel::stage::decode::DecodeUniforms,
    uv: [f32; 2],
) -> (Vec<f64>, u32) {
    use kernel::chart::*;
    fn of<C: kernel::stage::decode::UniformChart>(
        u: &kernel::stage::decode::DecodeUniforms,
        uv: [f32; 2],
    ) -> (Vec<f64>, u32)
    where
        f32: From<C::Param>,
    {
        use kernel::generated::links::Defaults;
        use kernel::stage::decode::{sample_with, tag, target};
        let chart = C::from_uniform(&u.chart);
        let o = sample_with::<C, Defaults, f32>(&chart, uv[0], uv[1], target(u.energy));
        let ic = o.ic;
        let d = o.descriptor;
        let mut v: Vec<f32> = ic.m.to_vec();
        for b in 0..3 {
            v.extend(ic.r[b]);
        }
        for b in 0..3 {
            v.extend(ic.p[b]);
        }
        v.extend([
            d.m0,
            d.m1,
            d.m2,
            d.q_mass,
            d.rho_mag,
            d.lambda_mag,
            d.rho_ratio,
            d.rho_angle,
            d.K_0,
            d.V_0,
            d.virial_ratio,
            d.r_min_pair_0,
        ]);
        (v.into_iter().map(f64::from).collect(), tag(o.degenerate))
    }
    match chart {
        DecodeChart::Latent => of::<Latent<f32>>(u, uv),
        DecodeChart::ShapeSphere => of::<ShapeSphere<f32>>(u, uv),
        DecodeChart::InvariantLE => of::<InvariantLE<f32>>(u, uv),
        DecodeChart::InvariantLK => of::<InvariantLK<f32>>(u, uv),
        DecodeChart::MassSimplex => of::<MassSimplex<f32>>(u, uv),
        DecodeChart::EuclidPlane => of::<EuclidPlane<f32>>(u, uv),
        DecodeChart::BurrauThetaK => of::<BurrauThetaK<f32>>(u, uv),
    }
}

#[test]
fn qa_m225_decode_off_lattice_and_pole() {
    check_off_lattice(false);
}

negative_control!(
    qa_m225_decode_off_lattice_and_pole,
    "a reference at the mirrored point must fail",
    expected = "decodeOnly",
    check_off_lattice(true)
);

// ── REQ-COL-006, R-436: each operand tested on its bits ─────────────────────────────────────────────────────────────

/// Non-finite bit patterns: `+∞`, `−∞`, a quiet NaN, a negative NaN, a signalling NaN.
const NON_FINITE: [u32; 5] = [
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_0000,
    0xffc0_0001,
    0x7f80_0001,
];
/// Finite edges that must be compared: the largest finite, the smallest subnormal, `−0`, the most negative finite.
const FINITE_EDGES: [u32; 4] = [0x7f7f_ffff, 0x0000_0001, 0x8000_0000, 0xff7f_ffff];
/// The finite operands the other places hold: `K = 0.5`, `V = −1.5`, `K₀ = 0.5`, `V₀ = −1.5`.
const BASE: [u32; 4] = [0x3f00_0000, 0xbfc0_0000, 0x3f00_0000, 0xbfc0_0000];

/// The cases, one per pixel: the four operands' bits and whether the sample must be compared.
fn operand_cases() -> Vec<([u32; 4], bool)> {
    let mut out = vec![(BASE, true)];
    for slot in 0..4 {
        for bits in NON_FINITE {
            let mut ops = BASE;
            ops[slot] = bits;
            out.push((ops, false));
        }
        for bits in FINITE_EDGES {
            let mut ops = BASE;
            ops[slot] = bits;
            out.push((ops, true));
        }
    }
    out
}

/// The agreement preset's WGSL up to its view, so a test's view calls its own `agreement` and `agreement_colour`;
/// `alter` rewrites it (the control).
fn agreement_prelude(alter: fn(String) -> String) -> String {
    let words = agreement::words();
    let text = words.view();
    let at = text.rfind("fn view(").expect("the words preset has a view");
    let prelude = alter(text[..at].to_owned());
    assert!(
        prelude.contains("fn agreement("),
        "the preset has no agreement()"
    );
    prelude
}

/// A view returning, per pixel (its column the case), `agreement(...)`'s valid word, or its colour.
fn operand_view(cases: &[([u32; 4], bool)], colour: bool) -> String {
    let mut arms = String::new();
    for (i, (ops, _)) in cases.iter().enumerate() {
        arms.push_str(&format!(
            "        case {i}u: {{ o = vec4<u32>({}u, {}u, {}u, {}u); }}\n",
            ops[0], ops[1], ops[2], ops[3]
        ));
    }
    let tail = if colour {
        "fn view(rc: RenderContext, l: Lanes) -> vec4<f32> {\n    let o = ops(u32(l.screen.pixel.x));\n    \
         let frag_xy = vec2<f32>(l.screen.pixel) + vec2<f32>(0.5);\n    \
         return vec4<f32>(agreement_colour(agreement(o.x, o.y, o.z, o.w), frag_xy), 1.0);\n}\n"
    } else {
        "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {\n    let o = ops(u32(l.screen.pixel.x));\n    \
         let a = agreement(o.x, o.y, o.z, o.w);\n    return vec4<u32>(select(0u, 1u, a.valid), 0u, 0u, 0u);\n}\n"
    };
    format!(
        "fn ops(i: u32) -> vec4<u32> {{\n    var o = vec4<u32>(0u);\n    switch (i) {{\n{arms}        default: {{}}\n    }}\n    \
         return o;\n}}\n{tail}"
    )
}

fn check_operands(alter: fn(String) -> String) {
    let gpu = gpu();
    let cases = operand_cases();
    let n = cases.len() as u32;
    let scene = Scene::lattice(DecodeChart::Latent, [n, 1], 8).unwrap();
    assert_eq!(scene.size(), (n, 1));
    let prelude = agreement_prelude(alter);
    let words = scene
        .draw_view(
            gpu.device(),
            gpu.queue(),
            &format!("{prelude}{}", operand_view(&cases, false)),
            ViewOutput::Words,
        )
        .unwrap();
    let colours = scene
        .draw_view(
            gpu.device(),
            gpu.queue(),
            &format!("{prelude}{}", operand_view(&cases, true)),
            ViewOutput::Colour,
        )
        .unwrap();
    let names = ["K", "V", "K₀", "V₀"];
    for (x, (ops, compared)) in (0u32..).zip(&cases) {
        let valid = words.words(x, 0)[0] != 0;
        let which = (0..4)
            .find(|&k| ops[k] != BASE[k])
            .map_or("none", |k| names[k]);
        assert_eq!(
            valid, *compared,
            "operands {ops:#010x?} ({which} altered): compared is {valid}, the bits say {compared}"
        );
        if !compared {
            let c = colour_at(&colours, x, 0);
            let hatch = render::present::debug_invalid([f64::from(x) + 0.5, 0.5]);
            assert!(
                (0..3).all(|k| (c[k] - hatch[k]).abs() <= 1e-5),
                "operands {ops:#010x?} ({which} non-finite): drawn {c:?}, not the invalid hatch {hatch:?}"
            );
        }
    }
}

#[test]
fn qa_m225_agreement_tests_each_operand_on_its_bits() {
    check_operands(|s| s);
}

negative_control!(
    qa_m225_agreement_tests_each_operand_on_its_bits,
    "the preset's WGSL without V₀'s bit test must be found",
    expected = "the bits say false",
    check_operands(|s| {
        let cut = s.replace(" || preset_non_finite(v_0)", "");
        assert_ne!(cut, s, "the control's edit did not apply");
        cut
    })
);

// ── REQ-COL-006: encode(decode(z)) in the fragment ──────────────────────────────────────────────────────────────────

/// A view: `frag_encode(frag_decode(slice_uv))`, its words group `group` of `(z₀…z₇, α, β, λ, notices)`.
fn roundtrip_view(group: usize) -> String {
    let pick = [
        "bitcast<u32>(e.z[0])",
        "bitcast<u32>(e.z[1])",
        "bitcast<u32>(e.z[2])",
        "bitcast<u32>(e.z[3])",
        "bitcast<u32>(e.z[4])",
        "bitcast<u32>(e.z[5])",
        "bitcast<u32>(e.z[6])",
        "bitcast<u32>(e.z[7])",
        "bitcast<u32>(e.alpha)",
        "bitcast<u32>(e.beta)",
        "bitcast<u32>(e.lambda)",
        "e.notices",
    ];
    let g = |k: usize| pick[4 * group + k];
    format!(
        "{}\nfn view(rc: RenderContext, l: Lanes) -> vec4<u32> {{\n    let d = frag_decode(l.chart.slice_uv);\n    \
         let e = frag_encode(d.m, d.r, d.p);\n    return vec4<u32>({}, {}, {}, {});\n}}\n",
        presets::ENCODE_WGSL,
        g(0),
        g(1),
        g(2),
        g(3)
    )
}

/// The canonical state of `z` at f64.
fn decoded(z: [f64; 8]) -> kernel::decode::Ic<f64> {
    use kernel::generated::links::Defaults;
    kernel::decode::decode::<Defaults, f64>(&kernel::decode::Latent { z }).ic
}

fn ic_flat(ic: &kernel::decode::Ic<f64>) -> Vec<f64> {
    let mut v = ic.m.to_vec();
    for b in 0..3 {
        v.extend(ic.r[b]);
    }
    for b in 0..3 {
        v.extend(ic.p[b]);
    }
    v
}

fn check_roundtrip(skew: bool) {
    let gpu = gpu();
    let f = factor();
    let scene = Scene::lattice(DecodeChart::Latent, [6, 6], 10).unwrap();
    let probe = scene.probe(gpu.device(), gpu.queue()).unwrap();
    let grid = scene.context.grid;
    let (width, height) = scene.size();
    let mut enc = vec![[0u32; 12]; probe.len()];
    for group in 0..3 {
        let image = scene
            .draw_view(
                gpu.device(),
                gpu.queue(),
                &roundtrip_view(group),
                ViewOutput::Words,
            )
            .unwrap();
        for y in 0..height {
            for x in 0..width {
                let q = grid.cell(x, y).quad as usize;
                for (k, w) in image.words(x, y).into_iter().enumerate() {
                    enc[q][4 * group + k] = w;
                }
            }
        }
    }
    for (w, e) in probe.iter().zip(&enc) {
        let uv = [f32w(w[37]), f32w(w[38])];
        let state = State {
            m: [0, 1, 2].map(|k| f32::from_bits(w[k])),
            r: [0, 1, 2].map(|b| [f32::from_bits(w[3 + 2 * b]), f32::from_bits(w[4 + 2 * b])]),
            p: [0, 1, 2].map(|b| [f32::from_bits(w[9 + 2 * b]), f32::from_bits(w[10 + 2 * b])]),
        };
        let cpu = dp::encode32(&state);
        for (k, what) in [(8, "α"), (9, "β"), (10, "λ")] {
            assert!(
                within(f32w(e[k]), f32w(cpu[k]), f),
                "at {uv:?}: the fragment encode's {what} {} is not the CPU's {}",
                f32w(e[k]),
                f32w(cpu[k])
            );
        }
        assert_eq!(
            e[11], cpu[11],
            "at {uv:?}: the fragment encode's notices differ from the CPU's"
        );
        let mut z_frag: [f64; 8] = [0, 1, 2, 3, 4, 5, 6, 7].map(|k| f32w(e[k]));
        if skew {
            z_frag.rotate_left(1);
        }
        let z_cpu: [f64; 8] = [0, 1, 2, 3, 4, 5, 6, 7].map(|k| f32w(cpu[k]));
        let z_pixel: [f64; 8] = [0, 1, 2, 3, 4, 5, 6, 7].map(|k| f32w(w[16 + k]));
        let (a, b, c) = (
            ic_flat(&decoded(z_frag)),
            ic_flat(&decoded(z_cpu)),
            ic_flat(&decoded(z_pixel)),
        );
        for k in 0..15 {
            assert!(
                within(a[k], b[k], f),
                "at {uv:?}: the fragment's encode(decode(z)) decodes to {} at value {k}, the CPU's encode to {}",
                a[k],
                b[k]
            );
            assert!(
                within(a[k], c[k], f),
                "at {uv:?}: the fragment's encode(decode(z)) decodes to {} at value {k}, ctx.chart.z to {}",
                a[k],
                c[k]
            );
        }
    }
}

#[test]
fn qa_m225_fragment_encode_of_fragment_decode() {
    check_roundtrip(false);
}

negative_control!(
    qa_m225_fragment_encode_of_fragment_decode,
    "the fragment's z read one component off must be found",
    expected = "decodes to",
    check_roundtrip(true)
);

// ── REQ-SYS-085: the read side's shape() is the translation's ───────────────────────────────────────────────────────

/// dd_decoder §3's shape-map names a hand transcription would carry.
const SHAPE_FORMULA: [&str; 3] = ["mu_rho", "mu_lambda", "m01"];

fn wgsl_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == "target" || name == ".git" {
            continue;
        }
        if path.is_dir() {
            wgsl_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "wgsl") {
            out.push(path);
        }
    }
}

/// The string literals of a Rust source (plain and raw), roughly: enough to find a WGSL formula one emits.
fn string_literals(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b[i] == b'r' && (b.get(i + 1) == Some(&b'"') || b.get(i + 1) == Some(&b'#')) {
            let mut j = i + 1;
            let mut hashes = 0;
            while b.get(j) == Some(&b'#') {
                hashes += 1;
                j += 1;
            }
            if b.get(j) == Some(&b'"') {
                let close = format!("\"{}", "#".repeat(hashes));
                if let Some(end) = src[j + 1..].find(&close) {
                    out.push(src[j + 1..j + 1 + end].to_owned());
                    i = j + 1 + end + close.len();
                    continue;
                }
            }
        }
        if b[i] == b'"' {
            let mut j = i + 1;
            while j < b.len() && b[j] != b'"' {
                if b[j] == b'\\' {
                    j += 1;
                }
                j += 1;
            }
            out.push(src[i + 1..j.min(b.len())].to_owned());
            i = j + 1;
            continue;
        }
        if b[i] == b'\'' && b.get(i + 2) == Some(&b'\'') {
            i += 3;
            continue;
        }
        i += 1;
    }
    out
}

fn shape_hits(wgsl: &[(String, String)], read_rs: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for (path, text) in wgsl {
        let lower = text.to_lowercase();
        for t in SHAPE_FORMULA {
            if lower.contains(t) {
                hits.push(format!("{path}: {t}"));
            }
        }
    }
    for s in string_literals(read_rs) {
        let lower = s.to_lowercase();
        for t in SHAPE_FORMULA {
            if lower.contains(t) {
                hits.push(format!("crates/ledger/src/gen/read.rs string: {t}"));
            }
        }
        if s.contains("fn shape(") {
            hits.push("crates/ledger/src/gen/read.rs string: fn shape(".to_owned());
        }
    }
    hits
}

fn tree() -> (Vec<(String, String)>, String) {
    let root = root();
    let mut files = Vec::new();
    for top in ["crates", "xtask", "fixtures"] {
        wgsl_files(&root.join(top), &mut files);
    }
    assert!(
        files.len() > 50,
        "the scan read only {} .wgsl files",
        files.len()
    );
    let texts = files
        .iter()
        .map(|p| (p.display().to_string(), std::fs::read_to_string(p).unwrap()))
        .collect();
    let read_rs = std::fs::read_to_string(root.join("crates/ledger/src/gen/read.rs")).unwrap();
    (texts, read_rs)
}

fn check_shape_scan(wgsl_plant: &str, rs_plant: &str) {
    let (mut wgsl, mut read_rs) = tree();
    if !wgsl_plant.is_empty() {
        wgsl.push(("planted.wgsl".to_owned(), wgsl_plant.to_owned()));
    }
    read_rs.push_str(rs_plant);
    let hits = shape_hits(&wgsl, &read_rs);
    assert!(
        hits.is_empty(),
        "a hand-written shape formula outside the shared source: {hits:?}"
    );
}

#[test]
fn qa_m225_read_side_shape_is_the_translation() {
    let root = root();
    let read_side =
        std::fs::read_to_string(root.join("crates/render/frag/generated/read_side.wgsl")).unwrap();
    let shape = std::fs::read_to_string(root.join("crates/ledger/wgsl/shape.wgsl")).unwrap();
    assert!(
        shape.starts_with("// GENERATED by `cargo xtask build-kernel`"),
        "shape.wgsl is not under the translation's header"
    );
    assert_eq!(
        read_side.matches("fn shape(").count(),
        1,
        "read_side.wgsl has not exactly one shape()"
    );
    assert_eq!(
        shape.matches("fn shape(").count(),
        1,
        "shape.wgsl has not exactly one shape()"
    );
    // The translated function, from its signature to the end of the file, appears in read_side.wgsl verbatim.
    let body = &shape[shape.find("fn shape(").unwrap()..];
    let body = body.trim_end();
    assert!(
        read_side.contains(body),
        "read_side.wgsl's shape() is not shape.wgsl's translated function"
    );
    // And shape() calls the translated body: the function's helpers come with it.
    let first_struct = shape
        .find("struct ")
        .expect("shape.wgsl has its words struct");
    let lifted = shape[first_struct..].trim_end();
    assert!(
        read_side.contains(lifted),
        "read_side.wgsl does not carry shape.wgsl's translation whole"
    );
    check_shape_scan("", "");
}

negative_control!(
    qa_m225_read_side_shape_is_the_translation,
    "a mu_rho formula planted in a .wgsl file must be found",
    expected = "a hand-written shape formula",
    check_shape_scan(
        "fn f(m: vec3<f32>) -> f32 { let mu_rho = m.x * m.y / (m.x + m.y); return mu_rho; }",
        ""
    )
);

mod read_rs_plant {
    use super::*;
    negative_control!(
        qa_m225_read_side_shape_is_the_translation,
        "a mu_rho formula planted in a string of read.rs must be found",
        expected = "a hand-written shape formula",
        check_shape_scan(
            "",
            "\nconst PLANT: &str = \"let mu_rho = m[0] * m[1] / (m[0] + m[1]);\";\n"
        )
    );
}

// ── REQ-COL-006, R-436: no infinite literal and no division by a literal zero in the generated snippets ─────────────

/// The float literals of WGSL `text` (decimal, an optional `f` or `h` suffix; integer literals, `u`/`i`-suffixed or
/// hex, are skipped), each with whether a `/` precedes it.
fn float_literals(text: &str) -> Vec<(String, f64, bool)> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let starts = b[i].is_ascii_digit()
            && (i == 0
                || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.'));
        if !starts {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < b.len()
            && (b[j].is_ascii_alphanumeric()
                || b[j] == b'.'
                || ((b[j] == b'+' || b[j] == b'-') && matches!(b[j - 1], b'e' | b'E')))
        {
            j += 1;
        }
        let token = &text[i..j];
        let divided = text[..i].trim_end().ends_with('/');
        i = j;
        if token.starts_with("0x") || token.ends_with('u') || token.ends_with('i') {
            continue;
        }
        let digits = token.trim_end_matches(['f', 'h']);
        let is_float =
            token.ends_with('f') || token.ends_with('h') || digits.contains(['.', 'e', 'E']);
        if !is_float {
            continue;
        }
        let value: f64 = digits.parse().unwrap_or(f64::INFINITY);
        out.push((token.to_owned(), value, divided));
    }
    out
}

/// No literal in `text` is infinite or rounds past f32's largest finite value, none is a divisor equal to 0, and no
/// identifier names an infinity.
fn check_no_infinite_literal(name: &str, text: &str) {
    let lits = float_literals(text);
    for (token, value, divided) in &lits {
        assert!(
            value.is_finite() && value.abs() <= f64::from(f32::MAX),
            "{name}: the literal {token} is infinite at f32: an infinite literal (R-436)"
        );
        assert!(
            !(*divided && *value == 0.0),
            "{name}: a division by the literal {token}: an infinity written as 1/0 (R-436)"
        );
    }
    for word in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        let w = word.to_ascii_lowercase();
        assert!(
            w != "inf" && w != "infinity",
            "{name}: the identifier {word}: an infinite literal (R-436)"
        );
    }
}

/// The generated snippets' texts, with `plant` appended to the decode.
fn snippets(plant: &str) -> Vec<(&'static str, String)> {
    vec![
        ("decode.wgsl", format!("{}{plant}", presets::DECODE_WGSL)),
        ("encode.wgsl", presets::ENCODE_WGSL.to_owned()),
    ]
}

fn check_snippets_literals(plant: &str) {
    for (name, text) in snippets(plant) {
        // The scan reads the snippet's literals: it finds the many naga writes (0f, 1f, π̂, the smallest normal).
        assert!(
            float_literals(&text).len() > 50,
            "{name}: the scan found too few float literals to be reading the file"
        );
        check_no_infinite_literal(name, &text);
    }
    // The −∞ the decode writes at a zero pair separation is its bits, 0xff800000, through a bitcast.
    assert!(
        presets::DECODE_WGSL.contains("bitcast<f32>(select(")
            && presets::DECODE_WGSL.contains("4286578688u"),
        "decode.wgsl writes no −∞ by its bits"
    );
}

#[test]
fn qa_m225_generated_snippets_have_no_infinite_literal() {
    check_snippets_literals("");
}

negative_control!(
    qa_m225_generated_snippets_have_no_infinite_literal,
    "a V written as 1/0 must be found",
    expected = "an infinity written as 1/0",
    check_snippets_literals("\nfn planted_v() -> f32 { return -1f / 0f; }\n")
);

mod infinite_literal_plant {
    use super::*;
    negative_control!(
        qa_m225_generated_snippets_have_no_infinite_literal,
        "a literal past f32's range must be found",
        expected = "an infinite literal",
        check_snippets_literals("\nconst PLANTED: f32 = -1e39f;\n")
    );
}
