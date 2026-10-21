# TASK-M3-38 — The shape sphere's mirror test, its runs: the run from Π·x the run mirrored, bitwise, fates equal and L_z and the frame's winding opposite across pixel pairs on a named non-chaotic fixture, and the frame's winding defined (R-544) ⚑

- **Milestone:** M3
- **Closes:** REQ-CHART-085, REQ-CHART-086
- **Depends on:** TASK-M3-04, TASK-M3-12, TASK-M2-43
- **Needs (earlier milestones):** REQ-CHART-082
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~250 lines, the tests included

## Goal
R-544 item 5 makes the mirror on the shape sphere chart a test, not an identification. TASK-M2-43 builds the test's
static half (REQ-CHART-082): the classification of the held coordinates as mirror-equivariant or not, the "does not
apply" case, the pixel pairs' shape and state checks and the bitwise `Π·x` check. The arms that need real integrator
runs could not be built there, since the stepper (TASK-M3-02), the wrapper (TASK-M3-04) and the outcome classifier
(TASK-M3-12) are not on main; they are this task's (REQ-CHART-085). This is not a deferral: the arms move to the task
that can build them.

- **The exactly mirrored run.** Where REQ-CHART-082's classification reports mirror-equivariant, the run from `Π·x`,
  `Π` applied to a decoded state, is the run from `x` mirrored, bitwise (R-544 §4, "bitwise where the integration is
  mirror-exact"): every sampled state reflected through the x-axis, the outcome and `t_end` equal, `L_z` and the
  frame's winding the negations of `x`'s.
- **Across pixel pairs.** The two pixels `(θ, φ)` and `(θ, π − φ)` decode to mirror images only to the decode's
  rounding ("else to rounding"), so their fates are compared on a named fixture whose pair runs the test shows to stay
  close over its horizon: outcome and `t_end` equal, `L_z` and the frame's winding opposite.
- **The frame's winding** (REQ-CHART-086, R-72). chart_reference §3.3 calls the winding one of "the signed ones" and
  no passage defines it. This task writes its definition into §3.3 for the physics reviewer: which vector's turn in
  the plane it accumulates, at what cadence, from what start, and its value where that vector vanishes or at a
  collision. It is not `θ̃` (REQ-INT-001), the shape sphere's longitude, which the mirror leaves unchanged
  (`u` and `v` are mirror-invariant, only `w` turns). The port's model, `checks/r544_check.py`'s `run()`, accumulates
  the turn of `ρ = r₁ − r₀` by principal-value deltas per step, from 0; it is a model to check against, not the
  definition.

## References
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/design/principia_chart_reference.md` § "3. Shape-sphere chart `(θ, φ)`"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"

## Deliverables
- Test `shape_sphere_mirror_runs` (`crates/kernel`), using TASK-M2-43's classification and decode, the wrapper's
  `integrate` and the outcome classifier, with its controls and its named fixture (masses, pairs, horizon and the
  pair-separation bound stated in the test).
- The frame's winding: its definition in `docs/design/principia_chart_reference.md` § "3.3 The chart map", beside
  "the signed ones (`L_z`, the winding)" (a port adds; the commit's Removed lines note as usual), and its computation
  from a run's states, with test `frame_winding`.

## Acceptance tests
- `cargo test -p kernel shape_sphere_mirror_runs` — REQ-CHART-085's verify: released from rest, and with a held
  momentum with zero `y`-components, the run from `Π·x` is the run from `x` mirrored, bitwise, at every sampled state,
  its outcome and `t_end` equal and its `L_z` and frame's winding the negations of `x`'s; on the named fixture, shown
  non-chaotic by the pair's state separation staying below its stated bound over the horizon, outcome and `t_end` are
  equal and `L_z` and the frame's winding opposite across each pixel pair; with a held `p_λ,y ≠ 0` it reports "does not
  apply" and asserts nothing; controls: R-141's gauge left in (`L_z` and the winding equal, not opposite), a momentum
  rule that turns `p_x` below the equator (outcome, `t_end` or winding broken), a configuration-only mirror for `Π`
  (`L_z` not opposite), and a fixture whose pair separation passes its bound (refused as chaotic), each fail
  (REQ-CHART-085).
- `cargo test -p kernel frame_winding` and review — REQ-CHART-086's verify: §3.3 states the definition (the vector,
  the cadence, the start at `t = 0`, the value where the vector vanishes or at a collision); a rigid rotation through
  `2πk` winds `k` turns; the run from `Π·x` winds the negation of `x`'s, bitwise; control: `θ̃` in its place is not
  opposite under `Π`, and fails; physics reviewer approved (REQ-CHART-086).

## Notes
- **⚑ Physics review is required** (R-544 is in the physics group); it does not merge on the code reviewer's word
  alone.
- **Applied per R-369 (TASK-M2-43's pre-flight, 21 Oct 2026; R-544's dated line).** REQ-CHART-082's run arms split
  into REQ-CHART-085, this task's, since TASK-M2-43 (after TASK-M2-08, TASK-M2-14 and TASK-M2-17) cannot reach a
  stepper, a wrapper or an outcome classifier; REQ-CHART-086 (R-72) added for the winding, which chart_reference
  §3.3's whole-sphere paragraph uses ("the signed ones (`L_z`, the winding)") with no definition.
- The precision follows R-544's "The mirror test's precision" (physics review 5603309678 of PR #273, P2): bitwise only
  for the exactly mirrored state; across pixel pairs, a named non-chaotic fixture, since a rounding-level difference in
  the IC changes a chaotic fate (physics.md §7, R-84).
- **Stop and queue** if the integration is not mirror-exact for the stepper the wrapper runs (an operation whose
  rounding is not symmetric under `y → −y`), rather than loosening "bitwise" to a tolerance.
