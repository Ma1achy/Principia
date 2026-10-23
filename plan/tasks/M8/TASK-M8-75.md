# TASK-M8-75 — Result, the GUI: the Result window, the result view and Compare (R-572)

- **Milestone:** M8
- **Closes:** REQ-GUI-366, REQ-GUI-367, REQ-GUI-368
- **Depends on:** TASK-M8-74, TASK-M8-17, TASK-M8-34
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~500 lines, the tests included

## Goal
File › Make a result…, Measure's kept region and the Inspector open the Result window, prefilled from the view and listing every field that differs from it, with an estimate from a timed 64-pixel pilot and a warning when `T` passes the precision's predictability horizon; Start, progress, Cancel, Resume and Copy command (R-572). File › Open result… opens a result view: badged in the header strip (R-523), drawn from the payload with nothing integrated, stain edits live, sim-key edits disabled with the reason, Explore from here, the scrubber snapping to the stored times. Compare flips against the live view, labelled, and Make the f32 twin gives the per-pixel difference map.

## References
- `docs/design/principia_results.md` § "3.1 Launch: the Result window"
- `docs/design/principia_results.md` § "3.4 Reopening: the result view"
- `docs/design/principia_results.md` § "3.5 Compare"
- `docs/gui/principia_render_gui_spec.md` § "Result… — the full-precision batch run (R-572)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-572 — Result: a view's region run at full precision, f32 or f64 only, on a uniform grid, as a batch job in the app or by `prin batch`, written to a result folder that reopens as a view drawn from what was stored; the user chooses the stored times, and one copy per pixel or all `E+1` copies is an option *(amends REQ-SYS-007 and philosophy §2; unparks philosophy §7.3's reduced and per-copy tiers as the Result's payload)*"
- `decisions.md` § "R-523 — Badges go in the chrome or the status line, never over the figure: the decoder badge sits in the figure's header strip, above the plot *(closes RQ-387; amends R-479)*"

## Deliverables
- `crates/gui/src/windows/result.rs` — the Result window, its three launch points, the pilot and the predictability warning, progress, Cancel, Resume and Copy command.
- The result view: the open-result request through the contract (the engine's source an opened result, `ViewUI` holding the window), the header-strip badge, the disabled sim-key controls with their reason, Explore from here, the snapping scrubber; Compare and the f32 twin's difference map.

## Acceptance tests
- `cargo test -p gui result_window` — opened from each of the three places it lists only the fields that differ from the view; the pilot's estimate doubles when the pilot's measured time is doubled by a test clock (control: a constant estimate fails); the predictability warning shows at T = 30 at f32 and not at T = 20 (control: a warning keyed on T = 50 fails); Copy command's text names the job file the app runs (REQ-GUI-366).
- `cargo test -p gui result_view` — a stain edit in a result view runs 0 integrations and is undoable; a horizon edit is refused with the reason; Explore from here gives a live view whose SimConfig equals the result's bitwise; the scrubber set to t = 22 snaps to the nearest stored time; a live view has no badge (control: a badge drawn on every view fails, and a result view that re-integrates on a stain edit fails) (REQ-GUI-367).
- `cargo test -p gui result_compare` — the flip carries its label; the f32 twin is the same job.json but for its precision, and its difference map against a payload that differs in exactly one pixel counts 1 (control: a map against itself counts 0, and a map comparing only the nominal copy's outcome misses a field difference and fails) (REQ-GUI-368).

## Notes
- R-572 (21 Oct 2026). The result view's source is engine state set by an open-result request (applied per R-369; the engine never reads `ViewUI`).
