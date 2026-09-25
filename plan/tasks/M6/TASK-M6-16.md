# TASK-M6-16 — Auto: device characterisation — limits, probe, info, solve and the tier inversion

- **Milestone:** M6
- **Closes:** REQ-PERF-034, REQ-PERF-048, REQ-PERF-050, REQ-PERF-051, REQ-PERF-052, REQ-PERF-053, REQ-PERF-054, REQ-PERF-055, REQ-PERF-041, REQ-PERF-039, REQ-PERF-038, REQ-PERF-088, REQ-PERF-090
- **Depends on:** TASK-M6-15
- **Needs (earlier milestones):** REQ-PERF-023, REQ-PERF-025, REQ-PERF-029, REQ-PERF-031, REQ-PERF-032, REQ-PERF-030, REQ-PERF-093
- **Reviewers:** code, qa, perf
- **Pitfalls:** PIT-3
- **Size:** ~490 lines

## Goal
Auto is device characterisation: the limits leg reads `adapter.limits`, `navigator.deviceMemory` and adapter type as authoritative hard caps; the warmed-up probe measures cost-per-dt-per-sample on a spread of cheap and expensive quads at the real 2(E+1) trajectory count and reports a high percentile; `adapter.info` is only a soft cross-check; the solve bounds N, E, live-set size and `MAX_REL_DEPTH` together from one measurement, clamps to the limits and applies thermal headroom, and sizes the cache cap. The tier is derived by inversion (eps, then budget, then cap) with the 16.7 ms target and 41.7 ms floor fallback recorded. Hard limits clamp every preset, including named tiers and Custom. Auto starts the live controller; re-detect re-runs the probe.

## References
- `docs/contracts/principia_gui_state_contract.md` § "6. Quality settings — preset selector over one struct (see `principia_quality_device_note.md`)"
- `docs/design/principia_memory_tiers.md` § "6. Auto-mode tier selection"
- `docs/design/principia_quality_device_note.md` § ""Auto" = device characterisation (three legs: limits · probe · info)"
- `docs/design/principia_quality_device_note.md` § "The five ways the probe can lie (guard each)"
- `docs/design/principia_quality_device_note.md` § "Hard limits clamp EVERY preset (including custom)"
- `docs/design/principia_quality_device_note.md` § "What this subsystem resolves (two previously-open questions)"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "4. Deriving the tier instead of choosing it"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "3. Percentiles, not means — and the specific thresholds"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/design/principia_dd_simstate_payload.md` § "7. Memory"
- `docs/design/principia_quality_device_note.md` § "Open sub-questions (settle at implementation)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/engine/src/quality/characterise.rs`: `Limits`, `Probe` (behind a `GpuTimer` trait: timestamp queries or batched wall-clock), `InfoPrior`, `solve()`; a mockable device interface for tests.
- `crates/engine/src/quality/derive_tier.rs`: the inversion against samples/s, bytes/s, the frame budget and `sea_fraction(eps)` (a trait until TASK-M6-19 lands), with the fallback flag.
- Clamp of every preset against the limits; the Custom slider maxima and tooltip text exposed to the GUI.
- `cargo xtask bench device-characterisation` recording thresholds and budget heuristics (unified vs discrete).

## Acceptance tests
- `cargo test -p engine auto_starts_arbiter` — Auto starts the arbiter; selecting a named tier or Custom stops it; re-detect re-runs the probe (REQ-PERF-034).
- `cargo test -p engine boot_tier_selection` — with mocked limits/probe: 2 GB integrated at 4K boots Low; a Medium memory fit that fails the compute probe is held at Low (REQ-PERF-048).
- `cargo test -p engine limits_hard_caps` — with mocked low limits, no solved knob or allocation exceeds them (REQ-PERF-050).
- `cargo test -p engine probe_protocol` — probe log shows discarded warm-up, ≥ 2 quad classes, real trajectory count, timing source, and a high-percentile result (REQ-PERF-051).
- `cargo test -p engine info_soft_prior` — fast probe + mobile-iGPU info → conservative defaults; no name-keyed table in the code (REQ-PERF-052).
- `cargo test -p engine solve_formula` — solve output for a mocked throughput matches the formula with headroom and clamping (REQ-PERF-053).
- `cargo test -p engine limits_clamp_every_preset` — High on a weak mocked device clamps down; Custom slider max equals the limit and the tooltip is present (REQ-PERF-054).
- `cargo test -p engine cache_cap_from_solve` — cache_cap ≤ hard ceiling and derived from the solve (REQ-PERF-055).
- `cargo test -p engine tier_inversion_order` — given synthetic throughput numbers the solver returns the expected three axes in order (REQ-PERF-041).
- `cargo xtask bench tier-derivation-fallback` — on a device that cannot hold 16.7 ms, the derived tier records the fallback (REQ-PERF-039).
- `cargo xtask bench device-characterisation` — device-characterisation runs record the thresholds and budget heuristics used (REQ-PERF-038).
- Proposal: the probe quad set, length, percentile, thermal headroom and inconsistency ratio, each with evidence; the human confirms them at the M6 gate (REQ-PERF-088).
- Proposal: the ladder (rung count, presets) and the unified/discrete budget heuristic with device-characterisation evidence; the human confirms them at the M6 gate (REQ-PERF-090).

## Notes
- Values the corpus leaves approximate with no calibration requirement: thermal headroom (~60–70%), the probe's percentile and dt count, the "wildly inconsistent" test — see Gaps. The boot-fit margin is REQ-PERF-093 (calibrated at M5, TASK-M5-10).
- RQ-100 ruled: R-113 — the memory-fit margin REQ-PERF-048's boot fit uses is REQ-PERF-093, calibrated at M5 (TASK-M5-10).
- Closes, for gaps the corpus leaves open: REQ-PERF-088 (R-71 calibration), REQ-PERF-090 (R-71 calibration) (classification accepted by R-132).
