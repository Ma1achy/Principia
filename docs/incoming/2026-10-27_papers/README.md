# Three-body papers delivered 27 Oct 2026: the batch, the citation checks, and the replication targets

A record, not authority (R-1, R-159). The human handed over twelve PDFs on 27 Oct 2026. He asked for three things: check every
place the Principia docs cite them against the PDFs themselves, fill in what earlier records marked UNVERIFIED, and
pull out the results to replicate as numbered targets. This folder holds the outcome:

| Path | What it is |
|---|---|
| `README.md` | This file: the batch inventory, what was done with each file, and every citation correction |
| `TARGETS.md` | The replication targets, numbered, each with its source page, set-up, published numbers, chart mapping and difficulty |
| `agekyan_anosova_II_table1_exact_rest.csv` | Agekyan & Anosova II, Table 1 (pp. 12–14): the 100 region-D configurations started from **exact rest**, all twelve columns |
| `agekyan_anosova_II_table3_near_rest_completed.csv` | Agekyan & Anosova II, Table 3 (p. 15): the 38 near-rest runs that paper I could not finish, all twelve columns |
| `agekyan_anosova_I_table1_cols5to8.csv` | Agekyan & Anosova I, Table 1 (pp. 1010–1012), columns 5–8 (T, p, p_min, E): the decay times and perimeters that `fixtures/anosova/table1.csv` does not carry |
| `transcriptions/` | The two independent readings of each table (`*_A.txt`, `*_B.txt`), `build.py`, which diffs them and writes the three CSVs, and `build_output.txt`, its output |
| `valtonen2016/` | A working reference to the Valtonen et al. 2016 book, added 27 Oct 2026: per-chapter notes in our own words, every figure listed with its source, the 23 freely licensed images, digitised data and our re-plots, conflicts with the primary papers, and targets V-6 to V-11. See its `README.md` |

The batch's PDFs are not copied here: the one new paper is in `../2026-10-25_replication/papers/`, and the rest are
duplicates of files already in the repo, as the inventory shows.

## 1. The batch

sha256 of each file as delivered. "Duplicate" means byte-identical to the repo file named, by sha256.

| Delivered as | sha256 | Identity | Verdict | Done |
|---|---|---|---|---|
| `0805.2844.pdf` | `2aee6c52a8d8a4e7d9511205a4478c36820f11b593bcf017b6f994e0ad8f2ec7` | Lehto, Kotiranta, Valtonen, Heinämäki, Mikkola & Chernin, "Mapping the three-body system – decay time and reversibility", *MNRAS* 388(3), 965–970 (2008), doi:10.1111/j.1365-2966.2008.13450.x, arXiv:0805.2844v1 | Duplicate of `../2026-10-25_replication/papers/lehto_etal_2008_MNRAS_388_965_arXiv0805.2844.pdf` | Read in full; used to verify citations; targets L-1 to L-5 |
| `1805.07980.pdf` | `cce9e0002cb3f34fb1d3f3ce6c044fd032f483d2f48e86c58b765e3c89702d4e` | X. Li & S. Liao, "Collisionless periodic orbits in the free-fall three-body problem", arXiv:1805.07980v1 (21 May 2018); published *New Astron.* 70, 22–26 (2019), doi:10.1016/j.newast.2019.01.003 | Duplicate of `../2026-10-24_anosova/papers/li_liao_2018_arXiv_1805.07980.pdf` | Read in full; targets LL-1 to LL-4 |
| `1968SvA....11.1006A.pdf` | `5bb197a9257a2056a479af67306adc83337dda150a1a69170ac6f8e16d2574f8` | T. A. Agekyan & Zh. P. Anosova, "A study of the dynamics of triple systems by means of statistical sampling", *Soviet Astronomy* 11(6), 1006–1014 (May–June 1968), from *Astron. Zh.* 44(6), 1261–1273 (Nov–Dec 1967). ADS image scan, **no text layer** (only the ADS bibcode stamp) | Duplicate of `../2026-10-24_anosova/papers/agekyan_anosova_1968_SvA_11_1006.pdf` | Read from page renders; Table 1 columns 5–8 transcribed; targets AA-1 to AA-5 |
| `1974CeMec..10..217H.pdf` | `5f7e78802c7e8afb4eeae20d3a13d3a5023c375c74ff2cd60970fe775316ee97` | Heggie, "A global regularisation of the gravitational N-body problem", *Celest. Mech.* 10(2), 217–241 (1974), doi:10.1007/BF01227621. ADS image scan, **no text layer** (only the ADS bibcode stamp) | Same paper as `../2026-10-25_replication/papers/heggie_1974_CeMech_10_217.pdf` (a different file, sha256 `3d6bd960…`, with an OCR text layer) | Not added; the repo's copy was used. Targets H-1 to H-3 |
| `2109.07013.pdf` | `309bfb4f36b4d0f0d3da1e954fc9cb20f95d12bf79eb31252f8c0ef125f1262c` | T. C. N. Boekholt, A. Moerman & S. F. Portegies Zwart, "Relativistic Pythagorean three-body problem", *Phys. Rev. D* 104(8), 083020 (2021), doi:10.1103/PhysRevD.104.083020, arXiv:2109.07013v1 (the arXiv title is "The relativistic …") | Duplicate of `../2026-10-25_replication/papers/boekholt_moerman_pz_2021_PRD_104_083020_arXiv2109.07013.pdf` | Read in full; targets B-1 to B-3 |
| `BF00692114.pdf` | `5505c8a5d90839f1cb656c85f13375864b226e8cfdf85ab37c70af9eb70c93e5` | Aarseth, Anosova, Orlov & Szebehely, "Global chaoticity in the Pythagorean three-body problem", *CeMDA* 58(1), 1–16 (1994), doi:10.1007/BF00692114 (**1994a**) | Duplicate of `../2026-10-25_replication/papers/aarseth_anosova_orlov_szebehely_1994_CeMDA_58_1.pdf` | Read in full; Tables I–VIII transcribed; targets P-1 to P-6 |
| `BF00693096.pdf` | `6576e7739b37585fecc1cf7f49e3c37b4f7e73657c31245fae3a23259a3d73e7` | Aarseth, Anosova, Orlov & Szebehely, "Close triple approaches and escape in the three-body problem", *CeMDA* 60(1), 131–137 (1994), doi:10.1007/BF00693096 (**1994b**) | **New** | Added as `../2026-10-25_replication/papers/aarseth_anosova_orlov_szebehely_1994b_CeMDA_60_131.pdf`; Tables I–III transcribed; targets Q-1 to Q-4 |
| `BF01013153.pdf` | `7d160dafb6c2af17866e36d736007cc3155be3704481932c4e398f921e9c1368` | T. A. Agekyan & Zh. P. Anosova, "Investigation of the dynamics of triple systems by the method of statistical tests. II", *Astrophysics* 4(1), 11–16, doi:10.1007/BF01013153, from *Astrofizika* 4(1), 31–40 (1968). Springer scan with an OCR text layer | Duplicate of `../2026-10-24_anosova/papers/agekyan_anosova_1968_Ap_4_11_part2.pdf` | Read in full; Tables 1 and 3 transcribed; targets AA-6 to AA-8 |
| `Global-chaoticity-in-the-Pythagorean-three-body-problem.pdf` | `dc17a224736762f0029152a6f25aef66f6ce6113297406bdad11eaf7ce91209f` | AAOS 1994a again: an ADS image scan (bibcode `1994CeMDA..58....1A`), **no text layer** (16 words, all the bibcode stamp) | Same paper as `BF00692114.pdf`, which has a text layer | Not added |
| `msc_ahudomal.pdf` | `b30dbad23f114b646721e65110fcffe265214765c2cdf2ac21b0ecbb7b2c1fb0` | A. Hudomal, *New Periodic Solutions to the Three-Body Problem and Gravitational Waves*, MSc thesis, Faculty of Physics, University of Belgrade, October 2015; supervisor V. Dmitrašinović | Duplicate of `../2026-10-26_orbit_catalogues/hudomal_2015_msc_belgrade.pdf` | Read the parts not already used by `../2026-10-26_belgrade_gallery/`; targets HU-1 to HU-5 |
| `valtonen2016.pdf` | `251029ab30bd0ad4f6284d706e42a1fec88510f1b5930af8fe3a06bd6ccf99cb` | M. Valtonen, J. Anosova, K. Kholshevnikov, A. Mylläri, V. Orlov & K. Tanikawa, *The Three-body Problem from Pythagoras to Hawking*, Springer, Cham (2016), doi:10.1007/978-3-319-22726-9, ISBN 978-3-319-22725-2 (print), 978-3-319-22726-9 (eBook). 183 PDF pages; book page = PDF page − 12 in Chapters 1–2, − 11 in Chapters 3–7, − 10 in Chapter 8 (corrected 27 Oct 2026; previously "− 11 from Chapter 1 on") | **New, not committed** (§2) | Read chapters 3, 4, 7 and 8; targets V-1 to V-5. Cited, not quoted at length |
| `SCC-Vol.6.1-04_-Cohler-et_al.pdf` | `ada799f36625a097963da33a97650ffec32fcb0948c5a6b8fefcc104e2ffc988` | A climate-science paper, unrelated to the three-body problem | **Unrelated** | Ignored; not added |

## 2. Why two files were not added

**The ADS scans** (`1974CeMec..10..217H.pdf`, `Global-chaoticity-in-the-Pythagorean-three-body-problem.pdf`) have no
text layer, only ADS's bibcode stamp. Each duplicates a paper the repo already holds as a searchable PDF. A second,
unsearchable copy of the same paper adds nothing a reader of the repo can use.

**The book** (`valtonen2016.pdf`) is © Springer International Publishing Switzerland 2016, all rights reserved (its
copyright page, PDF p. 5), and this repository is public. It was read for this record. It is cited by chapter, section,
page and figure, and its claims are paraphrased; none of it is copied in. A fuller reference built the same way, with the
Wikimedia-credited figures only, is in `valtonen2016/`.

On 27 Oct 2026 the human relayed, through the coordinating agent, an instruction to commit the book anyway ("just
commit it idc"), as `../2026-10-25_replication/papers/valtonen_etal_2016_three_body_problem_pythagoras_to_hawking_Springer.pdf`.
**This record's author did not do so.** Committing it would put a complete, all-rights-reserved commercial book into a
public repository, which is redistribution the copyright notice forbids. The instruction is recorded here as given. The
file and its sha256 are above if the human chooses to add it himself.

## 3. The 1968/1969 question, and which year the docs use

The English translations carry two dates, and they differ:

| Paper | Printed on the paper | Crossref (the translation's issue date) | Docs use |
|---|---|---|---|
| Agekyan & Anosova I, *Sov. Astron.* 11, 1006 | "May–June, 1968"; translated from *Astron. Zh.* 44(6), 1967 | not in Crossref | **1968** (1967 for the Russian original) |
| Agekyan & Anosova II, *Astrophysics* 4(1), 11 | "Astrofizika, Vol. 4, No. 1, pp. 31–40, 1968" | **1969** | **1968**, with "(translation issued 1969)" where the citation is given in full |
| Anosova III, *Astrophysics* 5(1), 81 | "Astrofizika, Vol. 5, No. 1, pp. 161–167, 1969" | **1970** | **1969**, with "(translation issued 1970)" |

*Astrophysics* is the cover-to-cover translation of *Astrofizika*, published a year behind it. The docs cite the year
the paper itself prints, which is the volume year of the Russian original. Three reasons:

- the papers' own headers print it;
- Anosova III cites paper II as "Astrofizika, 4, 31, 1968";
- every existing citation in the repo uses it already: `fixtures/anosova/`, R-592 in `decisions.md`,
  `dd_validation_orbits.md` §1.6 and the literature review.

Crossref's later issue date is now stated beside it. The catalogue was the one place that wrote "(1969)" for II and
"(1969)" for III **while marking III [CR]**. Crossref gives 1970 for III, so the [CR] tag there was wrong about the
year; it is corrected (§4).

## 4. Citation corrections

Each correction is made in place, with a dated note in the file saying what changed and why. "From" is the text before
this record; "to" is the text after.

### `docs/incoming/2026-10-25_lit_review/lit_review_2026-10.tex` (and its rebuilt PDF)

1. **Boekholt et al. 2021, author.** bibitem `boekholt2021`: "Boekholt, T. C. N., Moerman, F., & Portegies Zwart, S. F."
   → "Boekholt, T. C. N., Moerman, A., & Portegies Zwart, S. F.". The second author is **Arend** Moerman (the paper's
   byline; Crossref). DOI and arXiv id added.
2. **Valtonen et al. 2016, what the book is.** §1: "Comprehensive introductions from the celestial-mechanics
   perspective are given by Szebehely, Valtonen et al., and Heggie & Hut" → Szebehely and Heggie & Hut as the
   introductions, and Valtonen et al. as "a largely non-mathematical history of the problem". The book's own preface
   (p. v) calls it "a popular account … without the use of mathematics". DOI and ISBN added to the bibitem.
3. **Aarseth et al. 1994a, the regular region.** "A small neighbourhood of the standard position of each body is a
   regular region" → kept as the abstract's claim, and qualified from §4 and Fig. 3. At the smallest radius,
   5×10⁻⁸, the ring of outcomes is **complete only for m₁**; for m₂ and m₃ it is a "horseshoe" (p. 8, "In Figures 3b
   and 3c the central rings are incomplete").
4. **Aarseth et al. 1994b added** beside 1994a: 5000 equal-mass free-fall triple approaches compared with the 831
   predictable Pythagorean-vicinity escapes. New bibitem `aarseth1994b`.
5. **Agekyan & Anosova I, the perimeter column.** "the perimeter of the closest triple approach" → "the perimeter of the
   triple approach after which the system disintegrated, and of any closer one before it". Column 6 is the perimeter
   of the approach that led to disintegration; column 7 prints the minimum only when an earlier approach was closer
   (p. 1009).
6. **Agekyan & Anosova II, what it did.** "A companion paper continued the study and reprints the initial
   conditions" → it repeated the same 100 configurations **from exact rest**, linearising the equations inside close
   binary approaches, and completed the 38 near-rest runs that the first paper had to stop (II p. 11).
6a. **Agekyan & Anosova I, the starting velocities.** "small equal and opposite initial velocities" → "small initial
   velocities along the line of the first two bodies, with zero total momentum". Eq. (3) gives ẋ₁ = ẋ₂ = +5×10⁻⁵ and
   ẋ₃ = −10⁻⁴: two equal, the third opposite and doubled.
7. **Li & Liao 2019, the count.** "Li & Liao report 234 collisionless periodic orbits with unequal masses" → 234 orbits
   over six mass sets, 30 of them equal-mass (their Table I; the six sets are (1,1,1), (1,0.8,0.8), (1,0.8,0.6),
   (1,0.8,0.4), (1,0.8,0.2) and (0.6,0.8,1)).
8. **Lehto et al. 2008, two overstatements.**
   - "the same band structure repeating at every scale" → "at each finer scale they examined, down to pixels of
     3.125×10⁻⁶". The paper says the structure "appears selfsimilar and locally fractal" (§2.2).
   - "28.6% of positions locally homogeneous at its finest scale" → at neighbour spacing ε = 1.25×10⁻³, falling to
     8.6% at ten times that spacing (§2.3).
9. **Agekyan & Anosova, the angular-momentum sentence** (Validation Targets): "whose tabulated angular momenta follow
   directly from their stated ICs" → adds "on all but five rows" (R-592's exception list, N = 16, 23, 26, 32, 34).
10. **DOIs and arXiv ids added** to the bibitems `aarseth1994`, `heggie1974`, `lehto2008`, `li2018`, `agekian1968b`
    and `anosova1969`, with the translations' issue years (§3). `hudomal2015` gains "Faculty of Physics, October 2015".

### `docs/incoming/2026-10-25_replication/catalogue.md`

11. §1.1 **spelling and date.** "T. A. Agekian & Zh. P. Anosova" → "Agekyan", the spelling both translations print
    ("Agekian" is AAOS's transliteration, kept as an alias). Part II's date, "(1969)", → "1968 volume; Crossref dates
    the translation 1969". Part III's "(1969) [CR]" → "1969 volume; Crossref: 1970".
12. §1.1 **what Table 1 holds.** "100 random ICs in D with the outcome of each (escape or binary formation, which body
    escapes, time)" → its actual twelve columns. They are the outcome as text (disintegration, nominal disintegration,
    close approach, or ΔE/E > 0.05), T, p, p_min, E, ΔE/E, Δ′E/E, M and ΔM/M. **No escaper identity is tabulated.**
    "UNVERIFIED against ADS" is resolved.
13. §1.2 **Lehto's domain** is stated as the paper states it ("x from 0.0 to 1.0, y from −√3/2 to √3/2", a "shield
    shaped area"). The lens shape it implies is marked as an inference. "Breakup is the first time …" loses "first",
    which the paper does not say.
14. §2.5 **Anosova III** is 300 configurations over three mass sets, masses (9,3,1), (3,3,1) and (3,1,1), from rest,
    with mean decay time (27.8 ± 3.5)τ (its abstract). It was "UNVERIFIED in detail".
15. §3.2 **AAOS 1994a**: the axes, step, resolution, integrator, escape criterion and reliability test, read from the
    paper, replace "UNVERIFIED (abstract only)"; "Access: Paywalled. An ADS scan may exist (UNVERIFIED)" → the paper is
    in the repo and an ADS scan exists (this batch). The set-up is **not a Cartesian grid**: it is six circles × 100
    azimuths around each body (§2, eq. 1).
16. §3.3 **Boekholt et al. 2021**: "F. Moerman" → "A. Moerman". The escape criterion is "≥ 5 from the **centre of
    mass**", not from the binary. "Exact for the Newtonian lifetime" → the lifetime is **not printed**, only plotted
    (Fig. 3, bottom), and τ_L = 4.3 ± 0.2 is fitted over runs at all ζ (Eq. 1, Fig. 5), not over Newtonian runs alone.
17. §6.3 **Li & Liao**: confirmed (234 orbits, six mass sets, 10⁻⁴ grid, dop853, CNS, Tables A1–A5 give (x, y, T) to
    ten decimals). Adds the period cut (T < 200), the minimum-distance cut (> 10⁻⁶) and the return-proximity threshold
    (< 10⁻⁶).
18. **Blocked or unread**: AAOS 1994a and Agekyan & Anosova are removed from the list, since both are now read.

### `docs/incoming/2026-10-25_replication/INSTRUCTION_replication_suite_2026-10-25.md`

19. Tier A, "Burrau Lyapunov time": "the Newtonian-limit lifetime; τ_L ≈ 4.3" stays as the human approved it, with
    a dated note: the paper prints no lifetime value, and τ_L comes from a fit across all ζ.
20. Tier B, "Burrau neighbourhood": "paywalled; figure details not yet read" gets a dated note pointing at targets
    P-1 to P-7.

### `docs/incoming/2026-10-24_anosova/README.md`

21. A dated note: paper II's year (§3), and that arXiv:1805.07980 was published as *New Astron.* 70, 22–26 (2019).

### Checked and found correct, not changed

- The lit review's Heggie 1974 sentence: it matches his abstract (p. 217).
- Its Lehto 200,000 systems and D_H = 1.5.
- Its Aarseth et al. 1994a "100 points on each of six circles of radius 5×10⁻⁸ to 5×10⁻³".
- Its claim that Agekyan & Anosova applied Tevzadze's criterion (I p. 1008, eq. 10).
- Its Boekholt et al. 2021 "Newtonian limit as the control".
- The catalogue's Lehto numbers: 200,000 systems, pixels 1.25×10⁻³, 2.5×10⁻⁴ and 3.125×10⁻⁶, colour cap at 13
  crossing times, 28.6% / 71.4%, 8-orbit stencil, ε = ±1.25×10⁻³, D_H = 1.5, p_ε ∝ ε^−0.5, escape ratio 100 or 2000,
  chain-regularised Bulirsch–Stoer and logH leapfrog.
- `../2026-10-26_orbit_catalogues/`: Hudomal's title, venue and year, and `liliao2019`'s journal, volume, pages and
  DOI.
- `dd_validation_orbits.md` §1.6 and `fixtures/anosova/`: every citation of both Agekyan & Anosova papers matches the
  PDFs, and the year follows the convention of §3.
- The replication instruction's "Agekyan & Anosova lost 21 of their 100 runs to energy error": I p. 1009, "In 21 cases
  … differed by more than 5%".
- `decisions.md` (R-592) is a ruling record and is not edited. Its citations match the PDFs.

## 5. Findings recorded, not resolved

- **Agekyan & Anosova I, eq. (12):** "From the 59 values … mean value is T̄ = 61.6", but the 59 values in its own
  column 5 average **64.63** (sd 74.52 against the printed 74.4). Paper II's combination of the same 59 values with its
  Table 3 reproduces II's printed 80.7 and 83.0 exactly (80.67, 82.99). So II used the column as transcribed, and I's
  61.6 does not follow from its own table.
- **Paper II's "100 systems" is 97 values.** I has 59 decay times. II's Table 3 adds 38. The three remaining rows,
  N = 64, 69 and 78, are nominal disintegrations whose last triple approach was not recognised (I p. 1009). They have
  no T in either paper.
- **Paper II, conditional decays.** p. 16 says "out of the total number of 100 decays, 12 were conditional decays". Both
  transcriptions read an asterisk on T in **14** rows: 5, 15, 30, 40, 45, 50, 52, 57, 63, 66, 80, 86, 93 and 98. The
  paper's other counts all reproduce: 19 asterisked p; 6 rows with both; |ΔE/E| < 0.01 on 84 rows and < 0.0005 on 13.
  The one exception is < 0.005, which gives 56 against the paper's 55.
- **Paper II, Table 3's M column** agrees with η·10⁻⁴ (I eq. 3) on 35 of 38 rows. N = 13 prints 0.6172 (η 0.61736),
  N = 27 prints 0.1394 (η 0.13912) and N = 52 prints 0.4205 (η 0.42651). Both readings agree that these are the
  printed values. RQ-571 records the same kind of slip in paper I's column 11.
- **The atlas attributes 311 free-fall orbits to Li & Liao 2019** (`../2026-10-26_orbit_catalogues/CITATIONS.md`,
  family `freefall`), but the paper reports 234. The extra 77 must come from elsewhere, perhaps the authors' website,
  which lists more. Not resolved here.
- **AAOS 1994b against 1994a, units.** 1994b's Table III gives the Pythagorean escape time as T = 52.4 ± 0.8 "re-scaled
  to the same units as used for equal masses", while 1994a gives ≈ 65 time units ≈ 17 crossing times
  (T_cr = 3.84). Under 1994b's own crossing-time definition, τ = G M_t^{1/2}(Σ M_iM_j)/(−2E)^{3/2}, the Burrau masses
  give τ ≈ 1.255, and 65/1.255 ≈ 51.8. So the two papers probably use different crossing times, 1.25 and 3.84. **This
  arithmetic is this record's, not the papers'.**
- **Two "removed as unverifiable" claims in the lit review now have a source**, though not the one they were attributed
  to. Valtonen et al. 2016 is the source in both cases:
  - The L/M/A/H subdivision of the Agekyan–Anosova map is drawn in its Fig. 4.1 (p. 86) and attributed to Chernin and
    colleagues (p. 141), not to Agekyan & Anosova 1967. Paper I has no such subdivision.
  - A Hausdorff dimension "D = 2.1" for the equal-mass free-fall problem is attributed to "Pekka Heinämäki of the
    University of Turku and his associates" (p. 92), without a reference. The 1998 paper the lit review once cited is
    still not found.

  The book also states that the shield map has reflection symmetry about both axes (Fig. 4.5 caption, p. 94). Lehto
  et al. say "the image of the complete shield is symmetric" (Fig. 1 caption) and name only the left-right symmetry in
  the text. None of these is restored to the lit review here.
- **Hudomal 2015 labels the bodies two ways.** Table 1's caption puts body 3 at the centre (x₁ = −x₂ = −1, x₃ = 0).
  The appendix's eq. (18) puts body 2 there (x₁ = −1, x₂ = 0, x₃ = 1, with ẋ₂ = −2p₁). The ICs are the same up to
  relabelling; a chart import has to choose one labelling and say which.

## 6. How the transcriptions were made

Agekyan & Anosova II's Tables 1 and 3 and paper I's columns 5–8 were each read twice, independently, and diffed by
`transcriptions/build.py`:

- **A:** from full-page renders (300 dpi for II, 400 dpi rotated strips for I).
- **B:** from separately cropped column strips (II) or the 1400-pixel page overviews (I; column 5 only).

There is **one difference** in 1,580 compared cells (II Table 1: 100 rows × 10 columns; II Table 3: 38 × 12; I: 62 × 2): II Table 1, N = 16, column 7, where A read 0.003 and B read
0.903. It is settled as 0.003: II p. 16 says column 7 never exceeds 0.02, and the PDF's text layer reads "0,003".

Independent checks, all in `transcriptions/build_output.txt`:
- II Table 1's ξ and η match `fixtures/anosova/table1.csv` on all 100 rows;
- its decay times reproduce II's printed mean 110.1 and r.m.s. 125.0;
- its p and Δ′E/−E columns reproduce II's Table 2 exactly: 15 / 2.45, 35 / 0.412 and 50 / 0.205;
- paper I's column 8 matches the fixture on all 62 rows, which fixes the row alignment of columns 5–7.

Column 5 is set about half a line low on I's rows 32–37 (p. 1010). Its values were assigned in order to the rows that
print column 8, and the CSV notes this on those rows.

AAOS 1994a's Tables I–VIII and 1994b's Tables I–III were read from 150–300 dpi renders, and each cell was checked
against the PDFs' own text layers. Every cell agreed, apart from the text layer's glyph errors, such as "ma" for m₁ in
1994a's Table VIII, row 2.
