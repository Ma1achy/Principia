# TASK-M7-66 — Print graph field inputs: the `field` node, the stain's window, absent fields, and the four "Driven by the data" presets on their real fields (R-526, R-536, R-557) ⚑

- **Milestone:** M7
- **Closes:** REQ-COL-154, REQ-COL-155, REQ-COL-156, REQ-COL-157, REQ-COL-158, REQ-COL-159
- **Depends on:** TASK-M7-56, TASK-M7-57, TASK-M7-58, TASK-M7-05
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf, physics
- **Pitfalls:** none
- **Size:** ~800 lines, the tests and definitions included

## Goal
A payload field as a read-only `scalar` input, the same value the stain reads: its range the stain's window when the stain colours by the same field (`RANGE_AUTO` included), else the field's own `range_norm` default (R-557, RQ-478); Chart position never a field input; an absent field the node's default with "field not available at this tier". The four "Driven by the data" presets record their lab stand-in and read the real field in place of the stand-in node only (R-536): `t_end` for Relief of lifetime, the ensemble spread for Wet where chaotic, and for Unfinished where uncertain and Craquelure the fields this task defines. Three definitions (two windows, the two fields, invalid samples) and one calibration (the threshold windows) are written here under physics review.

## References
- `docs/design/principia_print_graph.md` § "4.4 Payload fields as inputs"
- `docs/design/principia_print_graph.md` § "10. Porting from the lab"
- `docs/design/principia_print_graph.md` § "11. Tests (each with an arm that can fail)"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `decisions.md` § "R-492 — Fidelity to the lab is measured per node and per preset against the bundled reference renders; a deviation the design requires (units, seeding, real fields for the lab's stand-in) is listed in the node's registry entry, never tuned to match; the design doc's §11 tests, each with an arm that can fail"
- `decisions.md` § "R-526 — A field input to the Print graph is normalised as the stain normalises that field, not over a fixed window recorded with the graph; which range that is, is RQ-478 *(closes RQ-390)*"
- `decisions.md` § "R-536 — Each "Driven by the data" preset's registry entry records the stand-in its lab graph actually uses; the port reads the real field *(closes RQ-401)*"
- `decisions.md` § "R-557 — A Print graph field input takes the stain's window when the stain colours by the same field, else the field's own `range_norm` default; the print stage's time is recorded but never moves the Auto rung, and its post level is the arbiter's motion-regime lever, under Auto only; the human approves PR #268 with its follow-ups *(closes RQ-478, RQ-479 and RQ-495; amends R-488)*"
- `decisions.md` § "R-558 — The state map's twenty entries are ruled: any chart may be a target, its inverse generated beside the view's; a tilted target the branch nearest the slice; never a criterion input; the vMF colouring kept and stated not one-to-one; the nominal copy; an ended run held; no history fields; the prototype's constants the starting values; Chart position a named seam-6 exception, with no fixed t; Replace-L's chroma scaling an option, "keep in gamut", off by default; Shape n̂ stays `vector`; the code's reading of the mirror and the clamp; the file's preset names and the trace's fade; "Edit stain…"; no figure ring under the trace; one colour node; `α_min = 0`; "none" a setting of the node *(closes RQ-480 to RQ-489 and RQ-496 to RQ-505; amends R-542)*"

## Deliverables
- `crates/render/src/print/nodes/field.rs` — the `field` node, its range rule and status.
- `crates/render/print_presets/data/` — the four presets on their fields.
- `docs/design/principia_print_graph.md` §4.4 — the three definitions (REQ-COL-155 to REQ-COL-157), with a "Removed lines" note.
- Tests `print_field_input`, `print_port_data_driven`.

## Acceptance tests
- `cargo test -p render print_field_input` — REQ-COL-154's verify (with the stain colouring `t_end` through a fixed window the input equals the stain's normalised `t_end` bitwise at every pixel, and under `RANGE_AUTO` it follows the stain's window; with the stain not colouring `t_end` it uses lin [0, T] and is unchanged by a zoom; with `ftle` absent at the tier the input takes the default and reports the status; controls: an input normalised over the visible range changes under the zoom and fails, a fallback to brightness fails, and `chart_position` as a field name is refused).
- Definition: §4.4 names the rule (for example the node nearest the colour slot, or the one the legend reads); the physics reviewer checks that it reads a window the stain already has and invents none (REQ-COL-155).
- Definition: §4.4 names each preset's field, a ledger field present at a stated tier, with the reason it carries the preset's meaning (uncertainty; sensitivity); the physics reviewer checks it (REQ-COL-156).
- Definition: §4.4 says what such a sample contributes to a neighbourhood operation and what its own pixel shows; the physics reviewer checks that no NaN reaches a blur and that an invalid sample never reads as a valid extreme (REQ-COL-157).
- Calibration: the PR proposes each window with the field's measured distribution over the reference views and the share of the figure each window marks; the physics reviewer checks it; the human confirms it at the M7 gate (REQ-COL-158).
- `cargo test -p render print_port_data_driven` — REQ-COL-159's verify (each preset's registry entry lists its stand-in as above; the stand-in node, and only it, is replaced by a `field` node; each renders on a fixture payload within REQ-COL-142's tolerance of a reference made by substituting the fixture's field for the stand-in in the lab's graph, or lists the deviation; controls: Wet with its threshold node replaced too fails the graph-shape check, and a preset reading brightness when its field is present fails).

## Notes
- ⚑ physics review required (R-485: the field inputs read the same values, with the same normalisation, as the stain); not merged on the code reviewer's word alone.
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
