# Principia — canonical architecture & design (the yardstick)

*The single statement of what Principia's architecture and design have **landed on**. Written from the decisions themselves — not summarised from the other docs — so it is a **reference to check the corpus against**, not a synthesis that could inherit corpus drift. Every other `principia_*.md` file must be consistent with this; where a file and this document disagree, one of them is wrong and it gets resolved, not papered over. The file-by-file verification pass (all 35 `principia_*.md` docs, read in full) has now confirmed the corpus against this document, so it is the **final canonical spec** — the navigational layer and the standing yardstick. It states decisions and invariants tersely and points to the authoritative doc for each; it does not re-derive the maths (that lives in the drill-downs and contracts).*

---

> **Evidence base.** Decisions marked *(vertical slice)* were measured in `prin-rs` — see its
> `FINDINGS.md`, `README.md` and `results/`. Every settled default in this spec should be traceable to
> where it was measured; a default without a citation becomes folklore, which has cost this project
> four days once already.

## 0. What Principia is

A browser-based **WebGPU research instrument** for the planar three-body problem's **8-dimensional, symmetry-reduced initial-condition (IC) manifold** — an interactive visualiser, a rigorous computational-physics engine, and the substrate for two papers (Paper 1: the tool + visual findings; Paper 2: quantitative characterisation — fractal dimension, Burrau neighbourhood). Public release via GitHub Pages. The instrument renders the IC manifold itself as the image: each pixel is an IC, coloured by the fate/diagnostics of its trajectory.

---

## 1. The substrate (the defining decision)

**One shared Rust engine, compiled two ways, with a thin TypeScript GUI shell.** *(Decided via the toolchain spike — `principia_spike_brief.md`, findings verified 2026-07.)*

1. **The whole CPU engine is Rust, compiled to wasm.** Scheduler, cache, quadtree, allocation ring, the physics reference — all Rust. The GUI is a thin TS shell owning only DOM + view-scratch state.
2. **The physics kernel is one Rust source, compiled to both targets:** to **f32 / SPIR-V** for the GPU via **rust-gpu** (SPIR-V → WGSL → browser WebGPU, which is Metal/Vulkan/D3D12 under the hood), and to **f64 (or higher)** for the CPU via ordinary `rustc`. Logic drift between CPU and GPU is therefore **structurally impossible** — there is one source, not two implementations.
3. **The GPU is f32-locked** (WebGPU has no f64). The CPU is precision-unconstrained: f64 by default, double-double / arbitrary where wanted. The GPU is for **throughput**; the CPU is for **accuracy and sanity-checking**.
4. **Two pipelines, two assembly mechanisms** (the split; `lowering` Part 2): the **compute** pipeline is **monomorphised Rust** (a chart/occupant is a type parameter the compiler specialises — no runtime source compilation, no runtime interpreter); the **fragment** (colour) pipeline is **hand-written WGSL**, assembled from keyed snippets and **hot-reloaded at runtime** (this is what keeps the custom-colour devkit alive — colour has no parity stakes). Consequence: **no runtime-authored custom *compute* occupants** (you cannot compile user Rust in the browser — build-time variants only); runtime-authored custom *colour* occupants stay.
5. **Two membranes** (`systems_architecture` §3): **GPU↔CPU** (dispatch down, tiny reductions/pulls up) and **wasm↔JS** (`set_field` edits in, GUI-sized snapshots out, canvas transferred once). The engine↔GUI firewall is enforced **structurally, per binding**: the **egui dev debug menu** (F3-toggled, same wasm binary) is walled off by the **crate graph** (engine crate exposes only the typed state surface `pub`); the later **TS production GUI** (across wasm↔JS) by the **binary split** (no shared linker). Both are interchangeable bindings over one contract — the dev GUI exercises `set_field`/snapshot exactly as the TS GUI will, earning no in-process shortcut.
6. **Native-first development.** Prove parity, branch-determinism, and the toolchain in a native `cargo test` before the browser. The native build can also run the survey headless and generate the Paper-2 numbers, while the browser build is the Paper-1 public artefact — same kernel, decoupled outputs.

**Vehicle rationale:** rust-gpu (uniform Rust incl. the kernel; keeps double-double open) was chosen over CubeCL (a `#[cube]` DSL island + ~f64 ceiling) because the spike's browser hop was clean.

---

## 2. The determinism law (the other defining decision)

**Continuous divergence is honest and welcome; branch divergence is forbidden and nearly free to forbid.** *(Established by the spike; the law is `principia_gpu_determinism_note.md`.)*

- **Two kinds of divergence.** *Continuous* values (positions, momenta, energy, the trajectory, and the **outcome class** it decides near a chaotic boundary) diverge at FP scale and then exponentially — that divergence **is the science** ("divergence is the observable"; the inspector overlay). *Branch decisions* decided instantaneously from the current state (`N_sub`, collision, terminal) must **not** fork on identical inputs, per step (R-84; labels on chaotic trajectories may still differ across precisions, because their inputs have diverged), because a forked branch means the two sides integrate on **different grids** — the CPU stops being a check on the *same* computation, and Paper 2's fractal boundary gains non-physical hash noise.
- **The mechanism (measured, attribution controlled).** The forks were **not** fast-math and **not** FMA (a controlled safe-math test overturned the fast-math guess). The cause is **inherent cross-implementation transcendental latitude**: a GPU's `pow`/`sqrt`/`div` and libm's differ by 1–3 ulp *in every math mode*, so any runtime transcendental feeding a `ceil` at an integer boundary can fork. No matched rounding fixes it.
- **The one-line law:** *a control-flow decision is frozen at build time or reduced to a comparison — never left hostage to a floating-point op the backend computes its own way.* (Same law as the read-side NaN/predicate rule.) Concretely: `N_sub` is a **bucket lookup** against 64 build-time-frozen f32 thresholds; collision is `d² < r_coll²`; horizon is an **integer step counter**; the `d²` feeding the comparison is **position-quantised** (f32 positions, lone-sub/mul, explicit `fma`); clamp in f32 **before** any float→int cast.
- **The GPU compile-discipline checklist** (each earned by a measured failure): comparison-only branches · no non-finite literals (naga rejects `f32::INFINITY`) · constant/unrolled indices (implicit bounds checks lower to the multi-level-exit shape that **silently truncated a loop** — wgpu#4449) · flag-in-`while`-condition loop shape, zero `break`s · clamp-before-cast · explicit `fma`.
- **Parity re-aimed, not retired.** Shared source kills *transcription* drift structurally — but the *same source* still passes through a shader compiler entitled to **silently miscompile** (the truncation is the proof), so the parity suite is **permanent**, re-aimed at **backend miscompilation + branch determinism**. It runs as a **native in-process `#[test]`** (instantiate CPU-f64, dispatch GPU, diff `SimState`): branch words bit-exact, continuous words loose. **Standing action item:** the spike was Metal-only; run the gate on a non-Metal backend (Vulkan/D3D12) before trusting the survey for Paper 2.

---

## 3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*

- **8 DOF, not 10, not 5.** The reduced manifold is `2 config ⊕ 4 momentum ⊕ 2 mass`. Full 12 → CoM removal → 8 phase + 2 mass = 10 before gauge → fix rotation (pin ρ̃ to +x) + scale (R̃ = 1, i.e. I = 1) removes 2 *configuration* DOF → **8**. Mass is *inside* the 8, a direction, never an external global.
- **Latent controls** `z ∈ ℝ⁸` pushed through a **factorised decoder** `D = D_mass × D_cfg × D_mom` (blocks `z[0:2]` config, `z[2:6]` momentum, `z[6:8]` mass); decode order **mass → config → momentum → derived invariants**. No latent coordinate on a gauge direction.
- **Canonicalise is the one seam to `(m, r, p)`**: the quotient onto the section (CoM, ρ̃ on +x, λ̃_y ≥ 0, I = 1). Everything converges through it; **the integrator has one input type `(m, r, p)`, forever**, and never learns the chart's name.
- **Dimensionless units** `G = M = I = 1` throughout — a single global horizon `T` is legitimate because scale is gauged.
- **Links carry a measure.** Compactification functions (softmax, sigmoid, warps) are registry entries — constraint-preserving, invertible (conditioned), C¹, each carrying its log-det Jacobian; quantitative claims either correct by the Jacobian or are barred (measure honesty).
- **Encode is the one upward door** (`inverse_encode_contract`, `dd_encode`): `E = (block inverses) ∘ C`; rigid ops act on the **full state** (positions *and* momenta); the order is subtract CoM → rescale → subtract boost → rotate → mirror, so CoM subtraction precedes `I` (R-23); the invariant fibre point is the chart's own forward construction (**encode reuses decode**); tolerances in physical units.

---

## 4. Charts & navigation *(authoritative: `chart_decoder_contract`)*

- **A chart is a 2D projection *through* the 8D manifold** — a mathematical object (a plane + fixed values for the other 6 DOF), not a camera/view-mode. Affine slice `z(s,t) = z₀ + (2s−1)q₁ + (2t−1)q₂` is one implementation of the general `Φ: [0,1]² → (m,r,p)`.
- **Four axis kinds (closed set):** raw latent · derived-in-block (needs a residual convention) · cross-block invariant (solves downstream; brings feasibility boundaries) · coupled curve (tilt well-posed only at a lock).
- **Navigation is chart construction:** pan/slice edit `z₀`, zoom/tilt edit the basis, the lock pins the centre — **all CPU-side edits to one uniform**; the GPU has no locked mode, no view mode, no second code path. Non-raw slice/tilt directions are tangents evaluated at the centre. **The sim key holds the slice plane** (`z₀`'s out-of-plane part, span{q₁, q₂}, the in-plane orientation): in-plane pan and zoom re-address; slicing out of the plane, tilting and rotating re-integrate; the lock changes neither (R-92).
- **Well-posedness:** a chart is valid iff its swept axes + conventions + slice pin all 8 DOF exactly once (annotation axes contribute none → a 1-swept-in-2D chart is well-posed as 1+7).

---

## 5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*

- **Lazy-in-time lockstep live-march.** Time is a **live clock**, not a stored dimension. A global **playhead** advances by fixed `dt`; every visible sample's `SimState` marches with it under a **barrier over the live set**; the fragment stage colours the current state. **No stored history — state is O(1) in time.** The GUI's time scrubber sets the display time and re-integrates progressively; it never replays stored frames ("no scrub" applies to exported animations only — R-66).
- **Never present mixed time** — the one bounded exception is **checkerboard** motion acceleration (uniform one-`dt` half-frame skew *only while the playhead advances*, self-erasing at rest/pause/endpoints, **force-off in export**).
- **Integrate and colour are separate passes — and separate mechanisms** (Rust compute / WGSL colour). Switching a render mode never recomputes sim data.
- **SSAA via E ensemble copies** (Halton (2,3) offsets; the ensemble copies *are* the SSAA pool *and* the spread pool; spread is derived at resolve, never stored; Benettin shadows are never SSAA samples). **Six tiers** Potato/Low/Medium/High/Ultra/Extreme; FTLE (Benettin) computed **Medium and up**; `render_scale` (internal render resolution ÷ display, 0.25–2.0) is a **render-side, non-sim-key** knob and the free continuous motion lever.
- **Blur is the single vocabulary item for "not current"** — spatially stale, temporally behind, or still arriving; sharp means true. **Never blank, never lies, never freezes** (baseline-first + blurred backdrop + the wasm-engine worker).
- **Trajectory count per render pixel:** `(E+1)` full uniform samples + their FTLE shadows (from Medium up); spread reduced at resolve. **Bundle atomicity:** a render pixel's whole bundle (nominal + E copies + all shadows) shares **one** checkerboard phase — all march or all hold.

---

## 6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*

- **Payload purity:** `SimState = f(IC, sim key)` — no scheduling/render/timing state in it. Buys free eviction, safe recompute, device-loss recovery, cross-chart cache sharing (v2).
- **The struct:** hot `SimState` **144 B (FTLE on) / 96 B (FTLE off)**, 8-byte aligned (recomputed with the closure field, R-40 / D6); a **separate cold word buffer** (~16 B/sample, mixed-radix ~76-symbol free-group word, touched per branch-cut crossing + at resolve, never per-step); `ICDescriptor` (64 B, with explicit padding; E₀ is derived as K₀ + V₀, not stored — R-86); `QuadReduction` (~80 B) the summary. Descriptor: bits 0–7 used, bits 8–9 `last_symbol`, 10–15 reserved (R-86).
- **Derive-at-read:** FTLE (`S_final/(n·dt)`, partial renorm finalised), ensemble spread (resolve-stage), the substep-log proxy (⌊log₂⌋ of the exact `total_substeps` u32 via `countLeadingZeros`, 0 for a total ≤ 1 — R-86), drift — all **derived at read, not stored fields**.
- **Read-side interface:** the read-side `SimState` type is **fixed across all tiers** (plain field syntax); a tier-absent feature reads the **NaN sentinel** (loud, but never load-bearing — WGSL fast-math may drop it); validity is `has_<feature>` compile-time consts **+ descriptor predicates** (`sd_is_failed`, `ftle_valid`), **never `isnan()`**.
- **Hybrid render-target memory model:** targets = `k_d ≈ 3 × display_px × 4` (swap chain + final composite, necessarily display-sized, fixed) + `k_r ≈ 3 × render_px × 4` (backdrop + intermediates, scale with `render_scale²`).
- **Deep zoom:** CPU (f64) owns global/nonlinear precision (quad centre/half-width, `x₀`, `J_D`); the GPU (f32) does only quad-local relative arithmetic (`x = x₀ + J_D·δ`). Screen-space floor is the everyday refinement boundary (view-relative, not terminal): in view, quads above it must split, and below it the criterion may supersample; `MAX_REL_DEPTH` caps every split beyond the screen floor, off-screen policy splits included (R-88, R-98); the true terminals are the linear-decoder `AT_F32_FLOOR` and the integration floor.
- **Threading:** the **entire frame loop is the wasm engine, in a Web Worker via `OffscreenCanvas`**; a second wasm context is the f64 inspector worker. Input via SharedArrayBuffer (cross-origin-isolated) else postMessage. **Two membranes, one law: big data never crosses** — reductions/snapshots only, never the payload/tree; **data boundaries, not object boundaries** (no wasm handles held by JS).

---

## 7. Precision & validation model *(authoritative: `parity_contract`, `validation_ground_truth_note`, `core_design`)*

- **Divergence is the observable** (day-one shipping feature: the pixel inspector). CPU/GPU divergence *for continuous values* is a free chaos diagnostic; branches never diverge (§2).
- **Two references, two jobs.** (a) The **shared kernel at f64** on the CPU — value is **precision** (removes numerical ambiguity), *not* independence (it shares source with the survey). (b) A **separate, deliberately-independent high-precision convergence integrator** (CPU arbitrary precision with convergence gating — raise the precision and tighten the tolerance until the result stops changing; double-double is a fast screen only, R-33) — value is **independence**, immune to shared-source bugs; used for the integration-floor falsifiability probe and the Burrau reference. *(This second reference is placed in the Precision ring; its full treatment is a live gap-hunt item — see §11.)*
- **Two suites compose.** The **ground-truth suite** validates physics against **external** truth (analytic Lagrange/Euler, periodic figure-eight, literature Burrau/Lehto) on the trustworthy f64 reference — it catches shared-source logic bugs precisely because the truth is external. The **parity suite** proves **GPU == CPU** (branch words exact, continuous windowed). Compose: **correct-CPU × GPU==CPU ⇒ correct-GPU.** Chaotic ground-truth cases (Burrau) are compared at **structural features only**, never pointwise.

---

## 8. Locked vocabulary

- **Sampling hierarchy:** **QUAD** (a quadtree node in IC-space) → **N×N SAMPLES** (each a full `SimState` simulation; `N = SAMPLES_PER_QUAD_AXIS`) → each **SAMPLE** rasterises to one **TILE** (its screen footprint) → **PIXELS**. One sample, one tile, no interpolation.
- **Keys & clocks:** **sim key** (change → re-integrate) · **render key** (change → recolour) · **playhead** (a live clock — advancing it is sim work; neither key) · **navigation** (in-plane pan and zoom re-address which quads are asked for; slicing out of the plane, tilting and rotating change the slice plane, which is on the sim key, and re-integrate; the lock changes neither — R-92). `render_scale` is render-side, **not** sim-key.
- **Two pipelines:** **kernel** = the compute side (Rust → SPIR-V); **shader** = the fragment side (WGSL). An **occupant** is a swappable slot fill (an `ADVANCE` integrator on the compute side, R-19; a colour/brightness/combiner/post function on the fragment side). *(The vertical slice added a **second compute axis**: **regularisation** is an occupant slot independent of the stepper — `none` / AZ / **Heggie (default)** / logH. See `principia_integrator_contract.md` Part 2b.)*
- **Retired terms (must not reappear):** `TileID`/`computeTile`/`samples_per_tile` (→ QUAD vocabulary / `computeQuad`) · `M` checkpoint count (→ `n_renorm`; no stored trajectory under lockstep) · a `TIMEOUT` state (→ reaching the horizon *is* `bounded`) · `sd_is_untrusted` (→ `sd_is_failed`) · "N ensemble shadows" (→ E ensemble copies that are full samples) · the `Math.fround`/f32-evaluation `N_sub` rule (→ the frozen threshold-table bucket lookup) · "a single TypeScript layout constant generates WGSL pack for the kernel" (→ one Rust layout definition → Rust kernel/host + WGSL fragment).

---

## 9. The load-bearing invariants (the walls — the primary comparison checklist)

Each wall, its one-line statement, and where it lives. **A file contradicting any of these is wrong.**

1. **The firewall** — Allocation (scheduler/cache) never reaches payload contents; a payload is the same regardless of when/whether/how it was scheduled. *[scheduler, caching]*
2. **Payload purity** — `SimState = f(IC, sim key)`; no scheduling/render/timing state inside. *[payload, caching]*
3. **Sim/render key split** — sim-key change re-integrates, render-key change recolours; `render_scale` invalidates nothing. *[render, caching, lowering]*
4. **Navigation is chart construction** — every gesture edits one uniform `(z₀,q₁,q₂)`; no view/camera object; the GPU has no modes. *[chart_decoder]*
5. **Integrator has one input type** — `(m,r,p)` forever; canonicalise is the one seam; the integrator never learns the chart's name. *[chart_decoder, core_design, integrator]*
6. **Charts lower, they do not interpret** — monomorphised per chart (a type parameter), never a runtime axis-kind interpreter. *[core_design, chart_decoder, lowering]*
7. **Measure honesty** — every arbitrary choice carries its Jacobian or is barred from quantitative claims; findings must survive link swaps + tilts. *[chart_decoder, deep_zoom]*
8. **Generate from one source** — bit layouts, link inverses, debug catalogue, export decoder from one **Rust layout definition**, emitted to Rust (kernel/host) + WGSL (fragment). *[generation_root, payload]*
9. **Totality** — every pixel gets a labelled output (escape/collision/bounded, sim_failed/decode_failed, or running); no timeout state; saturation is a confidence flag, not a terminal. *(Extended by the vertical slice: `Decision::Undetermined` and `footprint_undetermined` — a quad where **nothing integrated** reports `ensemble_spread` **5.6× smaller** than a healthy one, reading as better resolved, with `error_ratio_max` at exactly its converged 1.0000. Totality now covers the reduction layer, not only the pixel: a `filter(is_finite)` in a reduction is a silent discard.)* *[integrator, payload]*
10. **Shared-source logic equality** — CPU and GPU physics are one Rust source; logic drift structurally impossible; residual = precision + backend miscompilation. *[core_design, parity, gpu_determinism]*
11. **Branch determinism** — branch decisions bit-identical across backends on identical inputs, per step (R-84), via the comparison-only rule (frozen tables, integer counters, `d²`-comparisons; never runtime transcendentals); continuous values diverge freely. *[gpu_determinism, integrator dd]*
12. **Two membranes, one law** — GPU↔CPU and wasm↔JS; big data never crosses (reductions/snapshots only); data boundaries, not object boundaries. *[systems_architecture, caching, gui_state]*
13. **Never blank, never lies, never freezes** — baseline-first + blurred backdrop + wasm-engine worker; blur = "not current", sharp = true. *[render, caching, temporal]*
14. **Single playhead + checkerboard bounded exception** — one global barrier-synced playhead; never present mixed time; checkerboard is the one bounded, self-erasing, export-off exception. *[temporal, scheduler, checkerboard]*
15. **Bundle atomicity** — a render pixel's whole sample bundle shares one checkerboard phase; the mask is per-pixel, never per-trajectory. *[checkerboard]*
16. **Derive-at-read** — FTLE, spread, the substep-log proxy, drift are derived at read; spread is a resolve-stage reduction; not stored fields. *[payload, sampling, temporal]*
17. **NaN-sentinel read-side + descriptor-predicate validity** — fixed read-side type; tier-absent = NaN (loud, not load-bearing); validity via `has_<feature>` + descriptor predicates, never `isnan()`. *[render, lowering, payload]*
18. **Deterministic fixed-dt** *(amended — see `principia_integrator_contract.md` §2a: the
    count-bound relocates to a fixed **`tau`-schedule** for occupants that own their own time
    mapping. Same guarantee, occupant-declared. R-19: the count bound is on a fixed `tau`-schedule.)* — fixed `dt`, deterministic schedule, count-bound never wall-clock; the same spec re-renders the same result up to cross-backend f32 tolerance (branches exact). *[temporal, integrator, parity, export]*

---

## 10. Document map — reading order & authority

*This document is the **entry point**: read it first for the landed decisions and invariants, then follow the pointers below into the authoritative docs. Among the *detailed* docs, `systems_architecture` is the one to read first (the consolidated architecture map) — it sits just under this navigational layer, not above it. Where two docs overlap, the **authoritative** one wins (and the other should point to it).*

- **Spine:** `core_design` (the seams, terse), `systems_architecture` (ladder/rings/membranes/seam-catalogue/invariants/DAG).
- **Authoritative payload:** `dd_simstate_payload` — canonical for the struct, bit layouts, precision, derived-vs-stored (generation-root and render defer to it).
- **The determinism law:** `gpu_determinism_note` — canonical for branch determinism + GPU compile discipline (integrator/parity point to it).
- **Physics:** `chart_decoder_contract` (manifold/decoder/charts/navigation — the core), `inverse_encode_contract` (encode), `dd_decoder`/`dd_encode`/`dd_integrator` (the exact maths + tests), `integrator_contract` (wrapper/occupant/determinism seams), `symbolic_dynamics_contract` (word interpretation — **OPEN**).
- **Allocation/rendering/deployment:** `scheduler_contract`, `caching_contract`, `render_contract`, `lowering_contract`, `temporal_architecture_note`, `checkerboard_contract`, `memory_tiers`, `deep_zoom`, `sampling_msaa_note`, `coordinate_conventions_note`.
- **Codegen roots:** `dd_generation_root` (the ledger + link registry; **payload is canonical where they overlap**).
- **Colour/GUI/export:** `dd_colouring`, `gui_state_contract`, `trajectory_viewing`, `quality_device_note`, `export_animation_contract`.
- **Testing/validation:** `parity_contract`, `validation_ground_truth_note`, `debug_tooling_plan`.
- **Meta / superseded / throwaway:** `spec_pending_changes` (the register of pending changes, closed and archived), `ftle_shadow_precision_experiment`, `spike_brief` (throwaway), `parity_testing_note` (**SUPERSEDED** → parity_contract), `validation_scratchpad` (**SUPERSEDED** → validation notes).

---

## 11. Still open / downstream (not yet fully in the corpus)

- **GUI design** — written in step 6: `principia_render_gui_spec.md` is the GUI design (its conflicts with the corpus ruled by R-64 to R-68), on the corrected `gui_state_contract` (firewall = language boundary, scanned-fragment vs Rust-compute registry, state schema) as the foundation.
- **The independent convergence reference** — *what* it is is now settled (§7: a separate CPU arbitrary-precision integrator with convergence gating, double-double a fast screen only — R-33 — for shared-bug immunity, distinct from the shared-kernel-at-f64 *precision* reference — validation_ground_truth §correctness-factoring, parity §1); its full *treatment* (protocol, when it runs, how it plugs into the harness) is the gap still to close.
- **Symbolic-dynamics `dominant_pair` attribution** — OPEN (generator↔cut convention, punctured-sphere relation, third-pair algorithm) before per-pair quantities are trusted; the word *storage* is settled.
- **Milestone/implementation build plan** — written in step 7: `plan/MILESTONES.md` (M0–M8 and their exit gates) over `plan/requirements.yaml` (native-first: prove parity/toolchain → validate physics numerically → build the browser product on top). **The vertical slice has since collapsed the research phases** — integrator, step control, escape criterion and refinement policy are settled with evidence — so the plan is a *build* plan, and this governs over philosophy §7.8's research sequencing (R-74). One validation requirement stays in M3: the logH falsification check of the re-registration mechanism (R-74). The remaining unknowns are all interactive or GPU: the dispatch shape (settled on paper, `principia_systems_architecture.md` §5.5), cache eviction under real motion, and whether the frame budget holds. **Written after the GUI design pass**, since the GUI's scope sets half the milestones.
- **The non-Metal parity run** — a standing pre-Paper-2 action item (§2), a task not a doc.
- **The LaTeX spec is retired.** The markdown corpus is the only authority. What the corpus relied on from `principia_spec_revised.tex` and the two PDFs (the shape-sphere colour map, the COM-projection mini spec) has been ported into the files that rely on it, and the originals are archived under `docs/archive/spec_sources/` for the record only; where they disagree with the corpus, the corpus wins. The five pending edits (`QuadReduction` grain, Burrau leg-swap, constrained-inverse wording, the 5-DOF→8-DOF caption, body-index naming) are closed with the rest of the pending-changes register, now archived: each landed change is folded into its owning file, the caption is moot, and the last two are ruled: both Burrau charts are kept, each labelled with its quotient (R-27), and body indices are 0-based, with pair id `k` naming the side opposite body `k` (R-22).

---

*One Rust source compiled twice; the GPU for speed, the CPU for truth; branches identical, trajectories free to diverge. One manifold projected many ways; mass a direction, not a mode; the integrator blind to the chart. Time a march, not a store; blur means not-current, sharp means true. Two membranes, and big data crosses neither. This is the shape everything else must fit.*

---

> ### ⚠ ESCAPE CRITERION SUPERSEDED — see `principia_01_pitfalls.md` §2
> The old test (`E_rel > 0 ∧ receding`, optionally `∧ d > r_esc`) **fires on transients**: measured
> 0 of 895 escapes still unbound 8 boundaries later. It is replaced by
>
> ```
> ESCAPE  ⟺  |Δn̂| over a window < tau    AND    E_rel > 0
> ```
>
> **100% precision, 96.3% recall**, against 97.9% for the old test (on the legacy `t = 30` ground truth; the
> re-validation runs against check 2's independent ground truth, with the legacy set kept as a comparison — R-95). `receding` and `d > r_esc` are
> **redundant** once both hold (identical to the digit), so three tuned constants are eliminated.
> `tau` sits in a **383× gap** and is not tuned. Fires at `t≈10` rather than `t≈1.5` — **late rather
> than wrong**, which is correct for a *stored* `t_end`.
>
> **In production, escape ends the loop (R-103);** §2.4's three checks run in the validation harness. Freezing a
> trajectory whose displayed quantity is still moving is what produced the patchwork artefact
> (§1), and the checks are what guard against it. Collision stays terminal — it is a singularity, not a heuristic. **Once escape fires** (R-95), `state`
> reads escape and `t_end` is fixed; time averages (FTLE's `S/T` and the like) freeze at `t_esc`; `done` is set and the
> loop ends. The post-escape march for §2.4's checks runs only in the validation harness, on its own state; the payload
> never sees it (R-103). The window is sampled at macro-step
> boundaries for unregularised occupants and at sync boundaries for regularised ones.
