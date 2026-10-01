# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-192: four choices applied per R-204 that no ruling has settled: two older ones, and R-354's two *(R-298, TASK-M0-23, R-354)*

- **File, section:**
  - `decisions.md` § "R-298 — TASK-M0-17's items 12 and 15 accepted; a trace with no summary line is valid *(amends
    R-286)*": "*Applied per R-204 — veto?:* R-298 is in the "design" group … The type shape and the rest are in PR
    #79."
  - `plan/tasks/M0/TASK-M0-23.md` § "Deliverables": "`.github/workflows/mutants.yml` (applied per R-204, R-305 —
    veto?: its own workflow, …)".
  - `decisions.md` § "R-354 — The seven open veto items stand, R-252 stays amended, not superseded; settled "veto?"
    marks name their ruling, and open ones must be in the review queue": its two "Applied per R-204 — veto?" items.
- **What:** these were applied without asking (R-204) and are open for a veto:
  1. R-298's mark, PR #79's items a, c, d and e: `Trace` keeps `leak_flags` and `hot_paths` as options and adds a
     `session` field (a); a header line alone is an incomplete session with no frames (c); the writer writes an
     incomplete trace without a summary line and refuses one with a summary set (d); R-298 is in the "design" group
     (e). R-282 accepted #79's numbered items, R-299 ruled item b and R-304 items f–i; no ruling names a, c, d or e.
     - **Mark:** `decisions.md` — "A header line alone counts as an incomplete session with no frames"
  2. TASK-M0-23's deliverable, PR #65's item 16: the per-PR mutation gate is its own workflow, `mutants.yml`, which
     only `pull_request` runs, so a `push` run never reports a skipped `mutants-check`. R-305 accepted #65's items
     10–13; no ruling names items 14–16 (14 and 15 are in #65's description only).
     - **Mark:** `plan/tasks/M0/TASK-M0-23.md` — "its own workflow, which only `pull_request` runs"
  3. R-354: item 6 of the seven is read as R-297's whole "Applied per R-204" block (its design, GUI and amendment
     bullets with the placements), though the list named only the placements.
     - **Mark:** `decisions.md` — "R-297's mark heads one block, and item 6 is read as all of it"
  4. R-354: the check's design: open marks are named by open REVIEW_QUEUE.md entries ("**Mark:**" lines), not kept in
     a list of their own; the scan covers decisions.md, CLAUDE.md, `plan/` and `docs/` (less `docs/archive/` and
     `docs/reference/`), not PR descriptions; the negative controls are `plan/tools/veto_marks.py`'s built-in cases,
     run by every plan check, not a new `xtask/tests/plan_check.rs` case.
     - **Mark:** `decisions.md` — "the open marks live in REVIEW_QUEUE.md, where R-346 already keeps"
- **Options seen:**
  1. **Accept items 1–4 (recommended);** veto any by number, saying what replaces it.
  2. For item 4: keep the open marks in a list of their own (`plan/open_veto_marks.yaml`) rather than in the queue.
- **Needed:** accept or veto. Nothing waits on it: each is merged or applied as written. When it is ruled, each mark
  names the ruling in place of "veto?", in the same commit that moves this entry to the archive.

---
