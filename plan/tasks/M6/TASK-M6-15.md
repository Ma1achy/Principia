# TASK-M6-15 — QualitySettings on three axes, the tiers keyed off eps, and the preset selector

- **Milestone:** M6
- **Closes:** REQ-PERF-084, REQ-PERF-040, REQ-PERF-044, REQ-PERF-067, REQ-PERF-037, REQ-PERF-035, REQ-PERF-045, REQ-PERF-042, REQ-PERF-043, REQ-GUI-015
- **Depends on:** TASK-M5-11, TASK-M5-12
- **Needs (earlier milestones):** REQ-PERF-014, REQ-PERF-016, REQ-PERF-026, REQ-PERF-027, REQ-PERF-028, REQ-PERF-029, REQ-PERF-033, REQ-GEN-017
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~420 lines

## Goal
One `QualitySettings` struct carries the knobs and the three quality axes — `eps` (convergence target), frame budget, and the mandatory hard cap — defined in the quality/device note by this task (R-72, R-40). `eps` is the user-facing knob the refinement policy reads and each tier row sets; `N` and `MAX_REL_DEPTH` are tier-gated with a Custom override, as are ensemble copies and Benettin shadows; `E` is free-valued on internal rungs and snapped only on named tiers. The preset selector (Auto / named tier / Custom) fills the one struct; tiers ship with defaults keyed on crude specs and a written rationale; memory totals come from the generated payload width.

## References
- `docs/design/principia_quality_device_note.md` § "The reframe: quality is a preset selector populating one settings struct"
- `docs/design/principia_memory_tiers.md` § "4.1 The same six tiers on the three axes"
- `decisions.md` § "R-40 — Memory tiers and the quality device key off `eps` *(PL-5)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "3.5 THE TIER IS THREE COUPLED AXES, NOT ONE — v0.5"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "The three axes"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "v0.5 means the NUMBERS are guesses; the SHAPE is not"
- `docs/design/principia_memory_tiers.md` § "Principia — quality tiers & memory model (auto-mode reference)"
- `decisions.md` § "R-4 — "spec-keyed defaults" means the markdown's tier tables *(closes RQ-5)*"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `docs/read_first/principia_INDEX.md` § "Known open items"
- `decisions.md` § "R-73 — Apply the whole ruling-follow-up checklist now *(closes RQ-56)*"
- `docs/design/principia_dd_refinement_policy.md` § "1. The split rule — no exponent in the split test"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `docs/design/principia_memory_tiers.md` § "4. The six quality tiers"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `decisions.md` § "R-89 — Depth and E are not on the sim key *(closes RQ-40)*"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "7. The calibration campaign — collect everything, decide nothing yet"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "8. What this is not"
- `docs/contracts/principia_gui_state_contract.md` § "6. Quality settings — preset selector over one struct (see `principia_quality_device_note.md`)"
- `docs/contracts/principia_gui_state_contract.md` § "7. What a replacement GUI must honour (the teardown contract)"

## Deliverables
- Doc change: `docs/design/principia_quality_device_note.md` § "The reframe: quality is a preset selector populating one settings struct" — the `eps`, frame-budget and hard-cap fields with types and owners, and the granularity at which `eps` varies (REQ-PERF-084).
- `crates/engine/src/contract/quality.rs`: `QualitySettings`, `QualityPreset { Auto, Named(Tier), Custom }`, the tier table (memory_tiers §4 and §4.1 rows, labelled placeholders) with a rationale comment per default.
- `crates/engine/src/quality/tiers.rs`: the default-tier selector over crude specs only; memory totals from the ledger's generated payload width; the hard-cap stop.

## Acceptance tests
- Review checklist (physics) — the struct lists the three fields with types and owners, and the doc says at which granularity eps varies; the doc change is merged with the physics reviewer's approval (REQ-PERF-084).
- `cargo test -p engine three_axes_cap_stop` — QualitySettings has the three fields; a pathological slice stops at the cap instead of OOM (REQ-PERF-040).
- `cargo test -p engine quality_axes_fields` — QualitySettings has eps, frame_budget and hard_cap fields; a pathological slice with an artificially low cap stops at the cap instead of allocating further (REQ-PERF-044).
- `cargo test -p engine quality_keys_off_eps` — QualitySettings has the three axes; controller reports which binds; memory totals are computed from the generated payload width (REQ-PERF-067).
- Review checklist (perf) — QualitySettings carries eps; the refinement policy reads eps from it; each tier row sets eps (REQ-PERF-037).
- `cargo test -p engine tier_rows_populate` — each named tier populates N, MAX_REL_DEPTH, E and FTLE from its table row; Custom can override each (REQ-PERF-035).
- `cargo test -p engine e_free_valued` — the ladder admits E = 2, 4, 5, 6; named tiers use 0/0/1/3/7/15 (REQ-PERF-045).
- `cargo test -p engine default_tier_crude_specs` — the default-tier selector uses only the listed specs (REQ-PERF-042).
- Review checklist (perf) — the tier config documents the rationale per default (REQ-PERF-043).
- `cargo test -p engine preset_selector_one_struct` — all three modes write the same struct type; re-selecting Auto runs a fresh probe (REQ-GUI-015).

## Notes
- Definition written: REQ-PERF-084. "Re-selecting Auto runs a fresh probe" (REQ-GUI-015) is asserted against a mocked probe; the probe itself is TASK-M6-16.
