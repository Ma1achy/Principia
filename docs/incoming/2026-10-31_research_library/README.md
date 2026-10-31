# The owner's 31 Oct research library: Horn 2024, V-11 Part I, Zare & Chesley 1998 (a record)

31 Oct 2026. Ingest and transcription only. A record, not authority (R-1, R-159). No Principia code ran and nothing is
scored. Every number is a transcription, or our own arithmetic on one (`scripts/`). This directory changes no
requirement, ruling, task or PLAN.md row. Priority (3) of the owner's message, papers-3 to papers-6, is handled by a
separate triage and is not here.

## 1. The request

The owner, 31 Oct 2026, verbatim, the parts this record covers:

> Priorities: (1) Horn et al. 2024 Zenodo 10.5281/zenodo.11032352: Brutus flow-map data for 5000 planar equal-mass
> triples, CC BY 4.0 per Zenodo metadata. Verify the licence yourself; it's the first committable high-precision
> dataset. Don't commit the 930 MB .h5 to git: record the DOI and checksum, fetch on demand, and propose it as a target
> (peek/M9, non-gating). (2) V-11 Part I is papers/Myllari_2018_Stability_Hierarchical_Triples_I_MNRAS476_830.pdf. Put
> it in _local and close that gap.

> Add to task 15: Zare & Chesley 1998, "Order and chaos in the planar isosceles three-body problem", Chaos 8, 475,
> doi:10.1063/1.166329. In the library as papers/Zare_Chesley_1998_Order_and_Chaos.pdf; © AIP → _local. Planar isosceles,
> equal masses, exact invariant subset. Transcribe twice: Table I (measures m(A_k) of the escape-after-k-binary-collisions
> classes), Table II (period-k fixed points q_k, a_k, b_k), Table III (nested invariant-region measures converging to
> 0.0526, the permanent-capture island). Propose as an M9 statistical target, tier (a): reproduce their Poincaré section
> (isosceles, sampled at Q2 = 0 in their regularised coordinates) and compare the outcome-class measures and the 0.0526
> island. Document their coordinates and section definition before any comparison; non-gating.

The owner's later triage message says transcription happens later for approved items. The orchestrator's dispatch keeps
the explicit Zare & Chesley request above, so those three tables are transcribed here.

## 2. Licences and what is committed

| Source | Licence, verified how | Committed | Local only |
|---|---|---|---|
| Horn, Saz Ulibarrena, Koren & Portegies Zwart 2024, Zenodo dataset | **CC BY 4.0**, Zenodo's API, § 3.1 | DOI, checksums, Zenodo's JSON | nothing; the `.h5.1` is neither fetched nor committed |
| Horn et al. 2025, JCP 521, 113536 (the data's paper) | CC BY: Crossref's `vor` licence is `http://creativecommons.org/licenses/by/4.0/`; also the library's manifest. It could be committed; not asked for, so it is not | a short quotation | library copy read, not copied |
| Mylläri, Valtonen, Pasechnik & Mikkola 2018, MNRAS 476, 830 | No open licence: Crossref `license: null`. The page prints "© 2018 The Author(s). Published by Oxford University Press on behalf of the Royal Astronomical Society" and a Turku subscription stamp | equations and definitions, cited by page | `../_local/myllari_valtonen_pasechnik_mikkola_2018_MNRAS_476_830.pdf`, sha256 `625a5494…3b55c` |
| Zare & Chesley 1998, Chaos 8, 475 | No open licence: Crossref `license: null`. Page 475 prints "© 1998 American Institute of Physics" | three tables, definitions, cited by page | `../_local/zare_chesley_1998_Chaos_8_475.pdf`, sha256 `070d786c…a6f19` |

`git check-ignore` confirms both local paths (`.gitignore:21`). `git worktree remove` deletes such copies. The sha256s
in `sources.tsv` and the owner's library recover them.

Citations and DOIs were checked one request at a time against Crossref on 31 Oct: titles, authors, volumes and pages all
match the papers' first pages. No owner email was in any request (`User-Agent: principia-fetch/1.0`).

## 3. Horn et al. 2024 (priority 1)

### 3.1 The licence, from Zenodo

`GET https://zenodo.org/api/records/11032352`, 31 Oct 2026 15:12 UTC, saved unchanged as
`horn_2024/zenodo_record_11032352.json` (sha256 `5d312ea9fac2f9cf37c4054e06087eeeae61fbf87dc096ff018c8d94c7afac19`; the
same bytes as the copy in the owner's batch):

```
"metadata": { ... "access_right": "open", ... "license": {"id": "cc-by-4.0"} ... }
```

`GET https://zenodo.org/api/vocabularies/licenses/cc-by-4.0`, saved as `horn_2024/zenodo_vocabulary_cc-by-4.0.json`:

> "title": "Creative Commons Attribution 4.0 International"; "description": "The Creative Commons Attribution license
> allows re-distribution and re-use of a licensed work on the condition that the creator is appropriately credited."

So this is **committable with credit**. The record is `published`, status `done`, the only version of its concept
(concept DOI 10.5281/zenodo.11032351; one hit; 31 Oct), and its `version` field is null. The batch's "v1" is not in the
record.

Credit: P. Horn, V. Saz Ulibarrena, B. Koren, S. Portegies Zwart (2024), *Flow map data of the singel pendulum, double
pendulum and 3-body problem* [sic], Zenodo, doi:10.5281/zenodo.11032352, CC BY 4.0. Zenodo prints two creators' names
reversed ("Veronica, Saz Ulibarrena", "Simon, Portegies Zwart"); the paper gives them as here.

### 3.2 The files, by Zenodo's checksums

| File | Bytes | Zenodo `checksum` | Bytes in the owner's library (sha256, ours) |
|---|---:|---|---|
| `circ_3body_all_runs.h5.1` | 930367900 | `md5:873fac461d67f57db5d4d389bab3bed0` | `03b916d27c874089589ad91b37544be448b9aaabd122ef17629168a91f0ea20c` |
| `circ_3body_training.h5.1` | 9662307 | `md5:e8416577e763f349eb18c314450c07af` | `9a52b636cfb24587aad8815ed27975bf5ecd777bd6161f17981720296a0fc3c7` |

We ran `md5` on the owner's library copies, read-only: both match Zenodo. The sha256 column is the batch's
`checksums.sha256`, not re-run. Zenodo publishes md5 only. `horn_2024/fetch.sh` fetches on demand and verifies; it is
not run here. Nothing was downloaded.

**The files are untrusted.** They are pandas HDF5 stores whose `constants` column is object-typed, which pandas
unpickles. Read `block0_values` as plain arrays; never `pandas.read_hdf` them.

### 3.3 What the data is, like for like

From the paper, JCP 521, 113536, § 5.4 (printed p. 14, PDF p. 15), read from the page image:

> "we choose all masses to be dimensionless and all equal to one; m_i = 1. Further, we scale the system to G = 1"

> "The data of the 3-body system is generated using the arbitrarily high-precision code Brutus"

> "The random initial positions of the bodies are on a circle around the origin with the radius r drawn uniformly from
> [0.9, 1.2]. Furthermore, all three bodies have the same initial distance from each other. With these initial
> positions, one can calculate the momenta of the three bodies such that they would stay on the circle with radius r,
> orbiting the origin. These momenta are multiplied by different random uniform factors from [0.8, 1.2]"

> "only small accelerations and slow exchange in kinetic and potential energy occur until t ≈ 5. Afterwards, close
> encounters of the bodies may occur"

Zenodo's description: 5000 trajectories, `all_runs` to t = 10 with "a step size of 0.01"; training data to t = 5 at 0.5.
The paper's test horizon is t = 7, a different number. The batch's HDF5 inventory, **not read by us** (h5py does not
load on this Mac): 5000 run groups of 1001 states, and an `all_runs` aggregate repeating them (5005000 rows, 13 columns,
float64): don't double-count.

What that means for a comparison:

- **Accuracy is not stated.** Neither the record nor § 5.4 gives Brutus's tolerance or word length, and the export is
  float64. The batch's guide agrees: unverified. "High-precision" is the code's capability, not a stated bound on these
  numbers. Don't call this a converged reference until that is measured; a run-by-run Brutus convergence check is
  outside this record.
- **Frame.** By § 5.4's words the three momentum factors differ, so in general the total momentum and angular momentum
  are non-zero and the barycentre moves. That is our reading; **no stored row was checked**. A comparison reads the
  states as stored (a Galilean shift, if the engine requires one, recorded per run).
- **Not on a chart surface.** These are near-circular non-zero-velocity starts, imported per run, not free fall.
- Eq. (24) as printed sums j up to and including i, which would divide by zero. It is read as the usual pair sum;
  recorded, not corrected.

## 4. V-11 Part I (priority 2): the gap closed

Part I is the same bytes as the 28 Oct `sty237.pdf` (`../2026-10-28_rq598_data/README.md` § 1) and the same bytes as the
copy in the main checkout's `_local/`. Its content, and that it has no numerical tables, is in
`myllari_2018/README.md`. What V-11 was waiting for, its comparison number, is **eq. (13) with eq. (11) and (12), at the
paper's A levels**.

**R-638's route.** R-638 says V-11 is read "from the University of Turku repository's version", and the Turku file could
not be fetched. The owner's 31 Oct words name this publisher copy. *Applied:* the owner's later words, with the publisher
PDF local only as R-638 requires. R-638 also asks that a difference between versions in what V-11 needs be stopped and
queued; **the Turku version has not been compared**, so that check remains undone.

One conflict in Part I is queued, RQ-639 (§ 7).

## 5. Zare & Chesley 1998 (priority 4)

### 5.1 Coordinates and section (read before any comparison)

Page numbers are the journal's; PDF page = journal page − 473, since the PDF has a one-page AIP cover.

- **The subset** (pp. 476–477). m1 = m2; m1 and m2 at (∓x, y₁), m3 on the y-axis; invariant under the equations.
  x and y are "the Cartesian coordinates of m2 and m3 with respect to the center of mass of m1 and m2" (Fig. 1);
  u = ẋ, v = ẏ.
- **Hamiltonian** (p. 477). q1 = x, q2 = y, p1 = u, p2 = αv, α = m3/M. h is **not** the total energy: Cartesian
  E = 2 m1 h (`decode_check.txt` § 3, to 15 digits).
- **Regularisation** (p. 478). W = p1 Q1² + p2 Q2, so x = Q1², **y = Q2**, P1 = 2 Q1 u, P2 = αv; dτ = dt/Q1². Only the
  binary is regularised. "Q2 = 0" is therefore just **y = 0, syzygy, in plain coordinates**.
- **Units** (pp. 478–479). G = 1, 2(m1 + 4m3) = 1, −8h = 1, with m1 = m2 = m3, so **m = 0.1, α = 1/3, h = −1/8, E =
  −0.025**. To unit masses, G = 1, E = −1: masses × 10, lengths × 2.5, times × 1.25, velocities × 2 (ours).
- **The section** (p. 478). "the plane defined by Q2 = 0", Q1 ≥ 0, P2 ≥ 0 from eq. (11), which gives Q1² + P1² ≤ 1. Domain
  A = {Q1 > 0, Q1² + P1² < 1}. f maps (Q1, P1) to the next syzygy, folded by the reflections
  (Q1, P1, Q2, P2) → (−Q1, −P1, Q2, P2) and → (Q1, P1, −Q2, −P2).
- **Classes** (p. 480). "a non-negative integer k representing the number of binary collisions that occur **before the
  next syzygy crossing**". k = ∞ is "points in A_e that lead to hyperbolic escape of m3", and A_k means k < ∞.
- **Measure** (p. 481). μ(Ã) = area of Ã / area of A: a uniform area on (Q1, P1), **not** on (x, u), and not on a chart.
- **Their computation** (pp. 479–480). The lattice (mδ, nδ), δ = 0.005, 62,605 points; RKF 7(8), tolerance 10⁻¹⁴, stopped at
  10⁴ steps; "Only 266 (0.4%) of the lattice points were not computable (due to near-parabolicity)", with over a third of
  those "non-escapers". Time-reversal check: all but 3 points within 10⁻⁸. Escape: the Standish criterion and a Sundman-function modification.

**`decode_check.txt` confirms this decode on their own numbers**, integrating their eq. (6) with our DOP853, no engine:
the lattice has exactly 62605 points. Newton's accelerations match eq. (2) to 15 digits. From the printed q_k, f returns
to (q_k, 0) with exactly k binary collisions, for all six rows, to within the rounding of q_k amplified by |a_k|, and
finite differences of f give a_1 = −0.9882, b_1 = −0.0095 and values near each printed a_k, b_k (within 0.6 %, the
printed q_k being only near the fixed point). This is a check of the reading; it isn't a replication and scores nothing.

### 5.2 Three of the owner's phrases against the paper

These change what would be compared, so they are recorded. *Applied:* the paper's definitions, in § 5.1 and the row below.

| The owner's words | The paper |
|---|---|
| "measures m(A_k) of the escape-after-k-binary-collisions classes" | The symbol is μ. A_k (k < ∞) is k binary collisions **before the next syzygy**, with no escape implied (p. 480). Only A_∞ is escape. Table I isn't an escape distribution |
| "period-k fixed points q_k" | "fixed points of f on the Q1-axis", **period one**, "one in each A_k" (Table II, p. 487): k is the collision count per iterate |
| "0.0526, the permanent-capture island" | The paper's "permanent capture" is C₂ ∩ L₁ᴹ, "the product of a Cantor set with sub-intervals … and it has zero measure". R̃ is the "main invariant region" around p_1, measure 0.0526, whose orbits are never captured and never escape. It is the largest invariant neighbourhood of p_1 inside R_1 = A_1 ∩ f(A_1), and the paper allows stable islands outside it (p. 488), so 0.0526 is not the measure of all non-escaping motion |

### 5.3 The tables

| Table | Page | Cells compared (eye v OCR) | Disagreements | Resolved |
|---|---|---:|---:|---|
| I | 481 | 32 | 5 | key column: OCR read ∞ as "00" and returned nothing for four digits. All measures agree |
| II | 487 | 36 | 1 | key column: one digit unread. All q_k, a_k, b_k agree, minus signs included |
| III | 489 | 56 | 4 | key column: four digits unread. All measures agree |

Reading A is by eye from page images. Reading B is Apple Vision OCR of 250 dpi crops (`scripts/ocr.swift`), which cannot
reliably read a lone digit: `key_cells_ocr_raw.txt` holds a cell-by-cell retry, which supplied Table III's key cells where it
could (the whole-table pass read none) and added nothing to Tables I and II.
Unread key cells are settled by row order on the page, which is plainly 0–6, 1–6 and 1–14. **Every value cell agrees; only
key cells differ.**

The text layer is not a reading: `pdftotext` turns minus signs into the digit 2 (a_2 comes out `28.65`, a_1 `20.9882`).

### 5.4 Recorded, not corrected

- **Rotation number.** The paper prints α = 0.477…; a_1 = −0.9882 gives arccos(a_1)/2π = 0.47553. α = 0.477 would be a =
  −0.9896. Our finite difference agrees with the table's a_1 (−0.9882), not with 0.477 (`decode_check.txt`).
- **The fit.** μ(A_k) ~ C k^−β, C ≅ 0.1017, β ≅ 1.650, is stated for 1 ≤ k ≤ 50. Table I has k = 1 at 0.527 against the fit's
  0.1017. Fig. 3 shows k = 1 far above the line. From k = 3 on, the fit is within about 3 %.
- **Closure, ours.** Table I sums to 0.95299; the fit's tail over k ≥ 7 adds 0.04630; total 0.99929. So the numbers
  fit a measure of A, with the 0.4 % not computable unaccounted for.
- Table III's construction resolution isn't stated beyond the δ = 0.005 of § III; R̃ is thin, and one lattice point is
  1.6 × 10⁻⁵ of A, which is less than one unit of 0.0526's last digit, but boundary error over its long arc may not be.

## 6. Proposed targets

Exact text for a porter, in the form of `../2026-10-29_research_batches/README.md` § 10. **Not applied:** R-649 and R-650
are not yet in `decisions.md`, and task 15b's rows wait on the same port. Each is a proposal for the owner to rule, a
measurement and never a gate: PASS, EXPLAINED DIVERGENCE or UNEXPLAINED. Never tuned; a tolerance is a calibration
(R-71).

PLAN.md § 0, after the `RB29` row:

```
| `RL31` | `../2026-10-31_research_library/` | The owner's 31 Oct research library, priorities 1, 2 and 4: Horn 2024's DOI and checksums, Part I's stability formula, Zare & Chesley 1998's tables (two readings, diffed) and coordinates |
```

PLAN.md § 3.2, after task 15b's rows:

```
| HORN-1 (proposed, 31 Oct 2026) | R | C | `RL31/README.md` § 3; Horn et al. 2024, doi:10.5281/zenodo.11032352, CC BY 4.0, data on demand by md5 (`RL31/horn_2024/fetch.sh`); Horn et al. 2025 (JCP 521, 113536) § 5.4 | From each stored t = 0 state (unit masses, G = 1, planar; states read as stored), our state against the stored one at each stored t ≤ 10: a growth curve of the difference per run, reported beside the source's own conservation in the stored rows. Brutus's tolerance for these runs is not stated and the export is float64, so the stored rows are not assumed converged; any row past their own measured accuracy is not evaluated. `all_runs` repeats the runs and is not counted | fetched on demand, never committed | Path A (a per-run import, not a chart) | CPU IC inspector | exact (their measurement) | data on demand · proposed 31 Oct; R-649 tier (b) CPU IC inspector, scores in M9 or as an information-only peek, non-gating |
| ZC-1 (proposed, 31 Oct 2026) | R | B | `RL31/README.md` § 5; Zare & Chesley 1998 (Chaos 8, 475), Tables I–III: `RL31/zare_chesley_1998/` | Planar isosceles, equal masses, zero angular momentum, their section (y = 0 with ẏ > 0, Q1 = √x, P1 = 2Q1ẋ, uniform area on Q1² + P1² < 1; in unit masses, G = 1, E = −1, x and y × 2.5, times × 1.25, velocities × 2). On their lattice (δ = 0.005, 62,605 points) the measures of A_∞ and A_0 … A_6 (k binary collisions before the next syzygy; A_∞ hyperbolic escape), and R_1 … R_14's measures, μ(R̃) = 0.0526, by their Theorem 2. Their 266 not-computable points (0.4 %) and their lattice set the source's resolution; rounding must keep the run on the subset or the departure is reported | `RL31/zare_chesley_1998/table{1,2,3}_reconciled.csv`, `decode_check.txt` | Path A; no chart | outcome classifier; a binary-collision count; a syzygy event | statistical | data transcribed · proposed 31 Oct; R-649 tier (a) GPU explorer, scores in M9, non-gating |
| ZC-2 (proposed, 31 Oct 2026) | R | A | as ZC-1, Table II | Beside ZC-1, optional: from (q_k, 0), k = 1 … 6, f returns to the start after k binary collisions, to q_k's printed 5 decimals amplified by \|a_k\|; a_k and b_k at their printed digits | `RL31/zare_chesley_1998/table2_reconciled.csv` | Path A | regularisation | exact | data transcribed · proposed 31 Oct; R-649 tier (b) CPU IC inspector, scores in M9, non-gating |
```

And a dated note after the table:

```
*Note added 31 Oct 2026 (rows HORN-1, ZC-1, ZC-2; proposed, not ruled, not counted in § 3.3).* Horn 2024 is CC BY 4.0 by Zenodo's API, quoted in `RL31/README.md` § 3.1; its 930 MB file is fetched on demand and never committed, and its accuracy is not stated, so HORN-1 reads it as a measurement rather than a reference. Zare & Chesley's paper is local only. V-11's comparison number is now in `RL31/myllari_2018/`; REQ-VAL-330's text and TASK-M3-65 are unchanged.
```

ZC-2 goes beyond the owner's request, because the owner named Table II and tier (a) cannot bind its exact points; the
owner may drop it. V-11 needs no new row. Its existing row stands; its transcription now exists.

## 7. The queue

`REVIEW_QUEUE.md` gains RQ-639: Part I's "f g = 1" limit against its own eq. (11).

## 8. Not verified

- Horn: the stored rows, `constants` and Brutus's settings. Not read by us. The inventories quoted are the batch's. Momentum and angular momentum non-zero is inferred from § 5.4's words.
- Part I against the Turku submitted version (§ 4).
- Figure readings by eye (`myllari_2018/README.md`): approximate, not transcriptions.
- ZC's measures are not reproduced here, by design. Only the coordinates and Table II's points were exercised.

## 9. Files

`sources.tsv`; `horn_2024/` (Zenodo's JSON, `fetch.sh`); `myllari_2018/` (`README.md`, `formula_check.txt`);
`zare_chesley_1998/` (`table{1,2,3}_{eye,ocr,reconciled}.csv`, diffs, raw OCR, `decode_check.txt`); `scripts/`
(`zc_decode.py`, `myllari_formula.py`, `ocr.swift`, copied from task 15b's). Diffs were made by
`../2026-10-29_research_batches/scripts/diffreadings.py`. Every script runs as `python3 -I`, from outside the library.
