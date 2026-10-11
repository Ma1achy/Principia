# TASK-M1-18 — The per-field log floors (R-401)

- **Milestone:** M1
- **Closes:** REQ-TOOL-160
- **Depends on:** TASK-M1-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-3
- **Size:** ~200 lines, the re-rendered goldens included

## Goal
**R-401:** each log-scaled field view without a ledger floor has its own floor, in that field's units, proposed here
with evidence for the M1 gate, replacing the single ε = 2⁻²⁴ that TASK-M1-09 built. The six fields are `d_min`,
`dE_max`, `dLz_max`, `closure_min`, `rho_ratio` and `r_min_pair_0`. Each floor is a named presentation constant, read
by the numeric view template that TASK-M1-17 conforms to R-399, R-400 and R-403. Split from TASK-M1-17 (applied per
R-369; physics review 5482528309 of PR #194) so that RQ-271, which holds REQ-TOOL-160, holds only this task.

## References
- `decisions.md` § "R-401 — Each log-scaled field view has its own floor, in that field's units, proposed with evidence for the M1 gate; the single ε = 2⁻²⁴ is replaced"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-263 — §3.8 gains the optional key `floor?: <sim-key parameter>` *(closes RQ-158)*"
- `decisions.md` § "R-403 — `K_0` and `V_0` are `lin` on viridis over `[0, ∞)` and `(−∞, 0]`; a field carries the `diverging` scale only when its declared range spans zero *(closes RQ-261)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"
- `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library"
- `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug view)"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time, pre-integration)"

## Deliverables
- `crates/ledger`: one log floor per log field without a ledger floor (`d_min`, `dE_max`, `dLz_max`, `closure_min`,
  `rho_ratio`, `r_min_pair_0`), each a named presentation constant in that field's units, read by the template; not
  §3.8's `floor?` key, which R-263 gives a sim-key parameter. The REQ-TOOL-160 proposal: each floor, its units, and its
  evidence (where the field's stored values fall under it on M1's fixtures, and why it sits there), attached to the PR.
  The floors for `dE_max` and `dLz_max` are absolute, in the stored fields' own normalised units, since the corpus
  defines both as absolute maxima (R-401's note), and the proposal says so. `closure_min` is a dimensionless chord on
  the unit shape sphere, and its f32 precision floor of about 1e-7 (dd_simstate_payload § 1) is evidence for its
  floor.
- Every golden the floors change re-rendered (the log views of `m1-numeric`, and any merged task's golden that draws
  one of the six log views, TASK-M1-12's and TASK-M1-14's included), each `BASELINES.md` row citing R-401, proposed
  and confirmed at the M1 gate.

## Acceptance tests
- `cargo test -p ledger numeric_view_template` — every log view places raw at `dbg_log`'s form with its own field's
  floor, and two log fields with different floors place the same `|x|` differently (REQ-TOOL-160).
- Review checklist (physics): the REQ-TOOL-160 proposal gives each floor with its units and evidence, the floors
  for `dE_max` and `dLz_max` stated as absolute; the human confirms the floors at the M1 gate (REQ-TOOL-160).
- `cargo test -p ledger`, `cargo test -p render` and `cargo xtask golden` — every view other than the six log views
  renders as before.

## Notes
- Split from TASK-M1-17 (applied per R-369; physics review 5482528309 of PR #194, F1). RQ-271 holds REQ-TOOL-160:
  `rho_ratio`'s and `r_min_pair_0`'s formulas are REQ-PAY-088's (TASK-M2-03, M2), so this task waits for its ruling,
  while TASK-M1-17 proceeds with its four other closures. Until this task merges, the six views keep TASK-M1-09's
  built ε, 2⁻²⁴.
- REQ-TOOL-160 moved from TASK-M1-09, which built the single ε R-401 replaces, to TASK-M1-17, and from there to this
  task. R-403 confirmed R-401's absolute floors for `dE_max` and `dLz_max`.
- The qa tests of TASK-M1-09 that pin the single ε (for example `crates/ledger/tests/qa_TASK-M1-09.rs` and
  `crates/render/tests/qa_TASK-M1-09.rs`) are ruling-forced changes (R-401): qa makes them, and the PR lists each with
  the assertion it changes and the ruling that forces it (R-290, R-369).
- TASK-M7-05 depends on this task: its `log` compaction takes "each field's ε (its ledger `floor`, R-263, or its R-401
  display floor)".
