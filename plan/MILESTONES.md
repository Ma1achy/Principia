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
- **Requirements waiting on an open REVIEW_QUEUE entry** carry `rq:` in `requirements.yaml`. They stay in
  their milestone's gate; the milestone can't exit until the RQ is ruled and the requirement passes.

## Where the order comes from

The corpus gives four build orders. They agree, and the milestones follow them.

| Build order | Says | Milestones |
|---|---|---|
| canonical_spec §1, item 6, and §11 ("Milestone/implementation build plan") | "Native-first … prove parity/toolchain → validate physics numerically → build the browser product on top." | M0 (toolchain) → M3–M4 (physics validated natively, then parity on the GPU) → M8 (browser product) |
| debug_tooling_plan, "Build order within Phase 0 (the only forced staggering)" | 0a generation root → 0b synthetic payload harness → 0c kernel debug modes (`UV_PASSTHROUGH` first; `DECODE_PASSTHROUGH` once decode lands; `ROUNDTRIP` once encode lands) → structural views as soon as `RenderQuad` is defined → cross-checks per seam as components arrive. "Build the eyes before the thing seen." | 0a → M0; 0b and `UV_PASSTHROUGH` → M1; `DECODE_PASSTHROUGH`, `ROUNDTRIP` → M2; cross-checks land with their seam (M2–M6) |
| render_contract Part 6, "Why first, concretely" | "Build order stands: contract + SDK → debug catalogue → flat compute → quads → adaptive." | M0 → M1 → M4 → M5 → M6 |
| deep_zoom §3, "Three-layer quadtree" | "Built in three layers, each a working system … This is the build order": 0 flat grid → 1 quad cache + ancestor fallback → 2 adaptive refinement. | M4 → M5 → M6 |

Between the debug catalogue (M1) and flat compute (M4), the physics goes in. Decode and encode come first
(M2), because `DECODE_PASSTHROUGH` and `ROUNDTRIP` need them and the integrator consumes their output. Then
the integrator and events are validated on the CPU (M3), because canonical_spec §1 item 6 puts "prove
parity, branch-determinism, and the toolchain in a native `cargo test`" before the browser. Colour
composition (M7) and the GUI (M8) come last: render_contract's order ends at adaptive, and colour "has no
parity stakes" (canonical_spec §1, item 4). Philosophy §7.8 puts "the GUI, and actually building the thing"
third.

**One open question bears on the order: RQ-25.** Philosophy §7.8 puts the logH experiment and a
refinement mechanism "re-taken from scratch" *before* the build, but canonical_spec §11 says the vertical
slice "collapsed the research phases" and the refinement policy is "settled with evidence". The plan doesn't
choose. As drafted, the logH occupant and its comparison sit in M3 and the refinement policy's open
measurements in M6, so either ruling changes the contents of those milestones, not the order.

---

## M0 — Gates and scaffolding

**No physics.** The gates are built before anything they gate.

- The cargo workspace and crates under `crates/`, with CI running every runner below on every push.
- The substrate toolchain: one Rust source compiled to f32 SPIR-V → WGSL (rust-gpu) and to f64 for the CPU
  (canonical_spec §1, item 2), proven with a trivial kernel natively.
- The test harness, the golden-image runner, the numerical-gate runner and the benchmark runner. Each
  requirement's `verify.method` has a runner here, before the first requirement of that kind exists.
- The profiler CLI skeleton, schema v1 (R-56; dd_telemetry_and_tiers §5, render_gui_spec §G5 "Profiler").
- The contract crate skeleton: the typed surfaces the contracts name, with no behaviour.
- The ledger generator. This is the generation root (debug_tooling_plan step 0a): ledger → generated
  pack/unpack for Rust and WGSL, plus the codegen self-test. Nothing renders yet, and the tests are green.
  Every payload type later is generated from it, so it is the root of the build DAG.
- The process rules that hold from the first commit: the docs as the authority, nothing built from archived
  files, and every pitfall regression requirement that can be checked without physics.

<!-- gate:M0 -->
**Exit gate — 56 requirements** (and every earlier gate still green):

- PAY (20): REQ-PAY-001…020
- GEN (8): REQ-GEN-001…008
- SCHED (1): REQ-SCHED-001
- RENDER (2): REQ-RENDER-001…002
- TOOL (8): REQ-TOOL-001…008
- VAL (9): REQ-VAL-001…009
- SYS (8): REQ-SYS-001…008
<!-- /gate:M0 -->

## M1 — The synthetic payload and the eyes

debug_tooling_plan steps 0b and 0c (first half); render_contract "contract + SDK → debug catalogue".

- CPU-filled `SimState` / `ICDescriptor` / `RenderQuad` buffers. "Screen colours a hand-filled buffer; both
  surfaces green — before any physics."
- The fragment path from `SimState` to colour: `RenderContext`, the render SDK, and lowering of the fragment
  variants that exist so far.
- The field-view and sub-field debug shaders, with their accessor tests. These are the visual unit test of
  the bit layout.
- The `UV_PASSTHROUGH` kernel mode and the coordinate view. This catches a wrong Y convention at the first
  pixel (coordinate_conventions).
- Structural views over a defined `RenderQuad`, before any scheduler fills it.
- The baseline outcome palette that the field views need, pinned by golden images.

<!-- gate:M1 -->
**Exit gate — 70 requirements** (and every earlier gate still green):

- INT (1): REQ-INT-001
- PAY (13): REQ-PAY-021…033
- GEN (4): REQ-GEN-009…012
- RENDER (22): REQ-RENDER-003…024
- COL (5): REQ-COL-001…005
- GUI (1): REQ-GUI-001
- TOOL (20): REQ-TOOL-009…028
- VAL (3): REQ-VAL-010…012
- SYS (1): REQ-SYS-009
<!-- /gate:M1 -->

## M2 — Charts, decode and encode

debug_tooling_plan 0c: "`DECODE_PASSTHROUGH` once the decoder lands, `ROUNDTRIP` once encode lands".

- The chart registry and every chart in chart_reference's build list, with the `validate(u, v)` trait method
  (R-26).
- The decoder (shared CPU/GPU source), canonicalisation, the mirror deadband, and DEGENERATE handling.
- Energy normalisation η_E (R-25).
- Inverse encode, lookup and lock. The lock is chart construction on the CPU, in `SimConfig` (R-69).
- The `DECODE_PASSTHROUGH` and `ROUNDTRIP` modes and the round-trip residual view.
- The chart reference values as unit and property tests.

<!-- gate:M2 -->
**Exit gate — 124 requirements** (and every earlier gate still green):

- DEC (30): REQ-DEC-001…030
- ENC (20): REQ-ENC-001…020
- CHART (36): REQ-CHART-001…036
- INT (2): REQ-INT-002…003
- PAY (2): REQ-PAY-034…035
- GEN (3): REQ-GEN-013…015
- RENDER (3): REQ-RENDER-025…027
- COL (1): REQ-COL-006
- GUI (6): REQ-GUI-002…007
- TOOL (2): REQ-TOOL-029…030
- VAL (12): REQ-VAL-013…024
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
  - re-validating escape precision and recall;
  - δ_dep;
  - the tau gap;
  - the Yoshida-6 coefficient check before Yoshida-6 enters the shared source.
- The CPU-side parity suite (`computeIC`).

<!-- gate:M3 -->
**Exit gate — 149 requirements** (and every earlier gate still green):

- INT (53): REQ-INT-004…056
- EVT (20): REQ-EVT-001…020
- PAY (24): REQ-PAY-036…059
- GEN (1): REQ-GEN-016
- SCHED (2): REQ-SCHED-002…003
- RENDER (1): REQ-RENDER-028
- GUI (1): REQ-GUI-008
- TOOL (9): REQ-TOOL-031…039
- VAL (31): REQ-VAL-025…055
- PERF (4): REQ-PERF-001…004
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
**Exit gate — 84 requirements** (and every earlier gate still green):

- CHART (1): REQ-CHART-037
- INT (15): REQ-INT-057…071
- EVT (1): REQ-EVT-021
- SCHED (10): REQ-SCHED-004…013
- RENDER (8): REQ-RENDER-029…036
- TOOL (6): REQ-TOOL-040…045
- VAL (24): REQ-VAL-056…079
- PERF (9): REQ-PERF-005…013
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
**Exit gate — 131 requirements** (and every earlier gate still green):

- DEC (2): REQ-DEC-031…032
- CHART (2): REQ-CHART-038…039
- INT (1): REQ-INT-072
- EVT (1): REQ-EVT-022
- PAY (9): REQ-PAY-060…068
- GEN (1): REQ-GEN-017
- SCHED (42): REQ-SCHED-014…055
- REF (10): REQ-REF-001…010
- RENDER (20): REQ-RENDER-037…056
- COL (1): REQ-COL-007
- GUI (1): REQ-GUI-009
- TOOL (9): REQ-TOOL-046…054
- VAL (4): REQ-VAL-080…083
- PERF (20): REQ-PERF-014…033
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
**Exit gate — 113 requirements** (and every earlier gate still green):

- DEC (5): REQ-DEC-033…037
- PAY (4): REQ-PAY-069…072
- GEN (1): REQ-GEN-018
- SCHED (16): REQ-SCHED-056…071
- REF (30): REQ-REF-011…040
- GUI (6): REQ-GUI-010…015
- TOOL (4): REQ-TOOL-055…058
- VAL (12): REQ-VAL-084…095
- PERF (34): REQ-PERF-034…067
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
**Exit gate — 95 requirements** (and every earlier gate still green):

- GEN (4): REQ-GEN-019…022
- SCHED (1): REQ-SCHED-072
- RENDER (16): REQ-RENDER-057…072
- COL (39): REQ-COL-008…046
- GUI (16): REQ-GUI-016…031
- TOOL (14): REQ-TOOL-059…072
- VAL (3): REQ-VAL-096…098
- PERF (2): REQ-PERF-068…069
<!-- /gate:M7 -->

## M8 — The dev GUI, tooling, the browser product and the release gates

Philosophy §7.8 item 3: "the GUI, and actually building the thing". canonical_spec §11: "build the browser
product on top".

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
**Exit gate — 199 requirements** (and every earlier gate still green):

- ENC (1): REQ-ENC-021
- CHART (1): REQ-CHART-040
- GEN (1): REQ-GEN-023
- SCHED (1): REQ-SCHED-073
- RENDER (2): REQ-RENDER-073…074
- COL (2): REQ-COL-047…048
- GUI (111): REQ-GUI-032…142
- TOOL (36): REQ-TOOL-073…108
- VAL (13): REQ-VAL-099…111
- PERF (7): REQ-PERF-070…076
- SYS (24): REQ-SYS-039…062
<!-- /gate:M8 -->
