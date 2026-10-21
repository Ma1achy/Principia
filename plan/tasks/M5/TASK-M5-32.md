# TASK-M5-32 — `running_mean_divergence` and `coherence`: formulas and one-signed declared ranges (R-545, RQ-278)

- **Milestone:** M5
- **Closes:** REQ-PAY-109
- **Depends on:** TASK-M1-21, TASK-M5-01
- **Needs (earlier milestones):** REQ-GEN-035
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~150 lines

## Goal
**R-545** rules RQ-278: `running_mean_divergence` and `coherence` are one-signed. "TASK-M1-21 (R-72) gives each a
formula and declares a range that does not span zero, so both stay on viridis." TASK-M1-21 merged (PR #208) before the
ruling, so this task is its follow-up (R-545 A1, applied per R-369): it writes each member's formula into
dd_generation_root §3.7, and declares its range, with its citation, in the ledger (`crates/ledger`), replacing the
`(−∞, ∞)` PR #208 built for both. Neither range spans zero, so the numeric template keeps both `lin` on viridis.

## References
- `decisions.md` § "R-545 — The human's batch of 21 Oct 2026, ported by area as R-545 to R-559; and the ledger, payload and schema: `running_mean_divergence` and `coherence` one-signed, `alpha_energy` centred on 1.0, a substeps-per-pixel histogram in the frame record, a schema version that hashes only layout and kernel columns, and descriptor bits 10–14 for `I_max` with bit 15 R-37's departed bit *(closes RQ-278, RQ-286, RQ-297 and RQ-310; amends R-36 and R-56)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-426 — `q_mass` and `mean_y` stay on cool-warm provisionally, recorded as provisional at the M1 gate, and are re-checked when TASK-M2-03 and TASK-M3-13 declare their ranges; an undeclared range is a ledger gap, not a range that spans zero *(closes RQ-274; amends R-400)*"
- `docs/design/principia_dd_generation_root.md` § "Layout and §3.8 entries"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"

## Deliverables
- dd_generation_root §3.7: `running_mean_divergence`'s formula (the running mean of the per-footprint divergence whose
  latched running maximum is `running_max_divergence`) and `coherence`'s (complexity = 1 − coherence, scheduler
  contract Part 6, "Priority"), each with its declared range, which does not span zero, and its citation; the two table rows
  conformed and the "waiting on" mark removed.
- `crates/ledger`: both entries declared with those ranges and citations; the numeric template places neither on
  `ramp_coolwarm`.

## Acceptance tests
- `cargo test -p ledger declared_ranges` — `running_mean_divergence` and `coherence` are declared with ranges that do
  not span zero, each citing §3.7's formula, and both sit on viridis; control: a fixture entry declared `(−∞, ∞)` `lin`
  is placed on `ramp_coolwarm` (REQ-PAY-109).
- Review checklist (physics) — each formula and range against the corpus's uses (dd_generation_root §3.7, scheduler
  contract Part 6, "Priority"), approved before merge (REQ-PAY-109, R-72).

## Notes
- Applied per R-369 (R-545 A1): the human names TASK-M1-21, which merged before the ruling, in the M1 milestone whose
  gate has passed (R-432); the work is this follow-up's, placed at M5 beside REQ-PAY-093's confirmation.
- `alpha_energy` is not this task's: its range and placement are confirmed with REQ-PAY-093 at the M5 gate.
