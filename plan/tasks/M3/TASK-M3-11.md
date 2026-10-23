# TASK-M3-11 — The live shape readout, the lagged n̂ register and the closure fields

- **Milestone:** M3
- **Closes:** REQ-INT-042, REQ-PAY-043, REQ-PAY-055, REQ-PAY-056, REQ-PAY-059, REQ-VAL-055, REQ-INT-082, REQ-INT-087, REQ-INT-088, REQ-INT-089
- **Depends on:** TASK-M3-04, TASK-M3-07
- **Needs (earlier milestones):** REQ-INT-001, REQ-INT-003, REQ-CHART-036, REQ-PAY-009, REQ-GEN-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~600 lines

## Goal
Each macro-step the shape vector `n = (u, v, w)/I` is derived live from positions only (mass-weighted Jacobi), with no checkpoint array. The escape window's `n̂` from one window earlier is held in a lagged register updated at the occupant's sampling points (macro-step boundaries unregularised, sync boundaries regularised), and the SimState widths include it. `closure_min`/`closure_step` track the running minimum of `|n̂(t) − n̂(0)|` after the shape first departs from `n̂(0)` by more than `δ_dep`, departure latched per sample; `δ_dep` is set by a recorded measurement and the departed bit's placement is written into the ledger (R-37). θ̃ runs in the march with R-389's pole hold kept in `SimState` alone (R-397): the frozen reference in `_reserved` as a u16 code in steps of 2π/65535, 0xFFFF for "no reference", "inside the hold" recomputed from the current state, so a march's θ̃ is bit-identical however it is split into dispatches.

## References
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"
- `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_generation_root.md` § "3.5 The live-state block (replaces the checkpoint array — lockstep, ratified)"
- `docs/design/principia_dd_validation_orbits.md` § "6. The closure field — from validation to discovery"
- `decisions.md` § "R-37 — `t_min` is a departure threshold on the shape sphere *(PL-2 (b))*"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `open-questions.md` § "Pending-changes register"
- `decisions.md` § "R-29 — The escape criterion's undefined parts *(IE-1, amended)*"
- `decisions.md` § "R-95 — After escape fires *(closes RQ-48, in part)*"
- `docs/read_first/principia_01_pitfalls.md` § "2.2 The criterion"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-389 — `θ̃` starts at 0, and below a pole radius `r_pole` it holds with a frozen reference, adding the wrapped exit-minus-entry longitude on exit *(closes RQ-223)*"
- `decisions.md` § "R-392 — An IC that starts inside `θ̃`'s pole radius adds no delta at its first exit; `θ̃` counts from the exit longitude *(closes RQ-225)*"
- `decisions.md` § "R-397 — `θ̃`'s frozen pole reference is stored in `_reserved` as a u16, with 0xFFFF for none; `SimState`'s size is unchanged *(closes RQ-226)*"
- `docs/design/principia_dd_simstate_payload.md` § "`_reserved` (u16) — `θ̃`'s frozen pole reference (R-397)"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/contracts/principia_parity_contract.md` § "Tier B — integer-exact *given the same branch decisions* (integer & packed fields)"
- `decisions.md` § "R-580 — dd_integrator §5 test 8 drops "`retrograde` matches the L_z sign": the mirror leaves `θ̃` and `retrograde` unchanged and negates `L_z`; the test keeps `θ̃` continuous and `orbit_count` against a hand count, and gains the mirror pair, `θ̃(t)` and `retrograde` bitwise identical and `L_z` opposite *(closes RQ-537; changes dd_integrator §5 test 8 and REQ-INT-082)*"

## Deliverables
- `crates/kernel/src/driver/shape.rs` — `shape(r, masses)` (no momenta), per-macro-step readout.
- Ledger edits in `crates/ledger`: the lagged `n̂` register row and the departed bit (schema-version change); payload §1 width totals recomputed in `docs/design/principia_dd_simstate_payload.md`.
- `crates/kernel/src/driver/closure.rs` — `closure_min`/`closure_step` registers, departure gate.
- θ̃ in the march (R-397): the reference's encode and decode in `crates/kernel` (the step count and sentinel read from the register); the march's pole hold reading and writing `_reserved`, recomputing the inside test from the current state; ledger edits in `crates/ledger`: the register entries `theta_ref_steps` and `theta_ref_none`, added to the hashed stored-bits entries (schema-version change), and `_reserved`'s meaning in its declaration's comment.
- `crates/validation/src/measure/delta_dep.rs` + report `fixtures/gates/delta-dep/REPORT.md` — `|n̂(t) − n̂(0)|` distributions on periodic-orbit and generic fixtures.

## Acceptance tests
- `cargo test -p kernel shape_identities` — dd test 9: ‖n‖ = 1; equilateral → n_w = ±1; collinear → n_w = 0; n agrees with the IC Inspector's shapePoint on 5,000 random ICs to f64 round-off after the mirror fold (REQ-INT-042).
- `cargo test -p kernel theta_unwrap_real_orbit` — dd test 8: on a circulating bounded orbit θ̃ has no 2π jumps and orbit_count matches a hand count; the run from Π·x, Π (y → −y) applied to every position and momentum of the orbit's initial state, has θ̃(t) and retrograde bitwise identical to the run from x at every sampled step, and L_z opposite (REQ-INT-082, R-580). *Was:* "orbit_count matches a hand count and retrograde matches the L_z sign".
- `cargo test -p kernel closure_registers` — on a fixture trajectory closure_min matches an offline minimum over the stored shape path and closure_step equals the argmin step (REQ-PAY-043).
- Review (physics): the shape helper's signature takes r (and masses), not p (REQ-PAY-055).
- `cargo test -p kernel closure_departure` — a periodic orbit's closure_min ≈ 0 at closure_step ≈ period/dt_macro; closure before departure is ignored (REQ-PAY-056).
- Review (physics): the SimState layout table lists the lagged n̂ register and payload §1's width totals include it; `cargo test -p kernel lagged_nhat_sampling` — the register updates only at the occupant's sampling points (macro-step or sync boundaries) (REQ-PAY-059).
- `cargo test -p kernel theta_ref_code` — every code round-trips, `encode(decode(c)) = c`; the encode's error is at most π/65535 + 4·ulp_Real(2π) (+1.9e-6 at f32, +3.6e-15 at f64), whatever the evaluation order, and never yields 0xFFFF, and a truncating encode (no `+ ½`) fails that bound; decode lies in (−π, π) (REQ-INT-087).
- Review (physics): the sim-parity comparison treats `_reserved` as Tier N/S with θ̃ (one code apart allowed given the same entry state, mod 65535), never as Tier B; sentinel-or-code is exact across backends at the same `Real` and may differ at the disc edge across precisions; the one-code allowance needs REQ-INT-086's `r_pole` above about 2e-3 (REQ-INT-087; parity contract Tier B).
- `cargo test -p ledger theta_ref_register` — `theta_ref_steps` and `theta_ref_none` are register entries hashed into the schema version; changing either's value changes it (REQ-INT-087).
- `cargo test -p kernel theta_hold_state` — a fresh sample's `_reserved` is 0xFFFF; it is 0xFFFF outside the disc after every step; entry writes the code of the last longitude outside; exit adds `wrap(exit − decode(code))`, ±π adding +π, and writes 0xFFFF; an IC inside adds nothing at its first exit; the inside test is recomputed from the current state; the hold writes no descriptor bit: bits 10–14 stay zero and bit 15 is the departed bit's (REQ-VAL-055, R-545), unchanged by the hold (REQ-INT-088). *Was:* "descriptor bits 10–15 stay zero" (fix round 1 of PR #274, qa Q3).
- `cargo test -p kernel theta_resume_split` — one call against k ∈ {2, 3, 7} resumed calls at f32 and f64, split on a pole passage's entry step, inside it and on its exit step, and inside the disc for an IC that starts there: θ̃'s bits and `_reserved` are identical; a negative control holding the reference unquantised within a dispatch fails (REQ-INT-089).
- Review (physics): the alignment recheck (REQ-PAY-009) for `_reserved` and for each member this task adds (the lagged `n̂` register): sizes and offsets stated, and payload §1's totals recomputed.
- `cargo xtask gate delta-dep` — measurement report of |n̂(t) − n̂(0)| distributions on periodic-orbit and generic fixtures justifying δ_dep; the ledger row for the departed bit exists and the schema version changes (REQ-VAL-055).

## Notes
- qa exceptions forced by R-545 (RQ-310: bit 15 is R-37's departed bit, which this task writes for REQ-VAL-055), named per
  R-290 and R-369, each an `M` line on a qa file whose commits are all qa's: `crates/ledger/tests/qa_TASK-M0-10.rs:106-155`
  (`reserved_bits_qa_ledger_reserves_exactly_10_to_15`: bit 15 leaves the reserved span, which becomes 10–14);
  `crates/kernel/tests/qa_TASK-M0-10.rs:128-150` (`PackA`'s packer and "bits 10–15 are zero": bit 15 carries the
  departed field); and, if the packer's change reaches it, `crates/kernel/tests/qa_TASK-M0-10_r288.rs:223-289` (the
  reserved bits kept through `set_d_min`). TASK-M7-35 narrows the same two spans again for bits 10–14. The code reviewer
  confirms no other assertion is weakened.
- qa-file exceptions further to the above (qa re-check 5606819712 of PR #274, Q10; each span checked against the tree),
  forced by R-545 (RQ-310: bit 15 the departed bit):
  - `crates/kernel/tests/qa_TASK-M0-10.rs:333-371` (`reserved_bits_qa_zero_with_every_field_at_max`; `:359` asserts
    `PACKED_A_RESERVED == [(10, 6)]`), an `M` line under R-290, its commits all qa's.
  - `crates/ledger/tests/qa_TASK-M0-09.rs:172-179` (`check_packed`: `packed_a`'s reserved spans exactly `[(10, 6)]`).
    The file has implementer commits (382c9de2, 8952af9d, 7b3725ee beside qa's 2c567619), so R-290's qa-only rule does
    not cover it: it is an exception applied per R-369, the ruling forcing the change.
  - `crates/ledger/tests/qa_TASK-M1-08.rs:460-489` (`with_qa_cat`: a test field at bits 10–11 and the reserved span
    narrowed to 12–15; once bit 15 is a field the span overlaps it and `check::check`, `crates/ledger/src/gen/mod.rs:272`,
    panics), an `M` line under R-290, its commits all qa's.
  - If the packer's change reaches it (a departed argument to `pack_packed_a`, or a setter that writes the bit):
    `crates/kernel/tests/qa_TASK-M0-11.rs:239-337` (`qa_payload_sizes_descriptor_bits_10_to_15_never_written` and its
    controls, one reading "a packer that spends bit 15 must fail"), an `M` line under R-290, its commits all qa's.
- R-113 (RQ-94): REQ-INT-001's dd test 8 on a real orbit is split off as REQ-INT-082 and closed here; the synthetic-path half stays in TASK-M1-11.
- R-580 (RQ-537, 23 Oct 2026): dd test 8's "retrograde matches the L_z sign" is dropped; the mirror leaves θ̃ and
  retrograde unchanged and negates L_z, so it cannot hold for both members of a mirror pair. The mirror-pair arm takes
  its place, bitwise where the integration is mirror-exact (R-544 §4); if the stepper the march runs is not
  mirror-exact, stop and queue rather than loosen "bitwise" to a tolerance (applied per R-369, as TASK-M3-38).
- δ_dep must be relative or gap-set (pitfalls §3, 'A threshold on a quantity spanning decades must be relative'). The value is recorded with its measurement; R-37 says the value is set by measurement.
- R-397 (RQ-226), applied per R-369: TASK-M3-11 owns the storage of θ̃'s pole hold as well as θ̃ in the march, since it already edits the ledger with a schema-version change (REQ-VAL-055, REQ-PAY-059); no separate ledger task. The member keeps its name `_reserved` (R-343 and R-351 name its WGSL home `closure_step_reserved`), has no field entry, and gets no read-side accessor or catalogue view. The bring-up pattern keeps `_reserved = 0` (colour_composition Appendix A).
- R-397's quantisation applies wherever the march stores the reference: the hold's helper in `crates/kernel/src/shape.rs` (TASK-M1-11) may be kept, with the march quantising on entry, or changed. If the march quantises on entry and keeps the helper, TASK-M1-11's qa tests stay untouched. If this task makes the helper quantise, `crates/kernel/tests/qa_TASK-M1-11.rs` (the unquantised hold rule at :84–94) and `crates/kernel/tests/qa_TASK-M1-11_config.rs` (`expected` at :55–69, the f64 tolerance from :155, `edge_check!(edge64, f64, 1e-12)` at :504) break, since they compare `theta_step` and `theta_step_config` with the unquantised hold at tolerances far below π/65535; those are qa-file changes the PR lists and justifies (R-290), conditional on that design choice, not forced by the ruling.
- `crates/ledger/tests/qa_TASK-M0-12.rs:447–451` (`SECTION_3_8_HASHED`, and "five" in the module doc at :8) iterates its own five names and still passes with seven `STORED_BITS`; nothing forces a change. Its only commit is qa's (`f4cb7e8`), so extending it to seven is an ordinary R-290 `M` by qa with its reason. The implementer's own `crates/ledger/tests/schema_version.rs:512–514` passes at 7 = 7; this task updates its stale "today all five register entries are hashed" message.
- R-433 (15 Oct 2026, the M1 gate): `r_pole` = 0.01 is confirmed and is the constants register's `r_pole`
  (`crates/ledger/src/constants.rs`), marked provisional until M3 and citing REQ-INT-086; the march reads it from
  there, never as a literal. M3 re-confirms it against real orbits' near-pole arcs: this task's PR reports, for the
  real orbits it marches (REQ-INT-082), the arc each step covers near the pole disc against both `π·r_pole` and
  `(π/2)·r_pole`, as physics review 5436669140 of PR #158 asked, and the M3 gate gives the human that evidence;
  `cargo xtask gate-provisional --milestone M3` lists `r_pole` until the human re-confirms it. `r_pole` is not hashed
  (it decides which value `θ̃` and `_reserved` take, not what their bits mean, dd_generation_root §3.8 "The hash").
  Applied per R-369: `r_pole` is a payload-shaping wrapper parameter (integrator contract Part 3, "2. Wrapper
  config"; caching contract Part 2), so it is on the sim key whatever form this task gives it, and a change at the M3
  gate re-integrates and never serves payloads computed under the old value; the §3.8 hash text stands.
- The lagged `n̂` register (REQ-PAY-059) is a new SimState member after R-397's recheck: it costs 8 B per 8 B or less of its width (no free tail remains at f32), and the departed bit's placement does not use `_reserved`.
