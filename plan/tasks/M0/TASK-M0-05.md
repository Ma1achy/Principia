# TASK-M0-05 — The numerical-gate runner, and the convergence-under-refinement gate

- **Milestone:** M0
- **Closes:** REQ-VAL-002, REQ-VAL-004
- **Depends on:** TASK-M0-01, TASK-M0-04
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~400 lines

## Goal
`cargo xtask gate <gate>` runs a numerical gate from `fixtures/gates/<gate>/` against the threshold its requirement gives — or, where the corpus gives none, its calibration requirement's proposed value — and writes a gate report with sections for scatter, the number of regions sampled and the negative results (philosophy §4.6). The first gate is `convergence`: an aggregate quantity computed at successively finer sampling is quotable only if it converges (philosophy §4.5a), defined by R-171: samples are ordered coarse → fine (strides 32, 4, 1, 0, where stride 0 is unstrided, the finest); r_k = |x_k − x_{k−1}| / |x_{k−1}|; the gate passes iff r_k is strictly decreasing and the finest r_k is below the threshold. The pitfalls §3 record, in that order 0.2153 → 0.4423 → 0.5494 → 0.0947, gives r = 1.054, 0.242, 0.828 and must fail. At M0 the gate runs on the provisional threshold 0.1, named against REQ-VAL-135 and marked provisional (R-171); the calibrated threshold is proposed in M3, where the march gives a real converging aggregate (TASK-M3-34, R-113).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.5a A quantity that does not converge under refinement is measuring the sampling"
- `docs/read_first/principia_00_philosophy.md` § "4.6 Report the negative, and the messy"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-171 — The convergence gate, defined *(closes A1, A2, T2)*"

## Deliverables
- `crates/validation/src/gate/` — the gate trait (`run(fixture) -> GateReport`) and registry; `xtask/src/gate.rs` — `cargo xtask gate <gate>` and `cargo xtask gate --all` (registered in `cargo xtask ci`).
- Fixture format `fixtures/gates/<gate>/gate.json`: inputs, and the threshold given as a requirement id; the runner refuses a numeric threshold that names no requirement or calibration requirement.
- `GateReport` (markdown + JSON under `target/gates/`): verdict, threshold and its source id, scatter, region count, negative results; a conclusion drawn from fewer regions than the gate declares is flagged; the writer refuses a report with an empty section.
- `crates/validation/src/convergence.rs` — r_k and the R-171 pass rule; a sequence not ordered coarse → fine (strides strictly decreasing, 0 last) is refused.
- `fixtures/gates/convergence/pitfall_escape_fraction.json` — strides 32, 4, 1, 0 → 0.2153, 0.4423, 0.5494, 0.0947 (r = 1.054, 0.242, 0.828), expected to fail.
- `fixtures/gates/convergence/converging.json` — strides 32, 4, 1, 0 → 0.3000, 0.3200, 0.3260, 0.3275 (r = 0.0667, 0.0188, 0.0046), expected to pass.
- `fixtures/gates/convergence/gate.json` — threshold 0.1, named against REQ-VAL-135 and marked provisional; the report prints "provisional (REQ-VAL-135, R-171)" until M3 confirms the calibration.

## Acceptance tests
- `cargo xtask gate convergence` — `pitfall_escape_fraction.json` fails (r not strictly decreasing); `converging.json` passes (finest r = 0.0046 < 0.1); a sequence in the wrong order is refused; threshold: 0.1, provisional against REQ-VAL-135 (R-171; calibrated in M3, R-113) (REQ-VAL-004).
- `cargo test -p validation gate_report` — a report missing its scatter, region-count or negative-results section is refused; a conclusion from fewer regions than declared is flagged (REQ-VAL-002).

## Notes
- No physics runs in M0; the gate is exercised on recorded sequences. Its first physical use is M3 (escape fraction and the other aggregate quantities).
- RQ-93 ruled: R-113 — REQ-VAL-135 moves to M3 (TASK-M3-34 proposes it on a real converging aggregate); the runner and the gate stay here with the provisional threshold 0.1 (R-171).
