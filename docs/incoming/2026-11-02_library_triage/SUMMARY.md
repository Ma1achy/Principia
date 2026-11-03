# Library triage 81 to 153, 2 Nov 2026

Task 15d, items 3 to 5, for library batches papers-81 to papers-153, with ChatGPT's benchmark shortlist (papers-128) and
its final ranked guide (`CHATGPT_BENCHMARK_TRIAGE.md`, through pass 153) checked against the files. Source:
`~/Downloads/principia-research-library`, read by chunk. One row per real item in `triage-81-153.csv`, in the merged
header of `docs/incoming/2026-10-31_library_triage/` plus `group`, the reading pass that wrote the row. A row number is
the line in that file. A row the earlier triage already holds says "duplicates <file:row>" and carries the earlier
verdict unless it corrects it (see "Corrections to the earlier triage"). `gaps-precision.csv` holds the precision gaps.

**Proposals only.** Nothing here is a target until the owner rules. No requirement or plan changes. Tables are
transcribed later, twice, for approved items only. No PDF and no library file is committed; values from repositories
with no licence for their data are masked as `[value]`.

Words are as in `docs/incoming/2026-10-31_library_triage/SUMMARY.md` § "Words used" and its papers-14 to 80 section:
OFF-FILTER for the readers' EXCLUDE; PEEK-ONLY for unreviewed or AI-assisted sources; PARKED for hierarchical and
planetary systems, parked with the v2+ 3D extension (R-668).

Rulings cited, not re-proposed: R-661 (the precision set, BZ-1C), R-668 (hierarchical triples parked; site notes;
G = 1), R-673, and R-669 (Hénon 1974; Heggie & Sweatman 1991).

## The papers-11 to 13 addendum

`docs/incoming/2026-10-31_library_triage/triage-11-13.csv` is the papers-11 to 13 triage (25 rows, ranks NOW-22 to
NOW-27 and M9-34), committed with this PR because R-669 cites its ranks. Its rows were numbered as `triage.csv` rows 295
to 319, so "the addendum's row N" in `triage-81-153.csv` is line N − 293 of `triage-11-13.csv`. Each row here that
duplicates an addendum row cites both, as "row N (docs/incoming/2026-10-31_library_triage/triage-11-13.csv:N − 293)".

## Counts

221 rows, merged from 340 reading rows.

| Verdict | NOW | M9 | RECORD-ONLY | ALREADY-HAVE | SKIP | Rows |
|---|---|---|---|---|---|---|
| HIT | 7 | 16 | 4 | 0 | 1 | 28 |
| PLOT-ONLY | 0 | 0 | 8 | 0 | 0 | 8 |
| CONTEXT | 0 | 0 | 45 | 1 | 18 | 64 |
| PEEK-ONLY | 9 | 3 | 11 | 0 | 2 | 25 |
| PARKED | 0 | 0 | 9 | 0 | 55 | 64 |
| OFF-FILTER | 0 | 0 | 1 | 0 | 31 | 32 |
| Total | 16 | 19 | 78 | 1 | 107 | 221 |

| Status | Rows |
|---|---|
| candidate | 146 |
| duplicate | 71 |
| already-have | 4 |
| Total | 221 |

Of the 16 NOW rows, 10 carry standing ranks from earlier triage (NOW-9, NOW-10, NOW-20, NOW-22 to NOW-25, Bengochea
2011 and 2013); of the 19 M9 rows, 7 do (M9-33, M9-34 and the papers-14 to 80 M9 rows). New ranks run on after the
existing ones (NOW-28 to NOW-34, M9-35 to M9-46); the earlier ranks are not re-ranked against them.

## Top 10 NOW, by value ÷ effort

| # | Rank | Item | What it tests | Effort | Watch for | Row |
|---|---|---|---|---|---|---|
| 1 | NOW-28 | eucliss free-fall fate plane (papers-152), 400 × 240, t ≤ 200 | Free-fall escapes at equal masses: escaper fractions, escape-time histogram, survivors; the mirror-pair floor (gap 5) | S | AI-authored, unlicensed: peek, `_local`. Copy its escape predicate. Mirror-pair escaper agreement 0.97 (t < 10), 0.84 (10–20), 0.63 (20–50), 0.51 (> 50) | 204 |
| 2 | NOW-29 | cft-rebound Burrau divergence (papers-72) | f64 against fp128 and fp256 on Burrau to t = 73.4; an 8-member ulp ensemble | S | GPL-3.0-or-later: never committed. Unreviewed, AI-assisted. Raised from RECORD-ONLY | 221 |
| 3 | NOW-25 (standing) | Maissouradze 2026, 7 stable free-fall orbits | Free-fall closure at unequal masses, a 7.7e-5 passage | S | AI-assisted; data path is papers-12, not papers-128 | 90 |
| 4 | NOW-24 (standing) | Kandel 2026, 86 refined starts | L≠0 closure, equal masses | S | Ours: 7 are atlas rows and 1 nearly; peek on the other 78 | 190 |
| 5 | NOW-30 | Agekyan, Anosova & Orlov 1983, Tables 2-4 | Decay-time and escape statistics at rest in region D, 1000 runs, to 1000 crossing times. Peer-reviewed | M | Table 5 needs Tevzadze 1962's escape criterion (not held): its M9 part. Table 4 bin misprint | 4 |
| 6 | NOW-31 | eucliss unequal-mass velocity planes, m3 = 0.25 and 0.5 | Escape-by-60 fraction at unequal masses, L = 0 | S-M | `fate = 0` lumps survivors, step cap and skipped E ≥ 0 pixels. AI-authored | 205 |
| 7 | NOW-23 (standing) | Jeomseon 2026, Burrau interval boxes | Top tiers on Burrau, t = 59.5 to 61 | S-M | AI-assisted; parse as rationals | 78 |
| 8 | NOW-32 | mikha08-rgb/Nbody, figure-eight test | f32 on a GPU against f64 on the CPU, split growth over 40 periods | S | No licence: values masked. Conflates device and precision; needs KDK at fixed step | 188 |
| 9 | NOW-33 | Pickett 2026, close-approach start | One 3:4:5 start with a 0.0118 pair passage, three pair distances at t = 16.77 | S | AI-assisted; the author's own float check, not interval. Raised from RECORD-ONLY | 41 |
| 10 | NOW-34 | eljja threebody-engine certificate | FTLE convention at one start (norm, horizon, definition) | S | One period of a stable orbit; low physics value | 210 |

Unchanged and not re-ranked: NOW-22 Hénon 1974 (ruled), NOW-20 Li, Tao, Li & Liao 2025, NOW-9 Szebehely 1972, NOW-10 Horn,
Bengochea 2011 and 2013.

## Top 10 M9, by value ÷ effort

| # | Rank | Item | What it tests | Effort | Watch for | Row |
|---|---|---|---|---|---|---|
| 1 | M9-35 | Heggie 2000, MNRAS 318, L61 | Survival of figure-eight-like syzygy sequences in a random L = 0 virial ensemble of ~10⁴ at t = 10, 100, 1000 | M | One-figure numbers; units from Heggie & Mathieu 1986; a conflicting arXiv id in the batch | 66 |
| 2 | M9-36 | Anosova, Bertov & Orlov 1984, Astrofizika 20, 327 | L≠0 final-binary a and e distributions, survivors and lifetimes against L; 6000 runs to 1000 τ | L | The velocity recipe (Anosova 1969, Trudy AO LGU 26, 88) is not held: a definition requirement (R-72) if approved | 7 |
| 3 | M9-37 | Pavluchenko 2011 | Two-power-law lifetime tail (b1, b2 with errors) over free-fall region D to T = 5000 | M | arXiv only: a peek unless a refereed version is found; grid and time unit not printed | 114 |
| 4 | M9-38 | Wulff & Schebesch 2008, Wulff & Schilder 2009 | Rotating Eights near collision at J ≈ -6.6: period doubling, a fold, Floquet multipliers | M | 5-digit states need a rotating-frame corrector; ω's sign differs between the papers | 164, 165 |
| 5 | M9-39 | Galán et al. 2002, PRL 88, 241101 | The figure-eight's multipliers (7 digits) and action; a second source beside Kapela & Simó 2007 | M | Needs variational equations; the mass window is figure-only | 46 |
| 6 | M9-40 | Bogomolov, Pavluchenko & Toporensky 2011 | Lifetime-tail slopes for six mass sets, free fall, T = 7000 | M-L | arXiv only; n and d missing; fit window and a sign typo | 19 |
| 7 | M9-41 | Harrington 1970, AJ 75, 1140 | Outcome fractions of planar equal-mass parabolic binary-single scattering; capture limits direct and retrograde | M | Waits on a binary-plus-single chart, like Heggie & Sweatman (R-669); "parabolic" class undefined; 220 runs | 63 |
| 8 | M9-42 | Orlov, Petrova, Rubinov & Martynova 2004 | Metastable fraction of region D, about 3% of 300,000 | M | Rounded; the ejection radius is a definition requirement | 107 |
| 9 | M9-43 | Doumbouya 2026, BKM reproduction | E = 0, L = 0 escape and final-binary statistics, 4,933 seeds | L | Peek only: failed its own gates, AI-assisted ("Grok audit"). Was NOW-27 | 189 |
| 10 | M9-44 | Tian & Zhou 2003, Chin. Astron. Astrophys. 27, 209 | L≠0 equal-mass lifetimes by homology-map region | L | Table 1 fails its own arithmetic; sampling and root choice undefined | 148 |

Then: M9-45 Puodzius 2026 (peek; multipliers on held Belgrade orbits, raised from RECORD-ONLY), M9-46 ThreeBodyFractal
Image02 (peek; finite-radius stop in SI units, a new stop predicate needed), and Agekyan 1983's Table 5 (row 4).

## SKIP list

107 rows. Each row's `reason` column has the detail.

- **Planetary, hierarchical or restricted (67 rows), one line:** star-plus-planets, star-planet-moon, circumbinary-planet,
  co-orbital, free-floating-planet and test-particle problems fail "three massive bodies" or the planar subset, and the
  hierarchical ones are parked under R-668: rows 2, 5, 6, 17, 24, 25, 28, 31-36, 45, 47-52, 54-58, 60, 62, 69-72, 79, 81,
  84, 86-88, 92, 93, 99, 100, 108, 110-113, 116, 117, 120, 122, 123, 130, 131, 136, 139, 142, 154, 155, 158, 161-163,
  168-171, 222.
- **Duplicates already triaged (15 rows):** rows 14, 27, 30, 37, 39, 40, 59, 91, 119, 124, 134, 135, 140, 147, 175. Each
  cites its earlier row; row 37 (Dey 2026) corrects it (below).
- **Off-filter for other reasons (13 rows):**

| Row | Item | Reason |
|---|---|---|
| 115 | Peiponen 2019, BSc thesis | Large-N algorithms; no three-body content |
| 126 | Rauch & Holman 1998 | Near-Keplerian and Stark test problems; integrator background only |
| 132 | Schilder, Wulff & Schebesch, SYMPERCON manual | Software manual, no three-body numbers; GPL/GFDL, never committed |
| 137 | Smulsky 2012, GALACTICA | Software overview; no comparable three-body numbers |
| 138 | Smulsky 2016, Galactica manual | Manual; no printed three-body results |
| 143 | Strang, Caruso & Greydanus 2023 | ML method paper; no starts, nothing comparable |
| 153 | Toth et al. 2020, Hamiltonian Generative Networks | ML from pixels; no comparable dynamics numbers |
| 166 | Wulff & Schilder 2014 | Spatial families only, and plots only |
| 182 | papers-112: Rossa ThreeBody v1.0 and others | Hierarchical or spatial, no released outputs; Rossa's force is clamped |
| 187 | papers-124: KubeNBody | Softened forces, no output data |
| 199 | papers-84: Rosenman, orbitr | Notes and textbook code only |
| 203 | papers-150: ThreeBodyFractal3, fractalio and others | Code without saved results |
| 209 | papers-94: Charlie-Zhao-666 N-Body-Simulation | Performance timings, not planar three-body outcomes; no f32 comparison |

- **Context with nothing to test (12 rows):**

| Row | Item | Reason |
|---|---|---|
| 159 | Venkatesh 2021, journal layout | Duplicate layout; keep the CC BY arXiv copy if either is wanted |
| 176 | papers-100 to 114 zips and receipt | Packaging only |
| 183 | papers-113 | Notes only |
| 185 | papers-119: Taylor Center samples | Inputs without outputs |
| 186 | papers-120 | Screening notes; no content to test |
| 191 | papers-128 `data/{BPZ,CFT,HD127195,Quarles,NBotox}` | Duplicates; the earlier rows stand |
| 192 | papers-128 `BENCHMARK_SHORTLIST.md` and cards | Superseded by these rows |
| 193 | papers-131 | No data set; leads ruled out by the batch itself |
| 195 | papers-133 | Restricted or non-numerical leads only |
| 196 | papers-139 | ChatGPT navigation only |
| 211 | papers-98 notes (Faber, Boily & Portegies Zwart 2008) | No new data set |
| 214 | `CHATGPT_BENCHMARK_TRIAGE.md` | A map, not a source; its errors are below |

## ML and student projects, one line each

- Row 12, Balsells Rodas et al. 2021, [Re] Hamiltonian Generative Networks: ML reproducibility report; CONTEXT.
- Row 22, Cacciola 2025, physics-based ML for chaos: student project; raw Brutus histories not released; CONTEXT.
- Row 23, Cai, Portegies Zwart & Podareanu 2021, neural symplectic integrator: no reference numbers; GPL code; CONTEXT.
- Row 29, Chen et al. 2020, Symplectic Recurrent Neural Networks: ML paper; CONTEXT.
- Row 38, DiPietro & Zhu 2022, symplectic symbolic regression: ML paper; CONTEXT.
- Row 40, Farhani Asl et al. 2026, ML prediction of binary formation: already triaged; OFF-FILTER, SKIP.
- Row 42, Fotiadis et al. 2023, disentangled generative models: ML paper; CONTEXT.
- Row 85, Koufogazou Loukianova 2026, random-feature Hamiltonian networks: BSc thesis; CONTEXT.
- Row 125, Rich 2026, Lyapunov and Kelvin-simplex descriptors: ML-style descriptors; CONTEXT (was the addendum's
  OFF-FILTER); its figure-eight exponent is a conflict (below).
- Row 143, Strang, Caruso & Greydanus 2023: ML method; OFF-FILTER, SKIP.
- Row 153, Toth et al. 2020: ML from pixels; OFF-FILTER, SKIP.
- Row 184, papers-114 (Tiwari): ML project, notes only; CONTEXT.
- Row 194, SiTu et al. 2025, BERT-ANN: code only, spatial; CONTEXT.
- Row 206, Almkvist 2025, AI for trajectory prediction (TCN, reservoir, LSTM): student project, not a reference; CONTEXT.
- Row 208, Sheng (Simonwan03), ThreeBodyMLBench: data unobtainable (403); CONTEXT.
- Student work, not ML: Gajdůšek 2024 BSc (row 45, restricted, SKIP); Kontiainen 2022 (row 83, PEEK-ONLY); Rios, PHYS
  3820 course paper (row 109; use Szebehely & Peters 1967 directly); Peiponen 2019 BSc (row 115, SKIP); Jones MSc 2023
  (row 141, PEEK-ONLY); Suzuki 2011 graduation abstract (row 144); Calda, Reali & Ricci, Bocconi course notebook (row
  201); Tolton 2025, Dremich release (row 212, PEEK-ONLY, AI-assisted).

## ChatGPT's shortlist and final guide: what was wrong

Both are readings, not authority; every claim was checked against the native file or PDF. Rows 192 and 214 hold them.

**The shortlist (`batches/papers-128/2026-11-02/BENCHMARK_SHORTLIST.md`):**

- BPZ record layout: "two DAT files, 10000 initial and 10000 final records" is wrong. There are two ensembles of 10,000
  rows × 46 columns, and each row holds both the initial and the final state.
- BPZ masses: "positive unequal masses" is wrong. MOESM3 has equal masses (1/3 each); MOESM4 has 4/7, 2/7, 1/7.
- BPZ is not new: it is committed in `docs/incoming/2026-10-31_precision_set/`, covered by R-661 and BZ-1C (row 219).
- Kandel digits: "approximately 30-40-digit" is wrong. All 86 files store 30 digits; 40 is the working precision.
- CFT: the 8-member ulp ensemble (`ens8_*.rec`) is omitted, and so is the GPL-3.0-or-later licence.
- Maissouradze: the AI assistance is not stated (`main.tex` credits Claude).
- BKM (Doumbouya 2026): the package's "Grok audit" (AI assistance) is not stated; it failed its own gates.
- HD127195 (Öz 2026) and Quarles 2018 are shortlisted but fail the filter: a star plus two planets, and test-particle
  planets in a binary.
- File links point to `data/<family>/` in papers-128, where only `data/BKM` and `data/Kandel` exist; the rest are in
  earlier batches. The Maissouradze file is in papers-12, not papers-128.

**The final guide (`CHATGPT_BENCHMARK_TRIAGE.md`):**

- BPZ: its rule "exclude zero placeholders" goes against R-661, under which those rows are undetermined or censored and
  never dropped.
- HD127195 is ranked the strongest source for priorities 2 and 3. It is one star and two planets (parked, R-668), and a
  MEGNO capped at 8 is not an exponent (row 222).
- Quarles native map: called an unequal-mass outcome set. The paper treats the planets "as test particles in binary
  systems": the restricted problem (row 123).
- HD181433, HD20003, α Cen and De Cesare are as described, but are planetary or hierarchical and fail the filter.
- CFT: it is GPL-3.0-or-later and AI-assisted (`VALIDATION.md` cites a CLAUDE.md); the guide says neither.
- BKM: the AI assistance is not mentioned.
- De Cesare: 2,048 CSVs by our count against the guide's 2,074 (whole release), not reconciled.
- Kandel: "repeated refinements" can't be seen in the parameters (86 distinct at 10 digits); unverifiable without
  integrating.
- Hirayama's quadruple-precision Pythagorean chapter: not in the library; the only Hirayama item held is a one-page 2006
  abstract with no numbers.
- Links: the BKM and CFT links resolve to earlier byte-identical copies (papers-12, papers-72).
- `eucliss_releases.json` is missing from the library (checked 3 Nov 2026, the owner's spot-check, item 4; this file
  cited it as `papers-152/.../metadata/eucliss_releases.json`, "a 3-byte empty list"). Papers-152's `manifest.json` and
  `checksums.sha256` list `metadata/eucliss_releases.json` (3 bytes, sha256 `37517e5f…b570`), but its `metadata/`
  folder does not hold it. The library holds that hash once, as
  `batches/papers-75/2026-11-01/source_evidence/NBotox_releases.json`, a 3-byte empty list, and `INDEX.csv` line 11935
  maps the papers-152 path to it. So the citation is to that file, whose bytes, by the shared hash, are the listed
  file's: an empty list, `[]`.
- Confirmed: the status "no matched f32/f64/mixed planar chaotic ensemble", "open for native FTLE", "open for
  time-weighted occupation", the pass notes for 140 to 153, and Tomihisa's abstract maxima differing from its thesis.

## Conflicts between and inside sources

Every row with a conflict carries it in the `conflict` column:

- **Agekyan, Anosova & Orlov 1983 (row 4):** Table 4 prints a bin "30-30" (should read 20-30).
- **Alves Silva 2021 (row 5):** batch papers-136 called it a conditional HIT; it is a star and two planets (HD 181433).
- **Anosova et al. 1990 (row 8):** rights: NTRS says public use permitted, Cambridge says copyright NASA 1990 (batch
  reading, not verified). Stays `_local`.
- **Bogomolov, Pavluchenko & Toporensky 2011 (row 19):** fit window 3000 in the caption, 2000 in the text; a likely sign
  typo in Table I's (1,3,5) row; its equal-mass index exceeds Orlov et al.'s (the authors say so); Dandekar's index is a
  different window and quantity.
- **Heggie 2000 (row 66):** batch provenance names a conflicting arXiv record; the DOI and journal reference stand.
- **Iman, Zaidyn & Usipov 2026 (row 76):** displacing r3 with fixed velocities breaks the zero-L and centre-of-mass set-up
  it states.
- **Maissouradze 2026 (row 90):** chunk 10's data path (papers-128) is wrong; the file is in papers-12.
- **Nauenberg 2002 (row 104):** chunk 05 reads three massive unequal bodies, chunk 12 a planetary 1:1 resonance. Both
  are true of its two regimes; the comparable part is figure-only.
- **Pavluchenko 2011 (row 114):** its indices differ from Orlov et al. 2010's; Pavluchenko attributes the earlier low
  indices to the Td^-3/2 term.
- **Petrovich 2015 (row 116):** Table 1 "2pl-inc-0" reads 1 degree against 0 in the text (unchecked; parked anyway).
- **Rich 2026 against Puodzius 2026 (rows 121, 125):** Rich's figure-eight λ_max 0.286 ± 0.051 contradicts its linear
  stability (Puodzius, Simó 2002, Roberts 2007); Rich uses fixed-step RK4 at h = 0.01, likely a finite-time artefact.
- **Quarles 2018 (row 123):** phases 0-360 in the data against 0-180 in the prose (as `triage.csv:274`).
- **Tian & Zhou 2003 (row 148):** Table 1 fails its own arithmetic for H, L and M; the text's "one tenth" against the
  table's 1/23; Table 2's survivors against "followed until all have broken up"; Table 2 lacks region A.
- **Tomihisa 2011 (row 151):** abstract speed-up maxima 144× and 170× against the thesis's 103× and 121×.
- **Wulff & Schebesch 2008 against Wulff & Schilder 2009 (row 164):** ω printed negative in one and positive in the
  other: a frame or sign convention to check before use.
- **Zadunaisky 1979 (row 167):** Table I's and Table III's time units are inconsistent, so any comparison is unit-free.
- **Kandel 2026 (row 190):** the shortlist's digits (above); chunk 10's "papers-12 was never triaged" is wrong (the
  addendum covers it).
- **Liu 2026, Julia Atlas (row 197):** its figure-eight differs from the held 100-digit value by about 1e-13 in v and
  4e-12 in T (ours).
- **De Cesare 2021 (row 215):** 2,048 CSVs against the guide's 2,074.
- **BPZ (row 219) and CFT (row 221):** the shortlist and guide errors above.

## Corrections to the earlier triage

Recorded here; the earlier files are left unchanged, but for `triage.csv` rows 25, 79 and 220, which take the owner's
spot-check of 3 Nov (§ "The owner's spot-check of 3 Nov 2026").

- **Dey 2026** (`triage-14-80.csv:173`, CONTEXT, RECORD-ONLY, unread): now read. A two-body flyby with a near-massless
  perturber, so OFF-FILTER, SKIP (row 37).
- **cft-rebound** (`triage-14-80.csv:98`, "no licence statement seen"): GPL-3.0-or-later (NOTICE; GitHub API), so `_local`
  and never committed. Raised to a NOW peek, NOW-29 (row 221).
- **Quarles 2018** (`triage.csv:274`, HIT, RECORD-ONLY, commit): OFF-FILTER, SKIP, settled from the filter itself, since
  the paper treats the planets as test particles (the restricted problem); R-668 would park it too as hierarchical
  (row 123).
- **Read as parked under R-668** (the earlier rows predate it): Tory 2022 (`triage.csv:54`, M9-18; row 152), Tamayo 2021
  (`triage.csv:273`; row 146), Georgakarakos 2024 (`triage.csv:275`; row 53), Iwakura 2025 (`triage.csv:279`; row 77).
- **The papers-11 to 13 addendum (`triage-11-13.csv`):** Gimeno 2026 (addendum row 302, HIT, RECORD-ONLY) is PARKED, every
  family having one or two bodies at ≤ 1e-3 of the mass (row 54); Saito & Tanikawa 2026 (row 307) is PARKED (R-668);
  Rich 2026 (row 314, OFF-FILTER) is CONTEXT, RECORD-ONLY under the ML rule (row 125); Doumbouya 2026 (row 300, NOW-27)
  is an M9 peek, M9-43 (row 189); Harrington 1970 (row 306, RECORD-ONLY, parked) is M9-41, not scored, like Heggie &
  Sweatman under R-669 (row 63); Puodzius 2026 (row 303) is M9-45 (row 121); Pickett 2026 (row 304) is NOW-33 (row 41).
- **Coverage:** batches papers-12 and papers-13 are fully covered by the papers-11 to 13 addendum. Chunk 10's "never
  triaged" and its papers-128 data path are wrong.

## Peeks only

Never targets; scored, if at all, as a peek (`docs/verification/peek_log.md`).

- **Unreviewed GitHub or Zenodo releases:** eucliss (rows 204, 205; also AI-authored), cft-rebound (221), mikha08-rgb/Nbody
  (188), eljja threebody-engine (210), Kandel Orbit Hunter (190), ThreeBodyFractal Image02 (202), Puodzius 2026 (121),
  Iman, Zaidyn & Usipov 2026 (76), Liu's Julia Atlas (197), Deopura's sandbox (200), Das, Dirac Labs (172, 175).
  Pavluchenko 2011 and Bogomolov et al. 2011 (rows 114, 19) are arXiv only: peeks unless a refereed version is found.
- **AI-assisted sources:** Maissouradze 2026 (90), Jeomseon 2026 (78), Pickett 2026 (41), cft-rebound (221), eucliss
  (204, 205), "The Pythagorean Three-Body Conjecture" (174), Tolton 2025 (212), Can 2026 (147).
- **Failed reproductions:** Doumbouya 2026, BKM (189; failed its own gates, and AI-assisted), and RATISS-ETALONS (179).

## The precision-gap answer

No source in the library compares f32, f64 and a converged reference across an ensemble of planar three-body runs, and
none compares CPU with GPU. R-661 stands as a hypothesis Principia must measure. Closest: cft-rebound (f64 vs fp128/fp256,
one Burrau trajectory); Portegies Zwart 2026 Table 4 (single-run 32-bit energy errors); mikha08-rgb/Nbody (f32 GPU vs
f64 CPU, figure-eight only); eucliss free-fall plane mirror pairs (f64 round-off alone: the mirrored escaper agrees
0.97 of the time for t < 10, 0.84 for 10–20, 0.63 for 20–50 and 0.51 past 50, chance about 1/3; `gaps-precision.csv`
gap 5); Tomihisa 2011 (a CPU-GPU parity check must state whether FMA contraction is on).

`gaps-precision.csv` holds seven gaps, the owner's three first (matched f32/f64/mixed/converged ensembles; per-point FTLE
maps; shape-sphere residence statistics), each framed as something Principia could produce and publish. The other four:
CPU against GPU divergence; the per-point fate reproducibility floor at f64 (our mirror-pair measurement on the eucliss
plane); the f32 divergence-time curve on Burrau and the figure-eight; converged long-horizon reference states. Weaker
sources named there: eljja, RATISS, Charlie-Zhao, Anosova 1990, Pruett 2003.

## Open questions for the owner

1. LL-3 and Hénon 1974. LL-3 (REQ-VAL-223) waits on a paywalled Standish 1970 (R-630 item 9). Hénon 1974's Table I A = 0
   row is Standish's orbit and matches Li & Liao's F1(0.6, 0.8, 1) period to 2e-10 (ours). Should LL-3 take Hénon 1974,
   already held under R-669, as its independent printed source, so it stops waiting on Standish? Or keep waiting?
   Queued as RQ-672 in `REVIEW_QUEUE.md`.
2. Planetary systems. Star-plus-planets papers are read here as parked with the hierarchical triples (R-668), so they would
   return with the v2+ 3D extension. Are they parked, or out of scope for good?
   Queued as RQ-673 in `REVIEW_QUEUE.md`.
3. The precision gaps as Principia's own output. Gap 1 already has BZ-1C (R-661). Should per-point FTLE maps (gap 2, over
   the eucliss free-fall plane and the two unequal-mass planes) and time-weighted shape-sphere residence statistics (gap 3)
   be planned as M9 peek outputs, which needs new requirements, or stay recorded here?
   Queued as RQ-674 in `REVIEW_QUEUE.md`.

## The owner's spot-check of 3 Nov 2026

The owner spot-checked the triage on 3 Nov: of 14 items, 11 confirmed and none wrong, with six fixes asked for before
any becomes a target. The standing rule from it: the source page is re-read before a triage item becomes a target. Each
fix below was made from the source itself, re-read on 3 Nov.

1. **Li, Li, He & Liao 2021.** Both printed counts are quoted, with where each appears: 1658 and 1646 new in the
   abstract (p. 1) and § 4 (p. 9); 1653 and 1642 new in § 3.2 (p. 6). The workbook stores at most 15 significant
   digits, not 90 (`docs/incoming/2026-10-31_library_triage/triage.csv`:79; LI21-1's precision and § 5.2 in
   `docs/incoming/2026-10-29_research_batches/README.md`).
2. **Tanikawa, Umehara & Abe 1995, Table IV.** Re-read from p. 360 of the held PDF (`docs/incoming/_local/`). The
   transcription committed on 28 Oct (`docs/incoming/2026-10-28_external_sources/tanikawa_umehara_abe_1995/table4_reconciled.csv`)
   matches the page in all 80 cells. The two misprints are kept as printed and noted: row 9's x is printed 0.49111357,
   and 0.4911357 fits; row 1's α is printed π/2, and the rule gives π/3 (`triage.csv`:220;
   `docs/incoming/2026-10-28_external_sources/README.md`).
3. **Horn et al. 2025.** The t ≤ 10 horizon is unverified: it rests on the batch's provenance file and 5 runs, and the
   paper trains to t = 5 and tests to t = 7 (§ 5.4, p. 14). It is to be checked against the HDF5 itself when the HDF5 is
   next touched (row 216; `triage.csv`:25; gap 7).
4. **`eucliss_releases.json`.** Missing from the library; the held file with the same bytes is cited (§ "ChatGPT's
   shortlist and final guide: what was wrong"; row 214).
5. **The batch-152 fate plane (NOW-28).** Mirror-pair escaper agreement is 0.97 (t < 10), 0.84 (10–20), 0.63 (20–50)
   and 0.51 (> 50), which our recount from `fatemap.json` reproduces. It replaces "noise past t ≈ 20" (Top 10 NOW row 1;
   § "The precision-gap answer"; row 204; gap 5).
6. **Stone & Leigh 2019 and Ginat & Perets 2021.** Both were fetched and both are committed, though neither is in the
   research library. Stone & Leigh 2019 is
   `docs/incoming/2026-10-25_replication/papers/stone_leigh_2019_Nature_576_406_arXiv1909.05272.pdf` (arXiv:1909.05272,
   sha256 `125e3c2e…76c6`; arXiv's default licence, kept under the licence audit, `decisions.md` § R-643). Ginat &
   Perets 2021 is `docs/incoming/2026-10-31_fetch_list/papers/ginat_perets_2021_PRX_11_031020.pdf` (Phys. Rev. X 11,
   031020, CC BY 4.0, from the owner's 31 Oct fetch list, `docs/incoming/2026-10-31_fetch_list/README.md`). The triage
   mentions them only as context, Stone & Leigh in `triage.csv`:42's conflict. Nothing is left to fetch.

## Decisions recorded (applied per R-369)

- `EXCLUDE` in the readers' brief is `OFF-FILTER` here, and planetary and hierarchical items are `PARKED`, as in the
  papers-14 to 80 pass. The brief's line "planetary/hierarchical or restricted ... EXCLUDE" restates the existing filter
  ("three massive bodies") and R-668, not a new rule, so each flip of an earlier verdict is settled from the filter and
  R-668 (Quarles 2018 above).
- Huang & Innanen 1983 (row 74) and Harrington 1972 (row 64), hierarchical triples at equal masses, are parked under
  R-668, against the chunk readers' M9.
- Harrington 1970 (row 63) goes to M9, not scored, as R-669 treats Heggie & Sweatman 1991: binary-single scattering, not
  a hierarchical triple, so R-668's parking doesn't reach it.
- Hénon 1974 (row 68) and Heggie & Sweatman 1991 (row 67) are cited as ruled (R-669), not re-proposed; the collinear fit
  printed on p.564 settles the Sweatman thesis row.
- BKM (Doumbouya 2026) is kept `_local` until a peek needs it.
- Values from repositories with no licence for their data are masked as `[value]` (mikha08-rgb/Nbody, eucliss and
  others); values from published papers stay, with citation, under R-668 (items 3 and 9, the owner's data-reuse rule).
- Tomihisa 2011 (row 151) takes the guide verifier's reading over chunk 07's one-liner, since the owner put it in the
  precision-gap check.
- Anosova, Bertov & Orlov 1984's velocity recipe is Anosova 1969, Trudy AO LGU 26, 88, not held; R-673's Anosova 1969 is
  a different paper.
- cft-rebound and Pickett 2026 are raised from RECORD-ONLY to NOW peeks, and Puodzius 2026 to an M9 peek; Doumbouya
  2026 moves from NOW-27 to M9-43. New ranks run on after the existing ones without re-ranking them.
- Duplicates cite `<file:row>` and keep the earlier verdict unless corrected; corrections are recorded here, and the
  earlier files are left unchanged, but for the three `triage.csv` rows the owner's spot-check of 3 Nov corrected.
- The papers-11 to 13 addendum is committed unchanged as `docs/incoming/2026-10-31_library_triage/triage-11-13.csv`,
  because R-669 cites its ranks and these rows cite it. It adds a file and changes nothing existing.

Removed lines in this PR's commits: none in the first two; later commits reword the 17 stale addendum citations, this
file's addendum paragraph and this line, and the 3 Nov commits the R-668 and R-669 citations and the values the owner's
spot-check replaced, each listed in its commit's note.
