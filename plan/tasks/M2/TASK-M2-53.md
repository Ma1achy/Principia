# TASK-M2-53 — The f32 exponential sinc that never reads the backend's `sin` near 0 (R-696)

- **Milestone:** M2
- **Closes:** REQ-DEC-053, REQ-DEC-054
- **Depends on:** TASK-M2-28
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
R-696 takes RQ-687's option (b). At f32 the sinc at `crates/kernel/src/chart/shape_sphere.rs`:162 stops reading the backend's `sin` near 0, where the RTX 2080 SUPER's value took R-437's `p = q = 0` select and moved `rho_angle` by π/2, and f64's bits do not move.

## References
- `decisions.md` § "R-696 — RQ-687's option (b) now: a sinc form at `shape_sphere.rs`:162 that never reads the backend's `sin` near 0, gated to f32 so f64 bits do not move; option (a), the kernel's own `sin` and `cos`, is deferred to M4, decided on the cross-backend check, and RQ-629 with it"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi"

## Deliverables
- `crates/kernel/src/chart/shape_sphere.rs`: the sinc gated to f32 as `crates/kernel/src/arctan.rs` gates its form, in a form that reads no backend `sin` near 0; the note at :52-55 corrected, since RQ-687 found its "moves the output by rounding only" false for `rho_angle`; `crates/render/wgsl/decode.wgsl` regenerated from the source.
- The cut, if the form needs one, in the constants register, provisional until M2 (REQ-DEC-054).

## Acceptance tests
- `cargo test -p kernel exponential_sinc` — REQ-DEC-053's verify, with its controls: the backend's `sin` emulated as 0 near 0 gives `k` = 0 under the old form and not under the new; an ungated form moves an f64 bit.
- `qa_m225_kernel_rho_angle_at_the_lambda_pole`, in the GPU jobs that run it — passes as before on lavapipe and Metal (REQ-DEC-053).
- Proposal: REQ-DEC-054's cut and bound, or a form with no cut, with its evidence; reviewer-checked, confirmed by the human at the M2 gate and recorded in decisions.md.

## Notes
- Added by R-696 (4 Nov 2026, RQ-687 (b)).
- A run on the RTX is a measurement (R-649), reported on the issue, and gates nothing.
- `cos(r)` at :161 and the other sites RQ-687 lists stay the backend's. If the form moves a fixture's or golden's bits at f32, name them: re-baselining is a gate decision (R-110).
