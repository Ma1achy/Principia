# TASK-M0-00 — The crate map: every §7 node assigned to a crate, confirmed by the human

- **Milestone:** M0
- **Closes:** REQ-SYS-064
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~80 lines (docs)

## Goal
systems_architecture §7.1 assigns each node of the §7 build DAG to one crate of the R-146 layout (`kernel`, `ledger`, `engine`, `render`, `gui`, `validation`, `prin`, `xtask`; no contract crate, R-172) and lists the allowed workspace edges, each with the §7 arrow it realises. The draft was written with R-170; this task takes it through review, and the human confirms it. TASK-M0-01's `cargo xtask deps` table is transcribed from the confirmed map.

## References
- `docs/design/principia_systems_architecture.md` § "7. Hierarchy and dependency (the build DAG, abstract)"
- `docs/design/principia_systems_architecture.md` § "7.1 Crate map"
- `decisions.md` § "R-146 — The crate layout is confirmed *(closes RQ-76)*"
- `decisions.md` § "R-170 — The crate map *(closes G2)*"
- `decisions.md` § "R-172 — There is no contract crate *(closes C1, C4)*"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `decisions.md` § "R-185 — The crate map is confirmed; kernel → ledger is a build-dependency only *(closes TASK-M0-00)*"

## Deliverables
- `docs/design/principia_systems_architecture.md` §7.1: any change review asks for, and its status line changed to "confirmed (R-n)" once the human rules.
- The human's confirmation, recorded in `decisions.md` as a ruling.

## Acceptance tests
- Review checklist (code §5): every node of the §7 diagram appears in §7.1's table with exactly one crate; every allowed edge names the §7 arrow it realises; `gui` has no dependents and `ledger` no dependencies (REQ-SYS-064).
- The human's confirmation is in `decisions.md`, and §7.1's status line cites it (REQ-SYS-064).

## Notes
- First task of the build: no dependencies. TASK-M0-01 depends on it (R-170).
- A definition requirement (R-72): the deliverable is the doc change, and the human confirms it.
- **Done:** R-185 (26 Sep 2026) records the human's confirmation, and §7.1's status line cites it. The docs PR that recorded R-185 closes this task. TASK-M0-01 is unblocked.
