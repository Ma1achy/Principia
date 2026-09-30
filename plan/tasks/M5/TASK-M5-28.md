# TASK-M5-28 — The frame record, percentiles and the bounded telemetry file

- **Milestone:** M5
- **Closes:** REQ-TOOL-050, REQ-TOOL-051, REQ-TOOL-052, REQ-TOOL-053, REQ-TOOL-054, REQ-TOOL-117, REQ-TOOL-130, REQ-TOOL-140
- **Depends on:** TASK-M5-21
- **Needs (earlier milestones):** REQ-TOOL-001, REQ-TOOL-002, REQ-TOOL-005, REQ-TOOL-006, REQ-TOOL-008
- **Reviewers:** code, qa, perf, physics
- **Pitfalls:** none
- **Size:** ~520 lines

## Goal
Every frame the loop emits a record with frame_ms, quads_computed, quads_reused, samples, substeps_total,
playhead_dt, camera_delta, tree_depth_max (and leaf count) and stage_ms over integrate / reduce / colour / upload / present,
measured in every build and never compiled out. Reports give p50 / p95 / p99 / max, split moving from still frames by
camera_delta > 0 and headline frac_frames_over_41.7ms during motion. The file size is bounded by downsampling or idle
roll-up, the scheme and its N proposed as a calibration. `prin` batch renders emit the same record (camera_delta = 0, no
present stage) and one parser reads both. The frame record also carries R-288's two `d_min` counters: on the GPU an
atomic u32 buffer bound to each frame's dispatch and reset each frame, read back asynchronously with the telemetry
readback (R-294).

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "3. Percentiles, not means — and the specific thresholds"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/gui/principia_render_gui_spec.md` § "Profiler"
- `decisions.md` § "R-288 — R-281's counters: two per-frame atomic u32 counters in telemetry §2, on the existing readback *(closes RQ-171)*"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"
- `decisions.md` § "R-294 — R-288's counters belong to the frame; no mutable statics in the kernel *(amends R-288; closes RQ-174)*"

## Deliverables
- Doc change: `docs/gui/principia_render_gui_spec.md` § "Profiler" — the `deep_zoom_03` scenario (chart, camera path, depth and frame count) (definition, REQ-TOOL-130), with the "Removed lines" note; the scenario registered with `prin profile`.
- `crates/engine/src/telemetry/{record.rs, writer.rs}`; `crates/prin/src/profile/report.rs` (percentiles).
- Tests `crates/engine/tests/telemetry.rs`, `crates/prin/tests/report.rs`.
- `cargo xtask bench telemetry-overhead` (release build).
- The size-bound calibration proposal: file sizes of a long 60 fps session with idle stretches.
- The per-frame `d_min` counter buffer (R-294, REQ-TOOL-140): an atomic u32 storage buffer holding `dmin_nan_unset` and
  `dmin_negative_floored`, bound to each frame's dispatch and reset each frame; its asynchronous readback with the
  telemetry readback, never stalling the frame; the counters' values in that frame's record. Test
  `crates/engine/tests/dmin_counters.rs`.
- `crates/kernel` (R-294, REQ-TOOL-140): the survey entry point binds the per-frame `d_min` counter buffer, and the
  `d_min` packer increments it atomically on the GPU; one packer path for the f32 and f64 instantiations, no `#[cfg]`
  or type-dependent branch in its logic (code checklist § 1), and no mutable static.

## Acceptance tests
- `cargo test -p engine frame_record_fields` — a recorded frame deserialises with every field present (REQ-TOOL-050).
- `cargo test -p prin report_percentiles` — synthetic frame log: percentiles and the motion-only headline match hand-computed values (REQ-TOOL-051).
- `cargo test -p engine telemetry_bounded` — a simulated long 60 fps session with idle stretches produces a file below a bound and keeps every motion frame (REQ-TOOL-052).
- `cargo xtask bench telemetry-overhead` — release build emits stage timings; measured instrumentation overhead is negligible against 16.7 ms (REQ-TOOL-053).
- `cargo test -p prin profile_deep_zoom_03` — the Profiler section defines `deep_zoom_03`; `prin profile --scenario deep_zoom_03 --frames 600` run twice gives the same frame count and the same scope / event sequence; physics reviewer approved the definition (REQ-TOOL-130).
- `cargo test -p prin batch_parses_interactive` — a prin batch output parses with the interactive parser (REQ-TOOL-054).
- `cargo test -p engine frame_dmin_counters` — a frame whose march packs one NaN and one negative `d_min` records `dmin_nan_unset = 1` and `dmin_negative_floored = 1` in that frame's record; the next frame, with neither, records 0 and 0; the frame loop never waits on the readback (REQ-TOOL-140).
- Review checklist (perf) — file sizes of a long 60 fps session with idle stretches under the chosen scheme and N, with every motion frame kept; the proposed value is marked pending and the human confirms it at the M5 gate, then it is recorded in `decisions.md` (REQ-TOOL-117).

## Notes
- Calibrations (R-71) this task proposes: REQ-TOOL-117.
- Definitions (R-72) this task writes: REQ-TOOL-130.
- R-113 (RQ-93 M0-4): REQ-TOOL-006's deep_zoom_03 scenario and its 600-frame run are split off as REQ-TOOL-130 and closed here; M0 runs a synthetic scenario (TASK-M0-18).
- REQ-TOOL-117 is an R-71 calibration; the human confirms it at the M5 gate.
- R-294 (RQ-174): the GPU binding and readback of R-288's counters are this task's, applied per R-204 — veto?. The
  march that packs `d_min` is TASK-M4-05's, reached through TASK-M5-21. On the CPU, the packer's caller passes in a
  per-frame counters struct (TASK-M0-10); the kernel holds no mutable static.
- Size, applied per R-264: R-294's scope (the counter buffer and its bind group, the per-frame reset, the non-stalling
  readback, the record fields, the kernel binding and `dmin_counters.rs`) takes the estimate from ~400 to ~520 lines.
  The task stays one task: its complexity is one frame record and its readback, and nothing moves elsewhere.
