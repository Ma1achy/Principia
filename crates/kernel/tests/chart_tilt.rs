//! Tilt and slice on the five charts that hold their blocks by latent controls (`kernel::chart::tilt`; R-473, R-474,
//! R-507, R-544, R-554, R-613, R-633; chart_decoder_contract Part 4, § "Tilt (basis edit)"):
//! - `tilt_every_chart_*` (REQ-CHART-062's arms on the five charts, C2): untilted under both maps, no tilt path and a
//!   decode bitwise the chart's; `(a, b, d) = R·(u, v, 0)` to 1 ulp, `a`, `b` through the chart's own map and `d` on
//!   the control; the centre fixed under every tilt; the latent chart's basis edit; the tilted shape sphere bijective,
//!   each pixel counted once; RQ-471's corner `ray-degenerate` over the registry's five, unfolded, weighted 0, its
//!   shares undefined; the f32 instantiation.
//! - `tilt_mass_logits` (REQ-CHART-063, C7): the shape sphere tilted into each mass logit keeps every mass positive,
//!   the three summing to 1.
//! - `tilt_units` (REQ-CHART-064, R-507): the tilted axis's direction in the chart's parameters is the same at every
//!   zoom; the latent chart's tilt is the basis edit, unscaled.
//! - `slice_every_chart` (REQ-CHART-065's arms on the five charts, T6): a slice moves one hidden control, the decode at
//!   the centre following the link; never a stop on these charts.
//! - `tilted_extension` (REQ-CHART-066, X3): the types act on the tilted `(a, b)`.
//!
//! Each test registers its negative control (R-176).

use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Alpha, PRhoX};
use kernel::chart::shape_sphere::{self, pixel_centre, PixelCount};
use kernel::chart::tilt::{
    latent_basis, share, shift_controls, Hidden, LatentFrame, Shift, Tiltable,
};
use kernel::chart::{
    Chart, ChartDecode, Constraint, Float, InvariantLE, InvariantLK, Latent, Mixed, ShapeMap,
    ShapeSphere, SystemImage, Tilt, TiltMap, TiltSpec, Tilted, ValidationResult, REGISTRY,
};
use kernel::decode::{self, config, mass, momentum, Decoded};
use kernel::generated::links::{Defaults, LinkParams};
use validation::negative_control;

/// The five charts that hold their blocks by latent controls (R-633's split).
const FIVE: [&str; 5] = [
    "Latent",
    "ShapeSphere",
    "InvariantLE",
    "InvariantLK",
    "Mixed",
];
/// The view's centre in the chart's own parameters, off `(½, ½)` so a tilt's pivot is the view's.
const CENTRE: [f64; 2] = [0.37, 0.61];
/// The targets' slider spans: the Slice centre slider's range, `[−1, 1]`.
const SPAN: [f64; 2] = [2.0, 2.0];
/// The grid side of REQ-CHART-062's untilted arm.
const N: u32 = 33;

type Mx = Mixed<Alpha, PRhoX, f64>;

/// `(sin, cos)` of `deg` degrees through the kernel's own float trait, as the tilt computes them.
fn sc(deg: f64) -> (f64, f64) {
    Float::sin_cos(Float::to_radians(deg))
}

/// The decode's bits, as the Debug form of every float.
fn bits(d: &Decoded<f64>) -> String {
    format!("{d:?}")
}

/// A stored tilt.
fn spec(tau1: f64, tau2: f64, map: TiltMap, x: u32, y: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map,
        x,
        y,
    }
}

/// The pixel centres of an `n` × `n` grid over `[lo, hi]²`.
fn grid(n: u32, lo: f64, hi: f64) -> Vec<[f64; 2]> {
    let mut points = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let at = |k| lo + (hi - lo) * pixel_centre(k, n);
            points.push([at(i), at(j)]);
        }
    }
    points
}

fn sphere(map: ShapeMap) -> ShapeSphere<f64> {
    ShapeSphere {
        map,
        n0: [1.0, 0.0, 0.0],
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.3, -0.4],
        z_mom: [0.2, -0.1, 0.15, 0.05],
    }
}

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.3, -0.5],
        z_mass: [0.7, -0.4],
        k_max: 1.0,
        gamma_k: 1.5,
    }
}

fn mixed() -> Mx {
    Mixed::new(
        [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4],
        [0.05, 1.5],
        [-1.8, 1.8],
    )
}

/// The latent chart's frame: the plane `z_q0 × z_q1`, zoom ½, no rotation.
const FRAME: LatentFrame = LatentFrame {
    a: 2,
    b: 3,
    scale: 0.5,
    gamma: 0.0,
};
const Z0: [f64; 8] = [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4];

/// The latent chart of `FRAME` under `spec`.
fn latent(spec: &TiltSpec) -> Latent<f64> {
    let (q1, q2) = latent_basis(&FRAME, spec).expect("a target off the plane");
    Latent { z0: Z0, q1, q2 }
}

/// Runs `$body` on each of the four curved charts, `$c` the chart and `($x, $y)` two of its hidden coordinates.
macro_rules! for_curved {
    (|$c:ident, $x:ident, $y:ident| $body:block) => {{
        {
            let $c = sphere(ShapeMap::Spherical);
            let ($x, $y) = (2u32, 7u32);
            $body
        }
        {
            let $c = sphere(ShapeMap::Exponential);
            let ($x, $y) = (3u32, 6u32);
            $body
        }
        {
            let $c = InvariantLE(invariant());
            let ($x, $y) = (6u32, 0u32);
            $body
        }
        {
            let $c = InvariantLK(invariant());
            let ($x, $y) = (1u32, 7u32);
            $body
        }
        {
            let $c = mixed();
            let ($x, $y) = (4u32, 1u32);
            $body
        }
    }};
}

/// `chart` under `spec` about `CENTRE`.
fn tilted<C: Tiltable<Param = f64>>(chart: C, spec: &TiltSpec) -> Tilted<C, f64> {
    Tilted::new(chart, Tilt::new(spec, CENTRE, SPAN)).expect("hidden targets")
}

/// The held value of the hidden control `k`.
fn held<C: Chart<Param = f64>>(chart: &C, k: u32) -> f64 {
    chart.map(0.5, 0.5).z.z[k as usize]
}

// ── C2: untilted, no tilt path ────────────────────────────────────────────────────────────────────────────────────────

/// At `τ = 0` under both maps: no tilt path, and the decode, `map` and `validate` the chart's own at every pixel of a
/// 33 × 33 grid. `identity_path` forces the tilt path on with its identity columns (the control).
fn check_untilted<C: Tiltable<Param = f64> + ChartDecode + Copy>(
    chart: C,
    x: u32,
    identity_path: bool,
) {
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        let mut view = tilted(chart, &spec(0.0, 0.0, map, x, x));
        assert!(
            !view.tilt().on || identity_path,
            "an untilted view takes a tilt path"
        );
        if identity_path {
            view = Tilted::new(
                chart,
                Tilt {
                    on: true,
                    ..*view.tilt()
                },
            )
            .unwrap();
        }
        for [u, v] in grid(N, 0.0, 1.0) {
            let own = chart.decode::<Defaults, f64>(u, v);
            assert!(
                bits(&view.decode::<Defaults, f64>(u, v)) == bits(&own)
                    && bits(&view.decode_at::<Defaults, f64>(u, v)) == bits(&own)
                    && bits(&view.decode_at_params::<Defaults, f64>(u, v, &LinkParams::DEFAULT))
                        == bits(&own)
                    && format!("{:?}", view.map::<f64>(u, v))
                        == format!("{:?}", chart.map::<f64>(u, v))
                    && view.validate(u, v) == chart.validate(u, v),
                "{} at ({u}, {v}) is not bitwise the untilted chart's",
                chart.name()
            );
        }
    }
}

#[test]
fn tilt_every_chart_untilted_bitwise() {
    for_curved!(|c, x, _y| { check_untilted(c, x, false) });
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        let (q1, q2) = latent_basis(&FRAME, &spec(0.0, 0.0, map, 4, 4)).unwrap();
        let mut e_a = [0.0; 8];
        let mut e_b = [0.0; 8];
        e_a[2] = 0.5;
        e_b[3] = 0.5;
        assert_eq!(
            (q1, q2),
            (e_a, e_b),
            "the latent basis at τ = 0 is not the untilted plane's"
        );
    }
}

negative_control!(
    tilt_every_chart_untilted_bitwise,
    "an untilted path that multiplies by an identity R is not bitwise, so it must fail",
    expected = "is not bitwise the untilted chart's",
    {
        check_untilted(sphere(ShapeMap::Spherical), 2, true);
    }
);

// ── T2: (a, b, d) = R·(u, v, 0), a and b through the chart's map, d on the control ──────────────────────────────────

/// How a pixel's decode is formed for [`check_moves`]: the tilt path, or the control's `d` on the physical momentum.
#[derive(Clone, Copy, PartialEq)]
enum Offset {
    Control,
    Physical,
}

/// A 30° tilt `τ₁` into `x`: each pixel's `(a, b, d)` is `R·(ũ, ṽ, 0)` scaled back, to 1 ulp, and its decode is the
/// chart's own at `(a, b)` with the control `x` at its held value plus `d`, bitwise.
fn check_moves<C: Tiltable<Param = f64> + Copy>(chart: C, x: u32, offset: Offset) {
    let view = tilted(chart, &spec(30.0, 0.0, TiltMap::Sheet, x, x));
    let (s, c) = sc(30.0);
    let ulp = |want: f64, got: f64| {
        (want - got).abs() <= f64::EPSILON * want.abs().max(f64::MIN_POSITIVE)
    };
    for [u, v] in grid(17, 0.0, 1.0) {
        let ([a, b], shift) = view.tilt().place(u, v);
        let (du, dv) = (u - CENTRE[0], v - CENTRE[1]);
        assert!(
            ulp(CENTRE[0] + c * du, a) && ulp(CENTRE[1] + dv, b) && ulp(SPAN[0] * s * du, shift.dx),
            "(a, b, d) is not R·(u, v, 0)"
        );
        let mut got = view.decode::<Defaults, f64>(u, v);
        if offset == Offset::Physical {
            got = chart.decode::<Defaults, f64>(a, b);
            got.ic.p[1][0] += shift.dx;
        }
        let want = chart
            .sliced(x, held(&chart, x) + shift.dx)
            .unwrap()
            .decode::<Defaults, f64>(a, b);
        assert!(
            bits(&got) == bits(&want),
            "{} at ({u}, {v}) is not the chart's decode at (a, b) with d on its control",
            chart.name()
        );
    }
}

#[test]
fn tilt_every_chart_moves_a_b_d() {
    for_curved!(|c, x, _y| { check_moves(c, x, Offset::Control) });
}

negative_control!(
    tilt_every_chart_moves_a_b_d,
    "a d added to a physical value for a latent target is not the control's tilt, so it must fail",
    expected = "with d on its control",
    {
        check_moves(InvariantLE(invariant()), 6, Offset::Physical);
    }
);

/// The tilted shape sphere's shape at each pixel is the chart map's at its `(a, b)`: `tangent` takes it instead from
/// the tangent plane at the centre (the control), which drifts off the map away from the centre.
fn check_through_map(tangent: bool) {
    let chart = sphere(ShapeMap::Spherical);
    let view = tilted(chart, &spec(30.0, 20.0, TiltMap::Sheet, 2, 2));
    let masses = mass::decode::<Defaults, f64>(chart.z_mass);
    let h = 1e-6;
    let n_c = chart.point::<f64>(CENTRE[0], CENTRE[1]);
    let du = |k: usize| {
        (chart.point::<f64>(CENTRE[0] + h, CENTRE[1])[k]
            - chart.point::<f64>(CENTRE[0] - h, CENTRE[1])[k])
            / (2.0 * h)
    };
    let dv = |k: usize| {
        (chart.point::<f64>(CENTRE[0], CENTRE[1] + h)[k]
            - chart.point::<f64>(CENTRE[0], CENTRE[1] - h)[k])
            / (2.0 * h)
    };
    for [u, v] in grid(17, 0.0, 1.0) {
        let ([a, b], _) = view.at(u, v);
        let got = if tangent {
            let (da, db) = (a - CENTRE[0], b - CENTRE[1]);
            let n = [0, 1, 2].map(|k| n_c[k] + du(k) * da + dv(k) * db);
            let norm = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            shape_sphere::place(n.map(|x| x / norm), 1.0, 0.0, &masses)
        } else {
            view.decode::<Defaults, f64>(u, v).config
        };
        let want = shape_sphere::place(chart.point::<f64>(a, b), 1.0, 0.0, &masses);
        let close = (0..2).all(|k| {
            (got.rho_t[k] - want.rho_t[k]).abs() <= 1e-12
                && (got.lambda_t[k] - want.lambda_t[k]).abs() <= 1e-12
        });
        assert!(
            close,
            "the shape at ({u}, {v}) is not the chart map's at (a, b)"
        );
    }
}

#[test]
fn tilt_every_chart_through_the_chart_map() {
    check_through_map(false);
}

negative_control!(
    tilt_every_chart_through_the_chart_map,
    "a tilt applied to a tangent vector at the centre drifts off the chart map's axes, so it must fail",
    expected = "is not the chart map's at (a, b)",
    {
        check_through_map(true);
    }
);

/// The stored tilts the anchor arm runs: single, both angles under each map, over the pole, two targets, the corner.
fn tilts(x: u32, y: u32) -> [TiltSpec; 6] {
    [
        spec(30.0, 0.0, TiltMap::Sheet, x, x),
        spec(40.0, 70.0, TiltMap::Sheet, x, x),
        spec(150.0, -170.0, TiltMap::Sheet, x, x),
        spec(40.0, 70.0, TiltMap::Lean, x, x),
        spec(30.0, -45.0, TiltMap::Sheet, x, y),
        spec(90.0, -90.0, TiltMap::Lean, x, x),
    ]
}

/// The centre pixel decodes as the untilted chart's centre under every tilt; `pivot` turns about `(½, ½)` instead of
/// the view's centre (the control).
fn check_anchor<C: Tiltable<Param = f64> + Copy>(chart: C, x: u32, y: u32, pivot: [f64; 2]) {
    let own = chart.decode::<Defaults, f64>(CENTRE[0], CENTRE[1]);
    for s in tilts(x, y) {
        let view = Tilted::new(chart, Tilt::new(&s, pivot, SPAN)).unwrap();
        assert!(
            bits(&view.decode::<Defaults, f64>(CENTRE[0], CENTRE[1])) == bits(&own),
            "{} moved its centre under {s:?}",
            chart.name()
        );
    }
}

#[test]
fn tilt_every_chart_anchor_preserving() {
    for_curved!(|c, x, y| { check_anchor(c, x, y, CENTRE) });
    for s in tilts(4, 5) {
        let chart = latent(&s);
        assert_eq!(
            chart.map::<f64>(0.5, 0.5).z.z,
            Z0,
            "the latent centre moved under {s:?}"
        );
    }
}

negative_control!(
    tilt_every_chart_anchor_preserving,
    "a tilt about the square's middle, not the view's centre, moves the centre, so it must fail",
    expected = "moved its centre",
    {
        check_anchor(mixed(), 4, 1, [0.5, 0.5]);
    }
);

type Make = fn(ShapeSphere<f64>, Tilt<f64>) -> Option<Tilted<ShapeSphere<f64>, f64>>;

/// A tilt into a coordinate that is not hidden is refused, by `make` (the control admits it as untilted).
fn check_targets(make: Make) {
    let off = spec(30.0, 0.0, TiltMap::Sheet, 0, 0);
    let second = spec(30.0, 10.0, TiltMap::Sheet, 2, 0);
    for s in [off, second] {
        let view = make(sphere(ShapeMap::Spherical), Tilt::new(&s, CENTRE, SPAN));
        assert!(
            view.is_none(),
            "a tilt into an axis coordinate admitted: {s:?}"
        );
    }
}

/// A tilt into a coordinate that is not hidden is refused; an untilted view takes any; each chart's hidden set.
#[test]
fn tilt_every_chart_targets_hidden() {
    check_targets(Tilted::new);
    let flat = spec(0.0, 0.0, TiltMap::Sheet, 0, 0);
    assert!(Tilted::new(sphere(ShapeMap::Spherical), Tilt::new(&flat, CENTRE, SPAN)).is_some());
    for (k, hidden) in [
        (0, false),
        (1, false),
        (2, true),
        (5, true),
        (6, true),
        (7, true),
        (8, false),
    ] {
        assert_eq!(
            sphere(ShapeMap::Spherical).hidden(k),
            hidden,
            "shape sphere {k}"
        );
    }
    for (k, hidden) in [
        (0, true),
        (1, true),
        (2, false),
        (5, false),
        (6, true),
        (7, true),
        (8, false),
    ] {
        assert_eq!(InvariantLE(invariant()).hidden(k), hidden, "(L_z, E) {k}");
        assert_eq!(InvariantLK(invariant()).hidden(k), hidden, "(L_z, K) {k}");
    }
    for (k, hidden) in [
        (0, false),
        (1, true),
        (2, false),
        (3, true),
        (7, true),
        (8, false),
    ] {
        assert_eq!(mixed().hidden(k), hidden, "mixed {k}");
    }
    for (k, hidden) in [
        (1, true),
        (2, false),
        (3, false),
        (4, true),
        (7, true),
        (8, false),
    ] {
        assert_eq!(FRAME.hidden(k), hidden, "latent {k}");
    }
    let mut wide = FRAME;
    wide.a = 8;
    assert!(latent_basis(&wide, &spec(30.0, 0.0, TiltMap::Sheet, 4, 4)).is_none());
    wide = LatentFrame { b: 8, ..FRAME };
    assert!(latent_basis(&wide, &spec(30.0, 0.0, TiltMap::Sheet, 4, 4)).is_none());
    wide = LatentFrame { b: 2, ..FRAME };
    assert!(latent_basis(&wide, &spec(30.0, 0.0, TiltMap::Sheet, 4, 4)).is_none());
    assert!(latent_basis(&FRAME, &spec(30.0, 0.0, TiltMap::Sheet, 2, 2)).is_none());
    assert!(latent_basis(&FRAME, &spec(30.0, 0.0, TiltMap::Sheet, 4, 3)).is_none());
}

negative_control!(
    tilt_every_chart_targets_hidden,
    "a tilt into a coordinate the axes write is not a hidden coordinate's, so it must fail",
    expected = "a tilt into an axis coordinate admitted",
    {
        check_targets(|chart, tilt| Tilted::new(chart, Tilt { on: false, ..tilt }));
    }
);

/// A tilt into a mass logit, either target's, makes the masses vary per pixel, as `read` reports it.
fn check_mass_flag(read: fn(&Tilted<ShapeSphere<f64>, f64>) -> bool) {
    for (x, y, varies) in [(6, 6, true), (2, 7, true), (7, 2, true), (2, 3, false)] {
        let view = tilted(
            sphere(ShapeMap::Spherical),
            &spec(30.0, 20.0, TiltMap::Sheet, x, y),
        );
        assert_eq!(read(&view), varies, "per-pixel mass, targets ({x}, {y})");
    }
}

/// The tilted chart's other entries: its name, its energy flag, its per-pixel mass, its `map` and its `validate` on
/// the tilted `(a, b)`.
#[test]
fn tilt_every_chart_entries() {
    let le = tilted(
        InvariantLE(invariant()),
        &spec(30.0, 0.0, TiltMap::Sheet, 0, 0),
    );
    assert_eq!(le.name(), "InvariantLE");
    assert!(le.forbids_energy_normalisation());
    assert!(!tilted(mixed(), &spec(30.0, 0.0, TiltMap::Sheet, 4, 4)).forbids_energy_normalisation());
    assert!(
        !le.requires_per_pixel_mass(),
        "a configuration tilt varies no mass"
    );
    check_mass_flag(|view| view.requires_per_pixel_mass());
    let one = Tilt::new(&spec(30.0, 20.0, TiltMap::Sheet, 2, 2), CENTRE, SPAN);
    let view = Tilted::new(sphere(ShapeMap::Spherical), Tilt { y: 7, ..one }).unwrap();
    assert!(
        !view.requires_per_pixel_mass(),
        "one target's y is not a second target"
    );
    let flat = tilted(
        sphere(ShapeMap::Spherical),
        &spec(0.0, 0.0, TiltMap::Sheet, 6, 6),
    );
    assert!(
        !flat.requires_per_pixel_mass(),
        "an untilted view tilts no mass"
    );
    assert!(tilted(mixed(), &spec(0.0, 0.0, TiltMap::Sheet, 4, 4)).chart() == &mixed());
    let two = tilted(
        sphere(ShapeMap::Spherical),
        &spec(30.0, 20.0, TiltMap::Sheet, 2, 6),
    );
    let ([a, b], shift) = two.tilt().place(0.9, 0.2);
    let want = shift_controls(sphere(ShapeMap::Spherical).map::<f64>(a, b).z.z, &shift);
    assert_eq!(
        two.map::<f64>(0.9, 0.2).z.z,
        want,
        "map is not the chart's at (a, b), shifted"
    );
    assert_ne!(want, sphere(ShapeMap::Spherical).map::<f64>(a, b).z.z);
    // Past the edge on the invariant chart: the chart's own check on (a, b), a failure refused.
    let past = tilted(
        InvariantLK(invariant()),
        &spec(30.0, 0.0, TiltMap::Sheet, 0, 0),
    );
    for (u, v, valid) in [
        (0.5, 0.5, true),
        (-0.4, 0.5, false),
        (0.5, -0.2, false),
        (1.6, 0.5, false),
    ] {
        let got = past.validate(u, v);
        assert_eq!(
            got == ValidationResult::Valid,
            valid,
            "validate at ({u}, {v}): {got:?}"
        );
        if !valid {
            assert!(matches!(got, ValidationResult::Reject { .. }), "{got:?}");
        }
    }
    assert_eq!(
        past.validate(0.5, -0.2),
        ValidationResult::Reject {
            failed: Constraint::KNonNegative
        }
    );
    let clamp = tilted(mixed(), &spec(30.0, 0.0, TiltMap::Sheet, 4, 4));
    assert!(matches!(
        clamp.validate(-1.0, 0.5),
        ValidationResult::Reject {
            failed: Constraint::AxisDomain
        }
    ));
    assert!(matches!(
        clamp.validate(-1e300, 0.5),
        ValidationResult::Reject { .. }
    ));
    assert_eq!(clamp.validate(0.5, 0.5), ValidationResult::Valid);
}

negative_control!(
    tilt_every_chart_entries,
    "a mass tilt read as the untilted chart's held masses varies none, so it must fail",
    expected = "per-pixel mass",
    {
        check_mass_flag(|view| view.chart().requires_per_pixel_mass());
    }
);

// ── The latent chart: the basis edit ──────────────────────────────────────────────────────────────────────────────────

/// `|q|`, `q̂₁·q̂₂`, and each `q̂`'s components.
fn unit(q: [f64; 8]) -> [f64; 8] {
    let n = q.iter().map(|x| x * x).sum::<f64>().sqrt();
    q.map(|x| x / n)
}

fn dot(p: [f64; 8], q: [f64; 8]) -> f64 {
    (0..8).map(|i| p[i] * q[i]).sum()
}

/// The latent chart's tilt is the basis edit: a single tilt under Lean each axis is `σ(cos τ e_a + sin τ e_X)`,
/// bitwise, the same under Tip as sheet; two targets each axis toward its own; Tip as sheet at 45°/45° R-474 R9's
/// readout; `m` and its wrap bitwise equal; Tip as sheet orthonormal past 90°, and Lean each axis not (`lean`, the
/// control, checks Lean's skew as Tip as sheet's).
fn check_basis_edit(lean_as_sheet: bool) {
    let (s, c) = sc(30.0);
    let mut want = [0.0; 8];
    want[2] = c * 0.5;
    want[4] = s * 0.5;
    for map in [TiltMap::Lean, TiltMap::Sheet] {
        let (q1, q2) = latent_basis(&FRAME, &spec(30.0, 0.0, map, 4, 4)).unwrap();
        assert_eq!(q1, want, "the single tilt is not the basis edit");
        assert_eq!(q2[3], 0.5);
        let chart = latent(&spec(30.0, 0.0, map, 4, 4));
        let edit = Latent {
            z0: Z0,
            q1: want,
            q2,
        };
        for [u, v] in grid(9, -0.5, 1.5) {
            assert_eq!(
                bits(&chart.decode::<Defaults, f64>(u, v)),
                bits(&edit.decode::<Defaults, f64>(u, v)),
                "the tilted latent chart is not the basis edit's"
            );
        }
    }
    let (q1, q2) = latent_basis(&FRAME, &spec(30.0, -45.0, TiltMap::Sheet, 4, 5)).unwrap();
    let (s2, c2) = sc(-45.0);
    assert_eq!(
        (q1[2], q1[4], q1[5]),
        (c * 0.5, s * 0.5, 0.0),
        "two targets: q₁"
    );
    assert_eq!(
        (q2[3], q2[4], q2[5]),
        (c2 * 0.5, 0.0, s2 * 0.5),
        "two targets: q₂"
    );
    let sheet = latent_basis(&FRAME, &spec(45.0, 45.0, TiltMap::Sheet, 4, 4)).unwrap();
    let h = unit(sheet.0);
    let readout = [(2, 0.722), (3, -0.278), (4, 0.634)];
    assert!(
        readout.iter().all(|&(i, x)| (h[i] - x).abs() < 5e-4),
        "R-474 R9's H at 45°/45°: {h:?}"
    );
    let lean = latent_basis(&FRAME, &spec(45.0, 45.0, TiltMap::Lean, 4, 4)).unwrap();
    assert_ne!(sheet, lean, "the two maps agree with both angles set");
    let wrapped = 200f64.hypot(150.0);
    let k = (wrapped - 360.0) / wrapped;
    assert_eq!(
        latent_basis(&FRAME, &spec(200.0, 150.0, TiltMap::Sheet, 4, 4)),
        latent_basis(&FRAME, &spec(200.0 * k, 150.0 * k, TiltMap::Sheet, 4, 4)),
        "m and its wrap differ"
    );
    for t1 in [-170.0, -95.0, -30.0, 20.0, 60.0, 100.0, 135.0, 180.0] {
        for t2 in [-180.0, -120.0, -45.0, 15.0, 80.0, 110.0, 160.0] {
            let map = if lean_as_sheet {
                TiltMap::Lean
            } else {
                TiltMap::Sheet
            };
            let (q1, q2) = latent_basis(&FRAME, &spec(t1, t2, map, 4, 4)).unwrap();
            let n = |q: [f64; 8]| dot(q, q).sqrt() / 0.5;
            assert!(
                (n(q1) - 1.0).abs() < 1e-14
                    && (n(q2) - 1.0).abs() < 1e-14
                    && dot(q1, q2).abs() < 1e-14,
                "Tip as sheet is not orthonormal at ({t1}, {t2})"
            );
        }
    }
}

#[test]
fn tilt_every_chart_latent_basis_edit() {
    check_basis_edit(false);
}

negative_control!(
    tilt_every_chart_latent_basis_edit,
    "Lean each axis is skewed off the axes, so read as Tip as sheet it must fail",
    expected = "is not orthonormal",
    {
        check_basis_edit(true);
    }
);

// ── The tilted shape sphere: bijective, every pixel counted once (R-544, R-613) ───────────────────────────────────────

/// The shape sphere's tilted arms: into a momentum control at `τ₂ = 30°`, at `τ₁ = 30°`, Tip as sheet at 108°/144°
/// (`m = 180°`) and Lean each axis at 90°/30°.
fn sphere_arms() -> [TiltSpec; 4] {
    [
        spec(0.0, 30.0, TiltMap::Sheet, 2, 2),
        spec(30.0, 0.0, TiltMap::Sheet, 2, 2),
        spec(108.0, 144.0, TiltMap::Sheet, 2, 2),
        spec(90.0, 30.0, TiltMap::Lean, 2, 2),
    ]
}

/// Each arm on the spherical map and on the exponential map's cap straddling the equator reports bijective and counts
/// every pixel of the tilted view once: `fold` counts a pixel whose tilted `b` lies below ½ with its mirror (the
/// control).
fn check_one_to_one(fold: bool) {
    let n = 41;
    for map in [ShapeMap::Spherical, ShapeMap::Exponential] {
        for s in sphere_arms() {
            let view = Tilted::new(sphere(map), Tilt::new(&s, [0.5, 0.5], SPAN)).unwrap();
            assert_eq!(
                view.system_image(),
                SystemImage::Bijective,
                "{map:?} under {s:?}"
            );
            // A window zoomed by 4 about the centre: every tilted (a, b) lies in the square and in the cap.
            let window = grid(n, 0.375, 0.625);
            let mut total = view.counted_total(window.iter().copied());
            if fold {
                total -= window
                    .iter()
                    .filter(|&&[u, v]| view.at(u, v).0[1] < 0.5)
                    .count() as u64;
            }
            assert!(
                total == u64::from(n * n),
                "{map:?} under {s:?}: the count is not N²"
            );
            // The whole square: each pixel whose tilted (a, b) stays in it counted once.
            let square = grid(n, 0.0, 1.0);
            let inside = square
                .iter()
                .filter(|&&[u, v]| {
                    let [a, b] = view.at(u, v).0;
                    (0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b)
                })
                .count() as u64;
            assert_eq!(
                view.counted_total(square.iter().copied()),
                inside,
                "{map:?} under {s:?}"
            );
        }
    }
}

#[test]
fn tilt_every_chart_shape_sphere_one_to_one() {
    check_one_to_one(false);
    // The arm at m = 180° does take pixels past the square, which this count leaves to the axes' types.
    let view = Tilted::new(
        sphere(ShapeMap::Spherical),
        Tilt::new(&sphere_arms()[2], [0.5, 0.5], SPAN),
    )
    .unwrap();
    assert_eq!(
        view.pixel(0.02, 0.98),
        None,
        "a pixel past the square counted"
    );
    assert_eq!(view.pixel(0.5, 0.5), Some(PixelCount::System));
    let cap = Tilted::new(
        sphere(ShapeMap::Exponential),
        Tilt::new(&sphere_arms()[2], [0.5, 0.5], SPAN),
    )
    .unwrap();
    assert_eq!(cap.pixel(0.02, 0.98), Some(PixelCount::DomainEnd));
    // A cap past r = π: the antipode's pixels count once together, the domain's end not at all.
    let mut wide = sphere(ShapeMap::Exponential);
    wide.scale = std::f64::consts::PI;
    let view = Tilted::new(
        wide,
        Tilt::new(&spec(30.0, 0.0, TiltMap::Sheet, 2, 2), [0.5, 0.5], SPAN),
    )
    .unwrap();
    let ([a, _], _) = view.tilt().place(1.0, 0.5);
    assert_eq!(wide.exp_pixel(a, 0.5), PixelCount::System);
    let past = view.pixel(0.02, 0.02);
    assert_eq!(past, Some(PixelCount::DomainEnd), "{past:?}");
    let pts = [[0.5, 1.0], [0.5, 1.0], [0.5, 0.5], [0.02, 0.02]];
    assert_eq!(view.pixel(0.5, 1.0), Some(PixelCount::Antipode));
    assert_eq!(
        view.counted_total(pts),
        2,
        "the antipode counted once, the domain's end not at all"
    );
    let flat = Tilted::new(
        sphere(ShapeMap::Spherical),
        Tilt::new(&spec(0.0, 0.0, TiltMap::Sheet, 2, 2), CENTRE, SPAN),
    )
    .unwrap();
    assert_eq!(
        flat.at(1.5, -0.25).0,
        [1.5, -0.25],
        "an untilted pixel is its own point"
    );
    assert_eq!(flat.pixel(1.5, 0.5), None);
}

negative_control!(
    tilt_every_chart_shape_sphere_one_to_one,
    "a fold kept on the tilted shape sphere undercounts, so it must fail",
    expected = "the count is not N²",
    {
        check_one_to_one(true);
    }
);

// ── RQ-471's corner over the registry's five (R-554) ──────────────────────────────────────────────────────────────────

/// What a chart reports for a stored tilt: its `system_image`, each pixel's area weight and a share.
struct Report {
    image: SystemImage,
    weights: Vec<f64>,
}

/// The registry's chart `name`, one of the five, tilted by `s` (targets its own hidden coordinates), its untilted
/// `system_image` `untilted` where the chart declares none.
fn report(name: &str, s: &TiltSpec, untilted: SystemImage) -> Report {
    let pixels = grid(N, 0.0, 1.0);
    let curved = |image: SystemImage, weight: f64| Report {
        image,
        weights: vec![weight; pixels.len()],
    };
    let with = |x: u32| TiltSpec { x, y: x, ..*s };
    match name {
        "Latent" => Report {
            image: s.system_image(untilted),
            weights: pixels.iter().map(|_| with(4).area_weight()).collect(),
        },
        "ShapeSphere" => {
            let view = tilted(sphere(ShapeMap::Spherical), &with(2));
            curved(view.system_image(), view.tilt().weight)
        }
        "InvariantLE" => {
            let view = tilted(InvariantLE(invariant()), &with(6));
            curved(view.tilt().system_image(untilted), view.tilt().weight)
        }
        "InvariantLK" => {
            let view = tilted(InvariantLK(invariant()), &with(0));
            curved(view.tilt().system_image(untilted), view.tilt().weight)
        }
        "Mixed" => {
            let view = tilted(mixed(), &with(5));
            curved(view.tilt().system_image(untilted), view.tilt().weight)
        }
        other => panic!("{other} is not one of the five"),
    }
}

/// How the corner arm's report is altered for a control.
#[derive(Clone, Copy, PartialEq)]
enum Corner {
    AsBuilt,
    Folded,
    ShareZero,
    Bijective,
    Weighted,
}

/// Under Lean each axis with one target at 90°/90° and −90°/90°, over the registry's five: `ray-degenerate`, no fold,
/// every pixel's area weight 0, every share undefined.
fn check_corner(alter: Corner) {
    let five: Vec<&str> = REGISTRY
        .iter()
        .copied()
        .filter(|n| FIVE.contains(n))
        .collect();
    assert_eq!(five, FIVE, "the registry's five");
    for (t1, t2) in [(90.0, 90.0), (-90.0, 90.0)] {
        let s = spec(t1, t2, TiltMap::Lean, 0, 0);
        for name in &five {
            let mut r = report(name, &s, SystemImage::Bijective);
            if alter == Corner::Folded {
                r.image = SystemImage::NToOne(2);
            } else if alter == Corner::Bijective {
                r.image = SystemImage::Bijective;
            } else if alter == Corner::Weighted {
                r.weights.iter_mut().for_each(|w| *w = 1.0);
            }
            assert!(
                r.image == SystemImage::RayDegenerate,
                "{name} at ({t1}, {t2}) reports {} or applies a fold",
                r.image
            );
            assert!(
                r.weights.iter().all(|&w| w == 0.0),
                "{name}: a non-zero area weight"
            );
            let total: f64 = r.weights.iter().sum();
            let half: f64 = r.weights.iter().step_by(2).sum();
            let got = if alter == Corner::ShareZero {
                Some(0.0)
            } else {
                share(half, total)
            };
            assert!(got.is_none(), "{name}: a share reported as {got:?}");
        }
    }
}

#[test]
fn tilt_every_chart_ray_degenerate_corner() {
    check_corner(Corner::AsBuilt);
    // Recomputed on an edit: one degree off, two targets, or Tip as sheet, the untilted value; τ = 0 likewise.
    for s in [
        spec(89.0, 90.0, TiltMap::Lean, 2, 2),
        spec(90.0, -89.0, TiltMap::Lean, 2, 2),
        spec(90.0, 90.0, TiltMap::Lean, 2, 7),
        spec(90.0, 90.0, TiltMap::Sheet, 2, 2),
        spec(0.0, 0.0, TiltMap::Lean, 2, 2),
    ] {
        assert_eq!(
            s.system_image(SystemImage::NToOne(3)),
            SystemImage::NToOne(3),
            "{s:?}"
        );
        let view = tilted(sphere(ShapeMap::Spherical), &s);
        assert_eq!(view.system_image(), SystemImage::Bijective, "{s:?}");
        assert!(view.tilt().weight > 0.0, "{s:?}");
    }
    assert_eq!(share(1.0, 4.0), Some(0.25));
    assert_eq!(share(0.0, 4.0), Some(0.0));
    // The Gram factor under Lean each axis with one target, 1 under Tip as sheet and with two targets.
    let gram = spec(30.0, 30.0, TiltMap::Lean, 2, 2).area_weight();
    assert!(
        (gram - (1.0f64 - 0.0625).sqrt()).abs() < 1e-15,
        "Lean's Gram factor {gram}"
    );
    for (t1, t2) in [
        (40.0, 70.0),
        (-25.0, 80.0),
        (60.0, -15.0),
        (89.0, 89.0),
        (0.0, 50.0),
    ] {
        // √(1 − sin²τ₁ sin²τ₂), written as √(cos²τ₁ + sin²τ₁ cos²τ₂) to keep its precision near the corner.
        let ((s1, c1), c2) = (sc(t1), sc(t2).1);
        let want = Float::sqrt(c1 * c1 + s1 * s1 * c2 * c2);
        let got = spec(t1, t2, TiltMap::Lean, 2, 2).area_weight();
        assert!(
            (got - want).abs() < 1e-15,
            "Lean's Gram factor at ({t1}, {t2}): {got}, not {want}"
        );
    }
    assert_eq!(spec(30.0, 30.0, TiltMap::Sheet, 2, 2).area_weight(), 1.0);
    for (t1, t2) in [(40.0, 70.0), (45.0, 45.0), (10.0, 20.0), (13.0, 77.0)] {
        let sheet = spec(t1, t2, TiltMap::Sheet, 2, 2).area_weight();
        assert_eq!(
            sheet, 1.0,
            "Tip as sheet needs no Gram factor at ({t1}, {t2})"
        );
        let two = spec(t1, t2, TiltMap::Lean, 2, 3).area_weight();
        assert_eq!(two, 1.0, "two targets need no Gram factor at ({t1}, {t2})");
    }
    assert_eq!(spec(30.0, 30.0, TiltMap::Lean, 2, 3).area_weight(), 1.0);
    assert_eq!(spec(90.0, 90.0, TiltMap::Lean, 2, 3).area_weight(), 1.0);
    assert!(!spec(90.0, 90.0, TiltMap::Sheet, 2, 2).rank_loss());
    assert!(!spec(90.0, 90.0, TiltMap::Lean, 2, 3).rank_loss());
    assert!(!spec(90.0, 45.0, TiltMap::Lean, 2, 2).rank_loss());
    assert!(!spec(45.0, 90.0, TiltMap::Lean, 2, 2).rank_loss());
    assert!(spec(-90.0, -90.0, TiltMap::Lean, 2, 2).rank_loss());
}

negative_control!(
    tilt_every_chart_ray_degenerate_corner,
    "a fold applied at the corner reports n-to-1, so it must fail",
    expected = "applies a fold",
    {
        check_corner(Corner::Folded);
    }
);

mod corner_share_zero {
    use super::*;
    negative_control!(
        tilt_every_chart_ray_degenerate_corner,
        "a share reported as 0 at the corner, not undefined, must fail",
        expected = "a share reported as Some(0.0)",
        {
            check_corner(Corner::ShareZero);
        }
    );
}

mod corner_other_value {
    use super::*;
    negative_control!(
        tilt_every_chart_ray_degenerate_corner,
        "a chart reporting bijective at the corner must fail",
        expected = "reports bijective",
        {
            check_corner(Corner::Bijective);
        }
    );
}

mod corner_weighted {
    use super::*;
    negative_control!(
        tilt_every_chart_ray_degenerate_corner,
        "a non-zero area weight at the corner must fail",
        expected = "a non-zero area weight",
        {
            check_corner(Corner::Weighted);
        }
    );
}

/// The two maps agree with one angle set, bitwise, and with two targets; with one target and both set they differ.
fn check_maps(sheet_map: TiltMap) {
    for (t1, t2, x, y) in [
        (30.0, 0.0, 2, 2),
        (0.0, -50.0, 2, 2),
        (30.0, -50.0, 2, 3),
        (130.0, 170.0, 2, 3),
    ] {
        let sheet = spec(t1, t2, sheet_map, x, y).columns();
        let lean = spec(t1, t2, TiltMap::Lean, x, y).columns();
        assert_eq!(
            format!("{sheet:?}"),
            format!("{lean:?}"),
            "({t1}, {t2}) into ({x}, {y})"
        );
    }
    assert!(
        spec(30.0, -50.0, sheet_map, 2, 2).columns()
            != spec(30.0, -50.0, TiltMap::Lean, 2, 2).columns(),
        "the maps agree with both angles set"
    );
}

#[test]
fn tilt_every_chart_maps_agree() {
    check_maps(TiltMap::Sheet);
    let two = spec(30.0, -50.0, TiltMap::Lean, 2, 3).columns();
    assert_eq!(
        (two[0][3], two[1][2]),
        (0.0, 0.0),
        "two targets: each axis its own"
    );
    assert!(spec(0.0, 0.0, TiltMap::Sheet, 2, 2).untilted());
    assert!(!spec(0.0, 1.0, TiltMap::Sheet, 2, 2).untilted());
    assert!(!spec(1.0, 0.0, TiltMap::Sheet, 2, 2).untilted());
}

/// Rodrigues' rotation by `m` about `k = (τ₂ e_a − τ₁ e_b)/m`, written out here as R-474 K1 gives it, at the angles
/// as passed (no wrap).
fn rodrigues(t1: f64, t2: f64) -> [[f64; 4]; 2] {
    let m = Float::sqrt(t1 * t1 + t2 * t2);
    let (sm, cm) = sc(m);
    let (ka, kb) = (t2 / m, -t1 / m);
    let w = 1.0 - cm;
    [
        [cm + ka * ka * w, ka * kb * w, -kb * sm, 0.0],
        [ka * kb * w, cm + kb * kb * w, ka * sm, 0.0],
    ]
}

/// Tip as sheet's columns with one target and both angles set: the rotation at the stored angles up to a total tip of
/// 180°, `m = 180°` itself unwrapped and never folded (its tilted-axis component keeps the sign of `τ₁`), and past it
/// the rotation at the wrapped angles `τ·(m − 360)/m` (R-474 R3, K1). `wrap_at` is the tip past which the check
/// expects the wrap (the control's is 0, every tilt wrapped).
fn check_rotation(wrap_at: f64) {
    for (t1, t2) in [
        (30.0, 40.0),
        (45.0, 45.0),
        (100.0, 50.0),
        (-120.0, 60.0),
        (108.0, 144.0),
        (150.0, -170.0),
        (-180.0, 180.0),
    ] {
        let m = Float::sqrt(t1 * t1 + t2 * t2);
        let k = (m - 360.0) / m;
        let want = if m > wrap_at {
            rodrigues(t1 * k, t2 * k)
        } else {
            rodrigues(t1, t2)
        };
        let got = spec(t1, t2, TiltMap::Sheet, 2, 2).columns();
        assert!(
            format!("{got:?}") == format!("{want:?}"),
            "Tip as sheet at ({t1}, {t2}) is not the rotation"
        );
    }
    let half = spec(108.0, 144.0, TiltMap::Sheet, 2, 2).columns();
    assert!(
        half[0][2] > 0.0 && half[1][2] > 0.0,
        "m = 180° folded: {half:?}"
    );
}

#[test]
fn tilt_every_chart_tip_as_sheet_rotation() {
    check_rotation(180.0);
}

negative_control!(
    tilt_every_chart_tip_as_sheet_rotation,
    "a wrap of every total tip is not Tip as sheet's rotation below 180°, so it must fail",
    expected = "is not the rotation",
    {
        check_rotation(0.0);
    }
);

negative_control!(
    tilt_every_chart_maps_agree,
    "Lean each axis offered as the default map agrees with Lean everywhere, so the second arm must fail",
    expected = "the maps agree with both angles set",
    {
        check_maps(TiltMap::Lean);
    }
);

/// The tilt at f32, as the kernel holds it: the shape sphere tilted, decoded natively at f32, within f32's rounding
/// of the f64 decode.
fn check_f32(scale: f32) {
    let s = spec(40.0, 70.0, TiltMap::Sheet, 2, 2);
    let wide = tilted(sphere(ShapeMap::Spherical), &s);
    let t = *wide.tilt();
    let f = |x: f64| x as f32 * scale;
    let narrow = Tilt {
        on: t.on,
        two: t.two,
        rank_loss: t.rank_loss,
        centre: t.centre.map(|x| x as f32),
        columns: t.columns.map(|c| c.map(f)),
        x: t.x,
        y: t.y,
        span: t.span.map(|x| x as f32),
        weight: t.weight as f32,
    };
    let c = sphere(ShapeMap::Spherical);
    let chart: ShapeSphere<f32> = ShapeSphere {
        map: c.map,
        n0: c.n0.map(|x| x as f32),
        e1: c.e1.map(|x| x as f32),
        e2: c.e2.map(|x| x as f32),
        scale: c.scale as f32,
        inertia: 1.0,
        phase: 0.0,
        z_mass: c.z_mass.map(|x| x as f32),
        z_mom: c.z_mom.map(|x| x as f32),
    };
    let view = Tilted::new(chart, narrow).unwrap();
    for [u, v] in grid(9, 0.0, 1.0) {
        let lo = view.decode_at::<Defaults, f32>(u as f32, v as f32);
        let hi = wide.decode::<Defaults, f64>(u, v);
        let close =
            (0..3).all(|i| (0..2).all(|k| (f64::from(lo.ic.r[i][k]) - hi.ic.r[i][k]).abs() < 1e-4));
        assert!(close, "the f32 tilt at ({u}, {v}) is not the f64 one");
    }
}

#[test]
fn tilt_every_chart_f32() {
    check_f32(1.0);
}

negative_control!(
    tilt_every_chart_f32,
    "a narrowed tilt whose rotation is scaled is another tilt, so it must fail",
    expected = "is not the f64 one",
    {
        check_f32(1.5);
    }
);

// ── REQ-CHART-063: the mass tilt in logits ────────────────────────────────────────────────────────────────────────────

/// The shape sphere tilted into each mass logit at τ ∈ {15°, 45°, 89°, 135°}, as `τ₁` and as `τ₂`, over a 65 × 65
/// grid: every pixel's masses positive and summing to 1 within 4 ulp. `physical` adds `d` to `m₁` instead (the control).
fn check_mass_logits(physical: bool) {
    let chart = sphere(ShapeMap::Spherical);
    for x in [6, 7] {
        for tau in [15.0, 45.0, 89.0, 135.0] {
            for s in [
                spec(tau, 0.0, TiltMap::Sheet, x, x),
                spec(0.0, tau, TiltMap::Sheet, x, x),
            ] {
                let view = Tilted::new(chart, Tilt::new(&s, [0.5, 0.5], [8.0, 8.0])).unwrap();
                assert!(view.requires_per_pixel_mass());
                for [u, v] in grid(65, 0.0, 1.0) {
                    let mut m = view.decode::<Defaults, f64>(u, v).ic.m;
                    if physical {
                        m = chart.decode::<Defaults, f64>(u, v).ic.m;
                        m[0] += view.tilt().place(u, v).1.dx;
                    }
                    let sum = m[0] + m[1] + m[2];
                    assert!(
                        m.iter().all(|&mi| mi > 0.0) && (sum - 1.0).abs() <= 4.0 * f64::EPSILON,
                        "masses {m:?} at ({u}, {v}) off the simplex"
                    );
                }
            }
        }
    }
}

#[test]
fn tilt_mass_logits() {
    check_mass_logits(false);
}

negative_control!(
    tilt_mass_logits,
    "the same tilt added to the physical mass m₁ leaves the simplex, so it must fail",
    expected = "off the simplex",
    {
        check_mass_logits(true);
    }
);

// ── REQ-CHART-064: the units (R-507) ──────────────────────────────────────────────────────────────────────────────────

/// The tilted horizontal axis's direction in the chart's parameters, `d` per unit of `a`, at the window's edge at zoom
/// 1 and zoom 4, from the shape sphere tilted 30° into a momentum control. `half_width` takes the offsets in units of
/// the window's half-width instead (the control).
fn check_units(half_width: bool) {
    let view = tilted(
        sphere(ShapeMap::Spherical),
        &spec(30.0, 0.0, TiltMap::Sheet, 2, 2),
    );
    let direction = |zoom: f64| {
        // The window's edge is one half-width from the centre.
        let (edge, half) = (0.5 / zoom, 0.5 / zoom);
        let [da, _, d, _] = if half_width {
            let [a, b, x, y] = view.tilt().offsets(edge / half, 0.0);
            [a * half, b * half, x, y]
        } else {
            view.tilt().offsets(edge, 0.0)
        };
        d / da
    };
    assert!(
        direction(1.0).to_bits() == direction(4.0).to_bits(),
        "the tilted axis's direction depends on zoom"
    );
}

#[test]
fn tilt_units() {
    check_units(false);
    // Untilted, the decode is the chart's; on the latent chart the tilt is the basis edit, unscaled.
    check_untilted(sphere(ShapeMap::Spherical), 2, false);
    let (s, c) = sc(30.0);
    let (q1, _) = latent_basis(&FRAME, &spec(30.0, 0.0, TiltMap::Sheet, 4, 4)).unwrap();
    assert_eq!(
        (q1[2], q1[4]),
        (c * 0.5, s * 0.5),
        "the latent tilt is not the basis edit"
    );
    let scaled = s * 0.5 * SPAN[0];
    assert_ne!(
        q1[4], scaled,
        "a latent tilt scaled by the slider span is the basis edit"
    );
    // The offsets: the hidden offset scaled by its span, both rows, both targets.
    let two = Tilt::new(&spec(30.0, -45.0, TiltMap::Sheet, 2, 7), CENTRE, [2.0, 3.0]);
    let [da, db, dx, dy] = two.offsets(0.25, 0.5);
    let (s2, c2) = sc(-45.0);
    assert_eq!(
        [da, db, dx, dy],
        [c * 0.25, c2 * 0.5, 2.0 * (s * 0.25), 3.0 * (s2 * 0.5)]
    );
}

negative_control!(
    tilt_units,
    "a scale in window half-widths changes with zoom, so it must fail",
    expected = "depends on zoom",
    {
        check_units(true);
    }
);

// ── REQ-CHART-065: slice on the five ──────────────────────────────────────────────────────────────────────────────────

/// The decode at the centre of `chart` sliced to `value` on `k` follows the link's forward map on that block.
fn check_follows<C: Hidden<Param = f64>>(chart: &C, sliced: &C, k: u32, controls: [f64; 8]) {
    let d = sliced.decode::<Defaults, f64>(0.5, 0.5);
    let masses = mass::decode::<Defaults, f64>([controls[6], controls[7]]);
    match k {
        6 | 7 => assert_eq!(d.masses, masses, "{}: mass slice", chart.name()),
        0 | 1 => assert_eq!(
            d.config,
            config::decode::<Defaults, f64>([controls[0], controls[1]], &masses),
            "{}: configuration slice",
            chart.name()
        ),
        _ => assert_eq!(
            d.momentum.q,
            momentum::decode::<Defaults, f64>(
                [controls[2], controls[3], controls[4], controls[5]],
                &masses
            )
            .q,
            "{}: momentum slice",
            chart.name()
        ),
    }
}

/// Every hidden coordinate of `chart` sliced: only that control of the held ones moves, the decode at the centre
/// follows its link, and far slices keep the centre a system; a coordinate the axes write is refused.
fn check_slice<C: Hidden<Param = f64>>(chart: C, controls: impl Fn(&C) -> [f64; 8]) {
    let before = controls(&chart);
    for k in 0..9 {
        let value = 1.25 - 0.5 * f64::from(k);
        let Some(sliced) = chart.sliced(k, value) else {
            assert!(!chart.hidden(k), "{}: hidden {k} refused", chart.name());
            continue;
        };
        assert!(
            chart.hidden(k),
            "{}: {k} sliced but not hidden",
            chart.name()
        );
        let after = controls(&sliced);
        for i in 0..8 {
            let want = if i == k as usize { value } else { before[i] };
            assert_eq!(
                after[i],
                want,
                "{}: slice {k} moved control {i}",
                chart.name()
            );
        }
        check_follows(&chart, &sliced, k, after);
        for far in [-50.0, 50.0] {
            let far = chart.sliced(k, far).unwrap();
            assert_eq!(
                far.validate(0.5, 0.5),
                ValidationResult::Valid,
                "{}: slice {k} stops",
                chart.name()
            );
        }
    }
}

/// The shape sphere's mass slice keeps the masses on the simplex; `physical` writes the physical mass (the control).
fn check_simplex(physical: bool) {
    let chart = sphere(ShapeMap::Spherical);
    for k in [6, 7] {
        for value in [-30.0, -2.0, 0.0, 1.5, 40.0] {
            let mut m = chart
                .sliced(k, value)
                .unwrap()
                .decode::<Defaults, f64>(0.5, 0.5)
                .ic
                .m;
            if physical {
                m = chart.decode::<Defaults, f64>(0.5, 0.5).ic.m;
                m[0] = 0.05 * value;
            }
            let sum = m[0] + m[1] + m[2];
            assert!(
                m.iter().all(|&mi| mi > 0.0) && (sum - 1.0).abs() <= 4.0 * f64::EPSILON,
                "masses {m:?} off the simplex"
            );
        }
    }
}

#[test]
fn slice_every_chart() {
    let five: Vec<&str> = REGISTRY
        .iter()
        .copied()
        .filter(|n| FIVE.contains(n))
        .collect();
    assert_eq!(five, FIVE);
    for name in five {
        match name {
            "Latent" => {
                let chart = Latent {
                    z0: Z0,
                    q1: [0.0; 8],
                    q2: [0.0; 8],
                };
                let chart = Latent {
                    q1: latent(&spec(0.0, 0.0, TiltMap::Sheet, 4, 4)).q1,
                    ..chart
                };
                for k in 0..9 {
                    match FRAME.sliced(&chart, k, 0.75) {
                        Some(sliced) => {
                            assert!(FRAME.hidden(k));
                            let mut want = Z0;
                            want[k as usize] = 0.75;
                            assert_eq!(sliced.z0, want, "latent slice {k}");
                            assert_eq!(
                                bits(&sliced.decode::<Defaults, f64>(0.5, 0.5)),
                                bits(&decode::decode::<Defaults, f64>(&decode::Latent::new(want))),
                            );
                        }
                        None => assert!(!FRAME.hidden(k), "latent {k} refused"),
                    }
                }
            }
            "ShapeSphere" => {
                check_slice(sphere(ShapeMap::Spherical), |c| c.map::<f64>(0.5, 0.5).z.z)
            }
            "InvariantLE" => check_slice(InvariantLE(invariant()), |c| c.0.controls()),
            "InvariantLK" => check_slice(InvariantLK(invariant()), |c| c.0.controls()),
            "Mixed" => check_slice(mixed(), |c| c.controls::<Defaults, f64>(0.5, 0.5)),
            other => panic!("{other}"),
        }
    }
    check_simplex(false);
}

negative_control!(
    slice_every_chart,
    "a slice that writes the physical mass leaves the simplex, so it must fail",
    expected = "off the simplex",
    {
        check_simplex(true);
    }
);

// ── REQ-CHART-066: the extension after the tilt ───────────────────────────────────────────────────────────────────────

/// The shape sphere tilted 30° into a momentum control, its centre near the top: each pixel past `t = 1` decodes as
/// the chart at its tilted `(a, b)`, which is the pole-crossing rule's image `(a + ½, 2 − b)`. `before` applies the
/// pole crossing to `(u, v)` before the tilt (the control).
fn check_pole(before: bool) {
    let chart = sphere(ShapeMap::Spherical);
    let centre = [0.4, 0.85];
    let view = Tilted::new(
        chart,
        Tilt::new(&spec(0.0, 30.0, TiltMap::Sheet, 3, 3), centre, SPAN),
    )
    .unwrap();
    let mut past = 0;
    for [u, v] in grid(21, 0.1, 0.7).iter().map(|&[u, v]| [u, v + 0.45]) {
        let ([a, b], shift) = view.tilt().place(u, v);
        if b <= 1.0 {
            continue;
        }
        past += 1;
        let sliced = chart.sliced(3, held(&chart, 3) + shift.dx).unwrap();
        let got = if before && v > 1.0 {
            view.decode::<Defaults, f64>(u + 0.5, 2.0 - v)
        } else {
            view.decode::<Defaults, f64>(u, v)
        };
        assert_eq!(
            bits(&got),
            bits(&sliced.decode::<Defaults, f64>(a, b)),
            "the types act on (a, b)"
        );
        assert_eq!(view.validate(u, v), ValidationResult::Valid);
        let image = sliced.decode::<Defaults, f64>(a + 0.5, 2.0 - b);
        let close = (0..3).all(|i| {
            (0..2).all(|k| {
                (got.ic.r[i][k] - image.ic.r[i][k]).abs() < 1e-12
                    && (got.ic.p[i][k] - image.ic.p[i][k]).abs() < 1e-12
            })
        });
        assert!(
            close,
            "the pixel past the pole at ({u}, {v}) is not the pole-crossing image"
        );
    }
    assert!(past > 20, "too few pixels past the pole: {past}");
}

/// What a pixel of the tilted `(L_z, K)` chart is: a system (`true`) or not. `table` reads the chart's extension from
/// a per-chart table, the warp bounded on every side, instead of its axes' types on the tilted `(a, b)` (the control).
fn check_invariant(table: bool) {
    let chart = InvariantLK(invariant());
    let view = Tilted::new(
        chart,
        Tilt::new(&spec(30.0, 0.0, TiltMap::Sheet, 6, 6), [0.5, 0.5], SPAN),
    )
    .unwrap();
    let mut counts = [0; 3];
    for [u, v] in grid(25, -0.4, 1.4) {
        let ([a, b], shift) = view.tilt().place(u, v);
        let system = if table {
            (0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b)
        } else {
            view.validate(u, v) == ValidationResult::Valid
        };
        if !(0.0..=1.0).contains(&a) || b < 0.0 {
            counts[0] += 1;
            assert!(
                !system,
                "a pixel past a bounded edge at ({a}, {b}) is a system"
            );
        } else if b > 1.0 {
            counts[1] += 1;
            assert!(
                system,
                "a pixel past t = 1 at ({a}, {b}) is not the continuing side's system"
            );
            let d = view.decode::<Defaults, f64>(u, v);
            let sliced = chart.sliced(6, held(&chart, 6) + shift.dx).unwrap();
            assert_eq!(bits(&d), bits(&sliced.decode::<Defaults, f64>(a, b)));
            assert!(d.degenerate.is_none());
        } else {
            counts[2] += 1;
            assert!(system);
        }
    }
    assert!(counts.iter().all(|&c| c > 20), "every case met: {counts:?}");
}

#[test]
fn tilted_extension() {
    check_pole(false);
    check_invariant(false);
    // A saved oblique chart continues affinely: the latent chart's formula past the square.
    let chart = latent(&spec(30.0, 50.0, TiltMap::Sheet, 4, 4));
    for [u, v] in [[1.4, -0.3], [-0.6, 1.9], [2.5, 2.5]] {
        let (a, b) = (u + u - 1.0, v + v - 1.0);
        let z: [f64; 8] = std::array::from_fn(|i| chart.z0[i] + a * chart.q1[i] + b * chart.q2[i]);
        assert_eq!(
            chart.map::<f64>(u, v).z.z,
            z,
            "the oblique chart does not continue affinely"
        );
        assert_eq!(chart.validate(u, v), ValidationResult::Valid);
    }
}

negative_control!(
    tilted_extension,
    "a pole crossing applied to (u, v) before the tilt is not the types acting on (a, b), so it must fail",
    expected = "the types act on (a, b)",
    {
        check_pole(true);
    }
);

mod extension_table {
    use super::*;
    negative_control!(
        tilted_extension,
        "a per-chart extension table, bounded on every side, drops the continuing side, so it must fail",
        expected = "is not the continuing side's system",
        {
            check_invariant(true);
        }
    );
}

/// `Shift` with one target adds nothing at `y`; with two it adds `dy` there. `second` is what one target's `two` reads
/// (the control reads it as a second target).
fn check_shift(second: bool) {
    let z = [0.5; 8];
    let one = Shift {
        x: 3,
        dx: 0.25,
        y: 5,
        dy: 1.0,
        two: second,
    };
    assert_eq!(shift_controls(z, &one)[5], 0.5, "one target's y moved");
    let one = Shift { two: false, ..one };
    let two = Shift { two: true, ..one };
    let past = Shift { x: 8, y: 9, ..two };
    for k in 0..8 {
        let want = |s: &Shift<f64>| {
            0.5 + if s.x == k { s.dx } else { 0.0 } + if s.two && s.y == k { s.dy } else { 0.0 }
        };
        assert_eq!(
            shift_controls(z, &one)[k as usize],
            want(&one),
            "one target, {k}"
        );
        assert_eq!(
            shift_controls(z, &two)[k as usize],
            want(&two),
            "two targets, {k}"
        );
        assert_eq!(
            shift_controls(z, &past)[k as usize],
            0.5,
            "past the controls, {k}"
        );
    }
}

#[test]
fn tilt_every_chart_shift_controls() {
    check_shift(false);
}

negative_control!(
    tilt_every_chart_shift_controls,
    "one target's y read as a second target adds its offset, so it must fail",
    expected = "one target's y moved",
    {
        check_shift(true);
    }
);
