# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-173: TASK-M5-30 allows only `QuadReduction` to be read back per frame; R-288 reads two counters back each frame *(plan, TASK-M5-30, R-288)*

- **File, section:** `plan/tasks/M5/TASK-M5-30.md` § "Acceptance tests", the checklist line for REQ-SYS-033 (also
  REQ-SYS-033's verify detail in `plan/requirements.yaml`): "audit every buffer map/readback: only QuadReduction is read
  back per frame; other readbacks are behind a user action and async". REQ-SYS-033's statement: "QuadReduction must be
  the sole automatic GPU-to-CPU crossing". `decisions.md` § "R-288": "Two per-frame atomic u32 counters … in telemetry
  §2. They ride on the existing profiler/telemetry readback, not a new GPU→CPU channel (QuadReduction stays the sole
  automatic return of simulation data, R-142), and are read back asynchronously with a frame or two of latency, never
  stalling the frame."
- **What:** R-288's counters come back every frame, on the profiler/telemetry readback, with no user action. Read
  literally, TASK-M5-30's checklist line fails on them. R-288's own words arguably settle it: they aren't a new
  GPU→CPU channel, and `QuadReduction` stays the sole automatic return of *simulation data*. REQ-SYS-033's statement
  and checklist don't draw that line. The same task's Goal ("the only automatic GPU → CPU crossing is the
  `QuadReduction` readback") and REQ-SYS-036's line ("finds only the reduction readback and the two sanctioned pulls")
  read the same way.
- **Options seen:**
  1. **Reword REQ-SYS-033 to "simulation data" (recommended).** `QuadReduction` is the only automatic per-frame return
     of simulation data; the profiler/telemetry readback (R-288) is not simulation data. REQ-SYS-033's statement and
     verify detail change through `plan/tools/reqio.py`, and TASK-M5-30's checklist line follows.
  2. **Leave it as written.** TASK-M5-30 reconciles it when it builds the membrane audit, citing R-288.
- **Needed:** which one. Nothing is blocked now: TASK-M5-30 is an M5 task.
- **See also:** R-294 and RQ-174 (archived, ruled by R-294) concern the same per-frame readback of R-288's counters.
  R-294 bears on the options but doesn't choose between them. It keeps the counters automatic and per frame (an atomic
  u32 buffer bound and reset each frame), so TASK-M5-30's checklist line, read literally, still fails on them. It also
  puts their readback "with the telemetry readback", not on `QuadReduction` or a channel of its own, which is the line
  option 1 draws between simulation data and telemetry. TASK-M5-28 now builds that readback (R-294), and TASK-M5-30
  doesn't reach TASK-M5-28 through its Depends on, so under option 2 its membrane audit may run before the counters'
  readback exists.

## RQ-176: the per-PR mutation gate takes over two hours on a quarter of the merged PRs' diffs; is a per-PR limit practical? *(calibration, CI, TASK-M0-23, R-196, REQ-VAL-149)*

- **File, section:** `decisions.md` § "R-196 — Mutation testing joins the QA gate": "per PR, `cargo mutants --in-diff`
  on the changed code (with a per-PR time limit)" and "If the time limit makes per-PR runs impractical, raise it in
  REVIEW_QUEUE rather than dropping it." `plan/requirements.yaml` REQ-VAL-149, verify detail: "if no practical limit
  covers per-PR runs, a REVIEW_QUEUE entry instead (R-196)". `plan/tasks/M0/TASK-M0-23.md` § "Goal": "If no practical
  limit covers per-PR runs, that goes to REVIEW_QUEUE rather than the run being dropped (R-196)."
- **What:** R-272's measurement ran the job's own command (`cargo mutants --in-place --in-diff`, PR #65's
  `.cargo/mutants.toml`, cold target, `ubuntu-latest`) over 13 merged PRs' diffs (run 36724963688; the table is in PR
  #65's "Calibration proposal: REQ-VAL-149"). Ten finished: two had no mutant in the diff, seven took 4 to 74 minutes,
  and PR #72's took 133 minutes (40 mutants). PR #70's was cut off by a runner shutdown after 38 minutes, 2 of its 69
  mutants in; at the 4.4 minutes its first mutant took, all 69 would take about 5 hours, near GitHub's 6-hour job
  maximum. PR #71's and PR #47's were still running after 3h03m (16:57Z), past 180 minutes. The time goes on tests, not
  builds: the unmutated baseline's tests take 3.5 to 11 minutes in seven of the nine diffs with mutants, each mutant
  re-runs its package's tests, and one timed-out mutant costs five baselines (PR #75: one timeout, 53 of its 74
  minutes). PR #65 proposes 180 minutes, provisional (R-71, R-182): the longest completed run plus 30%. Under it, PR
  #70's, #71's and #47's diffs would fail the gate on time alone. Is that practical as a per-PR gate? The implementer
  does not decide that (R-196).
- **Options seen:**
  1. **Keep 180 minutes, per PR.** A PR whose gate runs over fails; it is split, or its mutants killed faster.
  2. **Make the gate faster, then re-measure.** Shard the mutants over a job matrix (`cargo mutants --shard k/n`), run
     the tests through cargo-nextest (`--test-tool nextest`), or lower the timeout multiplier. None is measured yet;
     each is a later measure/ run (R-272) and a CI change.
  3. **Change where the gate runs.** For example per PR only on the changed packages' fast tests, with the full
     in-diff run nightly. A scope change to R-196; the human's alone.
- **Needed:** which one, and whether 180 minutes stands meanwhile. TASK-M0-23 (PR #65) carries 180 provisionally so
  its CI runs; REQ-VAL-149 is confirmed at the M0 gate either way.
