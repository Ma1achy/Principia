# Milestones — M0 to M8

The build plan's milestones, in dependency order. Each has a scope, the build order it comes from, and an exit
gate stated as requirement ids from `plan/requirements.yaml`. The gate blocks are generated
(`plan/tools/milestones.py`); the prose is not.

## Assumptions

- **The implementation lives in this repo, in `crates/` next to `docs/`.** The corpus names two crates, the
  engine crate and the gui crate (canonical_spec §1, item 5: "the engine crate exposes only the typed state
  surface `pub`"); other crate boundaries are for the build to set and are not requirements.
- **The docs stay the authority.** `plan/` is derived from `docs/` and `decisions.md`. Where a requirement
  and its source disagree, the source wins and the requirement is the bug. A change of design goes into the
  docs first (with an RQ and a ruling, per the porting rule); the plan follows.
- **A requirement joins the gate of the earliest milestone at which the thing it constrains exists and can be
  verified,** and stays green in every later milestone. A measurement the corpus says is "set by
  measurement" is a requirement of the milestone where the measured thing first runs; it is never an
  implicit TODO.
- **A value the corpus doesn't give is a calibration requirement (R-71)** in the milestone that first needs
  it: the task proposes the value with its evidence, a reviewer checks it, and the human confirms it at that
  milestone's gate before it is recorded in `decisions.md`. A missing definition is a definition requirement
  (R-72), written into the docs by the task and reviewed by the physics reviewer.
- **Requirements waiting on an open REVIEW_QUEUE entry** carry `rq:` in `requirements.yaml`. They stay in
  their milestone's gate; the milestone can't exit until the RQ is ruled and the requirement passes.

## Where the order comes from

The corpus gives four build orders. They agree, and the milestones follow them.

| Build order | Says | Milestones |
|---|---|---|
| canonical_spec §1, item 6, and §11 ("Milestone/implementation build plan") | "Native-first … prove parity/toolchain → validate physics numerically → build the browser product on top." | M0 (toolchain) → M3–M4 (physics validated natively, then parity on the GPU) → M8 (browser product) |
| debug_tooling_plan, "Build order within Phase 0 (the only forced staggering)" | 0a generation root → 0b synthetic payload harness → 0c the kernel bring-up mode, then the §A fragment presets (UV first; DECODE once the decode port lands; ROUNDTRIP once encode lands; R-75) → structural views as soon as `RenderQuad` is defined → cross-checks per seam as components arrive. "Build the eyes before the thing seen." | 0a → M0; 0b, the bring-up mode and the UV preset → M1; the DECODE and ROUNDTRIP presets → M2; cross-checks land with their seam (M2–M6) |
| render_contract Part 6, "Why first, concretely" | "Build order stands: contract + SDK → debug catalogue → flat compute → quads → adaptive." | M0 → M1 → M4 → M5 → M6 |
| deep_zoom §3, "Three-layer quadtree" | "Built in three layers, each a working system … This is the build order": 0 flat grid → 1 quad cache + ancestor fallback → 2 adaptive refinement. | M4 → M5 → M6 |

Between the debug catalogue (M1) and flat compute (M4), the physics goes in. Decode and encode come first
(M2), because the DECODE and ROUNDTRIP presets need them and the integrator consumes their output. Then
the integrator and events are validated on the CPU (M3), because canonical_spec §1 item 6 puts "prove
parity, branch-determinism, and the toolchain in a native `cargo test`" before the browser. Colour
composition (M7) and the GUI (M8) come last: render_contract's order ends at adaptive, and colour "has no
parity stakes" (canonical_spec §1, item 4).

**The research phases are settled (R-74).** canonical_spec §11 governs: the vertical slice settled the
integrator, step control, escape criterion and refinement policy, so there is no research milestone before
the build. Philosophy §7.8's sequencing is marked superseded; what survives of it is one M3 validation
requirement, the logH falsification check of the re-registration mechanism.

---

## M0 — Gates and scaffolding

**No physics.** The gates are built before anything they gate.

- The cargo workspace and crates under `crates/`, with CI running every runner below on every push.
- The substrate toolchain: one Rust source compiled to f32 SPIR-V → WGSL (rust-gpu) and to f64 for the CPU
  (canonical_spec §1, item 2), proven with a trivial kernel natively.
- The test harness, the golden-image runner, the numerical-gate runner and the benchmark runner. Each
  requirement's `verify.method` has a runner here, before the first requirement of that kind exists.
- The profiler CLI skeleton, schema v1 (R-56; dd_telemetry_and_tiers §5, render_gui_spec §G5 "Profiler").
- The contract surfaces' skeleton, in the engine crate (`principia_gui_state_contract.md` §1): the typed surfaces the
  contracts name, with no behaviour.
- The ledger generator. This is the generation root (debug_tooling_plan step 0a): ledger → generated
  pack/unpack for Rust and WGSL, plus the codegen self-test. Nothing renders yet, and the tests are green.
  Every payload type later is generated from it, so it is the root of the build DAG.
- The process rules that hold from the first commit: the docs as the authority, nothing built from archived
  files, and every pitfall regression requirement that can be checked without physics.

<!-- gate:M0 -->
**Exit gate — 64 requirements** (and every earlier gate still green):

- PAY (21): REQ-PAY-001…020, REQ-PAY-087
- GEN (9): REQ-GEN-001…008, REQ-GEN-024
- SCHED (1): REQ-SCHED-001
- RENDER (2): REQ-RENDER-001…002
- TOOL (11): REQ-TOOL-001…008, REQ-TOOL-119…121
- VAL (11): REQ-VAL-001…009, REQ-VAL-135, REQ-VAL-138
- SYS (9): REQ-SYS-001…008, REQ-SYS-063
<!-- /gate:M0 -->

## M1 — The synthetic payload and the eyes

debug_tooling_plan steps 0b and 0c (first half); render_contract "contract + SDK → debug catalogue".

- CPU-filled `SimState` / `ICDescriptor` / `RenderQuad` buffers. "Screen colours a hand-filled buffer; both
  surfaces green — before any physics."
- The fragment path from `SimState` to colour: `RenderContext`, the render SDK, and lowering of the fragment
  variants that exist so far.
- The field-view and sub-field debug shaders, with their accessor tests. These are the visual unit test of
  the bit layout.
- The kernel's single bring-up mode (colour_composition Appendix A), and the UV preset and coordinate view on
  the fragment side (R-75). The coordinate view catches a wrong Y convention at the first pixel
  (coordinate_conventions).
- Structural views over a defined `RenderQuad`, before any scheduler fills it.
- The baseline outcome palette that the field views need, pinned by golden images.

<!-- gate:M1 -->
**Exit gate — 80 requirements** (and every earlier gate still green):

- INT (1): REQ-INT-001
- PAY (13): REQ-PAY-021…033
- GEN (5): REQ-GEN-009…012, REQ-GEN-027
- RENDER (24): REQ-RENDER-003…024, REQ-RENDER-075, REQ-RENDER-077
- COL (8): REQ-COL-001…005, REQ-COL-053, REQ-COL-055…056
- GUI (1): REQ-GUI-001
- TOOL (23): REQ-TOOL-009…028, REQ-TOOL-122…124
- VAL (4): REQ-VAL-010…012, REQ-VAL-122
- SYS (1): REQ-SYS-009
<!-- /gate:M1 -->

## M2 — Charts, decode and encode

debug_tooling_plan 0c: the DECODE preset "once the WGSL decode port lands", ROUNDTRIP "once encode lands" (R-75).

- The chart registry and every chart in chart_reference's build list, with the `validate(u, v)` trait method
  (R-26).
- The decoder (shared CPU/GPU source), canonicalisation, the mirror deadband, and DEGENERATE handling.
- Energy normalisation η_E (R-25).
- Inverse encode, lookup and lock. The lock is chart construction on the CPU, in `SimConfig` (R-69).
- The DECODE and ROUNDTRIP fragment presets and the round-trip residual view (R-75).
- The chart reference values as unit and property tests.

<!-- gate:M2 -->
**Exit gate — 155 requirements** (and every earlier gate still green):

- DEC (36): REQ-DEC-001…030, REQ-DEC-038…041, REQ-DEC-043…044
- ENC (29): REQ-ENC-001…020, REQ-ENC-022…027, REQ-ENC-029…031
- CHART (45): REQ-CHART-001…036, REQ-CHART-043…051
- INT (2): REQ-INT-002…003
- PAY (3): REQ-PAY-034…035, REQ-PAY-088
- GEN (5): REQ-GEN-013…015, REQ-GEN-025…026
- RENDER (3): REQ-RENDER-025…027
- COL (2): REQ-COL-006, REQ-COL-057
- GUI (6): REQ-GUI-002…007
- TOOL (2): REQ-TOOL-029…030
- VAL (15): REQ-VAL-013…024, REQ-VAL-118, REQ-VAL-120…121
- SYS (7): REQ-SYS-010…016
<!-- /gate:M2 -->

## M3 — The integrator, events and physics validation (CPU, native)

canonical_spec §1 item 6 and §11: "validate physics numerically", natively, before the browser.

- Stepper and regularisation occupants (Heggie default; AZ; logH; none). Substep control with the frozen
  N_sub bucket lookup. The fixed `dt_macro` march.
- Terminal events: collision, escape (closure + energy), ionisation, precedence, and the termination checks.
- The full `SimState` accumulator fill on the CPU.
- The validation orbits and ground truth. The independent convergence reference. The Inspector's RK45
  reference (R-33).
- The physics measurements the corpus leaves to the build:
  - re-validating escape precision and recall against check 2's independent ground truth, with the legacy
    t = 30 set as a comparison (R-95);
  - the logH falsification check of the re-registration mechanism (R-74);
  - δ_dep;
  - the tau gap;
  - the Yoshida-6 coefficient check before Yoshida-6 enters the shared source.
- The CPU-side parity suite (`computeIC`).

<!-- gate:M3 -->
**Exit gate — 181 requirements** (and every earlier gate still green):

- INT (61): REQ-INT-004…056, REQ-INT-073…074, REQ-INT-076…081
- EVT (22): REQ-EVT-001…020, REQ-EVT-023…024
- PAY (27): REQ-PAY-036…059, REQ-PAY-073, REQ-PAY-083, REQ-PAY-086
- GEN (1): REQ-GEN-016
- SCHED (2): REQ-SCHED-002…003
- RENDER (1): REQ-RENDER-028
- GUI (1): REQ-GUI-008
- TOOL (10): REQ-TOOL-031…039, REQ-TOOL-125
- VAL (47): REQ-VAL-025…055, REQ-VAL-115, REQ-VAL-117, REQ-VAL-119, REQ-VAL-123…124, REQ-VAL-126…134, REQ-VAL-136, REQ-VAL-139
- PERF (6): REQ-PERF-001…004, REQ-PERF-083, REQ-PERF-085
- SYS (3): REQ-SYS-017…019
<!-- /gate:M3 -->

## M4 — Flat compute on the GPU (deep_zoom layer 0)

render_contract "→ flat compute"; deep_zoom §3 layer 0: "defines the three contracts".

- The compute kernel over a flat grid, from the shared source. The dispatch shape (systems_architecture
  §5.5).
- The GPU determinism rules.
- The CPU↔GPU parity tiers: branches identical, trajectories free to diverge.
- The temporal march, lockstep and checkerboard.
- The integration-seam cross-checks (debug_tooling_plan §G).
- The first kernel benchmarks.

<!-- gate:M4 -->
**Exit gate — 98 requirements** (and every earlier gate still green):

- CHART (1): REQ-CHART-037
- INT (15): REQ-INT-057…071
- EVT (1): REQ-EVT-021
- PAY (2): REQ-PAY-074, REQ-PAY-085
- SCHED (11): REQ-SCHED-004…013, REQ-SCHED-084
- RENDER (9): REQ-RENDER-029…036, REQ-RENDER-078
- TOOL (8): REQ-TOOL-040…045, REQ-TOOL-126…127
- VAL (29): REQ-VAL-056…079, REQ-VAL-112, REQ-VAL-125, REQ-VAL-137, REQ-VAL-140…141
- PERF (12): REQ-PERF-005…013, REQ-PERF-077, REQ-PERF-081, REQ-PERF-086
- SYS (10): REQ-SYS-020…029
<!-- /gate:M4 -->

## M5 — Quads, the scheduler and caching (deep_zoom layer 1)

render_contract "→ quads"; deep_zoom §3 layer 1: "panning never blanks".

- The quadtree, `QuadRequest` flags, and the `RenderQuad` the scheduler fills.
- `QuadReduction` (the joint grain, `dominant_outcome`, impurity, ensemble spread).
- Ancestor fallback.
- The cache and its keys (sim and render), with blast radius and eviction.
- Memory tiers and budgets; the frame loop and per-frame budget.
- Sampling, MSAA and ensemble copies.

<!-- gate:M5 -->
**Exit gate — 151 requirements** (and every earlier gate still green):

- DEC (2): REQ-DEC-031…032
- CHART (3): REQ-CHART-038…039, REQ-CHART-041
- INT (2): REQ-INT-072, REQ-INT-075
- EVT (1): REQ-EVT-022
- PAY (14): REQ-PAY-060…068, REQ-PAY-075…079
- GEN (1): REQ-GEN-017
- SCHED (49): REQ-SCHED-014…052, REQ-SCHED-054…055, REQ-SCHED-074…076, REQ-SCHED-079, REQ-SCHED-083, REQ-SCHED-086…088
- REF (10): REQ-REF-001…010
- RENDER (22): REQ-RENDER-037…056, REQ-RENDER-076, REQ-RENDER-079
- COL (1): REQ-COL-007
- GUI (1): REQ-GUI-009
- TOOL (11): REQ-TOOL-046…054, REQ-TOOL-116…117
- VAL (4): REQ-VAL-080…083
- PERF (22): REQ-PERF-014…033, REQ-PERF-082, REQ-PERF-087
- SYS (8): REQ-SYS-030…037
<!-- /gate:M5 -->

## M6 — Adaptive refinement and deep zoom (deep_zoom layer 2)

render_contract "→ adaptive"; deep_zoom §3 layer 2 and §4.

- The refinement policy (`Policy::Tolerance`, R-15), its stopping conditions, and the quality
  controller.
- The relevance terms.
- The linearised decoder, the decode switchover and `AT_F32_FLOOR`.
- The |det J_D| measure weight.
- The integration-floor flags and the precision warnings.
- The symbolic-dynamics per-quad quantities.
- The refinement policy's open measurements.

<!-- gate:M6 -->
**Exit gate — 138 requirements** (and every earlier gate still green):

- DEC (6): REQ-DEC-033…037, REQ-DEC-042
- PAY (8): REQ-PAY-069…072, REQ-PAY-080…082, REQ-PAY-084
- GEN (1): REQ-GEN-018
- SCHED (23): REQ-SCHED-056…071, REQ-SCHED-077…078, REQ-SCHED-080…082, REQ-SCHED-089…090
- REF (38): REQ-REF-011…037, REQ-REF-039…040, REQ-REF-042…050
- GUI (6): REQ-GUI-010…015
- TOOL (4): REQ-TOOL-055…058
- VAL (13): REQ-VAL-084…095, REQ-VAL-142
- PERF (38): REQ-PERF-034…067, REQ-PERF-084, REQ-PERF-088…090
- SYS (1): REQ-SYS-038
<!-- /gate:M6 -->

## M7 — Colour composition and the display chain

Colour has "no parity stakes" (canonical_spec §1, item 4) and its own golden and property suite (parity
contract §7, "What this contract does *not* cover"). So it builds on a working field, after adaptive.

- The stain node graph (R-64), with typed node inputs (R-53) and the ≤8-node post chain.
- Presets as serialised graphs, and render-key hashing of a graph.
- The colour maths, including the OKLab transcription check.
- Palettes beyond M1's baseline.
- The display chain: stain → style → display scale → gamut clamp → colour-vision simulation → screen (R-67).
- Image embedding.

<!-- gate:M7 -->
**Exit gate — 109 requirements** (and every earlier gate still green):

- GEN (4): REQ-GEN-019…022
- SCHED (1): REQ-SCHED-072
- RENDER (18): REQ-RENDER-057…072, REQ-RENDER-080…081
- COL (45): REQ-COL-008…036, REQ-COL-038…046, REQ-COL-049…052, REQ-COL-054, REQ-COL-058…059
- GUI (16): REQ-GUI-016…031
- TOOL (20): REQ-TOOL-059…072, REQ-TOOL-109…113, REQ-TOOL-118
- VAL (3): REQ-VAL-096…098
- PERF (2): REQ-PERF-068…069
<!-- /gate:M7 -->

## M8 — The dev GUI, tooling, the browser product and the release gates

canonical_spec §11: "build the browser product on top".

- The dev GUI against the artboards in `docs/gui/design/`. R-68 applies: the artboards set layout, and
  corpus values win.
- The `gui_state_contract` schema, undo rules and firewall.
- Trajectory viewing, the IC scratchpad, and pointer channels.
- Export and animation.
- The full profiler and debug tooling UI.
- The frame budget, end to end.
- The browser build: wasm engine worker, `OffscreenCanvas`, and the two membranes.
- The non-Metal parity run: the standing pre-Paper-2 action (canonical_spec §11).

<!-- gate:M8 -->
**Exit gate — 229 requirements** (and every earlier gate still green):

- ENC (3): REQ-ENC-021, REQ-ENC-028, REQ-ENC-032
- CHART (1): REQ-CHART-040
- GEN (1): REQ-GEN-023
- SCHED (1): REQ-SCHED-073
- RENDER (2): REQ-RENDER-073…074
- COL (2): REQ-COL-047…048
- GUI (128): REQ-GUI-032…047, REQ-GUI-049…160
- TOOL (39): REQ-TOOL-073…092, REQ-TOOL-094…108, REQ-TOOL-114…115, REQ-TOOL-128…129
- VAL (16): REQ-VAL-099…103, REQ-VAL-105…111, REQ-VAL-113…114, REQ-VAL-116, REQ-VAL-143
- PERF (12): REQ-PERF-070…076, REQ-PERF-078…080, REQ-PERF-091…092
- SYS (24): REQ-SYS-039…062
<!-- /gate:M8 -->
