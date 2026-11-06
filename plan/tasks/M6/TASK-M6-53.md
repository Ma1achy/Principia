# TASK-M6-53 — The GUI track (R-390): the mock's value noise hashes its lattice in integers, so shared corners agree bitwise

- **Milestone:** M6
- **Closes:** REQ-GUI-412, REQ-GUI-418
- **Depends on:** TASK-M6-24
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~50 lines of WGSL, the tests and ci wiring beside them

## Goal
`noise.wgsl`'s corners are computed from integer lattice coordinates by an integer hash and an exact conversion, so a
corner two cells share is bitwise equal on every backend, and CI proves it on Metal and lavapipe with controls that fail
there. The RTX is a measurement (R-649). The mock's pattern changes, option A (R-689); nothing about the real kernel is
claimed.

## References
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-685 — The real kernel and the other WGSL and rust-gpu shaders are audited for the mock noise's pattern, hashes or lattice lookups where neighbouring invocations must agree bitwise and compile-time offsets feeding them; the fix is proposed as a task with its own ADR, plus a regression test that asserts shared lattice corners bitwise equal on every backend; no fix until lavapipe and the audit are in"
- `decisions.md` § "R-686 — The kernel cross-backend check is a measurement (R-649): the same IC set on NVIDIA Vulkan, Metal, lavapipe and CPU f32, outcome labels and per-trajectory branch-sensitive results compared, not pixels, the disagreement rate reported per backend pair, no tolerance chosen to make it pass; pixel comparisons across backends go on raw pre-colour values with the outlier tail reported, never 8-bit frame equality, Metal against lavapipe's 161,394 / 221 recorded as the clean-backend baseline; `DECODE_SWITCHOVER`'s spec notes that bitwise agreement must be tested on NVIDIA specifically"
- `decisions.md` § "R-689 — The mock's noise is option A: an integer u32 hash on integer lattice coordinates, and the mock's noise look may change *(closes RQ-691)*"
- `decisions.md` § "R-691 — The raw-value rule for pixel comparisons is for cross-backend investigation only; goldens stay as they are on CI backends *(closes RQ-692; amends R-686)*"
- `decisions.md` § "R-692 — R-656 is amended for TASK-M6-53: of `crates/gui/`, the paths the new `gpu-metal` step reads are GPU paths, and the rest of the crate is not *(amends R-656)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-206 — An unset `PRIN_GPU_BACKEND` defaults by platform"
- `decisions.md` § "R-656 — CI throughput: a push runs `ci.yml` only on `main` (and on R-272's `measure/` branches, which have no pull request), a pull request running it through `pull_request`; a pull request's superseded runs are cancelled, never a run on `main`; a docs-only pull request skips the build, test, GPU and mutants work, and one that changes no GPU-path file skips the GPU suites, inside the jobs, every required check still reporting success under its own name; mutants run on the merge-ready head; a bounded target cache is approved in principle, built in a later PR; no paid or self-hosted runner *(amends R-110, R-177, R-186, R-195, R-235, R-285 and R-335)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/notes/principia_gpu_determinism_note.md` § "The parity consequence (what shared source does and does not buy)"

Also read `plan/adr/ADR-0001-mock-noise-integer-lattice.md` § "Decision" and § "Must not claim": the design this task builds.

## Deliverables
- `crates/gui/src/mock/noise.wgsl`: `pcg`, `lattice_hash`, `corner`, `cell_corners`; `value_noise` as ADR-0001 states; the
  float `hash` gone; the header comment true.
- `crates/gui/src/tests/mock_lattice.rs` and `lattice_controls.wgsl` (test-only): the tests below, which append test-only
  entry points to `noise_source()`, so the shipped functions run rather than a copy, and read corners back as bits
  (`bitcast<u32>` to `Rgba32Uint`), so no float is canonicalised on the way.
- `crates/gui/Cargo.toml`: `naga` as a dev-dependency at wgpu 30's version, `wgsl-in`.
- `.github/workflows/ci.yml`: a `gpu-metal` step, `cargo nextest run -p gui lattice_ --no-capture`, and its header comment
  (:51), which says a change to `crates/gui/` alone skips the GPU suites.
- `scripts/ci_changes.sh`: what the step reads is a GPU path: `crates/gui/src/mock/*.wgsl`, `crates/gui/src/mock/canvas.rs`
  (`noise_source()`), `crates/gui/src/tests/mock_lattice.rs`, `crates/gui/src/tests/lattice_controls.wgsl` and
  `crates/gui/Cargo.toml`. `is_not_gpu`'s comment is made true, and `xtask/tests/ci_changes.rs` gains these cases.
- `plan/OPERATIONS.md` § "Merging"'s "no GPU-path file" bullet (:395), made true the same way.

## Acceptance tests
- `cargo test -p gui lattice_` on lavapipe and on Metal, both in the PR with the adapter line — the checks below (REQ-GUI-412).
- `lattice_shared_corners_bitwise`: across x, y and diagonal crossings, 2 views × 2 warps × 5 octaves, plus a negative-quadrant
  view and a far view, shared corners are equal bit for bit, each case with crossings > 0. Controls, through `rejects`:
  the ulp-nudged corner and the distributed-offset corner (`lattice_controls.wgsl`) (REQ-GUI-412).
- `lattice_corners_match_cpu_twin`: every corner equals the Rust twin's bits at its integer coordinates. Control, through
  `rejects`: a twin with one multiplier changed (REQ-GUI-412).
- `lattice_field_has_no_jump_lines`: 0 jump lines in `warp.x`, `warp.y`, `n` at the two views, through the shipped
  `stand_in_at`; tolerance: REQ-GUI-418 (calibrated), the detector's `JUMP_STEP` and `JUMP_RUN`. Control, through `rejects`: the
  same field on the distributed-offset noise (REQ-GUI-412).
- `lattice_source_is_integer` (no GPU): naga's IR of `noise_source()` holds no float binary operation in `lattice_hash`, and
  in `corner` only the one multiply by a power of two. Control, through `rejects`: the old `hash` (REQ-GUI-412).
- `lattice_raw_precolour_dump` on Metal and on lavapipe, and `lattice_raw_precolour_compare` on the two dumps, both `#[ignore]`
  so CI's `lattice_` filter doesn't run them, in the PR: raw f32 `n`, `warp.x` and `warp.y`, at centre 0/0, ×0.77, tilts 0, the count of pixels differing and the count
  beyond a threshold stated in the PR before the run. This is the raw pre-colour baseline R-686 leaves to be measured. It
  is a report and asserts nothing; never 8-bit frame equality (`docs/contracts/principia_parity_contract.md` § 4).
- `cargo test -p gui mock_figure_navigation`, `cargo test -p gui --test qa_TASK-M6-30`, `cargo test -p gui --test
  qa_TASK-M6-24` — pass with no threshold changed. A failure goes to `REVIEW_QUEUE.md`; nothing is loosened to pass.
- `cargo xtask screenshot --all` — before and after of `01_main/mock_shell` and `02_stain/mock_stain` in the PR, for the eye;
  no comparison across backends.
- `python3 plan/check_plan.py`.

## Notes
- **The pattern is option A (R-689; ADR-0001 § "Alternatives").** No golden changes: no fixture holds mock pixels, so nothing is
  re-baselined. *Was:* the fix started only after the owner ruled on the pattern, option A or option D.
- **Measurement, not a gate (R-649):** `lattice_old_hash_measurement` (`#[ignore]`) prints the old hash's unequal crossings on
  the adapter. On the RTX: a Windows session at the PR head, `PRIN_GPU_BACKEND=vulkan`, `cargo nextest run -p gui lattice_
  --run-ignored all --no-capture`, the adapter line posted to #390, and the owner looks at centre 0/0 ×0.77 and 0.18/0.41
  ×1.00. Modelled, not measured: old hash unequal 1031 of 7217, new 0. The outcome is PASS, EXPLAINED DIVERGENCE or
  UNEXPLAINED, and it blocks nothing. It is the one place the real defect meets the real test, which is why the arm stays
  in the repo, ignored.
- **Pixel comparisons across backends (R-686, R-691):** for cross-backend investigation, on raw pre-colour values, with the outlier
  tail reported as a count beyond a stated threshold; never 8-bit frame equality. Goldens stay as they are on CI backends. The recorded 8-bit baseline, Metal against lavapipe, is 161,394 of 1,274,316
  pixels differing, 221 by more than 32 of 255, on the old pattern (`docs/contracts/principia_parity_contract.md` § 4).
  `qa_TASK-M6-30`'s and `mock_figure_navigation`'s `TOL` and `TOLERANCE` compare frames on one backend, so the rule doesn't
  touch them.
- **Controls where Metal and lavapipe show no bug.** The old `hash` passes `lattice_shared_corners_bitwise` on both, so it can't
  be a control there. The distributed-offset corner writes the rewrite by hand: lavapipe is predicted to reject it, and
  Metal is unverified, since its fast-math might factor it back. Run it on Metal while authoring. If it does not reject
  there, that is a finding, not a tuning point: Metal prints its count, the PR says so, and the ulp-nudged corner, which
  fails everywhere by construction, keeps the control. All controls use `rejects` from `crates/gui/src/tests/support.rs`:
  gui has no `negative_control!`, which is `validation`'s, and `gui` → `validation` is forbidden (R-176).
- **The ci wiring stays here, and it meets R-656.** R-656 names `gui` among "the crates no GPU job builds", and this task's
  `gpu-metal` step builds it. R-685's "on every backend" is what forces the change: no gui test runs on Metal in CI today, and
  a gui-only PR skips `gpu-metal`. R-692 amends R-656 for this: only the five paths in Deliverables, what the step reads, become
  GPU paths. `ci_changes.sh` has one `gpu` flag, so such a PR runs all three GPU suites, not `gpu-metal` alone. Any other gui
  change still skips them; that gap is accepted, and it is listed below.
- **Two xtask tests.** `qa_r662_every_tracked_path_classifies_as_the_ruling_says` in `xtask/tests/qa_R-662_ci_scope.rs` fails:
  `ruling()` (:145) marks all of `crates/gui/` no-GPU, and it walks every tracked path. The ruling that changes that behaviour
  is R-692. Every commit of that file is a qa commit, so this is qa's ordinary `M` (R-290), for the code reviewer to confirm
  against R-692. The implementer's `ci_changes.sh` commit leaves xtask's tests red until qa's commit lands.
  `xtask/tests/qa_R-677_ci_tiers.rs` is untouched: its one gui case, `crates/gui/src/qa.rs`, stays true.
- The cost of building eframe on `macos-15` is unmeasured; the PR gives it.
- **What CI cannot catch:** NVIDIA itself, on which no job runs; a float hash coming back, which only `lattice_source_is_integer`
  stops, and only in `lattice_hash` and `corner`; a driver rewrite that depends on compilation context, since a test's entry
  point is not `fs_main`; the float path before `floor` and after the corners, not checked bitwise; a gui change outside the
  five GPU paths that breaks the step, which skips `gpu-metal` on its PR and first shows on `main`; other GPUs, on which
  nothing is measured. "Every backend" here means gui's two, since gui's wgpu has no `dx12` feature.
- **Claims nothing about the real paths.** ADR-0001 § "Must not claim".
- The PR has a "What to try" section (R-390).
