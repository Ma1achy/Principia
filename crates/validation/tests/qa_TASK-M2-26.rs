//! QA tests for TASK-M2-26 on the CPU, written from the requirements (REQ-RENDER-025, REQ-ENC-002), not from the
//! implementation.
//!
//! - REQ-RENDER-025, inverse-encode contract Part 8 ("Legitimate bright regions: clamp boundaries, feasibility edges,
//!   the mirror tie — *tagged expected*") and Part 4 ("the clamp flags are the honest boundary")
//!   (`qa_m226_tags_follow_what_the_trip_reports`): the ROUNDTRIP class of a trip is read off what the trip itself
//!   reports. Over foreign physical states (random scale, offset, orientation and parity, including extreme mass ratios
//!   and momenta that push a control past its clamp), each encoded by the one encode (`kernel::encode::encode`, f32,
//!   default links): `Class::of` on the encode's notices word as the preset reads it (`tie_notices(notices, true)`,
//!   the whole word): where the report says `lookup_clamped` the class is the clamp tag; otherwise untagged. `C` has
//!   no mirror step since TASK-M2-57, so no trip reports a mirror: the retired `lookup_mirrored` bit of the notices word
//!   is set on no state, and no state is tagged mirror tie, the lower-half states (`λ̃_y < 0` after `C`, which the
//!   mirror step used to reflect) among them. Where either decode's tag word is any `DEGENERATE` reason, the class is
//!   the feasibility tag whatever the encode reports; a non-finite trip is invalid. Each of the clamped, plain and
//!   plain lower-half cases is reached. Control: a notices word with the retired bit set on every lower-half state, as
//!   the mirror step set it, which tags them mirror tie. *Was:* where the report said `lookup_mirrored` and not
//!   clamped, the mirror-tie tag (R-82's tie); control: a class function blind to the mirror bit.
//! - Inverse-encode contract Part 4 ("One norm for every round trip") and Part 1's T3 (`D(E(x)) = C(x)`)
//!   (`qa_m226_shape_sphere_lower_hemisphere_untagged_within_eps_phys`): the shape sphere chart decodes each pixel as
//!   its own system through the one decode path, a lower-hemisphere pixel kept below the axis (`λ̃_y < 0`), on `C`'s
//!   section like every chart. Over a grid of the chart's lower hemisphere (CPU twin, f32, default links), every
//!   sample that does not clamp, with no `DEGENERATE` tag, is below the axis, is untagged, and its residual, measured
//!   against `D(z)` itself, is within `ε_phys`: the trip returns the pixel's own system, not its mirror image. The two
//!   differ there (the residual against the mirror image of `D(z)` is past `ε_phys`), so the check bites. No sample
//!   anywhere on the sphere is tagged mirror tie or sets the retired bit. Control: the encode's `z_β` folded onto the
//!   upper half (`z_β → π − z_β`, the retired mirror step on `β`), which sends the lower hemisphere's trips off `D(z)`.
//!   *Was:* the lower hemisphere off `C`'s section, the encode's mirror `C`'s own, the residual against `C(D(z))`;
//!   control: the twin read as if on the section, tagging the lower hemisphere mirror tie.
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

/// `word` is the notices word the class reads, from the encode's own and whether the state is in the lower half
/// (`λ̃_y < 0` after `C`): the encode's own word in the test.
fn check_tags(word: fn(u32, bool) -> u32) {
    let mut rng = Rng(0x2026_0226);
    let (mut clamped, mut plain, mut plain_lower, mut lower) = (0, 0, 0, 0);
    for i in 0..4000u32 {
        let x = state(&mut rng, i % 4);
        let e = kernel::encode::encode::<Defaults, f32>(&x);
        let own = kernel::stage::encode::notices(&e.report);
        assert_eq!(
            own & roundtrip::MIRRORED,
            0,
            "state {i}: the encode set the retired lookup_mirrored bit"
        );
        let below = e.canonical.lambda_t_y < 0.0;
        lower += usize::from(below);
        let notices = word(own, below);
        let cl = e.report.lookup_clamped();
        let got = Class::of(true, 0, 0, roundtrip::tie_notices(notices, true));
        assert_ne!(
            got,
            Class::MirrorTie,
            "state {i}: tagged mirror tie, which no trip reports (λ̃_y = {:e})",
            e.canonical.lambda_t_y
        );
        if cl {
            clamped += 1;
            assert_eq!(got, Class::Clamp, "state {i}: clamped, not tagged clamp");
        } else {
            plain += 1;
            plain_lower += usize::from(below);
            assert_eq!(
                got,
                Class::Untagged,
                "state {i}: nothing reported, tagged {got:?}"
            );
        }
        // A DEGENERATE decode on either side is the feasibility tag, whatever the encode reported.
        for reason in 1..=5u32 {
            assert_eq!(
                Class::of(true, reason, 0, notices),
                Class::Feasibility,
                "state {i}: first decode tag {reason}"
            );
            assert_eq!(
                Class::of(true, 0, reason, notices),
                Class::Feasibility,
                "state {i}: free decode tag {reason}"
            );
        }
        assert_eq!(
            Class::of(false, 0, 0, notices),
            Class::Invalid,
            "state {i}: a non-finite trip"
        );
    }
    println!(
        "qa_m226_tags: clamped {clamped}, plain {plain} ({plain_lower} in the lower half), lower half {lower}"
    );
    assert!(clamped > 0, "no state reached a clamp");
    assert!(plain > 0, "no state was plain");
    assert!(
        plain_lower > 0 && plain_lower < plain,
        "the plain states do not reach both halves: {plain_lower} of {plain} in the lower half"
    );
}

#[test]
fn qa_m226_tags_follow_what_the_trip_reports() {
    check_tags(|notices, _| notices);
}

negative_control!(
    qa_m226_tags_follow_what_the_trip_reports,
    "a notices word with the retired mirror bit set on the lower-half states, as the mirror step set it, tags them",
    expected = "tagged mirror tie",
    check_tags(|notices, below| if below {
        notices | roundtrip::MIRRORED
    } else {
        notices
    })
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

// ── The shape sphere's lower hemisphere: the pixel's own system, not its mirror image ───────────────────────────────

/// `‖x − D(z)‖_phys` over `ε_phys`, `z` the encode of `x` at f32, default links; with `fold`, `z_β` folded onto the
/// upper half first where it lies in the lower (`z_β > π/2`, so `β = z_β + π/2 > π`): `z_β → π − z_β`, the mirror's
/// map on `β`, the retired mirror step.
fn trip(x: &Ic<f32>, fold: bool) -> f64 {
    let mut z = kernel::encode::encode::<Defaults, f32>(x).z;
    if fold && z.z[1] > std::f32::consts::FRAC_PI_2 {
        z.z[1] = std::f32::consts::PI - z.z[1];
    }
    let back = kernel::decode::decode::<Defaults, f32>(&z).ic;
    roundtrip::ratio(validation::roundtrip::phys(
        &validation::roundtrip::widen(x),
        &validation::roundtrip::widen(&back),
    ))
}

/// `x` reflected in the `x` axis, positions and momenta: its mirror image.
fn mirror_image(x: &Ic<f32>) -> Ic<f32> {
    Ic {
        m: x.m,
        r: x.r.map(|v| [v[0], -v[1]]),
        p: x.p.map(|v| [v[0], -v[1]]),
    }
}

fn check_shape_sphere(fold: bool) {
    use kernel::chart::ShapeSphere;
    use kernel::stage::decode::{sample_with, tag, UniformChart};
    use kernel::stage::fragment::FragmentChart;
    let side = 48u32;
    let scene =
        validation::decode_preset::Scene::new(DecodeChart::ShapeSphere, [1, 1], 1, 20, &[[0, 0]])
            .unwrap();
    let u = scene.uniforms;
    let chart = ShapeSphere::<f32>::from_uniform(&u.chart);
    let (mut lower, mut real_mirror) = (0usize, 0usize);
    for j in 0..side {
        for i in 0..side {
            let uv = [
                (i as f32 + 0.5) / side as f32,
                (j as f32 + 0.5) / side as f32,
            ];
            let out = sample_with::<ShapeSphere<f32>, Defaults, f32>(&chart, uv[0], uv[1], None);
            let first = tag(out.degenerate);
            // The twin as the preset reads the shape sphere.
            let t = validation::roundtrip_preset::twin_of(
                &out.ic,
                first,
                <ShapeSphere<f32> as FragmentChart>::ON_SECTION,
            );
            assert_ne!(
                t.class,
                Class::MirrorTie,
                "shape sphere sample {uv:?}: tagged mirror tie"
            );
            assert_eq!(
                t.notices & roundtrip::MIRRORED,
                0,
                "shape sphere sample {uv:?}: the retired lookup_mirrored bit set"
            );
            let clamped = t.notices & roundtrip::CLAMPED != 0;
            if uv[1] < 0.5 && !clamped && t.tags == 0 {
                lower += 1;
                let e = kernel::encode::encode::<Defaults, f32>(&out.ic);
                assert!(
                    e.canonical.lambda_t_y < 0.0,
                    "lower hemisphere sample {uv:?}: λ̃_y = {:e}, not kept below the axis",
                    e.canonical.lambda_t_y
                );
                assert_eq!(
                    t.class,
                    Class::Untagged,
                    "lower hemisphere sample {uv:?}: tagged"
                );
                let own = trip(&out.ic, fold);
                if !fold {
                    assert_eq!(
                        own.to_bits(),
                        t.ratio().to_bits(),
                        "lower hemisphere sample {uv:?}: the twin's residual is not the trip's against D(z)"
                    );
                }
                assert!(
                    own <= 1.0,
                    "lower hemisphere sample {uv:?}: {own} ε_phys against D(z), past ε_phys"
                );
                // The mirror image is another system: against it the trip is far off.
                let back = kernel::decode::decode::<Defaults, f32>(&e.z).ic;
                let against_image = roundtrip::ratio(validation::roundtrip::phys(
                    &validation::roundtrip::widen(&mirror_image(&out.ic)),
                    &validation::roundtrip::widen(&back),
                ));
                if against_image > 1.0 {
                    real_mirror += 1;
                }
            }
        }
    }
    println!(
        "qa_m226_shape_sphere: {lower} unclamped lower-hemisphere samples, {real_mirror} past ε_phys against the mirror image of D(z)"
    );
    assert!(
        lower * 4 > (side * side) as usize,
        "too few unclamped lower-hemisphere samples: {lower}"
    );
    assert!(
        real_mirror * 2 > lower,
        "the mirror image is not told from D(z): {real_mirror} of {lower}"
    );
}

#[test]
fn qa_m226_shape_sphere_lower_hemisphere_untagged_within_eps_phys() {
    check_shape_sphere(false);
}

negative_control!(
    qa_m226_shape_sphere_lower_hemisphere_untagged_within_eps_phys,
    "the encode's z_β folded onto the upper half, the retired mirror step, sends the lower hemisphere off D(z)",
    expected = "ε_phys against D(z), past ε_phys",
    check_shape_sphere(true)
);
