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
- **The spec object is the artefact.** A keyframe animation serialises as `{keyframes: [(t_wall, ViewState, RenderState, playhead policy)], easing, fps, duration}` — provenance-complete, URL-encodable, re-renderable. The video is its shadow (Part 6).

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

Fixed `dt`, deterministic schedule, and **branch decisions pinned by the comparison-only rule** (integrator contract — bucket lookups against frozen f32 constants, not runtime-computed) ⇒ **the same spec object re-renders the same video** on the same backend, and matches across backends up to the parity contract's cross-backend f32 tolerances (values may differ at tolerance; branch decisions are identical on identical inputs, per step; terminal labels and outcome classes on chaotic trajectories may differ across precisions — `principia_parity_contract.md` governs, R-84). The exported artefact carries its full provenance (Part 6), so any figure or animation can be re-run and interrogated.

Frame-loop speed settings do not affect exported content — export ignores wall-clock pacing entirely and steps `dt` exactly per the spec object.

---

## Part 6 — Sharing: the spec is the object, the video is its shadow

Unchanged in principle, sharpened by lockstep: since there is no stored temporal data at all, **the only complete description of an animation is its spec** — `ViewState + sim key + render state + transport/keyframe spec`. Sharing ships that object (URL-encodable); receiving it re-renders bit-comparable playback (Part 5). Exported videos embed (or link) the spec so provenance travels with the pixels.

---

## Part 7 — The spotlight reel (attract mode)

Retained, rationale unchanged: a pre-encoded video of curated spotlights covers cold-start (the one moment nothing can be live-rendered — the snapshot backdrop's degenerate case, caching Part 5) and serves as the gallery's ambient mode. Now *produced by* the Part 4 blocking exporter from a curated list of spec objects; each reel segment displays its spec's provenance (subtitle cycling) and clicking through **loads the spec live** — the reel is a menu of shareable objects, not just a film.

---

*Time is a march, not a store. Play is the mechanism; pause freezes the clock, not the machine; restart is free because nothing is kept. The hero movie costs one integration; view-flights cost their frames; recolours cost nothing. Export is the same loop with a hard barrier and a file for an audience. The spec is the object — the video is its shadow — and the same spec renders the same movie, anywhere, again.*
