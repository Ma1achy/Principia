# High-precision initial conditions for the precision ladder's test 6, 27 Oct 2026

A record, not authority (R-1, R-159). It holds the reference data that test 6 of
`../2026-10-26_precision_ladder/PROPOSAL_precision_ladder_and_taylor_2026-10-26.md` (§5, "Orbit-catalogue closure per
tier") needs above f64, traced to its primary sources, and the agreement between those sources and the operator's
`atlas-params.json`. The human's words:

> "Pull high-precision initial conditions from primary sources into docs/incoming, with citations: Hristov et al.'s
> 100- and 180-digit databases, Simó's 30-digit choreographies, Li & Liao's extra digits. Transcribe twice.
> Cross-check locally against atlas-params.json (its `ic_hp` and `period_hp` fields), and commit only the agreement
> numbers. … The 99 atlas rows with no listed source are closure tests only, and are never cited."

The human's follow-up of 27 Oct 2026, on those rows:

> "The 99 high-precision rows with no listed source: probably Hristov, Hristova & Tanikawa 2026. Their per-family
> counts (I 12, II 2, III 4, IVa 3, IVb 9, V 6, VIIa 1, VIIb 5, li_IB 43, li_IIC 12) equal the catalogue's
> stab_src = hristov2026 counts for the same families. Check them against the 971-orbit stability database from #299.
> If they match, record them as hristov2026 in the agreement table; if not, say so."

Every citation key below is from `../2026-10-26_orbit_catalogues/CITATIONS.md`, which since 27 Oct 2026 cites each
reference in the journal's wording, checked against Crossref, with the site's beside it (its "Checked against the
journal"). `atlas-params.json` (sha256
`a5378e2b…`) was read read-only where it lies and is not in the repo; nothing here quotes it. Only derived numbers
from it are committed.

## What was found, in short

- **Hristov et al.: four public databases, 100 to 180 digits.** Every atlas row traced to them (937 rows: 840 by
  their listed source, 97 with an empty source) is identical, digit for digit, to the database, except the third
  body's velocity on the 73 `hristov2023` rows, which the atlas rounds to about 120 digits.
- **Simó: no 30-digit source exists in public.** Simó's own data has 4 digits (plot files) and his papers 8. His
  initial conditions at 13 to 17 significant digits are public only through Paul Masson's site, which received them
  from Simó. The atlas's 344 "30-digit" Simó rows are those 16-digit values, scaled to unit masses at 30 digits:
  **the digits past about 16 are arithmetic, not data**. They can test no tier above f64.
- **Li & Liao: no extra digits exist in public.** Their tables (paper, GitHub, SJTU site) give 10 decimals. The 5 atlas
  rows attributed to `liliao2017` that carry 100 digits are identical to rows of Hristov, Hristova & Tanikawa's
  971-orbit database (`hristov2026`): the extra digits are Hristov's, not Li & Liao's.
- **The 99 rows with no listed source: 97 are Hristov, Hristova & Tanikawa's, 2 are not.** 97 are identical, digit
  for digit, to rows of the 971-orbit database, and are recorded as `hristov2026`: inferred by matching, since the
  atlas lists no source. Their families are the human's ten, with his counts, which sum to 97, not 99. The other 2
  (family I) are in no Hristov table: they are identical to Li & Liao 2017's published 10-digit values.
- **No data file is committed**, because no source states terms that allow redistribution (see "Terms"). The
  retrieval list (`sources.tsv`, `fetch.sh`), the transcription scripts and the agreement numbers are committed.

## Terms: why the data is not committed

None of the sources states a licence or terms of use: the five pages of `db2.fmi.uni-sofia.bg` (its root, `3body123`,
`3bodychor462`, `3bodychor164`, `3bodystab971`), Masson's page and site, the SJTU pages, and the
`sjtu-liao/three-body` GitHub repository (no licence file; the API returns 404 for it). The three Hristov arXiv papers
carry arXiv's non-exclusive distribution licence, which covers the papers, not the databases. Silence grants nothing,
and the precedent here (`../2026-10-26_orbit_catalogues/`, `../2026-10-26_belgrade_gallery/`) is to commit
third-party data only under stated terms. So the values stay out of the repo. `fetch.sh` retrieves them byte for byte,
with a sha256 for each file. Committing them would need the authors' permission or a ruling.

## Sources

Retrieved 27 Oct 2026. Every file is listed in `sources.tsv` with its URL and sha256.

| Source (CITATIONS.md key) | Where | Rows | Digits | Files |
|---|---|---|---|---|
| `hristov2022`: Hristov et al. (2022), *New families of periodic orbits for the planar three-body problem computed with high precision*, CEUR Workshop Proceedings 3191, 45–50, arXiv:2205.14709 | https://db2.fmi.uni-sofia.bg/3body123/ (the paper's ref. 9) | 123 solutions, (vx, vy, T, T*) | 150 correct digits (148–150 significant) | `datafiles/all_150_digits_123.txt`; `datafiles/sol1.txt`…`sol123.txt` |
| `hristov2023`: Hristov et al. (2023), *A database of high precision trivial choreographies for the planar three-body problem*, LNCS 13858, 171–180, arXiv:2210.00594 | https://db2.fmi.uni-sofia.bg/3bodychor462/ (the paper's ref. 22) | 462 trivial choreographies | 180 correct digits | `datafiles/digits180.txt`; `datafiles/sol1.txt`…`sol462.txt` |
| `hristov2024nc`: Hristov et al. (2024), *Searching for New Nontrivial Choreographies for the Planar Three-Body Problem*, Physics of Particles and Nuclei 55, 495–497, doi:10.1134/S1063779624030444 | https://db2.fmi.uni-sofia.bg/3bodychor164/ | 164 nontrivial and 259 trivial choreographies | 180 correct digits | `nontriv_180.txt`, `triv_180.txt` |
| `hristov2026`: Hristov, Hristova & Tanikawa (2026), *An extensive search for stable periodic orbits of the equal-mass zero angular momentum three-body problem*, New Astronomy 125, arXiv:2510.22802 | https://db2.fmi.uni-sofia.bg/3bodystab971/ (the paper's ref. 40) | 971 linearly stable i.c.s (685 distinct solutions) | 100 correct digits | `ics_971_100.txt`, `ics_971_100_num.txt`, and the four region files `Shubart_S_num.txt`, `Intermediate_Moth_num.txt`, `Eight_num.txt`, `Broucke_num.txt` |
| `simo2001`: Simó (2002), *Dynamical properties of the figure eight solution of the three-body problem*, Contemporary Mathematics 292, 209–228, doi:10.1090/conm/292/04926 | Simó's page http://www.maia.ub.es/dsg/3body.html and preprint http://www.maia.ub.es/dsg/2001/0104simo.ps.gz | 345 choreographies | 4 digits (`choreo3.tar.gz`: plot points, `dat.cir` and `dat.orb`); 8 digits for three orbits in the preprint's table | `choreo3.tar.gz` |
| Not in CITATIONS.md: Paul Masson, *Simó's Three-Body Choreographies in Action*, Analytic Physics (web page), https://analyticphysics.com/Gravitational%20Choreographies/Sim%C3%B3's%20Three-Body%20Choreographies%20in%20Action.htm | its data file https://analyticphysics.com/www/data/simo-initial-conditions.txt | 345 rows, (c1…c5) | 13–17 significant digits | `simo-initial-conditions.txt` |
| `liliao2017`: Li & Liao (2017), *More than six hundred new families of Newtonian periodic planar collisionless three-body orbits*, Sci. China PMA 60, 129511 | GitHub `sjtu-liao/three-body` at commit `b96b0193579a9952299d710ed616f529bb7cf898`, `three-body-pictures.md`; https://numericaltank.sjtu.edu.cn/three-body/three-body-movies.htm; the arXiv tables (1705.00527v4) | 695 equal-mass orbits, (v1, v2, T) | 10 decimals | `three-body-pictures.md`, `three-body-movies.htm` |

Notes on the table:

- **The Hristov databases' own record of their papers.** The `3bodystab971` page names the journal as "New Astronomy,
  Volume 125, July 2026" with the ScienceDirect link `https://www.sciencedirect.com/science/article/pii/S1384107626000114`,
  matching CITATIONS.md's `hristov2026`. The `3bodychor164` page gives `hristov2024nc`'s title as "Searching for New
  Nontrivial Choreographies for the Planar Three-Body Problem", where the atlas's reference list has "Nontrivial
  choreographies of the planar three-body problem"; the DOI, 10.1134/S1063779624030444, is the same.
- **`hristov2024nc`'s title, checked 27 Oct 2026: the atlas's title is not the journal's.** The publisher's own
  page for the DOI (`https://link.springer.com/article/10.1134/S1063779624030444`) could not be read from this
  session: it answers with a bot-check page ("Client Challenge"), which was not bypassed. The publisher's own record
  of the DOI, deposited with Crossref (`https://api.crossref.org/works/10.1134/S1063779624030444`, publisher
  "Pleiades Publishing Ltd", fetched 27 Oct 2026, sha256 `402d7072e9df39d57cd5dd0c7df12dc6f101f9eedee04d6d8488176088617395`),
  gives: I. Hristov, R. Hristova, I. Puzynin, T. Puzynina, Z. Sharipov, Z. Tukhliev, *Searching for New Nontrivial
  Choreographies for the Planar Three-Body Problem*, Physics of Particles and Nuclei 55(3), 495–497 (June 2024). That
  agrees with the authors' page and not with the atlas's reference list
  (`../2026-10-26_orbit_catalogues/threebodyorbits_sources.json`). **Cite it by the Crossref title.** CITATIONS.md
  now does, with the site's wording beside it: RQ-580, which recorded the discrepancy, was closed by the human's
  instruction of 27 Oct 2026, and the table above quotes the corrected CITATIONS.md.
- **Masson's page is not a publication.** Its title is the page's own `<title>`. Its author is from the site's footer
  ("Paul Masson — Independent Physicist — San Francisco"). The page says Simó "was able to locate the initial
  conditions for all 345 planar choreographies and kindly provided them to me". It is a secondary source: it is
  used here only to show where the atlas's Simó digits come from, never as a reference for a tier.
- **Simó's normalisation**: masses 1/3, energy −1/2, centre of mass at the origin. Masson's c1…c5 give
  x1 = −2c1, x2 = x3 = c1, y2 = −y3 = c2, u1 = 0, u2 = −u3 = c3, v1 = −2c4, v2 = v3 = c4, and the period 3·c5.
  With masses 1 and the same positions, velocities scale by √3 and the period by 1/√3.
- **The search for a 30-digit Simó source and for Li & Liao's extra digits** covered Simó's page, his 2001 preprints
  (0102simo, 0104simo), `choreo3.tar.gz`, Masson's page, Li & Liao's arXiv v4, their GitHub repository and the SJTU
  pages. Neither was found. Nothing was substituted for them.

## Transcription, twice

Two independent routes, each from the files as downloaded:

- **Route A** (`route_a.py`): each database's combined file, split on whitespace, each value read by Python's
  `decimal.Decimal` and written in a canonical form `0.<digits>e<exponent>`.
- **Route B** (`route_b.py`): a byte-level regular expression and a hand-built canonical form, with no `Decimal`, read
  from the database's **other** files where it has them: the 123 and 462 per-solution files, the 971 numbered file and,
  separately, the four region files of the 971. The two 2024 files and Masson's file have no second copy, so route B
  re-reads the same bytes by its own code.
- **Li & Liao** (`liliao_routes.py`): route A from the GitHub `three-body-pictures.md`, route B from the SJTU
  `three-body-movies.htm`, two copies published separately.

`compare_ab.py` compares them value by value. Its output is `transcription.txt`. **Both routes agree on every value:**

| Table | Rows | Values | Differing |
|---|---|---|---|
| `3body123` (combined file against 123 per-solution files) | 123 | 492 | 0 |
| `3bodychor462` (combined file against 462 per-solution files) | 462 | 1,848 | 0 |
| `3bodychor164`, nontrivial | 164 | 656 | 0 |
| `3bodychor164`, trivial | 259 | 1,036 | 0 |
| `3bodystab971` (`ics_971_100.txt` against `ics_971_100_num.txt`; the four region files hold the same 971 rows) | 971 | 3,884 | 0 |
| Masson's Simó file | 345 | 1,725 | 0 |
| Li & Liao 2017 (GitHub against SJTU) | 695 | 2,085 | 0 |

A third download of all 599 files by `fetch.sh` matched every recorded sha256, and both routes reproduced their output
byte for byte from it.

## Agreement with `atlas-params.json`

`crosscheck.py`, output in `agreement.txt`. Each atlas row is matched to its database row by (vx, vy) within 1e-12.
All twelve initial-condition components and the period are then compared at full precision: "identical" means equal
as decimals. Digits agreed is ⌊−log₁₀(|a − b| / |b|)⌋, or ⌊−log₁₀|a|⌋ where the reference is 0. For the Hristov and
`liliao2017` rows the six positions are compared by the layout test: a row is matched only if its positions are
exactly (−1, 0), (1, 0), (0, 0), and every one of them is. For the Simó rows each of the twelve is compared to
Masson's value, as `agreement.txt` lists.

The atlas has 1,359 rows with high-precision fields:

- **99 with an empty source**, matched against `3bodystab971` the same way, by the human's follow-up of 27 Oct 2026:
  97 identical, recorded below as `hristov2026`; 2 not (see "The 99 rows with an empty source").
- **76 from the atlas's own notes and the CAP repository** (`bh2026` 3, `bh2026r3` 14, `bh2026r4` 42, `bhh2026` 15,
  `pancake2026` 1, `mothcap2026` 1). Out of scope: they have no other source.
- **1,184 in scope**, all matched, none unmatched:

| Source | Atlas rows | Matched in | vx, vy (body at −1 and at +1) | Third body (−2vx, −2vy) | Period |
|---|---|---|---|---|---|
| `hristov2022` | 9 | `3body123` | identical 9/9 (149–150 digits) | identical 9/9 | identical 9/9 (150 digits) |
| `hristov2023` | 73 | `3bodychor462` | identical 73/73 (178–180 digits) | **0/73 identical: the atlas rounds them to 118–120 digits; they agree to 119–121 digits, max abs diff 4.9e-120** | identical 73/73 (179–180 digits) |
| `hristov2024nc` | 167 | 164 in `nontriv_180.txt`, 3 in `triv_180.txt` | identical 167/167 (178–180 digits) | identical 167/167 | identical 167/167 (177–180 digits) |
| `hristov2026` | 586 | `3bodystab971` | identical 586/586 (97–100 digits) | identical 586/586 | identical 586/586 (97–100 digits) |
| `liliao2017` | 5 | **`3bodystab971`** | identical 5/5 (98–100 digits) | identical 5/5 | identical 5/5 (100 digits) |
| `hristov2026`, **inferred by matching**: empty source in the atlas | 97 of the 99 | `3bodystab971` (93 distinct rows) | identical 97/97 (99–100 digits) | identical 97/97 | identical 97/97 (99–100 digits) |
| `simo2001` | 344 | Masson's file (344 of 345; Simó's no. 1 has no atlas row) | positions: all six (x1 = −2c1, y1 = 0, x2 = x3 = c1, y2 = c2, y3 = −c2) identical 344/344 (11–17 significant digits); velocities: vx1 = 0 identical 344/344, vx2 = c3√3 agrees to 29–32 digits, vy1 = −2c4√3 to 28–32, vy2 = c4√3 to 27–32 (median 30 for each), max abs diff 5.0e-31 | vx3 = −c3√3 agrees to 29–32 digits, vy3 = c4√3 to 27–32 (median 30 for each), max abs diff 5.0e-31 | agrees to 30–34 digits (median 31), max abs diff 5.0e-31 |

Li & Liao's own published values for those 5 orbits, against the 100-digit ones: v1 to 8–10 digits (max abs diff
5.4e-10), v2 to 9–10 (1.4e-10), T to 9–10 (8.7e-8). Every published T, and v1 or v2 on three of the five (I.A 85,
104 and 112), differ from the 100-digit values by more than half a unit in the tenth decimal: the tables are not
correct to the last printed digit.

### The 99 rows with an empty source

The atlas lists no source for them, so their attribution is inferred by matching. Each row's family and stability
source are the catalogue's (`../2026-10-26_orbit_catalogues/threebodyorbits_catalogue.json`), joined by key:

| Family | Rows with an empty source | Identical to `3bodystab971`, all seven values | The catalogue's `stab_src` on them |
|---|---|---|---|
| `li_IB` | 43 | 43 | `hristov2026` 43 |
| `I` | 14 | 12 | `hristov2026` 12, `liliao2017-10digit` 2 |
| `li_IIC` | 12 | 12 | `hristov2026` 12 |
| `IVb` | 9 | 9 | `hristov2026` 9 |
| `V` | 6 | 6 | `hristov2026` 6 |
| `VIIb` | 5 | 5 | `hristov2026` 5 |
| `III` | 4 | 4 | `hristov2026` 4 |
| `IVa` | 3 | 3 | `hristov2026` 3 |
| `II` | 2 | 2 | `hristov2026` 2 |
| `VIIa` | 1 | 1 | `hristov2026` 1 |
| **Total** | **99** | **97** | `hristov2026` 97, `liliao2017-10digit` 2 |

- **The 97 that match are exactly the human's counts**, family by family, and exactly the catalogue's `stab_src =
  hristov2026` orbits (97 in the whole catalogue, all among these rows). Every one of the seven values compared (vx and
  vy of the bodies at −1 and +1, the third body's, and the period) is identical to the database's at 99–100 digits.
  They are recorded as `hristov2026`. The human's counts sum to 97, not 99: the 99 include 2 more.
- **The other 2 do not match**: `i_butterfly_i_8_a` and `i_butterfly_i_11_a`, family I, whose catalogue `stab_src` is
  `liliao2017-10digit`. No row of any Hristov table is within 1e-12 of them. All seven of their values are identical
  to Li & Liao 2017's published values for I.A-10 and I.A-19 (10 decimals, as `liliao_routes.py` transcribes them):
  their high-precision fields carry those 10 digits and no more. They are not `hristov2026`.

## What it means for test 6

- **Rows that can test tiers above f64: the 937 rows traced to Hristov et al.** 688 of them have 97–100 digits
  (`hristov2026`'s 586, the 97 with an empty source and the 5 filed under `liliao2017`), 9 have 150 and 240 have 180.
  Use the database's values, not the atlas's. For the 73 `hristov2023` rows, form the third body as −2(vx, vy) from
  the database: the atlas's ~120-digit third body is enough for every rung below `mp` at 120 digits, but not above.
- **The 344 Simó rows carry about 16 significant digits of data**, from Simó through Masson. Closure from them cannot
  fall below roughly 1e-16 at any tier, so they test nothing above f64. The proposal's "the 30-digit ones cannot test
  `dd64` (~32 digits)" understates it. Above f64 they are useful only after re-converging by Newton at the tier's
  precision, as the proposal says for rows with no high-precision values.
- **The 5 `liliao2017` rows** count as `hristov2026` data for precision. Cite Li & Liao for the orbit and Hristov,
  Hristova & Tanikawa for the digits.
- **The 97 rows with an empty source that match** are `hristov2026` data too: cite Hristov, Hristova & Tanikawa for
  the digits, saying the attribution is inferred by matching, and the family's source in CITATIONS.md for the orbit.
- **The 2 rows with an empty source that do not match** carry Li & Liao's 10 digits, fewer than f64 holds, so they
  cannot test even f64 as they stand; they are useful only after re-converging, like any row without high-precision
  values.

## Files

| Path | sha256 | What it is |
|---|---|---|
| `sources.tsv` | `247c7e2c1ce4a66570d7f9965c4f272d52c7aa0c544a3ae5b0a35bb5b095c3b4` | Every source file: local path, URL, sha256 at retrieval (27 Oct 2026). 599 files |
| `fetch.sh` | `f4e36de51a37c37554dd094640132310e637da6ca3e490b775eed3f65903a364` | Downloads `sources.tsv` into an empty directory and checks each hash |
| `route_a.py` | `1b1da53623968c57b2b0bab66eb578e8bcf729f52b5272b06f342a2ae92df0ba` | Transcription, route A |
| `route_b.py` | `3bf4be2f465a9e3f3cfc19ecc28d48a45c460d89842ac4353af8c5060903ab69` | Transcription, route B |
| `compare_ab.py` | `ba88de4d3fcfc188d5284b1b942fbf5251ed335c92fff966011e02c01a862eca` | Compares the two routes value by value |
| `liliao_routes.py` | `85ece4c09952178c0f0de3ee82040c1fac4699b1d04373a0798ee38ab2c73c5a` | Li & Liao 2017's tables, transcribed twice and compared |
| `crosscheck.py` | `7bc23fba3990c65675277c8a7e5c17e0f4216343c14fcd959748a203744d426d` | The cross-check against `atlas-params.json`, and since 27 Oct 2026 of its 99 rows with an empty source against `3bodystab971`. Prints derived numbers only |
| `transcription.txt` | `62b53f859518acd9b9f3a1ec1fbed173530d377b72420cf3e4db0b84023bfc5b` | `compare_ab.py`'s and `liliao_routes.py`'s output |
| `agreement.txt` | `73b23f0e7c51266ad04a83f047060459622b81af2a695078a822c6e0b219d9e7` | `crosscheck.py`'s output |

To reproduce (the downloads are untrusted data: run nothing from them, keep them out of the repo):

```
bash fetch.sh <empty dir>
python3 -I route_a.py <empty dir> a.json
python3 -I route_b.py <empty dir> b.json
python3 -I compare_ab.py a.json b.json
python3 -I liliao_routes.py <empty dir>/liliao_github/three-body-pictures.md <empty dir>/liliao_site/three-body-movies.htm ll.json
python3 -I crosscheck.py <path to atlas-params.json> a.json ll.json ../2026-10-26_orbit_catalogues/threebodyorbits_catalogue.json
```

Run these from this directory, with the output files outside the repo.
