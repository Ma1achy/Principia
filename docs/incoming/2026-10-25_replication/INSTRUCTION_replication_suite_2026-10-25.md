# Replication suite: reproduce published three-body results as engine tests

The human's instruction of 25 Oct 2026. His words:

> "thought maybe the agent could replicate these results? act as good tests on the engine and stuff? what can we
> replicate? want to plan this"

> "thought the agent could replicate say the basin maps from the isles of regularity paper etc, and maybe any other
> basin maps found before or outcome maps / tables etc"

> "but yes this plan A-D seems good."

This file is the plan. `catalogue.md`, beside it, is the source list: about 35 published items with their citations,
what each one publishes, and how hard it is to reproduce. Citations marked [CR] were checked against Crossref; anything
marked UNVERIFIED was not.

## Why

The existing validation suite (`dd_validation_orbits.md` §3) tests the integrator on periodic orbits. Its own §4 says
those are "necessary, not sufficient": periodic orbits are measure-zero and non-chaotic, so they say nothing about the
chaotic sea, which is what the instrument maps. Published maps and statistics of the general three-body problem are
independent measurements of exactly that regime. A shared derivation bug can survive the Python cross-check; it cannot
survive a result someone else computed with a different code.

## Rules (the ones that worked for Anosova, RQ-566 to RQ-573)

1. **The source is in the repo.** Each paper goes in `docs/incoming/<date>_<topic>/` with a README giving its citation
   and sha256. A value cited from memory or a secondary source is not a reference value.
2. **Transcribe twice.** Every table is transcribed twice, independently, and the two copies are diffed before the
   commit. Each row records its page.
3. **Tolerances are calibrations (R-71).** Each is proposed from a measurement, with its evidence, and batched for the
   gate.
4. **Never tune to match.** A disagreement is queued in `REVIEW_QUEUE.md` with both numbers, as RQ-570 was. It is not
   absorbed into a tolerance, a fit or an exception list without a ruling.
5. **Old integrators were weaker than ours.** Where we disagree with a pre-1990 result, the default suspicion is
   theirs, but it must be shown, not assumed. Agekyan & Anosova lost 21 of their 100 runs to energy error, and they
   say so themselves.
6. **Remap before comparing.** Coordinate conventions differ between papers. Agekyan–Anosova put two bodies at
   (∓½, 0). Lehto et al. use (0, 0) and (1, 0). Tanikawa uses the mirror labelling D11. Each map is converted into one of
   our charts, and the conversion is a tested function, not a figure overlay.
7. **State the escape criterion.** Published lifetimes depend on the criterion used (50a or 100a plus hyperbolic;
   r > 10, receding, E > 0; Fewbody's tidal tolerance). Outcome identity is robust once an escape is real; a lifetime is
   not. Each comparison records the paper's criterion and ours.
8. **A chaotic result is compared as a distribution, never pixel for pixel.** The exception is a result computed with
   a converged code (Brutus, clean numerical simulation). Those are trajectories, and can be compared per point.

## A. Exact: deterministic, matched to a tolerance

| target | source | what is asserted | needs |
|---|---|---|---|
| Burrau | Szebehely & Peters 1967 (*AJ* 72, 876; ADS free) | masses 4 and 5 bind and mass 3 escapes; the close-approach times and distances; breakup near t ≈ 59.4 | M3 integrator; a horizon past 59 |
| Periodic free-fall orbits | Hristov et al. 2024 (*CeMDA* 136, 7; 24,582 ICs to 80 digits in a public database); Li & Liao 2019 (*New Astron.* 70, 22; 234 orbits over six mass sets, unequal included) | each IC closes after its period T, no collision, no escape, at the integrator's order. Short periods only (they are unstable) | region D chart; mass simplex for the unequal sets |
| Velocity-slice periodic orbits | Šuvakov & Dmitrašinović 2013 (*PRL* 110, 114301; 15 ICs) | closure after T. On a chart over (ẋ₁, ẏ₁), the return-proximity or FTLE map has minima at the tabulated points | a chart with momenta: the first published non-zero-velocity test |
| Period against word length | Dmitrašinović et al. 2018 (*J. Phys. A* 51, 315101) | the published free-group words, and the near-linear T·\|E\|^{3/2} against N_w within each sequence | the word reader (symbolic dynamics contract) |
| Triple-collision points in D | Tanikawa & Mikkola 2021 (*CeMDA* 133, 52; Table 2: eleven points, e.g. (0.19208270, 0.30936018)) | binary-collision curves converge on each point; d_min scales towards it along the tabulated approach ICs (Table 4, at radii 1e-3 to 1e-5) | region D chart; regularisation |
| The Pythagorean family | Chitan, Mylläri & Haque 2021 (*MNRAS* 509, 1919): all 16 Pythagorean triangles with hypotenuse < 100, Newtonian runs | lifetimes and two-body encounter counts per triangle | the Burrau family charts. **This is the Burrau programme's own family, already integrated by someone else** |
| Burrau Lyapunov time | Boekholt, Moerman & Portegies Zwart 2021 (*PRD* 104, 083020; converged) | the Newtonian-limit lifetime; τ_L ≈ 4.3 | FTLE |
| Region D Table 1 | Agekyan & Anosova 1968 | under way in TASK-M2-20 | — |

The figure-eight closure test and the Lagrange and Euler landmarks are already in `dd_validation_orbits.md` §3. They
stay there.

## B. Maps and statistics: the chaotic sea

| target | source | what is compared | notes |
|---|---|---|---|
| **Converged lifetime grid over D** | Boekholt, Portegies Zwart & Valtonen 2020 (*MNRAS* 493, 3932; arXiv:2002.04029) | 1,212 points at spacing 1/64, every one converged with Brutus to t = 10⁴. Per-point where our run is reliable; otherwise a KS test on the lifetime distribution | **The best ground truth there is for region D**: converged by construction. Their lifetime runs to the last close approach before ejection, not to the criterion firing |
| The Maasai shield | Lehto et al. 2008 (*MNRAS* 388, 965; arXiv:0805.2844) | morphology of the lifetime map at their pixel sizes. Their summary statistics, reproduced with their own 8-orbit stencil: 28.6% locally homogeneous, Hausdorff dimension 1.5 | their coordinates, their escape criterion, lifetimes in crossing times |
| Symbol-sequence maps | Tanikawa & Mikkola 2008 (arXiv:0802.2465) and 2015 (*PASJ* 67, 115) | maps of the first 2–4 symbols over the free-fall plane, which are nearly geometric. Their Fig. 7 has first-symbol maps at **non-zero angular momentum**, virial ratio 0 to 0.7: one of very few published maps off the zero-velocity surface | their symbol convention exactly, including labelling. The velocity prescription for k > 0 has to be pinned from the PASJ paper |
| The equilateral-triangle experiment | Parischewsky, Trani & Leigh 2023 (*SciPost Phys. Core* 6, 016) | escaper-identity maps in side-ratio space around the Lagrange point; 82.1% chaotic against 17.9% regular | planar free fall: exactly our domain, at our z = 0 landmark |
| **Isles of regularity** | Trani et al. 2024 (*A&A* 689, A24) | the planar row of their map (inclination 0) as a 1D scan in binary phase: escaper sequence and their regularity statistic f_col (12 nearest neighbours, regular if > 2/3) | **Their full map is not planar**: it varies inclination. Only the ι = 0 row is in our domain. f_col is directly portable as a colouring, and their Tables 1–2 give quantitative outcome and regular fractions. Needs a binary-plus-single chart |
| Burrau neighbourhood | Aarseth, Anosova, Orlov & Szebehely 1994 (*CeMDA* 58, 1) | the regular island containing the Burrau IC, and where the escaping body changes | paywalled; figure details not yet read |
| Lifetime against Lyapunov time | Mikkola & Tanikawa 2007 (*MNRAS* 379, L21) | t_d ≈ 10 t_e^1.8 on free fall, two-segment fit | calibrates an FTLE colouring against a lifetime colouring on the same pixels |
| Lifetime tails | Orlov, Rubinov & Shevchenko 2010 (*MNRAS* 408, 1623) | algebraic tails, index about −1.45 to −1.70, per virial ratio | statistical |
| Unequal-mass escape probabilities | Manwadkar, Kol, Trani & Leigh 2021 (MNRAS, arXiv:2101.03661); Anosova 1969 (*Astrophysics* 5, 81) | per-body escape probability by mass set, matched there to about 1% | their runs are 3D; a planar re-run, not a lookup |
| Mass-plane stability map | Li, Li & Liao 2020 (arXiv:2007.10184) | period, angular momentum and stability contours over (m₁, m₂) for one orbit family | the closest published analogue to a mass-simplex chart |
| Ergodic outcome distributions | Stone & Leigh 2019 (*Nature* 576, 406) | binned binary energy and eccentricity distributions against their closed forms, which they also give for the planar case | the cut N_scramble ≥ 2 is needed for agreement |

## C. Numerical reliability

| target | source | what is asserted |
|---|---|---|
| Ensemble statistics survive trajectory error | Boekholt & Portegies Zwart 2015 (*Comput. Astrophys. Cosmol.* 2, 2; **public figshare data**) | individual double-precision runs diverge while the ensemble statistics agree (KS). This is the empirical basis of the f32/f64 honesty story, and of "never discard a copy" |
| Irreversibility | Boekholt et al. 2020 (above) | about 1% of region D is irreversible at ε = 10⁻⁷⁰. Needs a reversible occupant (logH's TTL mode) |
| Converged short trajectories | Breen et al. 2020 (*MNRAS* 494, 2465) | per-trajectory agreement with Brutus to t = 10, if their data can be obtained |

## D. The prototype's own results

prin-rs measured hundreds of numbers, each with the settings that produced it (`CLAUDE.md` in prin-rs, and its
`results/`). Each is a regression target: the new build reproduces it, or explains the difference with a measurement.
Some examples:

- figure-eight convergence order 2.08 with the landing clamp, 1.06 without;
- Heggie against Aarseth–Zare cadence sensitivity, chord 4.832e-2 against 5.162e-1 (`config_stability`, 256²);
- the outcome classifier against Fewbody's termination test: no wrong label on E < 0, settling earlier;
- f32 against f64 outcome labels agreeing on 1022 of 1024 near-field pixels;
- no near-field escape at t = 13.

A prototype number is not ground truth. It was measured once, on one implementation, and several were later
overturned. When the new build disagrees, the first question is which of the two is wrong.

## What cannot be replicated

- **Most of the scattering literature is 3D**: Hut & Bahcall, the full Trani map, Boekholt's Plummer ensembles,
  Gargantuan II. Only planar subsets apply.
- **Results with no published data and no figure detail** stay qualitative until the paper is in hand.
- **Results past the horizon cap.** RQ-555 keeps the horizon at most 200. Burrau's breakup at 59 fits; Boekholt's
  t = 10⁴ grid does not, except as a comparison over the systems that end before 200. If the cap blocks a target, queue
  it rather than raising the cap.

## Sequencing

1. **Now**, with what exists: fetch the papers, transcribe the tables, write the requirements (VAL-…) and the
   calibration entries.
2. **M3** (the integrator): tier A, tier C, and tier D's integrator entries.
3. **When the outcome classifier lands**: tier B outcome maps and statistics, and tier D's classifier entries.
4. **When a chart needs one**: the binary-plus-single chart (Trani's row, Manwadkar), the velocity-slice chart
   (Šuvakov), the side-ratio chart (Parischewsky). Each is queued as a chart question when its target comes up, not
   built ahead of it.

## Papers to fetch

**Paywalled** (the human can get these):

- Aarseth, Anosova, Orlov & Szebehely 1994, *CeMDA* 58, 1–16, doi:10.1007/BF00692114
- Heggie 1974, *Celest. Mech.* 10, 217–241, doi:10.1007/BF01227621
- Aarseth & Zare 1974, *Celest. Mech.* 10, 185–205, doi:10.1007/BF01227619
- Tanikawa & Mikkola 2015, *PASJ* 67, 115, doi:10.1093/pasj/psv087
- Anosova 1969, *Astrophysics* 5, 81–84, doi:10.1007/BF01013354
- Valtonen, Mylläri, Orlov & Rubinov 2005, *MNRAS* 364, 91, doi:10.1111/j.1365-2966.2005.09540.x
- Martynova, Orlov & Rubinov 2009, *Astron. Rep.* 53, 710, doi:10.1134/S1063772909080034
- Heinämäki, Lehto, Valtonen & Chernin 1999, *MNRAS* 310, 811, doi:10.1046/j.1365-8711.1999.02859.x
- Chitan, Mylläri & Haque 2021, *MNRAS* 509, 1919, doi:10.1093/mnras/stab3124 (also arXiv:2011.03046)

**Open:**

- Szebehely & Peters 1967 (ADS scan)
- Lehto et al. 2008 (arXiv:0805.2844)
- Boekholt et al. 2020 (arXiv:2002.04029)
- Trani et al. 2024 (arXiv:2403.03247)
- Parischewsky et al. 2023 (arXiv:2108.06335)
- Šuvakov & Dmitrašinović 2013 (arXiv:1303.0181)
- Dmitrašinović et al. 2018 (arXiv:1705.03728)
- Hristov et al. 2024 (arXiv:2308.16159, and their database)
- Li & Liao 2019 (arXiv:1805.07980)
- Tanikawa & Mikkola 2008 (arXiv:0802.2465)
- Tanikawa & Mikkola 2021 (the accepted manuscript at utupub.fi)
- Mikkola & Tanikawa 2007 (arXiv:0704.2671)
- Orlov et al. 2010 (arXiv:1004.2506)
- Stone & Leigh 2019 (arXiv:1909.05272)
- Manwadkar et al. 2021 (arXiv:2101.03661)
- Boekholt & Portegies Zwart 2015 (open access, with figshare data)
- Breen et al. 2020 (arXiv:1910.07291)
- Boekholt, Moerman & Portegies Zwart 2021 (arXiv:2109.07013)
- Li, Li & Liao 2020 (arXiv:2007.10184)

## Notes added 27 Oct 2026

Added after the papers named above were read in full (`../2026-10-27_papers/`). The human's words and the approved
tables above are left as they were; these notes correct or complete them.

- **Tier A, "Burrau Lyapunov time":** Boekholt, Moerman & Portegies Zwart 2021 **print no value** for the Newtonian
  lifetime; it is only plotted (Fig. 3, bottom; the plateau reads log₁₀ T ≈ 1.8). τ_L = 4.3 ± 0.2 is a fit over runs
  at every value of the dimensionless speed of light, relativistic ones included (Eq. 1, Fig. 5), so it is not a
  Newtonian-only number. Targets B-1 to B-3 in `../2026-10-27_papers/TARGETS.md` state what can be compared.
- **Tier A, "Region D Table 1":** paper II (Agekyan & Anosova, *Astrophysics* 4, 11) also gives the same 100
  configurations **from exact rest**, with decay times, in its Table 1. Both its tables and paper I's decay-time
  column are now transcribed, as targets AA-2 to AA-8.
- **Tier B, "Burrau neighbourhood":** the figure details are now read. The sampling is six circles × 100 azimuths
  around each body, not a grid, and Tables I–VIII are transcribed. A companion paper, Aarseth et al. 1994b (*CeMDA*
  60, 131), is now in `papers/`. Targets P-1 to P-6 and Q-1 to Q-4.
- **Papers to fetch:** Aarseth et al. 1994a and Heggie 1974 were delivered on 25 Oct 2026, and Aarseth et al. 1994b
  on 27 Oct 2026. Li & Liao (arXiv:1805.07980) is the *New Astron.* 70, 22 paper.
