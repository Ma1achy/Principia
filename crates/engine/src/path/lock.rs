//! The lock's edits as path families (`docs/design/principia_chart_path_sketch.md` § 1, "The lock"; chart_decoder_contract
//! Part 4, "The lock (projective microscope)" and "The lock's construction, defined"), each the formula of a
//! [`super::Segment`] constructor. **Locking does not move the view** (Part 4, "Setting the lock"):
//!
//! - **Setting the lock** on the affine chart ([`set`]): `z_locked = z₀ + (2s−1)q₁ + (2t−1)q₂`, the latent chart's own
//!   `Φ` at the clicked pixel; on a nonlinear chart ([`set_on_chart`]): `Φ` and `D` replayed on the CPU at the pixel's
//!   chart point, lookup's ladder, giving `z_locked` the controls the chart's axes set and the chart's frozen controls the
//!   rest. Either edits the lock group alone; the clicked pixel's address is the pin, held in the view
//!   ([`super::Pin`]). `z₀`, the basis and the window are not edited, so the pinned pixel decodes the clicked state bit
//!   for bit: it is the same pixel of the same chart.
//! - **Tilt and zoom turn about the pin** ([`hold_plane`], [`hold_window`]): after the basis or the window's half-widths
//!   change, the centre is re-solved so the kernel's own placement of the pinned address gives the pinned point again.
//! - **Snap-back** ([`hold_plane`] at `z_locked`) undoes the slice only: the centre re-solved so the pinned pixel decodes
//!   `z_locked`, the pin where it is on screen. **Centre on pin** ([`centre_on_pin`], [`centre_on_pin_chart`]) brings the
//!   pin to the centre; after snap-back it is `z₀ ← z_locked`.
//! - **Unlock** ([`unlock`]) clears the lock group and keeps `(z₀, q₁, q₂)`.
//! - **The re-centre** lock validation offers ([`recentre`]): the lock at the nearest feasible point, which moves the
//!   view by design (it is offered, never automatic).
//! - **A curved chart's tilt turns about the pin** ([`Pivoted`]): the pixel at window point `(u, v)` is placed at
//!   `point + R·(u − at)`, its hidden offsets `d0 + R_d·(u − at)`, so the pin, at `at`, is placed at `point + R·0`.

use kernel::chart::tilt::{Placement, Shift, Tilt, TiltSpec};
use kernel::chart::{Float, ValidationResult};
use kernel::decode::Ic;
use kernel::encode::by_kind::ByKind;
use kernel::encode::SimplexClamp;
use kernel::generated::links::Selection;

use super::family::edit;
use crate::contract::set_field::{SetField, SimField};
use crate::contract::sim_config::{Latent, Lock, Plane, SimConfig, Window, LATENT_DIM};
use crate::lock::{finite, point, FrozenControls, LockError, Nearest};
use crate::lookup::{self, Exact, Notices, Tolerances};

/// The lock group's edit: the flag and the anchor `z_locked`.
fn lock_group(z_locked: Latent) -> SetField {
    edit(SimField::Lock(Lock {
        locked: true,
        z_locked,
    }))
}

/// The lock at the affine chart's pixel `(s, t)` (Part 4, "Setting the lock"): `z_locked = z₀ + (2s−1)q₁ + (2t−1)q₂`,
/// CPU arithmetic with no readback, as the lock group's edit alone.
pub(crate) fn set(plane: &Plane, s: f64, t: f64) -> Result<SetField, LockError> {
    finite(&[s, t])?;
    let z_locked = point(plane, s, t);
    finite(&z_locked)?;
    Ok(lock_group(z_locked))
}

/// What a lock on a nonlinear chart finds beside its edit.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartLock {
    /// The window the pin is read through: the view's own, unchanged by the lock.
    pub window: Window,
    /// The pinned pixel's chart point, `window.at(s, t)`.
    pub point: [f64; 2],
    /// The clicked pixel's state, `D ∘ Φ` replayed on the CPU at [`ChartLock::point`].
    pub ic: Ic<f64>,
    /// The ladder's notices on the lock.
    pub notices: Notices,
}

/// The lock at the pixel `(s, t)` of `window` on the nonlinear `chart`, under the link selection `S` (the contract's
/// definition): `Φ` and `D` replayed on the CPU at the pixel's chart point, which must pass `validate` (R-469); the
/// state they decode through lookup's ladder, `norm` and `tol` its `‖·‖_phys` and tolerances, whose `z` gives
/// `z_locked` the controls the chart's axes set, every other control the chart's frozen one, bit for bit. The edit is the
/// lock group's alone: the window is not re-centred, so the pinned pixel is the clicked pixel of the same chart through
/// the same window, and decodes the clicked state bit for bit.
pub(crate) fn set_on_chart<S, C, N>(
    chart: &C,
    window: &Window,
    s: f64,
    t: f64,
    norm: N,
    tol: &Tolerances,
) -> Result<(SetField, ChartLock), LockError>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    C: ByKind + FrozenControls,
    f64: From<C::Param>,
    N: Fn(&Ic<f64>, &Ic<f64>) -> f64,
{
    finite(&[s, t])?;
    let [u, v] = window.at(s, t);
    finite(&[u, v])?;
    match chart.validate(u, v) {
        ValidationResult::Valid => {}
        refused => return Err(LockError::NotASystem(refused)),
    }
    let ic = chart.decode_at::<S, f64>(u, v).ic;
    let locked = lookup::lookup::<S, C, N>(chart, &Exact::Physical(ic), norm, tol)
        .map_err(LockError::Refused)?;
    let (frozen, set) = (chart.frozen(), chart.set_by_axes());
    let z_locked: Latent = std::array::from_fn(|k| match set[k] {
        true => locked.z[k],
        false => frozen[k],
    });
    Ok((
        lock_group(z_locked),
        ChartLock {
            window: *window,
            point: [u, v],
            ic,
            notices: locked.notices,
        },
    ))
}

/// Unlock: the lock group's `SetField`, flag and anchor cleared; `(z₀, q₁, q₂)` is not edited, so the view as it stands
/// is the new free state.
pub(crate) fn unlock(sim: &SimConfig) -> Result<SetField, LockError> {
    match sim.lock.locked {
        true => Ok(edit(SimField::Lock(Lock {
            locked: false,
            z_locked: [0.0; LATENT_DIM],
        }))),
        false => Err(LockError::NotLocked),
    }
}

/// The re-centre lock validation offers: the lock at the nearest feasible point, the `SetField` on `z₀` and the lock
/// group's, one command under one history key (gui_state_contract §2, R-466). It moves the view by design: it is an
/// offered action, the view brought to the nearest feasible point.
pub(crate) fn recentre(nearest: &Nearest) -> [SetField; 2] {
    [edit(SimField::Z0(nearest.z)), lock_group(nearest.z)]
}

/// Centre on pin on the affine chart: `z₀ ←` the pin's held point, by assignment, a navigation edit that brings the pin
/// to the centre. After snap-back the held point is `z_locked`, so the two are `z₀ ← z_locked`.
pub(crate) fn centre_on_pin(held: Latent) -> SetField {
    edit(SimField::Z0(held))
}

/// Centre on pin on a nonlinear chart: the window's centre moved to the pin's held chart point, its half-widths kept, and
/// `z₀` the centre's controls, as on the affine chart: those the chart's axes set are `z_locked`'s (the pin's, now at
/// the centre), the rest the chart's own frozen controls, its hidden coordinates. Right after a lock, before any
/// excursion, the frozen controls are `z_locked`'s, so `z₀ ← z_locked`.
pub(crate) fn centre_on_pin_chart(
    sim: &SimConfig,
    frozen: Latent,
    set_by_axes: [bool; LATENT_DIM],
    held: [f64; 2],
) -> [SetField; 2] {
    let z0: Latent = std::array::from_fn(|k| match set_by_axes[k] {
        true => sim.lock.z_locked[k],
        false => frozen[k],
    });
    let window = Window {
        centre: held,
        half: sim.chart.window.half,
    };
    [edit(SimField::Z0(z0)), edit(SimField::Window(window))]
}

// ── Holding the pin ────────────────────────────────────────────────────────────────────────────────────────────────

/// `x`'s place in the f64 total order (`−0` just below `+0`), as an integer.
fn ordered(x: f64) -> i64 {
    let bits = x.to_bits() as i64;
    match bits < 0 {
        true => i64::MIN - bits - 1,
        false => bits,
    }
}

/// The float at `k` in the total order of [`ordered`].
fn unordered(k: i64) -> f64 {
    f64::from_bits(match k < 0 {
        true => i64::MIN - k - 1,
        false => k,
    } as u64)
}

/// Whether a held point is exact, per component: the kernel's placement of the pinned address gives the target's bits.
pub type Held<const N: usize> = [bool; N];

/// The centre `c` for which `place(c)` gives `target` bit for bit in each component, `place` componentwise (component
/// `i` of its value reads component `i` of `c` alone) and non-decreasing in it, as the kernel's `z₀ + a·q₁ + b·q₂` and
/// `c + a·h` are (each a sum, rounded to nearest, with `c` one term). Per component, from `first`: a bracket of floats
/// whose values straddle the target, doubled until it does, then bisection for the least `c` whose value is not below
/// it; since the values are non-decreasing in `c`, the target is reached iff it is reached there or by one of its
/// neighbours (the neighbours cover `−0` and `+0`). Where it is not, no float reaches it: the centre's ulp is coarser
/// than the target's, so the sum steps over it, and the closer of the two candidates either side is kept. With which
/// components are exact. A target or a first candidate that is not finite keeps `first`.
fn solve<const N: usize>(
    first: [f64; N],
    target: &[f64; N],
    place: impl Fn(&[f64; N]) -> [f64; N],
) -> ([f64; N], Held<N>) {
    let mut c = first;
    let mut held = [false; N];
    for i in 0..N {
        if !(first[i].is_finite() && target[i].is_finite()) {
            continue;
        }
        let at = |k: i64| {
            let mut trial = c;
            trial[i] = unordered(k);
            place(&trial)[i]
        };
        let want = target[i];
        let k0 = ordered(first[i]);
        let (mut lo, mut hi, mut reach) = (k0, k0, 1i64);
        while !(at(lo) <= want && at(hi) >= want) && reach < 1 << 60 {
            (lo, hi) = (k0.saturating_sub(reach), k0.saturating_add(reach));
            (lo, hi) = (lo.max(ordered(f64::MIN)), hi.min(ordered(f64::MAX)));
            reach *= 2;
        }
        if !(at(lo) <= want && at(hi) >= want) {
            continue;
        }
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            match at(mid) < want {
                true => lo = mid,
                false => hi = mid,
            }
        }
        let near = (hi - 2..=hi + 2)
            .filter(|k| at(*k).to_bits() == want.to_bits())
            .min_by_key(|k| (k - hi).abs());
        (c[i], held[i]) = match near {
            Some(k) => (unordered(k), true),
            None => match (at(hi) - want).abs() < (want - at(lo)).abs() {
                true => (unordered(hi), false),
                false => (unordered(lo), false),
            },
        };
    }
    (c, held)
}

/// `z₀` re-solved for `plane`'s basis so that the kernel's `Φ` at the pinned address `pin` is `target` bit for bit,
/// component by component, wherever a float does ([`solve`], from the first candidate `target − Φ₀(pin)`, `Φ₀` the same
/// plane through the origin). Tilt and zoom about the pin hold the pinned point with it; snap-back holds `z_locked`.
pub fn hold_plane(plane: &Plane, [s, t]: [f64; 2], target: &Latent) -> (Latent, Held<LATENT_DIM>) {
    let through = |z0: &Latent| {
        point(
            &Plane {
                z0: *z0,
                q1: plane.q1,
                q2: plane.q2,
            },
            s,
            t,
        )
    };
    let offset = through(&[0.0; LATENT_DIM]);
    let first = std::array::from_fn(|i| target[i] - offset[i]);
    solve(first, target, through)
}

/// The window's centre re-solved for its half-widths so that `Window::at` at the pinned address `pin` is `target` bit
/// for bit, per axis, as [`hold_plane`].
pub fn hold_window(window: &Window, [s, t]: [f64; 2], target: &[f64; 2]) -> ([f64; 2], Held<2>) {
    let through = |c: &[f64; 2]| {
        Window {
            centre: *c,
            half: window.half,
        }
        .at(s, t)
    };
    let offset = through(&[0.0; 2]);
    let first = std::array::from_fn(|i| target[i] - offset[i]);
    solve(first, target, through)
}

// ── A curved chart's tilt about the pin ────────────────────────────────────────────────────────────────────────────

/// A curved chart's tilt turned about a pivot, the pin (chart_decoder_contract Part 4, "How the pin is held"): the pixel at window point `(u, v)` is placed
/// at `point + R·(u − at)` with hidden offsets `d0 + R_d·(u − at)`, the tilt's own [`Tilt::offsets`]. The pin, at `at`,
/// is placed at `point + R·0`: its own chart point and hidden offsets, bit for bit wherever adding a zero is exact (every
/// value but `−0`). At `at = point = c`, the window's centre, and `d0 = [−0, −0]` it is the kernel's [`Tilt`] about the
/// centre bit for bit (`x + (−0) = x` for every `x`). Nothing passes a non-zero `d0` yet: it is the pivot record's hidden
/// offset, which a lock placed on an already-tilted curved view and the untilt need, neither built here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pivoted {
    tilt: Tilt<f64>,
    at: [f64; 2],
    point: [f64; 2],
    d0: [f64; 2],
}

impl Pivoted {
    /// The tilt `spec` about the pivot at window point `at`, placed at chart point `point` with hidden offsets `d0`, the
    /// targets' slider spans `span`.
    pub fn new(
        spec: &TiltSpec,
        at: [f64; 2],
        point: [f64; 2],
        d0: [f64; 2],
        span: [f64; 2],
    ) -> Self {
        Pivoted {
            tilt: Tilt::new(spec, at, span),
            at,
            point,
            d0,
        }
    }
}

impl Placement<f64> for Pivoted {
    fn on(&self) -> bool {
        self.tilt.on
    }

    fn place<R: Float + From<f64>>(&self, u: R, v: R) -> ([R; 2], Shift<R>) {
        let w = <R as From<f64>>::from;
        let [da, db, dx, dy] = self.tilt.offsets(u - w(self.at[0]), v - w(self.at[1]));
        (
            [w(self.point[0]) + da, w(self.point[1]) + db],
            Shift {
                x: self.tilt.x,
                dx: dx + w(self.d0[0]),
                y: self.tilt.y,
                dy: dy + w(self.d0[1]),
                two: self.tilt.two,
            },
        )
    }

    fn targets(&self) -> ([u32; 2], bool) {
        ([self.tilt.x, self.tilt.y], self.tilt.two)
    }
}
