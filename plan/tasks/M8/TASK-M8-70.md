# TASK-M8-70 — The instrument stays honest in the GUI: the "Print: …" badge in the header strip, Styled / Plain export defaulting to Plain, and every readout blind to the print (R-489, R-523)

- **Milestone:** M8
- **Closes:** REQ-GUI-363, REQ-GUI-364, REQ-GUI-365
- **Depends on:** TASK-M8-68, TASK-M8-32, TASK-M8-22, TASK-M8-34, TASK-M8-14, TASK-M8-05
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
While a Print graph other than Plain is active the figure's header strip names it ("Print: Linocut"), never over the figure (R-523); the export dialog offers Styled and Plain and defaults to Plain for figure export, the graph travelling in the record either way; and the Inspector, Measure, the legend, the colour readout under the cursor and area statistics read the pre-print colours (the GUI's half of §11 test 2).

## References
- `docs/design/principia_print_graph.md` § "3. What never changes because of it"
- `docs/design/principia_print_graph.md` § "11. Tests (each with an arm that can fail)"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `docs/gui/principia_render_gui_spec.md` § "Export & share"
- `decisions.md` § "R-489 — The instrument stays honest: measurement never reads the print, goldens and scientific checks run under Plain, a styled figure is badged, and figure export defaults to Plain"
- `decisions.md` § "R-491 — `RenderState.print` is recorded by value everywhere `RenderState` goes; a graph of built-in nodes is round-trip tier 2, custom WGSL tier 3"
- `decisions.md` § "R-523 — Badges go in the chrome or the status line, never over the figure: the decoder badge sits in the figure's header strip, above the plot *(closes RQ-387; amends R-479)*"

## Deliverables
- The badge in the header strip; the Styled / Plain choice in the Export & share window; the readouts' sources asserted pre-print.
- Screenshot case `print_badge`; tests `print_badge`, `print_export_choice`, `print_readouts_blind`.

## Acceptance tests
- `cargo xtask screenshot print_badge` and `cargo test -p gui print_badge` — REQ-GUI-363's verify (under Linocut the header strip reads "Print: Linocut" and the badge's rect is disjoint from the plot's; under Plain no badge; control: a badge drawn in the canvas corner fails the disjointness check).
- `cargo test -p gui print_export_choice` — REQ-GUI-364's verify (with Linocut active the dialog opens on Plain and the exported pixels equal the Plain render; Styled exports the printed pixels; both records carry the graph; control: a dialog that defaults to Styled fails).
- `cargo test -p gui print_readouts_blind` — REQ-GUI-365's verify (with a heavy preset active every listed readout reads bitwise what it reads under Plain; mutation arm: a colour readout wired after the print stage fails).

## Notes
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
