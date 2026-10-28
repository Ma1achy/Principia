# Style guide for Gemini Code Assist

`AGENTS.md` § "Code Review Rules" is canonical. Below is a copy of that section; `xtask/tests/ai_review_rules.rs` fails
if it differs from `AGENTS.md`'s by a byte, so edit `AGENTS.md` and copy the section here.

## Code Review Rules

The brief for the advisory AI reviewers, Gemini Code Assist and Codex (`decisions.md` R-635). `.gemini/styleguide.md`
holds a byte-identical copy, checked by `xtask/tests/ai_review_rules.rs`. No finding here gates a merge.

### What review is for

- Physics against the reference implementation, prin-rs, as the contracts transcribe it. Check against the contracts,
  never against `docs/reference/` itself, which is a transcription source, not authority (R-159).
- The Aarseth–Zare and Heggie algebra transcribed correctly: `docs/contracts/principia_integrator_contract.md`
  Part 2b, `docs/design/principia_dd_integrator.md` §3.2.
- The outcome encoding, `state` and `detail`: `docs/design/principia_dd_simstate_payload.md` §2,
  `crates/ledger/src/display.rs`.
- Invariants quietly broken: `docs/contracts/principia_canonical_spec.md` §9, `plan/reviewers/physics.md`.
- Not Rust idiom or style: rustc, rustfmt and clippy with `-D warnings` cover those, in CI. Don't comment on them.

### Severity

- P0 / CRITICAL: a physics or invariant break, every non-negotiable below among them.
- P1 / HIGH: any correctness bug: a wrong result, a crash, a panic, a NaN path, an off-by-one, a wrong index, a race.
- P1 / HIGH: a test with no control arm, and every item under "Flag hard" and "Docs and plan consistency".
- Below the threshold, not posted: style and naming, and nothing else.

### Non-negotiables (P0 / CRITICAL)

- `error_ratio` is `sigma_E(t) / sigma_E(0)` with `sigma_E` the max deviation (never MAD), NaN-safe, aggregated over
  footprints by `max`: `docs/design/principia_dd_generation_root.md` §3.7 "Ensemble spread" (REQ-REF-003).
- `spread_event` is the disagreement over the event class, `class ⊕ detail`, over its attainable maximum
  `1 − 1/(E+1)`: dd_generation_root §3.7 "Ensemble spread" (REQ-REF-002), `decisions.md` R-18.
- Never discard an ensemble copy: every footprint keeps all `E+1` copies, a failed copy raises spread, and no drift
  filter or "failed" exclusion removes one: dd_generation_root §3.7 "Ensemble spread" (REQ-REF-004),
  `docs/read_first/principia_00_philosophy.md` §4.3.
- `r_coll` and `epsilon` are in canonical units, fixed at t = 0, never co-moving; ε > 0 runs are tagged and never
  pooled with ε = 0 runs: dd_generation_root §3.7 "Open" (REQ-SYS-018).
- Count the pairs below `r_coll` before classifying: one pair is a collision with that pair's `detail`; two or more
  is a triple collision, `detail = 3`: integrator contract Part 7, dd_integrator §3.6 (REQ-EVT-003).
- Test `is_finite` explicitly: every comparison with NaN is false, so a guard written as a comparison never fires
  (a NaN step size hangs the loop): `decisions.md` R-538 (REQ-INT-096).
- No `filter(is_finite)`, or any other skip of non-finite samples, in a reduction: it is a silent discard, and a quad
  where nothing integrated reports `Undetermined`: canonical spec §9 item 9 (Totality), `decisions.md` R-47
  (REQ-VAL-080), `crates/ledger/src/reduction.rs`.
- The f32 and f64 floors are separate: one `TINY` per working precision, never one shared constant; `TINY·TINY`
  underflows at f32, and a floored step is counted: integrator contract Part 3 and Part 6 (REQ-INT-105).
- No constant that differs between the f32 and f64 instantiations in a control-flow decision: integrator contract
  Part 2c, "Rules the new kernel must hold by construction".
- No fixed absolute tolerance in scale-invariant code: a threshold is relative, in canonical units, or bounded by its
  own attainable maximum: philosophy §4.2, `docs/read_first/principia_01_pitfalls.md` §3.

### Flag hard (P1 / HIGH)

- A test that cannot fail: no control arm, no `negative_control!` registered for it (`crates/validation/src/control.rs`;
  philosophy §4.4; `decisions.md` R-176).
- A conclusion read off a median alone: a median survives by ignoring the broken footprints; read the max or the
  distribution (`docs/design/principia_dd_predictability_horizon.md` §7.4).
- A hardcoded argument or output root: a path, directory, seed or size baked in where the caller should pass it.
- A new variant of an outcome enum missing from its exhaustive list: `State::ALL` and `Detail::ALL` in
  `crates/ledger/src/display.rs`; the refinement `Decision` (`docs/design/principia_dd_refinement_policy.md` §6) and
  its list, once built.
- Renders written into `results/`.

### Docs and plan consistency (P1 / HIGH)

- Code must do what the requirement or contract it cites says (REQ-*, TASK-*, contract sections); flag any mismatch:
  `plan/requirements.yaml`, `plan/tasks/`, `docs/contracts/`.
- A cited id (REQ, TASK, R-, RQ-) that doesn't exist, or whose text says something else: `plan/requirements.yaml`,
  `plan/tasks.yaml`, `decisions.md`, `REVIEW_QUEUE.md`, `docs/archive/review_queue/`.
- A doc or plan edit that contradicts another part of the same file, or a doc it cites.
- A change to behaviour that leaves its doc saying the old thing (doc drift).
- `docs/incoming/` is a record, not authority (R-1): never flag drift there. Nor in `workbench/`, `docs/archive/` or
  `docs/reference/`, which are never normative (R-159).

### How a finding is answered

- Every finding is verified as `CLAUDE.md`'s review rules require: fixed, or answered with the traced reason.
- Two reviewers agreeing is not verification.
