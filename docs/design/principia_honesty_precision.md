# Principia: honesty as a measurement — f32 against f64 in the trajectory viewer

**Adopted by R-574 and R-575 (21 Oct 2026, the human's gap sweep, PR 2).** This document is part of the corpus from
those rulings on: where it says "proposal", the proposal is what is built, except where R-574, R-575 or a later ruling
says otherwise. Its source is the bundle's `designs/honesty_precision.md` (drafted against main @45caf8be), recorded
byte for byte in `docs/incoming/2026-10-21_gap_sweep_results_honesty/` with the probe its marks were measured with
(`evidence/parting_probe.rs`, prin-rs PR #34) and its output. The human's "Decided" section (§5) supersedes the draft's
§3 where they differ; this document is written as decided. The owner's request, as the draft quotes it: "showing how
the fp32 and fp64 trajectories in real space & on the shape sphere differ in the trajectory viewer". Line numbers cited
below are the draft's, against main @45caf8be.

---

## 1. What exists

The Trajectory panel and the Inspector integrate a pixel's IC on the CPU at f64 (`principia_trajectory_viewing.md` §1;
`principia_render_gui_spec.md` §G2, §G8). Trajectory viewing §5 specified an optional, off-by-default **shape
divergence overlay**: a one-pixel GPU dispatch of the survey kernel at f32, drawn over the f64 `n(t)`; its §4 called
real space "CPU-f64 sugar, no comparison claimed" (*both before R-574*). The atlas runs at f32 with horizon `t ≈ 23`
(`principia_00_philosophy.md` §2, §4.1) against a default `T_horizon` of 50. The horizon is a field,
`ln(1/eps)/ftle(IC)` (`principia_dd_predictability_horizon.md` §4.2); 23 and 52 assume `λ ≈ 0.7` (§7.2), and the f32
value must be measured on the GPU (R-119, REQ-VAL-071). REQ-SYS-061 requires the atlas to state it (TASK-M8-41). The
README said "What it can't resolve is hatched"; R-467 and R-470 removed the hatch from presentation views.

**prin-rs evidence, before this design** (prin-rs `docs/RESULTS.md`, `results/output/f32_report.txt`; Aarseth–Zare,
near-field 32 × 32, `t = 13`): median `|dE/E|` 9.4e-6 at f32 against 2.8e-9 at f64; outcome labels agree on 1022 of
1024; two pixels with `|dE/E| > 1`; on the unconditioned branch 152 of 1024 labels flip. All inside the horizon, under
the old integrator: they say nothing about `t = 50`.

## 2. The gap, and the measurement that closes it

The overlay was optional, sphere-only and yielded no number. Nothing tied one trajectory's parting to the clicked atlas
pixel or to the stated horizon. And f64 was treated as the reference although at `T = 50` it is near its own horizon
(~52).

**The parting probe** (`evidence/parting_probe.rs`, prin-rs PR #34; its output `evidence/parting_probe_output.txt`):
1000 ICs, 250 from each of four regions (config_stability, near-field, deep interior, preset_shape), each run four
ways under Heggie to `t = 50` with termination off: f64, f64 nudged by one ulp per component, f32 from the f32-rounded
IC, and f32 nudged. Each run records the shape-sphere vector `n` on a fixed boundary grid (Δ = 0.1); three pairs are
compared by the chord `|n_a − n_b|` (0 to 2; random uniform pairs average 4/3). One IC had a run out of step budget and
is listed, not used (999 used). Transcribed from the output:

| reading | value |
|---|---|
| f32–f64 first-event fates agree, event before the chord passes 10⁻³ | 717 events, **0.999** (pooled) |
| the same, event after the chord passes 0.1 | 16 events, **0.250** (pooled; thin) |
| median `t_part(0.1)`, f32–f64 | deep interior 12.1, preset_shape 25.5, near-field 43.2, config_stability 38.9 (14 % parted by 50), pooled 22.2 |
| median `t_part(0.1)`, f64–f64′ | pooled 40.3, 5 % parted by 50 (deep interior 39.4, 19 %) |
| median `t_part(0.1)`, f32–f32′ | pooled 22.3, 47 % parted by 50 |

A single global "f32 trustworthy to ~23" is right pooled and wrong for any given point, which is why the readout is per
point.

## 3. The design

### 3.1 Four runs of one IC

| run | where | what it is |
|---|---|---|
| **f32** | GPU, the survey kernel, a one-pixel dispatch | what coloured the pixel: the same decode path, integrator occupant and nominal copy as the atlas |
| **f32 nudged** | GPU, the same dispatch | the f32 run from its decoded IC moved one ulp per component |
| **f64** | CPU, `computeIC`: the shared kernel at f64 under the f32 run's sim key | the same integrator occupant, coefficients and step rule as the f32 run, never the Inspector's adaptive RK45 |
| **f64 nudged** | CPU, the same | the f64 run from its IC moved one ulp per component |

**One integrator for all four runs** (physics review 5627073272 of PR #283, F3; applied per R-369). Both f64 runs, and
both nudged runs, take the f32 run's sim key: the same occupant (stepper × regularisation), coefficients and step rule,
so the chords measure precision and nothing else. An f64 run under another integrator would mix truncation error into
the f32–f64 chord, the "category error dressed as a bug" of integrator contract Part 6 ("Integrator-family mismatch in
cross-checks"); the marks were measured so, with Heggie at both precisions and one step rule (`parting_probe.rs`).
*Was:* the f64 row "| **f64** | CPU, `computeIC` | the Inspector's standard |" and the nudged row "| **f64 nudged** |
CPU | the f64 run from its IC moved one ulp per component |".

**The nudge.** Each component of the decoded Cartesian IC, the three bodies' positions and velocities in the plane
(twelve numbers), is moved one ulp toward `+∞` at its run's precision (`next_up`), as the probe does. f32 against
nudged f32 gives f32's own sensitivity at this point; f64 against nudged f64 gives f64's own trustworthy time, so f64
never poses as truth. How the GPU dispatch takes the nudge after its own decode, and what a zero component becomes
where the GPU flushes subnormals, is part of the f32 trace's definition (REQ-TOOL-127, trajectory viewing §5).

**The f32 dispatch writes `r_i(t)` as well as `n(t)`.**

**Past a terminal event.** The marks were measured with termination off, every run reaching the horizon, and each
run's fate its first event. The atlas latches its first terminal event and stops there (integrator contract Part 7).
Whether the viewer's four runs carry on past their first event, as measured, or stop there as the atlas does, and what
the chord reads after a run has stopped, is RQ-558. *Applied meanwhile:* its option (a), as measured: each run's fate
and `t_end` are its first event, latched as the atlas latches it, and the run carries on to the horizon for the chord;
a run that fails or runs out of its step budget ends there, shown as such with its end time, and its chords read
"ended at `t = …`" after it.

### 3.2 The chord and the two marks

**The chord** between two runs at time `t` is `|n_a(t) − n_b(t)|`, the distance between their signed shape-sphere
unit vectors (R-544), from 0 to 2; a non-finite value reads 2, as in the probe. It is read at the sync grid's
boundaries (R-565; the probe's grid was 0.1), for three pairs: **f32–f64** (what the atlas's precision costs),
**f64–f64′** (f64's own horizon) and **f32–f32′** (f32's own horizon).

**Two marks, measured and from the literature, both shown and editable** (the thresholds are `ViewUI`):

- **Faithful until**: the first boundary at which the chord exceeds **10⁻³**, the reliability threshold of Boekholt
  & Portegies Zwart (2015, 2020, 2023), stated as keeping the difference in the linear regime.
- **Parted at**: the first boundary at which the chord exceeds **0.1**, the decorrelation threshold (separation at the
  system's own scale: Boekholt 2015, Hernandez et al. 2020).

A pair whose chord never exceeds a mark reads "did not part by `T`" for it. `t_part(f32)` below is the f32–f64 pair's
"parted at", and `t_part(f64)` the f64–f64′ pair's.

**Self-check.** In a chaotic region the gap between the two marks is about `ln(100)/λ` (generally `ln(θ_part /
θ_faithful)/λ`, with `λ` this point's FTLE, the `ftle` its own f32 run writes, §3.3). A gap far off that says the
reading is in the noise floor or saturation, and the panel says "unreliable". How far is "far off" is a calibration
(R-71, REQ-VAL-198).

### 3.3 What the user sees

- **Real space and the shape sphere**: f64 solid, f32 dashed, the same body colours. *Applied per R-369 (the port):*
  the nudged runs are not drawn on the two views; they are in the separation plot.
- **A separation plot**, log scale: the three chords (each bounded by 2), and the real-space separation over the
  hyperradius, so escape does not swamp it.
- **The marks**, each pair's "faithful until" and "parted at", drawn on the plot and marked on both views and the
  playhead; otherwise "did not part by `T`".
- **The starting gap** `|Δr|`, `|Δv|` at `t = 0` (the f32 run starts from the f32 decode).
- **Fates side by side** (outcome, escaper, `t_end`), with one of: "settled before the runs parted"; "agree, but after
  parting: not evidence"; "not determined at f32 here".
- **The trust line**, about the atlas pixel at the atlas playhead `t_p`:
  - before `t_part(f32)`: "The atlas colour here is within f32's trustworthy time for this point."
  - after: "The atlas colour here is past f32's trustworthy time (parted at `t = 18.4`). It shows a possible outcome,
    not this point's." — the human's example: "the atlas shows t = 30; this point's f32 run parted at t = 18".
  - after `t_part(f64)` too: "Past f64's as well: neither run can be trusted here."
  - beside it the estimate `ln(1/eps₃₂)/ftle`, "estimate from FTLE", against the measured parting: a point check of the
    field REQ-SYS-061 states. Its `ftle`, and the self-check's `λ` (§3.2), is the one the IC's own f32 one-pixel
    dispatch writes under its sim key's FTLE tier (render contract `SimState`; lowering contract Part 3,
    `sample.ftle`): for an atlas pixel of the current view bitwise the atlas's own, because the run is; for any other
    IC (§3.4) that IC's own, never the atlas pixel it was edited from or the pixel now under it. When the sim key's
    FTLE tier is off or `ftle_valid` is false (R-254), the line shows no estimate and no self-check verdict, never a
    NaN or a neighbour's value, and states only the measured marks (physics review 5627518149 of PR #283, F7; applied
    per R-369).
  - "unreliable" when the self-check fails (§3.2).

**The wording that ships** is the three forms above (REQ-GUI-370); the human's "the atlas shows t = 30; this point's f32
run parted at t = 18" is the human's illustration of them, not a fourth form (gui review G9).

*Applied per R-369 (the port):* the trust line and the fates' wording key on the "parted at" mark, as the draft's
single threshold did and the human's example does; "faithful until" is drawn and reported beside them ("faithful
until `t = …`").

### 3.4 Where it lives

**The trust line shows in the trajectory inspector panel when a trajectory is clicked** (the human's words, §5): the
click inspector's panels, hosted in the Inspector window (§G8) and Explore's Trajectory panel (§G2) (R-65). The four
runs and their readouts are on by default in the Inspector; in the Trajectory panel, on for a pinned or kept
trajectory (a click pins one, R-453), off while the panel follows the cursor. The toggle and the two marks are
`ViewUI`. An exported Inspector figure carries all four runs' settings.

**In the Trajectory panel** (gui review 5627122883 of PR #283, G6; applied per R-369): a collapsible **Precision**
section, after Numbers in R-541 G4's panel order (render_gui_spec §G2). Collapsed it is one row, the trust line; expanded
it holds the marks, editable, the separation plot, the starting gap and the fates side by side; the f32 run is dashed
over the f64 in the panel's previews. The four runs and every readout are the same as the Inspector's.

**Whatever the IC's source** (gui review G7; physics review 5627216712 of PR #283, F5; applied per R-369): the four
runs run from the clicked trajectory's own IC, under the trajectory's own sim key (its occupant and coefficients, never
the current view's).
- **An atlas pixel of the current view**: the f32 run is the survey kernel's one-pixel dispatch through the atlas's
  decode path, bitwise what coloured the pixel, and the trust line takes REQ-GUI-370's three forms at the current atlas
  playhead `t_p`. An atlas pixel needs a live view: while the view source is `Result` (`principia_results.md` §3.4),
  no pixel is an atlas pixel. A **kept orbit** counts as an atlas pixel only if it was kept from an atlas pixel of a
  live view (the view source `Live` when it was kept), and only while the view it was kept from is unchanged: the view
  source `Live` now, the same sim key and the same chart view (`z₀`, the basis and the zoom level),
  bitwise; a result opened over it, even one whose `SimConfig` is bitwise the same, changes it. A stain or playhead
  edit leaves it unchanged; a pan, zoom, slice, chart or sim-key edit changes it (gui review 5627389542 of PR #283, R1;
  applied per R-369). An orbit kept in a result view is never an atlas pixel, even after *Explore from here* loads the
  result's `SimConfig` bitwise: its IC is a grid cell's, not one decoded through the atlas's path (gui review
  5627439632 of PR #283, R2; applied per R-369).
- **Any other IC** (one edited in the Inspector, one built in Create & locate, possibly off the slice, a kept orbit
  once its view has moved, or a pixel clicked in a result view, whose IC is its grid cell's and whose runs are under
  the result's own `SimConfig`: an f64 result's colour, and an f32 result's uniform grid with refinement off, are not
  the atlas's decode path; R1): there is no decode path, so the f32 run is the same survey kernel's one-pixel dispatch
  from the IC rounded to f32 (Cartesian), the nudge following the rounding and the starting gap being the f32
  rounding. The trust line makes no atlas claim, never "The atlas colour here …"; its forms mirror the atlas forms one
  for one, "these runs" in place of "the atlas colour here" (gui review 5627280803 of PR #283, N1; applied per
  R-369):
  - before `t_part(f32)`: "Not an atlas pixel: these runs are within f32's trustworthy time for this point."
  - after it: "Not an atlas pixel: these runs are past f32's trustworthy time (parted at `t = …`). The f32 run shows a
    possible outcome, not this point's."
  - after `t_part(f64)` too: "Not an atlas pixel: these runs are past f64's as well: neither run can be trusted here."
  - read at the same time as the atlas forms, the current atlas playhead `t_p`, against the same marks; beside it the
    same "estimate from FTLE" and, when the self-check fails, the same "unreliable", both from the `ftle` this IC's own
    f32 dispatch writes, and neither when its FTLE tier is off or `ftle_valid` is false (§3.3; F7).

*Was:* "**Whatever the IC's source** (gui review G7; applied per R-369): the four runs and the trust line run from the
clicked trajectory's own IC, whether it came from an atlas pixel, an Inspector edit, Create & locate, or a kept orbit
after the view has moved. The f32 run is the survey kernel's one-pixel dispatch of that IC through the same decode
path, so for an atlas pixel of the current view it is bitwise what coloured the pixel; the trust line reads the current
atlas playhead `t_p`." (F5: the decode path does not exist for an IC with no pixel, and the atlas wording would be false
for it.)

### 3.5 Nothing changes in the atlas

**The atlas stays f32 on the GPU, coloured as now** ("Why would we show different colours in the atlas for
precision?"). The draft's stain preset **How far f32 sees** is not made, and the draft's header text for the atlas
("Horizon `T = 50` · f32 trustworthy to about `t ≈ 23` (derived, not yet measured on this GPU)") is not applied: the
human decided "The header states the horizon; the viewer states each point's parting." What the header strip states,
against REQ-SYS-061's "The interactive atlas must state its predictability horizon", is RQ-556.

### 3.6 The horizon

The default horizon stays 50, and it can be **raised or lowered**, not floored at 50. This amends the Run window's
`T_horizon ∈ [50, 200]` (render_gui_spec § "Run — from the top bar") and R-132 (R-575). The lowest horizon, and
whether 200 still caps it, are RQ-555; meanwhile `T_horizon ∈ (0, 200]`, default 50, with `dt_macro =
max(10⁻³, T/65535)` unchanged.

### 3.7 The README line

> **Honesty** — *It shows where its precision runs out: open any point to watch its single- and double-precision
> orbits part, and when. Failed runs are kept and counted, never painted over.*

(R-467, R-470: failures draw `#141418`; the hatch lives in the debug view.)

## 4. Fit with rulings

- **Amends** trajectory viewing §5 (on by default in the Inspector; the numbers, real space, the nudged runs) and §4
  ("no comparison claimed" for real space). REQ-VAL-106 and REQ-TOOL-127 follow.
- **Within R-93:** that gate is about the automated cross-check; this is a per-point display reporting parting.
- **R-84:** precision-dependent labels on chaotic trajectories are expected; the panel says so.
- **Never discard:** a failed or budget-exhausted run is shown as such, with its end time (R-563).
- **R-132** is amended as to the horizon's range (R-575).

## 5. Decided (Malachy, 21 Oct 2026), superseding §3 of the draft where they differ

The human's words, verbatim from the design's "Decided" section:

> - **Four runs, not three**: f32, f64, and **each nudged by one ulp** ("Do fp32 nudged the smallest amount also").
>   f32 against nudged f32 gives f32's own sensitivity at this point.
> - **Two marks, measured and from the literature**, both shown and editable:
>   - **Faithful until**: f32 and f64 within a shape-sphere chord of **10⁻³**, the reliability threshold of Boekholt &
>     Portegies Zwart (2015, 2020, 2023), stated as keeping the difference in the linear regime.
>   - **Parted at**: chord **0.1**, the decorrelation threshold (separation at the system's own scale: Boekholt 2015,
>     Hernandez et al. 2020); random shapes average 4/3.
>   - Measured over 1000 ICs, four regions, Heggie to t = 50 (prin-rs PR #34, `parting_probe`): first-event fates agree
>     **99.9%** before the 10⁻³ mark and **25%** after the 0.1 mark (n = 16, so thin). Median parting at 0.1: deep
>     interior 12, preset_shape 26, near-field 43, config_stability 39 (14% parted), pooled 22. A single global "f32
>     trustworthy to ~23" is right pooled and wrong for any given point, which is why the readout is per point.
>   - Self-check: in a chaotic region the gap between the two marks is about ln(100)/λ. A gap far off that says the
>     reading is in the noise floor or saturation, and the panel says "unreliable".
> - **Where the trust line shows**: in the trajectory inspector panel when you click a trajectory, e.g. "the atlas shows
>   t = 30; this point's f32 run parted at t = 18". **Nothing changes in the atlas**: it stays f32 on the GPU, coloured as
>   now ("Why would we show different colours in the atlas for precision?").
> - **Horizon**: default stays 50, and it can be **raised or lowered**, not floored at 50 ("Keep horizon to 50. (Also
>   horizon shouldn't be capped to 50 should be able to raised or lowered)"). Amends the Run window's `T_horizon ∈ [50,
>   200]` (render_gui_spec §G5) and R-132. The header states the horizon; the viewer states each point's parting.

The draft's open questions, as closed: (1) the threshold, two marks, 10⁻³ and 0.1, visible and editable; (2) the
nudged f64 run, always, and the nudged f32 run with it; (3) which copy, the nominal (the draft's "copies on request" is
not taken up); (4) the default horizon against ≈ 23, the default stays 50 and the horizon can be raised or lowered.
*Applied per R-369 (the port), question 3:* the human's texts do not take up "copies on request", so the runs are of
the nominal copy, as the f32 run must be to equal the atlas pixel's.

## 6. Tests (each with what makes it fail)

- **same_run_never_parts**: f64 against itself: the chord is exactly 0 throughout. *Fails if* the harness adds a
  difference.
- **regular_orbit_does_not_part**: figure-eight: every pair reads "did not part by `T`" at the 0.1 mark. *Fails if* the
  mark fires.
- **chaotic_orbit_parts**: Burrau: `t_part(f32)` and `t_part(f64)` both found, f32's earlier. *Fails if* either is
  missing. The fixture's horizon may need to exceed 50 (up to 200) for f64–f64′ to part: in the probe it parted by
  50 in only 5 % of ICs (19 % in the deep interior); the task records the horizon it uses.
- **nudge_is_one_ulp**: each of the twelve components of the nudged IC is `next_up` of the run's own, at its precision.
  *Fails if* a component is unmoved or moved by more (control: a nudge by `ε·|x|` fails on a component near a power of
  two).
- **f32_run_is_the_atlas**: the f32 run's outcome and `t_end` equal the atlas pixel's nominal copy bitwise. Control: a
  CPU f32 run differs on a chaotic fixture, so emulation would fail it.
- **non_atlas_f32_start** (REQ-VAL-196; physics review 5627518149 of PR #283, observation; applied per R-369): for an
  IC edited in the Inspector, a pixel clicked in a result view and an orbit kept in a result view, the f32 run starts
  from the IC rounded to f32 (a result-view pixel's from its grid cell's IC, under the result's `SimConfig`), its
  starting gap the rounding. *Fails if* the run re-decodes a pixel through the atlas's decode path or takes the current
  view's sim key.
- **trust_line** (REQ-GUI-370; the same observation): an atlas pixel reads the three atlas forms and any other IC
  (§3.4) the three "Not an atlas pixel:" forms, compared as whole strings, with the estimate from its own dispatch's
  `ftle` and none when `ftle_valid` is false. *Fails if* the atlas wording shows for a non-atlas IC, a line is the
  bare prefix, or the estimate is read from the atlas pixel an IC was edited from.
- **marks_follow_edits**: editing a mark to 10⁻² moves "parted at" to the first boundary above 10⁻² on a recorded
  chord series. *Fails if* the mark is a constant.
- **ensemble_ratio**: over ≥ 256 chaotic ICs, the median `t_part(f32)/t_part(f64)` near `ln eps₃₂ / ln eps₆₄ ≈ 0.44`.
  Never read off one trajectory. Its expected value and tolerance are RQ-557 and REQ-VAL-199.
- **agreement_wording**: the same fate latched after parting reads "not evidence". Mutation: drop the before/after
  check and it reads "settled": fails.
- **self_check_unreliable**: with Δ = 0.1, a chord series whose two marks are one boundary apart (a measured gap of
  0.1) at λ = 0.7, where `ln(100)/λ ≈ 6.6` is expected, reads "unreliable" (the noise floor or saturation). Control:
  the same gap at λ = 46, where `ln(100)/λ ≈ 0.1` is expected, reads its times, not "unreliable". *Fails if* the
  self-check is absent, or keyed the wrong way round on λ. *Was:* "a chord series whose two marks are one boundary
  apart at a large FTLE reads "unreliable". *Fails if* the self-check is absent." (backwards: at a large FTLE that gap
  is the expected one; physics review 5627073272 of PR #283, F1).
- **atlas_unchanged**: the atlas frame is bitwise the same with the viewer's runs on and off. *Fails if* the viewer
  writes to the atlas's colours or stains.
