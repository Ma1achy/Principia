# TASK-M7-35 — The Chazy classifier: the rule, the Fewbody reference, the ten fixtures and the I_max latch

- **Milestone:** M7
- **Closes:** REQ-PAY-099, REQ-PAY-105, REQ-PAY-106, REQ-EVT-028, REQ-EVT-029, REQ-EVT-030, REQ-EVT-031, REQ-VAL-184, REQ-VAL-187
- **Depends on:** TASK-M3-10, TASK-M3-11, TASK-M3-17, TASK-M3-19
- **Needs (earlier milestones):** REQ-PAY-041, REQ-PAY-044, REQ-EVT-001, REQ-SYS-017
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10, PIT-3
- **Size:** ~500 lines (the classifier and its two-body helpers ~250, the Fewbody reference ~70, the latch ~80, the ten fixtures as the implementer's tests ~100)

## Goal
Port Chazy's final-motion classifier as the Chazy drill-down transcribes it from prin-rs PR #32 (R-442), not re-derived:
`chazy_class`, a read-side accessor derived at read, labels each sample and each ensemble copy at every playhead with
one of ten labels (H, HP, P, HE, PE, B-stable, B-so-far, transient, OS-candidate, excursion) by §3.2's rule in its
order, from the live state, the masses, `E_0`, `R_0 = √(I_0/M)` and the latch `I_max`, with §3.1's two-body `K` and `U`,
the parabolic band, the hierarchy ratio, §3.4's sign test for a pair's binding, `coarse`, §3.5's Mardling–Aarseth test,
§3.3's defaults and §3.6's tidal gate and settling as options. `I_max` is the running maximum of `I`, forward-only,
updated every step, per sample and per copy (§6). The Fewbody reference (§3.7) goes into the validation crate as the
reference, never a candidate. The ten fixtures of §12.1 are ported with their control arms, and the four classifier
parameters and the latch's quantisation are proposed with their evidence for the M7 gate (R-71).

## References
- `docs/design/principia_dd_chazy_final_motion.md` § "3.1 Inputs and definitions"
- `docs/design/principia_dd_chazy_final_motion.md` § "3.2 The rule, in order"
- `docs/design/principia_dd_chazy_final_motion.md` § "3.3 Defaults"
- `docs/design/principia_dd_chazy_final_motion.md` § "3.4 Two choices that are physics, not tuning"
- `docs/design/principia_dd_chazy_final_motion.md` § "3.5 Mardling–Aarseth"
- `docs/design/principia_dd_chazy_final_motion.md` § "3.6 The options the fixtures exercise"
- `docs/design/principia_dd_chazy_final_motion.md` § "3.7 The Fewbody reference"
- `docs/design/principia_dd_chazy_final_motion.md` § "3.8 The labels: ten"
- `docs/design/principia_dd_chazy_final_motion.md` § "4.1 The probe"
- `docs/design/principia_dd_chazy_final_motion.md` § "5.1 Derived, not stored"
- `docs/design/principia_dd_chazy_final_motion.md` § "6. The latch: `I_max`"
- `docs/design/principia_dd_chazy_final_motion.md` § "11.2 Calibrations to confirm at a gate (R-71)"
- `docs/design/principia_dd_chazy_final_motion.md` § "12.1 The fixtures to port"
- `docs/design/principia_dd_chazy_final_motion.md` § "12.2 New, for Principia"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `decisions.md` § "R-442 — Chazy final motion is a separate system with its own stains: hue carries the class, B is black `#141418` and one class, collision is magenta `#DA85B6`, transient draws as B, the working states have their own status map, and the seven-class palette is the default"
- `decisions.md` § "R-548 — The Chazy entries ruled: the seven-class palette stays the default; the subtitle ban covers captions over the figure only; the class freezes at escape fire and may differ from the escape event, the legend saying so; the class is a label, reported, not asserted; OS-candidate and B-stable ship marked provisional; B `#141418` and the grey `#8A8A90` are near-neutral, held to chroma ≤ 0.009; the rule keeps transient before B-stable, puts B-stable before excursion and lets H, HP, P and the two-stage H fire only at E ≥ 0; a render-side previous-class texture; the one-body views read `chazy_escaper`; the three panels are one figure *(closes RQ-311 to RQ-318 and RQ-320 to RQ-325)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-595 — Under R-548's `E ≥ 0` guard an E < 0 state that passes the dispersal step's geometric tests goes on to the hierarchy step, where it may read HE, PE, B-stable or excursion: the guard is for dispersal (H, HP, P), and a binary plus an escaper is the ordinary E < 0 escape *(closes RQ-521; corrects R-548's "falls through to …" list)*"

## Deliverables
- The classifier in the shared kernel source (generic `Real`, so CPU-f64 and GPU-f32 instantiate one rule), with the
  two-body helpers `pair_kup`, `out_kup` and `kepler`, the band, the hierarchy ratio, `coarse` and `ma_stable`, its
  parameters constants-register entries marked provisional until the M7 gate (R-427), and its `chazy_class` read-side
  accessor generated beside `energy_drift`.
- The `I_max` latch in the march: local per step, per sample and per copy, starting at `I_0`; persisted in the payload
  in descriptor bits 10–14 (R-545, RQ-310), log-quantised in half-octaves with REQ-PAY-106's quantisation, a ledger
  edit that changes the schema version, the generated layout test updated as `last_symbol`'s was.
- The Fewbody reference (`fewbody_resolved`, three-body) in `crates/validation`.
- The ten fixtures of §12.1 as tests, each with its control arm, and §12.2 items 10 and 11 (R-548: the receding
  stable hierarchy and the homothetic E < 0 expansion, their states written here); the live-playhead property test.
- The calibration proposals for ε, `h_far` (with the gate and δ), `os_ratio`, settling τ and the latch's quantisation,
  with the drill-down's evidence, in the PR.

## Acceptance tests
- `cargo test -p kernel chazy_fixtures` — the ten §12.1 fixtures with their control arms, on CPU-f64: HE; PE at outer `K/U = 1`, excursion at 0.5, and `coarse`; HE for the `e = 0.95` pair, its control inside the band with `K < U`; the two-stage H; H for three flying apart (E > 0); transient at E > 0 and B-so-far at rest; B-so-far with a fresh latch and OS-candidate at `I_max = 2·os_ratio·I`; B-stable with Mardling–Aarseth on, B-so-far off; the tidal gate's HE at d = 50, not at 20, the hierarchy gate committing on both; settling withholding HE when the shape moved; §12.2 item 10, B-stable for a stable hierarchy whose outer body is past `h_far` and receding (`v_r ≠ 0`), control: the code's order reads excursion; §12.2 item 11, B-so-far for the homothetic expansion with `E = −0.1 U₀` at 2.9 and 3.2–3.5 times its initial size, control: without the `E ≥ 0` guard it reads H, then P (R-548); R-595's arm, an E < 0 state passing step 1's geometric tests, with a bound pair in the parabolic band and the hierarchy gate passing, reads HE or PE from step 2, control: a classifier that skips step 2 for a guarded state reads B-so-far or OS-candidate and fails. Every output is one of the ten labels (REQ-PAY-099).
- `cargo test -p validation chazy_fewbody` — the reference arms: H for three flying apart, B-stable for the hierarchy (`ma_stable` true), HE at d = 50 and unresolved at d = 20 (tidal ratios 8 × 10⁻⁶ and 1.25 × 10⁻⁴), δ = 10⁻⁵ (REQ-VAL-184).
- `cargo test -p kernel chazy_i_max_latch` — the latch equals the offline maximum of `I` over a fixture trajectory at every step, never decreases, is `I_0` at step 0, survives persist boundaries as one uninterrupted latch, and differs between two ensemble copies of one sample (REQ-PAY-105).
- `cargo test -p kernel chazy_live_playhead` — property test: for random ICs the label at `t` from a run stopped at `t` equals the label at `t` from the run continued; control: a variant reading the state at `t + Δ` fails (REQ-VAL-187).
- Review (physics): the proposals for ε = 0.05 (REQ-EVT-028), `h_far` = 5 with the hierarchy gate and δ = 10⁻⁵ (REQ-EVT-029), `os_ratio` = 25 measured at the snapshot cadence while the latch updates every step (REQ-EVT-030), settling off (REQ-EVT-031) each with the drill-down's §4.1 and §11.2 evidence, confirmed by the human at the M7 gate; and the latch's log half-octave 5-bit quantisation as ruled (R-545), with its measured effect on the OS-candidate labels (REQ-PAY-106).

## Notes
- qa exceptions forced by R-545 (RQ-310: `I_max` takes descriptor bits 10–14), named per R-290 and R-369, each an `M`
  line on a qa file whose commits are all qa's: `crates/ledger/tests/qa_TASK-M0-10.rs:106-155` (the reserved span, by then
  10–14 after TASK-M3-11, is spent) and `crates/kernel/tests/qa_TASK-M0-10.rs:128-150` (the packer's descriptor bits);
  and `crates/kernel/tests/qa_TASK-M0-10_r288.rs:223-289` if `set_d_min`'s kept bits change. The code reviewer confirms
  no other assertion is weakened.
- qa-file exceptions further to the above (qa re-check 5606819712 of PR #274, Q10; each span checked against the tree),
  forced by R-545 (RQ-310: `I_max` takes bits 10–14):
  - `crates/kernel/tests/qa_TASK-M0-10.rs:333-371` (`reserved_bits_qa_zero_with_every_field_at_max`; `:359` asserts
    `PACKED_A_RESERVED`, by then `[(10, 5)]` after TASK-M3-11), an `M` line under R-290, its commits all qa's.
  - `crates/ledger/tests/qa_TASK-M0-09.rs:172-179` (`check_packed`: `packed_a`'s reserved spans).
    The file has implementer commits (382c9de2, 8952af9d, 7b3725ee beside qa's 2c567619), so R-290's qa-only rule does
    not cover it: it is an exception applied per R-369, the ruling forcing the change.
  - `crates/ledger/tests/qa_TASK-M1-08.rs:460-489` (`with_qa_cat`: a test field at bits 10–11 and the reserved span
    narrowed to 12–15; once bits 10–14 are `I_max` the field overlaps it and `check::check`, `crates/ledger/src/gen/mod.rs:272`,
    panics), an `M` line under R-290, its commits all qa's.
  - If the packer's change reaches it (an `I_max` argument to `pack_packed_a`, or a setter that writes bits 10–14):
    `crates/kernel/tests/qa_TASK-M0-11.rs:239-337` (`qa_payload_sizes_descriptor_bits_10_to_15_never_written` and its
    controls, one reading "a packer that spends bit 15 must fail"), an `M` line under R-290, its commits all qa's.
- R-548 (21 Oct 2026) rules the order, as the drill-down's §3.2 now gives it: transient before B-stable, B-stable
  requiring E < 0 (RQ-318, option (c)); B-stable before excursion (RQ-324, option (b)); H, HP, P and the two-stage H
  only at E ≥ 0 (RQ-325, option (b)). This is `classify`'s order with three changes, so prin-rs PR #32's measurements
  (the drill-down's §4) are the code's, kept as the record. Two fixtures are added (§12.2 items 10 and 11), their
  states written here under physics review. R-595 (25 Oct 2026, RQ-521 (a)): an E < 0 state that passes the dispersal step's
  geometric tests goes on to the hierarchy step and may read HE, PE, B-stable or excursion; no ported fixture reaches
  it, so its arm's state is written here under physics review. *Was:* "Whether an E < 0 state that passes the
  dispersal step's geometric tests may read HE, PE or excursion in the hierarchy step is RQ-521; REQ-PAY-099 waits on it
  for that clause only, and no fixture reaches it."
  *Was:* "The rule's order differs between the spec's table and the code in two places: … REQ-PAY-099 waits on the
  rulings for those clauses only. The fixtures reach neither case." and "`classify` enforces the E > 0 certificate by
  its order but never tests E in the dispersal step or the two-stage H; whether to guard them with E ≥ 0 is RQ-325, and
  REQ-PAY-099 waits on it for those clauses (drill-down §2)."
- Where `I_max` is stored is ruled (R-545, RQ-310): descriptor bits 10–14, log-quantised in half-octaves (32 levels to
  2¹⁶). REQ-PAY-105 and REQ-PAY-106 no longer wait. *Was:* "Where `I_max` is stored is RQ-310 (for the human). Until it
  rules, the latch is local and per step, and the persistence test runs on the latch's local value; REQ-PAY-105 and
  REQ-PAY-106 wait on it."
- Outside the fixtures `R_0` is computed from the initial state; the fixtures pass `R_0 = 1`, as written (§12.1).
- Applied per R-369: the classifier's code placement (shared kernel source, generated accessor) is this task's; the
  accessor must read only the live state, the masses, `E_0`, `R_0` and `I_max`.
