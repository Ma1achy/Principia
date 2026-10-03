# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-202: the per-PR mutants shard limit cuts off #124's run: n and the per-shard limit, a calibration *(REQ-VAL-149)*

- **File, section:**
  - `plan/requirements.yaml`, REQ-VAL-149 (`kind: calibration`): "The shard count n and the per-shard time limit of the
    sharded `cargo mutants --in-diff` job (R-196, R-302) must be calibrated: proposed with their evidence by the task
    that needs them, checked by a reviewer, confirmed by the human at the M0 gate and recorded in decisions.md."
  - `.github/workflows/mutants.yml`, the `mutants` job, both values marked provisional:
    - lines 45–47:
      ```
              # REQ-VAL-149: n = 8 shards, k = 0..7 (cargo-mutants counts shards from 0), provisional until the human
              # confirms it at the M0 gate (R-71, R-182, R-302). A change to n changes this list and `--shard k/8` below.
              shard: [0, 1, 2, 3, 4, 5, 6, 7]
      ```
    - lines 146–148:
      ```
              # REQ-VAL-149: 120 minutes per shard, provisional until the human confirms it at the M0 gate (R-71, R-182,
              # R-302).
              timeout-minutes: 120
      ```
    - line 153: `--shard ${{ matrix.shard }}/8 --sharding round-robin -o "$RUNNER_TEMP"`.
  - `decisions.md` § "R-305 — #65's provisional mutation values and items 10–13 stand; `mutants-check` becomes a
    required check on `main`", the human's words: "Provisional values stand: n = 8 shards, 120 min per shard, confirmed
    or replaced at the M0 gate."
  - `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the
    five kinds listed …", item 3: "Ask me only for: […] calibration values (batched at milestone gates)". § "R-71 — A
    missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*".
- **What:** PR #124 (TASK-M0-50, the WGSL lint's constant evaluator in `xtask/src/lint_wgsl.rs`) has every review
  approved or in progress, and on head 1ad796a every other CI job was green, but its `mutants` workflow failed: all 8
  shards hit the 120-minute limit.
  - Run 37092151020 on head 1ad796a: 387 of 714 mutants tested: 357 caught, 30 unviable, 0 missed, 0 timed out
    (the eight shards' `outcomes.json`; 89 or 90 mutants per shard).
  - Every shard ran 2h04m–2h07m (started 03:07–03:11Z, cut off 05:12–05:17Z on 2026-10-03); `mutants-check` then
    reported "a mutants shard did not pass … cut off by its limit (R-302)".
  - Each shard's baseline took 14.8–17.4 min (74–95 s build, 800–969 s test), and each tested mutant 117–138 s on the
    shard's mean (127 s, ~2.1 min, over all 387; xtask's test suite). At 89–90 mutants per shard, a shard's step needs
    about 188–224 min in all.
  - Runs 37090902283 (df6fd7c), 37089718258 (e9238b3) and 37080898940 (e924dda) were cut off at the limit the same
    way, on all 8 shards, as the evaluator grew. Runs 37081962758 (6d93115) and 37102126099 (fc5489f) were not: each
    ran on a qa commit whose tests fail until the fix that follows, so every shard's baseline failed ("cargo test
    failed in an unmutated tree, so no mutants were tested") after 14–22 min.
  - In those runs the per-mutant timeout cargo-mutants set from each shard's baseline was 1600–1938 s (shard 0, job
    111114569255: "Auto-set test timeout to 1600s" after an "88s build + 799s test" baseline; shard 7, job
    111114569194: "1938s" after "74s build + 968s test"). A hung mutant runs to that timeout, 27–32 min, in place of
    about 2 min, so each hang adds about 30 min to its shard.
  - #124's head has since moved to fd669b3 and then e2eb25b, each adding to the diff. `cargo mutants --list --in-diff`
    (which lists without building) over `git diff origin/main...<head>` gives 714 mutants at 1ad796a, 718 at fd669b3
    and **725 at e2eb25b**, 90 or 91 per shard (`--shard k/8 --sharding round-robin`). At the slowest shard's 138 s per
    mutant and 17.4-min baseline, the largest shard now needs about 227 min (about 224 at 1ad796a). Run 37104626084
    (fd669b3) started 06:55Z and was still running at 08:32Z; run 37107242606 (81c0171, a qa commit) failed every
    shard's baseline, as above, after 15–19 min; no `mutants` run on e2eb25b had started by 08:32Z.
  - Every other PR so far has fit inside n = 8 × 120 min. Over all `mutants.yml` runs, the longest *finished* `cargo
    mutants` step outside #124 is 52.0 min (#107, TASK-M0-13: run 36878731915, job 110424763538), and the longest
    successful PR run's longest step is 48.8 min (#120, TASK-M0-19: run 37085778406). The jobs on #120 that ran 63–66
    min (job 110954728563 in run 37042181185, 63.0 min; 110975680519 in 37048479624, 66.0 min; 110988078207 in
    37052200411, 64.0 min) did not finish their `cargo mutants` step: GitHub annotates each "The hosted runner lost
    communication with the server". A fourth #120 job, 111026322293 (run 37063744524), failed after 24 min with "No
    space left on device".
  - No local run covers CI's 714, 718 or 725. #124's body records only local `cargo mutants --in-diff` runs over one
    round's increment, restricted to the lint's test targets: before review, one on 0b4f9cf that left 25 survivors,
    each then covered by a test; `<git diff df6fd7c 1ad796a>`, 40 mutants (33 caught, 7 unviable, 0 missed); and
    `<git diff fc5489f fd669b3>`, 48 mutants (42 caught, 6 unviable, 0 missed).
- **Options seen:**
  1. **Keep n = 8 and raise the per-shard limit to 300 min (recommended):** #124's largest shard (about 227 min) then
     fits with two hung mutants (about 257 min with one, 287 with two). With the 7 min or so of setup each shard spends
     before the step (Mesa 5.0 min and build-kernel 1.2 min in job 110975680519), plus the check and the upload, the
     job stays under GitHub's 360-min maximum for a hosted job. Only a PR with a large diff waits longer; n, and so the
     runners per Rust PR, stay as they are, and it is the change with the fewest side effects. Every other PR so far
     ends in ≤ 52 min, so for them the limit is only a backstop.
  2. Keep n = 8 and raise the limit to 240 min. #124's largest shard fits with no hang, with about 13 min of headroom;
     one hung mutant (about 257 min) takes it past 240, and the run is cut off again.
  3. Raise n, e.g. to 16. Every Rust PR then uses more runners, and each shard rebuilds and re-runs the 15–17-min
     baseline. At 120 min it still doesn't fit: 725 mutants give 45 or 46 per shard, 46 × 138 s + 17.4 min ≈ 123 min.
  4. Keep n and the limit, and split #124 so that each part's diff fits. The evaluator is one file, and with roughly
     half the mutants each part would still sit near the limit.
  5. A one-off for #124: a full local `cargo mutants --in-diff` over #124's whole diff, which does not exist yet, run on
     the Mac, which has no 120-min limit, and the human accepts it as #124's evidence; n and the limit stay. At CI's
     ~2.1 min per mutant, run one at a time, the 725 mutants would take about 25 hours; the Mac may be faster.
- **Needed:** the human's choice. Option 1 (n = 8, 300 min) is recommended; option 2 (240 min) stays open, with the
  risk it states. n and the per-shard limit are calibration values (R-71; R-369 item 3), the human's to confirm, and
  R-305 fixes them until the M0 gate: "Provisional values stand: n = 8 shards, 120 min per shard,
  confirmed or replaced at the M0 gate." So the orchestrator can't change them, or waive the run for #124, without a
  ruling. REQ-VAL-149 is batched with the M0 gate's calibrations, but this one blocks a merge now. Only #124's merge
  waits on it; nothing else does.

---
