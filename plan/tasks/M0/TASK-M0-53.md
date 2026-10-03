# TASK-M0-53 — gate-report fails a requirement whose named test doesn't exist

- **Milestone:** M0
- **Closes:** REQ-SYS-079
- **Depends on:** TASK-M0-19
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~600 lines

## Goal
`cargo xtask gate-report` (TASK-M0-19) gives a unit-test, property-test or numerical-gate requirement "pass" whenever
the CPU suites pass, and never checks that the tests its `verify.detail` names exist. A cargo filter that matches no
test runs zero tests and still exits 0, so the report cannot tell an empty run from a passing one: the M0 gate
rehearsal (gate.yml run 37124629188, on bc1a7af) reported REQ-RENDER-083 passing while two of the tests its detail
names, `lint_wgsl_self_compare` and `lint_wgsl_finite_max`, were not yet on `main`.

For every requirement whose result comes from the hosted suites, the report now takes each
`cargo test -p <crate> [flags] <filter>` its detail names, and passes the requirement only when, for every one, a test
of that crate as the gate run built it with those flags matches the filter by cargo's own rule (a substring of the
test's path name). Otherwise the requirement fails with an outcome naming the crate, the filter and "no test matches",
and the report fails. The test listing is the gate run's own: gate.yml's `cpu` job lists the tests of each build the
details name (`cargo test <build> -- --list`, on the commit under test) and hands the listing to `gate-report`. A
hosted requirement whose detail names no such command keeps its suite's result, and the report marks it "no named test
to check".

## References
- `decisions.md` § "R-177 — Cadence *(closes G4, C6)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- `xtask/src/gate_report.rs`:
  - the named `cargo test` (and `cargo nextest run`) commands of a verify detail: crate, flags and filter;
  - the test listing, read from `cargo test <build> -- --list` output, and written by
    `cargo xtask gate-report --milestone <Mn> --write-test-list <file>`, which lists every build the gate's hosted
    requirements name;
  - the check: a hosted requirement fails, naming the crate, the filter and "no test matches", unless every named
    command matches a listed test of its build; one naming none is marked "no named test to check";
  - `--test-list <file>` on `gate-report --milestone <Mn> --results <file>`;
  - the module doc updated to match.
- `xtask/src/main.rs`: the new flags, and the usage text.
- `.github/workflows/gate.yml`: the `cpu` job writes the test listing on the commit under test and uploads it; the
  `gate-report` job downloads it and passes it to `--test-list`; the header comment updated to match.
- `xtask/tests/gate_report.rs`, with fixtures: a named test that exists passes; a filter matching nothing fails with the
  new outcome; a test that exists only under `--features controls`, with the listing lacking that build, fails; a
  detail naming several commands passes only when all match; each with its negative control (R-176).

## Acceptance tests
- `cargo test -p xtask gate_report_named_test` — the check and the listing, each with a registered negative control
  (REQ-SYS-079).
- `cargo test -p xtask --features controls gate_report_named_test` — the negative controls (R-176).
- `cargo xtask gate-report --milestone M0 --write-test-list <file>`, then `cargo xtask gate-report --milestone M0
  --results <file> --test-list <file>` on `main` — the PR lists every M0 gate requirement whose named test does not
  exist, and the count of M0's gate requirements in each class (named tests checked, no named test to check, not a
  hosted suite) (REQ-SYS-079).
- `gh workflow run gate.yml --ref task/TASK-M0-53 -f milestone=M0` — a hosted gate run; the PR shows its result and the
  report's non-pass lines (REQ-SYS-079).

## Notes
- The human's instruction, 3 Oct 2026, in their own words: "fix the gate report to check tests exist". It amends the
  decision RQ-201 recorded per R-369 (TASK-M0-19), that a unit-test, property-test or numerical-gate requirement passes
  whenever the CPU suites pass. No ruling is added to `decisions.md`.
- A requirement whose named test does not exist is not fixed here by editing its detail, nor is the check weakened for
  it; it is reported to the orchestrator.
