# TASK-M8-120 — The symmetry filter: a display option after rendering, GUI and export

- **Milestone:** M8
- **Closes:** REQ-CHART-135, REQ-CHART-136, REQ-CHART-137, REQ-CHART-138, REQ-GUI-424
- **Depends on:** TASK-M4-37, TASK-M8-112, TASK-M8-103, TASK-M8-32
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The owner's order of 7 Nov: an optional toggle that draws as transparent every pixel whose starting state is the symmetry image of another start present in the same chart, keeping one canonical representative of each pair. A display option applied after rendering, never a change to what's computed.

## References
- `docs/design/principia_chart_composition.md` § "6.6 The symmetry rule"
- `docs/design/principia_chart_composition.md` § "10.4 The symmetry filter"
- `docs/design/principia_chart_composition.md` § "8.8 Region D must match the papers (order item A5)"
- `docs/design/principia_chart_composition.md` § "6.2 Labels: reduced capability, never a refusal"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"

## Deliverables
- `crates/engine/src/chart_authoring/symmetry.rs`: the per-chart declaration, extensible; the partner check through signed `C` and the chart's own axis inverses, never the contract's `C` or `Enc`; under a swap, body-indexed outcome fields permuted; the CPU mask, off the sim key.
- The display pass's and the export writer's read of that mask.
- `crates/gui/src/windows/chart_builder/`: the toggle, "no duplicates in this chart", and the export dialog's toggle.
- The legend's and the sidecar's statement of the canonical choice.
- Each paper-frame preset's default on.

## Acceptance tests
- `cargo test -p engine symmetry_filter_display_only` — on and off: one sim key, payload and statistic, bit for bit; a new declared symmetry changes no filter code; control: the filter feeding a statistic (REQ-CHART-135).
- `cargo test -p engine symmetry_filter_rest_reconstructs` — the owner's test: on a rest chart declaring the mirror only, one copy, filtered + its mirror reproduces the full chart bitwise, outcome fields equal, signed fields flipped; mutations: a signed field not flipped, the canonical side off by one row (REQ-CHART-136).
- `cargo test -p engine symmetry_filter_fixed_velocity` — the owner's test: on a fixed-velocity shape-sphere chart, the `p_ρ = (0.1, 0.1)`, `p_λ = (0, 0)` fixture relative to `ρ̃` on `+x`, `Π·p ≠ p` asserted after signed `C` first, the filter blanks nothing and reads "no duplicates in this chart"; a partner out of view and in the domain still blanks; the swap blanks nothing at unequal masses and at eq. 3's near-rest start; the same chart at `p_ρ = (0.1, 0)` does blank; mutation: the partner check removed, blanking by the map's `(θ, π − φ)` alone (REQ-CHART-137).
- `cargo test -p engine symmetry_filter_statement` — the legend and sidecar state the choice; a paper-frame preset opens on, any other chart off; a target's score is one with the filter on or off; controls: the statement dropped, the default on for every chart (REQ-CHART-138).
- `cargo test -p gui symmetry_filter_toggle` — the toggle in both places, its no-partners state, one click to the full disc with no new run; control: the toggle left live with no partners (REQ-GUI-424).

## Notes
- The optional later "skip computing duplicates" export setting, off by default and labelled, is not built here, and no task owes it yet.
- The test names are this design's; the two the order asks for are the owner's.
- The mirror-step proposal (design § 6.6) isn't applied: the filter reads the signed map and doesn't assume it.
- Waits on its rulings item: RQ-736, whose conflicts include what "bitwise" means for its fixture and which preset the filter can't take to D. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
