# TASK-M9-25 — Hierarchical stability limits, prograde and retrograde (V-11)

- **Milestone:** M9
- **Closes:** REQ-VAL-330, REQ-VAL-331
- **Depends on:** TASK-M3-34
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~250 lines

## Goal
The replication suite's targets V-11 run as engine tests (`principia_dd_replication.md`; R-594, R-629). The hierarchical stability limits, prograde and retrograde, must be reproduced from their primaries as distributions (V-11). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"
- `decisions.md` § "R-638 — RQ-598's work starts now: GG-1's routes 2 (Fig. 1 read from its vector paths) and 3 (recomputed with Brutus), and a BR-1 Brutus ensemble on Breen's stated set-up from a fixed recorded seed, while the owner emails the authors; Fig. 1 must reconcile to exactly 1212 points a panel, and the cost per point is reported before any Brutus run is scaled or goes below ε = 10⁻⁶; every self-computed result is labelled ours *(refines R-637)*"
- `decisions.md` § "R-640 — Self-computed Brutus ensembles ("Brutus, run here") unblock TASK-M3-62 and TASK-M3-65 now, provisionally: recorded "vs Brutus, run here", never as replicating the paper, closing neither BR-1 nor GG-1, and re-scored against the authors' data when it arrives, theirs governing and each disagreement queued; the Fig. 1 extraction is the paper's own values and scores at its stated precision *(amends R-637; answers RQ-614)*"

## Deliverables
- The primaries' tables transcribed twice once in the repo (RQ-598).
- `crates/validation/src/gates/hierarchical.rs`; the calibration proposal.

## Acceptance tests
- `cargo xtask gate replication --target V-11` — the primaries once in the repo (rule 1; RQ-598); their tables transcribed twice; the limits at REQ-VAL-331's level; control: the prograde limit applied to retrograde runs must fail (REQ-VAL-330).
- `cargo xtask gate replication --target V-11 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-331).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-331.
- Waits on RQ-598: the task starts only once it is ruled, and builds no chart or diagnostic ahead of the ruling (R-594 item 5).
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *28 Oct 2026 (R-637, R-638, R-640):* RQ-598 is closed and the wait on it is lifted. V-11 proceeds from Part I (Mylläri et al. 2018), read from the University of Turku repository's version; its publisher PDF is held locally and never committed. Any table is transcribed twice and diffed (R-594 item 4); if what V-11 needs differs between the versions, stop and queue it. Valtonen & Karttunen 2006 § 10.6 (p. 271) is secondary, cited by page. Part II's definitions are transcribed in `docs/incoming/2026-10-28_rq598_data/transcriptions/v11_partII/`; its closed form is not V-11's comparison number. On 29 Oct 2026 the Turku file could not be fetched by script (`docs/incoming/2026-10-28_rq598_data/README.md` § 6).
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M3-65 (milestone M3). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each binds is § "12.2 Tiers"'s: (b); (a) statistically or qualitatively. Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M3 gate". Old ids in this file now read as their M9 ids: TASK-M3-62 → TASK-M9-22, TASK-M3-65 → TASK-M9-25. A wait on a ruling or data below holds this task's scoring, never a milestone.
