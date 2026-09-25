# prin-rs docs/NOTES.md — excerpts (commit 8600d45)

Verbatim excerpts of prin-rs `docs/NOTES.md` at `8600d45`, the passages the rulings cite (R-159, R-162, R-164). Line
numbers are those of the original file. Reference, not authority (R-1): see `../README.md`.

---

<!-- NOTES.md lines 2058-2109 -->

### BUILT. Predictions recorded before the 256^2 run, and what was already seen

logH is implemented: `src/integrate/logh/`, four `Integrator` variants (`LogHLeapfrog`,
`LogHRk4`, and the two `Plain*` controls, which are the same code path with `LhTime::None`), 22
tests across `logh_hamiltonian_fd`, `logh_march` and `logh_seam`. Phase D is
`examples/logh_arms.rs` at 256^2 over the six cases the 1024^2 gallery finished.

**The four predictions, in the state they were written before the run:**

  1. `logh_lf >= heggie` overall, if re-registration is the mechanism.
  2. logH should also win `far`, where AZ wins today on all 65536 pixels by a flat 0.7-0.9
     decades. That win is attributed to AZ never re-registering there; a method that never
     re-registers anywhere should not lose it.
  3. The drift field tracks FTLE positively, as the leapfrog's does and AZ's does not.
  4. **Added during planning:** `logh_rk4` behaves as a Sundman-transformed RK4 and loses most
     of the advantage, because `K + B == U` on shell and RK4 evaluates both denominators at the
     same point. If it matches `logh_lf`, this is wrong and so is Mikkola & Merritt's sentence
     it rests on.

**AND PREDICTION 3's TARGET NEEDED CORRECTING BEFORE IT COULD BE USED.** The number on record is
FTLE-vs-drift `+0.3048` for leapfrog and `-0.0820` for AZ. Read against the shifted controls in
the same table (`+0.0240` and `-0.1022`), **AZ's is a null, not a negative correlation** -- its
control is larger in magnitude. And the FTLE is computed *on the plain leapfrog*
(`src/physics/ftle.rs:26`), so the row that works has the **same integrator on both sides**. The
honest target is "leapfrog's drift tracks FTLE and AZ's does not", with the shared-arm confound
carried alongside any number quoted. Fixing it properly means an FTLE per occupant, which is
propagating a tangent vector through a regularised chart and is a separate build.

**WHAT HAS ALREADY BEEN SEEN, AND WHY IT IS NOT A RESULT.** Two things landed during
implementation and both bear on the predictions, so they are recorded here rather than
discovered later in a table:

  - **Prediction 4 is already supported, on the sharpest fixture available.** KDK traverses the
    two-body radial collision for all three pairs at identical step counts; RK4 does not
    complete at any step size tried. That is `tests/logh_march.rs`, not a field.
  - **Prediction 2 looks likely to FAIL.** A 24^2 smoke run of `logh_arms` on `far` put
    `logh_lf` **5.3 decades behind Heggie** and `logh_rk4` 3.2 behind. That is a 576-pixel grid
    at `max_steps = 60000`, run to check the harness prints, and this project has been burned
    twice by coarse grids -- one understated a maximum eightfold, another overstated a median
    twenty-sixfold. **It is not a result and is not quoted as one.** It is recorded because
    seeing it before the real run is exactly the circumstance in which a prediction quietly gets
    softened, and the prediction above is left as written.

**And a third thing that is a result, from the tests rather than the field:** logH does **not**
meet BRIEF §5's collision gate. `d_min < 1e-10` yes at `eta <= 3e-5`; `|dE/E| < 1e-12` never --
its drift *rises* with penetration depth, 1.1e-9 to 2.3e-6, where Heggie reads `5.422e-27` with
`drift_reg` flat at 4.4e-15. Heggie's KS map removes the `1/r` from the Hamiltonian; logH only
slows the clock, so the encounter has to be **resolved** rather than removed, and there is no
second, better-conditioned energy to report instead. **A chartless method is not a coordinate
regularisation with the coordinates left out**, and if prediction 1 fails this is the first
place to look for why.


---

<!-- NOTES.md lines 2573-2628 -->

## TTL IS BUILT AND VALIDATED, AND IT LOSES — MONOTONICALLY WORSE WITH MASS RATIO

Mikkola & Aarseth, CMDA **84** (2002) 343. `Omega = sum w_ij / r_ij` replaces logH's mass-weighted
`U`, with `W` carried in the state and advanced by `dW = (dOmega/dt) dt`.

**The weight choice makes the control an identity.** `w_ij = mbar^2` gives `Omega === U` exactly at
equal masses, so the `q = 1` row is an algebraic identity rather than a near-agreement. `w_ij = 1`
would have been simpler and left `Omega` on a different scale from `U`, so a comparison at fixed
`eta` would have scored the step size instead of the transformation.

`W` is registered **once at `t = 0`** and carried across every boundary — it is the analogue of
`B`, and re-seeding it per interval would discard the off-shell information the transformation runs
on and make the march depend on `n_sync`. `LhState` went 13 -> 14 components; `to_array13` is now
`to_array14` so a length mismatch is a compile error rather than a silent index shift.

**Validation, `tests/logh_ttl.rs`, five tests each with a non-vacuity arm:**

- `Omega === U` at equal masses to 1e-15, paired with the arm that they differ >10% at 90:1.
- `omega_dot` finite-differenced against `omega`, with a sign-flip mutation arm asserted to fire.
  Step `1e-3`, not `1e-8`: `Omega` is smooth and `O(1)`, so a small step surrenders digits to
  cancellation for nothing.
- **The `W`-vs-`Omega` gap converges at SECOND ORDER: `1.500e-5 -> 3.748e-6 -> 9.368e-7`, ratios
  `4.00` and `4.00`.** The first cut of this test asserted an absolute `< 1e-6`, measured `3.7e-6`,
  and would have been "fixed" by halving the step. An absolute tolerance cannot separate
  second-order error working as designed from a wrong `dW`; a convergence ratio can — a wrong term
  does not converge and a first-order one converges at 2x.
- End to end: equal-mass `|dr| 4.113e-14`, 90:1 ratio `|dr| 2.498e-5`.

**The sweep, `examples/ttl_mass_ratio.rs`, near-field configurations at 48^2, KDK, masses varied
and nothing else:**

```
      q   drift p50 logh    drift p50 ttl   gain(logH/TTL)   TTL cost
      1        8.363e-6         8.363e-6          -0.000      1.000x
      2        1.124e-5         5.786e-6          +0.288      0.999x
      5        1.147e-5         1.268e-5          -0.044      0.997x
     20        7.862e-6         2.334e-5          -0.472      0.996x
    100        2.683e-6         1.854e-5          -0.840      0.997x
   1000        2.369e-7         1.671e-5          -1.848      0.997x
```

Control exact at `q = 1`. `err>10` and `nonfin` **zero on every arm at every rung**, so neither
side is dead. Cost at parity throughout, so this is not a quality-for-work trade.

**The prediction — TTL beats logH at high ratio, ties at equal mass — is REFUTED on its first
arm.** No mechanism is offered.

The fact worth carrying: **the gap widens because logH IMPROVES 35x across the ladder**
(`8.4e-6 -> 2.4e-7`) while TTL stays flat near `1.7e-5`. Untested and stated as such: at `q = 1000`
the masses are `(0.4998, 0.4998, 0.0005)`, close to a two-body problem plus a test particle, and
whether that is still a mass-ratio sweep or a near-integrable limit is a separate measurement.

**IAS15 is not built.** Gauss-Radau nodes, a predictor-corrector loop and its own step control; its
only role here is a reference arm, since its per-lane variable work is already measured as fatal on
GPU (`warps hit 1.0000`).

