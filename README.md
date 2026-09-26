# Principia

Principia is a physics instrument for the planar three-body problem: a browsing instrument for a space nobody browses,
paired with a measuring instrument for the things browsing finds. One kernel, written once in Rust, runs on the GPU
(f32, through rust-gpu to WGSL) for the interactive atlas and on the CPU (f64) for the inspector and the prebake. It
maps initial conditions through a set of charts, integrates them with a regularised integrator, and colours each pixel
by what happened to that trajectory, saying where it stops knowing. It is built by coding agents from the plan in this
repository.

## Where to start

- `docs/read_first/`: what the instrument is for (`principia_00_philosophy.md`), the failures that shaped it
  (`principia_01_pitfalls.md`), and the map of every document (`principia_INDEX.md`).
- `plan/WORKFLOW.md`: how a task becomes merged code: one task, one branch, one PR, the reviewers, and the gates.
- `plan/MILESTONES.md`: the milestones M0 to M8 and each one's exit gate.
- `plan/BUILD_READINESS.md`: what is ready and what still waits before the build starts.

## What is authoritative

1. The markdown corpus in `docs/` (contracts, design, notes, read_first, gui) is the specification (R-1).
2. `decisions.md` records the human's rulings and overrides the docs where they differ. Open questions go to
   `REVIEW_QUEUE.md`.
3. `plan/` is derived from the two. Where a requirement and its source disagree, the source wins.

## What is not

- `workbench/`: working material (renders, prototypes, old pages).
- `docs/reference/`: the imported prin-rs code and notes, which are reference, not authority (R-159).
- `docs/archive/`: superseded documents and the finished untangling record.

Never cite these as normative, and never implement from them.
