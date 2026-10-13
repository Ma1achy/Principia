# TASK-M1-20 — `theta`'s view on the cyclic ramp (R-423)

- **Milestone:** M1
- **Closes:** REQ-RENDER-085
- **Depends on:** TASK-M1-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-3
- **Size:** ~60 lines, the re-rendered golden included

## Goal
**R-423:** `theta` is an angle, the unwrapped phase `θ̃`, the shape sphere's equatorial longitude accumulated in
radians (dd_integrator §3.7), so its generated numeric view is cyclic, not on R-400's diverging ramp. The ledger gives
`theta` the `cyclic` scale, and the template TASK-M1-17 conformed places `raw = fract(θ̃ / 2π)` on the fixed `[0, 1]`
with `ramp_twilight`, the prelude's LUT index 1, as it does for `rho_angle`. Here `fract(x) = x − ⌊x⌋`, WGSL's `fract`
and the Rust reference's `rem_euclid(1.0)`, not Rust's `f32::fract` (`x − trunc x`), so `raw ∈ [0, 1)` for negative
`θ̃` too (REQ-RENDER-085). The other five fields R-400 moved to
`ramp_coolwarm` (`q_mass`, `ftle`, `mean_y`, `S`, `diffusion`) stay there: physics' R-423 check confirmed `S`, `ftle`
and `diffusion`, and `q_mass` and `mean_y` are open as RQ-274, which holds the M1 gate's cool-warm item, not this task.

## References
- `decisions.md` § "R-423 — At the M1 gate physics checks the six fields R-400 moved to cool-warm; `theta`, an angle, takes the cyclic ramp, not the diverging one *(amends R-400)*"
- `decisions.md` § "R-400 — A numeric field view whose declared ledger range spans zero is on a diverging ramp centred at zero; fields that can't be negative keep viridis *(closes RQ-260)*"
- `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"

## Deliverables
- `crates/ledger/src/payload.rs`: `theta`'s scale `cyclic` (its range left as declared; a cyclic view is placed on the
  fixed `[0, 1]`), citing R-423; every generated output it changes regenerated (`cargo xtask codegen`).
- The `debug-views/generated-theta` golden re-rendered on `ramp_twilight`, its `BASELINES.md` row citing R-423,
  proposed and confirmed at the M1 gate.
- Every other golden that draws `theta`'s generated view re-rendered on `ramp_twilight` (any merged task's golden that
  draws it), each `BASELINES.md` row citing R-423, proposed and confirmed at the M1 gate.

## Acceptance tests
- `cargo test -p ledger numeric_view_template` — `theta`'s view places raw at `fract(θ̃ / 2π)` on the fixed `[0, 1]` with `ramp_twilight`, `θ̃` and `θ̃ + 2π` giving the same raw, and its header defaults to `RANGE_AUTO = 0`; for negative `θ̃`, `θ̃ = −π/2` gives raw 0.75, and `θ̃` and `θ̃ − 2π` give the same raw; `q_mass`, `ftle`, `mean_y`, `S` and `diffusion` keep `ramp_coolwarm`; controls: `theta` on `ramp_coolwarm` fails, and a trunc-based `fract` (Rust's `f32::fract`, giving −0.25 at `θ̃ = −π/2`) fails (REQ-RENDER-085).
- `cargo xtask golden debug-views` — `generated-theta` re-rendered on twilight, and any merged task's golden that draws `theta`'s generated view re-rendered with it; every golden that does not draw it unchanged (REQ-RENDER-085).
- Review checklist (physics): `θ̃` is the shape sphere's equatorial longitude, accumulated in radians, so one turn is 2π; the view shows the net longitude turned since the IC modulo one turn, and the winding count and sense stay with `orbit_count` and `retrograde` (REQ-RENDER-085).

## Notes
- New M1 work from R-423, applied per R-369: the human's words were "If theta is an angle it needs a cyclic ramp, not
  a diverging one", and the corpus makes `theta` an angle (dd_simstate_payload §1, "unwrapped phase"; dd_integrator
  §3.7, `orbit_count = ⌊|θ̃|/2π⌋`; render_contract Part 6's live views already draw "running `θ̃` → cyclic"). The M1
  gate waits on this task, as on every M1 task.
- TASK-M1-17's qa tests that pin `theta` on `ramp_coolwarm`, if any, are ruling-forced changes (R-423): qa makes them,
  and the PR lists each with the assertion it changes (R-290, R-369).
- REQ-TOOL-161's period, one turn, now covers `theta` as well as `rho_angle`.
- The negative-`θ̃` case and its trunc-based control pin `fract`'s convention (physics review 5484757609, F2, on PR
  #197): `retrograde = θ̃ < 0`, so half the domain is negative, and a test over `θ̃ ≥ 0` alone passes with either
  convention.
- Goldens drawn by other tasks (code review 5484761196, F4, on PR #197), as TASK-M1-18 does for the log views: this
  task depends only on TASK-M1-17, so another M1 task's goldens may merge first. Checked at TASK-M1-14's PR #198 head
  `1f4deb64`: its `m1-subfields` and `m1-synthetic` cases draw `state`, `detail`, `saturated`, `dmin_pair`,
  `last_symbol`, `energy_drift` and the outcome palette, none of them `theta`'s generated view. TASK-M1-12's live shape
  view of `θ̃` (`u_mode` 1) is a fragment-computed view, already on Twilight, not the generated one. The PR lists any
  golden it re-renders.
