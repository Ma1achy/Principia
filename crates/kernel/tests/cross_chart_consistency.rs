//! Cross-chart consistency: one state reached through two charts, a shape-sphere pixel and the latent
//! pixel its decoded state encodes to, gives the same `z` and state, and then the same payload and outcome.
//!
//! **The pair.** A pixel `(u, v)` of the shape sphere decodes to the canonical state `x` (`ShapeSphere`'s own decode,
//! `D ∘ Φ`). The encode `E` (`kernel::encode::encode`) takes `x` to `z = E(x)`. A latent chart is centred there,
//! `z₀ = z`, with `q₁ = h·ê_α`, `q₂ = h·ê_β`, read on a `65 × 65` grid whose middle pixel `(32, 32)` has its centre at
//! `(½, ½)` exactly, so that pixel is the chart's centre `z₀`. That pixel is the latent partner of `(u, v)`.
//!
//! - `cross_chart_consistency_z`: at each of 64 shape-sphere pixels, the latent partner's `Φ` gives `z` bit for bit.
//!   Control: a `z` with a negative-zero component, which `z₀ + 0·q₁ + 0·q₂` turns into `+0` (pitfalls §11), fails it.
//! - `cross_chart_consistency_state`: the partner's decode through the latent chart, `y`, is `x` within the encode's
//!   tolerance, `‖y − x‖_phys ≤ ε_phys` (`validation::roundtrip::{phys, eps_phys}`), at f64. Control: the partner one
//!   latent pixel off, `(33, 32)`, whose state is `h·2/65` away in `z_α`, fails it.
//! - `cross_chart_consistency_payload`: the shape-sphere pixel's own decode `x` and its latent partner's `y`, each
//!   narrowed once to f32 as the CPU field narrows its decode, give the same f32 IC bit for bit and the same
//!   degenerate tag; then they produce the same stored sample bit for bit, every byte of the FTLE tier's `SimState`
//!   and every word of the free-group word, under the default occupant, Heggie × RK4, at the CPU field's Heggie
//!   values; so the same outcome class. Also asserted: the free decode of the encoded `z` and the latent chart's pixel
//!   path produce the same sample. Control: the latent partner one pixel off, whose f32 IC differs from the shape
//!   pixel's.
//!
//! **Bit for bit across the charts** is the task's criterion. `x` and `y` are `ε_phys` apart at f64, not equal, but
//! both are narrowed to f32 before the march, so they are the same f32 IC unless a component of `x` sits within
//! `ε_phys` of an f32 rounding boundary, and the same IC marches to the same payload. The IC assertion is the part
//! that does not lean on the march's determinism.
//!
//! **Not the app's latent route.** The CPU field narrows the plane `z₀, q₁, q₂` itself to f32 before it decodes
//! (`engine::field::latent`), so a field view centred on `E(x)` decodes at an f32-rounded `z₀`, about 10⁻⁸ off `x`.
//! This test keeps the chart at f64 and covers the charts' algebra, not that path.
//!
//! **Inputs** (test inputs, not thresholds): the shape sphere is the round-trip registry's spherical chart
//! (`validation::roundtrip::charts::shape_sphere`), both hemispheres, its pixels the centres of an `8 × 8` grid, so no
//! pixel is on a pole or the seam; `h = 0.1`; the horizon `T = 10` at the drift suite's sim key
//! (`validation::drift::sim_key`), the register's `r_coll`.
//!
//! **Domain** (pitfalls §10): one chart pair, one link set (the defaults), 64 states, one horizon, one occupant.

use kernel::chart::{Chart, ChartDecode, Latent, ShapeSphere};
use kernel::decode::{self, Decoded, Ic};
use kernel::detect::collision::CollisionRadius;
use kernel::driver::benettin::BenettinParams;
use kernel::driver::producer::produce;
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use kernel::payload::{sd_detail, sd_state, FreeGroupWord, SimStateFTLE};
use kernel::regularisation::az::AzReal;
use kernel::regularisation::heggie::{HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::pairs::DefaultOccupant;
use kernel::regularisation::sync::delta_sync;
use validation::drift::sim_key;
use validation::negative_control;
use validation::roundtrip::{charts, eps_phys, phys};

/// The shape sphere's pixels across and up: 64 in all.
const SHAPE_N: u32 = 8;
/// The latent chart's pixels across and up: odd, so its middle pixel's centre is `(½, ½)`.
const LATENT_N: u32 = 65;
/// The latent chart's middle pixel, whose centre is its `z₀`.
const MIDDLE: u32 = LATENT_N / 2;
/// The latent chart's half-extent along `ê_α` and `ê_β`.
const H: f64 = 0.1;
/// The horizon.
const T: f64 = 10.0;
/// The collision threshold: the register's `r_coll` default (`ledger::constants::R_COLL`, which the kernel's tests
/// cannot reach), a test input.
const R_COLL: f64 = 1e-3;

/// The centre of pixel `i` of `n`.
fn centre(i: u32, n: u32) -> f64 {
    (f64::from(i) + 0.5) / f64::from(n)
}

/// The shape-sphere chart.
fn sphere() -> ShapeSphere<f64> {
    charts::shape_sphere::<f64>()
}

/// The 64 shape-sphere pixels, row by row.
fn shape_pixels() -> Vec<(f64, f64)> {
    (0..SHAPE_N)
        .flat_map(|j| (0..SHAPE_N).map(move |i| (centre(i, SHAPE_N), centre(j, SHAPE_N))))
        .collect()
}

/// The latent chart centred on `z`, spanning `±h` along `ê_α` and `ê_β`.
fn latent_at(z: [f64; 8]) -> Latent<f64> {
    let e = |k: usize| core::array::from_fn(|i| if i == k { H } else { 0.0 });
    Latent {
        z0: z,
        q1: e(0),
        q2: e(1),
    }
}

/// One shape-sphere pixel and its latent partner: the shape pixel's decode, its encoded `z`, the latent chart on it,
/// and the latent pixel `(i, j)` of that chart (`(MIDDLE, MIDDLE)` for the partner itself).
struct Pair {
    pixel: (f64, f64),
    shape: Decoded<f64>,
    z: [f64; 8],
    latent: Latent<f64>,
    at: (f64, f64),
}

/// The pair of the shape-sphere pixel `pixel`, its latent pixel `latent_pixel`.
fn pair(pixel: (f64, f64), latent_pixel: (u32, u32)) -> Pair {
    let shape = sphere().decode_at::<Defaults, f64>(pixel.0, pixel.1);
    let z = encode::<Defaults, f64>(&shape.ic).z.z;
    Pair {
        pixel,
        shape,
        z,
        latent: latent_at(z),
        at: (
            centre(latent_pixel.0, LATENT_N),
            centre(latent_pixel.1, LATENT_N),
        ),
    }
}

/// The state arm at every shape pixel against its latent pixel `latent_pixel` (the partner, or one off it for the
/// control): `‖y − x‖_phys ≤ ε_phys`. Returns the worst residual and the count
/// of pixels compared and of degenerate decodes among them.
fn state_arm(latent_pixel: (u32, u32)) -> (f64, usize, usize) {
    let tol = eps_phys::<f64>();
    let (mut worst, mut compared, mut degenerate) = (0.0f64, 0usize, 0usize);
    for pixel in shape_pixels() {
        let p = pair(pixel, latent_pixel);
        let y = p.latent.decode_at::<Defaults, f64>(p.at.0, p.at.1);
        let residual = phys(&p.shape.ic, &y.ic);
        assert!(
            residual <= tol,
            "the latent pixel's state is off the shape-sphere pixel's: pixel {:?}, ‖y − x‖_phys = {residual:e} > \
             ε_phys = {tol:e}",
            p.pixel
        );
        worst = worst.max(residual);
        compared += 1;
        degenerate += usize::from(p.shape.degenerate.is_some() || y.degenerate.is_some());
    }
    (worst, compared, degenerate)
}

/// The latent chart centred on `z` places `z` itself at its middle pixel, bit for bit.
fn z_placed(z: [f64; 8]) {
    let at = centre(MIDDLE, LATENT_N);
    let placed = latent_at(z).map(at, at).z.z;
    assert_eq!(
        placed.map(f64::to_bits),
        z.map(f64::to_bits),
        "the latent pixel's z is not the encoded z bit for bit: {z:?}"
    );
}

#[test]
fn cross_chart_consistency_z() {
    // Pass criterion, before the run: at every one of the 64 pixels, the partner's z is the encoded z bit for bit.
    let pixels = shape_pixels();
    for pixel in &pixels {
        z_placed(pair(*pixel, (MIDDLE, MIDDLE)).z);
    }
    eprintln!("z arm: {} pixels placed bit for bit", pixels.len());
}

negative_control!(
    cross_chart_consistency_z,
    "a z with a negative-zero component is not placed bit for bit: -0 + 0·q is +0",
    expected = "the latent pixel's z is not the encoded z bit for bit",
    {
        let mut z = pair(shape_pixels()[0], (MIDDLE, MIDDLE)).z;
        z[2] = -0.0;
        z_placed(z);
    }
);

#[test]
fn cross_chart_consistency_state() {
    // Pass criterion, before the run: every one of the 64 pixels compared, each within ε_phys.
    let (worst, compared, degenerate) = state_arm((MIDDLE, MIDDLE));
    eprintln!(
        "state arm: {compared} pixels, worst ‖y − x‖_phys = {worst:e} (ε_phys = {:e}), {degenerate} degenerate",
        eps_phys::<f64>()
    );
    assert_eq!(
        compared,
        (SHAPE_N * SHAPE_N) as usize,
        "every pixel compared"
    );
}

negative_control!(
    cross_chart_consistency_state,
    "the latent pixel one step off its partner is not the shape-sphere pixel's state",
    expected = "the latent pixel's state is off the shape-sphere pixel's",
    {
        state_arm((MIDDLE + 1, MIDDLE));
    }
);

// ── The payload ──────────────────────────────────────────────────────────────────────────────────────────────────

/// Heggie's values as the engine's CPU field runs them, copied by hand from `engine::field::heggie_params`, which the
/// kernel's tests cannot reach: they go stale silently if the field's change. The field marches through
/// `engine::field::outcome`, not `produce`; both sides of every comparison here use the same `produce`, so the copy
/// changes what is marched, not whether the comparison is fair.
struct FieldValues;

impl HgConfig for FieldValues {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: delta_sync::<R>(),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(4e4, 4e4),
            n_min: 100_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: DEFAULT_TIME,
        }
    }
}

/// The default occupant at the CPU field's Heggie values.
type Occupant = DefaultOccupant<FieldValues>;

/// A produced sample: its `SimState`'s bytes and its word.
#[derive(Clone, PartialEq, Eq)]
struct Payload {
    bytes: Vec<u8>,
    word: [u32; 4],
    state: u32,
    detail: u32,
}

/// `x` narrowed once to f32, as the field narrows its decode.
fn narrow(x: &Ic<f64>) -> Ic<f32> {
    Ic {
        m: x.m.map(|v| v as f32),
        r: x.r.map(|v| v.map(|w| w as f32)),
        p: x.p.map(|v| v.map(|w| w as f32)),
    }
}

/// The bits of `x`.
fn ic_bits(x: &Ic<f32>) -> Vec<u32> {
    x.m.iter()
        .chain(x.r.iter().flatten())
        .chain(x.p.iter().flatten())
        .map(|v| v.to_bits())
        .collect()
}

/// The stored sample of the decode `d`, narrowed to f32, under the default occupant to `T`.
fn payload(d: &Decoded<f64>) -> Payload {
    let mut words = [FreeGroupWord::default()];
    let s: SimStateFTLE = produce::<Occupant, f32, [FreeGroupWord]>(
        &narrow(&d.ic),
        d.degenerate,
        &sim_key::<f32>(T),
        CollisionRadius::from_r_coll(R_COLL),
        BenettinParams {
            delta_0: 1e-5,
            n_renorm: 10,
        },
        0,
        &mut words[..],
        false,
    );
    // SAFETY: `SimStateFTLE` is `repr(C)` with no padding, its two `u16` members adjacent in one 4-byte slot (payload §1;
    // `qa_TASK-M0-13.rs` reads it as 44 words), so all its bytes are initialised.
    let bytes = unsafe {
        std::slice::from_raw_parts(
            (&s as *const SimStateFTLE).cast::<u8>(),
            std::mem::size_of::<SimStateFTLE>(),
        )
    }
    .to_vec();
    Payload {
        bytes,
        word: words[0].free_group_word,
        state: sd_state(s.packed_a),
        detail: sd_detail(s.packed_a),
    }
}

/// The payload arm at every shape pixel against its latent pixel `latent_pixel`: the shape pixel's own decode and the
/// latent chart's decode at that pixel produce the same sample, and so do the free decode of the encoded `z` and the
/// latent chart's. Prints the outcome histogram.
fn payload_arm(latent_pixel: (u32, u32)) {
    let mut classes = std::collections::BTreeMap::<(u32, u32), usize>::new();
    let pixels = shape_pixels();
    for pixel in &pixels {
        let p = pair(*pixel, latent_pixel);
        let free = decode::decode::<Defaults, f64>(&decode::Latent::new(p.z));
        let chart = p.latent.decode_at::<Defaults, f64>(p.at.0, p.at.1);
        // The cross-chart claim that does not depend on the march: the same f32 IC and the same degenerate tag. Adopted
        // after the first run measured 64/64 bit-identical f32 ICs; justified by the ulp argument in the module's
        // note (‖y − x‖ ≲ 10⁻¹⁵ against an f32 half-ulp of ~6·10⁻⁸), not by that run.
        let same_ic = ic_bits(&narrow(&p.shape.ic)) == ic_bits(&narrow(&chart.ic));
        assert!(
            same_ic && p.shape.degenerate == chart.degenerate,
            "the f32 ICs differ across the charts: pixel {:?}, f32 ICs bit-identical: {same_ic}, degenerate tags {:?} \
             against {:?}",
            p.pixel,
            p.shape.degenerate,
            chart.degenerate
        );
        let (own, from_z, from_chart) = (payload(&p.shape), payload(&free), payload(&chart));
        assert!(
            own == from_chart,
            "the payloads differ across the charts: pixel {:?}, f32 ICs bit-identical: {same_ic}, outcome \
             (state, detail) {:?} against {:?}",
            p.pixel,
            (own.state, own.detail),
            (from_chart.state, from_chart.detail)
        );
        assert!(
            from_z == from_chart,
            "the payloads differ from the one z: pixel {:?}, outcome (state, detail) {:?} against {:?}",
            p.pixel,
            (from_z.state, from_z.detail),
            (from_chart.state, from_chart.detail)
        );
        *classes
            .entry((from_chart.state, from_chart.detail))
            .or_default() += 1;
    }
    eprintln!(
        "payload arm: {} pixels bitwise across the charts and from the one z; outcome (state, detail) histogram \
         {classes:?}",
        pixels.len()
    );
}

#[test]
fn cross_chart_consistency_payload() {
    // Pass criterion, before the run: at every one of the 64 pixels, the shape pixel and its latent partner store the
    // same bytes and word, and so do the two routes from the one z.
    payload_arm((MIDDLE, MIDDLE));
}

negative_control!(
    cross_chart_consistency_payload,
    "the latent pixel one step off its partner gives a different payload from the shape pixel's",
    expected = "differ across the charts",
    {
        payload_arm((MIDDLE + 1, MIDDLE));
    }
);
