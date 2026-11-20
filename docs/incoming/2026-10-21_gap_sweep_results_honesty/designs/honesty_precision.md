# Honesty as a measurement: f32 against f64 in the trajectory viewer

*Draft proposal against main @45caf8be. The owner: "showing how the fp32 and fp64 trajectories in real space & on the
shape sphere differ in the trajectory viewer". Not hatching; the debug views exist.*

## 1. What exists today

The Trajectory panel and Inspector integrate a pixel's IC on the CPU at f64 (`principia_trajectory_viewing.md` §1;
`principia_render_gui_spec.md` §G2 :376-400, §G8 :1192-1206). Trajectory viewing §5 (:117-129) specifies an optional,
off-by-default **shape divergence overlay**: a one-pixel GPU dispatch of the survey kernel at f32, drawn over the f64
`n(t)`; §4 (:86-115) calls real space "CPU-f64 sugar, no comparison claimed". The atlas runs at f32 with horizon
`t ≈ 23` (`principia_00_philosophy.md` §2 :37-40, §4.1 :130-137) against a default `T_horizon` of 50 (gui spec :947).
The horizon is a field, `ln(1/eps)/ftle(IC)` (`principia_dd_predictability_horizon.md` §4.2, :140-152); 23 and 52
assume `lambda ≈ 0.7` (§7.2, :266-276), and the f32 value must be measured on the GPU (R-119, REQ-VAL-071). REQ-SYS-061
requires the atlas to state it (TASK-M8-41). The README says "What it can't resolve is hatched" (`README.md` :34);
R-467/R-470 removed the hatch from presentation views.

**prin-rs evidence** (`docs/RESULTS.md` :525-538, `results/output/f32_report.txt`; Aarseth–Zare, near-field 32×32,
`t = 13`): median `|dE/E|` **9.4e-6 at f32 against 2.8e-9 at f64**; outcome labels agree on **1022 of 1024**; two
pixels with `|dE/E| > 1`; on the unconditioned branch **152 of 1024 labels flip**. All inside the horizon, under the
old integrator: they say nothing about `t = 50`.

## 2. The gap

The overlay is optional, sphere-only and yields no number. Nothing ties one trajectory's parting to the clicked atlas
pixel or to the stated horizon. And f64 is treated as the reference although at `T = 50` it is near its own horizon
(~52).

## 3. Proposed design

### Three runs of one IC

| run | where | what it is |
|---|---|---|
| **atlas f32** | GPU, survey kernel, one-pixel dispatch | what coloured the pixel: same decode path, integrator, nominal copy |
| **f64** | CPU, `computeIC` | the inspector's standard |
| **f64 nudged** | CPU | f64 from the IC moved one ulp per component |

The nudged run gives f64's own trustworthy time here, so f64 never poses as truth. The f32 dispatch writes `r_i(t)` as
well as `n(t)`.

### What the user sees

- **Real space and shape sphere**: f64 solid, f32 dashed, same body colours.
- **A separation plot**, log scale: shape-sphere chord f32–f64 and f64–nudged (bounded by 2); real-space separation
  over the hyperradius, so escape does not swamp it.
- **Parting times** `t_part(f32)` and `t_part(f64)`: first time the chord exceeds a threshold (default 0.1), drawn
  on the plot, marked on both views and the playhead. Otherwise "did not part by `T`".
- **The starting gap** `|Δr|`, `|Δv|` at `t = 0` (the f32 run starts from the f32 decode).
- **Fates side by side** (outcome, escaper, `t_end`), with one of: "settled before the runs parted"; "agree, but
  after parting: not evidence"; "not determined at f32 here".
- **One line about the atlas pixel**, at the atlas playhead `t_p`:
  - before `t_part(f32)`: "The atlas colour here is within f32's trustworthy time for this point."
  - after: "The atlas colour here is past f32's trustworthy time (parted at `t = 18.4`). It shows a possible outcome,
    not this point's."
  - after `t_part(f64)` too: "Past f64's as well: neither run can be trusted here."
  - beside it the atlas's own estimate `ln(1/eps₃₂)/ftle`, "estimate from FTLE", against the measured parting: a
    point check of the field REQ-SYS-061 states.

### The atlas statement

The header strip reads: "Horizon `T = 50` · f32 trustworthy to about `t ≈ 23` (derived, not yet measured on this
GPU)", changing to "measured" once REQ-VAL-071 runs. A stain preset, **How far f32 sees**, draws `ln(1/eps)/ftle`.

### Where it lives

On by default in the Inspector; in the Trajectory panel, on for pinned and kept trajectories, off while following the
cursor. Toggle and threshold are `ViewUI`. An exported Inspector figure carries all three runs' settings.

### The README line

> **Honesty** — *It shows where its precision runs out: open any point to watch its single- and double-precision
> orbits part, and when. Failed runs are kept and counted, never painted over.*

(R-467/R-470: failures draw `#141418`; the hatch lives in the debug view.)

## 4. Fit with existing rulings

- **Amends** trajectory viewing §5 (on by default in the Inspector; adds numbers, real space, the nudged run) and §4
  ("no comparison claimed" for real space).
- Within R-93: that gate is about the automated cross-check; this is a per-point display reporting parting.
- R-84: precision-dependent labels on chaotic trajectories are expected; the panel says so.
- Never discard: a failed or budget-exhausted run is shown as such, with its end time.

## 5. Open questions

1. **Threshold**: (i) chord 0.1; (ii) where the stain colour would change; (iii) a factor over the starting gap.
   *Recommend (i), visible and editable.*
2. **The nudged f64 run**: (i) always; (ii) on request; (iii) never. *Recommend (i)*: cheap, and stops f64 posing as
   truth at `T = 50`.
3. **Which copy**: (i) nominal; (ii) all `E+1`; (iii) nominal, copies on request. *Recommend (iii).*
4. **Default horizon 50 against ≈ 23**: (i) state it in the header; (ii) also warn as the playhead passes 23;
   (iii) lower the default. *Recommend (i)*; (iii) is a physics ruling.

## 6. Tests (each with what makes it fail)

- **same_run_never_parts**: f64 against itself: chord exactly 0 throughout. *Fails if* the harness adds a difference.
- **regular_orbit_does_not_part**: figure-eight: "did not part by `T`". *Fails if* the threshold fires.
- **chaotic_orbit_parts**: Burrau: both times found, f32's earlier. *Fails if* either is missing.
- **f32_run_is_the_atlas**: f32 outcome and `t_end` equal the atlas pixel's nominal copy bitwise. Control: a CPU f32
  run differs on a chaotic fixture, so emulation would fail it.
- **ensemble_ratio**: over ≥ 256 chaotic ICs, median `t_part(f32)/t_part(f64)` near `ln eps₃₂/ln eps₆₄ ≈ 0.44`.
  Never read off one trajectory.
- **agreement_wording**: same fate latched after parting reads "not evidence". Mutation: drop the before/after check
  and it reads "settled": fails.
- **header_says_derived**: with no GPU measurement, "derived". *Fails if* "measured".


## Decided (Malachy, 21 Oct 2026), superseding §3 where they differ

- **Four runs, not three**: f32, f64, and **each nudged by one ulp** ("Do fp32 nudged the smallest amount also").
  f32 against nudged f32 gives f32's own sensitivity at this point.
- **Two marks, measured and from the literature**, both shown and editable:
  - **Faithful until**: f32 and f64 within a shape-sphere chord of **10⁻³**, the reliability threshold of Boekholt &
    Portegies Zwart (2015, 2020, 2023), stated as keeping the difference in the linear regime.
  - **Parted at**: chord **0.1**, the decorrelation threshold (separation at the system's own scale: Boekholt 2015,
    Hernandez et al. 2020); random shapes average 4/3.
  - Measured over 1000 ICs, four regions, Heggie to t = 50 (prin-rs PR #34, `parting_probe`): first-event fates agree
    **99.9%** before the 10⁻³ mark and **25%** after the 0.1 mark (n = 16, so thin). Median parting at 0.1: deep
    interior 12, preset_shape 26, near-field 43, config_stability 39 (14% parted), pooled 22. A single global "f32
    trustworthy to ~23" is right pooled and wrong for any given point, which is why the readout is per point.
  - Self-check: in a chaotic region the gap between the two marks is about ln(100)/λ. A gap far off that says the
    reading is in the noise floor or saturation, and the panel says "unreliable".
- **Where the trust line shows**: in the trajectory inspector panel when you click a trajectory, e.g. "the atlas shows
  t = 30; this point's f32 run parted at t = 18". **Nothing changes in the atlas**: it stays f32 on the GPU, coloured as
  now ("Why would we show different colours in the atlas for precision?").
- **Horizon**: default stays 50, and it can be **raised or lowered**, not floored at 50 ("Keep horizon to 50. (Also
  horizon shouldn't be capped to 50 should be able to raised or lowered)"). Amends the Run window's `T_horizon ∈ [50,
  200]` (render_gui_spec §G5) and R-132. The header states the horizon; the viewer states each point's parting.
