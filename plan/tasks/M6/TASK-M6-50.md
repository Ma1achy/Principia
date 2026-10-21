# TASK-M6-50 — The GUI track (R-390): the Decoder window on the mock, its rows, staged Apply, live previews through the engine's link evaluator, the Chart section's Links row, and the decoder badge (R-478 to R-482)

- **Milestone:** M6
- **Closes:** REQ-GUI-251, REQ-GUI-252, REQ-GUI-254, REQ-GUI-255, REQ-GUI-256, REQ-GUI-257, REQ-GUI-258, REQ-GUI-259, REQ-GUI-260, REQ-GUI-262
- **Depends on:** TASK-M2-41, TASK-M2-21, TASK-M6-45, TASK-M6-42, TASK-M6-49
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~900 lines, the tests included

## Goal
On the mock engine: the Decoder window (Windows › Decoder…, a window under R-461's rules) with one row per block, a compatible-only dropdown, the parameters with their ranges, a live preview and a reset; its staged edits applied as one `SetField(SimField::Links)` and one undo entry; the Chart section's Links row, amber off the default; the optional link pin of a preset or saved chart, applied on load with a notice in one undo entry (R-516); the decoder badge in the figure's header strip (R-523); and `engine::contract::links`, the pure catalogue and f64 evaluator the previews call, so a preview is the kernel's own function.

## References
- `decisions.md` § "R-478 — Link functions belong to the decoder, which every chart passes through, not to a chart: they are chosen and tuned in a new Decoder window (Windows › Decoder…); the Manifold view's Chart section shows them in one compact row, tinted amber when any block is off its default; the Chart builder shows them read-only and owns the axis warps; a chart preset carries no links; the instruction's §6 is queued (RQ-380 to RQ-383)"
- `decisions.md` § "R-479 — The Decoder window: one row per block (Masses, Configuration α, Configuration β, Momenta), each with a dropdown of only the compatible registry entries, the link's parameters with their ranges, a preview and a reset; a footer with "Reset all to defaults", "Changing a link re-integrates every pixel" and Apply / Revert, the edits staged and applied as one undo entry; Home and the section resets touch no link; a non-default decoder named in the canvas badge (RQ-387) and in every sidecar and export"
- `decisions.md` § "R-480 — Each link has a preview, about 160 × 110 px in the line system's styles and live while a parameter is dragged: a one-dimensional block's curve with its bounds, the visible range, the centre, its resolution and the default's ghost; the masses' simplex triangle with the image of an even grid of z lines, the view's footprint, the centre and the equal-mass point; the axis warps the same widget; every preview evaluating the generated link functions on the CPU at f64, never a reimplementation"
- `decisions.md` § "R-482 — The tests of R-478 to R-481, each with an arm that can fail: the dropdown compatible-only, the defaults bitwise today's, a parameter change moving the result, the range's ends admissible, the log-det at a non-default value, provenance reproducing bitwise, the preview the kernel's own function, and Home leaving the links alone"
- `decisions.md` § "R-516 — A chart preset or a saved chart may optionally pin the links it was designed under; loading one whose pin differs from the active links applies the pin with the chart, one command and one undo entry, with a notice naming the blocks changed; an unpinned preset leaves the links alone *(closes RQ-380; amends R-478)*"
- `decisions.md` § "R-523 — Badges go in the chrome or the status line, never over the figure: the decoder badge sits in the figure's header strip, above the plot *(closes RQ-387; amends R-479)*"
- `docs/gui/principia_render_gui_spec.md` § "G7a. Decoder (no artboard; R-478 to R-482)"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `engine::contract::links`: the catalogue (entries per codomain, display names, tunable parameters with defaults and ranges) and the f64 evaluator.
- `SimField::Links` and its log line; `ViewUI`'s `Decoder` window.
- The Decoder window, its rows, footer and previews; the Chart section's Links row; the decoder badge in the figure's header strip.
- The link pin: "Pin the active links" on a preset or saved chart in the picker, its pin icon, and the load that applies it with its console notice (R-516).
- Tests `mock_links_not_chart`, `mock_links_row`, `mock_preset_pin`, `mock_decoder_rows`, `mock_decoder_apply`, `mock_home_leaves_links`, `mock_decoder_badge`, `mock_preview_1d`, `mock_preview_simplex`, `mock_preview_is_kernel`; a capture case for the window in `xtask` (R-390).

## Acceptance tests
- `cargo test -p gui mock_links_not_chart` — REQ-GUI-251's verify.
- `cargo test -p gui mock_links_row` — REQ-GUI-252's verify.
- `cargo test -p gui mock_preset_pin` — REQ-GUI-254's verify (R-516; *was:* `mock_preset_no_links`).
- `cargo test -p gui mock_decoder_rows` — REQ-GUI-255's verify (C1's GUI arm).
- `cargo test -p gui mock_decoder_apply` — REQ-GUI-256's verify.
- `cargo test -p gui mock_home_leaves_links` — REQ-GUI-257's verify (C8).
- `cargo test -p gui mock_decoder_badge` — REQ-GUI-258's verify (R-523).
- `cargo test -p gui mock_preview_1d` — REQ-GUI-259's verify.
- `cargo test -p gui mock_preview_simplex` — REQ-GUI-260's verify.
- `cargo test -p gui mock_preview_is_kernel` — REQ-GUI-262's verify (C7).

## Notes
- **R-523 (20 Oct 2026) rules RQ-387:** badges go in the chrome or the status line, never over the figure; the decoder badge sits in the figure's header strip, above the plot and outside it (applied per R-369: the header strip over the top bar). *Was:* "The badge's place waits on RQ-387; meanwhile D4 as written, the top-left corner (R-479 E7). A ruling for another place is a follow-up change to REQ-GUI-258; nothing waits."
- **R-516 (20 Oct 2026) rules RQ-380:** a preset or saved chart may pin links, applied on load with a notice, one undo entry; this task builds it on the mock, so it depends on TASK-M6-49 (saved charts; applied per R-369). The pin is the GUI's own settings, never `SimConfig`'s.
- A new capture case changes `xtask/tests/qa_TASK-M6-24.rs`'s `check_01_main` only if the qa reviewer adds it there, in this task's qa commit (R-290); the PR lists it.
- The previews do not wait on the line engine (TASK-M7-42): they read the same style presets (R-480 F1).
