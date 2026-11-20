# Chapter 4, "Fractals, Entropy and the Arrow of Time": the Agekian–Anosova map, entropy, fractals, symbolic dynamics, triple collisions

Valtonen et al. 2016, book pp. 85–98 (PDF pp. 96–109; PDF = book + 11 here). Our words; quotations marked and
paged. This is the chapter closest to Principia: the equal-mass free-fall problem on region D.

The book spells the name "Agekian"; the repository's papers use "Agekyan". Same person.

---

## 1. The map (pp. 85–87; Fig. 4.1)

**Set-up, as stated** (pp. 85–86): three equal masses, starting from rest. Bodies **a** and **b** on the horizontal
axis "two units apart", chosen as the most distant pair; the third body then lies anywhere inside the "curved
triangle" b–c–d. These starting positions, "first introduced by Agekian and Anosova, cover all possible
configurations" (p. 86).

**Geometry as drawn** (Fig. 4.1, axes read off the figure): a = (−1, 0), b = (1, 0), c = (0, 0), d = (0, √3); the
outer boundary b–d is the arc of radius 2 about a. Our digitisation puts that arc at **1.9982 ± 0.0034** from a,
which checks the calibration. This is region D at **twice** Agekyan & Anosova's scale (their bodies at (−½, 0),
(½, 0)), the same convention as Heinämäki et al. 1999, Fig. 1 (in the repo), whose caption defines the map: the
triangle is rescaled so that the longest side is ab and "it meets the shortest side at point b".

**The four regions** (Fig. 4.1 caption, p. 86): L near-Lagrangian (equilateral), M middle, A nearly aligned,
H hierarchical (third body close to b). The book does not define the boundaries; Chapter 7 attributes the categories
to Chernin and colleagues and defines them in words only (p. 141; see `ch7_perspective_frontiers.md`). Heinämäki et
al. 1999 (Fig. 1 caption) also define them in words only.

**Digitised boundaries** (`data/fig4_1_region_boundaries.csv`, fits in `data/fig4_1_boundary_fits.csv`, re-plot
`figures/replot_fig4_1_region_partition.png`). These fits are **our description of the drawing, not the book's
definition**:

| boundary | drawn as | our fit (a–b = 2) |
|---|---|---|
| L/M | dash-dot curve from (0.02, 0.98) to the outer arc near (0.49, 1.29) | distance from b = **1.3852 ± 0.0023** (64 px); i.e. a circle about b |
| H | dotted curve from (0.45, 0.03) to the outer arc near (0.89, 0.52) | distance from b = **0.535 ± 0.010** (117 px); a free circle fit gives centre (0.965, 0.020), r 0.498 |
| M/A | dash-dot line from (0, 0.42) to the H curve near (0.49, 0.18) | **y = 0.4246 − 0.5026 x**, rms 0.0028 (104 px) |

Pixel size 0.0055 (x) by 0.0057 (y). In units where a–b = 1, the L/M radius is 0.693 and the H radius 0.27.

**Example starts** (asterisks, ±0.006): 1 (M) (0.498, 0.963); 2 (L) (0.049, 1.502); 3 (A) (0.075, 0.219);
4 (H) (0.749, 0.249) (`data/fig4_1_example_points.csv`). They are **not** at the four drop positions of Heinämäki
et al. 1999 Fig. 1, where drop 3 sits on the M/A line and drop 4 on the H boundary. So Fig. 4.1 is a redrawing, not
a reprint.

**The map as phase space** (p. 86): after each step, recalibrate so that the two most distant bodies stay at a and b;
the evolution is a curve in the curved triangle. On escape the representative point "dives toward the right hand
corner (b)", the escaper being at a.

**The convex-hull definition of the arrow of time** (p. 87): start about 100 systems inside a small circle around one
point, follow them, draw the minimal convex curve round the cloud, and plot its area against time; "the arrow of
time … is the direction toward which the area increases". On a logarithmic scale this is the Kolmogorov–Sinai
entropy. This is the method of Heinämäki et al. 1999 (in the repo), who use 100 states per drop spread along a
segment of length 10⁻³, and the convex hull of the drop (their §§2–4).

## 2. Entropy and Kolmogorov–Sinai entropy (pp. 87–91)

- Thermodynamic background (Boyle, Mayer, Joule, Clausius, Carnot, Gibbs, Boltzmann, Eddington; pp. 87–89). Nothing
  numerical about three bodies.
- KS entropy: Kolmogorov and Sinai, **1958** and **1959**; a simplified form by **Zaslavsky (1985)**, *Chaos in
  Dynamical Systems*, used by Chernin and associates for the three-body problem (pp. 89–90).
- Definition used (pp. 90–91): follow nearly identical systems as points in the phase plane (the map is "an example
  of a simple phase space"); the area enclosing them, after taking a logarithm, is the entropy. "As soon as the
  universe has at least three particles, the Arrow of Time appears" (p. 91).

## 3. Fractals (pp. 91–93)

- Hausdorff (**1918**) and non-integer dimension; Mandelbrot coined "fractal" in **1975**; west coast of Britain
  **D = 1.25** (p. 91). Koch curve: length multiplies by **4/3** per step (p. 92). Sierpiński gasket (1915).
- **The three-body dimension** (p. 92): "In the three-body problem mentioned above (equal-mass, free fall problem of
  Agekian and Anosova) the embedding dimension is three. However, due to fractality, the Hausdorff dimension
  D = 2.1, as was discovered by Pekka Heinämäki of the University of Turku and his associates." **No reference is
  given.** The paragraph that follows introduces intermittency, the subject of Heinämäki, Lehto, Valtonen & Chernin
  1998, MNRAS 298, 790, doi:10.1046/j.1365-8711.1998.01661.x ("Three-body dynamics: intermittent chaos with strange
  attractor"; Crossref; cited by Heinämäki et al. 1999), which is the likeliest source. **Not in the repo.**
  - **CONFLICT, or two different quantities: not resolvable from the repo.** Lehto et al. 2008 (in the repo, same
    Turku group) give **D_H = 1.5** for "our image", i.e. the two-dimensional lifetime map, from p_ε ∝ ε^−0.5 (their
    §2.3). The book's 2.1 is in an embedding dimension of three. Which set the 2.1 measures (the book says the
    final-state space, see p. 93 below) is not stated.
- **Intermittency** (pp. 92–93): long stretches of chaotic "interplay" then an "ejection" (Szebehely's terms); an
  escape is an ejection that does not return.

## 4. Chaos in the three-body problem: the lifetime map (p. 93; Figs. 4.5–4.7)

- After escape the escape speed, the binary's eccentricity and its size tend to constants; "These numbers fully
  determine the final state of the system (thus the embedding dimension is 3)" (p. 93).
- **Lifetime map**: the lifetime before escape is coloured at the starting position of the third body, pixel by
  pixel; adding the symmetric copies of the curved triangle gives the "Maasai shield" (p. 93).
- Lehto and associates (Turku) find "typically it is not possible to predict the color of the pixels surrounding a
  given pixel. Only in exceptional areas the color is uniform", with coloured bands overlying random colouring, and
  the same on zooming (p. 93).
- This, the book argues, justifies random sampling, and the ergodic hypothesis gives "excellent statistical models"
  (p. 93). (See the Trani et al. 2024 conflict in `ch3_regularisation_orbits_statistics.md` §4.)

**Fig. 4.5** (p. 94): the whole shield, x ∈ [−1, 1], y ∈ [−√3, √3]. "Blue means short-lived, yellow long-lived."
The caption states reflection symmetry about **both** axes, from equal masses starting at rest. **No resolution,
horizon, escape criterion or colour scale is given.**

**Figs. 4.6–4.7** (p. 95): the quarter x ∈ [0, 1], y ∈ [0, √3], and three nested zooms, whose windows we read off
the tick labels (`data/fig4_5to4_9_axis_windows.csv`):

| panel | x (book) | y (book) | width | x (Lehto) | y (Lehto) |
|---|---|---|---|---|---|
| 4.6 right | 0.40–0.60 | 0.90–1.10 | 0.2 | 0.70–0.80 | 0.45–0.55 |
| 4.7 left | 0.500–0.520 | 0.910–0.930 | 0.02 | 0.750–0.760 | 0.455–0.465 |
| 4.7 right | 0.5080–0.5100 | 0.9140–0.9160 | 0.002 | 0.7540–0.7550 | 0.4570–0.4580 |

Lehto coordinates: x_L = (x + 1)/2, y_L = y/2 (Lehto et al. 2008 place the bodies at (0, 0) and (1, 0)). The windows
nest, each 10× (the first 5×) the next. The first window contains Lehto's "sea of apparent tranquility around
(x, y) = (0.7, 0.5)" (their §2.1).

**Are these Lehto et al.'s figures? No.** The book does not credit them. Lehto et al.'s Fig. 1 colours **red =
short-lived, dark blue = longest-lived** (over 13 initial crossing times); the book's colour key is the reverse
(blue short, yellow long), and Lehto's zooms (Figs. 2–3, pixel 2.5×10⁻⁴ and 3.125×10⁻⁶) do not match these windows.
So they are a separate rendering, probably by the same group. The source is **not identified**.

**Measured on the book's own Fig. 4.5** (embedded raster, 461×747 px, calibrated on the lens's own extremes, 201 px
per unit; method in `scripts/digitise.py`):

- *Symmetry.* Correlation of the blueness (B − R) with its own mirror image: **0.909** about the vertical axis and
  **0.916** about the horizontal (the latter peaking half a pixel off the nominal centre row). The correlation between
  a pixel and its immediate neighbour is only **0.78** (x) and **0.84** (y). So the figure is mirror-symmetric to
  better than one pixel, as its caption says.
- *Blue share.* The share of "blue" pixels in the lens is **0.71, 0.43 or 0.23** at B − R > 20, 40 or 60: entirely
  threshold-dependent. Recorded in `data/fig4_5_blue_fraction_raster.csv` (0.04 cells) as a colour class, **not** a
  lifetime.

## 5. Symbolic dynamics (pp. 93–96; Figs. 4.8–4.9)

- Coarse-grain the phase space into a few regions and record the sequence of regions visited; for the map the
  regions are H, A, M, L (pp. 93–94). Entropy can then be defined from symbol frequencies or transition frequencies
  (p. 94). Pioneered by **Alexeyev (1969)** (p. 94).
- **Sitnikov problem** (pp. 95–96; Fig. 4.8): equal primaries on Kepler orbits; a massless body moving on the axis
  through the centre of mass perpendicular to their plane. Alexeyev proved that for any chosen sequence (e.g. 9, 4,
  5, 2, …) there is a solution spending that many primary periods alternately above and below the plane, including
  capture from and escape to infinity (p. 96).
- **Entropy over the four-region partition** (p. 96; Fig. 4.9, p. 97): Chernin and associates "(including AM and VO)"
  computed the entropy of sequences such as MHMLMAHML…; "The short-lived systems in the Maasai shield correspond to
  low entropy. Low entropy orbits are in principle reversible. Long-lived systems have high entropy, and time
  reversal does not succeed with them. Roughly one half of the map is covered by non-reversible systems" (p. 96).
  Fig. 4.9 colour key: low entropy blue, high entropy light brown. **No entropy definition, sequence length or
  scale is given.** Probable source (not cited by the book): Mylläri, Orlov, Chernin, Martynova & Mylläri,
  "Symbolic Dynamics, Mixing and Entropy in the Three-Body Problem", *Open Astronomy* 25, 254 (2016),
  doi:10.1515/astro-2017-0127; or Mylläri, Martynova, Orlov & Chernin, *Serdica J. Computing* 1, 425 (2007),
  doi:10.55630/sjc.2007.1.425-432 (both found on Crossref, neither read, neither in the repo).
  - **Measured on the book's Fig. 4.9** (461×775 px, calibrated on the quarter's own extremes, 420 px per unit):
    the brown (high-entropy) share of the quarter is **0.83, 0.78 or 0.66** at R − B > 0, 25 or 60; blue is 0.17,
    0.14 or 0.11 (`data/fig4_9_brown_fraction_raster.csv`). **Internal tension:** if brown is "non-reversible", the
    figure shows two thirds to four fifths of the map, not "roughly one half". The colour scale is continuous and
    unlabelled, so where "non-reversible" begins cannot be read; the tension is recorded, not resolved.

## 6. Triple collisions and the arrow of time (pp. 97–98)

- Forward-then-reverse integration of systems like the Pythagorean problem: "When you try to do it, you will find
  that in about 50 % of cases you fail" (p. 98); better computation recovers more, "but qualitatively there is a
  difference with the forward and backward sense of time". **No accuracy, criterion or sample is given.**
  - *Consistent:* Boekholt, Portegies Zwart & Valtonen 2020 (in the repo), for the free-fall map: "For an accuracy
    close to double-precision, the fraction of irreversible solutions is about half, consistent with the results of
    Lehto et al. (2008)" (their §2, Results), the fraction falling as a power law with accuracy, to "up to five percent" of
    massive-black-hole triples needing better than Planck-length accuracy (abstract).
  - **Tension with Lehto et al. 2008** (in the repo), Fig. 4 (p. 5), read off by us at ±0.01: at their adopted
    recovery tolerance δ = 5×10⁻⁵ the unrecovered fraction is **≈ 0.65** (recovered ≈ 0.35); it falls to ≈ 0.52 (recovered ≈ 0.48) only at
    δ = 10⁻¹, the largest tolerance plotted. "About half" is therefore a statement about a tolerance, not a property of the problem.
- Triple-collision points exist inside the map; double collisions can be continued by an elastic bounce; the triple
  collision is "the ultimate barrier for time reversal" (p. 98). Sundman had to exclude triple collisions (p. 98).
- Converging representative points of 100 systems would mean the film is running backwards (KS entropy cannot
  decrease) (p. 98).

---

## Replication hooks

1. **Region D at the book's scale** (a–b = 2): Principia's Anosova entry scaled by 2. Check the digitised outer arc
   (radius 2 about a) as a unit test of the remap. → **V-1**.
2. **L/M/A/H colouring from the digitised partition**: L/M as |r₃ − b| = 1.385, H as |r₃ − b| < 0.535, M/A as
   y = 0.4246 − 0.5026x (a–b = 2). Use it to build symbol sequences and an entropy map, then compare the high-entropy
   share with Fig. 4.9's 0.66–0.83 and the text's "roughly one half". → **V-7** (new), **V-3**.
3. **The shield's two-axis symmetry, exactly**: a full-lens render must be mirror-symmetric to rounding; the book's
   own figure is symmetric to better than a pixel (0.909 / 0.916 against neighbour correlations 0.78 / 0.84). →
   **V-2**, **V-8** (new).
4. **The zoom ladder of Figs. 4.6–4.7**: render the three windows (book or Lehto coordinates above) and check that
   the banded/speckled texture survives at 10× and 100×. → **V-8** (new); compare with Lehto's ε-ladder, **L-3**.
5. **Reversal fraction against tolerance**: reproduce the shape of Lehto's Fig. 4 (unrecovered 0.71 → 0.52 over
   δ = 5×10⁻⁶ … 10⁻¹) rather than the book's single "about 50 %". → **V-5**, **L-4**.
6. **Lifetime distribution over region D** (exponential or heavy-tailed). → **V-9** (new).
