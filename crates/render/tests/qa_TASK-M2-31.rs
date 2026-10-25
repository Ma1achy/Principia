//! QA tests for TASK-M2-31 on the GPU, written from REQ-COL-070's statement and verify detail (R-436; R-79, R-114;
//! render_gui_spec §10.1; dd_decoder §3.6), not from the implementation:
//! - every numeric view of an f32 field the fragment reads as stored (the `ICDescriptor`'s twelve and the sample's
//!   `S`, `theta`, `mean_y`, `C_ty`, `E_0`, `Lz_0`, `closure_min`) draws each non-finite value, `±∞` and NaN payloads
//!   quiet and signalling of both signs, as the hatch, `debug_invalid(frag_xy)`: each such pixel bit-identical to the
//!   same scene's with the canonical NaN in its place (the acceptance's "pixel-identical to a NaN's"), and that within
//!   the harness tolerance of the hatch itself; every finite sample, `±0` and `±` the smallest subnormal among them,
//!   draws bit-identical to the NaN scene's too, so `±∞` moves no measured range end (`qa_gpu_non_finite_views_*`);
//! - `d_min`'s view and its field ramp draw the unset `+∞` in the "not yet" style, so the unset line comes before the
//!   guard on the GPU; the view draws `−∞` and NaN as the hatch, and the field ramp, a presentation view, draws them
//!   bounded / nothing yet's `#141418`, never the hatch (R-467 D14, REQ-COL-108; amended by qa for TASK-M2-34)
//!   (`qa_gpu_non_finite_d_min_*`).
//!
//! Each test registers its negative control (R-176).

use render::present;
use validation::golden_scene::{row_scene, Colouring, Look, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The harness's tolerance between a render and the colour it is compared with, per channel, linear RGB
/// (`render/tests/numeric_views.rs`'s `TOL`: the renders sit within 6e-7 of their twins on Metal).
const TOL: f64 = 1e-5;

/// The canonical quiet NaN (lowering Part 3a), the reference a non-finite value's pixels must equal.
const QNAN: u32 = 0x7FC0_0000;

/// The non-finite values probed: `+∞` (`rho_ratio`'s pole), `−∞` (`V_0`'s and `E_0`'s), and NaN payloads, quiet and
/// signalling, of both signs.
const NON_FINITE: [u32; 10] = [
    0x7F80_0000,
    0xFF80_0000,
    0x7FC0_0000,
    0xFFC0_0000,
    0x7FC0_0001,
    0x7F80_0001,
    0xFF80_0001,
    0x7FBF_FFFF,
    0x7FFF_FFFF,
    0xFFFF_FFFF,
];

/// Finite values, each sample's: `±0`, `±` the smallest subnormal, and two ordinary values.
const FINITE: [u32; 6] = [
    0x0000_0000,
    0x8000_0000,
    0x0000_0001,
    0x8000_0001,
    0x3E80_0000, // 0.25
    0x3F00_0000, // 0.5
];

/// The f32 fields the fragment reads as stored, each with a numeric view.
const FIELDS: [&str; 19] = [
    "m0",
    "m1",
    "m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
    "S",
    "theta",
    "mean_y",
    "C_ty",
    "E_0",
    "Lz_0",
    "closure_min",
];

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// Stores `v` in sample `i`'s `field`.
fn set(s: &mut Scene, field: &str, i: u32, v: f32) {
    match field {
        "m0" => s.set.ic(i).m0 = v,
        "m1" => s.set.ic(i).m1 = v,
        "m2" => s.set.ic(i).m2 = v,
        "q_mass" => s.set.ic(i).q_mass = v,
        "rho_mag" => s.set.ic(i).rho_mag = v,
        "lambda_mag" => s.set.ic(i).lambda_mag = v,
        "rho_ratio" => s.set.ic(i).rho_ratio = v,
        "rho_angle" => s.set.ic(i).rho_angle = v,
        "K_0" => s.set.ic(i).K_0 = v,
        "V_0" => s.set.ic(i).V_0 = v,
        "virial_ratio" => s.set.ic(i).virial_ratio = v,
        "r_min_pair_0" => s.set.ic(i).r_min_pair_0 = v,
        "S" => {
            s.set.sample(i).S(v);
        }
        "theta" => {
            s.set.sample(i).theta(v);
        }
        "mean_y" => {
            s.set.sample(i).mean_y(v);
        }
        "C_ty" => {
            s.set.sample(i).C_ty(v);
        }
        "E_0" => {
            s.set.sample(i).E_0(v);
        }
        "Lz_0" => {
            s.set.sample(i).Lz_0(v);
        }
        "closure_min" => {
            s.set.sample(i).closure_min(v);
        }
        other => panic!("no setter for `{other}`"),
    }
}

/// `field`'s scene: the non-finite values `NON_FINITE`, each replaced by `nan` where it is `Some`, then `FINITE`.
fn scene(field: &'static str, nan: Option<u32>) -> Scene {
    let n = (NON_FINITE.len() + FINITE.len()) as u32;
    let mut s = row_scene(field, n, Colouring::View(field)).unwrap_or_else(|e| panic!("{e}"));
    for (i, b) in (0u32..).zip(NON_FINITE.iter().map(|&b| nan.unwrap_or(b)).chain(FINITE)) {
        set(&mut s, field, i, f32::from_bits(b));
    }
    for (i, b) in (0u32..).zip(NON_FINITE.iter().map(|&b| nan.unwrap_or(b)).chain(FINITE)) {
        let (v, _) = s.value(i).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            v.to_bits(),
            b,
            "`{field}`'s sample {i} reads {v}, not {b:#010x}"
        );
    }
    s
}

fn near(a: [f32; 4], b: [f64; 3]) -> bool {
    (0..3).all(|c| (f64::from(a[c]) - b[c]).abs() <= TOL)
}

/// Checks `field`'s render `got` against `nan`, the render of its scene `s` with the canonical NaN in place of every
/// non-finite value: every pixel the same bits, and each non-finite sample's the hatch.
fn check_view(field: &str, s: &Scene, got: &[[f32; 4]], nan: &[[f32; 4]]) {
    assert_eq!(got.len(), nan.len(), "`{field}`: the images' sizes");
    let width = s.size().0;
    for (i, &b) in (0u32..).zip(NON_FINITE.iter().chain(&FINITE)) {
        let non_finite = (i as usize) < NON_FINITE.len();
        for (x, y) in s.pixels(i) {
            let k = (y * width + x) as usize;
            if non_finite {
                let hatch = Look::Invalid.colour([x, y]);
                assert!(
                    near(nan[k], hatch),
                    "`{field}`'s canonical NaN at pixel ({x}, {y}) is {:?}, not the hatch {hatch:?}",
                    nan[k]
                );
                assert!(
                    got[k].map(f32::to_bits) == nan[k].map(f32::to_bits),
                    "`{field}`'s non-finite {b:#010x} at pixel ({x}, {y}) is {:?}, not the NaN's hatch {:?}",
                    got[k],
                    nan[k]
                );
            } else {
                assert!(
                    got[k].map(f32::to_bits) == nan[k].map(f32::to_bits),
                    "`{field}`'s finite {b:#010x} at pixel ({x}, {y}) is {:?} beside the non-finite values, {:?} \
                     beside NaN: the measured range moved",
                    got[k],
                    nan[k]
                );
            }
        }
    }
}

/// Renders `s` with its view, `edit` (from, to) applied to the WGSL when given: a control's wrong view.
fn render(h: &GpuHarness, s: &Scene, edit: Option<(&str, &str)>) -> Vec<[f32; 4]> {
    match edit {
        None => s.render(h.device(), h.queue()),
        Some((from, to)) => {
            let src = s.colour().unwrap_or_else(|e| panic!("{e}"));
            assert!(src.contains(from), "`{}`'s view has no `{from}`", s.name);
            s.render_colour(h.device(), h.queue(), &src.replace(from, to))
        }
    }
    .unwrap_or_else(|e| panic!("{e}"))
}

fn check_views(edit: Option<(&str, &str)>) {
    let h = gpu();
    for field in FIELDS {
        let s = scene(field, None);
        let r = scene(field, Some(QNAN));
        let (got, nan) = (render(&h, &s, edit), render(&h, &r, None));
        check_view(field, &s, &got, &nan);
    }
}

#[test]
fn qa_gpu_non_finite_views_draw_the_hatch() {
    check_views(None);
}

negative_control!(
    qa_gpu_non_finite_views_draw_the_hatch,
    "views guarding the canonical NaN alone (TASK-M1-09's guard) draw +inf off the hatch",
    expected = "non-finite 0x7f800000 at pixel",
    check_views(Some((
        "(bitcast<u32>(raw) & 0x7f800000u) == 0x7f800000u",
        "bitcast<u32>(raw) == 0x7fc00000u"
    )))
);

#[cfg(feature = "controls")]
mod qa_gpu_non_finite_views_controls {
    use super::*;

    mod signalling_missed {
        use super::*;
        negative_control!(
            qa_gpu_non_finite_views_draw_the_hatch,
            "views guarding the quiet NaNs and ±inf but not a signalling NaN are caught",
            expected = "non-finite 0x7f800001 at pixel",
            check_views(Some((
                "(bitcast<u32>(raw) & 0x7f800000u) == 0x7f800000u",
                "(bitcast<u32>(raw) & 0x7fc00000u) == 0x7fc00000u || (bitcast<u32>(raw) & 0x7fffffffu) == 0x7f800000u"
            )))
        );
    }
}

// ── d_min: the unset +∞ first, then the guard ───────────────────────────────────────────────────────────────────

/// `d_min`'s probes, f16 bits in `packed_a`'s high half (payload §1), and the f32 each reads as: the unset `+∞`
/// (0x7C00, R-271), `−∞` (0xFC00) and a NaN (0x7E00), written whole (`packed_a`), since `set_d_min` stores neither
/// (R-79, R-281): a bitwise-adversarial descriptor (debug tooling plan, "Synthetic-first"); and 0.5, finite.
const D_MIN: [(u32, f32); 4] = [
    (0x7C00, f32::INFINITY),
    (0xFC00, f32::NEG_INFINITY),
    (0x7E00, f32::NAN),
    (0x3800, 0.5),
];

/// The look `d_min`'s sample `i` must draw: the unset `+∞` "not yet"; `−∞` and NaN the hatch in the view and, in the
/// field ramp (`ramp`), a presentation view, bounded / nothing yet's `#141418` decoded from its sRGB code (R-467
/// clause 2, D4, D14; colour_composition §1.4; amended by qa for TASK-M2-34); `None` for the finite.
fn d_min_look(i: usize, ramp: bool) -> Option<Look> {
    match i {
        0 => Some(Look::NotYet),
        1 | 2 if ramp => Some(Look::Flat(present::srgb8([0x14, 0x14, 0x18]))),
        1 | 2 => Some(Look::Invalid),
        _ => None,
    }
}

/// `d_min`'s scene, coloured by its view or, when `ramp`, its field ramp.
fn d_min_scene(ramp: bool) -> Scene {
    let colouring = if ramp {
        Colouring::Ramp {
            ramp: render::colour::field_ramp::FieldRamp::d_min().unwrap_or_else(|e| panic!("{e}")),
            override_on: false,
        }
    } else {
        Colouring::View("d_min")
    };
    let mut s = row_scene("d_min", D_MIN.len() as u32, colouring).unwrap_or_else(|e| panic!("{e}"));
    for (i, (h, _)) in (0u32..).zip(D_MIN) {
        let w = s.set.simstate(i).packed_a;
        s.set.sample(i).packed_a((w & 0xFFFF) | h << 16);
    }
    for (i, (_, v)) in (0u32..).zip(D_MIN) {
        let (got, _) = s.value(i).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            got.to_bits() == v.to_bits() || (v.is_nan() && got.is_nan()),
            "`d_min`'s sample {i} reads {got}, not {v}"
        );
    }
    s
}

fn check_d_min(what: &str, s: &Scene, got: &[[f32; 4]]) {
    let width = s.size().0;
    for (i, (_, v)) in D_MIN.iter().enumerate() {
        let Some(look) = d_min_look(i, what == "field ramp") else {
            continue;
        };
        for (x, y) in s.pixels(i as u32) {
            let want = look.colour([x, y]);
            let k = (y * width + x) as usize;
            assert!(
                near(got[k], want),
                "`d_min`'s {what} draws {} at pixel ({x}, {y}) as {:?}, not {look:?} {want:?}",
                v,
                got[k]
            );
        }
    }
}

/// `d_min`'s unset line and its guard, as R-436 orders them.
#[cfg(feature = "controls")]
const UNSET: &str =
    "    if (bitcast<u32>(raw) == 0x7f800000u) { return debug_not_yet(ctx.frag_xy); }\n";
#[cfg(feature = "controls")]
const GUARD: &str =
    "    if ((bitcast<u32>(raw) & 0x7f800000u) == 0x7f800000u) { return debug_invalid(ctx.frag_xy); }\n";

fn check_d_min_view(edit: Option<(&str, &str)>) {
    let h = gpu();
    let s = d_min_scene(false);
    check_d_min("view", &s, &render(&h, &s, edit));
}

#[test]
fn qa_gpu_non_finite_d_min_view_unset_first() {
    check_d_min_view(None);
}

negative_control!(
    qa_gpu_non_finite_d_min_view_unset_first,
    "d_min's view with the guard ahead of its unset line draws +inf as the hatch",
    expected = "`d_min`'s view draws inf",
    check_d_min_view(Some((
        &format!("{UNSET}{GUARD}"),
        &format!("{GUARD}{UNSET}")
    )))
);

#[cfg(feature = "controls")]
mod qa_gpu_non_finite_d_min_controls {
    use super::*;

    mod nan_alone {
        use super::*;
        negative_control!(
            qa_gpu_non_finite_d_min_view_unset_first,
            "d_min's view guarding the canonical NaN alone draws -inf off the hatch",
            expected = "`d_min`'s view draws -inf",
            check_d_min_view(Some((GUARD, "    if (bitcast<u32>(raw) == 0x7fc00000u) { return debug_invalid(ctx.frag_xy); }\n")))
        );
    }
}

fn check_d_min_ramp(wgsl: Option<(&str, &str)>) {
    let h = gpu();
    let s = d_min_scene(true);
    check_d_min("field ramp", &s, &render(&h, &s, wgsl));
}

#[test]
fn qa_gpu_non_finite_d_min_ramp_unset_first() {
    check_d_min_ramp(None);
}

negative_control!(
    qa_gpu_non_finite_d_min_ramp_unset_first,
    "d_min's field ramp testing the canonical NaN alone draws -inf on the ramp",
    expected = "`d_min`'s field ramp draws -inf",
    check_d_min_ramp(Some((
        "(bitcast<u32>(v) & 0x7f800000u) != 0x7f800000u",
        "bitcast<u32>(v) != 0x7fc00000u"
    )))
);

/// R-467 D14 and REQ-COL-108's control: a field ramp that still hatches `−∞` and NaN, as before R-467, is caught
/// (added by qa for TASK-M2-34).
#[cfg(feature = "controls")]
mod qa_gpu_non_finite_d_min_ramp_hatch {
    use super::*;
    negative_control!(
        qa_gpu_non_finite_d_min_ramp_unset_first,
        "d_min's field ramp drawing -inf and NaN in the hatch is caught",
        expected = "`d_min`'s field ramp draws -inf",
        check_d_min_ramp(Some((
            "return select(PRESENT_INVALID,",
            "return select(debug_invalid(ctx.frag_xy),"
        )))
    );
}
