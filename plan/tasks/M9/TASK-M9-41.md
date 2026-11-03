# TASK-M9-41 — Candidate original results: per-point FTLE maps over the batch-152 planes and a time-weighted shape-sphere occupancy accumulator (R-679)

- **Milestone:** M9
- **Closes:** REQ-VAL-344, REQ-VAL-345, REQ-VAL-346
- **Depends on:** TASK-M3-14, TASK-M4-13
- **Needs (earlier milestones):** REQ-INT-043, REQ-PAY-021, REQ-PAY-031, REQ-PAY-032, REQ-PAY-058
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
Two outputs no paper gives, planned by R-679 (RQ-674) as M9 peeks, non-gating, recorded as candidate original results, with no build before M9: per-point FTLE maps over the batch-152 planes, using the planned FTLE field, and shape-sphere residence statistics from a time-weighted occupancy accumulator. The accumulator's cell partition is defined first (R-72), then both are built and run, and each result is a row of the peek log.

## References
- `decisions.md` § "R-679 — The 3 Nov triage rulings: LL-3 takes Hénon 1974 as its independent printed source for Standish's orbit, its period agreement with Li & Liao to 2e-10 recorded, and stops waiting on Standish 1970; planetary (star-plus-planets) sources are parked with the hierarchical triples, revisited with the v2+ 3D extension, not out of scope for good; per-point FTLE maps over the batch-152 planes, from the planned FTLE field, and shape-sphere residence statistics, from a new time-weighted occupancy accumulator, are M9 peeks, non-gating, recorded as candidate original results, with no build before M9 *(closes RQ-672, RQ-673 and RQ-674; amends R-630 item 9; refines R-668 item 5)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_replication.md` § "12.5 Peeks"
- `docs/design/principia_dd_replication.md` § "12.2 Tiers"
- `docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"

## Deliverables
- The accumulator's definition in `docs/design/principia_dd_replication.md` § 12.5: the equal-area partition of the shape sphere (its scheme, and its resolution or how a result chooses it), where in a step the shape point is sampled, and the physical-time weight under every occupant's time mapping; approved by the physics reviewer before any code (REQ-VAL-346, R-72).
- The FTLE plane map in `crates/validation`: the planned FTLE field (REQ-INT-043, REQ-PAY-021, REQ-PAY-032) read at each start of a plane, at its pixel centres, at each stated horizon, invalid where `ftle_valid` is false and frozen at escape (REQ-PAY-058); no new FTLE (REQ-VAL-344).
- The occupancy accumulator in `crates/validation`, and the residence statistics over an ensemble with its sampling measure stated (REQ-VAL-345).
- The three FTLE maps and the residence statistics run, each recorded as a row of `docs/verification/peek_log.md` whose outcome reads "candidate original result", with its tier (R-649), inputs and conventions.

## Acceptance tests
- Review checklist (physics reviewer) — § 12.5's partition, sampling point and time weight are approved (REQ-VAL-346).
- `cargo test -p validation ftle_plane_map` — each pixel's map value equals the FTLE field read for that start alone, at each stated horizon, bit for bit at f64; `ftle_valid` false reads invalid; an escaping start's value is the one frozen at t_esc; controls: a map that reads another horizon than it labels, one that writes a number where `ftle_valid` is false, and one that reads a pixel's corner for its centre, each fail (REQ-VAL-344).
- `cargo test -p validation shape_occupancy` — the fractions sum to 1 within rounding; the equilateral Lagrange orbit puts all its time in the one cell holding the Lagrange point; a regularised occupant's run is weighted by physical time; the cells' areas are equal within the stated rounding; controls: an accumulator weighting by step count, one binning the shape point of the wrong step, and a partition with one cell doubled, each fail (REQ-VAL-345).
- Review checklist (physics reviewer) — each peek-log row states its tier, its horizons or ensemble and measure, the starts' source and the conventions (REQ-VAL-344, REQ-VAL-345).

## Notes
- Added by R-679 (3 Nov 2026, item 3, RQ-674; ported 3 Nov 2026). A build task for candidate original results, not a target: it is not scored, the results gate nothing, its tests block only its own merge, and TASK-M9-37's report does not wait on it.
- The batch-152 planes are the eucliss free-fall fate plane (400 × 240) and its two unequal-mass velocity planes (m₃ = 0.25 and 0.5), held locally and AI-authored, so peeks only (R-668 item 6): their starts are used, never their outcomes as a reference. Before a triage item is used, its source page is re-read (R-680).
