# Chapter 3, "From Comets to Chaos": regularisation, the Pythagorean problem, statistics, hierarchies

Valtonen, Anosova, Kholshevnikov, Mylläri, Orlov & Tanikawa 2016, *The Three-body Problem from Pythagoras to
Hawking*, Springer, doi:10.1007/978-3-319-22726-9. Chapter 3 is book pp. 51–84 (PDF pp. 62–95; PDF = book + 11 in
this chapter). Written in our own words; quotations are marked and paged. See `README.md` for the copyright
approach.

The book is a popular history. It states **no integrator, tolerance, horizon, escape criterion or unit of time** for
any result in this chapter. Every number below is the book's; where a primary paper in the repository says
something different, that is flagged **CONFLICT** and the primary paper governs.

---

## 1. Before regularisation (pp. 51–71), three-body facts only

- **Great Inequality of Jupiter and Saturn** (pp. 51–52). Laplace (1785) explained it as the near 5:2 commensurability:
  Jupiter laps Saturn five times for two Saturn orbits, so one conjunction in five falls at the same orbital place
  and accumulates. The resulting exchange of orbital speed is periodic with period **929 years**.
- **Lunar apsides** (pp. 52–55). The Moon's line of apsides turns with a period of **9 years**; lunar eccentricity
  **0.05** (p. 53). Newton accounted for half of the motion; Clairaut in 1747 proposed adding an inverse-fourth-power
  term to gravity, then in 1749 recovered the motion with the inverse square law alone. His second-order theory gave
  **85 %** of the apsidal motion (p. 55). Delaunay's series had **460 terms over 53 pages**, with an apsidal error of
  about **1 part in 10,000** (p. 55). Brown's 1919 tables (from Hill's theory) were **4 s** late on the January 1925
  eclipse in New York (p. 56).
- **Apophis** (p. 67): impact probability for 2029 once computed as **2.7 %**; a **800 m** "keyhole" in 2029 decides a
  2036 impact; size **325 m**.
- **King Oscar's prize** (pp. 68–69): announced in *Acta Mathematica* in 1885, deadline the King's 60th birthday
  (1889), prize a gold medal and **2500 crowns**; **12** entries; Poincaré won. The corrected reprint cost him
  **3500 crowns**, leaving him **1000 crowns** out of pocket. The prize question is quoted in Chapter 7 (p. 134).
- **Sundman** (pp. 69–71): PhD (Helsinki, 1899) on the Sun–Jupiter–asteroid problem at the 2:1 resonance (Kirkwood
  gap at **3.3** Earth orbital radii). His series solution appeared in *Acta Mathematica* (1913 issue, printed mid-1912;
  over 100 pages). The book says the series is far too long to evaluate; Chapter 7 gives Belorizky's term count.

## 2. Regularisation (pp. 71–74)

- The problem: close two-body approaches destroy numerical accuracy (p. 71).
- **Levi-Civita**: complex coordinates for the plane; "The main limitation of his method was that it could not be
  applied in three dimensions" (p. 72).
- **Kustaanheimo–Stiefel (KS)**: presented by Kustaanheimo at Oberwolfach in **1964**, hours after Stiefel had called
  three-dimensional orbit computation the most urgent open problem (p. 72). Coordinates are mapped to quaternions
  (one real and three imaginary parts; three used for position, one left over), integrated, and mapped back. The book
  says accuracy *improves* at two-body encounters (p. 73). Adopted by NASA; general three-body studies with KS in the
  late 1960s (p. 73).
- Chapter 7 continues the line: Heggie 1974, Mikkola & Aarseth's chain (1989), Mikkola & Tanikawa's algorithmic
  regularisation (1999). See `ch7_perspective_frontiers.md` §2.

## 3. The Pythagorean (Burrau) problem (pp. 74–76; Figs. 3.12–3.16)

**Set-up, as stated.** Bodies initially **at rest** at the corners of a 3–4–5 right triangle, with masses **3, 4, 5**
proportional to the opposite sides (p. 74). Fig. 3.12 (p. 75) prints the starting positions:

| body | mass | x | y |
|---|---|---|---|
| P1 | 3 | 1 | 3 |
| P2 | 4 | −2 | −1 |
| P3 | 5 | 1 | −1 |

(`data/fig3_12_pythagorean_ics.csv`; printed labels, not digitised. Figs. 3.12–3.15 are Szebehely & Peters 1967,
AJ 72, 876, doi:10.1086/110355, their Figs. 1, 2, 7 and 8.)

**History, as stated.** Proposed by **Meissel (1893)**, who expected periodicity; computed by **Burrau (1913)**, who
found repeated close two-body approaches but could not continue (p. 74); completed by **Szebehely & Peters (1967)**
(pp. 74–76).

**Outcome, as stated** (p. 76): at one stage the two heavier bodies nearly collide while the lightest is almost at
rest, so that the motion would have reversed had the configuration been exact. Eventually "The body of mass 3 units
escaped from the two others which formed a binary system of very high eccentricity." Fig. 3.13: initial orbits over
**10 time units**; Fig. 3.14: **50–60**; Fig. 3.15: **60–70**. The Fig. 3.14 caption credits "JA and associates"
with finding that the configuration (third body falling through the binary's centre) "is especially explosive".

**Checked against Szebehely & Peters 1967** (`../../2026-10-25_replication/papers/szebehely_peters_1967_AJ_72_876.pdf`):

- **CONFLICT, method.** The book says S&P used "the KS-transformation" (p. 74) and "the KS-regularization method"
  (pp. 74–75). S&P used **Levi-Civita's** planar transformation (their system "L"), switched on when two bodies were
  closer than 2, with a fifth-order Runge–Kutta (Zonneveld) integrator (S&P pp. 881–883: the Levi-Civita equations on
  p. 881, "Method of numerical integration" on p. 882, Table II on p. 883). KS is not mentioned in their method.
- **CONFLICT, Burrau's extent.** The Fig. 3.13 caption says Burrau completed the initial 10-unit orbits "only
  half-way". S&P: "Burrau's integration ended at t = 3.35" (S&P p. 878), i.e. about a third of the panel.
- *Consistent:* the near-collision with the lightest body nearly stopped (S&P: t₁₀ = 15.8299236, r₂₃ = 4×10⁻⁴,
  v₁ ≈ 0.04; S&P pp. 878–879); m₁ = 3 escapes and m₂, m₃ form the binary; Alexeev's hyperbolic–elliptic conditions are
  met at t = 69 (S&P p. 879), which fits "the complicated orbits still continue 6 times longer" (Fig. 3.13 caption).
- *Not supported by S&P:* "very high eccentricity" for the final binary is not stated in S&P (they give its period,
  ≈ 0.9). AAOS 1994a (in the repo) gives mean final-binary eccentricity ⟨e⟩ = 0.85 over the vicinity of the problem,
  with about 60 % in [0.9, 1.0] (AAOS 1994a p. 13) — consistent in spirit.
- *Unchecked attribution:* S&P do not mention Meissel.

**Sensitivity** (p. 76): solutions diverge under tiny variations; Szebehely later parametrised the variation with an
adjustable number (no reference given).

**Mikkola's near-Pythagorean bounce orbit** (p. 76; Fig. 3.16, p. 79). Masses 3, 4, 5; starting positions "a little
different from the corners of the Pythagorean triangle"; the two heavy bodies collide, and with an elastic bounce
continuation the motion retraces itself, the third body being momentarily at rest at the collision. The book adds
that periodic orbits with a bounce "are actually quite common, but they have no correspondence in nature" (p. 76).
**No initial conditions are printed.** We digitised the figure (`data/fig3_16_mikkola_markers.csv`, tracks in
`data/fig3_16_mikkola_tracks.csv`, re-plot `figures/replot_fig3_16_mikkola.png`):

| marker | x | y |
|---|---|---|
| m3 start (magenta) | 0.926 | 3.021 |
| m4 start (cyan) | −1.984 | −1.057 |
| m5 start (yellow) | 1.034 | −0.977 |
| m3 at the bounce (black ring) | 1.841 | 0.876 |
| m4–m5 collision (brown ring) | −0.619 | −0.291 |

Uncertainty ±0.018 in x, ±0.014 in y. Two internal checks pass: the mass-weighted centre of the start markers is
(0.0009, −0.0042) and of the bounce markers (−0.0044, 0.0005), both zero within the read-off error, as they must be
for a system with zero total momentum started at rest. That the starts are a zero-velocity state is **our
inference** from "trace back the same trajectories … All bodies return to the original positions" (p. 76); the book
does not say it.

*Pointer, unchecked:* Szebehely & Peters published "A new periodic solution of the problem of three bodies", AJ 72,
1187 (1967), doi:10.1086/110398 (Crossref). Whether it is this orbit, and why the book credits Mikkola, is not
established; the paper is not in the repository.

## 4. Statistical description (pp. 76–81)

- **Agekian & Anosova** (late 1960s, St Petersburg) began computing three-body solutions from random initial states
  and studying the distributions of final states; Standish did the same in the USA for the break-up ("slingshot")
  (p. 76).
- Representative St Petersburg samples: "usually involved 1000 solutions for each type of a triple system"; "About
  95 % of the systems were found to become unstable and lead to an escape of one of the members within the practical
  time limit of computations. Usually the lightest body had escaped by this time" (p. 76). **No horizon, mass set or
  criterion is given.** (Anosova 1969, in the repo, also finds that the triple is usually left by the lightest
  component: consistent.)
- Individual orbits are sensitive; the statistics are stable: "Thus the decay of the triple systems resembles
  radioactive decay, with a known half-life" (p. 77).
  - **CONFLICT.** Orlov, Rubinov & Shevchenko 2010 (in the repo; Orlov is a co-author of the book): "Contrary to
    earlier expectations of similarity with 'radioactive decay', the lifetime distributions … turn out to be
    heavy-tailed", with differential power-law index about **−1.7 to −1.4** (abstract). Manwadkar, Trani & Leigh 2020
    (in the repo): only the *ergodic* interactions decay exponentially; Lévy-flight interactions give a power-law tail
    that dominates the lifetime distribution's tail (abstract and §4). The book's sentence describes only the
    exponential part.
- **Cambridge**: Aarseth set Heggie and Valtonen (with Saslaw) to compute large regularised samples; "Both projects
  gave the same result" (p. 79). The Valtonen–Saslaw project produced **25,000 orbit solutions** (p. 80).
- **Monaghan** then derived final-state distributions by statistical-physics methods; "Some of them worked out very
  well, others not so well". Later "MV, Seppo Mikkola and others showed that all the theoretical distributions agree
  well with orbit statistics, when impossible orbits are properly weeded out. In this sense the three-body problem
  was finally solved" (p. 80). The final state meant is a binary plus an escaper.
  - **CONFLICT (framing and scope).** Stone & Leigh 2019 (in the repo) present their ergodic outcome distribution as
    completing "the project initiated by Monaghan" and find good agreement only "so long as we restrict ourselves to
    'resonant' encounters (the ∼50% of scatterings that undergo chaotic evolution)" (their abstract). Trani et al. 2024
    (in the repo) find regular trajectories occupying **28 % to 84 %** of the phase space depending on the set-up,
    whose "outcomes defy the predictions of statistical escape theories" (abstract).
- **Time reversal of the Pythagorean problem** (p. 81): run backwards, the light body falls in, is captured, and the
  three come to rest at the Pythagorean triangle, then repeat the forward history. Whether a slightly different
  incoming orbit would also be "stuck" is the scattering problem: Heggie, then Heggie & Hut (Princeton, 1980s).
- **Exchange** (p. 81): an intruder C replaces binary member B; first shown by Ludwig Becker (Glasgow), **1920**, who
  used it to explain high binary eccentricities. MV later showed masses and sizes of binaries also follow from the
  statistical theory (no reference).

## 5. Hierarchical systems (pp. 82–84)

- Strömgren (**1909**) computed a short arc of a hierarchical triple with large masses; **Bohlin (1923)** completed
  **three revolutions** of the outer body, its closest distance to the binary centre being **4.7 times the binary's
  major axis**, and found it stayed hierarchical (p. 82; Fig. 3.20, from Bohlin's 1923 Stockholm publication).
- **Lidov (1961)** and **Kozai (1962)**: with the two orbital planes at right angles, the inner eccentricity evolves
  towards unity (p. 82). Marchal: even greater instability than thought; orbit flips from prograde to retrograde,
  possible at inclinations other than 90° (p. 82).
- **Harrington (1972)**: the third body should stay at least **3.5** inner semi-major axes from the binary centre;
  "considering all possibilities, the basic safety limit has to be doubled" (p. 84). Harrington 1972 is Celest. Mech.
  6, 322, doi:10.1007/BF01231475 (Crossref; not in the repo). For the Valtonen–Karttunen limits see Chapter 7.

---

## Replication hooks

1. **The Pythagorean problem from rest** at the Fig. 3.12 positions (`data/fig3_12_pythagorean_ics.csv`): escaper
   m₁ = 3, binary m₂ + m₃, closest approach r₂₃ = 4×10⁻⁴ at t = 15.83, escape conditions at t = 69 — all from S&P 1967,
   not the book. → **V-4** (and P-1/P-2 for the neighbourhood).
2. **Mikkola's bounce orbit**: start at rest from the digitised markers, integrate to the m₄–m₅ collision, and shoot
   on the three start positions (four free numbers once the centre of mass is fixed) for the collision to coincide
   with v₃ = 0; compare the bounce positions with the digitised rings. → **V-6** (new).
3. **Lifetime-distribution shape**: on the free-fall surface, does the decay histogram look exponential (book
   p. 77) or carry a power-law tail (Orlov et al. 2010; Manwadkar et al. 2020)? → **V-9** (new).
4. **"About 95 %" and "usually the lightest"** (p. 76): a pointer only; set-up unknown. → **V-5**.
5. **Hierarchical stability**: Harrington's 3.5 (and "doubled"), Bohlin's 4.7 major axes. → **V-11** (new).

*28 Oct 2026 (licence audit): `szebehely_peters_1967_AJ_72_876.pdf` is held locally, not in the repo, at `docs/incoming/_local/` (gitignored); the citations above stand, and `docs/incoming/2026-10-25_replication/README.md` keeps each file's row and sha256.*
