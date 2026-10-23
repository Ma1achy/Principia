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

Keyframes are unchanged in *what they describe*, changed only in *how frames are produced*:

- **What interpolates:** `z₀` eased componentwise; zoom **log-lerped**; basis pairs by **Grassmannian interpolation** (geodesic between planes, re-orthonormalised per frame — never naive vector lerp, which leaves the manifold of frames); slice values eased; render uniforms lerped per their scale metadata; chart *switches* are cuts or dissolves (two renders composited), never parameter interpolation across chart types.
- **What each frame costs:** classified per Part 2 — segments that hold the sim key still are type (a) or (c); segments that move it are type (b).
- **The spec object is the artefact.** A keyframe animation serialises as `{keyframes: [(t_wall, ViewState, RenderState, playhead policy)], segments: [(easing, transition)], fps, duration}` — provenance-complete, URL-encodable, re-renderable. The video is its shadow (Part 6).
  **Easing is set per segment, between keyframes (R-576).** There is one segment between each pair of neighbouring
  keyframes, each with its own easing curve, which gives that segment's progress `s ∈ [0, 1]` at each frame for every
  eased field, and its transition at a chart change, a cut or a dissolve with its length. The playhead policy is one of
  hold, advance with wall time, or set to a value. The curves offered, the default curve and the playhead policies'
  semantics are a definition (R-72, REQ-TOOL-197), written here by TASK-M8-31. The screen that makes a spec is
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
    second curve, so the zoom's anchor never moves against the pan inside a segment. Each keyframe's own frame takes
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
    until `f_{k+1}`, RQ-561's interim (`principia_spotlight_reel.md` §3.3; REQ-GUI-383); which value it holds there is
    RQ-561's second question. Live playback other than after a reel takeover (the preview included, wherever it
    starts) is as above, so a preview started mid-segment shows keyframe `k`'s value where a takeover at that frame
    would hold the eased one (`principia_sequence.md` §3.6) (*was:* "Ordinary live playback, entering the segment at a
    keyframe's frame, is as above."; fix round 9 of PR #286, gui review 5631987230, G3). A
    dissolve's outgoing state follows the same rule. No such field is eased: easing one would be a new family of ICs
    of the kind RQ-384 needed a ruling for.
- **The decoder in a keyframe (R-483, X7).** A keyframe's `ViewState` is its `SimConfig`, in the one canonical
  serialisation, links included (`principia_gui_state_contract.md` §2; no second copy, X1). A change of a link, or of a
  link parameter, between keyframes is a sim-key change at that keyframe: a cut, as a chart switch is, in live
  playback and wherever a link id changes.
- **Link parameters interpolate in a blocking export only, flagged (R-520).** Between two keyframes whose link ids are
  the same and whose link parameters differ, a blocking export (Part 4) eases each differing parameter linearly in its
  value (`μ_max`, `q_max`) and re-integrates every frame at its own values. The segment's own `easing` curve (R-576)
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

---

## Part 4 — Export: the frame loop in blocking mode

Export runs the **same frame loop** as interactive playback with **one policy change: a hard barrier over *all* visible quads per captured frame.** No moving fallback, no blur, no progressive fill in exported pixels — a frame is captured only when every visible quad is at the frame's `t` and at the frame's required refinement. ("Exported frames are fully converged" survives, restated for lockstep: fully caught-up *and* fully refined per the export quality setting.)

- **Blocking is acceptable because the consumer is a file.** Latency doesn't matter; correctness does. The stall a hard barrier would inflict on a user is simply the export job taking its time.
- **The job is non-blocking to the *UI*** (unchanged from the old contract): it runs as a background job with progress %, ETA, and a live thumbnail of the last captured frame; cancellable; **streamed encode** (frames handed to the encoder as produced — never accumulate raw frames in memory; the no-history discipline applies to the exporter too).
- **Export quality tier** may exceed the interactive tier (more samples/quad, deeper refinement, full E — motion gating does not apply, scheduler Part 9) — priced into the estimate. Samples/quad is a sim-key difference; depth and `E` are not on the sim key (R-89).
- **Checkerboard is force-off in export, regardless of the user setting** (`principia_checkerboard_contract.md` §3/§6): every captured frame computes all visible pixels at one `t` — the strict single-playhead invariant holds unconditionally here. (The motion-time levers in general — checkerboard, E-reduction, coarse refinement floor — are interactive concessions; export takes none of them.)
- **Resolution/fps/duration** are job parameters; the exporter renders offscreen at target resolution, independent of the window.
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
