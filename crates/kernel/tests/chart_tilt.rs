//! Tilt and slice on every chart of the registry (`kernel::chart::tilt`, `kernel::chart::construction`; R-473, R-474,
//! R-507, R-544, R-554, R-613, R-633; chart_decoder_contract Part 4, § "Tilt (basis edit)"):
//! - `tilt_every_chart_*` (REQ-CHART-062's arms, C2): on the five charts that hold their blocks by latent controls,
//!   untilted under both maps, no tilt path and a decode bitwise the chart's; `(a, b, d) = R·(u, v, 0)` to 1 ulp, `a`,
//!   `b` through the chart's own map and `d` on the control; the centre fixed under every tilt; the latent chart's basis
//!   edit; the tilted shape sphere bijective, each pixel counted once; the f32 instantiation. On the six that hold a
//!   block by a construction (`tilt_every_chart_construction_*`, R-633): `d = 0` bitwise the untilted chart, `ν₀` moved
//!   by `d` with the masses kept, a rest start's momenta `D_mom` of the control from 0, the held `L_z` or `K` §2.2's
//!   construction at the tilted value under each of R-471's modes (R-708): warped, the default, no cell refused, the
//!   bottom row included, the held `L_z` by value at both signs of `d` (R-720), the held `K` in its warp coordinate and
//!   clamped at `t = 0` (R-709); linear, rows at fixed values, the refused cells no system and weighted 0. RQ-471's
//!   corner `ray-degenerate` over the whole registry, unfolded, weighted 0, its shares undefined.
//! - `tilt_mass_logits` (REQ-CHART-063, C7): the shape sphere tilted into each mass logit keeps every mass positive,
//!   the three summing to 1.
//! - `tilt_units` (REQ-CHART-064, R-507): the tilted axis's direction in the chart's parameters is the same at every
//!   zoom; the latent chart's tilt is the basis edit, unscaled.
//! - `slice_every_chart` (REQ-CHART-065's arms over the whole registry, T6): a slice moves one hidden control, the
//!   decode at the centre following the link, never a stop on the five; on the six, a slice moves only that held value,
//!   `ν₀`, the held `L_z` or `K`, or one momentum control, the held `L_z` by value at both signs (R-720). A slice past
//!   the edge stopping is REQ-CHART-060's.
//! - `tilted_extension` (REQ-CHART-066, X3): the types act on the tilted `(a, b)`.
//!
//! Each test registers its negative control (R-176).

use kernel::chart::burrau_acute;
use kernel::chart::construction::{
    momentum_control, Held, LzChart, RestChart, HELD_K, HELD_LZ, NU0,
};
use kernel::chart::invariant::{k_floor, l_max, Invariant};
use kernel::chart::mixed::{Alpha, PRhoX};
use kernel::chart::shape_sphere::{self, pixel_centre, PixelCount};
use kernel::chart::tilt::{
    latent_basis, share, shift_controls, Hidden, LatentFrame, Shift, Tiltable,
};
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, ChartDecode, Constraint,
    EuclidPlane, Float, HeldK, HeldLz, InvariantLE, InvariantLK, Latent, Mapped, Mapping,
    MassSimplex, Mixed, RestStart, ShapeMap, ShapeSphere, SystemImage, Tilt, TiltMap, TiltSpec,
    Tilted, ValidationResult, REGISTRY,
};
use kernel::decode::invariant::{infeasible, no_construction};
use kernel::decode::{self, config, mass, momentum, Decoded, Degenerate};
use kernel::generated::links::{
    Defaults, Link, LinkParams, LinkReal, Select, SigmoidAlpha, SigmoidBeta, SigmoidQ, SoftmaxTanh,
    SoftsignQ, TanhQ,
};
use kernel::payload::PayloadReal;
use std::io::Write;
use validation::negative_control;

/// The five charts that hold their blocks by latent controls (R-633's split).
const FIVE: [&str; 5] = [
    "Latent",
    "ShapeSphere",
    "InvariantLE",
    "InvariantLK",
    "Mixed",
];
/// The six charts that hold a block by a physical construction (R-633), in the registry's order.
const SIX: [&str; 6] = [
    "Burrau",
    "EuclidPlane",
    "BurrauThetaK",
    "BurrauThetaLz",
    "BurrauThetaDeltaM",
    "MassSimplex",
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

/// Tip as sheet's columns with `m` taken straight from the squares, whatever they underflow to.
fn columns_from_squares(t1: f64, t2: f64) -> [[f64; 4]; 2] {
    let m = (t1 * t1 + t2 * t2).sqrt();
    let (sm, cm) = m.to_radians().sin_cos();
    let (ka, kb) = (t2 / m, -t1 / m);
    let w = 1.0 - cm;
    [
        [cm + ka * ka * w, ka * kb * w, -kb * sm, 0.0],
        [ka * kb * w, cm + kb * kb * w, ka * sm, 0.0],
    ]
}

/// Tip as sheet at angles whose squares underflow is still the small rotation they name (R-474 K1), and the view keeps
/// its centre (R-473 J7): to first order `R·e_a = e_a + τ₁ e_X` and `R·e_b = e_b + τ₂ e_X`, `τ` in radians. The arms take
/// a sum of squares of 0, a subnormal one, a mixed pair, the least double, the exact boundary `2⁻¹⁰²²` and the least
/// sum above it. A sum exactly at `2⁻¹⁰²²` is normal, so it keeps the formula bit for bit, at a pair whose hypotenuse
/// rounds one ulp below `2⁻⁵¹¹`. `squares` puts the columns taken straight from the squares into the view (the control).
fn check_tiny_sheet(squares: bool) {
    let boundary = (2f64.powi(-511), 2f64.powi(-600));
    let above = (
        f64::from_bits(2f64.powi(-511).to_bits() + 1),
        2f64.powi(-600),
    );
    let angles = [
        (1e-200, 1e-200),
        (1e-200, -3e-200),
        (-1e-160, 2e-160),
        (2e-170, 1e-200),
        (f64::from_bits(1), f64::from_bits(1)),
        boundary,
        above,
    ];
    let own = mixed().decode::<Defaults, f64>(CENTRE[0], CENTRE[1]);
    for (t1, t2) in angles {
        let s = spec(t1, t2, TiltMap::Sheet, 4, 4);
        let mut tilt = Tilt::new(&s, CENTRE, SPAN);
        if squares {
            tilt.columns = columns_from_squares(t1, t2);
        }
        let [c1, c2] = tilt.columns;
        let flat = c1.iter().chain(&c2).all(|c| c.is_finite());
        assert!(flat, "a tiny sheet tilt's columns are not finite: {s:?}");
        let close = |got: f64, want: f64| (got - want).abs() <= 1e-14 * want.abs() + 1e-322;
        assert_eq!((c1[0], c1[1], c2[0], c2[1]), (1.0, 0.0, 0.0, 1.0), "{s:?}");
        assert!(
            close(c1[2], t1.to_radians()) && close(c2[2], t2.to_radians()),
            "a tiny sheet tilt is not its small rotation: {c1:?}, {c2:?}, {s:?}"
        );
        let view = Tilted::new(mixed(), tilt).unwrap();
        assert!(
            bits(&view.decode::<Defaults, f64>(CENTRE[0], CENTRE[1])) == bits(&own),
            "a tiny sheet tilt moved its centre: {s:?}"
        );
        let ([a, b], shift) = tilt.place(0.9, 0.2);
        assert!(
            a.is_finite() && b.is_finite() && shift.dx.is_finite(),
            "{s:?}"
        );
    }
    let (t1, t2) = (
        f64::from_bits(0x1ffe_f324_d87b_c070),
        f64::from_bits(0x1fe0_4301_7b23_1710),
    );
    assert_eq!(t1 * t1 + t2 * t2, f64::MIN_POSITIVE);
    assert_eq!(
        Tilt::new(&spec(t1, t2, TiltMap::Sheet, 4, 4), CENTRE, SPAN).columns,
        columns_from_squares(t1, t2),
        "a normal sum of squares left the formula as written"
    );
}

#[test]
fn tilt_every_chart_tiny_sheet_angles() {
    check_tiny_sheet(false);
}

negative_control!(
    tilt_every_chart_tiny_sheet_angles,
    "columns taken straight from squares that underflow hold NaN, so they must fail",
    expected = "columns are not finite",
    {
        check_tiny_sheet(true);
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

/// Under a turned, zoomed frame the basis edit is `q₁ = σ(cos γ û₁ + sin γ û₂)`, `q₂ = σ(cos γ û₂ − sin γ û₁)`, with
/// `û₁`, `û₂` the rotation's columns placed on `e_a`, `e_b` and the targets (R-473 T2, R-507 U2). `turn` is the sign
/// the check gives `γ` (the control's is −1, the frame turned the other way).
fn check_turned(turn: f64) {
    for (gamma, scale) in [(0.7, 0.5), (-2.1, 1.75), (std::f64::consts::FRAC_PI_2, 0.3)] {
        let frame = LatentFrame {
            gamma,
            scale,
            ..FRAME
        };
        for s in [
            spec(30.0, -45.0, TiltMap::Sheet, 4, 5),
            spec(40.0, 70.0, TiltMap::Sheet, 6, 6),
            spec(40.0, 70.0, TiltMap::Lean, 6, 6),
        ] {
            let [c1, c2] = s.columns();
            let place = |c: [f64; 4]| {
                let mut u = [0.0; 8];
                u[2] += c[0];
                u[3] += c[1];
                u[s.x as usize] += c[2];
                u[s.y as usize] += c[3];
                u
            };
            let (u1, u2) = (place(c1), place(c2));
            let (sg, cg) = Float::sin_cos(turn * gamma);
            let (q1, q2) = latent_basis(&frame, &s).unwrap();
            for i in 0..8 {
                let w1 = scale * (cg * u1[i] + sg * u2[i]);
                let w2 = scale * (cg * u2[i] - sg * u1[i]);
                assert!(
                    (q1[i] - w1).abs() < 1e-15 && (q2[i] - w2).abs() < 1e-15,
                    "the turned basis edit at γ = {gamma}, σ = {scale}, {s:?}: {q1:?}, {q2:?}"
                );
            }
        }
    }
}

#[test]
fn tilt_every_chart_latent_basis_turned() {
    check_turned(1.0);
}

negative_control!(
    tilt_every_chart_latent_basis_turned,
    "a frame turned the other way is not the basis edit, so it must fail",
    expected = "the turned basis edit",
    {
        check_turned(-1.0);
    }
);

/// The offsets `(Δa, Δb, d_X, d_Y) = ũ·R·e_a + ṽ·R·e_b`, each hidden offset times its target's slider span (R-473 T2,
/// R-507 U1), against the maps' closed forms: two targets `(cos τ₁ ũ, cos τ₂ ṽ, σ_X sin τ₁ ũ, σ_Y sin τ₂ ṽ)`, Lean each
/// axis with one target `(cos τ₁ ũ, cos τ₂ ṽ, σ_X (sin τ₁ ũ + sin τ₂ ṽ), 0)`, Tip as sheet with one target its
/// rotation's columns. `span` is the spans the check expects (the control's are swapped).
fn check_offsets(span: [f64; 2]) {
    let spans = [2.0, 0.5];
    let (t1, t2) = (30.0, -45.0);
    let ((s1, c1), (s2, c2)) = (sc(t1), sc(t2));
    let [r1, r2] = spec(40.0, 70.0, TiltMap::Sheet, 2, 2).columns();
    for (du, dv) in [(0.3, -0.2), (-0.45, 0.6), (0.1, 0.0), (0.0, -0.35)] {
        let two = Tilt::new(&spec(t1, t2, TiltMap::Lean, 2, 3), CENTRE, spans);
        let lean = Tilt::new(&spec(t1, t2, TiltMap::Lean, 2, 2), CENTRE, spans);
        let sheet = Tilt::new(&spec(40.0, 70.0, TiltMap::Sheet, 2, 2), CENTRE, spans);
        let cases = [
            (
                two,
                [c1 * du, c2 * dv, span[0] * s1 * du, span[1] * s2 * dv],
            ),
            (lean, [c1 * du, c2 * dv, span[0] * (s1 * du + s2 * dv), 0.0]),
            (
                sheet,
                [
                    r1[0] * du + r2[0] * dv,
                    r1[1] * du + r2[1] * dv,
                    span[0] * (r1[2] * du + r2[2] * dv),
                    0.0,
                ],
            ),
        ];
        for (tilt, want) in cases {
            let got = tilt.offsets(du, dv);
            assert!(
                (0..4).all(|i| (got[i] - want[i]).abs() < 1e-15),
                "the offsets at ({du}, {dv}): {got:?}, not {want:?}"
            );
            // The pixel at the centre plus the offset: `place` is the centre plus that pixel's offsets.
            let (u, v) = (CENTRE[0] + du, CENTRE[1] + dv);
            let [da, db, dx, dy] = tilt.offsets(u - CENTRE[0], v - CENTRE[1]);
            let ([a, b], shift) = tilt.place(u, v);
            assert_eq!(
                [a, b],
                [CENTRE[0] + da, CENTRE[1] + db],
                "place at ({u}, {v})"
            );
            assert_eq!(
                (shift.dx, shift.dy),
                (dx, dy),
                "place's shift at ({u}, {v})"
            );
        }
    }
}

#[test]
fn tilt_every_chart_offsets() {
    check_offsets([2.0, 0.5]);
}

negative_control!(
    tilt_every_chart_offsets,
    "each hidden offset scaled by the other target's span is wrong, so it must fail",
    expected = "the offsets at",
    {
        check_offsets([0.5, 2.0]);
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

/// The exponential map at scale π about `[0.5, 0.5]`, whose circle `r = π` meets the square's edges, under `tilt`.
fn wide_cap(tilt: &TiltSpec) -> Tilted<ShapeSphere<f64>, f64> {
    let mut wide = sphere(ShapeMap::Exponential);
    wide.scale = std::f64::consts::PI;
    Tilted::new(wide, Tilt::new(tilt, [0.5, 0.5], SPAN)).unwrap()
}

/// A tilted view's pixels on `r = π` count once per distinct hidden offset, and once together where `N_a = N_b = 0`
/// (chart_reference §3.3, REQ-CHART-090). `shared` counts them with one boolean across the view (the control).
fn check_antipode_by_offset(shared: bool) {
    let count = |view: &Tilted<ShapeSphere<f64>, f64>, points: &[[f64; 2]]| {
        if shared {
            let cell = |p: &[f64; 2]| view.pixel(p[0], p[1]);
            let systems = points
                .iter()
                .filter(|p| cell(p) == Some(PixelCount::System));
            let circle = points.iter().any(|p| cell(p) == Some(PixelCount::Antipode));
            systems.count() as u64 + u64::from(circle)
        } else {
            view.counted_total(points.iter().copied())
        }
    };
    // N_a ≠ 0: the same shape at d = −0.5 and +0.5, two systems whose masses differ.
    let view = wide_cap(&spec(90.0, 0.0, TiltMap::Sheet, 6, 6));
    let (near, far) = ([0.25, 1.0], [0.75, 1.0]);
    for [u, v] in [near, far] {
        assert_eq!(view.pixel(u, v), Some(PixelCount::Antipode));
    }
    let shifts = (view.at(0.25, 1.0).1.unwrap(), view.at(0.75, 1.0).1.unwrap());
    assert_eq!((shifts.0.dx, shifts.1.dx), (-0.5, 0.5));
    assert_eq!(count(&view, &[near, far]), 2, "two systems on the circle");
    // A pixel counted again, later in the walk and with another between, is still the one system.
    assert_eq!(count(&view, &[near, far, near, far, near]), 2);
    // A chord along the level line of d, its two ends one system.
    assert_eq!(count(&view, &[near, [0.25, 0.0]]), 1, "a chord at one d");
    assert_eq!(count(&view, &[near, [0.25, 0.0], far]), 2);
    // A system pixel at the same d is its own, and hides no pixel on the circle.
    assert_eq!(view.pixel(0.25, 0.5), Some(PixelCount::System));
    assert_eq!(count(&view, &[[0.25, 0.5], near]), 2);
    // The domain's end at the same d is no system and hides none either.
    assert_eq!(view.pixel(0.25, 1.5), Some(PixelCount::DomainEnd));
    assert_eq!(count(&view, &[[0.25, 1.5], near]), 1);
    // A chord at d = 0, its ends computed as −0 and +0: one value, one system.
    let view = wide_cap(&spec(-90.0, 0.0, TiltMap::Sheet, 6, 6));
    let zeros = [[0.5, 1.0], [0.5, 0.0]];
    let lo: Shift<f64> = view.at(0.5, 1.0).1.unwrap();
    let hi: Shift<f64> = view.at(0.5, 0.0).1.unwrap();
    assert_ne!(lo.dx.to_bits(), hi.dx.to_bits(), "the signs of 0");
    assert_eq!(count(&view, &zeros), 1, "−0 and +0 are one d");
    // Two targets: the second offset alone tells these apart.
    let view = wide_cap(&spec(0.0, 90.0, TiltMap::Sheet, 2, 3));
    let pair = [[1.0, 0.25], [1.0, 0.75]];
    let (lo, hi) = (view.at(1.0, 0.25).1.unwrap(), view.at(1.0, 0.75).1.unwrap());
    assert_eq!(lo.dx, hi.dx);
    assert_ne!(lo.dy, hi.dy);
    assert_eq!(count(&view, &pair), 2, "d_Y differs");
    // N_a = N_b = 0, decided on the stored angles: sin 180° leaves different computed d at the circle's two sides,
    // and they are one system.
    let view = wide_cap(&spec(180.0, 0.0, TiltMap::Sheet, 6, 6));
    assert_eq!(view.tilt().normal_zero, [true, true]);
    let sides = [[0.0, 0.5], [1.0, 0.5], [0.5, 1.0]];
    for [u, v] in sides {
        assert_eq!(view.pixel(u, v), Some(PixelCount::Antipode));
    }
    let (lo, hi) = (view.at(0.0, 0.5).1.unwrap(), view.at(1.0, 0.5).1.unwrap());
    assert_ne!(lo.dx, hi.dx, "rounding leaves a computed offset");
    assert_eq!(count(&view, &sides), 1, "d unchanged");
    // Untilted, the one system −n₀.
    let view = wide_cap(&spec(0.0, 0.0, TiltMap::Sheet, 6, 6));
    assert_eq!(count(&view, &sides), 1, "untilted, one system");
}

#[test]
fn tilt_every_chart_antipode_counts_per_hidden_offset() {
    check_antipode_by_offset(false);
}

negative_control!(
    tilt_every_chart_antipode_counts_per_hidden_offset,
    "one boolean shared across the view counts two systems on r = π as one, so it must fail",
    expected = "two systems on the circle",
    {
        check_antipode_by_offset(true);
    }
);

/// The tilted spherical map's pixels past the square: a repeat is uncounted only where the map keeps `d` (R-554 L3;
/// chart_decoder_contract § "Repeats and the canonical point"), else the view is affine there and each pixel is a
/// system of its own, counted (R-512 F1, F3). Arms:
/// - `check_pole`'s view, `τ₂ = 30°` into a momentum control about `[0.4, 0.85]`: `N_b ≠ 0`, so each pixel past the
///   pole is counted, and the count over the window is every pixel;
/// - `τ₁ = 30°` past θ's period: `N_a ≠ 0`, counted;
/// - `τ₂ = 30°` past θ's period: `N_a = 0`, a repeat, uncounted;
/// - Tip as sheet at `m = 180°` over a pole: `N_a = N_b = 0`, uncounted.
///
/// `old` counts with the rule that took no pixel past the square (the control).
fn check_affine_counted(old: bool) {
    let count = |view: &Tilted<ShapeSphere<f64>, f64>, points: &[[f64; 2]]| {
        if old {
            points
                .iter()
                .filter(|&&[u, v]| {
                    let [a, b] = view.at(u, v).0;
                    (0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b)
                })
                .count() as u64
        } else {
            view.counted_total(points.iter().copied())
        }
    };
    let past = |view: &Tilted<ShapeSphere<f64>, f64>, points: &[[f64; 2]]| {
        points
            .iter()
            .filter(|&&[u, v]| {
                let [a, b] = view.at(u, v).0;
                !((0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b))
            })
            .count() as u64
    };
    let view = |s: &TiltSpec, centre: [f64; 2]| {
        Tilted::new(sphere(ShapeMap::Spherical), Tilt::new(s, centre, SPAN)).unwrap()
    };
    // Over the pole, affine: every pixel counted.
    let pole = view(&spec(0.0, 30.0, TiltMap::Sheet, 3, 3), [0.4, 0.85]);
    let window: Vec<[f64; 2]> = grid(21, 0.1, 0.7)
        .iter()
        .map(|&[u, v]| [u, v + 0.45])
        .collect();
    let over = past(&pole, &window);
    assert!(over > 20, "too few pixels past the pole: {over}");
    assert_eq!(
        count(&pole, &window),
        window.len() as u64,
        "a pixel past the pole of an affine view is not counted"
    );
    // Past θ's period under τ₁, affine: counted.
    let theta = view(&spec(30.0, 0.0, TiltMap::Sheet, 3, 3), [0.9, 0.5]);
    // u over [0.6, 1.4], v over [0.3, 0.7]: past θ's period, never past a pole.
    let window: Vec<[f64; 2]> = grid(21, 0.0, 1.0)
        .iter()
        .map(|&[u, v]| [0.6 + 0.8 * u, 0.3 + 0.4 * v])
        .collect();
    assert!(past(&theta, &window) > 20);
    assert_eq!(
        count(&theta, &window),
        window.len() as u64,
        "a pixel past θ's period of an affine view is not counted"
    );
    // Past θ's period under τ₂ alone, N_a = 0: a repeat, not counted.
    let period = view(&spec(0.0, 30.0, TiltMap::Sheet, 3, 3), [0.9, 0.5]);
    let repeats = past(&period, &window);
    assert!(repeats > 20);
    assert!(window
        .iter()
        .all(|&[u, v]| (0.0..=1.0).contains(&period.at(u, v).0[1])));
    assert_eq!(
        period.pixel(1.3, 0.5),
        None,
        "a repeat past θ's period counted"
    );
    assert_eq!(count(&period, &window), window.len() as u64 - repeats);
    // Over a pole at m = 180°, N_a = N_b = 0: a repeat, not counted.
    let half = view(&spec(108.0, 144.0, TiltMap::Sheet, 3, 3), [0.5, 0.5]);
    let window = grid(21, 0.0, 1.0);
    let repeats = past(&half, &window);
    assert!(repeats > 20);
    assert_eq!(count(&half, &window), window.len() as u64 - repeats);
}

#[test]
fn tilt_every_chart_shape_sphere_affine_counted() {
    check_affine_counted(false);
}

negative_control!(
    tilt_every_chart_shape_sphere_affine_counted,
    "a count that takes no pixel past the square undercounts an affine view, so it must fail",
    expected = "a pixel past the pole of an affine view is not counted",
    {
        check_affine_counted(true);
    }
);

/// `τ₂ = 30°` alone (`N_a = 0`, `N_b ≠ 0`) about `[0.9, 0.85]`, a window past θ's period and the pole at once: the
/// spherical map repeats in `s` for every `t`, so a pixel past θ's period is a repeat, uncounted, wherever its `b` lies,
/// and one past the pole alone is affine, counted (R-554 L3; chart_decoder_contract § "Repeats and the canonical
/// point"; R-408). The count is the window less its pixels with `a` outside `[0, 1]`.
///
/// `old` counts with the rule that applied θ's period only with `b` in `[0, 1]` (the control).
fn check_period_and_pole(old: bool) {
    let view = Tilted::new(
        sphere(ShapeMap::Spherical),
        Tilt::new(&spec(0.0, 30.0, TiltMap::Sheet, 3, 3), [0.9, 0.85], SPAN),
    )
    .unwrap();
    // u over [0.6, 1.4], v over [0.55, 1.15]: past θ's period and the pole, each alone and both at once.
    let window: Vec<[f64; 2]> = grid(21, 0.0, 1.0)
        .iter()
        .map(|&[u, v]| [0.6 + 0.8 * u, 0.55 + 0.6 * v])
        .collect();
    let inside = |x: f64| (0.0..=1.0).contains(&x);
    let ab: Vec<[f64; 2]> = window.iter().map(|&[u, v]| view.at(u, v).0).collect();
    let tally = |f: &dyn Fn(bool, bool) -> bool| {
        ab.iter().filter(|&&[a, b]| f(inside(a), inside(b))).count()
    };
    assert!(
        tally(&|a_in, b_in| !a_in && !b_in) > 20,
        "too few pixels past both"
    );
    assert!(
        tally(&|a_in, b_in| a_in && !b_in) > 20,
        "too few pixels past the pole alone"
    );
    assert!(
        tally(&|a_in, b_in| !a_in && b_in) > 20,
        "too few pixels past θ's period alone"
    );
    let count = if old {
        tally(&|a_in, b_in| a_in || !b_in) as u64
    } else {
        view.counted_total(window.iter().copied())
    };
    let past_period = tally(&|a_in, _| !a_in) as u64;
    assert_eq!(
        count,
        window.len() as u64 - past_period,
        "a repeat past θ's period and the pole at once is counted"
    );
    // Each pixel past both, and its partner one period back, keep n̂ and d: the partner counts, the pixel does not.
    for (&[u, v], &[a, b]) in window.iter().zip(&ab) {
        if !inside(a) && !inside(b) {
            let counted = if old {
                Some(PixelCount::System)
            } else {
                view.pixel(u, v)
            };
            assert_eq!(
                counted, None,
                "a repeat past θ's period and the pole at once is counted"
            );
        }
    }
}

#[test]
fn tilt_every_chart_shape_sphere_period_and_pole() {
    check_period_and_pole(false);
}

negative_control!(
    tilt_every_chart_shape_sphere_period_and_pole,
    "a rule that applies θ's period only with b in [0, 1] counts a repeat past the period and the pole twice, so it \
     must fail",
    expected = "a repeat past θ's period and the pole at once is counted",
    {
        check_period_and_pole(true);
    }
);

// ── The lattice of repeats past t's period 2 (R-655, ruling RQ-631 (a)) ───────────────────────────────────────────────

/// The spherical map tilted by `s` about the square's centre into a momentum control.
fn spherical(s: &TiltSpec) -> Tilted<ShapeSphere<f64>, f64> {
    Tilted::new(sphere(ShapeMap::Spherical), Tilt::new(s, [0.5, 0.5], SPAN)).unwrap()
}

/// Whether the pixel at `(u, v)` is counted: by [`Tilted::pixel`], or under `old` by the round-2 rule, R-554 L3's
/// alone, which takes a repeat past θ's period iff `N_a = 0` and over a pole iff `N_a = N_b = 0`, and no lattice.
fn counted(view: &Tilted<ShapeSphere<f64>, f64>, u: f64, v: f64, old: bool) -> bool {
    if old {
        let [a, b] = view.at(u, v).0;
        let inside = |x: f64| (0.0..=1.0).contains(&x);
        let [na, nb] = view.tilt().normal_zero;
        !((!inside(a) && na) || (!inside(b) && na && nb))
    } else {
        view.pixel(u, v) == Some(PixelCount::System)
    }
}

/// The largest difference between two decodes' masses, positions and momenta.
fn ic_gap(x: &Decoded<f64>, y: &Decoded<f64>) -> f64 {
    let flat = |d: &Decoded<f64>| {
        let mut v = d.ic.m.to_vec();
        v.extend(d.ic.r.iter().flatten());
        v.extend(d.ic.p.iter().flatten());
        v
    };
    flat(x)
        .iter()
        .zip(flat(y))
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
}

/// Over `window`, each pixel and its partner `shift` away in the window's `(u, v)`, whose `(a, b)` lies `w` away: the
/// two decode the same system, to rounding, and exactly one of them is counted (R-408, R-655). `old` counts with the
/// round-2 rule (the control).
fn check_pairs(
    view: &Tilted<ShapeSphere<f64>, f64>,
    window: &[[f64; 2]],
    shift: [f64; 2],
    w: [f64; 2],
    old: bool,
) {
    for &[u, v] in window {
        let (p, q) = ([u, v], [u + shift[0], v + shift[1]]);
        let ([a, b], [a2, b2]) = (view.at(p[0], p[1]).0, view.at(q[0], q[1]).0);
        assert!(
            (a2 - a - w[0]).abs() < 1e-12 && (b2 - b - w[1]).abs() < 1e-12,
            "the partner of ({u}, {v}) is not w away"
        );
        let gap = ic_gap(
            &view.decode::<Defaults, f64>(p[0], p[1]),
            &view.decode::<Defaults, f64>(q[0], q[1]),
        );
        assert!(gap < 1e-9, "({u}, {v}) and its partner differ by {gap}");
        let once = counted(view, p[0], p[1], old) != counted(view, q[0], q[1], old);
        assert!(once, "a repeat counted with its partner at ({u}, {v})");
    }
}

/// `τ₁ = 30°` alone (`N_b = 0`, `N_a ≠ 0`): `t` and `t + 2` are one system, the pixel counted once, at `t` in
/// `[−½, 3/2)`; one unit up is another system. `old` counts with the round-2 rule (the control).
fn check_t_period(old: bool) {
    let view = spherical(&spec(30.0, 0.0, TiltMap::Sheet, 3, 3));
    // u over [0.1, 0.9], v over [−0.5, 1.5]: one period in t, each partner two units up, past the pole.
    let window: Vec<[f64; 2]> = grid(12, 0.0, 1.0)
        .iter()
        .map(|&[u, v]| [0.1 + 0.8 * u, -0.5 + 2.0 * v])
        .collect();
    check_pairs(&view, &window, [0.0, 2.0], [0.0, 2.0], old);
    let one = ic_gap(
        &view.decode::<Defaults, f64>(0.3, 0.2),
        &view.decode::<Defaults, f64>(0.3, 1.2),
    );
    assert!(one > 1e-3, "t and t + 1 decode one system");
}

#[test]
fn tilt_every_chart_shape_sphere_lattice_t_period() {
    check_t_period(false);
}

negative_control!(
    tilt_every_chart_shape_sphere_lattice_t_period,
    "the round-2 rule counts t and t + 2 at τ₁ = 30° alone twice, so it must fail",
    expected = "a repeat counted with its partner",
    {
        check_t_period(true);
    }
);

/// Lean each axis at `τ₁ = τ₂ = 45°` (`N_a = N_b`): `(s − 2, t + 2)` repeats `(s, t)`, the pair counted once. The
/// window's pixel moves `(Δa, Δb) = cos 45°·(Δu, Δv)`, so the partner is `2/cos 45°` along each. `old` counts with the
/// round-2 rule (the control).
fn check_lean_diagonal(old: bool) {
    let view = spherical(&spec(45.0, 45.0, TiltMap::Lean, 3, 3));
    let step = 2.0 / view.tilt().columns[0][0];
    let window = grid(12, 0.0, 1.0);
    check_pairs(&view, &window, [-step, step], [-2.0, 2.0], old);
}

#[test]
fn tilt_every_chart_shape_sphere_lattice_lean_diagonal() {
    check_lean_diagonal(false);
}

negative_control!(
    tilt_every_chart_shape_sphere_lattice_lean_diagonal,
    "the round-2 rule counts (s − 2, t + 2) and (s, t) under Lean 45°/45° twice, so it must fail",
    expected = "a repeat counted with its partner",
    {
        check_lean_diagonal(true);
    }
);

/// The strip `[dir, half]` of the generator `w` along `along`, as the kernel forms it.
fn strip_of(along: [f64; 2], w: [f64; 2]) -> ([f64; 2], f64) {
    let len = along[0].hypot(along[1]);
    ([along[0] / len, along[1] / len], w[0].hypot(w[1]) / 2.0)
}

/// A lattice arm: the stored tilt and its strip's `(along, w)`, or none.
type LatticeCase = (TiltSpec, Option<([f64; 2], [f64; 2])>);

/// The lattice's arms on the stored angles.
fn lattice_cases() -> Vec<LatticeCase> {
    let one = |t1, t2, map| spec(t1, t2, map, 2, 2);
    let two = |t1, t2, map| spec(t1, t2, map, 2, 3);
    let lean = |t1, t2| one(t1, t2, TiltMap::Lean);
    let sheet = |t1, t2| one(t1, t2, TiltMap::Sheet);
    let t_period = Some(([0.0, 1.0], [0.0, 2.0]));
    let mut cases = vec![
        (sheet(0.0, 0.0), None),
        (sheet(108.0, 144.0), None),
        (lean(90.0, 90.0), None),
        (lean(-90.0, 90.0), None),
        (lean(180.0, 0.0), None),
    ];
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        cases.extend([
            (one(30.0, 0.0, map), t_period),
            (one(0.0, 30.0, map), None),
            (two(30.0, 0.0, map), t_period),
            (two(30.0, 180.0, map), t_period),
            (two(0.0, 30.0, map), None),
            (two(30.0, 45.0, map), None),
            (two(45.0, 45.0, map), None),
        ]);
    }
    cases.extend([
        (lean(90.0, 30.0), t_period),
        (lean(30.0, 90.0), None),
        // tan τ₁ = ±tan τ₂: the ratio ±1.
        (lean(45.0, 45.0), Some(([1.0, -1.0], [2.0, -2.0]))),
        (lean(45.0, -135.0), Some(([1.0, -1.0], [2.0, -2.0]))),
        (lean(-135.0, 45.0), Some(([1.0, -1.0], [2.0, -2.0]))),
        (lean(30.0, 210.0), Some(([1.0, -1.0], [2.0, -2.0]))),
        (lean(30.0, -150.0), Some(([1.0, -1.0], [2.0, -2.0]))),
        (lean(45.0, -45.0), Some(([1.0, 1.0], [2.0, 2.0]))),
        (lean(45.0, 135.0), Some(([1.0, 1.0], [2.0, 2.0]))),
        (lean(-30.0, 30.0), Some(([1.0, 1.0], [2.0, 2.0]))),
        // One angle ±30°, the other ±60°: the ratio ±1/3 or ±3.
        (lean(30.0, 60.0), Some(([3.0, -1.0], [6.0, -2.0]))),
        (lean(390.0, 60.0), Some(([3.0, -1.0], [6.0, -2.0]))),
        (lean(-150.0, -120.0), Some(([3.0, -1.0], [6.0, -2.0]))),
        (lean(30.0, 120.0), Some(([3.0, 1.0], [6.0, 2.0]))),
        (lean(150.0, 60.0), Some(([3.0, 1.0], [6.0, 2.0]))),
        (lean(60.0, 30.0), Some(([1.0, -3.0], [2.0, -6.0]))),
        (lean(-60.0, 30.0), Some(([1.0, 3.0], [2.0, 6.0]))),
        (lean(60.0, -30.0), Some(([1.0, 3.0], [2.0, 6.0]))),
        // Irrational ratios, and an exact miss that rounds onto a half turn.
        (lean(30.0, 45.0), None),
        (lean(60.0, 45.0), None),
        (lean(30.0, 100.0), None),
        (lean(30.0, 30.000000000000004), None),
        (lean(2e-14, -179.99999999999997), None),
        // Tip as sheet, both angles set: N_a/N_b = τ₁/τ₂, read exactly from the doubles.
        (sheet(30.0, 20.0), Some(([20.0, -30.0], [4.0, -6.0]))),
        (sheet(-30.0, 20.0), Some(([20.0, 30.0], [4.0, 6.0]))),
        (sheet(30.0, -20.0), Some(([-20.0, -30.0], [-4.0, -6.0]))),
        (sheet(40.0, 70.0), Some(([70.0, -40.0], [7.0, -4.0]))),
        (sheet(0.1, 0.2), Some(([0.2, -0.1], [4.0, -2.0]))),
        (sheet(200.0, 30.0), Some(([30.0, -200.0], [3.0, -20.0]))),
        // Subnormal angles, 2^−1074 with 2^−1070 and with 2^−1022, read through their bits. Where the length is itself
        // subnormal, `along` is written on whole numbers, which no subnormal grid rounds.
        (
            sheet(f64::from_bits(1), f64::from_bits(16)),
            Some(([16.0, -1.0], [32.0, -2.0])),
        ),
        (
            sheet(f64::from_bits(1), f64::MIN_POSITIVE),
            Some((
                [f64::MIN_POSITIVE, -(f64::from_bits(1))],
                [2.0f64.powi(53), -2.0],
            )),
        ),
        (
            sheet(1e-300, 1e300),
            Some(([1e300, -1e-300], [f64::INFINITY, -2.0])),
        ),
    ]);
    cases
}

/// Each arm's strip, from the spec and from the params ([`TiltSpec::strip`], [`Tilt::new`]).
fn check_lattice(cases: &[LatticeCase]) {
    for (s, want) in cases {
        let want = want.map(|(along, w)| strip_of(along, w));
        let got = s.strip().map(|st| (st.dir, st.half));
        assert_eq!(got, want, "the lattice under {s:?}");
        assert_eq!(Tilt::new(s, CENTRE, SPAN).strip, s.strip(), "{s:?}");
    }
}

#[test]
fn tilt_every_chart_lattice_exact() {
    check_lattice(&lattice_cases());
    // 0.3 is not 3 × 0.1 in binary: a generator far past any view, and finite.
    let far = spec(0.1, 0.3, TiltMap::Sheet, 2, 2).strip().unwrap();
    assert!(far.half > 1e15 && far.half.is_finite(), "{far:?}");
}

negative_control!(
    tilt_every_chart_lattice_exact,
    "a lattice claimed at an irrational ratio, Lean 30°/45°, must fail",
    expected = "the lattice under",
    {
        check_lattice(&[(
            spec(30.0, 45.0, TiltMap::Lean, 2, 2),
            Some(([1.0, -1.0], [2.0, -2.0])),
        )]);
    }
);

/// The canonical strip's edges, half-open: `t` in `[−½, 3/2)` for `t`'s period, exactly at the edges and one ulp of the
/// offset past;
/// and along a diagonal, each term of the offset from `(½, ½)` read.
fn check_strip_edges(closed: bool) {
    let t = spec(30.0, 0.0, TiltMap::Sheet, 3, 3).strip().unwrap();
    let canonical = |a: f64, b: f64| {
        let p = (a - 0.5) * t.dir[0] + (b - 0.5) * t.dir[1];
        if closed {
            -t.half <= p && p <= t.half
        } else {
            t.canonical(a, b)
        }
    };
    let below = -0.5f64;
    let above = 1.5f64;
    assert!(canonical(0.5, below), "t = −½ is not canonical");
    assert!(
        !canonical(0.5, below - f64::EPSILON),
        "t just below −½ canonical"
    );
    assert!(
        canonical(0.5, above.next_down()),
        "t just below 3/2 not canonical"
    );
    assert!(!canonical(0.5, above), "t = 3/2 is canonical");
    assert!(!canonical(0.5, 2.0) && !canonical(0.5, -1.0));
    // Through the view: the pixel at t = −½ counts, the one at t = 3/2 does not.
    let view = spherical(&spec(30.0, 0.0, TiltMap::Sheet, 3, 3));
    assert_eq!(view.pixel(0.5, -0.5), Some(PixelCount::System));
    assert_eq!(view.pixel(0.5, 1.5), None);
    assert_eq!(view.pixel(0.5, 3.5), None);
    // Lean 45°/45°: w = (2, −2), |w|/2 = √2 along (1, −1)/√2.
    let d = spec(45.0, 45.0, TiltMap::Lean, 3, 3).strip().unwrap();
    for (a, b, inside) in [
        (1.9, 0.5, true),
        (0.5, 1.9, true),
        (1.9, 1.9, true),
        (2.9, 0.5, false),
        (0.5, 2.9, false),
        (-0.9, -0.9, true),
        (3.0, -1.0, false),
    ] {
        assert_eq!(
            d.canonical(a, b),
            inside,
            "({a}, {b}) on the diagonal strip"
        );
    }
}

#[test]
fn tilt_every_chart_lattice_strip_edges() {
    check_strip_edges(false);
}

negative_control!(
    tilt_every_chart_lattice_strip_edges,
    "a closed strip counts t = 3/2 with t = −½, so it must fail",
    expected = "t = 3/2 is canonical",
    {
        check_strip_edges(true);
    }
);

/// The direction of `(τ₂, −τ₁)` under the length the subnormal grid rounds to (the controls' form).
fn rounded_dir(t1: f64, t2: f64) -> [f64; 2] {
    let len = t2.hypot(t1);
    [t2 / len, -t1 / len]
}

/// The strip's `dir` at Tip as sheet's subnormal angles, given in ulps of `2⁻¹⁰⁷⁴`, is a unit vector along `(τ₂, −τ₁)`
/// (R-655). A length that rounds to exactly `2⁻¹⁰²²` is a normal length, so `dir` divides by it as it stands, at a pair
/// whose true length is below that by ≈0.4 of the grid. `rounded` keeps the rounded length throughout (the control).
fn check_subnormal_dir(rounded: bool) {
    let ulp = f64::from_bits;
    for (t1, t2) in [(1, 1), (2, 1), (3, 3), (1, 16), (1000, 1000), (1 << 51, 3)] {
        let (u1, u2) = (ulp(t1), ulp(t2));
        let dir = if rounded {
            rounded_dir(u1, u2)
        } else {
            spec(u1, u2, TiltMap::Sheet, 3, 3).strip().unwrap().dir
        };
        let length = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        assert!(
            (length - 1.0).abs() <= 4.0 * f64::EPSILON,
            "dir is not a unit vector at ({t1}, {t2}) ulps: {dir:?}, length {length}"
        );
        // Along (τ₂, −τ₁), read on the whole numbers.
        let cross = dir[0] * t1 as f64 + dir[1] * t2 as f64;
        assert!(
            cross.abs() <= 4.0 * f64::EPSILON * t1.max(t2) as f64,
            "dir is not along the generator at ({t1}, {t2}) ulps: {dir:?}"
        );
    }
    let (t1, t2) = (ulp(0xc_cccc_cccc_ccbd), ulp(0x9_9999_9999_99ae));
    assert_eq!(
        spec(t1, t2, TiltMap::Sheet, 3, 3).strip().unwrap().dir,
        [t2 / f64::MIN_POSITIVE, -t1 / f64::MIN_POSITIVE],
        "a length of exactly 2^-1022 is not divided as it stands"
    );
}

#[test]
fn tilt_every_chart_lattice_subnormal_dir_unit() {
    check_subnormal_dir(false);
}

negative_control!(
    tilt_every_chart_lattice_subnormal_dir_unit,
    "a dir taken from the length the subnormal grid rounds is off unit length, so it must fail",
    expected = "dir is not a unit vector at (1, 1) ulps",
    {
        check_subnormal_dir(true);
    }
);

/// At `τ₁ = τ₂ = 2⁻¹⁰⁷⁴` under Tip as sheet, `w = (2, −2)`, `|w|/2 = √2`: of the orbit `(1.4, −0.4) + j·w` the point with
/// its offset along `w` in `[−√2, √2)` is `(1.4, −0.4)`, and the orbit is counted once there (R-655). `rounded` judges
/// the orbit with the rounded length's `dir` (the control).
fn check_subnormal_orbit(rounded: bool) {
    let t = f64::from_bits(1);
    let s = spec(t, t, TiltMap::Sheet, 3, 3);
    let view = spherical(&s);
    let orbit = [[1.4, -0.4], [-0.6, 1.6], [3.4, -2.4]];
    let canonical: Vec<bool> = orbit
        .iter()
        .map(|&[a, b]| {
            if rounded {
                let dir = rounded_dir(t, t);
                let half = 2f64.sqrt();
                let p = (a - 0.5) * dir[0] + (b - 0.5) * dir[1];
                -half <= p && p < half
            } else {
                view.pixel(a, b) == Some(PixelCount::System)
            }
        })
        .collect();
    assert_eq!(
        canonical,
        [true, false, false],
        "the orbit is not counted once, at its canonical point"
    );
    assert_eq!(view.counted_total(orbit.iter().copied()), 1);
}

#[test]
fn tilt_every_chart_lattice_subnormal_orbit_once() {
    check_subnormal_orbit(false);
}

negative_control!(
    tilt_every_chart_lattice_subnormal_orbit_once,
    "a dir off unit length pushes the orbit's canonical point past the strip, uncounted, so it must fail",
    expected = "the orbit is not counted once",
    {
        check_subnormal_orbit(true);
    }
);

/// `[N_a = 0, N_b = 0]` on the stored angles (R-554 L3), at each arm's boundary.
fn check_normal_zero(cases: &[(TiltSpec, [bool; 2])]) {
    for (s, want) in cases {
        assert_eq!(s.normal_zero(), *want, "N = 0 under {s:?}");
        let t = Tilt::new(s, CENTRE, SPAN);
        assert_eq!(t.normal_zero, *want, "the params' N = 0 under {s:?}");
    }
}

/// The arms: untilted; a single angle under either map (Lean's closed form), at 0, ±180 and between; Lean with both set,
/// at the dead zone `|τ| = 90`; RQ-471's corner, no normal; Tip as sheet at `m == 180` and beside it; two targets per
/// offset.
fn normal_zero_cases() -> Vec<(TiltSpec, [bool; 2])> {
    let one = |t1, t2, map| spec(t1, t2, map, 2, 2);
    let two = |t1, t2, map| spec(t1, t2, map, 2, 3);
    let mut cases = vec![(one(0.0, 0.0, TiltMap::Sheet), [true, true])];
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        cases.extend([
            (one(30.0, 0.0, map), [false, true]),
            (one(0.0, 30.0, map), [true, false]),
            (one(180.0, 0.0, map), [true, true]),
            (one(-180.0, 0.0, map), [true, true]),
            (one(0.0, -180.0, map), [true, true]),
            (one(90.0, 0.0, map), [false, true]),
            (one(0.0, 90.0, map), [true, false]),
            (two(30.0, 0.0, map), [false, true]),
            (two(0.0, 30.0, map), [true, false]),
            (two(180.0, -180.0, map), [true, true]),
            (two(-180.0, 30.0, map), [true, false]),
            (two(90.0, 90.0, map), [false, false]),
        ]);
    }
    cases.extend([
        (one(30.0, 90.0, TiltMap::Lean), [true, false]),
        (one(30.0, -90.0, TiltMap::Lean), [true, false]),
        (one(90.0, 30.0, TiltMap::Lean), [false, true]),
        (one(-90.0, 30.0, TiltMap::Lean), [false, true]),
        (one(30.0, 45.0, TiltMap::Lean), [false, false]),
        (one(90.0, 90.0, TiltMap::Lean), [false, false]),
        (one(-90.0, 90.0, TiltMap::Lean), [false, false]),
        (one(108.0, 144.0, TiltMap::Sheet), [true, true]),
        (one(-108.0, 144.0, TiltMap::Sheet), [true, true]),
        (one(108.0, 144.000001, TiltMap::Sheet), [false, false]),
        (one(90.0, 90.0, TiltMap::Sheet), [false, false]),
        (one(180.0, 30.0, TiltMap::Sheet), [false, false]),
        (one(30.0, 90.0, TiltMap::Sheet), [false, false]),
    ]);
    cases
}

#[test]
fn tilt_every_chart_normal_zero_exact() {
    check_normal_zero(&normal_zero_cases());
}

negative_control!(
    tilt_every_chart_normal_zero_exact,
    "Tip as sheet beside m = 180 claimed to keep d must fail",
    expected = "N = 0 under",
    {
        check_normal_zero(&[(spec(108.0, 144.000001, TiltMap::Sheet, 2, 2), [true, true])]);
    }
);

// ── RQ-471's corner over the registry's five (R-554) ──────────────────────────────────────────────────────────────────

/// What a chart reports for a stored tilt: its `system_image`, each pixel's area weight and a share.
struct Report {
    image: SystemImage,
    weights: Vec<f64>,
}

/// A construction chart's report: the tilted view's `system_image` from the chart's own `image`, and each pixel's area
/// weight, 0 on a refused cell ([`Tilted::weight`]).
fn per_cell<C: Tiltable<Param = f64>>(view: &Tilted<C, f64>, image: SystemImage) -> Report {
    Report {
        image: view.tilt().system_image(image),
        weights: grid(N, 0.0, 1.0)
            .iter()
            .map(|&[u, v]| view.weight(u, v))
            .collect(),
    }
}

/// The registry's chart `name` tilted by `s` (targets its own hidden coordinates), its untilted `system_image`
/// `untilted` where the chart declares none.
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
        "MassSimplex" => per_cell(&held_view(simplex(), &with(NU0), NU_SPAN), untilted),
        "EuclidPlane" => per_cell(
            &held_view(euclid(), &with(2), SPAN[0]),
            euclid().chart.system_image(),
        ),
        "BurrauThetaDeltaM" => per_cell(
            &held_view(delta_m(), &with(4), SPAN[0]),
            delta_m().chart.system_image(),
        ),
        "Burrau" => per_cell(
            &held_view(nu_k(), &with(HELD_LZ), LZ_SPAN),
            nu_k().chart.system_image(),
        ),
        "BurrauThetaK" => per_cell(
            &held_view(theta_k(), &with(HELD_LZ), LZ_SPAN),
            theta_k().chart.system_image(),
        ),
        "BurrauThetaLz" => per_cell(
            &held_view(theta_lz(), &with(HELD_K), K_SPAN),
            theta_lz().system_image(),
        ),
        other => panic!("{other} is not a registered chart"),
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

/// Under Lean each axis with one target at 90°/90° and −90°/90°, over the whole registry: `ray-degenerate`, no fold,
/// every pixel's area weight 0, every share undefined.
fn check_corner(alter: Corner) {
    let every: Vec<&str> = REGISTRY
        .iter()
        .copied()
        .filter(|n| FIVE.contains(n) || SIX.contains(n))
        .collect();
    assert_eq!(every, REGISTRY, "the registry is the five and the six");
    for (t1, t2) in [(90.0, 90.0), (-90.0, 90.0)] {
        let s = spec(t1, t2, TiltMap::Lean, 0, 0);
        for name in &every {
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
        normal_zero: t.normal_zero,
        strip: t.strip,
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
    for name in REGISTRY {
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
            "MassSimplex" => slice_rest(simplex(), &[NU0], false),
            "EuclidPlane" => slice_rest(euclid(), &[], false),
            "BurrauThetaDeltaM" => slice_rest(delta_m(), &[], false),
            "Burrau" => {
                slice_lz(nu_k(), false);
                slice_lz(linear(nu_k()), false);
            }
            "BurrauThetaK" => {
                slice_lz(theta_k(), false);
                slice_lz(linear(theta_k()), false);
            }
            "BurrauThetaLz" => slice_k(),
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

mod slice_warp_lz {
    use super::*;
    negative_control!(
        slice_every_chart,
        "a slice of the held L_z moved in a warp coordinate is not L_z₀ + d, so it must fail",
        expected = "is not L_z₀ + d",
        {
            slice_lz(nu_k(), true);
        }
    );
}

mod slice_momenta {
    use super::*;
    negative_control!(
        slice_every_chart,
        "a rest start's momentum control sliced into a physical momentum is not D_mom of it, so it must fail",
        expected = "is not D_mom of the control",
        {
            slice_rest(euclid(), &[], true);
        }
    );
}

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

// ── R-633: the six charts that hold a block by a construction ─────────────────────────────────────────────────────────

/// `ν₀`'s slider span, a test's own: the corpus gives none (R-507 U1 names the slider span as the unit; the span of the
/// physical hidden coordinates is queued), so the kernel takes it as an input.
const NU_SPAN: f64 = 0.4;
/// The held `L_z`'s slider span, a test's own (as `NU_SPAN`).
const LZ_SPAN: f64 = 2.0;
/// The held `K`'s slider span, a test's own (as `NU_SPAN`).
const K_SPAN: f64 = 2.0;
/// `Φ_θ,L_z`'s held `K`.
const THETA_LZ_K: f64 = 0.5;

fn simplex() -> RestStart<MassSimplex<f64>, f64> {
    RestStart::new(MassSimplex {
        nu0: 0.5,
        eps_m: 1e-4,
    })
}

fn euclid() -> RestStart<EuclidPlane<f64>, f64> {
    RestStart::new(EuclidPlane {
        m_range: [1.0, 32.0],
        nu_range: [1.0 / 32.0, 31.0 / 32.0],
    })
}

fn delta_m() -> RestStart<BurrauThetaDeltaM<f64>, f64> {
    RestStart::new(BurrauThetaDeltaM {
        theta_range: [0.2, std::f64::consts::FRAC_PI_4],
        m_target: BurrauThetaDeltaM::equal_masses(),
    })
}

fn nu_k() -> HeldLz<Burrau<f64>, f64> {
    HeldLz::new(Burrau {
        nu_range: [0.1, 0.9],
        k_max: 1.0,
        gamma_k: 1.5,
    })
}

fn theta_k() -> HeldLz<BurrauThetaK<f64>, f64> {
    HeldLz::new(BurrauThetaK {
        theta_range: [0.2, std::f64::consts::FRAC_PI_4],
        k_max: 1.0,
        gamma_k: 1.5,
    })
}

fn theta_lz() -> BurrauThetaLz<f64> {
    BurrauThetaLz {
        theta_range: [0.2, std::f64::consts::FRAC_PI_4],
        k: THETA_LZ_K,
        lz_fraction: 1.0,
    }
}

/// `Φ_θ,L_z` warped, the default, under a warp of the test's own.
fn held_k() -> HeldK<f64> {
    HeldK::new(theta_lz(), 1.0, 1.5)
}

/// `chart` under the linear option, R-471 E2.
fn linear<C>(chart: HeldLz<C, f64>) -> HeldLz<C, f64> {
    HeldLz {
        mapping: Mapping::Linear,
        ..chart
    }
}

/// `chart` under `s` about `CENTRE`, each target's slider span `span`.
fn held_view<C: Tiltable<Param = f64>>(chart: C, s: &TiltSpec, span: f64) -> Tilted<C, f64> {
    Tilted::new(chart, Tilt::new(s, CENTRE, [span, span])).expect("a hidden target")
}

/// Runs `$body` on each of the six, `$c` the chart with its hidden coordinates, `$own` the registered chart and `$x`,
/// `$span` each hidden coordinate the arm takes with its slider span.
macro_rules! for_six {
    (|$c:ident, $own:ident, $x:ident, $span:ident| $body:block) => {{
        {
            let $c = simplex();
            let $own = $c.chart;
            for ($x, $span) in [(NU0, NU_SPAN), (2, SPAN[0]), (5, SPAN[0])] {
                $body
            }
        }
        {
            let $c = euclid();
            let $own = $c.chart;
            for ($x, $span) in [(3, SPAN[0])] {
                $body
            }
        }
        {
            let $c = delta_m();
            let $own = $c.chart;
            for ($x, $span) in [(4, SPAN[0])] {
                $body
            }
        }
        {
            let $c = nu_k();
            let $own = $c.chart;
            for ($x, $span) in [(HELD_LZ, LZ_SPAN)] {
                $body
            }
        }
        {
            let $c = theta_k();
            let $own = $c.chart;
            for ($x, $span) in [(HELD_LZ, LZ_SPAN)] {
                $body
            }
        }
        {
            let $c = theta_lz();
            let $own = $c;
            for ($x, $span) in [(HELD_K, K_SPAN)] {
                $body
            }
        }
        {
            let $c = held_k();
            let $own = $c.chart;
            for ($x, $span) in [(HELD_K, K_SPAN)] {
                $body
            }
        }
    }};
}

/// The mass simplex's decode `d` as the pre-R-633 reading has it: its configuration's `(α, β)`, so `z_α`, held at the
/// centre's value `centre` across the window, under the pixel's own masses.
fn held_alpha(d: &Decoded<f64>, centre: &Decoded<f64>) -> Decoded<f64> {
    let (rho_t, lambda_t) = config::hyperspherical(centre.config.alpha, centre.config.beta);
    let (rho, lambda, r) = config::reconstruct(&d.masses, rho_t, lambda_t);
    let mut out = *d;
    out.config.rho = rho;
    out.config.lambda = lambda;
    out.config.r = r;
    out.ic.r = r;
    out
}

/// `d = 0` into the hidden coordinate `x`: the decode with a zero offset, the chart untilted under both maps and the
/// view at `τ = 0`, each bit for bit the registered chart `own`'s over a 33 × 33 grid. `pre` reads the mass simplex as
/// the pre-R-633 reading does, `z_α` held at the centre's value (the control).
fn check_zero<C, O>(chart: C, own: O, x: u32, span: f64, pre: bool)
where
    C: Tiltable<Param = f64> + ChartDecode + Copy,
    O: ChartDecode<Param = f64> + Copy,
{
    let zero = Shift {
        x,
        dx: 0.0,
        y: x,
        dy: 0.0,
        two: false,
    };
    let centre = own.decode::<Defaults, f64>(CENTRE[0], CENTRE[1]);
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        let view = held_view(chart, &spec(0.0, 0.0, map, x, x), span);
        for [u, v] in grid(N, 0.0, 1.0) {
            let want = bits(&own.decode::<Defaults, f64>(u, v));
            let mut got = chart.decode_shifted::<Defaults, f64>(u, v, &zero, &LinkParams::DEFAULT);
            if pre {
                got = held_alpha(&got, &centre);
            }
            assert!(
                bits(&got) == want
                    && bits(&chart.decode::<Defaults, f64>(u, v)) == want
                    && bits(&chart.decode_at::<Defaults, f64>(u, v)) == want
                    && bits(&chart.decode_at_params::<Defaults, f64>(u, v, &LinkParams::DEFAULT))
                        == want
                    && bits(&view.decode::<Defaults, f64>(u, v)) == want
                    && view.validate(u, v) == own.validate(u, v),
                "{} at ({u}, {v}): d = 0 into {x} is not the untilted chart",
                own.name()
            );
        }
    }
}

/// A 30° tilt into `ν₀`: each pixel's decode is the chart's at `(a, b)` on the triangle at `ν₀ + d`, `d` its
/// offset, and its masses are the chart map's at `(a, b)`, bit for bit.
fn check_nu0() {
    let chart = simplex();
    let view = held_view(chart, &spec(30.0, 0.0, TiltMap::Sheet, NU0, NU0), NU_SPAN);
    let mut moved = 0;
    for [u, v] in grid(17, 0.0, 1.0) {
        let ([a, b], shift) = view.tilt().place(u, v);
        let got = view.decode::<Defaults, f64>(u, v);
        let at = MassSimplex {
            nu0: chart.chart.nu0 + shift.dx,
            ..chart.chart
        };
        assert!(
            bits(&got) == bits(&at.decode::<Defaults, f64>(a, b)),
            "the mass simplex at ({u}, {v}) does not move ν by its d"
        );
        assert!(
            format!("{:?}", got.ic.m)
                == format!("{:?}", chart.chart.decode::<Defaults, f64>(a, b).ic.m),
            "the mass simplex at ({u}, {v}) moved its masses"
        );
        assert_eq!(view.validate(u, v), ValidationResult::Valid);
        moved += usize::from(shift.dx != 0.0);
    }
    assert!(moved > 250, "too few pixels moved: {moved}");
}

#[test]
fn tilt_every_chart_construction_held() {
    for_six!(|c, own, x, span| { check_zero(c, own, x, span, false) });
    check_nu0();
}

negative_control!(
    tilt_every_chart_construction_held,
    "the mass simplex's z_α held at the centre's value across the window, the pre-R-633 reading, must fail d = 0",
    expected = "is not the untilted chart",
    {
        check_zero(simplex(), simplex().chart, NU0, NU_SPAN, true);
    }
);

/// A 30° tilt into the momentum control `k` of a rest start: each pixel's momenta are `D_mom` of the control at `d`,
/// the others 0, under the pixel's masses, its masses and configuration the chart's at `(a, b)`. `physical` adds `d`
/// to a physical momentum component instead (the control).
fn check_rest<C: RestChart<Param = f64>>(chart: RestStart<C, f64>, k: u32, physical: bool) {
    let view = held_view(chart, &spec(30.0, 0.0, TiltMap::Sheet, k, k), SPAN[0]);
    let mut moving = 0;
    for [u, v] in grid(17, 0.0, 1.0) {
        let ([a, b], shift) = view.tilt().place(u, v);
        let own = chart.chart.decode::<Defaults, f64>(a, b);
        let mut got = view.decode::<Defaults, f64>(u, v);
        if physical {
            got = own;
            got.ic.p[1][0] += shift.dx;
        }
        let mut z = [0.0; 4];
        z[(k - 2) as usize] = shift.dx;
        let want = momentum::decode::<Defaults, f64>(z, &own.masses);
        assert!(
            got.ic.p == want.p && got.momentum.q == want.q,
            "{} at ({u}, {v}): the momenta are not D_mom of the control from 0",
            chart.name()
        );
        assert!(
            format!("{:?}", (got.masses, got.config)) == format!("{:?}", (own.masses, own.config)),
            "{} at ({u}, {v}): the masses or the configuration moved",
            chart.name()
        );
        moving += usize::from(want.p != [[0.0; 2]; 3]);
    }
    assert!(moving > 250, "too few pixels moving: {moving}");
}

#[test]
fn tilt_every_chart_construction_rest_momentum() {
    for k in 2..6 {
        check_rest(simplex(), k, false);
        check_rest(euclid(), k, false);
        check_rest(delta_m(), k, false);
    }
}

negative_control!(
    tilt_every_chart_construction_rest_momentum,
    "a d added to a physical momentum for a latent target is not D_mom of the control, so it must fail",
    expected = "are not D_mom of the control from 0",
    {
        check_rest(euclid(), 2, true);
    }
);

/// `L_z = Σ rᵢ × pᵢ` and `K = Σ |pᵢ|²/2mᵢ` of a decode, and `|Σpᵢ|`.
fn invariants(d: &Decoded<f64>) -> (f64, f64, f64) {
    let (r, p, m) = (d.ic.r, d.ic.p, d.ic.m);
    let lz = (0..3).map(|i| r[i][0] * p[i][1] - r[i][1] * p[i][0]).sum();
    let k = (0..3)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
        .sum();
    let drift =
        (0..3).map(|i| p[i][0]).sum::<f64>().abs() + (0..3).map(|i| p[i][1]).sum::<f64>().abs();
    (lz, k, drift)
}

/// How the refused arm reads a cell, for a control.
#[derive(Clone, Copy, PartialEq)]
enum Refused {
    AsBuilt,
    Counted,
    Coloured,
}

/// A 30° tilt into the held value of a Burrau chart in linear mode, `held(a, b, d)` the pixel's `(L_z, K)`: a cell with
/// `K < L_z²/2I` is no system (tagged `INVARIANT_INFEASIBLE`, refused by `validate`, area weight 0), every other cell
/// valid, weighted as the view, its momenta §2.2's construction at `(L_z, K)`: `Σpᵢ = 0`, `L_z(p) = L_z`, `K(p) = K`.
fn check_refused<C: Tiltable<Param = f64> + Copy>(
    chart: C,
    x: u32,
    span: f64,
    held: impl Fn(f64, f64, f64) -> (f64, f64),
    alter: Refused,
) {
    let view = held_view(chart, &spec(30.0, 0.0, TiltMap::Sheet, x, x), span);
    let mut counts = [0; 2];
    for [u, v] in grid(33, 0.0, 1.0) {
        let ([a, b], shift) = view.tilt().place(u, v);
        let (lz, k) = held(a, b, shift.dx);
        let d = view.decode::<Defaults, f64>(u, v);
        let mut weight = view.weight(u, v);
        let mut valid = view.validate(u, v);
        if alter == Refused::Counted {
            weight = view.tilt().weight;
        } else if alter == Refused::Coloured {
            valid = view.chart().validate(a, b);
        }
        let name = chart.name();
        assert!(!view.held_at_floor(u, v), "{name} at ({u}, {v})");
        if infeasible(1.0, k, lz) {
            counts[0] += 1;
            assert_eq!(
                d.degenerate,
                Some(Degenerate::InvariantInfeasible),
                "{name} at ({u}, {v})"
            );
            assert!(
                valid
                    == ValidationResult::Reject {
                        failed: Constraint::KAboveLzBound
                    },
                "{name} at ({u}, {v}): a refused cell is a system"
            );
            assert!(
                weight == 0.0,
                "{name} at ({u}, {v}): a refused cell is counted"
            );
        } else {
            counts[1] += 1;
            assert_eq!(d.degenerate, None, "{name} at ({u}, {v})");
            assert_eq!(valid, ValidationResult::Valid, "{name} at ({u}, {v})");
            assert_eq!(weight, view.tilt().weight, "{name} at ({u}, {v})");
            let (got_lz, got_k, drift) = invariants(&d);
            let tol = 1e-12 * (1.0 + k);
            assert!(
                (got_lz - lz).abs() <= tol && (got_k - k).abs() <= tol && drift <= tol,
                "{name} at ({u}, {v}): not §2.2's construction at (L_z, K) = ({lz}, {k}): ({got_lz}, {got_k}), drift {drift}"
            );
        }
    }
    assert!(
        counts.iter().all(|&c| c > 30),
        "{}: every case met: {counts:?}",
        chart.name()
    );
}

/// The linear arm on the three Burrau charts (R-708): rows at `K_max·b` or on the centre's band, a held value by value.
fn check_burrau_refused(alter: Refused) {
    let c = linear(nu_k());
    check_refused(c, HELD_LZ, LZ_SPAN, |_, b, d| (d, c.chart.k_max * b), alter);
    let c = linear(theta_k());
    check_refused(c, HELD_LZ, LZ_SPAN, |_, b, d| (d, c.chart.k_max * b), alter);
    let c = theta_lz();
    check_refused(
        c,
        HELD_K,
        K_SPAN,
        |_, b, d| (c.lz(b), THETA_LZ_K + d),
        alter,
    );
    let held = HeldK {
        mapping: Mapping::Linear,
        ..held_k()
    };
    check_refused(
        held,
        HELD_K,
        K_SPAN,
        |_, b, d| (c.lz(b), THETA_LZ_K + d),
        alter,
    );
}

/// The tilt into the held `L_z` is the chart sliced at each pixel's `L_z + d`, bit for bit.
fn check_tilt_is_slice<C: LzChart<Param = f64>>(chart: HeldLz<C, f64>) {
    let view = held_view(
        chart,
        &spec(30.0, 0.0, TiltMap::Sheet, HELD_LZ, HELD_LZ),
        LZ_SPAN,
    );
    for [u, v] in grid(9, 0.0, 1.0) {
        let ([a, b], shift) = view.tilt().place(u, v);
        let sliced = chart.sliced(HELD_LZ, shift.dx).unwrap();
        assert_eq!(
            bits(&view.decode::<Defaults, f64>(u, v)),
            bits(&sliced.decode::<Defaults, f64>(a, b)),
            "{} at ({u}, {v})",
            chart.name()
        );
    }
}

#[test]
fn tilt_every_chart_construction_refused() {
    check_burrau_refused(Refused::AsBuilt);
    check_tilt_is_slice(nu_k());
    check_tilt_is_slice(theta_k());
    check_tilt_is_slice(linear(nu_k()));
    check_tilt_is_slice(linear(theta_k()));
    // At L_z = 0 the linear arm is the mapped chart's linear option, bit for bit.
    let mapping = [Mapping::Warped, Mapping::Linear];
    let (burrau, acute) = (
        Mapped::<_, Defaults>::new(nu_k().chart, mapping),
        Mapped::<_, Defaults>::new(theta_k().chart, mapping),
    );
    for [u, v] in grid(9, 0.0, 1.0) {
        assert_eq!(
            bits(&linear(nu_k()).decode::<Defaults, f64>(u, v)),
            bits(&burrau.decode::<Defaults, f64>(u, v))
        );
        assert_eq!(
            bits(&linear(theta_k()).decode::<Defaults, f64>(u, v)),
            bits(&acute.decode::<Defaults, f64>(u, v))
        );
    }
}

negative_control!(
    tilt_every_chart_construction_refused,
    "a refused cell counted in the area statistics must fail",
    expected = "a refused cell is counted",
    {
        check_burrau_refused(Refused::Counted);
    }
);

mod refused_coloured {
    use super::*;
    negative_control!(
        tilt_every_chart_construction_refused,
        "a refused cell coloured as a system, read at the held value of the centre, must fail",
        expected = "a refused cell is a system",
        {
            check_burrau_refused(Refused::Coloured);
        }
    );
}

/// How the warped arms decode a pixel, for a control.
#[derive(Clone, Copy, PartialEq)]
enum Warped {
    AsBuilt,
    /// The floor at the centre's held `L_z`, not the pixel's.
    CentreFloor,
    /// The held `L_z` moved in the warp coordinate `s`, `L_z = (2s − 1)·L_max(K)`.
    WarpLz,
    /// The held `L_z` held to one sign.
    OneSign,
    /// The held `K` moved by value.
    ByValue,
    /// `t < 0` mirrored onto `t > 0`.
    Mirror,
}

/// The pixels of a 33 × 33 grid over the square, and its bottom and top rows themselves, `t = 0` and `t = 1`.
fn rows() -> Vec<[f64; 2]> {
    let rows = (0..33).flat_map(|i| [[pixel_centre(i, 33), 0.0], [pixel_centre(i, 33), 1.0]]);
    grid(33, 0.0, 1.0).into_iter().chain(rows).collect()
}

/// The tilts of the warped arms: Tip as sheet on `τ₁`, which keeps the rows, and Lean each axis on `τ₂`.
fn warped_tilts(x: u32) -> [TiltSpec; 2] {
    [
        spec(30.0, 0.0, TiltMap::Sheet, x, x),
        spec(0.0, 30.0, TiltMap::Lean, x, x),
    ]
}

/// Warped, a tilt into the held `L_z` about `centre`: no cell refused, every one §2.2's construction at the held
/// `L_z₀ + d` by value, at both signs of `d`, and `K = L_z²/2I + K_max·t^γ_K` at the pixel's own `L_z`, past `K_max` at
/// the top where the floor takes it there.
fn check_warped_lz<C: LzChart<Param = f64>>(chart: HeldLz<C, f64>, centre: f64, alter: Warped) {
    let chart = HeldLz {
        lz: centre,
        ..chart
    };
    let own = chart.chart;
    let name = chart.name();
    let mut above = 0;
    for s in warped_tilts(HELD_LZ) {
        let view = held_view(chart, &s, LZ_SPAN);
        let mut signs = [0; 2];
        for [u, v] in rows() {
            let ([a, b], shift) = view.tilt().place(u, v);
            let lz = centre + shift.dx;
            let line = own.kinetic_at(b);
            let on = |moved: f64| own.decode_lz(a, moved, k_floor(moved) + line);
            let d = if alter == Warped::CentreFloor {
                own.decode_lz(a, lz, k_floor(centre) + line)
            } else if alter == Warped::WarpLz {
                on(centre + 2.0 * shift.dx * l_max(k_floor(lz) + line))
            } else if alter == Warped::OneSign {
                on(lz.abs())
            } else {
                view.decode::<Defaults, f64>(u, v)
            };
            assert_eq!(
                d.degenerate, None,
                "{name} at ({u}, {v}): a cell refused under tilt"
            );
            assert_eq!(view.validate(u, v), ValidationResult::Valid, "{name}");
            assert_eq!(view.chart().refusal(a, b, &shift), None, "{name}");
            assert_eq!(view.weight(u, v), view.tilt().weight, "{name}");
            let k = k_floor(lz) + line;
            let (got_lz, got_k, drift) = invariants(&d);
            let tol = 1e-12 * (1.0 + k);
            assert!(
                (got_lz - lz).abs() <= tol,
                "{name} at ({u}, {v}): the held L_z {got_lz} is not L_z₀ + d = {lz}"
            );
            assert!(
                (got_k - k).abs() <= tol && drift <= tol,
                "{name} at ({u}, {v}): K = {got_k} is not the floor plus the warp's line, {k}"
            );
            signs[usize::from(shift.dx > 0.0)] += 1;
            above += usize::from(k > own.k_max_at::<f64>());
        }
        assert!(
            signs.iter().all(|&c| c > 100),
            "{name}: both signs of d: {signs:?}"
        );
    }
    assert!(above > 0, "{name}: a column above K_max");
}

/// The held `L_z` arm on `(ν, K)` and `Φ_θ,K` (`Φ_strip,K` the same map), at a centre at 0 and at each sign.
fn check_warped_lz_all(alter: Warped) {
    for centre in [0.0, 0.15, -0.3] {
        check_warped_lz(nu_k(), centre, alter);
        check_warped_lz(theta_k(), centre, alter);
    }
}

#[test]
fn tilt_every_chart_construction_warped_lz() {
    check_warped_lz_all(Warped::AsBuilt);
}

negative_control!(
    tilt_every_chart_construction_warped_lz,
    "a floor computed at the centre's held L_z rather than the pixel's leaves cells with no construction, so it must fail",
    expected = "a cell refused under tilt",
    {
        check_warped_lz_all(Warped::CentreFloor);
    }
);

mod warped_lz_warp {
    use super::*;
    negative_control!(
        tilt_every_chart_construction_warped_lz,
        "a held L_z moved in a warp coordinate is not L_z₀ + d, so it must fail",
        expected = "is not L_z₀ + d",
        {
            check_warped_lz_all(Warped::WarpLz);
        }
    );
}

mod warped_lz_sign {
    use super::*;
    negative_control!(
        tilt_every_chart_construction_warped_lz,
        "a held L_z held to one sign is not L_z₀ + d, so it must fail",
        expected = "is not L_z₀ + d",
        {
            check_warped_lz_all(Warped::OneSign);
        }
    );
}

/// Warped, a tilt into the held `K₀` of `Φ_θ,L_z` under the warp `(k_max, gamma)`: no cell refused, the held `K ≥ 0`, the pixel's
/// `K = K_max·t^γ_K` at `t = t₀ + d` and the band at it. Where `t₀ + d < 0` the pixel is clamped at `t = 0`: a real
/// system, the rest start at its own shape, flagged, never mirrored (R-709). The clamped pixels and the pixels counted.
fn check_warped_k(k0: f64, k_max: f64, gamma: f64, alter: Warped) -> (usize, usize) {
    let chart = HeldK::new(
        BurrauThetaLz {
            k: k0,
            ..theta_lz()
        },
        k_max,
        gamma,
    );
    let c = chart.chart;
    let t0 = (k0 / k_max).powf(1.0 / gamma);
    let (mut clamped, mut total) = (0, 0);
    for s in warped_tilts(HELD_K) {
        let view = held_view(chart, &s, K_SPAN);
        for [u, v] in rows() {
            let ([a, b], shift) = view.tilt().place(u, v);
            let t = t0 + shift.dx;
            let d = if alter == Warped::ByValue {
                burrau_acute::decode_theta_lz_at(&c, a, c.lz(b), k0 + shift.dx).0
            } else if alter == Warped::Mirror {
                let k = k_max * t.abs().powf(gamma);
                burrau_acute::decode_theta_lz_at(&c, a, c.lz_at(b, k), k).0
            } else {
                view.decode::<Defaults, f64>(u, v)
            };
            assert_eq!(
                d.degenerate, None,
                "Φ_θ,L_z at ({u}, {v}): a cell refused under tilt"
            );
            assert_eq!(view.validate(u, v), ValidationResult::Valid);
            assert_eq!(view.chart().refusal(a, b, &shift), None);
            assert_eq!(view.weight(u, v), view.tilt().weight);
            let k = if shift.dx == 0.0 {
                k0
            } else {
                k_max * t.max(0.0).powf(gamma)
            };
            let lz = c.lz_at(b, k);
            let (got_lz, got_k, drift) = invariants(&d);
            assert_eq!(view.held_at_floor(u, v), t < 0.0, "at ({u}, {v})");
            if t < 0.0 {
                clamped += 1;
                assert!(
                    d.ic.p == [[0.0; 2]; 3] && got_k == 0.0 && got_lz == 0.0,
                    "at ({u}, {v}): a clamped pixel is not the rest start at its floor: {:?}",
                    d.ic.p
                );
            }
            let tol = 1e-12 * (1.0 + k);
            assert!(
                got_k >= 0.0
                    && (got_k - k).abs() <= tol
                    && (got_lz - lz).abs() <= tol
                    && drift <= tol,
                "at ({u}, {v}): not the warp's K = {k}, L_z = {lz}: ({got_k}, {got_lz})"
            );
            total += 1;
        }
    }
    (clamped, total)
}

/// The held `K` arm: a centre no tilt here takes below `t = 0` and one it does, under two `K_max`, at integer and
/// non-integer `γ_K`.
fn check_warped_k_all(alter: Warped) -> [(usize, usize); 2] {
    let mut sums = [(0, 0); 2];
    for gamma in [1.0, 1.5, 2.0, 2.5] {
        for (i, k0, k_max) in [(0, 0.9, 1.0), (1, 0.01, 1.0), (1, 0.01, 2.0)] {
            let (clamped, total) = check_warped_k(k0, k_max, gamma, alter);
            sums[i].0 += clamped;
            sums[i].1 += total;
        }
    }
    sums
}

#[test]
fn tilt_every_chart_construction_warped_k() {
    let [high, low] = check_warped_k_all(Warped::AsBuilt);
    assert_eq!(high.0, 0, "no pixel clamped at K₀ = 0.9");
    assert!(low.0 > 100, "the clamp arises: {low:?}");
    // R-709's report, written past the harness's capture so that the test output shows it.
    writeln!(
        std::io::stderr(),
        "R-709 clamped-pixel share: K₀ = 0.9: {}/{}; K₀ = 0.01: {}/{} = {:.4}",
        high.0,
        high.1,
        low.0,
        low.1,
        low.0 as f64 / low.1 as f64
    )
    .unwrap();
}

negative_control!(
    tilt_every_chart_construction_warped_k,
    "a held K moved by value leaves K < 0, cells with no construction, so it must fail",
    expected = "a cell refused under tilt",
    {
        check_warped_k_all(Warped::ByValue);
    }
);

mod warped_k_mirror {
    use super::*;
    negative_control!(
        tilt_every_chart_construction_warped_k,
        "t < 0 mirrored onto t > 0 is not the clamp, so it must fail",
        expected = "a clamped pixel is not the rest start",
        {
            check_warped_k_all(Warped::Mirror);
        }
    );
}

/// `L_z` values over thirty decades, both signs, none rounded to a tidy `l²/2`.
fn lz_scan() -> impl Iterator<Item = f64> {
    (1..4096).map(|i| {
        let x = (f64::from(i) * 0.618_033_988_749_894_9).fract();
        let magnitude = 10f64.powf(30.0 * x - 27.0);
        if i % 2 == 0 {
            magnitude
        } else {
            -magnitude
        }
    })
}

/// The bottom row, `t = 0`, at the pixel's own held `L_z`, is a system at every `L_z`, at f64 and at f32, on §2.2's exact
/// test. `rounded` takes the rounded `l²/2` for the floor, leaving the row to the rounding (the control).
fn check_bottom_row(rounded: bool) {
    let (narrow, acute) = (
        HeldLz::new(Burrau::<f32> {
            nu_range: [0.1, 0.9],
            k_max: 1.0,
            gamma_k: 1.5,
        }),
        HeldLz::new(BurrauThetaK::<f32> {
            theta_range: [0.2, std::f32::consts::FRAC_PI_4],
            k_max: 1.0,
            gamma_k: 1.5,
        }),
    );
    for lz in lz_scan() {
        let wide = nu_k().sliced(HELD_LZ, lz).unwrap();
        let d = if rounded {
            wide.chart.decode_lz(0.5, lz, 0.5 * lz * lz)
        } else {
            wide.decode::<Defaults, f64>(0.5, 0.0)
        };
        assert_eq!(
            d.degenerate, None,
            "the bottom row is blanked at L_z = {lz}"
        );
        assert_eq!(wide.validate(0.5, 0.0), ValidationResult::Valid);
        let d = theta_k()
            .sliced(HELD_LZ, lz)
            .unwrap()
            .decode::<Defaults, f64>(0.5, 0.0);
        assert_eq!(
            d.degenerate, None,
            "the bottom row is blanked at L_z = {lz}"
        );
        let lz = lz as f32;
        for d in [
            narrow
                .sliced(HELD_LZ, lz)
                .unwrap()
                .decode::<Defaults, f32>(0.5, 0.0),
            acute
                .sliced(HELD_LZ, lz)
                .unwrap()
                .decode::<Defaults, f32>(0.5, 0.0),
        ] {
            assert_eq!(
                d.degenerate, None,
                "the bottom row is blanked at L_z = {lz}"
            );
        }
    }
}

#[test]
fn tilt_every_chart_construction_bottom_row() {
    check_bottom_row(false);
}

negative_control!(
    tilt_every_chart_construction_bottom_row,
    "a bottom row left to the rounding of l²/2 is blanked, so it must fail",
    expected = "the bottom row is blanked",
    {
        check_bottom_row(true);
    }
);

/// An offset `dx` on the held `K`.
fn held_k_shift(dx: f64) -> Shift<f64> {
    Shift {
        x: HELD_K,
        dx,
        y: HELD_K,
        dy: 0.0,
        two: false,
    }
}

/// Linear `Φ_θ,L_z` past the centre's band (`b = 1.3`, `L_z = 1.6`, `L_z²/2 = 1.28`, `K₀ = 0.5`): the `L_z` axis is affine
/// (chart_reference §5.4), so the bound is `K + d`'s. The cell is a system where `K + d` reaches it and refused where
/// it doesn't. `centre` reads the chart's check at the centre's `K₀` instead (the control).
fn past_band<C: Tiltable<Param = f64> + Copy>(chart: C, centre: bool) {
    let name = chart.name();
    let view = held_view(
        chart,
        &spec(30.0, 0.0, TiltMap::Sheet, HELD_K, HELD_K),
        2.0 * K_SPAN,
    );
    let (u, v) = (1.0, 1.3);
    let ([a, b], shift) = view.tilt().place(u, v);
    assert!(b > 1.0 && (0.0..=1.0).contains(&a) && shift.dx > 0.78);
    let valid = if centre {
        view.chart().validate(a, b)
    } else {
        view.validate(u, v)
    };
    assert_eq!(
        valid,
        ValidationResult::Valid,
        "{name}: a cell past the centre's band that K + d reaches"
    );
    assert_eq!(view.weight(u, v), view.tilt().weight, "{name}");
    let d = view.decode::<Defaults, f64>(u, v);
    assert_eq!(d.degenerate, None, "{name}");
    let (lz, k, drift) = invariants(&d);
    let want = (2.0 * b - 1.0, THETA_LZ_K + shift.dx);
    assert!(
        (lz - want.0).abs() < 1e-12 && (k - want.1).abs() < 1e-12 && drift < 1e-12,
        "{name}: ({lz}, {k}) is not {want:?}"
    );
    // The same row where K + d falls short, at d = 0 and below K = 0: refused, weighted 0.
    for u in [CENTRE[0], 0.0] {
        assert_eq!(
            view.validate(u, v),
            ValidationResult::Reject {
                failed: Constraint::KAboveLzBound
            },
            "{name} at {u}"
        );
        assert_eq!(view.weight(u, v), 0.0, "{name} at {u}");
    }
    // The bound's two sides: L_z²/2 − K₀ = 0.78 and a little.
    let p = LinkParams::DEFAULT;
    let refused = ValidationResult::Reject {
        failed: Constraint::KAboveLzBound,
    };
    assert_eq!(
        chart.validate_shifted(0.5, 1.3, &held_k_shift(0.78), &p),
        refused,
        "{name}"
    );
    assert_eq!(
        chart.validate_shifted(0.5, 1.3, &held_k_shift(0.79), &p),
        ValidationResult::Valid,
        "{name}"
    );
}

/// The linear arm past the band, on the bare chart and on `HeldK` in linear mode.
fn check_past_band(centre: bool) {
    past_band(theta_lz(), centre);
    past_band(
        HeldK {
            mapping: Mapping::Linear,
            ..held_k()
        },
        centre,
    );
}

#[test]
fn tilt_every_chart_construction_past_band() {
    check_past_band(false);
}

negative_control!(
    tilt_every_chart_construction_past_band,
    "the L_z bound checked at the centre's K₀ refuses a cell K + d reaches, so it must fail",
    expected = "a cell past the centre's band",
    {
        check_past_band(true);
    }
);

/// A pixel with no construction: refused by the chart's check at its own precision, tagged, its momenta finite. `k` is
/// the pixel's `K*`, `blind` reads a check of `K*` alone instead of the chart's (the control).
fn check_refused_arm<C, R>(c: &C, [a, b]: [R; 2], dx: R, k: f64, blind: bool, name: &str)
where
    C: Tiltable<Param = R>,
    R: LinkReal + PayloadReal + From<f32>,
    f64: From<R>,
{
    let wide = held_k_shift(f64::from(dx));
    let (wa, wb) = (f64::from(a), f64::from(b));
    let got = if blind {
        (!k.is_finite()).then_some(Constraint::KAboveLzBound)
    } else {
        c.refusal(wa, wb, &wide)
    };
    assert_eq!(
        got,
        Some(Constraint::KAboveLzBound),
        "{name}: a K* with no finite 2K* is read as a system"
    );
    assert_eq!(
        c.validate_shifted(wa, wb, &wide, &LinkParams::DEFAULT),
        ValidationResult::Reject {
            failed: Constraint::KAboveLzBound
        },
        "{name}"
    );
    let own = Shift {
        x: HELD_K,
        dx,
        y: HELD_K,
        dy: <R as From<f32>>::from(0.0),
        two: false,
    };
    let d = c.decode_shifted::<Defaults, R>(a, b, &own, &LinkParams::DEFAULT);
    assert_eq!(
        d.degenerate,
        Some(Degenerate::InvariantInfeasible),
        "{name}"
    );
    let p = d.ic.p.iter().flatten();
    assert!(p.map(|&x| f64::from(x)).all(f64::is_finite), "{name}");
}

/// [`check_refused_arm`] and its neighbours at the float `$p`: `K = +∞`, a finite `K` past `max/2`, the floor plus the
/// warp's line past it and an `L_z` with no finite square, each refused on each chart that reaches it, and `K = max/2`,
/// the last value still a system (chart_reference §4.5).
macro_rules! no_finite_2k {
    ($name:ident, $p:ident) => {
        fn $name(blind: bool) {
            let bound = Some(Constraint::KAboveLzBound);
            let max = <$p>::MAX;
            let last = max * 0.5;
            let bare = |k: $p| BurrauThetaLz::<$p> {
                theta_range: [0.2, 0.7],
                k,
                lz_fraction: 1.0,
            };
            for (k, want) in [
                (<$p>::INFINITY, true),
                (<$p>::NAN, true),
                (max, true),
                (max * 0.75, true),
                (<$p>::from_bits(last.to_bits() + 1), true),
                (last, false),
                (last * 0.75, false),
            ] {
                assert_eq!(no_construction(k, 0.0), want, "{k}");
            }
            // K = +∞ on a warped held K, at L_z = 0 and on a row whose L_z·r has no finite float.
            let warped = HeldK::new(bare(1.0), 1.0, 1e5);
            for b in [0.5, 0.9] {
                check_refused_arm(&warped, [0.5, b], 1.0, f64::INFINITY, blind, "warped");
            }
            // K finite past max/2: by the chart's K, by K + d and linear.
            let past = f64::from(max) * 0.75;
            let linear = HeldK {
                mapping: Mapping::Linear,
                ..HeldK::new(bare(max * 0.75), 1.0, 1.5)
            };
            for b in [0.5, 0.9] {
                let (centre, moved) = (bare(max * 0.75), bare(1.0));
                check_refused_arm(&centre, [0.5, b], 0.0, past, blind, "bare");
                check_refused_arm(&moved, [0.5, b], max * 0.75, past, blind, "K + d");
                check_refused_arm(&linear, [0.5, b], 0.0, past, blind, "linear");
            }
            // The floor plus the line, each below max/2: past it only as a sum.
            let burrau = Burrau::<$p> {
                nu_range: [0.2, 0.4],
                k_max: max * 0.25,
                gamma_k: 1.0,
            };
            let held = |lz: $p| HeldLz::new(burrau).sliced(HELD_LZ, lz).unwrap();
            let sum = held((max * 0.81).sqrt());
            check_refused_arm(&sum, [0.5, 1.0], 0.0, past, blind, "floor plus line");
            // An L_z with no finite square, whose L_z·r can pass the float's range.
            let wild = held(max * 0.9);
            check_refused_arm(&wild, [0.5, 0.0], 0.0, past, blind, "no floor");
            // The chart's own check agrees, at its own precision, and layer 3 reads the pixel's decode.
            assert!(matches!(
                bare(max * 0.75).validate(0.5, 0.5),
                ValidationResult::Reject { .. }
            ));
            let none = held_k_shift(0.0);
            let layer_3 = Some(Constraint::DecodedMassPositive);
            assert_eq!(bare(1.0).refusal(f64::NAN, 0.5, &none), layer_3);
            assert_eq!(warped.refusal(f64::NAN, 0.5, &none), layer_3);
            assert_eq!(held(0.5).refusal(f64::NAN, 0.5, &none), layer_3);
            // The last value still a system.
            let c = bare(last);
            assert_eq!(c.validate(0.5, 0.5), ValidationResult::Valid);
            assert_eq!(c.refusal(0.5, 0.5, &none), None);
            assert_eq!(
                c.validate_shifted(0.5, 0.5, &none, &LinkParams::DEFAULT),
                ValidationResult::Valid
            );
            let own = Shift::<$p>::none();
            let d = c.decode_shifted::<Defaults, $p>(0.5, 0.5, &own, &LinkParams::DEFAULT);
            assert_eq!(d.degenerate, None);
            assert!(d.ic.p.iter().flatten().all(|x| x.is_finite()));
            // Through the view: weight 0, and not "held at floor".
            let s = spec(30.0, 0.0, TiltMap::Sheet, HELD_K, HELD_K);
            let wide = Tilt::new(&s, CENTRE, [2.0 * K_SPAN, 2.0 * K_SPAN]);
            let tilt = Tilt::<$p> {
                on: wide.on,
                two: wide.two,
                rank_loss: wide.rank_loss,
                centre: wide.centre.map(|x| x as $p),
                columns: wide.columns.map(|c| c.map(|x| x as $p)),
                x: wide.x,
                y: wide.y,
                span: wide.span.map(|x| x as $p),
                weight: wide.weight as $p,
                normal_zero: wide.normal_zero,
                strip: wide.strip,
            };
            let view = Tilted::new(warped, tilt).expect("a hidden target");
            assert_eq!(view.weight(1.0, 0.5), 0.0);
            assert!(!view.held_at_floor(1.0, 0.5));
            assert_eq!(
                view.validate(1.0, 0.5),
                ValidationResult::Reject {
                    failed: bound.unwrap()
                }
            );
        }
    };
}

no_finite_2k!(check_no_finite_2k_f64, f64);
no_finite_2k!(check_no_finite_2k_f32, f32);

#[test]
fn tilt_every_chart_construction_no_finite_2k() {
    check_no_finite_2k_f64(false);
    check_no_finite_2k_f32(false);
}

negative_control!(
    tilt_every_chart_construction_no_finite_2k,
    "a check on a finite K* alone reads a finite K* with no finite 2K* as a system, so it must fail",
    expected = "a K* with no finite 2K* is read as a system",
    {
        check_no_finite_2k_f64(true);
    }
);

/// The linear rows off the warp's line: at `v = 1e200` the row is `K = 1e200`, a system on R-471 E2's affine axis, where
/// the warp's `K_max·v^γ_K` overflows. The check reads the row's decode, keeping the axes' layers; warped, the pixel is
/// the line's and refused. `warp_line` reads the chart's own check instead (the control).
fn off_warp_line<C>(chart: C, warp_line: bool)
where
    C: LzChart<Param = f64>,
{
    let name = chart.name().to_string();
    let wide = 1.0e200;
    let warped = HeldLz::new(chart);
    let row = HeldLz {
        mapping: Mapping::Linear,
        ..warped
    };
    let none = held_k_shift(0.0);
    let p = LinkParams::DEFAULT;
    let valid = if warp_line {
        chart.validate(0.5, wide)
    } else {
        row.validate(0.5, wide)
    };
    assert_eq!(
        valid,
        ValidationResult::Valid,
        "{name}: a linear row off the warp's line is refused"
    );
    assert_eq!(
        row.validate_shifted(0.5, wide, &none, &p),
        ValidationResult::Valid,
        "{name}"
    );
    assert_eq!(
        row.decode::<Defaults, f64>(0.5, wide).degenerate,
        None,
        "{name}"
    );
    assert!(
        matches!(warped.validate(0.5, wide), ValidationResult::Reject { .. }),
        "{name}: warped, the pixel is the line's"
    );
    // The axes' layers stay, and the row past 2K*'s limit is no system.
    for (u, v) in [
        (f64::NAN, 0.5),
        (0.5, f64::NAN),
        (0.5, f64::INFINITY),
        (1.0e3, 0.5),
    ] {
        assert!(
            matches!(row.validate(u, v), ValidationResult::Reject { .. }),
            "{name} at ({u}, {v})"
        );
    }
    assert_eq!(
        row.validate(0.5, 1.0e308),
        ValidationResult::Reject {
            failed: Constraint::KAboveLzBound
        },
        "{name}"
    );
    assert_eq!(
        row.validate(0.5, -0.5),
        ValidationResult::Project {
            u: 0.5,
            v: 0.0,
            failed: Constraint::KNonNegative
        },
        "{name}"
    );
    assert_eq!(row.validate(0.5, 0.5), ValidationResult::Valid, "{name}");
}

fn check_off_warp_line(warp_line: bool) {
    off_warp_line(
        Burrau {
            gamma_k: 2.0,
            ..nu_k().chart
        },
        warp_line,
    );
    off_warp_line(
        BurrauThetaK {
            gamma_k: 2.0,
            ..theta_k().chart
        },
        warp_line,
    );
}

#[test]
fn tilt_every_chart_construction_linear_off_warp_line() {
    check_off_warp_line(false);
}

negative_control!(
    tilt_every_chart_construction_linear_off_warp_line,
    "the check on the warp's line refuses a linear row it never decodes, so it must fail",
    expected = "a linear row off the warp's line is refused",
    {
        check_off_warp_line(true);
    }
);

/// `ν₀ + d` rounding onto an end of `(0, 1)` at the chart's own precision: an f32 simplex whose f32 sum is exactly 1 or
/// exactly 0, inside at f64, is the decode's collision, refused with weight 0; one f32 float inside each end, and a
/// plain offset, stay systems. `wide` reads the test at f64 alone (the control).
fn check_simplex_own_precision(wide: bool) {
    let chart = RestStart::new(MassSimplex::<f32> {
        nu0: 0.5,
        eps_m: 1e-4,
    });
    let p = LinkParams::DEFAULT;
    let shift = |dx: f64| Shift {
        x: NU0,
        dx,
        y: NU0,
        dy: 0.0,
        two: false,
    };
    for (dx, end) in [(0.4999999701976776, 1.0), (-0.4999999999, 0.0)] {
        assert!(dx + 0.5 > 0.0 && dx + 0.5 < 1.0, "inside at f64");
        assert_eq!(0.5 + dx as f32, end, "the f32 sum");
        let got = if wide {
            None
        } else {
            chart.refusal(0.37, 0.3, &shift(dx))
        };
        assert_eq!(
            got,
            Some(Constraint::BurrauNu),
            "a ν at an end of (0, 1) at f32 is read as a system"
        );
        assert_eq!(
            chart.validate_shifted(0.37, 0.3, &shift(dx), &p),
            ValidationResult::Reject {
                failed: Constraint::BurrauNu
            }
        );
    }
    let inside = [0.25, 0.5 - f64::from(f32::EPSILON) / 2.0, 2.0e-8 - 0.5];
    for dx in inside {
        assert_eq!(chart.refusal(0.37, 0.3, &shift(dx)), None, "{dx}");
        assert_eq!(
            chart.validate_shifted(0.37, 0.3, &shift(dx), &p),
            ValidationResult::Valid,
            "{dx}"
        );
    }
}

#[test]
fn tilt_every_chart_construction_simplex_own_precision() {
    check_simplex_own_precision(false);
}

negative_control!(
    tilt_every_chart_construction_simplex_own_precision,
    "the test at f64 alone finds both ends inside (0, 1), so it must fail",
    expected = "a ν at an end of (0, 1) at f32 is read as a system",
    {
        check_simplex_own_precision(true);
    }
);

/// Each of the six's hidden coordinates, `hidden(k)` over `0..12`, and a tilt into any other refused; `admit` admits
/// it as untilted (the control).
fn check_construction_targets(admit: bool) {
    let refuse = |view: bool, name: &str, k: u32| {
        assert!(
            !view,
            "{name}: a tilt into {k}, which the chart does not hold, admitted"
        );
    };
    let make = |t: Tilt<f64>| if admit { Tilt { on: false, ..t } } else { t };
    let at = |k: u32| {
        make(Tilt::new(
            &spec(30.0, 0.0, TiltMap::Sheet, k, k),
            CENTRE,
            SPAN,
        ))
    };
    let want = |name: &str, k: u32| match name {
        "MassSimplex" => momentum_control(k) || k == NU0,
        "EuclidPlane" | "BurrauThetaDeltaM" => momentum_control(k),
        "Burrau" | "BurrauThetaK" => k == HELD_LZ,
        _ => k == HELD_K,
    };
    for k in 0..12 {
        let pairs = [
            (
                simplex().hidden(k),
                Tilted::new(simplex(), at(k)).is_some(),
                "MassSimplex",
            ),
            (
                euclid().hidden(k),
                Tilted::new(euclid(), at(k)).is_some(),
                "EuclidPlane",
            ),
            (
                delta_m().hidden(k),
                Tilted::new(delta_m(), at(k)).is_some(),
                "BurrauThetaDeltaM",
            ),
            (
                nu_k().hidden(k),
                Tilted::new(nu_k(), at(k)).is_some(),
                "Burrau",
            ),
            (
                theta_k().hidden(k),
                Tilted::new(theta_k(), at(k)).is_some(),
                "BurrauThetaK",
            ),
            (
                theta_lz().hidden(k),
                Tilted::new(theta_lz(), at(k)).is_some(),
                "BurrauThetaLz",
            ),
            (
                held_k().hidden(k),
                Tilted::new(held_k(), at(k)).is_some(),
                "BurrauThetaLz",
            ),
        ];
        for (hidden, admitted, name) in pairs {
            assert_eq!(hidden, want(name, k), "{name}: hidden {k}");
            if hidden {
                assert!(admitted, "{name}: a tilt into {k} refused");
            } else {
                refuse(admitted, name, k);
            }
        }
    }
}

/// A selection whose momentum links are `tanh_q`, which keeps a `−0`.
type TanhSel = Select<SoftmaxTanh, SigmoidAlpha, SigmoidBeta, TanhQ, TanhQ, TanhQ, TanhQ>;

#[test]
fn tilt_every_chart_construction_targets() {
    check_construction_targets(false);
    assert_eq!(
        [1, 2, 5, 6].map(momentum_control),
        [false, true, true, false],
        "the momentum controls are 2..6"
    );
    // Every registered momentum link maps 0 to 0, at both precisions (b1).
    for z in [0.0, -0.0] {
        assert_eq!(SigmoidQ::forward([z]), [0.0]);
        assert_eq!(TanhQ::forward([z]), [0.0]);
        assert_eq!(SoftsignQ::forward([z]), [0.0]);
        assert_eq!(SigmoidQ::forward([z as f32]), [0.0]);
        assert_eq!(TanhQ::forward([z as f32]), [0.0]);
        assert_eq!(SoftsignQ::forward([z as f32]), [0.0]);
    }
    // A rest start held at −0 is the rest start, bit for bit, under a link that keeps the −0.
    let held = RestStart {
        z_mom: [-0.0; 4],
        ..euclid()
    };
    assert_eq!(
        bits(&held.decode::<TanhSel, f64>(0.3, 0.6)),
        bits(&euclid().chart.decode::<TanhSel, f64>(0.3, 0.6)),
        "a rest start at −0 is not the rest start"
    );
    // The held values at the slice centre.
    assert_eq!(simplex().held(NU0), Some(0.5));
    assert_eq!(simplex().held(3), Some(0.0));
    assert_eq!(simplex().held(HELD_LZ), None);
    assert_eq!(euclid().held(NU0), None);
    assert_eq!(delta_m().held(NU0), None);
    assert_eq!(nu_k().held(HELD_LZ), Some(0.0));
    assert_eq!(nu_k().held(HELD_K), None);
    assert_eq!(Held::held(&theta_lz(), HELD_K), Some(THETA_LZ_K));
    assert_eq!(Held::held(&theta_lz(), NU0), None);
    assert_eq!(held_k().held(HELD_K), Some(THETA_LZ_K));
    assert_eq!(held_k().held(NU0), None);
    // The wrappers are the registered chart: its name and flags.
    assert_eq!((simplex().name(), nu_k().name()), ("MassSimplex", "Burrau"));
    assert_eq!(held_k().name(), "BurrauThetaLz");
    assert!(nu_k().forbids_energy_normalisation() && !euclid().forbids_energy_normalisation());
    assert!(held_k().forbids_energy_normalisation() && held_k().requires_per_pixel_mass());
    assert!(simplex().requires_per_pixel_mass() && theta_k().requires_per_pixel_mass());
    assert_eq!(
        format!("{:?}", held_k().map::<f64>(0.3, 0.4)),
        format!("{:?}", theta_lz().map::<f64>(0.3, 0.4))
    );
    assert_eq!(
        format!("{:?}", nu_k().map::<f64>(0.3, 0.4)),
        format!("{:?}", nu_k().chart.map::<f64>(0.3, 0.4))
    );
    assert_eq!(
        format!("{:?}", simplex().map::<f64>(0.3, 0.4)),
        format!("{:?}", simplex().chart.map::<f64>(0.3, 0.4))
    );
}

negative_control!(
    tilt_every_chart_construction_targets,
    "a tilt into a coordinate the chart does not hold admitted as untilted must fail",
    expected = "which the chart does not hold, admitted",
    {
        check_construction_targets(true);
    }
);

/// The checks of the held charts at the edges: a check that commits to a point the held value refuses there is
/// refused; one that commits to a feasible point, or rejects, is kept; and the per-cell weight untilted.
#[test]
fn tilt_every_chart_construction_validate() {
    let none = Shift::none();
    // Linear, (ν, K) at a held L_z: below t = 0 the chart projects to K = 0, where no construction realises L_z ≠ 0.
    let held = linear(nu_k()).sliced(HELD_LZ, 0.3).unwrap();
    assert_eq!(
        held.validate(0.5, -0.2),
        ValidationResult::Reject {
            failed: Constraint::KAboveLzBound
        }
    );
    assert_eq!(
        held.validate(0.5, 0.01),
        ValidationResult::Reject {
            failed: Constraint::KAboveLzBound
        }
    );
    assert_eq!(held.validate(0.5, 0.8), ValidationResult::Valid);
    assert!(matches!(
        nu_k().validate(0.5, -0.2),
        ValidationResult::Project { .. }
    ));
    assert_eq!(
        held.validate(-3.0, 0.5),
        ValidationResult::Reject {
            failed: Constraint::BurrauNu
        }
    );
    let view = Tilted::new(
        held,
        Tilt::new(
            &spec(0.0, 0.0, TiltMap::Sheet, HELD_LZ, HELD_LZ),
            CENTRE,
            SPAN,
        ),
    )
    .unwrap();
    assert_eq!((view.weight(0.5, 0.01), view.weight(0.5, 0.8)), (0.0, 1.0));
    // The mass simplex: a clamp onto the square kept where ν₀ + d is a triangle, refused where it is none.
    let s = simplex();
    let shift = |dx: f64| Shift {
        x: NU0,
        dx,
        y: NU0,
        dy: 0.0,
        two: false,
    };
    let p = LinkParams::DEFAULT;
    assert!(matches!(
        s.validate_shifted(1.4, 0.5, &shift(0.1), &p),
        ValidationResult::Clamp { .. }
    ));
    for (dx, refused) in [
        (-0.5, true),
        (0.5, true),
        (-0.49, false),
        (0.49, false),
        (0.6, true),
    ] {
        let want = if refused {
            ValidationResult::Reject {
                failed: Constraint::BurrauNu,
            }
        } else {
            ValidationResult::Valid
        };
        assert_eq!(
            s.validate_shifted(0.3, 0.3, &shift(dx), &p),
            want,
            "ν₀ + {dx}"
        );
        assert_eq!(
            s.refusal(0.3, 0.3, &shift(dx)).is_some(),
            refused,
            "ν₀ + {dx}"
        );
    }
    assert_eq!(
        s.validate_shifted(1.4, 0.5, &shift(0.5), &p),
        ValidationResult::Reject {
            failed: Constraint::BurrauNu
        }
    );
    assert_eq!(s.refusal(0.3, 0.3, &none), None);
    assert_eq!(euclid().refusal(0.3, 0.3, &shift(5.0)), None);
    // Φ_θ,L_z's held K: refused at the band's edge below the centre's K, its own check first.
    let c = theta_lz();
    let k_shift = |dx: f64| Shift {
        x: HELD_K,
        dx,
        y: HELD_K,
        dy: 0.0,
        two: false,
    };
    assert_eq!(
        c.refusal(0.5, 1.0, &k_shift(-0.1)),
        Some(Constraint::KAboveLzBound)
    );
    assert_eq!(c.refusal(0.5, 1.0, &k_shift(0.0)), None);
    assert_eq!(
        c.validate_shifted(0.5, 1.3, &k_shift(0.0), &p),
        ValidationResult::Reject {
            failed: Constraint::KAboveLzBound
        }
    );
    assert_eq!(
        c.validate_shifted(f64::NAN, 0.5, &k_shift(0.0), &p),
        ValidationResult::Reject {
            failed: Constraint::BurrauTheta
        }
    );
    assert_eq!(
        c.validate_shifted(0.5, f64::NAN, &k_shift(0.0), &p),
        ValidationResult::Reject {
            failed: Constraint::KAboveLzBound
        }
    );
    assert_eq!(
        c.validate_shifted(0.5, 1.3, &k_shift(-0.1), &p),
        ValidationResult::Reject {
            failed: Constraint::KAboveLzBound
        }
    );
    assert_eq!(
        c.validate_shifted(-2.0, 0.5, &k_shift(-0.1), &p),
        ValidationResult::Reject {
            failed: Constraint::BurrauTheta
        }
    );
    // The held L_z's own refusal and check, through the trait.
    let z = linear(nu_k());
    let lz_shift = |dx: f64| Shift {
        x: HELD_LZ,
        dx,
        y: HELD_LZ,
        dy: 0.0,
        two: false,
    };
    assert_eq!(
        z.refusal(0.5, 0.01, &lz_shift(0.3)),
        Some(Constraint::KAboveLzBound)
    );
    assert_eq!(z.refusal(0.5, 0.01, &lz_shift(0.0)), None);
    assert!(matches!(
        z.validate_shifted(0.5, -0.2, &lz_shift(0.0), &p),
        ValidationResult::Project { .. }
    ));
    assert_eq!(
        z.validate_shifted(0.5, -0.2, &lz_shift(0.3), &p),
        ValidationResult::Reject {
            failed: Constraint::KAboveLzBound
        }
    );
    assert_eq!(
        linear(theta_k()).refusal(0.5, 0.01, &lz_shift(0.3)),
        Some(Constraint::KAboveLzBound)
    );
    // Warped, the same cells are systems; below t = 0 the bounded side stays; a held L_z with no finite floor is
    // refused, at the chart's own check as at a tilt's.
    let warped = nu_k();
    assert_eq!(warped.refusal(0.5, 0.01, &lz_shift(0.3)), None);
    let held = warped.sliced(HELD_LZ, 0.3).unwrap();
    assert_eq!(held.validate(0.5, 0.01), ValidationResult::Valid);
    assert_eq!(held.validate(0.5, 0.0), ValidationResult::Valid);
    assert!(matches!(
        held.validate(0.5, -0.2),
        ValidationResult::Project { .. }
    ));
    let unfloored = warped.sliced(HELD_LZ, 1e200).unwrap();
    assert_eq!(
        unfloored.validate(0.5, 0.5),
        ValidationResult::Reject {
            failed: Constraint::KAboveLzBound
        }
    );
    assert_eq!(
        warped.refusal(0.5, 0.5, &lz_shift(1e200)),
        Some(Constraint::KAboveLzBound)
    );
    // Warped, Φ_θ,L_z: the band at the pixel's own K, past it refused, inside it not; the chart's own check first.
    let narrow = HeldK::new(
        BurrauThetaLz {
            lz_fraction: 0.5,
            ..theta_lz()
        },
        1.0,
        1.5,
    );
    assert_eq!(narrow.refusal(0.5, 1.4, &k_shift(0.1)), None);
    assert_eq!(
        narrow.refusal(0.5, 2.0, &k_shift(0.1)),
        Some(Constraint::KAboveLzBound)
    );
    let at_rest = HeldK::new(
        BurrauThetaLz {
            k: 0.0,
            ..theta_lz()
        },
        1.0,
        1.5,
    );
    assert_eq!(
        at_rest.validate_shifted(0.5, 1.5, &k_shift(0.5), &p),
        ValidationResult::Reject {
            failed: Constraint::KAboveLzBound
        }
    );
    assert_eq!(
        at_rest.validate_shifted(0.5, 0.5, &k_shift(0.5), &p),
        ValidationResult::Valid
    );
    assert_eq!(
        at_rest.validate_shifted(-2.0, 0.5, &k_shift(0.5), &p),
        ValidationResult::Reject {
            failed: Constraint::BurrauTheta
        }
    );
    // A centre the check refuses still decodes finitely, its K taken at 0.
    let below = held_k().sliced(HELD_K, -0.5).unwrap();
    assert!(matches!(
        below.validate(0.5, 0.5),
        ValidationResult::Reject { .. }
    ));
    assert!(below
        .decode_shifted::<Defaults, f64>(0.5, 0.5, &k_shift(0.3), &p)
        .ic
        .p
        .iter()
        .flatten()
        .all(|x| x.is_finite()));
    // The flag's boundary: t₀ + d = 0 exactly is not clamped, one step below it is; d = 0 and a linear chart never are.
    let flagged = HeldK::new(
        BurrauThetaLz {
            k: 0.25,
            ..theta_lz()
        },
        1.0,
        2.0,
    );
    assert!(!flagged.held_at_floor(&k_shift(0.0)));
    assert!(!flagged.held_at_floor(&k_shift(-0.5)));
    assert!(flagged.held_at_floor(&k_shift(-0.5 - 1e-12)));
    let by_value = HeldK {
        mapping: Mapping::Linear,
        ..flagged
    };
    assert!(!by_value.held_at_floor(&k_shift(-0.9)));
    assert!(at_rest.held_at_floor(&k_shift(-1e-3)));
    assert!(!at_rest.held_at_floor(&k_shift(1e-3)));
    assert!(!nu_k().held_at_floor(&lz_shift(-0.9)));
    // Through the view: flagged on a system, not past the chart's edge, not untilted.
    let view = held_view(
        at_rest,
        &spec(30.0, 0.0, TiltMap::Sheet, HELD_K, HELD_K),
        K_SPAN,
    );
    assert!(view.held_at_floor(0.1, 0.5));
    assert!(!view.held_at_floor(-5.0, 0.5));
    let off = held_view(
        at_rest,
        &spec(0.0, 0.0, TiltMap::Sheet, HELD_K, HELD_K),
        K_SPAN,
    );
    assert!(!off.held_at_floor(0.1, 0.5));
}

negative_control!(
    tilt_every_chart_construction_validate,
    "a check that ignores the held L_z admits a cell no construction realises, so it must fail",
    expected = "KAboveLzBound",
    {
        let held = nu_k().sliced(HELD_LZ, 0.3).unwrap();
        assert_eq!(
            held.chart.validate(0.5, 0.01),
            ValidationResult::Reject {
                failed: Constraint::KAboveLzBound
            }
        );
    }
);

// ── REQ-CHART-065 on the six: a slice moves only that held value ────────────────────────────────────────────────────

/// Every hidden coordinate of a rest start sliced: only that held value moves, and the decode at the centre follows,
/// a momentum control through `D_mom` under the chart's masses, `ν₀` the triangle's; any other index is refused.
/// `momenta` writes a sliced control's value as a physical momentum instead (the control).
fn slice_rest<C: RestChart<Param = f64> + Held<f64>>(
    chart: RestStart<C, f64>,
    physical: &[u32],
    momenta: bool,
) {
    for k in 0..12 {
        let value = 0.3 + 0.01 * f64::from(k);
        let Some(sliced) = chart.sliced(k, value) else {
            assert!(!chart.hidden(k), "{}: hidden {k} refused", chart.name());
            continue;
        };
        assert!(
            chart.hidden(k) && (momentum_control(k) || physical.contains(&k)),
            "{}: {k} sliced",
            chart.name()
        );
        for j in 0..12 {
            let want = if j == k { Some(value) } else { chart.held(j) };
            assert_eq!(
                sliced.held(j),
                want,
                "{}: slice {k} moved {j}",
                chart.name()
            );
        }
        let mut d = sliced.decode::<Defaults, f64>(0.5, 0.5);
        if momentum_control(k) {
            let mut z = [0.0; 4];
            z[(k - 2) as usize] = value;
            let own = chart.chart.decode::<Defaults, f64>(0.5, 0.5);
            if momenta {
                d.ic.p = own.ic.p;
                d.ic.p[2][(k % 2) as usize] = value;
            }
            assert_eq!(
                d.ic.p,
                momentum::decode::<Defaults, f64>(z, &own.masses).p,
                "{}: slice {k} is not D_mom of the control",
                chart.name()
            );
            assert_eq!(format!("{:?}", d.masses), format!("{:?}", own.masses));
        } else {
            assert_eq!(
                bits(&d),
                bits(&sliced.chart.decode::<Defaults, f64>(0.5, 0.5))
            );
            assert_ne!(bits(&d), bits(&chart.decode::<Defaults, f64>(0.5, 0.5)));
        }
    }
}

/// The held `L_z` sliced to each sign, past the untilted band too: only it moves, and the decode at the centre is §2.2's
/// construction at `L_z₀ + d` by value and the row's `K`: warped, the floor plus the warp's line, linear, `K_max·t`
/// (R-720). `warp` moves the held `L_z` in a warp coordinate instead (the control).
fn slice_lz<C: LzChart<Param = f64> + PartialEq + std::fmt::Debug>(
    chart: HeldLz<C, f64>,
    warp: bool,
) {
    let row = |lz: f64| {
        if chart.mapping == Mapping::Warped {
            k_floor(lz) + chart.chart.kinetic_at(0.8)
        } else {
            chart.chart.k_max_at::<f64>() * 0.8
        }
    };
    for k in 0..12 {
        for value in [0.3, -0.3, -0.75, 0.75] {
            let Some(sliced) = chart.sliced(k, value) else {
                assert_ne!(k, HELD_LZ, "{}: the held L_z refused", chart.name());
                continue;
            };
            assert_eq!(k, HELD_LZ);
            assert_eq!(
                (sliced.held(HELD_LZ), sliced.chart, sliced.mapping),
                (Some(value), chart.chart, chart.mapping)
            );
            let d = if warp {
                let moved = 2.0 * value * l_max(row(value));
                chart.chart.decode_lz(0.5, moved, k_floor(moved) + row(0.0))
            } else {
                sliced.decode::<Defaults, f64>(0.5, 0.8)
            };
            let (lz, kin, _) = invariants(&d);
            assert!(
                (lz - value).abs() < 1e-12,
                "{}: the sliced L_z {lz} is not L_z₀ + d = {value}",
                chart.name()
            );
            assert!(
                (kin - row(value)).abs() < 1e-12,
                "{}: K = {kin}",
                chart.name()
            );
        }
    }
}

/// `Φ_θ,L_z`'s held `K` sliced, by value in either mode: only it moves, and the decode at the centre has `K(p)` the
/// slice's.
fn slice_k() {
    let chart = theta_lz();
    for k in 0..12 {
        let Some(sliced) = chart.sliced(k, 0.8) else {
            assert_ne!(k, HELD_K, "the held K refused");
            assert!(held_k().sliced(k, 0.8).is_none(), "held K sliced {k}");
            continue;
        };
        assert_eq!(k, HELD_K);
        assert_eq!(sliced, BurrauThetaLz { k: 0.8, ..chart });
        assert_eq!(
            held_k().sliced(k, 0.8),
            Some(HeldK {
                chart: sliced,
                ..held_k()
            })
        );
        let (_, kin, _) = invariants(&sliced.decode::<Defaults, f64>(0.5, 0.7));
        assert!((kin - 0.8).abs() < 1e-12, "Φ_θ,L_z: K = {kin}");
        let (_, kin, _) = invariants(
            &held_k()
                .sliced(k, 0.8)
                .unwrap()
                .decode::<Defaults, f64>(0.5, 0.7),
        );
        assert!((kin - 0.8).abs() < 1e-12, "Φ_θ,L_z, warped: K = {kin}");
    }
}

/// A rest start whose flags are the opposite of every registered one's: it forbids `η_E` and holds uniform masses.
#[derive(Clone, Copy)]
struct Flipped(EuclidPlane<f64>);

impl Chart for Flipped {
    type Param = f64;

    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> kernel::chart::ChartOut<F> {
        self.0.map(u, v)
    }

    fn forbids_energy_normalisation(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "Flipped"
    }

    fn requires_per_pixel_mass(&self) -> bool {
        false
    }

    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.0.validate(u, v)
    }
}

impl RestChart for Flipped {
    fn decode_moving<S, R>(
        &self,
        u: R,
        v: R,
        z_mom: [R; 4],
        shift: &Shift<R>,
        p: &LinkParams<R>,
    ) -> Decoded<R>
    where
        S: kernel::generated::links::Selection,
        R: kernel::generated::links::LinkReal + kernel::payload::PayloadReal + From<f64>,
    {
        self.0.decode_moving::<S, R>(u, v, z_mom, shift, p)
    }
}

/// A held-`L_z` chart that allows `η_E` and holds uniform masses, the opposite of the registered ones.
#[derive(Clone, Copy)]
struct Allowing(Burrau<f64>);

impl Chart for Allowing {
    type Param = f64;

    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> kernel::chart::ChartOut<F> {
        self.0.map(u, v)
    }

    fn name(&self) -> &str {
        "Allowing"
    }

    fn requires_per_pixel_mass(&self) -> bool {
        false
    }

    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.0.validate(u, v)
    }
}

impl LzChart for Allowing {
    fn kinetic_at<F: Float + From<f64>>(&self, v: F) -> F {
        self.0.kinetic_at(v)
    }

    fn k_max_at<F: Float + From<f64>>(&self) -> F {
        self.0.k_max_at()
    }

    fn check_axes(&self, u: f64, v: f64) -> ValidationResult {
        self.0.check_axes(u, v)
    }

    fn decode_lz<R>(&self, u: R, lz: R, k: R) -> Decoded<R>
    where
        R: kernel::generated::links::LinkReal + kernel::payload::PayloadReal + From<f64>,
    {
        self.0.decode_lz(u, lz, k)
    }
}

/// The wrappers' flags are the chart's own, whatever it declares; `own` reads a fixed value instead (the control).
fn check_flags(own: bool) {
    let rest = RestStart::new(Flipped(euclid().chart));
    let lz = HeldLz::new(Allowing(nu_k().chart));
    let read = |x: bool, fixed: bool| if own { fixed } else { x };
    assert!(
        read(rest.forbids_energy_normalisation(), false)
            && !read(rest.requires_per_pixel_mass(), true)
            && !read(lz.forbids_energy_normalisation(), true)
            && !read(lz.requires_per_pixel_mass(), true),
        "a wrapper's flags are not its chart's"
    );
    assert!(rest.name() == "Flipped" && lz.name() == "Allowing");
}

#[test]
fn tilt_every_chart_construction_flags() {
    check_flags(false);
}

negative_control!(
    tilt_every_chart_construction_flags,
    "a wrapper reading the registered charts' common flags, not its chart's, must fail",
    expected = "a wrapper's flags are not its chart's",
    {
        check_flags(true);
    }
);
