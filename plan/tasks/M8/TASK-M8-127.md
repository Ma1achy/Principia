# TASK-M8-127 — The acute-angle Burrau charts over the full acute range, both labellings, the leg swap through the symmetry filter (R-875)

- **Milestone:** M8
- **Closes:** REQ-CHART-151
- **Depends on:** TASK-M8-120
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines, the tests included

## Goal
R-875 (RQ-745 (b)): the three acute-angle Burrau charts run over the full acute range, `θ ∈ (0, π/2)`, both labellings
of each shape shown, and the leg swap (`θ` and `π/2 − θ`) shown once only when the symmetry filter is on, declared as
that chart's symmetry. R-27's "shape only" fold gives way for these charts.

## References
- `decisions.md` § "R-875 — The acute-angle Burrau charts are re-expressed over the full acute range, both labellings of each shape shown, the leg swap shown once only by the symmetry filter; R-27's "shape only" fold charts give way *(closes RQ-745; amends R-27)*"
- `decisions.md` § "R-27 — Both Burrau charts are kept, each labelled with its quotient *(CD-7)*"
- `decisions.md` § "R-855 — The symmetry filter: an optional display option after rendering, never a change to what is computed, blanking a start only where its partner is in the chart's domain, its canonical choice stated, on by default for the paper-frame presets only"
- `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps"
- `docs/design/principia_chart_composition.md` § "10.4 The symmetry filter"

## Deliverables
- The three charts' decode over `θ ∈ (0, π/2)` in `crates/kernel`, the leg swap declared as each chart's symmetry for the
  filter (TASK-M8-120), the fold removed.
- REQ-CHART-024, REQ-CHART-025 and REQ-CHART-045 restated to the built charts through `plan/tools/reqio.py`, each keeping
  a "Was" (their notes say so).

## Acceptance tests
- `cargo test -p kernel burrau_acute_full_range` — each of the three charts decodes `θ` and `π/2 − θ` to the same
  triangle with its legs swapped, both drawn; with the filter on the drawn half reproduces today's shape-only chart bit
  for bit; controls: the fold left in, the swap undeclared, each failing (REQ-CHART-151).

## Notes
- Added by R-875 (8 Nov 2026), applied per R-369 for its placement: outside TASK-M4-37's gate, since it rebuilds three
  gallery charts, it waits on TASK-M8-120 alone, whose filter shows the swap once.
- The range comes back to the owner at the M2 gate within `(0, π/2)` (REQ-CHART-045's note).
