# TASK-M8-40 — Browser gates: Tier-N on real browsers, and the Playwright fragment goldens

- **Milestone:** M8
- **Closes:** REQ-VAL-116, REQ-COL-048
- **Depends on:** TASK-M8-38, TASK-M7-18
- **Needs (earlier milestones):** REQ-VAL-064, REQ-VAL-057, REQ-VAL-017, REQ-COL-052, REQ-COL-046
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~350 lines

## Goal
The browser build is checked against the native wgpu Tier-N tolerances (R-85): the Tier-N suite runs on one or two real browsers, every quantity within tolerance, out-of-tolerance quantities recorded. Colour output is verified at pre-release by Playwright + headless Chrome golden-image diffs of the fragment output, outside the parity suite, to the M7 golden tolerance.

## References
- `decisions.md` § "R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/contracts/principia_parity_contract.md` § "7. What this contract does *not* cover"

## Deliverables
- `web/tests/tier_n.spec.ts` — the Tier-N suite driven in a real browser, reporting per-quantity spread.
- `web/tests/fragment_golden.spec.ts` + `xtask` `cargo xtask golden browser-fragment` (pre-release job, not per-commit).

## Acceptance tests
- `cargo xtask gate browser-tier-n` — the browser build runs the Tier-N suite on one or two real browsers; every quantity falls within the native-wgpu tolerance; out-of-tolerance quantities are recorded (REQ-VAL-116).
- `cargo xtask golden browser-fragment` (tolerance: REQ-COL-052, calibrated at M7) — pre-release job diffs rendered fragment output against goldens (REQ-COL-048).

## Notes
- Which real browsers run the Tier-N check is not named ("one or two").
- Waits on RQ-79 (`REVIEW_QUEUE.md`): CI frequency, GPU hardware and browsers the corpus doesn't schedule.
