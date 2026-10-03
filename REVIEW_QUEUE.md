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
  - `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the
    five kinds listed …", item 3: "Ask me only for: […] calibration values (batched at milestone gates)". § "R-71 — A
    missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*".
- **What:** PR #124 (TASK-M0-50, the WGSL lint's constant evaluator in `xtask/src/lint_wgsl.rs`) has every review
  approved or in progress and every other CI job green, but its `mutants` workflow fails: all 8 shards hit the
  120-minute limit.
  - Run 37092151020 on head 1ad796a: 357 of 714 mutants tested, 357 caught, 30 unviable, 0 missed, 0 timed out.
  - Every shard ran 2h04m–2h05m (started 03:07–03:11Z, cut off 05:12–05:16Z on 2026-10-03); `mutants-check` then
    reported "a mutants shard did not pass … cut off by its limit (R-302)".
  - Each shard's baseline takes ~15 min (89 s build + 806 s test), and each mutant ~2.3 min (xtask's test suite). At
    ~89 mutants per shard, a shard needs ~200 min for its mutants alone.
  - Earlier runs on the branch (37090902283, 37089718258, 37081962758, 37080898940) failed the same way as the
    evaluator grew.
  - Every other PR so far has fit inside n = 8 × 120 min.
  - Locally, cargo-mutants over the same diffs (restricted to the lint test targets) found 0 missed.
- **Options seen:**
  1. **Raise the per-shard limit, e.g. to 240 min (recommended):** a GitHub-hosted job allows up to 360. Only a PR with
     a large diff waits longer; n, and so the runners per Rust PR, stay as they are. It is the change with the fewest
     side effects. #124's shards (~15 + ~200 min) would fit, with ~25 min of headroom.
  2. Raise n, e.g. to 16. Every Rust PR then uses more runners, and each shard rebuilds and re-runs the ~15-min
     baseline.
  3. Keep n and the limit, and split #124 so that each part's diff fits. The evaluator is one file, and with roughly
     half the mutants each part would still sit near the limit.
  4. A one-off for #124: the human accepts the local full run as its evidence, and n and the limit stay.
- **Needed:** the human's choice. n and the per-shard limit are calibration values (R-71; R-369 item 3), the human's to
  confirm. REQ-VAL-149 is batched with the M0 gate's calibrations, but this one blocks a merge now. Only #124's merge
  waits on it; nothing else does.

---
