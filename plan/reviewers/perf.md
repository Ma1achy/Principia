# Perf reviewer — checklist

Named on every task that closes a PERF-area requirement or any `benchmark` verify, or touches the frame loop or dispatch
(`plan/WORKFLOW.md` § "Task files"). Performance is a separate concern from parity (`docs/contracts/principia_parity_contract.md` § "7. What this contract does *not* cover");
the rule that runs through this checklist is that a failure is a measurement outcome, reported, never a silent
degradation (`docs/design/principia_dd_telemetry_and_tiers.md` § "6.5 The rule underneath all of these").

## 1. Frame budget and the frame record

- [ ] The frame budget is 16.7 ms as the goal, 41.7 ms as the hard floor (not a goal); a solve falls back toward 41.7 ms only when nothing fits, and records that it had to. `docs/design/principia_dd_telemetry_and_tiers.md` § "3. Percentiles, not means — and the specific thresholds"
- [ ] Frame time is reported as p50, p95, p99 and max, never a mean alone; the headline is `frac_frames_over_41.7ms` during motion, split from static frames by `camera_delta > 0`. `docs/design/principia_dd_telemetry_and_tiers.md` § "3. Percentiles, not means — and the specific thresholds"
- [ ] Every frame record carries the work it did (`quads_computed`, `quads_reused`, `samples`, `substeps_total`, `playhead_dt`, `camera_delta`, `tree_depth_max`) and `stage_ms` over the five stages (integrate / reduce / colour / upload / present). `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful"; `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"
- [ ] The measurement is always on (not compiled out); only the reporting is toggleable; `prin` and the interactive path emit the same frame record. `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode"
- [ ] A perf claim reports which resource bound (samples, bandwidth, memory, dispatch overhead), not only that a frame was slow. `docs/design/principia_dd_telemetry_and_tiers.md` § "4. Deriving the tier instead of choosing it"
- [ ] `prin profile diff base new --threshold P%` is the regression gate, and scenarios are deterministic. `docs/gui/principia_render_gui_spec.md` § "Profiler"; `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"

## 2. The three axes: eps, frame budget, hard cap

- [ ] The tier is three axes — `eps` (convergence target), frame budget (per-frame cap), hard cap (quads/bytes) — solved in that order; the hard cap exists. `docs/design/principia_dd_telemetry_and_tiers.md` § "3.5 THE TIER IS THREE COUPLED AXES, NOT ONE — v0.5"; `docs/design/principia_dd_telemetry_and_tiers.md` § "4. Deriving the tier instead of choosing it"
- [ ] The system always reports which axis bound: converged, budget-bound, or cap-bound; cap-bound says the image is not what was asked for. `docs/design/principia_dd_telemetry_and_tiers.md` § "ALWAYS REPORT WHICH AXIS BOUND"; `docs/design/principia_memory_tiers.md` § "4.1 The same six tiers on the three axes"
- [ ] The hard cap is a fraction of available memory, not an absolute byte count. `docs/design/principia_memory_tiers.md` § "4.1 The same six tiers on the three axes"
- [ ] Per-tier numbers are labelled placeholders in the config; real values come from the calibration campaign as calibration requirements, not from the PR. `docs/design/principia_memory_tiers.md` § "4.1 The same six tiers on the three axes"; `docs/design/principia_dd_telemetry_and_tiers.md` § "7. The calibration campaign — collect everything, decide nothing yet"; `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## 3. Memory tiers and budgets

- [ ] Memory is modelled as `payload = (E+1) × bytes × render_px` plus render targets `k_d × display_px × 4 + k_r × render_px × 4` plus fixed; `render_px = display_px × render_scale²`. `docs/design/principia_memory_tiers.md` § "3. Memory model — payload + render targets"
- [ ] Struct widths match the corpus: `SimState` 144 B (FTLE on) / 96 B (off), 8-byte aligned; word 16 B per copy; `ICDescriptor` 64 B with explicit padding; `QuadReduction` ~80 B. `decisions.md` § "R-40 — Memory tiers and the quality device key off `eps` *(PL-5)*"; `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"; `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- [ ] Allocation is budgeted before it is attempted (`quads × N² × (E+1) × sizeof(SimState)` against adapter limits); if it does not fit, the tier is reduced and reported. `docs/design/principia_dd_telemetry_and_tiers.md` § "6.2a Budgeting before allocating"
- [ ] Memory pressure has three states — comfortable, pressured (stop growing the tree, report cap-bound), reclaiming (evict deepest quads first). `docs/design/principia_dd_telemetry_and_tiers.md` § "Three states, not two"
- [ ] Eviction never drops the coarse ancestors or the pinned fallback chain (baseline cover, visible ancestors, backdrop identity's leaf cover). `docs/design/principia_dd_telemetry_and_tiers.md` § "Eviction order, and the trap in it"; `docs/contracts/principia_caching_contract.md` § "Part 4 — Baseline-first: an absolute tier, not a priority weight"
- [ ] The OOM / pressure path is run with an artificially low cap and shown to stay responsive, keep drawing, report cap-bound and never blank. `docs/design/principia_dd_telemetry_and_tiers.md` § "And test it deliberately"
- [ ] Hard device limits clamp every preset, Custom and named tiers included; a failed forced allocation falls back to a working tier with a message, never a crash or black screen; the red warning never blocks. `docs/design/principia_quality_device_note.md` § "Hard limits clamp EVERY preset (including custom)"; `docs/design/principia_memory_tiers.md` § "7. The "are you sure?" safety system (three severities, none blocking)"
- [ ] Payload buffers are sharded within WebGPU's per-binding and per-buffer limits, not one allocation. `docs/design/principia_memory_tiers.md` § "8. Caveats"
- [ ] The cache holds current state only (one state per entry), under a hard cap sized by device characterisation; no history "for smoothness". `docs/contracts/principia_caching_contract.md` § "Part 7 — The current-state cache (resume points, hard-capped)"; `docs/design/principia_temporal_architecture_note.md` § "Footguns (all re-applications of disciplines already established)"
- [ ] Refinement latches live with the resident quad and never pin memory. `decisions.md` § "R-99 — The latch is per footprint and lives with the resident quad *(closes RQ-59)*"
- [ ] The march updates state in place with no double-buffering, and stability metrics are O(1) accumulators, never time-series. `docs/design/principia_dd_simstate_payload.md` § "7. Memory"; `docs/design/principia_temporal_architecture_note.md` § "Footguns (all re-applications of disciplines already established)"
- [ ] Checkerboard is costed as a compute lever that saves no memory. `docs/contracts/principia_checkerboard_contract.md` § "1. What this is (and what it is NOT)"

## 4. Dispatch and workgroups

- [ ] One workgroup per quad, one thread per texel (64 at N = 8); each ensemble copy is the same kernel dispatched again with `copy_index` a uniform, and no kernel loops over copies; shared memory holds reduction accumulators, not live states. `decisions.md` § "R-102 — The ensemble isn't a baked variant *(closes RQ-62)*"; `decisions.md` § "R-124 — Apply the R-25, R-50 and R-102 follow-ups now *(closes RQ-92)*"; `docs/design/principia_systems_architecture.md` § "The shape"
- [ ] Workgroup storage and invocation use fits the spec ceilings (WebGPU 16 KB / 256; Metal 32 KB / 1024), checked against measured limits, not quoted figures. `docs/design/principia_systems_architecture.md` § "5.5 THE DISPATCH SHAPE — one thread per texel, one dispatch per ensemble copy"; `docs/design/principia_systems_architecture.md` § "Still to check"
- [ ] No worker-tile knob (`64/k` texels per thread) is added without a profile showing a specific need. `docs/design/principia_systems_architecture.md` § "Not doing: worker tiles"
- [ ] Dispatches are bounded (split into chunks that return, carrying state) so no long dispatch trips a watchdog or blocks the compositor. `docs/design/principia_dd_telemetry_and_tiers.md` § "6.3 Device loss, timeouts, driver resets"; `docs/design/principia_dd_telemetry_and_tiers.md` § "GPU"
- [ ] In-flight depth is kept shallow (the scheduler's 2–4 jobs); frames are not queued ahead. `docs/design/principia_dd_telemetry_and_tiers.md` § "GPU"; `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- [ ] A hidden tab or unfocused window stops GPU work (not throttles), and telemetry records it. `docs/design/principia_dd_telemetry_and_tiers.md` § "GPU"
- [ ] CPU pools leave at least one core free (configurable), background work (prebake, export, catch-up) runs at lower priority, and long work yields. `docs/design/principia_dd_telemetry_and_tiers.md` § "CPU"
- [ ] `⌈T/dt_macro⌉ ≤ 65535` is enforced at dispatch. `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"

## 5. The quality controller

- [ ] One arbiter owns every knob write; the steady-state and transition measurements are sensors, not controllers. `docs/design/principia_quality_device_note.md` § "1. One arbiter, many sensors"
- [ ] Knobs are a pure function of a slowly varying throughput model, not a reactive feedback loop. `docs/design/principia_quality_device_note.md` § "2. Model-based, not reactive"
- [ ] The policy's output is one rung on a designed ladder of ~8–12 rungs; the six named tiers are pinned rungs. `docs/design/principia_quality_device_note.md` § "3. The quality ladder"; `decisions.md` § "R-89 — Depth and E are not on the sim key *(closes RQ-40)*"
- [ ] Motion runs at `resting_rung − offset`, the offset playhead-depth-aware. `docs/design/principia_quality_device_note.md` § "4. Resting rung + motion offset (one ladder, two setpoints)"
- [ ] Timescales: per frame, the frame loop absorbs transients with no settings change; during motion, `render_scale` (or E and the refinement floor when locked to native) slides and snaps back at rest; rung changes happen only in sustained quiescence, over minutes. `docs/design/principia_quality_device_note.md` § "5. Timescale cascade (each layer absorbs what's too fast for the one above)"
- [ ] Render-key knobs change under motion or blur cover; sim-key knobs (`N`, FTLE, word) change only at natural invalidation moments; the controller never causes a visible recompute. `docs/design/principia_quality_device_note.md` § "6. Change under cover (the perceptual magic)"
- [ ] Hysteresis: drop fast, rise slow, a dead zone between, and failed-rung memory that decays. `docs/design/principia_quality_device_note.md` § "7. Hysteresis + failed-rung memory"
- [ ] Sensors are sim-time debt and p95 frame time (steady state) and time-to-resync / fallback-visible duration (transitions). `docs/design/principia_quality_device_note.md` § "8. Sense what the user feels, not what the GPU reports"
- [ ] The model persists under a provenance signature (adapter info, limits, probe/kernel version); mismatch re-probes; re-detect forces a fresh probe. `docs/design/principia_quality_device_note.md` § "9. Persist the model, not just the rung"
- [ ] A named tier or Custom switches the arbiter off entirely; the arbiter has a debug overlay showing its reasoning. `docs/design/principia_quality_device_note.md` § "10. Two sanctities: the user, and observability"
- [ ] The arbiter accounts for checkerboard's saving but never toggles it. `docs/contracts/principia_checkerboard_contract.md` § "8. Build-time settles (measure on the real system)"; `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- [ ] `E` is reduced under an active live march (`e_motion_gating`), with full E at rest and in export. `docs/contracts/principia_scheduler_contract.md` § "Part 9 — Ensemble / SSAA sampling (dispatch rules)"

## 6. The motion regime, checkerboard and never-blank

- [ ] Two scheduler regimes: in motion, only the full-canvas coarse cover of the current identity is dispatched (`PREVIEW_MODE`, complexity scoring suspended); at rest, full machinery and completeness resume. `docs/contracts/principia_caching_contract.md` § "Part 5 — The stale backdrop: blur means loading"
- [ ] Must-split above the screen floor is the at-rest target; during a gesture the frame budget governs and ancestors show, so there are never blanks. `decisions.md` § "R-108 — Must-split above the floor is the at-rest target *(closes RQ-68)*"; `decisions.md` § "R-88 — What stops in-view refinement *(closes RQ-39)*"
- [ ] `MAX_REL_DEPTH` caps every split beyond the screen floor (off-screen policy splits included) and is never a stop above the floor in view. `decisions.md` § "R-98 — `MAX_REL_DEPTH` caps every split beyond the screen floor *(closes RQ-58)*"; `decisions.md` § "R-88 — What stops in-view refinement *(closes RQ-39)*"
- [ ] Baseline cover is a hard tier above the priority queue; no refinement dispatches while the view's baseline cover is missing. `docs/contracts/principia_caching_contract.md` § "Part 4 — Baseline-first: an absolute tier, not a priority weight"
- [ ] Preview flotsam evicts first; epoch counters drop stale results instead of painting them. `docs/contracts/principia_caching_contract.md` § "Part 5 — The stale backdrop: blur means loading"; `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- [ ] The refinement floor is a motion lever that snaps back to the pixel floor at rest, not a persistent tier setting. `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- [ ] The canvas is never blank, never lies, never freezes: stale content stays up blurred, and blur is the only "not current" marker (no desaturation or tint). `docs/contracts/principia_caching_contract.md` § "Part 8 — The composed guarantee"; `docs/contracts/principia_caching_contract.md` § "Part 5 — The stale backdrop: blur means loading"
- [ ] Checkerboard marches half the render pixels per frame only while the playhead advances; the whole bundle (nominal, copies, shadows) shares one phase. `docs/contracts/principia_checkerboard_contract.md` § "1. What this is (and what it is NOT)"
- [ ] Checkerboard has three states (default On-permanent; motion-only; off), applies to the live set only, never to catch-up marches, and is forced off in export. `docs/contracts/principia_checkerboard_contract.md` § "3. The three-state control"
- [ ] A stale pixel shows its own `t − dt` value, never a spatial interpolation from neighbours. `docs/contracts/principia_checkerboard_contract.md` § "2. The mechanism: reconstruct-from-previous, resolved through SSAA"
- [ ] Checkerboard self-erases: at rest, pause and both timeline endpoints the stale half is marched one `dt` and the frame is full. `docs/contracts/principia_checkerboard_contract.md` § "5. Self-erasing: catch-up at rest, endpoints, and pause"
- [ ] Checkerboard is applied at render resolution, before the render → display upscale, and stacks with `render_scale`. `docs/contracts/principia_checkerboard_contract.md` § "4. Composition with render-scale (they stack; upscale helps)"
- [ ] The frame loop uses a fixed `dt` per sim step decoupled from render frames; promotion is gated on `quad.t == playhead` at a barrier; catch-up runs off-loop. `docs/contracts/principia_scheduler_contract.md` § "Part 7 — The frame loop (lockstep presentation)"

## 7. Responsiveness

- [ ] The main thread never waits on the GPU, a readback or scheduler work; readbacks are fire-and-forget, one in flight, latest wins; `onSubmittedWorkDone` is never awaited in the loop. `docs/contracts/principia_caching_contract.md` § "Part 6 — The responsiveness invariant: the main thread never waits"
- [ ] Pipelines are created with the async APIs; the finite variant set is precompiled at startup (~10 compute objects, ~6 fragment, 3 compositor); other fragment variants compile on selection with last-valid fallback. `docs/contracts/principia_caching_contract.md` § "Part 6 — The responsiveness invariant: the main thread never waits"; `docs/contracts/principia_lowering_contract.md` § "Part 4 — The precompile rule"
- [ ] Scheduler work (including f64 Jacobians) runs in the worker and is time-budgeted per frame, remainder deferred. `docs/contracts/principia_caching_contract.md` § "Part 6 — The responsiveness invariant: the main thread never waits"
- [ ] The last-coherent-frame snapshot is a GPU-side texture copy; bake uploads are debounced and never land inside the frame callback. `docs/contracts/principia_caching_contract.md` § "Part 6 — The responsiveness invariant: the main thread never waits"
- [ ] The whole frame loop is the wasm engine in a Web Worker via `OffscreenCanvas`; input arrives by SharedArrayBuffer when cross-origin-isolated, else coalesced postMessage, through one interface. `docs/contracts/principia_caching_contract.md` § "Part 6a — The threading model: the render loop lives in a worker (and that worker is the wasm engine)"
- [ ] Hover and click integration run in the dedicated inspector worker, cancel-on-move by generation counter, with a step budget that reports a partial answer when exhausted; dwell releases it. `docs/design/principia_trajectory_viewing.md` § "1. The single mechanism"; `docs/contracts/principia_render_contract.md` § "Part 7 — The hover trace (per-IC trajectory overlay)"
- [ ] Tier-3 overlays cost nothing when off and upload at most the ~16 KB per-visible-quad budget when on. `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders"

## 8. Measure, do not assume

- [ ] Whether `N = 16` fits WebGPU (256 invocations) or `N = 8` is forced is measured on the device, not assumed. `decisions.md` § "R-43 — `N = 16` vs 8 is measured; the thread-count inconsistencies are fixed now *(RS-3)*"; `docs/design/principia_systems_architecture.md` § "Still to check"
- [ ] The checkerboard `dt` ceiling, ramp-vs-gate, spread-extrapolation default and rest catch-up granularity are measured on the real render. `docs/contracts/principia_checkerboard_contract.md` § "8. Build-time settles (measure on the real system)"
- [ ] Render-target buffer counts (`k_d`, `k_r`) are confirmed once the render graph is built. `docs/design/principia_memory_tiers.md` § "8. Caveats"
- [ ] The yellow/red warning thresholds and the memory-fit margin are tuned against feel, not hard-coded. `docs/design/principia_memory_tiers.md` § "7. The "are you sure?" safety system (three severities, none blocking)"
- [ ] The controller's dead zone, settled-window length, rise evidence and failed-rung decay are measured one-number tunables. `docs/design/principia_quality_device_note.md` § "Open sub-questions (settle at implementation)"
- [ ] The marginal return of an extra CPU core is measured before using it. `docs/design/principia_dd_telemetry_and_tiers.md` § "CPU"
- [ ] Sustained, not burst, throughput sets the tier; thermal decline at constant settings is detected. `docs/design/principia_dd_telemetry_and_tiers.md` § "Thermal is the honest budget"
- [ ] No hard-coded device table picks the tier; it is derived from measured throughput. `docs/design/principia_dd_telemetry_and_tiers.md` § "4. Deriving the tier instead of choosing it"

## 9. Benchmarks

- [ ] Every `benchmark` requirement the task closes has a `cargo xtask bench <bench>` acceptance line whose output is in the PR, run against the requirement's threshold or its calibration requirement's proposal. `plan/WORKFLOW.md` § "The unit: one task, one branch, one PR"; `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- [ ] Benchmarks run nightly and at each milestone gate, not on every commit; the gate's run is the one the proposal or threshold is judged on. `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- [ ] A benchmark states the work held constant (fixed slices, zoom ladder, pan path, playhead march) and sweeps settings rather than sampling one. `docs/design/principia_dd_telemetry_and_tiers.md` § "1.1 The fixed suite — comparability across devices"

<!-- list:benchmarks -->
*34 requirements, generated from `plan/requirements.yaml` — do not edit by hand.*

**M3**
- [ ] REQ-PERF-004 — computeIC for a typical IC at t = 50 on one core < 16.7 ms (measured 6.1 ms; 1.6 ms at t = 13)

**M4**
- [ ] REQ-PERF-009 — profile the march kernel at production dispatch granularity; record bound type and spill counts
- [ ] REQ-PERF-011 — profile N = 8 vs N = 16 on a WebGPU target and record the result; a validation check asserts N² ≤ maxComputeInvocationsPerWorkgroup for every tier and Custom value

**M5**
- [ ] REQ-SCHED-023 — deep-zoom gesture landing with dozens of Jacobian quads: frame time stays within budget
- [ ] REQ-RENDER-055 — with every Tier 3 overlay off, no buffer upload or extra pass occurs (frame time equals the no-overlay baseline); on, the buffer is uploaded only while active
- [ ] REQ-TOOL-053 — release build emits stage timings; measured instrumentation overhead is negligible against 16.7 ms
- [ ] REQ-PERF-017 — time from slice change to first presented frame is one root-quad dispatch; frames keep presenting while refinement is incomplete
- [ ] REQ-PERF-019 — record peak process memory for the tier configs on a 16 GB unified-memory machine
- [ ] REQ-PERF-024 — measured input-to-photon latency with in-flight depth recorded in telemetry
- [ ] REQ-PERF-028 — config marks the six rows as placeholders; eps strictly decreases Potato → Extreme; the calibration campaign's recorded per-device results replace them
- [ ] REQ-PERF-093 — the proposal records the margin with its method: estimator figures against measured process memory across tiers and resolutions; recorded in decisions.md
- [ ] REQ-SYS-031 — under heavy scheduler load, main-thread frame/input latency shows no hitch (record max frame time)
- [ ] REQ-SYS-032 — navigation fixture: count dispatches until first current cover; assert no blank frame

**M6**
- [ ] REQ-SCHED-068 — scripted camera paths: compare in-view quads and stall (px texel size) for naive cull, widened margin and refill-derived margin; refill-derived must beat widened margin, and naive cull (170 px, 4 in view) is the known failure
- [ ] REQ-REF-032 — calibrate tau per the tolerance grid (charts × horizons × eps) at the first refine milestone and record it; config has no tau = k·eps derivation
- [ ] REQ-VAL-085 — motion trace with all three active: reconstruction error and visual quality recorded against each alone
- [ ] REQ-VAL-090 — scripted zoom over two octaves on config_stability and preset_shape_h1: converged in-view texel size stays flat (~1.5 px) and max_depth tracks camera target depth; quad count stays far below the alpha_lo = 0 degeneration (+49% / +222%)
- [ ] REQ-VAL-092 — a recorded results entry covers each of the three measurements
- [ ] REQ-PERF-038 — device-characterisation runs record the thresholds and budget heuristics used
- [ ] REQ-PERF-039 — on a device that cannot hold 16.7 ms, the derived tier records the fallback
- [ ] REQ-PERF-066 — record each tunable's value and the measurement that set it
- [ ] REQ-SYS-038 — measure deep-quad occupancy/register pressure with the two-path branch vs a baked linearised variant; record the result

**M7**
- [ ] REQ-PERF-068 — count pipeline objects created at startup and measure startup time
- [ ] REQ-PERF-069 — count display-res and render-res targets allocated by the built render graph and record k_d, k_r

**M8**
- [ ] REQ-TOOL-078 — exporting 600 frames keeps peak memory flat (independent of frame count); the UI stays responsive; cancel stops the job
- [ ] REQ-TOOL-087 — the suite replays deterministically and reports per-setting results across the sweep
- [ ] REQ-TOOL-090 — a long constant-scene run flags a throttling inference when fps declines monotonically
- [ ] REQ-VAL-101 — campaign records per device across all settings; derived boundaries recorded
- [ ] REQ-PERF-070 — frame time of the fill-in frame on the weakest target device, recorded
- [ ] REQ-PERF-073 — run Potato on a phone already owned; record frame rate and responsiveness
- [ ] REQ-PERF-074 — record the measured thresholds with their method
- [ ] REQ-PERF-078 — the proposal shows the total_substeps distribution (p1 to p99) of hover ICs and the per-frame cost of the chosen budget against the 60 fps frame
- [ ] REQ-PERF-080 — the proposal shows visible-quad counts measured over the deep-zoom scenarios and the resulting buffer size with the defined per-quad record
- [ ] REQ-SYS-060 — frame times with continuous hover + listen equal the no-hover baseline within noise; the work runs on the inspector worker
<!-- /list:benchmarks -->
