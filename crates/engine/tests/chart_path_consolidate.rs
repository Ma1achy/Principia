//! The chart path consolidated: one leaf per operation, every navigation edit through the stop, the gui's
//! routes onto the same leaves.
//!
//! **`path_merge_bitwise`.** The gui's own copies of pan (a drag), zoom (the wheel and + / −), the zoom row (`q·(σ′/σ)`,
//! now the zoom leaf's factor form), the lock at a pixel (K, lock here) and a Slice centre slider's move, frozen here as they stood on `main` ([`old`]), against the engine leaf
//! each now calls. Criterion, written before the run: every case's edits equal bit for bit, on the stage-1 planes and
//! 2,000 fuzzed planes.
//!
//! **`path_canonical_conversion`.** The routes whose formula differed from the leaf's in rounding alone, each now the
//! leaf's formula, old against new, with the largest difference against the count derived from the two formulas,
//! written before the run:
//! - a pan along one axis (the arrows, the centre `u` and `v` rows) is now the pan family's `z₀ + 2ds·q₁ + 2dt·q₂` with
//!   the other step 0, which adds `0·q`: it changes exactly the components where the old sum was `−0` and the added
//!   `0·q` is `+0`, each to `+0`, a difference of 0 in value. Criterion: the components that change are exactly those
//!   predicted, and each is a signed zero.
//! - the tilt (Tilt's sliders, the arrows on `τ₁`, `τ₂`, `γ`, the compass's step and drag), the gui's `View::basis` on
//!   std's `sin_cos`, is now the lean-each-axis tilt (`Family::R474Tilt`) through `kernel::chart::tilt::latent_basis`,
//!   the kernel's pinned `sin` and `cos`. Each within 1 ulp of the value, at most 1, so `|Δq_i| ≤ 8ε·σ` per component
//!   (four products of terms off by `ε` at most, and the rounding of two sums). Criterion: every component within
//!   `8ε·σ`.
//! - the compass's slice is not converted: modelled here first as normal to the plane, the gui's own test on CI
//!   measured its drawn normal 1.6·10⁻³ off the slice leaf's step on a tilted plane, which is not rounding; it stays
//!   the compass's (`crates/gui/tests/chart_path_consolidate.rs`, `path_slice_not_merged`).
//!
//! Each changed case's view also decodes to the same system (`validation::roundtrip::phys`, within `ε_phys`) at
//! every pixel of a 3 × 3 probe whose decoded configuration is farther than `δ` from a binary collision, the skipped
//! pixels counted and reported. (A first run read the zoom row's conversion on the sliders' rail alone, after a fuzzed
//! zoom past the gui's span failed off it; the physics review measured a reachable off-rail pixel 1.424·10⁻¹⁴ from the
//! old, past `ε_phys`. The zoom row now keeps its bits, and the arm asserts at every pixel, the rail only a reported
//! split.) The plan is unclear on
//! `δ`; chose `δ = ε/ε_phys = 1/64` of the configuration's extent, where one ulp of the latent point is `ε_phys` of the
//! state, because the decode's conditioning near a collision is what `ε_phys` does not cover.
//!
//! **`path_pin_kept`.** Locked off the centre, a zoom and a tilt through their leaves keep the pinned pixel on its held
//! point: bit for bit wherever `hold_plane` reports it held, and the leaf's view is its family constructor's, bit for bit.
//!
//! **`nav_stop_edit_path`**. On the linear `(L_z, E)` chart a pan past the feasible edge through the
//! leaf: the edits recorded hold the window's centre the kernel's stop accepts, bit for bit, the stop naming layer 2
//! and `LzEParabola`; a drag held past the edge sends nothing new, and one brought back inside sends its value.
//!
//! **Controls**: a pan with one operation reordered fails the merge; the requested value recorded, and a stop applied
//! after the edit was recorded, each fail the stop's test; a conversion's bound at a sixty-fourth of its derived count
//! fails, and so does `ε_phys` at a sixty-fourth; the zoom row sent as octaves fails the merge; a stopped pan's pin
//! re-read at its address leaves the clicked state.

// Not on Windows: the tilt's old route here is std's `sin_cos`, the platform's libm.
#![cfg(not(windows))]

mod path_support;

use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Mapping, Plane, SimConfig, Window};
use engine::path::leaf::{self, Affine, CentreOnPin, Emitted, OnLatent, Scale};
use engine::path::stop::{self, Check, Gate};
use engine::path::{
    CurvedTilt, CutLabel, Family, FrameRecord, LatentTilt, PathError, Pin, PinPoint, Segment,
    TiltMap, TiltSpec, View,
};
use kernel::chart::invariant::Invariant;
use kernel::chart::nav::{nav_stop, Edit as NavEdit, View as NavView};
use kernel::chart::{Constraint, InvariantLE, Mapped};
use kernel::decode::{decode, Ic, Latent as Controls};
use kernel::generated::links::{Defaults, LinkParams};
use path_support::{planes, sim};
use validation::negative_control;
use validation::roundtrip::phys;

// ── Draws ───────────────────────────────────────────────────────────────────────────────────────────────────────────

/// SplitMix64, from a fixed seed.
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A draw in `[lo, hi)`.
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 / (1u64 << 53) as f64)
    }

    /// An index below `n`.
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const SEED: u64 = 0x6D32_2D36_3400_0001;

/// The fuzzed planes: `z₀` in `[−1, 1]⁸` with some components `−0`, a basis at a scale `2^k`, `k ∈ [−12, 2]`, with some
/// components exactly 0; those whose basis spans a plane.
fn fuzzed(n: usize) -> Vec<Plane> {
    let mut r = Mix(SEED);
    let mut out = Vec::new();
    while out.len() < n {
        let scale = r.range(-12.0, 2.0).exp2();
        let mut v = |zero_odds: usize, s: f64| -> Latent {
            std::array::from_fn(|_| match r.below(zero_odds) {
                0 => -0.0,
                1 => 0.0,
                _ => s * r.range(-1.0, 1.0),
            })
        };
        let plane = Plane {
            z0: v(5, 1.0),
            q1: v(4, scale),
            q2: v(4, scale),
        };
        if engine::nav::frame(&plane).is_ok() {
            out.push(plane);
        }
    }
    out
}

/// The stage-1 planes and the fuzzed ones.
fn every_plane() -> Vec<Plane> {
    planes()
        .into_iter()
        .map(|(_, p)| p)
        .chain(fuzzed(2000))
        .collect()
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

fn z0_of(edits: &[SetField]) -> Latent {
    match edits {
        [SetField {
            edit: Edit::Sim(SimField::Z0(z0)),
            ..
        }] => *z0,
        other => panic!("not one edit of z₀: {other:?}"),
    }
}

fn basis_of(edits: &[SetField]) -> (Latent, Latent) {
    match edits {
        [SetField {
            edit: Edit::Sim(SimField::Basis { q1, q2 }),
            ..
        }] => (*q1, *q2),
        other => panic!("not one edit of the basis: {other:?}"),
    }
}

/// The affine view of `plane`.
fn view(plane: &Plane) -> View {
    View::latent(sim(plane.clone()))
}

// ── The gui's routes as they stood (deleted from crates/gui by this task) ──────────────────────────────────────────

mod old {
    use super::*;

    /// `a + k b`, the gui's `manifold_view::add`.
    pub fn add(a: &Latent, k: f64, b: &Latent) -> Latent {
        std::array::from_fn(|i| a[i] + k * b[i])
    }

    /// `navigate::pan` after its reading of the drag: `z₀ + 2ds·q₁ + 2dt·q₂`.
    pub fn pan(plane: &Plane, ds: f64, dt: f64) -> Latent {
        add(&add(&plane.z0, 2.0 * ds, &plane.q1), 2.0 * dt, &plane.q2)
    }

    /// `navigate::pan_by_keys` and the centre rows: `z₀ + 2f·q` along one basis vector.
    pub fn pan_one(plane: &Plane, f: f64, along: &Latent) -> Latent {
        add(&plane.z0, 2.0 * f, along)
    }

    /// `navigate::zoom_by`: the basis times `2^−octaves`.
    pub fn zoom_by(plane: &Plane, octaves: f64) -> (Latent, Latent) {
        let k = engine::exact::exp2(-octaves);
        (plane.q1.map(|x| x * k), plane.q2.map(|x| x * k))
    }

    /// `navigate::zoom_to`: the basis times `σ′/σ`.
    pub fn zoom_to(plane: &Plane, from: f64, to: f64) -> (Latent, Latent) {
        let k = to / from;
        (plane.q1.map(|x| x * k), plane.q2.map(|x| x * k))
    }

    /// `manifold_view::point`, the anchor `lock_at` set and recentred on.
    pub fn point(plane: &Plane, s: f64, t: f64) -> Latent {
        add(
            &add(&plane.z0, 2.0 * s - 1.0, &plane.q1),
            2.0 * t - 1.0,
            &plane.q2,
        )
    }

    fn axis(i: usize, k: f64) -> Latent {
        let mut v = [0.0; 8];
        v[i] = k;
        v
    }

    /// The gui's `View::basis` at the angles in degrees, as `Angles::apply` turns them into radians.
    #[allow(clippy::too_many_arguments)]
    pub fn basis(
        (a, b, x, y): (usize, usize, usize, usize),
        scale: f64,
        [tau1, tau2, gamma]: [f64; 3],
    ) -> (Latent, Latent) {
        let (s1, c1) = tau1.to_radians().sin_cos();
        let (s2, c2) = tau2.to_radians().sin_cos();
        let (sg, cg) = gamma.to_radians().sin_cos();
        let u1 = add(&axis(a, c1), s1, &axis(x, 1.0));
        let u2 = add(&axis(b, c2), s2, &axis(y, 1.0));
        let q1 = add(&u1.map(|v| v * cg * scale), sg * scale, &u2);
        let q2 = add(&u2.map(|v| v * cg * scale), -sg * scale, &u1);
        (q1, q2)
    }
}

// ── path_merge_bitwise ────────────────────────────────────────────────────────────────────────────────────────────

/// What runs the pan in the merge: the leaf, or the control's reordered formula.
#[derive(Clone, Copy, PartialEq)]
enum Merge {
    Leaf,
    #[cfg(feature = "controls")]
    Reordered,
    /// The zoom row sent as octaves, `o = log₂(σ/σ′)`, the conversion the first draft took.
    #[cfg(feature = "controls")]
    ZoomRowOctaves,
}

fn pan_through(plane: &Plane, ds: f64, dt: f64, how: Merge) -> Latent {
    match how {
        #[cfg(feature = "controls")]
        Merge::ZoomRowOctaves => pan_through(plane, ds, dt, Merge::Leaf),
        Merge::Leaf => z0_of(
            leaf::pan(&view(plane), ds, dt, &stop::affine())
                .expect("a finite pan")
                .edits(),
        ),
        #[cfg(feature = "controls")]
        Merge::Reordered => old::add(
            &old::add(&plane.z0, 2.0 * dt, &plane.q2),
            2.0 * ds,
            &plane.q1,
        ),
    }
}

/// The drags of the merge: `(Δx, Δy)` over a figure `(w, h)` points, read as the gui reads them, at f32.
fn drags(r: &mut Mix) -> Vec<(f64, f64)> {
    (0..8)
        .map(|_| {
            let (dx, dy) = (r.range(-300.0, 300.0) as f32, r.range(-300.0, 300.0) as f32);
            let (w, h) = (r.range(50.0, 900.0) as f32, r.range(50.0, 900.0) as f32);
            (-f64::from(dx) / f64::from(w), f64::from(dy) / f64::from(h))
        })
        .collect()
}

/// The zoom row's leaf by `how`: the factor `σ′/σ`, or the control's octaves.
fn zoom_row_through(plane: &Plane, sigma: f64, sigma_to: f64, how: Merge) -> (Latent, Latent) {
    let scale = match how {
        #[cfg(feature = "controls")]
        Merge::ZoomRowOctaves => Scale::Octaves((sigma / sigma_to).log2()),
        _ => Scale::Factor(sigma_to / sigma),
    };
    basis_of(
        leaf::zoom(&view(plane), scale, &stop::affine())
            .expect("a zoom")
            .edits(),
    )
}

fn check_merge(how: Merge) {
    let mut r = Mix(SEED ^ 1);
    let (mut pans, mut zooms, mut rows, mut locks, mut sliders) = (0, 0, 0, 0, 0);
    for plane in every_plane() {
        for (ds, dt) in drags(&mut r) {
            let want = old::pan(&plane, ds, dt);
            let got = pan_through(&plane, ds, dt, how);
            assert_eq!(
                bits(&got),
                bits(&want),
                "a pan by ({ds}, {dt}) of {plane:?}"
            );
            pans += 1;
        }
        for octaves in [1.0 / 8.0, -1.0 / 64.0, 1.0, -1.0, 0.1, r.range(-3.0, 3.0)] {
            let (q1, q2) = old::zoom_by(&plane, octaves);
            let leaf = leaf::zoom(&view(&plane), Scale::Octaves(octaves), &stop::affine())
                .expect("a zoom");
            let got = basis_of(leaf.edits());
            assert_eq!(
                (bits(&got.0), bits(&got.1)),
                (bits(&q1), bits(&q2)),
                "a zoom by {octaves}"
            );
            zooms += 1;
        }
        // The zoom row, `q·(σ′/σ)`, to zooms in the gui's span, ZOOM_RANGE = [−4, 48]; a factor that is not normal takes
        // the gui's fallback, not a zoom.
        let sigma = crate_scale(&plane);
        for to in [r.range(-4.0, 8.0), r.range(8.0, 48.0), 0.5, 3.0] {
            let sigma_to = (-to).exp2();
            if !(sigma_to / sigma).is_normal() {
                continue;
            }
            let (w1, w2) = old::zoom_to(&plane, sigma, sigma_to);
            let (n1, n2) = zoom_row_through(&plane, sigma, sigma_to, how);
            assert_eq!(
                (bits(&n1), bits(&n2)),
                (bits(&w1), bits(&w2)),
                "a zoom row to {to}"
            );
            rows += 1;
        }
        let (s, t) = (r.range(0.0, 1.0), r.range(0.0, 1.0));
        let z = old::point(&plane, s, t);
        let leaf = leaf::lock(
            &view(&plane),
            &Affine,
            (s, t),
            CentreOnPin::On,
            &stop::affine(),
        )
        .expect("a lock");
        let want = vec![
            SetField {
                edit: Edit::Sim(SimField::Z0(z)),
                no_history: false,
            },
            SetField {
                edit: Edit::Sim(SimField::Lock(Lock {
                    locked: true,
                    z_locked: z,
                })),
                no_history: false,
            },
        ];
        assert_eq!(format!("{:?}", leaf.edits()), format!("{want:?}"));
        assert_eq!(
            leaf.edits()
                .iter()
                .map(|e| match &e.edit {
                    Edit::Sim(SimField::Z0(v)) => bits(v),
                    Edit::Sim(SimField::Lock(l)) => bits(&l.z_locked),
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>(),
            vec![bits(&z), bits(&z)],
            "the lock at ({s}, {t})"
        );
        locks += 1;
        let k = r.below(8);
        let value = plane.z0[k] + r.range(-0.1, 0.1) * 2.0;
        let mut want = plane.z0;
        want[k] = value;
        let leaf = leaf::control(&view(&plane), k, value, &stop::affine()).expect("a slider");
        assert_eq!(bits(&z0_of(leaf.edits())), bits(&want));
        sliders += 1;
    }
    println!(
        "path_merge_bitwise: pans {pans}, zooms {zooms}, zoom rows {rows}, locks {locks}, sliders {sliders}: every case \
         bit for bit"
    );
}

#[test]
fn path_merge_bitwise() {
    check_merge(Merge::Leaf);
}

negative_control!(
    path_merge_bitwise,
    "a pan with one floating-point operation reordered is not the gui's pan bit for bit",
    expected = "a pan by",
    check_merge(Merge::Reordered)
);

mod zoom_row {
    use super::*;
    negative_control!(
        path_merge_bitwise,
        "the zoom row sent as octaves, log₂(σ/σ′), is not the gui's zoom row bit for bit",
        expected = "a zoom row to",
        check_merge(Merge::ZoomRowOctaves)
    );
}

// ── path_canonical_conversion ─────────────────────────────────────────────────────────────────────────────────────

/// The decoded state at the pixel `(s, t)` of `plane`.
fn state(plane: &Plane, s: f64, t: f64) -> Ic<f64> {
    decode::<Defaults, f64>(&Controls::new(old::point(plane, s, t))).ic
}

/// The smallest distance between two bodies over the configuration's extent.
fn collision_ratio(ic: &Ic<f64>) -> f64 {
    let d = |i: usize, j: usize| (ic.r[i][0] - ic.r[j][0]).hypot(ic.r[i][1] - ic.r[j][1]);
    let extent =
        ic.r.iter()
            .map(|r| r[0].hypot(r[1]))
            .fold(f64::MIN_POSITIVE, f64::max);
    d(0, 1).min(d(0, 2)).min(d(1, 2)) / extent
}

/// The sliders' rail, `|zᵢ| ≤ 1` (a span of 2): a reported split of the physical arm, which asserts at every pixel.
const RAIL: f64 = 1.0;

/// `ε_phys` and the collision landmarks' `δ` (the module's note).
fn eps_phys() -> f64 {
    ledger::constants::EPS_PHYS.number() * f64::EPSILON
}

fn delta() -> f64 {
    f64::EPSILON / eps_phys()
}

/// What the physical arm read of one case: the largest `phys` on the rail and off it, how many pixels each, and how
/// many were skipped within `δ` of a binary collision.
#[derive(Default)]
struct Probe {
    on: f64,
    on_n: usize,
    off: f64,
    off_n: usize,
    skipped: usize,
}

/// The decode of `old` against `new` at the 3 × 3 probe, every pixel farther than `δ` from a binary collision.
fn same_system(old: &Plane, new: &Plane) -> Probe {
    let mut p = Probe::default();
    for s in path_support::PROBE {
        for t in path_support::PROBE {
            let (a, b) = (state(old, s, t), state(new, s, t));
            if collision_ratio(&a) <= delta() || collision_ratio(&b) <= delta() {
                p.skipped += 1;
                continue;
            }
            let d = phys(&a, &b);
            assert!(!d.is_nan(), "a decode that is not finite at ({s}, {t})");
            let rail = |q: &Plane| old::point(q, s, t).iter().all(|z| z.abs() <= RAIL);
            match rail(old) && rail(new) {
                true => (p.on, p.on_n) = (p.on.max(d), p.on_n + 1),
                false => (p.off, p.off_n) = (p.off.max(d), p.off_n + 1),
            }
        }
    }
    p
}

/// One conversion's tally.
#[derive(Default)]
struct Tally {
    cases: usize,
    changed: usize,
    worst_ulps: f64,
    on: f64,
    on_n: usize,
    off: f64,
    off_n: usize,
    skipped: usize,
}

impl Tally {
    fn report(&self, name: &str, bound: &str) {
        println!(
            "path_canonical_conversion: {name}: {} cases, {} changed, largest {:.3} (bound {bound}); largest phys \
             {:.3e} over every probe pixel (ε_phys {:.3e}): {:.3e} over {} on the rail, {:.3e} over {} off it; {} \
             skipped within δ = {} of a binary collision",
            self.cases,
            self.changed,
            self.worst_ulps,
            self.on.max(self.off),
            eps_phys(),
            self.on,
            self.on_n,
            self.off,
            self.off_n,
            self.skipped,
            delta()
        );
    }

    /// Asserts `new` decodes within `ε_phys·scale` of `old` at every probe pixel, and tallies it.
    fn physical(&mut self, old: &Plane, new: &Plane, scale: f64) {
        let p = same_system(old, new);
        let d = p.on.max(p.off);
        assert!(
            d <= eps_phys() * scale,
            "a converted view decodes {d:e} from the old, past ε_phys"
        );
        self.on = self.on.max(p.on);
        self.off = self.off.max(p.off);
        self.on_n += p.on_n;
        self.off_n += p.off_n;
        self.skipped += p.skipped;
    }
}

/// The bounds' scale and `ε_phys`'s: 1, or a control's sixty-fourth.
fn check_conversion(bound_scale: f64, phys_scale: f64) {
    conversion(bound_scale, phys_scale);
}

/// [`check_conversion`], returning every changed case, its view before and after: the one-axis pans' and the tilts'.
fn conversion(bound_scale: f64, phys_scale: f64) -> Vec<(Plane, Plane)> {
    let mut cases = Vec::new();
    let gate = stop::affine();
    let mut r = Mix(SEED ^ 2);
    let mut keys = Tally::default();
    let mut tilts = Tally::default();
    for plane in every_plane() {
        // A pan along one axis.
        for (f, which) in [
            (r.range(-0.2, 0.2), 0),
            (r.range(-0.2, 0.2), 1),
            (0.05, 0),
            (-0.05, 1),
        ] {
            let (along, (ds, dt)) = match which {
                0 => (&plane.q1, (f, 0.0)),
                _ => (&plane.q2, (0.0, f)),
            };
            let was = old::pan_one(&plane, f, along);
            let now = z0_of(leaf::pan(&view(&plane), ds, dt, &gate).unwrap().edits());
            let neg = |x: f64| x.to_bits() == (-0.0f64).to_bits();
            let pos = |x: f64| x.to_bits() == 0;
            for i in 0..8 {
                // The terms the leaf adds that the old route did not: `0·q₂` after the step along `q₁`, or `0·q₁` before
                // the step along `q₂`. Each is `±0`, so it moves only a `−0`, to `+0`.
                let predicted = match which {
                    0 => neg(was[i]) && pos(0.0 * plane.q2[i]),
                    _ => neg(plane.z0[i]) && pos(0.0 * plane.q1[i]) && neg(2.0 * f * plane.q2[i]),
                };
                let changed = now[i].to_bits() != was[i].to_bits();
                assert_eq!(
                    changed, predicted,
                    "a one-axis pan's component {i} of {plane:?}"
                );
                if changed {
                    assert!(
                        now[i] == 0.0 && was[i] == 0.0,
                        "a one-axis pan changed {} to {}, not a signed zero",
                        was[i],
                        now[i]
                    );
                    keys.changed += 1;
                }
            }
            if !same(&was, &now) {
                let at = |z0: Latent| Plane {
                    z0,
                    ..plane.clone()
                };
                cases.push((at(was), at(now)));
            }
            keys.cases += 1;
        }
    }
    // The tilt, on the six presets' frames, one target and two.
    let presets = [(0, 1), (2, 3), (4, 5), (6, 7), (0, 6), (5, 6)];
    for _ in 0..4000 {
        let (a, b) = presets[r.below(presets.len())];
        let hidden: Vec<usize> = (0..8).filter(|&i| i != a && i != b).collect();
        let x = hidden[r.below(6)];
        let y = match r.below(2) {
            0 => x,
            _ => hidden[r.below(6)],
        };
        let scale = r.range(-10.0, 2.0).exp2();
        let from = [
            r.range(-90.0, 90.0),
            r.range(-90.0, 90.0),
            r.range(-180.0, 180.0),
        ];
        let to = [
            (from[0] + r.range(-20.0, 20.0)).clamp(-90.0, 90.0),
            (from[1] + r.range(-20.0, 20.0)).clamp(-90.0, 90.0),
            from[2],
        ];
        let (q1, q2) = old::basis((a, b, x, y), scale, from);
        let plane = Plane {
            z0: std::array::from_fn(|_| r.range(-1.0, 1.0)),
            q1,
            q2,
        };
        let (w1, w2) = old::basis((a, b, x, y), scale, to);
        let on = OnLatent {
            from: LatentTilt {
                a: a as u32,
                b: b as u32,
                scale,
                from,
            },
            gamma: to[2],
        };
        let spec = TiltSpec {
            tau1: to[0],
            tau2: to[1],
            map: TiltMap::Lean,
            x: x as u32,
            y: y as u32,
        };
        let leaf = leaf::tilt(&view(&plane), &on, &spec, &gate).expect("a tilt");
        let (n1, n2) = basis_of(leaf.edits());
        let mut changed = false;
        for (w, n) in w1.iter().chain(&w2).zip(n1.iter().chain(&n2)) {
            let bound = 8.0 * f64::EPSILON * scale * bound_scale;
            assert!(
                (w - n).abs() <= bound,
                "a tilt to {to:?}: {w} against {n}, past 8εσ"
            );
            tilts.worst_ulps = tilts.worst_ulps.max((w - n).abs() / (f64::EPSILON * scale));
            changed |= w.to_bits() != n.to_bits();
        }
        tilts.cases += 1;
        if changed {
            tilts.changed += 1;
            let (old, new) = (
                Plane {
                    q1: w1,
                    q2: w2,
                    ..plane.clone()
                },
                Plane {
                    q1: n1,
                    q2: n2,
                    ..plane.clone()
                },
            );
            tilts.physical(&old, &new, phys_scale);
            cases.push((old, new));
        }
    }
    keys.report("a one-axis pan, in ulps", "0: a signed zero");
    tilts.report("the tilt, in ε·σ", "8");
    cases
}

/// Whether `a` and `b` are the same bits.
fn same(a: &Latent, b: &Latent) -> bool {
    bits(a) == bits(b)
}

/// The plane's scale as the gui reads it on an axis-free basis: `|q₁|`.
fn crate_scale(plane: &Plane) -> f64 {
    plane.q1.iter().map(|x| x * x).sum::<f64>().sqrt()
}

#[test]
fn path_canonical_conversion() {
    check_conversion(1.0, 1.0);
}

negative_control!(
    path_canonical_conversion,
    "every bound at a sixty-fourth of its derived count is passed by some conversion",
    expected = "past",
    check_conversion(1.0 / 64.0, 1.0)
);

mod physical {
    use super::*;
    negative_control!(
        path_canonical_conversion,
        "ε_phys at a sixty-fourth is passed by some converted view's decode",
        expected = "past ε_phys",
        check_conversion(1.0, 1.0 / 64.0)
    );
}

// ── path_canonical_conversion_outcome ─────────────────────────────────────────────────────────────────────────────
//
// The outcome-class arm, on integrated runs: every changed case of `path_canonical_conversion` (the one-axis pans and
// the tilts), at every pixel of its 3 × 3 probe, the old view's decoded state against the new one's, each
// decoded at f64, the state rounded once to f32, then marched by the default occupant, Heggie × RK4, to the default
// horizon `T = 50` (`engine::trajectory::Integrated`), the dispatch-time label first. (Not the field's record bit for
// bit: the field narrows the plane to f32 before it decodes; the march is the field's.) The class is the
// field's `(state, detail)`, the escaping body and the colliding pair included.
//
// Pass, written before the run: no pixel outside the excluded set ends in another class. The excluded set is the stated
// landmarks', as the physical arm's: a pixel whose old or new decoded configuration is within `δ` of a binary collision.
// A pixel whose two records are the same f32 bits ends in one class by the march's purity
// (`trajectory_interface_integrator_reads_order_free`) and is counted, not re-marched; every other pixel is marched
// both ways. Reported: the cases, the pixels, the excluded (and of them, those whose records differ), the same records
// and the pairs marched.
//
// Control: a conversion that moves the view, `z₀` a quarter along its first coordinate, flips a class.

/// What the outcome arm read.
#[derive(Default, Debug)]
struct Classes {
    cases: usize,
    pixels: usize,
    excluded: usize,
    /// The excluded pixels whose two records are not the same bits: excluded and actually moved.
    excluded_moved: usize,
    same_record: usize,
    marched: usize,
}

/// The decoded state and its tag at the pixel `(s, t)` of `plane`.
fn decoded(plane: &Plane, s: f64, t: f64) -> (Ic<f64>, Option<kernel::decode::Degenerate>) {
    let d = decode::<Defaults, f64>(&Controls::new(old::point(plane, s, t)));
    (d.ic, d.degenerate)
}

/// Every case of `cases`, its new view through `convert`, compared by class at the probe.
fn check_classes(cases: &[(Plane, Plane)], convert: &dyn Fn(&Plane) -> Plane) -> Classes {
    use engine::trajectory::Integrated;
    let provider = Integrated::new(50.0).expect("the default horizon");
    let mut c = Classes::default();
    for (old, new) in cases {
        let new = convert(new);
        c.cases += 1;
        for s in path_support::PROBE {
            for t in path_support::PROBE {
                c.pixels += 1;
                let ((a, ta), (b, tb)) = (decoded(old, s, t), decoded(&new, s, t));
                if collision_ratio(&a) <= delta() || collision_ratio(&b) <= delta() {
                    c.excluded += 1;
                    let moved = !engine::trajectory::same_bits(
                        &Integrated::held(&a),
                        &Integrated::held(&b),
                    );
                    c.excluded_moved += usize::from(moved || ta != tb);
                    continue;
                }
                let (ha, hb) = (Integrated::held(&a), Integrated::held(&b));
                if engine::trajectory::same_bits(&ha, &hb) && ta == tb {
                    c.same_record += 1;
                    continue;
                }
                c.marched += 1;
                let class = |h: &Ic<f64>, tag| provider.outcome(h, tag).expect("a held start");
                let (x, y) = (class(&ha, ta), class(&hb, tb));
                assert_eq!(
                    x, y,
                    "a converted view's pixel ({s}, {t}) ends in another class"
                );
            }
        }
    }
    println!("path_canonical_conversion_outcome: {c:?}");
    c
}

#[test]
fn path_canonical_conversion_outcome() {
    let c = check_classes(&conversion(1.0, 1.0), &|p| p.clone());
    assert!(
        c.cases > 0 && c.pixels > c.excluded,
        "the arm read no pixel"
    );
}

negative_control!(
    path_canonical_conversion_outcome,
    "a conversion that moves the view flips a class",
    expected = "ends in another class",
    {
        check_classes(&conversion(1.0, 1.0), &|p| {
            let mut moved = p.clone();
            moved.z0[0] += 0.25;
            moved
        });
    }
);

// ── path_pin_kept ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Checks the pinned pixel of `b` is on `held`, bit for bit where `hold_plane` holds it.
fn on_the_pin(b: &View, address: [f64; 2], held: &Latent, what: &str) {
    let (_, exact) = engine::path::hold_plane(&b.sim.plane, address, held);
    let at = engine::lock::point(&b.sim.plane, address[0], address[1]);
    for i in 0..8 {
        if exact[i] {
            assert_eq!(
                at[i].to_bits(),
                held[i].to_bits(),
                "{what}: component {i} left its held point"
            );
        }
    }
}

#[test]
fn path_pin_kept() {
    check_pin(true);
}

negative_control!(
    path_pin_kept,
    "a zoom about the centre, the pin dropped, moves the pinned pixel off its held point",
    expected = "left its held point",
    check_pin(false)
);

/// The pin kept through the leaves; `pinned` false, the control, zooms the view with its pin dropped.
fn check_pin(pinned: bool) {
    let gate = stop::affine();
    let mut r = Mix(SEED ^ 3);
    let mut checked = 0;
    for plane in every_plane().into_iter().take(400) {
        let at = (r.range(0.05, 0.95), r.range(0.05, 0.95));
        let locked = leaf::lock(&view(&plane), &Affine, at, CentreOnPin::Off, &gate).unwrap();
        let a = locked.b().clone();
        let Some(Pin::At {
            address,
            point: PinPoint::Latent(held),
        }) = a.pin
        else {
            panic!("a lock off the centre pins its pixel");
        };
        let octaves = r.range(-2.0, 2.0);
        let zoomed = match pinned {
            true => leaf::zoom(&a, Scale::Octaves(octaves), &gate).unwrap(),
            false => leaf::zoom(
                &View {
                    pin: None,
                    ..a.clone()
                },
                Scale::Octaves(octaves),
                &gate,
            )
            .unwrap(),
        };
        let a = View {
            pin: zoomed.segment().a().pin,
            ..a
        };
        assert_eq!(
            zoomed.b().bytes(),
            Segment::zoom(&a, octaves).unwrap().b().bytes()
        );
        on_the_pin(zoomed.b(), address, &held, "a zoom");
        // A tilt from the axis plane's frame: on the axis plane a = 0, b = 1, untilted.
        let axis = Plane {
            z0: plane.z0,
            q1: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            q2: [0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        let a = leaf::lock(&view(&axis), &Affine, at, CentreOnPin::Off, &gate)
            .unwrap()
            .b()
            .clone();
        let Some(Pin::At {
            point: PinPoint::Latent(held),
            ..
        }) = a.pin
        else {
            unreachable!()
        };
        for map in [TiltMap::Lean, TiltMap::Sheet] {
            for (x, y) in [(3u32, 3u32), (3, 5)] {
                let spec = TiltSpec {
                    tau1: r.range(-80.0, 80.0),
                    tau2: r.range(-80.0, 80.0),
                    map,
                    x,
                    y,
                };
                let on = OnLatent {
                    from: LatentTilt {
                        a: 0,
                        b: 1,
                        scale: 0.5,
                        from: [0.0, 0.0, 0.0],
                    },
                    gamma: 0.0,
                };
                let tilted = leaf::tilt(&a, &on, &spec, &gate).unwrap();
                assert!(
                    matches!(tilted.segment().family(), Family::R474Tilt { .. }),
                    "a tilt from the frame's own angles is the R-474 family"
                );
                let direct = Segment::latent_tilt(&a, &on.from, &spec, 0.0).unwrap();
                assert_eq!(tilted.b().bytes(), direct.b().bytes());
                let record = tilted.segment().record(0.25).unwrap();
                let read = FrameRecord::read(&record.to_bytes())
                    .expect("an R-474 tilt's record reads back");
                assert_eq!(
                    read.frame().unwrap().bytes(),
                    tilted.segment().frame(0.25).unwrap().bytes()
                );
                on_the_pin(tilted.b(), address, &held, "a tilt");
                checked += 1;
            }
        }
    }
    println!("path_pin_kept: {checked} tilts and 400 zooms about an off-centre pin, the pinned pixel held");
}

// ── nav_stop_edit_path ────────────────────────────────────────────────────────────────────────────────────────────

/// The link parameters the stop reads: the defaults.
const P: LinkParams<f64> = LinkParams::DEFAULT;

/// `(L_z, E)` under the linear option, its frozen controls `z₀`'s.
fn le_linear(z0: &Latent) -> Mapped<InvariantLE<f64>, Defaults> {
    Mapped::new(
        InvariantLE(Invariant {
            z_cfg: [z0[0], z0[1]],
            z_mass: [z0[6], z0[7]],
            k_max: 1.0,
            gamma_k: 1.5,
        }),
        [kernel::chart::Mapping::Linear; 2],
    )
}

/// The `(L_z, E)` view at the square, linear on both axes.
fn le_view() -> View {
    let mut s: SimConfig = sim(Plane {
        z0: [0.3, -0.5, 0.0, 0.0, 0.0, 0.0, 0.7, -0.4],
        q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    });
    s.chart.mapping = [Mapping::Linear; 2];
    s.chart.window = Window::SQUARE;
    View::kernel(&le_linear(&s.plane.z0), s).expect("a registry chart")
}

fn le_gate() -> impl Gate {
    stop::chart(|v: &View| Some(le_linear(&v.sim.plane.z0)), P)
}

/// How the edit is recorded: through the leaf, or a control's.
#[derive(Clone, Copy, PartialEq)]
enum Record {
    Leaf,
    #[cfg(feature = "controls")]
    Requested,
    #[cfg(feature = "controls")]
    StopAfter,
}

/// The edits a pan by `ds` records, by `how`.
fn recorded(a: &View, ds: f64, how: Record, history: &mut Vec<Vec<SetField>>) -> leaf::Leaf {
    let leaf = leaf::pan(a, ds, 0.0, &le_gate()).expect("a pan");
    match how {
        Record::Leaf => history.push(leaf.edits().to_vec()),
        #[cfg(feature = "controls")]
        Record::Requested => history.push(Segment::pan(a, ds, 0.0).unwrap().edits().to_vec()),
        #[cfg(feature = "controls")]
        Record::StopAfter => {
            history.push(Segment::pan(a, ds, 0.0).unwrap().edits().to_vec());
            history.push(leaf.edits().to_vec());
        }
    }
    leaf
}

fn window_of(edits: &[SetField]) -> Window {
    match edits {
        [SetField {
            edit: Edit::Sim(SimField::Window(w)),
            ..
        }] => *w,
        other => panic!("not one edit of the window: {other:?}"),
    }
}

fn check_edit_path(how: Record) {
    let a = le_view();
    // The pan from the centre to s = 1, past the parabola at s = ½ + ½√½.
    let mut history = Vec::new();
    let leaf = recorded(&a, 0.5, how, &mut history);
    let kernel = nav_stop(
        NavEdit::Path {
            from: [0.5, 0.5],
            to: [1.0, 0.5],
        },
        |c: [f64; 2]| {
            Some(NavView {
                chart: le_linear(&a.sim.plane.z0),
                centre: c,
            })
        },
        &P,
    );
    let check = leaf.stopped().expect("the pan crosses the parabola");
    assert_eq!(check, Check::Failed(Constraint::LzEParabola));
    assert_eq!(check.layer(), Some(2), "the stop names layer 2");
    assert_eq!(kernel.stopped, Some(check));
    let after = window_of(history.last().expect("an entry"));
    assert_eq!(
        after.centre.map(f64::to_bits),
        kernel.value.map(f64::to_bits),
        "the recorded after is not the clamped centre"
    );
    assert_eq!(history.len(), 1, "one history entry, the clamped value's");
    assert!(matches!(leaf.segment().family(), Family::Stopped { .. }));
    // Its record reads back: rebuilt from `A` by the requested family, cut short at the same `λ`, `B` bit for bit.
    for s in [0.0, 0.5, 1.0] {
        let record = leaf.segment().record(s).unwrap();
        let read =
            FrameRecord::read(&record.to_bytes()).expect("a stopped segment's record reads back");
        assert_eq!(
            read.frame().unwrap().bytes(),
            leaf.segment().frame(s).unwrap().bytes()
        );
    }
    // A jump to a view whose centre fails is refused, `B` then `A` with no edits, and its record reads back.
    let mut forbidden = a.clone();
    forbidden.sim.chart.window.centre = [1.0, 0.5];
    let refused = leaf::jump(&a, forbidden, CutLabel::ChartSwitch, &le_gate());
    assert_eq!(refused.stopped(), Some(check));
    assert_eq!(
        refused.b().bytes(),
        a.bytes(),
        "a refused jump moved the view"
    );
    assert!(refused.edits().is_empty());
    for s in [0.0, 0.5, 1.0] {
        let record = refused.segment().record(s).unwrap();
        let read =
            FrameRecord::read(&record.to_bytes()).expect("a refused jump's record reads back");
        assert_eq!(read.frame().unwrap().bytes(), a.bytes());
    }
    // The next f64 along the line fails: the stop is the edge's.
    let next = kernel::chart::nav::Field::along(0.5f64, 1.0, kernel.lambda.next_up());
    let mut beyond = a.clone();
    beyond.sim.chart.window.centre = [next, 0.5];
    assert_eq!(
        le_gate().check(&beyond),
        Some(check),
        "the value past the stop passes"
    );
    // A drag: each frame from where it began. Past the clamp, then further past, then back inside.
    let mut sent = Emitted::default();
    let frames: Vec<Option<Vec<SetField>>> = [0.5, 0.7, 0.05]
        .into_iter()
        .map(|ds| sent.send(&leaf::pan(&a, ds, 0.0, &le_gate()).unwrap()))
        .collect();
    assert_eq!(
        window_of(frames[0].as_deref().expect("the first frame sends")).centre[0].to_bits(),
        kernel.value[0].to_bits()
    );
    assert_eq!(
        frames[1], None,
        "a drag held past the clamp sends nothing new"
    );
    let inside = window_of(frames[2].as_deref().expect("back inside sends its value"));
    let requested = window_of(Segment::pan(&a, 0.05, 0.0).unwrap().edits());
    assert_eq!(
        inside.centre.map(f64::to_bits),
        requested.centre.map(f64::to_bits)
    );
    println!(
        "nav_stop_edit_path: stopped at u = {:?} (λ = {:?}, {} steps), {check:?}, layer {:?}; the drag sent 2 of 3 frames",
        kernel.value[0],
        kernel.lambda,
        kernel.steps,
        check.layer()
    );
}

#[test]
fn nav_stop_edit_path() {
    check_edit_path(Record::Leaf);
}

negative_control!(
    nav_stop_edit_path,
    "the requested value recorded is not the clamped centre",
    expected = "the recorded after is not the clamped centre",
    check_edit_path(Record::Requested)
);

mod after {
    use super::*;
    negative_control!(
        nav_stop_edit_path,
        "a stop applied after the edit was recorded leaves the requested value in the history",
        expected = "one history entry",
        check_edit_path(Record::StopAfter)
    );
}

// ── The families this task builds, against the direct calls ───────────────────────────────────────────────────────

/// How the `R474Tilt` family's interior is checked: at the lerped angles, or the control's angles scaled from 0.
#[derive(Clone, Copy, PartialEq)]
enum Interior {
    Lerped,
    #[cfg(feature = "controls")]
    FromZero,
}

/// The lean-each-axis tilt (`Family::R474Tilt`) of a latent view against `kernel::chart::tilt::latent_basis` called
/// directly: `B` and the interior frames at `s = ¼, ½, ¾`, the angles lerped, under both maps, one target and two, `z₀`
/// untouched. Criterion: bit for bit.
fn check_r474(interior: Interior) {
    use kernel::chart::tilt::{latent_basis, LatentFrame};
    let mut r = Mix(SEED ^ 4);
    let mut cases = 0;
    for _ in 0..500 {
        let presets = [(0u32, 1u32), (2, 3), (4, 5), (6, 7), (0, 6), (5, 6)];
        let (a, b) = presets[r.below(presets.len())];
        let hidden: Vec<u32> = (0..8).filter(|&i| i != a && i != b).collect();
        let x = hidden[r.below(6)];
        let y = match r.below(2) {
            0 => x,
            _ => hidden[r.below(6)],
        };
        let map = match r.below(2) {
            0 => TiltMap::Lean,
            _ => TiltMap::Sheet,
        };
        let scale = r.range(-8.0, 2.0).exp2();
        let from = [
            r.range(-80.0, 80.0),
            r.range(-80.0, 80.0),
            r.range(-90.0, 90.0),
        ];
        let to = [
            r.range(-80.0, 80.0),
            r.range(-80.0, 80.0),
            r.range(-90.0, 90.0),
        ];
        let basis = |[t1, t2, g]: [f64; 3]| {
            latent_basis(
                &LatentFrame {
                    a,
                    b,
                    scale,
                    gamma: g.to_radians(),
                },
                &TiltSpec {
                    tau1: t1,
                    tau2: t2,
                    map,
                    x,
                    y,
                },
            )
            .expect("a frame's hidden targets")
        };
        let (q1, q2) = basis(from);
        let plane = Plane {
            z0: std::array::from_fn(|_| r.range(-1.0, 1.0)),
            q1,
            q2,
        };
        let spec = TiltSpec {
            tau1: to[0],
            tau2: to[1],
            map,
            x,
            y,
        };
        let tilt = LatentTilt { a, b, scale, from };
        let seg = Segment::latent_tilt(&view(&plane), &tilt, &spec, to[2]).unwrap();
        assert!(
            matches!(seg.family(), Family::R474Tilt { .. }),
            "a tilt from its own angles is the family"
        );
        for s in [0.25, 0.5, 0.75, 1.0] {
            let at: [f64; 3] = match interior {
                Interior::Lerped => std::array::from_fn(|i| from[i] + s * (to[i] - from[i])),
                #[cfg(feature = "controls")]
                Interior::FromZero => std::array::from_fn(|i| s * to[i]),
            };
            let at = if s == 1.0 { to } else { at };
            let (w1, w2) = basis(at);
            let frame = seg.frame(s).unwrap();
            assert_eq!(
                (bits(&frame.sim.plane.q1), bits(&frame.sim.plane.q2)),
                (bits(&w1), bits(&w2)),
                "the R-474 tilt's frame at s = {s} is not latent_basis at its angles"
            );
            assert_eq!(
                bits(&frame.sim.plane.z0),
                bits(&plane.z0),
                "a tilt moved z₀"
            );
        }
        cases += 1;
    }
    println!("path_r474_tilt_bitwise: {cases} tilts, both maps, one target and two, B and three interior frames each");
}

#[test]
fn path_r474_tilt_bitwise() {
    check_r474(Interior::Lerped);
}

negative_control!(
    path_r474_tilt_bitwise,
    "the angles scaled from 0, not lerped from A's, are not the family's interior",
    expected = "is not latent_basis at its angles",
    check_r474(Interior::FromZero)
);

/// The pan on a kernel map's window against the window's arithmetic: `B` and the interior frames, `c + 2(s·ds)·h`, bit
/// for bit; the pan a translation, the chart point at `(u + ds, v + dt)` before the one at `(u, v)` after within
/// `4ε·(|c| + 2|h|)`; and a pan of a tilted curved view refused, its tilt turning about the window's centre.
fn check_pan_window(end_for_interior: bool) {
    let a = le_view();
    let w = a.sim.chart.window;
    for (ds, dt) in [(0.125, -0.0625), (-0.3, 0.2), (0.0, 0.01)] {
        let seg = Segment::pan(&a, ds, dt).unwrap();
        for s in [0.25, 0.5, 0.75, 1.0] {
            let p = match end_for_interior {
                false => s,
                true => 1.0,
            };
            let want = [
                w.centre[0] + 2.0 * (p * ds) * w.half[0],
                w.centre[1] + 2.0 * (p * dt) * w.half[1],
            ];
            let got = seg.frame(s).unwrap().sim.chart.window;
            assert_eq!(
                got.centre.map(f64::to_bits),
                want.map(f64::to_bits),
                "the window pan's frame at s = {s}"
            );
            assert_eq!(got.half.map(f64::to_bits), w.half.map(f64::to_bits));
        }
        let after = seg.b().sim.chart.window;
        for (u, v) in [(0.2, 0.3), (0.5, 0.5), (0.9, 0.1)] {
            let (x, y) = (w.at(u + ds, v + dt), after.at(u, v));
            for i in 0..2 {
                let bound = 4.0 * f64::EPSILON * (w.centre[i].abs() + 2.0 * w.half[i].abs());
                assert!(
                    (x[i] - y[i]).abs() <= bound,
                    "the window pan is not a translation"
                );
            }
        }
    }
    let mut tilted = a.clone();
    tilted.tilt = Some(CurvedTilt {
        tau1: 10.0,
        tau2: 0.0,
        lean: false,
        x: 2,
        y: 2,
        span: [2.0, 2.0],
    });
    assert!(matches!(
        Segment::pan(&tilted, 0.1, 0.0),
        Err(PathError::NotBuilt(_))
    ));
}

#[test]
fn path_pan_window_bitwise() {
    check_pan_window(false);
}

negative_control!(
    path_pan_window_bitwise,
    "the interior evaluated at the end's parameter is not the family's",
    expected = "the window pan's frame at s = 0.25",
    check_pan_window(true)
);

// ── A stopped pan keeps its pin on the clicked state ───────────────────────────────────────────────────────────────

/// The pin after a stopped pan: the leaf's, or the control's, re-read at its old address.
#[derive(Clone, Copy, PartialEq)]
enum PinAfter {
    Leaf,
    #[cfg(feature = "controls")]
    Reread,
}

/// Locked off the centre of the linear `(L_z, E)` view, a pan past the parabola is stopped at `λ`. Criterion, written
/// before the run: the pin moves with the content by `λ` of the step, its address `(s − λ·ds, t − λ·dt)` and its held
/// point the clicked state's, bit for bit, so the pinned pixel of `B` is the clicked state's chart point to rounding.
fn check_stopped_pan_pin(how: PinAfter) {
    let mut a = le_view();
    let address = [0.3, 0.6];
    let clicked = a.sim.chart.window.at(address[0], address[1]);
    a.pin = Some(Pin::At {
        address,
        point: PinPoint::Chart(clicked),
    });
    let ds = 0.5;
    let leaf = leaf::pan(&a, ds, 0.0, &le_gate()).expect("a pan");
    let Family::Stopped { lambda, .. } = *leaf.segment().family() else {
        panic!("the pan crosses the parabola and is stopped");
    };
    let pin = match how {
        PinAfter::Leaf => leaf.b().pin,
        #[cfg(feature = "controls")]
        PinAfter::Reread => Some(Pin::at(leaf.b(), address[0], address[1])),
    };
    let Some(Pin::At {
        address: [s, t],
        point: PinPoint::Chart(point),
    }) = pin
    else {
        panic!("a stopped pan keeps an off-centre pin: {pin:?}");
    };
    assert_eq!(
        point.map(f64::to_bits),
        clicked.map(f64::to_bits),
        "the stopped pan's pin left its clicked state"
    );
    assert_eq!(
        [s, t].map(f64::to_bits),
        [address[0] - lambda * ds, address[1]].map(f64::to_bits)
    );
    let at = leaf.b().sim.chart.window.at(s, t);
    for i in 0..2 {
        assert!(
            (at[i] - clicked[i]).abs() <= 4.0 * f64::EPSILON,
            "the pinned pixel {at:?} is not the clicked point {clicked:?}"
        );
    }
    println!(
        "path_pin_kept_stopped_pan: λ = {lambda:?}, the pin at ({s}, {t}) on the clicked state"
    );
}

#[test]
fn path_pin_kept_stopped_pan() {
    check_stopped_pan_pin(PinAfter::Leaf);
}

negative_control!(
    path_pin_kept_stopped_pan,
    "a stopped pan's pin re-read at its old address marks the state that slid under it",
    expected = "left its clicked state",
    check_stopped_pan_pin(PinAfter::Reread)
);
