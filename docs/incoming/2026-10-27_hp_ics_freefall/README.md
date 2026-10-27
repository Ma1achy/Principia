# The free-fall, central-symmetry and Euler databases, and Li & Liao's free-fall tables, 27 Oct 2026

A record, not authority (R-1, R-159). It holds, committed, the high-precision initial conditions R-630 item 7 has
fetched for REQ-VAL-042 and R-631 has committed, beside the two other databases R-631 names from the same site, and the
authors' own table of Li & Liao's 316 free-fall orbits, which REQ-VAL-227 (LL-5) reads. The sister record
`../2026-10-27_hp_ics/` holds the four databases, Simó's archive and Li & Liao's 2017 tables that test 6 of the precision
ladder needed; this folder follows its layout (`sources.tsv`, `fetch.sh`, `data/<source>/<file>`).

The human's words, R-630 (27 Oct 2026), item 7:

> "7. Fetch Hristov et al. 2024's free-fall database (24,582 ICs, 80 digits) under the fetch-and-verify rule, for
> REQ-VAL-042."

and R-631, the same evening, which amends R-630 item 6:

> "Owner's ruling, amending R-630 item 6 and closing RQ-579 differently: commit the high-precision initial-condition
> data to the repo, with full citations. No permission request is required first; the owner will send a courtesy email
> himself (drop the draft from #310's description, or mark it superseded).
>
> Reasons on record (checked 27 Oct): no database page states terms; the describing papers for the free-fall 24,582
> (CeMDA 136, 7, 2024), central-symmetry 4,860 (CNSNS 2025), Euler 12,431 (Astron. Comput. 2024) and stable 971 (New
> Astron. 2026) are CC BY 4.0; the data are published numerical results, credited to their authors and never claimed
> as ours.
>
> 1. docs/incoming/2026-10-27_hp_ics (and a new folder for the free-fall 24,582 if it isn't there): run fetch.sh,
>    verify each file against its recorded sha256, and commit the files themselves.
> 2. Same for Simó's choreo3.tar.gz (345 choreographies; its README carries no terms) and the sjtu-liao/three-body
>    tables (no LICENSE file).
> 3. Each folder gets a SOURCES.md (or README update): authors, title, journal, DOI, arXiv id, database URL, fetch
>    date, sha256 per file, the paper's licence where it has one, and a line "published by the authors; included as
>    test data with citation; not ours". Correct the first author's name everywhere: Ivan Hristov (the draft said
>    "Ivaylo").
> 4. Update REQ-VAL-042, PL-6 and LL-5 to point at the committed files instead of fetch-only. Keep fetch.sh as the
>    provenance check.
> 5. If a dataset is very large (say > 20 MB), commit it compressed and say so.
> 6. Usual docs/ commit rules, one PR, leave it for the owner."

Citations, licences and every file's sha256 are in `SOURCES.md`. The first author is Ivan Hristov throughout, as the
database pages and Crossref give him; no file in the repo, or on PRs #307, #309 and #310, named him otherwise.

## What is here

Fetched 27 Oct 2026 from `https://db2.fmi.uni-sofia.bg/` (whose root page lists the eight databases) and from the
`sjtu-liao/three-body` GitHub repository at commit `b96b0193579a9952299d710ed616f529bb7cf898`. Each file was fetched
three times into empty directories; all three copies of each had the same sha256, recorded in `sources.tsv`. Row
counts are the files' line counts (each row is one initial condition, its values separated by spaces).

| Source | Database page | File (under `data/`) | Rows | Values per row | Size |
|---|---|---|---|---|---|
| Hristov, Hristova, Dmitrašinović & Tanikawa 2024 (*CeMDA* 136, 7), the free-fall orbits | `3bodyfree/` | `sofia_3bodyfree/sol_80.txt` | 24,582 | x, y, T, T* at 80 correct digits, ordered by T* | 8.4 MB |
| the same | `3bodyfree/` | `sofia_3bodyfree/sd_80.txt` | 236 | the self-dual (symmetric) solutions, as above | 81 kB |
| Hristov, Hristova, Puzynina, Sharipov & Tukhliev 2025 (*CNSNS* 151), central symmetry | `3bodysym4860/` | `sofia_3bodysym4860/free_100_4860.txt` | 4,860 | px, py, T, T* at 100 correct digits: the free-fall i.c.s | 2.1 MB |
| the same | `3bodysym4860/` | `sofia_3bodysym4860/euler_100_4860.txt` | 4,860 | vx, vy, T, T* at 100 correct digits: the Euler half-twist i.c.s | 2.1 MB |
| Hristov & Hristova 2024 (*Astron. Comput.* 49), Euler configuration | `3bodyeuler/` | `sofia_3bodyeuler/100_12431.txt` | 12,431 | vx, vy, T, T* at 100 correct digits | 5.3 MB |
| the same | `3bodyeuler/` | `sofia_3bodyeuler/add_332_100.txt` | 332 | the additional i.c.s close to the E = 0 curve, as above | 141 kB |
| the same | `3bodyeuler/` | `sofia_3bodyeuler/100_free.txt` | 35 | the free-fall i.c.s, as above | 15 kB |
| the same | `3bodyeuler/` | `sofia_3bodyeuler/100_stable.txt` | 8 | the stable i.c.s, as above | 3 kB |
| Li & Liao 2019 (*New Astron.* 70), the authors' table | GitHub | `liliao_github/free-fall-3b-pictures.md` | 316 orbits on nine mass sets | F_i(m₁, m₂, m₃), x, y, T at 10 decimals; bodies at (−0.5, 0), (0.5, 0), (x, y), at rest | 119 kB |
| the same | GitHub | `liliao_github/free-fall-3b-free-group-word.md` | 316 | each orbit's free-group word | 24 kB |

The total is 18.2 MB (18,179,212 bytes). No file is over 20 MB, so nothing is compressed (R-631 item 5).

- **The 316 orbits of Li & Liao's GitHub table** are on the nine mass sets of `../2026-10-27_validation_plan/PLAN.md`
  § 2.1, with its counts: (1, 1, 1) 30, (1, 0.8, 0.8) 29, (1, 0.8, 0.6) 69, (1, 0.8, 0.4) 44, (1, 0.8, 0.2) 32,
  (0.6, 0.8, 1) 30, (1, 0.1, 0.01) 48, (0.393, 0.288, 0.662) 19, (1, 0.412, 0.267) 15. So the 82 website-only orbits,
  of which the atlas carries 81 (LL-5), have published initial conditions after all, at 10 decimals, here. PLAN.md
  § 2.1 read the SJTU website's word table, which "gives words only, no initial conditions"; this GitHub table is the
  authors' other copy, and it carries them. It is recorded here and changes no target: LL-5 stays a closure test on
  the atlas's `ref_*` (rule 13).
- **The Euler database's 35 free-fall i.c.s and the central-symmetry database's 4,860 free-fall i.c.s** are free-fall
  orbits beside the 24,582, from the same group; how they overlap the 24,582 is not checked here.

## Terms

Committed under R-631. None of the three database pages (`3bodyfree/`, `3bodysym4860/`, `3bodyeuler/`, fetched 27 Oct
2026, sha256 below) states a licence or terms of use: a search of each for "licen", "terms", "copyright",
"permission" and "©" finds nothing, and the `sjtu-liao/three-body` repository has no licence file (the
GitHub API answers 404). The three papers describing the Sofia databases are CC BY 4.0, as Crossref's records of
their DOIs state; Li & Liao 2019's is not open (Crossref lists Elsevier's user licence only). The data are the
authors' published numerical results, credited to them in `SOURCES.md`: published by the authors; included as test
data with citation; not ours.

The pages read for terms, with their sha256 at 27 Oct 2026 (not committed: they are the databases' index pages):
`https://db2.fmi.uni-sofia.bg/` `99feeecaf63c56e9143dcb06a2b34dfb992071a40b7dfbdd216974a30a7cb4d6`,
`…/3bodyfree/` `f9eeeec92d423d47d98c6426b8f0b6a6a3e789bb27aa6590c67861b6ad398a41`,
`…/3bodysym4860/` `012d67a18b08ba8700d0d6becb89564c234652fbfd450cda1e95a4234b7a698d`,
`…/3bodyeuler/` `d29491562e6d3837f6524c8cc780c809ab092b081c4c85cbcd81fb430db02f35`.

## Not fetched

- **From the same three database pages:** the plots and animations (`pictures_600.tar.gz`, `pictures_sym.tar.gz`,
  `pic_euler.zip`, `pic_freefall.zip`, `animations.zip`, `pictures_200.zip`, `pictures_ff.zip`,
  `pictures_stable.zip`), the 35-digit supplement (`supplement.pdf`), the stability files (`res_eigen.txt`,
  `res_exponents.txt`, `eigenvalues_12431.txt`), the syzygy files (`syzygies.txt`, `syzygies.pdf`, `syz_12431.txt`),
  the 236 per-solution copies of `sd_80.txt` (`table/data1.txt`…) with their plots, and the Euler page's later
  addition of 421,562 i.c.s at 60 digits (`60digits.txt`, about 109 MB, and `400_thousand.pdf`). None is initial-condition
  data a requirement names, and the 421,562 are not among the datasets R-631 names.
- **The root page's other database**, `3body821` (821 satellites of the figure-eight orbit), which R-631 does not name.
- **The rest of `sjtu-liao/three-body`:** its other tables (the 3D and unequal-mass tables, the free-group-word tables
  of the 2017 and 2018 papers, the non-hierarchical supplementary data), its images, models and papers. No requirement
  reads them; Li & Liao 2017's table is in `../2026-10-27_hp_ics/data/liliao_github/`.
- **Li & Liao's SJTU free-fall pages** (`http://numericaltank.sjtu.edu.cn/three-body/free-fall-3b/free-fall-3b.htm`
  and its word table), recorded by URL and sha256 prefix in PLAN.md § 2.1, stay recorded only (R-630 item 8, which
  R-631 does not amend). The GitHub table above carries the same 316 orbits with their initial conditions.

## R-630 item 9: the three papers, all paywalled, listed for the owner

R-630 item 9: "Fetch Tanikawa & Mikkola 2021, Standish 1970 and Yasko & Orlov 2015 (TM-1, LL-3). If any is paywalled,
list it for the owner to fetch instead." Checked 27 Oct 2026: none has an open copy. OpenAlex reports each as closed
("oa_status": "closed", no repository full text), arXiv has none of them (a search of its records by author and title),
and Crossref lists only the publishers' own licences. So no PDF is committed, and each is for the owner to fetch:

| Paper | DOI | Target | Licence found |
|---|---|---|---|
| Kiyotaka Tanikawa & Seppo Mikkola, *Numerical confirmation of the existence of triple collision orbits inside the domain of the free-fall three-body problem*, Celestial Mechanics and Dynamical Astronomy 133, 52 (2021) | https://doi.org/10.1007/s10569-021-10050-8 | TM-1 (REQ-VAL-240, TASK-M3-49) | Springer's text-and-data-mining licence only |
| E. Myles Standish, *New Periodic Orbits in the General Problem of Three Bodies*, in G. E. O. Giacaglia (ed.), *Periodic Orbits, Stability and Resonances*, 375–381 (Springer Netherlands, 1970) | https://doi.org/10.1007/978-94-010-3323-7_34 | LL-3 (REQ-VAL-223, TASK-M3-47) | none listed |
| P. P. Yasko & V. V. Orlov, *Search for periodic orbits in Agekyan and Anosova's region D for the general three-body problem*, Astronomy Reports 59, 404–413 (2015) | https://doi.org/10.1134/S106377291505008X | LL-3 (REQ-VAL-223, TASK-M3-47) | Springer Nature's text-and-data-mining licence only |

Standish's chapter is the one Li & Liao 2019 cite as their ref. 25 ("E. Standish, Giacaglia, G.E.O. (eds) Periodic
Orbits, Stability and Resonances (Springer Netherlands, 1970)", `../2026-10-24_anosova/papers/li_liao_2018_arXiv_1805.07980.pdf`);
Crossref's record of that book lists one chapter by Standish, this one. TM-1 and LL-3 wait for the owner's copies
(R-630 item 9; TASK-M3-47 and TASK-M3-49).

## Files

| Path | What it is |
|---|---|
| `sources.tsv` | Every source file: local path under `data/`, URL, sha256 at retrieval (27 Oct 2026). 10 files |
| `fetch.sh` | Downloads `sources.tsv` into an empty directory and checks each hash: the provenance check |
| `data/` | The 10 files themselves, as fetched |
| `SOURCES.md` | Each source's citation, its paper's licence, and every file's sha256 |
| `.gitattributes` | `data/**` is never line-ending converted, so the committed bytes keep their hashes |

To check the committed files, from this folder:

```
cd data && tail -n +2 ../sources.tsv | awk -F'\t' '{print $3"  "$1}' | shasum -a 256 -c
```

To check that the sources still serve the same bytes (the downloads are untrusted data: run nothing from them):

```
bash fetch.sh <empty dir>
```
