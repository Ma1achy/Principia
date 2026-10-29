# RQ-598: the papers, the data hunt, and the human's rulings, 28 Oct 2026

A record, not authority (R-1, R-159). On 28 Oct 2026 the human delivered five PDFs for RQ-598. Its targets are BR-1
and GG-1, defined in `../2026-10-27_validation_plan/PLAN.md` §3.2, and V-11, defined in `../2026-10-27_papers/TARGETS.md`.
RQ-598 itself also cites `docs/design/principia_dd_replication.md` §5.10 and §5.12, which are on the unmerged
`port/R-594-replication` (#309). He asked two things: whether the two data sets the targets need are
public anywhere, and for the work below to go to the agent as a PR. The data hunt was done in a claude.ai session.
Its findings are in §2. The human's rulings are in §3, quoted as he gave them. The work they start is in §4.

Every file in `papers/` is byte-for-byte as delivered.

## 1. The batch

| Delivered as | sha256 | Identity | Verdict |
|---|---|---|---|
| `sty237.pdf` | `625a5494b0709c2bda4fae11e6dbacf6bfb6c63b6e0caa13c1cbcb653f63b55c` | A. Mylläri, M. Valtonen, A. Pasechnik & S. Mikkola, "Stability of hierarchical triples – I. Dependence on inner eccentricity and inclination", *MNRAS* 476(1), 830–841 (2018), doi:10.1093/mnras/sty237. Publisher PDF, text layer. Source for V-11 | **New, not committed** (below) |
| `Stability_of_hierarchical_triples___II_…_surface.pdf` | `76eab52c4473695e311bb836a53cea1936d53e3d09eb4cc7ba965a573e6662c7` | A. Pasechnik, A. Mylläri, M. Valtonen & S. Mikkola, "Stability of hierarchical triples – II. The inclination ι = 140° resonance in the stability surface", *MNRAS* 525(2), 1929–1935 (2023), doi:10.1093/mnras/stad2372. Self-archived parallel-published version from the University of Turku repository. States "The data underlying this article will be shared on reasonable request". Licence: **CC BY 4.0** (Crossref, version of record from 2023-08-04). The Turku cover sheet's boilerplate "All rights reserved" contradicts this; the article itself reads "© 2023 The Author(s)" | **New**: `papers/pasechnik_myllari_valtonen_mikkola_2023_MNRAS_525_1929.pdf` |
| `1910.07291v1.pdf` | `54af9e932987019dab7c76a45744f45d392aedfb7c6510eb1bf18e4bd430497f` | Breen, Foley, Boekholt & Portegies Zwart (2020), arXiv:1910.07291v1 | Duplicate (git blob `f9b57f3`) of `../2026-10-25_replication/papers/breen_etal_2020_MNRAS_494_2465_arXiv1910.07291.pdf`. Not added |
| `staa452.pdf` | `9577338175d8210122d672300c2b34ae9949ecd1c2be3ca17e374a3dc3ab3a0f` | Boekholt, Portegies Zwart & Valtonen, *MNRAS* 493(3), 3932–3937 (2020), doi:10.1093/mnras/staa452. The **accepted publisher version**, with journal page numbers (Fig. 1 is on p. 3934) | Same paper as `../2026-10-25_replication/papers/boekholt_pz_valtonen_2020_MNRAS_493_3932_arXiv2002.04029.pdf`, a different file. Not added; cite journal pages from the human's copy if a page number is needed |
| `Mauri_Valtonen_and_Hannu_Karttunen.-.The_Three_Body_Problem.pdf` | `5710ce62c616b5552c133ddd8cc4eda58fa895f465e04dcb742ecbbeb698a333` | M. Valtonen & H. Karttunen, *The Three-Body Problem*, Cambridge University Press (2006; copyright page reads © 2005), ISBN 9780521852241. 353 PDF pages. Stability of triples is §10.6, book p. 271 = PDF p. 279 | **Not committed** (below) |

**Why the book is not committed.** Its copyright page reads "© Cambridge University Press 2005. This publication is in
copyright … no reproduction of any part may take place without the written permission of Cambridge University Press",
and this repository is public. This is the same decision `../2026-10-27_papers/README.md` §2 recorded for
Valtonen et al. 2016. The human asked for it to be pushed; this record's author did not push it, and says so here
rather than dropping it silently. The sha256 is above. Cite it by section and page and paraphrase it, as
`../2026-10-27_papers/valtonen2016/` does for the other book. The human holds the file.

*28 Oct 2026: the human's copy is held locally, gitignored, at
`../_local/valtonen_karttunen_2006_three_body_problem_CUP.pdf` (sha256 above), so agents can read it. It is not in the
repo and is never committed. The same holds for `staa452.pdf`, the journal version of Boekholt et al. 2020, at
`../_local/boekholt_pz_valtonen_2020_MNRAS_493_3932.pdf`: its page 3 has the vector Fig. 1 that §4 task 2 reads.*

**Why Part I is not committed** (found in review on PR #327, 28 Oct 2026). Every page of the delivered `sty237.pdf`
carries "Downloaded from https://academic.oup.com/mnras/article-abstract/476/1/830/4830128 by Turun Yliopiston
Kirjasto user". That makes it an institutional-subscription download. It has no Creative Commons or open-access
statement, and Crossref lists no licence for doi:10.1093/mnras/sty237. MNRAS was hybrid in 2018. Unpaywall's one open
copy is the University of Turku repository's **submitted version** (oai:utupub.fi:11111/33559), with no licence stated.
The first cut of this PR committed the publisher PDF in error. The PR was re-cut from `main` without it, so that it
never reaches `main`'s history. The human's copy is held locally, gitignored, at
`../_local/myllari_valtonen_pasechnik_mikkola_2018_MNRAS_476_830.pdf` (sha256 above). If a committed copy is wanted,
add the Turku submitted version under its own sha256, after checking its terms.

## 2. The data hunt: neither data set is public

Searched: each paper's text and data-availability statement, the authors' later papers, Google, and GitHub through web
search. GitHub's own search API is not reachable from that session, so a repo-scoped search is still open (§4, task 1).

- **BR-1** (Breen et al. 2020). No data statement and no code or data release. The initial positions were drawn **at
  random** (§2 of the paper), so the ~10⁴ trajectories cannot be rebuilt from the paper. The set-up is fully stated:
  three equal masses at rest, x₁ ≡ (1, 0), x₂ random in the region of their Fig. 1, x₃ fixed by the barycentre,
  integrated to t ≈ 10 with Brutus. Appendix A: ε = 10⁻¹¹ with L_w = 128 bits against ε = 10⁻¹⁰ with L_w = 88 bits,
  converged when the phase distance δ (their eq. A1) stays below 0.1. Outputs every 2⁻⁸ time units, 2561 points per
  trajectory. Converged in over 90% of cases; the failures sit near the singular point (−0.5, 0).
- **GG-1** (Boekholt, Portegies Zwart & Valtonen 2020). No data statement. The 2023 sequel (Boekholt & Portegies Zwart,
  "Gargantuan chaotic gravitational three-body systems II", arXiv:2311.07651) says "The data underlying this article
  will be shared on reasonable request to the corresponding author", and draws its own Plummer initial conditions
  rather than reusing these. **The initial conditions are fully specified**: a uniform grid of spacing 0.015625 (1/64)
  over the Agekyan–Anosova map gives 1212 points (§2 and Appendix A). The map is defined in
  `../2026-10-24_anosova/papers/agekyan_anosova_1968_SvA_11_1006.pdf`. Brutus runs at L_w = 1024 bits fixed. The
  per-point lifetimes appear only in Fig. 1 (two panels: ε = 10⁻⁶ left, ε = 10⁻⁷⁰ right; colour log₁₀ T on 0–3;
  black for T > 1000). The paper's K–S test gives p = 0.72 between the two panels.
- **A false lead.** TIB's Leibniz Data Manager listed a "Three-Body Problem Dataset" beside Breen et al. in search
  results. The human opened it on 28 Oct 2026 and got **404 Dataset not found**. It is not a source.
- **V-11** needs no external data. The criterion is in Part I. The book's §10.6 is secondary.

**Fig. 1 of GG-1 is vector, not raster.** Found while checking whether it could be digitised, and recorded here as a
lead, not a result. Page 3 of the publisher PDF (p. 3934) holds 2469 filled circular paths about 2.9 pt across, 8
Bézier segments each, at 62 distinct x-centres with a 4.61 pt pitch. Split at the page midline, the left panel has
1238 paths at 1215 distinct positions (23 repeated, 26 black) and the right panel 1231 at 1217 (14 repeated, 19
black). Every non-black fill matches a matplotlib `jet` 256-entry colour within an L1 distance of 0.005. So each dot's
log₁₀ T is recoverable to about one colour step, 3/256 ≈ 0.012 dex, and it is exact for the black class. The counts do
not yet reconcile to 1212 per panel; §4 task 2 does that.

## 3. The human's rulings, 28 Oct 2026

Quoted from his messages, with the context needed to read them.

- **RQ-598.** Option (a): fetch the papers (this PR) and email the corresponding authors for both data sets. The human
  sends the emails himself.
- **GG-1**, on the three routes offered (1 email, 2 extract Fig. 1, 3 recompute with Brutus): "lets do 2 and 3 for
  GG-1 first then while we wait for a reply from the email 1".
- **BR-1**: "we could do the Brutus stuff while i email". This rules in an independently computed Brutus ensemble on
  the paper's stated set-up while the authors' data is awaited.

Earlier, unanswered, and not ruled here: whether a self-computed ensemble may stand in for the authors' data **when
scoring the target**, or is a cross-check only. Until it is ruled, label every self-computed number as ours,
"Brutus, run here", never as the paper's.

## 4. For the agent

Open REVIEW_QUEUE entries and ruling numbers as the workflow requires. RQ-591 to RQ-598 are on an unmerged branch
at the time of writing, so check where RQ-598 lives before writing against it.

1. **Record the rulings** in §3 against RQ-598 and the targets BR-1, GG-1 and V-11. Do a repo-scoped search on GitHub
   for released Breen or Boekholt data before building anything. The claude.ai session could not. Record the result
   either way.
2. **GG-1, route 2: extract Fig. 1.** Read the fills and centres from the PDF's vector paths. PyMuPDF
   `page.get_drawings()` on page index 2 of the human's `staa452.pdf` (sha256 above), or the arXiv copy if its Fig. 1
   is also vector: check. Then:
   - Map centres to (x, y) from the axis ticks.
   - Snap them to the 1/64 grid.
   - Map colours to log₁₀ T through `jet`'s 256-entry table on [0, 3]. Black is T > 1000, a censored value, not a
     number.
   - Reconcile each panel to exactly 1212 points before writing anything. Explain the repeats, which may be black dots
     drawn over coloured ones, and the extra positions. **If it does not reconcile, stop and queue it.**
   - Write a CSV per panel: grid index, x, y, LUT index, log₁₀ T or "censored >1000", and residual colour distance.
     Add a `build.py` that regenerates it, in the style of `../2026-10-27_papers/transcriptions/`.
   - State the precision honestly: one LUT step, about 0.012 dex.
3. **GG-1, route 3: recompute with Brutus.** Brutus ships with AMUSE (`github.com/amusecode/amuse`); verify the path and
   licence. Use the 1212 grid points on the Agekyan–Anosova map and the paper's convergence and lifetime definitions
   (§2 and Appendix A). Start with the ε = 10⁻⁶ panel. **Report the measured cost per point before scaling up**, and
   before any finer ε. 10⁻⁷⁰ at 1024 bits up to T = 1000 may be very expensive. Then compare with route 2 point by
   point, reporting the distribution and not only the agreement fraction. The two panels themselves differ point by
   point (the paper says so), so compare like ε with like. PLAN.md's GG-1 row also binds here: the paper's runs go to t = 10⁴, past R-610's cap of
   200. Compare only the systems that end before 200, unless a ruling says otherwise.
4. **BR-1: a Brutus ensemble on Breen's stated set-up**, as in §2. Draw the initial positions from a **fixed, recorded
   seed**. Use their two (ε, L_w) pairs and their δ < 0.1 convergence test, and record the converged fraction against
   their "over 90%". Again, report cost before scaling. These are our trajectories, not theirs. Label them so.
5. **V-11**: proceed from Part I. Use the book's §10.6 (p. 271) only as secondary, cited by page.

Sources for §2: [arXiv:1910.07291](https://arxiv.org/abs/1910.07291),
[arXiv:2002.04029](https://arxiv.org/abs/2002.04029), [arXiv:2311.07651](https://arxiv.org/abs/2311.07651),
[Turku repository, Part I](https://www.utupub.fi/items/e9258b49-d38c-4b0b-99ec-dbb4316997da/full),
[Turku repository, Part II](https://www.utupub.fi/items/c3516cb3-aa7c-4600-9bf6-712ee29c16d1/full),
[TIB LDM (404)](https://service.tib.eu/ldmservice/dataset/three-body-problem-dataset).

## 5. Outcome of § 4's tasks, 29 Oct 2026 (appended; nothing above is changed)

This section supersedes the record proposed on PR #334, which is not merged. Its work is carried here with every value from the authors'
unlicensed NVM files removed, and with the physics review of #334 applied.

1. **Rulings recorded; the search.**
   - No ruling is added to `decisions.md`. The instructions that drove the pilot are recorded below as quoted, not as rulings. The open
     questions are RQ-612 to RQ-616 in `REVIEW_QUEUE.md`. RQ-598 itself is on PR #309 and not on `main`; the entries refer to it there.
   - The repo-scoped search, as the pilot recorded it: two public repositories were named in the instructions quoted below,
     `github.com/pgbreen/NvMgeneration` (Brutus as used in the BR-1 paper, GPL-3.0) and `github.com/pgbreen/NVM` (BR-1 trajectories and a
     trained network, no licence). No Boekholt et al. GG-1 data was found. The search was not exhaustive: GitHub's search API was not
     reachable from the machine that ran it, and a web search for the lifetime maps found nothing.
   - **Correction to § 2 above** ("BR-1 … No data statement and no code or data release"): some of BR-1's trajectories are public, in
     `pgbreen/NVM` @ `a35c378`, with no licence. The pilot counted 180 files (136 with the full 2561 rows, 44 truncated), under 2% of the
     paper's 10,000; the count is not verified from committed material. No value from these files is committed; any copy stays in
     `docs/incoming/_local/`. RQ-615 asks how they may be used.
2. **GG-1 route 2** is `transcriptions/gg1_fig1/`: Fig. 1 read from its vector paths, 1212 points per panel, reconciled. It is the one
   record of Fig. 1 in the repo; the pilot's comparison cites it.
3. **GG-1 route 3** (cost pilot, ε = 10⁻⁶ at L_w = 1024, plus one point at 10⁻⁸ and 10⁻¹⁰): `brutus_run_here/REPORT.md`. Its times are
   converted to the paper's Heggie–Mathieu units there. Open: the lifetime definition and the (3, 43) cell (RQ-612); `p_max`, the word length
   and the ε = 10⁻⁷⁰ cost (RQ-613).
4. **BR-1**, 12 runs at ε = 10⁻⁶ from the authors' own initial conditions, not from a seeded draw as § 4 task 4 says:
   `brutus_run_here/REPORT.md` § 3. Whether that use of the authors' files stands is RQ-615; the convergence criterion's reading is RQ-616;
   whether a self-computed ensemble may score is RQ-614.
5. **V-11**: what Part II gives is in `transcriptions/v11_partII/`. Part I is held locally (§ 1) and was not read here; **V-11 proceeds
   from Part I in a later local task.** Part II's closed-form Q does not follow from its own equations (coefficient 1.70, not the printed
   3.7) and is not V-11's comparison number.

**Instructions as quoted in #334 (unverified as the owner's words); not ported as a ruling.** #334 quoted two messages as the owner's,
verbatim. The committed material cannot confirm that; both read as working briefs (they name files, sections and tasks), and the owner's
28 Oct words already on `main` (§ 3) differ from the first. They are recorded here as what the pilot was told, and nothing here rests on
them as a ruling:

> PR #327 (incoming/2026-10-28-rq598-data) carries the RQ-598 papers and my rulings; read docs/incoming/2026-10-28_rq598_data/README.md in full, then do §4 tasks 1–5. Merge #327 through the normal review. Rulings: GG-1 routes 2 (extract Fig. 1 from the vector paths) and 3 (recompute with Brutus) now; BR-1 Brutus ensemble on Breen's stated set-up now, fixed recorded seed; I'm emailing Breen and Boekholt in parallel. Gates: stop and queue if Fig. 1 doesn't reconcile to exactly 1212 points per panel; report measured cost per point before scaling any Brutus run or going below eps=1e-6. Label all self-computed results as ours, not the papers'. Open question for me: may a self-computed ensemble score a target, or is it cross-check only?

> 1. Build Brutus from github.com/pgbreen/NvMgeneration @ 98906f4 (Breen's own README: "Brutus: version used in Newton vs the Machine"; GPL-3.0, license.txt), not AMUSE's copy. Record the build exactly.
> 2. For BR-1, do not draw random starting points. Use the authors' own: github.com/pgbreen/NVM @ a35c378, trainex/data/run10–99.dat and validex/data/run9910–9999.dat (row 1 = x1, y1, x2, y2; body 3 from the centre of mass; 2^-8 cadence). Run a small stated sample of them at eps = 1e-6 and compare your trajectory to theirs point by point. That both measures cost and checks your build reproduces theirs. Keep any copy of the NVM files in docs/incoming/_local/ (no licence; never commit). Their eps 1e-11/1e-10 pairs stay estimate-only, as before.

The one line #334 quotes that reads as the owner's own, given on seeing the pilot's cost report:

> Go ahead with ε = 1e-8 and 1e-10 on one point each, do v-11 and review queue book keeping. also open a pr

The probes below ε = 10⁻⁶ in `brutus_run_here/REPORT.md` § 2b are the ones that line approves. Anything further below ε = 10⁻⁶, any
further probe (such as p_max 8 and 16 at 10⁻²⁰ and 10⁻³⁰), and any scaling of GG-1 or BR-1 waits for the owner's go-ahead.
