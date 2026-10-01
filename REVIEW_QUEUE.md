# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---


## RQ-187: Whether `open-questions.md`'s audit section D, "deliberately parked", holds still-open items under R-334 *(R-334, open-questions.md, REQ-PERF-028, REQ-INT-033)*

- **File, section:**
  - `decisions.md` § "R-334 — check_plan.py proves every still-open item in `open-questions.md` maps to a requirement or
    a REVIEW_QUEUE entry": "each still-open item in `open-questions.md` names the requirement or the open REVIEW_QUEUE
    entry that carries it."
  - `open-questions.md` § "Audit section D — deliberately parked": "**Audit section D — deliberately parked** (audit,
    logged in step 5; each is parked with its reason in its owning file): `00_philosophy.md` §7 (post-1.0);
    reversibility replay and KS regularisation (integrator contract Part 6); cross-chart payload sharing (caching
    contract Part 3); quantised checkpoints (moot under lockstep); batch literature import (a validation-phase tool);
    tier-table numbers (guesses by design, calibrated from telemetry)."
  - `open-questions.md`, its first line: "Everything left open, each with its source."
  - `docs/read_first/principia_00_philosophy.md` § "7.7 The rule for this section": "A parked idea earns its place by
    naming what it buys and why it waits. If either is missing it is not parked, it is undecided".
- **What:** R-334 names the items it wants mapped, and section D is not among them. Section D sits in a file of things
  "left open", but each of its items is parked with a reason, which philosophy §7.7 sets apart from undecided. So it
  isn't clear whether they are still-open items that `plan/check_plan.py` must see mapped. Two of them have a
  requirement: the tier-table numbers (REQ-PERF-028, which cites this section) and KS regularisation (REQ-INT-033: "with
  Kustaanheimo–Stiefel regularisation deferred to v2"). The others (philosophy §7, reversibility replay, cross-chart
  payload sharing, quantised checkpoints, batch literature import) have none found.
- **Options seen:**
  1. **Parked is not open (recommended).** Section D's items are decisions to wait, each with its reason in its owning
     file; the section is marked parked, with no *Carried by* note, and the check does not see it.
  2. **Parked is open.** Each item names what carries it: REQ-PERF-028 and REQ-INT-033 where they exist, and a new
     requirement, or a retirement note, for each of the rest.
- **Needed:** which one. Until then section D names this entry, so the check passes.
