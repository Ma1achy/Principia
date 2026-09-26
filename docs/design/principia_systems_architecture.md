# Principia — systems architecture

*The consolidated map. Everything here is already specified in the contracts — this document assembles the scattered pieces into one view. It is deliberately high-level: responsibilities, contracts, dataflows, hierarchy. Per-component drill-down files (with tests) come later. Among the detailed docs this is the one to read first — sitting just under `principia_canonical_spec.md`, the navigational layer that states the landed decisions and points here.*

*The architecture has three legitimate views, presented in this order: the **ladder** (the conceptual pipeline — what things mean), the **rings** (cross-cutting services wrapped around it), and the **membrane** (the deployment view — where things run). Earlier drafts led with the third; the first is the real hierarchy.*

---

## 0. The ladder — the organising abstraction

Data makes a one-way trip from a human intention to an image. Each rung transforms it and hands it down:

```
INTENT      navigation · lock · keyframes · playhead        (gestures & time)
                └─ constructs ↓
QUESTION    CHART — a projection of the 8D manifold;
            defines a *family* of ICs (each pixel one member)
                └─ per point: Φ ↓                     ▲ ENCODE — the door back in
MATTER      z → decode → canonicalise → (m, r, p)        (lock · lookup · literature ICs)
                └─ ↓
PHYSICS     SIMULATE — wrapper(occupant) · detect · classify
                └─ ↓
MEMORY      PAYLOAD — the recorded answer (SimState + ICDescriptor)
                └─ ↓
MEANING     COLOUR — the stain graph; colour *is* the data
                └─ ↓
IMAGE       COMPOSITE → SCREEN
```

Three facts of the ladder's geometry:

- **The waist is Memory.** Everything above it produces the payload; everything below consumes it. The payload is the one concept invariant across all three views.
- **The two keys are ladder geometry.** The **sim key** governs everything above the waist (change → re-integrate); the **render key** governs everything below (change → recolour). The recompute boundary is literally a horizontal line on the ladder.
- **Encode is the only upward edge** — the quotient map from the physical world back onto the manifold. The lock, IC lookup, and literature reproduction all enter through this one door.

---

## 1. The rings — cross-cutting services

**The test:** a *rung* is a stage data passes through and is transformed by. A *ring* is a system nothing passes through — it decides **how, when, how often, or how faithfully** the rungs fire, without ever changing what a given firing produces.

| Ring | Contents | Wraps | Service | Must guarantee |
|---|---|---|---|---|
| **Allocation** | scheduler · cache · quad pyramid · the two regimes · `MAX_REL_DEPTH` | Question → Memory | *which points on the manifold get evaluated, and when* — the chart defines the set; allocation chooses the sampling | **the firewall**: a payload is `f(IC, sim key)` regardless of when/whether/how-often scheduled; baseline-first is a hard tier; refinement density is never read as probability; in-plane navigation re-addresses; a new slice plane re-integrates (R-92); navigation never invalidates what is already computed |
| **Precision** | the CPU-f64 inspector/hover witness (the shared kernel at f64, in its dedicated CPU worker) · f32 GPU survey · linearised decode (`x₀ + J_D·δ`) · **the independent high-precision convergence reference** (a *separate* integrator, not the shared kernel — CPU arbitrary precision with convergence gating, double-double only as a fast screen (R-33), for the integration-floor falsifiability probe and Burrau ground truth; its independence is the point — fuller treatment in validation / the gap-hunt) | Matter → Memory | *how faithfully* the Matter and Physics rungs run — the inspector witness is the *same shared kernel* at higher precision (f64 or double-double), so its logic equality with the survey is **structural** (one source); the convergence reference is *deliberately independent* so it can catch shared-source bugs the parity path cannot. Numerics is the sole independent variable **for continuous values**; **branch decisions are held bit-identical across backends on identical inputs, per step (R-84), by the comparison-only rule** (`principia_gpu_determinism_note.md`) — a forked branch is a *different computation*, not honest divergence. **Divergence is exposed, never reconciled**; match-integrator mode for honest comparison |
| **Time & motion** | the frame loop (playhead) · keyframe interpolator · export job · spotlight reel | Question → Image | **the playhead is a live clock**: the frame loop marches the Physics rung by fixed `dt` and presents the barrier-synced live set (lockstep — temporal note; scheduler Part 7); **keyframes/export re-run the ladder per frame where the sim key moves** | no stored history — state is O(1) in time; exported frames fully caught-up and refined (blocking barrier, no fallback); the shareable object is the spec, the video its shadow; the job never blocks the UI |
| **Observation** | debug catalogue (field + cross-check views, passthrough modes) · hover trace · **sonification** · **telemetry / profiling** | every rung | a tap on each rung's output — *the display is the assertion*; each view certifies one producer or one seam | generated from the layout table, exhaustive by construction; observation never perturbs (taps read payloads and uniforms only). **Telemetry is FIRST-CLASS, not a debug mode** — the frame record and `stage_ms` are part of the render loop's contract, always present, with only the *reporting* toggleable. Instrumentation that can be compiled out will be, and then it measures the debug build (`principia_dd_telemetry_and_tiers.md` §5.5). **Hover trace and sonification are two projections of ONE integration**, not two subsystems (`principia_scratchpad_pointer_channels.md`, normative only where render_gui_spec, trajectory_viewing or a ruling cites it — R-96) |
| **Provenance** | ViewState / sim-key / link-id / animation-spec serialisation · sharing · gallery · **in-pixel embedding (LSB)** | every rung | records the ladder's entire configuration so any figure or animation can be re-run and interrogated | everything a result is conditional on **travels with it**; URL-encodable; re-renderable. **An image carries its own config in the low bit of every RGB pixel** — full slice, sim, colour mode and ramp window, plus a custom shader's source from 128² up (`principia_dd_image_embedding.md`). Max pixel delta **1 of 255**; survives crop, rotation, flip, PNG re-encode and alpha stripping. **Off by default for figure export** — a scientific figure must be exactly the pixels the renderer produced |
| **Deployment** | the CPU/GPU + wasm/JS membranes (§3) · lowering/`resolve()` · monomorphised compute + hand-WGSL fragment assembly (lowering Part 2) · precompile | maps the rungs onto hardware, and the engine onto wasm | *where each rung runs* — and the machinery that exists *only because* rungs are split across devices | see §3; a gesture can never reach the compiler; instant things precompile, re-integrating changes compile lazily inside their own cost |

Note the classification insight: **lowering belongs to the Deployment ring, not the ladder** — in a single-device world it wouldn't exist. And the scheduler is a ring, not a rung: no pixel's data ever passes *through* it; it only turns the crank.

---

## 2. Component inventory, by rung

| Rung | Components | Responsibility | Must guarantee |
|---|---|---|---|
| **Intent** | navigation (view state) · lock · keyframe/playhead state | owns `(z₀, q₁, q₂)` + slice values; pan/slice edit the centre, zoom/tilt edit the basis; the lock pins the centre | **navigation is chart construction** — every gesture is an edit to one uniform; no view/camera object exists apart from the chart |
| **Question** | chart system (authoring, validation, registry) — plus **encode** as the rung's upward door | charts as 2D projections through the 8D; four axis kinds; link registry per block; flags (`system_image`, feasibility, coupling, annotation). Encode: `physical → z`, the quotient onto the decode's section | every chart handed down is **well-posed** (8 DOF pinned exactly once); links constraint-preserving, invertible, C¹, carrying their measure. Encode: T1/T2/T3; rigid ops act on the **full state**; **encode reuses decode** for fibre choices; tolerances in physical units |
| **Matter** | link functions (generated Rust) · factorised decoder · canonicaliser | `z → mass × config × momentum → canonicalise → (m, r, p)` | the decoder factorises; no latent coordinate on a gauge direction; canonicalise is the one seam every input converges through |
| **Physics** | wrapper (loop, substep, project, monitor, detect, state readout) · occupant `ADVANCE` slot (R-19) · detectors | integrate to horizon; classify; pack | **one input type forever**; the integrator never learns the chart's name; occupants carry a capability profile; **wrapper *branch decisions* (`N_sub`, collision, terminal) are bit-identical across all backends on identical inputs, per step (R-84), via the comparison-only rule** (frozen threshold table, integer horizon, `d²`-comparisons — not runtime transcendentals; `principia_gpu_determinism_note.md` / integrator dd §3.3), and the wrapper loop uses the flag-in-condition/zero-break shape (integrator contract Part 1) that survives SPIR-V→WGSL; continuous values diverge freely; every pixel gets a labelled output (totality) |
| **Memory** | `SimState` (live, tier-sized, 8-aligned, 144/96 B eff (R-40 / D6) — hot per-step state) + a parallel **word buffer** (~16 B/copy, the cold per-crossing/resolve symbolic word (append on branch-cut crossing), indexed identically) + `ICDescriptor` (64 B, explicit padding, E₀ derived — R-86), GPU-resident; `QuadReduction` as its ~80 B summary | the marching answer — the live state at the playhead, O(1) in time (lockstep; temporal note). Hot state and cold word split by access pattern (word touched per branch-cut crossing + at resolve, never per-step) | pure: `f(IC, sim key, t)` under the fixed-`dt` march; the compute pipeline (monomorphised Rust → SPIR-V) and the fragment pipeline (hand-WGSL) meet **only** here — the payload is the sole interface between the two, and the split-mechanism boundary (lowering Part 2); leaves the GPU only via the sole automatic reduction or sanctioned pulls |
| **Meaning** | fragment pipeline (the stain — a free, typed node graph over a fixed `combiner` + `OUT` backbone, R-64) · baked-texture tier | payload → colour at the current playhead | the graph is data — nodes, wires and per-node params serialise with `RenderState` (gui_state_contract §5); wires are type-checked and acyclic; L-ownership; categorical values never averaged |
| **Image** | compositor passes (backdrop ▸ blur ▸ composite) ▸ display stages (style ▸ display scale ▸ gamut clamp ▸ colour-vision simulation — R-67) · screen | layers → final frame | sits **above** the stain graph; blur means exactly "not current" — nearby identity, behind the playhead, or still arriving (one grammar: sharp is real, fuzzy is arriving — temporal note); **never blank, never lies, never freezes** |

---

## 3. The membrane — the deployment view (demoted, not diminished)

**The CPU decides *what* gets computed and *when*; the GPU decides *what it computes to*.** This is the critical *implementation* invariant — the firewall, purity, and honest divergence all live on it — but it is a mapping of the ladder onto hardware, not the conceptual hierarchy. Rungs Intent and Question run CPU-side; Matter through Image run GPU-side; **rings run wherever they must and cross the membrane freely** (the Precision ring's witness is CPU; Allocation is CPU; Observation taps both sides).

The membrane is thin, typed, and enumerable — exactly five crossings:

| Crossing | Type | Direction | Nature |
|---|---|---|---|
| **Dispatch** | uniforms (`SimUniforms`, chart params, per-quad `c,h,x₀,J_D,T`, flags) | CPU → GPU | per frame / per quad; the only thing navigation touches |
| **Render config** | stain-node params · playhead `t` | CPU → GPU | the render key's path; never touches sim buffers |
| **Bake** | equirect texture | wasm engine → GPU | colour cache tier; chart- and IC-independent |
| **Reduction** | `QuadReduction` (~80 B/quad) | GPU → CPU | the **sole automatic** return; feeds the Allocation ring only |
| **Sanctioned pulls** | single-IC f32 GPU trace (click inspector's divergence overlay) · columnar decode (export) | GPU → CPU | user-initiated, tiny, async, latest-wins. (The hover trace is NOT a pull — it re-integrates on the CPU via `computeIC` in the inspector worker and reads no payload; render Part 7) |

The Deployment ring's machinery — `resolve()`, monomorphised compute pipelines (and hand-WGSL fragment assembly — lowering Part 2), the precompile rule — exists to serve this mapping: code differences baked, moving values as uniforms, and *a gesture can never reach the compiler*.

**There are now two membranes, and they obey the same law.** Under the substrate decision (`principia_spike_brief.md`: whole CPU engine in Rust → wasm, thin TS GUI shell) the CPU/GPU membrane above is joined by a **wasm↔JS membrane** — the boundary between the wasm engine (which owns *all* CPU-side state: scheduler, cache, quad pyramid, the Precision-ring reference, the physics) and the TS GUI shell (which owns *only* DOM and view-scratch state). This is the firewall (seam 9 / GUI contract §1) made physical: the engine and GUI do not share a linker, so the one-way rule is a fact of the binary rather than a lint. It is thin and enumerable in the same way:

| Crossing | Type | Direction | Nature |
|---|---|---|---|
| **Field edit** | `set_field(path, value)` — serialised data | JS → wasm | the GUI's only write; navigation, quality and the clock's playhead writes (marked "no history" — transport itself is `ViewUI`, R-101) all arrive as one of these |
| **State snapshot** | GUI-*sized* state (view state, tier, scalars the panels show) — serialised data | wasm → JS | throttled to **~10 Hz**, never per-frame (caching Part 6a); the GUI (egui) redraws at frame rate from the latest snapshot (R-94); **never engine-sized** (payload / quad tree / reductions stay wasm-side, summarised only) |
| **Canvas transfer** | `OffscreenCanvas` handle | JS → wasm | **once, at startup**; the engine then drives `wgpu` against it directly. The sole handle that crosses; no state crosses with it |

**The shared law on both membranes: big data never crosses.** The CPU/GPU membrane returns only the ~80 B `QuadReduction` automatically and sanctioned tiny pulls otherwise; the wasm/JS membrane carries only GUI-sized snapshots and single field edits. The payload, the tree, and the reductions are summarised across each boundary, never shipped whole — the *same* discipline stated twice, once per membrane. And the boundary must stay a **data** boundary, not an **object** one: the GUI holds no wasm handles, only the last snapshot (GUI contract §1 — wasm-bindgen makes handing JS a live Rust struct easy, and that would make the firewall decorative).

---

## 4. Dataflow

### The pixel's life = one ladder traversal

```
Intent ──constructs──▶ Chart ──validate──▶ resolve() [Deployment ring]
                                                │ dispatch ▼
      per quad:  (s,t) ─Φ─▶ z ─decode─▶ canonicalise ─▶ (m,r,p)
                                                │
                     wrapper( occupant ADVANCE → t_target ) · project · monitor · detect
                                                ▼
                          PAYLOAD  [the waist — GPU-resident]
                                                │  @ playhead t
                    stain graph: sources ─▶ colour / brightness ─▶ combiner ─▶ (post)* ─▶ OUT
                                                ▼
                blurred backdrop ▸ fresh cover ▸ trace/overlays ▸ style ▸ display scale ▸ gamut clamp ▸ CVD ▸ SCREEN   (R-67)
```

### The return paths

1. **Automatic:** `QuadReduction → Allocation ring` — coherence, impurity, spread, suspect fraction → split/keep/merge. Nothing else automatic ever comes back.
2. **User pulls:** click inspector → an on-demand single-IC f32 GPU trace (divergence overlay); export → columnar decode through the layout-table decoder. (Hover pulls nothing — CPU `computeIC` in the inspector worker.)
3. **Display only:** the screen texture → the backdrop's snapshot tier (never readable as data).

### The two keys as ladder geometry

```
SIM KEY      (above the waist)  chart id+params · slice plane (z₀'s out-of-plane part, span{q₁,q₂}, in-plane orientation — R-92) · warps · link ids ·
             occupant · T/dt/thresholds · tier (sim-key components: N/FTLE/word; E is live, cached per copy_index — R-89) · schema   ⇒ re-integrate (march re-boots; a tier's render_scale component invalidates nothing — caching Part 2)
RENDER KEY   (below the waist)  stain graph (nodes, wires, sources) · node params · overlays ·
             palette/compaction                               ⇒ recolour only
THE PLAYHEAD  is neither key — it is the live clock (frame loop): advancing it
             is sim work (the march); it never invalidates, it only progresses
NAVIGATION   in-plane pan/zoom ⇒ re-address which quads are asked for
             (revealed quads catch up to the playhead off-loop);
             slice out of the plane / tilt / rotate ⇒ a new slice plane (sim key) ⇒ re-integrate;
             lock ⇒ neither (R-92)
```

---

## 5. The seam catalogue

Each seam: the two parties, what crosses, and the single invariant that holds it. These are the contracts *between* components — most are **ladder seams** (between rungs); a few are **ring laws**.

| # | Seam | Crosses | The invariant |
|---|---|---|---|
| 1 | physics definition ↔ numerics *(Precision ring law)* | one source, compiled f64 + f32 | logic equality is **structural** (one Rust source); precision is the only difference in *continuous* values (exposed, never reconciled); **branch decisions are bit-identical across backends on identical inputs, per step** (R-84; comparison-only rule — `gpu_determinism_note`); the residual to test is backend miscompilation (`principia_parity_contract.md`) |
| 2 | chart → decoder | latent z (or physical via 8) | the decoder factorises `mass × config × momentum`; no latent coordinate on a gauge direction |
| 3 | decoder/canonicalise → integrator | `(m, r, p)` | **one input type forever**; the integrator is stateless and chart-blind |
| 4 | occupant ↔ wrapper | `ADVANCE(state, t_now, t_target, params)` (R-19) | the occupant is the only swappable line; the wrapper owns the target and project/monitor/detect, the occupant owns how it gets there; wrapper **branch decisions bit-identical across backends on identical inputs, per step** (R-84; comparison-only rule; loop is flag-in-condition/zero-break — integrator contract Part 1) |
| 5 | integrate → render *(the waist, above)* | typed payload | integrate/colour split: render changes never recompute; debug views are free |
| 6 | compute pipeline ↔ fragment pipeline | the payload buffer | the two meet **only** here — compute is monomorphised shared-Rust (parity-critical, single-sourced), fragment is hand-WGSL (parity-free, runtime devkit); different mechanisms by design (lowering Part 2) |
| 7 | stain pipeline ↔ compositor | layer textures | compositor above the stain graph; blur is a compositor pass; blur means exactly "not current" (spatially stale / temporally behind / arriving — one grammar) |
| 8 | physical world → manifold | `encode = inverses ∘ C` | quotient onto the section: rigid ops act on the **full state**; CoM subtracted, then scale rescaled (R-23); everything discarded is reported |
| 9 | Allocation ring ↔ the ladder *(ring law)* | dispatch decisions | the **firewall**: payloads are pure of all scheduling state; two users, different quadtrees, same answers |
| 10 | scheduler ↔ cache *(within Allocation)* | (identity, validity) keys | identity says which quad; validity says computed how; preview ≠ refined |
| 11 | adaptive structure ↔ quantitative claims *(ring law)* | — (deliberately no crossing) | adaptive is for **looking**; uniform grids + `|det J_D|` are for **measuring** |
| 12 | interaction ↔ GPU work *(Deployment ring law)* | field edits in / snapshots out; **the whole frame loop runs in a worker (OffscreenCanvas) — and that worker is the wasm engine** (§3 second membrane) | the main thread is a pure TS input pump + DOM-GUI host — it owns no sim state (it *cannot*, being a different binary), so it cannot hitch on physics or freeze the GUI; edits via `set_field` (SharedArrayBuffer where cross-origin-isolated, else postMessage), snapshots throttled; a gesture can never reach the compiler; the compositor always has something legal to draw (caching Part 6a) |
| 13 | layout table → {pack, unpack, export, catalogue} | generated code (Rust + WGSL) | **one source generates all four**, to both targets (Rust kernel/host, WGSL fragment); a field without metadata fails generation loudly |
| 14 | live instrument ↔ shared artefacts *(Provenance/Time rings)* | export job / spec objects | exported pixels are converged data only; the spec is the object, the video its shadow |

---

## 5.5 THE DISPATCH SHAPE — one thread per texel, one dispatch per ensemble copy

**Measured by the lowering spike, and it is a design decision the CPU never forced.**

At `N=8, E+1=8` a quad is **512 trajectories × 144 B = 72 KB** (recomputed at 144 B, R-40 / D6). Workgroup storage limits:

| | WebGPU | Metal |
|---|---|---|
| shared memory | **16 KB** | **32 KB** |
| invocations | **256** | 1024 |

**Nothing fits anywhere** — 4.5× over on WebGPU, 2.25× on Metal — and the two backends bind for
*different* reasons, so there is no single strictest one to design against. And it does not improve on
better hardware: **these are spec ceilings, not performance ones.** A 5090 has the same 32 KB per
workgroup as an M3.

### The shape

```
ONE WORKGROUP PER QUAD
  64 threads, one per TEXEL (N²)
  each ensemble copy is the SAME KERNEL DISPATCHED AGAIN, copy_index a uniform (R-102, R-89)
  the across-copy REDUCTION is its own RESOLVE PASS, after all E+1 dispatches (R-135)
```

**This makes the working set independent of `E+1` entirely.** The ensemble becomes a **time** cost
(8 copies are 8 dispatches, 8× as long) rather than a **space** cost — which is what you want, since `E+1` is a
quality knob and space limits are hard ceilings while time is a budget.

**The across-copy reduction is its own resolve pass (R-135).** It runs after all `E+1` copy dispatches for a quad
complete, reads their `SimState` slices, and writes the footprint resolve and the `QuadReduction` fields.

### Why parallelism is not lost

**A frame has hundreds to thousands of quads.** Dropping from 512 threads per quad to 64 means running
**8× more quads concurrently**. Total threads in flight is unchanged; the GPU does not care whether
occupancy comes from within a workgroup or across them. **Per-quad latency is 8× worse; frame
throughput is identical** — and latency only matters for the root quad, which is 8 copies of one
trajectory.

64 is also a good workgroup size on its own: two warps on NVIDIA, one wavefront on AMD, two SIMD
groups on Apple.

### And serial copies probably IMPROVE load balance

> **Was (R-102):** kept as history, not current design. It argued for copies serial within one thread; each copy is now
> its own dispatch (above) and the across-copy reduction its own resolve pass (R-135).

Trajectory cost is bimodal with a **~100× p1→p99 spread**. With copies **parallel across threads**, a
warp finishes when its *slowest* thread does — you pay the **max** of a heavy-tailed distribution.
With copies **serial within a thread**, each thread's cost is a **sum of 8 draws**, with roughly
√8 ≈ 2.8× lower relative variance.

**Max of a heavy tail is much worse than sum of a few.** The serial arrangement is likely better
balanced, not merely adequate.

### Not doing: worker tiles

The general form is `64/k` threads each handling `k` texels. **`k = 1` is the natural point** — `k = 2`
drops the workgroup below one wavefront on AMD, and going the other way needs shared memory for live
states, which is the original problem. A tiling knob would be a tunable with no measured need behind
it, and this project's history says that is a liability. **Revisit only if profiling shows a specific
problem** — the profiler is being built anyway.

### Still to check

**`N` scales too.** At `N = 16` a quad wants **256 threads**, which hits WebGPU's invocation ceiling
exactly. `N = 8` may be forced on WebGPU regardless — **measure it rather than assume**, since limits
in this area have already been misquoted once (a spec fetch gave 49152 for workgroup storage; measured
Metal is 32768, and Dawn runs on Metal, so the cited figure was impossible).

---

## 6. Cross-cutting invariants (the load-bearing walls)

*The architecture-level set. The complete enumeration (18 walls, with the per-doc pointers) is `principia_canonical_spec.md` §9; these ten are the ones this map most turns on.*

1. **The firewall** (seam 9): Allocation never reaches payload contents.
2. **Payload purity**: `SimState = f(IC, sim key)`. Buys free eviction, safe recompute, device-loss recovery, cross-chart cache sharing (v2).
3. **The sim/render key split** (seam 5): the recompute boundary — a horizontal line on the ladder.
4. **Navigation is chart construction** (Intent rung): all gestures are uniform edits; the lock is chart construction too (it lives in `SimConfig` and is undoable — R-69); the GPU has no modes.
5. **Never blank, never lies, never freezes** (Image rung + Deployment): baseline-first + blurred live backdrop + the frame loop in the **wasm-engine worker** (the main thread is a *different binary* — a TS shell owning only DOM — so it *cannot* hold sim state and the GUI cannot freeze; §3 second membrane, caching Part 6a). Blur is the single vocabulary item for "not current"; sharp means true.
6. **Measure honesty** (seam 11 + link registry): every arbitrary choice (link, warp, refinement density) either carries its Jacobian or is barred from quantitative claims; findings must survive link swaps and tilts to count as dynamics.
7. **Generate from one source** (seam 13): bit layouts, link inverses, debug catalogue, export decoder — never hand-duplicated; the generator now targets **Rust** (the kernel's pack/unpack + host) *and* **WGSL** (the fragment-side unpack + generated debug views), still from the one layout table.
8. **Totality** (Physics rung): every pixel gets a labelled output — a dynamical outcome (escape/collision/bounded — the last includes reaching the horizon; there is no separate timeout state, payload §2), a `sim_failed`/`decode_failed` terminal, or (under lockstep) `RUNNING` while in flight. Substep-cap saturation is a *confidence flag* on the eventual dynamical outcome, not a terminal label — the trajectory still reaches an outcome. Nothing is dropped; failure regions and low-confidence regions are both data.
9. **Shared-source logic equality, and branch determinism** (Precision + Physics rungs; the substrate walls): the CPU and GPU physics are **one Rust source** compiled twice, so logic/transcription drift is *structurally impossible* — the residual difference is precision (the honest signal) plus what each shader compiler does with the source (caught by parity, now re-aimed at **backend miscompilation**, `principia_parity_contract.md`). And **branch decisions** (`N_sub`, collision, terminal) are held **bit-identical across all backends on identical inputs, per step (R-84), by the comparison-only rule** (frozen threshold tables, integer counters, `d²`-comparisons — never a runtime transcendental; `principia_gpu_determinism_note.md`), because a forked branch is a *different computation*, not honest divergence — it breaks the CPU's standing as a check on the *same* run, and it injects non-physical noise into the very basin boundary Paper 2 measures. Continuous values diverge freely; branch words never.
10. **Two membranes, one law: big data never crosses** (Deployment): the GPU↔CPU membrane returns only the ~80 B `QuadReduction` automatically (+ tiny sanctioned pulls); the wasm↔JS membrane carries only GUI-sized snapshots and single `set_field` edits. The payload, the quad tree, and the reductions are *summarised* across each boundary, never shipped whole — one discipline stated twice — and each boundary is a **data** boundary, not an **object** one (no live handles across; §3).

---

## 7. Hierarchy and dependency (the build DAG, abstract)

Arrows read "requires". The *can't-exist-before* graph, not the milestone plan.

```
                    layout table ─────────────┐
                         │                     ▼
                   pack/unpack gen      debug catalogue gen
                         │                     │
   link registry ──▶ decoder (factorised) ──▶ compute KERNEL (Rust→SPIR-V) ◀── occupants + wrapper
        │                │                         │
        │           canonicalise ◀── encode        │
        │                │                         ▼
   chart system ──▶ validation ──▶ resolve/lowering ──▶ dispatch
        │                                              │
   navigation (uniform edits) ─────────────────────────┤
                                                       ▼
                                            payload (SimState/ICDescriptor)
                                                       │
                        fragment assembly (WGSL, per graph) ──▶ compositor ──▶ screen
                                                       │
                                    QuadReduction ──▶ scheduler ◀──▶ cache
                                                       │
                       inspector / hover (CPU f64+)  │  animation/export runner
                                    └──────── consume payloads + view state ────────┘
```

Reading it: the **layout table and link registry are roots** (everything generated flows from them); the **decoder is the first real artefact**; **canonicalise is the neck**; the **payload is the waist**; the **Allocation ring** sits beside the flow, deciding what flows, never what it contains.

### 7.1 Crate map

*R-170: drafted from §7 and R-146. **Confirmed by R-185**, with the kernel → ledger edge a build-dependency only.*

Each node of the graph above is assigned to one crate of the confirmed layout (R-146; there is no contract crate, R-172).
`cargo xtask deps` checks the workspace's crate graph against the allowed edges below (REQ-SYS-004). Edges *inside* one
crate (decoder before kernel, canonicalise before the integrator) are not visible to a crate-graph check; they stay a
code-review item.

| §7 node | crate | note |
|---|---|---|
| layout table | `ledger` | a root: no workspace dependency |
| link registry | `ledger` | a root |
| pack/unpack gen | `ledger` | the generator; its Rust output lands in `kernel`, its WGSL output in `render` |
| debug catalogue gen | `ledger` | the generator; its output (the debug fragment variants) lands in `render` |
| decoder (factorised) | `kernel` | shared CPU/GPU source |
| chart system | `kernel` | the chart maps and `validate(u, v)` (R-26) |
| canonicalise, encode | `kernel` | shared source; lookup and lock (CPU, `SimConfig`) are in `engine` |
| compute KERNEL, occupants + wrapper | `kernel` | compiled twice: f32 SPIR-V → WGSL, and native f64; `no_std`, so rust-gpu can compile it (R-185) |
| validation (the §7 node: chart-aware validation before lowering) | `engine` | not the `validation` crate, which is the test harness |
| resolve/lowering, dispatch | `engine` | |
| navigation (uniform edits) | `engine` | behind the typed surface (`crates/engine/src/contract/`) |
| payload (`SimState`/`ICDescriptor`) | `ledger` → `kernel` | the layout is the ledger's; the generated types are the kernel's; the buffers are the engine's |
| fragment assembly, compositor, screen | `render` | |
| QuadReduction, scheduler, cache | `engine` | |
| inspector / hover (CPU f64+), animation/export runner | `engine` | their windows are in `gui` |

Crates outside the graph: `gui` (the dev GUI: depends on `engine`'s typed surface only; nothing depends on it),
`validation` (the harness: may depend on any crate; others reach it only as a dev-dependency, R-176), `prin` (the CLI:
depends on `engine`), `xtask` (the runners: reads `cargo metadata`; no crate depends on it).

**Allowed workspace edges** (arrows read "depends on"):

| from | to | the §7 arrow it realises |
|---|---|---|
| `kernel` | `ledger` | layout table → pack/unpack gen → kernel; link registry → decoder. **Build-dependency only (R-185)**: the ledger generates code into the kernel at build time; a normal dependency on this edge is forbidden |
| `render` | `ledger` | layout table → pack/unpack gen (WGSL), debug catalogue gen |
| `engine` | `ledger`, `kernel` | kernel → dispatch; chart system → validation → resolve/lowering |
| `engine` | `render` | payload → fragment assembly (the frame loop and dispatch drive the fragment side) |
| `gui` | `engine` | GUI → state → engine (gui_state_contract §1) |
| `prin` | `engine` | |
| `validation` | any of the above except `gui` and `prin` | the harness exercises each seam; where it needs the CLI it runs the built `prin` binary as a separate process (R-187) |
| any except `gui` (dev-dependency only) | `validation` | R-176, R-187. Never a normal or build dependency, so the no_std kernel and rust-gpu builds never see it. In `kernel` and `ledger`, a test that uses `validation` is an integration test (`tests/`), not a unit test in `src/`, because the dev-dependency cycle would give unit tests two copies of the crate; `cargo xtask deps` enforces it |

The kernel is `no_std` (R-185). Every other workspace edge is forbidden; in particular `ledger` depends on nothing, `kernel` on nothing but `ledger` (and that only as a build-dependency) — normal and build dependencies; dev-dependencies per the `validation` row above (R-187),
`render` never on `engine` (the edge would run against the payload's direction), and nothing on `gui`.

---

## 8. What drills down from here

**Ring drill-downs now written:** Observation → `principia_dd_telemetry_and_tiers.md` (the frame
record, `stage_ms`, deriving tiers from measured hardware, graceful failure) and
`principia_scratchpad_pointer_channels.md` (trace / sound / inspector as three projections of one
integration; normative only where render_gui_spec, trajectory_viewing or a ruling cites it — R-96). Provenance → `principia_dd_image_embedding.md` (LSB payload, tile grid, the three
recovery searches). Allocation → `principia_dd_refinement_policy.md` (`Policy::Tolerance`, `eps`,
`alpha_area` as a dimension, merging under a live playhead).

Each rung and each ring gets its own drill-down file when planned in detail, with its tests: internals, its side of each seam it touches, and the test list (unit tests per contract clause; the debug views as visual assertions; round-trip/gauge sweeps for encode; the golden-IC suite for the integrator; the firewall test — scheduled twice, byte-identical payloads — for Allocation). The seam catalogue in §5 is the test index: **every row is at least one integration test.**

---

*One ladder from intent to image, a waist at Memory, one upward door at Encode. Around it, six rings that decide when, how often, how faithfully, and where — never what. The two membranes (GPU↔CPU, wasm↔JS) are facts about deployment, not about meaning. Sim key above the waist, render key below; and no ring may touch what a rung returns.*
