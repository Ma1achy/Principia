# TASK-M8-71 — Print graph provenance and round trip: `RenderState.print` by value in every destination, tiers 2 and 3, the custom node dropped and badged, and keyframes (R-484, R-491)

- **Milestone:** M8
- **Closes:** REQ-TOOL-191, REQ-TOOL-192, REQ-TOOL-193, REQ-TOOL-194
- **Depends on:** TASK-M8-53, TASK-M8-31, TASK-M8-32, TASK-M8-69, TASK-M7-66
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~500 lines, the tests included

## Goal
`RenderState.print` recorded by value (a preset as the graph it expanded to, its id a label) in snapshot JSON, share links, pxpack, the embedded record, the `tEXt` chunk, the profiler header's `config` and the animation spec, round-tripping bitwise (§11 test 10, R-484 RT1); every `PrintGraph` field in R-484's tier table, built-ins tier 2, custom WGSL tier 3, a dropped tier recreating as the design doc §9 says, badged; numeric settings interpolating across keyframes and a structure or preset change a cut, with the definition of which settings count as numeric.

## References
- `docs/design/principia_print_graph.md` § "9. Provenance"
- `docs/design/principia_print_graph.md` § "6.1 Determinism"
- `docs/design/principia_print_graph.md` § "11. Tests (each with an arm that can fail)"
- `docs/design/principia_dd_image_embedding.md` § "6. What travels"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/principia_render_gui_spec.md` § "Export & share"
- `decisions.md` § "R-484 — The round-trip rule: importing any export is the identity on `SimConfig` and `RenderState`, bitwise in the canonical serialisation, on every path but the image's low-bit embedding, which drops whole tiers from the bottom (custom source, then colour) and never tier 1, physics, writing nothing rather than a partial record; the degrading is reported by the writer, the record and the reader, and badged; every field is assigned a tier in one table, exhaustive at compile time; four tests keep it from drifting; splitting tier 2 is queued (RQ-386)"
- `decisions.md` § "R-491 — `RenderState.print` is recorded by value everywhere `RenderState` goes; a graph of built-in nodes is round-trip tier 2, custom WGSL tier 3"

## Deliverables
- `crates/engine/src/contract/` — `PrintGraph` in `RenderState`'s serialisation and the tier table.
- `docs/design/principia_print_graph.md` §9 — the numeric-settings definition (REQ-TOOL-194), with a "Removed lines" note.
- Tests `print_round_trip`, `print_tiers`, `print_keyframes`.

## Acceptance tests
- `cargo test -p engine print_round_trip` — REQ-TOOL-191's verify (such a graph survives each of the seven destinations bitwise; control: a record that stores a preset by id alone fails after the built-in preset fixture is changed).
- `cargo test -p engine print_tiers` — REQ-TOOL-192's verify (every `PrintGraph` field is in the tier table; an image too small for tier 3 recreates the graph with the custom node missing and the fallback badge set; one too small for tier 2 recreates Plain, badged; control: a `PrintGraph` field added to a fixture without a tier fails the table's completeness check (RT4)).
- `cargo test -p engine print_keyframes` — REQ-TOOL-193's verify (`depth` 0.2 → 0.8 over ten frames passes through 0.5 at the midpoint; Linocut → Halftone switches at the keyframe with no blended frame; control: an interpolator that cross-fades the two graphs fails).
- Definition: §9 classes every setting type; the gui reviewer checks that no interpolated value ever feeds a seeded stream a non-integer seed (REQ-TOOL-194).

## Notes
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
