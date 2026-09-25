# TASK-M7-31 — Image embedding: what travels — the contract-named payload and the build-hash refusal

- **Milestone:** M7
- **Closes:** REQ-TOOL-065, REQ-TOOL-066, REQ-TOOL-112, REQ-TOOL-067
- **Depends on:** TASK-M7-14, TASK-M7-17, TASK-M7-27
- **Needs (earlier milestones):** REQ-SYS-011, REQ-GUI-007, REQ-RENDER-043
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The embedded payload carries the build hash, the slice (chart id + params, z₀ over the 8-D latent, q₁ and q₂ with zoom as their common scale, slice values, lock flag, z_locked, δ), the sim settings (T_horizon, dt_macro, N_max, r_sub, gamma_sub, r_coll, r_close, eps_E, eps_L, tau, the escape window, and the integrator occupant as stepper × regularisation), the ensemble (E, N, no jitter field), the full colour spec including brightness field, polarity and ramp window, and the tier settings used — every field named as the contracts name it (R-81). Recreating across a decoder change is refused, the detecting build-hash component defined in dd_image_embedding §6 (R-72). Colour modes travel as enum IDs plus parameters, WGSL source only for genuinely ejected nodes, and any recreate fallback to a different colouring is visible.

## References
- `docs/design/principia_dd_image_embedding.md` § "6. What travels"
- `decisions.md` § "R-81 — The embedded record uses the contract names *(closes RQ-32)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-83 — The slice scale lives in q *(closes RQ-34)*"
- `decisions.md` § "R-80 — Samples per footprint *(closes RQ-31)*"

## Deliverables
- `crates/render/src/embed/payload.rs` — the payload from `SimConfig`, the slice and `RenderState`, and recreate-side validation.
- `docs/design/principia_dd_image_embedding.md` §6 — the build-hash component that detects a decoder change, with the "Removed lines" note.
- Tests.

## Acceptance tests
- `cargo test -p render embed_payload_fields` — embed → extract round-trips every listed field; field names match gui_state_contract §2 and integrator_contract Part 3; the record has no jitter_frac and no 10-component z0 (REQ-TOOL-065).
- `cargo test -p render embed_build_hash_refusal` — a payload with a mismatched build hash yields a refusal (REQ-TOOL-066).
- Review (physics): dd_image_embedding §6 names the component and what changes it; a decode or chart-version change alters it and an unrelated build change does not (REQ-TOOL-112).
- `cargo test -p render embed_colour_spec` — the default preset payload contains no source; an ejected node's payload contains its source; an unknown enum id triggers a visible fallback notice (REQ-TOOL-067).

## Notes
- Definition (R-72) REQ-TOOL-112.
- Accepted interpretation (revised checkpoint A): `rEsc`, `eta` and `nSync` are dropped from the embedded record.
- The colour "enum ID" is the registry / preset id of each node (TASK-M7-13, -17).
