# TASK-M4-30 — The header records "Off" only for drivers measured, otherwise wgpu's path with "driver unverified" (R-694)

- **Milestone:** M4
- **Closes:** REQ-TOOL-221
- **Depends on:** TASK-M4-03
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
`crates/engine/src/compute.rs`:58 has `Api::Vulkan => Some(all(StageMode::Off))`, a table entry: nothing observes the driver. R-694 keeps wgpu's path as what is recorded and withholds the claim about the driver unless it has been measured.

## References
- `decisions.md` § "R-694 — With the compute setting off, R-297's bit-identity means the branch decisions of REQ-INT-057 and the Tier B fields only, continuous values free across drivers; `E_rel` and `|Δn̂|` take written forms with no driver division or `√` in the decision; the header records "Off" only for drivers measured, otherwise wgpu's path with "driver unverified" *(closes RQ-685; refines R-297)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"

## Deliverables
- The measured list beside the header code, with its definition: which runs qualify, what keys an entry (adapter name, driver version, or both), each entry naming its run; Metal and lavapipe are the first candidates, on CI's runs (REQ-TOOL-221, R-72).
- `crates/engine/src/compute.rs` and `crates/engine/src/contract/fast_math.rs`: the header reads the adapter's identity, and records "Off", or wgpu's path with "driver unverified"; `fast_math.rs`:24's doc says what it records.
- `crates/validation/src/gpu.rs`:978-979: "on every backend" narrowed to the two named jobs (R-694). `crates/validation/tests/qa_TASK-M0-44.rs`:3-4 says the same: qa's to change, the ruling naming it (R-290).

## Acceptance tests
- `cargo test -p engine session_header_driver` — REQ-TOOL-221's verify, with its control.
- `cargo test -p engine session_header_fast_math` — REQ-TOOL-141's cases still pass on Metal and lavapipe (REQ-TOOL-221).
- Review checklist (physics reviewer) — the list's definition; no entry without its run (REQ-TOOL-221).

## Notes
- Added by R-694 (4 Nov 2026, RQ-685, the honesty part of (c)).
- One vocabulary: a driver recorded "driver unverified" is unverified, not supported. This is no `StageMode::Unknown`, and the Run window's control stays enabled; R-303 is the browser's alone.
- A driver measured to fail is not listed.
