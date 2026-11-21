# TASK-M7-75 — The escape-rule agreement view: how many escape rules agree on each pixel's outcome, and which one disagrees

- **Milestone:** M7
- **Depends on:** TASK-M3-34, TASK-M7-35
- **Reviewers:** code, physics
- **Size:** ~250 lines

## Goal
The escape re-validation (TASK-M3-34) makes the escape event's rule selectable: Closure, Chazy, Fewbody, and the
distance and energy floors. This view shows where they disagree. Each pixel shows how many of the selectable
escape rules agree on its outcome at the playhead. A second mode shows which rule disagrees.

Every rule runs on the same march: no extra integration, no second pass. Each rule's state (Chazy's I_max latch,
Closure's settling state, the floors' tests) is carried by the one march, and each rule's label at the playhead
comes from that. No combined rule: this view reports disagreement, it does not vote.

## Deliverables
- Each selectable escape rule evaluated on the one march, its label at the playhead available to the view. The
  extra per-sample storage or work is measured and stated in the PR.
- The agreement view, count mode: the number of rules that agree with the pixel's outcome, as a debug view (or a
  locked preset) beside the others.
- The agreement view, which-rule mode: for a pixel where exactly one rule disagrees, that rule; where several do, a
  "several" class, with the set reported in the pixel's readout.
- The mapping from each rule's classes to the outcome codes, the same one TASK-M3-34 states (Chazy's HE is
  "escape, body k", H is "triple ejection"); collision stays the collision radius, on or off, and is not one of the
  rules compared.

## Acceptance tests
- A fixture where every rule gives the same label (a clean hyperbolic escape, well past every rule's commit
  time): the count is the number of rules on every pixel. Control: one rule's label forced to another outcome gives
  count − 1 and names that rule.
- Burrau between the real ejection (t ≈ 60) and Closure's label (t ≈ 158): Closure is the rule that disagrees.
  The window where it does is measured and reported, not asserted to a value.
- One integration per sample: the march's step count with the view on equals the step count with it off, bit for
  bit on the stored record. Control: a view that re-integrates a sample fails it.
- The count and which-rule mode on the CPU and GPU agree on every pixel of the fixtures.

## Notes
- "Agree on its outcome" is read as agreeing with the outcome the pixel is coloured by (the default rule's label).
  If that proves the wrong reading, the largest agreeing group is the alternative; say which in the PR.
