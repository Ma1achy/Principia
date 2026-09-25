# TASK-M5-02 — Quality tiers and the one settings struct

- **Milestone:** M5
- **Closes:** REQ-PERF-014, REQ-PERF-026, REQ-PERF-027, REQ-PERF-028, REQ-PERF-032, REQ-PERF-033, REQ-RENDER-052
- **Depends on:** TASK-M4-19
- **Needs (earlier milestones):** REQ-SYS-001, REQ-SYS-005, REQ-SYS-029, REQ-INT-065, REQ-RENDER-014
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~320 lines

## Goal
The six quality tiers exist as named presets populating one `QualitySettings` struct: render_scale, E, FTLE,
word, N and depth per memory_tiers §4, eps / frame budget / hard cap per §4.1 as labelled placeholders (eps tightening
monotonically), FTLE from Medium up with a Custom override, E free-valued 0–15, render_scale restricted to the four tier
values with a Custom slider over 0.25–2.0, the interactive frame budget 16 ms and export ignoring it. Later M5 tasks read
their knobs from this struct.

## References
- `docs/contracts/principia_canonical_spec.md` § "5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*"
- `docs/design/principia_memory_tiers.md` § "4. The six quality tiers"
- `docs/design/principia_memory_tiers.md` § "4.1 The same six tiers on the three axes"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "v0.5 means the NUMBERS are guesses; the SHAPE is not"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "Then invert"
- `docs/read_first/principia_INDEX.md` § "Known open items"
- `open-questions.md` § "Open questions"
- `docs/design/principia_quality_device_note.md` § "Open sub-questions (settle at implementation)"
- `docs/notes/principia_sampling_msaa_note.md` § "Uniform samples: every sample is a full, normal SimState"
- `docs/notes/principia_sampling_msaa_note.md` § "Ensemble copies ARE the SSAA samples"
- `docs/notes/principia_sampling_msaa_note.md` § "Amendments this makes"
- `docs/design/principia_memory_tiers.md` § "2. Two resolutions — display and internal-render (sample density is not a separate knob)"
- `docs/design/principia_memory_tiers.md` § "What `render_scale` gets you (and its character)"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `decisions.md` § "R-4 — "spec-keyed defaults" means the markdown's tier tables *(closes RQ-5)*"

## Deliverables
- `crates/engine/src/contract/quality.rs`: `QualityTier` (six variants plus Custom), `QualitySettings`, the preset table
  (placeholder rows marked as such, each value traced to its memory_tiers row).
- `crates/engine/src/quality.rs`: render-target size from display size × render_scale; frame-budget accessor that the
  export path does not consult.
- Unit tests `crates/engine/src/contract/tests/quality.rs`; `xtask bench tier-table` reports the placeholder rows and the eps order.

## Acceptance tests
- `cargo test -p engine quality_tier_enum` — the tier enum has six variants; has_ftle is false for Potato/Low and true from Medium (REQ-PERF-014).
- `cargo test -p engine quality_preset_rows` — each named preset's QualitySettings matches its row (REQ-PERF-026).
- `cargo test -p engine quality_ftle_flag` — tier presets' ftle flag; Custom with render_scale 0.25 accepts ftle = on (REQ-PERF-027).
- `cargo xtask bench tier-table` — config marks the six rows as placeholders; eps strictly decreases Potato → Extreme; the calibration campaign's recorded per-device results replace them (REQ-PERF-028).
- `cargo test -p engine frame_budget_default` — default frame_budget = 16 ms; export path does not consult it (REQ-PERF-032).
- `cargo test -p engine quality_e_values` — tier table E values; E = 0 renders with no copies (REQ-PERF-033).
- `cargo test -p engine render_scale_target` — tier presets only produce the four values; Custom accepts 0.25–2.0; render target size equals display × render_scale (REQ-RENDER-052).

## Notes
- §4.1's eps / frame budget / hard cap numbers are placeholders by R-4 and the telemetry doc ("the NUMBERS are
  guesses"); the calibration campaign replaces them. The bench asserts the placeholder marking and the eps order, never the
  values.
- N and depth are "indicative" in §4; this task records them as given, no new values.
