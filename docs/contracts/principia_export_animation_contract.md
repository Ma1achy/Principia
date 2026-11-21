# Principia — playback, export & sharing contract

*Rewritten under the temporal-architecture reversal (ratified): the scrub/checkpoint model is gone; **playback is the temporal mechanism**, driven by the frame loop (scheduler Part 7). This contract covers the transport controls, the two playback types and their costs, the keyframe system, the export job (the frame loop in blocking mode), determinism, sharing, and the spotlight reel. What was removed: per-pixel `n(t)` replay from checkpoint knots, the interpolate-and-renormalise knot policy, "payloads are time-complete", scrub as a render-key operation.*

---

## Part 1 — Playback is the temporal mechanism

Time in Principia is a **live march**, not a stored dimension. The global playhead advances by fixed `dt` (frame loop); every visible pixel's `SimState` marches with it; the fragment stage colours the current state. There is no scrub — the playhead is a clock, not a slider over stored data. That applies to exported animations only: the dev GUI's time scrubber sets the display time and re-integrates progressively, and never replays stored frames (R-66).

**Transport controls (free from statelessness):**

| Control | Semantics |
|---|---|
| **play / pause** | pause freezes the *playhead*, not the compute: the live set holds coherently at the frozen `t`; navigation still works (reveals catch up to the frozen `t` in the background and promote) |
| **restart** | playhead → 0, live-set states discarded and re-decoded. Cheap — there is no history to manage |
| **loop** | auto-restart at `t_end` (the horizon `T`, or when every visible sample has latched terminal) |
| **speed** | a `dt`-per-wall-second setting read by the GUI's clock — never tied to device frame rate (determinism, Part 5) |

*12 Nov 2026 (R-926 item 5; the chart audit's F16):* a restart's "re-decoded. Cheap" reads as re-evaluated on the CPU, priced by the cost model's CPU-evaluation term (`principia_sequence.md`'s estimate). *Was:* the row as written.

Transport is `ViewUI` state (`principia_gui_state_contract.md` §2): not undoable, not on the sim key — changing it never re-integrates (R-96). The GUI's clock reads it and advances `RenderState`'s playhead each frame through a `SetField` marked "no history", so playback never enters undo; a manual scrub is one coalesced entry (R-101).

**Playback under navigation.** Pan/zoom during playback reveals quads that catch up `0 → playhead` (or resume from a cached state — caching Part 7) off-loop, shown blurred via the moving fallback (their live ancestor, animating), promoting at a barrier when synced. The screen is never mixed-time, never frozen, never lying (frame loop, three-failure-modes rule). Late-playback reveals cost more than early ones — the honest price of the moment being asked for — but the cost is background compute, never a stall.

**"Pause at `t_end`" equivalence.** Playing to the horizon and pausing performs exactly the integration the old eager system performed up front for every quad — so worst case ties the old design on compute while beating it on memory; every earlier frame is strictly cheaper.

---

## Part 2 — The two playback types and their costs

**(a) Time-playback at a fixed view — the crystallisation movie (the hero, and the cheap one).** View parameters fixed; only the playhead advances. One march of the viewport, `0 → T`, rendering (or capturing) every frame *along the way*. Total cost = **one full integration of the visible set** — you cannot animate time more cheaply than computing it once. Basins emerge out of churn as samples latch terminal; the temporal refinement accumulators sharpen boundaries as the structure declares itself (scheduler Part 8).

**(b) View-animation — keyframes over `(z₀, basis, zoom, chart, render params)`.** When a *sim-key-touching* parameter interpolates per frame (z₀ slices out of the plane, the basis tilts or rotates, the chart morphs — in-plane pan and zoom only re-address, R-92), **each frame is a different family of ICs** — so each exported frame requires its own blocking march `0 → t_frame` of that frame's viewport. This is inherently per-frame-expensive — and it was *equally or more* expensive under the old model (which integrated each frame's ICs to full `t_end` before colouring). Lockstep's per-frame cost `0 → t_frame ≤ 0 → t_end` is a strict improvement.

**(c) Render-only animation.** Keyframes that touch only render-key parameters (palette sweeps, slot uniforms, overlay fades) at a fixed view and fixed playhead recolour the current states — free, no integration, exactly the sim/render split.

Mixed animations decompose per keyframe segment into (a)/(b)/(c); the export planner prices each segment accordingly and reports the estimate before the job runs.

---

## Part 3 — The keyframe system (retained)

*7 Nov 2026 (R-850, ruling RQ-721 (a), amended):* a chart change may morph, live and in export, as one arm of the
chart-path type of `docs/design/principia_chart_composition.md` § 17; cut and dissolve are still offered, a changed link
id is still a cut (R-520), and the lines below that say a chart switch is only a cut or a dissolve give way. TASK-M2-60
carries the change into this Part, REQ-TOOL-075 and REQ-GUI-376.

*15 Nov 2026 (TASK-M2-60, closing REQ-CHART-112):* carried. The ruling, as R-925, R-926, R-936, R-943 and R-949 read
it, is now this Part's body: **the chart path** below defines the path; a chart change's transition is a cut, a
dissolve or a morph; and **a morph's frames are recorded** says what each frame carries. A changed link id is a chart
change here and may morph (R-949), so the note's "a changed link id is still a cut (R-520)" no longer holds. Each line
this replaces keeps its *Was*.

Keyframes are unchanged in *what they describe*, changed only in *how frames are produced*:

- **What interpolates:** `z₀` eased componentwise; zoom **log-lerped**; basis pairs by **Grassmannian interpolation** (geodesic between planes, re-orthonormalised per frame — never naive vector lerp, which leaves the manifold of frames); slice values eased; render uniforms lerped per their scale metadata; a chart *switch* is a cut, a dissolve (two renders composited) or a morph (the state blend of **the chart path** below), never parameter interpolation across chart types. *Was:* "chart *switches* are cuts or dissolves (two renders composited), never parameter interpolation across chart types" (replaced by R-850).
- **What each frame costs:** classified per Part 2 — segments that hold the sim key still are type (a) or (c); segments that move it are type (b).
- **The spec object is the artefact.** A keyframe animation serialises as `{keyframes: [(t_wall, ViewState, RenderState, playhead policy)], segments: [(easing, transition)], fps, duration}` — provenance-complete, URL-encodable, re-renderable. The video is its shadow (Part 6).
  **Easing is set per segment, between keyframes (R-576).** There is one segment between each pair of neighbouring
  keyframes, each with its own easing curve, which gives that segment's progress `s ∈ [0, 1]` at each frame for every
  eased field, and its transition at a chart change, a cut, a dissolve with its length or a morph with its path
  (`cut`, `dissolve { length }` or `morph { path, retime, align }`, design § 17.13; *was:* "a cut or a dissolve with its
  length", replaced by R-850). A changed link id is a chart change here (R-949). **A new chart-change segment is a
  morph** on every pair of ends (R-850, R-1028), cut and dissolve still offered; a saved spec's segments reopen as
  saved (R-484), so a saved cut stays a cut. *17 Nov 2026 (R-1028, RQ-830 Q1 (a)):* *Was:* "**A new chart-change
  segment is a morph** where the ends allow (R-850; which ends allow is RQ-830)". `path` names the segment's path (**the chart path**
  below), the default by its name, `block line` (design § 17.9). `retime` is on or off: on, the blend's path parameter
  is `g(s)`, `g` design § 17.9's table from arc length to path parameter, pinned at `g(0) = 0` and `g(1) = 1`, so the
  blend moves at constant visual speed and the easing keeps the timing (REQ-CHART-116). `align` is design § 17.10's
  alignment of an unlocked path, reaching the identity at `s = 1` (REQ-CHART-116). The playhead policy is one of
  hold, advance with wall time, or set to a value. The curves offered, the default curve and the playhead policies'
  semantics are a definition (R-72, REQ-TOOL-197), written here by TASK-M8-31. *8 Nov 2026 (R-901):* easing and the
  path curve are a property of each path segment, shared by live morphs, saved paths and export. Where a segment's
  path is one path segment, the segment's easing is that segment's timing; where a keyframe pair is joined by several
  path segments (a tilt, then a morph), each carries its own timing, concatenated, and REQ-TOOL-197's one `s` per frame is
  read per path segment. The screen that makes a spec is
  `principia_sequence.md` (R-577). *Was:* "A keyframe animation serialises as `{keyframes: [(t_wall, ViewState,
  RenderState, playhead policy)], easing, fps, duration}`" (stale value, replaced by R-576).
  **The frame grid, the progress and the playhead (fix round 6 of PR #286, physics review 5631678451, P1; applied per
  R-369; fixed points of REQ-TOOL-197's definition, which TASK-M8-31 completes with the curves and the default).**
  - *Frames.* Frame `f = 0, 1, 2, …` is at wall time `f / fps` from the spec's start. A keyframe's **own frame** is the
    frame nearest its `t_wall`, `f_k = ⌊t_wall_k · fps + ½⌋` in f64, so a `t_wall` off the frame grid (a drag sets it
    continuously, `principia_sequence.md` §3.2) still has one frame. The keyframes' own frames must increase strictly:
    the exporter and the reel build refuse a spec in which two keyframes share a frame, naming both, and the Sequence
    keeps a dragged keyframe's frame strictly between its neighbours' and refuses an fps that would give two keyframes
    one frame, saying which. Frames before the first keyframe's frame take the first keyframe's state; frames after the
    last keyframe's frame take the last keyframe's state, the playhead under its policy.
  - *The segment-local fraction.* Segment `k` runs from frame `f_k` to frame `f_{k+1}`, and at frame `f` in it
    `u = (f − f_k) / (f_{k+1} − f_k)`, the quotient of two integers in f64, so `u` is exactly 0 and 1 at its ends. The
    frame two segments share, `f_{k+1}`, belongs to the segment it starts (`u = 0` there); the last keyframe's frame
    belongs to the last segment (`u = 1`). Either way it is that keyframe's own frame.
  - *One `s` per frame.* `s = ease_k(u)`, the segment's own curve (R-576), one value per frame, drives **every**
    interpolated field of the segment: `z₀` componentwise, `a + s·(b − a)`; the zoom, log-lerped,
    `exp(ln a + s·(ln b − ln a))`; the basis, at parameter `s` along the Grassmannian geodesic; the slice values; the
    render uniforms per their scale metadata; and R-520's link parameters. No field takes `u` directly and none has a
    second curve, so the zoom's anchor never moves against the pan inside a segment. *15 Nov 2026 (R-850, R-901;
    design § 17.9):* one exception: in a morph segment with `retime` on, the blend's path parameter is `g(s)`, the
    one second curve; every other field of the segment keeps `s`, and `g` changes no frame's content, only when it
    is shown (the chart-space model's Theorem 5, REQ-CHART-160). Each keyframe's own frame takes
    that keyframe's `SimConfig` and `RenderState` bit for bit, every field, as R-520 F2 has it for the link parameters,
    so a manifest's hash at a keyframe's frame is the keyframe's.
  - *Linear is offered.* The curves offered include **linear**, `s(u) = u`; the others and the default are the
    definition's.
  - *The playhead* (`RenderState`'s `Playhead { t }`) is not eased by the render-uniform rule: over segment `k` it
    follows keyframe `k`'s policy, `p_k` being keyframe `k`'s playhead. **Hold**: `p_k` at every frame of the segment.
    **Advance with wall time**: `p_k + r_k · (f − f_k) / fps`, where `r_k`, in units of `t` per wall second, is stored
    in the spec with the policy, as `advance { rate }`, set when the policy is chosen from the Transport's speed at that
    moment and editable (the Transport's `speed` is `ViewUI`, which the engine never reads, so the spec carries its
    own). **Set to a value**: the playhead is eased, `p_k + s · (p_{k+1} − p_k)`, keyframe `k`'s playhead being the
    value set on it. Every frame's playhead is clamped into `[0, T]`, `T` that frame's horizon (`SimConfig`'s
    `Horizon { t }`), which inside segment `k` is keyframe `k`'s, by the next bullet, and from keyframe `k + 1`'s own
    frame keyframe `k + 1`'s; an advance that reaches `T` stays there for the rest of the segment. After the last
    keyframe the last keyframe's policy runs on, as REQ-GUI-383 has it. (*Was:* "Every frame's playhead is clamped into
    `[0, T]`, `T` that frame's horizon (`SimConfig`'s `Horizon { t }`); an advance that reaches `T` stays there for the
    rest of the segment."; fix round 7 of PR #286, physics review 5631849506, P5.)
  - *A field that does not interpolate* (fix round 7 of PR #286, physics review 5631849506, P5; applied per R-369, the
    corpus's cut at the second keyframe): every field this Part does not list as interpolated, the Run lane's occupant,
    horizon, `E` and thresholds (`principia_sequence.md` §3.3), the chart and the link ids, and, in live playback, the
    link parameters (R-520 F3), takes keyframe `k`'s value at every frame `f_k ≤ f < f_{k+1}` and keyframe `k + 1`'s
    from its own frame, as the decoder bullet's "a cut" and R-520 F3's "a cut at the second keyframe" have it. **One
    exception, which this bullet does not decide** (fix round 8 of PR #286, physics review 5631902011, P6; applied per
    R-369): after a reel takeover inside the segment, the live player instead holds the takeover's link-parameter value
    until `f_{k+1}` and cuts there (R-624, closing RQ-561: the interim is the rule, and the difference from the film
    after the takeover frame is accepted; `principia_spotlight_reel.md` §3.3; REQ-GUI-383). *Was:* "RQ-561's interim
    (…); which value it holds there is RQ-561's second question." Live playback other than after a reel takeover (the preview included, wherever it
    starts) is as above, so a preview started mid-segment shows keyframe `k`'s value where a takeover at that frame
    would hold the eased one (`principia_sequence.md` §3.6) (*was:* "Ordinary live playback, entering the segment at a
    keyframe's frame, is as above."; fix round 9 of PR #286, gui review 5631987230, G3). A
    dissolve's outgoing state follows the same rule. No such field is eased: easing one would be a new family of ICs
    of the kind RQ-384 needed a ruling for. *15 Nov 2026 (R-850, R-925):* in a morph segment the chart and the link
    ids are not cut: every frame strictly between the two keyframes' own frames shows the blend chart at that frame's
    path parameter (**the chart path** below), which eases no chart parameter and no link function, only the two end
    charts' states; each keyframe's own frame stays its keyframe's, bit for bit.
- **The decoder in a keyframe (R-483, X7).** A keyframe's `ViewState` is its `SimConfig`, in the one canonical
  serialisation, links included (`principia_gui_state_contract.md` §2; no second copy, X1). A change of a link, or of a
  link parameter, between keyframes is a sim-key change at that keyframe. A changed link id takes the segment's
  transition, a cut, a dissolve or a morph, as any chart change does, in live playback as in export (R-850, R-949); a
  changed link parameter is the next bullet's. *Was (15 Nov 2026, TASK-M2-60):* "a cut, as a chart switch is, in live
  playback and wherever a link id changes." (replaced by R-850 and R-949).
  *11 Nov 2026 (R-949, amending R-520 F1 and R-483 X7):* a change of a link id may morph, as a chart switch may (R-850):
  the morph blends the two charts' states and interpolates no link function. It is still a sim-key change at that
  keyframe, and a cut where the segment is a cut. *Was:* "and wherever a link id changes", an unconditional cut.
- **Link parameters interpolate in a blocking export only, flagged (R-520).** Between two keyframes whose link ids are
  the same and whose link parameters differ, a blocking export (Part 4) eases each differing parameter linearly in its
  value (`μ_max`, `q_max`) and re-integrates every frame at its own values. **The segment chooses** (R-623, closing
  RQ-560, amending R-520): each such segment records `interpolate` (the default) or `cut`; under `cut` the blocking
  export cuts at the second keyframe as live playback does, and the flag (REQ-TOOL-185) names only the segments that
  interpolate. The segment's own `easing` curve (R-576)
  gives the segment's progress `s ∈ [0, 1]` at each frame, as for every other eased field; the value is `a + s·(b − a)`, clamped
  into `[min(a, b), max(a, b)]` so that rounding never puts a frame past an end or outside the parameter's allowed
  range, and each keyframe's own frame takes that keyframe's value bit for bit, so its sim key is the keyframe's
  (applied per R-369, physics review P1 of PR #265). It is a type (b) segment, each frame a different family of ICs (Part 2),
  priced before the job runs. Live playback never interpolates them: there the change stays a cut at the second
  keyframe. The export is flagged: the planner names each such segment before the job runs ("link parameters
  interpolated: every frame a different decoder"), the exported artefact carries the flag beside its embedded spec,
  listing the segments and the parameters, and each frame's own record holds that frame's values in its `SimConfig`.
  *Was:* "a cut, as a chart switch is, never an interpolation. Whether link parameters may be interpolated across
  keyframes in a blocking export, re-integrating every frame, is queued (RQ-384); meanwhile they are cut." (stale
  value, replaced by R-520). *Was* (before R-576): "The spec's `easing` curve gives the segment's progress" (stale
  value, replaced by R-576).
- **The chart path (R-850, closing RQ-721; R-925; R-926 items 1 and 3; R-936; R-943; REQ-CHART-112).** The definition
  the build tasks cite; `docs/design/principia_chart_composition.md` § 17 is its design and its measurements.
  - *A view is a point, an edit a path.* A view is a point in chart space: a chart, a map from named coordinates to a
    system state and a time, composed with a flat window of a centre and a basis (R-926 item 1). Every view edit is a
    path through that space, and one path abstraction serves them all: tilt, zoom, chart switch, lock, Save view as
    chart and morph (R-850). **A path is window edits plus the state blend.** A pan, a zoom, a slice, a rotation and a
    tilt are edits of the window on one map, a window turn on one map being a tilt (R-925, R-926 item 1); only a change
    of map is a blend. Where a path is a special case, that operation's existing code evaluates it, as an exact
    specialisation (R-850).
  - *The morph.* "Morph" names the state blend only (R-925). A morph moves each pixel's system from its system on chart
    A to its system on chart B along a path, and three things define it (R-943): **the pairing**, the same pixel on
    both charts; **the path**; and **the easing**, its timing and nothing else. Both ends are exact: at `s = 0` every
    pixel is chart A's system and at `s = 1` chart B's, bit for bit (R-926 item 3). The ends never run the blend: `s` is
    compared exactly, `0.0` taking A's own path and `1.0` B's, never left to arithmetic, since `a + (b − a) ≠ b` on 23%
    of f64 pairs (design § 17.4; *15 Nov 2026, physics review 5912914414 of PR #550*). A blend is a chart (R-925): every
    feature that works on a chart works on a frame of it, area statistics the one exception, none taken on a mid-morph
    frame and the two end charts keeping their own (R-936, R-944); where a blend folds there is no true copy, and the
    backwards answer is the nearest one, labelled (R-936).
  - *The default path* is the shortest in each block's natural geometry (R-943), run block by block on each end's own
    signed state (design § 17.2), with `s` the path parameter:

    ```
    mass      μ_k = ln(m_k/m₀)   a + s(b − a)                  the mass coordinates, the logs of the mass ratios (R-936)
    shape     n̂                  the great circle, n̂_A to n̂_B   stays on the shape sphere, signed; no reprojection
    momentum  q                  a + s(b − a)                  body frame
    time      t                  a + s(b − a)                  (R-926 item 3)
    ```

    Its name, in a spec, a planner row and a frame's record, is `block line` (design § 17.9). **The mass block's
    saturation** (R-975, closing RQ-787 (a); design § 17.11, "Out of feasible", case 2; *15 Nov 2026, Codex review of
    PR #550*): where the blended masses raise `M01_TINY` between two valid ends, the pixel saturates and is never
    blank: it holds the blended `m₁ : m₀` and lifts the light pair just to the allowed minimum, `M₀₁ = ε_M01`, the rest
    of the blend unchanged, a valid system, continuous in `s` and across pixels and equal to the plain blend where the
    dip begins and ends (R-943); R-936's log-ratio path stands for every pair, and how the lifted state sits against
    the guard, which fires strictly below `ε_M01`, is the building task's definition under physics review.
    **The great circle's tie** (design § 17.2): at `n̂_B = −n̂_A` the great circle is not unique; those pixels are a
    declared seam, generically isolated, a curve only for a symmetric pair of charts, and the tie is broken by a pure
    function of the pixel in R-82's pattern. Which function is a definition the corpus does not give (R-72): RQ-832.
    *16 Nov 2026 (applied per R-369):* RQ-832 is closed as R-72's, from the physics review of PR #550 (review
    5912994647, A1): the definition is REQ-CHART-213, to be written by TASK-M2-61 under physics review.
    *21 Nov 2026 (TASK-M2-61, physics review 6002173203 of PR #24):* written, in design § 17.2, "The great circle's two
    degenerate cases, defined": the general circle `atan2(‖w‖, c)` wherever `‖w‖ > 0` on the same side, `c ≥ 0`,
    and `‖w‖ > √ε_64` on the far side, `c < 0`; equal ends (`w = 0` with `c ≥ 0`) holding `n̂_A`; and the tie at `c < 0`, `‖w‖ ≤ √ε_64` (inclusive),
    the circle through the coordinate axis least aligned with `n̂_A` (lowest index on a tie), a pure function of the
    pixel, flagged `antipodal_tie`, a declared seam.
    **The time block and the playhead:** how the time row meets the playhead policies of this Part (hold, advance with
    wall time, set to a value) in a morph segment is not settled: RQ-831. (*15 Nov 2026, physics review 5912914414 of
    PR #550, P3 and P4.*)
  - *Another path* is allowed if both ends are exactly the end charts, every in-between frame is a valid system and it
    is continuous; a path is the owner's choice, not physics (R-943). Only the default is built now; none is designed
    out: the morph transition names its path, and nothing assumes the default.
  - *Not morphs.* Design § 17.2's other arms, the per-axis blend (`AxisBlend`), the pair-map bend (`PairMap`) and the
    per-field path (`PerField`), are retired as morphs and offered as maps (R-936); its tilt, zoom and plane-rotation
    arms are window edits (R-925).
  - *Two tests per operation (R-850).* Run through the path abstraction, each operation (1) reproduces today's
    behaviour bitwise, and (2) reproduces its own path family's formula, never compared across families: a tilt
    against R-474's formula, a zoom against the view's own, a morph against the block line, the default path above
    (design § 17.3). Two families may join the same ends by different paths (the chart-space model's Theorem 3), so
    their gap is expected, not a tolerance. *Was (15 Nov 2026, physics review 5912914414 of PR #550, P1):* "(the latent
    line, the latent plane geodesic, the chart-parameter geodesic)", the owner's list in R-850, written before "blend in
    Z only" was withdrawn (RQ-736, R-853): the morph's family is the block line, which is not the latent line (design
    § 17.2), so testing a morph against the latent line would compare across families.
  - *In a segment.* A morph segment's two ends are its keyframes' views, each its chart in its window, paired pixel for
    pixel, under `align` as the spec bullet has it. At frame `f` its path parameter is `s = ease_k(u)`, the blend
    evaluated at `g(s)` with `retime` on; the render uniforms follow this Part's rules for every segment, and how the
    playhead's policy combines with the time block is RQ-831's. *Was (15 Nov 2026, physics review 5912914414 of PR
    #550, P4):* "the render uniforms and the playhead follow this Part's rules for every segment". It is a type (b)
    segment (Part 2): every frame a different chart, each needing its own march `0 → t_frame`.
- **A morph's frames are recorded (design § 17.5, § 17.9 and § 17.13; R-520 F5's pattern).** Each exported frame's own record
  holds that frame's own `SimConfig`: at a keyframe's own frame the keyframe's, and at a frame strictly between, the
  blend chart that frame shows, given by its two end charts, its path's name and its path parameter, with the hash of
  `g` where `retime` is on (a pure function of the two ends and the path, computed in f64 on the CPU, design § 17.9).
  The easing is not in it: timing changes no frame's content (REQ-CHART-160). *15 Nov 2026 (physics review 5912914414
  of PR #550, P2; design § 17.5):* recorded **by value**, `(A, B, path, s)`: `A` and `B` each the whole end chart and
  view, never a name; `s` the path parameter as its f64 bits, in the canonical serialisation (R-309), the eased `s`
  itself, with `retime` on recorded beside `g`'s hash and `g(s)` recomputed from them, never stored in its place; the
  alignment `T` (`align`, design § 17.10), since `B` is evaluated at `T_s p`; and, under a lock, the anchor's words
  (design § 17.8; R-902, the lock a property of every path). It round-trips per R-484. A frame of a segment that does
  not morph is a plain view, its `SimConfig` that view's. **A mid-morph frame never claims a held value** (R-850, the
  owner's amendment, item 3), save where its path holds it, as design § 17.11's table gives (R-965 item 4): a value
  on at one end only, or forbidden at either end, is not held and is labelled `hold off mid-path`. The planner's row for a morph segment
  names its path and carries the flag "a morph: every frame a different decoder", and the exported artefact carries
  the flag beside its embedded spec, listing the morph segments and their paths, as R-520's flag lists its segments.

---

## Part 4 — Export: the frame loop in blocking mode

Export runs the **same frame loop** as interactive playback with **one policy change: a hard barrier over *all* visible quads per captured frame.** No moving fallback, no blur, no progressive fill in exported pixels — a frame is captured only when every visible quad is at the frame's `t` and at the frame's required refinement. ("Exported frames are fully converged" survives, restated for lockstep: fully caught-up *and* fully refined per the export quality setting.)

- **Blocking is acceptable because the consumer is a file.** Latency doesn't matter; correctness does. The stall a hard barrier would inflict on a user is simply the export job taking its time.
- **The job is non-blocking to the *UI*** (unchanged from the old contract): it runs as a background job with progress %, ETA, and a live thumbnail of the last captured frame; cancellable; **streamed encode** (frames handed to the encoder as produced — never accumulate raw frames in memory; the no-history discipline applies to the exporter too).
- **Export quality tier** may exceed the interactive tier (more samples/quad, deeper refinement, full E — motion gating does not apply, scheduler Part 9) — priced into the estimate. Samples/quad is a sim-key difference; depth and `E` are not on the sim key (R-89).
- **Checkerboard is force-off in export, regardless of the user setting** (`principia_checkerboard_contract.md` §3/§6): every captured frame computes all visible pixels at one `t` — the strict single-playhead invariant holds unconditionally here. (The motion-time levers in general — checkerboard, E-reduction, coarse refinement floor — are interactive concessions; export takes none of them.)
- **Resolution/fps/duration** are job parameters; the exporter renders offscreen at target resolution, independent of the window.
- **`render_scale` = 1, and the colour-vision simulation off unless chosen (R-582).** The exporter renders at the export's own resolution whatever the view's `render_scale`, as the checkerboard is forced off, and never applies the viewer's colour-vision simulation (`ViewUI.cvd`) unless the user picks "As seen with", which is recorded (`principia_export_details.md` §3.2). Every measured colour window is frozen at the window on screen when the export starts (R-581, §3.1), and every mark length is in base px, scaled by `output_width / base_width` (R-583, §3.3).
- **The encoders (R-131).** Native: PNG frames, GIF, and MP4 through a system `ffmpeg` when one is present. Browser: PNG frames (zipped), GIF via a wasm encoder, and MP4/WebM through WebCodecs where supported. GIF is one encoder in both builds: the Rust `gif` crate (MIT or Apache-2.0), with `color_quant` for palettes, compiled to wasm for the browser (R-155).

---

## Part 5 — Determinism & reproducibility

Fixed `dt`, deterministic schedule, and **branch decisions pinned by the comparison-only rule** (integrator contract — bucket lookups against the sim key's f32 thresholds, generated on the host and shipped bit-exact, R-502, not runtime-computed on the device; *was:* "against frozen f32 constants") ⇒ **the same spec object re-renders the same video** on the same backend, and matches across backends up to the parity contract's cross-backend f32 tolerances (values may differ at tolerance; branch decisions are identical on identical inputs, per step; terminal labels and outcome classes on chaotic trajectories may differ across precisions — `principia_parity_contract.md` governs, R-84). The exported artefact carries its full provenance (Part 6), so any figure or animation can be re-run and interrogated.

Frame-loop speed settings do not affect exported content — export ignores wall-clock pacing entirely and steps `dt` exactly per the spec object.

**The round trip (R-484).** "The same spec re-renders the same video" holds only if the spec read back is the spec
written: importing a spec object is the identity on each keyframe's `SimConfig` and `RenderState`, field for field and
bitwise in the canonical serialisation, the links and their parameters included (R-483). The spec is not a lossy path;
only an image's low-bit embedding degrades (`principia_dd_image_embedding.md` §6, "Tiers").

---

## Part 6 — Sharing: the spec is the object, the video is its shadow

Unchanged in principle, sharpened by lockstep: since there is no stored temporal data at all, **the only complete description of an animation is its spec** — `ViewState + sim key + render state + transport/keyframe spec`. Sharing ships that object (URL-encodable), its `SimConfig` and `RenderState` in their one canonical serialisation (`principia_gui_state_contract.md` §2, R-309); receiving it re-renders bit-comparable playback (Part 5). Exported videos embed (or link) the spec so provenance travels with the pixels.

**How a recording carries it (R-584; `principia_video_carrier.md`).** Always, and once per file, in the format's own metadata: a GIF's comment block and an application extension, an MP4's standard comment, a WebM's tag, each holding the compressed record; PNG frames keep the low-bit record in every frame; and the settings file is always written beside the export, since no metadata survives an upload to most websites. A frame's playhead follows from its index and the recording's settings.

**Receiving a spec (R-483, X4; R-484, RT1).** The spec carries, beside its state, the build hash and the schema version.
A spec naming a link the receiving build lacks, or one whose definition changed, is "state present, cannot recreate",
naming the link, and is never re-rendered under a default link (`principia_dd_image_embedding.md` §6, "The decoder's
identity"); one written before R-483, with `"links":{}`, re-renders under the receiver's defaults only if its build hash
is the receiver's, or on the user's explicit acceptance, labelled as assumed. Import is the identity on every
keyframe's `SimConfig` and `RenderState` (R-484), so receiving it re-renders what was sent.

---

## Part 7 — The spotlight reel (attract mode)

Retained, rationale unchanged: a pre-encoded video of curated spotlights covers cold-start (the one moment nothing can be live-rendered — the snapshot backdrop's degenerate case, caching Part 5) and serves as the gallery's ambient mode. Now *produced by* the Part 4 blocking exporter from a curated list of spec objects; each reel segment displays its spec's provenance (subtitle cycling) and clicking through **loads the spec live** — the reel is a menu of shareable objects, not just a film.

**R-571:** the reel's machinery (this Part; REQ-TOOL-084) is built and tested natively, in the native app and so in v1 web; its presentation, the spotlight reel that passes for Explore (`designs/reel_click.md`, in the gap sweep's bundle), is v2 web's.

**R-578: what the reel is, and what touching it does** (`principia_spotlight_reel.md`). The film is rendered once per
release, offline, by the Part 4 exporter (`cargo xtask reel`), and shipped as a video file with a manifest mapping
every film frame to its segment, spec, frame within the spec and the hash of the state it was rendered from; the app
only plays it, and the engine starts computing at the takeover. The reel plays inside the Explore view, looking like
Explore running; **any interaction**, not only a click, **takes over live at that exact frame** (the frame's spec,
state and playhead, the identity, R-484) **and keeps playing**, the film frame the snapshot backdrop until the live
render has resolved over it (`principia_caching_contract.md` Part 5). "Clicking through loads the spec live" above
stands, read so. The build refuses a spec whose link entry hashes do not match the build (R-521).

---

*Time is a march, not a store. Play is the mechanism; pause freezes the clock, not the machine; restart is free because nothing is kept. The hero movie costs one integration; view-flights cost their frames; recolours cost nothing. Export is the same loop with a hard barrier and a file for an audience. The spec is the object — the video is its shadow — and the same spec renders the same movie, anywhere, again.*
