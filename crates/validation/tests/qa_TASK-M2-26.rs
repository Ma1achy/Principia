//! QA tests for TASK-M2-26 on the CPU, written from the requirements (REQ-RENDER-025, REQ-ENC-002), not from the
//! implementation.
//!
//! - REQ-RENDER-025, inverse-encode contract Part 8 ("Legitimate bright regions: clamp boundaries, feasibility edges,
//!   the mirror tie — *tagged expected*") and Part 4 ("the clamp flags are the honest boundary")
//!   (`qa_m226_tags_follow_what_the_trip_reports`): the ROUNDTRIP class of a trip is read off what the trip itself
//!   reports. Over foreign physical states (random scale, offset, orientation and parity, including extreme mass ratios
//!   and momenta that push a control past its clamp), each encoded by the one encode (`kernel::encode::encode`, f32,
//!   default links): `Class::of` on the encode's notices word as read on a chart whose decode lands on `C`'s section
//!   (`tie_notices(notices, true)`, the whole word, where `lookup_mirrored` is R-82's tie): where the report says
//!   `lookup_clamped` and not `lookup_mirrored` the class is the clamp tag; `lookup_mirrored` and not clamped, the
//!   mirror-tie tag; both, one of those two; neither, untagged. Where either decode's tag word is any `DEGENERATE`
//!   reason, the class is the feasibility tag whatever the encode reports; a non-finite trip is invalid. Each of the
//!   clamped-only, mirrored-only and plain cases is reached. Control: a class function blind to the mirror bit.
//! - Inverse-encode contract Part 4 ("One norm for every round trip"), Part 1's T3 (`D(E(x)) = C(x)`), R-544 and R-82
//!   (`qa_m226_shape_sphere_lower_hemisphere_untagged_within_eps_phys`): the shape sphere chart decodes each pixel as
//!   its own system, off `C`'s section in the lower hemisphere, so there the encode's mirror is `C`'s own, not R-82's
//!   tie. Over a grid of the chart's lower hemisphere (CPU twin, f32, default links), every sample the encode mirrors
//!   and does not clamp, with no `DEGENERATE` tag, is untagged and its residual, measured against `C(D(z))`, within
//!   `ε_phys`; and the mirror is real there (the residual against `D(z)` itself is past `ε_phys`), so the check bites.
//!   No sample anywhere on the sphere is tagged mirror tie. Control: the twin read as if on the section, which tags the
//!   lower hemisphere mirror tie.
//! - REQ-RENDER-025 ("log-scaled", "dark everywhere except … tagged expected"; colour_composition §6, Discipline 1:
//!   "ROUNDTRIP-red means the same thing every time") (`qa_m226_colour_is_log_scaled_and_red_only_untagged`): an
//!   untagged trip within `ε_phys` is black; past it, its red rises by the same step for every doubling of the residual
//!   until full scale (log-scaled), and only an untagged trip ever has a red channel, so red is never an expected tag.
//!   Each tag's colour is visible (non-black) even within tolerance, and the three tags differ from one another.
//!   Control: a linear ramp, which fails the equal-step-per-doubling check.
//! - REQ-RENDER-025, R-75 (`qa_m226_no_kernel_variant`): the kernel's baked variants are exactly the bring-up mode, and
//!   ROUNDTRIP is a library preset, a colour view assembled for the latent and `(L_z, E)` charts. Control: a variant
//!   list with a ROUNDTRIP mode added.
//!
//! **Domain** (pitfalls §10): stateless encode/decode of single states; no trajectory.

use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use render::bind::ViewOutput;
use render::presets::roundtrip::{self, Class};
use render::presets::DecodeChart;
use validation::negative_control;

/// A small deterministic generator (SplitMix64), so the states are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[lo, hi)`.
    fn in_range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 / (1u64 << 53) as f64)
    }
}

/// A foreign physical state of kind `kind`: 0 ordinary, 1 an extreme mass ratio, 2 huge momenta, 3 a near-coincident
/// inner pair; random offset, orientation, scale and parity.
fn state(rng: &mut Rng, kind: u32) -> Ic<f32> {
    let mut w = [
        rng.in_range(0.2, 1.0),
        rng.in_range(0.2, 1.0),
        rng.in_range(0.2, 1.0),
    ];
    if kind == 1 {
        w[(rng.next() % 3) as usize] = 1e-9;
    }
    let total: f64 = w.iter().sum();
    let m = w.map(|x| x / total);
    let mut r = [[0.0f64; 2]; 3];
    for ri in &mut r {
        *ri = [rng.in_range(-2.0, 2.0), rng.in_range(-2.0, 2.0)];
    }
    if kind == 3 {
        r[1] = [r[0][0] + 1e-9, r[0][1]];
    }
    let pscale = if kind == 2 { 1e6 } else { 1.0 };
    let p0 = [
        rng.in_range(-1.0, 1.0) * pscale,
        rng.in_range(-1.0, 1.0) * pscale,
    ];
    let p1 = [
        rng.in_range(-1.0, 1.0) * pscale,
        rng.in_range(-1.0, 1.0) * pscale,
    ];
    let mut p = [p0, p1, [-(p0[0] + p1[0]), -(p0[1] + p1[1])]];
    // Rigid motion, scale and parity: rotate, reflect, scale and offset positions; rotate and reflect momenta.
    let th = rng.in_range(0.0, std::f64::consts::TAU);
    let (c, s) = (th.cos(), th.sin());
    let flip = if rng.next().is_multiple_of(2) {
        1.0
    } else {
        -1.0
    };
    let scale = rng.in_range(0.1, 10.0);
    let off = [rng.in_range(-5.0, 5.0), rng.in_range(-5.0, 5.0)];
    let rot = |v: [f64; 2]| [c * v[0] - s * v[1] * flip, s * v[0] + c * v[1] * flip];
    for ri in &mut r {
        let v = rot(*ri);
        *ri = [scale * v[0] + off[0], scale * v[1] + off[1]];
    }
    for pi in &mut p {
        *pi = rot(*pi);
    }
    Ic {
        m: m.map(|x| x as f32),
        r: r.map(|v| v.map(|x| x as f32)),
        p: p.map(|v| v.map(|x| x as f32)),
    }
}

// ── The class follows what the trip reports ─────────────────────────────────────────────────────────────────────────

fn check_tags(class_of: fn(bool, u32, u32, u32) -> Class) {
    let mut rng = Rng(0x2026_0226);
    let (mut clamped_only, mut mirrored_only, mut plain, mut both) = (0, 0, 0, 0);
    for i in 0..4000u32 {
        let x = state(&mut rng, i % 4);
        let e = kernel::encode::encode::<Defaults, f32>(&x);
        let notices = kernel::stage::encode::notices(&e.report);
        let (cl, mi) = (e.report.lookup_clamped(), e.report.lookup_mirrored());
        let got = class_of(true, 0, 0, roundtrip::tie_notices(notices, true));
        match (cl, mi) {
            (true, false) => {
                clamped_only += 1;
                assert_eq!(got, Class::Clamp, "state {i}: clamped, not tagged clamp");
            }
            (false, true) => {
                mirrored_only += 1;
                assert_eq!(
                    got,
                    Class::MirrorTie,
                    "state {i}: mirrored, not tagged mirror tie"
                );
            }
            (true, true) => {
                both += 1;
                assert!(
                    matches!(got, Class::Clamp | Class::MirrorTie),
                    "state {i}: clamped and mirrored, tagged {got:?}"
                );
            }
            (false, false) => {
                plain += 1;
                assert_eq!(
                    got,
                    Class::Untagged,
                    "state {i}: nothing reported, tagged {got:?}"
                );
            }
        }
        // A DEGENERATE decode on either side is the feasibility tag, whatever the encode reported.
        for reason in 1..=5u32 {
            assert_eq!(
                class_of(true, reason, 0, notices),
                Class::Feasibility,
                "state {i}: first decode tag {reason}"
            );
            assert_eq!(
                class_of(true, 0, reason, notices),
                Class::Feasibility,
                "state {i}: free decode tag {reason}"
            );
        }
        assert_eq!(
            class_of(false, 0, 0, notices),
            Class::Invalid,
            "state {i}: a non-finite trip"
        );
    }
    println!("qa_m226_tags: clamped only {clamped_only}, mirrored only {mirrored_only}, both {both}, plain {plain}");
    assert!(clamped_only > 0, "no state reached a clamp alone");
    assert!(mirrored_only > 0, "no state was mirrored alone");
    assert!(plain > 0, "no state was plain");
}

#[test]
fn qa_m226_tags_follow_what_the_trip_reports() {
    check_tags(Class::of);
}

negative_control!(
    qa_m226_tags_follow_what_the_trip_reports,
    "a class blind to the mirror bit misses the mirror tie",
    expected = "not tagged mirror tie",
    check_tags(|f, t, tf, n| Class::of(f, t, tf, n & !roundtrip::MIRRORED))
);

// ── The colour ───────────────────────────────────────────────────────────────────────────────────────────────────────

fn check_colour(colour: fn(Class, f64, [f64; 2]) -> [f64; 3]) {
    let xy = [3.5, 7.5];
    // Within tolerance, untagged is black.
    for r in [0.0, 1e-6, 0.25, 0.5, 0.999, 1.0] {
        assert_eq!(
            colour(Class::Untagged, r, xy),
            [0.0; 3],
            "untagged within ε_phys at ratio {r} is not black"
        );
    }
    // Past it: only red, rising, by an equal step for every doubling (log-scaled), up to a full scale.
    let red = |r: f64| {
        let c = colour(Class::Untagged, r, xy);
        assert_eq!(
            (c[1], c[2]),
            (0.0, 0.0),
            "untagged at ratio {r} has a non-red channel: {c:?}"
        );
        c[0]
    };
    assert!(red(1.5) > 0.0, "a residual past ε_phys is not red");
    let step = red(4.0) - red(2.0);
    assert!(step > 0.0, "the red does not rise with the residual");
    let mut r = 2.0;
    let mut saturated = false;
    while r < 1e30 {
        let (a, b) = (red(r), red(2.0 * r));
        assert!(b >= a, "the red falls from ratio {r} to {}", 2.0 * r);
        if b < 1.0 && !saturated {
            assert!(
                ((b - a) - step).abs() <= 1e-9,
                "not log-scaled: doubling from ratio {r} adds {} red, not {step}",
                b - a
            );
        } else {
            saturated = true;
        }
        r *= 2.0;
    }
    assert!(
        saturated && red(1e30) <= 1.0,
        "the red never reaches a full scale"
    );
    // Tags: no red at any residual, visible within tolerance, distinct from each other.
    let tags = [Class::Feasibility, Class::Clamp, Class::MirrorTie];
    for t in tags {
        for r in [0.0, 0.5, 1.0, 2.0, 1e3, 1e9, 1e30] {
            let c = colour(t, r, xy);
            assert_eq!(c[0], 0.0, "{t:?} at ratio {r} has red: {c:?}");
            assert!(
                c.iter().any(|v| *v > 0.0),
                "{t:?} at ratio {r} is black, its tag invisible"
            );
        }
    }
    for (i, a) in tags.iter().enumerate() {
        for b in &tags[i + 1..] {
            assert_ne!(
                colour(*a, 0.5, xy),
                colour(*b, 0.5, xy),
                "{a:?} and {b:?} draw alike"
            );
        }
    }
}

#[test]
fn qa_m226_colour_is_log_scaled_and_red_only_untagged() {
    check_colour(roundtrip::colour);
}

negative_control!(
    qa_m226_colour_is_log_scaled_and_red_only_untagged,
    "a linear ramp past ε_phys is not log-scaled",
    expected = "not log-scaled",
    check_colour(|c, r, xy| match c {
        Class::Untagged => [((r - 1.0).max(0.0) / 1e6).min(1.0), 0.0, 0.0],
        other => roundtrip::colour(other, r, xy),
    })
);

// ── No kernel variant ────────────────────────────────────────────────────────────────────────────────────────────────

fn check_variants(variants: &[(&str, bool)]) {
    assert_eq!(
        variants,
        &[("bring_up", true)],
        "the kernel's variants are not exactly the bring-up mode"
    );
    let p = render::presets::library()
        .into_iter()
        .find(|p| p.name() == "ROUNDTRIP")
        .expect("ROUNDTRIP is in the preset library");
    assert_eq!(
        p.output(),
        ViewOutput::Colour,
        "ROUNDTRIP is not a colour view"
    );
    for chart in [DecodeChart::Latent, DecodeChart::InvariantLE] {
        p.module(chart).unwrap_or_else(|e| {
            panic!("ROUNDTRIP does not assemble as a fragment view on {chart:?}: {e}")
        });
    }
}

#[test]
fn qa_m226_no_kernel_variant() {
    check_variants(kernel::bringup::VARIANTS);
}

negative_control!(
    qa_m226_no_kernel_variant,
    "a ROUNDTRIP kernel mode beside the bring-up mode is refused",
    expected = "not exactly the bring-up mode",
    check_variants(&[("bring_up", true), ("roundtrip", true)])
);

// ── The shape sphere's lower hemisphere: C's own mirror, not the tie ────────────────────────────────────────────────

fn check_shape_sphere(on_section: bool) {
    use kernel::chart::ShapeSphere;
    use kernel::stage::decode::{sample_with, tag, UniformChart};
    let side = 48u32;
    let scene =
        validation::decode_preset::Scene::new(DecodeChart::ShapeSphere, [1, 1], 1, 20, &[[0, 0]])
            .unwrap();
    let u = scene.uniforms;
    let chart = ShapeSphere::<f32>::from_uniform(&u.chart);
    let (mut lower_mirrored, mut real_mirror) = (0usize, 0usize);
    for j in 0..side {
        for i in 0..side {
            let uv = [
                (i as f32 + 0.5) / side as f32,
                (j as f32 + 0.5) / side as f32,
            ];
            let out = sample_with::<ShapeSphere<f32>, Defaults, f32>(&chart, uv[0], uv[1], None);
            let first = tag(out.degenerate);
            // The shape sphere is off C's section: the twin as the preset reads it.
            let t = validation::roundtrip_preset::twin_of(&out.ic, first, on_section);
            assert_ne!(
                t.class,
                Class::MirrorTie,
                "shape sphere sample {uv:?}: tagged mirror tie, off R-82's band"
            );
            let clamped = t.notices & roundtrip::CLAMPED != 0;
            if uv[1] < 0.5 && t.notices & roundtrip::MIRRORED != 0 && !clamped && t.tags == 0 {
                lower_mirrored += 1;
                assert_eq!(
                    t.class,
                    Class::Untagged,
                    "lower hemisphere sample {uv:?}: mirrored by C and tagged"
                );
                assert!(
                    t.ratio() <= 1.0,
                    "lower hemisphere sample {uv:?}: {} ε_phys against C(D(z)), past ε_phys",
                    t.ratio()
                );
                // The mirror is real: against D(z) itself the trip is far off.
                let raw = validation::roundtrip_preset::twin_of(&out.ic, first, true);
                if raw.ratio() > 1.0 {
                    real_mirror += 1;
                }
            }
        }
    }
    println!(
        "qa_m226_shape_sphere: {lower_mirrored} lower-hemisphere samples mirrored by C, {real_mirror} past ε_phys against D(z)"
    );
    assert!(
        lower_mirrored * 4 > (side * side) as usize,
        "too few lower-hemisphere samples mirrored by C: {lower_mirrored}"
    );
    assert!(
        real_mirror * 2 > lower_mirrored,
        "the mirror is not visible against D(z): {real_mirror} of {lower_mirrored}"
    );
}

#[test]
fn qa_m226_shape_sphere_lower_hemisphere_untagged_within_eps_phys() {
    check_shape_sphere(false);
}

negative_control!(
    qa_m226_shape_sphere_lower_hemisphere_untagged_within_eps_phys,
    "the shape sphere read as if on C's section tags its lower hemisphere mirror tie",
    expected = "tagged mirror tie",
    check_shape_sphere(true)
);
