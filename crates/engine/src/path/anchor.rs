//! Paths under a lock (TASK-M2-65; design `principia_chart_composition.md` § 17.8; the chart-space model's § 5,
//! Theorems 7 and 8): the stored anchor, the re-solve that re-centres each end on it, the sliding anchor where an end
//! can't hold it, and the offset form a locked blend's pixels take.
//!
//! - **The anchor** ([`Anchor`]) is stored as the signed starting state, its fifteen `(m, r, p)` words, not as a latent
//!   point: the state the pinned pixel showed when it was recorded ([`record`]), its mirror half kept, since the decode
//!   is signed. A latent `z_locked` decodes to it only on the latent map; on a nonlinear map within `ε_phys` at best.
//!   The pin's screen position is read back from the view ([`pin_address`]); an old lock with no pin, on a view where
//!   no address can be derived, takes the centre, labelled `assumed`.
//! - **Re-centring an end** ([`recentre`]): an end holds the anchor where its own placement can put the anchor at the
//!   pin. A latent end's centre is re-solved so the pin is the anchor's latent point (`hold_plane`); a kernel map's
//!   window centre is the solver core's nearest point ([`nearest`]), and the end holds the anchor where that point's
//!   state is within the lock's tolerance of it in `‖·‖_phys`.
//! - **The slide** ([`Track`]): where the end can't hold it, the frame's pin shows `block_line(a, B_x(s), s)`, `B_x` the
//!   end placed at the chart point `x`, `x(s)` the followed local minimiser of that state's block distance to the anchor
//!   (`lock_solve::follow`), from the nearest point at the first frame. It is labelled `not nearest` at a frame where
//!   the restart rule finds a point nearer by more than the lock's tolerance, and it stops at the last frame where the
//!   followed minimiser exists (`Failure::Ended`), labelled "the followed system ends here".
//! - **The offset form** ([`OffsetForm`]): a pixel's offset from its end's pin state is taken in the tangent space at
//!   that state (the log map on `n̂`, additively on `μ` and the momenta), split into the first-order part `J(c − pin)`
//!   and a remainder; the first-order parts are carried along the geodesic between the two ends' tangent frames
//!   (§ 17.7's principal angles, `atan2`), their areas geometrically interpolated, and only the remainders blend
//!   linearly. So the frame's tangent at the pin keeps its area where a linear blend of offsets squashes it.
//! - **The select**: under a lock whose ends both hold the anchor, the pin's pixel takes the stored words by a
//!   comparison of the address, so it is the anchor bit for bit on every frame (§ 17.8: offset alone is not bitwise).

use kernel::chart::lock_solve::{self, Continuity, Damping, Failure, Report, Spec, Status, Target};
use kernel::decode::descriptor;
use kernel::decode::{Decoded, Ic};

use super::blend::{block_line, line_offset, Blocks, Leaves, Pixel};
use super::lock::hold_plane;
use super::{hold_window, Map, PathError, Pin, PinPoint, View, ViewWindow};

/// The lock's tolerance in `‖·‖_phys`: `lock_tolerance·ε_R` at f64 (REQ-CHART-158, a calibration).
pub const LOCK_TOLERANCE: f64 = ledger::constants::LOCK_TOLERANCE.number() * f64::EPSILON;

/// The re-solve's own residual target, `‖r‖` in block coordinates: `lock_solve_tolerance·ε_R` at f64, below the lock's
/// tolerance, so a solve stopping at its target is not already at the tolerance its hold is judged by (from the solves'
/// measured floor: the sweep's criterion C1, `path_lock.rs`, on its two re-solving hold pairs only).
pub const SOLVE_TOLERANCE: f64 = ledger::constants::LOCK_SOLVE_TOLERANCE.number() * f64::EPSILON;

// The re-solve's target is below the lock's tolerance, at compile time; `path_lock_solve_below_lock` checks the same
// with [`solve_below_lock`] and its control.
const _: () = assert!(
    solve_below_lock(SOLVE_TOLERANCE, LOCK_TOLERANCE),
    "the re-solve's target must be below the lock's tolerance"
);

/// Whether a re-solve's residual target `solve` is below the lock's tolerance `lock`, so a hold is never judged at the
/// point its solve stopped: [`SOLVE_TOLERANCE`] and [`LOCK_TOLERANCE`] are checked against it at compile time.
pub const fn solve_below_lock(solve: f64, lock: f64) -> bool {
    solve < lock
}

/// A control's and a measurement's reach into the re-solve, on this thread only: the solver's residual target replaced,
/// the re-hold's hold decided by the block residual as it was before it was decided in `‖·‖_phys`, and each solve the
/// re-solve runs logged with its report.
#[cfg(feature = "controls")]
pub mod probe {
    use core::cell::{Cell, RefCell};
    use kernel::chart::lock_solve::Report;

    /// One solve the re-solve ran: which (`"nearest"`, `"rehold"`, `"rehold slide"`, `"slide first"`, `"follow"`), its
    /// frame's `s` (0 where it has none) and its report.
    #[derive(Clone, Copy, Debug)]
    pub struct Solve {
        /// Which solve.
        pub what: &'static str,
        /// Its frame's `s`.
        pub s: f64,
        /// Its report.
        pub report: Report<f64>,
    }

    /// The switches, set for one call of [`with`].
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Switches {
        /// The solver's residual target in place of [`super::SOLVE_TOLERANCE`].
        pub target: Option<f64>,
        /// The re-hold's hold decided as it was before it was decided in the norm: a kernel map's by the block residual
        /// against the lock's tolerance, a latent end's held unchecked.
        pub block_hold: bool,
        /// The solver's first-order optimality tolerance in place of [`super::spec`]'s `10⁻⁸`: a diagnostic's one
        /// variable changed.
        pub optimality: Option<f64>,
        /// Each nearest-point and re-hold solve also run again from the same seeds with a residual target of 0, its
        /// report logged as `"nearest floor"` or `"rehold floor"`: where the solve stops when nothing stops it early,
        /// its floor. The path itself goes on with the solve at its own target.
        pub floor: bool,
        /// The slide's residual formed as main formed it, `block_line(a, B_x, s) − a` by subtracting the anchor back
        /// off the frame's state (its target the anchor), in place of [`crate::path::blend::line_offset`] (target 0).
        pub cancelling: bool,
        /// The damping's ceiling in place of [`super::spec`]'s `10¹²`: a diagnostic's one variable changed.
        pub ceiling: Option<f64>,
        /// Each follow's direct solve seeded at the restart rule's point at its own frame, not at the last frame's
        /// point: a diagnostic's one variable changed.
        pub restart_seed: bool,
        /// A follow whose direct solve fails read as the followed minimiser's end, as main read it, in place of
        /// [`crate::path::PathError::FollowFailed`].
        pub failed_is_end: bool,
        /// The slide's slopes by the three-point central difference, as main took them, in place of the five-point one
        /// (both at `ε^{1/3}` of the window's half-width).
        pub three_point: bool,
        /// The solver's polishing step accepted where `‖P r‖²` falls, in place of `‖Jᵀr‖²`: a diagnostic's one variable
        /// changed (`lock_solve::Spec::polish_projected`).
        pub polish_projected: bool,
        /// `recentre`'s hold decided on the nearest point's own decode (`placed_at`'s window), as main decided it, in
        /// place of the decode of the view it returns (`place`'s).
        pub placed_at_hold: bool,
    }

    thread_local! {
        static SWITCHES: Cell<Switches> = Cell::new(Switches::default());
        static LOG: RefCell<Option<Vec<Solve>>> = const { RefCell::new(None) };
    }

    /// `f` run with `switches` set and every solve logged; the switches and log in force before restored after, `f`
    /// panicking or not, so a call nests inside another.
    pub fn with<T>(switches: Switches, f: impl FnOnce() -> T) -> (T, Vec<Solve>) {
        struct Restore(Switches, Option<Vec<Solve>>);
        impl Drop for Restore {
            fn drop(&mut self) {
                SWITCHES.with(|c| c.set(self.0));
                LOG.with(|l| *l.borrow_mut() = self.1.take());
            }
        }
        let restore = Restore(
            SWITCHES.with(|c| c.replace(switches)),
            LOG.with(|l| l.borrow_mut().replace(Vec::new())),
        );
        let out = f();
        let log = LOG.with(|l| l.borrow_mut().take()).unwrap_or_default();
        drop(restore);
        (out, log)
    }

    pub(super) fn switches() -> Switches {
        SWITCHES.with(Cell::get)
    }

    pub(super) fn log(what: &'static str, s: f64, report: &Report<f64>) {
        LOG.with(|l| {
            if let Some(v) = l.borrow_mut().as_mut() {
                v.push(Solve {
                    what,
                    s,
                    report: *report,
                });
            }
        });
    }
}

/// A solve logged for [`probe`]; nothing without the `controls` feature.
#[inline]
fn logged(what: &'static str, s: f64, report: &Report<f64>) {
    #[cfg(feature = "controls")]
    probe::log(what, s, report);
    let _ = (what, s, report);
}

/// The lock's anchor: the signed starting state, stored as its words, and whether the pin it was recorded at
/// was assumed at the centre because no address could be derived.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    /// The words of `(m, r, p)`: `m₀ m₁ m₂`, `r₀ r₁ r₂` (x then y), `p₀ p₁ p₂` (x then y), as IEEE bits.
    pub words: [u64; 15],
    /// The pin's address was assumed at the centre.
    pub assumed: bool,
    /// The view this record rides on can't hold the state: its pin shows its nearest point to it, labelled, and the
    /// record is kept unchanged for the way back (a chart switch to a target that can't hold the pinned state).
    pub slid: bool,
}

impl Anchor {
    /// The anchor of the state `ic`.
    pub fn of(ic: &Ic<f64>, assumed: bool) -> Self {
        let mut w = [0u64; 15];
        for i in 0..3 {
            w[i] = ic.m[i].to_bits();
            w[3 + 2 * i] = ic.r[i][0].to_bits();
            w[4 + 2 * i] = ic.r[i][1].to_bits();
            w[9 + 2 * i] = ic.p[i][0].to_bits();
            w[10 + 2 * i] = ic.p[i][1].to_bits();
        }
        Anchor {
            words: w,
            assumed,
            slid: false,
        }
    }

    /// The stored state.
    pub fn ic(&self) -> Ic<f64> {
        let f = |k: usize| f64::from_bits(self.words[k]);
        Ic {
            m: [f(0), f(1), f(2)],
            r: [[f(3), f(4)], [f(5), f(6)], [f(7), f(8)]],
            p: [[f(9), f(10)], [f(11), f(12)], [f(13), f(14)]],
        }
    }

    /// The stored state's blocks.
    pub fn blocks(&self) -> Blocks {
        Blocks::of(&self.ic())
    }

    /// Whether two anchors hold the same state, bit for bit (the `assumed` label aside).
    pub fn same_state(&self, other: &Anchor) -> bool {
        self.words == other.words
    }
}

// ── The pin's screen position ──────────────────────────────────────────────────────────────────────────────────────

/// The pin's address in the window's `[0,1]²` and whether it was assumed: the view's stored pin where it has one (a
/// click's, never assumed); for an old lock with no pin on the latent map, the address whose `Φ` is `z_locked`, solved
/// on the plane, the centre where `z_locked` is off the plane (an excursion: the pin is the current `z₀`, § 17.8); and
/// the centre, `assumed`, where none can be derived: a degenerate plane, whose `q₁` and `q₂` span no plane, or a kernel
/// map, whose chart is not in the view. `None` while free.
pub fn pin_address(view: &View) -> Option<([f64; 2], bool)> {
    if let Some(pin) = view.pin {
        return Some((pin.address(), false));
    }
    if !view.sim.lock.locked {
        return None;
    }
    let ViewWindow::Plane(plane) = view.window() else {
        return Some(([0.5, 0.5], true));
    };
    let dot = |a: &[f64; 8], b: &[f64; 8]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let d: [f64; 8] = std::array::from_fn(|i| view.sim.lock.z_locked[i] - plane.z0[i]);
    let (g11, g12, g22) = (
        dot(&plane.q1, &plane.q1),
        dot(&plane.q1, &plane.q2),
        dot(&plane.q2, &plane.q2),
    );
    let det = g11 * g22 - g12 * g12;
    // A plane whose Gram determinant is within rounding of 0 relative to its sides spans no plane.
    if det.partial_cmp(&(64.0 * f64::EPSILON * g11 * g22)) != Some(core::cmp::Ordering::Greater) {
        return Some(([0.5, 0.5], true));
    }
    let (b1, b2) = (dot(&plane.q1, &d), dot(&plane.q2, &d));
    let a = (g22 * b1 - g12 * b2) / det;
    let b = (g11 * b2 - g12 * b1) / det;
    let off: f64 = (0..8)
        .map(|i| d[i] - a * plane.q1[i] - b * plane.q2[i])
        .map(|x| x * x)
        .sum::<f64>()
        .sqrt();
    let size = dot(&view.sim.lock.z_locked, &view.sim.lock.z_locked).sqrt()
        + dot(&plane.z0, &plane.z0).sqrt()
        + (g11.sqrt() + g22.sqrt());
    match off <= 64.0 * f64::EPSILON * size {
        true => Some(([(a + 1.0) / 2.0, (b + 1.0) / 2.0], false)),
        false => Some(([0.5, 0.5], false)),
    }
}

/// The state `view` shows at `address`, through `leaves` (a blend view through its own decode).
///
/// A locked view with a recorded anchor shows the stored words at its pin (the select): the same view as `B` of a path
/// that ends there and as a view on its own, so the end rule and the anchor rule agree at the pin (§ 17.8: "a locked B,
/// as a view, carries the words too").
pub fn state_at<L: Leaves>(view: &View, leaves: &L, address: [f64; 2]) -> Result<Pixel, PathError> {
    let pixel = match &view.map {
        Map::Blend(blend) => return blend.decode(leaves, address),
        _ => Pixel::State {
            decoded: Box::new(leaves.decode(view, address)?),
            flags: Default::default(),
        },
    };
    let at_pin = view.sim.lock.locked
        && pin_address(view)
            .is_some_and(|(pin, _)| pin.map(f64::to_bits) == address.map(f64::to_bits));
    match (view.anchor.filter(|a| at_pin && !a.slid), pixel) {
        (Some(anchor), Pixel::State { decoded, flags }) => Ok(Pixel::State {
            decoded: Box::new(selected(&anchor, &decoded)),
            flags,
        }),
        (_, pixel) => Ok(pixel),
    }
}

/// A pixel's state; [`PathError::Refused`] on a blank pixel.
pub(crate) fn decoded(pixel: Pixel) -> Result<Decoded<f64>, PathError> {
    match pixel {
        Pixel::State { decoded, .. } => Ok(*decoded),
        Pixel::Blank(_) => Err(PathError::Refused("the pin shows no state")),
    }
}

/// The locked `view` with its lock's record: the pin at its address ([`pin_address`]), kept where it has one, and the
/// anchor, the state the view shows there, by value. Recording again after a slice or a slider's move records the
/// state the pin now shows: with an excursion the words are the pin's current state (§ 17.8). [`LockError::NotLocked`]
/// while free.
///
/// [`LockError::NotLocked`]: crate::lock::LockError::NotLocked
pub fn record<L: Leaves>(view: &View, leaves: &L) -> Result<View, PathError> {
    let (address, assumed) =
        pin_address(view).ok_or(PathError::Lock(crate::lock::LockError::NotLocked))?;
    let state = decoded(state_at(view, leaves, address)?)?;
    let pin = match view.pin {
        Some(pin) => pin,
        None if assumed => Pin::Centre,
        None => Pin::at(view, address[0], address[1]),
    };
    Ok(View {
        pin: Some(pin),
        anchor: Some(Anchor::of(&state.ic, assumed)),
        ..view.clone()
    })
}

// ── The re-solve: a kernel map's nearest point ─────────────────────────────────────────────────────────────────────

/// A kernel map's end placed so its pin shows the chart point `x`: the window's centre moved by `x − window.at(pin)`.
///
/// A pin held at a chart point holds it there too, since a tilted view turns about the pin's held point
/// (`View::pivoted`), so its pin shows that point whatever the window.
fn placed_at(view: &View, pin: [f64; 2], x: [f64; 2]) -> View {
    let mut out = view.clone();
    let w = &view.sim.chart.window;
    let at = w.at(pin[0], pin[1]);
    out.sim.chart.window.centre = [w.centre[0] + (x[0] - at[0]), w.centre[1] + (x[1] - at[1])];
    hold_pin_at(&mut out, x);
    // The re-solve reads the chart's own state at the pin, never a lock's stored words through the select.
    out.anchor = None;
    out
}

/// The pin of `view`, where it is held at a chart point, held at `x` instead.
fn hold_pin_at(view: &mut View, x: [f64; 2]) {
    if let Some(Pin::At {
        address,
        point: PinPoint::Chart(_),
    }) = view.pin
    {
        view.pin = Some(Pin::At {
            address,
            point: PinPoint::Chart(x),
        });
    }
}

/// The blocks of the state a kernel map's end shows at its pin, placed at the chart point `x`: the map the solver core
/// reads ([`lock_solve::Map`]).
struct Placed<'a, L> {
    view: &'a View,
    leaves: &'a L,
    pin: [f64; 2],
    step: f64,
}

impl<L: Leaves> Placed<'_, L> {
    fn blocks(&self, x: [f64; 2]) -> Option<Blocks> {
        let view = placed_at(self.view, self.pin, x);
        let state = decoded(state_at(&view, self.leaves, self.pin).ok()?).ok()?;
        let blocks = Blocks::of(&state.ic);
        blocks
            .vector()
            .iter()
            .all(|v| v.is_finite())
            .then_some(blocks)
    }
}

impl<L: Leaves> lock_solve::Map<f64, 9> for Placed<'_, L> {
    fn value(&self, x: [f64; 2]) -> Option<[f64; 9]> {
        self.blocks(x).map(|b| b.vector())
    }

    fn jacobian(&self, x: [f64; 2]) -> Option<[[f64; 9]; 2]> {
        lock_solve::central_difference(|p| self.value(p), x, self.step)
    }
}

/// The frame family of a slide: the state the frame at `s` shows at its pin, `block_line(a, B_x, s)`, `B_x` the end
/// placed at `x`.
struct Slide<'a, L> {
    placed: Placed<'a, L>,
    anchor: Blocks,
}

impl<L> Slide<'_, L> {
    /// The target the family's value is solved toward: 0, its value being the frame's offset from the anchor.
    fn target(&self) -> Target<f64, 9> {
        #[cfg(feature = "controls")]
        if probe::switches().cancelling {
            return Target {
                value: self.anchor.vector(),
                weight: [1.0; 9],
            };
        }
        Target {
            value: [0.0; 9],
            weight: [1.0; 9],
        }
    }
}

impl<L: Leaves> lock_solve::Family<f64, 9> for Slide<'_, L> {
    /// The frame's pin state's offset from the anchor, `block_line(a, B_x, s) − a`, formed directly
    /// ([`line_offset`]): near `s = 0` that offset is of size `s`, and the frame's state less the anchor would carry
    /// the anchor's own rounding against it (measured, `path_lock_slide_stall`: the follow stalled at the slide's
    /// first frames).
    fn value(&self, s: f64, x: [f64; 2]) -> Option<[f64; 9]> {
        let b = self.placed.blocks(x)?;
        #[cfg(feature = "controls")]
        if probe::switches().cancelling {
            return Some(block_line(&self.anchor, &b, s).0.vector());
        }
        Some(line_offset(&self.anchor, &b, s))
    }

    /// Five-point central differences ([`five_point`]) at the three-point difference's step, `ε^{1/3}` of the window's
    /// half-width. Measured where the slides stalled (`path_lock_slide_stall`'s D0 and D4): the three-point difference's
    /// truncation there is about `1.5·10⁻⁷` of the slopes on Burrau and `4·10⁻⁸` on the rest chart (it falls 4× as the
    /// step halves), not the `ε^{2/3}` rounding it was taken to be, and the follow's first-order optimality, `10⁻⁸`,
    /// can't be reached through it; the five-point difference at the same step agrees with itself to about `10⁻¹¹`, and
    /// its stencil, `±2` steps, stays as narrow (a wider step, `ε^{1/5}`, crossed the rest chart's curvature and failed).
    fn jacobian(&self, s: f64, x: [f64; 2]) -> Option<[[f64; 9]; 2]> {
        #[cfg(feature = "controls")]
        if probe::switches().three_point {
            return lock_solve::central_difference(|p| self.value(s, p), x, self.placed.step);
        }
        five_point(|p| self.value(s, p), x, self.placed.step)
    }
}

/// The five-point central difference of `value` at `x`, step `h` per coordinate: `(f(x−2h) − 8f(x−h) + 8f(x+h) −
/// f(x+2h))/12h`, its truncation `O(h⁴)` against the three-point difference's `O(h²)`, its rounding `O(ε/h)` as theirs.
fn five_point(
    value: impl Fn([f64; 2]) -> Option<[f64; 9]>,
    x: [f64; 2],
    h: f64,
) -> Option<[[f64; 9]; 2]> {
    let mut out = [[0.0; 9]; 2];
    for (axis, column) in out.iter_mut().enumerate() {
        let at = |k: f64| {
            let mut p = x;
            p[axis] = x[axis] + k * h;
            value(p)
        };
        let (m2, m1, p1, p2) = (at(-2.0)?, at(-1.0)?, at(1.0)?, at(2.0)?);
        for i in 0..9 {
            column[i] = ((m2[i] - p2[i]) + 8.0 * (p1[i] - m1[i])) / (12.0 * h);
        }
    }
    Some(out)
}

/// The solver core's settings for the lock's re-solve. The plan is unclear here; chose the solver core's own test
/// settings (`lock_solver_core`), because the re-solve is that solve on the blocks, its residual target
/// [`SOLVE_TOLERANCE`], below the lock's tolerance so a hold is never judged at the point the solve stopped, and
/// first-order optimality to `10⁻⁸`, not `10⁻¹⁰`, because the slopes here are central differences: measured on the
/// Burrau chart, at `10⁻¹⁰` the follow failed to converge at its second frame and the slide was tagged ended there. (The
/// three-point difference was taken then to be good to `ε^{2/3}`; measured, its truncation reaches `10⁻⁷` of the slopes
/// near the slide's minimisers, which is why the slide's own slopes are now five-point, [`five_point`].)
fn spec() -> Spec<f64> {
    #[cfg(feature = "controls")]
    let target = probe::switches().target.unwrap_or(SOLVE_TOLERANCE);
    #[cfg(not(feature = "controls"))]
    let target = SOLVE_TOLERANCE;
    spec_at(target)
}

/// [`spec`] with the residual target `target`.
fn spec_at(target: f64) -> Spec<f64> {
    #[cfg(feature = "controls")]
    let optimality = probe::switches().optimality.unwrap_or(1e-8);
    #[cfg(not(feature = "controls"))]
    let optimality = 1e-8;
    #[cfg(feature = "controls")]
    let ceiling = probe::switches().ceiling.unwrap_or(1e12);
    #[cfg(not(feature = "controls"))]
    let ceiling = 1e12;
    #[allow(unused_mut)]
    let mut spec = Spec::new(
        200,
        target,
        optimality,
        1e-12,
        Damping {
            initial: 1e-3,
            increase: 10.0,
            decrease: 0.1,
            ceiling,
        },
    );
    #[cfg(feature = "controls")]
    {
        spec.polish_projected = probe::switches().polish_projected;
    }
    spec
}

/// One frame of the slide on `b` toward `anchor` at `s`, solved directly from `seed` with the re-solve's settings (and
/// the probe's switches): a measurement's reach into the follow's own solve.
#[cfg(feature = "controls")]
pub fn slide_frame<L: Leaves>(
    b: &View,
    leaves: &L,
    pin: [f64; 2],
    anchor: &Anchor,
    s: f64,
    seed: [f64; 2],
) -> Report<f64> {
    let family = Slide {
        placed: Placed {
            view: b,
            leaves,
            pin,
            step: step(b),
        },
        anchor: anchor.blocks(),
    };
    lock_solve::solve(
        &lock_solve::Frame { family: &family, s },
        &family.target(),
        seed,
        &spec(),
    )
}

/// The continuity test the followed minimiser is held to, in chart coordinates scaled by the window's half-width `h`:
/// G8's ratio 0.6, agreement within `10⁻⁶·h`, the solver core's tested depth, and a floor at the solve's own precision.
/// The plan is unclear here; chose the floor `10⁻⁷·h`, not the solver core's test's `10⁻⁹`, because a solve that stops
/// at first-order optimality `10⁻⁸` on central-difference slopes ([`spec`]) places a minimiser that does not move only
/// to about that, and a re-solve of a still minimiser moves it by that much whatever the frame's length. Measured on
/// the constructed cut-locus chart, whose followed well does not move with `s`: at `10⁻⁹·h` the test read the still
/// minimiser as ended at `s = 3/64`; at `10⁻⁷·h` it follows it to `s = 1`.
fn continuity(h: f64) -> Continuity<f64> {
    Continuity {
        ratio: 0.6,
        floor: 1e-7 * h,
        agree: 1e-6 * h,
        depth: 32,
    }
}

/// The restart rule's seeds: the window's own point at the pin, then its centre, its four edge midpoints and its four
/// corners, so the search covers the window the user sees.
fn seeds(view: &View, pin: [f64; 2]) -> [[f64; 2]; 10] {
    let w = &view.sim.chart.window;
    let mut out = [w.at(pin[0], pin[1]); 10];
    let mut k = 1;
    for j in [0.5, 0.0, 1.0] {
        for i in [0.5, 0.0, 1.0] {
            out[k] = w.at(i, j);
            k += 1;
        }
    }
    out
}

/// What the re-solve found on a kernel map's end: the chart point at the pin, its state and the solver's report.
#[derive(Clone, Debug, PartialEq)]
pub struct Nearest {
    /// The chart point the pin is placed at.
    pub x: [f64; 2],
    /// Its state.
    pub ic: Ic<f64>,
    /// The solver core's report (the restart rule's).
    pub report: Report<f64>,
}

/// The step of a central difference in chart coordinates: `ε^{1/3}` of the window's half-width.
fn step(view: &View) -> f64 {
    let h = view.sim.chart.window.half;
    crate::exact::cbrt(f64::EPSILON) * h[0].abs().max(h[1].abs())
}

/// The nearest point in the kernel map's end `view` to `anchor` (§ 17.8's definition): the chart point placed at the pin
/// whose state's blocks are nearest the anchor's, `B`'s hidden coordinates as its view holds them, by the solver core's
/// restart rule from [`seeds`]. [`PathError::WrongMap`] on a latent or blend end, whose placement is not a chart point.
pub fn nearest<L: Leaves>(
    view: &View,
    leaves: &L,
    pin: [f64; 2],
    anchor: &Anchor,
) -> Result<Nearest, PathError> {
    if !matches!(view.window(), ViewWindow::Chart(_)) || matches!(view.map, Map::Blend(_)) {
        return Err(PathError::WrongMap);
    }
    let placed = Placed {
        view,
        leaves,
        pin,
        step: step(view),
    };
    let target = Target {
        value: anchor.blocks().vector(),
        weight: [1.0; 9],
    };
    let report = lock_solve::restart(&placed, &target, &seeds(view, pin), &spec());
    logged("nearest", 0.0, &report);
    #[cfg(feature = "controls")]
    if probe::switches().floor {
        let floor = lock_solve::restart(&placed, &target, &seeds(view, pin), &spec_at(0.0));
        logged("nearest floor", 0.0, &floor);
    }
    if !report.converged() {
        return Err(PathError::Refused("the re-solve found no nearest point"));
    }
    let state = decoded(state_at(&placed_at(view, pin, report.x), leaves, pin)?)?;
    Ok(Nearest {
        x: report.x,
        ic: state.ic,
        report,
    })
}

/// How a locked blend's end met the anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LockedEnd {
    /// `B` holds the anchor: every frame shows the stored words at the pin.
    Holds,
    /// The anchor slides to `B`'s followed minimiser, labelled "not nearest" where another point is nearer at `s = 1`.
    Slides {
        /// Another point of `B` is nearer the anchor at `s = 1`.
        not_nearest: bool,
    },
    /// The followed minimiser ends: the path stops at `s`, its last frame, labelled "the followed system ends here".
    Ended {
        /// The last frame's `s`.
        s: f64,
    },
}

/// A slide's frame end: `end` placed at the chart point `x`, its anchor dropped (a placement, not a pinned state).
pub fn placed(end: &View, pin: [f64; 2], x: [f64; 2]) -> View {
    View {
        anchor: None,
        ..place(end, pin, x)
    }
}

/// `view` placed so its pin shows the chart point `x`, the window's centre by `hold_window`, so the pin's own address
/// gives `x` bit for bit wherever a centre can.
pub fn place(view: &View, pin: [f64; 2], x: [f64; 2]) -> View {
    let mut out = view.clone();
    let (centre, _) = hold_window(&view.sim.chart.window, pin, &x);
    out.sim.chart.window.centre = centre;
    hold_pin_at(&mut out, x);
    out
}

// ── Re-centring an end ─────────────────────────────────────────────────────────────────────────────────────────────

/// One frame of a slide: its parameter, the chart point the end is placed at, and whether another point is nearer.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    /// The frame's `s`.
    pub s: f64,
    /// The chart point the end is placed at.
    pub x: [f64; 2],
    /// Labelled "not nearest": the restart rule finds a point nearer by more than the lock's tolerance.
    pub not_nearest: bool,
}

/// A slide's track: the followed minimiser at each frame of a fixed grid, `s = k/n`, from the first frame, and where it
/// ended, labelled "the followed system ends here".
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Track {
    /// The pin's address.
    pub pin: [f64; 2],
    /// The frames, in order of `s`.
    pub nodes: Vec<Node>,
    /// The followed minimiser ended after the last node.
    pub ended: bool,
}

impl Track {
    /// The chart point the end is placed at, at `s`: the first node's below it, a straight line between nodes, the last
    /// node's at and past it.
    pub fn at(&self, s: f64) -> [f64; 2] {
        let n = &self.nodes;
        let Some(first) = n.first() else {
            return [0.0, 0.0];
        };
        if s <= first.s {
            return first.x;
        }
        for w in n.windows(2) {
            if s <= w[1].s {
                let f = (s - w[0].s) / (w[1].s - w[0].s);
                return std::array::from_fn(|i| w[0].x[i] + f * (w[1].x[i] - w[0].x[i]));
            }
        }
        n[n.len() - 1].x
    }

    /// The last frame's `s`: 1 unless the followed minimiser ended.
    pub fn end(&self) -> f64 {
        self.nodes.last().map_or(0.0, |n| n.s)
    }

    /// Whether the frame at `s` is labelled "not nearest": its bracketing nodes' label, either one's.
    pub fn not_nearest(&self, s: f64) -> bool {
        let n = &self.nodes;
        for w in n.windows(2) {
            if s >= w[0].s && s <= w[1].s {
                return w[0].not_nearest || w[1].not_nearest;
            }
        }
        n.last().is_some_and(|l| s >= l.s && l.not_nearest)
            || n.first().is_some_and(|f| s <= f.s && f.not_nearest)
    }
}

/// How an end meets the anchor.
#[derive(Clone, Debug, PartialEq)]
pub enum Recentred {
    /// The end holds the anchor: re-centred so its pin shows it, within the lock's tolerance.
    Holds(View),
    /// The end can't hold it: the anchor slides along `track` to the followed minimiser.
    Slides {
        /// The end as the request gave it, the track's placements applied to it.
        end: View,
        /// The track.
        track: Track,
    },
}

/// The end `b` re-centred on `a`'s anchor at `a`'s pin (§ 17.8): a latent end's centre re-solved so the pin is the
/// anchor's latent point ([`hold_plane`] at the pin's held point on a latent `A`, at `z_locked` elsewhere), the end
/// then holding the anchor's state; a kernel map's end placed at its nearest point ([`nearest`]), holding the anchor
/// where that point's state is within the lock's tolerance of it in `norm`; else the slide's track over `frames`
/// frames ([`slide`]). The held end carries `a`'s anchor and pin; a sliding end its own pin.
pub fn recentre<L, N>(
    a: &View,
    b: &View,
    leaves: &L,
    norm: N,
    frames: usize,
) -> Result<Recentred, PathError>
where
    L: Leaves,
    N: Fn(&Ic<f64>, &Ic<f64>) -> f64,
{
    let anchor = a.anchor.ok_or(PathError::NotBuilt(
        "a locked path from a view with no recorded anchor",
    ))?;
    let (pin, _) = pin_address(a).ok_or(PathError::Lock(crate::lock::LockError::NotLocked))?;
    let pin_of = |v: &View| match pin == [0.5, 0.5] {
        true => Pin::Centre,
        false => Pin::at(v, pin[0], pin[1]),
    };
    let held = |v: View| -> View {
        let mut v = View {
            anchor: Some(anchor),
            ..v
        };
        v.sim.lock = a.sim.lock.clone();
        v.pin = Some(pin_of(&v));
        v
    };
    match (&b.map, b.window()) {
        (Map::Blend(_), _) => Err(PathError::NotBuilt("a locked path to a blend")),
        (_, ViewWindow::Plane(plane)) => {
            let z = match a.pin {
                Some(Pin::At {
                    point: PinPoint::Latent(z),
                    ..
                }) => z,
                _ if matches!(a.window(), ViewWindow::Plane(_)) => {
                    crate::lock::centre(&a.sim.plane)
                }
                _ => a.sim.lock.z_locked,
            };
            let (z0, _) = hold_plane(plane, pin, &z);
            let mut out = b.clone();
            out.sim.plane.z0 = z0;
            // The target's own decode at the pin, before it carries the anchor: read through the select it would show
            // the stored words whatever it holds. On a nonlinear start after an excursion `z_locked` is stale (a
            // slider's move does not change it) and this refuses: that start's pinned state has no latent point here.
            out.anchor = None;
            let shown = leaves.decode(&out, pin)?;
            let out = held(out);
            match norm(&anchor.ic(), &shown.ic) <= LOCK_TOLERANCE {
                true => Ok(Recentred::Holds(out)),
                false => Err(PathError::Refused(
                    "the latent end does not hold the anchor's latent point",
                )),
            }
        }
        (_, ViewWindow::Chart(_)) => {
            // No converged nearest point (one on the edge of `B`'s states, say) holds nothing: the anchor slides.
            if let Ok(found) = nearest(b, leaves, pin, &anchor) {
                // The hold is judged on the state the returned view shows at the pin (`place`'s window, its anchor
                // dropped so the select can't show the stored words), as `rehold` judges it, not on the solver's own
                // decode through `placed_at`'s window, which can differ from it by a few ε.
                let at = place(b, pin, found.x);
                let own = leaves
                    .decode(
                        &View {
                            anchor: None,
                            ..at.clone()
                        },
                        pin,
                    )
                    .map(|d| d.ic);
                #[cfg(feature = "controls")]
                let own = match probe::switches().placed_at_hold {
                    true => Ok(found.ic),
                    false => own,
                };
                // A decode that fails at the nearest point holds nothing there: the anchor slides.
                if own.is_ok_and(|ic| norm(&anchor.ic(), &ic) <= LOCK_TOLERANCE) {
                    return Ok(Recentred::Holds(held(at)));
                }
            }
            let track = slide(b, leaves, pin, &anchor, frames)?;
            Ok(Recentred::Slides {
                end: b.clone(),
                track,
            })
        }
    }
}

/// The slide's track over `frames` frames (`s = k/frames`): the minimiser of the frame's pin state's block distance to
/// the anchor, at the first frame by the restart rule, then followed frame to frame (`lock_solve::follow`), each frame
/// labelled "not nearest" where the restart rule finds a point whose residual is below the followed one's by more than
/// the lock's tolerance; it ends at the last frame before the followed minimiser ends (`Failure::Ended`, or a frame
/// whose direct solve fails with its step leaving `B`'s states, [`leaves_states`]). [`PathError::FollowFailed`] where a
/// frame's solve fails anywhere else: the solver's failure, never read as an end.
pub fn slide<L: Leaves>(
    b: &View,
    leaves: &L,
    pin: [f64; 2],
    anchor: &Anchor,
    frames: usize,
) -> Result<Track, PathError> {
    if frames < 2 {
        return Err(PathError::Parameter);
    }
    let h = b.sim.chart.window.half[0]
        .abs()
        .max(b.sim.chart.window.half[1].abs());
    let family = Slide {
        placed: Placed {
            view: b,
            leaves,
            pin,
            step: step(b),
        },
        anchor: anchor.blocks(),
    };
    let target = family.target();
    let (spec, cont) = (spec(), continuity(h));
    let seeds = seeds(b, pin);
    let restart = |s: f64| {
        lock_solve::restart(
            &lock_solve::Frame { family: &family, s },
            &target,
            &seeds,
            &spec,
        )
    };
    let nearer = |followed: &Report<f64>, s: f64| {
        let best = restart(s);
        best.converged() && best.residual < followed.residual - LOCK_TOLERANCE
    };
    let s1 = 1.0 / frames as f64;
    let first = restart(s1);
    logged("slide first", s1, &first);
    if !first.converged() {
        return Err(PathError::Refused("the re-solve found no nearest point"));
    }
    let mut nodes = vec![Node {
        s: s1,
        x: first.x,
        not_nearest: false,
    }];
    let mut ended = false;
    for k in 2..=frames {
        let s = k as f64 / frames as f64;
        let last = nodes[nodes.len() - 1];
        #[cfg(feature = "controls")]
        let last = match probe::switches().restart_seed {
            true => Node {
                x: restart(s).x,
                ..last
            },
            false => last,
        };
        let report = lock_solve::follow(&family, &target, last.s, last.x, s, &spec, &cont);
        logged("follow", s, &report);
        #[cfg(feature = "controls")]
        let report = match (probe::switches().failed_is_end, report.status) {
            (true, Status::Failed(_)) => Report {
                status: Status::Failed(Failure::Ended),
                ..report
            },
            _ => report,
        };
        match report.status {
            Status::Converged(_) => nodes.push(Node {
                s,
                x: report.x,
                not_nearest: nearer(&report, s),
            }),
            // The continuity test, or a probe inside the frame, finds no one minimiser joining the frame's ends: the
            // followed system ends here.
            Status::Failed(Failure::Ended) => {
                ended = true;
                break;
            }
            // The frame's own solve failed. Where its step from the last frame's point leaves `B`'s states, the
            // minimiser has run into their edge, an end the direct solve meets as a stall; anywhere else the follow
            // failed, which says nothing of where the minimiser ends.
            // The plan is unclear here (it defines an ending by the continuity test, and a direct solve's failure not at
            // all); chose this, because the constructed ending's minimiser runs into the edge of `B`'s states and its
            // direct solve stalls there, its step leaving them, while the slide's stalls at its first frames (PR #72's
            // C6) were short steps inside them.
            Status::Failed(failure) => match leaves_states(&family, &target, s, last.x) {
                true => {
                    ended = true;
                    break;
                }
                false => return Err(PathError::FollowFailed { s, failure }),
            },
        }
    }
    Ok(Track { pin, nodes, ended })
}

/// Whether the Gauss–Newton step at the frame `s` from `x` (the point the follow converged at, the frame before) leaves
/// the family's states: its value, or its slopes, not finite at `x` or at `x + δ`, `JᵀJδ = −Jᵀr`. Where the followed
/// minimiser runs into the edge of `B`'s states the step toward it does; where a solve stalls on its own precision the
/// step is a short one inside them.
fn leaves_states<F: lock_solve::Family<f64, 9>>(
    family: &F,
    target: &Target<f64, 9>,
    s: f64,
    x: [f64; 2],
) -> bool {
    let (Some(v), Some(j)) = (family.value(s, x), family.jacobian(s, x)) else {
        return true;
    };
    let r: [f64; 9] = core::array::from_fn(|i| target.weight[i] * (v[i] - target.value[i]));
    let dot = |p: &[f64; 9], q: &[f64; 9]| (0..9).map(|i| p[i] * q[i]).sum::<f64>();
    let (a11, a12, a22) = (dot(&j[0], &j[0]), dot(&j[0], &j[1]), dot(&j[1], &j[1]));
    let (g1, g2) = (dot(&j[0], &r), dot(&j[1], &r));
    let det = a11 * a22 - a12 * a12;
    let d = [(a12 * g2 - a22 * g1) / det, (a12 * g1 - a11 * g2) / det];
    if !(det > 0.0 && d.iter().all(|x| x.is_finite())) {
        return false;
    }
    family.value(s, [x[0] + d[0], x[1] + d[1]]).is_none()
}

// ── A mid-morph slice: `B` re-held ─────────────────────────────────────────────────────────────────────────────────

/// `b` re-placed to hold the state `a` shows at its pin (a mid-morph slice under a lock moved it), on a blend at share
/// `s`, and whether it holds it. Placed first where it would hold: a latent `b` by [`hold_plane`] at `a`'s pinned
/// latent point, where `a` is latent; a kernel map's `b` at the nearest point to that state, solved from `b`'s own
/// placement at the pin (the chart point its pin shows). It holds, as [`recentre`] decides, where the state the placed
/// `b` shows at the pin is within the lock's tolerance of `a`'s pinned state in `norm`, its own decode read through
/// `leaves`, never the select. A latent `b` that does not is refused, as [`recentre`] refuses it. Where a kernel map's
/// `b` can't hold, the anchor slides: `b` placed at the local minimiser of the frame's pin-state distance,
/// `d(block_line(a, B_x, s), a)` (§ 17.8's slide), solved from the same placement, so a slice of a sliding pair
/// continues its slide rather than snapping to `b`'s nearest point.
/// The plan is unclear here (a slice mid-morph goes to each end "in its own hidden coordinates ... about the pinned
/// state"); chose to slice each end in its own hidden coordinates and then re-solve `b`'s placement this way, because
/// the lock is the path's: both ends show one pinned state where `b` can, and the slide where it can't.
/// [`PathError::NotBuilt`] for a latent `b` with a nonlinear `a`, whose pinned state has no latent point here;
/// [`PathError::Refused`] where the nearest-point solve does not converge, [`PathError::FollowFailed`] where the slide's
/// own does not.
pub fn rehold<L, N>(
    a: &View,
    b: &View,
    leaves: &L,
    norm: N,
    s: f64,
) -> Result<(View, bool), PathError>
where
    L: Leaves,
    N: Fn(&Ic<f64>, &Ic<f64>) -> f64,
{
    let (pin, _) = pin_address(a).ok_or(PathError::Lock(crate::lock::LockError::NotLocked))?;
    let shown = decoded(state_at(a, leaves, pin)?)?;
    // The placed `b`'s own state at the pin against `a`'s pinned state, in `norm` (as `recentre`): its anchor dropped,
    // so the select can't show the stored words in its place.
    let within = |v: &View| -> Result<bool, PathError> {
        let own = View {
            anchor: None,
            ..v.clone()
        };
        let at = leaves.decode(&own, pin)?;
        Ok(norm(&shown.ic, &at.ic) <= LOCK_TOLERANCE)
    };
    let latent = |out: View| -> Result<(View, bool), PathError> {
        #[cfg(feature = "controls")]
        if probe::switches().block_hold {
            return Ok((out, true));
        }
        match within(&out)? {
            true => Ok((out, true)),
            false => Err(PathError::Refused(
                "the latent end does not hold the pinned state",
            )),
        }
    };
    let (mut out, holds) = match (b.window(), a.pin, a.window()) {
        (
            ViewWindow::Plane(plane),
            Some(Pin::At {
                point: PinPoint::Latent(z),
                ..
            }),
            _,
        ) => {
            let mut out = b.clone();
            out.sim.plane.z0 = hold_plane(plane, pin, &z).0;
            latent(out)?
        }
        (ViewWindow::Plane(plane), _, ViewWindow::Plane(ap)) => {
            let mut out = b.clone();
            out.sim.plane.z0 = hold_plane(plane, pin, &crate::lock::point(ap, pin[0], pin[1])).0;
            latent(out)?
        }
        (ViewWindow::Plane(_), _, _) => {
            return Err(PathError::NotBuilt(
                "a latent end re-held to a nonlinear end's pinned state",
            ))
        }
        (ViewWindow::Chart(w), _, _) => {
            let seed = match b.pin {
                Some(Pin::At {
                    point: PinPoint::Chart(x),
                    ..
                }) => x,
                _ => w.at(pin[0], pin[1]),
            };
            let target_blocks = Blocks::of(&shown.ic);
            let placed = Placed {
                view: b,
                leaves,
                pin,
                step: step(b),
            };
            let target = Target {
                value: target_blocks.vector(),
                weight: [1.0; 9],
            };
            let near = lock_solve::solve(&placed, &target, seed, &spec());
            logged("rehold", s, &near);
            #[cfg(feature = "controls")]
            if probe::switches().floor {
                let floor = lock_solve::solve(&placed, &target, seed, &spec_at(0.0));
                logged("rehold floor", s, &floor);
            }
            let held = place(b, pin, near.x);
            // A decode that fails at the nearest point holds nothing there: the anchor slides.
            let holds = near.converged() && within(&held).unwrap_or(false);
            #[cfg(feature = "controls")]
            let holds = match probe::switches().block_hold {
                true => near.converged() && near.residual <= LOCK_TOLERANCE,
                false => holds,
            };
            if holds {
                (held, true)
            } else {
                let family = Slide {
                    placed,
                    anchor: target_blocks,
                };
                let frame = lock_solve::Frame { family: &family, s };
                let slid = lock_solve::solve(&frame, &family.target(), seed, &spec());
                logged("rehold slide", s, &slid);
                match slid.status {
                    Status::Converged(_) => {}
                    Status::Failed(failure) => return Err(PathError::FollowFailed { s, failure }),
                }
                (place(b, pin, slid.x), false)
            }
        }
    };
    out.anchor = None;
    Ok((out, holds))
}

// ── The offset form ────────────────────────────────────────────────────────────────────────────────────────────────

/// A tangent vector at a state's blocks: `(Δμ₁, Δμ₂, v_n (3, tangent to n̂), Δp_ρ (2), Δp_λ (2))`.
pub type Tangent = [f64; 9];

fn dot3(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm3(a: &[f64; 3]) -> f64 {
    dot3(a, a).sqrt()
}

/// The log map at `x`: `y`'s offset from `x`, the shape by the sphere's log map (`atan2` of the angle, as § 17.7).
pub fn log(x: &Blocks, y: &Blocks) -> Tangent {
    let c = dot3(&x.n, &y.n);
    let w = [
        y.n[0] - c * x.n[0],
        y.n[1] - c * x.n[1],
        y.n[2] - c * x.n[2],
    ];
    let nw = norm3(&w);
    let v = match nw > 0.0 {
        true => {
            let theta = libm::atan2(nw, c);
            [theta * w[0] / nw, theta * w[1] / nw, theta * w[2] / nw]
        }
        false => [0.0; 3],
    };
    [
        y.mu[0] - x.mu[0],
        y.mu[1] - x.mu[1],
        v[0],
        v[1],
        v[2],
        y.p_rho[0] - x.p_rho[0],
        y.p_rho[1] - x.p_rho[1],
        y.p_lambda[0] - x.p_lambda[0],
        y.p_lambda[1] - x.p_lambda[1],
    ]
}

/// The exponential map at `x`: the blocks `o` away, the shape along the great circle of `o`'s part tangent to `n̂`.
pub fn exp(x: &Blocks, o: &Tangent) -> Blocks {
    let raw = [o[2], o[3], o[4]];
    let along = dot3(&raw, &x.n);
    let v = [
        raw[0] - along * x.n[0],
        raw[1] - along * x.n[1],
        raw[2] - along * x.n[2],
    ];
    let theta = norm3(&v);
    let n = match theta > 0.0 {
        true => {
            let (s, c) = (libm::sin(theta), libm::cos(theta));
            std::array::from_fn(|i| c * x.n[i] + s * v[i] / theta)
        }
        false => x.n,
    };
    Blocks {
        mu: [x.mu[0] + o[0], x.mu[1] + o[1]],
        n,
        p_rho: [x.p_rho[0] + o[5], x.p_rho[1] + o[6]],
        p_lambda: [x.p_lambda[0] + o[7], x.p_lambda[1] + o[8]],
    }
}

/// A tangent vector at the shape `from` carried to the shape `to` by the rotation that takes one to the other about
/// their common normal; unchanged where they are equal.
fn carry(v: &Tangent, from: &[f64; 3], to: &[f64; 3]) -> Tangent {
    if from.map(f64::to_bits) == to.map(f64::to_bits) {
        return *v;
    }
    let k = [
        from[1] * to[2] - from[2] * to[1],
        from[2] * to[0] - from[0] * to[2],
        from[0] * to[1] - from[1] * to[0],
    ];
    let sin = norm3(&k);
    if sin == 0.0 {
        return *v;
    }
    let k = k.map(|c| c / sin);
    let cos = dot3(from, to);
    let phi = libm::atan2(sin, cos);
    let (s, c) = (libm::sin(phi), libm::cos(phi));
    let u = [v[2], v[3], v[4]];
    let kxu = [
        k[1] * u[2] - k[2] * u[1],
        k[2] * u[0] - k[0] * u[2],
        k[0] * u[1] - k[1] * u[0],
    ];
    let ku = dot3(&k, &u);
    let r: [f64; 3] = std::array::from_fn(|i| u[i] * c + kxu[i] * s + k[i] * ku * (1.0 - c));
    let mut out = *v;
    out[2..5].copy_from_slice(&r);
    out
}

fn dot9(a: &Tangent, b: &Tangent) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm9(a: &Tangent) -> f64 {
    dot9(a, a).sqrt()
}

/// The QR factors of a frame `[j₁ j₂]`: `q₁, q₂` orthonormal and `(r₁₁, r₁₂, r₂₂)`, `r₁₁, r₂₂ > 0`; `None` for a frame
/// with no area.
fn qr(j: &[Tangent; 2]) -> Option<([Tangent; 2], [f64; 3])> {
    let r11 = norm9(&j[0]);
    if r11.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater) {
        return None;
    }
    let q1 = j[0].map(|x| x / r11);
    let r12 = dot9(&q1, &j[1]);
    let v: Tangent = std::array::from_fn(|i| j[1][i] - r12 * q1[i]);
    let r22 = norm9(&v);
    if r22.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater) {
        return None;
    }
    Some(([q1, v.map(|x| x / r22)], [r11, r12, r22]))
}

/// A unit vector orthogonal to `avoid`, from the coordinate axes, the mass and momentum axes first (always tangent).
fn orthogonal(avoid: &[Tangent]) -> Tangent {
    let mut best = ([0.0; 9], 0.0);
    for k in [0usize, 1, 5, 6, 7, 8, 2, 3, 4] {
        let mut e = [0.0; 9];
        e[k] = 1.0;
        for a in avoid {
            let d = dot9(&e, a);
            for i in 0..9 {
                e[i] -= d * a[i];
            }
        }
        let n = norm9(&e);
        if n > 0.5 {
            return e.map(|x| x / n);
        }
        if n > best.1 {
            best = (e.map(|x| x / n), n);
        }
    }
    best.0
}

/// The angle `x` wrapped into `(−π, π]`.
fn wrap(x: f64) -> f64 {
    let pi = core::f64::consts::PI;
    let t = x.rem_euclid(2.0 * pi);
    match t > pi {
        true => t - 2.0 * pi,
        false => t,
    }
}

/// The frame at `s` along the geodesic between the tangent frames `ja` and `jb` (§ 17.7): their orthonormal parts by
/// the principal angles, each `atan2(‖w‖, a·b)`, the in-plane rotation eased the short way; their triangular parts with
/// `r₁₁` and `r₂₂` geometrically interpolated, so the frame's area moves from `A`'s to `B`'s monotonically and never
/// through 0. An antipodal principal pair (opposite handedness) turns through a direction orthogonal to both frames.
/// A frame with no area blends linearly.
pub fn transport(ja: &[Tangent; 2], jb: &[Tangent; 2], s: f64) -> [Tangent; 2] {
    let (Some((qa, ra)), Some((qb, rb))) = (qr(ja), qr(jb)) else {
        return [0, 1].map(|c| std::array::from_fn(|i| ja[c][i] + s * (jb[c][i] - ja[c][i])));
    };
    let m = [
        [dot9(&qa[0], &qb[0]), dot9(&qa[0], &qb[1])],
        [dot9(&qa[1], &qb[0]), dot9(&qa[1], &qb[1])],
    ];
    // M = R(φ)·diag(σ₁, σ₂)·R(θ), both factors rotations, σ₂ signed.
    let e = (m[0][0] + m[1][1]) / 2.0;
    let f = (m[0][0] - m[1][1]) / 2.0;
    let g = (m[1][0] + m[0][1]) / 2.0;
    let h = (m[1][0] - m[0][1]) / 2.0;
    let (qq, rr) = (crate::exact::hypot(e, h), crate::exact::hypot(f, g));
    let sigma = [qq + rr, qq - rr];
    let (a1, a2) = (libm::atan2(g, f), libm::atan2(h, e));
    let (theta, phi) = ((a2 - a1) / 2.0, (a2 + a1) / 2.0);
    let (sp, cp) = (libm::sin(phi), libm::cos(phi));
    let (st, ct) = (libm::sin(theta), libm::cos(theta));
    // U = R(φ), V = R(−θ): the principal vectors a = Q_A·U, b = Q_B·V.
    let u = [[cp, sp], [-sp, cp]];
    let v = [[ct, -st], [st, ct]];
    let combo = |q: &[Tangent; 2], c: [f64; 2]| -> Tangent {
        std::array::from_fn(|i| c[0] * q[0][i] + c[1] * q[1][i])
    };
    let pa = [combo(&qa, u[0]), combo(&qa, u[1])];
    let pb = [combo(&qb, v[0]), combo(&qb, v[1])];
    let mut used: Vec<Tangent> = vec![pa[0], pa[1]];
    let g_at: Vec<Tangent> = (0..2)
        .map(|i| {
            let w: Tangent = std::array::from_fn(|k| pb[i][k] - sigma[i] * pa[i][k]);
            let nw = norm9(&w);
            let (dir, angle) = match nw > f64::EPSILON * 16.0 {
                true => (w.map(|x| x / nw), libm::atan2(nw, sigma[i])),
                false if sigma[i] >= 0.0 => return pa[i],
                false => {
                    let d = orthogonal(&used);
                    used.push(d);
                    (d, core::f64::consts::PI)
                }
            };
            let (sn, cs) = (libm::sin(s * angle), libm::cos(s * angle));
            std::array::from_fn(|k| cs * pa[i][k] + sn * dir[k])
        })
        .collect();
    // The in-plane rotation W(s) from Uᵀ = R(−φ) to Vᵀ = R(θ), the short way.
    let turn = -phi + s * wrap(theta + phi);
    let (sw, cw) = (libm::sin(turn), libm::cos(turn));
    let q: [Tangent; 2] = [
        std::array::from_fn(|k| cw * g_at[0][k] + sw * g_at[1][k]),
        std::array::from_fn(|k| -sw * g_at[0][k] + cw * g_at[1][k]),
    ];
    let geo = |x: f64, y: f64| crate::exact::pow(x, 1.0 - s) * crate::exact::pow(y, s);
    let r11 = geo(ra[0], rb[0]);
    let r12 = ra[1] + s * (rb[1] - ra[1]);
    let r22 = geo(ra[2], rb[2]);
    [
        q[0].map(|x| x * r11),
        std::array::from_fn(|k| r12 * q[0][k] + r22 * q[1][k]),
    ]
}

/// How a locked blend frame's pixels are formed: the offset form (the transported first-order part plus the blended
/// remainder) and the select at the pin. A control (the select removed, offsets blended linearly) sets the switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OffsetForm {
    /// The pin's pixel takes the stored words where both ends hold the anchor.
    pub select: bool,
    /// The first-order parts are transported; `false` blends the offsets linearly, the squashing § 17.7 measures.
    pub transport: bool,
    /// A control's switch: the transport run from `B`'s frame to `A`'s, so a frame near `A` takes `B`'s tangent.
    #[cfg(feature = "controls")]
    pub reversed: bool,
    /// A control's switch: the frame treated as holding the pinned state whether `B` holds it or not.
    #[cfg(feature = "controls")]
    pub forced_hold: bool,
}

impl Default for OffsetForm {
    fn default() -> Self {
        OffsetForm {
            select: true,
            transport: true,
            #[cfg(feature = "controls")]
            reversed: false,
            #[cfg(feature = "controls")]
            forced_hold: false,
        }
    }
}

/// One end's offsets about its pin state: the pin state's blocks, the first-order frame `J` at the pin by a central
/// difference of step `ε^{1/3}` in the address, and the offset at the pixel.
pub(crate) struct EndOffsets {
    pub base: Blocks,
    pub j: [Tangent; 2],
    pub at: Tangent,
}

/// `end`'s offsets at `address` about its pin state `base` (§ 17.8): `log_base` of its pixel, and `J` at the pin; `None`
/// where any of the pixels has no state.
pub(crate) fn end_offsets<L: Leaves>(
    end: &View,
    leaves: &L,
    pin: [f64; 2],
    base: Blocks,
    address: [f64; 2],
) -> Result<Option<EndOffsets>, PathError> {
    let blocks_at = |c: [f64; 2]| -> Result<Option<Blocks>, PathError> {
        Ok(match state_at(end, leaves, c)? {
            Pixel::State { decoded, .. } => {
                let b = Blocks::of(&decoded.ic);
                b.vector().iter().all(|x| x.is_finite()).then_some(b)
            }
            Pixel::Blank(_) => None,
        })
    };
    let h = crate::exact::cbrt(f64::EPSILON);
    let mut j = [[0.0; 9]; 2];
    for (axis, col) in j.iter_mut().enumerate() {
        let (mut plus, mut minus) = (pin, pin);
        plus[axis] += h;
        minus[axis] -= h;
        let (Some(bp), Some(bm)) = (blocks_at(plus)?, blocks_at(minus)?) else {
            return Ok(None);
        };
        let (op, om) = (log(&base, &bp), log(&base, &bm));
        *col = std::array::from_fn(|i| (op[i] - om[i]) / (2.0 * h));
    }
    let Some(y) = blocks_at(address)? else {
        return Ok(None);
    };
    Ok(Some(EndOffsets {
        base,
        j,
        at: log(&base, &y),
    }))
}

/// The locked frame's pixel at `address`, `0 < s < 1`, from the ends' offsets about their pin states and the frame's
/// pin state `base` (the anchor on a held path, the slide's `block_line(a, B_x, s)` on a sliding one): the remainders
/// `o − J(c − pin)` blended, the first-order parts transported ([`transport`]), all carried to `base`'s shape first,
/// then the exponential map at `base`.
pub(crate) fn offset_pixel(
    a: &EndOffsets,
    b: &EndOffsets,
    base: &Blocks,
    pin: [f64; 2],
    address: [f64; 2],
    s: f64,
    form: OffsetForm,
) -> (Blocks, bool) {
    let carry_a = |v: &Tangent| carry(v, &a.base.n, &base.n);
    let carry_b = |v: &Tangent| carry(v, &b.base.n, &base.n);
    let d = [address[0] - pin[0], address[1] - pin[1]];
    let (ja, jb) = (
        [carry_a(&a.j[0]), carry_a(&a.j[1])],
        [carry_b(&b.j[0]), carry_b(&b.j[1])],
    );
    let (oa, ob) = (carry_a(&a.at), carry_b(&b.at));
    let o: Tangent = match form.transport {
        true => {
            let ra: Tangent = std::array::from_fn(|i| oa[i] - (ja[0][i] * d[0] + ja[1][i] * d[1]));
            let rb: Tangent = std::array::from_fn(|i| ob[i] - (jb[0][i] * d[0] + jb[1][i] * d[1]));
            #[cfg(feature = "controls")]
            let t = match form.reversed {
                true => transport(&jb, &ja, s),
                false => transport(&ja, &jb, s),
            };
            #[cfg(not(feature = "controls"))]
            let t = transport(&ja, &jb, s);
            std::array::from_fn(|i| t[0][i] * d[0] + t[1][i] * d[1] + ra[i] + s * (rb[i] - ra[i]))
        }
        false => std::array::from_fn(|i| oa[i] + s * (ob[i] - oa[i])),
    };
    (exp(base, &o), false)
}

/// The frame's pin state on a sliding path: `block_line(a, b, s)` of the ends' pin states, and whether the shape's
/// great circle met its antipodal tie.
pub(crate) fn slid(a: &Blocks, b: &Blocks, s: f64) -> (Blocks, bool) {
    block_line(a, b, s)
}

/// The select: the stored words at the pin, as a decode, its masses, configuration and momenta `like`'s (end `A`'s own
/// decode of the pin, the same state to its words), its IC and descriptor the anchor's.
pub(crate) fn selected(anchor: &Anchor, like: &Decoded<f64>) -> Decoded<f64> {
    let ic = anchor.ic();
    Decoded {
        ic,
        descriptor: descriptor::describe_decoded(&ic, &like.config),
        ..*like
    }
}
