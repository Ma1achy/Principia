# TASK-M2-27 — The Inspector's invariance and round-trip gates on the Rust decode and encode

- **Milestone:** M2
- **Closes:** REQ-VAL-021, REQ-VAL-022, REQ-VAL-121
- **Depends on:** TASK-M2-16
- **Needs (earlier milestones):** REQ-VAL-007, REQ-VAL-006
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~300 lines

## Goal
The IC Inspector's verified bounds are re-established on the Rust decode and encode: the four whole-system handles (translate-all, boost-all, rotate-all, scale-all) each leave z fixed to 1.6×10⁻¹⁴ over 2×10⁴ random ICs, and decode → canonicalise → z round-trips to 1.2×10⁻¹³ over 2×10⁴ random z with |z| < 4, with the canonical CoM at the origin to 1.7×10⁻¹⁶, equilateral configurations at a pole and collinear ones on the equator exactly. The Rust Inspector's gates are proposed from the re-measurement.

## References
- `docs/notes/ic_inspector_scratchpad.md` § "Status — as built (supersedes stale details below)"
- `docs/notes/ic_inspector_scratchpad.md` § "Whole-system handles — the invariance audit"
- `docs/notes/ic_inspector_scratchpad.md` § "The quotient set (the thing being audited)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- Gates `cargo xtask gate inspector-handles` and `cargo xtask gate inspector-roundtrip` in `crates/validation/` (fixtures `fixtures/gates/inspector/`, seeded).
- The calibration proposal comparing the Rust measurements with the reference tool's.

## Acceptance tests
- `cargo xtask gate inspector-handles` — 2×10⁴ random ICs, each handle applied: max ‖Δz‖ ≤ 1.6×10⁻¹⁴ (REQ-VAL-021).
- `cargo xtask gate inspector-roundtrip` — 2×10⁴ random z, |z| < 4: max round-trip error ≤ 1.2×10⁻¹³; canonical CoM ≤ 1.7×10⁻¹⁶ from the origin; equilateral → pole and collinear → w = 0 exactly (REQ-VAL-022).
- Calibration: the Rust decode/encode bounds re-measured (reference tool: 1.2×10⁻¹³ round trip, 1.6×10⁻¹⁴ handles) and the gates stated; reviewer-checked, confirmed by the human at the M2 gate, recorded in `decisions.md` (REQ-VAL-121).

## Notes
- These are z-space gates; |z| < 4 keeps s(1−s) ≥ ~0.018, far from the 10⁻⁶ clamps, so inverse_encode Part 4's "never in z near saturation" rule is respected. The handle gate at 1.6×10⁻¹⁴ is the reference tool's number; if the Rust measurement differs, REQ-VAL-121's proposal says so rather than loosening the gate (physics checklist §1).
