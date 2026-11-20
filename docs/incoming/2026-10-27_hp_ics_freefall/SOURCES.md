# Sources of the committed free-fall, central-symmetry and Euler databases and Li & Liao's free-fall tables (`data/`)

Committed by R-631 (27 Oct 2026), which amends R-630 item 6 and closes RQ-579 differently: "commit the
high-precision initial-condition data to the repo, with full citations." Every file under `data/` is the file
`fetch.sh` downloads, byte for byte, at the path `sources.tsv` gives it. Each was fetched on 27 Oct 2026 three times,
into three empty directories, and all three copies had the same sha256, recorded in `sources.tsv` and below; the
third was by `fetch.sh` itself ("all 10 files fetched and match sources.tsv"). Nothing here is ours: the values are
the authors' published numerical results, credited to them below. No file is over 20 MB, so none is compressed.

Where a paper states a licence it is given, as Crossref's record of the DOI lists it, checked 27 Oct 2026. No
database page, and no data file, states terms of its own (R-631's reasons; `README.md`, "Terms").

## Hristov et al. 2024, free fall

- **Authors:** Ivan Hristov, Radoslava Hristova, Veljko Dmitrašinović, Kiyotaka Tanikawa
- **Title:** *Three-body periodic collisionless equal-mass free-fall orbits revisited*
- **Journal:** Celestial Mechanics and Dynamical Astronomy 136, 7 (2024)
- **DOI:** https://doi.org/10.1007/s10569-023-10177-w
- **arXiv:** arXiv:2308.16159
- **Database:** https://db2.fmi.uni-sofia.bg/3bodyfree/ (24,582 i.c.s at 80 correct digits, (x, y, T, T*), ordered by T*; and the 236 self-dual solutions)
- **Fetched:** 27 Oct 2026
- **The paper's licence:** CC BY 4.0 (Crossref: https://creativecommons.org/licenses/by/4.0)
- published by the authors; included as test data with citation; not ours
- **Files (2):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `sofia_3bodyfree/sol_80.txt` | `81dc1b70828fd5960dec57f64ac88f7945fa5c8eddf1cfb6ea33b3475c55373c` | https://db2.fmi.uni-sofia.bg/3bodyfree/sol_80.txt |
| `sofia_3bodyfree/sd_80.txt` | `e37786b8a09432c519c07230cd03059651cfcfe3895435c85ba84c6471e7fd8e` | https://db2.fmi.uni-sofia.bg/3bodyfree/sd_80.txt |

## Hristov et al. 2025, central symmetry

- **Authors:** Ivan Hristov, Radoslava Hristova, Taisya Puzynina, Zarif Sharipov, Zafar Tukhliev
- **Title:** *Numerical search for three-body periodic free-fall orbits with central symmetry*
- **Journal:** Communications in Nonlinear Science and Numerical Simulation 151, 109066 (December 2025)
- **DOI:** https://doi.org/10.1016/j.cnsns.2025.109066
- **arXiv:** arXiv:2503.00432
- **Database:** https://db2.fmi.uni-sofia.bg/3bodysym4860/ (4,860 orbits at 100 correct digits: the Euler half-twist i.c.s (vx, vy, T, T*) and the free-fall i.c.s (px, py, T, T*), each ordered by T*)
- **Fetched:** 27 Oct 2026
- **The paper's licence:** CC BY 4.0 (Crossref: http://creativecommons.org/licenses/by/4.0/)
- published by the authors; included as test data with citation; not ours
- **Files (2):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `sofia_3bodysym4860/free_100_4860.txt` | `74a23a8392a4ec75b3e855973a6d4cfcacc90de84784688f516a366d6185dccb` | https://db2.fmi.uni-sofia.bg/3bodysym4860/free_100_4860.txt |
| `sofia_3bodysym4860/euler_100_4860.txt` | `e93d789a7c1dff2931ff2772754d4072d464704ec35cb9f466274bfc5d2977d3` | https://db2.fmi.uni-sofia.bg/3bodysym4860/euler_100_4860.txt |

## Hristov & Hristova 2024, Euler configuration

- **Authors:** Ivan Hristov, Radoslava Hristova
- **Title:** *An efficient approach for searching three-body periodic orbits passing through Eulerian configuration*
- **Journal:** Astronomy and Computing 49, 100880 (October 2024)
- **DOI:** https://doi.org/10.1016/j.ascom.2024.100880
- **arXiv:** arXiv:2404.16526
- **Database:** https://db2.fmi.uni-sofia.bg/3bodyeuler/ (12,431 i.c.s at 100 correct digits, (vx, vy, T, T*), ordered by T*; the additional 332 near the E = 0 curve, the 35 free-fall and the 8 stable i.c.s)
- **Fetched:** 27 Oct 2026
- **The paper's licence:** CC BY 4.0 (Crossref: http://creativecommons.org/licenses/by/4.0/)
- published by the authors; included as test data with citation; not ours
- **Files (4):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `sofia_3bodyeuler/100_12431.txt` | `a9aecab26fd24b2e0d4ef6b892430a25c698edf0b0783665c4c6a0ad77b278fa` | https://db2.fmi.uni-sofia.bg/3bodyeuler/100_12431.txt |
| `sofia_3bodyeuler/add_332_100.txt` | `492e8a3a546cdd1103667257f2f6eb56bf8b9e1f0618972c9467b76e9eb9d744` | https://db2.fmi.uni-sofia.bg/3bodyeuler/add_332_100.txt |
| `sofia_3bodyeuler/100_free.txt` | `37e0234cc3ffd784aeaf95f7890dc936d4dfa34be3f119fdc3cf6d28e767cd12` | https://db2.fmi.uni-sofia.bg/3bodyeuler/100_free.txt |
| `sofia_3bodyeuler/100_stable.txt` | `9cf585bb35c6237a0ac9be9753f8aaf650c4d25f372d06e65b0738281e37f88c` | https://db2.fmi.uni-sofia.bg/3bodyeuler/100_stable.txt |

## liliao2019

- **Authors:** Xiaoming Li, Shijun Liao
- **Title:** *Collisionless periodic orbits in the free-fall three-body problem*
- **Journal:** New Astronomy 70, 22–26 (2019)
- **DOI:** https://doi.org/10.1016/j.newast.2019.01.003
- **arXiv:** arXiv:1805.07980
- **Database:** GitHub `sjtu-liao/three-body` at commit `b96b0193579a9952299d710ed616f529bb7cf898` (no LICENSE file: the API answers 404): the 316 free-fall orbits, (x, y, T) at 10 decimals, and their free-group words
- **Fetched:** 27 Oct 2026
- **The paper's licence:** none stated as open: Crossref lists only Elsevier's user licence
- published by the authors; included as test data with citation; not ours
- **Files (2):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `liliao_github/free-fall-3b-pictures.md` | `7fc7b577d609589d0c82ce35cb46964030639d53e8f494a427d66c838d179c81` | https://raw.githubusercontent.com/sjtu-liao/three-body/b96b0193579a9952299d710ed616f529bb7cf898/free-fall-3b-pictures.md |
| `liliao_github/free-fall-3b-free-group-word.md` | `074da671c0cfe57273f0d4dc9361ce21e140ef0c95891075b1a027ab5ec69139` | https://raw.githubusercontent.com/sjtu-liao/three-body/b96b0193579a9952299d710ed616f529bb7cf898/free-fall-3b-free-group-word.md |

## Checking the committed files

From this folder: `cd data && tail -n +2 ../sources.tsv | awk -F'\t' '{print $3"  "$1}' | shasum -a 256 -c`
checks every committed file against its recorded sha256. `bash fetch.sh <empty dir>` stays the provenance
check: it shows that the sources still serve the same bytes.
