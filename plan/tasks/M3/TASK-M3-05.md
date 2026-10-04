# TASK-M3-05 — COM projection and invariant monitoring

- **Milestone:** M3
- **Closes:** REQ-INT-010, REQ-INT-011, REQ-INT-012, REQ-INT-030, REQ-INT-040, REQ-VAL-033, REQ-TOOL-149, REQ-INT-085, REQ-TOOL-150
- **Depends on:** TASK-M3-04, TASK-M2-24
- **Needs (earlier milestones):** REQ-PAY-010, REQ-PAY-031, REQ-GEN-007, REQ-DEC-025, REQ-TOOL-009
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10, PIT-3
- **Size:** ~350 lines

## Goal
After every `STEP` the wrapper's callback projects out the CoM position and total linear momentum (with `M` computed once and cached), on the parity surface with identical arithmetic and order on CPU and GPU, in CoM-frame particle coordinates; it never restores E or L_z. Before the loop it stores `E_0`, `L_{z,0}`; after each projected step it accumulates `ΔE_final`, `max|ΔE|`, `ΔL_z` final and `max|ΔL_z|` with the floored relative forms. The drift suspect predicates are read-time predicates over the stored latches, reading the occupant's `symplectic` flag.

## References
- `docs/contracts/principia_integrator_contract.md` § "COM projection and invariant monitoring"
- `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper"
- `docs/design/principia_dd_integrator.md` § "3.4 COM projection (per `STEP`; the policy is integrator contract Part 1)"
- `docs/design/principia_dd_integrator.md` § "3.5 Invariant monitoring (per `STEP`, post-projection)"
- `docs/contracts/principia_integrator_contract.md` § "Part 5 — Units and the horizon (inherited scale gauge)"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `docs/design/principia_dd_generation_root.md` § "Refinement — the scaling exponent"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/design/principia_dd_integrator.md` § "5. Unit tests"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug view)"
- `decisions.md` § "R-379 — `dbg_sentinel`'s suspect styling hook is an extension point; TASK-M3-05 defines the styling and applies it *(closes RQ-204)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_debug_tooling_plan.md` § "D. Payload field views — `SimState` scalars (ledger §3.4)"
- `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)"
- `decisions.md` § "R-381 — The drift views offer `symlog`, `lin` and `log`, with `symlog` the default; the value fed to `dbg_sentinel` is the compacted value *(closes RQ-206)*"
- `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"

## Deliverables
- `crates/kernel/src/driver/project.rs` — `project_com` (position–momentum form; velocity form for a velocity occupant), `M` cached.
- `crates/kernel/src/driver/monitor.rs` — `E_0`/`L_{z,0}` capture, per-step accumulation, `δ_E`, `δ_L` with `eps_E`, `eps_L` floors.
- Read side: the drift suspect predicates (energy-suspect on relative δE, L_z-suspect on absolute ΔL_z) in the generated accessor layer, reading `symplectic` from the profile; no stored suspect bits.
- The suspect styling (R-379): its look, and which drift suspect predicate drives which style, defined in render contract Part 5 "Presentation layer" (R-72; the physics reviewer approves before merge); applied by the energy-drift and L_z-drift views from the suspect predicates on the preset's ramp output, which under the viridis palette swap is `dbg_sentinel`'s output. `dbg_sentinel(x, frag_xy)` keeps its signature, and its absence hatch and literal-value rule hold on either ramp (REQ-TOOL-149).
- The same definition gives the value each drift view passes to `dbg_sentinel`: the field-view preset's compacted value, so that the view still shows the drift's magnitude (a raw drift ≪ 1 lands at the ramp's middle). The drift views offer colour_composition's `symlog` (signed, through the midpoint, with the field's floor `eps_E` / `eps_L`), `lin` (diverging, linear, over `[−R, R]`) and `log` (unsigned, with the floor) compactions; `symlog` is the default (`f_edrift` = `energy_drift · diverging · symlog`), and `lin` and `log` are reached through the presets' compaction override (R-381; REQ-TOOL-149). Each compaction has its own placement and ramp (decided per R-369 after physics review 5407380838; REQ-TOOL-149). `symlog`: the compacted value takes `dbg_sentinel`'s place `t = 0.5 + 0.5·x/(1 + |x|)` on the preset's `diverging(c−,c0,c+)` by default, 0 at the neutral `c0`, its three colours given by the definition (R-72), and on viridis, `dbg_sentinel`'s ramp, through the palette swap every preset inherits (colour_composition §6; RQ-208, decided per R-369, option 3). `lin`: `range_norm(x, −R, R, false, ·)` (render_gui_spec §10.1), the fixed range, clamped, on the `diverging` ramp (viridis through the palette swap), 0 at `c0`, a drift beyond `±R` saturating to `c−` or `c+`. `log`: the drift's magnitude `|x|`, unsigned, at `dbg_log`'s place `1 − 1/(1 + s)` (`s` the floored log value) on the sequential viridis ramp, never the diverging one (colour_composition §1.2 sends positive fields to sequential), the view stating that it shows `|drift|`. Only the signed compactions, `symlog` and `lin`, use the `diverging` ramp.
- The default range `R` of the drift views' `lin` override, proposed with evidence (R-71, R-381; REQ-TOOL-150). One linear `R` cannot resolve drifts that span decades (pitfalls §3); `symlog`, the default, is the view for that, so `R` separates the non-symplectic occupants' drifts and the symplectic round-off band may read as neutral under `lin`.
- The drift suspect predicates' thresholds, energy-suspect on relative δE and L_z-suspect on absolute ΔL_z, proposed with evidence (R-71; REQ-INT-085). ε_E and ε_L are δE's floors, not these thresholds.

## Acceptance tests
- `cargo test -p kernel com_projection` — after one STEP from a state with nonzero CoM offset and momentum, R_com and P_com are zero to rounding; M is not recomputed per step (REQ-INT-010).
- Review (physics): no code path modifies state to restore E or L_z (REQ-INT-011).
- `cargo test -p kernel invariant_monitor` — rest start (E_0 ≠ 0, L_z0 = 0): δ_L finite; a synthetic spike-then-recover trace yields max|ΔE| ≫ |ΔE_final| (dd test 7) (REQ-INT-012).
- `cargo xtask gate projection-parity` — property test: stepOnce on shared states, the projected state is bit-identical across repeated CPU-f32 runs and follows the one written operation order; no Jacobi conversion inside the step (REQ-INT-030).
- `cargo test -p kernel drift_suspect_read_time` — the payload has no suspect bits; the predicate is evaluated at read; Euler + the energy-drift view lights SUSPECT_ENERGY everywhere (REQ-INT-040).
- `cargo test -p kernel close_encounter_drift_shape` — dd test 7: a close-encounter IC shows max|ΔE| ≫ |ΔE_final| in the stored max vs final (REQ-VAL-033).
- `cargo test -p render drift_suspect_styling` — the energy-drift and L_z-drift views render a synthetic payload with three samples: energy-only suspect (Euler lighting SUSPECT_ENERGY, its `dLz_max` set below the L_z-suspect threshold), L_z-only suspect (set through the synthetic payload's `dLz_max`, its energy drift below the energy-suspect threshold) and non-suspect (a symplectic occupant below both thresholds, REQ-INT-085). It fails if, in either view, the energy-only sample's pixel shows the L_z styling or the L_z-only sample's pixel the energy styling; if a suspect sample's pixel lacks the style its predicate drives, in a view the definition applies that style to; or if the non-suspect sample's pixel, in either view, differs from the preset's ramp output on the compacted value, the default `symlog` on the `diverging` ramp, with the absence NaN hatched as `dbg_sentinel` draws it (R-381; RQ-208, decided per R-369, option 3; REQ-TOOL-149).
- `cargo test -p render drift_view_compaction` — the energy-drift and L_z-drift views default to `symlog` with the field's floor on the `diverging` ramp, and accept the `lin` and `log` compaction overrides and the viridis palette swap; a non-suspect sample's pixel equals its compaction's placement on its ramp: `symlog` at `t = 0.5 + 0.5·x/(1 + |x|)` on `diverging`, and `dbg_sentinel` of the compacted value under the viridis palette swap; `lin` at `range_norm(x, −R, R, false, ·)` on `diverging`; `log` at `dbg_log`'s `1 − 1/(1 + s)` of `|x|` on viridis. It fails if a view's default is not `symlog` on `diverging`; if an override or the palette swap is refused or ignored; if a pixel differs from its compaction's placement on its ramp; if a negative `lin` drift does not land on the `c−` side of `c0`, or a drift beyond `±R` is not saturated to `c−` or `c+`; or if `log` draws on the `diverging` ramp rather than viridis, or draws a negative and a positive drift of equal magnitude in different colours (R-381; RQ-208, decided per R-369, option 3; decided per R-369 after physics review 5407380838; REQ-TOOL-149).
- Review (physics): the suspect styling's definition in render contract Part 5 "Presentation layer" gives its look, the predicate driving each style and the value each drift view passes to `dbg_sentinel`, the preset's compacted value, `symlog` by default with `lin` and `log` as overrides (R-381), and each compaction's placement and ramp: `symlog` at the midpoint place on `diverging` with its three colours by default and viridis through the palette swap (RQ-208, decided per R-369, option 3), `lin` at `range_norm` over the clamped `[−R, R]` on `diverging`, and `log` as `|x|` at `dbg_log`'s place on viridis, its view stating it shows `|drift|` (decided per R-369 after physics review 5407380838), and the views match it (REQ-TOOL-149).
- Proposal: the energy-suspect and L_z-suspect thresholds, with relative δE and absolute ΔL_z, final and max, for Euler, RK4 and the symplectic occupants (KDK, Yoshida-4, Yoshida-6) on dd test 2's long bounded orbit and dd test 7's close-encounter IC as evidence, and which suspect predicates each occupant lights on each. Energy: on test 2's orbit Euler lights SUSPECT_ENERGY and no symplectic occupant does (dd test 2); test 7's symplectic max|ΔE| spike is expected close-encounter stress (dd §3.5, test 7; integrator contract Part 2), so the energy criterion is not applied there and the record says whether the spike lights it. L_z: measured and recorded on both; its pass criterion waits on RQ-207. The human confirms them at the M3 gate (REQ-INT-085).
- Proposal: the default symmetric range `R` of the drift views' `lin` override, for `energy_drift` and `Lz_drift`, with the drifts measured, final and max, for Euler, RK4 and the symplectic occupants on dd test 2's orbit and dd test 7's IC as evidence, and the `lin` view at the proposed `R` (`range_norm` over the fixed, clamped `[−R, R]`, on `diverging`) beside the default `symlog` view. It passes only if, on test 2's orbit, Euler's and RK4's drifts render as distinct colours, neither saturated at `±R`, and fails if they draw the same colour or either saturates; the symplectic occupants' band may read as neutral, and test 7's symplectic spike is recorded (decided per R-369 after physics review 5407380838). The human confirms it at the M3 gate (R-71, R-381; REQ-TOOL-150).

## Notes
- Per-trajectory projection of the Benettin shadow and ensemble copies is TASK-M3-14's (REQ-INT-039).
- RQ-97 ruled: R-113 — REQ-INT-030's CPU-f32 vs GPU-f32 match on Metal is dropped from M3; M4 covers it by REQ-VAL-061 (one step).
- RQ-204 ruled by R-379: the suspect styling is this task's to do.
- RQ-206 ruled by R-381: the drift views offer `symlog`, `lin` and `log`, with `symlog` the default; the value fed to `dbg_sentinel` is the compacted value.
- RQ-208 decided per R-369, option 3: the drift views' ramp is the preset's `diverging` through a neutral by default (colour_composition §1.2, §6), and `dbg_sentinel`'s viridis through the palette swap (render contract Part 5).
- Decided per R-369 after physics review 5407380838 (PR #137), recorded in decisions.md R-381: `log` shows `|drift|` at `dbg_log`'s place on the sequential viridis ramp, so an energy loss never reads as a gain; `lin` is `range_norm` over the fixed, clamped `[−R, R]`, so ±R reaches the ramp's ends and REQ-TOOL-150's criterion can fail.
- RQ-207 (open): the L_z-suspect threshold's pass criterion, the occupants and fixtures on which it must and must not light. REQ-INT-085's L_z part waits for its ruling.
- Closes, for gaps the corpus leaves open: REQ-INT-085 (R-71 calibration; PR #137's physics review, 5406355539); REQ-TOOL-150 (R-71 calibration, the `lin` range; R-381).
