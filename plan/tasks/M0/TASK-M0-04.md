# TASK-M0-04 — The test harness: native wgpu self-test dispatch, proptest and negative controls

- **Milestone:** M0
- **Closes:** REQ-VAL-007
- **Depends on:** TASK-M0-01
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~400 lines

## Goal
`crates/validation` carries the shared test harness. A native in-process `wgpu` harness opens a headless device (requesting no optional features), compiles a WGSL compute module, dispatches it over storage buffers and reads the result back in the same process (parity_contract §6). A shared proptest configuration records its seed on failure. And a negative-control registry: every test registers the discriminating control that must make it fail — a mutation, a contaminated input, a sign-flipped variant or a comparison that must differ — and `cargo xtask controls` runs every control and fails if any control passes or any test has none, so no test in the suite is one that cannot fail (philosophy §4.4; pitfalls §9's general form).

## References
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `decisions.md` § "R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*"

## Deliverables
- `crates/validation/src/gpu.rs` — `GpuHarness::new()` (headless, no surface, no optional features), `run_wgsl(module, entry, inputs) -> Vec<u32>`, and the adapter info (name, backend, driver) exposed for TASK-M0-19's session header.
- `crates/validation/src/control.rs` — the negative-control registry (`negative_control!(test, description, control)`) and a `controls` feature under which each control runs its test against the control input.
- `crates/validation/src/prop.rs` — the shared proptest config (case count, seed printed on failure).
- `xtask/src/controls.rs` — `cargo xtask controls`: lists the workspace's tests, runs `cargo test --workspace --features controls`, and fails when a control passes or a test has no control; registered in `cargo xtask ci`.
- Harness self-tests: a WGSL identity kernel; a WGSL kernel reading a top-bit-set word with the i32 and the u32 `extractBits` overloads.

## Acceptance tests
- `cargo test -p validation gpu_harness` — a WGSL identity dispatch round-trips 2¹⁶ u32 words bit-exact on native in-process wgpu.
- `cargo test -p validation gpu_harness_can_fire` — on words with bit 31 set, the i32 `extractBits` dispatch differs from the u32 one: the harness's reachable output includes the sign-extension failure.
- `cargo xtask controls` — every test in the workspace has a registered negative control and every control makes its test fail; a fixture test with no control, and one whose control passes, each fail the command (REQ-VAL-007).
- Review checklist (qa §3): no test is arithmetically impossible or true by construction (the n_hot < N² quantile case, a distinct-value count bounded below the claimed effect) (REQ-VAL-007).

## Notes
- Every later task's tests register their controls here; the qa reviewer checks each control is discriminating (qa §3).
- See Gaps: the GPU CI runner.
- The harness lives in `crates/validation` ("the validation harness" in the plan layout) so that the codegen self-test and, from M4, the native parity suite share one device path.
