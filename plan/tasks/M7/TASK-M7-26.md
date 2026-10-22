# TASK-M7-26 — The print stage: the Print graph's executor in style's place, Plain the bitwise identity, measurement blind to it, debug views bypassing it, goldens under Plain (R-485, R-487, R-489; was the style stage)

- **Milestone:** M7
- **Closes:** REQ-RENDER-105, REQ-RENDER-106, REQ-RENDER-107, REQ-RENDER-108, REQ-COL-027
- **Depends on:** TASK-M7-25, TASK-M7-21, TASK-M7-54
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~700 lines, the tests included

## Goal
The graph executor, in the display chain where style was: SimState → stain → [the line sets drawn through] → Print graph → display scale → gamut clamp → [the line sets drawn over] → colour-vision simulation → screen. Plain, the default, is the bitwise identity (§11 test 1); a Print graph edit is render-only, touching no sim key, quad cache or quad request. Measurement never reads the print (the engine's half of §11 test 2); debug views bypass it (R-487 K1); every stain golden and scientific check sets Plain explicitly. The task writes the definition of how a graph edited past the slot template marks its sheet nodes (R-533 G2), which TASK-M7-64's not-a-system rule uses.

## References
- `docs/design/principia_print_graph.md` § "2. Where it sits"
- `docs/design/principia_print_graph.md` § "3. What never changes because of it"
- `docs/design/principia_print_graph.md` § "5. The slot chain: palette and medium compose"
- `docs/design/principia_print_graph.md` § "11. Tests (each with an arm that can fail)"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"
- `docs/design/principia_colour_composition.md` § "7. Preset table & golden-image obligation"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `decisions.md` § "R-67 — The display chain *(closes RQ-23)*"
- `decisions.md` § "R-485 — Post-processing is the Print graph, a node graph after the stain and separate from it; it replaces the Display window's "Style", and R-130's v1 styles become presets of it, still available *(supersedes R-130)*"
- `decisions.md` § "R-487 — Lines, traces, axes and grids are drawn over the Print graph by default, and a line set can opt in to being rendered through it (`through_medium`), as the design doc §2 draws R-67's chain *(amends R-67)*"
- `decisions.md` § "R-489 — The instrument stays honest: measurement never reads the print, goldens and scientific checks run under Plain, a styled figure is badged, and figure export defaults to Plain"
- `decisions.md` § "R-531 — The lines drawn over the Print graph keep R-476 M2's place, after the gamut clamp and before the colour-vision simulation; the lines drawn through sit before the Print graph; the design doc §2's diagram is corrected to match *(closes RQ-395; amends R-487)*"
- `decisions.md` § "R-533 — Under a Print graph the sheet prints the whole frame and the marks only where the figure is a system *(closes RQ-398; amends R-470 and R-489)*"

## Deliverables
- `crates/render/src/print/exec.rs` — runs a graph's generated passes on the stain's output at render resolution, scheduling sub-graphs and typed intermediates; Plain short-circuits to the identity.
- `crates/render/src/display/chain.rs` — the print stage in style's place, and the debug-view bypass.
- `xtask golden` and the validation harness setting Plain explicitly.
- `docs/design/principia_print_graph.md` §3 — the sheet-node definition (REQ-RENDER-108), with the gui reviewer's approval.
- Tests `print_stage`, `print_measurement_blind`, `print_debug_bypass`, `goldens_force_plain`.

## Acceptance tests
- `cargo test -p render print_stage` — REQ-RENDER-105's verify (under Plain the stage's output bytes equal its input bytes on every frame of the stain golden suite (§11 test 1); the pass order is asserted stain → print → display scale → gamut clamp → CVD; changing the graph leaves the sim key, the quad cache and the quad requests unchanged; controls: any non-Plain preset fixture differs from its input, and a Plain that resamples its input fails the identity).
- `cargo test -p render print_measurement_blind` — REQ-RENDER-106's verify (with a heavy preset fixture active, the colour readout's buffer, the area statistics' input and a stain golden read bitwise what they read under Plain; mutation arm: a readout wired after the print stage fails).
- `cargo test -p render print_debug_bypass` — REQ-RENDER-107's verify (with a non-Plain graph set, View › Debug view and a stain whose colour occupant is `debug/` render bitwise as under Plain; control: a presentation stain under the same graph differs from Plain).
- Definition: §3 says, for a graph built in the editor, how a node is known to be sheet or mark (for example a registry flag or the slot the node descends from), and what a node fed by both is; the gui reviewer checks that the slot template's own split (Ground, Age, Viewing the sheet) falls out of it unchanged (REQ-RENDER-108).
- `cargo test -p render goldens_force_plain` — REQ-COL-027's verify (the golden-image and validation harnesses set Plain explicitly; control: a harness run with a user RenderState holding a non-Plain graph still renders Plain, and one that inherits it fails a stain golden).

## Notes
- **Was the style stage (applied per R-369):** R-485 replaces style with the Print graph and supersedes R-130, so this task, which held the style stage, now holds the print stage; R-130's eight styles are Print graph presets in TASK-M7-65 (R-532), which closes REQ-COL-044. The task's earlier text is kept below under "Was".
- The print stage's frame-record keys (`stage_ms.post`, `post_level`) are TASK-M8-72's.
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.

## Was (before R-485)

The task's text before R-485 replaced style with the Print graph, kept as written:

> *TASK-M7-26 — The style stage: figure-only styles, plain for scientific checks*
>
> - **Milestone:** M7
> - **Closes:** REQ-COL-044, REQ-COL-027
> - **Depends on:** TASK-M7-25
> - **Needs (earlier milestones):** none
> - **Reviewers:** code, qa, gui
> - **Pitfalls:** none
> - **Size:** ~350 lines
>
> *Goal*
> The optional style stage at the head of the display chain, applied to the figure only (chrome and legend unstyled), offering in v1 plain, watercolour & pencil and the seven print presets, as the poster's implementation (`workbench/principia_poster_both_sides.html` and its press module) defines them (R-130): watercolour & pencil is the poster's painted treatment; print is the paper.design halftone presets with the poster's patches (per-plate slip, cell ceiling); the golden-image and validation harnesses force style = plain.
>
> *References*
> - `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
> - `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
> - `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"
> - `decisions.md` § "R-67 — The display chain *(closes RQ-23)*"
> - `decisions.md` § "R-130 ✱ — The styles are the poster's *(closes RQ-107)*"
>
> *Deliverables*
> - `crates/render/shaders/wgsl/compositor/style_*.wgsl` and `crates/render/src/display/style.rs` — plain, the poster's painted treatment (watercolour & pencil), and the seven print presets ported from the poster's press module with its per-plate slip and cell ceiling (R-130).
> - The style selector in the Display window.
> - `xtask golden` and the validation harness forcing `style = plain`.
>
> *Acceptance tests*
> - `cargo xtask screenshot 04_windows` (one scenario per v1 preset: plain, watercolour & pencil, the seven print presets) — each preset against 04_windows.png's Display window; chrome and legend are unstyled (REQ-COL-044).
> - Review (code, qa): the golden-image and validation harnesses force style = plain (REQ-COL-027).
>
> *Notes*
> - RQ-107 ruled: R-130 — the styles are defined by the poster's implementation; render_gui_spec's "print · Poster78, more" is read as the seven print presets. This settles the blocking gap. The human may veto (design ruling).
