# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-191: R-348–R-351's choices applied per R-204, and whether R-351's lint covers the other stand-ins *(R-348, R-349, R-351, plan/OPERATIONS.md, CLAUDE.md)*

- **File, section:**
  - `decisions.md` § "R-348 — Mutants runs get a per-mutant timeout and a per-process memory cap on test processes;
    both values are calibrated": "mutants runs get two caps"; the numbering note.
  - `decisions.md` § "R-349 — Agents never delete or modify anything outside the repository and its build and scratch
    directories without asking first, caches included": "outside the repo and its build/scratch directories …
    caches included".
  - `decisions.md` § "R-351 — #107's `closure_step_reserved` offsets stand; the bit-pattern unset check becomes a
    `cargo xtask lint` rule over fragment-stage WGSL": "fails on isinf/isnan, or comparisons against inf/NaN
    constants, in fragment-stage WGSL".
  - `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)": "never
    `isinf` or `isnan` (nor a float comparison standing in for them, such as `x != x` or `x > 65504.0`)".
- **What:** these were applied without asking (R-204) and are open for a veto:
  1. The message numbered its rulings from R-347, which was taken; they are recorded as R-348 to R-351 (R-278).
  2. R-348: REQ-VAL-179, a requirement for the caps themselves, beside the two calibrations (REQ-VAL-180, the
     per-mutant timeout; REQ-VAL-181, the memory cap).
  3. R-348: TASK-M0-49 waits for TASK-M0-14 (#96), which rewrites `mutants.yml`.
  4. R-348: TASK-M0-19, which writes the nightly full run, depends on TASK-M0-49 and applies the same caps at the same
     values.
  5. R-348: "mutants runs" includes a local run, not CI's alone: it uses the same caps at the same values, each where
     its machine enforces it. macOS is not known to enforce `ulimit -v` and has no `prlimit`, so a local Mac run may
     have the timeout only; TASK-M0-49 checks. (The other reading: the caps are CI's only.)
  6. R-349: what is inside (the checkouts and worktrees, their target directories, the session's scratch directory)
     and what is outside (`~/.cargo`, `~/.rustup`, the rust-gpu cache, the repository's Actions caches, the system
     temp folder, shell and git configuration, the SSD's folders).
  7. R-349: a file or folder `plan/OPERATIONS.md` names as the orchestrator's is inside wherever it lives: the Mac's
     away-mode log on the SSD (`/Users/malachy/principia-ssd/overnight-log.md`).
  8. R-349: a build's own cache writes (cargo's registry, rustup's pinned toolchain, build-kernel's rust-gpu backend)
     are part of the build; an agent's own deletion or edit of a cache asks first.
  9. R-349: what a build, test or tool writes in its normal course outside the repository is part of running it:
     tests' and tools' files under `std::env::temp_dir()` (`xtask/tests/deps.rs`, `xtask/src/deps.rs`,
     `crates/validation/src/spawn.rs`, `crates/prin/src/profile/diff.rs`) and cargo-mutants' temporary copy of the
     tree; an agent's own deletion or edit there asks first.
  10. R-349: `scripts/cloud-setup.sh`'s installs are asked for by R-346 and R-347; anything beyond them asks first.
  11. R-351: "fragment-stage WGSL" is every WGSL file under `crates/render/frag/`.
  12. R-351: the rule joins `cargo xtask lint wgsl` rather than a new subcommand.
  13. R-351: an inf or NaN constant includes a constant expression that evaluates to one (a `bitcast<f32>` of an inf
      or NaN bit pattern).
- **And one question:** the render contract names `x != x` and `x > 65504.0` as float comparisons standing in for
  `isnan` and `isinf`. Neither compares against an inf or NaN constant, so under R-351's words the lint does not cover
  them, and REQ-RENDER-001's checklist grep alone checks them.
- **Options seen:**
  1. **Accept items 1–13 (recommended);** veto any by number, saying what replaces it.
  2. For the question: (a) the lint covers only R-351's words, the checklist grep the two stand-ins (as applied); or
     (b) the lint also fails on a self-comparison of a float and on a comparison against `65504.0` (f16's largest
     finite value), and REQ-RENDER-083 and TASK-M0-50 widen to match.
- **Needed:** accept or veto, and (a) or (b). Nothing waits on it: TASK-M0-49 and TASK-M0-50 can be built as written,
  and (b) would add rules to TASK-M0-50 before it starts.
