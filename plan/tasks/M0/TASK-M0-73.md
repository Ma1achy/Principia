# TASK-M0-73 — Mutation testing off the merge path: the PC's weekly sweep of the kernel

- **Machine:** pc (the PC's own session builds it; the Mac never dispatches it). Not a checkpoint.

## Goal
Mutation testing never holds a pull request. It runs on the PC (in WSL2) as a weekly sweep of the kernel crate, in
the background, yielding to the PC's GPU runs. It's a measurement: never a required check, and it never turns `main`
red. No gate check, no backlog file and no job-queue changes. Each sweep's surviving mutants go to the Mac as a
`to:mac` issue in `Ma1achy/principia-ops`; the Mac treats each real survivor as a missing test in the next PR that
touches that file. The Mac runs no mutation jobs.

## Touches
A scheduled job on the PC (outside the runner's folders and account, `plan/MACHINES.md`), `cargo mutants -p kernel`
with the repo's existing configuration, and the `to:mac` issue it files.

## Tests
The sweep's own: a known-surviving mutant planted in a scratch copy shows in its list, and a caught one doesn't.

