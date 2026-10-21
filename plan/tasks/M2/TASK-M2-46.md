# TASK-M2-46 — The Inspector's gates per R-551: the pole and equator checks judged, the Randomise arm reported, and the gates' bounds proposed

- **Milestone:** M2
- **Closes:** REQ-VAL-022, REQ-VAL-121, REQ-VAL-193
- **Depends on:** TASK-M2-27, TASK-M2-43
- **Needs (earlier milestones):** REQ-VAL-007, REQ-VAL-006
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~200 lines

## Goal
TASK-M2-27 built the Inspector's gates and merged with REQ-VAL-022 and REQ-VAL-121 held (RQ-355, RQ-475 to RQ-477).
R-551 rules what the held parts mean: the round trip draws in the ball `‖z‖ < 4` at the fixed, recorded seed, its
1.2×10⁻¹³ provisional under REQ-VAL-121; a collinear state's `z_β` sits at its clamp image with its `lookup_clamped`
bit set, bit for bit, and no equilateral state sets it; an equal-mass equilateral state's `|w − 1|` and a collinear
state's `|w|` lie within a calibrated bound; the Inspector's Randomise distribution runs as a second arm of the handles
gate, reported until its bound is calibrated and never used to loosen 1.6×10⁻¹⁴. This task judges the shape checks,
adds the Randomise arm, and proposes the bounds.

## References
- `decisions.md` § "R-551 — The Inspector's gates: a collinear state's `z_β` at its clamp image with its bit set, bit for bit, and `|w − 1|` at the pole and `|w|` at the equator within a calibrated bound; `|z| < 4` is the ball `‖z‖ < 4`; the gated random IC is `g₀·D(z)`, held at 1.6×10⁻¹⁴, and the Inspector's Randomise a second arm reported until REQ-VAL-121 calibrates its bound *(closes RQ-475, RQ-476 and RQ-477)*"
- `docs/notes/ic_inspector_scratchpad.md` § "Status — as built (supersedes stale details below)"
- `docs/notes/ic_inspector_scratchpad.md` § "Whole-system handles — the invariance audit"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- In `cargo xtask gate inspector-roundtrip` (`crates/validation/`): the shape checks over RQ-475's families (random
  gauge elements; equal-mass equilateral triangles, `L⁺` and `L⁻`; collinear states with masses from `z_μ` on
  `[−4, 4)²`) judged: the `lookup_clamped` bit and `z_β`'s clamp image bit for bit, `|w − 1|` and `|w|` against
  REQ-VAL-121's bounds; each with a registered negative control (R-176).
- In `cargo xtask gate inspector-handles`: the Randomise arm beside REQ-VAL-021's, its per-handle maxima reported, its
  verdict separate.
- The calibration proposal for the shape bounds and the Randomise arm's bound, with TASK-M2-27's proposal for the
  round trip and the handles.

## Acceptance tests
- `cargo xtask gate inspector-roundtrip` — the fixed, recorded seed's 2×10⁴ `z` in the ball `‖z‖ < 4`: max
  `‖z − E(D(z))‖ ≤ 1.2×10⁻¹³`, canonical CoM ≤ 1.7×10⁻¹⁶; every collinear state's `z_β` at its clamp image with
  `lookup_clamped` set and no equilateral state's set, bit for bit; max `|w − 1|` and max `|w|` within REQ-VAL-121's
  bounds; controls: a state moved off the line by one ulp is caught, and a bound tightened below the measurement fails
  (REQ-VAL-022).
- `cargo xtask gate inspector-handles` — REQ-VAL-021's arm at 1.6×10⁻¹⁴ unchanged; the Randomise arm's max `‖Δz‖`
  per handle reported over seeded draws; control: feeding the Randomise arm's maximum into REQ-VAL-021's verdict fails
  (REQ-VAL-193).
- Calibration: the shape bounds and the Randomise arm's bound proposed from the Rust measurement (RQ-475's evidence:
  2.2×10⁻¹⁶ and 6.8×10⁻¹⁴; qa's 1.82×10⁻¹⁴ and 1.69×10⁻¹⁴ on Randomise), reviewer-checked, confirmed by the human at
  the M2 gate, recorded in `decisions.md` (REQ-VAL-121).

## Notes
- Applied per R-369: the held requirements move here from TASK-M2-27 (merged); REQ-VAL-021 stands as built there.
- Applied per R-369: this task depends on TASK-M2-43, because R-547 changes canonicalisation's step 0, which
  the round trip runs; the bounds are measured on the final `C`.
- The pole check runs at equal masses only, where `L±` are the poles (R-549, RQ-355); at unequal masses the
  equilateral configuration is not at `w = ±1`.
