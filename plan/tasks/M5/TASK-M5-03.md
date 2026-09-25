# TASK-M5-03 — Quad addresses in the slice plane's frame (the per-slice quadtree)

- **Milestone:** M5
- **Closes:** REQ-SCHED-040, REQ-SCHED-086, REQ-SCHED-055, REQ-DEC-032
- **Depends on:** TASK-M4-08
- **Needs (earlier milestones):** REQ-SCHED-001, REQ-SCHED-007, REQ-SYS-009, REQ-GUI-001, REQ-TOOL-019, REQ-TOOL-027, REQ-CHART-003
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~420 lines

## Goal
The engine has a per-slice quadtree whose addresses `QuadID (level, i, j)` are taken in the slice plane's own
frame, relative to the plane anchor (z₀ at the last re-integrating event, R-97), against the post-flip Y-up frame. The
CPU computes each quad's f64 centre and half-width from the address; the level-0 cell size of the slice plane's frame is
defined in deep_zoom §1 (REQ-SCHED-086). An in-plane pan or zoom changes which addresses are requested and leaves every
resident address unchanged. No manifold-level hierarchy exists.

## References
- `docs/design/principia_coordinate_conventions_note.md` § "Code paths that must honour the single convention (each is a separate path)"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"
- `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)"
- `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision"
- `docs/read_first/principia_00_philosophy.md` § "8.1 An 8D BVH over the manifold, instead of a per-slice quadtree"
- `docs/read_first/principia_00_philosophy.md` § "8. Considered and rejected — with the numbers, so they are not reinvented"
- `decisions.md` § "R-97 — Quad addresses live in the slice plane *(closes RQ-57)*"
- `decisions.md` § "R-100 — No per-cell Halton rotation *(closes RQ-60)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `docs/design/principia_deep_zoom.md` §1: the level-0 cell size in the slice plane's frame and how centre and
  half-width follow from it (REQ-SCHED-086), with the "Removed lines" note.
- `crates/engine/src/quadtree/{address.rs, tree.rs}`: `QuadID`, plane anchor, address → (c, h) in f64, children, parent,
  viewport → requested address set.
- `fixtures/quad/asymmetric_ic.ron`: an asymmetric IC fixture (bodies placed so a mirror is visible).
- Tests `crates/engine/tests/quad_address.rs`: CPU/GPU address agreement (the GPU side through the kernel's quad-local
  coordinate), pan/zoom invariance of resident addresses, orientation.

## Acceptance tests
- `cargo test -p engine quad_address` — CPU- and GPU-computed addresses agree for every quad; an in-plane pan or zoom leaves every resident quad's address unchanged; linearised decode within a quad is not mirrored relative to the full decode (REQ-SCHED-040).
- Review checklist (physics) — deep_zoom §1 states the level-0 cell size in the slice plane's frame and how the per-quad centre and half-width follow from it; the address unit test uses it; physics reviewer approved; the doc change is in this PR and the physics reviewer approves it before merge (REQ-SCHED-086).
- `cargo test -p engine quad_address_level0_scale` — the test that follows from the written definition: deep_zoom §1 states the level-0 cell size in the slice plane's frame and how the per-quad centre and half-width follow from it; the address unit test uses it; physics reviewer approved (REQ-SCHED-086).
- Review checklist (code) — Acceleration structure is the per-slice quadtree; no manifold-level hierarchy exists (REQ-SCHED-055).
- `cargo test -p engine quad_local_orientation` — an asymmetric IC fixture decodes to the same IC at a given UV on CPU and GPU; exported image orientation matches the reference (REQ-DEC-032).

## Notes
- Definitions (R-72) this task writes: REQ-SCHED-086.
- REQ-SCHED-040's third assertion (linearised decode within a quad is not mirrored relative to the full decode)
  needs `x₀ + J_D·δ`; the linearised decoder's x₀/J_D computation lands in TASK-M5-04 (REQ-DEC-036) and the switchover in M6 (REQ-DEC-033/037). This task asserts it with a fixture
  `x₀`/`J_D` computed on the CPU by finite differences of the full decoder (see Gaps in the milestone report).
- RQ-99 ruled: R-113, option (a) — REQ-DEC-036 moves to M5, closed by TASK-M5-04 (which depends on this task); this task's third assertion keeps its fixture `x₀`/`J_D`.
