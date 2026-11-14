//! The libm-pin measurement's harness (R-991, REQ-CHART-212): the first slice's three charts, each decoded in f64 at
//! every sample of its grids, the record each sample gives hashed twice, at f64 and after narrowing to f32, so that two
//! runs, two paths or two machines can be compared record by record (`cargo xtask measure libm-pin`).
//!
//! **The charts** (R-991: the first slice's): the latent chart, the shape sphere's exponential map and the `(L_z, E)`
//! invariant chart, each at the fixed parameters the DECODE presets hold (`validation::decode_preset::uniforms`), held
//! at f64, the shape sphere under its exponential map.
//!
//! **The grids**: 1920 × 1080 samples per chart, at the home window `[0, 1]²` and zoomed 2¹⁰ and 2²⁰ about its centre,
//! sample `(x, y)` at `u = ½ + ((x + ½)/W − ½)·2⁻ᵏ`, `v = ½ + ((y + ½)/H − ½)·2⁻ᵏ`.
//!
//! **A record** is what the integrator reads, the decoder's one output (chart_decoder_contract Part 2, "the integrator's
//! input is `(mᵢ, rᵢ, pᵢ)` and nothing else"): the sample's `Ic`, through `kernel::stage::sample_with` at f64 (the CPU
//! evaluator, `engine::decode_dispatch::decode_only_with`'s body) and, narrowed, the 15 words `write_ic` writes. Its
//! digest is a u64: the high half an FNV-1a hash of the 15 f64 bit patterns, the low half one of the 15 f32 words. Two
//! records whose digests' halves differ differ at that level; two equal halves are taken as equal records, a collision
//! being `2⁻³²` per differing record.

use kernel::chart::invariant::Invariant;
use kernel::chart::{ChartDecode, InvariantLE, Latent, ShapeMap, ShapeSphere};
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{ic_words, sample_with, write_ic};

/// The charts' names, in the order a file holds them.
pub const CHARTS: [&str; 3] = ["latent", "shape-sphere-exponential", "invariant-lz-e"];

/// The zooms, as `k` in `2ᵏ`.
pub const ZOOMS: [u32; 3] = [0, 10, 20];

/// The grid's width.
pub const WIDTH: u32 = 1920;

/// The grid's height.
pub const HEIGHT: u32 = 1080;

/// The arm this build measures: the kernel's f64 path, pinned or the platform's (`std-maths`), and the control.
pub fn arm() -> &'static str {
    match (cfg!(feature = "std-maths"), cfg!(feature = "perturb-sin")) {
        (false, false) => "pinned",
        (true, false) => "std",
        (false, true) => "pinned-perturbed-sin",
        (true, true) => "std-perturbed-sin",
    }
}

/// The latent chart at the DECODE preset's parameters.
fn latent() -> Latent<f64> {
    Latent {
        z0: [0.3, -0.2, 0.1, -0.4, 0.25, 0.05, 0.2, -0.35],
        q1: [0.5, 0.2, -0.3, 0.1, 0.0, 0.2, 0.4, -0.1],
        q2: [-0.2, 0.5, 0.2, 0.3, -0.25, 0.1, -0.3, 0.3],
    }
}

/// The shape sphere at the DECODE preset's parameters, under the exponential map.
fn shape_sphere() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.2,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.3, -0.2],
        z_mom: [0.2, -0.1, 0.3, 0.15],
    }
}

/// The `(L_z, E)` chart at the DECODE preset's frozen controls and the register's warp.
fn invariant_le() -> InvariantLE<f64> {
    InvariantLE(Invariant {
        z_cfg: [0.4, -0.3],
        z_mass: [0.2, -0.5],
        k_max: ledger::constants::K_MAX.number(),
        gamma_k: ledger::constants::GAMMA_K.number(),
    })
}

/// The f64 record of `chart` at `(u, v)`.
fn ic_of<C>(chart: &C, u: f64, v: f64) -> Ic<f64>
where
    C: ChartDecode<Param = f64>,
{
    sample_with::<C, Defaults, f64>(chart, u, v, None).ic
}

/// The 64-bit FNV-1a hash of `words`.
fn fnv(words: impl IntoIterator<Item = u64>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for w in words {
        for byte in w.to_le_bytes() {
            h ^= u64::from(byte);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

/// `ic`'s digest: the f64 level's hash in the high half, the narrowed record's in the low (the module's note).
pub fn digest(ic: &Ic<f64>) -> u64 {
    let wide = [
        ic.m[0], ic.m[1], ic.m[2], ic.r[0][0], ic.r[0][1], ic.r[1][0], ic.r[1][1], ic.r[2][0],
        ic.r[2][1], ic.p[0][0], ic.p[0][1], ic.p[1][0], ic.p[1][1], ic.p[2][0], ic.p[2][1],
    ];
    let narrow = Ic {
        m: ic.m.map(|x| x as f32),
        r: ic.r.map(|v| v.map(|x| x as f32)),
        p: ic.p.map(|v| v.map(|x| x as f32)),
    };
    let mut words = [0u32; 15];
    debug_assert_eq!(ic_words(), words.len());
    write_ic(&mut words, 0, &narrow);
    let fold = |h: u64| (h >> 32) ^ (h & 0xFFFF_FFFF);
    (fold(fnv(wide.map(f64::to_bits))) << 32) | fold(fnv(words.map(u64::from)))
}

/// The sample point `(u, v)` of `(x, y)` on a `width × height` grid zoomed `2^zoom` about `(½, ½)`.
pub fn point(x: u32, y: u32, width: u32, height: u32, zoom: u32) -> (f64, f64) {
    let scale = 1.0 / f64::from(1u32 << zoom);
    let at = |i: u32, n: u32| 0.5 + ((f64::from(i) + 0.5) / f64::from(n) - 0.5) * scale;
    (at(x, width), at(y, height))
}

/// Fills `out`, one row's digests, `width` of them, of chart `chart` (an index of [`CHARTS`]) at row `y`.
pub fn row(chart: usize, zoom: u32, y: u32, width: u32, height: u32, out: &mut [u64]) {
    let (l, s, e) = (latent(), shape_sphere(), invariant_le());
    for (x, slot) in (0..width).zip(out.iter_mut()) {
        let (u, v) = point(x, y, width, height, zoom);
        *slot = digest(&match chart {
            0 => ic_of(&l, u, v),
            1 => ic_of(&s, u, v),
            _ => ic_of(&e, u, v),
        });
    }
}

/// The wasm build's entry: `n` u64 slots for [`libm_pin_fill`], leaked to the caller, the wasm runtime's script.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn libm_pin_alloc(n: u32) -> *mut u64 {
    let mut v = vec![0u64; n as usize];
    let p = v.as_mut_ptr();
    core::mem::forget(v);
    p
}

/// The wasm build's entry: one chart's grid at one zoom, `width × height` digests written from `out` (from
/// [`libm_pin_alloc`]), row by row.
///
/// # Safety
/// `out` must hold `width × height` u64 slots.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub unsafe extern "C" fn libm_pin_fill(
    chart: u32,
    zoom: u32,
    width: u32,
    height: u32,
    out: *mut u64,
) {
    let all = core::slice::from_raw_parts_mut(out, (width * height) as usize);
    for (y, slots) in (0..height).zip(all.chunks_mut(width as usize)) {
        row(chart as usize, zoom, y, width, height, slots);
    }
}
