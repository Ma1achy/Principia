# TASK-M8-40 — Browser gates: Tier-N on real browsers, and the Playwright fragment goldens

- **Milestone:** M8
- **Closes:** REQ-VAL-116, REQ-COL-048, REQ-VAL-144
- **Depends on:** TASK-M8-38, TASK-M7-18
- **Needs (earlier milestones):** REQ-VAL-064, REQ-VAL-057, REQ-VAL-017, REQ-COL-052, REQ-COL-046, REQ-VAL-059, REQ-INT-059
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~350 lines

## Goal
The browser build is checked against the native wgpu Tier-N tolerances (R-85): the Tier-N suite runs on Chrome stable and Safari (R-110), every quantity within tolerance, out-of-tolerance quantities recorded. Colour output is verified nightly, on GUI and colour PRs and at each gate (R-134) by Playwright + headless Chrome golden-image diffs of the fragment output against the same baselines native wgpu offscreen has set since M1 (R-110), outside the parity suite, to the M7 golden tolerance. Parity Tier L's branch decisions and the 100-macro-step dispatch are checked through the browser's WGSL path (REQ-VAL-144).

## References
- `decisions.md` § "R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/contracts/principia_parity_contract.md` § "7. What this contract does *not* cover"

- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `web/tests/tier_n.spec.ts` — the Tier-N suite driven in Chrome stable and Safari (R-110), reporting per-quantity spread.
- `web/tests/tier_l.spec.ts` — the Tier-L boundary-state set and the 100-macro-step `done`-flag dispatch through the browser build's WGSL path, in Chrome stable and Safari, diffed against the CPU branch words (REQ-VAL-144).
- `web/tests/fragment_golden.spec.ts` (Playwright) + `xtask` `cargo xtask golden browser-fragment` (nightly, on GUI and colour PRs and at each gate, R-134; not per-commit), diffing against the same baselines the native wgpu offscreen renderer has set since M1 (R-110).

## Acceptance tests
- `cargo xtask gate browser-tier-n` — the browser build runs the Tier-N suite on Chrome stable and on Safari (R-110); every quantity falls within the native-wgpu tolerance; out-of-tolerance quantities are recorded (REQ-VAL-116).
- `cargo xtask golden browser-fragment` (tolerance: REQ-COL-052, calibrated at M7) — the Playwright job (nightly, GUI and colour PRs, gates; R-134) renders the fragment goldens in the browser build and diffs them against the native-wgpu baselines within tolerance; no baseline file changes without a recorded gate decision (R-110) (REQ-COL-048).
- `cargo xtask gate browser-tier-l` — in Chrome stable and Safari the boundary-state set's Tier-L decisions (including `N_sub`) and a 100-macro-step dispatch's branch words equal the CPU's, 0 forks (REQ-VAL-144).

## Notes
- Waits on **RQ-119** (whether the colour suite runs headless Chrome only or the R-110 pair) — REQ-COL-048 carries it.
- RQ-79 ruled: R-110 — the browsers are Chrome stable and Safari; goldens render with native wgpu offscreen from M1 and the M8 Playwright suite checks against the same baselines, with no re-baselining without a gate decision.
- RQ-97 ruled: R-113 — the browser legs of REQ-VAL-059, REQ-INT-059 and REQ-INT-028 are REQ-VAL-144, closed here with REQ-VAL-116.
