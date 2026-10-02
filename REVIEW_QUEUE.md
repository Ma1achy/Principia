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

## RQ-199: `qa_TASK-M0-29.rs` pins the unsharded steps R-366's change to `support/qa_m0_01.rs` replaced, and neither qa nor the implementer may change it *(R-366, R-290)*

- **File, section:**
  - `decisions.md` § "R-366 — qa updates TASK-M0-45's pinned CI step lines; nextest shards by `hash:<k>/4`; #117's
    other items stand *(closes RQ-197; amends R-290, R-336 and R-360 as they apply)*", the human's text: "A named R-290
    exception: in TASK-M0-45's qa commit, qa changes only the CI step matches in qa_TASK-M0-22_r235.rs and
    support/qa_m0_01.rs to the exact sharded forms; the code reviewer confirms nothing else changed."
  - The same, *Applied:* "in TASK-M0-45's qa commit, qa changes only the CI step matches in
    `xtask/tests/qa_TASK-M0-22_r235.rs` (`check_controls_job_beside_the_tests`'s two single-job matches) and
    `xtask/tests/support/qa_m0_01.rs` (`check_the_ci_workflow`'s two commands) to the exact sharded forms,
    `cargo nextest run --workspace --partition hash:${{ matrix.shard }}/4` and
    `cargo xtask ci --partition ${{ matrix.shard }}/4`, and, as option 1 has it, moves those checks' negative controls'
    edit targets so they match again. Nothing else in the two files changes, and the code reviewer confirms it."
  - `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends
    R-237)*": "qa may modify or delete test files that only qa has ever committed to (checked with git log). […] The
    implementer still never edits qa's files."
  - `xtask/tests/qa_TASK-M0-29.rs` on `origin/task/TASK-M0-45`, lines 521–552
    (`qa_m0_29_ci_workflow_check_accepts_the_per_push_steps` and its negative control), whose synthetic workflows
    call `check_the_ci_workflow` with the unsharded steps:
    - line 529: `\x20       run: cargo nextest run --workspace\n\`
    - line 533: `\x20       run: cargo xtask ci\n",`
    - line 547: `\x20       run: cargo nextest run --workspace\n\`
    - line 551: `\x20       run: cargo xtask ci\n",`
- **What:** qa's commit 0f917a1 on `task/TASK-M0-45` applied R-366's exception: `check_the_ci_workflow` in
  `xtask/tests/support/qa_m0_01.rs` now looks for `run: cargo nextest run --workspace --partition hash:${{ matrix.shard
  }}/4` and `run: cargo xtask ci --partition ${{ matrix.shard }}/4`, exact matches. `qa_TASK-M0-29.rs` calls that
  check on its own synthetic workflows, which still name `run: cargo nextest run --workspace` and `run: cargo xtask ci`,
  so `qa_m0_29_ci_workflow_check_accepts_the_per_push_steps` now fails ("ci.yml has no step `run: cargo nextest run
  --workspace --partition …`"), and its negative control, which expects the `cargo xtask deps` message, gets the
  nextest one first and fails too (qa's review 5392627743 on PR #117). The file has an implementer commit, 9405734
  (TASK-M0-33), which changed lines 529 and 547 from `cargo test --workspace` to `cargo nextest run --workspace`; its
  other commit is qa's 49aba44. So R-290 doesn't let qa change it, R-366's exception names only
  `qa_TASK-M0-22_r235.rs` and `support/qa_m0_01.rs`, and the implementer never edits qa's files (R-290). Nobody may make
  the fix.
- **Options seen:**
  1. **Extend R-366's exception to those four strings in `qa_TASK-M0-29.rs` only (recommended).** They take the same
     exact sharded forms, `cargo nextest run --workspace --partition hash:${{ matrix.shard }}/4` and
     `cargo xtask ci --partition ${{ matrix.shard }}/4`, inside a synthetic matrix job if the check needs one (it matches
     `run:` lines only, so it should not). Nothing else in the file changes, and the code reviewer confirms it. qa
     applies it in a follow-up qa commit on TASK-M0-45, and the orchestrator's R-237 check accepts `M` on that file in
     that commit.
  2. `check_the_ci_workflow` accepts either form, unsharded or sharded. That changes `support/qa_m0_01.rs` again,
     beyond R-366's "exact sharded forms", and lets the check pass a `ci.yml` that no longer shards.
  3. Leave it to the implementer. That needs an exception to R-290's "The implementer still never edits qa's files".
- **Needed:** which option. PR #117 (TASK-M0-45) waits on it.

---
