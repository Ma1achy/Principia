# Operations: the orchestrator's manual

What a new orchestrator session needs to run the build, beyond what `CLAUDE.md`, `plan/WORKFLOW.md` and
`plan/CURRENT_RULES.md` already say. It holds the rules and lessons that lived only in the orchestrator's local memory
and its session notes until 1 Oct 2026, so that a fresh session, on the human's Mac or on a Linux cloud machine, needs
nothing from outside the repository (R-346).

How to read it:
- A rule with a ruling cites it (R-n), and `decisions.md` holds its words; where this file and a ruling disagree, the
  ruling wins. A practice with no ruling of its own carries the date it was set, and stands under R-346.
- **Mac only** marks what holds only on the human's Mac (its paths, APFS clones, `memory_pressure`, Metal, perf runs).
  **Linux cloud** says what a cloud machine does instead.
- The Linux items R-346 applied per R-204 (RQ-190) are ruled by R-347, which accepted them, and cite it.

## Where the build loop runs (R-357)

The build loop runs on the human's Mac: implementers, qa's tests, builds, benchmarks and merges. **Cloud sessions
aren't viable for the build loop; they suit read-and-think work: reviews, audits and docs.** A cloud session failed
its environment checks on 2 Oct 2026 on three blockers. What each would need is applied per R-204, accepted by R-363:
1. **Disk.** The machine had ~30 GB, and one warm build reached 24 GB. It would need a larger disk (≥ 100 GB, for
   three agents' targets and the seed within § "Resources"' thresholds), or builds trimmed to fit.
2. **GitHub GraphQL is blocked,** so the `gh pr` commands, which use GraphQL, fail; REST works. It would need GraphQL
   allowed through the proxy. Until then every agent posts its review through REST (§ "Reviewers"), and a cloud
   session's orchestrator uses REST for the rest:

   | `gh pr` command | REST through `gh api` |
   |---|---|
   | `gh pr list` | `gh api repos/Ma1achy/Principia/pulls` (open PRs) |
   | `gh pr create` | `gh api repos/Ma1achy/Principia/pulls -f title=<title> -f head=<branch> -f base=main -F body=@<file>` (POST) |
   | `gh pr view` | `gh api repos/Ma1achy/Principia/pulls/N`; `--jq .head.sha` for the head, `--jq .mergeable_state` (`clean` is `mergeStateStatus` `CLEAN`) |
   | `gh pr checks` | `gh api repos/Ma1achy/Principia/commits/<sha>/check-runs` |
   | `gh pr diff` | `git diff origin/main...<head>`, or `gh api repos/Ma1achy/Principia/pulls/N -H "Accept: application/vnd.github.diff"` |
   | `gh pr edit --base main` | `gh api -X PATCH repos/Ma1achy/Principia/pulls/N -f base=main` |
   | `gh pr edit --body-file` | `gh api -X PATCH repos/Ma1achy/Principia/pulls/N -F body=@<file>` |
   | `gh pr reopen` | `gh api -X PATCH repos/Ma1achy/Principia/pulls/N -f state=open` |
   | `gh pr comment` | `gh api repos/Ma1achy/Principia/issues/N/comments -F body=@<file>` |
   | `gh pr review --comment` | `gh api repos/Ma1achy/Principia/pulls/N/reviews --method POST --input - < <file>`, where `<file>` holds the JSON object `{"event":"COMMENT","commit_id":"<sha>","body":"<body>"}`, written first by its own command (R-373) |
   | `gh pr merge --merge --match-head-commit` | `gh api -X PUT repos/Ma1achy/Principia/pulls/N/merge -f merge_method=merge -f sha=<full head sha>` |

   Resolving a review thread (R-276) has no REST call; it waits for GraphQL.
3. **Branch deletion is blocked by the proxy.** It would need the proxy to allow ref deletes. GitHub's "Automatically
   delete head branches" removes merged PR branches server-side (§ "Merging"); any other delete, such as a `measure/`
   branch's (R-272), still needs it.

The **Linux cloud** notes below, `scripts/cloud-setup.sh` among them, stand for a cloud session's read-and-think work.
*29 Oct 2026:* Not used: no cloud sessions (R-645, R-647, R-648).

### The weekly consistency audit (R-636)

Adopted by R-636 (closing RQ-605): **report-only, never blocking, run on demand when the owner asks (for example
before a release or a milestone gate), with no scheduled task or job** (the owner, 28 Oct 2026). No merge, task or
milestone waits on it, and no CI workflow runs it.
- **What it reads:** `plan/` (`requirements.yaml`, `tasks.yaml`, `plan/tasks/`, `MILESTONES.md`, `WORKFLOW.md`,
  `OPERATIONS.md`, `reviewers/`), `docs/design/`, `docs/contracts/` and `decisions.md`. Not `docs/incoming/`,
  `docs/archive/`, `docs/reference/` or `workbench/`, which are records, not authority (R-1, R-159).
- **What it checks:** every cross-reference resolves (a file and § heading, an R-, RQ-, REQ- or TASK- id, a contract
  Part); contradictions between documents (two statements of one rule, value or definition that disagree, and a
  ruling's text against the doc lines it says it was applied in); requirements with no task, and tasks with no
  requirement. `python3 plan/check_plan.py` already checks part of this; the audit adds the contradictions and the
  references `check_plan.py` doesn't scan.
- **What it reports:** to the owner, each time it runs, as one PR holding the mechanical fixes (a stale reference, a
  heading renamed) and a `REVIEW_QUEUE.md` entry for each contradiction, with file, section and quoted text. The PR
  merges as any PR does, on its reviews and green CI; it blocks nothing else.
- **Who runs it:** there is no schedule. When the owner asks, the orchestrator runs it as a local agent, and names
  the reviewers of its PR then (R-369).

## Start here

1. **A cloud session runs `scripts/cloud-setup.sh` first**, from the repository root. It installs exactly what CI's
   Linux jobs install: the Rust toolchains (the `dtolnay/rust-toolchain` steps' and the root `rust-toolchain.toml`'s,
   with their components), the apt packages (Mesa's lavapipe), cargo-nextest and cargo-mutants, and Python with its
   packages (PyYAML). It reads every version from the files CI reads, never from a copy of its own, then exports
   `PRIN_GPU_BACKEND` as CI's Linux jobs set it (`vulkan`) and runs `python3 plan/check_plan.py` as a smoke test.
   `scripts/cloud-setup.sh --dry-run` prints what it would install, one item per line. Running it again is safe: it
   skips what is already installed. Then do what it prints, so the next shell keeps the setup: put `$HOME/.cargo/bin`
   on PATH and export `PRIN_GPU_BACKEND`.
   - CI's `xtask/tests/cloud_setup.rs` fails if the script and CI's Linux jobs disagree on any item or version, if CI
     installs anything by a means the script doesn't know, or if the script holds a version literal (R-346); and if
     cargo-nextest and cargo-mutants don't take their prebuilt route, with `cargo install --locked` only after a failed
     download (R-347).
   - When a workflow gains a new kind of install step, the script refuses to run, naming the step. Teach both the
     script and the test the new step in the same PR.
   - How it installs (R-346, R-347): the toolchains through rustup, the apt packages through apt-get, the Python
     packages through pip. cargo-nextest comes from its official prebuilt installer (`get.nexte.st`) and cargo-mutants
     through cargo-binstall (prebuilt; cargo-binstall itself from its official installer when missing), each at CI's
     pinned version; each falls back to `cargo install --locked <tool>@<version>` only if its download fails. The dry
     run prints each item's method as its fourth word.
   - **The cloud machine needs network access** to `get.nexte.st` and to GitHub's releases (cargo-binstall and the
     prebuilt tools it fetches), besides crates.io, `sh.rustup.rs` and the apt and PyPI mirrors. Without it, the two
     tools fall back to building from source, which still needs crates.io.
   - The script warns, and does not fail, when the machine's `python3` is another minor version than the one CI sets
     up (R-347): `plan/check_plan.py` and the xtask tools need only Python 3 with PyYAML.
2. **Read** `CLAUDE.md`, `plan/WORKFLOW.md`, `plan/CURRENT_RULES.md`, this file and `REVIEW_QUEUE.md` (everything open).
   Then `gh pr list --repo Ma1achy/Principia` for the PRs in flight (in a cloud session, its REST form, § "Where the
   build loop runs"). The loop needs `gh`, signed in to GitHub
   (`gh auth status`); CI's runner images have it, so the setup script doesn't install it.
3. **Agent definitions** (`.claude/agents/`) load only when a session starts. After one changes, the session must be
   restarted (`claude --continue`); `/clear` doesn't reload them (26 Sep 2026).
4. **The repository** is `Ma1achy/Principia`, public (renamed from `prin-impl` on 25 Sep 2026). On the Mac the
   checkout is `~/src/Principia`. `main` carries the tag `plan-v1` (a59a772), the plan the build started from: never
   move it; a plan change that needs a tag gets a new one.
5. **What is ready.** A task is ready when every task in its Depends on is done. A task is done when its
   `task/<TASK-id>` PR has merged, or its `plan/tasks.yaml` entry says `status: done`. TASK-M0-00 is done that way
   (R-185, no task PR), and is never listed as ready (29 Sep 2026).

## Roles and the loop

The roles, the loop and the read-only check are in `CLAUDE.md` § "The main session orchestrates; it never implements
or reviews" and `plan/WORKFLOW.md` § "The review loop". In addition:
- **Run independent work in parallel** (the human, 27 Sep 2026): every ready task starts at once, each in its own
  worktree and target directory, within the agent cap (§ "Resources"). Never start a task whose dependencies aren't
  merged, and never stack a task on an unmerged PR (28 Sep 2026; R-362). The one exception is R-388's early start: a
  task whose other dependencies have all merged may start off its one unmerged dependency's head, once every reviewer
  that dependency's task names has approved that head and its CI is green there apart from mutants (applied per R-369:
  never with two unmerged dependencies, which would be stacking, R-362). It opens no PR until the dependency merges,
  then merges `main` in and opens its PR against `main`; if the dependency changes beyond added tests first, re-check
  the task before its PR opens (`plan/WORKFLOW.md` § "The unit: one task, one branch, one PR").
- **Every approval sits on the head.** Before a merge, each named reviewer's `VERDICT: APPROVE` is on the latest
  commit, or carried over to it under R-260 (§ "Reviewers").
- **A compile check beats a token scan.** Don't enforce a source rule by reading tokens when the compiler can check
  it: R-187's token scan took six rounds of bypasses before R-191 replaced it with a compile check (PR #16).
- **The PR description** gives the commit hashes, a summary, and each REVIEW_QUEUE entry or ruling applied. When qa's
  commit is pushed, update the description, which the implementer wrote before it: its counts and "untouched" claims go
  stale, and it lists each `M` or `D` line of qa's with its reason (R-290).

## Dispatching

- **Rulings travel only in an opening prompt (R-289).** A ruling reaches an agent only in the opening prompt of a
  fresh dispatch, in the human's own words. If one lands while an agent is mid-task, let it finish its current step and
  stop, then dispatch a fresh agent with the ruling. A message to a running agent carries only coordination facts (a
  path, a merge order, "main moved") or a review finding. Relaying R-286 and R-288 by message on PR #79 got the agent
  blocked by the permission classifier, even for `git status`. If an agent reports a classifier block, don't retry that
  path: tell the human.
- **Pre-flight ready-next tasks (R-388).** While tasks are in flight, have a read-only agent pre-flight each
  ready-next task, one whose every dependency is merged or in review: against its task file and References, that each
  cited section exists and that no value, definition or decision it needs is missing or in conflict. The agent writes
  nothing; run the read-only check on it as on a reviewer (§ "Reviewers"). File each gap in `REVIEW_QUEUE.md` then,
  with file, section and quoted text, rather than when the implementer reaches it.
- **What every dispatch names:**
  - the task id, and the PR number for a reviewer or a fix round;
  - the agent's worktree and its `CARGO_TARGET_DIR` (§ "Paths and warm builds");
  - for an implementer, the target directory of its local mutants run, `<target>-mutants`, and that it deletes it
    straight after (§ "Reviewers", its **Mutants** item; R-388); for a task that may touch `crates/gui/`, the
    `--build-timeout` of its gui run and the hard limits that run takes (R-439);
  - for a task started early off a dependency's approved head, that head's sha, and that the PR waits for the
    dependency to merge (R-388; § "Roles and the loop");
  - the environment: `CARGO_INCREMENTAL=0`, `RUSTUP_AUTO_INSTALL=0` (R-414; § "Toolchain"), and on the Mac the PATH
    fix (§ "Paths and warm builds"), with `/opt/homebrew/opt/rustup/bin` first on the PATH exported before `job
    submit`, since a job runs with its submitter's PATH; no job counts, which the job queue sets (R-658);
  - that every heavy command goes through the job queue, at the agent's priority, with these two lines (R-658;
    § "Resources", "The job queue"):
    `"/Volumes/X10 Pro/Principia/orch/jobq/bin/xtask" job submit <worktree> --priority <p> --label <TASK-id> --env
    CARGO_TARGET_DIR=<target> -- <command>` prints a job id; `"/Volumes/X10 Pro/Principia/orch/jobq/bin/xtask" job
    wait <id>` blocks, prints the log path and exits with the job's status (re-run it, or run it in the background,
    if it would outlast the agent's call);
  - a private scratch subdirectory for PR bodies and temp files, `<scratchpad>/<pr>-<role>/`. Agents sharing one
    scratch directory overwrote each other's `body.md`, and PR #54's description briefly showed TASK-M0-09's
    (30 Sep 2026);
  - any ruling since the task file was written, verbatim (R-289);
  - for a reviewer, the request to judge the PR's size and to name each decision it made (§ "Reviewers", R-369);
  - for a renamed or reused target, what to clean first (§ "Pitfalls").
- **No cloud sessions, and no briefs or instructions addressed to the owner (R-647, a standing rule; R-645, R-648).**
  The orchestrator dispatches every agent locally itself, and never asks the owner to start, paste into, restart or
  check a session. Written instructions for an agent are internal task files in the orchestrator's notes
  (`orch/tasks/`), not briefs to the owner.
- **Fresh or resumed.** A reviewer is always a fresh subagent, never a fork (CLAUDE.md). A fix round with a ruling in
  it is a fresh dispatch (R-289). A fix round without one may go to the implementer that is still running, by message.
- **Sizing.** Have the implementer measure a few sample controls before sizing a task: per-control estimates ran ~30%
  low, since a `negative_control!` call is ~10 lines once formatted, and controls in a new `tests/*.rs` target copy
  qa's helpers (TASK-M0-24 and TASK-M0-25, RQ-143, RQ-144).

## Reviewers

- **Their own checkout (R-219).** Each reviewer gets its own detached worktree at the PR head
  (`git worktree add --detach <dir> <head>`) and its own `CARGO_TARGET_DIR`, both named in the dispatch, and both
  removed when it is done. After it returns, run the read-only check (`git status --porcelain`, HEAD unmoved;
  CLAUDE.md).
- **Handing a worktree on.** Never `git worktree move` a worktree, and never rename a target directory between
  worktrees or roles: compiled test binaries keep the absolute paths of `CARGO_MANIFEST_DIR`, `CARGO_TARGET_TMPDIR` and
  `CARGO_BIN_EXE_*`, cargo doesn't rebuild them after a move, and the controls then fail falsely (PR #79 and PR #83,
  30 Sep 2026). Hand the next reviewer the same worktree path, with the new head checked out there, or make a fresh
  worktree.
- **Who re-checks what** (R-260, as `xtask/src/reviews_check.rs` checks it):
  - after qa's add-only commit (`qa: tests for <TASK-id>`, only `A` lines under qa's paths), every named reviewer's
    approval carries over to it, and the code reviewer re-checks that commit alone (R-229);
  - after any other commit, a qa commit with any `M` or `D` line or any commit that is not qa's, no approval carries
    over: every reviewer the task names (qa, code, physics, gui, perf) posts again on the new head (`plan/WORKFLOW.md`
    steps 5 and 6). After an `M` or `D` qa commit, dispatch all of them at once, qa's re-approval included, not one
    after another (#78, #79, 30 Sep 2026).
- **How they post (R-357, R-373).** Each reviewer posts its verdict through REST, never `gh pr review`, so it works on
  the Mac and in a cloud session alike, in one fixed form, path first and flags after:
  `gh api repos/Ma1achy/Principia/pulls/N/reviews --method POST --input -`. The human's allow rule,
  `Bash(gh api repos/Ma1achy/Principia/pulls/*/reviews*)`, matches that form only; the permission check blocked the
  old `-f event=COMMENT -f commit_id=<sha> -F body=@-` form as an external-system write (R-373). Standard input
  carries the JSON object `{"event":"COMMENT","commit_id":"<head sha>","body":"<review body>"}`, the body headed
  `VERDICT: APPROVE <role>` or `VERDICT: CHANGES <role>` (R-175). It takes two steps, each its own command (the agent
  files give both in full): first a heredoc feeds the body to `python3 -c '…json.dumps(…)…' <head sha> <file>`, which
  writes the object to `<file>`, `review.json` in the reviewer's scratch folder (`<scratchpad>/<pr>-<role>/`, § "Dispatching",
  or `$TMPDIR/<pr>-<role>/` if the dispatch names none), outside its worktree; then
  `gh api repos/Ma1achy/Principia/pulls/N/reviews --method POST --input - < <file>` posts it, with nothing before
  `gh`. The allow rule matches a command by how it begins, so a post that began with `python3 … |` matched no rule and
  the permission check judged each one itself; it blocked one on #123, and the two-step form posted there (review
  5396798807). A reviewer uses no other form; if the post is blocked, it stops and reports it. `event` `COMMENT` is
  what `gh pr review --comment` posted. `commit_id` attaches
  the review to the head reviewed, which `reviews-check` compares with the PR head (R-260); qa passes the head it was
  given, before its own unpushed commit. A reviewer reads the diff with `git diff origin/main...HEAD` in its worktree,
  after `git fetch origin`, and checks CI with `gh api repos/Ma1achy/Principia/commits/<head sha>/check-runs`. The
  agent files say so (`.claude/agents/`).
- **Size and decisions (R-369).** Each reviewer judges the PR's size: big because the task is big merges normally; big
  because the work is sloppy or bloated is a finding, fixed or split. The implementer and the reviewers decide what
  isn't on R-369's list of questions for the human (§ "Asking the human"), and each decision goes in the PR
  description, "applied per R-369: <what>", not only in a reply: qa may not read the implementer's replies (#72,
  30 Sep 2026). Ask for both in the first dispatch. Reviewers no longer class "veto?" items: R-234's classes went with
  it (R-369).
- **Mutants.** The implementer runs `cargo mutants --in-diff` on its own diff against `origin/main` before it requests
  review (opens the PR), and again before it pushes fixes for survivors (R-388). It tests the boundary of every
  comparison and match arm it adds. Reviewers don't run it, and CI's mutants gate stays the authority (`mutants.yml`,
  R-302): a clean local run doesn't replace a green `mutants-check`. The local run gets a target directory of its own
  (`<target>-mutants`), since a mutants run can leave a mutated build that cargo treats as fresh, deleted straight
  after. It runs through the job queue (R-658), at the implementer's priority (`build`, or `fix` in a fix round), on
  the head the implementer is about to push, never on work in progress. A local run uses R-348's two caps, the
  per-mutant timeout and the memory cap on test processes, at the values CI uses (REQ-VAL-180, REQ-VAL-181), where the
  machine enforces them (applied per R-204, accepted by R-352): on macOS a local run gets the timeout only, and CI's
  Linux runners enforce both caps (R-352). The `mutants::skip` marker doesn't compile without the `mutants` crate as a
  dependency (E0433) and skips a whole function; an equivalent mutant gets a test, a behaviour-preserving rewrite
  (R-197), or a justified entry in `.cargo/mutants-equivalent.toml` (R-202).
  - **A local run that mutates the gui crate uses hard limits (R-439, amending R-388).** A local gui-crate run crashed
    the Mac twice (two kernel watchdog panics, 15 Oct 2026): a mutant made a gui test process allocate about 10 GB,
    with 14 to 20 copies under nextest on an 18 GB machine. So where the diff touches `crates/gui/`, the gui crate's
    mutants run on their own, as `cargo mutants --in-diff <diff> --package gui -j 1 --timeout 90 --build-timeout
    <generous>`, with nextest at one test thread (`--cargo-test-arg=--test-threads=1`), `RUST_TEST_THREADS=1` (no effect under nextest; `--test-threads 1` is the binding limit) and a
    narrow nextest filter (`--cargo-test-arg=-E --cargo-test-arg='<filterset>'`). Applied per R-369: the filterset
    selects the gui tests that exercise the diff's files, and the PR names it; a test the filter leaves out can only
    leave a local survivor, which CI's run, the gate, still sees. The `--build-timeout`, in seconds, is the
    dispatch's, at least the gui crate's cold build. The 90 s per mutant is the ruling's, in place of CI's value for
    this run only; CI's caps (REQ-VAL-180, REQ-VAL-181) are unchanged. The rest of the diff, every other crate, runs
    as above, unchanged, in a run of its own that leaves gui out (`--exclude 'crates/gui/**'`). The orchestrator's
    memory guard (§ "Resources") runs throughout.

## qa commits

qa commits `qa: tests for <TASK-id>` locally and doesn't push. Before pushing it, from qa's worktree:
1. There is exactly one new commit, titled `qa: tests for <TASK-id>`.
2. `git diff --name-status HEAD~1 HEAD` lists only paths under `crates/*/tests/`, `xtask/tests/` or `fixtures/`
   (R-237).
3. Each line is `A`, or `M` or `D` on a file whose every earlier commit, by `git log --format=%s -- <file>`, is a qa
   commit (R-290), or on a file a ruling names as an exception: R-335 (`xtask/tests/qa_TASK-M0-22_r235.rs`), R-336
   (TASK-M0-45's test splits), R-342 (the two `qa_TASK-M0-38.rs` files), R-366 (TASK-M0-45's CI step matches in
   `xtask/tests/qa_TASK-M0-22_r235.rs` and `xtask/tests/support/qa_m0_01.rs`) and R-371 (the four unsharded step strings
   in `xtask/tests/qa_TASK-M0-29.rs`), or one the PR records under R-369 (below).
4. Push with `git push origin HEAD:task/<TASK-id>` and confirm with `git ls-remote`. Add each `M` or `D` line to the
   PR description, with its reason.

Anything else: reject it (`git reset --hard <head before qa>`) and dispatch qa again. The implementer never edits qa's
files (R-290). The one-round exceptions on #74 and #78 stand. When a ruling forces a change to a file the rules above
don't open to qa, the orchestrator and the reviewers decide the exception and record it in the PR, naming the file and
what may change: qa changes only that, and the code reviewer confirms nothing else changed. It is not a question for
the human, and it is not filed in REVIEW_QUEUE (R-369).

## Merging

Who merges: the orchestrator, once § "Autonomy (R-369)"'s conditions hold, whether the human is away or not. To
merge:
1. Wait until `gh pr view N --json mergeStateStatus` shows `CLEAN`. Every review thread is resolved (R-276).
2. Check the merged tree: in a scratch detached worktree at `origin/main`, `git merge` the PR branch, and any other PR
   about to merge, then run `python3 plan/check_plan.py` and `python3 plan/tools/current_rules.py --check`, and the PR's
   own new checks if `main` has moved since its CI ran. "Require branches to be up to date" is off (R-266), so nothing
   else catches a clash: #70's CI was green, but `main` had since gained #75's `spawn::TIMEOUT` uses, which #70 renamed,
   and the merged tree failed to compile (30 Sep 2026).
3. `gh pr merge N --merge --match-head-commit <full sha>`. Never `--admin` (R-266), never squash or rebase.
4. Then, as a separate command, never chained after the merge with `;`: confirm the remote branch is gone, since
   GitHub's "Automatically delete head branches" deletes it on merge (R-357): `git ls-remote --exit-code origin
   refs/heads/<branch>` exits 2, finding no such ref. Don't delete it yourself; if it is still there, tell the human.
   Then delete the local branch, remove the PR's worktrees and their target directories, and prune
   (`git fetch --prune`, `git worktree prune`) (R-345). A refused merge with chained cleanup once removed #54's
   worktree.
5. If `reviews-complete` stays red only from the `pull_request`-event run, which fails before any review and stays a
   separate check suite, re-run it: `gh run rerun <id>` (R-266, R-276).
6. To re-run CI, prefer re-running the failed jobs (`gh run rerun <id> --failed`) over closing and reopening the PR
   (the owner's note, R-633).

**Merge order for stacked PRs.** Don't stack (R-362, § "Roles and the loop"). Deleting a base branch closes the PR
stacked on it rather than retargeting it (PR #2, 25 Sep 2026), and with auto-delete on (R-357), merging a PR deletes
its branch at once. So if a stack is ever needed, per PR n: `gh pr edit n+1 --base main` first, then merge n
(CLAUDE.md § Git, R-345, R-362). To recover a closed child: push its branch back at the merged PR's `headRefOid`,
`gh pr reopen`, then `gh pr edit --base main`.

**Required checks.** Branch protection is the human's (`plan/HUMAN_SETUP.md` §2); the orchestrator never changes it,
except for adding `mutants-check`, the one change R-305 allows. The human removed `gpu-kernel` from the required
checks until #96 (TASK-M0-14) merges, since PRs off `main` never report it. Tell the human the moment #96 merges, so
they can add it back.

**Numbers.**
- **RQ ids.** Before filing an RQ, take the next free id across every `origin/*` branch: `git fetch`, then search each
  branch's `REVIEW_QUEUE.md` and `docs/archive/review_queue/`. Two agents once both filed RQ-188.
- **An RQ open only on a PR branch.** When a rulings PR rules on it, the rulings PR archives it in
  `docs/archive/review_queue/`, since `plan/tools/rulings.py` rejects a cited RQ that is neither open nor archived. The
  task PR's next fix pass deletes its open copy.
- **Ruling numbers.** The human may number a ruling, and their number wins: renumber any provisional one. If the human
  numbers a batch from a number already taken, record it from the next free number, in order, and note the shift under
  the first (R-278).

## Autonomy (R-369)

The human's standing rule of 2 Oct 2026 (R-369), which replaced the size rules and the "veto? holds the merge"
practice, in force whether the human is away or not. Keep working through the plan under `plan/WORKFLOW.md` and
`CLAUDE.md`. Progress is the goal: decide and continue, and stop only when nothing at all can proceed.

**Merge** a PR once all of these hold:
- every reviewer the task names has posted `VERDICT: APPROVE` on the head;
- CI is green on the head, every job;
- `python3 plan/check_plan.py` passes, and so does § "Merging"'s check of the merged tree;
- it waits on no open `REVIEW_QUEUE.md` entry.

Its size and its decisions don't hold it: the reviewers judge size (§ "Reviewers", § "Size"), and each decision made
without asking is in the PR and goes in the next summary. If the human vetoes one later, fix it in a follow-up PR.
A PR that waits on a question stays open, with the reason written down; go on to the next ready task.

**Order of work** (28 Sep 2026): run every ready task at once, within the agent cap. Priority went to the ledger chain,
TASK-M0-07 to TASK-M0-15; R-336 makes TASK-M0-45 the next M0 task to start, at high priority.
R-355 and R-356 (2 Oct 2026) named the eight tasks, TASK-M0-15, TASK-M0-45 to TASK-M0-50 and TASK-M0-51, the reader
task. R-357 (2 Oct 2026): they run on the Mac, not in a cloud session, and M0 finishes in this order:
1. TASK-M0-45 and TASK-M0-49 first (both make CI cheaper);
2. then TASK-M0-15, TASK-M0-46, TASK-M0-47, TASK-M0-48, TASK-M0-50 and TASK-M0-51, in parallel within the CPU, memory
   and disk limits (§ "Resources"), and TASK-M0-52, the `ci` shards' shared test build, once TASK-M0-45 merges
   (R-372, applied per R-369);
3. then TASK-M0-19, with its benchmarks run on the Mac (R-186), and TASK-M0-44;
4. stop before the M0 gate, and lay out its six calibrations together (REQ-VAL-138, REQ-VAL-149, REQ-VAL-151,
   REQ-VAL-156, REQ-VAL-180 and REQ-VAL-181), each with its measurements and proposed value, so the human can confirm
   them in one sitting.

R-376 (3 Oct 2026) confirmed the six: REQ-VAL-138 0 steps, a per-backend reference near a tie; REQ-VAL-149 8 shards ×
300 min, a ceiling, not a target; REQ-VAL-151 256 cases; REQ-VAL-156 600 s; REQ-VAL-180 2.0× the baseline, 60 s floor;
REQ-VAL-181 8 GiB, `--core=1`. R-377 (3 Oct 2026): the M0 gate is passed once #124 (TASK-M0-50) merges and every
requirement in M0's exit gate is green on `main`; then confirm it in the summary, tag that commit of `main` `m0-gate`,
and start M1.
R-386 (6 Oct 2026, applied per R-369): before the M1 gate passes, lay out REQ-COL-061's proposal beside REQ-COL-055's
and check `cvd_hatch_distinct` (TASK-M7-20) green on `main`. The M1 gate report lists only the gate blocks of M1 and
earlier milestones, so it does not show REQ-COL-061, which is in M7.
R-428 to R-432 (14 Oct 2026): the M1 gate passes. R-428 confirms the nine register entries provisional until M1 (the
six log floors, `rho_ratio`'s and `r_min_pair_0`'s provisionally until M2, and the hatch's colours and stripe width),
R-429 REQ-COL-061's criterion, R-430 the ramps as reported, and R-431 four R-369 choices; R-432 passes the gate. Once
the rulings PR has merged, tag that commit of `main` `m1-gate`, confirm it in the summary, and start M2. RQ-279 (six
M1 calibrations the message does not name) does not hold the gate.
R-433 to R-435 (15 Oct 2026): RQ-279 is closed and the M1 gate's other calibrations are confirmed: R-433 `r_pole`
(a register entry provisional until M3), the running grey, the triple swatches, the banding bound and the ‖n‖ = 1
tolerance (a register entry); R-434 the stipple, its dots changed to `#909090` and its goldens re-rendered; R-435 the
131 baselines. They ride the same rulings PR, so `m1-gate` is tagged on its merge as R-432 has it.
R-390 (6 Oct 2026): the GUI track, TASK-M6-24 to TASK-M6-29, starts now, in parallel with the physics and renderer
chain, on spare agent slots only, and never holds up M1 or M2 (§ "The GUI track (R-390)").
R-415 (10 Oct 2026): **merging before an earlier milestone's gate.** A task may merge before an earlier milestone's
gate has passed if its dependencies have merged and it relies on no value or behaviour that gate calibrates or could
change (`plan/WORKFLOW.md` § "Human checkpoints: the milestone gates"). Apply the test before merging, and record it
in the PR as "applied per R-415": the gates it passes over, the calibration and definition requirements of those
gates checked (`plan/MILESTONES.md`), and why the task relies on none of them; when in doubt, don't merge early.
TASK-M7-02 to TASK-M7-04, merged before the M1 gate (#145, #148, #150), get an after-the-fact check against the test,
a separate read-only dispatch whose findings go in the next summary (and any reliance found, in `REVIEW_QUEUE.md`).
TASK-M7-05 merges early only if it passes the same test, and only once R-411's port (RQ-265) has merged.

Merge under the conditions above, and file every question for the human in `REVIEW_QUEUE.md` (R-369).

**Never, without the human** (R-369's list; R-349):
- make or record a new ruling (applying an existing one is fine);
- confirm a calibration value;
- pass a milestone gate: stop before it;
- drop or defer a requirement;
- change branch protection or any repository setting;
- force-push or edit merged history. That includes amending and force-pushing your own fresh branch, even before a PR
  exists: fix a mistake with a new commit (30 Sep 2026);
- delete or modify anything outside the repository and its build and scratch directories (§ "Resources").

A physics choice that changes results, where the docs genuinely don't settle it, is the human's too. When one of these
comes up, file it in `REVIEW_QUEUE.md` and carry on with other work.

**Reporting.** Don't stop just to report. At a natural point (a stop, a gate, or when the human asks) give one batched
summary:
- what was decided: each "applied per R-369" decision, with its PR, for the human to veto afterwards;
- what merged: PR, task and head;
- what is still open, and why;
- every question in `REVIEW_QUEUE.md`, in one list;
- anything surprising;
- free disk and the memory-pressure level at each checkpoint (R-252, R-295).

## The GUI track (R-390)

The human's ruling of 6 Oct 2026 (R-390): a dev GUI the human runs on the Mac with `cargo run -p gui --features mock`,
built on a mock engine, in six tasks, TASK-M6-24 to TASK-M6-29, one per ORDER item, each its own PR, reviewed by code,
qa and gui. The human's feedback added TASK-M6-30 (R-406) and TASK-M6-31 (R-409), and later TASK-M6-32 to TASK-M6-36
(R-450 to R-453; TASK-M6-35 also physics), TASK-M6-37 (R-451 B11, also physics), TASK-M6-38 to TASK-M6-44
(R-457 to R-463; TASK-M6-44 also perf) and TASK-M6-45 (R-466; also physics, for its definition only), track tasks like
the six.
- **Priority and resources.** The physics and renderer chain keeps priority for agent slots. A GUI-track task, its
  reviewers and its fix rounds take only a slot that no ready chain task needs, within § "Resources"' memory and disk
  limits. When slots are short, the chain's ready task gets the next free one; a running GUI-track agent is not
  stopped, but the next slot doesn't go to the track. The track never holds up M1 or M2: no M1 or M2 task depends on
  it, and none waits for a slot it holds.
- **A track task the chain waits on takes the chain's priority** (applied per R-369, review 5434769963 on PR #157).
  TASK-M6-21 depends on TASK-M6-26, TASK-M6-22 on TASK-M6-24 and TASK-M6-28, TASK-M7-22 on TASK-M6-29 and TASK-M7-25 on
  TASK-M6-28. A track task that one of these chain tasks waits on, directly or through the track tasks before it, takes
  the chain's priority for agent slots, with its reviewers and fix rounds, once that chain task's other dependencies
  are merged or in flight. So the chain never waits on a track task that never gets a slot. No M1 or M2 task depends on
  the track, so this never takes a slot from M1 or M2.
- **When it merges.** Each track task merges once its reviews pass and CI is green, without waiting for the gates of M1
  to M5 (`plan/WORKFLOW.md` § "Human checkpoints: the milestone gates"). Each depends only on merged work and on the
  track's task before it.
- **"What to try".** Each track PR's description has a "What to try" section: the clicks and keys that show what
  changed, on `cargo run -p gui --features mock` (`plan/WORKFLOW.md` § "The unit: one task, one branch, one PR").
- **Sources and silences.** `decisions.md`, then `docs/gui/design/GUI_DESIGN_NOTES.md`, then the artboards in
  `docs/gui/design/`, then `docs/gui/principia_render_gui_spec.md`, then `docs/contracts/principia_gui_state_contract.md`;
  the higher wins, and R-68 still makes corpus values win over artboard values. Where they're silent on look, feel or
  behaviour, the orchestrator and the reviewers decide and record it as "applied per R-369"; calibrations and physics
  choices stay as R-71 and R-369 give them.
- **The human's feedback.** The human tries each merged screen and sends look-and-feel notes as plain text. Each note is
  a GUI design ruling: record it in `decisions.md` at the next free number, in the human's words, in the "design" group
  of `plan/rule_groups.yaml`, as any ruling is recorded. Apply it in the porting rule's order: the docs first
  (`docs/gui/design/GUI_DESIGN_NOTES.md`, or render_gui_spec where the note settles a spec point), then the plan (the
  task it changes, or a new task), then the code, through a task PR reviewed like any other. A note that contradicts
  a ruling or the corpus is flagged in the PR and the next summary (CLAUDE.md § "Rulings").
- **Look choices hold (R-416, 10 Oct 2026).** TASK-M6-26's look choices (PR #183's § "Look choices") and TASK-M6-31's,
  when it lands, are neither confirmed nor changed until the human has tried the M6-31 mock. Don't put them to the
  human before then; list them in TASK-M6-31's PR for that try.
- **Contract changes.** A track task that adds to the contract surface re-runs the conformance suite on both engines,
  and the code reviewer checks the field is as the corpus names it (R-390's "Contract fields").
- **M8's GUI track (R-627, 27 Oct 2026).** TASK-M8-05, TASK-M8-06, TASK-M8-07, TASK-M8-13, TASK-M8-19, TASK-M8-20, TASK-M8-23 and TASK-M8-24 are track tasks too, built on the mock engine with every rule
  above; their dependence on TASK-M6-21 and TASK-M6-22 is satisfied by the mock's precision events and quality tier.
  **TASK-M8-05, the Explore shell, goes first** among them: it takes the first track slot M8 gets. Each has a small
  wiring task, TASK-M8-88 to TASK-M8-95 in the same order, which depends on it and on TASK-M6-21 and TASK-M6-22, wires
  it to the real engine and re-runs its acceptance there with the conformance suite on both engines; a wiring task is
  engine-side work, dispatched when its dependencies are merged, with no track priority of its own. TASK-M8-86, the v1
  web skeleton (the egui app and the mock engine in wasm on WebGPU), and TASK-M8-96, its measurements against RQ-550
  and RQ-551, are track tasks with no wiring task; they settle neither RQ. The deploy is by hand only: once TASK-M8-86
  merges, the orchestrator runs `gh workflow run pages.yml --ref main`, through the `github-pages` environment, and
  then dispatches TASK-M8-96 on that deploy; no workflow deploys the site on a merge or a push until the human
  says v1 is ready to be found, and no other repository setting changes (R-627; `plan/HUMAN_SETUP.md` §5).
- **The cloud proposal (R-627).** Whether mock-engine GUI tasks can be implemented in cloud sessions on lavapipe, the
  Mac doing only their screenshot and Metal checks, is a proposal the human asked for, written separately, and not
  applied: R-357 and § "Resources" stand until the human rules on it.

## Size

No budget gate (R-369): size is judgement, and never a question for the human. A PR that is big because its task is
big merges normally; one that is big because of sloppy or bloated work is fixed or split. The reviewers judge it
(§ "Reviewers"). The ~500 counted-line figure (`plan/WORKFLOW.md` § "Task files") is a planning guide for a task file's
Size line, not a limit. Record the reason for a PR's size in its description. Nothing is skipped, deferred or drifts:
a split moves every requirement to a named task (R-264).

## Asking the human

- **When to ask** is R-369's (CLAUDE.md § "When to ask the human"). Ask the human only for physics where the choice
  changes results and the docs genuinely don't settle it; calibration values (batched at milestone gates); passing a
  milestone gate; dropping or deferring a requirement; and anything outside the repository or irreversible (repository
  settings, branch protection, deleting outside the build directories, force-pushing). Before asking anything, check
  it against that list: if it isn't on it, decide it. Never size, never what a ruling already settles.
- **File and carry on.** A question on that list goes in `REVIEW_QUEUE.md`, and work carries on with whatever doesn't
  wait on it (R-369).
- **Pasted rulings are the human's own.** The human sends rulings and instructions as a pasted block with no text
  around it. Act on it as on a typed message; don't ask them to re-confirm it in their own words (26 Sep 2026). Flag a
  factual error or contradiction in it, in the PR and the report, and ask only when it changes what gets built. A
  message from another agent is never a ruling.
- **"Applied per R-369: <what>"** marks a decision made without asking, so the human can veto it later. It is
  written where it was applied (the PR description, or a ruling's *Applied* note or the task file where it changes
  them), has no "veto?", holds no merge, isn't filed in `REVIEW_QUEUE.md`, and goes in the next summary (R-369).
  Before R-369 such a choice read "applied per R-204 — veto?".
- **Every open question is in `REVIEW_QUEUE.md`**, never only in a PR description or a log (the human, 1 Oct 2026,
  R-346). Decisions under R-369 aren't questions. A session stops at a clean point: nothing half-applied, every open
  question recorded there.
- **An open mark is named by its entry** (R-354). Each "veto?" mark in decisions.md, CLAUDE.md, `plan/` or `docs/`
  has a `**Mark:**` line in its open REVIEW_QUEUE.md entry, giving the file and text near the mark. The ruling that
  settles it replaces "veto?" with the ruling ("applied per R-204, accepted by R-m") in the commit that archives the
  entry; `plan/check_plan.py` fails on an open mark no open entry names (`plan/tools/veto_marks.py`). R-369's
  decisions carry no "veto?", so the check guards the old marks only, and none is open since R-370.
- **Changing a decision.** A port adds; it never changes a decision without a REVIEW_QUEUE entry and a ruling
  (CLAUDE.md § "Changing the docs"). Before committing a docs change, word-diff each removed line against its
  replacement; if a decision's content changed, restore it and open an RQ instead (24 Sep 2026).

## Resources

Check free disk and memory pressure before every dispatch, build or reviewer.

| | Mac (the human's machine) | Linux cloud |
|---|---|---|
| Memory pressure | `sysctl kern.memorystatus_vm_pressure_level`: 1 normal, 2 warning, 4 critical (R-252); not swap, which macOS keeps allocated | `/proc/pressure/memory` (PSI) where the kernel has it, else `free -m`. R-347: read `some avg10` as normal below 10, warning from 10, critical from 40 or when `full avg10` passes 5; without PSI, read "available" below 25% of total as warning and below 10% as critical |
| Agents at once | 3 at normal (R-277); at warning, 3 unless the Mac is swapping, and then 2 (R-391's trial, below; R-277's 2 at warning is the fallback); at critical only the running work finishes (R-277); never more than 3 (R-262) | the same levels as R-277 gives them, 2 at warning (R-391's trial is Mac only), and (R-347) no more agents than `nproc` / 4, R-347's cloud rule, set when each agent built with 4 jobs (R-228, superseded by R-658, whose queue is Mac only) |
| Free disk | aim for ≥ 25 GB; start nothing below 15 GB; below 20 GB, clean (R-262, 28 Sep 2026). Read `df -h`, not `du`: `du` counts APFS clones in full | the same thresholds (R-347), read with `df -h "$HOME"` |
| Build settings | heavy commands through the job queue only, each job with `CARGO_BUILD_JOBS` and `RUST_TEST_THREADS` at the core count (R-658, below); `CARGO_INCREMENTAL=0` | the same |

**The swap check (R-391, a trial from 6 Oct 2026; Mac only).** At warning, before each dispatch, read "Pageouts" from
`vm_stat` twice, 60 s apart (`vm_stat | awk '/Pageouts/ {print $2}'`; wait with Monitor, not a foreground `sleep`).
If it rose by more than 1000 in that minute (about 16 MB a minute at the Mac's 16 KB pages), the Mac is swapping: allow
2 agents. Otherwise allow 3, R-262's cap (R-391's "3 or more" is three under that cap). Log both readings with the
dispatch. The threshold is the orchestrator's proposal, applied per R-369; the baseline on 6 Oct 2026, at pressure 2
with two agents running, was 39 and 2 page-outs a minute, with 0 swap-outs. Each day of the trial, log one line
(§ "Logs"): the agents running, the page-out rates read, the dispatches held to 2, and every timeout or flaky failure
with its run or PR. If timeouts or flaky failures rise above the days before the trial, revert to R-277's 2 at warning
at once, log it, and tell the human in the next summary. After three days, report the log in the next summary; the
trial runs until the human rules on it or it is reverted.

**The memory guard (R-439).** While any agent builds or tests on the Mac, the orchestrator keeps a memory guard
running, `/Volumes/X10 Pro/Principia/orch/mem-guard.sh`. Every 2 s it reads `ps -axo pid=,rss=,comm=` and selects
only a process whose executable's path starts with the targets directory, `/Volumes/X10 Pro/Principia/targets/`: the
test binaries, never rustc or a shell whose arguments merely mention the path. A selected process whose resident memory
passes 3 GB (3 × 1024 × 1024 KB) is killed by its PID (`kill -9 <pid>`). macOS has no `prlimit`, so a local run has no
per-process memory cap of its own (R-352): the guard is the Mac's stand-in, not CI's cap, which stays REQ-VAL-181's.
Applied per R-369: it selects by the executable's path and kills by PID, never by `pkill -f` on a pattern; it logs its
start and each kill to `/Volumes/X10 Pro/Principia/orch/mem-guard.log` (§ "Logs"); and it is stopped when the
session's last agent finishes, leaving no process behind.

**The job queue (R-658; Mac).** No agent runs a heavy command directly, the orchestrator included: it submits it to
the local job queue, `cargo xtask job` (`xtask/src/jobq.rs`), and waits for it.
- **Heavy and light.** Heavy is anything that compiles: cargo build, check, test, clippy, doc and run; `cargo xtask`
  (but for its `job` command); cargo mutants and nextest; benches and GPU tests. Light runs directly: docs, git, gh,
  grep, `python3` checks such as `plan/check_plan.py`, `cargo fmt`, `cargo metadata`, reading logs.
- **How an agent calls it.** Through the installed client, `"/Volumes/X10 Pro/Principia/orch/jobq/bin/xtask"`, so
  that calling the queue compiles nothing:

  ```
  export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
  "/Volumes/X10 Pro/Principia/orch/jobq/bin/xtask" job submit <worktree> --priority fix --label TASK-M2-36 \
    --env CARGO_TARGET_DIR=<target> -- cargo test -p kernel --test roundtrip
  "/Volumes/X10 Pro/Principia/orch/jobq/bin/xtask" job wait <id>
  ```

  `submit` prints the job id; `wait` blocks until the job ends, prints how it ended and its log path, and exits with
  the job's status (its exit code; 124 timed out, 125 lost, 126 not started, 130 cancelled). The job doesn't belong to
  its caller: an agent whose call would time out runs `wait` in the background or again, and an agent that dies leaves
  its job to finish or time out. `job status` lists the runner, the running jobs, the queue in run order and the recent
  jobs; `job cancel <id>` drops a queued job or ends a running one with its process group. After a merge that changes
  `xtask/src/jobq.rs`, the orchestrator builds xtask from `main` through the queue and runs `<target>/debug/xtask job
  install`, which copies it to the client's path.
- **Priorities,** highest first, first in, first out within each: `merge`, merge-ready verification of a final head (a
  reviewer's check of the PR head, the orchestrator's pre-merge checks); `fix`, fix-round tests; `build`, a new task's
  builds; `peek`, exploratory runs and measurements.
- **The runner** runs two heavy jobs at a time (`concurrency` in the config; measured, below). Each job gets
  `CARGO_BUILD_JOBS` and `RUST_TEST_THREADS` at the core count (11 on the Mac), in place of R-228's four per agent,
  unless its `--env` sets them (a gui mutants run's `RUST_TEST_THREADS=1`, R-439). `--env CARGO_TARGET_DIR=<target>` is
  required, and a queued job whose target directory a running job uses waits, so no two jobs share one; directories are
  compared made canonical, without a trailing `/` and in lower case, so two spellings of one are one. A job keeps the
  submitter's `PATH`, `HOME`, locale and its `CARGO_*`, `RUSTUP_*` and `RUST*` settings, but not what `cargo run` sets
  for the submitter itself, the target directory or the job counts; its `--env` comes last, so `--env PATH=…` replaces
  the submitter's PATH. The job's command is looked up on that PATH, not the runner's, so an agent exports PATH with
  `/opt/homebrew/opt/rustup/bin` first before it submits: rustup's proxies there read `rust-toolchain.toml` and have
  clippy and fmt, and `~/.cargo/bin`'s cargo has neither (jobs failed with "cargo: command not found" or without
  clippy and fmt on 30 Oct 2026, applied per R-369). `submit` starts the runner
  when none is alive, and so does `wait`, at most one every 5 s, so no fault of the lock can multiply runners, and reaps
  it. With nothing queued or running for 60 s the runner exits, and nothing else stays behind. No launchd.
- **Not mutation-tested locally.** xtask's mutants are not run on the Mac: a mutant that breaks the live-runner check
  or the lock starts runners in process groups of their own, outside any test's reach, and a local `cargo mutants` run
  of the first draft (30 Oct 2026) pushed the Mac past its per-user process limit, failing every agent's forks. CI's
  mutants gate runs them, on a throwaway runner. Nothing of the queue is excluded from mutation (R-196's xtask
  exclusions stay `main.rs` and `codegen.rs`); a survivor that is truly equivalent goes into
  `.cargo/mutants-equivalent.toml` (R-202).
- **State** lives in `/Volumes/X10 Pro/Principia/orch/jobq/`, outside the repository: `jobs/<id>.json`, one file per
  job; `logs/<id>.log`; `runner.lock`, the runner's pid, heartbeat, concurrency and cores per job; `state.lock`, a
  `flock` every change to the state holds briefly, with no `ps` call or grace wait inside it, and the holder's pid and
  the time it took it written in it: a call that can't take it within `lock_wait` seconds fails, naming that pid and
  how long it has held it; `config`; `runner.log`; `violations.log`.
- **Robust to restarts.** A job runs in a process group of its own; past its timeout (3600 s unless `--timeout` says
  otherwise, measured on a monotonic clock from the job's start) or when cancelled, the runner sends the group SIGTERM,
  then SIGKILL after 10 s; whenever the job's own process ends, the runner kills what is left of its group. A runner
  lock whose pid is dead or whose heartbeat is more than 120 s old has expired, and the next `submit` or `wait` starts a
  runner in its place. A new runner ends the group of every job a dead one left running, and queues it again, or marks
  it lost after its third start.
- **The limits of that recovery** (applied per R-369). A runner killed between a job's spawn and its save leaves that
  start untracked: its file still says queued, so the job runs again while that one copy has no timeout or cancel and
  may overlap the second start on one target directory. Cargo's own lock serialises the builds, but test binaries can
  overlap, so a verdict from that run may be flaky. `runner.log` shows the start, and `violations.log` a heavy copy,
  its group being in no running job. No lock is orphaned: `state.lock` goes with the dead runner, `runner.lock` expires
  at 120 s and cargo's lock goes with its process. Closing the window would take a gate wrapper around every job, out of
  proportion to one spawn return and one small write. A second limit: when a job's file holds no leader start time, the
  new runner ends a live group only if its leader started within 5 s of the job's recorded start; a leader further off
  is left alone, as possibly another process's, and the job is queued again.
- **A failed save in a live runner** starts nothing and runs nothing the file doesn't record. If the save after a spawn
  fails, the runner kills and reaps that start's group and the job stays queued, so at most one copy is alive and none
  after the failure; each later pass tries again. If the save at a job's end fails, the runner keeps the job, with its
  exit status and end time, and saves it again on the next pass. A pass with a failed save starts no job and counts as
  idle, so a runner that can't write exits after `idle` seconds, and the next runner ends and queues again each job
  still marked running.
- **Violations.** While it runs, the runner reads `ps` every 30 s and appends each heavy process that no queue job
  started to `violations.log`, with its pid, parent, cwd and command: only the topmost of a chain, each once. It never
  kills one. The orchestrator reads the log at its summaries.
- **The config,** `orch/jobq/config`, one `key = value` per line: `concurrency` (2), `timeout` (3600), `stale`
  (120), `sample` (30), `idle` (60), `grace` (10), `tick_ms` (500), `lock_wait` (30), `cores`, and `env KEY=VALUE`
  lines every job starts with.
- **Agents.** R-391's and R-262's counts still bound the agents dispatched; each writes, reviews and waits, and the
  queue alone bounds the heavy load (applied per R-369).
- **Measured** (30 Oct 2026, R-658 item 6): the three task heads' targeted tests from warm APFS-cloned targets, each
  mode twice, in the order a, b, c, c, b, a, with the background load at the start below 1.5 × the cores (round 1,
  then round 2, in each cell):

  | Run (background load1 at start) | Wall-clock | Peak load1 | Memory pressure, worst | Page-outs/min, mean / worst | Swap-outs |
  |---|---|---|---|---|---|
  | (a) before: three at once, `CARGO_BUILD_JOBS=4` (12.8; 10.8) | 112 s; 89 s | 25.7; 22.9 | 2; 1 | 262 / 484; 18 / 25 | 16,536; 0 |
  | (b) the queue, one job at a time (13.1; 13.0) | 135 s; 135 s | 15.6; 16.8 | 2; 1 | 4 / 7; 1 / 2 | 0; 0 |
  | (c) the queue, two at a time (9.9; 12.6) | 101 s; 97 s | 15.4; 19.5 | 1; 2 | 2 / 4; 63 / 93 | 0; 0 |

  Two at a time verified the three heads in 97–101 s, one at a time in 135 s, so the runner runs two. Three agents'
  own builds at four jobs each were about as fast (89–112 s) but peaked at a load of 23–26 and paged out, and that
  pattern, at seven agents with mutants and a GPU sweep beside them, is what took the load to about 100.

**Cleaning disk** (28 Sep 2026): delete each reviewer's worktree and target when its review ends, and each task's when
its PR merges (R-345). Below 20 GB, `cargo clean` stale targets (merged or abandoned first, then the main checkout's),
and clear mutants and scratch builds. Never delete sources, uncommitted work, open PR branches or `~/.cargo`'s registry
caches. Three parallel builds once left the Mac's disk at 117 MiB free (30 Sep 2026).

**Nothing outside the repository without asking (R-349).** No agent, the orchestrator included, deletes or modifies
anything outside the repository and its build and scratch directories without asking the human first, caches included.
- Inside: the checkouts (the main checkout and every worktree), their target directories (the seed and a mutants run's
  `<target>-mutants` among them) and the session's scratch directory. Cleaning these, as above, needs no asking.
- Outside: `~/.cargo` (registry, git checkouts, installed tools), `~/.rustup`, the rust-gpu cache
  (`~/.cache/rust-gpu`; `~/Library/Caches/rust-gpu` on the Mac), the repository's Actions caches on GitHub, the system
  temp folder outside the scratch directory, shell and git configuration, and the SSD's folders other than the
  away-mode log (§ "Logs"), which, named here as the orchestrator's, is inside.
- A build's own writes to its caches are part of the build: cargo filling its registry, rustup installing the pinned
  toolchain, `cargo xtask build-kernel` building rust-gpu's backend in its cache (R-350). An agent's own deletion or
  edit there asks first: emptying the rust-gpu cache for a cold run, clearing `~/.cargo`, removing a toolchain,
  deleting an Actions cache entry.
- What a build, test or tool writes in its normal course outside the repository is part of running it: tests' and
  tools' files under `std::env::temp_dir()`, and cargo-mutants' temporary copy of the tree. An agent's own deletion or
  edit there asks first.
- `scripts/cloud-setup.sh`'s installs are asked for by R-346 and R-347; anything beyond them asks first.
- These readings are R-349's items applied per R-204, accepted by R-352.

## Paths and warm builds

**Mac only.**
- Worktrees go in `/Users/malachy/principia-work/worktrees/`, target directories in
  `/Users/malachy/principia-work/targets/`, on the internal disk (R-262). Until 29 Sep 2026 they lived on the external
  SSD (`/Users/malachy/principia-ssd`, a link to `/Volumes/X10 Pro/Principia`), which dropped access twice even with Full
  Disk Access; work started there finished there. The SSD folder also holds the human's own folders: leave them alone.
- `targets/seed` is a warm build of `main`, with and without the `controls` features. A new target starts as an APFS
  clone, `cp -cR /Users/malachy/principia-work/targets/seed <new target>`, which is instant; a build from another
  worktree then recompiles only the workspace's crates (~30 s, against 150–280 s cold). Refresh the seed after merges.
  Clone only for a worktree at a different path from the seed's own: a clone used from the seed's path isn't rebuilt,
  and keeps using the seed's `tmp/` and binaries (29 Sep 2026).
- There is no `env.sh`: each dispatch gives the environment (§ "Dispatching"). Put `$HOME/.cargo/bin` and
  `$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin` on PATH. `cargo` on PATH is `~/.cargo/bin/cargo`, a
  standalone binary with no fmt or clippy that shadows rustup's toolchain; the second directory supplies `cargo-fmt`
  and `clippy-driver`. Before that fix, agents wrongly reported rustfmt and clippy missing (26 Sep 2026). For the
  pinned nightly (§ "Toolchain"), put that toolchain's `bin` directory first instead. For the job queue, export PATH
  with `/opt/homebrew/opt/rustup/bin` first before `job submit`: a job runs with its submitter's PATH, and rustup's
  proxies there pick the pinned nightly from `rust-toolchain.toml` (§ "Resources", "The job queue"; R-658).
- `syspolicyd` can stall the launch of freshly built binaries: a stuck test run sits at 0% CPU. After a long agent run,
  check `ps` for leftovers.
- The terminal and Claude Code have Full Disk Access (29 Sep 2026). If "Operation not permitted" failures come back,
  report them to the human; they are not test findings.
- sccache: measured again on 30 Oct 2026 (R-658), a cold `cargo build -p xtask` of one commit in two worktrees, each
  with its own target directory, through one cache. With `CARGO_TARGET_DIR` in the environment, as every job has it, 0
  of 175 compile requests hit, with or without `SCCACHE_BASEDIRS` (sccache 0.18): sccache hashes every `CARGO_*`
  variable, and the target directory is one. With it given as `--target-dir` instead, 117 of 175 hit (66.9%), the
  dependency crates, with or without `SCCACHE_BASEDIRS`; the workspace's crates and those reading `OUT_DIR` missed. The
  hits saved about 10 s of a 100 s cold build (99–110 s against 90–91 s, at a background load of about 45). Not enabled,
  pending the owner's decision: every job's command would have to carry `--target-dir`, and jobs start from APFS-cloned
  warm targets whose dependencies are already built, which leaves it nothing to hit. `--remap-path-prefix` through
  `RUSTFLAGS` can't be used here: the paths hold a space, which `RUSTFLAGS` splits on. (Tried first on 27 Sep 2026, with
  no hits.)

**Linux cloud.**
- Use `$HOME/principia-work/worktrees/` and `$HOME/principia-work/targets/`, with the same rules: one worktree and one
  target per agent (R-219), never moved or renamed (§ "Reviewers").
- A warm seed works as on the Mac: copy it with `cp -a --reflink=auto targets/seed <new target>`, which clones on a
  filesystem that supports it (btrfs, XFS) and copies in full elsewhere, costing disk (§ "Resources").
- After `scripts/cloud-setup.sh`, `cargo` is rustup's proxy, which reads `rust-toolchain.toml` itself: no PATH fix.
- *Untested* (R-347): sccache may share builds between worktrees only if their paths are kept out of its cache keys
  (path remapping); the Mac's trial had no hits because they weren't.

## Toolchain

- TASK-M0-14 (#96) pins the whole workspace to rust-gpu's nightly in `rust-toolchain.toml`. Stable clippy fails on
  `spirv_std` (E0514), so clippy runs on the pinned nightly. On the Mac, put that toolchain's `bin` directory first on
  PATH; on Linux, rustup's proxy picks it from the file.
- `scripts/cloud-setup.sh` reads the toolchain from what CI reads. While `main` has no `rust-toolchain.toml`, it
  installs stable as CI's `dtolnay/rust-toolchain@stable` steps do; once #96 merges, it installs the nightly the file
  pins, and installs stable only if a Linux job still asks for it.
- **The native build runs without MIR GVN (R-547, 21 Oct 2026; RQ-299).** The pinned nightly miscompiles plain Rust
  above opt-level 0 in rustc's MIR GVN pass, so the native build sets `-Zmir-enable-passes=-GVN`, scoped to the host
  target and never the SPIR-V build, and a CI job runs the native tests at the release profile (TASK-M2-49,
  REQ-TOOL-188 and REQ-TOOL-189). The fault is reported upstream; the pin moves to a fixed nightly once
  `rustc_codegen_spirv` builds against one, and the flag goes with it.
- **Toolchains change only through the pin (R-414, 10 Oct 2026; R-349).** `~/.rustup`'s stable stays at 1.99.0;
  nothing updates, reinstalls or removes it. Every dispatch sets `RUSTUP_AUTO_INSTALL=0` (§ "Dispatching"), so
  rustup's proxy never installs a toolchain on its own; a new toolchain arrives only by a change to
  `rust-toolchain.toml`, through a PR.

## Metal and perf (Mac only)

- `gpu-metal` and `metal_hosted_probe` run only on macOS: on CI's `macos-15` runner, or on the Mac. A cloud session
  runs the GPU suites on lavapipe (`PRIN_GPU_BACKEND=vulkan`, R-169), as CI's Linux jobs do, and leaves Metal to CI.
- Benchmarks and performance gates run on the human's Mac via `prin profile` (R-186), never on a hosted runner or a
  cloud machine. A cloud session never reports a performance number of its own: it asks the human for the run, and its
  PR says it waits for it (`plan/WORKFLOW.md`).

## `measure/` branches (R-272)

A throwaway `measure/<what>` branch may be pushed so that CI takes a measurement (REQ-VAL-138's lavapipe max-step and
RQ-164's ubuntu mutants timing were taken this way). Record the number in the PR or `decisions.md`, delete the branch
straight after, and never open a PR from it. If the permission classifier refuses the push, show the human the rule;
don't route around it.

## Pitfalls

- **A target directory moved or renamed between worktrees** fails tests falsely (§ "Reviewers"). Where one has been
  moved anyway, before the next run: touch every `crates/*/tests/*.rs` and `xtask/tests/*.rs` (mtime only), run
  `cargo clean -p xtask` in it, and delete its nested `tmp/qa_TASK-M0-01-alias-target`, which `clean -p` doesn't reach
  (#79, 30 Sep 2026).
- **`gh pr edit` outside a git checkout** fails with "not a git repository": run it from the repository, or pass
  `--repo Ma1achy/Principia`.
- **Never chain cleanup after `gh pr merge` with `;`** (§ "Merging").
- **Approvals on an older head** don't count, except as R-260 carries them (§ "Reviewers").
- **A ruling relayed to a running agent** gets it blocked (R-289).

## Logs

- **Mac only.** The running away-mode log is `/Users/malachy/principia-ssd/overnight-log.md`.
- **R-391's trial log (Mac only).** Each day of the trial, one line in the away-mode log: the date; the agents running
  (the peak, and how many dispatches ran at warning with 3); the page-out rises read (the lowest and highest per
  minute, and each dispatch held to 2); every timeout or flaky failure, local or in CI, with its run or PR; and whether
  the trial was reverted (§ "Resources").
- **The memory guard's log (R-439; Mac only).** `/Volumes/X10 Pro/Principia/orch/mem-guard.log`: the guard's start,
  with its limit and PID, and each kill, with the PID, RSS and executable (§ "Resources").
- **The job queue's logs (R-658; Mac only).** In `/Volumes/X10 Pro/Principia/orch/jobq/`: `runner.log`, each runner's
  start, jobs and exit; `logs/<id>.log`, each job's output; `violations.log`, the heavy processes run outside the queue
  (§ "Resources").
- **Linux cloud.** A cloud machine's disk may not outlive the session. Under R-347,
  keep the running log in the session's scratch directory, and post the away-mode summary as the session's final
  message and as a comment on each PR it concerns.
- On either machine, a log is never where a question lives: open questions go in `REVIEW_QUEUE.md`
  (§ "Asking the human").
