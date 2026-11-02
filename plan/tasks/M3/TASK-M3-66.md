# TASK-M3-66 — Dispatch refuses `g` and `m_total` other than 1 (R-668)

- **Milestone:** M3
- **Closes:** REQ-SYS-096
- **Depends on:** TASK-M3-06
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~60 lines

## Goal
The kernel runs only at `G = 1` and `M = 1` (integrator contract Part 5). `check_carried` checks the carried lengths, thresholds and `epsilon`, and neither `g` nor `m_total`, so a `SimUniforms` with `g = 2` validates and its `G = 1` run is keyed under `g = 2` (RQ-668). Dispatch refuses either one other than 1, with a typed reason, as it refuses `ε ≠ 0`; another `G` is converted at the import boundary, which `validation::import` already does (R-668).

## References
- `decisions.md` § "R-668 — The 1 Nov library triage rulings: the site-note families stay, marked "operator's own search, unreviewed", closure peeks only and never targets, `bhhsat` with them unless Davoust & Broucke's comparison matches, `suvakov_pc` closure only; a digit match is a trace, marked "inferred"; values printed in a paper, or in a public database a paper points to, are stored with credit and citation, an author emailed only as a last resort; unreviewed and AI-assisted sources are peeks only; hierarchical triples are parked with the v2+ 3D extension; RQ-666 is confirmed; the kernel stays at `G = 1`, another `G` converted at the import boundary and what can't be converted refused *(closes RQ-666 and RQ-668; corrects R-468)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 5 — Units and the horizon (inherited scale gauge)"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/design/principia_dd_validation_orbits.md` § "2. What the suite tests that the current gates do not"

## Deliverables
- `crates/engine/src/dispatch/validate.rs` — `check_carried` refuses `g` and `m_total` other than 1, NaN and ±∞ included, each with its own `DispatchError` variant carrying the value, in the module note's order.
- The module notes of `validate.rs` and `sim_uniforms.rs` name the refusal.

## Acceptance tests
- `cargo test -p engine units_refusal` — dispatch refuses g and m_total each at 2, 0, −1, the nearest f64 on either side of 1, NaN and ±∞, with a typed reason naming the field and its value, and the sim-key tests that key g = 2 or m_total = 2 without dispatching still pass; control: g = m_total = 1 dispatches, and a check_carried that skips g, or m_total, fails (REQ-SYS-096).

## Notes
- Added by R-668 (1 Nov 2026, RQ-668; the port, 2 Nov 2026). `m_total` is refused beside `g`, applied per R-369: Part 5 fixes `M = 1` beside `G = 1`, and `SimUniforms` carries both.
- `crates/engine/tests/sim_uniforms.rs` keys `g: 2.0` and `m_total: 2.0` to show each field moves the sim key; it never dispatches them, so it stands.
- The import boundary is not this task's: `validation::import` (TASK-M2-20) converts by the recorded rescale and refuses a `G` or a mass that is not finite and positive.
