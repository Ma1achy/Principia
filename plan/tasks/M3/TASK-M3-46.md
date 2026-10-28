# TASK-M3-46 — Li & Liao's free-fall orbits: closure, rest at T/2 and the alias pairs (LL-1, LL-2, the first batch's DRAFT-VAL-02), and the website-only rows (LL-5)

- **Milestone:** M3
- **Closes:** REQ-VAL-203, REQ-VAL-204, REQ-VAL-227
- **Depends on:** TASK-M3-42, TASK-M2-13
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~400 lines

## Goal
The replication suite's targets LL-1, LL-2, LL-5 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Each of Li & Liao 2019's 234 free-fall initial conditions (Tables A1–A5) must, under the default occupant at f64, have every velocity vanish at T/2 and return to rest in its initial configuration at T; and for each pair the atlas aliases (F1/F2, F12/F13, F23/F24 at masses (1, 1, 1); F11/F16 at (1, 0.8, 0.6)) the T/2 rest configuration, rescaled into region D, must equal the partner row with T·|E|^{3/2} equal. The 81 atlas free-fall rows on the three mass sets only the authors' website lists — (1, 0.1, 0.01), (0.393, 0.288, 0.662) and (1, 0.412, 0.267) — must close after one period as closure tests only, never cited as Li & Liao 2019's published results (rule 13) (LL-5). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M3 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.3 Free-fall periodic orbits: Li & Liao 2019"
- `docs/design/principia_dd_replication.md` § "6. The first batch"
- `docs/design/principia_dd_replication.md` § "4.1 The atlas's 311 free-fall orbits against Li & Liao's 234"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-631 — The high-precision initial-condition data is committed to the repo with full citations, no permission asked first: Hristov et al.'s databases, the free-fall, central-symmetry and Euler ones beside them, Simó's archive and Li & Liao's tables, each folder with a SOURCES.md; REQ-VAL-042, PL-6 and LL-5 point at the committed files, and fetch.sh stays as the provenance check *(amends R-630; closes RQ-579 with option (a))*"
- `docs/design/principia_dd_validation_orbits.md` § "1.1 Free-fall periodic orbits — the best fit for this project"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `fixtures/li_liao_2019/tables_a1_a5.csv`, Tables A1–A5 transcribed twice from the paper, the two copies and their diff kept beside it, each row with its page.
- `crates/validation/src/gates/free_fall.rs`: closure at T and rest at T/2 per row, the alias pairs' T/2 states rescaled into region D, and the 81 website-only atlas rows as closure tests only.
- The calibration proposal for the T/2 and T tolerances.

## Acceptance tests
- `cargo xtask gate replication --target LL-1` — reference data: Tables A1–A5 transcribed twice into a CSV (rule 2) from docs/incoming/2026-10-24_anosova/papers/li_liao_2018_arXiv_1805.07980.pdf, not the atlas's rows; unequal sets on the MassSimplex chart; residuals at T/2 and T per row against REQ-VAL-204; control: the velocities at T/4 must be non-zero, so the T/2 test reads a minimum and not a constant; each alias pair's T·|E|^{3/2} from our run, beside the paper's (REQ-VAL-203).
- `cargo xtask gate replication --target LL-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-204).
- `cargo xtask gate replication --target LL-5` — the atlas's ref_* for those rows, by OC-1's method and within OC-1's calibrated tolerance (REQ-VAL-202); the rows are named by mass set and atlas key; no figure attributes them to the paper; the authors' 316-orbit list is read from its committed copy, `docs/incoming/2026-10-27_hp_ics_freefall/data/liliao_github/free-fall-3b-pictures.md` (R-631), and the SJTU website pages stay recorded by URL and sha256 only (R-630 item 8); control: one row integrated to 0.9·T must fail (REQ-VAL-227).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-204.
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- Part of the first batch (`principia_dd_replication.md` § 6), ported from PLAN.md § 6's drafts by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
