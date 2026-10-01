# TASK-M0-50 — `cargo xtask lint wgsl` fails on `isinf`, `isnan`, comparisons against inf, NaN or finite-max constants, and self-comparisons in fragment-stage WGSL

- **Milestone:** M0
- **Closes:** REQ-RENDER-083
- **Depends on:** TASK-M0-13
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
Fast-math (R-297) may optimise `isinf`, `isnan` and comparisons against inf or NaN away, and may fold or break a float
compared with itself or against a finite-max stand-in used as an inf check, so an unset-check on a value read in a
fragment shader tests its bit pattern (R-343): `pa_d_min_is_unset` against `PA_D_MIN_UNSET = 0x7c00u`. TASK-M0-13
checks that with a review-checklist grep. Under R-351 and R-352 it becomes an automated lint: `cargo xtask lint` fails
on `isinf` or `isnan`, on a comparison against an inf or NaN constant, on a float compared with itself (`x != x`,
`x == x`), and on a comparison against 65504.0 or another finite-max stand-in, in fragment-stage WGSL, naming the file,
the line and the rule, and naming a bit-pattern test as the fix. The checklist grep stays as a backup.

## References
- `decisions.md` § "R-352 — RQ-191's thirteen items stand; the fragment-stage lint also fails on a float compared with itself and on comparisons against finite-max stand-ins *(closes RQ-191; amends R-348 and R-351)*"
- `decisions.md` § "R-351 — #107's `closure_step_reserved` offsets stand; the bit-pattern unset check becomes a `cargo xtask lint` rule over fragment-stage WGSL"
- `decisions.md` § "R-343 — The fragment unpack layer binds `SimStateFTLE` at `@group(1) @binding(0)` and the word buffer at `@group(1) @binding(1)`; WGSL forms of `closure_step` and the schema version *(closes RQ-188)*"
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"
- `decisions.md` § "R-271 — `d_min`'s unset value is +inf; stored values never reach 0.0 *(closes RQ-163, amends payload §1)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"

## Deliverables
- `xtask/src/lint_wgsl.rs` (TASK-M0-13's): a rule that fails, naming the file, the line and the rule, and naming a
  bit-pattern test (R-343) as the fix, on:
  - **`isinf` and `isnan`:** any use of either (in naga's IR, `IsInf` and `IsNan`, or a call to a function of either
    name);
  - **inf and NaN constants:** any float comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`) one of whose operands is an inf
    or NaN constant, a constant expression that evaluates to one included, such as `bitcast<f32>(0x7f800000u)`;
  - **a float compared with itself (R-352):** any comparison, by `==`, `!=`, `<`, `<=`, `>` or `>=`, of a float scalar
    or vector expression with itself: the same naga expression on both sides, or structurally equal expressions
    reading the same `let`, argument, variable or buffer element (the same index expression), with no store to that
    variable between the two reads;
  - **finite-max stand-ins (R-352):** any comparison, by any of the six operators and in either operand order, one of
    whose operands is a finite-max stand-in: binary16's largest finite value, 65504 (generation root §3.8's
    `f16_finite_max`), or binary32's, 3.40282347e38 (`0x7f7fffff`), of either sign, in any spelling (`65504.0`,
    `65504.`, `6.5504e4`, `65504.0f`, `65504h`, `3.40282347e38`, `3.4028235e38`), as a `bitcast<f32>` of its bit
    pattern (`0x477fe000u`, `0x7f7fffffu`) or as any other constant expression that evaluates to it.

  It runs over every WGSL file under `crates/render/frag/`, generated or written by hand, as part of
  `cargo xtask lint wgsl`, which `cargo xtask ci` runs. The existing rules keep checking the generated file as they do.
- Fixtures under `xtask/tests/fixtures/lint_wgsl/`: one breaking each case the acceptance tests list, and the clean
  fixtures each test passes.
- The tests below, each with a registered negative control (R-176).

## Acceptance tests
- `cargo test -p xtask lint_wgsl_unset` — the fixtures that call `isinf`, call `isnan`, compare a float against an inf
  constant (`bitcast<f32>(0x7f800000u)`) and compare one against a NaN constant each fail `cargo xtask lint wgsl`,
  naming the file, the line and the rule; the clean fixture passes; a WGSL file under `crates/render/frag/` outside
  `generated/` is linted too; each with a registered negative control (REQ-RENDER-083, R-351; the lint can fire).
- `cargo test -p xtask lint_wgsl_self_compare` — fixtures comparing a float with itself by `!=`, by `==` and by each of
  `<`, `<=`, `>` and `>=`, a `let` with itself, a buffer element read twice, and a `vec2<f32>` with itself each fail
  `cargo xtask lint wgsl`, naming the file, the line, the rule and the bit-pattern fix; the clean fixture passes. It
  holds the near misses an over-broad rule would fail: two different floats (`x != y`, `a.x == a.y`), two different
  buffer elements (`buf[i].v == buf[j].v`), a variable compared with its own earlier value after it is reassigned
  (`let old = v; v = v + 1.0; old != v`), and an unset value tested by its bits. Each with a registered negative
  control (REQ-RENDER-083, R-352; the lint can fire).
- `cargo test -p xtask lint_wgsl_finite_max` — fixtures comparing a float against 65504 spelled `65504.0`, `65504.`,
  `6.5504e4`, `65504.0f` and `65504h`, against `-65504.0`, against 3.40282347e38 and its negation, against
  `bitcast<f32>(0x477fe000u)` and `bitcast<f32>(0x7f7fffffu)`, with the constant on the left (`65504.0 < x`), and by
  each of `<`, `<=`, `>`, `>=`, `==` and `!=` each fail `cargo xtask lint wgsl`, naming the file, the line, the rule
  and the bit-pattern fix. The test asserts that the finite-max rule is the one that fires, not merely that the lint
  fails: the `65504h` fixture also trips TASK-M0-13's "no `enable f16`" rule, so its test and its negative control
  check for the finite-max rule's name among the findings. The clean fixture passes. It holds the near misses an
  over-broad rule would fail: comparisons against 65503.0 (below f16's maximum), against `3.4028233e38` and
  `bitcast<f32>(0x7f7ffffeu)` (below f32's) and against 1.0, a clamp with `min(x, 65504.0)` (no comparison), and an
  unset value tested by its bits. Each with a registered negative control (REQ-RENDER-083, R-352; the lint can
  fire).
- `cargo xtask lint wgsl` — on the workspace: passes; the generated `payload_unpack.wgsl`, whose `pa_d_min_is_unset`
  compares bits, is clean (REQ-RENDER-083, R-343, R-352).
- Review checklist (code) — REQ-RENDER-001's grep (no `isinf`, `isnan` or self-comparison in the generated WGSL) is
  still in the checklist, as the backup (R-351).

## Notes
- Applied per R-204, accepted by R-352 (R-351): "fragment-stage WGSL" is every WGSL file under `crates/render/frag/`;
  the rule joins `cargo xtask lint wgsl` rather than a new subcommand; a constant expression that evaluates to inf or
  NaN counts as a constant.
- Applied per R-204 — veto? (R-352): the finite-max stand-ins are the list above (65504 and 3.40282347e38, of either
  sign, in any spelling, as a `bitcast<f32>` of its bit pattern or as another constant expression); a self-comparison
  is caught by `<`, `<=`, `>` and `>=` as well as by `==` and `!=`; "the same expression" is read structurally, and
  covers float vectors.
- The lint reads naga's IR, where a literal's spelling is gone, so every spelling of the same value is one check; the
  fixtures spell it several ways to show that. `65504h` is an f16 literal and needs `enable f16`. REQ-RENDER-001's ban
  on `enable f16` (R-317) covers the generated WGSL only, so a hand-written file under `crates/render/frag/` may hold
  f16, and the lint must catch a finite-max comparison there. Two consequences:
  - With naga 30.0.1 (`Cargo.lock`), the lint's validator is built with `Capabilities::SHADER_FLOAT16`; without it,
    the `65504h` fixture fails validation before the rule runs.
  - TASK-M0-13's existing "no `enable f16`" rule also fires on that fixture, so its acceptance test and negative
    control check that the finite-max rule fires, as above.
- WGSL has no `isInf` or `isNan` built-in, so in WGSL source the names reach naga only as calls to a function so named
  (or fail to parse, which the lint already reports); naga's `IsInf` and `IsNan` come from other front ends. The rule
  checks both.
- Size: R-352's two rules and their tests take the task from ~200 to ~350 counted lines (R-211), within one PR.
- Reviewers as TASK-M0-13's, which wrote the lint.
