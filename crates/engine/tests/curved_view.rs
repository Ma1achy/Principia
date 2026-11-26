//! Save view as chart on curved views, the pivot record at a centred pin, and tilts into the held `ν₀`, `L_z` and `K`
//! (TASK-M2-63), through the chart-path object.
//!
//! **The criteria**, written before the first run:
//! - `saved_curved_view`: a shape-sphere view through a zoomed, off-centre window, tilted into a mass logit by
//!   [`Segment::curved_tilt`], saved by [`Segment::save_view_curved`], its definition through its words, and reopened at
//!   zero tilt ([`View::reopened`]): the sim key equal, and every pixel of a 17 × 17 grid of the window's addresses decodes
//!   bit for bit as the tilted view does; the same for the untilted view, which holds no tilt of its own. Then a 20° tilt into the saved target: at every pixel the kernel's placement
//!   `(a, b, d_X)` is the test's own `R·(R′·(ũ, ṽ, 0))`, `R` and `R′` read only from `TiltSpec::columns` and
//!   `TiltSpec::inputs`, each component within one ulp of the largest magnitude summed into it. The deviation from `R·R′`
//!   formed as a matrix first, then applied, is printed in ulps, a measurement. Controls: a save that drops the slice
//!   centre (the square's centre stored in place of the view's) does not reopen bitwise; a tilt into the saved target
//!   placed with the offsets added (`R′`'s offset on the target beside `R`'s, not through `R`) is not `R·R′`.
//! - `curved_pivot`: an `(L_z, E)` view tilted into a mass logit and zoomed, then locked at its centre: the record holds
//!   the pin's screen position `(½, ½)`, not assumed, and the pinned state, its chart point and hidden offsets, which
//!   decode the clicked state bit for bit, `d0` zero by value; after each of a chain of tilts and zooms about the pin the
//!   pinned pixel, through the view's own placement, decodes the pinned state bit for bit. Control: the pivot stored as
//!   chart coordinates (the pin's chart point in the record's screen-position slot) moves the pinned pixel.
//!   `curved_pivot_never_pinned`: a never-pinned tilted view's placement is the kernel's tilt about the window's centre, bit
//!   for bit, at every grid pixel, and a view with neither tilt nor record writes stage 1's bytes; control: a record
//!   placed on a never-pinned view moves it.
//! - `held_span_tilt` (R-920's predicates, chart_reference §4.5): at a single 90° tilt in the zoom-1 window, at every
//!   probe pixel, the decoded hidden value equals the sibling axis's at the same `u`: `ν₀` the Euclid plane's
//!   `ν(u) = ν_min + (ν_max − ν_min)·u`, read off the decoded triangle; the held `L_z` `√(2K_max)·(2u − 1)`, the decoded
//!   `L_z`; the held `K` `K_max·(t₀ + u − ½)^γ_K`, the decoded `K`; each within `1e-12·(1 + |value|)`. Controls: a span
//!   of 2, and a span read at the view's centre (the held `L_z`'s `2√(2K)` at the centre row's `K`), fail. A pan at fixed
//!   `τ` leaves each span bitwise unchanged; control: the centre-read span moves. A saved tilted chart reopens bitwise
//!   with its stored span at every probe pixel; control: reopened at the latent span 2 it does not.
//! - `held_k_calibration` (R-833, REQ-CHART-141), the proposal set by measurement: `K_max = K₀`, the chart's calibrated
//!   `K` (R-833), so `t₀ = 1` exactly; and `γ_K` the largest of the candidates `{1, 1.5, 2, 3}` under which every pixel of
//!   the default view tilted 90° into the held `K` (the zoom-1 square, the rail's whole extent `t₀ ± ½`, the chart's whole
//!   `θ` range) is a bound system, `E = K + U < 0`, as the chart's own `K₀` was chosen to make every one of its systems
//!   bound (the register's `burrau_theta_lz_k`); the largest because it is nearest the `γ_K = 2` every other warped `K`
//!   axis uses. Under the proposal no pixel of the default view tilted 15°, 30°, 45°, 60° or 90° is held at the floor or
//!   refused. Printed, measurements: each candidate's bound share at 90°, and the share of the default 30° view's
//!   pixels each other value moves (its decoded `K` off the proposal's by more than 1 %). Control: the invariant
//!   charts' `γ_K = 2` put in its place.
//!   `held_span_tilt_spans`: R-920's table, value by value, a mixed chart's frozen control's 2 among them; control:
//!   every span read as a latent slider's 2.
//! - `curved_tilt_frames`: the curved tilt family's interior frames, its record round trip, and a non-finite angle
//!   refused (its criterion beside it).

mod path_shim;
mod path_support;

use engine::contract::sim_config::Window;
use engine::path::{held_k, CurvedTilt, Map, Segment, SliderSpan, View, NU0_SPAN};
use kernel::chart::construction::{HELD_K, HELD_LZ, NU0};
use kernel::chart::saved::{ParentParams, SaveParent, Saved};
use kernel::chart::tilt::{Placement, Shift, TiltMap, TiltSpec, Tiltable};
use kernel::chart::{
    Burrau, BurrauThetaLz, Chart as _, ChartDecode, HeldLz, MassSimplex, RestStart, ShapeSphere,
};
use kernel::decode::{Decoded, Ic};
use kernel::generated::links::{Defaults, LinkParams};
use ledger::constants::{
    BURRAU_THETA_LZ_K, BURRAU_THETA_MAX, BURRAU_THETA_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K,
    K_MAX, MASS_SIMPLEX_EPS_M,
};
use path_support::{invariant_le, key, sim, sphere, window};
use validation::negative_control;

/// The off-centre, zoomed window of the save and pivot checks.
const W: [f64; 2] = [0.4, 0.55];

/// A 17 × 17 grid of the window's addresses, its edges and centre among them.
fn grid() -> Vec<[f64; 2]> {
    let at = |k: u32| f64::from(k) / 16.0;
    (0..=16)
        .flat_map(|j| (0..=16).map(move |i| [at(i), at(j)]))
        .collect()
}

/// A decode's every word, `−0` and NaN told apart.
fn bits(d: &Decoded<f64>) -> String {
    format!("{d:?}")
}

/// A free view of `chart` through `window`.
fn free<C: kernel::chart::Chart>(chart: &C, w: Window) -> View {
    let mut s = sim(engine::contract::sim_config::Plane {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    });
    s.chart.window = w;
    View::kernel(chart, s).expect("a registry chart")
}

/// `a` tilted by `spec`, through the object.
fn tilt<C: SliderSpan>(a: &View, chart: &C, spec: TiltSpec) -> View {
    Segment::curved_tilt(a, chart, &spec)
        .expect("a curved tilt")
        .b()
        .clone()
}

fn sheet(tau1: f64, tau2: f64, x: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map: TiltMap::Sheet,
        x,
        y: x,
    }
}

/// One ulp of `x`'s magnitude.
fn ulp(x: f64) -> f64 {
    let x = x.abs();
    x.next_up() - x
}

// ── saved_curved_view ─────────────────────────────────────────────────────────────────────────────────────────────

/// How the view is saved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Save {
    /// The object's Save view as chart.
    Object,
    /// The slice centre dropped: the square's centre stored in place of the view's.
    #[cfg(feature = "controls")]
    DropCentre,
}

/// How a tilt of the saved chart is placed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Compose {
    /// The kernel's composition, through the reopened view.
    Kernel,
    /// `R′`'s offset on the saved target added beside `R`'s.
    #[cfg(feature = "controls")]
    OffsetsAdded,
}

/// The saved target: the shape sphere's first mass logit.
const MASS: u32 = 6;

/// The tilted sphere view, the saved view, and the saved definition read back from its words.
fn saved_sphere(save: Save) -> (View, View) {
    saved_sphere_at(save, sheet(25.0, -40.0, MASS))
}

/// The sphere view tilted by `spec`, saved through `save` and read back from its words; at zero angles the view is
/// untilted and holds no tilt of its own.
fn saved_sphere_at(save: Save, spec: TiltSpec) -> (View, View) {
    let free_view = free(&sphere(), window(W, 0.25));
    let a = match spec.untilted() {
        true => free_view,
        false => tilt(&free_view, &sphere(), spec),
    };
    let seg = match save {
        Save::Object => Segment::save_view_curved(&a, &sphere()),
        #[cfg(feature = "controls")]
        Save::DropCentre => match a.tilt {
            Some(t) => Segment::save_curved(&a, &sphere(), t.spec(), [0.5, 0.5], t.span),
            None => Segment::save_view_curved(&a, &sphere()),
        },
    }
    .expect("Save view as chart");
    let Map::Saved(saved) = &seg.b().map else {
        panic!("Save view as chart gives no saved chart");
    };
    let read = Saved::from_words(&saved.to_words()).expect("the words read back");
    let b = View {
        map: Map::Saved(Box::new(read)),
        ..seg.b().clone()
    };
    (a, b)
}

/// The saved view reopened at zero tilt, against the view it was saved from: tilted, then untilted.
fn check_reopens(save: Save) {
    check_reopens_at(save, sheet(25.0, -40.0, MASS));
    check_reopens_at(save, sheet(0.0, 0.0, MASS));
}

fn check_reopens_at(save: Save, spec: TiltSpec) {
    let (a, b) = saved_sphere_at(save, spec);
    assert_eq!(key(&a.sim), key(&b.sim), "the sim key moved");
    let tilted = a.tilted(sphere()).expect("the tilted view");
    let reopened = b
        .reopened::<ShapeSphere<f64>>()
        .expect("the saved view reopened");
    let w = a.sim.chart.window;
    for [s, t] in grid() {
        let [u, v] = w.at(s, t);
        assert_eq!(
            bits(&reopened.decode::<Defaults, f64>(u, v)),
            bits(&tilted.decode::<Defaults, f64>(u, v)),
            "the saved view does not reopen bitwise at ({s}, {t})"
        );
    }
}

#[test]
fn saved_curved_view_reopens_bitwise() {
    check_reopens(Save::Object);
}

negative_control!(
    saved_curved_view_reopens_bitwise,
    "a saved curved chart that drops the slice centre",
    expected = "does not reopen bitwise",
    check_reopens(Save::DropCentre)
);

/// `Σ terms`, summed left to right, and the largest magnitude in it.
fn sum(terms: &[f64]) -> (f64, f64) {
    let total = terms.iter().fold(0.0, |acc, t| acc + t);
    (total, terms.iter().fold(0.0f64, |m, t| m.max(t.abs())))
}

/// The test's own `R·(R′·(ũ, ṽ, 0))` at window point `(u, v)`: `(a, b, d_X)` and the largest magnitude summed into each,
/// `R` the saved tilt about `s` with spans `span`, `R′` the new tilt about the view's centre `c`, both into `X`.
fn composed(
    saved: &TiltSpec,
    s: [f64; 2],
    span: f64,
    new: &TiltSpec,
    c: [f64; 2],
    [u, v]: [f64; 2],
) -> [(f64, f64); 3] {
    let [r1, r2] = saved.columns();
    let [ix, _] = saved.inputs().expect("R has an input at X");
    let [n1, n2] = new.columns();
    let (du, dv) = (u - c[0], v - c[1]);
    let y: [f64; 4] = std::array::from_fn(|k| n1[k] * du + n2[k] * dv);
    let (p0, p1) = ((c[0] - s[0]) + y[0], (c[1] - s[1]) + y[1]);
    let row = |k: usize| sum(&[r1[k] * p0, r2[k] * p1, ix[k] * y[2]]);
    let (da, ma) = row(0);
    let (db, mb) = row(1);
    let (dx, mx) = row(2);
    [
        (s[0] + da, ma.max(s[0].abs()).max((s[0] + da).abs())),
        (s[1] + db, mb.max(s[1].abs()).max((s[1] + db).abs())),
        (span * dx, span * mx),
    ]
}

/// `R·R′` formed as a matrix first, then applied: `(a, b, d_X)`.
fn product_first(
    saved: &TiltSpec,
    s: [f64; 2],
    span: f64,
    new: &TiltSpec,
    c: [f64; 2],
    [u, v]: [f64; 2],
) -> [f64; 3] {
    let [r1, r2] = saved.columns();
    let [ix, iy] = saved.inputs().expect("R has an input at X");
    let r = [r1, r2, ix, iy];
    let [n1, n2] = new.columns();
    let m = |col: [f64; 4], k: usize| (0..4).fold(0.0, |acc, j| acc + r[j][k] * col[j]);
    let (du, dv) = (u - c[0], v - c[1]);
    let at = |k: usize| m(n1, k) * du + m(n2, k) * dv;
    [s[0] + at(0), s[1] + at(1), span * at(2)]
}

fn check_composes(compose: Compose) {
    let (_, b) = saved_sphere(Save::Object);
    let Map::Saved(def) = &b.map else {
        unreachable!()
    };
    let Saved::Curved(curved) = **def else {
        panic!("not a curved chart")
    };
    let new = sheet(20.0, 0.0, MASS);
    let c = tilt(&b, &sphere(), new);
    let reopened = c.reopened::<ShapeSphere<f64>>().expect("reopened");
    let w = c.sim.chart.window;
    let (mut worst, mut worst_product) = (0.0f64, 0.0f64);
    for [s, t] in grid() {
        let [u, v] = w.at(s, t);
        let ([a, bb], shift) = match compose {
            Compose::Kernel => Placement::place(reopened.tilt(), u, v),
            #[cfg(feature = "controls")]
            Compose::OffsetsAdded => {
                let untilted = sheet(0.0, 0.0, MASS);
                let ([a, bb], mut shift) = Placement::place(
                    curved
                        .open::<ShapeSphere<f64>>(&untilted, w.centre, curved.span)
                        .expect("open")
                        .tilt(),
                    u,
                    v,
                );
                let [n1, n2] = new.columns();
                let e = n1[2] * (u - w.centre[0]) + n2[2] * (v - w.centre[1]);
                shift.dx += curved.span[0] * e;
                ([a, bb], shift)
            }
        };
        assert_eq!(shift.x, MASS, "the offset is not on the saved target");
        let want = composed(
            &curved.tilt,
            curved.centre,
            curved.span[0],
            &new,
            w.centre,
            [u, v],
        );
        for (i, (got, (value, largest))) in [a, bb, shift.dx].into_iter().zip(want).enumerate() {
            let off = (got - value).abs();
            assert!(
                off <= ulp(largest),
                "the tilt into the saved target does not decode as R·R′ at ({s}, {t}), component {i}: \
                 {got:e} against {value:e}, off by {off:e} > one ulp of {largest:e}"
            );
            worst = worst.max(off / ulp(largest));
        }
        let product = product_first(
            &curved.tilt,
            curved.centre,
            curved.span[0],
            &new,
            w.centre,
            [u, v],
        );
        for (got, (want, (_, largest))) in [a, bb, shift.dx]
            .into_iter()
            .zip(product.into_iter().zip(want))
        {
            worst_product = worst_product.max((got - want).abs() / ulp(largest));
        }
    }
    eprintln!(
        "saved_curved_view: R·(R′·x) to {worst:.3} ulp at worst; R·R′ formed first, then applied, to {worst_product:.3} ulp (measured)"
    );
}

#[test]
fn saved_curved_view_composes() {
    check_composes(Compose::Kernel);
}

negative_control!(
    saved_curved_view_composes,
    "a tilt into the saved target placed with the offsets added",
    expected = "does not decode as R·R′",
    check_composes(Compose::OffsetsAdded)
);

// ── curved_pivot ──────────────────────────────────────────────────────────────────────────────────────────────────

/// How the pivot is recorded.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Record {
    /// The object's record.
    Object,
    /// The pin's chart point stored in the record's screen-position slot.
    #[cfg(feature = "controls")]
    ChartCoordinates,
}

fn ic_bits(ic: &Ic<f64>) -> Vec<u64> {
    ic.m.iter()
        .chain(ic.r.iter().flatten())
        .chain(ic.p.iter().flatten())
        .map(|x| x.to_bits())
        .collect()
}

fn check_pivot(record: Record) {
    let chart = invariant_le();
    let tol = engine::lookup::Tolerances::register(128.0 * f64::EPSILON);
    let a = free(&chart, window([0.45, 0.55], 0.25));
    let a = tilt(&a, &chart, sheet(25.0, 0.0, MASS));
    let a = Segment::zoom(&a, 0.5).expect("a zoom").b().clone();
    let (seg, found) = Segment::lock_on_chart(&a, &path_shim::lockable(&chart), (0.5, 0.5), &tol)
        .expect("a lock at the centre");
    let mut view = seg.b().clone();
    let pivot = view.pivot.expect("a pivot record");
    let state = chart
        .decode_shifted::<Defaults, f64>(
            pivot.point[0],
            pivot.point[1],
            &Shift {
                x: MASS,
                dx: pivot.d0[0],
                y: MASS,
                dy: pivot.d0[1],
                two: false,
                ..Shift::none()
            },
            &LinkParams::DEFAULT,
        )
        .ic;
    assert_eq!(
        ic_bits(&state),
        ic_bits(&found.ic),
        "the record's pinned state is not the clicked state"
    );
    assert_eq!(pivot.d0.map(|d| d == 0.0), [true, true], "d0 is not zero");
    let _ = record;
    #[cfg(feature = "controls")]
    if record == Record::ChartCoordinates {
        let w = view.sim.chart.window;
        view.pivot = Some(engine::path::Pivot {
            address: w.at(0.5, 0.5),
            ..pivot
        });
    }
    let pinned = |v: &View| {
        let [u, w] = v.sim.chart.window.at(0.5, 0.5);
        v.tilted(chart)
            .expect("the view")
            .decode::<Defaults, f64>(u, w)
            .ic
    };
    type Step<'a> = &'a dyn Fn(&View) -> View;
    let steps: [Step; 5] = [
        &|v| tilt(v, &chart, sheet(-30.0, 15.0, MASS)),
        &|v| Segment::zoom(v, 1.25).expect("a zoom").b().clone(),
        &|v| {
            tilt(
                v,
                &chart,
                TiltSpec {
                    tau1: 10.0,
                    tau2: -20.0,
                    map: TiltMap::Lean,
                    x: MASS,
                    y: 0,
                },
            )
        },
        &|v| Segment::zoom(v, -0.6).expect("a zoom").b().clone(),
        &|v| tilt(v, &chart, sheet(60.0, 0.0, MASS)),
    ];
    for (i, step) in steps.iter().enumerate() {
        view = step(&view);
        assert_eq!(
            ic_bits(&pinned(&view)),
            ic_bits(&state),
            "the pinned pixel moved after step {i}"
        );
    }
    let kept = view.pivot.expect("the record is kept");
    assert_eq!(
        (kept.address.map(f64::to_bits), kept.assumed),
        ([0.5f64.to_bits(); 2], false),
        "the record's screen position is not the pin's"
    );
    assert_eq!(kept, pivot, "the record moved");
}

#[test]
fn curved_pivot() {
    check_pivot(Record::Object);
}

negative_control!(
    curved_pivot,
    "the pivot stored as chart coordinates",
    expected = "the pinned pixel moved",
    check_pivot(Record::ChartCoordinates)
);

/// Whether a never-pinned view is given a record.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NeverPinned {
    Object,
    #[cfg(feature = "controls")]
    Recorded,
}

fn check_never_pinned(variant: NeverPinned) {
    let chart = sphere();
    let bare = free(&chart, window(W, 0.25));
    let text = String::from_utf8(bare.bytes()).expect("utf-8");
    assert!(
        !text.contains("\"tilt\"") && !text.contains("\"pivot\""),
        "an untilted, never-pinned view's bytes are not stage 1's: {text}"
    );
    for spec in [
        sheet(25.0, -40.0, MASS),
        TiltSpec {
            tau1: -15.0,
            tau2: 35.0,
            map: TiltMap::Lean,
            x: MASS,
            y: 3,
        },
    ] {
        let v = tilt(&bare, &chart, spec);
        let v = Segment::zoom(&v, 0.75).expect("a zoom").b().clone();
        #[allow(unused_mut)]
        let mut v = v;
        #[cfg(feature = "controls")]
        if variant == NeverPinned::Recorded {
            v.pivot = Some(engine::path::Pivot {
                address: [0.5, 0.5],
                assumed: true,
                point: [0.5, 0.5],
                d0: [0.0, 0.0],
            });
        }
        let _ = variant;
        let t = v.tilt.expect("a tilt");
        let w = v.sim.chart.window;
        let today = kernel::chart::tilt::Tilt::new(&spec, w.centre, t.span);
        let object = v.placement().expect("a placement");
        for [s, tt] in grid() {
            let [u, vv] = w.at(s, tt);
            let (p, q) = (Placement::place(&object, u, vv), today.place(u, vv));
            assert_eq!(
                format!("{p:?}"),
                format!("{q:?}"),
                "a never-pinned view is not today's tilt at ({s}, {tt})"
            );
        }
    }
}

#[test]
fn curved_pivot_never_pinned() {
    check_never_pinned(NeverPinned::Object);
}

negative_control!(
    curved_pivot_never_pinned,
    "a record placed on a never-pinned view",
    expected = "is not today's tilt",
    check_never_pinned(NeverPinned::Recorded)
);

// ── held_span_tilt ────────────────────────────────────────────────────────────────────────────────────────────────

/// Which span the tilt carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Span {
    /// R-920's, through the object.
    Object,
    /// A latent slider's span, 2.
    #[cfg(feature = "controls")]
    Two,
    /// Read at the view's centre: the held `L_z`'s `2√(2K)` at the centre row's `K`.
    #[cfg(feature = "controls")]
    CentreRead,
}

/// The mass simplex at rest, `ν₀ = ½`, the register's buffer.
fn simplex() -> RestStart<MassSimplex<f64>, f64> {
    RestStart::new(MassSimplex {
        nu0: 0.5,
        eps_m: MASS_SIMPLEX_EPS_M.number(),
    })
}

/// The `(ν, K)` chart at the register's defaults, holding `L_z = 0`.
fn nu_k() -> HeldLz<Burrau<f64>, f64> {
    HeldLz::new(Burrau {
        nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    })
}

/// `Φ_θ,L_z` at the register's defaults, holding `K₀` under the calibrated warp.
fn theta_lz() -> kernel::chart::HeldK<f64> {
    held_k(BurrauThetaLz {
        theta_range: [BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number()],
        k: BURRAU_THETA_LZ_K.number(),
        lz_fraction: 1.0,
    })
}

/// The `(ν, K)` chart's `K` at the window's centre row, under its warp.
#[cfg(feature = "controls")]
fn k_at_centre(w: &Window) -> f64 {
    K_MAX.number() * w.centre[1].powf(GAMMA_K.number())
}

/// The span a tilt carries, by the variant.
fn span_of(view: &View, k: u32, variant: Span) -> f64 {
    let own = view.tilt.expect("a tilt").span[0];
    let _ = k;
    match variant {
        Span::Object => own,
        #[cfg(feature = "controls")]
        Span::Two => 2.0,
        #[cfg(feature = "controls")]
        Span::CentreRead => match k {
            HELD_LZ => 2.0 * (2.0 * k_at_centre(&view.sim.chart.window)).sqrt(),
            _ => own,
        },
    }
}

/// `view` with its tilt's spans set to `span`.
fn with_span(view: &View, span: f64) -> View {
    View {
        tilt: view.tilt.map(|t| CurvedTilt {
            span: [span, span],
            ..t
        }),
        ..view.clone()
    }
}

/// The probe: every eighth of `u` across the zoom-1 window, at five rows.
fn probe() -> Vec<[f64; 2]> {
    (0..=8)
        .flat_map(|i| [0.1, 0.3, 0.5, 0.7, 0.9].map(|t| [f64::from(i) / 8.0, t]))
        .collect()
}

fn dist(p: [f64; 2], q: [f64; 2]) -> f64 {
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt()
}

/// `ν` of the Burrau right triangle the decode holds: `s₀₂/s₁₂ = 2ν/(1 + ν²)`.
fn nu_of(d: &Decoded<f64>) -> f64 {
    let r = d.ic.r;
    let rho = dist(r[0], r[2]) / dist(r[1], r[2]);
    (1.0 - (1.0 - rho * rho).sqrt()) / rho
}

fn lz_of(d: &Decoded<f64>) -> f64 {
    (0..3)
        .map(|i| d.ic.r[i][0] * d.ic.p[i][1] - d.ic.r[i][1] * d.ic.p[i][0])
        .sum()
}

fn k_of(d: &Decoded<f64>) -> f64 {
    (0..3)
        .map(|i| (d.ic.p[i][0].powi(2) + d.ic.p[i][1].powi(2)) / (2.0 * d.ic.m[i]))
        .sum()
}

/// One arm of the 90° check: `chart` tilted 90° into `k` in the zoom-1 window, each probe pixel's decoded hidden value
/// `read` against the sibling axis's `sibling(u)`.
fn arm<C>(
    name: &str,
    chart: C,
    k: u32,
    variant: Span,
    read: fn(&Decoded<f64>) -> f64,
    sibling: &dyn Fn(f64) -> f64,
) where
    C: SliderSpan + Tiltable<Param = f64> + ChartDecode + Copy,
{
    let view = tilt(&free(&chart, Window::SQUARE), &chart, sheet(90.0, 0.0, k));
    let view = with_span(&view, span_of(&view, k, variant));
    let tv = view.tilted(chart).expect("the tilted view");
    for [s, t] in probe() {
        let [u, v] = view.sim.chart.window.at(s, t);
        let got = read(&tv.decode::<Defaults, f64>(u, v));
        let want = sibling(u);
        assert!(
            (got - want).abs() <= 1e-12 * (1.0 + want.abs()),
            "{name}: at a 90° tilt the hidden value at u = {u} is {got}, not the sibling axis's {want}"
        );
    }
}

fn check_ninety(variant: Span) {
    let (nu_min, nu_max) = (EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number());
    arm("ν₀", simplex(), NU0, variant, nu_of, &|u| {
        nu_min + (nu_max - nu_min) * u
    });
    let k_max = K_MAX.number();
    arm("held L_z", nu_k(), HELD_LZ, variant, lz_of, &|u| {
        (2.0 * k_max).sqrt() * (2.0 * u - 1.0)
    });
    let held = theta_lz();
    let t0 = (held.chart.k / held.k_max).powf(held.gamma_k.recip());
    arm("held K", held, HELD_K, variant, k_of, &|u| {
        held.k_max * (t0 + u - 0.5).powf(held.gamma_k)
    });
}

#[test]
fn held_span_tilt_ninety() {
    check_ninety(Span::Object);
}

negative_control!(
    held_span_tilt_ninety,
    "a span of 2",
    expected = "not the sibling axis's",
    check_ninety(Span::Two)
);

mod centre_read {
    use super::*;
    negative_control!(
        held_span_tilt_ninety,
        "a span read at the view's centre",
        expected = "not the sibling axis's",
        check_ninety(Span::CentreRead)
    );
}

/// A pan of `view` at fixed `τ`: its window's centre moved, then the same tilt again through the object.
fn panned<C: SliderSpan>(view: &View, chart: &C, spec: TiltSpec) -> View {
    let mut moved = view.clone();
    moved.sim.chart.window.centre = [0.31, 0.72];
    tilt(&moved, chart, spec)
}

fn pan_arm<C: SliderSpan>(name: &str, chart: C, k: u32, variant: Span) {
    let spec = sheet(30.0, 0.0, k);
    let view = tilt(&free(&chart, Window::SQUARE), &chart, spec);
    let after = panned(&view, &chart, spec);
    assert_eq!(
        span_of(&after, k, variant).to_bits(),
        span_of(&view, k, variant).to_bits(),
        "{name}: a pan at fixed τ moved the span"
    );
}

fn check_pan(variant: Span) {
    pan_arm("ν₀", simplex(), NU0, variant);
    pan_arm("held L_z", nu_k(), HELD_LZ, variant);
    pan_arm("held K", theta_lz(), HELD_K, variant);
}

#[test]
fn held_span_tilt_pan() {
    check_pan(Span::Object);
}

negative_control!(
    held_span_tilt_pan,
    "a span read at the view's centre",
    expected = "a pan at fixed τ moved the span",
    check_pan(Span::CentreRead)
);

/// Whether the reopened chart keeps its stored span.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stored {
    Object,
    #[cfg(feature = "controls")]
    Latent,
}

fn saved_arm<C>(name: &str, chart: C, k: u32, stored: Stored)
where
    C: SliderSpan + Tiltable<Param = f64> + ChartDecode + SaveParent + ParentParams + Copy,
{
    let view = tilt(&free(&chart, Window::SQUARE), &chart, sheet(30.0, 0.0, k));
    let seg = Segment::save_view_curved(&view, &chart).expect("Save view as chart");
    let Map::Saved(saved) = &seg.b().map else {
        panic!("no saved chart")
    };
    let Some(Saved::Curved(curved)) = Saved::from_words(&saved.to_words()) else {
        panic!("not a curved chart")
    };
    assert_eq!(
        curved.span[0].to_bits(),
        view.tilt.expect("a tilt").span[0].to_bits(),
        "{name}: the stored span is not the view's"
    );
    #[cfg(feature = "controls")]
    let curved = match stored {
        Stored::Latent => kernel::chart::saved::Curved {
            span: [2.0, 2.0],
            ..curved
        },
        Stored::Object => curved,
    };
    let _ = stored;
    let reopened = View {
        map: Map::Saved(Box::new(Saved::Curved(curved))),
        ..seg.b().clone()
    };
    let (a, b) = (
        view.tilted(chart).expect("the tilted view"),
        reopened.reopened::<C>().expect("reopened"),
    );
    for [s, t] in probe() {
        let [u, v] = view.sim.chart.window.at(s, t);
        assert_eq!(
            bits(&b.decode::<Defaults, f64>(u, v)),
            bits(&a.decode::<Defaults, f64>(u, v)),
            "{name}: the saved tilted chart does not reopen bitwise at ({s}, {t})"
        );
    }
}

fn check_saved(stored: Stored) {
    saved_arm("ν₀", simplex(), NU0, stored);
    saved_arm("held L_z", nu_k(), HELD_LZ, stored);
    saved_arm("held K", theta_lz(), HELD_K, stored);
}

#[test]
fn held_span_tilt_saved() {
    check_saved(Stored::Object);
}

negative_control!(
    held_span_tilt_saved,
    "a saved tilted chart reopened at the latent span, not its stored one",
    expected = "does not reopen bitwise",
    check_saved(Stored::Latent)
);

/// How a chart's span is read.
type SpanRead = fn(&dyn Fn(u32) -> Option<f64>, u32) -> Option<f64>;

fn own(span: &dyn Fn(u32) -> Option<f64>, k: u32) -> Option<f64> {
    span(k)
}

fn check_spans(read: SpanRead) {
    let at = |chart: &dyn Fn(u32) -> Option<f64>, k: u32| read(chart, k);
    assert_eq!(NU0_SPAN, 15.0 / 16.0, "ν₀'s span is not 15/16");
    assert_eq!(
        at(&|k| simplex().slider_span(k), NU0),
        Some(15.0 / 16.0),
        "ν₀'s span is not 15/16"
    );
    assert_eq!(
        at(&|k| simplex().slider_span(k), 2),
        Some(2.0),
        "a momentum control's span is not 2"
    );
    assert_eq!(
        at(&|k| nu_k().slider_span(k), HELD_LZ),
        Some(2.0 * (2.0 * K_MAX.number()).sqrt()),
        "the held L_z's span is not 2√(2K_max)"
    );
    assert_eq!(
        at(&|k| theta_lz().slider_span(k), HELD_K),
        Some(1.0),
        "the warped held K's span is not 1"
    );
    let linear = kernel::chart::HeldK {
        mapping: kernel::chart::Mapping::Linear,
        ..theta_lz()
    };
    assert_eq!(
        at(&|k| linear.slider_span(k), HELD_K),
        Some(linear.k_max),
        "the linear held K's span is not K_max"
    );
    assert_eq!(
        at(&|k| sphere().slider_span(k), MASS),
        Some(2.0),
        "a mass logit's span is not 2"
    );
    assert_eq!(
        at(&|k| sphere().slider_span(k), 0),
        None,
        "an axis is given a span"
    );
    let m = path_support::mixed();
    let frozen = (0..8).find(|k| kernel::chart::tilt::Hidden::hidden(&m, *k));
    let frozen = frozen.expect("a mixed chart has a frozen control");
    assert_eq!(
        at(&|k| m.slider_span(k), frozen),
        Some(2.0),
        "a mixed chart's frozen control's span is not 2"
    );
}

#[test]
fn held_span_tilt_spans() {
    check_spans(own);
}

/// Every span read as a latent slider's.
#[cfg(feature = "controls")]
fn latent(span: &dyn Fn(u32) -> Option<f64>, k: u32) -> Option<f64> {
    span(k).map(|_| 2.0)
}

negative_control!(
    held_span_tilt_spans,
    "every span read as a latent slider's 2",
    expected = "ν₀'s span is not 15/16",
    check_spans(latent)
);

// ── held_k_calibration ────────────────────────────────────────────────────────────────────────────────────────────

/// `Φ_θ,L_z` at `K₀` under the warp `(k_max, gamma_k)`.
fn theta_lz_at(k_max: f64, gamma_k: f64) -> kernel::chart::HeldK<f64> {
    kernel::chart::HeldK::new(theta_lz().chart, k_max, gamma_k)
}

/// The default tilted view's 32 × 32 pixel centres.
fn pixels() -> Vec<[f64; 2]> {
    let at = |k: u32| (f64::from(k) + 0.5) / 32.0;
    (0..32)
        .flat_map(|j| (0..32).map(move |i| [at(i), at(j)]))
        .collect()
}

/// `E = K + U` of a decoded state, `G = 1`.
fn energy(d: &Decoded<f64>) -> f64 {
    let ic = &d.ic;
    let pair = |i: usize, j: usize| -ic.m[i] * ic.m[j] / dist(ic.r[i], ic.r[j]);
    k_of(d) + pair(0, 1) + pair(0, 2) + pair(1, 2)
}

/// The default view tilted 90° into the held `K` on `τ₂`, the `L_z` axis, so the `θ` axis keeps the chart's whole range.
fn ninety_on_lz() -> TiltSpec {
    sheet(0.0, 90.0, HELD_K)
}

/// How many of the 90°-tilted view's pixels are bound under `held`, and how many it has.
fn bound_at_ninety(held: kernel::chart::HeldK<f64>) -> (usize, usize) {
    let view = tilt(&free(&held, Window::SQUARE), &held, ninety_on_lz());
    let tv = view.tilted(held).expect("the tilted view");
    let all = pixels();
    let bound = all
        .iter()
        .filter(|[u, v]| energy(&tv.decode::<Defaults, f64>(*u, *v)) < 0.0)
        .count();
    (bound, all.len())
}

/// The candidate exponents, in increasing order.
const GAMMAS: [f64; 4] = [1.0, 1.5, 2.0, 3.0];

fn check_calibration(proposal: kernel::chart::HeldK<f64>) {
    let k0 = proposal.chart.k;
    assert_eq!(proposal.k_max, k0, "K_max is not the chart's calibrated K");
    let t0 = (k0 / proposal.k_max).powf(proposal.gamma_k.recip());
    assert_eq!(t0, 1.0, "t₀ is not 1");
    let mut passing = Vec::new();
    for gamma in GAMMAS {
        let (bound, n) = bound_at_ninety(theta_lz_at(k0, gamma));
        eprintln!(
            "held_k_calibration: γ_K = {gamma}: {bound}/{n} = {:.4} of the 90° view's pixels bound",
            bound as f64 / n as f64
        );
        if bound == n {
            passing.push(gamma);
        }
    }
    let chosen = passing.last().copied();
    assert_eq!(
        Some(proposal.gamma_k),
        chosen,
        "γ_K is not the largest candidate that keeps the 90° view bound (passing: {passing:?})"
    );
    let n = pixels().len();
    for tau in [15.0, 30.0, 45.0, 60.0, 90.0] {
        let view = tilt(
            &free(&proposal, Window::SQUARE),
            &proposal,
            sheet(tau, 0.0, HELD_K),
        );
        let tv = view.tilted(proposal).expect("the tilted view");
        let floored = pixels()
            .into_iter()
            .filter(|&[u, v]| tv.held_at_floor(u, v))
            .count();
        let refused = pixels()
            .into_iter()
            .filter(|&[u, v]| tv.validate(u, v) != kernel::chart::ValidationResult::Valid)
            .count();
        eprintln!("held_k_calibration: τ = {tau}°: {floored}/{n} held at the floor, {refused}/{n} refused");
        assert_eq!(
            (floored, refused),
            (0, 0),
            "the default tilted view at τ = {tau}° leaves the warp"
        );
    }
    let k_at = |held: kernel::chart::HeldK<f64>| -> Vec<f64> {
        let view = tilt(
            &free(&held, Window::SQUARE),
            &held,
            sheet(30.0, 0.0, HELD_K),
        );
        let tv = view.tilted(held).expect("the tilted view");
        pixels()
            .into_iter()
            .map(|[u, v]| k_of(&tv.decode::<Defaults, f64>(u, v)))
            .collect()
    };
    let base = k_at(proposal);
    let others = [
        ("K_max = K₀/2", k0 / 2.0, proposal.gamma_k),
        ("K_max = 2K₀", 2.0 * k0, proposal.gamma_k),
    ]
    .into_iter()
    .chain(
        GAMMAS
            .iter()
            .filter(|g| **g != proposal.gamma_k)
            .map(|g| ("γ_K other", k0, *g)),
    );
    for (label, k_max, gamma_k) in others {
        let held = theta_lz_at(k_max, gamma_k);
        let mut change: Vec<f64> = k_at(held)
            .iter()
            .zip(&base)
            .map(|(k, b)| (k - b).abs() / b.abs())
            .collect();
        let moved = change.iter().filter(|c| **c > 0.01).count();
        change.sort_by(f64::total_cmp);
        let (median, max) = (change[n / 2], change[n - 1]);
        eprintln!(
            "held_k_calibration: {label} (K_max = {k_max}, γ_K = {gamma_k}): moves {moved}/{n} = {:.3} of the default \
             30° view's pixels' K by > 1 % (|ΔK|/K median {median:.3}, max {max:.3})",
            moved as f64 / n as f64
        );
    }
}

#[test]
fn held_k_calibration() {
    check_calibration(theta_lz());
}

negative_control!(
    held_k_calibration,
    "the invariant charts' γ_K = 2 in place of the measured one",
    expected = "γ_K is not the largest candidate",
    check_calibration(theta_lz_at(BURRAU_THETA_LZ_K.number(), GAMMA_K.number()))
);

// ── The curved tilt's frames ──────────────────────────────────────────────────────────────────────────────────────
//
// The criterion: a curved tilt from 0° to 40° is a `CurvedTilt` family whose frame at `s` holds the tilt at `40·s`
// degrees, its map, targets and spans the end's, and the frame its record names, read back from its bytes, is that
// frame bit for bit; a tilt that changes its target is a cut; a non-finite angle is refused. Control: the frame taken
// as `A`, as a cut's is.

/// The frame between a segment's ends.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Between {
    Object,
    #[cfg(feature = "controls")]
    AsCut,
}

fn check_frames(between: Between) {
    let chart = sphere();
    let a = free(&chart, window(W, 0.25));
    let seg = Segment::curved_tilt(&a, &chart, &sheet(40.0, 0.0, MASS)).expect("a curved tilt");
    assert!(
        matches!(seg.family(), engine::path::Family::CurvedTilt { .. }),
        "not a curved tilt family"
    );
    for s in [0.25, 0.5, 0.75] {
        let frame = match between {
            Between::Object => seg.frame(s).expect("a frame"),
            #[cfg(feature = "controls")]
            Between::AsCut => seg.a().clone(),
        };
        let held = frame.tilt.map(|t| (t.tau1, t.tau2, t.x, t.span));
        assert_eq!(
            held,
            Some((0.0 + s * 40.0, 0.0, MASS, [2.0, 2.0])),
            "the frame at s = {s} is not the tilt at 40·s"
        );
        let record = engine::path::FrameRecord::read(&seg.record(s).expect("a record").to_bytes())
            .expect("the record reads back");
        assert_eq!(
            record.frame().expect("the record's frame").bytes(),
            frame.bytes(),
            "the record names another frame at s = {s}"
        );
    }
    assert!(
        matches!(
            Segment::curved_tilt(&a, &chart, &sheet(f64::NAN, 0.0, MASS)),
            Err(engine::path::PathError::Nav(
                engine::nav::NavError::NonFinite
            ))
        ),
        "a non-finite angle is accepted"
    );
    let other = Segment::curved_tilt(seg.b(), &chart, &sheet(40.0, 0.0, 7)).expect("a tilt");
    assert!(
        matches!(
            other.family(),
            engine::path::Family::Cut {
                label: engine::path::CutLabel::TiltReplaces
            }
        ),
        "a tilt into another target is not a cut"
    );
}

#[test]
fn curved_tilt_frames() {
    check_frames(Between::Object);
}

negative_control!(
    curved_tilt_frames,
    "the frame taken as A, as a cut's is",
    expected = "is not the tilt at 40·s",
    check_frames(Between::AsCut)
);
