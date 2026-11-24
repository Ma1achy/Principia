# Image artefact triage

What to do when a rendered field looks wrong. From pitfalls §1 (the patchwork) and §1.7. Work through it in order. Each
step is a measurement; write down the number before you form a theory.

## 1. Look for the three signatures (pitfalls §1.7)

- **Smooth, clean geometric boundaries in a chaotic field.** Fractal boundaries are ragged. Circles, straight edges
  and smooth arcs are level sets of something smooth, usually a termination time. A real basin boundary is sharp but
  ragged; a stratum seam is sharp and clean.
- **Structures that start and stop.** A continuous feature (a ribbon) broken into segments is a field sampled at
  inconsistent times, not a field with gaps.
- **Two implementations agreeing where tame and diverging where not.** That puts the fault in logic, not physics.
  It is the strongest single clue there is.

If none of the three is present, the image is probably physics: say what you measured and stop here.

## 2. Is the boundary a level set of `t_end`?

Render `t_end` for the same view, with nothing else changed, as greyscale. `cargo xtask golden patchwork` writes it
as `t_end_stop.png` for the zero view. Lay it over the flagged image:

- Do the suspicious boundaries run along `t_end` contours (lines of one grey level)? If they do, the field is a
  patchwork of time strata. Every pixel froze its displayed quantity at its own `t_end`.
- Count the distinct `t_end` values among the terminated pixels (`t_end_histogram.csv`). Around 32 values, one per
  old sync boundary, is the patchwork's own fingerprint (pitfalls §1.4).
- Run the seam check (`validation::stop_images::seams`): it counts 4-neighbour pairs on different `t_end` strata
  that have a hard edge in the stopped image where the reference is smooth.

## 3. Run the independent occupant on the same ICs

Render the same view (same chart, window, horizon and decode) with a second, independent occupant: an unregularised
stepper (KDK or Yoshida-4) beside the regularised default (Heggie × RK4), or the RK45 reference in the Precision ring.
Then compare:

- **Tame regions** (smooth gradients, ribbons, slowly varying outcome) must agree. If they do, the physics core is
  right, and a discontinuity there is logic: termination, freezing, colouring or chart.
- **Wild regions** (fractal basins, chaotic scattering) may disagree pixel by pixel. That is sensitivity, not a bug.

## 4. Change one variable

Diagnose with a controlled experiment, not an argument (pitfalls §1.3). Render twice with exactly one variable
changed. Use `cargo xtask golden repro <suite>/<case> --vary <field>=<a>,<b>` for an image suite. For termination, use
`cargo xtask golden patchwork`, which compares the live march with the stop option. Close a fix only against the
symptoms it moved.

## 5. Rule out the usual wrong theories with one measurement each (pitfalls §1.2)

- **8-bit quantisation** steps by about 3 and makes monotone staircases. It never makes an isolated bright line.
  Read the RGB values along a line across the feature.
- **The colour map.** If the structure survives event, greyscale and hue modes, it is not colouring.
- **Chart geometry** (an energy circle, an angle wedge). Predict the radius or angle and measure it. "Right family,
  wrong instance" is not a match.
