# TASK-M5-04 — QuadRequest: flags, per-quad uniforms and quad-local sample positions

- **Milestone:** M5
- **Closes:** REQ-DEC-031, REQ-PAY-064, REQ-SCHED-024
- **Depends on:** TASK-M5-03
- **Needs (earlier milestones):** REQ-TOOL-013, REQ-TOOL-015, REQ-SYS-021, REQ-SYS-023, REQ-PERF-012, REQ-INT-065, REQ-GEN-010
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~380 lines

## Goal
The per-quad dispatch request exists: `QuadRequest` carries the quad's centre and half-width (f64 on the CPU,
passed as f32) and the bit-packed `flags` word exactly per scheduler Part 5's table (bits 6–7 reserved, the bring-up
mode a baked variant, never a flag bit). The kernel computes sample positions as `u = centre + half·(2t − 1)`,
`t = (i + 0.5)/N`, never from min/max bounds; `x₀` and `J_D` travel in a separate per-quad uniform buffer bound only for
`DECODE_MODE` quads, and the kernel's only use of them is `x₀ + J_D·δ` in f32.

## References
- `docs/contracts/principia_scheduler_contract.md` § "`QuadRequest.flags`"
- `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision"
- `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)"
- `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)"
- `docs/contracts/principia_render_contract.md` § "Part 8 — Errata against the older design docs"
- `docs/contracts/principia_lowering_contract.md` § "Compute side"
- `decisions.md` § "R-39 — `FULL_RETENTION` keeps bit 4, owned by the measurement path *(PL-4, amended)*"
- `decisions.md` § "R-41 — `DEBUG_MODE` uses the baked variants, not flag bits *(RS-1 (b))*"
- `decisions.md` § "R-75 — The kernel keeps one debug mode *(closes RQ-26)*"

## Deliverables
- `crates/engine/src/contract/quad_request.rs`: `QuadRequest`, `QuadFlags` constants; the WGSL constants generated from
  the same table.
- `crates/kernel/src/quad_local.rs`: sample position from (c, h, i, N); the linear-path uniform binding.
- `crates/engine/src/dispatch/request.rs`: building requests from the quadtree (f64 → f32 at the seam).
- `fixtures/gates/quad-local-uv/`: depth-30 quads; `cargo xtask gate quad-local-uv`.

## Acceptance tests
- `cargo xtask gate quad-local-uv` — at depth 30, the N samples of a quad decode to N distinct f32 positions matching the f64 reference within one f32 ulp of h; min/max bounds are not read by the kernel (REQ-DEC-031).
- `cargo test -p engine quad_request_flags` — CPU and WGSL flag constants match the table; bits 6–7 unused by any code path (REQ-PAY-064).
- `cargo test -p kernel per_quad_uniforms` — per-quad uniforms carry c, h, x₀, J_D; kernel computes only x₀ + J_D·δ (REQ-SCHED-024).

## Notes
- The flag-constant test compares the CPU and WGSL tables bit by bit and must fail on a single swapped bit
  (pitfalls §9: a check whose output set cannot include the failure tells nothing).
- Filling `x₀`/`J_D` from the linearised decoder is M6; here the buffer layout, binding rule and kernel arithmetic are
  built and tested with fixture values.
- Waits on RQ-99 (`REVIEW_QUEUE.md`): M5 requirements that need M6, M7 or M8.
