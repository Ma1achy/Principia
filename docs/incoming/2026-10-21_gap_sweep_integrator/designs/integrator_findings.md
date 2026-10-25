# Integrator: what prin-rs already settled, what was tested today, and what the contract should say

Items 1, 2, 10 and 11 of today's gap sweep. prin-rs was read first; the tests below were run in prin-rs
(`examples/heggie_stepper_probe.rs`, `budget_horizon_probe.rs`, `sync_grid_probe.rs`, uncommitted in the working tree).

## 1. The default occupant: Heggie × RK4, not Heggie × KDK

**The corpus says** "Heggie with KDK leapfrog is the general default" (render_gui_spec, Run window :949–950;
REQ-GUI-102 tests "occupant = Heggie×KDK"), the integrator contract's Part 2 table names Yoshida-4 as the default
stepper (:85), and REQ-INT-006/017 say every stepper composes with every regularisation.

**prin-rs settled it the other way.** Its integrator registry (`src/integrate/mod.rs`): "there is no `Az + Leapfrog`
and no `Heggie + Leapfrog`. Their `Gamma` couples position and momentum, so neither Hamiltonian is separable and a
drift-kick-drift composition does not apply." Heggie runs with RK4, and every result behind the Heggie default (31 of
32 gallery cases) was taken with RK4. The integrator contract itself says the same of AZ (Part 2a :326).

**Tested today** on the figure-eight, fixed fictitious step, no step control, both of Heggie's time transformations:

```
                     order    calls/step   energy over 100 orbits
rk4                   3.99        4.0      bounded, 5e-7 to 3e-6
kdk (explicit)        0.8–1.2     3.0      diverges (to 1e78 and beyond)
implicit midpoint     2.00        9–10     bounded, 2e-4, symplectic
generalised leapfrog  2.0        15–20     unstable over long runs (explained below)
```

- The leapfrog a reader of "Heggie with KDK" would write is **first order and diverges**. It is not a usable occupant.
- A genuinely symplectic Heggie stepper exists but is **implicit**: implicit midpoint holds energy bounded, at 2–2.5×
  RK4's force evaluations and about 400× RK4's error at the same step.
- The generalised (Störmer–Verlet) leapfrog, also implicit, reaches order 2 over one orbit but grows without bound
  over 100. **Cause, measured:** it does not keep Heggie's Eq. (9), `Σ qᵢ = 0`, which is quadratic in the regularised
  positions (`qᵢ = Qᵢ²`). Implicit midpoint keeps every quadratic invariant exactly; a partitioned method like
  Störmer–Verlet keeps only those mixing positions with momenta (Hairer, Lubich & Wanner, ch. IV.2), so Eq. (9) is
  not kept. Implicit midpoint holds the residual at 1e-15–1e-13; the leapfrog's grows as h² per orbit and keeps
  growing, and the energy breaks once it reaches about 1.8e-2, in every run, at every step size, under both time
  transformations. The control is implicit midpoint started deliberately off the constraint: it stays bounded at
  1e-3 and 1e-2, and at 3e-2 holds under one time transformation and diverges under the other. My earlier "worse at
  smaller steps" was a misread: smaller steps break later (2.2, 4.4, 8.8 orbits); the figure quoted was the maximum
  over runs that had already diverged. Nothing below depends on it; it adds one row to the warning table's reasons.

**Proposed:**
- **The user chooses both, freely, and no combination is forbidden** (Malachy, 21 Oct: "don't forbid combos of
  integrators & regularisers… maybe just have a warning somewhere that they might not work"). The regularisation
  (None, AZ, Heggie, …) and the stepper (RK4, KDK, Yoshida, …) stay separately selectable, every pair runs.
- **A pair known not to work carries a warning**, not a block: shown beside the selector and in the Problems panel,
  and recorded in every export's settings, so a result made with it says so. The text states what was measured, for
  example: "Heggie and AZ are not separable, so a leapfrog or Yoshida step is first order here and can diverge
  (figure-eight: order ~1, diverged within 100 orbits)". Runs that diverge are undetermined, never discarded.
- **Each such pair must still be defined**, so its results are reproducible: a leapfrog on a non-separable
  Hamiltonian means the explicit kick-drift-kick tested above (kick at (u, p), drift at (u, p_half), kick at
  (u1, p_half)); a Yoshida step is the same composition with Yoshida's coefficients.
- With regularisation **off**, every stepper is fine (the Hamiltonian is separable), and the symplectic steppers are
  the natural choice there.
- The default is **Heggie × RK4**, the combination that was measured.
- Replace "every stepper composes with every regularisation" (REQ-INT-006, -017) with "every pair runs; the pairs
  measured not to work carry a warning", and keep a table of which pairs are measured good, measured bad, or
  unmeasured. REQ-GUI-102's default becomes Heggie×RK4.
- A symplectic regularised occupant, if ever wanted, is an implicit stepper and gets its own entry in the table.

## 2. What bounds Heggie's steps (the "integration floor")

**The corpus** defines the integration floor as substep saturation (`N_sub == N_max`, the `saturated` bit:
deep_zoom :125, scheduler Part 3 :93, REQ-SCHED-063/077), and the contract still calls regularisation "v2" (Part 6
:403; REQ-INT-033 "no KS state representation in v1"). Heggie has no `N_sub` and no `N_max`.

**prin-rs's answer** (`src/integrate/heggie/driver.rs`, `src/ensemble/pixel.rs`, `results/step_budget/`):

- A **whole-run step budget**, `max_steps = 480,000`. Running out ends the run: `budget_exhausted`, the copy is not
  finite, the footprint is **undetermined** and the scheduler reports `Decision::Undetermined`. Never discarded.
- Two **advance-anyway** signals are carried separately: the separation floor fired (`r_floored`) and steps held at
  the entry cap (`n_cap_hits`). They let a run continue under-resolved and are counted, not terminal.
- Measured over 35 charts at t = 13: running out of budget is the **only** failure mechanism present
  (`flagged == budget` in every row), every chart clears at 480k, and **a too-small budget manufactures work**
  (truncated footprints read undetermined and split).
- `error_ratio` is blind to a truncated run (it reads its converged 1.0000), so the budget flag must be read directly.

**Tested today: the budget does not scale with the horizon.** Same grid, n_sync scaled with t_max:

```
                    t=13, 480k    t=50, 480k         t=50, unbounded   step rate p99 (per unit time)
burrau_nu_k         0             42 of 576 (7.3%)   0                 ~1.8e4
latent_mixed_h3     0             0                  0                 ~4.5e3
config_stability    0             0                  0                 ~1.1e3
latent_shape        0             0                  0                 ~3e2  (control: never binds)
```

The rate is nearly the same at 13 and 50. Rates are averaged over a pixel's copies; the budget applies per copy, so
the worst copy needs more.

**Proposed:**
- The contract states that regularisation is v1, Heggie is the default (R-161), and removes "v2" and REQ-INT-033.
- The integration floor depends on the occupant the user chose. **With regularisation off**, the corpus's existing
  substep machinery still applies unchanged (`N_sub`, `N_max`, the `saturated` bit). **For a regularised
  occupant**, it is: **step budget exhausted → undetermined** (terminal), with
  `r_floored` and cap hits as the counted, non-terminal signals. The `saturated` bit's meaning is restated in those
  terms; REQ-SCHED-063/077 are re-pointed.
- The budget is a **rate**, steps per unit physical time, so it is defined for a live playhead with no fixed end.
  Starting value to calibrate: about 4e4 per unit time per copy (2× the worst measured pixel mean), confirmed at its
  gate. Every loop stays bounded (rule 3).

## 10. Sync boundaries, frames and the wedges

**The wedges are fixed in prin-rs**, and not by Heggie: ablated one at a time, the **predictive step limit alone**
removes them (dense-pale 0.0026 → 0.0001); the `dtau` fix removes the non-finite pixels; the landing clamp is what
makes the march second order. All three are per-step and live-compatible. Heggie's own contribution is that it never
re-registers, so its boundary cadence sensitivity is ~10× smaller than AZ's.

**The open question** was how sync boundaries relate to the frame loop's macro-steps. **Tested today**: Heggie, physical
step held fixed (eta scaled with n_sync), closure window held fixed in time, grid made up to 32× finer:

```
                  ×4 finer (0.10)   ×16 (0.025)    ×32 (0.0125, about a frame)
cost              +1–3%             +17–25%        +40–52%
shape change      ~1e-11–1e-10      ~1e-10         ~1e-10   (far ~1e-6)
labels flipped    0                 ≤1 of 1024     ≤4 of 1024
```

**Proposed:**
- The sync grid is a **fixed quantum of physical time**, `Δ_sync`, the same for every run, independent of the
  horizon and of frame timing. Never `t_max / n_sync`. (The standing rule "n_sync fixed while t_max varies compares
  different discretisations" becomes impossible by construction.)
- Step size is set from `Δ_sync`, not from whatever target a call asks for, and the march **only stops on the grid**.
  So live march, catch-up and re-boot all reach the same state at the same grid time, bitwise.
- The playhead shows the latest boundary at or before it. `Δ_sync` is a calibration value: 0.1 costs ~2%, 0.0125
  (frame-scale) ~50%.
- Time-based windows (the escape closure window) are stated in physical time, never in boundaries.

## 11. Which energy is checked under Heggie

**prin-rs carries both** (`HgOut::drift` from the Cartesian state, `drift_reg` from the regularised one):
- The Cartesian read-out is ill-conditioned after a close passage (280× to 7100× worse on the radial collision) while
  `drift_reg` stays at round-off. So the **two-body collision acceptance test gates on the regularised energy**, and a
  committed test fails if the Cartesian number ever falls with `eta`.
- Per-pixel drift (the diagnostic field, `error_ratio`) uses the Cartesian energy, and is only read with termination
  **off**: a run parked at a collision reads its own cancellation, which produced five wrong conclusions in a row.

**Proposed:** the contract's monitored energy is both, named: the regularised energy for acceptance and for any
per-step suspect flag under a regularised occupant; the Cartesian energy for diagnostics, read only from runs that did
not stop at an event. REQ-VAL-045 gates on the regularised energy.
