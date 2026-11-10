# TASK-M3-70 — The drift views' neutral band: scaled by the rounding, and on the field-ramp path (R-928, R-929)

- **Milestone:** M3
- **Closes:** REQ-TOOL-227
- **Depends on:** TASK-M3-05
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines, the tests included

## Goal
symlog's neutral band on a current drift scales with the rounding itself, never with the size of the reference: a drift with `|x| ≤ k'·2⁻²³·(K + |V|)` for the energy, or `|x| ≤ k'·2⁻²³·S` for `L_z` with `S = Σ(|xᵢp_{y,i}| + |yᵢp_{x,i}|)`, is drawn as 0, `k'` from the rounding bound (15 with a CPU reference and 22 with a GPU one for the energy; 5 and 6 for `L_z`), in place of TASK-M3-05's `k·2⁻²³·|reference|` with `k = 128` (R-928). The band is applied where the drift is read, so a field ramp on `energy_drift` or `Lz_drift` has it as the three drift debug views do; `compact_symlog` itself is unchanged (R-929). `lin` and `log` keep no band, a non-finite operand passes through first, and the value passed to `dbg_suspect` stays the drift.

## References
- `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug view)"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `decisions.md` § "R-928 — RQ-637, `symlog`'s neutral band near a zero reference: option (b), the band scales by the rounding itself, `k'·2⁻²³·(K + |V|)` for the energy and `k'·2⁻²³·S` for `L_z`, `k'` from the bound *(closes RQ-637; amends R-652)*"
- `decisions.md` § "R-929 — RQ-638, the neutral band on a field ramp: option (a), the band is applied where the drift is read, so a field ramp on `energy_drift` or `Lz_drift` has it, and the fresh-sample test is extended to that path *(closes RQ-638; refines R-652)*"
- `decisions.md` § "R-652 — The four drift goldens are re-baselined as TASK-M3-05 renders them, each `BASELINES.md` row leading with R-652, the derived case at `lin`; they are confirmed at the M3 gate *(closes RQ-625)*"
- `decisions.md` § "R-931 — Goldens: a golden or reference image that is provably wrong is fixed, everything that depends on it is updated, and tests run against the fixed version; an incorrect thing is never treated as correct, and a bug is never replicated for the sake of matching it *(no RQ; refines R-110)*"

## Deliverables
- The band of render contract Part 5's dated note of 10 Nov 2026, in the WGSL and CPU-mirror readers of `energy_drift` and `Lz_drift`: the three drift debug views and the field-ramp path, one definition.
- Render contract Part 5 gains `k'` and the rounding bound it comes from, written out, for the physics reviewer.
- The extended `drift_view_neutral_fresh_sample_renders_exactly_neutral` test in `crates/render`, with its controls (R-176).

## Acceptance tests
- `cargo test -p render drift_view_neutral_fresh_sample_renders_exactly_neutral` — a fresh sample renders exactly neutral on every backend in the three drift debug views and through a field ramp on `energy_drift` and on `Lz_drift`, on every sample of the test's set, those with `E_0` or `Lz_0` near zero included (dd_integrator test 2's fresh figure-eight among them); a drift just past the band keeps its own place; controls: the band relative to `|E_0|` or `|Lz_0|` (`k = 128`) fails on the near-zero samples, and a field ramp without the band fails the field-ramp arm (REQ-TOOL-227).
- Review (physics): `k'` and its rounding bound, as written in render contract Part 5 (REQ-TOOL-227).

## Notes
- New (10 Nov 2026), from R-928 (RQ-637 (b)) and R-929 (RQ-638 (a)). Until it merges the band is TASK-M3-05's.
- Any golden the change moves is reported with its reason and re-baselined only as R-110 and R-931 allow.
- A value the contract's note leaves out, if the build finds one, is a calibration requirement (R-71), never a guess.
