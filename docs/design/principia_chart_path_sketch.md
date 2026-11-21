# The chart-path object: type sketch and rationale (stage 1)

*TASK-M2-61's gate. The core types of the chart-path object as one module, `crates/engine/src/path/`, with a one-page
rationale. It is reviewed against `principia_chart_space_model.md` (§ 1 to § 5, § 8) and
`principia_chart_composition.md` § 17 before any of it is implemented. Stage 1 implements only what today's operations
use. Everything marked **sketch** is typed here so later stages have a place to go, and carries no behaviour until its
stage's task builds it.*

*Revision 2, after the code review of PR #24 (review 6002086849): the view carries the whole `SimConfig`; the lock's
set, recentre and snap-back are named cuts; a slider move is its own family; the raw tilt names its single-vector and
pair forms; the shape block's same-side degenerate case has its rule; alignment moves out of `Timing` and the record
gains optional fields; segments are built only by constructors. Revision 3, after the physics review (review
6002094291): where a nonlinear chart's hidden coordinates live; the R-474 tilt family typed over both maps; one
evaluator per window shape; `s` canonicalised; the oblique slice a constructor of a view; the tie's asymmetry under
reversal and its step at `s = 1`. Revision 4, from the approving reviews' notes (6002173203, 6002179968), as built: the
general circle wherever `‖w‖ > 0` on the same side (`c ≥ 0`); a kernel map holds its registry index only; a family's `from` is read
from `A`; a raw tilt is a family only where it starts at `A`; a cut's and a commit's `B` is stored and wins; the record's
serialisation.*

## 1. Rationale (one page)

**A view is a point; an edit is a path.** A view is a **map** (a chart: named coordinates to a system state) seen
through a **window** `(c, Q)`. Every view edit is a **segment**: two views, a named **family**, its own **timing**, and
(sketch) its alignment. A **path** is a chain of segments (the model's Theorem 4). The **lock** is state carried in the
view; its edits are segments like any other, and stage 1 keeps what they do today (below).

**The view carries the whole `SimConfig`.** Today's sim key holds the plane `(z₀, q₁, q₂)` and `Chart.window
{centre, half}` side by side whichever chart is shown, and beside them the slice frame, the mapping, the links, the
horizon, the kernel variant and the rest. So `View { map, sim }` holds the `SimConfig` as it is, and the window `(c, Q)`
is read from it:

| Today (`SimConfig`) | Window `(c, Q)` |
|---|---|
| `plane: Plane { z0, q1, q2 }`, read by the latent map | `c = z₀ ∈ ℝ⁸` relative to the plane anchor (the origin of `Z`), `Q = [q₁ q₂]`, 8×2, f64 |
| `chart.window: { centre, half }`, read by a kernel map | `c = centre ∈ ℝ²`, `Q = diag(half)`, 2×2, f64 |

Each family writes only the fields it acts on (a pan writes `plane.z0`; a lock on a nonlinear chart writes `plane.z0`,
`lock` and `chart.window` in one segment), and every other field passes through untouched. So a view's `SimConfig`, its
canonical serialisation and its sim-key bytes are today's, byte for byte. A general `Q` on a kernel map (a rotated
window) is a **sketch**.

**Where a nonlinear chart's hidden coordinates live.** In `sim.plane.z0`, as today: a kernel map's frozen controls are
`z₀`'s (chart_reference §2.1, "Untilted, they are `z₀`'s at every pixel"), and `Map::Kernel` holds the registry entry
and its own parameters, the chart value being rebuilt on the view's `z₀` when it is evaluated. So a lock on a nonlinear
chart, which writes `z₀` and the window, is a view edit, not a map edit; only `Cut` (a chart switch) and `Commit` change
the map.

**One evaluator per window shape.** A family acts on the window shape its route acts on, by one evaluator per shape:
stage 1 builds the plane's (the only shape an engine route edited then). Stage 2 adds the chart window's evaluator for
zoom (`family::zoom_window`), which zoom about a lock's pin on a nonlinear chart needs; pan on a chart window exists only
as a GUI route (`gui/src/explore/manifold_view/navigate.rs`) until the consolidation task (TASK-M2-64), its evaluator
an added one, not a branch.

**One family per formula that exists in code. Special cases are families, never if-branches** (§ 17 header). In
floating point, "an operation is its family's member" means the same formula in the same order (model § 8). So each
family's interior frame `H_s` calls the *same* function the direct operation calls, at the family's parameter scaled by
`s`, and is tested bitwise against it. Where no such scaled call exists, the operation is a **cut**: it has no interior
(model § 3's chart switch, "the path's interior is skipped"), and it is labelled with why. The families:

| Family | Today's route | `H_s`, `0 < s < 1` | Model § 3 |
|---|---|---|---|
| `Pan { ds, dt }` | `nav::pan` | `pan(A, s·ds, s·dt)` | reparametrisation |
| `Zoom { octaves }` | `nav::zoom` | `zoom(A, s·octaves)` (log-stepped: `2^(−s·octaves)`) | reparametrisation |
| `Slice { d, t }` | `nav::slice` only | `slice(A, d, s·t)`: along `off_plane(d)` | latent translation |
| `Control { k, from, to }` | `lock::move_slider` | `move_slider(A, k, from + s·(to − from))`: `z₀[k]` assigned, no projection | an excursion along `ê_k` |
| `RawTilt { form: Single(which), … }` | `nav::ChartConstructor::tilt` | `tilt(committed, A, which, τ_A + s(τ_B − τ_A), target)` | R-474's raw tilt (§ 0.2) |
| `RawTilt { form: Pair, … }` | `nav::ChartConstructor::tilt_pair` | `tilt_pair` from the committed plane, each `τᵢ` lerped, its target fixed | R-474's raw tilt |
| `Cut { label }` | the chart switch (no engine code); the lock's set, recentre and snap-back; jumps under a lock; a fresh oblique plane | `A`: no interior | — |
| `Commit` | Save view as chart (`kernel::chart::saved::Saved`) | `A`: the same pixels, another map | — |

The two tilt forms are two arms, not one: `tilt` keeps the other vector as `‖current‖·unit(committed)`, where
`tilt_pair` at angle 0 renormalises it through `tilt_tangent`, and the reviewer measured the two differ in the kept
vector's bits on 4,007 of 10,000 Gaussian draws. A raw tilt's targets are fixed for the segment: a segment whose two
ends tilt toward different targets cannot be written: `TiltForm` holds one target per vector for the whole segment, so
lerping `τ` always has a frame, and a change of target is two segments joined at `τ = 0`, or a cut. **A raw tilt is a
family only where it starts at `A`**: its constructor takes the angle at `A` (`from`) and builds a `RawTilt` only where
the committed plane's tilt at `from` is `A`'s basis to rounding (32 ulps of each vector's length: two normalisations of
an 8-vector); otherwise, as for a tilt of one vector from a plane whose other vector is already tilted (raw tilts replace
rather than compose), the direct operation jumps, and the segment is a cut labelled `TiltReplaces`. A slider move's
`from` is read from `A` (`A.z₀[k]`), never given by the caller. Each form reads `current`, `A`'s plane, for the lengths
it keeps. R-474's two maps (Tip as sheet, Lean each axis) are one typed family, `R474Tilt`, over either map (a latent
view through `kernel::chart::tilt::latent_basis`, a curved one through `Tilted`); no engine route calls them today (the
GUI's compass does), so it is `NotBuilt` in stage 1 and TASK-M2-64 builds its evaluators.

**A slider move is `Control`, not `Slice`.** `move_slider` assigns `z₀[k]`, a step along `ê_k` with, in general, an
in-plane part; `Slice`'s interior would follow `off_plane(ê_k)` and not tend to `B`. `Control`'s interior is
`move_slider` itself at the lerped value, so `H_s → B` as `s → 1`; its test has a control arm, a slider move from a
point whose step has an in-plane part evaluated through `Slice`, which fails that limit. Snap-back is an assignment of
the whole `z₀` with no scaled form in today's code, so it is a cut, not a new translation family (a latent translation
`z₀ᴬ + s(z₀ᴮ − z₀ᴬ)` would be new arithmetic, a stage-2 choice with the sliding anchor).

**The lock (stage 2: locking does not move the view).** A lock (`lock::set`, `set_on_chart`) edits the lock group
alone: a `Cut` labelled `CutLabel::Lock`, its `B` differing from `A` in the lock group and the pin, which the view holds
beside its `SimConfig` (`View.pin`: the clicked address and the point it is held at), so the sim key does not move.
**Centre on pin** is its own cut (`CentreOnPin`), `z₀ ←` the pin's held point (on a kernel map the window's centre too);
the stage-1 lock, which re-centred, is the two in one command (`Segment::lock_centred`, `lock_on_chart_centred`,
labelled `LockRecentre`, the setting on), and reproduces every stage-1 lock digest bit for bit. **Zoom and the raw tilt
turn about the pin** inside their families: the basis (or a kernel map's window, zoom's second evaluator) changes, then
the centre is re-solved so the pinned pixel keeps its held point (`lock::hold_plane`, `hold_window`), exact wherever a
float gives it (chart_decoder_contract Part 4, "How the pin is held"). Snap-back (`SnapBack`) re-solves `z₀` so the
pinned pixel is `z_locked` again, the pin unmoved; the re-centre lock validation offers stays `LockRecentre`. A curved
chart's tilt turns about the pin through `lock::Pivoted`, a placement of the kernel's tilt. Unlock clears the lock group
and the pin. The lock's re-recorded baseline cases are `fixtures/chart_composition/lock_stage2.json`. *Was (stage 1):*
"Today a lock is not "lock state only" … each is a `Cut` labelled `CutLabel::LockRecentre`." `lock::curve_tilt`
returns a vector, not a view edit: it stays in `lock`, reading the tilt formula `path::tilt_tangent(γ′(ν₀), length, τ,
d)`, with no family until a route uses it.

**The end rule** (§ 17.4). `s` is f64 and compared exactly: `s == 0.0` returns `A`, and `s == 1.0` returns `B`, the
stored values, never through the family's arithmetic. So every family's ends are bitwise its keyframes, and a zoom's or
a tilt's frame is a plain view: **no `(A, B, family, s)` triple enters any sim key** (§ 17.5). Only a blend frame
(stage 3 and later) would add a new `ChartSpec` arm, so no existing file's key moves.

**The path parameter.** `s` is canonicalised on entry: `−0.0` becomes `+0.0`, so one frame has one record, and NaN or
`s` outside `[0, 1]` is refused (`PathError::Parameter`).

**The oblique slice** (`nav::oblique`) builds a fresh plane from two vectors; it is a constructor of a view, and as an
edit of an existing view it is a `Cut` (until `PlaneRotation` is built).

**A kernel map holds its registry index only.** Its parameters are taken by value where a family needs them (a lock, a
save), as today's functions take them: today's `SimConfig` holds no chart, and a kernel map's hidden coordinates are
`z₀`'s, so storing the chart's words (`Mixed` writes all eight of its frozen controls) would keep a second copy of `z₀`
that a lock or a slider move leaves stale, and two records of one frame could differ.

**Segments are built only by constructors.** Each constructor is the operation: it computes `B` by today's function and
stores `A`, `B` and the family's parameter. A segment read back from a record is rebuilt through its constructor from
`A` and the parameter, and refused if that `B` is not the stored `B` bit for bit, so a record cannot hold a `B` its
family does not reach. A cut's and a commit's `B` has no parameter to rebuild it from (a lock on a nonlinear chart takes
the chart, the tolerances and the norm): it is stored, and it wins.

**Timing is the segment's** (R-901), and it is timing only. `Timing` holds the easing (and, sketch, the retime table);
a frame at playback fraction `u` is `frame(ease(u))`, so easing changes when a frame is shown, never what it shows
(Theorem 5): the family never sees `u`. The easing is a monotone curve with `e(0) = 0` and `e(1) = 1` exactly: stage 1
builds `Linear` and a pinned, monotone piecewise-linear table (the form any curve can be sampled into); which curves are
offered, and the default, are TASK-M8-31's (REQ-TOOL-197); the table is the mechanism any such curve is sampled
into, provisional until then, and `path_easing_timing_only` uses one as its fixture. **Alignment is content, not timing** (§ 17.10: `B` evaluated
at `T_s p`), so it is a field of the segment beside the family, read by `frame` (sketch). Concatenation is
`Path(Vec<Segment>)`, refused unless each join is bitwise (`B` of one segment equals `A` of the next, by serialised
bytes), so the shared end is one frame (Theorem 4). One `s` per frame, per segment (Part 3). A path's frame is addressed by its
segment and its place in it, `(k, u)`, as Part 3's frame grid is, never by one fraction of the whole path: `k/n` is
not exact in f64 for most `n`, and a keyframe's frame would then come from the segment before it's arithmetic.

**A saved chart's view is not edited in stage 1.** Save view as chart gives a view whose map is the saved chart, its
definition by value (an oblique one's `z₀`, `q₁`, `q₂`) beside the window in `SimConfig`; an edit of that view would
leave the two disagreeing, and no engine route edits one today (its tilts compose in the kernel, `Composed`; editing
and saving again are TASK-M2-55's and TASK-M2-63's). So every window edit, a slider move, a lock and saving again on it
are refused (`NotBuilt`).

**A keyframe segment** (`path_keyframe_bitwise`). There is no keyframe interpolator on `main`, so nothing to reproduce.
In stage 1 a keyframe pair that differs in several fields is a chain of single-family segments, each against its direct
operation; Part 3's several fields eased together on one `s` is the typed `WindowEase` arm, `NotBuilt`.

**The record.** A frame is recorded by value as `(A, B, family, s)`, `s` as its f64 bits, `A` and `B` whole views
(§ 17.5), with a byte serialisation that round-trips bit for bit: JSON with every float written exactly and `s` and a
saved chart's words as IEEE bits in hex, not the canonical JCS text (R-309), because JCS writes `−0` as `0`. Optional fields, absent unless set (so
absent ⇒ today's bytes): the alignment `T`, the retime table's hash, and, under a lock, the anchor's words. The easing is
not in it: timing changes no frame's content.

**Jumps under a lock are cuts, labelled.** Until stage 2's sliding anchor exists (R-849), every edit that jumps the view
while locked (Home, the resets, presets, steers, capture, a chart switch) is a `Cut` carrying `CutLabel::JumpUnderLock`
("a cut under the lock: the anchor does not slide until stage 2"). This PR states the non-conformance with R-849 (no
jump under a lock) until then. Those jumps are GUI routes today (below), so stage 1 builds the label and the
constructors; the GUI starts using them in the consolidation task (TASK-M2-64).

**What stage 1 does not build (the fence, R-863).** The state blend (`Blend`, the morph, its default path `block line`
and any other path it names), `PlaneRotation`, `WindowEase`, R-474's tilt family (`R474Tilt`, R-474's Tip as
sheet and Lean each axis through the kernel's `Tilted`), axes and primitives, pair maps, holds, retiming, alignment and
the stored anchor and pivot are typed in § 2 and return `Err(NotBuilt(..))` until their stage. **The blend names its
path and assumes none** (R-943): `Blend.path` is a `BlendPath` enum whose only variant today is `BlockLine`, and nothing
matches on it outside the blend's own evaluator.

**What exists today, and the rule applied to each operation.** An engine route is re-expressed as a constructor and
then deleted; a kernel-only route gets a thin constructor tested against the direct kernel call; an operation with
neither is a sketch with one "the plan is unclear" comment.

| Operation | Route on `main` | Stage 1 |
|---|---|---|
| Pan | `engine/src/nav/mod.rs` `pan` | `Segment::pan`; `nav::pan` deleted |
| Zoom | `nav::zoom` | `Segment::zoom`; deleted |
| Slice | `nav::slice` | `Segment::slice`; deleted |
| Tilt (affine) | `nav::ChartConstructor::{tilt, tilt_pair}` | `Segment::raw_tilt` / `raw_tilt_pair` on `path::Constructor`; `tilt_tangent` and `tilt_vector` stay as the family's formula |
| Tilt (curve axis) | `lock::curve_tilt` (a vector) | stays in `lock`, reading the tilt formula `path::tilt_tangent`; no family |
| Tilt (R-474's maps) | kernel `latent_basis`, `Tilted<C, P>`; the GUI's compass; no engine route | sketch (`R474Tilt`) |
| Lock | `engine/src/lock.rs` `set`, `set_on_chart`, `recentre`, `snap_back`, `unlock`, `move_slider` | `path::lock`'s constructors (`Cut` with its label; `Control`); the old functions deleted; stage 2: the lock moves no view |
| Oblique slice | `nav::oblique` (a plane, not an edit) | `path::oblique`, the same function, a constructor of a view's plane; as an edit, a `Cut`; deleted from `nav` |
| Chart switch | none in the engine (a `SimConfig` has no chart id) | `Segment::cut` |
| Save view as chart | kernel `Saved::latent`, `Saved::curved` | `Segment::commit`, the kernel call unchanged |

**The GUI's own routes are not these families.** The GUI does not call `engine::nav` or `engine::lock`: it has copies
with other formulas (`gui/src/explore/manifold_view/navigate.rs` `pan`, `zoom_by`, `zoom_to` (a ratio of scales, not
`2^−octaves`), `pan_by_keys`; `manifold_view/lock.rs` `lock_at`; `explore/compass.rs` `slice` (`z₀ + amount·n`, no
`off_plane`) and `tilt` (through `Angles` on a `SliceFrame`)). Stage 1's families will not carry them bitwise as they
stand, so the consolidation task (TASK-M2-64) either adds arms for them or takes a recorded bit change.

**Chart maps and decode stay in `kernel`.** The object holds kernel chart values and calls `Φ` and `D`; it changes no
kernel bit.

## 2. The module

```rust
// crates/engine/src/path/mod.rs — sketch. `✓` = built in stage 1; `sketch` = typed, `Err(NotBuilt)` until its stage.

use crate::contract::sim_config::{Latent, SimConfig};
use crate::nav::BasisVector;
use kernel::chart::saved::Saved;

// ── The view: a point in chart space ───────────────────────────────────────────────────────────────────────────────

/// A view: a map seen through the window its `SimConfig` holds. ✓ (Latent, Kernel, Saved); sketch (Composed, Blend).
pub struct View { pub map: Map, pub sim: SimConfig }

/// The map, a chart: named coordinates to a signed state. Maps are kernel values; the object never re-implements Φ.
pub enum Map {
    Latent,                       // ✓ Φ = z₀ + (2s−1)q₁ + (2t−1)q₂ (kernel::chart::Latent), window = sim.plane
    Kernel(KernelMap),            // ✓ a registry chart: its REGISTRY index only (see § 1), window = sim.chart.window
    Saved(Saved),                 // ✓ a saved chart (kernel::chart::saved::Saved), by value
    Composed(Composed),           // sketch: axes x primitives x pair map x holds (stage 3, TASK-M2-66)
    Blend(Box<Blend>),            // sketch: the state blend of two views (stage 3+)
}

/// The window (c, Q), f64, read from the view. ✓ (Plane, Chart); sketch (General, a rotated window on a kernel map).
pub enum Window<'a> {
    Plane(&'a Plane),             // latent: c = z₀, Q = [q₁ q₂]
    Chart(&'a ChartWindow),       // kernel map: c = centre, Q = diag(half)
    General { c: [f64; 2], q: [[f64; 2]; 2] },
}

// ── Paths ──────────────────────────────────────────────────────────────────────────────────────────────────────────

/// One edit: two views, a named family, its alignment (content) and its timing. Fields private: built by constructors.
pub struct Segment { a: View, b: View, family: Family, align: Option<Align> /* sketch */, timing: Timing }

/// A chain of segments, each join bitwise (Theorem 4). ✓
pub struct Path(Vec<Segment>);

/// The named families. Each interior frame calls the direct operation's own function at the scaled parameter.
pub enum Family {
    Pan { ds: f64, dt: f64 },                                    // ✓ nav::pan
    Zoom { octaves: f64 },                                       // ✓ nav::zoom
    Slice { d: Latent, t: f64 },                                 // ✓ nav::slice (along off_plane(d))
    Control { k: usize, from: f64, to: f64 },                    // ✓ lock::move_slider (z₀[k] assigned)
    RawTilt { committed: Plane, form: TiltForm },                // ✓ R-474's raw tilt from the committed plane
    Cut { label: CutLabel },                                     // ✓ no interior
    Commit,                                                      // ✓ Save view as chart: same pixels, another map
    WindowEase,                                                  // sketch: Part 3's fields eased together on one s
    R474Tilt { from: kernel::chart::tilt::TiltSpec, to: kernel::chart::tilt::TiltSpec }, // sketch: Tip as sheet, Lean each axis; latent and curved
    PlaneRotation,                                               // sketch: § 17.7's geodesic between two planes
    Blend(BlendPath),                                            // sketch: the state blend, path named
}

/// The raw tilt's two forms, each its own route. Targets are the segment's, the same at both ends. ✓
pub enum TiltForm {
    Single { which: BasisVector, from: f64, to: f64, target: Latent },             // ChartConstructor::tilt
    Pair { from: [f64; 2], to: [f64; 2], targets: [Latent; 2] },                    // ChartConstructor::tilt_pair
}

/// Why a cut is a cut. ✓
pub enum CutLabel {
    ChartSwitch,      // today's chart switch: a jump (model § 3)
    Lock,             // lock::set, set_on_chart: the lock group and the pin, the view unchanged (stage 2)
    LockRecentre,     // recentre, and a lock with the centre-on-pin setting on
    CentreOnPin,      // the pinned pixel brought to the centre
    SnapBack,         // z₀ ← z_locked, by assignment
    Unlock,           // the lock group cleared; the view unchanged
    JumpUnderLock,    // Home, resets, presets, steers, capture while locked: non-conformant with R-849 until stage 2
}

/// A blend's path, by name; nothing outside the blend's evaluator assumes `BlockLine` (R-943). sketch
pub enum BlendPath { BlockLine }

/// § 17.10's alignment T, in the window's similarity group: content, read by `frame`. sketch
pub struct Align { pub rotation: f64, pub offset: [f64; 2] }

/// A segment's timing (R-901): when frames are shown, never what they show. ✓ (easing); sketch (retime)
pub struct Timing { pub easing: Easing, pub retime: Option<RetimeHash> }

/// Monotone, e(0) = 0 and e(1) = 1 exactly. The offered curves and the default are TASK-M8-31's (REQ-TOOL-197). ✓
pub enum Easing { Linear, Table(MonotoneTable /* knots pinned at (0,0) and (1,1), strictly increasing */) }

impl Segment {
    /// The frame at path parameter s: s == 0.0 → a, s == 1.0 → b (stored, never computed), else the family's H_s. ✓
    pub fn frame(&self, s: f64) -> Result<View, PathError>;
    /// The frame shown at playback fraction u: `frame(easing(u))`. ✓
    pub fn at(&self, u: f64) -> Result<View, PathError>;
    /// The record of the frame at s, by value. ✓
    pub fn record(&self, s: f64) -> FrameRecord;
}

/// `(A, B, family, s)` by value, s as f64 bits; canonical bytes that round-trip bit for bit. Optional fields absent
/// unless set, so absent ⇒ the stage-1 bytes. Reading rebuilds the segment by its constructor and refuses a B that
/// differs. ✓ (the four; the optional fields' slots); sketch (their values)
pub struct FrameRecord {
    pub a: View, pub b: View, pub family: Family, pub s_bits: u64,
    pub align: Option<Align>, pub retime_hash: Option<u64>, pub anchor_words: Option<[u64; 15]>,
}

// ── The lock ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// The lock: today's group in `View.sim.lock` (✓), plus the stored anchor, the pin and the pivot (sketch, stage 2 /
/// TASK-M2-62, TASK-M2-63). Not a separate path: its edits are segments (`Cut`, `Control`).
pub struct LockExtra { pub anchor: Option<Anchor>, pub pin: Option<Pin>, pub pivot: Option<Pivot> }

/// The anchor as stored words: the signed starting state (R-867), (m, r, p): 3 + 6 + 6 f64 words. sketch
pub struct Anchor { pub ic_words: [u64; 15] }

/// The pin's screen position: stored, or assumed at the centre where none can be derived (R-899 Q4). sketch
pub enum Pin { Stored([f64; 2]), AssumedCentre }

/// A curved chart's pivot record, stored as the pinned state (R-899 Q5, untilt (a1)). sketch
pub struct Pivot { pub state_words: [u64; 15] }

// ── The map side (all sketch; stage 3 and later) ──────────────────────────────────────────────────────────────────

pub struct Composed { pub axes: [Axis; 2], pub pair: Option<PairMap>, pub holds: Vec<Hold> }
pub struct Axis { pub primitive: Primitive, pub warp: Warp }
pub enum Primitive { Latent(usize), Energy, Lz, Kinetic, Time /* … the registry, TASK-M2-66; time, stage 5 */ }
pub enum Warp { Linear, Log, Power(f64) }
pub enum PairMap { Polar, LogPolar }                     // offered as maps, never as morphs (R-936)
pub enum Hold { Energy(f64), Lz(f64), K(f64) }           // TASK-M8-118's and TASK-M2-69's
pub struct Blend { pub a: View, pub b: View, pub path: BlendPath, pub holds: Vec<Hold>, pub s_bits: u64 }
```

**Errors.** `PathError::{Nav, Lock, NotBuilt, WrongMap, Join, Rebuilt, Parameter, Easing, Refused, Record}`: a family
given a view of the wrong map is refused, never coerced; `Rebuilt` is a record whose `B` its constructor does not reach.

## 3. The great circle of the default path's shape block, and its two degenerate cases (REQ-CHART-213; for physics review)

The block line takes the shape block on the great circle from `n̂_A` to `n̂_B`. It is degenerate at both ends of the range
of `c = n̂_A·n̂_B`: where `n̂_B = n̂_A` the direction `ŵ` is `0/0`, and where `n̂_B = −n̂_A` the circle is not unique. The
definition, in R-82's pattern (one deterministic rule, a pure function of the pixel, its special pixels declared):

- **The general circle.** `w = n̂_B − c·n̂_A`, `θ = atan2(‖w‖, c)` (§ 17.7, not `arccos`),
  `n̂(s) = cos(sθ)·n̂_A + sin(sθ)·ŵ`, `ŵ = w/‖w‖`. `‖w‖` is computed scaled (its largest component factored out, as
  `hypot`), so it underflows to 0 only where `w` is 0. It applies on the same side (`c ≥ 0`) wherever `‖w‖ > 0`, and on
  the far side (`c < 0`) wherever `‖w‖ > √ε_64` (`ε_64` the f64 machine epsilon, `√ε_64 ≈ 1.49·10⁻⁸`).
- **The same side** (`c ≥ 0`, so `c = 0`, the equilateral-to-collinear morph's orthogonal ends, takes the general circle,
  `‖w‖ = 1`; physics review of `d87578b`). Near `n̂_B = n̂_A`, `ŵ`'s relative error `ε/‖w‖` is multiplied by `sin(sθ) ≈ s‖w‖`, so the
  frame's error stays about `ε`: the physics review measured it at or below 2.6·10⁻¹⁶ against a 50-digit reference
  at every separation from 10⁻⁶ down to 10⁻¹⁷, wherever `‖w‖ > 0` (review 6002173203). So the general circle is taken
  there, and only at `w = 0` exactly (with `c ≥ 0`), where `ŵ` is `0/0`, does the frame hold `n̂(s) = n̂_A` for `0 < s < 1`, the end rule
  giving `n̂_B`, which is then `n̂_A` to rounding. Never NaN; not a seam. *Revision 3 held `n̂_A` for all
  `‖w‖ ≤ √ε_64`, a step of up to `√ε_64` at `s = 1`, not of rounding size (review 6002173203).*
- **The antipodal tie** (`c < 0` and `‖w‖ ≤ √ε_64`). Here `ŵ`'s error is not damped (`sin(sθ) ~ 1` mid-path), so the
  threshold is needed. The side the tie falls on: the threshold is inclusive.
  `ê` is the coordinate axis of the shape sphere's own `(n₁, n₂, n₃)` with the smallest `|n̂_A·ê|`, the lowest index on a
  tie; `k̂ = (n̂_A × ê)/‖n̂_A × ê‖` (`‖n̂_A × ê‖ ≥ √(2/3)`, well conditioned); `n̂(s) = cos(πs)·n̂_A + sin(πs)·(k̂ × n̂_A)`.
  It depends on `n̂_A` alone, so on the pixel alone. Every pixel taking it is flagged `antipodal_tie`, a declared seam of
  the blend (model Theorem 10). It is not symmetric under reversal: the reversed segment `B → A` takes the circle of
  `n̂_B`, a different one; harmless for playback, and declared. At a tie pixel the step between `s = 1 − ε` and `s = 1`
  is up to about `√ε_64`, not the rounding size of § 17.4: it lies inside the declared seam.
- `c = 0` is on the same side: for unit `n̂_A`, `n̂_B` it has `‖w‖ = 1`, so the general circle. The tie stays `c < 0`,
  `‖w‖ ≤ √ε_64`. The ends are untouched (the end rule).

This definition is written into design § 17.2 and the export contract Part 3; the great circle and its seam flag
are lowered by TASK-M2-81.

## 4. The tests stage 1 owes

`path_pan_bitwise`, `path_zoom_bitwise`, `path_tilt_bitwise`, `path_slice_bitwise`, `path_lock_bitwise`,
`path_chart_switch_bitwise`, `path_save_view_bitwise`, `path_presets_bitwise`: each operation through the object against
a baseline recorded from today's code on its own commit before the re-expression (sim-key bytes, IC words, and an image
hash of a small CPU field). Each family's interior frames against its direct operation at the scaled parameter.
`path_keyframe_bitwise` (a chain of single-family segments), `path_ends_bitwise`, `path_record_round_trip`,
`path_easing_timing_only`, `path_concat_join`. Controls: a floating-point operation reordered in a family; the general
geodesic in the tilt family's place; a path triple in a zoom's sim key; an end through the family's arithmetic; `s` at
f32; an easing applied to a frame's content; a second easing beside the segment's.
