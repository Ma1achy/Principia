# TASK-M1-18 — The per-field log floors (R-401)

- **Milestone:** M1
- **Closes:** REQ-TOOL-160, REQ-SYS-081, REQ-SYS-082
- **Depends on:** TASK-M1-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-3
- **Size:** ~400 lines, the re-rendered goldens and the register's `provisional` key included (R-427)

## Goal
**R-401:** each log-scaled field view without a ledger floor has its own floor, in that field's units, proposed here
with evidence for the M1 gate, replacing the single ε = 2⁻²⁴ that TASK-M1-09 built. The six fields are `d_min`,
`dE_max`, `dLz_max`, `closure_min`, `rho_ratio` and `r_min_pair_0`. Each floor is a named presentation constant, read
by the numeric view template that TASK-M1-17 conforms to R-399, R-400 and R-403. Split from TASK-M1-17 (applied per
R-369; physics review 5482528309 of PR #194) so that RQ-271, which held REQ-TOOL-160, held only this task. R-419
ruled it (option (c) for both fields), so this task waits on nothing.

**R-427** (the human's veto of this task's floor functions, PR #204): the floors must not become functions to get past
the constants lint. Each floor is a constants-register entry (dd_generation_root §3.8), marked provisional until the
M1 gate and read from the register. The register held no provisional value, so this task gives it one, for every gate:
the `provisional` key (REQ-SYS-081). Nothing in the lint's crates is wrapped in a function to get past it
(REQ-SYS-082): `prelude::hatch()`'s proposed colours and stripe width move into the register with the floors. And the
proposal states whether `d_min`'s floor at 2⁻¹⁴, f16's smallest normal, is intended when `d_min` is stored down to
2⁻²⁴ (R-271), so that the closest approaches all land at the bottom of the ramp. This task does not choose the floor
by that question; it proposes and gives the evidence.

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
- `decisions.md` § "R-419 — `rho_ratio`'s and `r_min_pair_0`'s log floors are confirmed provisionally at M1 and re-confirmed at M2; `rho_ratio`'s assumes the mass-weighted ‖ρ̃‖/‖λ̃‖ and says so; for `r_min_pair_0` only the evidence is provisional *(closes RQ-271)*"
- `decisions.md` § "R-427 — Veto of TASK-M1-18's floor functions: the floors are constants-register entries, marked provisional until the M1 gate and read from there, and the register holds provisional values for every gate; TASK-M1-18 states whether `d_min`'s floor at f16's smallest normal is intended; no veto on decisions 1, 2 and 4–7"
- `decisions.md` § "R-182 — Escape fixtures are defined; proposed tolerances are provisional in CI *(closes T4, T5)*"
- `decisions.md` § "R-271 — `d_min`'s unset value is +inf; stored values never reach 0.0 *(closes RQ-163, amends payload §1)*"
- `decisions.md` § "R-251 — TASK-M0-08 accepted at ~918; its register items, with the hash covering value, type and class"

## Deliverables
- `crates/ledger`: one log floor per log field without a ledger floor (`d_min`, `dE_max`, `dLz_max`, `closure_min`,
  `rho_ratio`, `r_min_pair_0`), each a named presentation constant in that field's units, read by the template; not
  §3.8's `floor?` key, which R-263 gives a sim-key parameter. The REQ-TOOL-160 proposal: each floor, its units, and its
  evidence (where the field's stored values fall under it on M1's fixtures, and why it sits there), attached to the PR.
  The floors for `dE_max` and `dLz_max` are absolute, in the stored fields' own normalised units, since the corpus
  defines both as absolute maxima (R-401's note), and the proposal says so. `closure_min` is a dimensionless chord on
  the unit shape sphere, and its f32 precision floor of about 1e-7 (dd_simstate_payload § 1) is evidence for its
  floor. `rho_ratio`'s and `r_min_pair_0`'s floors are proposed provisionally (R-419): `rho_ratio`'s assumes the
  mass-weighted ‖ρ̃‖/‖λ̃‖, and the proposal says so; `r_min_pair_0`'s is a distance in `G = M = I = 1` and only its
  evidence, M1's showcase values, is provisional. Both are re-confirmed at M2 by TASK-M2-03 (REQ-TOOL-163).
- Every golden the floors change re-rendered (the log views of `m1-numeric`, and any merged task's golden that draws
  one of the six log views, TASK-M1-12's and TASK-M1-14's included), each `BASELINES.md` row citing R-401, proposed
  and confirmed at the M1 gate.

- `crates/ledger/src/constants.rs`: the register's `provisional` key (R-427; REQ-SYS-081), as dd_generation_root §3.8
  gives it: an entry carries `provisional: gate(milestone)` with its proposed number or threshold and its calibration
  requirement as its citation, and is read like any other entry (`number()`, at compile time); generation fails on a
  provisional entry whose value is `calibration`, that names no gate, or whose citation is not a calibration
  requirement, naming the constant; the register lists its provisional entries by gate, for the gate report; the mark
  and its gate are not hashed into the schema version (`crates/ledger/src/version.rs`).
- The six floors as register entries (R-427; REQ-SYS-082), each marked provisional until the M1 gate, citing
  REQ-TOOL-160, read by the template from the register; no `*_log_floor()` function returns a literal, and no other
  function stands in for an entry. `rho_ratio`'s and `r_min_pair_0`'s entries say that at the M1 gate they are
  re-marked provisional until M2 (R-419, REQ-TOOL-163). dd_generation_root §3.8 lists the provisional entries in a
  table beside the payload ledger's, each with its gate and requirement.
- `prelude::hatch()`'s proposed violet `#9B00FF`, cyan `#48FFFF` and 4 px stripe (REQ-COL-055, R-71) as register
  entries marked provisional until the M1 gate, read by the prelude's generator from the register (REQ-SYS-082).
- The survey (REQ-SYS-082), attached to the PR: every function in `crates/{kernel,ledger,engine}` that returns a
  numeric literal, each either moved into the register or named, in its doc, as ledger or interface data (as
  `payload.rs`'s continuation tables and `canonical_qnan_bits()` are), with the reason.
- `d_min`'s floor (R-427): the REQ-TOOL-160 proposal states whether the floor at 2⁻¹⁴, f16's smallest normal, is
  intended when `d_min` is stored down to 2⁻²⁴ (R-271), with evidence: where `d_min`'s stored values in
  [2⁻²⁴, 2⁻¹⁴) fall on the ramp under the floor (the raw values at their ends), how many of M1's fixture values lie
  there, and what a floor at f16's smallest subnormal (2⁻²⁴, the register's `f16_min_subnormal`) or another proposed
  value would show instead. If the PR proposes a different floor, it gives that floor's evidence as for the others.

## Acceptance tests
- `cargo test -p ledger numeric_view_template` — every log view places raw at `dbg_log`'s form with its own field's
  floor, and two log fields with different floors place the same `|x|` differently (REQ-TOOL-160).
- Review checklist (physics): the REQ-TOOL-160 proposal gives each floor with its units and evidence, the floors
  for `dE_max` and `dLz_max` stated as absolute, `rho_ratio`'s stating that it assumes the mass-weighted ‖ρ̃‖/‖λ̃‖
  and `r_min_pair_0`'s that its evidence is provisional, both marked provisional until M2 (R-419); the human confirms
  the floors at the M1 gate (REQ-TOOL-160).
- `cargo test -p ledger`, `cargo test -p render` and `cargo xtask golden --all` — every view other than the six log views
  renders as before.
- `cargo test -p ledger constants_provisional` — a provisional entry builds and reads its number at compile time like
  any other; controls: a provisional entry whose value is `calibration`, one that names no gate, and one citing a
  corpus section each fail generation naming the constant; the list by gate returns the six floors and the hatch's
  numbers for M1; adding or removing the mark on a hashed fixture entry leaves the schema version unchanged, and
  changing its value changes it (REQ-SYS-081).
- `cargo xtask lint constants` passes, and review checklist (code): the floors and the hatch's numbers are read from
  register entries marked provisional until M1, no function returns them as literals, and the PR's survey lists every
  function in `crates/{kernel,ledger,engine}` that returns a numeric literal, each moved into the register or named
  in its doc as ledger or interface data (REQ-SYS-082).
- Review checklist (physics): the proposal states whether `d_min`'s floor at 2⁻¹⁴ is intended given storage down to
  2⁻²⁴ (R-271), with the evidence above; the M1 gate gives it to the human (REQ-TOOL-160; R-427).

## Notes
- Split from TASK-M1-17 (applied per R-369; physics review 5482528309 of PR #194, F1). RQ-271 held REQ-TOOL-160:
  `rho_ratio`'s and `r_min_pair_0`'s formulas are REQ-PAY-088's (TASK-M2-03, M2). R-419 (11 Oct 2026) rules it,
  option (c) for both: the two floors are proposed here provisionally and re-confirmed at M2, so this task no longer
  waits. Until this task merges, the six views keep TASK-M1-09's
  built ε, 2⁻²⁴.
- REQ-TOOL-160 moved from TASK-M1-09, which built the single ε R-401 replaces, to TASK-M1-17, and from there to this
  task. R-403 confirmed R-401's absolute floors for `dE_max` and `dLz_max`.
- The qa tests of TASK-M1-09 that pin the single ε (for example `crates/ledger/tests/qa_TASK-M1-09.rs` and
  `crates/render/tests/qa_TASK-M1-09.rs`) are ruling-forced changes (R-401): qa makes them, and the PR lists each with
  the assertion it changes and the ruling that forces it (R-290, R-369).
- TASK-M7-05 depends on this task: its `log` compaction takes "each field's ε (its ledger `floor`, R-263, or its R-401
  display floor)".
- R-427 (13 Oct 2026) vetoes this task's first form (PR #204), its six floors as `pub const fn`s in
  `crates/ledger/src/gen/numeric.rs`, made so because `cargo xtask lint constants` refuses a numeric `const` there not
  read from the register and the register could hold no unconfirmed number. Applied per R-369: the register's
  `provisional` key is built here, not by a separate task, since this task is its first consumer and the M1 gate needs
  it; `prelude::hatch()`, the precedent PR #204 cited, is the same pattern and moves with the floors. The task's scope
  grows by REQ-SYS-081 and REQ-SYS-082.
- The human's "marked provisional until the M1 gate" and R-419's "confirmed provisionally at M1 and re-confirmed at M2"
  are read together (R-427, flagged to the human): all six floors are provisional until M1; at the M1 gate the four
  others are confirmed and `rho_ratio`'s and `r_min_pair_0`'s are re-marked provisional until M2.
