# TASK-M7-64 — Print graph port, Slots: the six slot nodes and the slot chain template; Palette on the finished colour; screen media and Viewing; the sheet, never the marks, on a pixel that is not a system (R-524, R-525, R-533)

- **Milestone:** M7
- **Closes:** REQ-COL-151, REQ-COL-152, REQ-RENDER-115
- **Depends on:** TASK-M7-56, TASK-M7-58, TASK-M7-60
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~1000 lines, the tests included

## Goal
The Slots category (palslot, ground, onground, unfinished, age, view), each against its lab reference, and the slot chain as a graph: render → Palette → Medium → On the ground ◄ Ground → Unfinished → Age → Viewing → output, each slot a `preset:` sub-graph, an empty slot the identity, the template the Print panel edits. Palette acts on the finished colour (R-524). The `screen_medium` flag is set on the display-modelling nodes from the catalogue and listed; choosing a screen medium empties Viewing in the same edit (R-525 V1, V2). Under a non-Plain graph a not-a-system pixel shows the bare sheet (Ground, Age, Viewing), never a mark, with TASK-M7-26's sheet-node definition (R-533, §11 test 11).

## References
- `docs/design/principia_print_graph.md` § "3. What never changes because of it"
- `docs/design/principia_print_graph.md` § "5. The slot chain: palette and medium compose"
- `docs/design/principia_print_graph.md` § "10. Porting from the lab"
- `docs/design/principia_print_graph.md` § "11. Tests (each with an arm that can fail)"
- `docs/design/principia_print_graph.md` § "4.3 Node types are a registry"
- `docs/design/principia_print_graph.md` § "6.2 Units: a medium must look the same at every export size"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `decisions.md` § "R-486 — The Print graph's node set is the lab's, all 78 node types and all 103 presets, ported from GLSL to WGSL; its data format is the Python prototype's, as the design doc §4 merges them *(supersedes R-130, with R-485)*"
- `decisions.md` § "R-492 — Fidelity to the lab is measured per node and per preset against the bundled reference renders; a deviation the design requires (units, seeding, real fields for the lab's stand-in) is listed in the node's registry entry, never tuned to match; the design doc's §11 tests, each with an arm that can fail"
- `decisions.md` § "R-524 — The Print graph's Palette slot acts on the finished colour, never before the stain's lightness channel is composed *(closes RQ-388)*"
- `decisions.md` § "R-525 — The Viewing slot is off by default while the Medium is a screen medium, and the panel shows the reason *(closes RQ-389)*"
- `decisions.md` § "R-533 — Under a Print graph the sheet prints the whole frame and the marks only where the figure is a system *(closes RQ-398; amends R-470 and R-489)*"

## Deliverables
- `crates/render/src/print/nodes/` — the six slot nodes; `crates/render/src/print/slots.rs` — the template, slot sub-graphs and the screen-medium rule.
- `crates/render/src/print/exec.rs` — the sheet / marks split at not-a-system pixels, read from the classification the stain reads.
- Golden suite `print-slots`; tests `print_screen_medium`, `print_sheet_not_a_system`; the PR lists the nodes flagged `screen_medium`.

## Acceptance tests
- `cargo xtask golden print-slots` — REQ-COL-151's verify (against the baselines in `fixtures/golden/print/` (R-146, R-110; the M8 browser suite checks the same baselines) (the lab's references, pinned there per colour_composition §7 "The Print graph's goldens"), every node of the category, and every preset listed, renders within REQ-COL-142's median and p99 of its reference or is listed in its registry entry with the deviation's reason (§11 test 9); the category's `ref_px` and `fraction` nodes pass §11 test 4 and its `temporally_stable` nodes test 5; controls: one node rendered with a default parameter doubled exceeds the tolerance and fails, a node compared to another node's reference fails, and a registry entry whose listed deviation is removed while the port still deviates fails; the slot chain with Palette "limited", Medium Watercolour pencil, Ground cold-press and Age 40 years equals the same four sub-graphs wired as nodes, bitwise, and with every slot empty equals Plain; control: a Palette slot spliced before the stain's lightness composition changes the stain's lightness channel and fails).
- `cargo test -p render print_screen_medium` — REQ-COL-152's verify (the flagged list holds `crt`, `lcd` and `ledmatrix` (R-525 V1's CRT scanlines, LCD subpixels and LED pitch; REQ-RENDER-110's three nodes) and not `linocut`, and equals the list the PR gives; choosing "CRT television" with Viewing set empties Viewing in one RenderField::Print edit; setting Viewing again keeps it; choosing Linocut after it leaves Viewing as it is; controls: a registry with `ledmatrix` unflagged fails, a registry with `linocut` flagged fails, and a medium whose sub-graph holds `crt` only inside a `preset:` node is still a screen medium, a check that looks only at top-level nodes failing it).
- `cargo test -p render print_sheet_not_a_system` — REQ-RENDER-115's verify (under a slot-chain graph every not-a-system pixel equals the graph's bare sheet there (Ground, then Age and Viewing), no palette colour appears on one (R-477 C9), with Ground empty it equals the background through Age and Viewing, and under Plain it is R-470's blank; with Overlays › Chart › forbidden region on, every not-a-system pixel equals the bare sheet with the forbidden-region hatch composited over it, and with it off the bare sheet alone; controls: an executor that runs the marks on every pixel fails, and one that draws the sheet over the hatch, hiding it, fails the hatch arm).

## Notes
- The Viewing reason the panel shows (R-525 V3) is TASK-M8-68's.
- The example "limited palette + watercolour pencil + cold-press + 40 years" (§5) is the slot chain's acceptance graph, so the task waits on the Paint port; the screen-medium arm on the Screen port.
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
- **Fix round 8 of PR #281 (qa review 5608745757, findings 1, 4, 7, applied per R-369):** the acceptance lines follow REQ-COL-151's, REQ-COL-152's and REQ-RENDER-115's new verifies; a golden-image verify runs under `cargo xtask golden <suite>` with its baselines in `fixtures/golden/print/` (R-146, R-110). *Was:* "Tests `print_port_slots`, `print_screen_medium`, `print_sheet_not_a_system`; the PR lists the nodes flagged `screen_medium`."; "`cargo test -p render print_port_slots` — REQ-COL-151's verify (every node of the category, and every preset listed, renders within REQ-COL-142's median and p99 of its reference or is listed in its registry entry with the deviation's reason (§11 test 9); the category's `ref_px` and `fraction` nodes pass §11 test 4 and its `temporally_stable` nodes test 5; controls: one node rendered with a default parameter doubled exceeds the tolerance and fails, a node compared to another node's reference fails, and a registry entry whose listed deviation is removed while the port still deviates fails; the slot chain with Palette "limited", Medium Watercolour pencil, Ground cold-press and Age 40 years equals the same four sub-graphs wired as nodes, bitwise, and with every slot empty equals Plain; control: a Palette slot spliced before the stain's lightness composition changes the stain's lightness channel and fails)."; "`cargo test -p render print_screen_medium` — REQ-COL-152's verify (the flagged list equals the PR's list; choosing "CRT television" with Viewing set empties Viewing in one RenderField::Print edit; setting Viewing again keeps it; choosing Linocut after it leaves Viewing as it is; control: a medium whose sub-graph holds `crt` only inside a `preset:` node is still a screen medium, and a check that looks only at top-level nodes fails it)."; "`cargo test -p render print_sheet_not_a_system` — REQ-RENDER-115's verify (under a slot-chain graph every not-a-system pixel equals the graph's bare sheet there (Ground, then Age and Viewing), no palette colour appears on one (R-477 C9), with Ground empty it equals the background through Age and Viewing, and under Plain it is R-470's blank; control: an executor that runs the marks on every pixel fails).".
