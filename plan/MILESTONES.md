# Milestones — M0 to M8

The build plan's milestones, in dependency order. Each has a scope, the build order it comes from, and an exit
gate stated as requirement ids from `plan/requirements.yaml`. The gate blocks are generated
(`plan/tools/milestones.py`); the prose is not.

## Assumptions

- **The implementation lives in this repo, in `crates/` next to `docs/`.** The corpus names two crates, the
  engine crate and the gui crate (canonical_spec §1, item 5: "the engine crate exposes only the typed state
  surface `pub`"). The rest of the layout is confirmed by R-146 (plan/WORKFLOW.md, "Conventions"); there is no separate
  contract crate (R-172).
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
| canonical_spec §1, item 6, and §11 ("Milestone/implementation build plan") | "Native-first … prove parity/toolchain → validate physics numerically → build v1 web on top" (R-571; *was:* "build the browser product on top") | M0 (toolchain) → M3–M4 (physics validated natively, then parity on the GPU) → M8 (the native app and v1 web, R-571) |
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

- The cargo workspace and crates under `crates/`, with CI running each runner below at the cadence
  `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*" gives it.
- The substrate toolchain: one Rust source compiled to f32 SPIR-V → WGSL (rust-gpu) and to f64 for the CPU
  (canonical_spec §1, item 2), proven with a trivial kernel natively.
- The test harness, the golden-image runner, the numerical-gate runner, the benchmark runner and the GUI
  screenshot runner (`decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"). Each
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
**Exit gate — 122 requirements** (and every earlier gate still green):

- PAY (22): REQ-PAY-001…005, REQ-PAY-007…020, REQ-PAY-087, REQ-PAY-091…092
- GEN (13): REQ-GEN-001…008, REQ-GEN-024, REQ-GEN-028…029, REQ-GEN-031…032
- SCHED (1): REQ-SCHED-001
- RENDER (3): REQ-RENDER-001…002, REQ-RENDER-083
- TOOL (20): REQ-TOOL-001…008, REQ-TOOL-119…121, REQ-TOOL-134, REQ-TOOL-136, REQ-TOOL-138…139, REQ-TOOL-141, REQ-TOOL-144…145, REQ-TOOL-147…148
- VAL (38): REQ-VAL-001…009, REQ-VAL-138, REQ-VAL-147…167, REQ-VAL-169, REQ-VAL-175…176, REQ-VAL-178…181
- SYS (25): REQ-SYS-001…008, REQ-SYS-063…079
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
- **Also at the M1 gate (R-386):** REQ-COL-061, the hatch's colours distinguishable from the palette under each
  colour-vision simulation, its criterion confirmed by the human beside REQ-COL-055. It is in M7's generated gate
  because TASK-M7-20, the M7 task that builds the simulation, closes it, and no task closes an earlier milestone's
  requirement. The M1 gate does not pass until REQ-COL-061 is green on `main`, so **TASK-M7-20 merges before the M1
  gate**.
  **At the gate (R-429):** the criterion is confirmed beside REQ-COL-055: τ is the OKLab distance from `#FF00FF` to
  `#E034C6`, 0.101; the hatch passes by colour, or by its pattern when the two simulated stripes are more than τ apart.
  The pattern argument holds at 1:1 device pixels only; a downscaled export blends the stripes to a flat colour, which
  under achromatopsia sits on the grey ramp.
- **Also at the M1 gate (R-423): physics checks the six fields R-400 moved to cool-warm** (PR #195, TASK-M1-17):
  `q_mass`, `ftle`, `mean_y`, `S`, `theta` and `diffusion`. Before the gate report, the orchestrator dispatches a
  read-only physics check of `q_mass`, `ftle`, `mean_y`, `S` and `diffusion`: that each is a signed quantity whose
  zero is a meaningful centre for a diverging ramp centred at zero. A field that fails goes to `REVIEW_QUEUE.md` for
  the human, and the gate report gives the result for each. `theta` is an angle, so it moves to the cyclic ramp
  (TASK-M1-20, REQ-RENDER-085), an M1 task the gate waits on.
  **The check's result** (run early, in physics review 5484757609 of PR #197): `S`, `ftle` and `diffusion` pass, each
  signed at finite time with zero a meaningful centre, none an angle. `q_mass` and `mean_y` are not confirmed:
  `q_mass`'s formula (REQ-PAY-088, M2) and `mean_y`'s `y` (REQ-PAY-073, M3) are undefined, and each ledger range is
  the unbounded default. They were **RQ-274**, ruled by **R-426**: both stay on `ramp_coolwarm` provisionally, and
  the gate report gives them as provisional, neither confirmed nor failed; TASK-M2-03 (REQ-PAY-097) and TASK-M3-13
  (REQ-PAY-098) declare their ranges, and physics re-checks each ramp then. R-426 also makes an undeclared range a
  ledger gap, not one that spans zero: TASK-M1-21 (REQ-GEN-035), an M1 task the gate waits on, declares the ranges the
  corpus settles, and the gate report gives its list of the gaps that remain. `C_ty`, on cool-warm by the same
  default and not among the six, was **RQ-277**, decided per R-369 (option (a), PR #205): signed, as `diffusion` is,
  since `C_ty = diffusion · C_tt(n)` with `C_tt(n) > 0`; it stays on `ramp_coolwarm`, and the gate report gives it
  as confirmed with the human's veto open.
  **At the gate (R-430):** the ramps are as reported: `S`, `ftle` and `diffusion` pass; `q_mass` and `mean_y` are
  provisional on cool-warm, re-checked at M2 and M3; `theta` is cyclic; `C_ty` stands as decided per R-369, no veto.
- **Also at the M1 gate (R-427): the constants register's provisional entries.** The gate report lists each register
  entry marked provisional until M1 (TASK-M1-18's six per-field log floors, and the hatch's colours and stripe width),
  and the human confirms or changes each; an entry still so marked blocks the gate. At their confirmation
  `rho_ratio`'s and `r_min_pair_0`'s floors are re-marked provisional until M2 (R-419, REQ-TOOL-163). The report also
  gives TASK-M1-18's statement of whether `d_min`'s floor at 2⁻¹⁴, f16's smallest normal, is intended when `d_min` is
  stored down to 2⁻²⁴ (R-271), so that the closest approaches land at the bottom of the ramp, with its evidence.
  **At the gate (R-428):** the nine are confirmed as proposed. Seven are confirmed register entries citing R-428:
  `log_floor_d_min` 2⁻¹⁴, `log_floor_dE_max` and `log_floor_dLz_max` 2⁻²⁰, `log_floor_closure_min` 2⁻²³,
  `hatch_violet` `#9B00FF`, `hatch_cyan` `#48FFFF` and `hatch_half_period` 4 px. `log_floor_rho_ratio` 2⁻²³ and
  `log_floor_r_min_pair_0` 2⁻¹⁴ are marked provisional until M2, citing REQ-TOOL-163. `d_min`'s floor at 2⁻¹⁴ is
  intended: 2⁻²⁰ would compress `d_min` ∈ [6e-5, 1] from half the ramp to an eighth of it. The six M1 calibrations
  that are not register entries, which the message does not name, are RQ-279.
  **At the gate (R-433 to R-435, 15 Oct 2026; RQ-279 closed):** `r_pole` = 0.01 (REQ-INT-086), now a register entry
  marked provisional until M3; the running grey `#4E4E4E` (REQ-COL-053); the triple swatches `#D6A1FF` and `#000097`
  (REQ-COL-062), with no veto on a first-step triple collision in the t=0 orange; the banding bound, 1/16 of a step
  (REQ-TOOL-152); the ‖n‖ = 1 tolerance, 64 U = 2⁻¹⁸ (REQ-VAL-122), the register's `n_norm_tol`; all confirmed as
  proposed (R-433). The stipple (REQ-COL-064) is confirmed with its dots changed to `#909090`, so that a 4 × 4 block
  clears τ = 0.101 (0.115), and the seven goldens that draw it are re-rendered (R-434). The 131 baselines recorded
  for the gate are confirmed (R-435).
- **The M1 gate passed (R-432, 14 Oct 2026)** on gate run 38629476853 on `main` at ab65c273 (230 of 230 requirements
  passing, `gate-provisional` red only for the nine entries R-428 confirms) and R-428 to R-431. `main` is tagged
  `m1-gate` once R-428's rulings PR has merged, and M2 starts.

<!-- gate:M1 -->
**Exit gate — 108 requirements** (and every earlier gate still green):

- INT (2): REQ-INT-001, REQ-INT-086
- PAY (14): REQ-PAY-021…033, REQ-PAY-090
- GEN (7): REQ-GEN-009…012, REQ-GEN-027, REQ-GEN-033, REQ-GEN-035
- RENDER (27): REQ-RENDER-003…024, REQ-RENDER-075, REQ-RENDER-077, REQ-RENDER-084…086
- COL (14): REQ-COL-001…005, REQ-COL-053, REQ-COL-055…056, REQ-COL-062…064, REQ-COL-067…069
- GUI (1): REQ-GUI-001
- TOOL (34): REQ-TOOL-009…028, REQ-TOOL-122…124, REQ-TOOL-131, REQ-TOOL-137, REQ-TOOL-152…157, REQ-TOOL-160…162
- VAL (5): REQ-VAL-010…012, REQ-VAL-122, REQ-VAL-182
- SYS (4): REQ-SYS-009, REQ-SYS-080…082
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
- The outcome view conformed to R-444 to R-448 (TASK-M2-32): nine physical classes, triple collision `#F29620` and
  triple ejection `#611597`, `running` and every failure drawn `#141418`, a forbidden chart region hatched from the
  chart's classification (REQ-COL-102), the OKLab-uniform swatch set, and `K = 9` in greyscale and hue mode. The
  separation evidence (REQ-COL-104, REQ-COL-105, R-71) is filed in M2; which gate confirms it, and the re-rendered
  goldens' decision, are RQ-330's.
  R-454 (closing RQ-330): the M2 gate confirms the new swatches and that evidence, and the three re-rendered
  goldens are re-baselined under R-444 to R-446.
- R-504 (closing RQ-373): the four goldens R-467 changes are re-baselined by TASK-M2-34; the M2 gate confirms the
  canvas background's value and the changed tiles with REQ-COL-110's NaN / Inf style, which is not magenta (R-505).
- Display names (R-449; TASK-M2-33): one conversion function names what a person reads, the bodies 1, 2, 3 and a pair
  by its two bodies, never its pair id, on every surface built so far; machine-readable exports stay 0-based and say
  so in their header or schema.

<!-- gate:M2 -->
**Exit gate — 217 requirements** (and every earlier gate still green):

- DEC (41): REQ-DEC-001…030, REQ-DEC-038…041, REQ-DEC-043…048, REQ-DEC-050
- ENC (34): REQ-ENC-001…020, REQ-ENC-022…027, REQ-ENC-029…031, REQ-ENC-034…038
- CHART (67): REQ-CHART-001…036, REQ-CHART-043…051, REQ-CHART-053, REQ-CHART-055, REQ-CHART-058, REQ-CHART-060…070, REQ-CHART-073, REQ-CHART-075…078, REQ-CHART-081…082, REQ-CHART-089
- INT (2): REQ-INT-002…003
- PAY (5): REQ-PAY-034…035, REQ-PAY-088, REQ-PAY-095, REQ-PAY-097
- GEN (13): REQ-GEN-013…015, REQ-GEN-025…026, REQ-GEN-034, REQ-GEN-036…042
- RENDER (3): REQ-RENDER-025…027
- COL (12): REQ-COL-006, REQ-COL-057, REQ-COL-060, REQ-COL-070, REQ-COL-101…105, REQ-COL-108…110
- GUI (7): REQ-GUI-002…007, REQ-GUI-188
- TOOL (5): REQ-TOOL-029…030, REQ-TOOL-163, REQ-TOOL-188…189
- VAL (18): REQ-VAL-013…024, REQ-VAL-118, REQ-VAL-120…121, REQ-VAL-174, REQ-VAL-192…193
- SYS (10): REQ-SYS-010…016, REQ-SYS-083…085
<!-- /gate:M2 -->

## M3 — The integrator, events and physics validation (CPU, native)

canonical_spec §1 item 6 and §11: "validate physics numerically", natively, before the browser.

- Stepper and regularisation occupants (Heggie default; AZ; logH; none), with the split reversibility flags
  (R-500). Substep control with the N_sub bucket lookup over the threshold table generated per sim key (R-502;
  *was:* "the frozen N_sub bucket lookup"); its maximum length is confirmed at this gate (REQ-INT-094). The fixed `dt_macro` march.
- Terminal events: collision, escape (closure + energy), ionisation, precedence, and the termination checks.
- The full `SimState` accumulator fill on the CPU.
- The validation orbits and ground truth. The independent convergence reference. The Inspector's RK45
  reference (R-33).
- The physics measurements the corpus leaves to the build:
  - re-validating escape precision and recall against check 2's independent ground truth, with the legacy
    t = 30 set as a comparison (R-95);
  - re-confirming `r_pole` (REQ-INT-086) against real orbits' near-pole arcs: confirmed at 0.01 at the M1 gate and
    marked provisional until M3, it blocks the M3 gate's `gate-provisional` job until the human re-confirms it
    (R-433; TASK-M3-11);
  - the logH falsification check of the re-registration mechanism (R-74);
  - δ_dep;
  - the tau gap;
  - the Yoshida-6 coefficient check before Yoshida-6 enters the shared source.
- The CPU-side parity suite (`computeIC`).

<!-- gate:M3 -->
**Exit gate — 227 requirements** (and every earlier gate still green):

- ENC (1): REQ-ENC-033
- CHART (4): REQ-CHART-059, REQ-CHART-072, REQ-CHART-085…086
- INT (83): REQ-INT-004…032, REQ-INT-034…056, REQ-INT-073…074, REQ-INT-076…082, REQ-INT-084…085, REQ-INT-087…092, REQ-INT-094…095, REQ-INT-097…106, REQ-INT-108…109
- EVT (25): REQ-EVT-001…020, REQ-EVT-023…026, REQ-EVT-032
- PAY (31): REQ-PAY-036…059, REQ-PAY-070, REQ-PAY-073, REQ-PAY-083, REQ-PAY-086, REQ-PAY-098, REQ-PAY-110…111
- GEN (1): REQ-GEN-016
- SCHED (2): REQ-SCHED-002…003
- RENDER (1): REQ-RENDER-028
- GUI (1): REQ-GUI-008
- TOOL (14): REQ-TOOL-031…039, REQ-TOOL-125, REQ-TOOL-132, REQ-TOOL-149…150, REQ-TOOL-219
- VAL (55): REQ-VAL-025…055, REQ-VAL-115, REQ-VAL-117, REQ-VAL-119, REQ-VAL-123…124, REQ-VAL-126…136, REQ-VAL-139, REQ-VAL-146, REQ-VAL-168, REQ-VAL-170…171, REQ-VAL-190…191, REQ-VAL-195
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
**Exit gate — 106 requirements** (and every earlier gate still green):

- CHART (3): REQ-CHART-037, REQ-CHART-052, REQ-CHART-074
- INT (16): REQ-INT-057…071, REQ-INT-083
- EVT (1): REQ-EVT-021
- PAY (2): REQ-PAY-074, REQ-PAY-085
- SCHED (12): REQ-SCHED-004…013, REQ-SCHED-084, REQ-SCHED-097
- RENDER (11): REQ-RENDER-029…036, REQ-RENDER-078, REQ-RENDER-082, REQ-RENDER-099
- TOOL (8): REQ-TOOL-040…045, REQ-TOOL-126…127
- VAL (30): REQ-VAL-056…079, REQ-VAL-112, REQ-VAL-125, REQ-VAL-137, REQ-VAL-140…141, REQ-VAL-177
- PERF (13): REQ-PERF-005…013, REQ-PERF-077, REQ-PERF-081, REQ-PERF-086, REQ-PERF-094
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
**Exit gate — 179 requirements** (and every earlier gate still green):

- DEC (3): REQ-DEC-031…032, REQ-DEC-036
- CHART (3): REQ-CHART-038…039, REQ-CHART-041
- INT (2): REQ-INT-072, REQ-INT-075
- EVT (1): REQ-EVT-022
- PAY (20): REQ-PAY-006, REQ-PAY-060…068, REQ-PAY-075…079, REQ-PAY-089, REQ-PAY-093…094, REQ-PAY-096, REQ-PAY-109
- GEN (2): REQ-GEN-017, REQ-GEN-030
- SCHED (53): REQ-SCHED-014…052, REQ-SCHED-054…055, REQ-SCHED-074…076, REQ-SCHED-079, REQ-SCHED-083, REQ-SCHED-086…088, REQ-SCHED-098…100, REQ-SCHED-103
- REF (13): REQ-REF-001…010, REQ-REF-045, REQ-REF-052…053
- RENDER (22): REQ-RENDER-037…056, REQ-RENDER-076, REQ-RENDER-079
- COL (2): REQ-COL-007, REQ-COL-071
- GUI (1): REQ-GUI-009
- TOOL (19): REQ-TOOL-046…054, REQ-TOOL-116…117, REQ-TOOL-130, REQ-TOOL-133, REQ-TOOL-140, REQ-TOOL-158, REQ-TOOL-164…166, REQ-TOOL-187
- VAL (6): REQ-VAL-080…083, REQ-VAL-172…173
- PERF (24): REQ-PERF-014…033, REQ-PERF-082, REQ-PERF-087, REQ-PERF-093, REQ-PERF-095
- SYS (8): REQ-SYS-030…037
<!-- /gate:M5 -->

## M6 — Adaptive refinement and deep zoom (deep_zoom layer 2)

render_contract "→ adaptive"; deep_zoom §3 layer 2 and §4.

- The refinement policy (`Policy::Tolerance`, R-15), its stopping conditions, and the quality
  controller.
- The relevance terms.
- The decode switchover and `AT_F32_FLOOR`. The linearised decoder's x₀ and J_D arrive in M5 (R-113).
- The |det J_D| measure weight.
- The integration-floor flags and the precision warnings.
- The symbolic-dynamics per-quad quantities.
- The refinement policy's open measurements.
- **The GUI track on a mock engine (R-390):** TASK-M6-24 to TASK-M6-29, with TASK-M6-30 and TASK-M6-31 from the human's
  feedback (R-406, R-409) and TASK-M6-32 to TASK-M6-36 from it again (R-450 to R-453) with TASK-M6-37 (R-451 B11), and TASK-M6-38 to TASK-M6-44 from it a third time (R-457 to R-463) with TASK-M6-45 (R-466), the dev GUI built against a mock engine with
  one contract conformance suite for the mock and the real engine. It sits in M6 because TASK-M6-21, TASK-M6-22,
  TASK-M7-22 and TASK-M7-25 build on it and no task may depend on a later milestone's; it starts now, on spare agent
  slots, merges without waiting for the gates of M1 to M5, and never holds up M1 or M2.

<!-- gate:M6 -->
**Exit gate — 223 requirements** (and every earlier gate still green):

- DEC (6): REQ-DEC-033…035, REQ-DEC-037, REQ-DEC-042, REQ-DEC-049
- PAY (5): REQ-PAY-069, REQ-PAY-080…082, REQ-PAY-084
- GEN (1): REQ-GEN-018
- SCHED (29): REQ-SCHED-056…071, REQ-SCHED-077…078, REQ-SCHED-080…082, REQ-SCHED-089…096
- REF (38): REQ-REF-011…037, REQ-REF-039…040, REQ-REF-042…044, REQ-REF-046…051
- GUI (87): REQ-GUI-010…015, REQ-GUI-162, REQ-GUI-165…179, REQ-GUI-181…183, REQ-GUI-189…226, REQ-GUI-229, REQ-GUI-231…240, REQ-GUI-251…252, REQ-GUI-254…260, REQ-GUI-262, REQ-GUI-265, REQ-GUI-346…347
- TOOL (5): REQ-TOOL-055…058, REQ-TOOL-159
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
- The display chain: stain → style → display scale → gamut clamp → colour-vision simulation → screen (R-67), the Print
  graph standing where style was (R-485).
- Image embedding.
- **Before the M7 gate (R-385):** the hybrid embedding (`k = 25`) re-measured on real rendered figures, and
  §5's ~8 % reconciled with the 37.6 % raw low-bit error measured on random canvases (REQ-TOOL-151, TASK-M7-34).
- **Chazy final motion (R-442):** the classifier, its derived fields and its stains (`principia_dd_chazy_final_motion.md`;
  TASK-M7-35 to TASK-M7-40), after the M3 event tasks and the M7 backbone tasks they use (RQ-316, as R-548 confirms).
  Its calibrations (ε, `h_far`, `os_ratio`, settling, `I_max`'s quantisation, the palettes, the energy lightness range
  and `q`'s compaction) are confirmed at this gate, the seven-class palette staying the default (R-548, RQ-311). The
  Chazy entries are ruled (RQ-311 to RQ-318 and RQ-320 to RQ-325 by R-548, RQ-319 by R-467, RQ-310 by R-545); RQ-521
  is ruled by R-595 (a guarded E < 0 state goes on to the hierarchy step). *Was:* "RQ-521 stays open, holding
  REQ-PAY-099's one clause and so the gate until it is ruled."
  The one-body views (R-443; TASK-M7-41, with `chazy_escaper` and `chazy_qk` in TASK-M7-36): their palettes and the
  mass tie tolerance are confirmed at this gate too; the escaping body is `chazy_escaper` (R-548, RQ-321) and the grey
  `#8A8A90` stays, near-neutral (R-548, RQ-322).
- **The Print graph (R-485 to R-492, R-524 to R-536, R-557; `principia_print_graph.md`):** its data model, registry and
  executor in style's place (TASK-M7-54, TASK-M7-26, TASK-M7-55); the lab's 78 node types ported one category at a
  time against the lab's renders, each lab preset in the port task of the last category its nodes need (TASK-M7-56 to
  TASK-M7-64); R-130's eight styles as the Poster family (TASK-M7-65); the field inputs, under physics review
  (TASK-M7-66); `through_medium` in the line renderer (TASK-M7-67); and the cost classes measured (TASK-M7-68). Its
  calibrations (the port and scale tolerances, the data-driven presets' windows, the cost-class thresholds) are
  confirmed at this gate. R-618 rules RQ-538: `device_px` nodes and the sheet run in a
  second pass at screen resolution (REQ-RENDER-110, REQ-RENDER-120; TASK-M7-69, a leaf). *Was:* "RQ-538 holds
  REQ-RENDER-110 (TASK-M7-69, a leaf), and so this gate, until it is ruled."

<!-- gate:M7 -->
**Exit gate — 260 requirements** (and every earlier gate still green):

- EVT (5): REQ-EVT-027…031
- PAY (10): REQ-PAY-099…108
- GEN (5): REQ-GEN-019…022, REQ-GEN-043
- SCHED (1): REQ-SCHED-072
- RENDER (45): REQ-RENDER-057…072, REQ-RENDER-080…081, REQ-RENDER-087…096, REQ-RENDER-101…115, REQ-RENDER-118, REQ-RENDER-120
- COL (125): REQ-COL-008…036, REQ-COL-038…046, REQ-COL-049…052, REQ-COL-054, REQ-COL-058…059, REQ-COL-061, REQ-COL-065…066, REQ-COL-072…100, REQ-COL-106…107, REQ-COL-112…120, REQ-COL-122…134, REQ-COL-136…159
- GUI (23): REQ-GUI-016…031, REQ-GUI-151…152, REQ-GUI-335…338, REQ-GUI-340
- TOOL (30): REQ-TOOL-059…072, REQ-TOOL-109…113, REQ-TOOL-118, REQ-TOOL-142, REQ-TOOL-151, REQ-TOOL-171…174, REQ-TOOL-179…181, REQ-TOOL-186
- VAL (11): REQ-VAL-096…098, REQ-VAL-145, REQ-VAL-183…189
- PERF (5): REQ-PERF-068…069, REQ-PERF-096…097, REQ-PERF-113
<!-- /gate:M7 -->

## M8 — The dev GUI, tooling, v1 web and the release gates

canonical_spec §11: "build v1 web on top" (R-571; *was:* "build the browser product on top"). M8 ships the native app
and v1 web, the whole native app compiled to run in the browser, with the same interface, features and stains, on
the GitHub Pages site (TASK-M8-41). v2 web, a purpose-built browser product designed later as its own thing, is in
no milestone yet (R-571). *Was (title):* "The dev GUI, tooling, the browser product and the release gates".

- The dev GUI against the artboards in `docs/gui/design/`. R-68 applies: the artboards set layout, and
  corpus values win.
- The `gui_state_contract` schema, undo rules and firewall.
- Trajectory viewing, the IC scratchpad, and pointer channels.
- Export and animation.
- The full profiler and debug tooling UI.
- The frame budget, end to end.
- The browser build, v1 web (R-571): the whole native app in the browser; the wasm engine worker, `OffscreenCanvas`,
  and the two membranes: the three crossings for engine state plus an enumerated platform list, both checked in
  CI (R-619), and egui in its own wasm context on the main thread, the engine in its worker (R-620). *Was:* "and the
  two membranes, as written until RQ-550 (what v1 web crosses) and RQ-551 (where its GUI runs) are ruled."
- The non-Metal parity run: the standing pre-Paper-2 action (canonical_spec §11).
- The Chazy legends (R-442; TASK-M8-45), on the generated legend.
  With R-443's one-body legends and the three-body panels, one figure split into three (R-548, RQ-323).
- **The Print graph in the GUI (R-485 to R-492, R-557):** the Print panel and the line sets' "Through the medium",
  the graph editor, the badge and Styled / Plain export, provenance and round trip, degrade while moving and the print
  stage's budget (TASK-M8-68 to TASK-M8-73). `settle_ms`, the print share and the thumbnail budget are confirmed at
  this gate.

<!-- gate:M8 -->
**Exit gate — 456 requirements** (and every earlier gate still green):

- ENC (3): REQ-ENC-021, REQ-ENC-028, REQ-ENC-032
- CHART (11): REQ-CHART-040, REQ-CHART-054, REQ-CHART-056…057, REQ-CHART-071, REQ-CHART-079…080, REQ-CHART-083…084, REQ-CHART-087…088
- INT (3): REQ-INT-093, REQ-INT-096, REQ-INT-107
- GEN (1): REQ-GEN-023
- SCHED (5): REQ-SCHED-073, REQ-SCHED-101…102, REQ-SCHED-104…105
- REF (1): REQ-REF-054
- RENDER (8): REQ-RENDER-073…074, REQ-RENDER-097…098, REQ-RENDER-100, REQ-RENDER-116…117, REQ-RENDER-119
- COL (7): REQ-COL-047…048, REQ-COL-111, REQ-COL-121, REQ-COL-135, REQ-COL-160…161
- GUI (255): REQ-GUI-032…047, REQ-GUI-049…150, REQ-GUI-153…161, REQ-GUI-163…164, REQ-GUI-180, REQ-GUI-184…187, REQ-GUI-227…228, REQ-GUI-230, REQ-GUI-241…250, REQ-GUI-253, REQ-GUI-261, REQ-GUI-263…264, REQ-GUI-266…283, REQ-GUI-308…334, REQ-GUI-339, REQ-GUI-341…345, REQ-GUI-348…400
- TOOL (82): REQ-TOOL-073…092, REQ-TOOL-094…108, REQ-TOOL-114…115, REQ-TOOL-128…129, REQ-TOOL-135, REQ-TOOL-143, REQ-TOOL-146, REQ-TOOL-167…170, REQ-TOOL-175…178, REQ-TOOL-182…183, REQ-TOOL-185, REQ-TOOL-190…218
- VAL (23): REQ-VAL-099…103, REQ-VAL-105…111, REQ-VAL-113…114, REQ-VAL-116, REQ-VAL-143…144, REQ-VAL-194, REQ-VAL-196…200
- PERF (27): REQ-PERF-070…076, REQ-PERF-078…080, REQ-PERF-091…092, REQ-PERF-098…112
- SYS (30): REQ-SYS-039…062, REQ-SYS-086…091
<!-- /gate:M8 -->
