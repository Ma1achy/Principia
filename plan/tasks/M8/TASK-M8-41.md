# TASK-M8-41 — Release gates: the non-Metal parity run, the seam test index, the atlas horizon label, v1 web on GitHub Pages

- **Milestone:** M8
- **Closes:** REQ-VAL-099, REQ-VAL-105, REQ-SYS-061, REQ-SYS-046, REQ-SYS-086
- **Depends on:** TASK-M8-40, TASK-M8-31, TASK-M8-32, TASK-M4-13, TASK-M8-42, TASK-M8-76
- **Needs (earlier milestones):** REQ-VAL-057, REQ-TOOL-043, REQ-SYS-026, REQ-VAL-064
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~300 lines

## Goal
The release gates exist: the parity suite passes on a non-Metal backend (Vulkan or D3D12), recorded before any Paper-2 numbers (it gates Paper 2, not the build, R-58); every row of the seam catalogue maps to at least one integration test; the atlas states its predictability horizon and every export or statistic built from atlas data carries it and is labelled exploratory; and v1 web, the release build, loads from GitHub Pages in a WebGPU browser and renders an IC-manifold slice coloured by outcome: the whole native app compiled to run in the browser, with the same interface, features and stains, and where the browser cannot do something the native app does, the app says so in place (R-571).

## References
- `docs/contracts/principia_canonical_spec.md` § "2. The determinism law (the other defining decision)"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"
- `decisions.md` § "R-58 — The non-Metal parity run gates Paper 2, not the build *(TO-2)*"
- `docs/notes/principia_gpu_determinism_note.md` § "The parity consequence (what shared source does and does not buy)"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_systems_architecture.md` § "8. What drills down from here"
- `docs/read_first/principia_00_philosophy.md` § "The interactive atlas — a hypothesis generator"
- `docs/read_first/principia_00_philosophy.md` § "4.1 The instrument must say where it stops knowing"
- `docs/read_first/principia_00_philosophy.md` § "3. The consequence people get wrong"
- `docs/contracts/principia_canonical_spec.md` § "0. What Principia is"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-571 — The browser: the native app first, then v1 web, the whole native app compiled to run in the browser on the GitHub Pages site, then v2 web, a separate product designed later *(replaces the browser item of the gap sweep's `INSTRUCTION_now.md` backlog; queues RQ-550 and RQ-551)*"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `decisions.md` § "R-619 — v1 web has two lists of crossings: the three for engine state, unchanged, and a separate, enumerated platform list, both enforced in CI *(closes RQ-550; amends REQ-SYS-059's "exactly three" for v1 web)*"
- `decisions.md` § "R-621 — The header strip states `T` and the f32 trust time, measured for the current view rather than a constant; the per-point trust stays in the viewer *(closes RQ-556; refines R-574's "nothing changes in the atlas" as to the header)*"

## Deliverables
- `cargo xtask gate parity-non-metal` — runs the M4 parity suite on a real (hardware) Vulkan / D3D12 GPU — lavapipe, the per-commit second backend, does not satisfy it (R-110) — and records the result.
- `xtask/src/seam_index.rs` — `cargo xtask seam-index`: every seam 1–14 of systems_architecture §5 is named by at least one integration test's tag.
- Horizon field threaded into exports and statistics built from atlas data (`crates/engine/src/export/provenance.rs`).
- `.github/workflows/pages.yml` — the GitHub Pages deploy of v1 web (`web/`, with the whole native app compiled to wasm).

## Acceptance tests
- `cargo xtask gate parity-non-metal` — the parity suite passes on a real (hardware) Vulkan or D3D12 GPU, not lavapipe (R-110); recorded before Paper-2 survey results (REQ-VAL-099).
- `cargo xtask seam-index` and Review checklist (qa reviewer) — a test index maps each of seams 1–14 to at least one integration test (REQ-VAL-105).
- Review checklist (physics reviewer) — Atlas outputs carry the horizon (field) with them; any export or statistic built from atlas data is labelled as exploratory (REQ-SYS-061).
- Review checklist (qa reviewer) — the release build loads from the GitHub Pages site in a WebGPU browser and renders an IC-manifold slice whose samples are ICs coloured by outcome; its GUI is the gui crate's, compiled to wasm32 with the engine, with no second GUI codebase under web/; every window the native app opens (render_gui_spec §G5) opens in it, and its stain library lists the native app's stains (REQ-SYS-046).
- Review checklist (gui reviewer) — each native feature with a control that v1 web lacks (at least the compute fast-math control, R-303, and each such feature R-619's platform list leaves out of the browser) is, in the GitHub Pages build, where the native app has it, disabled, with a note that the browser cannot do it; none is missing, moved or redesigned; control: a native control absent from the browser build fails the check (REQ-SYS-086).

## Notes
- REQ-VAL-099 gates Paper 2, not the build (R-58, R-110): it's in the M8 gate as a recorded run.
- R-571 (21 Oct 2026): the release on GitHub Pages is v1 web, the whole native app in the browser; M8 ships the native app and v1 web, and v2 web, a separate browser product, is in no milestone. REQ-SYS-046 is restated, REQ-SYS-086 added ("says so in the app" read as R-303's pattern, applied per R-369), the gui reviewer added, and the task depends on TASK-M8-42, the screen sweep, so the build it checks is the whole app (applied per R-369). R-619 (25 Oct 2026, RQ-550 (a)) sets REQ-SYS-086's list: the three gaps with no native control are each crossed by a row of the platform list (files picked or dropped, row 7, kept in the page's storage, row 11; persistence, row 11; `prin profile query --live`, row 14), so the list does not grow by them (applied per R-369). *Was:* "REQ-SYS-086's list waits on RQ-550." Gui review 5608379011 of PR #282 (applied per R-369): REQ-SYS-086 covers gaps with a native control; the control-less gaps (runtime-authored occupants, persistence across reloads, `prin profile query --live`) are RQ-550's. *Was (acceptance):* "each native feature v1 web lacks (… and each feature RQ-550's ruling leaves out …)". *Was (title):* "GitHub Pages"; *was (goal):* "and the release build loads from GitHub Pages …"; *was (deliverable):* "the GitHub Pages deploy of `web/`".
- R-621 (25 Oct 2026, RQ-556 (b)): the header strip states T and the f32 trust time measured for the current view, built by TASK-M8-76 (REQ-GUI-398), which this task now depends on; REQ-SYS-061's statement is unchanged and closes here after it (applied per R-369). *Was:* "- **Depends on:** TASK-M8-40, TASK-M8-31, TASK-M8-32, TASK-M4-13, TASK-M8-42".
