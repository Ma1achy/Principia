# Valtonen et al. 2016: a working reference for Principia

M. Valtonen, J. Anosova, K. Kholshevnikov, A. Mylläri, V. Orlov & K. Tanikawa, *The Three-body Problem from Pythagoras
to Hawking*, Springer, Cham (2016). doi:10.1007/978-3-319-22726-9; ISBN 978-3-319-22725-2 (print),
978-3-319-22726-9 (eBook). The PDF read is `valtonen2016.pdf`, 183 pages, sha256
`251029ab30bd0ad4f6284d706e42a1fec88510f1b5930af8fe3a06bd6ccf99cb` (see `../README.md`).

A record, not authority. The owner asked for the parts of the book Principia needs, written down so that nobody has
to reopen the PDF, with its figures and plots extracted.

## What this is, and what it is not

The book is © Springer International Publishing Switzerland 2016, **all rights reserved** (copyright page, PDF p. 5),
and this repository is public. So the folder is **our reference to the book, not a reproduction of it**:

- **Prose** is paraphrased and condensed. Short quotations appear only where the exact wording matters (a definition, a
  stated criterion, a claim we dispute); each is in quotation marks with its page.
- **Facts, numbers, equations, initial conditions, parameter values and statements of results** are recorded in full,
  with pages. The book has no numerical tables.
- **Figures.** Only images whose captions credit **Wikimedia Commons or Wikipedia** are committed (23 of 78), each with
  its credit and the licence the preface states (GNU FDL / GNU GPL v3; preface p. vi), and any change noted. Every other
  figure stays out. For those we record the source paper where the caption or text gives one, and for quantitative
  plots we **digitised** what can be read reliably into CSV and drew **our own re-plots**, labelled as such.
- A local PNG crop of **every** figure (78, at 300 dpi) was made for the owner and is **not** in the repository.

The book is a popular history. It gives **no integrator, resolution, horizon, escape criterion or time unit** for any
map or statistic. Its numbers are pointers to primary papers, not reference values. Where a primary paper in the
repository says otherwise, the files below say **CONFLICT** and the primary paper governs.

## Page numbers

Book page numbers are used throughout, with PDF pages where useful. The offset is **not** constant, because the PDF
omits two blank versos (book pp. 50 and 146):

| Book pages | Chapters | PDF page |
|---|---|---|
| 1–49 | 1–2 | book + 12 |
| 51–145 | 3–7 | book + 11 |
| 147–164 | 8 | book + 10 |

(`../TARGETS.md` previously said "PDF page = book page + 11 from Chapter 1 on"; corrected there.)

## Index

| File | Contents |
|---|---|
| `ch1_2_5_6_summaries.md` | Chapters 1, 2, 5, 6: short summaries, three-body facts only |
| `ch3_regularisation_orbits_statistics.md` | Chapter 3: regularisation, the Pythagorean problem, Mikkola's bounce orbit, statistical description, hierarchical systems |
| `ch4_maps_entropy_fractals_symbolic.md` | Chapter 4: the Agekian–Anosova map and its L/M/A/H partition, KS entropy, fractals, the lifetime maps, symbolic dynamics, triple collisions and reversibility |
| `ch7_perspective_frontiers.md` | Chapter 7: history, regularisation after KS, numerical studies, hierarchical stability limits, symbolic sequences, figure-eight and "line dance" |
| `ch8_relativistic.md` | Chapter 8: the relativistic three-body problem, the post-Newtonian Pythagorean problem, three black holes |
| `figures.md` | All 78 figures: page, content, source paper, credit, licence, and what was committed, digitised or re-plotted |
| `figures/` | The 23 freely credited images (`fig_*.jpg`) and our 5 re-plots (`replot_*.png`) |
| `data/` | Digitised data: `fig3_12_*`, `fig3_16_*`, `fig4_1_*`, `fig4_5_*`, `fig4_9_*`, `fig4_5to4_9_axis_windows.csv`, `fig8_1_*` (CSV headers state method, calibration and uncertainty) |
| `scripts/digitise.py` | Regenerates every CSV and re-plot from the PDF: `python3 -I scripts/digitise.py <valtonen2016.pdf> .` |

Each chapter file ends with **Replication hooks**, cross-referenced to `../TARGETS.md` **V-1 to V-11** (V-6 to V-11
added with this folder).

## What the digitisation found

- **Mikkola's near-Pythagorean bounce orbit** (Fig. 3.16), for which the book prints no initial conditions: start
  positions (0.926, 3.021), (−1.984, −1.057), (1.034, −0.977) for masses 3, 4, 5, ±0.018/±0.014; bounce positions
  (1.841, 0.876) and (−0.619, −0.291). Both sets have their mass-weighted centre at the origin within the read-off error.
- **The L/M/A/H partition** (Fig. 4.1), which the book never defines: as drawn, L/M is a circle of radius 1.385 about
  body b, H a circle of radius ≈ 0.535 about b, and M/A the line y = 0.4246 − 0.5026x (a–b = 2). Our description, not
  the book's definition.
- **The book's own maps**: Fig. 4.5 is mirror-symmetric about both axes to better than a pixel; Fig. 4.9 shows
  high entropy over 0.66–0.83 of the quarter (threshold-dependent), against the text's "roughly one half".
- **Valtonen 1976** (Fig. 8.1): marker positions in a centre-of-mass frame that is constant to 1 px; the caption's
  speeds conserve momentum.

## Conflicts with primary papers in the repository

Each is stated in full, with pages, in the chapter file named.

1. **Szebehely & Peters 1967** used **Levi-Civita** regularisation with a fifth-order Runge–Kutta integrator, not KS
   as the book says (book pp. 74–75; S&P pp. 881–883). `ch3`.
2. **Burrau's integration ended at t = 3.35** (S&P p. 878), not "half-way" through the first 10 time units (Fig. 3.13
   caption, p. 75). `ch3`.
3. **"Radioactive decay, with a known half-life"** (p. 77) against heavy, power-law lifetime tails: Orlov, Rubinov &
   Shevchenko 2010 (abstract) and Manwadkar, Trani & Leigh 2020. `ch3`.
4. **"The three-body problem was finally solved"** statistically (p. 80) against Stone & Leigh 2019 (agreement only for
   the ~50 % resonant encounters) and Trani et al. 2024 (regular trajectories occupy 28–84 % of phase space and defy
   statistical escape theory). `ch3`.
5. **Hausdorff dimension 2.1** (p. 92, Heinämäki and associates, unreferenced) against **D_H = 1.5** in Lehto et al.
   2008 — possibly different quantities; not resolvable from the repository. `ch4`.
6. **Figs. 4.5–4.7 are not Lehto et al. 2008's figures**: colour key reversed (book blue = short-lived; Lehto red =
   short, dark blue = long) and zoom windows different. Not a contradiction of a result, but `../TARGETS.md` V-2
   treated them as Lehto's lens. `ch4`.
7. **"About 50 %" of reversals fail** (p. 98): consistent with Boekholt, Portegies Zwart & Valtonen 2020 at
   double-precision accuracy, but Lehto et al. 2008's Fig. 4 shows ≈ 65 % unrecovered at their adopted δ = 5×10⁻⁵ and
   ≈ 52 % only at δ = 10⁻¹. A tension, not a contradiction. `ch4`.
8. **Heggie 1974 "showed first how this could be done for the four-body problem"** (p. 137): Heggie 1974 is a global
   regularisation of the **three**-body problem extending Aarseth & Zare 1974, generalised to N bodies. `ch7`.
9. **Post-Newtonian Pythagorean problem across mass units** (pp. 148–149): Chitan, Mylläri & Haque 2022 find, for the
   3–4–5 triangle, mergers already at 10⁵ M☉ with standard accuracy and only one merger in 2000 crossing times at
   10⁹ M☉; they report the 1995 study as merger-dominated from 8 × 10⁶ M☉. `ch8`.

**Internal inconsistencies of the book** (not against a primary paper): Belorizky's 10⁸⁰,⁰⁰⁰ terms are for one sixth of
the Lagrange triangular period (p. 6) or one orbital period of Sun–Jupiter–Saturn (p. 136); Sundman 1912 (p. 6) or 1913
(p. 136); OJ287's primary 18 billion (p. 152) or 17 / 18.3 billion (Fig. 8.6); Fig. 4.9's brown share against "roughly one
half" (p. 96). Book claims not supported by the cited primary: the Pythagorean final binary's "very high eccentricity"
(not stated by S&P 1967; AAOS 1994a gives ⟨e⟩ = 0.85 over the vicinity).

**Checked and consistent**: the Pythagorean set-up and outcome (S&P 1967); the escaper is usually the lightest body
(Anosova 1969); region D's construction (Heinämäki et al. 1999 Fig. 1); the convex-hull KS-entropy method
(Heinämäki et al. 1999); two-axis symmetry of the rest-surface map (Lehto et al. 2008); the figure-eight history
(Moore 1993; Chenciner & Montgomery 2000). The book quotes **no AAOS escape time or crossing time**, so the AAOS
1994a/1994b crossing-time question recorded in `../README.md` §5 cannot be tested against it.

## What could not be read reliably

- **Colour scales**: none of Figs. 4.5–4.7 and 4.9 has one, so lifetimes and entropies cannot be digitised; only colour
  classes, whose shares depend on the threshold (stated in the CSVs).
- **Fig. 8.2** (576 escape velocities): points overlap; not digitised.
- **Figs. 3.13–3.15** (Szebehely & Peters orbits): not digitised, because the primary paper is in the repository.
- **Fig. 3.20** (Bohlin) and **Fig. 7.2** (line dance): no length scale.
- **Fig. 3.16 tracks** are an unordered pixel cloud; where tracks cross, pixels may be assigned to the wrong body.
- Images are 150 ppi JPEGs; every position carries ±1 px of that grid (stated per CSV).
