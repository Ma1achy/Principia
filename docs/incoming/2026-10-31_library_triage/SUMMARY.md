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
| HIT | 103 |
| PLOT-ONLY | 17 |
| CONTEXT | 45 |
| OFF-FILTER | 22 |
| NOT-READ | 72 |
| REJECTED | 4 |
| Total | 263 |

| Recommendation | Rows |
|---|---|
| NOW | 33 |
| M9 | 34 |
| RECORD-ONLY | 66 |
| ALREADY-HAVE | 93 |
| SKIP | 33 |
| REJECTED | 4 |
| Total | 263 |

| Status | Rows |
|---|---|
| candidate | 96 |
| already-have | 163 |
| rejected | 4 |
| Total | 263 |

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

**For the queue, once you say so; this PR files nothing:** PLAN row BR-1 still says the data isn't available, stale against
R-641. REQ-VAL-319 and design 5.12 say Manwadkar's Table 1 where the probabilities are in Table 2. PLAN's 437 L≠0 rows list
families summing to 435; the other 2 are `rose` rows with refinement drift. RQ-640 and RQ-642 as above. PLAN 2.1 as above.
Not held and not found: 3body821 (its L unread), any Hristov data at L≠0, a Manwadkar 2024.

## Decisions recorded (applied per R-369)

- Merged Horn (4 rows), Moth13 (3), Neagu (2). Moved the small `li_*` families from M9 to NOW with their group.
- Added rows: 72 byte-identical already-have PDFs, unread; Moore & Nauenberg 2006 (spot-read by this pass, since no chunk
  read it); Farrés, Ferreira & Prado and dosier.
- Masked numbers taken from `_local` sources as `[value]`, on the terms under "Words used", and shortened the atlas
  licence note to the terms.
- Cohler is off-topic. `bhunt` is a catalogue family name.

Removed lines: none.
