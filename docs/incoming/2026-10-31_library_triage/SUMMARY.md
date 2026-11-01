# Library triage, 31 Oct 2026

Task 15d. Sources: `~/Downloads/principia-research-library`, read by chunk, merged here against
`docs/incoming/2026-10-27_papers/TARGETS.md` and `docs/incoming/2026-10-27_validation_plan/PLAN.md`. One row per item in
`triage.csv`.

**Proposals only.** Nothing here is a target until the owner rules. No requirement or plan changes. Tables are
transcribed later, twice, for approved items only.

## Words used

- **Already have:** data we hold; it stays usable, as a target or anything else. Not "excluded".
- **Rejected:** ruled out. Only dosier, stipefrkovic (R-642 item 2, RQ-598 scoring), Farrés 2017, Ferreira & Prado 2023.
- **OFF-FILTER:** fails the filter (three massive bodies, planar subset, printed numbers or a data file). The readers'
  brief called this EXCLUDE; the word is changed so it can't be read as a ruling.
- **NOW:** a cheap, non-gating peek at data already usable. **M9:** at or after M9, with new measurement or machinery.
- **`[value]`** in `triage.csv`: a number taken from a `_local` source, rounded ones included: results, coefficients,
  thresholds, run settings and start coordinates, unless it is on the kept list below. The page and table reference stays,
  so the number is one lookup away. This masking is a judged reading of each `_local` row, not a proof: a number left in was
  judged to fit the list. Kept in those rows: references (page, table, equation, section); counts of what a source holds
  (rows, tables, runs, digits); units and normalisation labels such as G=1; exact values that define a set-up or name a
  family (zeros, pi, 180 degrees, mass ratios such as 1:2:3, resonance names); the Burrau start, which is already a target;
  and our own measurements that carry no source value. Atlas rows are `_local` because of `atlas-params.json`; their figures
  come from the committed CSV and catalogue and from our own sample. `[digits in source]`: a long digit string, left out
  everywhere.

## Counts

| Verdict | Rows |
|---|---|
| HIT | 111 |
| PLOT-ONLY | 21 |
| CONTEXT | 58 |
| OFF-FILTER | 27 |
| NOT-READ | 72 |
| REJECTED | 4 |
| Total | 293 |

| Recommendation | Rows |
|---|---|
| NOW | 35 |
| M9 | 35 |
| RECORD-ONLY | 84 |
| ALREADY-HAVE | 93 |
| SKIP | 42 |
| REJECTED | 4 |
| Total | 293 |

| Status | Rows |
|---|---|
| candidate | 126 |
| already-have | 163 |
| rejected | 4 |
| Total | 293 |

NOT-READ is the 72 PDFs byte-identical to something already in `docs/incoming`; they were skipped, as asked.

## Top 10 NOW, by value ÷ effort

Value is what the internal checks can't test, on a region the vertical slice never ran. Items 1 to 4 are the atlas
(https://www.threebodyorbits.com) and are all in the committed `threebodyorbits_ics.csv` or the source data. They sit
inside OC-1, OC-3, OC-4 and LL-*, so a peek needs no new target. Closure tolerances stay calibration requirements (R-71).

| # | Item | What it tests | Effort | Watch for |
|---|---|---|---|---|
| 1 | Hristov long-horizon families: `hs_eight`, `hs_schubart`, `hs_moth`, `hs_chor`, `hs_broucke` (662 rows) | Long-horizon closure at equal masses, L=0; nearly all rows stable | S | Measured closure from doubles governs the floor |
| 2 | Li & Liao, Li, Jing & Liao: `li_*` (1,349 unequal-mass rows of 1,957) and `li2021` (7) | Unequal masses, which the slice never ran | S (M for stability) | Vasiljević's 1,317 are stability labels on these rows, not starts; the label field isn't in the committed catalogue |
| 3 | Free fall: Li & Liao's committed authors' table (316) and atlas `freefall` (311) | Free-fall closure, return to rest, nine mass sets | S on the table, M on the atlas | `pub_*` isn't centred; `ref_*` isn't exactly at rest; 81 website-only rows are closure only |
| 4 | Broucke A, R, S (`brouckeA`, `brouckeR`, `brouckeS`, 52 rows) | L≠0 closure in the inertial frame; S adds unequal masses at L≠0 | S | They do **not** test rotating-frame closure |
| 5 | Kapela & Simó 2017, rotating Eights (3 states) | Rotating-frame closure at L≠0, the thing item 4 can't do; slow error growth on stable orbits | S to peek, M to score | Return time and angle aren't the authors'; find them by section crossings |
| 6 | Majumder 2026 Moth13 (CC BY 4.0) | Long horizon, unequal masses, state at 245 known times, with rigorous bounds | S | Mesh is non-uniform; take node times from the file. Unrefereed |
| 7 | Habl 2026 (686 orbits, CC BY 4.0) | Unequal masses, periods the slice never ran, free fall and Euler | S | Unreviewed, AI-assisted; closure needs a high-precision path; stability columns weaker than closure |
| 8 | Janković, Dmitrašinović & Šuvakov 2020 (99 orbits) | L≠0 across a wide range, a whole family at once | S | 12-decimal starts may not reproduce the printed closure. 57 are also in the gallery: usable |
| 9 | Szebehely 1972, mass effects (125 runs) | Free fall at unequal masses from exact starts: who escapes, when, final binary | S to M | Peek at early escapes first; late rows need the precision ladder; compare class, not exact time |
| 10 | Horn et al. 2025 (5,000 runs, CC BY 4.0) | Statistical external check at L≠0, equal masses | S | About 1e-8 as a reference: grade at 1e-7, not ground truth. Short horizon, mild encounters |

Then, in order: 11 Szebehely 1972 velocity effects; 12 Minesaki 2019 elliptic Lagrange case (analytic answer, one row);
13 Saito 2013 two-orbit bracket (the full scan is M9); 14 Anosova & Nebukin 1991; 15 Neagu (filtered, `_local`, advisory);
16 Broucke 1979 isosceles; 17 Liu 2018; 18 Adler 2016 Burrau positions (information only); 19 Moore & Nauenberg 2006
(an ordinary candidate; it has no target only because it is in the catalogue).

## Top 10 M9, by value ÷ effort

| # | Item | What it tests | Effort | Watch for |
|---|---|---|---|---|
| 1 | Hristov tier ladder: `chor462` (147 rows not in the atlas), `hs_nchor`, `hs_family` | Closure above f64 at the longest periods we hold | S to read; needs the tiers | Most rows too unstable to close from doubles; period match, not row identity |
| 2 | Davoust & Broucke 1982 (136 rows) | True rotating-frame closure at L≠0, plus stability coefficients | M | Dot-matrix digits: two careful readings. Check for shared orbits with Hénon 1977 (HE-1) |
| 3 | Hadjidemetriou & Christides 1975 (17 interior rows) | Unequal masses and L≠0 with exact closure, and a fold in the family | M | Text layer spaces its digits: transcribe by eye |
| 4 | SJTU planar unequal-mass tables (not fetched) | Published starts at unequal mass at source, not the atlas's copy | M | Unread: format and precision unknown. See question 3 |
| 5 | Standish 1972 (8 planar sets of 100) | Escape and final-binary statistics against L | M | n≈100 per set; old integrator |
| 6 | Monaghan 1976 I | Planar binary eccentricity distribution, a closed form | S once Q-2's ensemble exists | Needs that ensemble first; don't gate on the whole ensemble |
| 7 | Simó choreographies, low-S rows (`simo`, about 26 of 344) | L≠0 closure from doubles, with the T/3 permutation | M | The rest measure a horizon, not a pass; digits past about 16 are arithmetic |
| 8 | Anosova & Orlov 1992 | Stable fraction at L≠0; long-lived dumb-bell orbit; which mass escapes | M | Velocity set-up is in Anosova 1986; digits in figure labels need a second reading |
| 9 | Zare & Chesley 1998 | Repeated binary collisions: outcome fractions, one reflective orbit's monodromy | M | Needs a regularised occupant, a syzygy event and a collision counter. In the 15c branch as well |
| 10 | Vynatheya 2022, stability on comparable masses | Long-horizon stability classification, planar slices | M | Needs a hierarchical measurement class (question 5). Its scores conflict with Tory 2022 |

The other 22 are ranked in `triage.csv`: Harrington 1975, HopOn figure 2 data, Sofia free-fall and Euler sweep (41,000+
committed rows no target reads, L=0 and equal masses only), Mikkola & Tanikawa 1999, Loughran, Kapela & Simó 2007,
Chenciner et al. 2005, Tory 2022, Hayashi 2022, Manwadkar et al. 2021 and 2020 (3D comparators beside DS-1, reported as a
difference, never pass or fail), Tanikawa 2000, Tanikawa et al. 2019, Simmons 2021, Simó & Martínez 1988, Mel'nikov 2013,
Martynova & Orlov 2014, Monaghan 1976 II, Mushkin & Katz 2020, Saslaw 1974, Figueras & Haro 2025, Antoniadou & Voyatzis
2014.

## Atlas corrections carried

- Vasiljević's 1,317 are stability labels on Li, Jing & Liao rows; they add no starts.
- Hénon 1976 gives one stability label, not a family. No Hénon family or Broucke B is in our copy.
- Broucke A and R close in the inertial frame, so they don't test rotating-frame closure. A true relative-periodic target
  needs the Broucke or Hénon tables, or items 5 (NOW), 2 and 3 (M9) above.
- About 26 of 344 Simó rows close from doubles.
- Measured closure from doubles governs any floor, never the catalogue's ~1e-28, which is the double-double value. Each
  tolerance stays a calibration requirement (R-71). Sample figures are not carried here.
- `atlas-params.json` is never committed, and no value from it is here. Cite the atlas by its link.
- The five site-note families (`r4new`, `rosette`, `bhhsat`, `bhunt`, `pancake`; 81 rows) are in the catalogue: closure
  only, never cited as published results. See question 1.
- The 99 empty-source rows: see question 2.

## Rejected

| Item | Reason |
|---|---|
| dosier, `TBP-Group-39-TrainData` | R-642 item 2: no data, different initial conditions. **Not in the library:** INDEX, skipped-zips list, every text file, file names and zip members searched, no hit. Row added so the rejection is on the list |
| stipefrkovic: `data/Frkovic/raw` (100 CSV), its code zip and provenance | R-642 item 2: non-zero velocities, Verlet. The data path doesn't carry the name. The rest of `ml_three_body` is not this repo |
| Farrés 2017 | Restricted problem |
| Ferreira & Prado 2023 | Restricted problem |

## SKIP list

Rows that are already-have duplicates are `ALREADY-HAVE`, not SKIP. No row is skipped for being "already excluded": each
one that said so was judged again. That changed Janković 2020, Hadjidemetriou & Christides 1975, Liu 2018, Habl 2026 and
Hudomal's thesis.

| Item | Verdict | Reason |
|---|---|---|
| `batches/Downloads_(loose)/ asna.19131950602.html, asna.19131950602-...` | OFF-FILTER | No data. |
| `batches/Downloads_(loose)/lit_review.tex` | CONTEXT | Not a source |
| `batches/codex_2026-10-26_gallery-originals/work (10 JSON, 2 logs, 1...` | OFF-FILTER | Fails the filter on substance: no bodies, no numbers. |
| `batches/codex_2026-10-27_valtonen-book-captures/work/ (92 files)` | OFF-FILTER | Front and back matter of a book already held in full. |
| `batches/codex_2026-10-28_breen-nvm-data/work/NN.h5` | OFF-FILTER | Network weights are no reference. |
| `batches/papers-2/(top) README.md, manifest.json, checksums.sha256` | OFF-FILTER | No data of their own. |
| `batches/papers-4/2026-10-31/code/JonathanMushkin__HopOn__52a4f61b6c...` | OFF-FILTER | Fails the planar filter. |
| `batches/papers-5/2026-10-31/code/ (joang_taylor2_dist zip, taylor2_...` | CONTEXT | No data |
| `batches/planar_three_body_research/data/Quarles_planar_i0_first_par...` | OFF-FILTER | Earth-mass planet (about 3e-6 of the system), finite-radius collisions: the owner's 'finite-radius stars, out of scope'. |
| `from_zips/three_body_papers/SCC-Vol.6.1-04_-Cohler-et_al.pdf` | OFF-FILTER | Off-topic: a stray file in the zip, not research for Principia. |
| `papers/Agol_2021_NbodyGradient.pdf` | CONTEXT | No printed three-body result |
| `papers/Antoniadou_2014_Coorbital.pdf` | OFF-FILTER | Fails the planar filter in its observable |
| `papers/Antoniadou_2016_Eccentric_Orbits.pdf` | PLOT-ONLY | Conference note with one input table |
| `papers/Babbs_2024_Planar_Lifetimes.pdf` | HIT | Non-converged first-order numerics are not a reference: a mismatch would say nothing about Principia. |
| `papers/Barbieri_2018_Nekhoroshev_Estimates.pdf` | CONTEXT | Bounds can't be falsified by a run at any horizon we can reach |
| `papers/Boekholt_2022_TIDYMESS.pdf` | OFF-FILTER | Fails the filter: tidal physics, not point masses |
| `papers/Greydanus_2019_Hamiltonian_Neural_Networks.pdf` | OFF-FILTER | No trajectory numbers to compare |
| `papers/Hietarinta_1993_incomplete_preprint.pdf` | OFF-FILTER | Superseded by the journal copy already held |
| `papers/Jin_2020_SympNets.pdf` | OFF-FILTER | ML scores only |
| `papers/Kumar_2021_Differential_Euler.pdf` | OFF-FILTER | No printed numbers or data |
| `papers/Lalande_2022_Stability.pdf` | OFF-FILTER | Fails the planar filter and prints no comparable numbers. |
| `papers/Merrath_TUM_1764550_Thesis.pdf` | OFF-FILTER | No reference data for a planar three-body comparison |
| `papers/Migaszewski_Gozdziewski_2018_Kepler25.pdf` | CONTEXT | Fit-degenerate, tiny mass ratios, no comparable output. |
| `papers/Musielak_Quarles_2015_Three_Body_Review.pdf` | CONTEXT | Review with no comparable numbers |
| `papers/Myllari_2016_ACA_Abstract.pdf` | CONTEXT | Abstract with figures only, and the 2007 paper covers the method. |
| `papers/Pereira_2025_Physics_Informed_Three_Body.pdf` | CONTEXT | Benchmarks neural networks and offers no usable data, so it tests nothing in Principia |
| `papers/Roa_2016_KS_Hopf.pdf` | CONTEXT | Its novelty is specific to KS, which R-607 does not build |
| `papers/Saz_Ulibarrena_2024_Hybrid_Nbody.pdf` | OFF-FILTER | Fails the filter: planetary hierarchy, 3-D. |
| `papers/Saz_Ulibarrena_2025_RL_Adaptive_Time_Stepping.pdf` | PLOT-ONLY | Nothing checks our physics. |
| `papers/Szebehely_1972_scan_UNIDENTIFIED.pdf` | OFF-FILTER | Failed download. |
| `papers/TUM_1785709_Thesis.pdf` | OFF-FILTER | No comparable numbers |
| `papers/Valtonen_et_al_2016_Pythagoras_to_Hawking_preview_scans.pdf` | OFF-FILTER | Front matter only |
| `papers/Voyatzis_2017_Rectilinear_Eccentric.pdf` | OFF-FILTER | Fails three massive bodies. |
| `papers/Dindar_2013_Swarm_NG.pdf` and fixtures | HIT | Meets the filter by the letter only; GPL; tests nothing beyond internal checks. |
| `papers/Hernandez_Dehnen_2023_Reversible_Switching.pdf` | OFF-FILTER | Restricted problem and planetary systems. |
| `papers/Venkatesh_2021_Coupling_Recoupling.pdf` | CONTEXT | Qualitative note, nothing to compare. |
| `papers/Farhani_Asl_2026_Binary_Formation.pdf` | OFF-FILTER | 3D, no planar subset. |
| `papers/Farhani_Asl_2026_Binary_Single_ML.pdf` | OFF-FILTER | 3D. |
| `papers/Portegies_Zwart_et_al_2022_Chaos_Manybody_*.pdf` | CONTEXT | Not planar three-body. |
| `papers/Wang_Hernandez_Zheng_Huang_2026_P3T_CONTEXT.pdf` | OFF-FILTER | Clusters, not three bodies. |
| `papers/Heggie_1996_Statistics_Small_N.pdf` | CONTEXT | Clusters, method only. |
| Batch apparatus, papers-7 to papers-10 | CONTEXT | Notes and retrieval evidence; packaging of the rows above. |

## Conflicts between and inside sources

Values are in the sources named; the `conflict` column in `triage.csv` marks every row.

- **Horn:** Zenodo says Brutus at arbitrary precision; three readers measured about 1e-8. Grade at 1e-7.
- **Moth13:** manuscript and data guide say a uniform mesh; it is non-uniform.
- **Loughran:** status conflict: one batch note calls it unreviewed reproductions, another HIT. Its stable window also
  disagrees with its own trace. Not rejected.
- **Neagu:** readers split M9 and NOW; applied NOW, with a filter. Stays `_local`.
- **Janković:** the bundle's "42 new" is against the wrong catalogue; our audit gives another split.
- **Kapela & Simó:** `2026-10-28_external_sources/README.md` lines 102-104 attach the two preprints and `choreo.zip` to the
  wrong years, against their titles.
- **REQ-VAL-041:** its period and energy pair differs slightly from the scale-free value of both Kapela & Simó papers.
- **Hierarchical stability:** Harrington and Hayashi disagree on inclination; Donnison, Harrington and Black give three
  equal-mass critical values; Hayashi's coefficient differs from Mardling & Aarseth's; Tory and Vynatheya score Mardling &
  Aarseth very differently (different samples and definitions); Saito's fit misses its own printed points at the low end.
- **Lifetimes:** Anosova & Nebukin's class means differ between authors by more than the errors; Standish and Szebehely
  differ on escape fractions on different ensembles; Anosova & Orlov's dumb-bell probability differs from Benest's.
- **Inside single papers:** Broucke 1979 (half periods, table against table; energy normalisation); Minesaki 2019 (text
  against printed start); Simmons 2021 (abstract against theorem; a likely misprint); Simó & Martínez 1988 (two suspect
  rows); Zare & Chesley 1998 (a rotation number against its trace); Mel'nikov 2013 (period in the text against its own
  equations); Manwadkar 2020 and 2021 (table labels, one likely dropped digit); Szebehely 1972 (text against table, twice).
- **Tanikawa 2000** time unit and **Martynova & Orlov 2014** count: rows cite RQ-640 and RQ-642. Those numbers are named in
  `2026-10-29_research_batches/README.md` but aren't in `REVIEW_QUEUE.md`.
- **Library README** files the Belgrade gallery under threebodyorbits.com. The gallery is three-body.ipb.ac.rs, a separate
  source.
- **Free-fall website rows:** PLAN 2.1 says the 82 website-only orbits have words but no initial conditions. The authors'
  committed table holds their starts, so PLAN 2.1 is out of date on this.

## Addendum, 1 Nov 2026

Rows 265 to 294 of `triage.csv` (30 rows) come from two later reads: the papers-7 to papers-10 batches, and the owner's
precision set below. Ranks NOW-20, NOW-21 and M9-33 are added after the existing ones; the first pass's ranks and the Top
10 tables above are not re-ranked against them.

- **NOW-20, Li, Tao, Li & Liao 2025 (`_local`).** Table 1 prints T and theta for 16 orbits at L≠0, equal masses. It is the
  only source here that scores rotating-frame closure with the authors' own return time and angle (question 4). Our sample
  run closes all 18 rows only with the printed rotation. One rights-reserved table, one reading, a sign misprint in the
  caption and an off-by-one L label: see the row.
- **NOW-21, three-body-search (MIT).** 143 equal-mass L≠0 orbits with full starts, closing in the inertial frame. Unreviewed
  and AI-produced, so a peek only, as question 6 asks of Habl and Majumder.
- **M9-33, the BPZ cold records.** See below.
- **Planetary and hierarchical sets:** Tamayo 2021, Quarles 2018, Georgakarakos 2024, Iwakura 2025 and others are
  RECORD-ONLY, on question 5. Iwakura has three comparable masses and would outrank the planetary sets if that opens.
  Georgakarakos's ip=180, ep>0 records equal its ip=0 ones to every printed digit: don't use them until that's explained.
- **Onishi 2026:** read for scope. Paper and held source disagree on whether the runs are planar; not comparable as it stands.

## Precision against statistics

The owner's set: does any paper test f32 ensemble statistics, and do the thresholds quoted from Boekholt & Portegies Zwart
2015 (BPZ) hold? Read from the PDFs; nothing in the library was run. "Ours" marks our own reading or computation.

- **No paper in this set tests f32 ensemble statistics.** The gap stands for this set, worded as "not found in these
  records": it is not an exhaustive search. The nearest sentence is the abstract of Portegies Zwart 2026, that fp32 "could
  be used for statistical ensemble calculations". That paper runs no ensemble, and its § 4.2.1 says no 32-bit Pythagorean
  solution is reliable enough for any scientific computation. Its 32-bit evidence is single runs on CPU, one code (Table 4:
  fp32 |dE/E0| 1.3e-4 on the figure-eight, 0.57 on Pythagorean, at eta = 1e-4).
- **BPZ is f64 against converged, N = 3.** Its "unbiased" claim holds only after runs with |dE/E| > 0.1 are removed, and the
  number removed is not stated ("a few"). So the claim doesn't fit never-discard (R-563) as it stands; Fig. 14, showing the
  ensemble cut and uncut, fits as a method if the count is reported. Rows whose run hit the CPU limit are kept with zeroed
  coordinates, t, e and Lw, so they lose their end time and carry no flag.
- **The pair "eta <= 2^-5 and |dE/E| < 0.1" merges two results.** It describes the error distribution of the inaccurate runs
  (abstract, § 7). The KS result is strict, eta < 2^-5 (Fig. 6, § 6.3), and cold a_bin already differs above 2^-6 (§ 5.2).
  Portegies Zwart & Boekholt 2014 give another step limit, [value], on the same starts, and state the energy bound as a
  conjecture. Ours: cite 2015 for the limit and say the value moved.
- **The cold records are usable, on the converged side only.** MOESM3 and MOESM4 hold Brutus runs, 10,000 each, CC BY 4.0 per
  figshare. Hermite results are not published, so Fig. 6's KS tests can't be recomputed. Ours, with `python3 -I` on the
  records: largest out-of-plane position 4.3e-9 and 3.4e-9, so planar up to a rotation; largest final |dE/E| 6.7e-5 and
  1.6e-4; undissolved at the stored t = 501, 14.6% and 8.7% (paper ~10% for cold 1:2:4); half-lives 41 and 24 crossings (2014
  caption [value] and [value]); all-zero rows 98 and 16. Treating the zero rows as undissolved is our assumption. t = 501
  means stopped, not undissolved at that time. A recomputation of Table 1's binding-energy indices was inconclusive.
- **Energy error is not a certificate.** Rein, Brown & Kanda 2025 (Solar System, not three bodies) show KS passing where an
  instability rate already differs, and say relative energy error is only a necessary condition. They keep unstable runs in
  their distributions, the best model here for never-discard.
- **Punctuated Chaos 2023** calls statistical validity "an article of faith" and reliability "a fundamental unsolved
  problem", citing BPZ: a limit on how strongly BPZ is quoted. **Portegies Zwart & Boekholt 2023** adds irreversibility fits
  and one clause on statistics.

| Paper | Precisions compared | N; geometry | Ensemble | f32 | Large-error runs | Never-discard |
|---|---|---|---|---|---|---|
| BPZ 2015 | f64 vs converged | 3; warm 3D, cold planar | 4×10⁴ | none | shown both ways; claim needs the cut | method fits; claim doesn't; zeroed rows conflict |
| BPZ records | converged only | 3; planar after rotation | 2×10⁴ | none | none above 1e-3 | rows kept; 114 lack an end time |
| PZB 2014 | f64 vs converged | 3; as above | 4×10⁴ | none | not stated | silent; claim conditional |
| PZ 2026 | fp16 to fp128, posits vs converged | 3, 100; planar and 3D | none | single runs only | tabulated | fits for 32-bit |
| BPZH 2023 | none | model | none | none | silent | silent |
| PZB 2023 | tolerance scan | 3 and up | size not given, irreversibility only | none | silent | silent |
| Rein 2025 | step scan, f64 | 9; 3D | ~2,000 | none | kept, reported | fits |

**Claims that can't be drawn:** that f32 ensemble statistics match f64 or the true statistics, on any start set; "fp32 is
fine for ensembles (Portegies Zwart 2026)"; any |dE/E| value as a rule for Principia; eta <= 2^-5 for a non-Hermite
integrator or f32; statistics preserved past the horizon or at L≠0 in the plane; that f32 on a GPU behaves like these CPU
runs. Three different "0.1"s shouldn't be read across: BPZ's energy cut, the phase-space distance delta, and Principia's
parted mark, a shape-sphere chord (R-574).

**What a Principia f32 against f64 comparison would need (proposals, ours).** Per start and per precision: stop time and
status (ended, censored, failed), outcome class, escaper, binary a and e, final energy error. Every statistic on the full
ensemble first, failed runs as a class; any energy cut shown beside it with the count and which runs. Paired tests alongside
two-sample KS, because starts are shared and KS can miss a real effect (Fig. 14, Rein); an equivalence margin, not
"p > 0.05"; a correction for many statistics; lifetimes as censored data. Margin and sample size are calibration
requirements (R-71). For scale, by the standard formula: two-sample KS at 5% has a critical distance near 0.019 at n = 10⁴
and 0.12 at n = 256, so REQ-VAL-199's 256 starts couldn't see outcome differences of a few percent. References: the cold
records, which we already have in the library, truncated at T; Principia's own f64 on the same starts, with a higher tier
on a subset. A planar L≠0 statistical reference wasn't found, so f32 at L≠0 would be a self-comparison and must say so. A
paired f32, f64, converged comparison with all runs kept isn't in these records, whatever its outcome.

**Repo statements that overstate these sources, reported and not edited:**

1. PLAN BZ-1 (`docs/incoming/2026-10-27_validation_plan/PLAN.md:345`): "Its method is what REQ-VAL-199 and "never discard a
   copy" already apply." BPZ removes runs for its headline claim; REQ-VAL-199 keeps all 256 starts and measures another
   quantity.
2. PLAN BZ-1: "out of scope as a replication: 3D Plummer ICs" and "figshare (not fetched)". The cold sets are planar and two
   of the four files are held. What stands: no published Hermite side, and t ≤ 500 against R-610's cap of 200 (units to convert).
3. `docs/incoming/2026-10-25_replication/catalogue.md` § 6.1 merges the error-distribution claim with the KS claim, calls BPZ
   a direct analogue of never-discard, and says the starts are 3D, which holds for the warm sets only.
4. `docs/incoming/2026-10-25_replication/INSTRUCTION_replication_suite_2026-10-25.md:89`: "the empirical basis of the f32/f64
   honesty story". The paper has no f32. That evidence has to come from elsewhere; ours: Principia's own measurement.

**Not held:** the BPZ warm supplements (additional files 1 and 2); the Hermite results; the Zenodo data of Portegies Zwart
2026, which the paper names with no identifier.

## Licence check: `agekyan_anosova_1968_Ap_4_11_part2.pdf`

On RQ-610's list, first item: `docs/incoming/2026-10-24_anosova/papers/agekyan_anosova_1968_Ap_4_11_part2.pdf`, sha256
prefix `7d160daf` matches, tracked, no open licence seen. It stays until ruled, as the owner said on 28 Oct. The library's
`from_zips/three_body_papers/BF01013153.pdf` is byte-identical. `agekyan_anosova_1968_SvA_11_1006.pdf` and the Anosova
1969 translation, also in this triage's already-have rows, are on RQ-610 too.

## Open questions for the owner

1. **Does a site note count as "published"?** The five families (`r4new`, `rosette`, `bhhsat`, `bhunt`, `pancake`; 81 rows)
   are treated as closure only. `pancake` is marked draft. `suvakov_pc` (13 rows, personal communication) has the same
   problem.
2. **The 99 empty-source high-precision rows.** Both facts stand: they are closure only until traced, by your instruction;
   and `2026-10-27_hp_ics/README.md` infers 97 are `hristov2026` by digit match and 2 Li & Liao. Does the digit match count
   as tracing?
3. **SJTU tables.** Does R-631 reach the unfetched planar unequal-mass tables? The GitHub repository has no LICENSE file.
4. **Rotating-frame closure.** No atlas family tests it. Is Kapela & Simó 2017 (NOW) plus Davoust & Broucke 1982 (M9)
   enough, or fetch the Broucke 1975 and Hénon 1976 tables? You said those PDFs are for citation only.
5. **Hierarchical triples.** Vynatheya, Harrington, Tory, Hayashi, HopOn, Mushkin, Antoniadou and Figueras need a
   BinaryPlusSingle measurement that isn't planned (RQ-593). Are they in scope? If not, M9 items 10 onward shrink.
6. **Unreviewed sources.** May Habl 2026 and Majumder 2026 (both AI-assisted, unrefereed) become targets, or peeks only?
7. **Li & Liao free-fall table.** It could replace hand transcription for LL-1, which needs a ruling.
8. **Horn in the repo.** Commit the paper plus the 9.7 MB training file, or a sample? The 930 MB file stays out either way.
9. **Li, Tao, Li & Liao 2025.** Table 1 is rights reserved, and there is no data file. Is transcribing it for approved use
   acceptable, and should the authors be asked for their long-decimal data? Does question 4's answer change with it?
10. **BPZ cold records.** Commit both files (CC BY 4.0, 11.6 and 10.6 MB), or a sample? Is BZ-1 reopened for the cold sets?

**For the queue, once you say so; this PR files nothing:** PLAN row BR-1 still says the data isn't available, stale against
R-641. REQ-VAL-319 and design 5.12 say Manwadkar's Table 1 where the probabilities are in Table 2. PLAN's 437 L≠0 rows list
families summing to 435; the other 2 are `rose` rows with refinement drift. RQ-640 and RQ-642 as above. PLAN 2.1 as above.
Not held and not found: 3body821 (its L unread), any Hristov data at L≠0, a Manwadkar 2024. Also, from the addendum: the four
statements under "Repo statements that overstate these sources".

## Decisions recorded (applied per R-369)

- Merged Horn (4 rows), Moth13 (3), Neagu (2). Moved the small `li_*` families from M9 to NOW with their group.
- Added rows: 72 byte-identical already-have PDFs, unread; Moore & Nauenberg 2006 (spot-read by this pass, since no chunk
  read it); Farrés, Ferreira & Prado and dosier.
- Masked numbers taken from `_local` sources as `[value]`, on the terms under "Words used", and shortened the atlas
  licence note to the terms.
- Cohler is off-topic. `bhunt` is a catalogue family name.
- Addendum: appended the rows after row 264 so that existing row numbers stay; added ranks after the existing ones without
  re-ranking. Folded two duplicates into their rows: the published Tory 2022 into M9-18, and the further Quarles 2020 files
  into row 236. Set `licence_action` to `_local` on Farhani Asl's Binary Formation row, whose held PDF is arXiv non-exclusive;
  the deposit is unread and not held. Masked values from unlicensed or `_local` sources where a `commit` row quoted them: the
  2014 half-lives in the records row, Onishi's repository values, and a stop distance in the Quarles 2018 row, whose paper stays out.

Removed lines in the first-pass commit (735cde24): none. Each later commit carries its own note.

## Papers-14 to 80, 1 Nov 2026

**Proposals only.** Nothing here becomes a target until the owner rules. No requirement or plan changes. Source:
`~/Downloads/principia-research-library`, final at 374 unique PDFs by the owner's count, including papers-80, the last
batch. One row per item in `triage-14-80.csv`, in the columns of `triage.csv` plus `group`, the reading pass that wrote
the row. A row number is the line in that file. `batch-notes-14-79.md` holds each batch's own claims, all marked
unverified: they are ChatGPT's notes, and only what a row says it checked was checked.

As the owner asked, reading went first to the four gaps, then peer-reviewed planar hits. 2026 unreviewed preprints,
hierarchical, circumbinary and context items are one line each, most from the first page only. Unreviewed or AI-assisted
sources are peeks only. Hierarchical triples are parked with the v2+ 3D extension.

### Counts

Rows, not papers: `PENN.pdf`, `3BodyProblem.pdf` and the Brasser 2023 PDF each appear in two rows (the Brasser row 29
says to dedupe). The PENN rows (92, 146) and the Brasser rows (29, 104) agree; `3BodyProblem.pdf` differs on purpose, as
row 61 is its escape file (NOW, a peek) and row 96 the essay itself (SKIP).

| Verdict | NOW | M9 | RECORD-ONLY | SKIP | Rows |
|---|---|---|---|---|---|
| HIT | 4 | 7 | 1 | 0 | 12 |
| PLOT-ONLY | 0 | 0 | 5 | 0 | 5 |
| CONTEXT | 0 | 0 | 28 | 34 | 62 |
| PEEK-ONLY | 2 | 0 | 17 | 16 | 35 |
| PARKED | 0 | 0 | 20 | 34 | 54 |
| OFF-FILTER | 0 | 0 | 0 | 5 | 5 |
| Total | 6 | 7 | 71 | 89 | 173 |

The 7 M9 rows are all read hits. The 12 unread ESCALATE rows below are PARKED and RECORD-ONLY under the owner's
hierarchical ruling, which shrinks the M9 list. Pinto 2026 (row 147), which the readers also marked M9, was then read in
full: PLOT-ONLY and RECORD-ONLY, because it prints no outcome numbers and names no data file. Of the 6 NOW rows,
Szebehely 1972 (row 45) is the existing NOW-9, unchanged; Kumar 2014 (row 46) is only a cross-check on Chandra & Bhatnagar;
the essay's escape file (row 61) is unreviewed, a peek, with no licence, so its values stay masked here.

### The four gaps

No gap is closed.

- **Per-point Lyapunov or FTLE file: none found.** The nearest are a per-system table of planetary MEGNO and Lyapunov
  times (row 5, parked), a 31x31 MEGNO grid for one star and two planets (row 6, parked, unreviewed), a MEGNO catalogue (row 7, parked) and
  six running-exponent trajectories (row 8, already have). The Sofia lead below would be per orbit, not a map.
- **Planar final-binary distribution as numbers: none found.** What is printed is per case or a deterministic family: Chandra
  & Bhatnagar Table V and Alexander Table II (below), and Szebehely 1972 Table I (row 45, already NOW-9). Valtonen 1976
  prints binned counts for 3D sets only; its planar data are figures (row 49). Kumar 2016's "distributions" are theory
  formulas evaluated per orbit (row 47). Row 61's 200 runs have no a or e. Pinto 2026
  plots a stability index of the binary eccentricity (Fig. 6b, p.9), with no numbers (row 147).
- **f32 statistics at N=3: no test found.** The nearest is Smith 1977, A&A 61, 305-312 (row 2): 5 examples per level, N=16,
  3D, the same starts at d=17, 7, 6 and 5 significant figures, with t tests on ensemble statistics (Tables 1 to 4, pp.307-308).
  It finds no significant distortion at d=6 and d=7, and blow-up at d=5, on chopped IBM 360 arithmetic rather than IEEE
  f32. It is a design precedent, not a target. Pruett 2003 p.307-308 (row 39, parked) has true single precision on a planar
  three-body case, but one trajectory; Brasser 2023 (row 29) has FP32 against FP64 statistics on 80 terrestrial-planet runs.
- **L≠0 and unequal-mass digits: new, at small masses or low precision.** Bengochea 2011 and 2013 give peer-reviewed L≠0 digits,
  2013 with unequal masses in 3 rows, but all with two small masses; Hénon 1974 (Celest. Mech. 10, 375-388; approved by the owner's "14 yes", 1 Nov, ranked
  NOW-22 in the papers-11 to 13 triage, which isn't on `main`, so not yet in `triage.csv`) and Davoust remain the
  comparable-mass cases. Tsouroplis & Zagouras span more masses at about 1e-3. Chandra & Bhatnagar add a nonzero-L equal-mass
  family. Broucke & Boggs Table I has two closing rows, 1 and 40, that the atlas lacks (row 65).

### New top items

Checks are ours, in f64, unless the row says otherwise; each row has the detail and the misprints found.

| Item | Where | What | Check | Row | Proposal |
|---|---|---|---|---|---|
| Bengochea 2011 | Table 2, p.404 | 6 horseshoe orbits, L≠0, 15 significant digits | all 6 close | 62 | NOW |
| Bengochea 2013 | Table 1, p.412 | 6 rows, 5 new orbits, rotation closure | all 6 close with rotation +2 theta_c | 63 | NOW |
| Alexander 1986 | Table II, p.207 | 2 planar equal-mass L=0 starts; final binary a, e, agreed among methods | T1 agrees in a; e differs in the third figure; T2 unfinished | 41 | NOW |
| Chandra & Bhatnagar 2000 I | Table V, p.391 | final a, e, v, theta; 187 members; L≠0; ICs on p.374 | 9 members finished, about 3 figures at best; two misprints | 40 | M9 |
| Tsouroplis & Zagouras 1984 | Tables I-VII, pp.84-92 | 96 asymmetric orbits, unequal masses, 5-6 decimals | 73 of 96 reach reduced closure below 2e-2; a closure at about 1e-3 | 64 | M9 |
| Valtonen 1975 | Table II, p.80 | escape frequency per body by mass ratio, within 1 degree of planar, n=120 each | by eye, not transcribed; not run | 42 | M9 |
| Valtonen 1974 | Table V, p.220 | lifetime quartiles, 3 disk sets, n=118-139 | not run; not reproducible from this paper alone | 43 | M9 |

### The Sofia lead

The likeliest route to per-orbit exponents is three files on the Sofia databases, `res_exponents.txt`, `res_eigen.txt` and
`eigenvalues_12431.txt`, listed under "Not fetched" in `docs/incoming/2026-10-27_hp_ics_freefall/README.md` (row 3). **This is
inferred.** We have not read them, so their contents, row count and units are unknown. The inference: a repository file in
papers-70 carries a "published" exponent on 518 free-fall rows that join `sol_80.txt` on T* (518 of 518, sampling every 24th
distinct solution); that the source is `res_exponents.txt` is our guess. If it holds, it would compare a per-orbit exponent point by point on
starts already committed. It needs a monodromy or variational measurement on our side, and it isn't an FTLE map.

### Not read

- **12 ESCALATE rows, first page only:** De Cesare & Capuzzo-Dolcetta 2021 (row 111), Donnison & Mikulskis 1994, 1995 and 1996
  (114 to 116; the first two are scans, with an empty text layer), Georgakarakos 2013 (124), Kiseleva et al. 1994, two papers
  (132, 133), McDonald 1986 (140), Trifonov 2018, two copies (158, 159),
  Walker 1980 (160), Walker & Roy 1983 (161). What these rows say they hold is the batches' claim.
- **Scanned or otherwise not read:** Gander & Hrebicek (row 121), `hd127195_stability.pdf` (172), the NAOJ 2003 and 2004
  volumes (143, 144).
- **Archives:** the zip and tar archives under `batches/` (142 by listing, 123 of them zip) were not opened for this triage.
  Files a row names as header-only, or not opened, are marked in the row.

### Decisions recorded (applied per R-369)

- `EXCLUDE` in the readers' brief is `OFF-FILTER` here, as in `triage.csv`, on the terms under "Words used". One overlap cell
  reworded the same way (row 4).
- Values from published papers stay in the rows, with citation, under the owner's data-reuse ruling of 1 Nov 2026 (items 3
  and 9); that is later than the `[value]` masking above. Measured values and start states from repository files that are
  not a paper, and have no licence for their data, are masked as `[value]` (rows 3, 4, 8, 9, 61, 99). File sizes, row
  counts, field names, a sampler's set-up and statistics we computed ourselves stay. The essay behind row 61 has no licence
  (`repository_metadata.json`: license null) and isn't established as a paper, so the ruling isn't applied to it.
- The 12 PARKED ESCALATE rows, which the readers marked M9, are RECORD-ONLY under the owner's parking ruling of 1 Nov 2026
  (item 5). No other PARKED row was marked M9 or NOW. That ruling doesn't reach Pinto 2026 (row 147), which is not
  hierarchical. It is a peer-reviewed planar paper (received, revised and accepted dates printed, p.1), so it was read in
  full; it has no table and no data file, so it stays PLOT-ONLY and RECORD-ONLY on the scope filter.
- Hierarchical rows, star-plus-planets ones included, are PARKED whatever else they hold (rows 6, 7, 13, 14, 18, 39, 72, 81,
  90, 103, 141, 171). Every row was searched in all its columns for `hierarchical|triple|circumbinary|Kozai|exomoon|S-type|P-type`;
  the rows that match and aren't PARKED are these. Papers on a method for hierarchical triples stay CONTEXT (162, 163). The
  word is about something else on the rest: an equal-mass triple collision or encounter (40, 41), the editor named Kozai (43),
  triples of mass values (45), and rows that say they are not hierarchical (61, 147).
- A row whose status says unreviewed is PEEK-ONLY, not CONTEXT, ML papers included (rows 134, 151, 166), or PARKED if hierarchical.
- `licence_action` follows what was verified on the held copy. No terms verified there is `_local until checked`: none seen,
  not read, a batch's claim, a journal's usual terms, or a copyright line beside a licence not read. Non-redistributable terms
  verified there are `_local`, with or without the words `(fails CC BY/BY-SA/CC0/PD)`. A CC-style or public-domain licence
  verified there is committable, however the cell words it. A row of several copies takes the least settled, unless its cell
  names each copy (rows 73, 91, 92). The one row outside this is row 3: nothing is held, and its action waits on the owner.
- `_local` in `licence_action` is where a file would go if wanted. This PR holds no PDF, and adds nothing under `_local`.
