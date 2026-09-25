# TASK-M3-21 — Payload purity and the parity-tier tags

- **Milestone:** M3
- **Closes:** REQ-SCHED-002, REQ-VAL-026
- **Depends on:** TASK-M3-19
- **Needs (earlier milestones):** REQ-PAY-005
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1, PIT-1.7
- **Size:** ~300 lines

## Goal
A quad's SimState is shown to be a pure function of (IC, sim key, playhead t) on the CPU: identical by continuous march, off-loop catch-up, evict + re-boot, and under different visit orders and frame budgets. Every drill-down test carries its parity tier, with the integrator suite tagged per parity §8.

## References
- `docs/contracts/principia_scheduler_contract.md` § "Part 1 — The firewall: the scheduler is arbitrary about *what* it looks at, never about *what* it sees"
- `docs/contracts/principia_scheduler_contract.md` § "Part 7 — The frame loop (lockstep presentation)"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/design/principia_dd_integrator.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_dd_integrator.md` § "5. Unit tests"
- `docs/contracts/principia_integrator_contract.md` § "What this preserves, and what it costs"
- `docs/contracts/principia_caching_contract.md` § "Part 3 — Cross-chart sharing: permitted, deferred"
- `docs/design/principia_dd_simstate_payload.md` § "0. The two buffers (split by access pattern)"
- `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_systems_architecture.md` § "8. What drills down from here"
- `docs/contracts/principia_parity_contract.md` § "8. Test index (drill-down suite → tier)"

## Deliverables
- `crates/engine/tests/purity.rs` — the four-route property test over `computeQuad` with resumable marching.
- A test attribute/registry `#[parity_tier(L|B|N|S)]` and a lint in `xtask` that fails on an untagged integrator test or a tag that disagrees with parity §8's index.

## Acceptance tests
- `cargo test -p engine payload_purity` — property test: for random ICs the state at a fixed t via (a) continuous march, (b) off-loop catch-up after reveal, (c) evict + re-boot, (d) different visit orders / frame budgets is bit-identical on the same backend (REQ-SCHED-002).
- `cargo xtask lint parity-tiers` — each test in the suite is tagged L/B/N/S matching parity §8 (L: substep integer, detector labels and priority; N: order scaling, one-step and drift; S: Burrau smoke and divergence structure) (REQ-VAL-026).

## Notes
- None.
