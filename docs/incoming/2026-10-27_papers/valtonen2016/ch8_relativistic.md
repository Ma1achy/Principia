# Chapter 8, "Black Holes and Quasars": the relativistic three-body and three-black-hole problems

Valtonen et al. 2016, book pp. 147–164 (PDF pp. 157–174; **PDF = book + 10** in this chapter, because book p. 146,
a blank verso, is absent from the PDF). Our words; quotations marked and paged.

Principia integrates Newtonian gravity. This chapter is recorded because its results are stated as departures from
the Newtonian Pythagorean problem, which Principia does compute, and because the mass-scaling statements can be
checked against a primary paper already in the repository.

---

## 1. The first relativistic three-body calculation (pp. 147–148; Fig. 8.1)

- MV, **1976**: the first general three-body calculation in General Relativity, with a regularisation "based on the
  previous work of Heggie" (p. 147). Statistically the character survives: one body escapes, but the remaining binary
  "is very temporary at best, and usually merges to make a single body"; from the centre of mass two bodies are seen
  escaping (p. 147).
- **Fig. 8.1** (p. 148; credit Valtonen, A&A 46, 435, 1976 — no DOI on Crossref): masses **m₁ = 0.5, m₂ = 0.5,
  m₃ = 1.0**; positions marked at times 1, 2, 3, 4; "At time 4.56 two black holes collide and the third black hole
  flies out with a speed of about 24,000 km/s. By recoil, the merged black hole flies out with speed of about
  8000 km/s." Units of time and length are not defined beyond a "1 UNIT" scale bar.
  - From the digitised tracks (`data/fig8_1_valtonen1976_positions.csv`): it is **m₂ and m₃** that meet at
    t = 4.56; m₁ leaves. Momentum check, ours: 0.5 × 24,000 = 12,000 = 1.5 × 8000. The caption's speeds conserve
    momentum if the merged mass is 1.5 (no radiated mass).
  - The mass-weighted centre of the digitised markers is constant to **1 px** (0.01 unit) over t = 1–4, so the figure
    is drawn in the centre-of-mass frame. Positions relative to it (figure units, ±0.005, scale ±1.5 %):

    | t | m₁ (0.5) | m₂ (0.5) | m₃ (1.0) |
    |---|---|---|---|
    | 1 | (0.752, 1.788) | (0.238, 2.542) | (-0.500, -2.168) |
    | 2 | (0.669, 1.856) | (-0.240, 1.716) | (-0.213, -1.774) |
    | 3 | (0.083, 1.765) | (-0.201, 0.915) | (0.061, -1.340) |
    | 4 | (-0.691, 1.072) | (0.042, 0.339) | (0.326, -0.715) |
    | 4.56 | (-0.966, 0.430) | (0.328, -0.139) | (0.289, -0.176) |

    (Copied from the CSV, which also holds the pixel values. Re-plot: `figures/replot_fig8_1_markers.png`.)

## 2. Post-Newtonian Pythagorean problem (pp. 147–149)

- Early 1990s, MV, Mikkola and associates (Turku): post-Newtonian force law, "first used to solve the Pythagorean
  three-body problem" (p. 147). In Newtonian gravity the solution scales with mass; in post-Newtonian gravity
  different masses give different orbits, partly because of gravitational-radiation losses (p. 148).
- With **distance unit 1 pc** and separations 3, 4, 5 (masses 3, 4, 5 opposite):
  - mass unit **10⁵ M☉**: similar to the Newtonian solution (not identical, by chaos); the third body escapes (p. 148);
  - **×10** (10⁶ M☉): the final escape "is still qualitatively the same" (p. 148);
  - **×100** (10⁷ M☉): relativity "starts to differ … more clearly"; one body is still thrown out, at **900 km/s**,
    the "heavier binary" at **300 km/s** (p. 148);
  - **×1000** (10⁸ M☉): "all three bodies merge together into a single body" (p. 149).
  - Momentum check, ours: 3 × 900 = 9 × 300, i.e. the escaper is the mass-3 body, as in the Newtonian case.
- Escape speeds from galaxy centres for comparison: about **600 km/s** for M32, about **3000 km/s** for the biggest
  galaxies (p. 149).
- The primary for these runs is Valtonen, Mikkola & Pietilä 1995, "Burrau's three-body problem in the
  post-Newtonian approximation", MNRAS 273, 751, doi:10.1093/mnras/273.3.751 (Crossref; **not in the repo**).

**CONFLICT, with Chitan, Mylläri & Haque 2022** (MNRAS 509, 1919; in the repo), who re-ran the 3–4–5 problem with
post-Newtonian terms to order 2.5, distance unit 1 pc, ARCcode (logarithmic-Hamiltonian leapfrog):

- "Around the mass unit range of 10^5.5–10^5.6 M☉, there was a transition from escape dominated dynamics to merger
  dominated dynamics" (abstract) — for their ensemble of sixteen Pythagorean triangles.
- For the 3–4–5 triangle itself, at **10⁵ M☉** their standard run gives **two mergers**; halving or doubling the
  accuracy parameter gives escape and a binary instead, "similarly to the 1995 solution" (their §3.3, p. 1925–1926).
  At **10⁹ M☉** "only one merger occurred in 2000 crossing times", whereas in the 1995 study "merging of all bodies
  occurred very quickly" (p. 1925).
- They summarise the 1995 study as "Mergers dominated from the 8 × 10⁶ M☉ while escapes dominated at smaller mass
  units" (p. 1925); the book places the change between 10⁷ (escape at 900 km/s) and 10⁸ M☉ (triple merger).
- Chitan et al. attribute the differences to code changes since 1995 amplified by chaos (p. 1926). The book's
  single-run statements are therefore not reproduced by the later primary paper; neither is a reference value.

## 3. Black holes, gravitational waves, OJ287 (pp. 149–153)

Three-body facts only:

- Schwarzschild radius about **3 km** for the Sun, **30 km** for ten solar masses (p. 150).
- PSR 1913+16: Taylor & Weisberg (2004) agreement with gravitational-wave energy loss within **0.2 %** (p. 152).
- **OJ287**: primary more massive than the Sun "by a factor of 18 billion" (p. 152); Fig. 8.6 (p. 163) gives
  "about 17 billion suns (18.3 billion according to latest measurements)" and a secondary of "about 100 million suns
  (150 million, to be exact)". Energy loss agrees with GR within **2 %** (p. 152). Major flares about every **12
  years**, recorded since **1891**; modelled as a three-body problem (black-hole binary plus disk gas) (pp. 152–153);
  the 2007 flare predicted to **1 day** (p. 153). Light-travel distance **3.45 billion years** (p. 7) or "3.5 billion
  light years" (Preface p. vi).

## 4. The three-black-hole problem (pp. 155–162)

- Hierarchical galaxy assembly brings black holes together; at most four at once (two binaries) (p. 155). MV,
  Mikkola and associates began a study in **1990** (p. 155).
- End state as in Newtonian dynamics, a binary plus an escaper, but the escape is typically fast enough for both to
  leave the galaxy in opposite directions; then the binary merges by radiation (p. 156). The breakup is faster than in
  the Pythagorean problem, "because the Pythagorean problem has exceedingly close encounters between two bodies
  which lead to black hole collisions rather than escapes. In this way the Pythagorean problem is rather special"
  (p. 156).
- **Slingshot theory** of double radio sources: Saslaw, Aarseth & MV (**1974**); Rees & Saslaw (**1975**); Lin &
  Saslaw (**1977**); De Young (**1977**) (p. 156).
- **Basu et al. 1993** (A&A 272, 417; no DOI on Crossref): gas in disks around a merging binary squeezed into streams
  up to **one quarter of the speed of light**, lasting a few years (p. 158). **Fig. 8.2** (p. 158): **576**
  relativistic three-body problems, one per disk particle; radial distance encodes escape speed, "35 units correspond
  to 75,000 km/s". Not digitised: the points overlap heavily.
- Stellar response to a binary black hole, JA and KT; Arp 5 / NGC 3664 (p. 158).
- **Basu & MV (1991)**: a **10⁵ M☉** satellite black hole around a **10⁸ M☉** one, ejected at up to **half the
  speed of light** by a second big black hole; redshift about **0.3** if receding (p. 159). Markarian 205 / NGC 4319:
  velocity difference about **20,000 km/s**, **6.7 %** of c, i.e. **880 million light years** by the Hubble law
  (p. 160).
- **Gravitational-wave recoil** ("rocket"): Fitchett (**1983**), hundreds of km/s for non-spinning holes (p. 160);
  Pietilä et al. (**1995**), about **1000 km/s** (p. 161); Campanelli et al. (**2013**), above 1000 km/s with spin,
  up to **4000 km/s** in the extreme (p. 161).
- **Full-GR three black holes**: Campanelli, Lousto & Zlochower 2008, PRD 77 (Fig. 8.5, p. 161); Crossref:
  "Close encounters of three black holes", PRD 77, 101501, doi:10.1103/PhysRevD.77.101501. Equal masses,
  non-spinning; a single solution. "it is possible to calculate millions of solutions [post-Newtonian] in the same
  amount of time that even one solution in full General Relativity takes" (Fig. 8.5 caption).
- HE0450-2958 (Haehnelt et al. **2005**): slingshot favoured over recoil because it gives about **ten times** higher
  speeds (p. 162). First triple black hole in a galaxy: Deane et al. (p. 162).

## 5. The no-hair test (pp. 162–164)

- OJ287 as a three-body problem (spinning black-hole binary plus a disk gas cloud). Gopakumar proposed the test; MV
  and associates showed in **2011** that the primary has no "hair"; the December 2015 impact was observed and
  announced on **10 March 2016** (pp. 163, footnote 2).

---

## Replication hooks

1. **Post-Newtonian Pythagorean problem across mass units** 10⁵–10⁹ M☉ at 1 pc: escape versus merger, escaper and
   binary speeds (book: 900 and 300 km/s at 10⁷ M☉). Needs a post-Newtonian force law, which Principia does not have;
   the target is Chitan et al. 2022 (in the repo), not the book. → **V-10** (new).
2. **Momentum bookkeeping as a sanity check** on any relativistic port: the book's speeds satisfy
   0.5 × 24,000 = 1.5 × 8000 (Fig. 8.1) and 3 × 900 = 9 × 300 (p. 148). → **V-10**.
3. **The Newtonian limit of Fig. 8.1**: masses 0.5, 0.5, 1.0 and digitised positions at t = 1–4 in the
   centre-of-mass frame. Velocities and the initial state (t = 0) are not given, so the run cannot be re-started from
   the book; only a pointer to Valtonen 1976. → **V-10**.
