# TASK-M8-54 — The trace fade's general off-plane distance in the pulled-back mass metric (R-509 part 3, R-550 J7, R-596) ⚑

- **Milestone:** M8
- **Closes:** REQ-GUI-266, REQ-GUI-400
- **Depends on:** TASK-M8-51
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~250 lines, the tests included

## Goal
On a chart with no proxy, a trace fades off the plane by the general distance, the distance in `z` from the trajectory's encoded point to the slice plane, encoded per step, in the metric `g = Jᵀ W J` on `z` (R-596, RQ-522 (b)): `J` the decoder's own Jacobian `∂(r, p)/∂z` of its particle-frame output (after canonicalisation, the scale gauge and `η_E`), eight columns, computed by REQ-DEC-049's method (f64, central difference, REQ-DEC-034's step rule, `J_D`'s as deep_zoom §2 writes it, R-633)
at the slice centre, not R-550's compass-view Jacobian; `W` the mass-weighted (kinetic) metric on positions and its dual on momenta, in canonical units, its masses those decoded at the slice centre
(physics review 5644374613 of PR #293, F1; *was:* "`J` R-550's eight-column f64 central-difference Jacobian `∂x/∂z` at the slice centre");
the distance is the `g`-length of the part of `z − z₀` `g`-orthogonal to the slice plane (applied per R-369). Where `g`
is singular along a direction of `z`, this task defines the reading in lines §8 (REQ-GUI-400, R-72), the physics
reviewer approving it, and REQ-GUI-266's verify checks it. TASK-M8-51 draws no fade there meanwhile. *Was:* "physics
review here settles the reading and its arm". *Was:* "in the metric on `z` RQ-522 rules (R-509 Y3; RQ-341's ruling, R-550, gives the latent
Jacobian and no metric on `z`, R-550 J7)".

## References
- `decisions.md` § "R-509 — `principia_lines.md` §11's six questions are ruled: contours in the stain's expression syntax; the playhead reads any ledger-declared payload field; the trace fade uses the proxy where it is defined, else no fade and says so, the real distance waiting on RQ-341; trace labels win; class edges move to the line system; Screen contour the default, smart ink opt-in *(closes RQ-377; amends R-476)*"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/design/principia_lines.md` § "11. Open questions"
- `docs/design/principia_trajectory_viewing.md` § "3. Hover trace — projection onto the current (tilted, sliced) plane"
- `decisions.md` § "R-596 — The trace fade's off-plane distance is taken in the physical mass metric pulled back through the decoder's Jacobian, `g = Jᵀ W J`, `W` the mass-weighted (kinetic) metric on positions and its dual on momenta, in canonical units *(closes RQ-522; settles R-509 (3)'s and R-550's open metric)*"
- `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision"
- `decisions.md` § "R-550 — The compass's physical views and steer: an eight-column f64 central-difference Jacobian `∂x/∂z` of the decoder, read after `η_E`; Jacobi's eight `α, β`, the four Jacobi momenta and two mass fractions, Shape's `θ, φ, θ̇, φ̇, L_z, Ṙ` and the two mass fractions, both of full rank, so the steer is `J⁻¹` with the projector test refusing; each coordinate divided by a fixed reference range *(closes RQ-341, RQ-342, RQ-343, RQ-344, RQ-345 and RQ-346; amends R-452)*"

## Deliverables
- The per-step encode and the distance in the trace renderer (`crates/render/src/lines/`), in R-596's metric (*was:* "per RQ-341's ruling").
- Test `trace_fade_distance`.
- Doc change: lines §8 — how the distance is read where `g` is singular along a direction of `z` (REQ-GUI-400, R-72).

## Acceptance tests
- `cargo test -p render trace_fade_distance` — REQ-GUI-266's verify.
- Review checklist (physics) — lines §8 states, for a direction of `z` along which `g` is singular at the slice centre,
  how the `g`-orthogonal part of `z − z₀` and its `g`-length are taken, and REQ-GUI-266's verify checks it
  (REQ-GUI-400).

## Notes
- **⚑ Physics review is required**: the distance and its metric; it does not merge on the code reviewer's word alone.
- Definition written here (R-72; the physics reviewer approves before merge): REQ-GUI-400.
- **A leaf task; R-596 rules RQ-522** (RQ-341 ruled by R-550, J7): nothing depends on it. *Was:* "A leaf task waiting
  for RQ-522 (`rq: RQ-522` on REQ-GUI-266; …)", and before it "waiting for RQ-341 (`rq: RQ-341` on REQ-GUI-266)". Numbered past the links port's (unmerged) TASK-M8-52 and TASK-M8-53 (applied per R-369).
- **It supersedes REQ-GUI-249's no-fade arm** on the charts the general distance covers, and its settings line "fade: no off-plane distance on this chart (RQ-341)" there (REQ-GUI-266; R-509 Y3). **qa's file:** TASK-M8-51's qa test of REQ-GUI-249's latent-chart arm (`crates/render/tests/qa_TASK-M8-51.rs`, `trace_fade`'s "every vertex at full opacity and the settings line names RQ-341" on the latent chart) changes in this task's qa commit and only there, the PR listing its `M` line with the reason, the code reviewer confirming no other assertion is weakened (R-290; the ruling forcing the change, applied per R-369, gui review 5587650119 of PR #255, G13).
- *Was (title, before R-618/R-596):* "The trace fade's general off-plane distance, as RQ-522 rules (R-509 part 3, R-550 J7) ⚑".
