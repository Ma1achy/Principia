# External sources from the 28 Oct ChatGPT/Codex search, and the owner's 29 Oct additions: a record

A record, not authority (R-1, R-159). It ingests and transcribes; it scores nothing against Principia and runs no engine
code. Every number committed as ours comes from our own reading of the source, diffed against a second reading. Every
target it proposes is a measurement, never a precondition for building anything (R-649, R-650, the owner, 29 Oct 2026),
and none is applied here: the proposed PLAN.md rows and review-queue entries are in § "To apply once #328 and #335
merge", for a porter.

Contents:

- [The requests](#the-requests-verbatim)
- [Method](#method)
- [Layout](#layout)
- [Sources and licences](#sources-and-licences)
- [Findings, source by source](#findings-source-by-source)
- [Corrections and contradictions found in the sources and in the repo](#corrections-and-contradictions)
- [To apply once #328 and #335 merge](#to-apply-once-328-and-335-merge)
- [Disclosures](#disclosures)

## The requests, verbatim

The owner, 28 Oct 2026:

> New sources from a ChatGPT/Codex search, on this Mac: /Users/malachy/Documents/Codex/2026-10-28/i-m-validating-a-high-precision/outputs/ (kapela_simo_comparison_package.zip, planar_three_body_research_complete.zip, and the two unzipped folders). Ingest as docs/incoming/2026-10-28_external_sources/ (a record, not authority), on its own branch and PR, after the current queue work, as a local agent.
> Priorities: (1) Tanikawa, Saito & Mikkola 2019, CeMDA 131, 24, doi:10.1007/s10569-019-9902-8: Table 1, 11 free-fall triple-collision configurations; transcribe twice yourself and diff against ChatGPT's Tanikawa_Saito_Mikkola_2019_Table1.json; propose as a target beside the Tanikawa 2008/2015 ones. (2) Janković, Dmitrašinović & Šuvakov 2020, CPC 250, 107052, doi:10.1016/j.cpc.2019.107052: treat ChatGPT's 99-record transcription as reading A, do reading B yourself from the PDF, diff; re-run its overlap audit against docs/incoming/2026-10-26_orbit_catalogues/ (claim: 42 not in threebodyorbits.com). (3) Kapela & Simó 2007 (Nonlinearity 20, 1241) and 2017 (Nonlinearity 30, 965; arXiv:1105.3235): keep raw archives, re-check the Cartesian conversion independently; label the return times "uncertified, computed by ChatGPT". (4) Record but don't build targets from Kapela & Zgliczyński 2003, Adler 2016 (CC BY-SA 3.0, ~6 digits), Peter 2025 (Zenodo 10.5281/zenodo.17635887, MIT, double precision only), Quarles 2020 (CC BY 4.0 Zenodo 3579202, finite-radius stars, out of scope).
> Licences: commit only openly licensed files (Peter MIT; Adler CC BY-SA with attribution; Kapela choreo.zip GPL v2 code); the Janković PDF (© Elsevier) and the Kapela results archives (no licence) go to docs/incoming/_local/. Transcribed numbers with citations may be committed.
> Correction to record: Orlov, Titov & Shombina 2016 (Astron. Rep. 60, 1083, doi:10.1134/S1063772916110056) is "Periodic orbits in the free-fall three-body problem", not free fall with rotation; subscription-only, unchecked.
> Also queue Portegies Zwart & Boekholt 2018 (from my earlier message) in the same record.

The owner, 28 Oct 2026 (late), on Tanikawa, Saito & Mikkola 2019:

> Tanikawa, Saito & Mikkola 2019 (CeMDA 131, 24, doi:10.1007/s10569-019-9902-8): I have the accepted manuscript (post-review, pre-copyedit, no open licence; sha256 aea7fde764ef23edefba173a8c4ca8af977b2e8fc559d5abd2ec6f6e3b8f7859, in my Downloads as Tanikawa_Saito_Mikkola.pdf). Put it in docs/incoming/_local/. Table 1 (manuscript p.24): 11 triple-collision initial points; the claude.ai session's text extraction matches ChatGPT's transcription on all 11 rows, but both come from the same text layer, so do your own second reading from the rendered page (R-594 rule 4) before committing the CSV. Set-up (§3): m1=m2=m3=1, body 2 at (−0.5,0), body 3 at (0.5,0), body 1 at the tabulated (x,y), zero velocities (G=1 assumed; check the paper's units statement). t is the practical integration endpoint near triple collision, not an exact collision time: score as "approaches triple collision near t, with the stated symbol-sequence digits", not as an exact time. Note: p.25 says the orientation is "negative" in both the even- and odd-digit cases; flag it as a contradiction in the paper and build nothing on it. Propose as a target beside the Tanikawa 2008/2015 ones, in the external-sources ingest.

The owner, 29 Oct 2026, the expanded package:

> More sources from ChatGPT, on this Mac: /Users/malachy/Documents/Codex/2026-10-28/i-m-validating-a-high-precision/outputs/planar_three_body_research_expanded(.zip). Add to the external-sources ingest (task 15) AFTER the PR backlog is drained; don't start now.
> Priorities: (1) Dandekar, Springer, Zard, Trani, Kol & Birnholtz 2026, arXiv:2608.30414: planar escape statistics, 8 mass sets, nonzero L; verify the arXiv licence yourself (reported CC BY 4.0); re-transcribe Tables 1-4 and B.1-B.3 as reading B against ChatGPT's reading A and diff; convert G=4π² to G=1 independently; propose as statistical targets beside MK-1/AN-1/SL-1, noting their ~1e-5 energy monitoring. (2) Dandekar & Kol 2025, CeMDA 137, 32, doi:10.1007/s10569-025-10262-2 (CC BY 4.0): Table 2's eight planar phase-volume rows, same transcribe-twice rule; pair with (1). (3) Mikkola & Merritt 2008, AJ 135, 2398: N=3 benchmark (masses 1, 1e-3, 1e-4, from rest, t=1000); re-derive the planar rotation yourself; integrator stress test, not a replication target. (4) Mel'nikov, Orlov & Shevchenko 2013, Astron. Rep. 57, 429: the six T_d vs T_L fits, beside MT-1. (5) Record only: Rose & Dullin 2013, Johnstone & Rucinski 1991, Nauenberg 2001, Rein & Spiegel 2015, Dandekar et al. 2022. Licences: commit only CC-licensed PDFs; others to docs/incoming/_local/. Orlov, Titov & Shombina 2016 remains unobtained.

The owner, 29 Oct 2026, on Orlov, Titov & Shombina 2016 (superseding "unobtained" above):

> Orlov, Titov & Shombina 2016 (Astron. Rep. 60, 1083; doi:10.1134/S1063772916110056) is now obtained — I'll give you the PDF from my Downloads. © Pleiades: put it in docs/incoming/_local/, do not commit. Record it as context only: no tables, no orbit ICs or periods; Figs 3–4 are colour maps of closeness of return to the Hill surface (cube-root scaled) over (θ,φ) ∈ [0,π/2]×[0,π/3], runs stopped after three pairwise approaches (not a lifetime/outcome map); S-orbit regions near θ=0, isosceles near θ=π/2. At most a qualitative visual comparison for the free-fall maps. Mark it resolved in the unobtained list. Do this with task 15, after the PR backlog.

The owner, 29 Oct 2026, two messages on Titov 2024 and Li & Liao 2018:

> [1] New source: V. B. Titov, DSc dissertation "Общая задача трех тел в пространстве форм", SPbU 2024, https://disser.spbu.ru/files/2024/disser_titov.pdf (177 pp). No licence stated: docs/incoming/_local/, transcriptions with citation may be committed. (1) Appendix B: 33 periodic orbits with barycentric ICs (x, y, ẋ, ẏ per body, 7 decimals) plus A, E, |J|, ω; masses 0.95/0.95/1.1, 1.05/1.05/0.9, 0.99/1.01/1.0 and the Table 4 m1 sweep; nonzero L, unequal masses. Transcribe twice (reading A/B, diff). They are Fourier-fit solutions periodic in a frame rotating at ω, so the closure test is state(T) vs state(0) rotated by ωT; expect closure only to ~1e-7, a tolerance not a bound. Independently verify E and J from the ICs before using any row. (2) Tables 10 (collinear, orbits 5–10 free-fall) and 11 (isosceles, orbits 8–9 brake orbits): ρ0/E0 and periods in regularised time, 5–6 digits; decode the chapter 5 regularisation and time convention before comparing. (3) Check Li & Liao 2018, arXiv:1805.07980 (234 free-fall periodic orbits, Tables A1–A5) against the existing threebodyorbits exclusion before treating it as new. Orlov, Titov & Shombina 2016: obtained, no numerical data, context only.
>
> [2] Two PDFs from me (in my Downloads): disser_titov.pdf and 1805.07980v1.pdf. Ingest with task 15, after the PR backlog. Titov: as in my previous note (no licence → _local; transcribe Appendix B's 33 ICs + Tables 10/11 twice). Li & Liao, "Collisionless periodic orbits in the free-fall three-body problem", arXiv:1805.07980v1: check the arXiv licence (commit only if CC) and the journal ref (believed New Astronomy 2019; verify). Tables A1–A5: 234 free-fall periodic orbits, r1=(−0.5,0), r2=(0.5,0), r3=(x,y), all at rest, G=1; x, y, T to 10 digits, return proximity <1e-6 via CNS; A6–A10 are free-group words. Same frame as Tanikawa–Saito–Mikkola, so each orbit is a point on the free-fall maps. Transcribe twice and diff. Before proposing a target, check against the threebodyorbits exclusion (Li–Liao free-fall sets may already be there). If new, propose as a closure test: integrate each IC to T and check ||y(T)−y(0)||, with 1e-6 as the authors' stated level, not a bound.

The owner, 29 Oct 2026, the Springer batch:

> Springer batch from me: ~/Downloads/springer (README.md + download_manifest.json + checksums.sha256 inside; ignore t.txt, my scratch). Task 15, after the PR backlog. All © Springer except Pasechnik 2023 Part II (CC BY 4.0) → _local unless CC. Priority: Tanikawa, Umehara & Abe 1995, CeMDA 62, 335, doi:10.1007/BF00692285 — free-fall collision points: Table IV (first 20 triple-collision ICs, x,y to 7 digits, vertex angle), Tables IIa/b, IIIa–c (binary-collision ICs with collision time, Δx/Δy accuracy 1e-7–1e-8). Scanned 1995 PDF with bad OCR: transcribe twice from the page images, diff. Confirm their frame against Tanikawa–Saito–Mikkola 2019 before use. Propose as collision targets: integrate each IC, check d_min→0 at the stated time for the stated pair. Secondary: Anosova 1986 (Tables V, X: escape-time distribution, prograde/retrograde fractions; coarse, statistical only); Umehara & Tanikawa 2000 (check for tables, likely context). Context only: Saito & Tanikawa 2007/2009 (rectilinear), Aarseth & Zare 1974 and Heggie 1974 (integrator originals; skip if already in repo). Pasechnik 2023 Part II is 3D: check whether it duplicates the Pasechnik 2023 already committed in #327.

The owner's 29 Oct rulings on targets (R-649, R-650; quoted in the orchestrator's dispatch) govern every proposed row
below: each target is a measurement with a tier, (a) GPU explorer (statistical or qualitative) or (b) CPU IC inspector
(exact: collision points, periods, converged states); outcomes are PASS, EXPLAINED DIVERGENCE or UNEXPLAINED; the
comparison must be like for like (frame, units, masses, event definition, the source's stated accuracy); scoring waits
for M9, except information-only peeks.

## Method

- **Untrusted input.** Every input (ChatGPT's packages and the owner's downloads) was copied into a new, empty scratch
  directory, the zips unzipped there with `unzip -n` after a path-traversal check (no `..`, no absolute paths), and
  read only as data. No script from the packages was run; where an audit had to be re-run, it was re-implemented in
  `scripts/` and run with `python3 -I`, paths as arguments. The owner's original folders were not modified. The
  Springer batch's `checksums.sha256` was re-run on our copies: all eight PDFs match.
- **Two readings of every table.** Reading A is a text layer (ours via `pdftotext`, or ChatGPT's JSON where it had
  one). Reading B is from the rendered page image only: Apple Vision OCR (`scripts/ocr.swift`, run on 300–600 dpi
  renders, sometimes three passes with a two-of-three consensus) plus reading crops by eye, the eye fixes listed in each
  folder's `*_eye_fixes.csv`. For the 1986 and 1995 scans, which have no usable text layer, reading A is OCR and
  reading B is by eye. `scripts/diff_readings.py` compares cell by cell as strings (and as numbers where stated); every
  disagreement was resolved against the page image, and the reconciled CSV carries the page's value.
- **Recomputation.** Wherever a source prints a quantity derived from its own inputs (E, L, J, a unit conversion, a
  rotation), it was recomputed with our own script (`mpmath` at 50 digits, or `sympy`) and every disagreement is
  reported below. These are checks of the sources, not of Principia.
- **Licences** were verified from the file itself or its source page (arXiv abstract pages, the Zenodo and Crossref
  APIs, the Stack Exchange API), never from ChatGPT's claim. Anything unclear went to `docs/incoming/_local/`.
- **Network.** One request at a time, with pauses (16 s for arXiv, whose robots.txt asks for 15), respecting
  robots.txt, never getting round a paywall: ScienceDirect and OUP answered 403 and were left there. See
  [Disclosures](#disclosures) for one mistake in the requests' User-Agent.

## Layout

| Path | What |
|---|---|
| `INVENTORY.tsv` | every file in the three zips, the three unzipped folders and the owner's two download batches: path, size, sha256, kind, claimed source, licence (verified or not), disposition, zip-vs-folder equality |
| `sources.tsv`, `fetch.sh` | every source file used: where it lives (committed or `_local/`), size, sha256, origin, URL, whether `fetch.sh` can re-fetch it; `fetch.sh --check` verifies what is present |
| `scripts/` | our own scripts; each states its usage, inputs and conventions in its docstring |
| `<source>/` | per source: `*_reading_a_*.csv`, `*_reading_b*.csv`, `*_diff.txt`, `*_reconciled.csv`, and checks |
| `docs/incoming/_local/…` (gitignored by `.gitignore:21`) | the PDFs and archives that may not be committed; listed in `sources.tsv` |

**The packages against their folders** (`scripts/inventory.py`): the Kapela–Simó zip (25 files) equals its folder, which
adds only a `.DS_Store`; the complete package's zip (80 files) equals its folder except the root `.DS_Store`, which
differs in bytes; the expanded package's zip (131 files) equals its folder exactly, and its `previous_verified_bundle/`
(80 files) is byte-identical to the complete package.

## Sources and licences

| Source | Licence (how verified) | Committed or local-only | What we did | Result |
|---|---|---|---|---|
| Tanikawa, Saito & Mikkola 2019, CeMDA 131, 24 (accepted manuscript) | none (accepted manuscript; the owner's copy) | local-only: `_local/tanikawa_saito_mikkola_2019/` (sha256 aea7fde7… checked) | Table 1, two readings | 0 disagreements in 77 cells; proposed TSM-1 |
| Janković, Dmitrašinović & Šuvakov 2020, CPC 250, 107052 | © 2019 Elsevier B.V., "All rights reserved" (in the PDF) | local-only | Tables 3–6, two readings; Cartesian conversion; overlap audit | 0 disagreements in 693 cells; the claim "42 not in threebodyorbits.com" is not confirmed as worded: 42 are absent from the Belgrade gallery (three-body.ipb.ac.rs), and all 99 are absent from the threebodyorbits.com snapshot (OC26) |
| Kapela & Simó 2007, Nonlinearity 20, 1241 (author preprint, `kamstab.pdf`) | none stated (none in the PDF) | local-only | conversion check | see below |
| Kapela & Simó 2017, Nonlinearity 30, 965 (author preprint, `nosym.pdf`; arXiv:1105.3235) | arXiv nonexclusive-distrib 1.0 (abstract page) | local-only | conversion check | see below |
| `choreo.zip` (Kapela & Simó 2017 code) | GPL v2 (root `license.txt`; 480 of 846 files carry the CAPD GPL notice; Kapela's `private/` files say only "Copyright (C) Tomasz Kapela 2006") | local-only: `_local/external_sources_2026-10-28/kapela_simo/choreo.zip` (1,231,363 bytes, sha256 2291f82141a993e5840835d0525d91b36071558ffde155b205ab9bb4eec127d7; from https://ww2.ii.uj.edu.pl/~kapela/papers/nosym/choreo.zip, which `fetch.sh` re-fetches). GPL v2 is not a licence the owner allows for commits; see the note below the table | read only; not built | — |
| `KAMstability.tar.gz`, `results.zip` (Kapela & Simó) | none (no licence file in either archive) | local-only | raw logs compared | byte-identical to the package's `raw/` copies |
| Kapela & Zgliczyński 2003, Nonlinearity 16, 1899 (author preprint) | none stated | local-only | pointer only | moved to task 15b (its certified figure-eight enclosure) |
| Adler 2016, Space Exploration Stack Exchange answer 15369 | CC BY-SA 3.0 (Stack Exchange API: Mark Adler, posted 2016-05-22, edited 2016-05-23) | **committed**: our transcription, `record_only/adler_2016/`, with attribution | 15 rows, two readings | 0 disagreements in 105 cells |
| Peter 2025, Zenodo 10.5281/zenodo.17635887 | MIT (Zenodo API `mit-license`; all 13 files' md5 match the record) | neither: `fetch.sh` retrieves it (applied per R-369: record only, so not kept) | record | double precision only; no target |
| Quarles et al. 2020, Zenodo 3579202 and arXiv:1912.11019v1 | data CC BY 4.0 (Zenodo API); paper arXiv nonexclusive-distrib 1.0 | paper local-only; data not kept | record | finite-radius stars, out of scope; no target |
| Dandekar, Springer, Zard, Trani, Kol & Birnholtz 2026, arXiv:2608.30414v1 | CC BY 4.0 (arXiv abstract page) | **committed**: `dandekar_2026/Dandekar_et_al_2026_arXiv_2608.30414.pdf` | Tables 1–4, B.1–B.3, two readings; G conversion; Table 1 recomputed | 3 disagreements in 752 cells, all resolved for the page; proposed DS-1, DS-2 |
| Dandekar & Kol 2025, CeMDA 137, 32, doi:10.1007/s10569-025-10262-2 | CC BY 4.0 (in the PDF: "Open Access This article is licensed under a Creative Commons Attribution 4.0 International License") | **committed**: `dandekar_kol_2025/Dandekar_Kol_2025_CeMDA_137_32.pdf`; its supplement (no licence stated) local-only | Table 2, two readings | 0 disagreements in 40 cells; paired with DS-1 |
| Mikkola & Merritt 2008, AJ 135, 2398 (arXiv:0709.3367v2) | arXiv nonexclusive-distrib 1.0 | local-only | Table 1 (N = 3) and Tables 2–3 (N_B = 3), two readings; rotation re-derived | 0 disagreements in 36 cells; stress test, no row |
| Mel'nikov, Orlov & Shevchenko 2013, Astron. Rep. 57, 429, doi:10.1134/S1063772913060048 | © Pleiades (in the PDF) | local-only (the owner's copy, sha256 59e4cc19… = his batch's SHA256SUMS) | fit table, two readings | 0 disagreements in 36 cells; proposed MOS-1 |
| Rose & Dullin 2013, CeMDA 117, 169 (arXiv:1302.0912v4) | arXiv nonexclusive-distrib 1.0 | local-only | record | no target |
| Johnstone & Rucinski 1991, PASP 103, 359, doi:10.1086/132828 | none found in the PDF (scanned) | local-only | record | no target |
| Nauenberg 2001, Phys. Lett. A 292 (arXiv:nlin/0112003v2) | arXiv assumed-1991-2003 | local-only | record | no target |
| Rein & Spiegel 2015, MNRAS 446, 1424 (arXiv:1409.4779v2) | arXiv nonexclusive-distrib 1.0 | local-only | record | no target |
| Dandekar et al. 2022, CeMDA 134, 55 (arXiv:2205.04294v1) | arXiv nonexclusive-distrib 1.0 | local-only | record | no target |
| Orlov, Titov & Shombina 2016, Astron. Rep. 60, 1083, doi:10.1134/S1063772916110056 | © Pleiades | local-only: `_local/orlov_titov_shombina_2016/` (sha256 1a872913… checked) | context | **resolved: obtained 29 Oct 2026, local-only, context only** |
| Titov 2024, DSc dissertation, SPbU | none stated | local-only: `_local/titov_2024/` (sha256 7d281adf… checked) | Appendix B, Tables 2–4, 10, 11, two readings; E and J recomputed; chapter 5 decoded | 0 disagreements in 842 cells; proposed TI-1 (23 orbits) |
| Li & Liao 2018, arXiv:1805.07980v1; New Astronomy 70, 22–26 (2019), doi:10.1016/j.newast.2019.01.003 | arXiv nonexclusive-distrib 1.0; journal: Elsevier (Crossref lists only Elsevier's TDM licence) | local-only: `_local/li_liao_2018/` (sha256 cce9e000… checked) | Tables A1–A5, two readings; overlap with OC26 and with LL-5's table | 0 disagreements in 1,872 cells; 0 new orbits; no new row |
| Tanikawa, Umehara & Abe 1995, CeMDA 62, 335, doi:10.1007/BF00692285 | © 1995 Kluwer (in the PDF) | local-only | Tables IIa, IIb, IIIa–c, IV, two readings; units and frame decoded | proposed TUA-1, TUA-2 |
| Anosova 1986, Ap&SS 124, 217, doi:10.1007/BF00656037 | © 1986 D. Reidel (in the PDF) | local-only | Tables V and X, two readings | 0 disagreements; proposed AN-2, contingent on RQ-651 |
| Umehara & Tanikawa 2000, CeMDA 76, 187, doi:10.1023/A:1008305928852 | © 2000 Kluwer | local-only | checked for tables | no tables; context |
| Saito & Tanikawa 2007, CeMDA 98, 95, doi:10.1007/s10569-007-9070-0 | © Springer Science+Business Media B.V. 2007 | local-only | context | rectilinear; no target |
| Saito & Tanikawa 2009, CeMDA 103, 191, doi:10.1007/s10569-008-9175-0 | © Springer Science+Business Media B.V. 2009 | local-only | context | rectilinear; no target |
| Aarseth & Zare 1974; Heggie 1974 | © 1974 D. Reidel | not copied: already in the repo's record | checked | see below |
| Pasechnik, Mylläri, Valtonen & Mikkola 2023, MNRAS 525, 1929, doi:10.1093/mnras/stad2372 | CC BY 4.0 (Crossref, version of record, from 2023-08-04; the PDF: "© 2023 The Author(s)") | not committed: a duplicate of #327's committed copy | compared | byte-identical duplicate |
| Portegies Zwart & Boekholt 2018, CNSNS 61, 160 | — | already ingested in PR #332: `../2026-10-29_burrau_pzb2018/` | pointer | proposed SP-2; RQ-611 |

**Why `choreo.zip` is local-only.** The owner's 28 Oct request (quoted above) listed it among the files to commit. Two
later standing rules of the owner's govern instead: newly fetched material is committed only under CC BY, CC BY-SA, CC0
or public domain, everything else staying under the gitignored `docs/incoming/_local/`; and no GPL code is committed
into Principia. So it is kept local only. This record was first assembled on branch `data/external-sources-2026-10-28`
(head a3b75269) with the archive committed; that branch was never merged, and this record supersedes it, rebuilt
without the archive.

**Licence evidence for the two committed PDFs** (re-checked 29 Oct 2026; both CC BY 4.0, a licence the owner allows):

- *Dandekar, Springer, Zard, Trani, Kol & Birnholtz 2026*, arXiv:2608.30414v1,
  `dandekar_2026/Dandekar_et_al_2026_arXiv_2608.30414.pdf` (sha256 19b4e04b…). The arXiv abstract page
  (https://arxiv.org/abs/2608.30414v1) links its licence as `<a href="http://creativecommons.org/licenses/by/4.0/"
  title="Rights to this article" class="has_license">`, with the icon `by-4.0.png`: CC BY 4.0. The PDF itself states no
  licence; its page header carries the journal template's "Astronomy & Astrophysics manuscript no. main … ©ESO 2026", a
  template mark on a preprint, not a licence statement; arXiv's licence, chosen by the submitter, is the one it is
  distributed under.
- *Dandekar & Kol 2025*, CeMDA 137, 32, doi:10.1007/s10569-025-10262-2,
  `dandekar_kol_2025/Dandekar_Kol_2025_CeMDA_137_32.pdf` (sha256 a9ba19fe…). Crossref's `license` field for the DOI
  (`https://api.crossref.org/works/10.1007/s10569-025-10262-2`) lists two entries, both
  `"URL": "https://creativecommons.org/licenses/by/4.0"`, start 2025-10-01, delay 0 days, `"content-version": "vor"`
  and `"content-version": "tdm"`. The PDF, p. 1: "© The Author(s) 2025"; p. 29: "Open Access This article is licensed
  under a Creative Commons Attribution 4.0 International License, which permits use, sharing, adaptation, distribution
  and reproduction in any medium or format, as long as you give appropriate credit to the original author(s) and the
  source, provide a link to the Creative Commons licence, and indicate if changes were made. … To view a copy of this
  licence, visit http://creativecommons.org/licenses/by/4.0/." Its supplement states no licence and stays local-only.

Both are committed unchanged, with this attribution: the authors and sources as cited above, licence CC BY 4.0
(https://creativecommons.org/licenses/by/4.0/), no changes made to the files.

ChatGPT's own files (READMEs, notes, JSON, scripts, text extractions, saved web pages) are listed in `INVENTORY.tsv` and
not committed: they are untrusted output, and every transcription in them is at most reading A.

## Findings, source by source

### Tanikawa, Saito & Mikkola 2019 (Table 1, 11 triple-collision points)

- **Readings.** A: ChatGPT's JSON and the manuscript's text layer, which agree (one reading). B: OCR of the rendered
  p. 24, with the digits OCR dropped (an `8` in rows 1–3) read by eye. 0 disagreements in 77 cells.
  `tanikawa_saito_mikkola_2019/table1_reconciled.csv`.
- **Set-up (§3), quoted:** "We put body 2 of mass m2 at A(−0.5, 0) and body 3 of mass m3 at B(0.5, 0) both on the
  x-axis of the (x, y)-plane. We put body 1 of mass m1 at any place P in D = {(x, y) : x ≥ 0; y ≥ 0; (x + 0.5)² + y² ≤
  1}." … "We consider the equal mass case, i.e., m1 = m2 = m3 = 1." … "In this figure, D11 corresponds to Anosova's
  region."
- **Units.** The paper states no units sentence. Its eq. (2) writes the potential with G; its eq. (4), the form it
  integrates, is "U = m2 m3/R1 + m1 m3/R2 + m1 m2/R3", without G. So G = 1 is implied, not stated.
- **The contradiction (pp. 25–26), quoted:** "Geometrically, the orientation of the triangle is negative in the former
  case just before the triple collision, whereas the orientation is negative in the latter case." Both cases say
  "negative"; nothing is built on it. The same paragraph cites "Figs. 8, 11, and 11" (one figure twice), and p. 24
  quotes the same symbol sequence for 10(g) and 10(h).
- **Scoring wording** (the owner): t is the practical integration endpoint near triple collision, not an exact
  collision time; score "approaches triple collision near t, with the stated symbol-sequence digits".
- **Proposed:** TSM-1, tier (b).

### Janković, Dmitrašinović & Šuvakov 2020 (Tables 3–6, 99 records)

- **Readings.** A: ChatGPT's 99 records. B: OCR of the page images (81 rows), 16 rows by eye, 2 OCR trailing-dot errors
  fixed by eye (rows 43, 91). 0 disagreements in 693 cells. `jankovic_2020/tables3to6_reconciled.csv`.
- **Units**, pp. 6–7, quoted: "(we take all three masses to equal m = 1, and set the gravitational constant G = 1,
  which sets the units in our system)"; b = 1 (§3.3).
- **Cartesian conversion** (`scripts/jankovic_overlap.py`, 50 digits): ours against ChatGPT's, largest difference
  4.98×10⁻²⁵; eq. (5)'s E against the Cartesian E, 1.3×10⁻⁴⁹.
- **Overlap**, against `docs/incoming/2026-10-26_orbit_catalogues/` (on main) and `../2026-10-26_belgrade_gallery/`.
  Rule 1: a row matches a Belgrade gallery satellite when a and c are equal as decimals and L agrees to the gallery's
  digits: **57 of 99 match, 42 do not** (rows 3, 7, 9, 12, 51–80, 82, 83, 85, 87, 88, 89, 91, 93). The claim's
  count, 42, is the count absent from the Belgrade gallery (three-body.ipb.ac.rs), not from threebodyorbits.com.
  Rule 2: every equal-mass, E < 0 row of the threebodyorbits.com snapshot (2,215 of 3,943) compared by the
  scale-invariant pair (|L||E|^½, T|E|^{3/2}): no candidate at relative tolerance 10⁻⁶, 10⁻⁴ or 10⁻³, for any of the 99;
  |L||E|^½ alone at 10⁻⁴ finds 7 coincidences, none matching in T. So **none of the 99 is in the threebodyorbits.com
  snapshot**, and the claim "42 not in threebodyorbits.com" is not confirmed as worded: all 99 are absent from OC26,
  and 42 are absent from the Belgrade gallery.
- **Row 99** prints L = 0.93555 (5 digits; its neighbours 0.935548917), c positive, k = 58, T = 2.57: transcribed as
  printed. Table 3's caption gives d_min < 10⁻⁷.

### Kapela & Simó 2007 and 2017

- **Raw archives** kept local (no licence); the package's `raw/` logs are byte-identical to the files in the archives.
  `choreo.zip` (GPL v2) is local-only too, under `_local/external_sources_2026-10-28/kapela_simo/` (sha256
  2291f821…), because GPL v2 is not a licence the owner allows for commits; `fetch.sh` re-fetches it.
- **Conversion check** (`scripts/kapela_conversion.py`, `mpmath` at high precision), on all 5 records ChatGPT converted (KS2007 Table 1,
  KS2007 archive stability, KS2017 eight, eight_488, eight_148): **largest disagreement 5.4×10⁻⁹⁰ (KS2017 eight_148,
  vy1)**; E − h about 10⁻⁹¹. The KS2007 reduction follows the preprint's §4: z = (x1, vy1, x2, y2, vx2, vy2),
  (y1, vx1) = (0, 0), body 3 from the zero centre of mass and momentum. ChatGPT's "paper Table 1" nominal point is the
  archive's interval midpoint, not the printed z̄. The archive's `M_2PI` is defined in
  `private/stability/3stability.cpp` (lines 27 and 163) as 3·fl(fl(2π)/3) = 6.2831853071795857879…, re-derived.
- **Return times:** `kapela_simo/return_times_UNCERTIFIED_computed_by_chatgpt.csv`, every row labelled
  **"uncertified, computed by ChatGPT"**. They are a non-interval integration of nominal points, not the authors'
  results; the archive prints no physical return-time interval. Not re-derived here.

### Record only: Kapela & Zgliczyński 2003, Adler 2016, Peter 2025, Quarles 2020

- **Kapela & Zgliczyński 2003**, Nonlinearity 16, 1899: the certified figure-eight enclosure. Moved to task 15b, which
  records it as a priority target: `../2026-10-29_research_batches/README.md` § 5.5 (on branch
  `data/research-batches-2026-10-29`, not yet merged).
- **Adler 2016**, Space Exploration Stack Exchange answer 15369 to question 15364, CC BY-SA 3.0, by Mark Adler: Burrau
  positions at 15 times, ~6 digits. Our reading B parsed the answer's own code block; 0 disagreements in 105 cells
  against ChatGPT's. Committed with attribution as `record_only/adler_2016/positions_reconciled.csv` (licence CC BY-SA
  3.0; changes: transcribed to CSV). No target: ~6 digits, and SP-1 and SP-2 already cover Burrau.
- **Peter 2025**, Zenodo 10.5281/zenodo.17635887, MIT: trajectories of the figure-eight, butterfly I, moth I and
  yin-yang I branch 2a, and notebooks. No target: double precision only.
- **Quarles et al. 2020**, Zenodo 3579202 (CC BY 4.0) and arXiv:1912.11019, "Orbital Stability of Circumstellar
  Planets in Binary Systems". No target: out of scope (the owner: finite-radius stars).

### Dandekar et al. 2026 (planar escape statistics)

- **Readings.** A: ChatGPT's JSON. B: OCR plus 13 eye fixes. 752 cells: **3 disagreements**, all in Table B.2's
  escaper column for the (15, 15, 15) rows, where the page prints "15 (0)", "15 (1)", "15 (2)" and A dropped the
  bracketed label; resolved for the page. 47 more differ only by a lost trailing zero (A stored JSON numbers).
- **Units**, §3.1, p. 5, quoted: "We adopt units such that G = 4π², mass is measured in solar masses (M⊙), and distance
  is measured in astronomical units (AU), implying that time is measured in years. Note that MKTL21 instead adopts
  G = 1, M⊙, and AU."
- **Conversion to G = 1** (`scripts/dandekar2026_units.py`): keep mass (M⊙) and length (AU); then the time unit is
  √(AU³/(G M⊙)), so 1 yr = 2π units. E scales by 1/(4π²) and L by 1/(2π). Table 1 recomputed from eq. (28) matches every
  printed E, L and L0 but one: (8, 21, 21) E, where eq. (28) gives −1873.6457 (rounding to −1873.65) against the
  printed −1873.64. In G = 1 units, for example (15, 15, 15): E = −27.000069, L = 91.856275. Our converted Table 1
  equals ChatGPT's to 0 relative difference. Table 2's fit range 3,000 yr to 10⁶ yr is 18,849.6 to 6.28×10⁶ G = 1
  time units.
- **Accuracy**, p. 6, quoted: "The relative error in total energy (E) is monitored periodically at every crossing time
  τcr for the system throughout the simulation to ensure that it is bounded by 10−5 … the relative error in total
  angular momentum (L) also remains bounded by 10−5." Close encounters and long excursions are advanced analytically
  as two-body motions (p. 6), and "an individual simulation result would not be reliable for the purpose of
  determining the exact trajectories".
- **The ensemble**, §3.1, p. 5, quoted: "We work in the center-of-mass frame and consider an initial configuration
  consisting of a bound binary (ma, mb) in a circular orbit and a distant single body (ms), with both the single body
  and the binary center of mass initially at rest. The lightest body among the three is chosen to be the single body
  (ms). The initial semi-major axis of the binary (a) is set to 5 AU, while the initial separation between the binary
  center of mass and the single body (d) is set to 100 AU." For the planar case, one million runs per mass set, "selecting
  the initial binary phase randomly from a uniform distribution over the interval [0, 2π]", inclination 0.
- **Breakup**, §3.2, p. 6, quoted: "the algorithm first checks whether any two bodies form a bound binary whose apoapsis
  separation is at least ten times smaller than their distance from the single body. If this condition is satisfied,
  the algorithm then checks whether the center of mass of the bound binary is on a hyperbolic orbit with respect to the
  single body and moving away from it. If both conditions are met, the system is classified as having undergone
  breakup. This algorithm is executed periodically throughout the simulation at every crossing time of the system."
  Footnote 5: "The crossing time for a system is defined as τcr := G M^{5/2}/(−2E)^{3/2}, where M denotes the total
  mass." The lifetime τD is the time of that breakup.
- **The two cuts**, §4.1, eq. (30), quoted: "non-prompt ejections cut: τD ≥ 30 yrs; ergodic cut: τD ≥ 80 yrs and
  τgap ≥ 10 yrs", where "τgap denotes the time interval between the end of the last sub-escape excursion and the final
  escape". In G = 1 (M⊙, AU) units: 30 yr = 188.5, 80 yr = 502.7, 10 yr = 62.8. A sub-escape excursion is defined only
  in words (§2, p. 3: "motions in which a bound binary and the single body begin moving away from each other, attain a
  large separation, but eventually fall back toward each other due to insufficient kinetic energy"), with no
  threshold, so τgap, and with it the ergodic cut, cannot be reproduced exactly from the paper; the non-prompt cut
  needs only τD.
- **Which subset each table uses.** Table B.1 holds two probabilities: P(2d,np), on the non-prompt subset (%(np), 46%
  of runs for (15, 15, 15), 46–99% over the eight sets; standard errors 0.0001–0.0007), and P(2d), on the ergodic
  subset (%(es), 12% for (15, 15, 15), 6–15%; standard errors 0.0003–0.0014). Figure 2 and §4.2's comparisons use the
  ergodic cut; so do Table B.3 (§4.7; prograde or retrograde by "whether the angular momentum of the remnant inner
  binary (LB) is prograde or retrograde relative to the direction of the total angular momentum (L)", §2.8), Table 4
  (§4.7, eq. (35)) and Table B.2 (§4.3, maximum likelihood on L_F,c ≤ L_F ≤ L_F,c + 30). Table 2 (§4.5) uses all runs
  with τD ≥ 80 yr, without the τgap condition: "The linear regression fits for the power-law index of the lifetime
  distributions in the region 3000 ≤ τD ≤ 10⁶ using log-log scale", on the differential lifetime distribution (Fig. 7);
  the binning is not stated.
- **Relevance to MK-1.** MK-1 (Manwadkar, Kol, Trani & Leigh 2021, "MKTL21") is blocked because its runs are 3D. This
  paper is the same group's planar counterpart, one million planar runs per mass set, with Table 3 comparing planar
  against 3D. MK-1 is left unchanged; DS-1 is proposed beside it.
- **Proposed:** DS-1 (per-body escape probabilities, prograde/retrograde), DS-2 (lifetime tails), tier (a).

### Dandekar & Kol 2025 (Table 2, planar phase volume)

- **Readings.** A: ChatGPT's JSON. B: by eye, with an OCR check. 0 disagreements in 40 cells (20 equal only as
  numbers). `dandekar_kol_2025/table2_reading_b_eye.csv`.
- Its |E| and L are Dandekar 2026's Table 1 in G = 1 units, but L is **truncated**, not rounded, to two decimals
  (113.389 → 113.38, 152.1595 → 152.15, 91.8559 → 91.85).
- Paired with DS-1: σ̄₂d(E, L) is the flux theory's input for the predicted escape probabilities (Dandekar 2026,
  Table B.1's theory columns). It is not a separate measurement, so it gets no row of its own.

### Mikkola & Merritt 2008 (the N = 3 benchmark)

- **Readings.** Table 1's N = 3 rows and Tables 2–3's N_B = 3 rows: 0 disagreements in 36 cells.
- **Rotation into the plane**, re-derived (`scripts/mikkola_merritt_rotation.py`, `mikkola_merritt_2008/rotation_check.txt`).
  The three bodies satisfy Z = X (body 1 at the origin, bodies 2 and 3 with Z − X = 0 exactly), so they lie in the plane
  X − Z = 0 through the origin, with unit normal n = (1, 0, −1)/√2. Take the orthonormal in-plane basis
  e₁ = (1, 0, 1)/√2 and e₂ = (0, 1, 0); then x = r·e₁ = (X + Z)/√2 = √2 X, y = r·e₂ = Y. All velocities are zero, so
  nothing else rotates. Pairwise distances are preserved to 1.3×10⁻⁵¹ and E is unchanged; ours against ChatGPT's
  planar coordinates, 4.4×10⁻¹⁷ (theirs are binary64).
- **No row.** It is an integrator stress test (masses 1, 10⁻³, 10⁻⁴, from rest, to t = 1000), not a replication.
  PLAN.md has no stress-test row; the nearest are H-1 and H-2 (tier C, integrator comparisons). Whether such a row
  belongs there is left to the porter.
- arXiv's journal reference reads "Mon.Not.Roy.Astron.Soc.135:2398,2008"; the paper is AJ 135, 2398.

### Mel'nikov, Orlov & Shevchenko 2013 (the six fits)

- **Readings.** 0 disagreements in 36 cells (2 equal only as numbers). `melnikov_2013/fit_table_reconciled.csv`.
- **The fit**, p. 438: "Parameters of the fit Td = C TLi^γi on the interval 2 × 10³ ≤ Td ≤ 10⁵", i = 1…6, one fit per
  Lyapunov exponent; i = 1 (the largest): γ₁ = 1.71 ± 0.03, C = 10.8 ± 1.7, R² = 0.881.
- **Not free fall.** The set-up is a weakly hierarchical triple near the 2:1 period resonance with a retrograde outer
  body (eqs (2)–(3), Δx ∈ [0, 1], G = 1, unit masses, inner binary circular with radius 1, disruption at r ≥ 5d). MT-1
  (Mikkola & Tanikawa 2007, free fall, t_d ≈ 10 t_e^1.8) has similar numbers but is a different experiment.
- **Horizon:** T_d from 2×10³ to 10⁵ lies past R-610's cap of 200 (R-636: "not evaluated within cap", never a failure).
- **Proposed:** MOS-1, tier (a).

### Record only: Rose & Dullin 2013, Johnstone & Rucinski 1991, Nauenberg 2001, Rein & Spiegel 2015, Dandekar et al. 2022

- **Rose & Dullin 2013**, CeMDA 117, 169, arXiv:1302.0912 (nonexclusive): "A symplectic integrator for the symmetry
  reduced and regularised planar 3-body problem with vanishing angular momentum", tested on the figure-eight, the
  Pythagorean orbit and a periodic collision orbit. No target: an integrator-method paper, in its own reduced
  variables; its examples are covered by the periodic-orbit and Burrau rows.
- **Johnstone & Rucinski 1991**, PASP 103, 359 (no licence found): "Statistical properties of planar
  zero-angular-momentum equal-mass triple systems": 2,850 free-fall runs on a 0.01 grid over the Agekyan–Anosova
  region, bodies at (±0.5, 0), unit masses, G = 1. No target, as the owner directed: its results are greyscale maps
  of the initial-condition plane (Fig. 1(a)–(d)), with no tables, at most a qualitative comparison of the kind the
  free-fall map rows already make.
- **Nauenberg 2001**, Phys. Lett. A 292, arXiv:nlin/0112003 (assumed-1991-2003): "Periodic orbits for three
  particles with finite angular momentum", equal masses, continuing Lagrange's circular solution and the figure-eight
  in angular momentum, for forces 1/r^{p+1}. No target: no tabulated initial conditions.
- **Rein & Spiegel 2015**, MNRAS 446, 1424, arXiv:1409.4779 (nonexclusive): the IAS15 integrator. No target: its tests
  are integrator benchmarks, not three-body results.
- **Dandekar et al. 2022**, CeMDA 134, 55, arXiv:2205.04294v1 (nonexclusive): "Regularized phase-space volume for
  the three-body problem", the quantity that sets the disintegration time scale in the flux-based theory. No target:
  a theory paper; the planar values DS-1 needs are Dandekar & Kol 2025's Table 2.

### Orlov, Titov & Shombina 2016: resolved, context only

- **The file:** the owner's copy, 756,604 bytes, sha256 1a872913fd271e9200ecad2e6e26c29e824208516f23878ae419f59da52bbb4c
  (checked), at `docs/incoming/_local/orlov_titov_shombina_2016/S1063772916110056.pdf`. `git check-ignore -v`:
  `.gitignore:21:/docs/incoming/_local/`.
- **The title** (p. 1083), quoted: "Periodic Orbits in the Free-Fall Three-Body Problem". Not free fall with rotation.
  The repo nowhere describes it as "with rotation": a grep of main finds no line describing this paper; the wording
  survives only in ChatGPT's `ORLOV_2016_ACCESS_AUDIT.md`, line 19 ("the earlier lead's wording 'with rotation' is not
  established by this abstract"). No repo line needs a porter's fix.
- **The owner's three points against the paper:**
  1. *Figs 3–4 are colour maps of closeness of return to the Hill surface, cube-root scaled, over (θ, φ) ∈ [0, π/2] ×
     [0, π/3].* Holds for Fig. 3 only. Fig. 3 is "A region on the Hill surface", the region (θ, φ) ∈ [0, π/2] × [0, π/3]
     (one twelfth of the surface, p. 1086), and "we considered the cube root of the distance. The color scale indicates
     how close a point is to the Hill surface in the case of a re-encounter". Fig. 4 is "The Hill surface", the whole
     surface: "We also constructed the entire Hill surface (Fig. 4)" (p. 1087).
  2. *Runs stopped after three pairwise approaches; not lifetime or outcome maps.* Holds: "The integration was stopped
     after three pairwise approaches" (p. 1086).
  3. *S-orbit regions near θ = 0, isosceles near θ = π/2.* Holds (p. 1088): S orbits on the equator θ = 0, isosceles
     orbits at the pole θ = π/2.
- Set-up: h = −1, the closeness threshold c₀ = 10⁻³, MATLAB, Waldvogel's regularisation. No tables, ICs or periods.
  Use: at most a qualitative visual comparison for the free-fall maps.

### Titov 2024 (DSc dissertation, "Общая задача трех тел в пространстве форм", "The general three-body problem in shape space")

**Appendix B, 33 orbits** (B2-01…16, B3-01…13, T4-02…05; the figure-eight B.1 has no IC block). Each row: masses, A
(action), ω (frame rotation rate), E, |J|, and the barycentric ICs x₁, x₂, x₃, ẏ₁, ẏ₂, ẏ₃ (7 decimals); every other
coordinate is zero (the bodies start on the x-axis, moving along y).

- **Readings.** A: the text layer (`scripts/titov_textlayer_reading_a.py`, NFC-normalised, private-use glyphs
  dropped). B: OCR of the page images, with 54 cells read by eye (every IC crop viewed; all OCR misses or dropped minus
  signs). **0 disagreements in 462 cells.** `titov_2024/appB_reconciled.csv`.
- **Masses** from the prose: B2-01…13 0.95/0.95/1.1; B2-14…16 1.05/1.05/0.9; B3 0.99/1.01/1.0; T4 m₂ = 2 − m₁, m₃ = 1,
  ω = 1/2 (Table 4's header, "m1+m2=2, m3=1.0, ω=1/2").
- **E and J recomputed** (`scripts/titov_recompute_EJ.py`, 50 digits, G = 1; `titov_2024/appB_EJ_recomputed.csv`).
  The appendix prints |J|; the recomputed J is negative for 12 rows. **25 of 33 check within one last-place unit
  against the appendix's own E and |J|.** The eight that fail:

  | Row | E | J | Note |
  |---|---|---|---|
  | B2-10 | 6.6 units off | checks | Table 2's −1.049610 is 4.6 units off |
  | B2-11 | printed E, J are B2-10's | as E | recomputed −1.147637, 2.627705; Table 2's −1.161810, 2.58582 also disagree; momentum −0.022: the ICs are inconsistent |
  | B2-16 | printed E, J are B2-14's | as E | recomputed equals Table 2's −0.881539, 2.33447 |
  | B3-06 | 19 units off | 100 units off | momentum −2×10⁻⁵ |
  | B3-08 | 1.5 units off | checks | momentum 1.8×10⁻⁶ |
  | B3-09 | 4.6 units off | checks | |
  | B3-12 | 4.3 units off | checks | |
  | T4-03 | printed E, J are T4-02's | as E | recomputed −0.696347, 1.096486; Table 4's −0.696348 (1.3 units), 1.09648 agree |

  Worst disagreement in a row whose E and J are not copies: B3-06's J, 100 last-place units.
- **Which orbits TI-1 keeps: one rule** (`scripts/titov_selection.py`, `titov_2024/selection.txt`). The appendix
  carries the ICs, so an orbit is kept only if (1) the appendix's own E and |J| check, and (2) where a main-text table
  prints E or |J| to more decimals than the appendix, that more precise value checks too. A main-text value at the
  appendix's own precision that disagrees while the appendix checks is the table's error, not the orbit's. The same
  script bounds what the ICs' 7-decimal rounding (±0.5×10⁻⁷ in each of the six) can move E and |J|, to first order:
  at most 0.70×10⁻⁶ in E (B2-13) and 0.04×10⁻⁵ in |J| over all 33, so a disagreement of a whole last place in Table
  2's 6-decimal E is not the ICs' rounding. Clause (2) excludes two orbits the appendix alone passes:
  - **B2-12**: the appendix prints E = −1.36608 (5 decimals), which checks; Table 2 prints −1.366082, and the
    recomputation gives −1.3660756, 6.4×10⁻⁶ away, against a rounding bound of 0.60×10⁻⁶;
  - **B2-13**: the appendix prints −1.46075, which checks; Table 2 prints −1.460752, the recomputation −1.4607493,
    2.7×10⁻⁶ away, against a bound of 0.70×10⁻⁶.

  These are the same kind of disagreement that excludes B2-10 (6.6×10⁻⁶ against the appendix's 6-decimal E, bound
  0.35×10⁻⁶), seen only because Table 2 prints one more decimal. Whether Table 2 or the ICs are wrong is not
  settled by the source; either way the ICs are not shown to carry the printed orbit, so both are left out. **TI-1
  keeps 23 orbits; 10 are excluded** (B2-10, B2-11, B2-12, B2-13, B2-16, B3-06, B3-08, B3-09, B3-12, T4-03).
- **Main-text Tables 2, 3, 4** (pp. 51, 59): A text layer, B OCR (+ 9 stability marks '−' read by eye): 0
  disagreements (128, 112 and 35 cells). Table 2 is "Орбиты с симметрией 2-1" ("orbits with 2-1 symmetry"). Columns,
  translated: A (action), E (energy), |C| (angular momentum, the appendix's |J|), ω, [I_min, I_max] (range of the moment
  of inertia), "Рис" (figure; л/п left/right panel), "Уст" (stable: + yes, − no). Their own errors against the appendix
  and the recomputation: Table 2 row 3 |C| 1.22094 (appendix and recomputation 1.22042); row 4 E −0.850687 (row 7's
  value; appendix and recomputation −0.659868); row 8 |C| 2.44831 (appendix and recomputation 2.46683); row 4 prints "[3.27315,3.308455"; rows 12 and 13 E −1.366082 and −1.460752, 6.4 and 2.7 units of 10⁻⁶ from the
  recomputation, beyond the ICs' rounding (the table's error or the ICs', see the rule above); row 10 E −1.049610,
  4.6 units off (B2-10, excluded on the appendix already). Table 3 lists an orbit (A = 14.09948) not in Appendix B;
  its row 11 |C| 1.19288 against 2.39407; A 16.76479 against the appendix's 16.76478.
- **Frame and period.** The ICs are inertial; the rotation is counterclockwise, checked on B2-01: the rotating-frame
  velocity plus ω × r gives ẏ₁ = −0.2750279 against the printed −0.2750278. The period is 2π in t (pp. 52–53:
  "проинтегрировав уравнения движения на один период (2π)", "integrating the equations of motion over one period
  (2π)"); the integrals fluctuate by at most 2×10⁻⁷ relative over it, and solutions whose error exceeds 10⁻⁵ are not
  given (pp. 52–53).
- **Proposed:** TI-1, tier (b), on the 23 orbits the rule keeps. B2-16's E and J check only against Table 2, so it
  is left out (clause (1): its appendix E and J are B2-14's).

**Chapter 5, Tables 10 (collinear) and 11 (isosceles)**

- **Readings.** A text layer, B OCR with eye fixes: 0 disagreements (60 and 45 cells). Columns, translated: ρ₀ (or E₀
  for starred brake orbits), T ("в единицах регуляризованного времени", "in units of regularised time"), C (the
  collision sequence on the regularised plane; a, b, c, d the collision rays), C_c (the sequence on the unregularised
  plane), ":r" free-fall orbits (the sequence shown for the first half-period).
- **Decoded** (`scripts/titov_ch5_decode.py`, `titov_2024/ch5_decoding_check.txt`):
  - unit masses, G = 1, energy h = −1 (pp. 111, 126); ϱ² = I, the moment of inertia about the centre of mass (checked
    from the paper's distance formulas);
  - the time transformation, p. 108, quoted: "H′ = u(q)(H − h). Если x(τ) – решение задачи с таким гамильтонианом, то
    x(t) – решение задачи с исходным гамильтонианом H и значением постоянной энергии h, где dτ/dt = u⁻¹(q)" ("If x(τ)
    solves the problem with this Hamiltonian, then x(t) solves the problem with the original Hamiltonian H and energy
    constant h, where dτ/dt = u⁻¹(q)"); collinear u = 1 + cos 4E (5.15), isosceles u = 1 − cos 2E (5.36). So
    **dt = u dτ, and the physical period is t = ∮ u dτ**;
  - starts: on a collision ray, p_ϱ = 0 and p_E² = 2√6 ϱ ((5.16), (5.37)); Table 10's starred rows start on the Euler
    ray E = 0 with p_ϱ = 0 and p_E from H′ = 0; Table 11's starred rows are brake orbits at rest at E₀ with ϱ₀ from
    V = −h;
  - the kinetic-energy coefficients were derived here from Cartesian positions (sympy) and agree with (5.11) and
    (5.32)–(5.34) once (5.33)'s misprint is corrected.
- **Integrated** from the printed starts (DOP853, tolerance 10⁻¹²): collinear row 1 (Schubart) returns at τ = 7.825147
  against 7.825149 (distance 4.3×10⁻⁶), physical period t = 9.966127; isosceles row 2 within 10⁻⁷, row 6 2.8×10⁻⁶; rows
  1, 3 and 4 about 3×10⁻⁴; the others do not close at the printed 5–6 digits. The paper itself says ϱ₀ needs 8–9 or
  11–12 digits (p. 116).
- **Misprints found:** the text's "pE = 2√6ϱ" (pp. 110, 125, 126) against (5.16) and (5.37)'s p²_E = 2√6ϱ (our
  derivation agrees with p²_E); (5.33)'s "(3 + cos 3E)²" against (5.34)'s (3 + cos 2E)²; Fig. 5.5's caption "на луче
  E = 0" against the text's "на луче E = π/4"; the ray labels "a, b, c, d – лучи E = kπ/4, k = 1…4" against the
  collision rays π/4 + kπ/2; (5.20)'s h term lacks (5.15)'s (1 + cos 4E) factor. Also (5.14) fixes h = −1/2 for its
  zero-velocity curve, while the tables use h = −1.
- **No target.** The decoding is unambiguous, so no RQ is needed; but the printed 5–6 digits are too few for most rows
  to close, so no row is proposed.

### Li & Liao 2018 (Tables A1–A5, 234 free-fall periodic orbits)

- **The PDF** (the owner's copy, sha256 cce9e000…, checked) is byte-identical to the committed
  `docs/incoming/2026-10-24_anosova/papers/li_liao_2018_arXiv_1805.07980.pdf`.
- **Licence:** arXiv nonexclusive-distrib 1.0 (abstract page), so local-only. **Journal reference**, from arXiv's
  journal-ref and Crossref: New Astronomy 70 (2019) 22–26, doi:10.1016/j.newast.2019.01.003, issued July 2019; the
  journal's own page (ScienceDirect) answered 403, so it was not read directly.
- **Readings.** A: the text layer (234 rows). B: a two-of-three OCR consensus (full page at 300 dpi, the column at 600
  dpi, two strips at 300 dpi upscaled 2×) plus 15 cells (5 rows) by eye. **0 disagreements in 1,872 cells**
  (234 rows × 8 columns), from `python3 -I scripts/diff_readings.py li_liao_2018/tablesA1toA5_reading_a_textlayer.csv
  li_liao_2018/tablesA1toA5_reading_b.csv table+m1+m2+m3+Fi > li_liao_2018/tablesA1toA5_diff.txt` (a composite key:
  Table A1 holds two mass sets, so `table+Fi` repeats).
  `li_liao_2018/tablesA1toA5_reconciled.csv`. Tables: A1 (1, 1, 1) and (1, 0.8, 0.8); A2 (1, 0.8, 0.6); A3
  (1, 0.8, 0.4); A4 (1, 0.8, 0.2); A5 (0.6, 0.8, 1). Tables A6–A10 (free-group words) are recorded, not transcribed;
  nothing is built on them.
- **Frame:** r₁ = (−0.5, 0), r₂ = (0.5, 0), r₃ = (x, y), all at rest, G = 1: Tanikawa, Saito & Mikkola's frame, but
  with the free body numbered 3 (TSM number it 1). Each orbit is a point on the free-fall maps.
- **Overlap.** Rule: same masses, the same x and y as decimals. Against the threebodyorbits.com snapshot (`OC26`,
  family `freefall`, `li_liao_2018/overlap_oc26.csv`): **230 present, 4 absent** (A1 F2, F13, F24; A2 F16), exactly
  PLAN.md § 2.1's four aliases; T differs (OC26 − paper) by a median 2.3×10⁻¹¹, at most 5.1×10⁻⁵ (A2 F28), 38 rows
  above 10⁻⁶. Against the authors' own 316-orbit table that LL-5 reads
  (`../2026-10-27_hp_ics_freefall/data/liliao_github/free-fall-3b-pictures.md`, `scripts/liliao_vs_ll5.py`,
  `li_liao_2018/overlap_ll5_authors_table.csv`): **all 234 present, with x, y and T identical**. New orbits: **0**, so
  no new row; LL-1's "not yet transcribed" is now met (a note is proposed below).
- `../2026-10-27_hp_ics/` holds Li & Liao's 2017 tables, a different paper.

### Tanikawa, Umehara & Abe 1995 ("Collision orbits I")

The scan has no usable text layer, so reading A is OCR of the page images and reading B is by eye, both from 300–600
dpi renders.

- **Readings**, disagreements per table: IIa 17 of 216 cells, IIb 33 of 240, IIIa 21 of 152, IIIb 17 of 200, IIIc 14
  of 176, IV 2 of 100. Every one is a cell OCR left blank, except 4 garbled θ tokens in IIb (rows 16, 17, 18, 23); no
  number OCR delivered differed from the eye reading. All resolved for the page; the 6 numeric cells resting on the eye
  reading alone were re-checked by eye. `tanikawa_umehara_abe_1995/table*_reconciled.csv`: 123 binary-collision rows,
  20 triple-collision rows.
- **Columns** (p. 347, quoted in part): x, y; θ, "the position angle θ defined in Section 2"; Time, "the time when this
  encounter occurs"; v/h and φ, the survey direction and the sign change of φ ("'h' means that the survey is horizontal
  from left to right. 'v' means that the survey is vertical from below to above"); Δx/Δy, "the size of grid elements for
  a one-dimensional survey when |φ| becomes less than 10⁻² degrees". "An asterisk on the shoulder of a number indicates
  that the value of R is changed. Usually, R = 10⁻³ is adopted. Single and double asterisks correspond to R = 10⁻⁴ and
  R = 10⁻⁵, respectively." Collision types: type 1 is the 2–3 pair, type 2 the 1–3 pair, type 3 the 1–2 pair; IIa and
  IIIa are type 1, IIb and IIIb type 3, IIIc type 2. Time is when the n_min-th encounter occurs (n_min = 2 in Table II;
  the tabulated n_min, 3 to 6, in Table III). Table IV (p. 360): "Positions of the first twenty triple collision
  points on Γ₃. n_min is the number of minima of the smallest distance, (x, y) the position of triple collision
  points, and α is the vertex angle of initial isosceles triangle", n_min = 1 to 20, with x, y (7 decimals) and α.
- **The encounter**, §2, p. 339, quoted: "we consider that binary collision is regularized and orbits are extended
  beyond this singularity. We do not consider this extension for triple collision." … "Let r12, r23 and r13 be the
  mutual distances of mass points m1, m2 and m3 and let r = min{r12, r23, r13}. … In the course of time, r(t)
  oscillates, i.e., it takes minima and maxima. Fix a small number R > 0 and an integer n = n_min > 0." The n-th
  encounter is the n-th minimum of r(t), at t = t_n. The type is set there: if r(t) = R at some t*_n with
  t'_{n−1} ≤ t*_n ≤ t_n (t'_{n−1} the (n−1)-th maximum), the colliding pair is the i, j with r(t*_n) = r_ij; types
  0, 4, 5 and 6 cover r(t_n) > R, a tie (isosceles or equilateral), r < R throughout [t_{n−1}, t_n], and a triple
  collision before the n-th minimum. A binary collision before the n-th minimum is one minimum, and the orbit is
  continued through it.
- **Frame**, eq. (1), p. 335, quoted: "Let mass points m₂ and m₃ stand still at A(−0.5, 0) and B(+0.5, 0), respectively
  in the (x, y) plane and m₁ stand still at a point P(x, y) where (x, y) ∈ D = {(x, y) : x ≥ 0, y ≥ 0, (x + 0.5)² + y² ≤
  1}." **Identical** to Tanikawa, Saito & Mikkola 2019's (§3, which cites this paper): the same bodies at the same
  points, the same D. No remap.
- **Units**, p. 342, quoted: "In the units G = 1, M = 1, AB = 1". M means each mass (m = 1), decoded by integration
  (`scripts/tua1995_units_check.py`, `tanikawa_umehara_abe_1995/units_check.txt`): from (0.42, 0.4369369) and
  (0.3386657, 0.38) the 2–3 collision comes at t = 0.654709 and 0.603364 with m = 1, the tabulated 0.6547 and 0.6034;
  m = 1/3 gives 1.134 and 1.045. A third point, IIa row 27 (0.1820201, 0.04, double-asterisked, R = 10⁻⁵), was not
  reproduced by this unregularised check (a close 1–3 approach comes first); that is a limit of the check, not a
  finding against the paper.
- **Anomalies in the source:** IIa rows 22 and 23 print the same x, θ and Time at different y (0.134, 0.120); IIb
  prints θ 0.2181 at rows 23 and 25, out of sequence; Table IV row 9 prints x = 0.49111357 (8 decimals), where
  0.4911357 fits α and the circle; Table IV row 1 prints α = π/2 at (0, √3/2), where the angle PAB is π/3. Table IV's
  points lie on the circle (x + 0.5)² + y² = 1 to about 10⁻⁶ (`table4_consistency.txt`), and α equals the angle PAB to
  about 10⁻⁷.
- **Umehara & Tanikawa 2000** quotes Table IIa's row (0.2702483, 0.3), which matches our transcription.
- **Proposed:** TUA-1 (binary collisions) and TUA-2 (triple collisions), tier (b).
- **Cross-reference:** this is "Collision orbits I". Task 15b's record covers Tanikawa 2000, "Collision orbits II":
  `../2026-10-29_research_batches/README.md` § 5.6 (branch `data/research-batches-2026-10-29`, not yet merged), whose
  § 5.1 reaches the same frame finding for Li et al. 2021, Tanikawa & Umehara 1998, Tanikawa 2000 and Martynova & Orlov
  2014.

### Anosova 1986

- **Readings.** Table V (escape-time distribution) and Table X (prograde/retrograde fractions against k₀): A OCR, B by
  eye, 0 disagreements (50 and 9 cells). Each Table V column sums to 1.000.
- **Table V, quoted notes:** "Ia: plane problem; equal masses; k₀ = 0, L = 0. IIa: plane problem; equal masses; k₀ ≠ 0,
  L ≠ 0. IIIa: three-dimensional problem; equal masses; k₀ ≠ 0, L ≠ 0. IIIb: three-dimensional problem; chance choice
  of masses and k₀." Column Ia (planar, equal masses, at rest: free fall) is the candidate.
- **The time unit is not settled.** Eq. (1), p. 218, as printed: τ = G √(Σᵢ Mᵢ Σ_{i≠j} MᵢMⱼ) / (−2E)^{3/2}, with the
  radical drawn over both sums. That is dimensionally inconsistent (it gives G M^{3/2}, where a crossing time needs
  G M^{5/2}); and whether Σ_{i≠j} runs over ordered pairs (twice the unordered sum) is not said. Eq. (2) prints the mean
  size as d = G Σ MᵢMⱼ / 2E, positive only with −2E. The repo records Aarseth, Anosova, Orlov & Szebehely 1994b's
  τ = G M_t^{1/2}(Σ MᵢMⱼ)/(−2E)^{3/2} (`../2026-10-27_papers/README.md` line 201, `TARGETS.md` line 255). Also
  unstated for Table V: the escape distance ρ* (20d in Anosova et al. 1984, "in other experiments discussed here,
  ρ* = 30d", p. 229) and the number of runs. RQ-651 below.
- **Table X** (rotating systems): no target.
- **Proposed:** AN-2, tier (a), contingent on RQ-651.

### Umehara & Tanikawa 2000; Saito & Tanikawa 2007 and 2009

- **Umehara & Tanikawa 2000**, CeMDA 76, 187: no tables. Context only (binary and triple collisions causing instability
  in the free-fall problem).
- **Saito & Tanikawa 2007**, CeMDA 98, 95, and **2009**, CeMDA 103, 191: the rectilinear (collinear) problem by symbol
  sequences. Context only: rectilinear, and their results are maps and sequences, not tabulated inputs to reproduce.

### Aarseth & Zare 1974 and Heggie 1974: already in the repo

Both are recorded in `docs/incoming/2026-10-25_replication/README.md`, lines 13–14, with the same sha256 as the owner's
copies (Aarseth & Zare d1969c53…, Heggie 3d6bd960…), and PLAN.md's H-2 cites Aarseth & Zare. The PDFs themselves are in
the owner's `_local/`, not committed. Skipped, as the owner asked.

### Pasechnik, Mylläri, Valtonen & Mikkola 2023, Part II: a duplicate

Byte-identical (sha256 76eab52c…) to #327's committed
`docs/incoming/2026-10-28_rq598_data/papers/pasechnik_myllari_valtonen_mikkola_2023_MNRAS_525_1929.pdf`: same title,
same DOI (10.1093/mnras/stad2372), same bytes. Licence: CC BY 4.0 per Crossref's version-of-record licence (from
2023-08-04) and the PDF's "© 2023 The Author(s)"; the University of Turku cover sheet says "All rights reserved", and the
OUP article page answered 403. Publisher: Oxford University Press for the RAS. Nothing new committed.

### Portegies Zwart & Boekholt 2018

Already ingested in PR #332: `../2026-10-29_burrau_pzb2018/` (proposed SP-2; RQ-611).

### Unobtained, or not verified

- Orlov, Titov & Shombina 2016: **resolved: obtained 29 Oct 2026, local-only, context only.**
- Li & Liao 2018's journal page (ScienceDirect, 403): the reference rests on Crossref and arXiv.
- Pasechnik 2023's OUP page (403): the licence rests on Crossref and the PDF.
- Johnstone & Rucinski 1991: no licence statement found, so local-only.

## Corrections and contradictions

- **Tanikawa's frame (RQ-650).** `docs/incoming/2026-10-25_replication/catalogue.md` line 13 says: "Tanikawa's papers
  use D11 = {x ≥ 0, y ≥ 0, (x−0.5)² + y² ≤ 1}, the mirror labelling." The same claim is in
  `docs/design/principia_dd_replication.md` line 564, `docs/incoming/2026-10-25_replication/INSTRUCTION_replication_suite_2026-10-25.md`
  line 45 and PLAN.md line 707 ("Tanikawa uses the mirror labelling D11"), and TM-1 and TS-1 carry a "D11 mirror
  remap". Two of Tanikawa's papers say otherwise: Tanikawa, Umehara & Abe 1995, eq. (1), and Tanikawa, Saito & Mikkola
  2019, §3, both put the free body in D = {x ≥ 0, y ≥ 0, (x + 0.5)² + y² ≤ 1} with the others at (∓0.5, 0), and the
  2019 paper says "D11 corresponds to Anosova's region". Task 15b's record (§ 5.1) finds the same for four more
  papers, and its § 10.4 leaves TM-1's "D11 mirror remap" as it is until the owner confirms. Two of Tanikawa &
  Mikkola's papers, read here:
  - **Tanikawa & Mikkola 2008** (arXiv:0802.2465, "A trial symbolic dynamics of the planar three-body problem",
    committed at `../2026-10-25_replication/papers/tanikawa_mikkola_2008_arXiv0802.2465.pdf`), p. 3, quoted: "We
    want to extend the free fall problem with equal masses (Anosova & Orlov 1986; Tanikawa et al. 1995), in which
    bodies 2 and 3 initially stand still at (−0.5, 0) and (0.5, 0), respectively, whereas body 1 stands still at
    (x, y) ∈ D11 = {x ≥ 0, y ≥ 0, (x − 0.5)² + y² ≤ 1}." This is very likely where the repo's "mirror labelling"
    sentence comes from. It is probably a typo: the paper cites Tanikawa et al. 1995 for the set-up, whose eq. (1)
    prints (x + 0.5)², and the region as printed does not keep AB the longest side: (1.4, 0.3) lies in it
    (0.9² + 0.3² = 0.90 ≤ 1) with PA = 1.92 > AB = 1.
  - **Tanikawa & Mikkola 2015** (PASJ 67, 115; the owner's copy, local-only:
    `../_local/tanikawa_mikkola_2015_PASJ_67_115.pdf`, see `../2026-10-25_replication/README.md` line 85), p. 115-2,
    eq. (9), quoted: "Initially, body m2 stands still at (−0.5, 0), and body m3 stands still at (0.5, 0) of the
    (x, y)-plane. Body m1 stands still at any position in D = {(x, y) : x ≥ 0, y ≥ 0, (x + 0.5)² + y² ≤ 1}."; Fig. 1's
    caption, p. 115-3: "Anosova's D region is equivalent with any of them, say, region D11."

  Tanikawa & Mikkola 2021 (TM-1's source) was not read here. Whether TM2008's (x − 0.5)² is a typo is for the
  ruling.
- **Titov 2024:** the copied E and J rows (B2-11, B2-16, T4-03), the inconsistent B2-11 ICs, Tables 2–3's own errors and
  chapter 5's misprints, all listed above.
- **Tanikawa, Saito & Mikkola 2019:** the p. 25 orientation sentence, "Figs. 8, 11, and 11".
- **Dandekar 2026:** the (8, 21, 21) E; Dandekar & Kol 2025's truncated L.
- **Tanikawa, Umehara & Abe 1995:** the anomalies listed above.
- **Anosova 1986:** eq. (1)'s units (RQ-651).
- **Mikkola & Merritt 2008:** arXiv's journal reference names MNRAS for an AJ paper.
- **Orlov, Titov & Shombina 2016:** the title is "Periodic Orbits in the Free-Fall Three-Body Problem"; no repo line
  says otherwise.

## To apply once #328 and #335 merge

Nothing here has been applied: PLAN.md, REVIEW_QUEUE.md and decisions.md are left to the porter. (#328 merged on 29 Oct
2026, while this record was being written; #335 is open.) Each row uses PLAN.md § 3's columns; "tier (a)/(b)" in
Status is R-649's tier, beside the T column's 25 Oct tier. If R-649's port changes the columns, the porter maps these
fields onto them. Ids are new (none is in PLAN.md on main at d3983980); the prefixes name the source.

### 1. `docs/incoming/2026-10-27_validation_plan/PLAN.md`, § 3.2 Replication: after the row `TS-2`, add

```
| TSM-1 (proposed, 29 Oct 2026) | R | A | Tanikawa, Saito & Mikkola 2019 (CeMDA 131, 24, doi:10.1007/s10569-019-9902-8), Table 1 and §3: `../2026-10-28_external_sources/README.md` § "Tanikawa, Saito & Mikkola 2019"; beside TS-1, TS-2, TM-1, TUA-1 | From each of Table 1's 11 starts (m₁ = m₂ = m₃ = 1, G = 1 implied by eq. (4); body 2 at (−0.5, 0), body 3 at (0.5, 0), body 1 at the tabulated (x, y), all at rest), the run **approaches triple collision near the tabulated t, with the stated number of symbol-sequence digits**; t is the authors' practical integration endpoint near triple collision, not an exact collision time. The source's level, pp. 24–25, quoted: "Here we provide the coordinates in the precision of 8-digits … We use standard double precision arithmetic and provide the reliable initial 8 digits, which are in fact sufficient to reproduce our results": the starts carry 8 reliable digits, a tolerance, not a bound. Nothing is built on p. 25's orientation sentence (a contradiction in the paper) | `../2026-10-28_external_sources/tanikawa_saito_mikkola_2019/table1_reconciled.csv` (transcribed twice, 0 disagreements in 77 cells); the paper is local-only | Anosova entry: the same frame (§3: "D11 corresponds to Anosova's region"), no remap | regularisation (TASK-M3-08); a syzygy-symbol sequence (none planned, as TS-1) | exact (approach near t; the digit count) | data transcribed · proposed (29 Oct 2026) · tier (b), CPU IC inspector · scores in M9, or earlier as an information-only peek · outcome PASS / EXPLAINED DIVERGENCE / UNEXPLAINED |
```

### 2. PLAN.md, § 3.2: after the row `TM-1`, add

```
| TUA-1 (proposed, 29 Oct 2026) | R | A | Tanikawa, Umehara & Abe 1995 (CeMDA 62, 335, doi:10.1007/BF00692285), Tables IIa, IIb, IIIa–IIIc, eq. (1), pp. 342, 347: `../2026-10-28_external_sources/README.md` § "Tanikawa, Umehara & Abe 1995" | From each of the 123 tabulated binary-collision points (m₁ = m₂ = m₃ = 1, G = 1, AB = 1: "M = 1" read as each mass, decoded by integration; m₂ at (−0.5, 0), m₃ at (0.5, 0), m₁ at (x, y), all at rest), the stated pair (type 1: 2–3, Tables IIa, IIIa; type 3: 1–2, IIb, IIIb; type 2: 1–3, IIIc) has d_min → 0 at its n_min-th encounter (n_min = 2 in Table II, as tabulated in Table III), at the tabulated Time (4 decimals). The encounter is the paper's (§2, p. 339): the n-th minimum of r(t) = min{r12, r23, r13}, with binary collisions "regularized and orbits are extended beyond this singularity", so each earlier binary collision counts as one minimum and the run continues through it; the type is the pair with r(t) = r_ij when r(t) falls to R on the way into that minimum. Principia's closest-approach events are not this by default; the count is of minima of the smallest pairwise distance. Δx/Δy is the authors' survey grid size and R the small radius of their collision-type definition (§2; 10⁻³, or 10⁻⁴ and 10⁻⁵ where asterisked), not bounds | `../2026-10-28_external_sources/tanikawa_umehara_abe_1995/table{2a,2b,3a,3b,3c}_reconciled.csv` (two readings from the page images, every disagreement an OCR blank or garbled token, resolved for the page); the paper is local-only | Anosova entry: the same frame as TSM-1, no remap | regularisation (TASK-M3-08) | exact (the pair, the encounter count, the time to 4 decimals) | data transcribed · proposed (29 Oct 2026) · tier (b), CPU IC inspector · scores in M9, or earlier as an information-only peek · outcome PASS / EXPLAINED DIVERGENCE / UNEXPLAINED |
| TUA-2 (proposed, 29 Oct 2026) | R | A | Tanikawa, Umehara & Abe 1995, Table IV ("Positions of the first twenty triple collision points") | From each of the 20 points, **started on Γ₃ from α**: m₁ at (−0.5 + cos α, sin α), so PA = AB = 1 exactly and the isosceles symmetry holds (set-up otherwise as TUA-1; applied per R-369: the caption puts the points "on Γ₃" and gives α as "the vertex angle of initial isosceles triangle"), all three pairwise distances → 0 together at the n_min-th encounter, n_min = 1…20, defined as in TUA-1 (§2, p. 339: the n-th minimum of r(t) = min{r12, r23, r13}, binary collisions regularised and continued through; Table IV's caption: "n_min is the number of minima of the smallest distance"). Row 1 is (0, √3/2), Lagrange's homothetic collapse, started from α = π/3 (the angle PAB there; the printed α = π/2 is not used); row 9 starts from its printed α, and its printed x = 0.49111357 is read as 0.4911357 (it fits α and the circle). **Not from the printed (x, y)**: they miss Γ₃ by a circle residual \|(x + 0.5)² + y² − 1\| of 0.7 to 1.9×10⁻⁶ (rows 2–20, `table4_consistency.txt`), about ten times what 7-decimal rounding can produce (at most 1.4×10⁻⁷), while α equals the angle PAB to about 10⁻⁷. Off Γ₃ the symmetry, and with it the exact triple collision, is lost, so a run from the printed (x, y) is a different experiment. The source's level: α to 7 decimals (±0.5×10⁻⁷), the printed (x, y) within 2×10⁻⁶ of the arc; a tolerance, not a bound | `../2026-10-28_external_sources/tanikawa_umehara_abe_1995/table4_reconciled.csv` (two readings, 2 OCR blanks resolved); `table4_consistency.txt` | Anosova entry, on the boundary arc (x + 0.5)² + y² = 1 | regularisation (TASK-M3-08) | exact (simultaneous collision at the stated encounter) | data transcribed · proposed (29 Oct 2026) · tier (b), CPU IC inspector · scores in M9, or earlier as an information-only peek · outcome PASS / EXPLAINED DIVERGENCE / UNEXPLAINED |
```

### 3. PLAN.md, § 3.1 Verification: after the row `LL-1`, add

```
| TI-1 (proposed, 29 Oct 2026) | V | A | Titov 2024, DSc dissertation (SPbU), Appendix B, pp. 51–60: `../2026-10-28_external_sources/README.md` § "Titov 2024" | Each of the 23 Appendix B orbits kept by the README's one rule (`titov_2024/selection.txt`: the appendix's E and \|J\| match a recomputation from the printed ICs, and so does any main-text value printed to more decimals) (B2-01…09, B2-14, B2-15, B3-01…05, B3-07, B3-10, B3-11, B3-13, T4-02, T4-04, T4-05; G = 1; masses as the README gives per row) closes over one period t = 2π in the frame rotating counterclockwise at ω: state(2π) against R(2πω)·state(0). About 10⁻⁷ is the source's level (ICs to 7 decimals; integrals fluctuating ≤ 2×10⁻⁷ relative), a tolerance, not a bound. Tables 2–3 mark some orbits unstable ('−'); no orbit is labelled in advance for that. A closure miss counts as EXPLAINED DIVERGENCE (R-649 item 3) only when a measurement shows the mechanism, for example the one-period growth of a perturbation of the ICs at their printed rounding (±0.5×10⁻⁷) reaching the observed miss; without that measurement it is UNEXPLAINED. B2-10, B2-11, B2-16, B3-06, B3-08, B3-09, B3-12 and T4-03 are excluded (their appendix E or J does not check), and B2-12 and B2-13 (Table 2's 6-decimal E disagrees with the recomputation by 6.4 and 2.7×10⁻⁶, beyond the ≤ 0.70×10⁻⁶ the ICs' rounding allows) | `../2026-10-28_external_sources/titov_2024/appB_reconciled.csv` (33 rows, transcribed twice, 0 disagreements in 462 cells); `appB_EJ_recomputed.csv`; `selection.txt`; the dissertation is local-only | BodyPlane at unequal masses and L ≠ 0; MassSimplex point | M3 integrator (TASK-M3-36) | exact (closure to the stated tolerance) | data transcribed · proposed (29 Oct 2026) · tier (b), CPU IC inspector · scores in M9, or earlier as an information-only peek · outcome PASS / EXPLAINED DIVERGENCE / UNEXPLAINED |
```

### 4. PLAN.md, § 3.1: in the row `LL-1`, at the end of its Status cell, append

```
 · note, 29 Oct 2026 (proposed): Tables A1–A5 are now transcribed twice, 0 disagreements in 1,872 cells (`../2026-10-28_external_sources/li_liao_2018/tablesA1toA5_reconciled.csv`); all 234 equal the authors' 316-orbit table that LL-5 reads, in x, y and T; 230 are in the atlas (§ 2.1); no new orbit
```

### 5. PLAN.md, § 3.2: after the row `MT-1`, add

```
| MOS-1 (proposed, 29 Oct 2026) | R | B | Mel'nikov, Orlov & Shevchenko 2013 (Astron. Rep. 57, 429, doi:10.1134/S1063772913060048), the fit table, p. 438, and §2: `../2026-10-28_external_sources/README.md` § "Mel'nikov, Orlov & Shevchenko 2013"; beside MT-1 | T_d = C·T_Li^γi for i = 1…6 on 2×10³ ≤ T_d ≤ 10⁵, near the 2:1 period resonance (i = 1: γ₁ = 1.71 ± 0.03, C = 10.8 ± 1.7, R² = 0.881). **Not free fall**: a weakly hierarchical triple with a retrograde outer body (eqs (2)–(3), Δx ∈ [0, 1], G = 1, unit masses, inner binary circular of radius 1, disruption at r ≥ 5d); MT-1's free-fall result is a different experiment | `../2026-10-28_external_sources/melnikov_2013/fit_table_reconciled.csv` (transcribed twice, 0 disagreements in 36 cells); the paper is local-only | new: a hierarchical slice | the Lyapunov spectrum (only the largest exponent is planned, TASK-M3-14); outcome classifier | distribution | data transcribed · proposed (29 Oct 2026) · tier (a), GPU explorer · scores in M9, or earlier as an information-only peek · outcome PASS / EXPLAINED DIVERGENCE / UNEXPLAINED · T_d past R-610's cap of 200 is "not evaluated within cap" (R-636) until a raised-cap run |
```

### 6. PLAN.md, § 3.2: after the row `MK-1`, add

```
| DS-1 (proposed, 29 Oct 2026) | R | B | Dandekar, Springer, Zard, Trani, Kol & Birnholtz 2026 (arXiv:2608.30414v1, CC BY 4.0), Tables 1, 3, B.1, B.3, §3.1; Dandekar & Kol 2025 (CeMDA 137, 32), Table 2: `../2026-10-28_external_sources/README.md` § "Dandekar et al. 2026"; beside MK-1, AN-1, SL-1 | Planar per-body escape probabilities for eight mass sets, each compared only with the same selection applied to our runs, never with an all-runs fraction. **Ensemble** (§3.1): centre-of-mass frame; the lightest body the single, the other two in a circular binary, a = 5 AU; the single and the binary's centre of mass at rest, d = 100 AU apart; binary phase uniform on [0, 2π]; planar; one million runs per set. **Breakup** (§3.2, p. 6): "any two bodies form a bound binary whose apoapsis separation is at least ten times smaller than their distance from the single body", and "the center of mass of the bound binary is on a hyperbolic orbit with respect to the single body and moving away from it", checked once every crossing time τcr = G M^{5/2}/(−2E)^{3/2} (footnote 5); τD is the breakup time. **Cuts** (§4.1, eq. (30)): non-prompt, τD ≥ 30 yr; ergodic, τD ≥ 80 yr and τgap ≥ 10 yr, τgap running from the end of the last sub-escape excursion to the final escape (G = 1 units: 188.5, 502.7, 62.8). **Table B.1 holds two probabilities**: P(2d,np) on the non-prompt subset (46% of runs for (15, 15, 15); standard error 0.0007) and P(2d) on the ergodic subset (12%; standard error 0.0014); each is scored against its own cut. The paper defines a sub-escape excursion only in words, with no threshold (§2, p. 3), so the ergodic cut needs a definition requirement (R-72) before P(2d) or anything on the ergodic subset is scored; P(2d,np) needs only τD. The prograde/retrograde split, Table B.3, is on the ergodic subset (§4.7), prograde meaning the remnant binary's L_B along the total L (§2.8); the flux theory's columns rest on Dandekar & Kol 2025's planar phase volume σ̄₂d(E, L). Units: G = 4π², M⊙, AU, yr in the paper; G = 1 with M⊙ and AU kept, 1 yr = 2π (E/4π², L/2π). The paper monitored E and L only to a relative 10⁻⁵ and advanced close encounters and long excursions as two-body motions, so it is a statistical reference, never per run | `../2026-10-28_external_sources/dandekar_2026/table{1,B1,B3}_reconciled.csv` and `../2026-10-28_external_sources/dandekar_kol_2025/table2_reading_b_eye.csv` (each transcribed twice; 3 label disagreements in 752 cells, resolved for the page); `dandekar_2026/units_check.txt` | new: BinaryPlusSingle at a mass point (as MK-1) | outcome classifier | distribution | data transcribed · proposed (29 Oct 2026) · tier (a), GPU explorer · scores in M9, or earlier as an information-only peek · outcome PASS / EXPLAINED DIVERGENCE / UNEXPLAINED · the planar counterpart of MK-1's 3D runs (MKTL21) · runs whose escape comes after R-610's cap of 200 are "not evaluated within cap" (R-636) |
| DS-2 (proposed, 29 Oct 2026) | R | B | Dandekar et al. 2026, Tables 2, 4, B.2 | The lifetime distribution's power-law tail for each mass set: Table 2's index, for example (15, 15, 15): −1.692 ± 0.015, fitted from 3,000 yr to 10⁶ yr, 1.885×10⁴ to 6.28×10⁶ G = 1 units. **Subset and method** (§4.5, Table 2's note): all runs with τD ≥ 80 yr (no τgap condition), τD and breakup as DS-1; "linear regression on the lifetime distributions in log-log space for the region 3000 ≤ τD ≤ 10⁶", on the differential distribution (Fig. 7), binning not stated. Table B.2's α by escaper: maximum likelihood on L_F,c ≤ L_F ≤ L_F,c + 30 (§4.3), and Table 4's prograde/retrograde overlap coefficient (§4.7, eq. (35)), both on the ergodic subset, so both wait on DS-1's definition requirement | `../2026-10-28_external_sources/dandekar_2026/table{2,4,B2}_reconciled.csv` | new: BinaryPlusSingle at a mass point | outcome classifier; a horizon of 10⁶ yr | distribution | data transcribed · proposed (29 Oct 2026) · tier (a), GPU explorer · the whole fit range lies past R-610's cap of 200: "not evaluated within cap" (R-636) until a raised-cap run · scores in M9 · outcome PASS / EXPLAINED DIVERGENCE / UNEXPLAINED · E monitored to 10⁻⁵ only, as DS-1 |
```

### 7. PLAN.md, § 3.2: after the row `AN-1`, add

```
| AN-2 (proposed, 29 Oct 2026) | R | B | Anosova 1986 (Ap&SS 124, 217, doi:10.1007/BF00656037), Table V column Ia, eqs (1)–(2): `../2026-10-28_external_sources/README.md` § "Anosova 1986"; beside AN-1 | The distribution of escape time over ten bins (0–10, 10–20, …, 300–400, > 400 τ) for "plane problem; equal masses; k₀ = 0, L = 0" (free fall): 0.168, 0.081, 0.078, 0.066, 0.057, 0.212, 0.216, 0.085, 0.028, 0.009. Coarse: the number of runs and the escape distance (20d or 30d) are not stated | `../2026-10-28_external_sources/anosova_1986/table5_reconciled.csv` (two readings, 0 disagreements in 50 cells); the paper is local-only | Anosova entry | outcome classifier | distribution | data transcribed · proposed (29 Oct 2026) · tier (a), GPU explorer · waits on RQ-651 (the time unit τ) · bins past R-610's cap of 200 are "not evaluated within cap" (R-636) · scores in M9, or earlier as an information-only peek · outcome PASS / EXPLAINED DIVERGENCE / UNEXPLAINED |
```

### 8. Mikkola & Merritt 2008: no row

PLAN.md has no row for integrator stress tests; the nearest, H-1 and H-2, are tier C integrator comparisons. No row is
proposed; whether one belongs there is for the porter.

### 9. `REVIEW_QUEUE.md`: add two entries

Numbering: the dispatch said to start at RQ-617 (the highest on any remote branch when it was written was RQ-616). By
the time of this commit, `origin/rulings/2026-10-29-r637` had taken RQ-617 to RQ-619 and task 15b's
`origin/data/research-batches-2026-10-29` RQ-640 to RQ-642, so these two take **RQ-650 and RQ-651**, clear of both.
**If either is taken when this is applied, renumber to the next free ones**, and the references above with them.

```
## RQ-650: Which frame Tanikawa's free-fall papers use: the repo says "the mirror labelling D11 = {(x−0.5)² + y² ≤ 1}", three of the papers say Anosova's region and one prints the mirror *(physics; `docs/incoming/2026-10-25_replication/catalogue.md` line 13; `docs/design/principia_dd_replication.md` line 564; PLAN.md line 707; context: TM-1, TS-1, TS-2, proposed TSM-1 and TUA-1)*

*Queued 29 Oct 2026 by the agent ingesting the external sources (`docs/incoming/2026-10-28_external_sources/README.md` §
"Corrections and contradictions").*

- **File, section, quoted:**
  - `docs/incoming/2026-10-25_replication/catalogue.md` line 13: "Tanikawa's papers use D11 = {x ≥ 0, y ≥ 0,
    (x−0.5)² + y² ≤ 1}, the mirror labelling."
  - `docs/design/principia_dd_replication.md` line 564 and PLAN.md line 707: "Tanikawa uses the mirror labelling D11."
  - Tanikawa, Umehara & Abe 1995, eq. (1): "Let mass points m₂ and m₃ stand still at A(−0.5, 0) and B(+0.5, 0) … m₁
    stand still at a point P(x, y) where (x, y) ∈ D = {(x, y) : x ≥ 0, y ≥ 0, (x + 0.5)² + y² ≤ 1}."
  - Tanikawa, Saito & Mikkola 2019, §3: the same D, and "D11 corresponds to Anosova's region".
  - Tanikawa & Mikkola 2008 (arXiv:0802.2465; committed at
    `docs/incoming/2026-10-25_replication/papers/tanikawa_mikkola_2008_arXiv0802.2465.pdf`), p. 3: "We want to extend
    the free fall problem with equal masses (Anosova & Orlov 1986; Tanikawa et al. 1995), in which bodies 2 and 3
    initially stand still at (−0.5, 0) and (0.5, 0), respectively, whereas body 1 stands still at (x, y) ∈ D11 =
    {x ≥ 0, y ≥ 0, (x − 0.5)² + y² ≤ 1}."
  - Tanikawa & Mikkola 2015 (PASJ 67, 115; the owner's copy, local-only, not committed:
    `docs/incoming/_local/tanikawa_mikkola_2015_PASJ_67_115.pdf`), p. 115-2, eq. (9): "Initially, body m2 stands still
    at (−0.5, 0), and body m3 stands still at (0.5, 0) of the (x, y)-plane. Body m1 stands still at any position in
    D = {(x, y) : x ≥ 0, y ≥ 0, (x + 0.5)² + y² ≤ 1}."; Fig. 1's caption, p. 115-3: "Anosova's D region is equivalent
    with any of them, say, region D11."
- **What is settled:** the 1995 and 2019 papers use the Agekyan–Anosova frame itself, so TSM-1 and TUA-1 need no remap;
  task 15b's record (`docs/incoming/2026-10-29_research_batches/README.md` § 5.1, unmerged) finds the same for Li et
  al. 2021, Tanikawa & Umehara 1998, Tanikawa 2000 and Martynova & Orlov 2014, and its § 10.4 holds TM-1's remap
  wording for the owner's confirmation; this entry is the question that confirmation answers.
- **Also read:** Tanikawa & Mikkola 2015 uses Anosova's region, (x + 0.5)², like the 1995 and 2019 papers. Tanikawa
  & Mikkola 2008 prints (x − 0.5)², very likely the source of the repo's "mirror labelling" sentence; it is probably a
  typo, since the paper cites Tanikawa et al. 1995 (whose eq. (1) prints (x + 0.5)²) for this very set-up, and the
  printed region does not keep AB the longest side: (1.4, 0.3) is inside it, with PA = 1.92 > AB = 1.
- **What is not:** whether TM2008's (x − 0.5)² is a typo or the frame its runs used, and which frame Tanikawa &
  Mikkola 2021 (TM-1's source, not read here) uses; so whether the rows' "D11 mirror remap" and the general sentence
  stand.
- **Options seen:** (a) a porter reads Tanikawa & Mikkola 2021's set-up section and corrects the sentence to name the
  papers that use each frame, with TM2008's (x − 0.5)² recorded as printed; (b) drop "mirror" and name D11 as TSM 2019
  and TM2015 define it.
- **Applied meanwhile:** nothing; the proposed rows TSM-1 and TUA-1 state their frame from their own papers.

## RQ-651: Anosova 1986's time unit τ: eq. (1) as printed is not a time *(physics; `docs/incoming/2026-10-28_external_sources/README.md` § "Anosova 1986"; context: proposed AN-2, AN-1)*

*Queued 29 Oct 2026 by the agent ingesting the external sources.*

- **File, section, quoted:** Anosova 1986 (Ap&SS 124, 217), p. 218, eq. (1) as printed: τ = G √(Σᵢ Mᵢ Σ_{i≠j} MᵢMⱼ) /
  (−2E)^{3/2}, the radical over both sums; eq. (2): d = G Σ_{i≠j} MᵢMⱼ / 2E. `docs/incoming/2026-10-27_papers/README.md`
  line 201 records Aarseth, Anosova, Orlov & Szebehely 1994b's τ = G M_t^{1/2}(Σ MᵢMⱼ)/(−2E)^{3/2}.
- **What is not settled:** eq. (1) gives G M^{3/2}/(−2E)^{3/2}, not a time; whether Σ_{i≠j} counts each pair once or
  twice; Table V's escape distance (20d or 30d, p. 229) and its number of runs.
- **Options seen:** (a) read τ as 1994b's (the radical over Σ Mᵢ only, each pair once); (b) treat AN-2 as shape-only
  (compare the normalised distribution up to a free time scale); (c) drop AN-2.
- **Applied meanwhile:** nothing; AN-2 is proposed as waiting on this.
```

## Disclosures

- **An email address in a User-Agent.** In one early batch of requests (to the Crossref, Zenodo and Stack Exchange APIs,
  while verifying licences), the User-Agent carried the owner's email address as a `mailto:` contact. It should not
  have. Every request since has used `Principia-ingest/1.0 (record check)`, and `fetch.sh` sends that, with no address.
- **`_local/` is ignored through `.gitignore` line 21** (`/docs/incoming/_local/`), not through `.git/info/exclude` as
  the dispatch said; `git check-ignore -v` on every local file prints `.gitignore:21:/docs/incoming/_local/`.
- **Rebuilt without the GPL archive.** The first build of this record (branch `data/external-sources-2026-10-28`,
  head a3b75269, never merged) committed `kapela_simo/choreo.zip`. This record supersedes it; the archive was never on
  `main`.
- **Owner copies placed per the dispatch:** `_local/tanikawa_saito_mikkola_2019/`, `_local/orlov_titov_shombina_2016/`,
  `_local/titov_2024/`, `_local/li_liao_2018/`; everything else under `_local/external_sources_2026-10-28/`.
- **Not read, copied or recorded:** the owner's scratch file in his Springer download folder, as he asked.
