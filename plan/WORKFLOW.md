# Workflow

1. One task, one branch, one PR. Tasks come from `plan/PLAN.md` in order, each listed in `plan/tasks.yaml`
   (id, checkpoint, title, status). The branch is `task/<id>` (e.g. `task/TASK-C1-01`), which the README's
   progress grid reads. A task is at most 20 lines: the goal, what it touches, how it's tested, the spec
   sections that bind it. Keep tasks small: a day's work, merged the same day if it can be.
2. The implementer writes the code and its tests (each with a control arm), runs them, and pastes the
   output in the PR. While working: `cargo check` and its own crate's tests; the full relevant set once,
   before opening the PR.
3. One reviewer checks whether it's right: the code, the tests, the physics where relevant. Add the physics
   reviewer only when the diff touches integrator, regularisation, decoder, chart or encode algebra, or a
   tolerance.
4. Blocking findings: a wrong result, a broken invariant, a failing test, security. Anything else is a note
   in the PR, fixed now if trivial, otherwise left.
5. At most two fix rounds. Merge when CI is green and the reviewer approves. Merge commits, not squash.
6. Run implementers in parallel on tasks that don't touch the same files, as memory allows. Reviewers don't
   build: they read the diff and CI's test results, and run a test themselves only to check a specific
   suspicion. So reviews can run anywhere, including the PC (`plan/MACHINES.md`), and the Mac's cores go to
   implementers.
7. Each day, one message to the owner: a screenshot of the real app, the product PRs merged, anything
   broken. Plain English throughout; a task or PR number only in brackets after what it is.
8. At a checkpoint (`plan/PLAN.md`): show the running app, the key numbers and the calls worth a second
   look; then a fix pass from the owner's reply.

At most 40 lines.
