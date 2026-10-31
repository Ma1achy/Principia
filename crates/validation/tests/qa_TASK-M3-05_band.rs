//! QA tests for TASK-M3-05's round on the owner's ruling (d) of 31 Oct 2026 (R-652; render contract Part 5, "The neutral
//! band") and on a non-finite energy (R-47, R-79, R-436; payload §1, §2), from those texts and not from the
//! implementation. The ramp and its places are written out here.
//! - A fresh sample renders exactly neutral on the backend the test runs on, with the fresh sample a real one: the
//!   default decoder's IC, its `E_0` and `Lz_0` as the march's monitor stores them, not the read side's own values
//!   (`qa_band_fresh_decoded_sample_is_exactly_neutral`). The contract bounds the rounding only where `k_E`, `k_L` ≤ 128,
//!   so those are the domain; the other draws are counted and printed (pitfalls §10; philosophy §4.6).
//! - The references' fresh tile is that neutral (`qa_band_references_hold_the_neutral_fresh_tile`).
//! - A drift past the band keeps its own place, on its own side (`qa_band_drift_past_it_keeps_its_place`).
//! - The band never hides a lit predicate (`qa_band_never_hides_a_lit_predicate`).
//! - A non-finite energy or `L_z` is marked failed for every occupant and at both precisions, and stays marked once a
//!   later value is finite (`qa_non_finite_*`), and a sample with a non-finite operand draws invalid, not the ramp
//!   (`qa_drift_views_draw_a_non_finite_operand_invalid`).

use kernel::decode::{self, Ic, Latent};
use kernel::driver::advance::Advance;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::monitor::Monitor;
use kernel::driver::wrapper::WrapperReal;
use kernel::generated::links::Defaults;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::Geom;
use kernel::payload::{pb_dE_max, pb_dLz_max, sd_detail, sd_state, STATE_SIM_FAILED};
use render::present::{self, Rgb};
#[cfg(feature = "controls")]
use render::registry;
#[cfg(feature = "controls")]
use validation::drift::figure_eight;
use validation::drift::{ic_at, integrate_fixture, AsR, Fixture};
use validation::golden_scene::{row_scene, Colouring, DebugCase, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

const ENERGY: &str = "debug/generated/energy_drift";
const LZ: &str = "debug/generated/Lz_drift";
const DERIVED: &str = "debug/derived/energy_drift";
const EPS: f64 = 1e-6;
/// `k·2⁻²³`, `k = 128` (render contract Part 5).
const BAND: f64 = 128.0 / 8_388_608.0;
const R: [[f32; 2]; 3] = [[0.625, -0.25], [-0.5, 0.5], [-0.125, -0.25]];
const P: [[f32; 2]; 3] = [[0.125, 0.25], [-0.25, -0.0625], [0.125, -0.1875]];

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn view(id: &str) -> Colouring {
    Colouring::Debug(Box::leak(Box::new(DebugCase {
        name: format!("qa {id}"),
        id: id.to_owned(),
        params: Vec::new(),
        nudge: 0,
    })))
}

#[cfg(feature = "controls")]
fn source(id: &str) -> String {
    registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no registry entry `{id}`"))
        .source
}

/// The view with `from` replaced by `to` in its source, the controls' way of breaking it.
#[cfg(feature = "controls")]
fn broken(id: &str, from: &str, to: &str) -> Colouring {
    let src = source(id);
    assert!(src.contains(from), "`{id}` has no `{from}`");
    Colouring::Probe(src.replace(from, to), Vec::new())
}

fn image(h: &GpuHarness, s: &Scene) -> Vec<Rgb> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect()
}

/// The sRGB decode of an 8-bit code.
fn linear(v: u8) -> f64 {
    let v = f64::from(v) / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// `c0` `#DDDDDD`, written out.
fn c0() -> Rgb {
    [linear(0xDD); 3]
}

/// `diverging(c−, c0, c+)` at `t`, mixed in linear RGB.
fn diverging(t: f64) -> Rgb {
    let lo = [linear(0x3B), linear(0x4C), linear(0xC0)];
    let hi = [linear(0xB4), linear(0x04), linear(0x26)];
    let (a, b, u) = if t < 0.5 {
        (lo, c0(), 2.0 * t)
    } else {
        (c0(), hi, 2.0 * t - 1.0)
    };
    [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * u)
}

fn symlog(x: f64) -> Rgb {
    let s = (1.0 + x.abs() / EPS).ln().copysign(x);
    diverging(0.5 + 0.5 * s / (1.0 + s.abs()))
}

fn bytes(c: Rgb) -> [u8; 3] {
    c.map(|v| (v * 255.0).round() as u8)
}

// ── A fresh sample ───────────────────────────────────────────────────────────────────────────────────────────────

/// A small generator of latent draws, so the fixture is the same on every backend.
fn draws(n: usize) -> Vec<[f64; 8]> {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    };
    (0..n).map(|_| std::array::from_fn(|_| next())).collect()
}

/// The contract's `k_E = (9K + 15|V|)/|E_0|` and `k_L = 5·S/|Lz_0|`, in f64 from the state itself.
fn bounds(ic: &Ic<f32>, e_0: f32, lz_0: f32) -> (f64, f64) {
    let f = |x: f32| f64::from(x);
    let k: f64 = (0..3)
        .map(|i| (f(ic.p[i][0]).powi(2) + f(ic.p[i][1]).powi(2)) / (2.0 * f(ic.m[i])))
        .sum();
    let v: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(a, b)| {
            f(ic.m[a]) * f(ic.m[b])
                / (f(ic.r[a][0]) - f(ic.r[b][0])).hypot(f(ic.r[a][1]) - f(ic.r[b][1]))
        })
        .sum();
    let s: f64 = (0..3)
        .map(|i| (f(ic.r[i][0]) * f(ic.p[i][1])).abs() + (f(ic.r[i][1]) * f(ic.p[i][0])).abs())
        .sum();
    ((9.0 * k + 15.0 * v) / f(e_0).abs(), 5.0 * s / f(lz_0).abs())
}

/// Fills tile `i` of `s` with a fresh `ic`, its references the march's monitor's, its references moved by `offset` of
/// themselves for the control; returns whether the sample is inside the band's bound for the view.
fn fresh(s: &mut Scene, i: u32, ic: &Ic<f32>, energy: bool, offset: f32) -> bool {
    let cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let m = cadence.monitor_state();
    let (e_0, lz_0) = (m.e_0(), m.lz_0());
    s.set
        .sample(i)
        .r(ic.r)
        .p(ic.p)
        .E_0(e_0 * (1.0 + offset))
        .Lz_0(lz_0 * (1.0 + offset))
        .times(1000, 500)
        .drift_max(0.0, 0.0);
    let d = s.set.ic(i);
    d.m0 = ic.m[0];
    d.m1 = ic.m[1];
    d.m2 = ic.m[2];
    let (k_e, k_l) = bounds(ic, e_0, lz_0);
    if energy {
        k_e <= 128.0
    } else {
        k_l <= 128.0
    }
}

/// Each in-domain fresh draw of `id` renders exactly `c0`; at least `least` of `draws` of them are in the domain. Out of
/// the domain nothing is asserted, as the contract says such a sample may show: the count is printed.
fn check_fresh(id: &str, energy: bool, colouring: Colouring, offset: f32, least: usize) {
    const TILES: u32 = 8;
    let h = gpu();
    let ics: Vec<Ic<f32>> = draws(96)
        .into_iter()
        .map(|z| ic_at::<f32>(&decode::decode::<Defaults, f64>(&Latent::new(z)).ic))
        .collect();
    let (mut inside, mut outside, mut outside_neutral) = (0, 0, 0);
    let want = bytes(c0());
    for chunk in ics.chunks(TILES as usize) {
        let mut s =
            row_scene("qa band", TILES, colouring.clone()).unwrap_or_else(|e| panic!("{e}"));
        let domain: Vec<bool> = (0u32..)
            .zip(chunk)
            .map(|(i, ic)| fresh(&mut s, i, ic, energy, offset))
            .collect();
        let img = image(&h, &s);
        let width = s.size().0;
        for (i, &inside_bound) in (0u32..).zip(&domain) {
            let neutral = s
                .pixels(i)
                .into_iter()
                .all(|(x, y)| bytes(img[(y * width + x) as usize]) == want);
            if inside_bound {
                inside += 1;
                assert!(neutral, "`{id}`: fresh draw {i} is not neutral");
            } else {
                outside += 1;
                outside_neutral += usize::from(neutral);
            }
        }
    }
    println!(
        "`{id}`: {inside} inside the bound; {outside} outside, {outside_neutral} of them neutral"
    );
    assert!(inside >= least, "`{id}`: only {inside} draws in the domain");
}

#[test]
fn qa_band_fresh_decoded_sample_is_exactly_neutral() {
    check_fresh(ENERGY, true, view(ENERGY), 0.0, 48);
    check_fresh(DERIVED, true, view(DERIVED), 0.0, 48);
    check_fresh(LZ, false, view(LZ), 0.0, 48);
}

negative_control!(
    qa_band_fresh_decoded_sample_is_exactly_neutral,
    "references off by four bands: not a fresh sample",
    expected = "is not neutral",
    check_fresh(ENERGY, true, view(ENERGY), 4.0 * BAND as f32, 48)
);

/// Reads tile `tile` of a case's reference.
fn reference_tile(case: &str, tile: usize) -> Vec<[u8; 3]> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/golden")
        .join(case)
        .join("reference.png");
    let file = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .unwrap_or_else(|e| panic!("{e}"));
    let mut buf = vec![0; reader.output_buffer_size().expect("a size")];
    let info = reader
        .next_frame(&mut buf)
        .unwrap_or_else(|e| panic!("{e}"));
    let channels = info.color_type.samples();
    let (w, h) = (info.width as usize, info.height as usize);
    assert_eq!((w, h), (64, 8), "{case}: the showcase row's size");
    let mut out = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
            let at = (y * w + tile * 8 + x) * channels;
            out.push([buf[at], buf[at + 1], buf[at + 2]]);
        }
    }
    out
}

/// A case's tile `tile` is one flat neutral: the references hold the image bytes as rendered, `c0` in linear RGB.
fn check_reference_neutral(case: &str, tile: usize) {
    let want = bytes(c0());
    assert!(
        reference_tile(case, tile).iter().all(|p| *p == want),
        "{case}: tile {tile} is not the neutral {want:?}"
    );
}

#[test]
fn qa_band_references_hold_the_neutral_fresh_tile() {
    for case in [
        "debug-views/generated-energy_drift",
        "debug-views/generated-Lz_drift",
        "debug-views/derived-energy_drift",
    ] {
        check_reference_neutral(case, 0);
    }
}

negative_control!(
    qa_band_references_hold_the_neutral_fresh_tile,
    "a tile that holds a real drift",
    expected = "is not the neutral",
    check_reference_neutral("debug-views/generated-energy_drift", 1)
);

// ── A drift past the band ─────────────────────────────────────────────────────────────────────────────────────────

/// `(H, L_z)` of the dyadic state, by a tile of the scene.
fn invariants(s: &mut Scene, i: u32) -> (f32, f32) {
    fill(s, i, 0.0, 0.0, 0.0, 0.0);
    let read = s.read(i);
    (read.energy_drift, read.Lz_drift)
}

fn fill(s: &mut Scene, i: u32, e_0: f32, lz_0: f32, de_max: f32, dlz_max: f32) {
    s.set
        .sample(i)
        .r(R)
        .p(P)
        .E_0(e_0)
        .Lz_0(lz_0)
        .times(1000, 500)
        .drift_max(de_max, dlz_max);
}

/// Multiples of the band, each way. The rounding of `H` on the state is under 128 of its 2⁻²³ units, so a drift of `f`
/// bands is known to `1 ± 1/f`; the brackets below hold that much.
const PAST: [f64; 4] = [2.0, 16.0, -2.0, -16.0];

/// Each tile of `PAST` is on its own side of `c0` and between the colours of its drift over and under by the rounding.
fn check_past(id: &str, energy: bool, colouring: Colouring) {
    let h = gpu();
    let mut s =
        row_scene("qa band", PAST.len() as u32, colouring).unwrap_or_else(|e| panic!("{e}"));
    let mut xs = Vec::new();
    for (i, f) in (0u32..).zip(PAST) {
        let (e, l) = invariants(&mut s, i);
        let reference = f64::from(if energy { e } else { l });
        let x = f * BAND * reference.abs();
        assert!(reference != 0.0, "the state's reference is 0");
        if energy {
            fill(&mut s, i, (f64::from(e) - x) as f32, l, 0.0, 0.0);
        } else {
            fill(&mut s, i, e, (f64::from(l) - x) as f32, 0.0, 0.0);
        }
        xs.push(x);
    }
    let img = image(&h, &s);
    let width = s.size().0;
    for (i, (x, f)) in (0u32..).zip(xs.iter().zip(PAST)) {
        let (lo, hi) = (
            symlog(x * (1.0 - 1.0 / f.abs())),
            symlog(x * (1.0 + 1.0 / f.abs())),
        );
        for (px, py) in s.pixels(i) {
            let got = img[(py * width + px) as usize];
            assert!(
                bytes(got) != bytes(c0()),
                "`{id}`: the band swallowed {f} bands"
            );
            for k in 0..3 {
                let (a, b) = (lo[k].min(hi[k]) - 1e-3, lo[k].max(hi[k]) + 1e-3);
                assert!(
                    (a..=b).contains(&got[k]),
                    "`{id}`: {f} bands: channel {k} is {}, off its place [{a}, {b}]",
                    got[k]
                );
            }
        }
    }
}

#[test]
fn qa_band_drift_past_it_keeps_its_place() {
    check_past(ENERGY, true, view(ENERGY));
    check_past(DERIVED, true, view(DERIVED));
    check_past(LZ, false, view(LZ));
}

negative_control!(
    qa_band_drift_past_it_keeps_its_place,
    "a band 8 times wider swallows two bands",
    expected = "the band swallowed",
    check_past(
        LZ,
        false,
        broken(LZ, "ctx.sample.Lz_0)", "8.0 * ctx.sample.Lz_0)")
    )
);

/// A drift inside the band with both predicates lit keeps its lines: rows and columns in black over `c0`.
fn check_lit(id: &str, colouring: Colouring) {
    let h = gpu();
    let mut s = row_scene("qa band", 1, colouring).unwrap_or_else(|e| panic!("{e}"));
    let (e, l) = invariants(&mut s, 0);
    fill(&mut s, 0, e, l, 1024.0, 1024.0);
    let img = image(&h, &s);
    let width = s.size().0;
    for (x, y) in s.pixels(0) {
        let line = y % 4 == 0 || x % 4 == 0;
        let want = if line { [0, 0, 0] } else { bytes(c0()) };
        let got = bytes(img[(y * width + x) as usize]);
        assert_eq!(
            got, want,
            "`{id}`: pixel ({x}, {y}) with both predicates lit"
        );
    }
}

#[test]
fn qa_band_never_hides_a_lit_predicate() {
    for id in [ENERGY, DERIVED, LZ] {
        check_lit(id, view(id));
    }
}

negative_control!(
    qa_band_never_hides_a_lit_predicate,
    "the styling handed the band's reading: nothing lit",
    expected = "with both predicates lit",
    check_lit(LZ, broken(LZ, "suspect_lz(ctx.sample.dLz_max)", "false"))
);

// ── Non-finite ────────────────────────────────────────────────────────────────────────────────────────────────────

/// A failed run's stored marks, for the occupant `A` at `R` on `fixture`.
fn check_marked<A: Advance, R: WrapperReal>(what: &str, fixture: &Fixture, detail: u32)
where
    f64: AsR<R>,
{
    let (out, _) = integrate_fixture::<A, R>(fixture);
    assert_eq!(
        sd_state(out.packed_a),
        STATE_SIM_FAILED,
        "{what}: not marked failed"
    );
    assert_eq!(sd_detail(out.packed_a), detail, "{what}: the detail");
    assert_eq!(pb_dE_max(out.packed_b).to_bits(), 0, "{what}: dE_max");
    assert_eq!(pb_dLz_max(out.packed_b).to_bits(), 0, "{what}: dLz_max");
}

fn fixture(name: &'static str, r: [[f64; 2]; 3], p: [[f64; 2]; 3]) -> Fixture {
    Fixture {
        name,
        ic: Ic {
            m: [0.25, 0.25, 0.5],
            r,
            p,
        },
        horizon: 0.002,
    }
}

/// Far apart and moving, so `E` is finite and `L_z` alone is non-finite at `scale`.
fn lz_only(scale: f64) -> Fixture {
    fixture(
        "L_z alone non-finite",
        [[scale, 0.0], [-scale, scale], [0.0, -scale]],
        [[1e6, 1e6], [-1e6, 1e6], [0.0, -2e6]],
    )
}

/// Coincident bodies from rest: `E₀ = −∞`, the state NaN after the first kick.
fn coincident() -> Fixture {
    fixture(
        "coincident",
        [[0.0, 0.0], [0.0, 0.0], [0.0, 0.5]],
        [[0.0; 2]; 3],
    )
}

fn check_every_occupant<A: Advance>(name: &str) {
    check_marked::<A, f32>(&format!("{name}, f32, coincident"), &coincident(), 0);
    check_marked::<A, f64>(&format!("{name}, f64, coincident"), &coincident(), 0);
    check_marked::<A, f32>(&format!("{name}, f32, L_z alone"), &lz_only(4e33), 2);
}

#[test]
fn qa_non_finite_marks_every_occupant_failed() {
    check_every_occupant::<Euler>("Euler");
    check_every_occupant::<Kdk>("KDK");
    check_every_occupant::<Rk4>("RK4");
    check_every_occupant::<Yoshida4>("Yoshida-4");
    check_every_occupant::<Yoshida6>("Yoshida-6");
}

negative_control!(
    qa_non_finite_marks_every_occupant_failed,
    "a healthy run",
    expected = "not marked failed",
    check_marked::<Kdk, f32>("the figure-eight", &figure_eight(), 2)
);

/// A NaN measurement followed by finite states stays non-finite.
fn check_stays(seen_after: bool) {
    let m = [0.25_f64, 0.25, 0.5];
    let r = [[-0.5, -0.25], [0.5, -0.25], [0.0, 0.25]];
    let p = [[0.1, 0.0], [-0.1, 0.0], [0.0, 0.0]];
    let geom = Geom::new(&m, &r);
    let mut mon = Monitor::new(&m, &r, &p, &geom);
    if seen_after {
        let bad = [[f64::NAN, 0.0], [-0.1, 0.0], [0.0, 0.0]];
        mon.update(&m, &r, &bad, &geom);
    }
    for _ in 0..3 {
        mon.update(&m, &r, &p, &geom);
    }
    assert!(mon.non_finite(), "the monitor forgot a NaN measurement");
}

#[test]
fn qa_non_finite_stays_past_a_finite_state() {
    check_stays(true);
}

negative_control!(
    qa_non_finite_stays_past_a_finite_state,
    "no NaN measured",
    expected = "forgot a NaN",
    check_stays(false)
);

/// Sets tile `i` to the stored output of a failed run, as a `SimState` carries it; the masses are the IC's.
fn failed_tile(s: &mut Scene, i: u32, fx: &Fixture) {
    let (out, _) = integrate_fixture::<Kdk, f32>(fx);
    s.set
        .sample(i)
        .r(out.r)
        .p(out.p)
        .E_0(out.E_0)
        .Lz_0(out.Lz_0)
        .times(2, 1)
        .drift_max(pb_dE_max(out.packed_b), pb_dLz_max(out.packed_b))
        .state(sd_state(out.packed_a))
        .detail(sd_detail(out.packed_a));
    let d = s.set.ic(i);
    d.m0 = 0.25;
    d.m1 = 0.25;
    d.m2 = 0.5;
}

/// The first pixel of tile `i` that is not the invalid hatch, with what it is.
fn off_hatch(s: &Scene, img: &[Rgb], i: u32) -> Option<String> {
    let width = s.size().0;
    s.pixels(i).into_iter().find_map(|(x, y)| {
        let got = bytes(img[(y * width + x) as usize]);
        let want = bytes(present::debug_invalid([
            f64::from(x) + 0.5,
            f64::from(y) + 0.5,
        ]));
        (got != want).then(|| format!("pixel ({x}, {y}) is {got:?}, the hatch {want:?}"))
    })
}

/// The cases of [`check_invalid`], by tile.
const NON_FINITE: [&str; 4] = [
    "a stored -inf reference on a finite state",
    "a NaN reference, its sign set, on a finite state",
    "a failed run's output: coincident bodies, E_0 = -inf",
    "a failed run's output: overflowing momenta, E_0 = +inf",
];

/// R-79, R-436 and render_gui_spec §10.1: a non-finite operand draws the hatch, never a place on the ramp. Every view and
/// case is judged before it fails, so the failure lists them all.
fn check_invalid(ids: &[(&str, Colouring)]) {
    let h = gpu();
    let mut failures = Vec::new();
    for (id, colouring) in ids {
        let mut s = row_scene("qa band", 4, colouring.clone()).unwrap_or_else(|e| panic!("{e}"));
        fill(&mut s, 0, f32::NEG_INFINITY, f32::NEG_INFINITY, 0.0, 0.0);
        let nan = f32::from_bits(0xFFC0_0000);
        fill(&mut s, 1, nan, nan, 0.0, 0.0);
        failed_tile(&mut s, 2, &coincident());
        failed_tile(
            &mut s,
            3,
            &fixture(
                "momenta whose squares overflow at f32",
                [[-0.5, -0.25], [0.5, -0.25], [0.0, 0.25]],
                [[3e19, 0.0], [-3e19, 0.0], [0.0, 0.0]],
            ),
        );
        let img = image(&h, &s);
        for (i, what) in (0u32..).zip(NON_FINITE) {
            // That run's `L_z` is finite, so the L_z view's drift is a value, shown as it is (R-79).
            if *id == LZ && i == 3 {
                continue;
            }
            if let Some(off) = off_hatch(&s, &img, i) {
                failures.push(format!("`{id}`, {what}: {off}"));
            }
        }
    }
    assert!(failures.is_empty(), "not invalid:\n{}", failures.join("\n"));
}

#[test]
fn qa_drift_views_draw_a_non_finite_operand_invalid() {
    check_invalid(&[
        (ENERGY, view(ENERGY)),
        (DERIVED, view(DERIVED)),
        (LZ, view(LZ)),
    ]);
}

negative_control!(
    qa_drift_views_draw_a_non_finite_operand_invalid,
    "a view with no hatch at all",
    expected = "not invalid",
    check_invalid(&[(
        LZ,
        broken(
            LZ,
            "let x = ctx.sample.Lz_drift;",
            "let x = 0.0 * ctx.sample.Lz_0 + 1.0;"
        ),
    )])
);
