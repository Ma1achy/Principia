# Replication catalogue: published IC maps and outcome tables for the general planar three-body problem

Compiled 2026-10-25. Scope: the general (non-restricted) problem. The one restricted exception is the Sitnikov problem.

**How the citations were checked.** Every citation marked **[CR]** was resolved against Crossref (authors, title, journal, volume, pages/article number, DOI). **[arXiv]** means the arXiv full text was read for the figure and parameter details quoted. **UNVERIFIED** marks anything I could not check against a primary record in this session; those details come from secondary citations or memory. Some details could not be read because ADS, OUP, Turku and some arXiv/ar5iv pages were blocked for this agent; they are listed at the end.

**Revised 27 Oct 2026** against the PDFs delivered that day (`../2026-10-27_papers/README.md`). The entries for
Agekyan & Anosova (§1.1, §2.5), Lehto et al. (§1.2), Aarseth et al. 1994a (§3.2), Boekholt et al. 2021 (§3.3), the
Šuvakov slice (§2.3) and Li & Liao (§6.3) were checked against the papers themselves. Their UNVERIFIED marks are
resolved and their errors corrected, each with a "*27 Oct 2026*" note saying what changed. Aarseth et al. 1994b is
added as §3.5. Each entry points at its targets in `../2026-10-27_papers/TARGETS.md`.

**Coordinate conventions differ between papers.** This matters for pixel-exact replication. The Agekyan–Anosova (AA) convention puts two bodies at (−0.5, 0) and (+0.5, 0) and the third at P = (x, y) in D = {x ≥ 0, y ≥ 0, (x+0.5)² + y² ≤ 1}. Lehto et al. 2008 instead use bodies at (0,0) and (1,0) with the third at (x, y), x ∈ [0,1], |y| ≤ √3/2. Tanikawa's papers use D11 = {x ≥ 0, y ≥ 0, (x−0.5)² + y² ≤ 1}, the mirror labelling. Breen et al. 2020 fix x1 = (1,0) at unit distance from the barycentre. Each map has to be remapped to the engine's own region-D chart before comparing.

**Escape criteria differ.** They include 50×a_bin + hyperbolic (Orlov, Mikkola & Tanikawa), 100×a + hyperbolic (TSUNAMI, Parischewsky), r > 10 + receding + E > 0 (Brutus papers), semi-major-axis ratio ≥ 100 or ≥ 2000 (Lehto), and Fewbody's tidal tolerance. Lifetimes near a boundary in lifetime are criterion-dependent. Outcome identities are robust once escape is real.

---

## (1) Maps over region D / free fall (equal masses, zero velocity)

### 1.1 Agekyan & Anosova 1967/68: the original region-D map and the 100-IC table
- **Citation [verified on the PDFs, 27 Oct 2026]:**
  - **Part I:** T. A. Agekyan & Zh. P. Anosova, "A study of the dynamics of triple systems by means of statistical
    sampling", *Astron. Zh.* 44(6), 1261–1273 (Nov–Dec 1967); English translation *Soviet Astronomy* 11(6), 1006–1014
    (May–June 1968), ADS `1968SvA....11.1006A`. Not in Crossref.
  - **Part II [CR]:** "Investigation of the dynamics of triple systems by the method of statistical tests. II",
    *Astrofizika* 4(1), 31–40 (1968) = *Astrophysics* 4(1), 11–16, doi:10.1007/BF01013153. The repo cites the 1968
    volume year that the paper prints; Crossref dates the translation's issue 1969.
  - **Part III [CR]:** Z. P. Anosova, *Astrophysics* 5(1), 81–84, doi:10.1007/BF01013354, from *Astrofizika* 5(1),
    161–167 (1969). Same convention: the volume year is 1969, and Crossref dates the translation 1970.
  - Both translations spell the first author "Agekyan"; "Agekian" is the transliteration Aarseth et al. 1994 use.
  - *27 Oct 2026, was:* "T. A. Agekian & Zh. P. Anosova … **UNVERIFIED** against ADS … *Astrophysics* 4, 11–16
    (1969) … *Astrophysics* 5, 81–84 (1969) [CR]". The papers are now read, and the [CR] year for Part III was wrong:
    Crossref gives 1970.
- **Access:** ADS scan (Sov. Astron.), Springer (Astrophysics, paywalled), arar.sci.am (Armenian digital library, Russian original).
- **What can be replicated:**
  - The definition of region D (I, Fig. 1 and eq. 8).
  - **Part I's Table 1** (pp. 1010–1012): 100 ICs in D started *near* rest (eq. 3). Its twelve columns are N, ξ, η,
    the outcome as text (disintegration, nominal disintegration, close approach, or ΔE/E > 0.05), T (in the
    per-IC crossing time τ of eq. 7), p, p_min, E, ΔE/E, Δ′E/E, M and ΔM/M. **No escaper identity is tabulated.**
    *27 Oct 2026, was:* "holds 100 random ICs in D with the outcome of each (escape or binary formation, which body
    escapes, time)", which came from the brief, not the paper.
  - **Part II** repeats the same 100 configurations from **exact rest** (Table 1, pp. 12–14) and completes the 38
    near-rest runs Part I could not finish (Table 3, p. 15).
- **Data:** Part I's ξ, η, E, M and outcome are in `fixtures/anosova/table1.csv` (R-592). Its T, p and p_min, and
  Part II's Tables 1 and 3 in full, were transcribed twice on 27 Oct 2026 into `../2026-10-27_papers/*.csv`.
- **Targets:** `../2026-10-27_papers/TARGETS.md` AA-1 to AA-8; AA-6, exact rest, is the strongest.
- **Difficulty / caveat:** A per-IC comparison is possible, but agreement is only expected for short-lived systems. The 1967 integrator was unregularised and low-precision, so long-lived cases cannot be expected to match. Treat a mismatch on long-lived rows as information about *their* integration, not about the engine. Part III gives the first unequal-mass statistics (see §2).

### 1.2 Lehto, Kotiranta, Valtonen, Heinämäki, Mikkola & Chernin 2008: the high-resolution lifetime ("Maasai shield") and reversibility map
- **Citation [CR][arXiv]:** H. J. Lehto et al., "Mapping the three-body system – decay time and reversibility", *MNRAS* 388, 965–970 (2008), doi:10.1111/j.1365-2966.2008.13450.x (the DOI ends 13450, not 13460), arXiv:0805.2844. The precursor is Lehto, Heinämäki, Chernin & Valtonen, "Masaai warrior shield: the life time of a three body system", *IAU Colloq.* 174, 314–317 (2000), doi:10.1017/S0252921100055196 **[CR]**.
- **Access:** arXiv (open).
- **What can be replicated:**
  - **Domain:** Bodies at (0,0) and (1,0), the third at (x, y) with x ∈ [0,1] and y ∈ [−√3/2, √3/2]. Equal masses, at rest, L = 0, G = 1. The paper calls it "a shield shaped area" and gives only these ranges. That it is the lens |P| ≤ 1, |P − (1,0)| ≤ 1 is an inference from its "R is the longest separation" (*27 Oct 2026*).
  - **Fig. 1:** Lifetime map, pixel 1.25×10⁻³, colour capped at 13 crossing times (red < 1, orange < 2, dark blue > 13).
  - **Fig. 2:** The same map at pixel 2.5×10⁻⁴.
  - **Fig. 3:** Zoom at pixel 3.125×10⁻⁶, linear scale to 20 crossing times.
  - **Fig. 7:** Reversibility fraction ξ.
  - **Numbers:** About 200 000 systems. A local-homogeneity test on 8-orbit stencils (ε = ±1.25×10⁻³) finds 28.6% homogeneous and 71.4% sensitive. Hausdorff dimension D_H = 1.5, with p_ε ∝ ε^−0.5.
  - **Escape criterion:** Semi-major-axis ratio ≥ 100 with the escaper hyperbolic and receding, or a ratio ≥ 2000 alone (p. 6, after Anosova 1986). Breakup: the escaper has E ≥ 0 and is at r ≥ 1 from both other bodies (p. 2). *27 Oct 2026, was:* "Breakup is the first time …"; the paper does not say "first".
  - **Zoom windows, read off the axes:** Fig. 2 has lower-left corner (0.70, 0.45) and spans x to 0.80, y to about 0.62. Fig. 3 is x ∈ [0.700, 0.70125], y ∈ [0.450, 0.45125].
  - **Targets:** `../2026-10-27_papers/TARGETS.md` L-1 to L-5.
  - **Integrators:** Chain-regularised Bulirsch–Stoer (Mikkola–Aarseth), plus logH leapfrog for the reversal test.
- **Data:** Not released. Figures only.
- **Difficulty:** Qualitative to semi-quantitative. Morphology (the shield) and lifetime bands can be compared directly. The 28.6% homogeneous fraction and D_H = 1.5 are summary statistics that can be replicated with the same stencil. This is the most natural first target for the AA/region-D chart, and the colour scale is in crossing times rather than t, so check the unit. **Caveat:** The crossing-time unit and the R normalisation (R is the longest side of the initial triangle) need care.

### 1.3 Boekholt, Portegies Zwart & Valtonen 2020: "Gargantuan" converged lifetime map of region D (Brutus)
- **Citation [CR][arXiv]:** T. C. N. Boekholt, S. F. Portegies Zwart & M. Valtonen, "Gargantuan chaotic gravitational three-body systems and their irreversibility to the Planck length", *MNRAS* 493, 3932–3937 (2020), doi:10.1093/mnras/staa452, arXiv:2002.04029.
- **Access:** arXiv (open).
- **What can be replicated:**
  - **Domain:** The AA homology map, equal masses, from rest.
  - **Sampling:** Uniform grid with spacing 0.015625 (1/64), 1212 ICs.
  - **Precision:** Brutus at 1024-bit words, tolerance ε from about double precision down to 10⁻⁷⁰.
  - **Horizon:** Until permanent breakup or t = 10⁴.
  - **Escape:** r > 10 from the binary centre of mass, receding, E > 0.
  - **Fig. 1:** Lifetime maps at ε = 10⁻⁶ and 10⁻⁷⁰. Black marks systems living longer than 1000. A KS test gives p = 0.72, so the two maps are statistically indistinguishable.
  - **Fig. 2:** Irreversible fraction against ε, about 1% at 10⁻⁷⁰.
  - **Fig. 3:** Distribution of amplification factors.
- **Data:** Not stated. Brutus is public in AMUSE.
- **Difficulty:** The grid is coarse (1212 points), so a near-exact per-IC lifetime comparison against *converged* solutions is feasible in principle if the points can be regenerated. Statistically, the lifetime distribution can be tested with a KS test. **This is the best "ground truth" lifetime dataset for region D**, because every point is converged by construction. **Caveat:** Lifetime is measured back to the last closest approach before ejection, not to criterion firing.

### 1.4 Tanikawa & Mikkola 2015: symbol-sequence maps of the free-fall plane
- **Citation [CR]:** K. Tanikawa & S. Mikkola, "Symbol sequences and orbits of the free-fall three-body problem", *PASJ* 67, 115 (2015), doi:10.1093/pasj/psv087. The precursor **[arXiv]**: Tanikawa & Mikkola, "A trial symbolic dynamics of the planar three-body problem", arXiv:0802.2465 (2008); proceedings venue **UNVERIFIED**.
- **Access:** The PASJ paper is paywalled. arXiv:0802.2465 is open.
- **What can be replicated:**
  - **Domain:** Bodies 2 and 3 at (∓0.5, 0), body 1 in D11.
  - **Symbols:** A symbol is written at each syzygy where the signed triangle area changes sign. Symbols 1–3 are "+→−" and 4–6 are "−→+", and the longest edge picks the digit.
  - **0802.2465 figures:** Maps coloured by the first 2, 3 and 4 digits (Figs. 3, 5, 6). **With angular momentum:** first-digit maps for virial ratio k = T/|U| from 0 to 0.7 (Fig. 7). Fig. 8 zooms into the Lagrange point.
  - **PASJ 2015:** The global structure of the plane from symbol sequences, plus the isosceles and collinear boundaries with sequences of triple-collision and periodic orbits.
- **Data:** Figures. Integrator: algorithmic regularisation / logH (Mikkola–Tanikawa 1999, *CeMDA* 74, 287, doi:10.1023/A:1008368322547 **[CR]**).
- **Difficulty:** **Exact in principle.** A symbol sequence of finite length is a discrete, integrator-robust label away from the binary-collision curves. The first-digit map is nearly a geometric partition, so even the first two to three digits are a stringent qualitative test. **The k > 0 maps are rare published IC maps with non-zero velocity** (see §2.2). **Caveat:** Symbol conventions have to be reproduced exactly, including the body labelling.

### 1.5 Tanikawa, Saito & Mikkola 2019, and Tanikawa & Mikkola 2021: triple-collision points inside D
- **Citations [CR]:**
  - K. Tanikawa, M. M. Saito & S. Mikkola, "A search for triple collision orbits inside the domain of the free-fall three-body problem", *CeMDA* 131, 24 (2019), doi:10.1007/s10569-019-9902-8.
  - K. Tanikawa & S. Mikkola, "Numerical confirmation of the existence of triple collision orbits inside the domain of the free-fall three-body problem", *CeMDA* 133, 52 (2021), doi:10.1007/s10569-021-10050-8. **[read: accepted manuscript at utupub.fi]**
- **Access:** Accepted manuscripts are self-archived at utupub.fi (Turku).
- **What can be replicated:**
  - **2021, Table 2:** Coordinates of eleven triple-collision points (TCPs) in D, one at (0.19208270, 0.30936018), with body and edge directions and triangle orientation.
  - **2021, Table 1:** Points along three binary-collision curves (BCCs) approaching a TCP.
  - **2021, Table 4:** ICs at radii 10⁻³, 10⁻⁴ and 10⁻⁵ from the TCP, at azimuths 60°, 120° and 180°.
- **Data:** The tables are in the paper.
- **Difficulty:** **Exact.** These are named points where three BCCs meet. A collision/`d_min` map rendered around a TCP must show the curves converging there, and the "slingshot directions" give a checkable outcome per sector. Few published items in this list are this sharp a test of a regularised integrator. **Caveat:** Triple collision is reached only asymptotically, so test `d_min` scaling, not exact collision.

### 1.6 Tanikawa, Umehara & Abe 1995; Tanikawa 2000; Umehara & Tanikawa 2000: collision curves and escape in free fall
- **Citations [CR]:**
  - K. Tanikawa, H. Umehara & H. Abe, "A search for collision orbits in the free-fall three-body problem I. Numerical procedure", *CeMDA* 62, 335–362 (1995), doi:10.1007/BF00692285.
  - K. Tanikawa, "… II", *CeMDA* 76, 157–185 (2000), doi:10.1023/A:1008313912013.
  - H. Umehara & K. Tanikawa, "Binary and triple collisions causing instability in the free-fall three-body problem", *CeMDA* 76, 187–214 (2000), doi:10.1023/A:1008305928852.
  - Background: Umehara's thesis, *The Free-Fall Three-Body Problem: Escape and Collision* (SOKENDAI, 1998), open at ir.soken.ac.jp/records/373.
- **Access:** The CeMDA papers are paywalled. The thesis is open (23 MB PDF).
- **What can be replicated:** The binary-collision curves (first, second, ... collision) across the free-fall domain. Escape orbits cluster along particular BCCs and TCPs. Map details are **UNVERIFIED** here: the abstracts were read, the figures were not.
- **Difficulty:** Qualitative to exact. The location of the first BCC is a geometric, integrator-robust curve. "Escape clusters near collision curves" is a qualitative structural prediction for the engine's escape-identity map.

### 1.7 Mylläri, Martynova, Orlov & Chernin 2007: symbolic dynamics and entropy over the homology map
- **Citation [verified on journal site]:** A. Mylläri, A. Martynova, V. Orlov & A. Chernin, "Symbolic dynamics in the free-fall equal-mass three-body problem", *Serdica J. Computing* 1(4), 425–432 (2007), doi:10.55630/sjc.2007.1.425-432. Not checked in Crossref.
- **Access:** Open (journal site, EuDML).
- **What can be replicated:** A 2D homology map scanned at step 0.001 in both axes. Symbol sequences come from binary/triple encounter states and configuration changes. Histograms of Shannon and Markov entropy show distinct peaks, which the authors read as ergodic components.
- **Difficulty:** Statistical (the entropy histograms). The symbol definition differs from Tanikawa's.

### 1.8 Martynova & Orlov 2014: families of moderate-lifetime trajectories in D
- **Citations [CR]:**
  - A. I. Martynova & V. V. Orlov, "The decay of triple systems", *Astron. Rep.* 58, 756–766 (2014), doi:10.1134/S1063772914100084.
  - A. I. Martynova & V. V. Orlov, "Trajectories of bodies in triple systems with moderate lifetimes", *Astron. Rep.* 58, 767–781 (2014), doi:10.1134/S1063772914100096.
  - Related: A. I. Martynova, V. V. Orlov & A. V. Rubinov, "Metastability in the evolution of triple systems", *MNRAS* 344, 1091–1096 (2003), doi:10.1046/j.1365-8711.2003.06876.x.
- **Access:** Paywalled.
- **What can be replicated:** Equal masses, zero velocity. The sets of ICs decaying within about 10–20 mean crossing times form distinct structures (families) in D, together with a trajectory classification. Figure details are **UNVERIFIED** (abstract only).
- **Difficulty:** Qualitative. Compare the engine's lifetime map, banded at 10–20 t_cr, against their family structures.

### 1.9 Heinämäki, Lehto, Valtonen & Chernin 1999: Kolmogorov–Sinai entropy in three-body dynamics
- **Citation [CR]:** P. Heinämäki, H. J. Lehto, M. J. Valtonen & A. D. Chernin, "Chaos in three-body dynamics: Kolmogorov–Sinai entropy", *MNRAS* 310, 811–822 (1999), doi:10.1046/j.1365-8711.1999.02859.x.
- **Access:** Paywalled (OUP). An ADS scan probably exists (**UNVERIFIED**).
- **What can be replicated:** KS-entropy (Lyapunov-type) estimates over free-fall ICs. Whether there is a 2D map over D, and at what resolution, is **UNVERIFIED** (abstract not retrievable). The brief also mentions a 1998 paper, which I could not locate.
- **Difficulty:** Statistical at best. This is mainly a cross-check for an FTLE colouring.

### 1.10 Breen, Foley, Boekholt & Portegies Zwart 2020: 10 000 converged short free-fall trajectories
- **Citation [CR][arXiv]:** P. G. Breen, C. N. Foley, T. Boekholt & S. Portegies Zwart, "Newton versus the machine: solving the chaotic three-body problem using deep neural networks", *MNRAS* 494, 2465–2470 (2020), doi:10.1093/mnras/staa713, arXiv:1910.07291.
- **Access:** arXiv (open).
- **What can be replicated:**
  - **Setup:** Equal masses, at rest. x1 is fixed at (1,0), x2 lies in the unit semicircle with x ≤ 0, and x3 follows from the centre of mass.
  - **Runs:** 10 000 Brutus runs (ε = 10⁻¹¹, 128-bit words), integrated to t = 10 with output every 2⁻⁸.
- **Data:** No availability statement in the text.
- **Difficulty:** Exact trajectory comparison is possible on any IC, because Brutus is converged. **Caveat:** The horizon is short (t ≤ 10), and the text has a sign inconsistency about the singular point (−0.5, 0) versus (0.5, 0).

---

## (2) Maps with unequal masses or non-zero velocities (especially valuable)

### 2.1 Trani, Leigh, Boekholt & Portegies Zwart 2024: "Isles of regularity" (unequal masses, binary + single)
- **Citation [CR][arXiv]:** A. A. Trani, N. W. C. Leigh, T. C. N. Boekholt & S. Portegies Zwart, "Isles of regularity in a sea of chaos amid the gravitational three-body problem", *A&A* 689, A24 (2024), doi:10.1051/0004-6361/202449862, arXiv:2403.03247.
- **Access:** Open access (A&A and arXiv).
- **What can be replicated:**
  - **Masses:** 17.5, 15 and 12.5 M☉.
  - **Initial state:** A circular binary with a = 5 au and a single body at rest at d = 100 au. **Non-zero velocities** (the binary orbit), but the system is a hierarchical infall with zero net bulk motion.
  - **Map axes:** Binary true longitude λ ∈ [0, 2π) against inclination ι ∈ [−π/2, π/2] relative to the binary–single line. The full map uses 10⁶ samples, uniform with cos ι weighting. Planar runs are the ι = 0 line only; the map itself is 3D-inclined.
  - **Zooms:** Fourteen successive ×5-per-axis zooms of 10⁵ samples each.
  - **Fig. 3:** Escaper identity and the regularity fraction f_col (k = 12 nearest neighbours, regular if f_col > 2/3).
  - **Figs. 5–6:** Zooms by escaper and by log lifetime.
  - **Fig. 9 / Tables 1–2:** TSUNAMI against Brutus at zoom 14.
  - **Fig. 10:** A Planck-scale 320×320 zoom with Brutus.
  - **Fig. A.1:** Nine phase maps for hyperbolic setups.
  - **Horizon:** 10⁹ yr. Breakup is a hyperbolic, diverging single.
- **Data:** No availability statement. TSUNAMI is described in Trani et al. 2019 and Trani & Spera 2023 (arXiv:2206.10583); its public availability is **UNVERIFIED**.
- **Difficulty:** Statistical and qualitative. The outcome fractions and regular fractions in Tables 1–2 are quantitative. **Caveat:** The (λ, ι) plane is **not planar**: only the ι = 0 row lies in a planar engine's domain, so a planar engine can replicate the ι = 0 cut (a 1D scan in λ) and its escaper sequence. The regularity statistic f_col is directly portable as a pixel colouring.

### 2.2 Tanikawa & Mikkola 2008: first-symbol maps at non-zero angular momentum
See §1.4. The arXiv:0802.2465 Fig. 7 gives first-digit maps for k = T/|U| = 0 … 0.7 over D11 ∪ D12. Velocities are set so that angular momentum is maximal for the triangle; the exact D12 parametrisation is **not detailed** there. This is one of very few published 2D IC maps in the general problem with non-zero velocity. **Difficulty:** Qualitative until the velocity prescription is pinned down from the PASJ papers.

### 2.3 Šuvakov & Dmitrašinović 2013: a 2D velocity slice with periodic-orbit ICs (equal masses)
- **Citation [CR][arXiv]:** M. Šuvakov & V. Dmitrašinović, "Three classes of Newtonian three-body planar periodic orbits", *PRL* 110, 114301 (2013), doi:10.1103/PhysRevLett.110.114301, arXiv:1303.0181. Follow-up: Šuvakov & Dmitrašinović, arXiv:1312.7002 (search near the figure-eight).
- **Access:** arXiv (open).
- **What can be replicated:**
  - **Slice:** Collinear positions (−1,0), (1,0), (0,0), with velocities parameterised by (ẋ₁, ẏ₁). Body 2 has the same velocity and body 3 has −2× it. L = 0, equal masses, G = 1.
  - **Table:** Fifteen ICs and periods (butterfly I is (0.30689, 0.12551) with T = 6.2356; I also read figure-eight-related entries and others).
  - **Search:** About 50 candidate regions at return proximity ~10⁻¹.
- **Data:** The table is in the paper. The full catalogue is on the Belgrade three-body gallery (three-body.ipb.ac.rs, **UNVERIFIED** URL).
- **Map over this slice (27 Oct 2026):** Hudomal's 2015 thesis, Fig. 8 (p. 26), is a −log₁₀ return-proximity map over (p₁, p₂) ∈ [0.3, 0.5] × [0.1, 0.5] at 2000 × 4000 points. Fig. 34 (p. 55) is a zoom on butterfly III. Targets HU-1 to HU-5 in `../2026-10-27_papers/TARGETS.md`.
- **Difficulty:** **Exact.** This is a 2D chart with momenta, and a return-proximity or FTLE map over (ẋ₁, ẏ₁) must show minima at the tabulated points. Each listed orbit must close to about 10⁻⁵ after T. Strongly recommended for a "slice with momenta" chart.

### 2.4 Li & Liao 2017; Li, Jing & Liao 2018: thousands of periodic ICs, equal and unequal masses
- **Citations [CR]:**
  - X. Li & S. Liao, "More than six hundred new families of Newtonian periodic planar collisionless three-body orbits", *Sci. China PMA* 60, 129511 (2017), doi:10.1007/s11433-017-9078-5.
  - X. Li, Y. Jing & S. Liao, "Over a thousand new periodic orbits of a planar three-body system with unequal masses", *PASJ* 70, 64 (2018), doi:10.1093/pasj/psy057.
  - X. Li, X. Li & S. Liao, "One family of 13315 stable periodic orbits of the non-hierarchical unequal-mass triple system", arXiv:2007.10184 **[arXiv]**; journal ref **UNVERIFIED**.
- **Access:** arXiv and the project site numericaltank.sjtu.edu.cn/three-body (open).
- **What can be replicated:**
  - **Slice:** The same collinear/velocity slices as Šuvakov, with unequal masses in the 2018 paper.
  - **2007.10184:** A continuation over m₁ ∈ [0.8, 1.1] and m₂ ∈ [0.7, 1.2] (m₃ = 1, step 0.001) finds 135 445 orbits, 13 315 of them linearly stable. Fig. 2 gives contour maps of period and angular momentum **over the mass plane**, and Fig. 3 the stability region there.
- **Data:** Full IC tables in the supplements and on the website. They were verified with arbitrary-precision "clean numerical simulation".
- **Difficulty:** **Exact** (periodicity checks). The m₁–m₂ stability map is a published **mass-plane map**, the closest published analogue to a mass-simplex chart, though restricted to one orbit family.

### 2.5 Anosova 1969: first unequal-mass statistics from region D
- **Citation [CR]:** Z. P. Anosova, "Investigation of the dynamics of triple systems by the Monte Carlo method. III. The case of components with different masses", *Astrophysics* 5(1), 81–84, doi:10.1007/BF01013354, from *Astrofizika* 5(1), 161–167 (1969). Crossref dates the translation 1970; the repo cites the 1969 volume year the paper prints (*27 Oct 2026*).
- **From its abstract (27 Oct 2026):** 300 configurations from rest in three mass sets, (9, 3, 1), (3, 3, 1) and (3, 1, 1). Every run decayed after a close triple approach. Mean decay time (27.8 ± 3.5)τ. The lightest body usually escapes, and the mass spread shortens decay by about a factor of three.
- **Access:** Paywalled (Springer). The Russian original is *Astrofizika* 5, 161–167.
- **What can be replicated:** Escape probabilities by mass. The lightest body escapes preferentially. Statistical; the tables are **UNVERIFIED** in detail.
- **Difficulty:** Statistical, small-N, old integrator.

### 2.6 Valtonen, Mylläri, Orlov & Rubinov 2005: rotating triples drawn from region D
- **Citations [CR]:**
  - M. Valtonen, A. Mylläri, V. Orlov & A. Rubinov, "Dynamics of rotating triple systems: statistical escape theory versus numerical simulations", *MNRAS* 364, 91–98 (2005), doi:10.1111/j.1365-2966.2005.09540.x.
  - Precursor: same authors, "Dynamics of rotating triple systems", *Astron. Lett.* 29, 41–49 (2003), doi:10.1134/1.1537377.
- **Access:** Paywalled.
- **What can be replicated:**
  - **Sample:** About 120 000 equal-mass systems with E₀ = −1. Rotation parameter w ∈ {0.005, 0.1, 1, 2, 4, 6}.
  - **"Method D":** Positions random in AA region D, isotropic velocities, virial ratio uniform in (0,1).
  - **Outputs:** Binary-energy and escape-angle distributions compared to ergodic theory.
  - **Integration:** Three-body regularisation with Bulirsch–Stoer, up to 1000 crossing times.
- **Difficulty:** Statistical. A good test of the engine's outcome statistics at L ≠ 0.

### 2.7 Martynova, Orlov & Rubinov 2009: stability regions of non-hierarchical triples
- **Citations [CR]:**
  - A. I. Martynova, V. V. Orlov & A. V. Rubinov, "The structure of non-hierarchical triple system stability regions", *Astron. Rep.* 53, 710–721 (2009), doi:10.1134/S1063772909080034.
  - Companion: A. I. Martynova & V. V. Orlov, "Stability regions in the vicinity of a periodic Ducati orbit", *Astron. Lett.* 35, 572–578 (2009), doi:10.1134/S1063773709080076.
- **Access:** Paywalled.
- **What can be replicated:** Maps of long-lived (stable) IC regions around periodic orbits (Schubart, Ducati), likely with non-zero velocities. The axes and parameters are **UNVERIFIED** (abstracts not retrievable).
- **Difficulty:** Qualitative. Worth fetching: if the maps are 2D slices through a periodic-orbit IC, they are directly comparable to the engine's lifetime/FTLE charts.

### 2.8 Boekholt & Portegies Zwart 2024: "Gargantuan II", angular momentum dependence
- **Citation [CR][arXiv]:** T. C. N. Boekholt & S. F. Portegies Zwart, "Gargantuan chaotic gravitational three-body systems II. Dependence on angular momentum and astrophysical scale", *MNRAS* 536, 2993–3006 (2024), doi:10.1093/mnras/stae2782, arXiv:2311.07651.
- **Access:** arXiv (open).
- **What can be replicated:**
  - **Ensembles:** Not a 2D map. Twelve equal-mass Plummer ensembles of 2048 realisations each, with ⟨L⟩/⟨L_vir⟩ from 0 to 1, reached by velocity rescaling via the virial ratio Q.
  - **Precision and horizon:** Brutus, ε iterated down to 10⁻⁹⁰, horizon 10⁴ Hénon units. Escape is r > 10, receding, E > 0.
  - **Table 1:** The fraction of fundamentally unpredictable triples, 29.8% (virialised) against 5.2% (L = 0).
- **Difficulty:** Statistical. These ensembles are 3D, so they are relevant only as context.

---

## (3) Burrau / Pythagorean neighbourhood

### 3.1 Szebehely & Peters 1967: the complete Burrau solution
- **Citation [CR]:** V. Szebehely & C. F. Peters, "Complete solution of a general problem of three bodies", *AJ* 72, 876 (1967), doi:10.1086/110355.
- **Access:** ADS scan (open: articles.adsabs.harvard.edu/pdf/1967AJ.....72..876S).
- **What can be replicated:** Masses 3, 4, 5 at rest at (1,3), (−2,−1), (1,−1), G = 1. The masses 4 and 5 form a binary and the mass-3 body escapes. Breakup is around t ≈ 59.4 (Janssens' review quotes 59.406); the escape is asymptotic over t ≈ 60–70. The paper gives the times and distances of close approaches, and the final binary's elements.
- **Difficulty:** **Exact.** The trajectory is non-chaotic enough to reproduce to many digits with a regularised integrator. **Caveat:** This is the engine's `near-field` IC. Its escape lies beyond the project's t = 13 horizon, consistent with the standing finding.

### 3.2 Aarseth, Anosova, Orlov & Szebehely 1994a: global chaoticity around the Pythagorean IC
- **Citation [CR]:** S. J. Aarseth, J. P. Anosova, V. V. Orlov & V. G. Szebehely, "Global chaoticity in the Pythagorean three-body problem", *CeMDA* 58(1), 1–16 (1994), doi:10.1007/BF00692114. Called **1994a** from 27 Oct 2026, to separate it from §3.5.
- **Access:** In the repo (`papers/aarseth_anosova_orlov_szebehely_1994_CeMDA_58_1.pdf`). An ADS image scan also exists (bibcode `1994CeMDA..58....1A`). *27 Oct 2026, was:* "Paywalled (Springer). An ADS scan may exist (**UNVERIFIED**)."
- **What can be replicated** (read from the paper, 27 Oct 2026; was "The axes, step and resolution are **UNVERIFIED** (abstract only)"):
  - **Displacements.** One body is displaced at a time onto **six circles**, δ = 5×10⁻⁸, 5×10⁻⁷, …, 5×10⁻³, about its Burrau position, at **100 azimuths** α = 3.6n°, n = 0–99 (§2, eq. 1). That is 1800 runs. This is a polar sampling, not a Cartesian grid.
  - **Integrator and escape.** Aarseth–Zare with Bulirsch–Stoer, tolerance 10⁻¹³, repeated at 10⁻¹⁵ when needed. Escape is Standish's criterion outside 3R_grav (R_grav = 3.67), or conditional escape beyond 5R_grav; the time limit is 200.
  - **Reliability.** A run is kept only if time reversal returns the coordinates and the velocities to r.m.s. < 0.001 ("predictable"). 831 runs are predictable escapes.
  - **Outputs.** Escape angle β, escaper identity and escape time T (Figs. 2–4). Table I gives escape times by δ; Tables II–VII give β, A, E, P, I, R and the reversal errors by δ; Table VIII gives escaper statistics (75/15/10%).
  - The **regular region around each standard position**: at δ = 5×10⁻⁸ the β-ring is complete for m₁ (β ≈ 70°) and a "horseshoe" for m₂ and m₃ (p. 8).
  - Fine scans at 0.36° steps, δ = 3, 5 and 7 ×10⁻⁷, in 36° sectors: qualitative only.
  - **Targets:** `../2026-10-27_papers/TARGETS.md` P-1 to P-6, with Tables I–VIII transcribed.
- **Difficulty:** Qualitative to semi-quantitative. The existence and rough extent of the regular island around the nominal IC is a direct test of the BodyPlane chart centred on Burrau. **Caveat:** The 1994 integrators were regularised but double precision, and the authors themselves flag unreliable regions.

### 3.3 Boekholt, Moerman & Portegies Zwart 2021: relativistic Pythagorean problem (Newtonian limit as control)
- **Citation [CR][arXiv]:** T. C. N. Boekholt, A. Moerman & S. F. Portegies Zwart, "Relativistic Pythagorean three-body problem", *Phys. Rev. D* 104(8), 083020 (2021), doi:10.1103/PhysRevD.104.083020, arXiv:2109.07013. *27 Oct 2026, was:* "F. Moerman"; the second author is Arend Moerman (byline, Crossref).
- **Access:** arXiv (open).
- **What can be replicated:** Not a 2D map.
  - Lifetime against ζ, the dimensionless speed of light (Fig. 3), with the Newtonian limit for ζ > 10⁶.
  - A reversibility test perturbs the lightest body's x in the tenth decimal place, integrating to t = 100 and back.
  - Lyapunov time τ_L = 4.3 ± 0.2 (Eq. 1, Fig. 5).
  - Escape requires r ≥ 5 from the **centre of mass**, receding and E > 0, all held for 100 time units.
- **Difficulty:** Exact in principle (converged Brutus), but **the Newtonian lifetime is not printed**. Fig. 3's plateau reads log₁₀ T_l ≈ 1.8, a figure reading only. τ_L is **fitted over runs at all ζ**, relativistic included, so it calibrates an FTLE only loosely. Table I (p. 10) gives a ζ = 10⁶ reversal at 1PN to 40 digits, not a Newtonian state.
  - *27 Oct 2026, was:* "Exact for the Newtonian lifetime (converged Brutus). The τ_L ≈ 4.3 estimate is a useful FTLE calibration for the near-field chart."
  - **Targets:** `../2026-10-27_papers/TARGETS.md` B-1 to B-3.

### 3.5 Aarseth, Anosova, Orlov & Szebehely 1994b: close triple approaches before escape (added 27 Oct 2026)
- **Citation [CR]:** S. J. Aarseth, J. P. Anosova, V. V. Orlov & V. G. Szebehely, "Close triple approaches and escape in the three-body problem", *CeMDA* 60(1), 131–137 (1994), doi:10.1007/BF00693096.
- **Access:** In the repo (`papers/aarseth_anosova_orlov_szebehely_1994b_CeMDA_60_131.pdf`).
- **What can be replicated:**
  - **Ensembles.** 2500 equal-mass systems from rest with the third body random in region D, and 2500 with all positions uniform in a unit circle R. Both are compared with the 831 predictable Pythagorean-vicinity escapes of §3.2.
  - **Escape.** Standish, or conditional escape at 10d, d the virial radius.
  - **Parameters.** The triple approach at minimum moment of inertia (perimeter < 2d) is described by T, P, I, Y, Z, Ψ, V_b, V_e, A and E. Table I gives their means, Table II their ten-bin histograms with medians, and Table III D + R against 𝒫.
  - **Monaghan's planar eccentricity distribution** is printed as a binned row, which is exactly computable.
  - **A random-geometry control for Z** needs no integration.
- **Difficulty:** Statistical. The A distribution's cut-off at ≈ 0.4 is made by the 10d conditional-escape rule. 1994b's T for 𝒫 (52.4) is in its own crossing-time unit, not 1994a's T_cr = 3.84.
- **Targets:** `../2026-10-27_papers/TARGETS.md` Q-1 to Q-4, with Tables I–III transcribed.

### 3.4 Chitan, Mylläri & Haque 2021: Burrau revisited (16 Pythagorean triangles)
- **Citation [CR][arXiv]:** A. S. Chitan, A. Mylläri & S. Haque, "Relativistic effects on triple black holes: Burrau's problem revisited", *MNRAS* 509, 1919–1928 (2021/22), doi:10.1093/mnras/stab3124, arXiv:2011.03046.
- **Access:** arXiv (open).
- **What can be replicated:** The Newtonian runs cover all 16 Pythagorean triangles with hypotenuse below 100, each mass equal to the length of the opposite side, from rest. They give lifetimes in crossing times and counts of two-body encounters, using ARCcode (logH). The 2D maps are over (mass unit, distance unit), which is relevant only in the post-Newtonian regime.
- **Difficulty:** Exact per IC for the **16 Newtonian "Pythagorean family" ICs**, a nice discrete test set of unequal-mass free-fall ICs on the mass simplex.

---

## (4) Scattering / hierarchical maps

### 4.1 Hut 1983; Hut & Bahcall 1983: binary–single scattering, phase scans and topology
- **Citations [CR]:**
  - P. Hut & J. N. Bahcall, "Binary-single star scattering. I. Numerical experiments for equal masses", *ApJ* 268, 319 (1983), doi:10.1086/160956.
  - P. Hut, "The topology of three-body scattering", *AJ* 88, 1549 (1983), doi:10.1086/113445.
  - Later: Hut, "Binary-single-star scattering. III", *ApJ* 403, 256 (1993), doi:10.1086/172199.
- **Access:** ADS scans (open).
- **What can be replicated:** Equal-mass binary–single cross-sections for exchange, ionisation and resonance against v∞/v_c, in tables. The topology paper gives 1D scans of outcome against impact parameter or phase, showing smooth intervals separated by fractal resonance intervals.
- **Difficulty:** Statistical (cross-sections) or qualitative (scans). **Caveat:** Most of this work is 3D. Planar subsets are the authors' illustrative scans only.

### 4.2 Boyd & McMillan 1993: chaotic scattering IC maps
- **Citation [CR]:** P. T. Boyd & S. L. W. McMillan, "Chaotic scattering in the gravitational three-body problem", *Chaos* 3, 507–523 (1993), doi:10.1063/1.165956.
- **Access:** Paywalled (AIP).
- **What can be replicated:** Initial-value-space maps of outcome for a binary plus an incoming star, with regular regions separated by "rivers" of chaos and scaling inside the rivers. The maps evolve with the incoming velocity and mass. The axes (believed to be impact parameter against binary phase), masses and resolution are **UNVERIFIED**.
- **Difficulty:** Qualitative. Historically the first "basin map" pictures in the general problem.

### 4.3 Trani et al. 2024, Appendix A: hyperbolic setups
See §2.1: Fig. A.1 gives nine phase maps for hyperbolic binary–single encounters, the same (λ, ι) axes.

---

## (5) Statistical outcome distributions and tables

### 5.1 Stone & Leigh 2019: ergodic statistical solution
- **Citation [CR][arXiv]:** N. C. Stone & N. W. C. Leigh, "A statistical solution to the chaotic, non-hierarchical three-body problem", *Nature* 576, 406–410 (2019), doi:10.1038/s41586-019-1833-8, arXiv:1909.05272.
- **Access:** arXiv (open).
- **What can be replicated:**
  - **Runs:** Fewbody (RK8 Prince–Dormand, tidal tolerance 10⁻⁵), equal point masses, a = 1 AU binary, b = 0, v∞ = 0.01 v_c.
  - **Ensembles:** Runs A, B and C with binary eccentricity 0, 0.5 and 0.9 (L̃₀ = 1.0, 0.87, 0.44), about 10⁵ runs each.
  - **Outputs:** Binned outcome distributions of binary energy, eccentricity and inclination compared with closed-form ergodic predictions. The cut N_scramble ≥ 2 is needed for agreement.
- **Data:** Fewbody is public.
- **Difficulty:** Statistical. The analytic distributions are a model-independent target for planar runs, though the planar (2D) ergodic forms differ from the 3D ones (Stone & Leigh give both).

### 5.2 Kol 2021 and Manwadkar, Kol, Trani & Leigh 2021: flux-based theory and its test (unequal masses)
- **Citations [CR]:**
  - B. Kol, "Flux-based statistical prediction of three-body outcomes", *CeMDA* 133, 17 (2021), doi:10.1007/s10569-021-10015-x.
  - V. Manwadkar, B. Kol, A. A. Trani & N. W. C. Leigh, "Testing the flux-based statistical prediction of the three-body problem", *MNRAS* (2021), doi:10.1093/mnras/stab1689, arXiv:2101.03661 **[arXiv]**. Volume and pages are **UNVERIFIED** (Crossref lists none).
- **Access:** arXiv (open).
- **What can be replicated:**
  - **Table 1:** Eight mass sets, from (15,15,15) to (5,15,25) M☉.
  - **Setup:** A circular binary with a = 5 AU and the single at rest at 100 AU. True longitude uniform in [0, 2π], inclination uniform in [0, π]. 10⁶ runs per set with TSUNAMI.
  - **Escape probabilities per body:** Matched to about 1% against P ∝ k^{3/2}.
  - **Threshold exponent:** α ≈ 1.97, against a predicted 2.
  - **Lifetime tail slope:** −1.69 to −1.73.
- **Difficulty:** Statistical. **The per-mass-set escape probabilities are the best published unequal-mass outcome table** for testing a mass-simplex chart's escape-identity colouring in aggregate. The setup is 3D, so a planar re-run needs ι restricted to {0, π}.

### 5.3 Ginat & Perets 2021: random-walk statistical theory
- **Citation [CR]:** Y. B. Ginat & H. B. Perets, "Analytical, statistical approximate solution of dissipative and nondissipative binary-single stellar encounters", *Phys. Rev. X* 11, 031020 (2021), doi:10.1103/PhysRevX.11.031020.
- **Access:** Open access (PRX).
- **What can be replicated:** Analytic outcome distributions (binary energy, eccentricity) and a lifetime distribution, compared against scattering simulations.
- **Difficulty:** Statistical. The theory is an alternative null to Stone & Leigh.

### 5.4 Manwadkar, Trani & Leigh 2020: Lévy flights, scramble density and lifetimes
- **Citation [CR][arXiv]:** V. Manwadkar, A. A. Trani & N. W. C. Leigh, "Chaos and Lévy flights in the three-body problem", *MNRAS* 497, 3694–3712 (2020), doi:10.1093/mnras/staa1722, arXiv:2004.05475.
- **Access:** arXiv (open).
- **What can be replicated:** Metrics derived from AA maps and the homology radius: Scramble Density and the LF index. Ergodic interactions have exponential lifetimes and Lévy flights have power-law tails. Mass ejection probabilities in the ergodic limit. The IC details were not in the abstract I could read (**UNVERIFIED**); I believe the setup is TSUNAMI with binary + single as in §5.2.
- **Difficulty:** Statistical. **"Scramble" counting (time spent in the democratic region of the AA map)** is a portable per-pixel diagnostic for the engine.

### 5.5 Parischewsky, Trani & Leigh 2023: the equilateral-triangle experiment (perturbed Lagrange free fall)
- **Citation [CR][arXiv]:** P. Parischewsky, A. A. Trani & N. W. C. Leigh, "Chaos in the vicinity of a singularity in the three-body problem: the equilateral triangle experiment in the zero angular momentum limit", *SciPost Phys. Core* 6, 016 (2023), doi:10.21468/SciPostPhysCore.6.1.016, arXiv:2108.06335.
- **Access:** Open (SciPost and arXiv).
- **What can be replicated:**
  - **Setup:** Equal 1 M☉ masses at rest on a 1 au equilateral triangle, strictly planar, L = 0. Each body is displaced uniformly within a circle of radius r_gen.
  - **Set A:** 10⁷ runs at r_gen = 0.5 au. **Set B:** 5×10⁶ runs at r_gen = 10⁻⁷ au, near the singularity.
  - **Maps:** Outcome maps in **(S1/S2, S1/S3)** side-ratio space, coloured by escaper identity, with the equilateral point at (1,1).
  - **Classification:** 82.1% chaotic and 17.9% regular, with the N_scramble ≥ 4 classifier.
  - **Integration:** TSUNAMI (logH + Bulirsch–Stoer, tolerance 10⁻¹³) to 10⁵ yr. Escape at 100×a with E > 0.
- **Difficulty:** Semi-quantitative. **This is planar free fall, which is exactly the engine's domain**, near the Lagrange landmark (the engine's z = 0 latent landmark). The side-ratio chart maps directly onto region D or the shape sphere. The escaper map and the regular/chaotic fractions can be replicated.

### 5.6 Orlov, Rubinov & Shevchenko 2010; Mikkola & Tanikawa 2007; Urminsky & Heggie 2009: lifetime and Lyapunov statistics
- **Citations [CR]:**
  - V. V. Orlov, A. V. Rubinov & I. I. Shevchenko, "The disruption of three-body gravitational systems: lifetime statistics", *MNRAS* 408, 1623–1627 (2010), doi:10.1111/j.1365-2966.2010.17239.x, arXiv:1004.2506. 10⁵ equal-mass systems per virial ratio k ∈ {0, 0.1, …, 0.9}, at E = −1. Escape at 50×a and hyperbolic, up to t = 10⁵. Algebraic tails with differential index about −1.45 to −1.70, and integral index 0.42–0.71 (theory: −2/3).
  - S. Mikkola & K. Tanikawa, "Correlation of macroscopic instability and Lyapunov times in the general three-body problem", *MNRAS* 379, L21–L24 (2007), doi:10.1111/j.1745-3933.2007.00326.x, arXiv:0704.2671 **[arXiv]**. Covers 10⁴ runs each at k = 0 (planar free fall), 0.1 and 0.5. The fit t_d ≈ 10 t_e^1.8 holds for about 90% of runs, and the two-segment fit is t_d ≈ 5 t_e^2.3 for t_e < 6 and t_d ≈ 62 t_e above it. The t_d tail is exponential with α = 1/250.
  - D. J. Urminsky & D. C. Heggie, "On the relationship between instability and Lyapunov times for the three-body problem", *MNRAS* 392, 1051–1059 (2009), doi:10.1111/j.1365-2966.2008.14149.x, arXiv:0810.5461. This is the **Sitnikov problem**: eccentricity 0.61, ICs on a disk in (phase, ż₀), 10⁴ ICs, Bulirsch–Stoer at 10⁻¹². The same two-part t_d(t_λ) law: β ≈ 2.5 for 1 < t_λ < 6, and about 1.1 above.
- **Difficulty:** Statistical. **The Mikkola–Tanikawa free-fall (k = 0) t_d against t_λ relation is a direct calibration for an FTLE-versus-lifetime pair of colourings on region D.**

### 5.7 Leigh, Shara & Geller 2016: lifetimes of equal-mass small-N systems
- **Citation [CR]:** N. W. C. Leigh, M. M. Shara & A. M. Geller, "When does a star cluster become a multiple star system? I. Lifetimes of equal-mass small-N systems", *MNRAS* 459, 1242–1247 (2016), doi:10.1093/mnras/stw735.
- **What can be replicated:** Half-lives for N = 3 and above. Statistical, 3D.

---

## (6) Numerical reliability studies

### 6.1 Boekholt & Portegies Zwart 2015: Brutus against double precision
- **Citation [CR][arXiv]:** T. Boekholt & S. Portegies Zwart, "On the reliability of N-body simulations", *Comput. Astrophys. Cosmol.* 2, 2 (2015), doi:10.1186/s40668-014-0005-3, arXiv:1411.6671.
- **Access:** Open access. Data are on figshare (springernature.figshare.com/collections/On_the_reliability_of_N-body_simulations/3696685).
- **What can be replicated:**
  - **Ensembles:** Four sets of 10⁴ each (equal and 1:2:4 masses, virialised and cold), from Plummer N = 3. Cold sets are E = −0.25, democratic.
  - **Results:** At least half of the double-precision runs diverge from the converged solution, yet the ensemble statistics agree (KS tests) if the step parameter is ≤ 2⁻⁵ and runs with |dE/E| > 0.1 are rejected. Escaper identity is classified as accurate, preserved or exchanged.
- **Data:** **Figshare dataset available.**
- **Difficulty:** Statistical. **A direct analogue of the project's "never discard a copy" and `error_ratio` rules**, and the 1:2:4 cold set is an unequal-mass free-fall ensemble. **Caveat:** The ICs are 3D Plummer draws, not planar.

### 6.2 Portegies Zwart, Boekholt, Por, Hamers & McMillan 2022; Boekholt, Portegies Zwart & Heggie 2023
- **Citations [CR]:**
  - S. F. Portegies Zwart et al., "Chaos in self-gravitating many-body systems: Lyapunov time dependence of N and the influence of general relativity", *A&A* 659, A86 (2022), doi:10.1051/0004-6361/202141789, arXiv:2109.11012.
  - T. C. N. Boekholt, S. F. Portegies Zwart & D. C. Heggie, "Punctuated chaos and indeterminism in self-gravitating many-body systems", *IJMPD* 32, 2342003 (2023), doi:10.1142/S0218271823420038, arXiv:2308.14803.
- **What can be replicated:** The growth rate per crossing time against N, and a "punctuated" (event-driven) divergence model. No three-body IC maps. Useful only for interpreting FTLE.

### 6.3 Hristov, Hristova, Dmitrašinović & Tanikawa 2024; Li & Liao 2019: high-precision periodic free-fall ICs inside D
- **Citations [CR]:**
  - I. Hristov, R. Hristova, V. Dmitrašinović & K. Tanikawa, "Three-body periodic collisionless equal-mass free-fall orbits revisited", *CeMDA* 136, 7 (2024), doi:10.1007/s10569-023-10177-w, arXiv:2308.16159 **[arXiv]**.
  - X. Li & S. Liao, "Collisionless periodic orbits in the free-fall three-body problem", *New Astron.* 70, 22–26 (2019), doi:10.1016/j.newast.2019.01.003, arXiv:1805.07980 **[arXiv]**.
- **Access:** arXiv (open). The Hristov database, with 24 582 ICs given as (x, y, T, T*) to 80 digits, is at db2.fmi.uni-sofia.bg/3bodyfree/.
- **What can be replicated:**
  - **Domain:** Points in AA region D, with bodies at (∓0.5, 0).
  - **Li & Liao:** 234 orbits for six mass sets, including (1,1,1), (1,0.8,0.8) through (1,0.8,0.2), and (0.6,0.8,1), found on a 10⁻⁴ grid with DOP853 and confirmed by clean numerical simulation. Tables A1–A5 give (x, y, T).
    - **Verified on the paper, 27 Oct 2026.** The counts are 30, 29, 69, 44, 32 and 30 (Table I). Values are printed to 10 decimals with trailing zeros dropped. The search kept periods < 200 and minimum separations > 10⁻⁶, and accepted a return proximity < 10⁻⁶.
    - Every orbit has all velocities zero at T/2, a brake orbit (p. 3).
    - Targets LL-1 to LL-4 in `../2026-10-27_papers/TARGETS.md`.
  - **Hristov:** 12 409 distinct equal-mass solutions with T* < 80. Tables 1–2 give 35-digit ICs.
- **Difficulty:** **Exact.** Each IC must return to its starting configuration after period T, with zero escape and no collision. These are isolated, unstable, **non-escaping** pixels inside D, and the engine must reproduce them as long-lived points embedded in short-lifetime regions. The unequal-mass Li & Liao sets give **free-fall test points on the mass simplex**. Hardest-to-fake integrator test in the list. **Caveat:** The orbits are unstable, so expect closure only to about the integrator's precision times e^{T/τ_L}. Use the short-period ones.

---

## Sitnikov (the one restricted exception)
- **Urminsky & Heggie 2009** (above, §5.6): an IC disk map in (phase, ż₀) at e = 0.61, with escape time and Lyapunov time per IC.
- **T. Kovács & B. Érdi, "Transient chaos in the Sitnikov problem"**, *CeMDA* 105, 289–304 (2009), doi:10.1007/s10569-009-9227-0 **[CR]**. Escape rate κ = 0.126 (τ = 7.88) at e = 0.57, as quoted in Kovács' slides; the attribution to which paper and volume is unclear. The slides cite "CeMDA 104, 237", but Crossref gives 105, 289, so treat 104, 237 as **UNVERIFIED**.
- **R. Dvorak, "Numerical results to the Sitnikov-problem"**, *CeMDA* 56, 71–80 (1993), doi:10.1007/BF00699721 **[CR]**. Stability maps in (z₀, ż₀) for several eccentricities (**UNVERIFIED** detail).
- **Collinear relatives** (a planar subset): J. Hietarinta & S. Mikkola, "Chaos in the one-dimensional gravitational three-body problem", *Chaos* 3, 183–203 (1993), doi:10.1063/1.165984 **[CR]**. Poincaré-section maps over a 100×180 lattice of ICs for two mass choices, and 10×18 lattices for 105 mass choices, separating scattering "scallops", chaotic scattering and quasi-periodic regions. Also K. Tanikawa & S. Mikkola, "One-dimensional three-body problem via symbolic dynamics", *Chaos* 10, 649–657 (2000), doi:10.1063/1.1287064 **[CR]**, and M. M. Saito & K. Tanikawa, *CeMDA* 98, 95–120 (2007), doi:10.1007/s10569-007-9070-0, and 103, 191–207 (2009), doi:10.1007/s10569-008-9175-0 **[CR]**. These are relevant if the engine exposes the collinear boundary Γ1 of region D.

---

## Blocked or unread (for follow-up)
- The arXiv abstract page for 2403.03247 and the ar5iv pages for 0704.2671 were blocked; the PDFs were read instead.
- research.utu.fi (BotStopper) blocked the Tanikawa–Mikkola and Gargantuan records; Crossref was used.
- These figures were not read: Heinämäki et al. 1999 (MNRAS 310, 811), Martynova et al. 2009 (Astron. Rep. 53, 710), Boyd & McMillan 1993, and the Umehara–Tanikawa figures. All are paywalled or abstract-only.
- *27 Oct 2026:* Aarseth et al. 1994a (CeMDA 58, 1) and Agekyan & Anosova 1967/68 (Sov. Astron. 11, 1006) are now read from the PDFs, and are removed from this list. *Was:* "…, Aarseth et al. 1994 (CeMDA 58, 1), Boyd & McMillan 1993, …" and "Agekian & Anosova 1967/68 (Sov. Astron. 11, 1006) was not verified against ADS."

## Integrator and data availability summary
- **Public codes:** Brutus (in AMUSE); Fewbody (Fregeau et al. 2004, public); ARC/logH (Mikkola, on request or partly public). TSUNAMI public availability is **UNVERIFIED** (described in arXiv:2206.10583).
- **Public datasets:** The Hristov et al. free-fall periodic-IC database (80 digits); the Li & Liao periodic-orbit tables (numericaltank.sjtu.edu.cn); the Boekholt & Portegies Zwart 2015 figshare collection; the Šuvakov & Dmitrašinović orbit gallery.

*28 Oct 2026 (licence audit): `aarseth_anosova_orlov_szebehely_1994_CeMDA_58_1.pdf`, `aarseth_anosova_orlov_szebehely_1994b_CeMDA_60_131.pdf` are held locally, not in the repo, at `docs/incoming/_local/` (gitignored); the citations above stand, and `docs/incoming/2026-10-25_replication/README.md` keeps each file's row and sha256.*
