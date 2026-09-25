# TASK-M6-04 — The stop rule: alpha_area over two levels, neighbour agreement, and the R-42 fixes

- **Milestone:** M6
- **Closes:** REQ-REF-015, REQ-REF-016, REQ-REF-017, REQ-REF-023, REQ-REF-024, REQ-REF-025, REQ-REF-040, REQ-REF-043
- **Depends on:** TASK-M6-03
- **Needs (earlier milestones):** REQ-REF-001, REQ-REF-002, REQ-PAY-076, REQ-PAY-077
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3, PIT-7
- **Size:** ~420 lines

## Goal
The `Floor` decision exists: `alpha_area = log2(unresolved_area(coarse) / unresolved_area(children))`, judged over two levels from the grandparent's quadrant, compared with `alpha_lo = 0.005`; an empty mask is told from a full one by `n_unresolved` and the floor is refused on a negative exponent (R-42). Unresolved footprints count as structure only by neighbour agreement (≥ 2 of 8 neighbours sharing class within the calibrated angle on the shape sphere). `QuadReduction`'s refinement members `alpha_area`, `alpha_energy`, `worst_energy_drift` are f16; `|alpha_energy − 1| > 0.05` marks a quad's exponents untrustworthy; `alpha` is ordinal only. The task proposes the neighbour-agreement angle (R-71).

## References
- `docs/design/principia_dd_generation_root.md` § "Refinement — the scaling exponent"
- `decisions.md` § "R-42 — The `alpha_area` defects: tell empty from full by `n_unresolved`; refuse the floor on a negative exponent *(RS-2 (b)+(c))*"
- `decisions.md` § "R-73 — Apply the whole ruling-follow-up checklist now *(closes RQ-56)*"
- `docs/design/principia_dd_refinement_policy.md` § "2. The stop rule — `alpha_area` is a DIMENSION, not a tuned constant"
- `docs/design/principia_dd_refinement_policy.md` § "2.1 Two structural details, each of which cost a measurement"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `docs/read_first/principia_INDEX.md` § "Known open items"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `docs/design/principia_dd_refinement_policy.md` § "2.2 TWO KNOWN DEFECTS IN `alpha_area` — both ruled (R-42)"
- `decisions.md` § "R-107 — Apply the RQ-67 follow-ups; GUI_DESIGN_NOTES may be conformed *(closes RQ-67)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/engine/src/refine/alpha_area.rs`: two-level exponent from the grandparent quadrant; `n_unresolved` empty/full discrimination; negative-exponent refusal; `alpha_lo` default 0.005 in config.
- `crates/engine/src/refine/structure.rs`: per-footprint neighbour agreement with the angle as a named config value.
- Ledger members `alpha_area`, `alpha_energy`, `worst_energy_drift` (f16) in `crates/ledger`; `alpha_energy` trust flag.
- Fixtures in `fixtures/gates/alpha_area/`: synthetic line/sea masks, the 15-split shore fixture, empty/full/negative-exponent masks, white-noise and coherent structure fields.
- `cargo xtask gate alpha-two-level` and `cargo xtask gate neighbour-angle` (the calibration evidence).

## Acceptance tests
- `cargo test -p engine refinement_members` — member types; alpha_area computed per refinement_policy §2 (REQ-REF-015).
- `cargo test -p engine alpha_energy_trust` — a quad with alpha_energy = 1.1 is flagged (REQ-REF-016).
- Review checklist (code) — no code computes 2^-alpha as a gain forecast (REQ-REF-017).
- `cargo test -p engine alpha_area_masks` — on synthetic masks, a line reads 1, a sea reads 0; default config has alpha_lo = 0.005 (REQ-REF-023).
- `cargo xtask gate alpha-two-level` (fixture `fixtures/gates/alpha_area/shore`) — on a line (shore) fixture across 15 splits, alpha_area lies in [1.00, 1.05] (the one-level form reads 0 for edge cells) (REQ-REF-024).
- `cargo test -p engine neighbour_agreement` — white-noise fixture: structured count ≈ chance (a few in ten thousand); coherent fixture: neighbours within 11° count (REQ-REF-025).
- `cargo test -p engine alpha_area_r42_defects` — Empty-mask and full-mask fixtures give different results (not both 0.0000); a negative-exponent fixture is refused (no d > 2); alpha_lo = 0.005 (REQ-REF-040).
- `cargo xtask gate neighbour-angle` — decisions.md records the exact angle with its evidence: structured counts on the white-noise fixture (≈ chance) and on coherent fixtures at the chosen angle; proposal with its evidence in the PR, reviewer-checked; the human confirms the value at the M6 gate and it is recorded in `decisions.md` (REQ-REF-043).

## Notes
- Calibration proposed: REQ-REF-043 (the ~11° angle). REQ-REF-025's test uses the calibrated value (tolerance: REQ-REF-043 (calibrated)).
- PIT-9: the empty/full test must be able to fail — assert different results, not `!= 0`.
