# The owner's three 29 Oct research batches, and papers-2: a record

29 Oct 2026. Ingest and transcription only. Nothing here was scored against Principia's engine and no engine code ran;
every number is a transcription, or arithmetic on transcribed numbers by the scripts in `scripts/`. Every proposed
target is a measurement, not a gate (R-649, R-650 — the owner's 29 Oct rulings, quoted in § 10), and is written out
in § 10 as exact text for PLAN.md and REVIEW_QUEUE.md, to be applied after R-649 and R-650 are ported (task 4b). This
directory changes no requirement, ruling, task or existing row.

## 1. The request

The owner, 29 Oct 2026, verbatim:

> Three new research batches from me (ChatGPT-collected, inventory only): ~/Downloads/papers/2026-10-29/ (15 papers +
> 2 supplements; README, manifest.json, rejected.md, coverage.md, RETRY_DOWNLOADS.md), its further_research/ subfolder
> (11 PDFs), and ~/Downloads/research ChatGPT/ (5 papers + data/ JSON). Task 15, after the PR backlog. All licences
> "none stated" → docs/incoming/_local/; commit only transcriptions with citations. ChatGPT's data/*.json
> transcriptions are reading A only: transcribe independently from page images and diff. Verify every citation/DOI
> yourself.
> Priority targets: (1) Li, Li, He & Liao 2021, CeMDA 133, 46 + supp XLSX: 1653 free-fall triple-collision ICs (x, y,
> T) and symbol sequences, equal masses, CNS, collision distance <1e-8. Ingest the XLSX bytes-unchanged, parse,
> cross-check its rows against the paper's Table 2 sample. Check the frame matches Tanikawa–Saito–Mikkola 2019 and
> confirm it isn't covered by the excluded catalogue. Propose as a free-fall collision target: d_min→0 at T. (2) Li et
> al. 2025, Ap&SS 370, 99, Table 1: 8 unequal-mass free-fall triple collisions (stopping criterion 1e-6). (3) Tanikawa
> & Mikkola 2021 (CeMDA 133, 52), Tables 1–4, including the non-collision controls. Tanikawa & Mikkola 2023 (CeMDA 135,
> 54) supplement Tables 2–8: decode the alpha2/beta1 velocity parametrisation from §2 and §5 before any use; Table 1 is
> a recap of known seeds. (4) Kapela & Zgliczyński 2003, Nonlinearity 16, 1899: certified figure-eight enclosure, the
> only rigorous bound in the corpus. (5) Tanikawa 2000 Collision orbits II (8 tables). Secondary: Yasko 2015 / Iasko
> 2014 (use only the authors' own columns in Iasko, not the reproduced k′,φ′,T′); Hénon 1977 (positive-mass rows only);
> Orlov 2002/2003, Mikkola & Hietarinta 1989/1991 (collinear statistics); Hernandez 2015, Rantala 2020 (integrator
> benchmarks; check Rantala's labelling); Broucke 2006 (rotating Jacobi variables). Check Moore 2006 and Vanderbei
> 2004 against the periodic-catalogue exclusion. Context: the rest per the READMEs. Burrau 1913 and Schubart 1956 were
> blocked on Wiley: fetch the ADS scans instead.

And, the same day, verbatim:

> Also ~/Downloads/papers-2/ (README, manifest.json, checksums): Burrau 1913 (AN 195, 113; ADS 1913AN....195..113B):
> historical source of the Pythagorean set-up, record only, not a target (SP-2 is the converged reference). Schubart
> 1956 (AN 283, 17; ADS 1956AN....283...17S): Table 3 (PDF p.5, 23 rows: u, xi, eta, r13, x2, t) and the period
> relation on pp.21–22, equal-mass collinear periodic orbit, G=1. Transcribe twice from the scan; propose as a collinear
> closure test alongside Hénon 1977 and Titov's Table 10. Both "© Wiley-VCH", no licence: _local.

The ADS scans were therefore not fetched: papers-2 supplied both papers.

## 2. Method

- **Inputs are untrusted data.** Each batch was copied into its own new scratch directory; the two zips were listed,
  checked for path traversal (no `..`, no absolute paths) and unzipped there with `unzip -n`. Nothing in them was run.
  Every script here is ours, kept in `scripts/`, and runs as `python3 -I` with paths as arguments.
- **Checksums re-run on the copies:** `papers/2026-10-29/checksums.sha256` 23 of 23 OK; `research ChatGPT/SHA256SUMS.txt`
  16 of 16 OK; `further_research/checksums.sha256` 15 of 15 OK, both in the folder and inside the zip; papers-2
  `checksums.sha256` 4 of 4 OK. The `further_research/` folder holds the same 11 PDFs as the zip, not only notes: its
  notes are byte-identical to the zip's and both pass the same 15 checksums, so they agree.
- **Readings.** Reading B is ours, by eye from rendered page images (150–300 dpi), never a text layer: Broucke 2006's
  text layer drops minus signs that the page prints, for instance. The second reading is Apple Vision OCR of the same
  page images (`scripts/ocr.swift`); where whole-table OCR returned rotated glyphs it was run cell by cell
  (`scripts/ocr_cells.sh`). Where ChatGPT gave a transcription (`data/*.json`), it is reading A, diffed against
  reading B (`scripts/diffreadings.py`, cell by cell); every disagreement was resolved against the page, and the
  resolution is written in the diff file or, where the diff file carries none, in § 4's Resolution column. Reconciled
  CSVs are what the page prints.
- **Citations and DOIs.** All 33 DOIs were looked up in the Crossref REST API one request at a time
  (`scripts/crossref_check.py` → `doi_check.tsv`): every one resolves, and author, year, volume and pages agree with
  the batch manifests and the papers' first pages, with one note (§ 9). arXiv licences were read from the abstract
  pages, one request every 3 s (`scripts/arxiv_licence.py` → `arxiv_licences.tsv`).
- **Not done:** no engine code, no integration, no cargo build, no external code cloned or built, no account or
  paywall. `fetch.sh` was tested offline against our own copies only.

## 3. Licences, and what is committed

No paper or supplement in the batches carries an open licence. Crossref records only publisher terms or text-and-data-
mining licences (Springer/Springer Nature TDM, Elsevier TDM user licence, IOP TDM and copyright, Wiley terms,
Cambridge terms, OUP CHORUS standard model) or nothing; the first pages print publisher copyright; the arXiv versions
carry arXiv's non-exclusive distribution licence or the 1991–2003 assumed licence. So **every PDF and the Li 2021 XLSX
stay local** in `docs/incoming/_local/research_batches_2026-10-29/` (gitignored, `.gitignore:21`, confirmed with
`git check-ignore -v`), and only our transcriptions, derived checks and notes are committed, with citations. The Li
2021 XLSX's licence: none found (Springer's ESM terms, no data licence; the owner's copy came from the authors'
ResearchGate upload, which states none), so its bytes stay local and only our parse of its numbers
(`li2021/li2021_xlsx_ics.csv`) and our checks are committed. `sources.tsv` lists each local file with its sha256, the
verified licence evidence and how `fetch.sh` gets it back: Hernandez 2015, Moore 2006 and Rantala 2020 are
byte-identical to their arXiv versions (hashes checked) and are fetched from arXiv; the rest are subscription papers,
which `fetch.sh` only checks once you place them; Burrau and Schubart are the owner's copies and `fetch.sh` leaves them
out.

| Source | DOI (Crossref-verified) | Licence evidence (verified 29 Oct) | Committed | Label | Folder |
|---|---|---|---|---|---|
| Li, Li, He & Liao 2021, CeMDA 133, 46 (+ XLSX) | 10.1007/s10569-021-10044-6 | Springer TDM; "© The Author(s), under exclusive licence to Springer Nature" | transcriptions, XLSX parse | target (proposed LI21-1) | `li2021/` |
| Li, Dong, Li & He 2025, Ap&SS 370, 99 | 10.1007/s10509-025-04488-7 | Springer Nature TDM; exclusive licence | transcription | target (LI25-1) | `li2025/` |
| Tanikawa & Mikkola 2021, CeMDA 133, 52 | 10.1007/s10569-021-10050-8 | Springer TDM; exclusive licence | transcription | target data (TM-1 note) | `tanikawa_mikkola_2021/` |
| Tanikawa & Mikkola 2023, CeMDA 135, 54 (+ supplement) | 10.1007/s10569-023-10163-2 | Springer Nature TDM; exclusive licence | transcription | target (TM23-1) | `tanikawa_mikkola_2023/` |
| Kapela & Zgliczyński 2003, Nonlinearity 16, 1899 | 10.1088/0951-7715/16/6/302 | Crossref none; "© 2003 IOP Publishing Ltd and LMS"; arXiv math/0304404 assumed-1991-2003 | transcription | target (KZ-1) | `kapela_zgliczynski_2003/` |
| Tanikawa 2000, CeMDA 76, 157 | 10.1023/A:1008313912013 | Springer Nature TDM | transcription | target, positions (T2000-1); RQ-640 | `tanikawa_2000/` |
| Yasko & Orlov 2015, Astron. Rep. 59, 404 | 10.1134/S106377291505008X | Springer Nature TDM | transcription | context (approximate returns) | `yasko_2015/` |
| Iasko & Orlov 2014, Astron. Rep. 58, 869 | 10.1134/S1063772914110080 | Springer Nature TDM | transcription (authors' columns) | context | `iasko_2014/` |
| Hénon 1977, Cel. Mech. 15, 243 | 10.1007/BF01228465 | Springer TDM; "Copyright © 1977 by D. Reidel" | transcription (positive-mass rows) | target (HE-1) | `henon_1977/` |
| Orlov, Petrova & Martynova 2002, Astron. Rep. 46, 932 | 10.1134/1.1522082 | Springer Nature TDM | transcription | target, statistics (OR02-1) | `orlov_2002/` |
| Orlov, Petrova & Martynova 2003, Astron. Rep. 47, 254 | 10.1134/1.1562220 | Springer Nature TDM | transcription | target, statistics (OR03-1) | `orlov_2003/` |
| Mikkola & Hietarinta 1989, CeMDA 46, 1 | 10.1007/BF02426707 | Crossref none | transcription | target, statistics (MH89-1) | `mikkola_hietarinta_1989/` |
| Mikkola & Hietarinta 1991, CeMDA 51, 379 | 10.1007/BF00052929 | Springer TDM | transcription | target (MH91-1) | `mikkola_hietarinta_1991/` |
| Hernandez & Bertschinger 2015, MNRAS 452, 1934 | 10.1093/mnras/stv1439 | Crossref none; arXiv 1503.02728v2 non-exclusive | transcription | conditional, information only (HB15-1) | `hernandez_2015/` |
| Rantala et al. 2020, MNRAS 492, 4131 | 10.1093/mnras/staa084 | OUP CHORUS standard; arXiv 2001.03180v1 non-exclusive | transcription (§ 5.2 values) | information beside SP-2 (RA20-1) | `rantala_2020/` |
| Broucke, Elipe & Riaguas 2006, CSF 30, 513 | 10.1016/j.chaos.2005.11.082 | Elsevier TDM user licence | transcription | no target until RQ-641 | `broucke_2006/` |
| Moore & Nauenberg 2006, JCND 1, 307 | 10.1115/1.2338323 | Crossref none; arXiv math/0511219v2 non-exclusive | transcription | excluded (in the catalogue) | `moore_2006/` |
| Vanderbei 2004, Ann. NY Acad. Sci. 1017, 422 | 10.1196/annals.1311.024 | Wiley terms; arXiv astro-ph/0303153 assumed-1991-2003 | transcription | context (no ICs) | `vanderbei_2004/` |
| Tanikawa & Umehara 1998, CeMDA 70, 167 | 10.1023/A:1008301405839 | Springer Nature TDM | transcription | context (three examples) | `tanikawa_umehara_1998/` |
| Martynova & Orlov 2014, Astron. Rep. 58, 756 | 10.1134/S1063772914100084 | Springer Nature TDM | transcription | target, statistics (MO14-1); RQ-642 | `martynova_orlov_2014/` |
| Mel'nikov, Orlov & Shevchenko 2013, Astron. Rep. 57, 429 | 10.1134/S1063772913060048 | Springer Nature TDM | fit table only | task 15 item 11 owns it | `melnikov_2013/` |
| Schubart 1956, AN 283, 17 | 10.1002/asna.19562830105 | Wiley terms; "© WILEY-VCH"; ADS 1956AN....283...17S | transcription | target (SCH-1) | `schubart_1956/` |
| Burrau 1913, AN 195, 113 | 10.1002/asna.19131950602 | Wiley terms; ADS 1913AN....195..113B | none (quote only) | record only | § 8 |
| Context papers (§ 7) | see § 7 | see `sources.tsv` | none | context | — |

## 4. Transcriptions: readings, diffs and resolutions

"Eye" is reading B; "OCR" the second reading from the same page images; "A" is ChatGPT's JSON. Counts are cells (or
numbers, for whole-table OCR sequences) compared.

| Source, table (PDF page) | Rows | Readings | Disagreements | Resolution |
|---|---|---|---|---|
| Li 2021 Table 1, isosceles IT-0…IT-4 (p. 6) | 5 | eye, OCR | 1 of 20 | IT-1's lone sequence "1" dropped by OCR; page prints 1 |
| Li 2021 Table 2, GT sample (p. 9) | 6 | eye, OCR | 0 of 24 | — ; and 18 of 18 values equal the workbook rounded (`li2021/table2_vs_xlsx.txt`) |
| Li 2021 XLSX | 1653 | our parse of the sheet XML, bytes unchanged | — | 1653 labels unique, IC and sequence sheets aligned (`li2021/li2021_xlsx_parse_report.txt`) |
| Li 2025 Table 1 (p. 7) | 8 | eye, OCR | 5 of 40 | OCR dropped lone masses "1", "2", "6" and shifted one cell; page prints them |
| Tanikawa & Mikkola 2021 Tables 1–4 | 9, 11, 11, 9 | eye, OCR | 0, 6, 0, 0 | Table 2: OCR dropped five lone row numbers and split one row; page settles all |
| Tanikawa & Mikkola 2023 supplement Tables 2–8 | 432 | eye, OCR | 11 of 3888 | OCR mirror-image fragments of 6 cells and 5 phantom rows; page settles all |
| Kapela & Zgliczyński 2003 Table 1 + x0, δ (p. 1906) | 20 numbers | eye, OCR | 0; δ superscript lost by OCR | δ = 10⁻⁶ from the page |
| Tanikawa 2000 Tables I–VIII + p. 162 table | 17/16/17/9/16/15/10/16 + 3 | A, eye, OCR | A–B 0 in all eight; B–OCR 0 | Table II: OCR read old-style 0 as O; Table V row 12's y re-OCR'd as a band (0.4262813) |
| Schubart 1956 Table 3 (p. 5) | 23 | eye, OCR | 1 of 138 | u = 0.5, x2: OCR −0.30340, page −0.30346 (zoom; and the identity x2 = ξ − η/3) |
| Hénon 1977 Tables I–IV, positive-mass rows | 20/24/13/24 | eye, OCR | 0 of 624 numbers | Table IV: a speck read "0:46", page 0.46 |
| Yasko 2015 table | 22 | eye, OCR | 2 | row 22's sequence: our eye read 9 B, the page prints 10 (column projection count); row 3's dash remark |
| Iasko 2014 table, authors' columns k, φ, T, notes | 23 | eye, OCR | 0 of 115 | — |
| Orlov 2002 Table 1 | 8 | eye, OCR | 0 of 8 | — |
| Orlov 2003 table (35 × 5) | 35 | eye, OCR per cell | 2 | two lone "0" cells empty in OCR; page prints 0 |
| Mikkola & Hietarinta 1989 Tables 1–4 | 5/4/2/7 | eye, OCR (Table 3 per cell) | 0/0/2/0 | Table 3's "9.×10⁷" (t8, both rows): OCR lost the superscript |
| Mikkola & Hietarinta 1991 Table I | 15 | eye, OCR | 3 of 165 (2 kinds) | a speck read as a minus on (80, 0) k_t; the 10³ superscripts |
| Hernandez 2015 Table 1 | 2 | eye, OCR | 0 of 18 | — |
| Rantala 2020 § 5.2 values (p. 14) | 10 | eye, OCR | 0; three superscripts lost by OCR | 10⁸, 10⁸, 10⁻⁴ from the page |
| Broucke 2006 Tables 1–3 | 2/2/13 | eye, OCR | 0/2/1 | OCR dropped two decimal points and one r = 0.5 |
| Vanderbei 2004 Tables 1–2 | 7/11 | eye, OCR | 0/0 | — |
| Moore 2006 Table 3 and ICs | 8/3 | eye, OCR | 0/0 | — |
| Tanikawa & Umehara 1998, Fig. 4 points (p. 172–173) | 3 | A, eye | 0 of 9 | — |
| Martynova & Orlov 2014, grid summary and Figs. 9–15 (pp. 760–764) | 12 | A, eye | 0 of 48 | — |
| Mel'nikov et al. 2013, fit table (p. 438) | 6 | A, eye | 0 of 36 | — |

The owner asked for some tables twice by name; each of those has two readings above. ChatGPT gave reading A for
Tanikawa 2000, Tanikawa & Umehara 1998, Martynova & Orlov 2014 and Mel'nikov 2013 only; ChatGPT's
`Melnikov_2013_planar_examples.json` is its own computation from the paper's formulas (it says so: "Derived
double-precision evaluation of published formulas"), not a transcription, so it is recorded here as computed by
ChatGPT, unverified, and not used.

## 5. Findings

### 5.1 Free-fall frames (Li 2021, TSM 2019, Tanikawa 2000, TU 1998, MO 2014, Li 2025)

All of these use one frame. Li et al. 2021 § 2.1 (p. 3): "The initial locations of body-2 and body-3 are at (−0.5, 0)
and (0.5, 0), respectively. … the initial position (x, y) of body-1 is set in the Agekyan and Anosova's region D:
D = {(x, y)|x ≥ 0; y ≥ 0; (x + 0.5)² + y² ≤ 1}", and p. 2: "G = 1 and the masses of the three bodies m1 = m2 = m3 = 1",
at rest. Tanikawa, Saito & Mikkola 2019 (accepted manuscript, p. 9; task 15 item 2): "We put body 2 of mass m2 at
A(−0.5, 0) and body 3 of mass m3 at B(0.5, 0) both on the x-axis … We put body 1 of mass m1 at any place P in
D = {(x, y) : x ≥ 0; y ≥ 0; (x + 0.5)² + y² ≤ 1}. … We consider the equal mass case, i.e., m1 = m2 = m3 = 1", and "D11
corresponds to Anosova's region". The same: no mirror, no relabelling of positions. The check: all 11 TSM 2019 Table 1
points are Li 2021 workbook rows, within 1.1e-9 to 1.5e-8 in position and 3.4e-7 to 1.6e-5 in T, which also confirms
the same time unit (`li2021/workbook_checks.txt` item 3). Tanikawa & Umehara 1998 (p. 168) and Martynova & Orlov 2014
(pp. 756–758, Agekyan–Anosova's A, B, C at (−0.5, 0), (0.5, 0), (ξ, η)) use the same D; Tanikawa 2000 refers to Paper I
for it. Li 2025 (Fig. 1, p. 2) keeps the positions and varies the masses (m3 = 1 throughout Table 1).

### 5.2 Li et al. 2021: the workbook, Table 2, and the exclusion

- The XLSX parses to 1653 rows (label, x, y, T) and 1653 symbol sequences, labels unique and aligned, 12 to 15
  significant digits per value as stored. All 1653 lie in D; no duplicate (x, y); T from 0.4817 to 17.447; sequences 3
  to 120 symbols over {1, 2, 3}.
- Table 2's six printed rows (16 digits) equal the workbook's values rounded, 18 of 18.
- Accuracy stated by the authors: "d(T, q) < 10⁻⁸" for the Newton iteration's stop, "error tolerance of 10⁻⁶⁰", and
  "The minimum of any two bodies' distance is larger than 10⁻⁶ before the three bodies collide" (pp. 4–6). Their
  levels, not bounds.
- **The exclusion.** The repo records no separate exclusion list: grepping `docs/incoming/2026-10-26_orbit_catalogues/`,
  PLAN.md and REVIEW_QUEUE.md finds "exclu…" only in a licence quote. The exclusion is therefore the threebodyorbits
  snapshot itself (PLAN.md's `OC26`, already targeted by OC-1 to OC-6). Its free-fall rows with equal unit masses at
  rest: 27. **Matching rule:** same frame, position distance < 1e-6, either mirror. **Overlap: 0 of 1653**; the
  closest pair is 1.19e-3 apart (freefall_part1_f17111 and GT-770). The Li 2021 set is new to the corpus.

### 5.3 Li et al. 2025

Eight unequal-mass triple-collision orbits continued from equal-mass ones (m3 = 1; m1, m2 in the table), CNS at
tolerance 10⁻⁶⁰ and 90 digits; the Newton stop "when the distance function d(Tc, q) < 10⁻⁶" (p. 4): the authors' level,
not a bound. The collision point is the barycentre, ((m1 x − (m2 − m3)/2)/Σm, m1 y/Σm) (p. 3). Its (1, 1) row is Li
2021's GT-6 (= TSM 2019 point 3: (0.10677930, 0.52012268)).

### 5.4 Tanikawa & Mikkola 2021 and 2023

- 2021: Table 2's 11 triple-collision points are TSM 2019's Table 1 (and so Li 2021 GT rows). Table 4's nine
  non-collision controls are TSM point 1 displaced by 10⁻³, 10⁻⁴ and 10⁻⁵ at azimuths 60°, 120° and 180°; their
  geometry checks to the printed digits (largest residual 5.4e-9, `table4_geometry_check.txt`, written by
  `scripts/tm21_table4_geometry.py`). They are the negative cases for the Li 2021 target: their minimum of Li's d(t, q)
  (eq. 4), against the displacement, is the scaling a collision start is read against (§ 10.3, LI21-1).
- 2023, decoded (`tanikawa_mikkola_2023/decode_check.txt`, `scripts/tm23_decode.py`): unit masses, G = 1; body 1 at
  (x, y), bodies 2 and 3 at (∓0.5, 0); barycentric R1 = (2/3)(x, y), R2 = −(1/3)(x, y) − (0.5, 0),
  R3 = −(1/3)(x, y) + (0.5, 0); V1 = −2α2 R1 + β1 R2, V2 = α2 R1 − 2β1 R2, V3 = −V1 − V2 (eqs. 20–23). All 432 rows
  then have zero momentum, zero angular momentum and a fixed barycentre, exactly. **One conflict inside the paper:**
  Table 8's caption writes V1 = (1 − t)(−2α2)R1 + tβ1R2, V2 = (1 − t)α2R1 + t(−2β1)R2; § 6.1's text writes
  V1 = t(−2α2R1) + sβ1R2, V2 = t(α2R1) + s(−2β1R2). The data settle it: under the text's form, Table 8's (t, s) =
  (1, 0) and (0, 1) rows reproduce rows of Tables 2 and 7 exactly (difference 0), and (−1, 0) to 1e-7; under the
  caption's form they miss by 1.4e-2 to 4.3e-2. So the text's form is used and the caption is a slip; no RQ, since it
  is decoded unambiguously. Table 1 recaps known seeds and is not transcribed as new.

### 5.5 Kapela & Zgliczyński 2003: the certified enclosure

- In their frame (eq. 1; unit masses, G = 1) the initial condition is E(v, u) = (1, 0, −1, 0, 0, 0, v, u, v, u, −2v,
  −2u). The proof's box: x0 = (0.347116768716, 0.532724944657), δ = 10⁻⁶ (p. 1906), and the interval-Newton image
  N(x0, [X]) = [0.347116886243943, 0.347116889993313] × [0.532724941587373, 0.532724949187495], inside [x0 ± δ],
  widths equal to the printed diameters to 1e-15 (`kapela_zgliczynski_2003/enclosure_check.txt`). The paper prints no
  period; the convexity proof's 53 steps of h = 0.01 with T = 12 T̃ give T in (6.24, 6.36], coarse.
- Energy over the box: [−1.287142011685, −1.287141979584], which contains the figure-eight energy that REQ-VAL-041's
  source quotes (`docs/design/principia_dd_validation_orbits.md` § 0, E = −1.2871419918); its period 6.32591398 lies
  in (6.24, 6.36].
- **The catalogue's published figure-eight is outside the proven box.** threebodyorbits V.1.A, same frame:
  `pub_*` (0.347113, 0.532727) lies 3.886e-6 and 2.051e-6 beyond the box, more than its 6-decimal rounding (5e-7);
  `ref_*` (0.34711688811892694, 0.5327249453880303) lies inside. So the published 6-decimal V.1.A start is not a
  rounding of the true orbit, and a figure-eight test must start from ≥ 9 correct digits.

### 5.6 Tanikawa 2000 (Collision orbits II)

- Eight tables: Tables I–IV are sequences of triple-collision points accumulating at region boundaries (n_min, x, y,
  α, time); V–VII binary-collision curves of types 1–3; VIII near-isosceles curves of types 1 and 3; and an unnumbered
  table on p. 162. In-paper cross-checks all hold (`tanikawa_2000/crosschecks.txt`).
- The Fig. 11 triple-collision point (0.2794981, 0.5759310) is Li 2021's GT-36 (= TSM point 8) to 1.06e-6, about 7
  units of the 7th decimal. The "y-axis point nearly at y = 0.18169" is Li 2021's isosceles IT-1, y =
  0.18169092589591662, to its 5 digits.
- **Units are not stated:** "m1 = m2 = m3" (p. 157), with neither the mass value, G nor a time unit. Positions are
  scale-free in equal-mass free fall, so the x, y columns compare like for like. The time columns do not: no
  Tanikawa 2000 triple-collision point coincides with a Li 2021 orbit (closest 1.1e-2, `tanikawa_2000/vs_li2021.txt`),
  so the unit cannot be fixed that way. RQ-640.
- p. 172 points to Tanikawa, Umehara & Abe 1995's Table IV (task 15 item 16).

### 5.7 Schubart 1956, decoded (pp. 21–22; `schubart_1956/decode_check.txt`, `scripts/schubart_decode.py`)

- Normalisation: m = 1, G = 1; length unit r13 = r23 = 1 at the 1–2 collision at t = 0. Variables: u = 2∫dt/(r12 r23);
  ξ = r12/2; η = r23 + r12/2; so r13 = ξ + η and x2 = ξ − η/3. The 23 rows satisfy both identities to 1.0e-5 (the
  table's 5 decimals), and Simpson integration of dt/du = ξ(η − ξ) reproduces the t column to 5.4e-6.
- Period relation: x2 = 0 at t* = 0.21470 ("einigermaßen sicher" to the last decimal), period 4t* = 0.85880;
  r13 max = 1.22949; lim (4/r12 − ṙ12²) = 4.91610 at the collision. The linear zero of x2 from the last two rows gives
  t* = 0.21470, as printed.
- Energy E = −lim/4 − 2 = −3.229025. Cartesian state at x2 = 0 (barycentric, on the x-axis, order m1, m2, m3):
  q = (−0.614745, 0, +0.614745), v = (−0.528426, +1.056852, −0.528426); the table's own slope of x2 gives 1.057, the
  energy 1.05685. Rescaled to E = −1: q3 = 1.985027, v2 = 0.588137, T = 4.98309.
- Cross-checks: Hénon 1977 Table IV's equal-mass row, rescaled, gives x3 1.323348 against 1.323351, u −0.207939
  against −0.207938, T 4.698084 against 4.698107 (inside the ±1.1e-4 that t*'s 5 decimals allow); Mikkola &
  Hietarinta 1991's equal-mass row R 1.98502 against 1.985027; Iasko 2014's orbit 14 k 0.203 against 0.20599 and T
  0.90 τ against 0.90415 τ; Orlov 2002 Fig. 1 places Schubart's orbit at k = 0.206.

### 5.8 Hénon 1977 and Mikkola & Hietarinta 1991

- Hénon: G = 1, Σm = 1, E = −½ Σ m_i m_j, time origin at u1 = u3. 81 positive-mass rows; Σm exact, energies against
  eq. 5 within 2.8e-6 (6-decimal rows); the doubly critical orbit appears mirrored in Tables I and II. Stated accuracy
  "of the order of 10⁻⁸" for the critical orbits. Hénon's k is twice Mikkola & Hietarinta's.
- MH 1991 Table I: masses sum to 3, E = −1. Masses follow from (Ia, Ib) exactly; arccos k_l gives λ. Decoded from the
  data: P = T / R^{3/2}, T the period in these units, not the period itself as the caption reads — on the four
  Ib = 0 rows Hénon covers (interpolated), R, k_l and P agree with Hénon to 1e-4 or better; the (1.49, 0.02, 1.49) row
  does not, because interpolating Hénon's table fails as m2 → 0. Decoded unambiguously by the data, so no RQ (the same
  rule as § 5.4).

### 5.9 Collinear and free-fall statistics

- Orlov et al. 2002 Table 1 (equal masses, h = −1, G = 1; k = 0.01(0.01)0.99, φ = 1°(1°)360°, 35,640 starts; stop at
  escape by Marchal and Yoshida's criterion, at R > 30, or t = 100): fractions sum to 1.000; "triple close
  encounters 0.005" is the φ = 45°, 225° lines, 198/35,640 = 0.00556.
- Orlov et al. 2003 table: the "1–5" column equals the sum of n = 1…5 to 0.001; 10 of 70 mirrored entries differ by up
  to 0.004, the table's noise.
- Mikkola & Hietarinta 1989: E = −1, a 100 × 180 (R, θ) grid (eq. 7), 17,670 break-ups, z = |E|/B. Tables 1 and 2 sum
  to 100 % (±0.2); Table 2's "all" row is the case-weighted mean of its two rows to 0.1; Table 4's "true" column is
  Table 3's one-directional row (its caption says "Table 2": a slip). Table 4's k = 6 rational-model value prints 784.2;
  the model computes 782.2 (k = 7: 1628.8 against 1628.9). Recorded; a paper slip, nothing depends on it.
- Martynova & Orlov 2014 (page images, p. 760): "306,346 versions … In 306,320 … decay … T < t_crit = 10,000,000 T_cr.
  In the 26 remaining cases …"; power-law index α = 1.74 over 10⁵–10⁶ T_cr (Fig. 2; literature 1.4–2.2). T_cr =
  √G (Σm)^{5/2}/(2|E|)^{3/2} (p. 759; ChatGPT's JSON writes G for √G, immaterial at G = 1). **Our count of interior grid
  points of D at step 0.001 is 306,402 (arc excluded) or 306,406 (arc included), 56 or 60 more than printed**
  (`martynova_orlov_2014/checks.txt`). RQ-642.
- Mel'nikov et al. 2013's fit table (task 15 item 11 owns this paper): our reading equals ChatGPT's, 36 of 36; nothing
  is proposed here.

### 5.10 Integrator benchmarks

- Hernandez & Bertschinger 2015 Table 1: the Pythagorean problem to t = 2.0; DKD leapfrog at h = 10⁻⁴: |ΔE/E0| 8.2e-6,
  ΔL 1.2e-13, t_cpu 7.5; their φ_h² at h = 0.0015: 3.7e-6. A statement about a fixed-step DKD leapfrog only: it
  applies to Principia only if an occupant is that integrator.
- Rantala et al. 2020 § 5.2, labelling checked against the page: the prose places the masses "in such a manner that
  the least massive SMBH is opposite to the shortest side" (Burrau's arrangement), but the subscripts r13 = 30,
  r12 = 40, r23 = 50 pc put the longest side opposite M1, the least massive. The subscripts are a slip; the prose,
  the outcome (M1 escapes) and β1 ≈ 71.4° all match Burrau. β1 is defined (eq. 32) as arctan(Y1/X1) with the triangle
  oriented as Aarseth et al. 1994's Fig. 1, which the corpus already holds (`../2026-10-27_papers/TARGETS.md`, "Common
  set-up" and P-1: "β = 71.4 degrees, corresponding to the original Pythagorean triangle"), so no RQ. Results
  (AR-CHAIN and MSTAR agree to ~10⁻⁴): β1 ≈ 71.4°, a2,3 ≈ 5.5 pc (0.55 in Burrau's units, lengths scaled by 10 pc),
  e2,3 ≈ 0.99.

### 5.11 Periodic-catalogue checks (Moore 2006, Vanderbei 2004, Yasko 2015, Iasko 2014, Broucke 2006)

- Moore & Nauenberg 2006 criss-cross: the Fourier series reproduces the printed ICs (truncation: vy3 1.03660 against
  1.03678); E = −1.107938, L = −0.744624, T = 2π. By the scale invariants T|E|^{3/2} and L²|E| it is threebodyorbits
  broucke_broucke_a_7 to 5.6e-6 relative: already in the catalogue, so excluded.
- Vanderbei 2004: stability eigenvalues only, no ICs (FigureEight3, Ducati3, Hill3_15, Lagrange3 are the three-body
  ones; the catalogue has no Ducati). Context, no target.
- Yasko 2015: 0 of 22 match the catalogue's 27 equal-mass free-fall rows (rule: distance < 1e-3 and |ΔT| < 0.05 τ).
  Y17 lies 2.85e-2 from F1 with the same period: plausibly F1 found coarsely; recorded, not counted.
- Iasko 2014 (authors' k, φ, T only): 9 of 21 match Šuvakov–Dmitrašinović catalogue rows (rule: |Δk| < 0.005,
  |Δφ| < 0.01, |ΔT| < 0.05 τ), including orbit 19 (i_butterfly_i_5_a), presented as new. Orbit 10's k = 0.418
  disagrees with the paper's own k′ = 0.428396 while φ and T agree with k′: an internal slip, recorded. Approximate
  returns at 2–3 digits; no target.
- Broucke et al. 2006 (`broucke_2006/decode_attempt.txt`, written by `scripts/broucke2006_decode.py`): the rotating
  Jacobi variables cannot be decoded from this paper. Read as G = 1, m = 1/3, the Table 1 states have E > 0; no single
  G fits both rows. The definitions are in its refs. [7] (AIAA 2002-4544) and [10] (Broucke & Boggs 1975), not in the
  repo. RQ-641. What holds: Table 2's y/r equals Table 1's y to 1e-8; Table 3's f1 and f3 follow from a1, a2 as its
  footnote defines.

## 6. Labels

Each source's label is in § 3's table: **target** (proposed in § 10, tier stated there), **context** (cited, no
target, reason in § 5 or § 7), **excluded** (already in the threebodyorbits catalogue), **record only** (Burrau).

## 7. Context entries (the owner: "the rest per the READMEs")

Each is local only; citation and DOI verified (Crossref), licence as in `sources.tsv`.

| Paper | DOI | Why no target |
|---|---|---|
| Boyd & McMillan 1993, Chaos 3, 507, "Chaotic scattering in the gravitational three-body problem" | 10.1063/1.165956 | Scattering statistics in figures; the batch README lists no table of per-point values |
| Broucke & Walker 1980, Cel. Mech. 21, 73, rectilinear problem | 10.1007/BF01230249 | The owner ranks it context. Its Table I (p. 77; 11 rows, equal masses 1/3, X = Y = 1, T/2 not T) holds rectilinear periodic and triple-collision orbits that would sit beside SCH-1; not transcribed in this ingest |
| Chen 2013, Nonlinearity 26, 2875, periodic brake orbits in the isosceles problem | 10.1088/0951-7715/26/10/2875 | Existence proofs and figures; no numerical table verified |
| Doedel et al. 2003, IJBC 13, 1353 | 10.1142/S0218127403007291 | Scalar figure-eight Floquet values and actions in the text; the figure-eight is covered by REQ-VAL-041 and KZ-1 |
| Hietarinta & Mikkola 1993, Chaos 3, 183, collinear chaos | 10.1063/1.165984 | Maps in figures; the collinear statistics targets come from MH 1989 and Orlov 2002/2003 |
| Kuwabara & Tanikawa 2007, Chaos 17, 033105, rotation in the planar problem | 10.1063/1.2753126 | Maps and statistics in figures; no per-point table |
| Mikkola & Tanikawa 2013, New Astron. 20, 38, logarithmic-Hamiltonian code | 10.1016/j.newast.2012.09.004 | A method paper; code on request |
| Mikkola & Hietarinta 1990, CeMDA 47, 321, positive energies | 10.1007/BF00051009 | Maps in figures only (the batch README: "plot-only") |
| Mikkola 2008, "A brief history of regularisation", Proc. IAU 3 (S246), 218 | 10.1017/S1743921308015639 | A review; its numerical test has seven bodies |
| Tanikawa & Mikkola 2000, CeMDA 76, 23, triple collisions in the one-dimensional problem | 10.1023/A:1008397420958 | Collision curves in figures; no table of points |
| Kapela & Zgliczyński 2003, the four- and six-body sections | (as § 3) | Not three-body |

## 8. Burrau 1913 and Schubart 1956 (papers-2)

- **Burrau 1913**, "Numerische Berechnung eines Spezialfalles des Dreikörperproblems", Astronomische Nachrichten 195
  (Nr. 4662), 113–118; doi:10.1002/asna.19131950602; ADS 1913AN....195..113B. The owner's copy is the ADS scan; ©
  Wiley-VCH, no licence: local only, left out of `fetch.sh`. **Record only, not a target.** The set-up, from the page
  image (p. 113): "m1 = 5, m2 = 4, m3 = 3. Anfangsgeschwindigkeiten = 0. Anfangsabstände ρ1 = (m2 m3) = 5,
  ρ2 = (m3 m1) = 4, ρ3 = (m1 m2) = 3." Each mass sits opposite the side of its own length; Burrau numbers the masses
  5, 4, 3, where Szebehely & Peters and Aarseth et al. number them 3, 4, 5. The converged reference is SP-2, Portegies
  Zwart & Boekholt 2018: `../2026-10-29_burrau_pzb2018/`.
- **Schubart 1956**, "Numerische Aufsuchung periodischer Lösungen im Dreikörperproblem", Astronomische Nachrichten 283
  (1), 17–22; doi:10.1002/asna.19562830105; ADS 1956AN....283...17S. © Wiley-VCH, no licence: local only, left out of
  `fetch.sh`. Table 3 transcribed twice from the scan (§ 4), decoded and checked (§ 5.7); proposed as SCH-1 (§ 10).

## 9. Not obtained, or not verified

- Tanikawa 2000's mass and time normalisation (RQ-640); Broucke 2006's variables (RQ-641); the 56-point gap in
  Martynova & Orlov's grid count (RQ-642).
- The Li 2021 XLSX's licence: none found; bytes local.
- Crossref dates Mikkola's "A brief history of regularisation" 2007 (the IAU Symposium 246 volume); the batch manifest
  cites it as 2008. Volume and pages agree; recorded only.
- Tanikawa 2000's "time" for its triple-collision points is short (0.39–0.49 in Table I) beside Li 2021's T for
  nearby orbits (6.9–11.3), so either the unit or the quantity differs; part of RQ-640.
- Not transcribed, as the owner ranks them context: Broucke & Walker 1980's Table I; Tanikawa & Mikkola 2023's Table 1
  (recap of seeds).

## 10. To apply after R-649 and R-650 are ported (task 4b)

#328 merged on 29 Oct 2026 (20:10 UTC) and #335 at 21:51 UTC. A porter applies these after R-649 and R-650 are ported
into decisions.md (task 4b), in this order: PLAN.md, then REVIEW_QUEUE.md, renumbering where needed. RQ numbers start
at RQ-640 as assigned. Every row below is a proposal. Each
follows the owner's 29 Oct rulings (R-649 and R-650 in this task's dispatch; verbatim: "A target is a MEASUREMENT, not
a gate. Failing one never blocks building"; "(a) GPU explorer … expected to agree statistically or qualitatively only
…; (b) CPU IC inspector: f64/high precision; exact targets (collision points, periods, converged states) bind here";
"Three valid outcomes: PASS; EXPLAINED DIVERGENCE …; UNEXPLAINED"; "Before calling anything a failure, check the
comparison is like-for-like (frame, units, masses, definition of the event, the source's stated accuracy)"; and early
peeks are "INFORMATION ONLY"). The PLAN.md column **T** keeps its 25 Oct letters (A exact, B statistics, C
reliability); the R-649 tier goes in **Status**.

### 10.1 PLAN.md § 0, the abbreviation table: add after the `HP27` row

```
| `RB29` | `../2026-10-29_research_batches/` | The owner's three 29 Oct research batches and papers-2: transcriptions (two readings each, diffed), decodings, `sources.tsv`, `fetch.sh` (papers local only, no open licence) |
```

### 10.2 PLAN.md § 3.1 (Verification): add after the REQ-VAL-047 row

```
| KZ-1 (proposed, 29 Oct 2026) | V | A | `RB29/README.md` § 5.5; Kapela & Zgliczyński 2003 (Nonlinearity 16, 1899), p. 1906 and Table 1: `RB29/kapela_zgliczynski_2003/`; cross-references REQ-VAL-041 | The figure-eight start REQ-VAL-041 uses, put in their frame (outer bodies at (±1, 0), middle at 0, unit masses, G = 1, at the collinear instant), has (v, u) inside the certified box [0.347116886243943, 0.347116889993313] × [0.532724941587373, 0.532724949187495], a proven bound; its period lies in the derived (6.24, 6.36] | `RB29/kapela_zgliczynski_2003/table1_reconciled.csv`, `enclosure_check.txt` | Path A | none (arithmetic on the start) for the box; M3 for the period | exact (interval containment) | data transcribed · proposed 29 Oct; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek) |
```

### 10.3 PLAN.md § 3.2 (Replication): add after the SP-2 row and its notes

```
| LI21-1 (proposed, 29 Oct 2026) | R | A | `RB29/README.md` §§ 5.1–5.2; Li, Li, He & Liao 2021 (CeMDA 133, 46), Tables 1–2 and supplementary workbook: `RB29/li2021/` | From each of the 1653 workbook starts (and Table 1's five isosceles rows), equal unit masses at rest, G = 1, the three bodies meet at the barycentre (x/3, y/3) at the workbook's T. The measured quantity is Li et al.'s d(t, q) = \|r(t; q) − f(q)\| (eq. 4, p. 4), the distance of the 6-vector of positions from the collision point f(q) = (x/3, y/3, x/3, y/3, x/3, y/3); for unit masses it is √Σ\|r_i − r_cm\|². It is compared twice: the time of min_t d against the workbook's T, and the size of min_t d against what the start's digits allow (12 to 15 significant digits in the workbook, § 5.2). A start given to finite digits cannot reach d = 0, so the discriminator is a scaling: min_t d along Tanikawa & Mikkola 2021's Table 4 controls, TSM point 1 displaced by 10⁻³, 10⁻⁴ and 10⁻⁵ (§ 5.4), as TM-1 frames it, gives min_t d against the displacement, and each collision start's min_t d is read against that scaling at its own digits. The tolerance of that reading is a calibration requirement (R-71), not set here. No binary collision before T: the closest-pair separation d_min stays above 10⁻⁶ (the authors' statement, p. 6). The authors' level is d(T, q) < 10⁻⁸ (CNS, tolerance 10⁻⁶⁰): their level, not a bound. Table 1's isosceles rows (p. 6) differ in precision (`RB29/li2021/table1_precision.txt`): IT-0 is Lagrange's solution, exact y = √3/2, and p. 5 gives "the precision of our numerical result is about 10⁻¹¹" (the printed y is √3/2 − 6.45×10⁻¹¹); IT-1 and IT-2 print 17 and 16 significant figures; IT-3's y prints as 2.3930000000000000e−02, which is 0.02393, 4 significant figures, and IT-4's as 8.8999999999999999e−03, the binary double nearest 0.0089, 2 significant figures. IT-3 and IT-4 are therefore compared at their own digits, not at the authors' 10⁻⁸ level. The Table 4 starts are the negative controls: each must read as not a collision on the scaling, which shows the measurement can fire before a workbook row is read | `RB29/li2021/li2021_xlsx_ics.csv`, `table1_reconciled.csv`, `table1_precision.txt`; `RB29/tanikawa_mikkola_2021/table4_reconciled.csv` | Anosova entry (same D, no mirror: `RB29/README.md` § 5.1) | regularisation; a triple-collision measure (Li's d(t, q): its minimum and the time of it) beside the closest-pair d_min | exact (event time; min_t d scaling) | data transcribed · proposed 29 Oct; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek) |
| LI25-1 (proposed, 29 Oct 2026) | R | A | `RB29/README.md` § 5.3; Li, Dong, Li & He 2025 (Ap&SS 370, 99), Table 1: `RB29/li2025/` | The 8 unequal-mass starts (m3 = 1; m1, m2 as printed) reach triple collision at the printed Tc, at the mass-weighted barycentre, measured by Li 2025's d(Tc, q) (eq. 5, p. 3), as LI21-1. The authors stop their Newton iteration at d(Tc, q) < 10⁻⁶: their level, not a bound | `RB29/li2025/table1_reconciled.csv` | Anosova entry at a mass point | as LI21-1; mass-simplex chart (TASK-M2-13) | exact | data transcribed · proposed 29 Oct; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek) |
| TM23-1 (proposed, 29 Oct 2026) | R | A | `RB29/README.md` § 5.4; Tanikawa & Mikkola 2023 (CeMDA 135, 54), § 2, § 6.1 and supplement Tables 2–8: `RB29/tanikawa_mikkola_2023/` | The 432 starts with non-zero velocities, decoded by eqs. 20–23 (§ 6.1's form for Table 8), reach triple collision; positions to the 7 printed decimals | `RB29/tanikawa_mikkola_2023/supp_reconciled.csv`, `decode_check.txt` | Path A (velocities are not a chart axis) | as LI21-1 | exact | data transcribed · proposed 29 Oct; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek) |
| T2000-1 (proposed, 29 Oct 2026) | R | A | `RB29/README.md` § 5.6; Tanikawa 2000 (CeMDA 76, 157), Tables I–VIII: `RB29/tanikawa_2000/` | Positions only: Tables I–IV's points are triple-collision starts and V–VIII's binary-collision starts of the stated type at the n_min-th encounter, each to its printed decimals; the sequences accumulate at the stated boundaries. The time columns wait for RQ-640 | `RB29/tanikawa_2000/reconciled/` | Anosova entry | as LI21-1 for Tables I–IV; for V–VIII, the closest-pair d_min of the stated pair at the n_min-th encounter | exact (positions) | data transcribed · proposed 29 Oct; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek); times blocked: RQ-640 |
| SCH-1 (proposed, 29 Oct 2026) | R | A | `RB29/README.md` § 5.7; Schubart 1956 (AN 283, 17), Table 3 and pp. 21–22: `RB29/schubart_1956/`; beside HE-1 and Titov's Table 10 (task 15) | Collinear closure: from q = (−0.614745, 0, 0.614745), v = (−0.528426, 1.056852, −0.528426), unit masses, G = 1 (E = −3.229025), the orbit returns after T = 0.85880 (at E = −1: q3 = 1.985027, v2 = 0.588137, T = 4.98309). Tolerance from the table's digits, not a bound: one unit in the 5th decimal (t* ± 5×10⁻⁶, so T ± 2×10⁻⁵; r13 max = 1.22949 ± 5×10⁻⁶); the 23 table rows as intermediate states | `RB29/schubart_1956/table3_reconciled.csv`, `decode_check.txt` | Path A | regularisation (binary collisions every half period) | exact (to the table's digits) | data transcribed · proposed 29 Oct; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek) |
| HE-1 (proposed, 29 Oct 2026) | R | A | `RB29/README.md` § 5.8; Hénon 1977 (Cel. Mech. 15, 243), Tables I–IV, positive-mass rows: `RB29/henon_1977/` | Each of the 81 collinear interplay orbits (G = 1, Σm = 1, E = −½ Σ m_i m_j) closes after its printed T from (x1, x3, u1 = u3); where a stability diagnostic exists, k1 and k2 as printed. Hénon states his critical orbits to "the order of 10⁻⁸"; the tables print 6 decimals | `RB29/henon_1977/table{1..4}_reconciled.csv` | Path A | regularisation | exact | data transcribed · proposed 29 Oct; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek) |
| MH91-1 (proposed, 29 Oct 2026) | R | A | `RB29/README.md` § 5.8; Mikkola & Hietarinta 1991 (CeMDA 51, 379), Table I: `RB29/mikkola_hietarinta_1991/` | The 15 Schubart-family orbits (masses from (Ia, Ib), Σm = 3, E = −1) close; the period is P R^{3/2} (decoded, § 5.8); k_l, k_t as printed. The Ib ≠ 0 rows (unequal outer masses) are beyond HE-1 | `RB29/mikkola_hietarinta_1991/tableI_reconciled.csv` | Path A | regularisation | exact | data transcribed · proposed 29 Oct; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek) |
| OR02-1 (proposed, 29 Oct 2026) | R | B | `RB29/README.md` § 5.9; Orlov, Petrova & Martynova 2002 (Astron. Rep. 46, 932), Table 1: `RB29/orlov_2002/` | Over their grid (equal masses, h = −1; k = 0.01(0.01)0.99 × φ = 1°(1°)360°), the outcome fractions: ejections 0.599 (left 0.308, right 0.291), departures 0.370 (conditional 0.015, precise 0.355), undisrupted 0.026, triple close 0.005; under their stop (Marchal–Yoshida escape, R > 30, or t = 100) | `RB29/orlov_2002/table1_reconciled.csv` | new: a collinear (k, φ) start (none planned) | outcome classifier, collinear | distribution | data transcribed · proposed 29 Oct; R-649 tier (a) GPU explorer; scores in M9 (or as an information-only peek); blocked: no collinear chart |
| OR03-1 (proposed, 29 Oct 2026) | R | B | `RB29/README.md` § 5.9; Orlov, Petrova & Martynova 2003 (Astron. Rep. 47, 254), its table: `RB29/orlov_2003/` | Unequal-mass collinear outcome fractions by (ε1, ε2) and n, over their virial-coefficient × velocity-angle grid | `RB29/orlov_2003/table_reconciled.csv` | as OR02-1 | as OR02-1 | distribution | data transcribed · proposed 29 Oct; R-649 tier (a) GPU explorer; scores in M9 (or as an information-only peek); blocked: as OR02-1 |
| MH89-1 (proposed, 29 Oct 2026) | R | B | `RB29/README.md` § 5.9; Mikkola & Hietarinta 1989 (CeMDA 46, 1), Tables 1–4: `RB29/mikkola_hietarinta_1989/` | Equal-mass collinear, E = −1, the 100 × 180 (R, θ) grid of eq. 7: escape percentages by z (Table 2) and the lifetime distribution (Table 3), 17,670 break-ups, under their break-up criterion | `RB29/mikkola_hietarinta_1989/table{1..4}_reconciled.csv` | as OR02-1 | as OR02-1 | distribution | data transcribed · proposed 29 Oct; R-649 tier (a) GPU explorer; scores in M9 (or as an information-only peek); blocked: as OR02-1 |
| MO14-1 (proposed, 29 Oct 2026) | R | B | `RB29/README.md` § 5.9; Martynova & Orlov 2014 (Astron. Rep. 58, 756), pp. 756–764: `RB29/martynova_orlov_2014/` | Free fall over D at step 0.001, boundaries excluded: the decay-time distribution's peaks for T < 30 T_cr (Figs. 3–8) under Standish's criterion; the 26 survivors of 306,346 at 10⁷ T_cr and α = 1.74 over 10⁵–10⁶ T_cr are past R-610's cap of 200 | `RB29/martynova_orlov_2014/summary_captions_reconciled.csv` (figures otherwise) | Anosova entry | outcome classifier (Standish) | distribution (qualitative for the peaks) | data transcribed · proposed 29 Oct; R-649 tier (a) GPU explorer; scores in M9 (or as an information-only peek); the long-time part blocked by the horizon (R-610); grid count: RQ-642 |
| HB15-1 (proposed, 29 Oct 2026) | R | C | `RB29/README.md` § 5.10; Hernandez & Bertschinger 2015 (MNRAS 452, 1934), Table 1: `RB29/hernandez_2015/` | Conditional: only for an occupant that is a fixed-step DKD leapfrog. On the Pythagorean problem to t = 2.0 at h = 10⁻⁴: \|ΔE/E0\| = 8.2×10⁻⁶ and ΔL = 1.2×10⁻¹³ | `RB29/hernandez_2015/table1_reconciled.csv` | Path A (Burrau start) | a fixed-step DKD occupant (none planned) | exact (their measurement) | data transcribed · proposed 29 Oct; information only; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek) if such an occupant exists |
| RA20-1 (proposed, 29 Oct 2026) | R | A | `RB29/README.md` § 5.10; Rantala et al. 2020 (MNRAS 492, 4131), § 5.2: `RB29/rantala_2020/`; beside SP-1, SP-2 and P-1 | Burrau's outcome, two figures: M1 (mass 3) escapes at β1 ≈ 71.4° (eq. 32, Aarseth et al. 1994's orientation), the remaining binary has a ≈ 0.55 (Burrau units) and e ≈ 0.99 | `RB29/rantala_2020/section5_2_eye.csv` | BodyPlane Burrau pixel | outcome classifier; a horizon past the escape | exact to two figures | data transcribed · proposed 29 Oct; information only beside SP-2; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek) |
```

And, after the table, this dated note:

```
*Note added 29 Oct 2026 (rows KZ-1, LI21-1, LI25-1, TM23-1, T2000-1, SCH-1, HE-1, MH91-1, OR02-1, OR03-1, MH89-1, MO14-1,
HB15-1, RA20-1; proposed, not ruled, not counted in § 3.3).* The owner's three 29 Oct research batches and papers-2 are
recorded in `RB29/`: every paper is local only (no open licence; `RB29/sources.tsv`), and every table used here is
transcribed twice and diffed (`RB29/README.md` § 4). Each row is a measurement under R-649: PASS, EXPLAINED DIVERGENCE or
UNEXPLAINED, never a gate, with the tier it binds. Like for like: LI21-1, LI25-1, TM23-1 and T2000-1 share one frame
with TSM 2019 and TM-1 (§ 5.1 there: same D, no mirror of positions); the authors' accuracy statements are their
levels, not bounds; SCH-1's tolerance is the table's last digit. The Li 2021 set is new: 0 of its 1653 starts are in
the threebodyorbits snapshot (`OC26`). KZ-1 adds a proven bound beside REQ-VAL-041, and finds that `OC26`'s published
V.1.A start (6 decimals) lies outside it while the refined one lies inside (`RB29/README.md` § 5.5).
```

### 10.4 PLAN.md § 3.2: TM-1 (this changes an existing row, so it needs the orchestrator's decision before it is applied)

TM-1's Status reads "blocked: the paper is not in the repo (rule 1)". Tanikawa & Mikkola 2021 is now cited with its
sha256 in `RB29/sources.tsv` and kept local (Springer, no open licence), and Tables 1–4 are transcribed twice in
`RB29/tanikawa_mikkola_2021/`, as SP-2 and `HP27` meet rule 1 for their sources. If that is accepted, replace TM-1's
Ref. data and Status cells with:

```
| … | Tanikawa & Mikkola 2021, Tables 1, 2, 4: `RB29/tanikawa_mikkola_2021/` (transcribed twice; the paper local only, `RB29/sources.tsv`) | … | data transcribed · proposed; R-649 tier (b) CPU IC inspector; scores in M9 (or as an information-only peek) |
```

and its Chart cell's "after the D11 mirror remap" stays as it is unless the owner confirms § 5.1's finding that, for
TSM 2019 and these papers, D11 is D with no mirror of positions.

### 10.5 REVIEW_QUEUE.md: three new entries, appended after the last open entry

```
## RQ-640: Tanikawa 2000's time unit, for the time columns of its eight tables (proposed T2000-1) *(the owner's request of 29 Oct 2026, "Tanikawa 2000 Collision orbits II (8 tables)"; `docs/incoming/2026-10-29_research_batches/README.md` § 5.6; context: none built)*

*Queued 29 Oct 2026 by the agent ingesting the owner's 29 Oct research batches.*

- **File, section:** `docs/incoming/2026-10-29_research_batches/README.md` § 5.6; Tanikawa 2000, CeMDA 76, 157, p. 157:
  "m1 = m2 = m3", with no mass value, no G and no time unit stated in the paper.
- **What is settled without it:** the positions. Equal-mass free fall is scale-free in position, the frame is TSM 2019's
  (§ 5.1), and the Fig. 11 point agrees with Li et al. 2021's GT-36 to 1.06e-6.
- **What is not:** the 'time' columns of Tables I–VIII. No Tanikawa 2000 point coincides with a Li 2021 orbit (closest
  1.1e-2), so the unit cannot be fixed from a shared orbit; and Table I's times (0.39–0.49) are short beside Li 2021's T
  for nearby orbits (6.9–11.3), so the unit or the quantity may differ.
- **Options seen:** (a) read the normalisation from Paper I (Tanikawa, Umehara & Abe 1995, task 15 item 16) and decode;
  (b) compare positions only, permanently; (c) ask the author.
- **Applied meanwhile:** (b) for now. T2000-1 compares positions; its times wait.

## RQ-641: Broucke, Elipe & Riaguas 2006's rotating Jacobi variables cannot be decoded from the paper *(the owner's request of 29 Oct 2026, "Broucke 2006 (rotating Jacobi variables)"; `docs/incoming/2026-10-29_research_batches/README.md` § 5.11, `broucke_2006/decode_attempt.txt`; context: none built)*

*Queued 29 Oct 2026 by the agent ingesting the owner's 29 Oct research batches. For the owner: (a) is a fetch from
publishers, outside the repo.*

- **File, section:** Broucke et al. 2006, CSF 30, 513, § 3 and Tables 1–3; its refs. [7] (Broucke, Elipe & Riaguas,
  AIAA 2002-4544) and [10] (Broucke & Boggs 1975, Cel. Mech. 11, 13–38), where the variables are defined, are not in
  the repo.
- **What is settled:** the transcription (two readings, all differences resolved against the page); Table 2's y/r
  equals Table 1's y to 1e-8; Table 3's f1, f3 follow from a1, a2.
- **What is not:** the map from (y, ẋ, T) to an inertial state. Read as G = 1 with masses 1/3, both Table 1 states have
  E > 0 (unbound); with masses 1, T|E|^{3/2} is 4.08 and 3.45, not the figure-eight's 9.2377; no single G fits both rows.
- **Options seen:** (a) the owner fetches refs. [7] and [10] into `docs/incoming/_local/`, and the decoding is redone;
  (b) leave Broucke 2006 as context.
- **Applied meanwhile:** (b). No target is proposed from it.

## RQ-642: Martynova & Orlov 2014's grid count: 306,346 printed, 306,402 counted *(the owner's request of 29 Oct 2026, ChatGPT's `Martynova_Orlov_2014_figure_initial_conditions.json` against the paper; `docs/incoming/2026-10-29_research_batches/README.md` § 5.9; context: proposed MO14-1)*

*Queued 29 Oct 2026 by the agent ingesting the owner's 29 Oct research batches.*

- **File, section:** Martynova & Orlov 2014, Astron. Rep. 58, 756, p. 760: "We scanned the inner part of the domain D
  in both coordinates with steps Δξ = Δη = 0.001 (the boundaries were not included). In all, we considered 306,346
  versions of the initial conditions. In 306,320 of these cases … decay … In the 26 remaining cases …"; D as bounded
  by the axes and the unit arc about (−0.5, 0) (p. 758).
- **What is settled:** 306,320 + 26 = 306,346, as printed.
- **What is not:** an exact integer count of the grid points strictly inside D at step 0.001 gives 306,402 (306,406
  with the arc included), 56 (60) more than printed (`martynova_orlov_2014/checks.txt`). Which points they left out,
  or whether their grid was offset, the paper does not say. A like-for-like survivor count needs the same grid.
- **Options seen:** (a) compare fractions, 26/306,346 against ours over 306,402, and record the 56 as a stated
  difference; (b) ask the authors which points were dropped; (c) treat the count as UNEXPLAINED in the score.
- **Applied meanwhile:** (a), with the difference stated beside the score.
```

## 11. Files

- `INVENTORY.tsv`: every file of the three batches and papers-2 (including the zips' entries), with size, sha256, role
  and disposition.
- `sources.tsv`, `fetch.sh`: the papers kept local, their hashes, verified licence evidence and how to get each back.
- `doi_check.tsv`, `dois.txt`, `arxiv_licences.tsv`: the citation and licence checks.
- One folder per source (§ 3's table): `*_eye.csv` reading B, `*_ocr*` the second reading, `*_chatgpt_A.csv` reading
  A where one exists, `*_diff*.txt` the diffs (each resolution in the file or in § 4), `*_reconciled.csv` the page's
  values, and `checks.txt` or similar for the arithmetic.
- `scripts/`: our scripts, run with `python3 -I`; `ocr.swift` is the OCR helper (Apple Vision), `render.sh` and
  `crop.sh` render page images with `pdftoppm`.
