# TASK-M8-41 — Release gates: the non-Metal parity run, the seam test index, the atlas horizon label, GitHub Pages

- **Milestone:** M8
- **Closes:** REQ-VAL-099, REQ-VAL-105, REQ-SYS-061, REQ-SYS-046
- **Depends on:** TASK-M8-40, TASK-M8-31, TASK-M8-32, TASK-M4-13
- **Needs (earlier milestones):** REQ-VAL-057, REQ-TOOL-043, REQ-SYS-026, REQ-VAL-064
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~300 lines

## Goal
The release gates exist: the parity suite passes on a non-Metal backend (Vulkan or D3D12), recorded before any Paper-2 numbers (it gates Paper 2, not the build, R-58); every row of the seam catalogue maps to at least one integration test; the atlas states its predictability horizon and every export or statistic built from atlas data carries it and is labelled exploratory; and the release build loads from GitHub Pages in a WebGPU browser and renders an IC-manifold slice coloured by outcome.

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
## Deliverables
- `cargo xtask gate parity-non-metal` — runs the M4 parity suite on a real (hardware) Vulkan / D3D12 GPU — lavapipe, the per-commit second backend, does not satisfy it (R-110) — and records the result.
- `xtask/src/seam_index.rs` — `cargo xtask seam-index`: every seam 1–14 of systems_architecture §5 is named by at least one integration test's tag.
- Horizon field threaded into exports and statistics built from atlas data (`crates/engine/src/export/provenance.rs`).
- `.github/workflows/pages.yml` — the GitHub Pages deploy of `web/`.

## Acceptance tests
- `cargo xtask gate parity-non-metal` — the parity suite passes on a real (hardware) Vulkan or D3D12 GPU, not lavapipe (R-110); recorded before Paper-2 survey results (REQ-VAL-099).
- `cargo xtask seam-index` and Review checklist (qa reviewer) — a test index maps each of seams 1–14 to at least one integration test (REQ-VAL-105).
- Review checklist (physics reviewer) — Atlas outputs carry the horizon (field) with them; any export or statistic built from atlas data is labelled as exploratory (REQ-SYS-061).
- Review checklist (qa reviewer) — the release build loads from the GitHub Pages site in a WebGPU browser and renders an IC-manifold slice whose samples are ICs coloured by outcome (REQ-SYS-046).

## Notes
- REQ-VAL-099 gates Paper 2, not the build (R-58, R-110): it's in the M8 gate as a recorded run.
