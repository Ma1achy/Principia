# Principia

A Rust instrument for the planar three-body problem: pick a 2D slice through the 8D space of initial
conditions, integrate every pixel, colour each by how it ends, and measure what the picture shows.

## Where things are
- The spec: `docs/contracts/` and `docs/design/` (what was intended, and why); `docs/read_first/principia_INDEX.md`
  is the map. The code is the source of truth (see Decisions).
- Why things are the way they are: `docs/read_first/` (philosophy, pitfalls). Read the pitfalls before any
  physics or numerics change.
- Crates: `kernel` (physics, `no_std`, shared CPU/GPU), `engine`, `render`, `gui`, `ledger` (generated
  tables), `validation` (gates and cross-checks), `prin` (CLI). `xtask` builds the kernel and runs lints,
  gates and goldens.
- The plan: `plan/PLAN.md`. Known gaps: `plan/KNOWN.md`.
- `docs/history/`: the old rulings and requirements, frozen, because constants and gates cite them by id. Never
  edit it. R-, RQ-, REQ- and TASK- ids in comments are historical labels, not instructions.

## Build and test
- `cargo xtask build-kernel` before GPU tests; `cargo nextest run` for tests.
- `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` must be clean.
- On the Mac, compile through the job queue (`orch/`), never directly.

## Quality bar
- A test that guards a property has a control arm that fails when the property is broken. A test that can't
  fail is not a test.
- Write the pass criterion before running a measurement. Never move a threshold to pass.
- Internal correctness tests, anchor tests (figure-eight closure, Burrau) and CPU/GPU agreement within stated
  tolerances block a merge. Comparisons with published or outside numbers are measurements: report them,
  never block on them.
- A suspicious number gets a measurement, not a story. Read the per-pixel distribution, not only an
  aggregate. When an image looks wrong, reproduce it with one variable changed.
- When something goes wrong, the fix includes a test that would have caught it, not a new rule here.

## How work runs
`plan/WORKFLOW.md`. Product first: each week the app does something visibly better. No PRs that only change
process, plans or records, except to fix a red main. To the owner, name things in plain English, with any id in
brackets after it for reference: "the real field in the window (TASK-C1-01)", never "C1-01" alone.

## Decisions
Nothing is set in stone. The code is the source of truth, and any decision can be changed. When the plan or a
doc is unclear, wrong or has a gap, do the sensible thing and leave one comment where it matters: "The plan is
unclear here; chose X because Y." When the owner says change it, change the code (and that comment). Don't
record history, cite rulings or stack decisions on decisions: git is the record. Fix a doc only if it misleads.

## Ask the owner first only for
- money; contacting anyone outside;
- licences and publishing (commit only CC BY, CC BY-SA, CC0 or public-domain papers; anything else goes in
  the gitignored `docs/incoming/_local/`; `atlas-params.json` is never committed);
- deleting history, force-pushing or admin bypass;
- branch protection, required checks, or the Windows runner's security rules (push to main, schedule and
  dispatch only; the repo and actor guard; `contents: read`; no secrets; SHA-pinned actions; no Docker);
- anything on the owner's machines outside the repo and its build folders.

Decide everything else yourself; list the calls worth a second look at the next checkpoint. If one answer is
clearly right, do it and say so. To ask, make the question stand alone: what it's about in plain words, the
passage quoted, why it's unclear, the options with their trade-offs, and your pick and why; never just a file
and section. Never wait: if it blocks a task, move to work that doesn't depend on it (the GUI, say). No answer
by the next checkpoint means go with your pick.

## This file: at most 60 lines. To add a rule, remove one.
