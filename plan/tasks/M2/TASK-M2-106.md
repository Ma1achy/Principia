# TASK-M2-106 — Pin every transcendental the CPU chart evaluator reaches through the pinned libm, enforced by a test (R-991)

- **Milestone:** M2
- **Closes:** REQ-CHART-212
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~400 lines, the test and the measurement's harness included

## Goal
The owner's answer to RQ-799 (R-991): "RQ-799: (a)." Every f64 transcendental reachable from the CPU chart evaluator
(sin, cos, sin_cos, tan, atan2, asin, acos, exp, exp_m1, ln, ln_1p, powf, hypot, cbrt, sinh, cosh, tanh and any added
later) goes through the pinned pure-software libm (`libm = "=0.2.16"`, `force-soft-floats`, as `crates/kernel/Cargo.toml`
already pins it for the `N_sub` table, R-502), through the `Real` trait's methods as `arctan2` and `sinc` already are;
`sqrt` is excepted, IEEE 754 requiring it correctly rounded. A test fails on any stray call, with a planted call as its
control. So every record the CPU writes is the same on every machine, before any record is written on the new route
(TASK-M2-86 depends on this task).

## References
- `decisions.md` § "R-991 — RQ-799, the maths library: option (a), pin, enforced; every f64 transcendental reachable from the CPU chart evaluator goes through the pinned pure-software libm, `sqrt` excepted, a test failing on any stray call with a planted call as its control, so records are identical on every machine; fixtures that change are a recorded conversion *(closes RQ-799)*"
- `decisions.md` § "R-502 — The `N_sub` thresholds are generated per sim key on the host, with a pinned pure-software libm, shipped as exact f32 bits and read by constant index; the table's maximum length is a calibration value, refused above it; a test asserts the default config reproduces the build-time table bitwise *(closes RQ-370)*"
- `decisions.md` § "R-931 — Goldens: a golden or reference image that is provably wrong is fixed, everything that depends on it is updated, and tests run against the fixed version; an incorrect thing is never treated as correct, and a bug is never replicated for the sake of matching it *(no RQ; refines R-110)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 2 — The decoder"
- `decisions.md` § "R-197 — Who may fix, suppress or configure a lint *(closes RQ-134)*"

## Deliverables
- `crates/kernel/src/real.rs`: each transcendental above a `Real` method whose f64 body calls the pinned libm (libm's
  `expm1` and `log1p` for `exp_m1` and `ln_1p`), and the f32 body unchanged where the GPU path needs it; the call sites
  RQ-799's entry lists, read on `main` at 54fa96fc (`chart/invariant.rs`, `construction.rs`, `physical.rs`, `burrau.rs`, `burrau_acute.rs`,
  `shape_sphere.rs`, `tilt.rs`, `generated/links.rs`, `encode/ingest.rs`) and the link generator's emitted `cosh`
  (`crates/ledger/src/gen/links.rs`) moved onto those methods.
- A test in the style the entry calls `discipline_lint`'s, over the CPU evaluator's source: any `Float::` or inherent
  `f64` transcendental outside the `Real` impl fails, `sqrt` excepted; its control, a planted `Float::sin`, fails. Not a
  clippy `disallowed-methods` entry, which would be lint configuration (R-197).
- Every fixture written from the f64 path whose bits change, regenerated and recorded as a conversion (R-931), listed
  in the PR with the count of changed values per fixture.
- The entry's measurement and its report: on TASK-M2-86's three charts and grids, records that differ bitwise, at f64
  and after narrowing, between the std path and the pinned path on the Mac, and under the pinned path across the Mac,
  Linux, Windows and the wasm build, each against the Mac; the wasm runtime and the Windows host named before the run.

## Acceptance tests
- `cargo test -p kernel transcendental_pin` — the test above: no stray transcendental in the evaluator's source; the
  planted `Float::sin` fails (REQ-CHART-212).
- `cargo test -p kernel real_pinned_bits` — each `Real` method's f64 result equals the pinned libm's on a fixed set of
  inputs, bit for bit; control: a method left on std fails on an input where the two differ, found and named in the PR
  (REQ-CHART-212).
- `cargo xtask measure libm-pin` — the measurement: PASS when, under the pinned path, the count of differing records
  against the Mac is 0 on Linux, Windows and wasm at both levels; the std/pinned counts and the std path's cross-machine
  counts reported as the size of the change; control: a one-ulp perturbation of `sin` gives a non-zero count at the f64
  level, or the harness is void. The Windows and wasm arms run where the PR says they ran (REQ-CHART-212).

## Notes
- Entered 13 Nov 2026 by R-991 (applied per R-369: task scoping). TASK-M2-86's chain had no task that builds the pin, so
  this task is new and TASK-M2-86 depends on it: the owner recommended the pin "before any record is written on the
  new route".
- The doc comment at `crates/kernel/src/real.rs:49–50` is corrected apart from this task (the report's D-12); this task
  makes what the corrected comment then says true.
- The speed cost of a software libm is measured by TASK-M2-86's own benchmark (`cargo xtask bench first_slice`,
  REQ-PERF-120), which runs on the pinned path.
- **Released with R-987.** A chart task, entered when R-923 is lifted (R-987): dispatched once the pull request porting
  the owner's reply of 13 Nov 2026 merges; integrator (M3) tasks keep priority for agent slots.
