# TASK-M0-20 — The screenshot runner

- **Milestone:** M0
- **Closes:** REQ-TOOL-134
- **Depends on:** TASK-M0-01, TASK-M0-04
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
`cargo xtask screenshot <suite>` is the runner the `GUI screenshot` verify method needs (RQ-93, R-113). It captures a GUI surface headless (native wgpu offscreen) for the GUI reviewer's layout comparison against its artboard (R-68), or runs a presence-only check of named controls where no artboard exists (R-129). Split out of TASK-M0-06 by R-183.

## References
- `decisions.md` § "R-68 — Artboard values are illustrative; corpus values win *(closes RQ-24)*"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*"
- `decisions.md` § "R-183 — TASK-M0-06 is split *(closes S1; reverses R-156's size exemption)*"
- `decisions.md` § "R-177 — Cadence *(closes G4, C6)*"

## Deliverables
- `xtask/src/screenshot.rs` — `cargo xtask screenshot <suite>`: renders a GUI surface headless (native wgpu offscreen) and writes the capture beside the artboard it names (`docs/gui/design/NN_*.png`) under `target/screenshot/`, for layout comparison only (R-68). A case may instead be presence-only (R-129): it lists the controls or items it must contain, and the runner asserts them. Not in the per-commit `cargo xtask ci`: `.github/workflows/screenshot.yml` runs `cargo xtask screenshot --all` on pull requests touching `crates/gui/**` or `docs/gui/**` (a GUI PR, R-177), and `gate.yml` runs it at the gates (R-110).
- `fixtures/screenshot/selftest/` — a minimal egui surface with two named controls: one layout case against a checked-in reference image, one presence-only case.
- Negative controls for this task's tests (R-176).

## Acceptance tests
- `cargo xtask screenshot selftest` — the layout case writes the capture beside its reference; the presence-only case passes, and fails naming the control when one control is removed. The runner can fire, and it exists before the first `GUI screenshot` requirement, REQ-GUI-014 in M6 (REQ-TOOL-134).

## Notes
- Split from TASK-M0-06 (R-183). Its id is the next free one, because ids are never renumbered.
- The first `GUI screenshot` requirements are M6's (REQ-GUI-014, REQ-TOOL-058). REQ-TOOL-010 was once named here as M1's, but R-153 moved it to golden images.
