# Portegies Zwart & Boekholt 2018: the converged Burrau (Pythagorean) reference, 28 Oct 2026

**A record, not authority** (R-1, R-159), like every `docs/incoming/` folder. It transcribes one published table and
proposes one target (PLAN.md § 3.2, row SP-2, **proposed**). Nothing here changes a requirement, a ruling or a task.

S. F. Portegies Zwart and T. C. N. Boekholt, "Numerical verification of the microscopic time reversibility of Newton's
equations of motion: Fighting exponential divergence", *Commun. Nonlinear Sci. Numer. Simul.* **61**, 160–166 (2018),
doi:10.1016/j.cnsns.2018.02.002; arXiv:1802.00970.

## The owner's request, 28 Oct 2026, verbatim

> New replication source (fetch locally): Portegies Zwart & Boekholt 2018, "Numerical verification of the microscopic
> time reversibility of Newton's equations of motion: Fighting exponential divergence", Commun. Nonlinear Sci. Numer.
> Simul. 61, 160 (2018); arXiv:1802.00970. Converged Brutus solution of the Pythagorean (Burrau) problem: escape t ≈
> 63.4; eps = 1e-24, Lw ≥ 136 bits; Table 2 = positions/velocities at t = 100 and 200 to 40 significant figures; journal
> supplementary material has converged snapshots. Fetch the arXiv PDF (check its arXiv licence: commit only if CC, else
> docs/incoming/_local/) and the journal's supplementary files if publicly downloadable. Transcribe Table 2 twice and
> diff (R-594 rule 4). Caution: Table 1 and Appendix B give inconsistent initial positions, and one Table 2 row is
> garbled in the arXiv text extraction; reconcile against the journal version, queue anything unresolved. Propose it as
> a new target (Burrau converged reference) in the validation plan; note its horizon t = 200 vs R-610's cap of 200.

## R-594's rule 4, verbatim

R-594 is recorded in `decisions.md` on PR #309 (branch `port/R-594-replication`), not yet on `main`; its item 4, in the
human's words:

> 4. Transcribe every published table twice and diff the copies before committing them as fixtures. Never tune to
>    match. A disagreement is a REVIEW_QUEUE entry with both numbers, as RQ-570 was.

## What is in this folder

| Path | What it is |
|---|---|
| `sources.tsv` | every file fetched: path, URL, licence or terms as stated, sha256, committed or local-only |
| `fetch.sh` | re-downloads `sources.tsv` into an empty directory and checks each sha256 |
| `table2/table2_pass_a_pdftotext.csv` | pass A: Table 2 parsed from `pdftotext -layout` |
| `table2/table2_pass_b_ocr.csv` | pass B: Table 2 from OCR of the rendered page image |
| `table2/pass_b_ocr_raw_positions.tsv`, `…_velocities.tsv` | the OCR's raw lines (box mid-y, box min-x, text), before parsing |
| `table2/table2_diff.txt` | the cell-by-cell diff of A against B: 84 cells (48 numeric), **0 disagreements** |
| `table2/table2_reconciled.csv` | the reconciled table (identical to both passes) |
| `table2/conservation_check.txt` | energy, angular momentum, momentum and centre of mass from the reconciled table |
| `scripts/` | `pass_a.py`, `pass_b.py`, `ocr.swift`, `diff.py`, `conserve.py`: the scripts that made the above |

Every CSV has the columns `run, t, mass, x, y, vx, vy`, `run` ∈ {unperturbed, perturbed}, `t` ∈ {100, 200}, `G = 1`.
Each number is the printed string, verbatim, never rounded or re-formatted: the paper says (Appendix B) "We did not
round the numbers but present them as they are represented in the computer in 40 significant figures". As printed,
the positions at t = 100 carry 40 or 41 significant digits, those at t = 200 39 or 40, and the velocities 40 or 41
(counted from the first non-zero digit, trailing zeros included); the caption says "40 decimal places".

## Sources and licences

| Item | Source | Licence or terms | sha256 | Here |
|---|---|---|---|---|
| arXiv:1802.00970v1 (the only version: the abs page's history lists v1 alone, 3 Feb 2018, and `/abs/1802.00970v2` returns 404) | https://arxiv.org/pdf/1802.00970v1 | arXiv non-exclusive licence to distribute 1.0, not CC | `878f0fc5…58a13b` | local-only (`_local/burrau_pzb2018/`) |
| Leiden repository copy, hdl 1887/71686 | https://scholarlypublications.universiteitleiden.nl/download/1887/71686/1?col=81522199370002711 | none stated; landing page says "Open Access", version "Not Applicable (or Unknown)" | `5544b397…175957` | local-only |
| Journal version of record, CNSNS 61, 160 | https://doi.org/10.1016/j.cnsns.2018.02.002 → ScienceDirect PII S1007570418300303 | Crossref lists only Elsevier's TDM user licence (`https://www.elsevier.com/tdm/userlicense/1.0/`); OpenAlex: `oa_status` "green", the publisher's copy not open | — | **not fetched**: ScienceDirect answered 403 to a plain request (as it did for its robots.txt); not worked around |
| Journal supplementary material ("Snapshots of the converged solution are available in the online supplementary material", § 2) | the article page | unknown | — | **not fetched**: it is reached only through the article page above. RQ-611 |

The Leiden copy is the arXiv PDF: the same creation date (6 Feb 2018 01:28:50 GMT) and page count, 2 bytes longer, and
its `pdftotext -layout` text is identical to v1's. So there is one text of the paper here, arXiv v1; no second version
exists to compare, and the version of record could not be read. Requests: one at a time, 16 s apart on arXiv (its
robots.txt sets `Crawl-delay: 15`; `/pdf/` and `/abs/` are not disallowed, `/e-print` and `/src` are, so the TeX source
was not fetched), Crossref and OpenAlex's public APIs once each, the Leiden landing page and file once each.

## Method: Table 2 twice

Table 2 is on page 9 of v1, set landscape (rotated 90°).

- **Pass A, text extraction.** `pdftotext -layout` (poppler) of the whole PDF; `scripts/pass_a.py` parses the lines
  after "Table 2." by their labels ("Unperturbed configurations", "Midpoint (t=100)", "End (t=200)", the mass on a
  particle's first line), pairing each first line (mass, x, vx) with the next (y, vy).
- **Pass B, OCR of the rendered image.** Page 9 rendered at 600 dpi (`pdftoppm -r 600 -f 9 -l 9 -png`), rotated upright
  (`sips -r 90`), cropped to the position column (`sips -c 2830 1830 --cropOffset 1360 1740`) and the velocity column
  (`sips -c 2830 1930 --cropOffset 1360 3600`), and read by Apple's Vision text recogniser (`scripts/ocr.swift`:
  `VNRecognizeTextRequest`, accurate level, language correction off). `scripts/pass_b.py` keeps the lines that are
  whole numbers, sorts them top to bottom, and assigns them in the table's printed order (four blocks of three
  particles, two lines each). Pass B never reads the PDF's text layer.
- **Diff.** `scripts/diff.py` compares the two CSVs cell by cell, as strings: **0 disagreements in 48 numbers** (84
  cells with the labels). The reconciled CSV is therefore both passes' common content; nothing needed resolving.

**The garbled row.** The plain text extraction mangles Table 2; the layout extraction does not lose a digit:

- `pdftotext` without `-layout` (reading order) emits the 24 positions in order but the velocity column as four
  blocks of six in the order perturbed t = 200, perturbed t = 100, unperturbed t = 200, unperturbed t = 100, after the
  positions and separated from them, so a parse in reading order gives every row the wrong velocities; it also splits
  Table 1 into single numbers (`1`, `3`, `0 0`, `-2`, `-1 0 0`, …).
- In the `-layout` extraction one row is cut off from its particle: **the perturbed run's t = 200 second line for mass 4,
  `-0.999999999999679846094415534203355381441` (y) and `3.828886532501206909565529941811723471334e-14` (vy)**, comes
  after five blank or whitespace-only lines, cut off from its first line (`4  -1.999999999999957698016272046329507322690
  8.052002826247233760595438150140762918720e-14`); the unperturbed t = 200 block has the page number `9` between
  masses 4 and 5 in the same way, though there the rows stay adjacent.

Which of these the owner saw is not recorded; the second is the one row. Pass A pairs lines by structure, not by
adjacency, so both breaks are harmless there, and pass B (the page image) agrees with it in every digit. The journal
version could not be read (above), so the reconciliation rests on arXiv v1's page image; RQ-611 asks whether to check
it against the version of record and the supplement.

## The initial positions

**Table 1** (page 3), as rendered:

| | mass | position (x, y) | velocity (vx, vy) |
|---|---|---|---|
| Unperturbed | 3 | 1, 3 | 0, 0 |
| | 4 | -2, -1 | 0, 0 |
| | 5 | 1, -1 | 0, 0 |
| Perturbed | 3 | 1.0000000001, 3 | 0, 0 |
| | 4 | -2, -1 | 0, 0 |
| | 5 | 1, -1 | 0, 0 |

**Appendix B**, verbatim: "The three masses are located in the corners of a Pythagorean triangle with (1, 3) for the
first mass, (-2, 1) for the second and (1, -1) for the third and most massive object. … In Tab. 1 we present these
coordinates, and the initial realization of our perturbed solution."

**Burrau's standard as the repo records it** (`docs/design/principia_dd_validation_orbits.md` § 2, the Burrau
constants, REQ-VAL-174): "masses `(3, 4, 5)` at rest at `(1, 3)`, `(−2, −1)` and `(1, −1)` with `G = 1`", centre of mass
at the origin, `E = −769/60 = −12.8167`.

**Finding: Table 1 is Burrau's standard and is what the converged run used; Appendix B's `(-2, 1)` is a sign slip in the
text.** Evidence, from the reconciled table (`table2/conservation_check.txt`):

1. **The t = 200 rows are the start, returned to.** The run integrates to t = 100, reverses every velocity and
   integrates to t = 200 (§ 2: "At this moment, we compare the resulting positions and velocities with the original
   initial conditions"). The unperturbed t = 200 positions lie within 1.9×10⁻¹³ of Table 1's (mass 4 at
   `(-1.99999999999997…, -0.99999999999981…)`), and 2.0 from Appendix B's `(-2, 1)` in mass 4's y; the perturbed run's
   within 3.2×10⁻¹³ of Table 1's perturbed start (its mass 3 at x = `1.000000000100049…`, the 10⁻¹⁰ offset kept).
2. **Energy.** Table 1 gives `E₀ = −769/60 = −12.81666…`; Appendix B's positions give `−(32/√13 + 15/4) =
   −12.62520…`. The unperturbed run's E at t = 100 is `−12.816666666666666666666413…`, Table 1's to 2×10⁻²³
   relative, and 1.5×10⁻² from Appendix B's.
3. **Centre of mass.** Table 1's start has its centre of mass at the origin; Appendix B's at `(0, 2/3)`. The table's
   centre of mass is at the origin to 5×10⁻²⁸ at both times.

So nothing is unsettled about which start was used, and no RQ is needed for it. The run's units are `G = 1` with the
positions and masses as given ("The values are dimensionless modified N-body units", Appendix B; § 2: "the units of mass
and length are implicitly defined in Table 1").

## The consistency check (a record, not a target result)

From the reconciled table, in 60-digit decimal arithmetic (`scripts/conserve.py`; output in
`table2/conservation_check.txt`). Nothing is integrated.

| State | E | \|E − E₀\| / \|E₀\| | L | (Px, Py) | centre of mass |
|---|---|---|---|---|---|
| unperturbed, t = 0 (Table 1) | −12.81666666666666666666666666666… | — | 0 | 0 | (0, 0) |
| unperturbed, t = 100 | −12.81666666666666666666641349387… | 2.0×10⁻²³ | 1.2×10⁻²⁶ | (8.3×10⁻²⁹, −1.8×10⁻²⁹) | (4.9×10⁻²⁸, −1.3×10⁻²⁸) |
| unperturbed, t = 200 | −12.81666666666666666666602749063… | 5.0×10⁻²³ | −2.3×10⁻²⁶ | (−3.0×10⁻²⁹, 5.6×10⁻³⁰) | (−2.8×10⁻²⁹, −2.0×10⁻²⁹) |
| perturbed, t = 0 (Table 1) | −12.81666666663786666666553319166… | — | 0 | 0 | (2.5×10⁻¹¹, 0) |
| perturbed, t = 100 | −12.81666666663786666666502468546… | 4.0×10⁻²³ | 1.0×10⁻²⁶ | (1.1×10⁻²⁹, −1.4×10⁻²⁹) | (2.5×10⁻¹¹, −7.8×10⁻³⁰) |
| perturbed, t = 200 | −12.81666666663786666666459015069… | 7.4×10⁻²³ | −2.5×10⁻²⁶ | (−6.1×10⁻²⁹, 3.0×10⁻²⁹) | (2.5×10⁻¹¹, 2.0×10⁻²⁸) |

- **Energy** holds to about 22–23 significant digits of the 40 printed, in both runs and at both times: the table is
  consistent with Brutus at ε = 10⁻²⁴, and the digits past about the 23rd are not conserved quantities' digits.
- **Angular momentum** stays at 0 to 2.5×10⁻²⁶ and **momentum** at 0 to 6×10⁻²⁹.
- **The centre of mass is at rest**: at the origin to 5×10⁻²⁸ in the unperturbed run, and at `(2.5×10⁻¹¹, 0)` in the
  perturbed one, which is exactly `3 × 10⁻¹⁰ / 12`, the offset's own shift, at both times.
- The t = 200 velocities are at most 8.1×10⁻¹⁴, against 0 at the start; the positions at most 3.2×10⁻¹³ from it. This is
  what the paper means by recovering the initial offset (10⁻¹⁰) after 9 orders of growth.

## What the target would test, and its horizon

PLAN.md § 3.2, row **SP-2**, proposed (see its dated note under the § 3.2 table). In short: pointwise agreement with the
converged state at t = 100 and at t = 200 (after the velocity reversal at t = 100), each under a convergence
reference; escape near t ≈ 63.4. The paper's escape time is its own: "one particle escapes at t ≃ 63.4" (§ 2), its
footnote 3 defining escape ("receding and its binding energy with respect to the system is negative"), and footnote
4 dating the dissolution by a fit to δ(t): "At t ≃ 63.4 the linear behavior overtakes the exponential behavior of δ".

The run's whole length is t = 200, which is R-610's cap exactly; the horizon note is in PLAN.md beside the row.
