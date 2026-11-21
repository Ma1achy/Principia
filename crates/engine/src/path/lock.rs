//! The lock's edits as path families (`docs/design/principia_chart_path_sketch.md` § 1, "The lock in stage 1 keeps what
//! it does today"; chart_decoder_contract Part 4, "The lock (projective microscope)" and "The lock's construction,
//! defined"; TASK-M2-23): moved here from `lock` unchanged, each the formula of a [`super::Segment`] constructor.
//!
//! - **Setting the lock** on the affine chart ([`set`]): `z_locked = z₀ + (2s−1)q₁ + (2t−1)q₂`, the latent chart's own
//!   `Φ` at the clicked pixel; on a nonlinear chart ([`set_on_chart`]): `Φ` and `D` replayed on the CPU at the pixel's
//!   chart point, lookup's ladder, giving `z_locked` the controls the chart's axes set and the pixel's own frozen
//!   controls the rest, bit for bit, and the chart's window re-centred there. Today a lock re-centres the view, so each
//!   is a cut labelled `LockRecentre`: one command, one history entry (gui_state_contract §2, R-466). "The lock doesn't
//!   move the view" (the chart-space model's § 0.7) is stage 2's change.
//! - **Snap-back** ([`snap_back`]) is `z₀ ← z_locked`, by assignment, bit for bit: a cut labelled `SnapBack`.
//! - **Unlock** ([`unlock`]) clears the lock group and keeps `(z₀, q₁, q₂)`: a cut labelled `Unlock`.
//! - **The re-centre** lock validation offers ([`recentre`]): the lock at the nearest feasible point, a `LockRecentre`
//!   cut.

use kernel::chart::ValidationResult;
use kernel::decode::Ic;
use kernel::encode::by_kind::ByKind;
use kernel::encode::SimplexClamp;
use kernel::generated::links::Selection;

use super::family::edit;
use crate::contract::set_field::{SetField, SimField};
use crate::contract::sim_config::{Latent, Lock, Plane, SimConfig, Window, LATENT_DIM};
use crate::lock::{finite, point, FrozenControls, LockError, Nearest};
use crate::lookup::{self, Exact, Notices, Tolerances};

/// The lock's edits at `z_locked`: the recentring, a `SetField` on `z₀` (§G4), then the lock group, flag and anchor.
/// The pair is one command: the binding sends it in one frame under one history key, so the history keeps one entry
/// and one undo restores both (gui_state_contract §2, "A command of several edits is one history entry"; R-466).
fn lock_edits(z_locked: Latent) -> [SetField; 2] {
    [
        edit(SimField::Z0(z_locked)),
        edit(SimField::Lock(Lock {
            locked: true,
            z_locked,
        })),
    ]
}

/// The lock at the affine chart's pixel `(s, t)` (Part 4, "Setting the lock"): `z_locked = z₀ + (2s−1)q₁ + (2t−1)q₂`,
/// CPU arithmetic with no readback, as the recentring `SetField` on `z₀` and the lock group's.
pub(crate) fn set(plane: &Plane, s: f64, t: f64) -> Result<[SetField; 2], LockError> {
    finite(&[s, t])?;
    let z_locked = point(plane, s, t);
    finite(&z_locked)?;
    Ok(lock_edits(z_locked))
}

/// What a lock on a nonlinear chart finds beside its edits.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartLock {
    /// The window re-centred on the clicked chart point, its half-widths kept: the value of the command's window edit.
    pub window: Window,
    /// The clicked pixel's state, `D ∘ Φ` replayed on the CPU: the centre pixel's under the re-centred window.
    pub ic: Ic<f64>,
    /// The ladder's notices on the lock.
    pub notices: Notices,
}

/// The lock at the pixel `(s, t)` of `window` on the nonlinear `chart`, under the link selection `S` (the contract's
/// definition): `Φ` and `D` replayed on the CPU at the pixel's chart point, which must pass `validate` (R-469); the
/// state they decode through lookup's ladder, `norm` and `tol` its `‖·‖_phys` and tolerances, whose `z` gives
/// `z_locked` the controls the chart's axes set, every other control the chart's frozen one, bit for bit, so the chart
/// rebuilt on `z₀ = z_locked` decodes the clicked state at the window re-centred on the chart point, bit for bit.
/// `z_locked`'s own decode is within `tol`'s `ε_phys` of that state where the ladder clamps none of the controls the
/// axes set; where it clamps one, `z_locked` holds the clamp's image and [`ChartLock::notices`] carry `lookup_clamped`.
/// The edits, `z₀`, the lock group and the window, are one command under one history key (gui_state_contract §2,
/// R-466; R-69; REQ-SYS-011).
pub(crate) fn set_on_chart<S, C, N>(
    chart: &C,
    window: &Window,
    s: f64,
    t: f64,
    norm: N,
    tol: &Tolerances,
) -> Result<([SetField; 3], ChartLock), LockError>
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
    let recentred = Window {
        centre: [u, v],
        half: window.half,
    };
    let [z0, lock] = lock_edits(z_locked);
    Ok((
        [z0, lock, edit(SimField::Window(recentred))],
        ChartLock {
            window: recentred,
            ic,
            notices: locked.notices,
        },
    ))
}

/// Snap-back: `δ → 0` exactly, the `SetField` on `z₀` setting it to the stored anchor, bit for bit.
pub(crate) fn snap_back(sim: &SimConfig) -> Result<SetField, LockError> {
    match sim.lock.locked {
        true => Ok(edit(SimField::Z0(sim.lock.z_locked))),
        false => Err(LockError::NotLocked),
    }
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
/// group's, one command under one history key (gui_state_contract §2, R-466).
pub(crate) fn recentre(nearest: &Nearest) -> [SetField; 2] {
    lock_edits(nearest.z)
}
