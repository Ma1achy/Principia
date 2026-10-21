# TASK-M8-53 — The round-trip rule: every export the identity on `SimConfig` and `RenderState`, no empty struct, every destination carrying the links, and the tiers reported in the GUI (R-483, R-484)

- **Milestone:** M8
- **Closes:** REQ-TOOL-176, REQ-TOOL-177, REQ-TOOL-178, REQ-GUI-263
- **Depends on:** TASK-M8-01, TASK-M8-28, TASK-M8-31, TASK-M8-32, TASK-M8-33, TASK-M7-44, TASK-M8-48, TASK-M8-50, TASK-M8-46, TASK-M8-52, TASK-M2-21, TASK-M2-36, TASK-M2-37, TASK-M2-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~600 lines, the tests included

## Goal
Importing any export is the identity on `SimConfig` and `RenderState` (T1, over a state with every leaf off its launch value, with the default state as control); no struct of either serialises to `{}` (T2); each of the six destinations carries a non-default link and parameter (C9); the Export & share window reports the tiers that fit before the share, and Import picture reports the tiers recovered and badges a fallback (RT3).

## References
- `decisions.md` § "R-483 — Export, sharing and the embedded record carry the decoder: `SimConfig`'s `Links` holds each block's link id and parameter values, the one place they live, written in full even at the defaults; the embedded record's list gains the links, the tilt fields, the energy-axis mappings and a saved chart by value; a record's decoder is the build's registry plus its links, an unknown or changed link refused and never substituted; an old record reads under its own build's defaults; capacity is re-measured and a record never truncated; a link change between keyframes is a cut; the sim key and the compatibility signature gain the parameter values; two questions queued (RQ-384, RQ-385)"
- `decisions.md` § "R-484 — The round-trip rule: importing any export is the identity on `SimConfig` and `RenderState`, bitwise in the canonical serialisation, on every path but the image's low-bit embedding, which drops whole tiers from the bottom (custom source, then colour) and never tier 1, physics, writing nothing rather than a partial record; the degrading is reported by the writer, the record and the reader, and badged; every field is assigned a tier in one table, exhaustive at compile time; four tests keep it from drifting; splitting tier 2 is queued (RQ-386)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/contracts/principia_export_animation_contract.md` § "Part 5 — Determinism & reproducibility"
- `docs/contracts/principia_export_animation_contract.md` § "Part 6 — Sharing: the spec is the object, the video is its shadow"
- `docs/gui/principia_render_gui_spec.md` § "Export & share"
- `docs/gui/principia_render_gui_spec.md` § "G9. Import picture · saved views · record a sweep (`09_importrecord.png`)"

## Deliverables
- T1's state generator and the round trip over every path.
- T2's struct-walking serializer.
- C9 over the six destinations.
- The tier report in Export & share and Import picture, and the fallback badge, in the figure's header strip (R-523).
- Tests `round_trip_identity`, `no_empty_structs`, `every_destination_links`, `export_tier_report`.

## Acceptance tests
- `cargo test -p engine every_destination_links` — REQ-TOOL-176's verify (C9).
- `cargo test -p engine round_trip_identity` — REQ-TOOL-177's verify (T1).
- `cargo test -p engine no_empty_structs` — REQ-TOOL-178's verify (T2).
- `cargo test -p gui export_tier_report` — REQ-GUI-263's verify.

## Notes
- T2 fires on today's `Links {}`, `Chart {}`, `Slice {}`, `Integrator {}`, `Collision {}`, `Quality {}` and `Palette {}` (R-484 J9); this task depends on the tasks that fill them, so main never goes red. The PR records that T2 fired on all seven before they were filled, as the human asked.
- Physics reviews that tier 1 round-trips everything that decides integration; this task does not merge on the code reviewer's word alone.
- `ViewUI` is not exported; `f3_fill` rides beside the two structs outside the identity (R-484 J3).
