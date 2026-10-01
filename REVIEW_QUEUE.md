# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---


## RQ-186: R-326's conditional cache save puts a step-level `if:` in the `xtask-ci` job, which `qa_TASK-M0-22_r235.rs` rejects, and R-290 lets neither qa nor the implementer change that file *(code, qa, TASK-M0-14, PR #96, R-326, R-290, REQ-VAL-166)*

- **File, section:**
  - `decisions.md` § "R-326 — Actions caches are saved only on pushes to `main`; pull-request jobs restore only": "every
    cache step in every workflow restores in every run, and saves only in a run triggered by a push to `main`."
  - `decisions.md` § "R-290 — qa may change test files that only qa has committed to": "qa may modify or delete test
    files that only qa has ever committed to (checked with git log). … The implementer still never edits qa's files."
  - `xtask/tests/qa_TASK-M0-22_r235.rs`, `check_controls_job_beside_the_tests` (line 115):
    `!has_key(controls_lines, "if:") && !has_key(controls_lines, "continue-on-error:")`, message "the controls job can be
    skipped or pass with a finding". `has_key` matches a line at any indent in the job, so a step's `if:` counts as the
    job's. The file's history: 422b742 "qa: tests for TASK-M0-22", then implementer commits 9405734 (TASK-M0-33) and
    3b84aa5 (TASK-M0-38). So R-290 does not let qa change it.
  - `plan/requirements.yaml` REQ-VAL-166 asks only that CI run every control "in a job of its own, in parallel with the
    tests, failing on any finding (R-235)". The test's own header reads the `if:` as the job's: "the controls job is
    neither skipped (`if:`) nor allowed to fail".
- **What:** the `xtask-ci` job builds the kernel inside `cargo xtask ci` and caches `~/.cache/rust-gpu` (R-320,
  REQ-SYS-075). Under R-326, PR #96 saves that cache with an `actions/cache/save` step under
  `if: github.event_name == 'push' && github.ref == 'refs/heads/main'`. That is a step-level `if:`: it does not let the
  job be skipped. All four tests that call the check fail on it:
  - `qa_r235_ci_runs_every_control_in_a_job_beside_the_tests`
  - `qa_r235_ci_does_not_run_the_listing_only_form`
  - `qa_r235_controls_job_does_not_wait_for_the_tests`
  - `qa_r235_controls_job_has_the_tests_gpu_backend`

  So does the last one's control. No form of R-326's save avoids an `if:`: `actions/cache/save` has no condition
  input, and `Swatinem/rust-cache`'s `save-if` covers only its own cargo cache.
- **Options seen:**
  1. **qa may narrow that one check to the job's own `if:` (indent 4) (recommended).** This is an exception to R-290
     for this file, on the ground R-290 already names: a ruling, R-326, changed the behaviour it tests. The code
     reviewer confirms that nothing else in the file changes.
  2. **`xtask-ci` saves no rust-gpu cache.** It restores the one `gpu-kernel` saves (`rust-gpu-gpu-kernel-…`), so the job
     has no `if:`. REQ-SYS-075's "under a key naming its job" would then read as the saving job's key for a restore-only
     step, as `xtask-ci`'s restore of `ci`'s fixture pool already does (R-285). R-326's "the rust-gpu cache keeps its own
     key" would read the same way.
  3. **`xtask-ci` restores its own key and never saves it.** No `if:`, but the job's kernel build is then always cold:
     about 4 min more per run (#96 measured cold `build-kernel` steps of 3 min 37 s to 4 min 12 s), on a job that is
     already over R-325's ~10.5 min.
- **Needed:** which one. This blocks PR #96's merge, since the `ci` job's tests fail on these four until it is settled.
  More generally, R-290 does not say who may change a qa file that also has implementer commits when a ruling changes
  the behaviour it tests.
