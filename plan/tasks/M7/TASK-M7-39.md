# TASK-M7-39 — Chazy parity: the f32 against f64 label disagreement, measured and reported

- **Milestone:** M7
- **Closes:** REQ-VAL-183
- **Depends on:** TASK-M7-36, TASK-M4-04, TASK-M4-05, TASK-M4-18
- **Needs (earlier milestones):** REQ-VAL-060, REQ-VAL-061, REQ-VAL-057
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~250 lines (the gate harness and its report)

## Goal
The Chazy class flips on the sign of a two-body energy and on band edges in `K/U`, so CPU-f64 and GPU-f32 can differ
at samples within ulps of a sign change (R-442; the Chazy drill-down §8). Measure, on a gallery chart at several
playheads, the fraction of samples whose class differs between the two, report it, and check that every differing
sample lies within the band of a sign change. The prototype never measured f32 (§9 item 2); the measurement comes first,
and the fraction is reported with no threshold: `chazy_class` is a label on a real trajectory, and the rate and the
band check are the gate (R-548, RQ-314; *was:* "and the threshold on the fraction follows RQ-314").

## References
- `docs/design/principia_dd_chazy_final_motion.md` § "8. Parity"
- `docs/design/principia_dd_chazy_final_motion.md` § "9. Gaps between the prototype and Principia"
- `docs/design/principia_dd_chazy_final_motion.md` § "12.2 New, for Principia"
- `docs/contracts/principia_parity_contract.md` § "2. The three tiers"
- `docs/contracts/principia_parity_contract.md` § "Tier N — tight numerical (checkable because nothing amplifies)"
- `decisions.md` § "R-442 — Chazy final motion is a separate system with its own stains: hue carries the class, B is black `#141418` and one class, collision is magenta `#DA85B6`, transient draws as B, the working states have their own status map, and the seven-class palette is the default"
- `decisions.md` § "R-548 — The Chazy entries ruled: the seven-class palette stays the default; the subtitle ban covers captions over the figure only; the class freezes at escape fire and may differ from the escape event, the legend saying so; the class is a label, reported, not asserted; OS-candidate and B-stable ship marked provisional; B `#141418` and the grey `#8A8A90` are near-neutral, held to chroma ≤ 0.009; the rule keeps transient before B-stable, puts B-stable before excursion and lets H, HP, P and the two-stage H fire only at E ≥ 0; a render-side previous-class texture; the one-body views read `chazy_escaper`; the three panels are one figure *(closes RQ-311 to RQ-318 and RQ-320 to RQ-325)*"

## Deliverables
- `cargo xtask gate chazy-parity`: classify a gallery chart on CPU-f64 and GPU-f32 (native wgpu) from the same states
  at several playheads; report the disagreement fraction per playhead; for each differing sample, report the nearest
  sign change or band edge and its margin.

## Acceptance tests
- `cargo xtask gate chazy-parity` — prints the disagreement fraction over samples at each playhead and fails if a differing sample is not within the band of a sign change of a two-body energy or a `K/U` band edge; the fraction is reported, not asserted, with no threshold on it (R-548, RQ-314) (REQ-VAL-183).

## Notes
- R-548 (RQ-314, option (a)): `chazy_class` is the parity contract's "a label on a real trajectory", reported, not
  asserted; REQ-VAL-183's measured rate and its band check are the gate, and it no longer waits. *Was:* "How a
  categorical label fits the parity tiers (the spec's "Tier N" against the contract's "a label on a real trajectory")
  is RQ-314; REQ-VAL-183 waits on it for the threshold only. The measurement itself does not wait."
