# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-198: R-366's number, applied per R-204: the human's text numbered it R-365, which was already taken *(R-366, R-365)*

- **File, section:**
  - `decisions.md` § "R-366 — qa updates TASK-M0-45's pinned CI step lines; nextest shards by `hash:<k>/4`; #117's
    other items stand *(closes RQ-197; amends R-290, R-336 and R-360 as they apply)*": its numbering note, "The human's
    text says R-365, which was already taken by the RQ-196 ruling; recorded as R-366".
  - `decisions.md` § "R-365 — RQ-196's A1 and A2 stand as built: each `xtask-ci` shard builds the kernel before its
    controls; `h = 0` gives 0 *(closes RQ-196)*", on `main` since PR #113.
- **What:**
  - **A. Applied without asking (R-204), open for a veto:**
    1. The human's ruling of 2 Oct 2026 on RQ-197 reads "RQ-197: option 1 (R-365)". R-365 already closes RQ-196, so
       the ruling is recorded as R-366, the next free number, as R-348 was when the human's message named R-347. PR
       #117 (TASK-M0-45) cites it as R-366.
       - **Mark:** `decisions.md` — "recorded as R-366, applied per R-204"
- **Options seen:**
  1. **Accept the number R-366 (recommended).** Ruling numbers are permanent, and R-365 is already cited by its own
     entry, RQ-196's archive and the plan.
  2. Veto, naming the number to use instead.
- **Needed:** accept or veto. Nothing waits on it: TASK-M0-45 builds R-366 as written. When ruled, the mark names the
  ruling in place of "veto?".

---
