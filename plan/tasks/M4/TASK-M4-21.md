# TASK-M4-21 — Link parameters on the GPU: the uniform, bitwise today's at the defaults, and a parameter change that moves only the masses it should (R-481, R-482)

- **Milestone:** M4
- **Closes:** REQ-RENDER-099, REQ-CHART-074
- **Depends on:** TASK-M4-06, TASK-M2-41, TASK-M2-21
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~300 lines, the tests included

## Goal
The baked kernels read the tunable link parameters from the chart params' uniform block, set from `SimConfig`'s `Links`, so a parameter change is a uniform edit and not a variant; at the defaults every payload and golden is bitwise today's (C2); `μ_max = 4` against 5 moves the payloads of a latent view at the fixed non-zero mass logit `z_μ = (0.5, −0.3)` and not those of a latent view with both mass logits at 0 (C3; R-626, RQ-574 (a): the mass simplex chart sets its masses directly, so `μ_max` cannot move them; the logit applied per R-369). *Was:* "`μ_max = 4` against 5 moves the mass simplex chart's payloads and not those of a latent view with both mass logits at 0 (C3)".

## References
- `decisions.md` § "R-481 — Link parameters are adjustable ⚑: each registry entry declares its tunable parameters with a name, a default, an allowed range and a citation, today `μ_max` on `softmax_tanh` and `q_max` on the momentum links, `α_min` staying fixed; every value of a range keeps Part 2.5's three hard requirements, `check` testing its ends and a sampled interior; the values are recorded beside the link id wherever it is recorded; the log-det reweights at the active value; the generated code reads the parameters as a uniform, bitwise today's at the defaults *(amends R-10)*"
- `decisions.md` § "R-482 — The tests of R-478 to R-481, each with an arm that can fail: the dropdown compatible-only, the defaults bitwise today's, a parameter change moving the result, the range's ends admissible, the log-det at a non-default value, provenance reproducing bitwise, the preview the kernel's own function, and Home leaving the links alone"
- `decisions.md` § "R-626 — R-482 C3's mass-varying chart is a latent configuration view at a fixed non-zero mass logit, or a mixed chart with a mass axis, not the mass simplex chart, which sets its masses directly and cannot show `μ_max` moving them *(closes RQ-574; amends R-482 C3's choice of chart)*"
- `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part 2.5)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Links carry a measure"

## Deliverables
- The parameters in the uniform block and their upload from `Links`.
- Tests `link_params_uniform`, `link_param_moves`.

## Acceptance tests
- `cargo test -p engine link_params_uniform` — REQ-RENDER-099's verify (C2 on renders and payloads; every existing golden unchanged).
- `cargo test -p engine link_param_moves` — REQ-CHART-074's verify (C3 on the latent view at `z_μ = (0.5, −0.3)`, R-626, with the logits-at-0 control).

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics. It covers C3's payloads (R-481 G8).
- Perf: the uniform adds a few loads per sample, at the march's start and at a lock, never per integrator step (R-481 G7); the perf reviewer confirms no per-step load.
- **R-626 rules RQ-574** (25 Oct 2026): C3's mass-varying chart is not the mass simplex chart, which sets its masses directly and reads no mass link (`crates/kernel/src/chart/mass_simplex.rs`), so `μ_max` cannot reach its payloads; the first arm runs on a latent view at a fixed non-zero mass logit.
- No golden is re-baselined: a moved fixture or golden at the defaults is a C2 failure (R-482).
