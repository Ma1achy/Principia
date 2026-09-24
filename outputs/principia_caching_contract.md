# Principia — caching & display continuity contract

*Eighth doc. Two halves that must be designed together: what the cache keys, keeps, and invalidates; and what the screen shows while the cache is catching up. The governing guarantee for the second half: **the canvas is never blank and never lies** — something always shows, and anything not current is visibly marked as such.*

---

## Part 1 — Two-level keying: identity vs validity

The spec has a quad address `QuadID (z, tx, ty)` (the spec's old name `TileID` is retired — 'tile' now means a sample's screen footprint, memory-tiers §1) and, separately, a payload compatibility signature described as "stricter than the quad cache key" — but the relationship was never nailed. The model:

- **Identity — *which region of which chart*.** `QuadID` alone is incomplete: `(z, tx, ty)` doesn't say which chart or which plane. Full identity is `(chart id + params, z₀, q₁, q₂) + QuadID`. Two quads with different bases are different identities even at the same `(z,tx,ty)` — an agent that keys on `QuadID` alone will cache-collide across charts and across tilt positions.
- **Validity — *computed under which physics config*.** The payload compatibility signature: chart/decode version, link ids, integrator occupant + config, horizon `T`, enabled metrics (tier flags), event thresholds, ensemble mode, payload schema version. Same identity, different signature → different payload (e.g. recomputed after a threshold change; preview vs refined).
- **Time — *the state's clock*.** Under lockstep a cached `SimState` additionally carries the `t` it had reached. It is directly presentable only at that `t`; for any later playhead it is a **resume point** — march `t_cached → playhead`, which is strictly cheaper than a `0 → playhead` re-boot. (Fixed-`dt` determinism makes resuming exact: the resumed state equals the never-evicted state.)

> **The word buffer is part of the payload.** The free-group word lives in a separate buffer (ledger §3.3a) but is logically one payload with its `SimState` — same identity, same signature, same `t`. Everywhere this contract says "sim buffers," it means **both** the `SimState` buffer and the parallel word buffer: they evict together, recompute together, resume together, and share the compatibility signature (the word depends on the same `IC, sim key` — notably event thresholds, since encounter detection drives symbol append). A `SimState` at index `i` and a word at index `i` must always describe the same trajectory at the same playhead; the two physical buffers never desynchronise.

Cache lookup = identity match **and** signature match (+ the entry's `t` as resume metadata). Identity says "the quad you're asking about"; validity says "computed the way you're currently asking"; `t` says "how far along it already is."

---

## Part 2 — What invalidates what (the dependency graph)

Every knob has a blast radius. Consolidated from the render, integrator, and scheduler contracts:

| Change | Blast radius |
|---|---|
| Render mode, palette, brightness binding, combiner, post, overlays | **Nothing.** Render key only — recolour existing buffers |
| Colour occupant params (κ, C, swatches…) | Re-bake the equirect texture (~ms). Sim buffers intact |
| Pan, slice, tilt, zoom, lock, chart-mode switch | **Nothing invalidated.** These change *which identities you're requesting*, not the validity of anything computed. Old-identity quads stay cached and valid — navigate back and they're still there |
| `T`, `dt_macro`, thresholds, `eps` floors | **All sim buffers** (signature change; the march re-boots from `t = 0`) |
| Integrator occupant | All sim buffers (signature) |
| Link function selection, chart/decode version | All sim buffers (signature — the ICs themselves change) |
| Quality tier | Sim buffers for the affected **sim-key** components only (E, FTLE, word, N — signature: tier flags; and `T` if preview). The tier's `render_scale` component invalidates nothing (row below) |
| `MAX_REL_DEPTH`, frame budget, in-flight limit | **Nothing.** Scheduler knobs — change what gets *scheduled*, never what anything computed *to* |
| `render_scale`, lock-to-native | **Nothing.** Not sim-key (payload purity — render resolution is not in the sim key): it moves the *refinement target* (screen floor at render-pixel size). Lowering leaves existing deep quads valid-but-deeper-than-needed; raising computes new deeper quads, masked by blur (quality/device note) |

The navigation row is the subtle one and follows directly from Part 4 of the chart contract: **tilting doesn't invalidate — it re-addresses.** A tilt gesture is a stream of new identities; the cache grows, nothing in it becomes wrong.

**The bake cache is a different animal entirely.** The equirect texture keys on `(colour occupant + params)` only — chart-independent, IC-independent, quad-independent. One texture serves every quad of every chart. It does not participate in the quad cache or its eviction; it has its own trivial lifecycle (rebake on param change, debounced).

---

## Part 3 — Cross-chart sharing: permitted, deferred

The same IC is reachable from many charts, and by the firewall (scheduler Part 1) its `SimState` is a pure function of `(IC, sim key, t)` — so payloads computed under one chart are, in principle, *correct* for any other chart that lands on the same IC. **IC-keyed (rather than chart+quad-keyed) caching is therefore permitted by the architecture.** It is deferred to v2 because the addressing is genuinely hard: floating-point `z` as a key, point-granularity vs quad-granularity mismatch, near-miss tolerance. v1 keys on `(identity, signature)` as above. The upgrade path stays open precisely because the firewall guarantees payload purity — nothing in v1 needs to change for v2 sharing to be sound, only the lookup.

---

## Part 4 — Baseline-first: an absolute tier, not a priority weight

**Refinement starts from the coarsest cover of the current viewport and works down. Always.** The spec has this as emergent behaviour (`ensure_baseline_tiles`, ancestor fallback, priority weights) — this contract hardens it into an invariant, because a weight can be starved:

> **Never dispatch a refinement job while the current view's baseline cover is missing.** Baseline dispatch is a hard tier above the priority queue, not a large weight inside it.

- **The baseline cover** = the coarsest quad(s) spanning the viewport at useful resolution near the camera depth. At deep zoom this is *not* the depth-0 domain root (uselessly coarse) — it's the viewport-covering quads a few levels above camera depth. Something low-resolution appears in one or two dispatches; refinement then proceeds beneath it.
- **The fallback chain is evict-exempt.** Cost-weighted LRU gets one hard exemption class: the baseline cover and its ancestor chain for the visible region, **plus the backdrop identity's leaf cover (Part 5)**, are pinned. Everywhere else eviction is free (recompute is safe, by payload purity); breaking the visible fallback chain is the one eviction that visibly hurts — it's the difference between "blurry for a moment" and "blank."

---

## Part 5 — The stale backdrop: blur means loading

When navigation changes identity (tilt, slice, chart switch), the previously displayed content is not *wrong* — it is correct data for an epsilon-nearby plane, and for a continuous gesture consecutive identities are close enough that the stale image is a genuinely good visual approximation. It should therefore **stay on screen, blurred**, until new-identity quads replace it.

**Mechanism: a two-tier backdrop — live stale layer first, snapshot as fallback.**

*Primary — the live stale layer.* The stale quads render through the normal pipeline from their cached `SimState`s — but under lockstep those states are **frozen at whatever `t` each had reached** when it left the live set (stale quads do not march; only the live set does — scheduler Part 7). So the backdrop is the last coherent identity's leaf cover, re-rendered every frame at the **current render config** but at each quad's **frozen time**, blurred, composited under the fresh-identity quads (painter's algorithm unchanged). This *strengthens* the blur semantics rather than weakening them: a blurred region is honestly not-current in space *and* time — sharp = live-and-in-time, blurred = stale-and-frozen, one signal.

- **It does NOT animate — and that is the semantics.** Stale quads are frozen at whatever `t` each had reached when it left the live set (they do not march; only the live set does — temporal note). The backdrop is a *still* of the old identity, breathing only in colour (next bullet). This reinforces the blur grammar rather than weakening it: blurred = stale-and-frozen in space *and* time. *(An earlier draft claimed the backdrop animates via per-pixel `t_end` replay — that was eager-model vocabulary; under lockstep there is no stored future to replay, and the frozen still is the honest display.)*
- **It re-colours.** Palette/mode changes mid-gesture apply to the backdrop too — colour stays *current data semantics* on slightly-old geometry. A snapshot would show a dead palette, colliding with colour-is-data.
- **Blur is a compositor pass, not a quad effect.** Blur is a neighbourhood operation, so it cannot run inside a quad's own fragment shader: the stale layer renders to a texture through the ordinary four-slot pipeline, a **separable blur** (two cheap passes) runs on that texture, then composite. This pins the placement: **the compositor sits above the four-slot pipeline** and owns layers (backdrop, fresh cover, overlays); each layer is produced by the fixed pipeline. Blur and CVD are linear and quads composite opaquely, so per-layer processing commutes with composition — no correctness wrinkle.
- **Coherence rule, sharpened:** the backdrop *reference* updates **only on an at-rest baseline (refined, non-preview)**. Mid-gesture coarse covers never become the backdrop — otherwise a long drag degrades the backdrop into a near-duplicate of the fresh coarse layer and the "blurred old detail → coarse live glimpses" staircase collapses. One stable backdrop identity per gesture; it swaps on gesture-end refinement.
- **Pinning & cost:** the backdrop identity's leaf cover joins the Part 4 evict-exempt class — exactly two identities pinned (current + backdrop), swapping atomically on at-rest update. Cost is one extra leaf-cover's fragment work plus the blur passes — bounded, trivial next to sim compute.

*Fallback — the snapshot.* When there are **no payloads behind the picture** — the spotlight-reel handoff (a video frame has nothing to live-render) or a cold cache — the original screen-texture snapshot applies, blurred, under the same coherence discipline. The snapshot is the degenerate backdrop, not the primary one.

**Blur only. No desaturation, no tinting.** Colour is data in this system: several modes carry information partly or wholly in chroma (greyscale-readable LUTs, monochrome debug views, spread/metric fields mapped to chroma). A desaturated backdrop would *collide with those encodings* — it would read as a false data statement under any chroma-carrying mode — and on monochrome views it does nothing, so it can't even mark staleness reliably. Blur is the one operation orthogonal to every colour encoding: it degrades spatial frequency, which no render mode uses as a data channel, and it universally reads as "loading." One marker, one meaning, zero collisions.

**Continuous gestures: the two-regime scheduler policy.** Dragging a tilt/slice/zoom/pan produces (for tilt and slice) a new identity per frame; no quadtree fills at 60 fps. The scheduler therefore has **two regimes, and the switch is a mode change, not a priority adjustment**:

- **In-motion (gesture active).** The goal flips entirely from *depth* to *coverage*. The only thing dispatched is the **full-canvas coarse cover** of the current identity — one or two dispatches' worth of quads, `PREVIEW_MODE`, a few levels above camera depth. **Complexity scoring is suspended, not down-weighted**: `P_complexity` is actively harmful mid-gesture — it would spend the tiny per-frame budget sharpening one interesting boundary quad instead of covering the canvas. The user's question mid-gesture is "what does this new slice roughly look like?"; the answer is a complete coarse picture, never a sharp fragment. Priority collapses to visibility × coverage.
- **At-rest (debounce fired).** The full machinery resumes: full `T`, complexity-driven refinement, baseline-first tier, normal priority weights.

Supporting mechanics:

- **Identity sampling, not tracking.** The in-motion regime samples the identity stream — coarse cover for the identity as of *now*; when that lands, for the identity as of *then* — while the blurred backdrop carries continuity between samples. The user sees: blurred old detail → coarse live glimpses updating every few frames → sharp refinement blooming on stop. That staircase *is* the "get the gist" experience.
- **Preview flotsam evicts first.** Intermediate coarse covers for identities the gesture sailed past go straight to the bottom of the LRU — below even smooth interior quads. Valid but preview-quality and for abandoned identities; without this rule the cache fills with drag debris.
- **Epochs do the cancellation.** Coarse-cover jobs for identity-at-frame-N that complete after the gesture has moved on are dropped (or filed as flotsam), never painted. Already in the scheduler contract; no mid-flight cancellation needed.
- **Zoom is the gentle case.** Zoom doesn't change identity (same `z₀`, same basis — scaled `q`), so ancestor fallback already shows the right content upscaled and the coarse cover mostly *exists in cache*. The regime applies uniformly, but zoom usually satisfies it for free; tilt and slice, which change identity per frame, are what genuinely need it.

**No contamination path — and nothing can create one.** The backdrop must never be readable as data, and structurally nothing reads it: the lock computes `z` from `(s,t)` via the *current* chart on the CPU (never from the screen), exports read payloads, the inspector integrates fresh. The backdrop — live layer reference or snapshot — is pure display state, cleanly on the "what we look at" side of the firewall. The hover trace obeys the same rule from the other side: it draws only from current-identity payloads — no payload under the cursor (backdrop showing through) means no trace.

**v2 polish, noted:** under a *locked* tilt, the anchor pixel's stale value is exactly correct (the lock is the fixed point of a basis edit) and staleness grows with distance from centre — so the honest stale rendering is a radially graded blur. v1 uniform blur is fine; recorded because it's a case of the navigation contract predicting display behaviour.

---

## Part 6 — The responsiveness invariant: the main thread never waits

A perfect two-regime scheduler is worthless if the main thread hitches for 40 ms mid-tilt. The invariant, in the same form as the others:

> **The main thread never waits on the GPU, on a readback, or on any scheduler work. Input → view-state update → uniform write → draw is the only synchronous path, and it is O(1). Everything else is asynchronous and yields.** *(This invariant is now realised structurally by the worker split in Part 6a — the main thread doesn't wait on scheduler work because it doesn't run scheduler work. The traps below are the ones that apply inside the loop wherever it runs.)*

WebGPU makes this mostly free *if not fought*: `queue.submit()` doesn't block, dispatches land whenever, `mapAsync` is promise-based. A gesture frame — **in the worker** (Part 6a) — is: read the input SAB, update `(z₀, q₁, q₂)`, write one uniform, draw the current leaf cover + blurred backdrop — microseconds; the main thread's entire contribution is having written the input delta. All compute is already decoupled: results land whenever they land, the painter's-algorithm compositor picks them up next frame. The contract's job is naming the five ways an agent will accidentally fight it:

1. **Synchronous readback.** Never `await onSubmittedWorkDone()` in the frame loop; never block on `mapAsync` before drawing. The `QuadReduction` readback (the sole *automatic* GPU→CPU crossing) is fire-and-forget: request the map, continue, consume in whatever later frame it resolves. The hover-trace sample readback (render contract Part 7) follows the identical discipline — one in flight, latest wins, never awaited.
2. **Pipeline compilation on the gesture path.** Module/pipeline creation can cost tens of ms (the driver turning SPIR-V into a native pipeline; and custom *fragment* WGSL additionally compiling from source), and a chart switch creates a different compute pipeline from its **pre-built SPIR-V** variant (lowering Part 4 — no runtime *source* compile on the compute side) on first use mid-gesture, hitching exactly when responsiveness matters most. Rule: `createComputePipelineAsync`/`createRenderPipelineAsync` always, and **precompile the variant set at startup** (the lowering table is finite: every chart type's compute variant, the render-slot combinations in use). Custom shader edits already have the async-compile + last-valid-pipeline fallback from the hot-reload design; that discipline is global.
3. **Unbounded CPU scheduler work per frame.** Collect-visible → score → top_k is fine at normal quad counts, but the f64 work — per-quad centre/half-width and especially the **linearised-decoder Jacobians** (two f64 decodes per axis per deep quad) — piles up when a gesture lands deep and dozens of quads need `x₀ + J_D`. Time-budgeting this on the main thread was tried and **failed under real load — the scheduler hitched the gesture and the DOM GUI froze.** The architecture answers it structurally (Part 6a): the scheduler runs in the worker, not on the main thread, so it *cannot* touch main-thread responsiveness. Within the worker it is still time-budgeted per frame (N quads, remainder deferred — per-quad independent, trivially incremental) so it never stalls the loop's own frame either.
4. **The snapshot copy.** The last-coherent-frame snapshot is a GPU-side texture-to-texture copy, cheap. Never implement it as readback-and-reupload — the snapshot never leaves the GPU.
5. **Bake uploads mid-gesture.** The equirect bake is debounced JS + `copyExternalImageToTexture` — already off the gesture path by the 120 ms debounce. Just never let a bake land synchronously inside the frame callback.

The structural point: the two regimes govern *what the GPU computes*; this invariant governs *what the CPU waits for*. They compose — and the reason a frame never has to wait is that **the compositor always has something legal to draw** (Part 5). The display-continuity guarantee is precisely what makes the non-blocking guarantee achievable; they are two faces of one design.

### Part 6a — The threading model: the render loop lives in a worker (and that worker is the wasm engine)

Non-blocking-on-one-thread is not enough. Under real load the CPU scheduler still stalls the gesture, and — the sharper failure — **the DOM GUI freezes**, because it shares the main thread with the scheduler and gets starved when scheduling spikes. The fix is structural, and under the substrate decision (`principia_spike_brief.md`) it is *doubly* structural: **the entire frame loop — WebGPU device, scheduler, cache, quadtree, all compute and render — is the wasm engine, running in a Web Worker via `OffscreenCanvas`. The main thread is a thin TS shell: an input pump and a DOM-GUI host, a *different binary*, and it owns no simulation state.** This is the wasm↔JS membrane (systems-architecture §3); the firewall is now enforced by the linker, not merely by discipline.

```
MAIN THREAD (TS shell)                   WORKER (wasm engine)
── owns the real <canvas>                ── owns the WebGPU device (drives wgpu directly)
   → transferControlToOffscreen() ─────▶ ── owns the OffscreenCanvas surface (transferred once)
── captures pointer/wheel/key/resize      ── runs the rAF frame loop
── runs the DOM GUI (was freezing)        ── integrates input → view state (z₀,q₁,q₂,zoom)
── owns NO sim state (cannot — diff binary)── the whole scheduler (collect/score/top-k, f64 Jacobians)
                                          ── all compute dispatches + render pass
        two channels ↓                    ── the cache, quadtree, ALL state
   INPUT/edits → SharedArrayBuffer / set_field (hot, low-latency, always-fresh)
   SNAPSHOT    ↔ postMessage (cold: GUI-sized state echo, click results)
```

Why the wasm engine owns **everything** rather than offloading just the scheduler piece: it avoids both traps that make WebGPU-in-worker painful, and the substrate makes both airtight. **(1) No GPU-command marshaling** — the engine owns the device, so dispatches never cross a boundary. **(2) No state mirroring** — all state (view, quadtree, cache, payload) lives in the wasm engine; the TS shell holds none of it, so there is nothing to keep in sync — and *cannot* hold it, being a separate binary (the boundary is a **data** boundary, not an object one: no wasm handles cross — GUI contract §1). The main thread is **provably** incapable of hitching on physics because it does not have — cannot have — the physics.

**Two channels, two rhythms:**

- **Input / field edits → `SharedArrayBuffer` (hot path).** The live pointer position and accumulated zoom/pan/tilt deltas live in a SAB both sides see; the TS shell writes the latest via `Atomics`, the engine reads it at the top of every frame. Discrete settings arrive as `set_field(path, value)` (GUI contract §1). **Zero message-queue latency and old input cannot pile up** — the engine always sees the freshest state, which is what makes the camera feel instant. (Coalesce pointer events into the SAB; do not post one message per `pointermove`.)
- **State snapshot ↔ `postMessage` (cold path).** The engine posts a **GUI-sized** snapshot for display — current auto-tier, fps, quad count, "loading" state, the read-only Custom fields showing what auto chose — **throttled to ~10 Hz, never per-frame**, and **never engine-sized** (the payload / quad tree / reductions stay wasm-side, summarised only — the big-data-never-crosses law). Click-to-inspect: the shell sends cursor coords, the engine does the fire-and-forget f64 re-integration (in the inspector worker — a second wasm context) and posts the result back.

**Input transport is one path with a boot-selected backend, not two architectures.** The worker/OffscreenCanvas split (and the wasm engine inside it) is unconditional. The *only* thing that varies is how input deltas reach the engine, decided once by a capability check at startup: **SharedArrayBuffer if cross-origin-isolated, postMessage-per-frame otherwise.** Both write into the same engine-side view-state integrator through the same interface — the scheduler and loop never know which backend is live. SAB is strictly the lower-latency option; the postMessage fallback coalesces input into one message per main-thread rAF tick (marginally higher latency, correct everywhere). Ordinary feature detection, not a forked codebase.

**Deployment for SAB.** SharedArrayBuffer needs COOP + COEP cross-origin isolation (`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: require-corp`/`credentialless`), and GitHub Pages serves no custom headers. **`coi-serviceworker`** (gzuidhof) supplies them: one self-hosted script beside `index.html`, same-origin (not a CDN), HTTPS (GH Pages is), registering a service worker that synthesises the headers with a single first-load reload. When it's active, the boot check selects SAB; when it isn't (service-worker failure, restrictive embedding, old browser), the same check selects the postMessage backend. No configuration, no second build.

**Browser support & debugging.** OffscreenCanvas + WebGPU-in-worker is solid in Chromium and improving in Firefox/Safari; worker contexts are fully debuggable (devtools attaches to the worker, breakpoints and `console` work). Resize is main-side (`ResizeObserver` on the element) → one message to the worker → swapchain reconfigure; never hot.

---

## Part 7 — The current-state cache (resume points, hard-capped)

What is cached under lockstep is **current state only — one timestep per quad, never trajectory, never a time-series.** The cache's job changes from "hold completed answers" to "hold resume points": a revisited quad resumes `t_cached → playhead` instead of re-booting `0 → playhead`.

- **Hard budget cap, not a smoothness dial.** The cache has a fixed memory ceiling, **sized by the device-characterisation phase** (`principia_quality_device_note.md`): a fraction of a detected VRAM budget (from `adapter.limits` + the boot probe), never a fixed constant — so it scales down on weak devices along with the compute settings. One characterisation sets the compute knobs *and* this cap together. The temptation to "cache a bit of history for smoothness" is exactly the eager-in-time siren that produced the V2 memory problem — the invariant is *one state per entry, bounded entry count*, and any growth pressure is answered by eviction, never by relaxing either.
- **Eviction stays cost-weighted LRU** and stays *safe* (fixed-`dt` purity: re-booting reproduces the state exactly) — but the cost model now includes `t_cached` (a deep-`t` resume point saves more recompute than a shallow one, so it resists eviction more).
- **The pinned classes survive unchanged**: baseline cover + visible ancestor chain + backdrop identity's leaf cover (Part 4/5) — pinned as *frozen states*, which is all the backdrop needs.
- **Refinement latches are not cache entries.** The "this quad proved interesting" decision is tiny CPU quad metadata that persists independently of state eviction (scheduler Part 8) — evicting a quad's state never forgets its refinement status.

## Part 8 — The composed guarantee

The four invariants interlock:

1. **Baseline-first** (Part 4) → something *current* appears within a dispatch or two of any navigation.
2. **Stale backdrop, blurred + two-regime scheduling** (Part 5) → something *approximately right and honestly marked* shows from the first frame, and a complete coarse picture of the new identity arrives before any sharp fragment.
3. **Purity + keyed caching** (Parts 1–3) → navigating back re-serves old quads instantly, eviction is safe everywhere except the pinned visible chain, and nothing stale can leak into any data path.
4. **The main thread never waits** (Part 6, realised structurally by the worker split in Part 6a) → the gesture itself never hitches, regardless of what the GPU is chewing on.

Net: **the canvas is never blank, never lies, and never freezes.** Blur is the single vocabulary item for "not current"; everything sharp is real, current data; and the hand on the slider always gets its frame.

---

*Identity says which quad; validity says computed how. Navigation re-addresses, never invalidates. Baseline is a tier, not a weight; the visible fallback chain is pinned. Stale content stays up, blurred — blur means loading, and it is the only thing blur means. Sharp means true.*
