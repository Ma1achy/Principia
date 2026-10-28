# AGENTS.md

Authority: `CLAUDE.md`, and the `docs/` corpus with `decisions.md` overriding it (R-1); the map is `docs/read_first/principia_INDEX.md`.

## Code Review Rules

The brief for the three advisory AI reviewers, Codex, Gemini (the Gemini CLI action) and GitHub Copilot, for the trial
`decisions.md` R-635 records. `GEMINI.md` and `.github/copilot-instructions.md` hold byte-identical copies, checked by
`xtask/tests/ai_review_rules.rs`. No finding here gates a merge.

### What review is for

- Physics against the reference implementation, prin-rs, as the contracts transcribe it: check against
  `docs/contracts/` and `docs/design/`, never against `docs/reference/`, a transcription source, not authority (R-159).
- The Aarseth–Zare and Heggie algebra transcribed correctly: `docs/contracts/principia_integrator_contract.md`
  Part 2b, `docs/design/principia_dd_integrator.md` §3.2.
- The outcome encoding, `state` and `detail`: `docs/design/principia_dd_simstate_payload.md` §2,
  `crates/ledger/src/display.rs`.
- Invariants quietly broken: `docs/contracts/principia_canonical_spec.md` §9, `plan/reviewers/physics.md`.
- Correctness bugs: a wrong result, a crash, a panic, a NaN path, an off-by-one, a wrong index, a race.
- Code or docs drifting from the plan: the "Docs and plan consistency" checks below.
- Not Rust idiom or style: rustc, rustfmt and clippy with `-D warnings` cover those, in CI. Don't comment on them.

### Severity

| Finding | Severity |
| --- | --- |
| A physics or invariant break, every non-negotiable below among them | P0 / CRITICAL |
| A correctness bug | P1 / HIGH |
| A test with no control arm, and every "Flag hard" item | P1 / HIGH |
| Doc or plan drift: every "Docs and plan consistency" item | P1 / HIGH |
| Style and naming | below: not reported |

### Non-negotiables (P0 / CRITICAL)

- `error_ratio` is `sigma_E(t) / sigma_E(0)` with `sigma_E` the max deviation, aggregated over footprints by `max`:
  `docs/design/principia_dd_generation_root.md` §3.7 "Ensemble spread" (REQ-REF-003).
- `spread_event` is the disagreement over the event class, `class ⊕ detail`: dd_generation_root §3.7 "Ensemble
  spread" (REQ-REF-002), `decisions.md` R-18.
- Never discard an ensemble copy: all `E+1` copies kept, a failed copy raises spread: dd_generation_root §3.7
  "Ensemble spread" (REQ-REF-004), `docs/read_first/principia_00_philosophy.md` §4.3.
- `r_coll` and `epsilon` are canonical, fixed at t = 0, never co-moving: dd_generation_root §3.7 "Open"
  (REQ-SYS-018).
- Two or more pairs below `r_coll` is a triple collision, `detail = 3`; one pair is that pair's collision: integrator
  contract Part 7, dd_integrator §3.6 (REQ-EVT-003).
- Test `is_finite` explicitly; a comparison with NaN is false, so a guard written as one never fires: `decisions.md`
  R-538 (REQ-INT-096).
- No `filter(is_finite)`, or other skip of non-finite samples, in a reduction: a silent discard (Totality):
  canonical spec §9 item 9, `decisions.md` R-47 (REQ-VAL-080), `crates/ledger/src/reduction.rs`.
- The f32 and f64 floors are separate, one `TINY` per precision, and no constant that differs between the two decides
  control flow: integrator contract Parts 2c, 3 and 6 (REQ-INT-105).
- No fixed absolute tolerance in scale-invariant code; a threshold is relative or in canonical units: philosophy §4.2,
  `docs/read_first/principia_01_pitfalls.md` §3.

### Flag hard (P1 / HIGH)

- A test that cannot fail: no control arm, no `negative_control!` registered for it (`crates/validation/src/control.rs`;
  philosophy §4.4; `decisions.md` R-176).
- A conclusion read off a median alone: read the max or the distribution
  (`docs/design/principia_dd_predictability_horizon.md` §7.4).
- A hardcoded argument or output root: a path, directory, seed or size baked in where the caller should pass it.
- A new outcome variant missing from its exhaustive list: `State::ALL` and `Detail::ALL` in
  `crates/ledger/src/display.rs`; the refinement `Decision` (`docs/design/principia_dd_refinement_policy.md` §6) and
  its list, once built.
- Renders written into `results/`.

### Docs and plan consistency (P1 / HIGH)

- Code must do what the requirement or contract it cites says (REQ-*, TASK-*, contract sections):
  `plan/requirements.yaml`, `plan/tasks/`, `docs/contracts/`.
- A cited id (REQ, TASK, R-, RQ-) that doesn't exist, or whose text says something else: `plan/requirements.yaml`,
  `plan/tasks.yaml`, `decisions.md`, `REVIEW_QUEUE.md`, `docs/archive/review_queue/`.
- A doc or plan edit that contradicts another part of the same file, or a doc it cites.
- A change to behaviour that leaves its doc saying the old thing.
- Never flag drift in `docs/incoming/`: a record, not authority (R-1). Nor in `workbench/`, `docs/archive/` or
  `docs/reference/`, never normative (R-159).

### How a finding is answered

- Every finding is verified as `CLAUDE.md`'s review rules require: fixed, or answered with the traced reason.
- Two reviewers agreeing is not verification.
