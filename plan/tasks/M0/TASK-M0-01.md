# TASK-M0-01 — The cargo workspace, the crates and CI

- **Milestone:** M0
- **Closes:** REQ-SYS-004
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The cargo workspace exists under `crates/` with the plan's crates — `kernel`, `ledger`, `contract`, `engine`, `render`, `gui`, `validation`, `prin` — and `xtask`, each a compiling stub with no behaviour. A GitHub Actions workflow runs the build, `cargo test --workspace` and `cargo xtask ci` on every push; `cargo xtask ci` is the single entry point into which every later runner registers (plan-check, controls, gate, golden, codegen, bench). The crate graph is itself checked: `cargo xtask deps` reads `cargo metadata` and fails when a workspace dependency edge runs against the build DAG of systems_architecture §7 — the layout table (the ledger) is a root with no workspace dependency, the payload precedes its consumers, and nothing depends on the gui crate (canonical_spec §1 item 5; gui_state_contract §1).

## References
- `docs/design/principia_systems_architecture.md` § "7. Hierarchy and dependency (the build DAG, abstract)"
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/design/principia_dd_generation_root.md` § "1. What it is"

## Deliverables
- `Cargo.toml` (workspace), `.cargo/config.toml` (the `xtask` alias), `.gitignore` additions for `target/`.
- `crates/{kernel,ledger,contract,engine,render,gui,validation}/` — `Cargo.toml` + `src/lib.rs` stubs; `crates/prin/` — a binary stub (`prin --help`).
- `xtask/` — `cargo xtask ci` (runs the registered runners in order; empty list at this task) and `cargo xtask deps`.
- `xtask/src/deps.rs` — the allowed-edge table, each edge commented with the §7 arrow it realises. Invariants asserted: `ledger` has no workspace dependency; `kernel` depends on no workspace crate but `ledger`; no crate depends on `gui`; every workspace edge is in the table. Reads `cargo metadata --format-version 1` (or a fixture JSON for tests).
- `.github/workflows/ci.yml` — on push and pull_request: toolchain setup, cache, `cargo build --workspace`, `cargo test --workspace`, `cargo xtask deps`, `cargo xtask ci`.
- `xtask/tests/fixtures/metadata_*.json` — the workspace graph plus one forbidden edge each (`kernel → engine`, `engine → gui`, `ledger → contract`).

## Acceptance tests
- `cargo build --workspace` and `cargo test --workspace` — green in CI on the PR head; the CI log shows `cargo xtask ci` running on push.
- `cargo xtask deps` — passes on the workspace (REQ-SYS-004: the crate graph has no edge against the systems_architecture §7 DAG).
- `cargo test -p xtask deps` — each forbidden-edge fixture (`kernel → engine`, `engine → gui`, `ledger → contract`) fails, naming the edge (REQ-SYS-004; the check can fire).

## Notes
- First task of the build: no dependencies.
- The crate layout is the plan's proposal, pending the reviewer (MILESTONES "Assumptions": crate boundaries other than the engine crate and the gui crate are the build's to set). If the reviewer changes it, this task's stubs and the deps table change with it.
- The rust-gpu toolchain pin lands with TASK-M0-14, not here.
- Module-level DAG edges inside one crate (decoder before kernel, canonicalise before integrator) are not visible to a crate-graph check; they stay a code-review item (REQ-SYS-004 is checked by crate graph plus review).
