# TASK-M0-44 — Compute pipelines under an explicit fast-math setting: our own MSL through wgpu's passthrough on Metal, and each stage's mode in the session header

- **Milestone:** M0
- **Closes:** REQ-SYS-074, REQ-TOOL-141
- **Depends on:** TASK-M0-14, TASK-M0-19
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~300 lines

## Goal
Compute shaders, the simulation, compile under an explicit fast-math setting that the project sets and records: off by
default, on only as an opt-in optimisation, never inherited silently from a backend (R-297). wgpu 30 compiles MSL with
the default `MTLCompileOptions`, which has fast-math on, and offers no switch for it (RQ-175's measurement). So on
Metal, off means the project compiles its own MSL with fast-math off and loads it through wgpu's passthrough, for the
compute pipelines only; vertex and fragment pipelines, the display, keep wgpu's own path and may keep fast-math on.
Every compute pipeline is created through one entry point that takes the setting, so the harness's dispatches now and
the kernel's variant table later (TASK-M4-06) go through it. The session header (TASK-M0-19) records each shader
stage's mode, compute, vertex and fragment, as compiled on the running backend.

## References
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/notes/principia_gpu_determinism_note.md` § "The one-line law"
- `decisions.md` § "R-84 — Branch decisions across precisions *(closes RQ-35)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"

## Deliverables
- The compute-pipeline entry point (in `crates/engine`, beside the dispatch it serves; the harness in
  `crates/validation/src/gpu.rs` calls it): it takes the fast-math setting, `Off` by default. On Metal it translates
  the module to MSL, compiles it with fast-math off (in whichever form wgpu's passthrough accepts: compile options, an
  in-source math-mode pragma, or a precompiled library), and loads it through the passthrough; on a backend whose own
  path compiles without fast-math and offers no switch, it uses that path (R-297's Applied note).
- The setting as a typed value, with its default, that later tasks carry on the sim key (TASK-M4-08), in the embedded
  record (TASK-M7-31) and in the GUI (TASK-M8-43).
- A lint in `cargo xtask ci`: no compute pipeline is created outside the entry point, and no vertex or fragment
  pipeline goes through the passthrough.
- The session header's per-stage fast-math modes (compute, vertex, fragment), in the probe TASK-M0-19 builds, and in
  profiler schema v1's header.
- A probe compute shader and the test `compute_fast_math`, run in the `gpu-metal` and `gpu-lavapipe` jobs.
- Negative controls for this task's tests (R-176).

## Acceptance tests
- `cargo test -p validation compute_fast_math` in the `gpu-metal` and `gpu-lavapipe` jobs — the probe computing `(x + 0.5) / 255.0` for every x in 0..256, and a fuzzed set of f32 divisions, matches the CPU's correctly rounded f32 division bit for bit with the setting off, on both backends; on Metal with the setting on, it differs on at least one input (RQ-175's columns), so the switch is shown to act; the setting defaults to off (REQ-SYS-074).
- `cargo xtask lint compute-pipelines` — no compute pipeline is created outside the entry point, and no vertex or fragment pipeline goes through the passthrough; a seeded violation of each fails it (REQ-SYS-074).
- `cargo test -p engine session_header_fast_math` — the header carries the compute, vertex and fragment modes; on Metal with the default setting it reads compute off and vertex and fragment on; with the setting on it reads compute on; on lavapipe compute reads off whatever the setting (recorded as compiled); a header missing a stage fails to parse (REQ-TOOL-141).

## Notes
- The probe is RQ-175's measurement turned into a test: on hosted Metal, `(x + 0.5) / 255.0` matched `(x + 0.5) ×
  f32(1/255)` under wgpu's default compile, one ulp from correctly rounded division on 94 of 256 columns.
- If wgpu 30's passthrough can't load compute MSL with fast-math off on the hosted `macos-15` runner, that is a
  REVIEW_QUEUE entry, not a workaround.
- Fast-math off doesn't remove cross-implementation transcendental latitude; branch inputs stay comparison-only
  (`principia_gpu_determinism_note.md`), whatever the math mode.
