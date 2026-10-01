# TASK-M0-50 — `cargo xtask lint wgsl` fails on `isinf`, `isnan` and comparisons against inf or NaN constants in fragment-stage WGSL

- **Milestone:** M0
- **Closes:** REQ-RENDER-083
- **Depends on:** TASK-M0-13
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
Fast-math (R-297) may optimise `isinf`, `isnan` and comparisons against inf or NaN away, so an unset-check on a value
read in a fragment shader tests its bit pattern (R-343): `pa_d_min_is_unset` against `PA_D_MIN_UNSET = 0x7c00u`.
TASK-M0-13 checks that with a review-checklist grep. Under R-351 it becomes an automated lint: `cargo xtask lint`
fails on `isinf` or `isnan`, or on a comparison against an inf or NaN constant, in fragment-stage WGSL, naming the
file, the line and the rule. The checklist grep stays as a backup.

## References
- `decisions.md` § "R-351 — #107's `closure_step_reserved` offsets stand; the bit-pattern unset check becomes a `cargo xtask lint` rule over fragment-stage WGSL"
- `decisions.md` § "R-343 — The fragment unpack layer binds `SimStateFTLE` at `@group(1) @binding(0)` and the word buffer at `@group(1) @binding(1)`; WGSL forms of `closure_step` and the schema version *(closes RQ-188)*"
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"
- `decisions.md` § "R-271 — `d_min`'s unset value is +inf; stored values never reach 0.0 *(closes RQ-163, amends payload §1)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"

## Deliverables
- `xtask/src/lint_wgsl.rs` (TASK-M0-13's): a rule that fails, naming the file, the line and the rule, on any use of
  `isinf` or `isnan` (in naga's IR, `IsInf` and `IsNan`, or a call to a function of either name), and on any float
  comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`) one of whose operands is an inf or NaN constant, a constant expression
  that evaluates to one included, such as `bitcast<f32>(0x7f800000u)`. It runs over every WGSL file under
  `crates/render/frag/`, generated or written by hand, as part of `cargo xtask lint wgsl`, which `cargo xtask ci` runs.
  The existing rules keep checking the generated file as they do.
- Fixtures under `xtask/tests/fixtures/lint_wgsl/`: one breaking each case above.
- The tests below, each with a registered negative control (R-176).

## Acceptance tests
- `cargo test -p xtask lint_wgsl_unset` — the fixtures that call `isinf`, call `isnan`, compare a float against an inf
  constant (`bitcast<f32>(0x7f800000u)`) and compare one against a NaN constant each fail `cargo xtask lint wgsl`,
  naming the file, the line and the rule; the clean fixture passes; a WGSL file under `crates/render/frag/` outside
  `generated/` is linted too; each with a registered negative control (REQ-RENDER-083, R-351; the lint can fire).
- `cargo xtask lint wgsl` — on the workspace: passes; the generated `payload_unpack.wgsl`, whose `pa_d_min_is_unset`
  compares bits, is clean (REQ-RENDER-083, R-343).
- Review checklist (code) — REQ-RENDER-001's grep (no `isinf`, `isnan` or self-comparison in the generated WGSL) is
  still in the checklist, as the backup (R-351).

## Notes
- Applied per R-204 — veto? (R-351, RQ-191): "fragment-stage WGSL" is every WGSL file under `crates/render/frag/`; the
  rule joins `cargo xtask lint wgsl` rather than a new subcommand; a constant expression that evaluates to inf or NaN
  counts as a constant.
- RQ-191 asks whether the lint also covers the render contract's other stand-ins, `x != x` and `x > 65504.0`. Until it
  is ruled, they stay with the checklist grep; if the human widens the lint, this task's rule and fixtures widen before
  it starts.
- WGSL has no `isInf` or `isNan` built-in, so in WGSL source the names reach naga only as calls to a function so named
  (or fail to parse, which the lint already reports); naga's `IsInf` and `IsNan` come from other front ends. The rule
  checks both.
- Reviewers as TASK-M0-13's, which wrote the lint.
