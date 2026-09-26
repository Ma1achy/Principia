# TASK-M0-01 — The cargo workspace, the crates and CI

- **Milestone:** M0
- **Closes:** REQ-SYS-004
- **Depends on:** TASK-M0-00
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The cargo workspace exists under `crates/` with the plan's crates — `kernel`, `ledger`, `engine`, `render`, `gui`, `validation`, `prin` — and `xtask`, each a compiling stub with no behaviour. There is no `contract` crate: the typed surfaces live in `crates/engine/src/contract/` (R-146, R-172). A GitHub Actions workflow runs the build, `cargo test --workspace` and `cargo xtask ci` on every push; `cargo xtask ci` is the single per-push entry point into which every later per-commit runner registers (plan-check, controls, gate, golden, codegen); bench runs nightly and screenshots on GUI PRs, in their own workflows (R-177). The crate graph is itself checked: `cargo xtask deps` reads `cargo metadata` and fails when a workspace dependency edge is outside the allowed-edge table of systems_architecture §7.1, the crate map (R-170) — the layout table (the ledger) is a root with no workspace dependency, the payload precedes its consumers, and nothing depends on the gui crate (canonical_spec §1 item 5; gui_state_contract §1).

## References
- `docs/design/principia_systems_architecture.md` § "7. Hierarchy and dependency (the build DAG, abstract)"
- `docs/design/principia_systems_architecture.md` § "7.1 Crate map"
- `decisions.md` § "R-170 — The crate map *(closes G2)*"
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/design/principia_dd_generation_root.md` § "1. What it is"
- `decisions.md` § "R-146 — The crate layout is confirmed *(closes RQ-76)*"
- `decisions.md` § "R-172 — There is no contract crate *(closes C1, C4)*"
- `decisions.md` § "R-177 — Cadence *(closes G4, C6)*"
- `decisions.md` § "R-185 — The crate map is confirmed; kernel → ledger is a build-dependency only *(closes TASK-M0-00)*"
- `decisions.md` § "R-187 — kernel and ledger may take validation as a dev-dependency; validation never depends on prin *(closes RQ-129)*"

## Deliverables
- `Cargo.toml` (workspace), `.cargo/config.toml` (the `xtask` alias), `.gitignore` additions for `target/`.
- `crates/{kernel,ledger,engine,render,gui,validation}/` — `Cargo.toml` + `src/lib.rs` stubs; `crates/prin/` — a binary stub (`prin --help`).
- `xtask/` — `cargo xtask ci` (runs the registered runners in order; empty list at this task) and `cargo xtask deps`.
- `xtask/src/deps.rs` — the allowed-edge table, transcribed from systems_architecture §7.1, each edge commented with the §7 arrow it realises. Invariants asserted: `ledger` has no workspace dependency; `kernel` depends on no workspace crate but `ledger`, and on `ledger` only as a build-dependency (`kind: "build"` in `cargo metadata`; a normal dependency on that edge fails, R-185); no crate depends on `gui`; every workspace edge is in the table. Reads `cargo metadata --format-version 1` (or a fixture JSON for tests).
- `.github/workflows/ci.yml` — on push and pull_request: toolchain setup, cache, `cargo build --workspace`, `cargo test --workspace`, `cargo xtask deps`, `cargo xtask ci`.
- `xtask/tests/fixtures/metadata_*.json` — the workspace graph plus one forbidden edge each (`kernel → engine`, `engine → gui`, `ledger → engine`, and `kernel → ledger` as a normal dependency rather than a build-dependency).

## Acceptance tests
- `cargo build --workspace` and `cargo test --workspace` — green in CI on the PR head; the CI log shows `cargo xtask ci` running on push.
- `cargo xtask deps` — passes on the workspace (REQ-SYS-004: the crate graph has no edge outside systems_architecture §7.1's allowed edges).
- `cargo test -p xtask deps` — each forbidden-edge fixture (`kernel → engine`, `engine → gui`, `ledger → engine`, normal `kernel → ledger`) fails, naming the edge and, for the last, its dependency kind (REQ-SYS-004; the check can fire).

## Notes
- Depends on TASK-M0-00: the crate map must be confirmed first (R-170).
- The crate layout is confirmed by R-146; R-172 removes the `contract` crate an earlier draft of this task added.
- The rust-gpu toolchain pin lands with TASK-M0-14, not here.
- Module-level DAG edges inside one crate (decoder before kernel, canonicalise before integrator) are not visible to a crate-graph check; they stay a code-review item (REQ-SYS-004 is checked by crate graph plus review).
