# TASK-M8-119 — The virial pair: its axis kind, descriptor and entry

- **Milestone:** M8
- **Closes:** REQ-CHART-103, REQ-CHART-139
- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M2-37, TASK-M8-117
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
The kernel's `AxisVirial` is built on TASK-M2-37: this gives it the descriptor the validator needs, the registry's row, and a baked `(configuration axis, Q)` pair inside the cap.

## References
- `docs/design/principia_chart_composition.md` § "9. Then virial"
- `docs/design/principia_chart_composition.md` § "3. The primitive registry"
- `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid"
- `decisions.md` § "R-817 — A warped virial axis with an asked bottom `Q₀ > 0` at a held `L_z ≠ 0` puts the floor under the rise only: the pixel's ratio is `max(Q₀, Q_min(U)) + (Q_hi − Q₀)·t^γ_K` *(closes RQ-700)*"
- `decisions.md` § "R-877 — A warped virial axis at a held `L_z ≠ 0` with `Q₀ > 0` is labelled `Q`, the ratio, the columns where `Q₀` is infeasible marked as the floor is, and the state's own ratio in the readout *(closes RQ-741)*"

## Deliverables
- `crates/engine/src/chart_authoring/`: `InvariantQuantity`'s or its kind's row, as RQ-717 rules.
- One baked entry.
- `kernel::chart::physical`'s virial axis at `Q₀ > 0` and a held `L_z ≠ 0`: the floor under the rise only, `max(Q₀, Q_min(U)) + (Q_hi − Q₀)·t^γ_K`, the additive form unchanged at `Q₀ = 0` (R-817, RQ-700 (b); REQ-CHART-139).

## Acceptance tests
- `cargo test -p engine virial_descriptor` — the validator takes it; refused beside hold E and a K axis; forward by mapping: linear agrees with `virial_ratio`; warped at a held L_z ≠ 0 it reads the axis's own coordinate, the inverse of R-817's decode, `Q₀ + 2(K − max(Q₀·|U|/2, L_z²/2I))/|U|`: at `Q₀ = 0` that is `2(K − L_z²/2I)/|U|`, labelled `Q above the floor`; at `Q₀ > 0` the state's own ratio `2K/|U|` where `Q₀ ≥ Q_min(U)` and that ratio less `Q_min(U) − Q₀` elsewhere; `forward(inverse(x)) = x` on every arm, at `Q₀ = 0` and at `Q₀ > 0` on columns with `Q_min(U)` below, at and above `Q₀` (`Q₀ = 1`, `Q_min(U) = 0.25`, `t = 0` reads 1); controls: no descriptor refused, the warped row read as the state's ratio at `Q₀ = 0`, the `Q₀ = 0` form on a `Q₀ > 0` axis where `Q₀ ≥ Q_min(U)`, mutation: a factor changed in the inverse, the row being a scale solve (REQ-CHART-103). The label at `Q₀ > 0` is RQ-741's.
- `cargo test -p kernel virial_floor_under_rise` — REQ-CHART-139's arms and control.
- `cargo test -p engine virial_descriptor` also: at `Q₀ > 0` the axis is labelled Q, a column with `Q_min(U) > Q₀` marked and one with `Q_min(U) ≤ Q₀` unmarked, the readout the state's own ratio (R-877; REQ-CHART-103).

## Notes
- **Build order (R-863):** stage 3, the primitive registry and composed axes, each primitive with its round-trip test. It depends on stage 2's task, TASK-M8-117, and merges only with its tests green.
- *Was, until R-817:* RQ-700 is outside TASK-M4-37's gate, so nothing in Depends on holds it: the arm for a held L_z ≠ 0, where the axis reads above the floor (§ 9), doesn't start until RQ-700 is ruled, by this note. TS-2 stays "not scored: no fixed-k velocity chart" (R-669).
- Waits on its rulings item: RQ-717. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
- *7 Nov 2026 (R-816, R-838, R-817):* the virial ratio is kind 3 (RQ-682 (a), RQ-717), and RQ-700 is ruled (b). Both waits end.
- *7 Nov 2026 (physics review of PR #443, N1):* the floor is computed in `K`, `K = max(Q₀·|U|/2, L_z²/2I) + (Q_hi − Q₀)·t^γ_K·|U|/2`, not as the ratio `max(Q₀, Q_min(U)) + …` multiplied by `|U|/2` afterwards: `L_z²/(I·|U|)·|U|/2` isn't `L_z²/2I` in floating point, so only the form in `K` keeps the decode at `Q₀ = 0` bit for bit the built additive form's (REQ-CHART-139). The ratio form in chart_reference § 2.1 is the same quantity, read in `Q`.
- *7 Nov 2026 (Codex on PR #443):* the forward and its round trip follow R-817's decode: the axis's coordinate is chart_reference § 2.1's `Q(t) = Q₀ + (Q_hi − Q₀)·t^γ_K`, so on a column where `Q₀` is feasible it is the state's own ratio, and `Q above the floor` holds only at `Q₀ = 0`. Waits on RQ-741 for the label at `Q₀ > 0` only; the rest goes ahead.
- *8 Nov 2026 (R-877):* RQ-741 is ruled (a): at `Q₀ > 0` the axis is labelled Q, the infeasible columns marked as the floor is, the state's own ratio in the readout.
