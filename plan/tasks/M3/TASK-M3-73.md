# TASK-M3-73 — The FTLE views already built state the shadow's provisional calibration, from one source (R-976)

- **Milestone:** M3
- **Closes:** REQ-GUI-438
- **Depends on:** TASK-M3-14, TASK-M1-12
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~120 lines, the tests included

## Goal
R-976 (RQ-794 (c)) keeps the FTLE shadow's calibration, (‖δ₀‖, n_renorm, dd test 10's tolerance) = (3e-3, 300, 0.2),
provisional and unsupported on close-encounter chaotic orbits, "stated wherever the FTLE is shown", until the M3 gate
confirms REQ-INT-076's values after the re-run (TASK-M3-72). The FTLE views built by TASK-M1-12 (the finalised `ftle`
view and the running `S/t` view, and every catalogue view that reads `ftle`) show real values once TASK-M3-14 merges.
This task builds the one source of the statement, which falls silent once REQ-INT-076's confirmed values are recorded,
and puts it on those views. Every later surface that shows the FTLE reads the same source (REQ-GUI-437, REQ-GUI-439 to
REQ-GUI-442, REQ-VAL-356).

## References
- `decisions.md` § "R-976 — RQ-794, what REQ-INT-076 proposes for the FTLE shadow's ‖δ₀‖ and `n_renorm`: option (c), (3e-3, 300, 0.2) stay provisional and stated as unsupported on close-encounter chaotic orbits wherever the FTLE is shown; the pre-registered eight-orbit calibration is re-run unchanged, then `min_regions` ≥ 6 proposed; the M3 gate confirms no final value until the re-run *(closes RQ-794)*"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/design/principia_dd_integrator.md` § "3.8 Co-computations (tier-gated, ride the forward pass)"

## Deliverables
- The one source: the statement and its switch, read from whether REQ-INT-076's values are recorded as confirmed, never
  a second copy of the text in a view.
- The field views that read `ftle` carry it, in their label or legend line.
- `cargo test -p render ftle_views_state_provisional`, with its controls (R-176).

## Acceptance tests
- `cargo test -p render ftle_views_state_provisional` — every catalogue view that reads `ftle` (the finalised `ftle`
  and running `S/t` views among them) carries the statement from the one source while REQ-INT-076's values are
  unconfirmed; controls: a view of `ftle` without it, and the statement still shown once the confirmed values are
  recorded, each fail (REQ-GUI-438).

## Notes
- New (12 Nov 2026), from R-976, applied per R-369 (task scoping; code review 5875707244 of PR #503, F4): the views
  already built need a task of their own, since REQ-GUI-437's first closer, TASK-M4-13, lands after the M3 gate.
