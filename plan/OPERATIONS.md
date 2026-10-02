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
  merged, and never stack a task on an unmerged PR (28 Sep 2026; R-362).
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
- **What every dispatch names:**
  - the task id, and the PR number for a reviewer or a fix round;
  - the agent's worktree and its `CARGO_TARGET_DIR` (§ "Paths and warm builds");
  - the environment: `CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4` (R-228), `CARGO_INCREMENTAL=0`, and on the Mac the
    PATH fix (§ "Paths and warm builds");
  - a private scratch subdirectory for PR bodies and temp files, `<scratchpad>/<pr>-<role>/`. Agents sharing one
    scratch directory overwrote each other's `body.md`, and PR #54's description briefly showed TASK-M0-09's
    (30 Sep 2026);
  - any ruling since the task file was written, verbatim (R-289);
  - for a reviewer, the request to judge the PR's size and to name each decision it made (§ "Reviewers", R-369);
  - for a renamed or reused target, what to clean first (§ "Pitfalls").
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
- **Mutants.** Don't run `cargo mutants` locally; CI's shards do (`mutants.yml`, R-302). If one has to run locally, give
  it a target directory of its own (`<target>-mutants`), since a mutants run can leave a mutated build that cargo treats
  as fresh, and delete it straight after. A local run uses R-348's two caps, the per-mutant timeout and the memory cap
  on test processes, at the values CI uses (REQ-VAL-180, REQ-VAL-181), where the machine enforces them (applied per
  R-204, accepted by R-352): on macOS a local run gets the timeout only, and CI's Linux runners enforce both caps
  (R-352). The `mutants::skip` marker doesn't compile without the `mutants` crate as a dependency (E0433) and skips a
  whole function; an equivalent mutant gets a test, a behaviour-preserving rewrite (R-197), or a justified entry in
  `.cargo/mutants-equivalent.toml` (R-202).

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
| Agents at once | 3 at normal, 2 at warning, at critical only the running work finishes (R-277); never more than 3 (R-262) | the same levels, and (R-347) no more agents than `nproc` / 4, since each builds with 4 jobs |
| Free disk | aim for ≥ 25 GB; start nothing below 15 GB; below 20 GB, clean (R-262, 28 Sep 2026). Read `df -h`, not `du`: `du` counts APFS clones in full | the same thresholds (R-347), read with `df -h "$HOME"` |
| Build settings | `CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4` (R-228), `CARGO_INCREMENTAL=0` | the same |

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
  pinned nightly (§ "Toolchain"), put that toolchain's `bin` directory first instead.
- `syspolicyd` can stall the launch of freshly built binaries: a stuck test run sits at 0% CPU. After a long agent run,
  check `ps` for leftovers.
- The terminal and Claude Code have Full Disk Access (29 Sep 2026). If "Operation not permitted" failures come back,
  report them to the human; they are not test findings.
- sccache was tried and dropped: no hits across target directories, since their paths enter its hash (27 Sep 2026).

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
- **Linux cloud.** A cloud machine's disk may not outlive the session. Under R-347,
  keep the running log in the session's scratch directory, and post the away-mode summary as the session's final
  message and as a comment on each PR it concerns.
- On either machine, a log is never where a question lives: open questions go in `REVIEW_QUEUE.md`
  (§ "Asking the human").
