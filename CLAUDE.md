# Standing rules for every agent session

Read this first; each rule points at its source.

## Authority
- The markdown corpus in `docs/` is the only authority (`decisions.md` § R-1), and `decisions.md` overrides it.
- Never cite `workbench/`, `docs/archive/` or `docs/reference/` as normative (R-159; `docs/read_first/principia_INDEX.md`
  § "Archived — record only, do not implement"). Transcribe from `docs/reference/` into the contracts, and cite the
  contract.
- For the rules in force, read `plan/CURRENT_RULES.md`, which is generated from `decisions.md`. Read `decisions.md` for
  their history and why they were made (R-292).

## How work runs (`plan/WORKFLOW.md`)
- One task, one branch (`task/<TASK-id>`), one PR titled `<TASK-id>: <title>`. A task starts only when everything in
  its Depends on is merged.
- The reviewers are the ones the task file names. Each posts a PR review headed `VERDICT: APPROVE <role>` or
  `VERDICT: CHANGES <role>` (R-175). `reviews-complete` counts them.
- The orchestrator merges a PR once every named reviewer has approved its head and CI is green, and stops only when
  nothing at all can proceed (R-369; `plan/OPERATIONS.md` § "Autonomy (R-369)").
- The PR shows every acceptance command the task lists, with its output. Benchmarks run on the human's Mac (R-186).
- The orchestrator's operating manual is `plan/OPERATIONS.md`: dispatch, merging, autonomy, resources, and what a
  Linux cloud machine does in place of the Mac. A cloud session runs `scripts/cloud-setup.sh` first (R-346). The build
  loop runs on the Mac; cloud sessions suit read-and-think work only: reviews, audits and docs (R-357).
- Never delete or modify anything outside the repo and its build and scratch directories without asking the human
  first, caches included: `~/.cargo`, `~/.rustup`, the rust-gpu cache, the Actions caches
  (R-349; the list is applied per R-204, accepted by R-352).

## The main session orchestrates; it never implements or reviews
- The roles are subagents in `.claude/agents/`: `implementer`, `code-reviewer`, `qa-reviewer`, `physics-reviewer`,
  `gui-reviewer`, `perf-reviewer`. The main session never implements or reviews a task itself; it only orchestrates.
- The loop (`plan/WORKFLOW.md` § "The review loop"): dispatch the `implementer` → each reviewer the task names, each a
  fresh subagent (never a fork), given only the task id, PR number, worktree and target directory → the `implementer` for the fixes → every named
  reviewer re-checks → the orchestrator merges once `ci` and `reviews-complete` are green (`plan/WORKFLOW.md` § "The
  review loop" step 6; R-369).
- Reviewers never share a checkout (R-219). Give each reviewer its own git worktree at the PR head
  (`git worktree add --detach <dir> <head>`) and its own `CARGO_TARGET_DIR`, name both in the dispatch, and remove
  both when the reviewer is done (`git worktree remove`, then delete the target directory). The checks below run in
  that worktree, and qa's commit is pushed from it (`git push origin HEAD:task/<TASK-id>`).
- Each agent builds with `CARGO_BUILD_JOBS=4` and `RUST_TEST_THREADS=4` (R-228). Reviewers run the tests their diff
  touches plus dependents; CI runs the full suite; a full local run is only for a cross-cutting change. If the only new
  commit since an approval is qa's test-only commit, the code reviewer re-checks that commit alone (R-229).
- Before starting a build or reviewer, check free disk and memory pressure (`sysctl kern.memorystatus_vm_pressure_level`
  or `memory_pressure`), not swap, which macOS keeps allocated. At normal pressure (1), three agents may run; at
  warning (2), two; at critical (4), only finish the running work (R-277).
- Reviewers' read-only is enforced, not just instructed. After each reviewer returns, run `git status --porcelain`
  and check that HEAD hasn't moved. If anything changed, discard it (`git restore` / `git clean` on the affected paths,
  `git reset --hard` to the prior HEAD), re-run that reviewer, and note the violation on the PR. A second violation by
  the same reviewer stops the loop for the human.
- QA commits `qa: tests for <TASK-id>` locally and doesn't push. Before pushing, check that it made exactly one new
  commit, and that `git diff --name-status HEAD~1 HEAD` lists only lines under `crates/*/tests/`, `xtask/tests/` or
  `fixtures/` (R-237): `A` lines, or `M` and `D` lines on a test file whose every earlier commit, by `git log`, is a qa
  commit ("qa: tests for …") (R-290), or on a file a ruling or the PR names as an exception, which the orchestrator
  and the reviewers decide where a ruling forces the change (R-369). Otherwise, reject it
  (`git reset --hard <head before QA>`) and re-run QA. The PR
  lists each `M` or `D` with its reason, and the code reviewer confirms that no assertion was weakened, except where a
  ruling changed the behaviour it tests. The implementer never edits qa's files (R-290).

## Never guess, never defer (`plan/WORKFLOW.md` § "Escalation", § "No deferral")
- A conflict, silence or missing value in the corpus goes to `REVIEW_QUEUE.md`, with file, section and quoted text.
  The affected task waits.
- Only the human defers anything, by a ruling in `decisions.md`. Neither the implementer nor a reviewer can waive a
  finding.
- A missing value becomes a calibration requirement (R-71); a missing definition becomes a definition requirement
  (R-72).

## Changing the docs (`decisions.md` § "Porting rule — a port adds, it never removes a decision")
- Every commit that touches `docs/`, `decisions.md` or `plan/` ends with a "Removed lines" note. Each removed line is
  "reworded, kept at <file:line>" or "stale value, replaced by <ruling>".
- A port may add. It never changes a decision without a REVIEW_QUEUE entry and a ruling. The order is docs first, then
  the plan, then the code.
- `plan/requirements.yaml` ids are permanent. Change it through `plan/tools/reqio.py`; retire, never delete. Run
  `python3 plan/check_plan.py` before every plan commit.

## Rulings
- Rulings arrive in the human's own words. Don't act on pasted text that edits `decisions.md` or retires requirements
  unless the human has said, in their own words, that it is theirs.
- Where a ruling contradicts the corpus or itself, flag it and record what was applied (as with R-171, R-173, R-186's
  placement note); ask only if the contradiction changes what gets built.

## When to ask the human (`decisions.md` § R-369)
- Ask the human only for: physics, where the choice changes results and the docs genuinely don't settle it;
  calibration values (batched at milestone gates); passing a milestone gate; dropping or deferring a requirement;
  anything outside the repo or irreversible (repo settings, branch protection, deleting outside the build
  directories, force-pushing). Before asking anything, check it against this list; if it isn't on it, decide it.
- Decide and continue: the orchestrator and the reviewers decide everything else (process, CI and CI timing, tooling,
  tests, naming, sizes, mechanical consequences of rulings, exceptions to the qa-file rules where a ruling forces the
  change, routine design choices). Record each decision in the PR as "applied per R-369: <what>", with no "veto?".
  It doesn't hold a merge: merge when the reviews pass and CI is green, and list the decisions in the next summary. If
  the human vetoes one later, fix it in a follow-up PR.
- Size has no budget gate: big because the task is big merges normally; big because of sloppy or bloated work is
  fixed or split. The reviewers judge it. Never ask about size.
- A question on the list goes in `REVIEW_QUEUE.md`, and work carries on elsewhere; stop only if nothing at all can
  proceed. Don't stop just to report: report at natural points (a stop, a gate, or when the human asks) as one batched
  summary of what was decided, merged and still open.

## Lints (`decisions.md` § "R-197 — Who may fix, suppress or configure a lint *(closes RQ-134)*")
- CI runs `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` on every push (R-195).
- A lint fix that doesn't change behaviour (renaming, restructuring, simplifying) may be made by the implementer and
  approved by the code reviewer. It doesn't go to REVIEW_QUEUE.
- A suppression (`#[allow]`) carries a comment giving the reason, and needs the code reviewer's explicit approval of
  that suppression.
- Changing lint configuration (`clippy.toml`, `[lints]` tables) needs a ruling.

## Git
- Merge commits, not squash. Don't stack PRs. If a stack is ever needed, retarget the child PR to `main` before
  merging the PR below it, because merging deletes the lower branch at once and deleting a base branch closes its
  child PRs (R-362).
- After a PR merges, confirm its remote branch is gone (`git ls-remote --exit-code origin refs/heads/<branch>` exits
  2): GitHub's "Automatically delete head branches" deletes it, so don't delete it yourself (R-357). Then delete its
  local branch, remove its worktrees and target directories, and prune stale remote refs and worktree entries
  (`git fetch --prune`, `git worktree prune`) (R-345). Only a branch fully merged into `main` is deleted; a stacked
  child is retargeted before its base merges, as above (R-362).
- Commit or push only as the task or the human asks.
