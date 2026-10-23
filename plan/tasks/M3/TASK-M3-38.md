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
(TASK-M3-12) are not on main; they are this task's (REQ-CHART-085). Taking them from the M2 gate to M3 is a deferral,
ruled by the human: R-579 (23 Oct 2026, RQ-523 (a)), which moves them here, after TASK-M3-04 and TASK-M3-12. *Was:*
"they are drafted as this task's (REQ-CHART-085). Taking them from the M2 gate to M3 is a deferral that waits on the
human's ruling, RQ-523 (code review 5605370280 of PR #276, F1); REQ-CHART-085 holds on it (`rq: RQ-523`), and this
task's run arms wait on it." and, before that, "This is not a deferral: the arms move to the task that can build
them."

- **The exactly mirrored run.** Where REQ-CHART-082's classification reports mirror-equivariant, the run from `Π·x`,
  `Π` applied to a decoded state, is the run from `x` mirrored, bitwise (R-544 §4, "bitwise where the integration is
  mirror-exact"): every sampled state reflected through the x-axis, the outcome and `t_end` equal, `L_z` and the
  frame's winding the negations of `x`'s.
- **Across pixel pairs.** The two pixels `(θ, φ)` and `(θ, π − φ)` decode to mirror images only to the decode's
  rounding ("else to rounding"), so their fates are compared on a named fixture whose pair runs the test shows to stay
  close over its horizon: outcome and `t_end` equal, `L_z` and the frame's winding opposite.
- **The frame's winding** (REQ-CHART-086, R-72). chart_reference §3.3 calls the winding one of "the signed ones" and
  no passage defines it. This task writes its definition into §3.3 for the physics reviewer: which vector's turn in
  the plane it accumulates, at what cadence, from what start, its value where that vector vanishes or at a
  collision, and its value at a per-step turn of exactly `±π`: dd_integrator §3.7 closes the wrap at `+π` (R-389), so
  a turn of `+π` mirrors to `−π`, which wraps back to `+π`, and the negation under `Π` breaks at that tie; the
  definition either sets the tie so the negation stays exact or the test excludes it explicitly (physics review
  5635496144 of PR #276, P2; applied per R-369). *Was:* "and its value where that vector vanishes or at a
  collision." It is not `θ̃` (REQ-INT-001), the shape sphere's longitude, which the mirror leaves unchanged
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
- `decisions.md` § "R-579 — The mirror test's run arms move to M3: REQ-CHART-082's runs become REQ-CHART-085, closed by TASK-M3-38 after TASK-M3-04 and TASK-M3-12; REQ-CHART-082 keeps at M2 the classification, the "does not apply" case, the shape and state pair checks and the bitwise `Π·x` check *(closes RQ-523; a deferral by ruling, from the M2 gate to M3; settles the split R-544's dated line drafted)*"

## Deliverables
- Test `shape_sphere_mirror_runs` (`crates/kernel`), using TASK-M2-43's classification and decode, the wrapper's
  `integrate` and the outcome classifier, with its controls and its named fixture (masses, pairs, horizon and the
  pair-separation bound stated in the test).
- The frame's winding: its definition in `docs/design/principia_chart_reference.md` § "3.3 The chart map", beside
  "the signed ones (`L_z`, the winding)" (a port adds; the commit's Removed lines note as usual), and its computation
  from a run's states, with test `frame_winding`.

## Acceptance tests
- `cargo test -p kernel shape_sphere_mirror_runs` — REQ-CHART-085's verify: released from rest, and with a held
  momentum with zero y-components, the run from Π·x, Π applied to a decoded state, is the run from x mirrored,
  bitwise, at every sampled state (positions and momenta reflected), its outcome and t_end equal, its L_z and frame's
  winding the negations of x's; on a named fixture (the masses, the pairs and a horizon stated in the test, which the
  test shows non-chaotic there by the pair's state separation staying below a stated bound over the horizon) the
  outcome and t_end are equal and L_z and the frame's winding opposite across each pixel pair; with a held p_λ,y ≠ 0
  it reports "does not apply" and asserts nothing; controls: R-141's gauge left in (L_z and the winding equal, not
  opposite), a momentum rule that turns p_x below the equator (outcome, t_end or winding broken), a configuration-only
  mirror in the every-sampled-state comparison, applied where the run's momenta have y-components (the test asserting
  that some sampled state has a p_y ≠ 0; sampled states not reflected), and a fixture whose pair separation passes its
  bound over the horizon (the test refuses it as chaotic), each fail (REQ-CHART-085). *Was:* "a configuration-only
  mirror for `Π` (`L_z` not opposite)": the test runs where the held momenta's `y`-components are zero, so at the
  initial state that mirror is `Π` and cannot fail (qa review 5635487490 of PR #276, Q3; applied per R-369). The line
  is regenerated from the verify, word for word (applied per R-369); the paraphrase it replaces said the same.
- `cargo test -p kernel frame_winding` and review — REQ-CHART-086's verify: chart_reference §3.3 states the frame's
  winding: the vector whose turn it accumulates, the cadence, its start at t = 0, its value where the vector vanishes
  or at a collision, and its value at a per-step turn of exactly ±π (set so the negation under Π stays exact, or
  excluded from the test explicitly); cargo test -p kernel frame_winding: a rigid rotation through 2πk winds k turns,
  the run from Π·x winds the negation of x's, bitwise (if the definition excludes the ±π tie, on runs the test shows
  free of a per-step turn of exactly ±π); control: θ̃ in its place is not opposite under Π, and fails; physics
  reviewer approved (REQ-CHART-086). *Was:* "the value where the vector vanishes or at a collision);" and "winds the
  negation of `x`'s, bitwise;" (physics review 5635496144 of PR #276, P2; applied per R-369). The line is regenerated
  from the verify, word for word (applied per R-369); the paraphrase it replaces said the same.

## Notes
- **⚑ Physics review is required** (R-544 is in the physics group); it does not merge on the code reviewer's word
  alone.
- **The split, ruled by R-579 (TASK-M2-43's pre-flight, 21 Oct 2026; R-544's dated line).** REQ-CHART-082's run
  arms split into REQ-CHART-085, this task's, since TASK-M2-43 (after TASK-M2-08, TASK-M2-14 and TASK-M2-17) cannot reach a
  stepper, a wrapper or an outcome classifier; REQ-CHART-086 (R-72) added for the winding, which chart_reference
  §3.3's whole-sphere paragraph uses ("the signed ones (`L_z`, the winding)") with no definition; REQ-CHART-086 does
  not wait. *Was:* "**Applied per R-369 (TASK-M2-43's pre-flight, 21 Oct 2026; R-544's dated line).**"
  **Ruled by R-579 (23 Oct 2026, RQ-523 (a)):** the split stands and is no longer a draft. Depends on keeps TASK-M2-43
  beside the two R-579 names, TASK-M3-04 and TASK-M3-12, since this task's test uses TASK-M2-43's classification and
  decode (applied per R-369). *Was:* "**A draft that waits on RQ-523 (TASK-M2-43's pre-flight, 21 Oct 2026; R-544's
  dated line).**"
- **RQ-537** (ruled, R-580): dd_integrator §5 test 8's "`retrograde` matches the L_z sign" against the mirror, which leaves
  `θ̃` unchanged and negates `L_z`; the frame's winding defined here must not lean on test 8's reading.
  **Ruled by R-580 (23 Oct 2026):** the clause is dropped from test 8, which gains the mirror pair in its place (the run
  from `Π·x` has `θ̃(t)` and `retrograde` bitwise identical and `L_z` opposite; REQ-INT-082, TASK-M3-11). So `θ̃` is
  mirror-invariant by ruling as well, and the frame's winding, signed under `Π`, is not `θ̃`. *Was:* "**RQ-537**
  (open)".
- The precision follows R-544's "The mirror test's precision" (physics review 5603309678 of PR #273, P2): bitwise only
  for the exactly mirrored state; across pixel pairs, a named non-chaotic fixture, since a rounding-level difference in
  the IC changes a chaotic fate (physics.md §7, R-84).
- **Stop and queue** if the integration is not mirror-exact for the stepper the wrapper runs (an operation whose
  rounding is not symmetric under `y → −y`), rather than loosening "bitwise" to a tolerance.
