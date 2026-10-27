# TASK-M8-44 — The shape sphere past [0,1]² as the whole sphere: no hemisphere mode, and the area statistics over the whole sphere (R-544) ⚑

- **Milestone:** M8
- **Closes:** REQ-CHART-057, REQ-CHART-083
- **Depends on:** TASK-M8-87, TASK-M2-28
- **Needs (earlier milestones):** REQ-CHART-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
**Re-scoped by R-544 (21 Oct 2026).** The shape sphere chart is the whole sphere, so past `[0,1]²` it has no
hemisphere mode: φ's span is R-14's `φ = π(1 − t)`, pole to pole, in every view; φ is pole-crossing at both edges and
nothing is folded at the equator, a pixel below it its own system; there is no remap and no toggle (R-544, withdrawing
R-412). TASK-M8-05 builds the extension types and the counting on the whole sphere; this task makes sure no
shape-sphere descriptor, extension type or classifier carries a hemisphere mode, a remap or an equator fold, and adds
the whole-sphere area statistics: each pixel its own system, counted once, no statistic folding the lower hemisphere
onto the upper, and where R-544 item 5's symmetry holds every fate of a mirror-invariant class counted twice, equally,
so those fractions are equal on a fixture whose pairs stay close over the horizon, and elsewhere differ only by
the pairs whose fate a rounding-level difference in the decode flips, a signed class mapping to its mirror class
(physics reviews 5603309678, P2, and 5603529679, P7, of PR #273, applied per R-369).

## References
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `decisions.md` § "R-412 — With one hemisphere of the shape sphere drawn, φ's span is remapped to end at the equator and its formula continues past it; the primary range is pole to equator *(closes RQ-264)*"
- `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity failures are forbidden *(closes RQ-263)*"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"

## Deliverables
- In `crates/kernel/src/chart/`, the shape sphere's φ axis pole-crossing at both edges with no hemisphere mode, read by
  TASK-M8-05's extension types and classifier.
- Cases added to the tests `chart_extension` and `area_stats_primary_range` for the whole sphere, and the test
  `area_stats_whole_sphere`.

## Acceptance tests
- `cargo test -p kernel chart_extension` and `cargo test -p kernel area_stats_primary_range` and Review checklist
  (physics reviewer) — REQ-CHART-057's verify: `t = 1` and `t = 0` are the poles and `t = ½` the equator; past either
  pole φ is pole-crossing (θ + π); a pixel at `(θ, φ)` and its mirror `(θ, π − φ)` decode to two different systems and
  both are counted inside `[0,1]²`; no shape-sphere descriptor, extension type or classifier carries a hemisphere mode,
  a remap or an equator fold; controls: R-412's remap `φ = (π/2)(1 − t)`, a φ axis folded at the equator, and a
  classifier counting only `w ≥ 0`, each fail; the physics reviewer checks it against R-544 (REQ-CHART-057).
- `cargo test -p kernel area_stats_whole_sphere` and Review checklist (physics reviewer) — REQ-CHART-083's verify:
  released from rest over a grid symmetric about the equator, every pixel integrated once and counted once; on a named
  fixture whose pair runs the test shows non-chaotic over its horizon, a mirror-invariant class's count (outcome,
  escaper, collision pair) over the whole sphere twice its count over the upper half and its fraction equal, and a
  signed class's count over the lower half its mirror class's over the upper; with a held momentum that is
  not mirror-equivariant the counts are taken as they come, no symmetry assumed; controls: a count folded to `w ≥ 0`
  over the whole sphere's total, and a fold left in that copies the lower half from its twins, each fail
  (REQ-CHART-083).

## Notes
- **⚑ Physics review is required** (R-544, R-408); it does not merge on the code reviewer's word alone.
- **Re-scoped by R-544.** *Was:* "The shape sphere past [0,1]² with one hemisphere drawn: φ's extension and primary
  range (RQ-264)", building R-412's remap, the continuation into the mirror hemisphere and the one-hemisphere primary
  range, after TASK-M8-06's toggle. R-544 withdraws R-412 and the toggle, so the dependency on TASK-M8-06 goes; it
  depends on TASK-M2-28 for the whole-sphere render (REQ-CHART-002), and through it on TASK-M2-43's signed decode.
  `checks/r544_check.py`'s check 6 is the port's model of REQ-CHART-083.
- Split from TASK-M8-05 (applied per R-369, R-408 B4, code review 5478800754, F4). TASK-M8-55 and TASK-M8-56 depend
  on it.
- A statistic measured under one hemisphere says so, beside R-544 (R-544 item 7). None in the corpus was (R-544's
  "Results measured under one hemisphere").
- R-627 (27 Oct 2026): this task runs on the real engine, so where it depended on a task R-627 moves to the mock it depends on that task's real-engine side, which depends on it: TASK-M8-05 by TASK-M8-87 (applied per R-369). *Was:* "- **Depends on:** TASK-M8-05, TASK-M2-28".
