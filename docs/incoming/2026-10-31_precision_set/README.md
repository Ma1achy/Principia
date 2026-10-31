# The precision set of 31 Oct 2026: what the papers and records establish about f32, and what they do not

A record, not authority (R-1, R-159). The ruling is R-661. Nothing here is a target until adopted; `PLAN.md`'s BZ-1C is
the proposal, and every threshold stays a calibration requirement (R-71).

**In one line:** no paper in this set tests the ensemble statistics of an f32 N = 3 run. Whether tier (a)'s f32 statistics
agree with f64's or with a converged reference is a **hypothesis Principia will measure**.

Labels: **[paper]** the authors state it, quoted from the file named. **[ran]** computed here, by `tools/`, output beside
the script. **[inference]** this record's reading.

## Files, citations, licences

Every licence below was read at its source on 31 Oct 2026 (R-643). No request carried an email address; the user agent
was curl's.

| File | sha256 | Bytes | Citation | Licence, at source |
|---|---|---|---|---|
| `papers/boekholt_pz_2015_ComAC_2_2_version_of_record.pdf` | `11db8248f2239e76008a92513b5426f8461ed507534cdf47a6495707ac2c5f68` | 3,557,232 | T. Boekholt & S. Portegies Zwart, "On the reliability of N-body simulations", *Comput. Astrophys. Cosmol.* 2, 2 (2015), doi:10.1186/s40668-014-0005-3 | CC BY 4.0: the PDF's p. 1, "distributed under the terms of the Creative Commons Attribution License (http://creativecommons.org/licenses/by/4.0)"; Crossref's `vor` licence the same URL |
| `data/40668_2014_5_MOESM1_ESM.dat` | `373afad6cbb9d7d9e1c67d8bbb52e259cc6f71b35fa53b7f7203f5233894ce1b` | 13,217,507 | Boekholt & Portegies Zwart (2017), MOESM1 of *On the reliability of N-body simulations*, figshare, doi:10.6084/m9.figshare.c.3696685_D3.v1; warm equal masses | CC BY 4.0: figshare's API, this article (4671367) |
| `data/40668_2014_5_MOESM2_ESM.dat` | `b3ef991242e84196c6aff1b8c37cd7c62bdc437059b59053507b30b18817a090` | 11,776,588 | Likewise MOESM2, doi:10.6084/m9.figshare.c.3696685_D1.v1; warm, stored masses 4/7, 2/7, 1/7 | CC BY 4.0: this article (4671328) |
| `data/40668_2014_5_MOESM3_ESM.dat` | `7ebc21aa5ecce9b23b0ac2a1db2eccb2e8bd3f59ea7555360d6231f38ac494ae` | 11,557,802 | Likewise MOESM3, doi:10.6084/m9.figshare.c.3696685_D2.v1; cold equal masses | CC BY 4.0: this article (4671346) |
| `data/40668_2014_5_MOESM4_ESM.dat` | `2c29d50557ab0cd0241ebd1ec064a53c4383806e5fbe3cb88d86c7a961a2d5a8` | 10,590,959 | Likewise MOESM4, doi:10.6084/m9.figshare.c.3696685_D4.v1; cold, stored masses 4/7, 2/7, 1/7 | CC BY 4.0: this article (4671382) |
| `papers/portegies_zwart_2026_arXiv2608.17032v1.pdf` | `b2421b3025e6233e333be419b839e501195c5d5e9cc9b5e9974872a8a3ba74dc` | 1,266,677 | S. Portegies Zwart, "Validating direct solvers for Newton's gravitational N-body problem, and the systematic comparison between IEEE floating point and Posits", arXiv:2608.17032v1 (17 Aug 2026), a preprint | CC BY 4.0: arXiv's abstract page |
| `papers/portegies_zwart_boekholt_2023_arXiv2209.03347v1.pdf` | `8ed7848228527b3329f1b5b99e9f25058bc2a3bf412ce855478fd10d4a41e9f4` | 860,180 | S. Portegies Zwart & T. Boekholt, "The paradox of infinitesimal granularity: Chaos and the reversibility of time in Newton's theory of gravity", arXiv:2209.03347v1; *AIP Conf. Proc.* 2872, 050003 (2023), doi:10.1063/5.0163734 | CC BY 4.0: arXiv's abstract page, for this arXiv copy |

All four data files came from figshare (`ndownloader.figshare.com/files/` 7624663, 7624624, 7624642, 7624678); each file's
MD5 matches figshare's record, and MOESM3 and MOESM4 are byte-identical to the owner's copies. The arXiv PDFs were fetched
again from arXiv and match the owner's copies.

Kept out of the repo, in `docs/incoming/_local/precision_set/` (ignored, `.gitignore` line 21; R-643):

| File | sha256 | Citation | Licence, at source |
|---|---|---|---|
| `portegies_zwart_boekholt_2014_ApJL_785_L3_arXiv1402.6713.pdf` | `6ed1dc83e81961fa258023d3716afe84556c4c182144809003da514d5fcc7900` | Portegies Zwart & Boekholt, *ApJL* 785, L3 (2014), doi:10.1088/2041-8205/785/1/L3, arXiv:1402.6713 | arXiv non-exclusive distribution licence 1.0 |
| `boekholt_pz_heggie_2023_IJMPD_32_2342003_arXiv2308.14803.pdf` | `707f3968ca7977559bb5040ee5861fbb0264c1a5d8e0bf24167aa20ad8692224` | Boekholt, Portegies Zwart & Heggie, *IJMPD* 32, 2342003 (2023), doi:10.1142/S0218271823420038, arXiv:2308.14803 | arXiv non-exclusive distribution licence 1.0 |
| `rein_brown_kanda_2025_OJAp_8_arXiv2507.04987v2.pdf` | `cb0783bdfbda1eabef06e8d33d223f558de91a36f272330eefe52fb696f94575` | Rein, Brown & Kanda, *Open J. Astrophys.* 8 (2025), doi:10.33232/001c.154745, arXiv:2507.04987v2 | arXiv: CC BY-NC-SA 4.0, not an allowed licence. Crossref lists CC BY 4.0 for the journal's version, which is not held |

`pdftotext` drops the digits from the version of record's body, so numbers were read from the arXiv copy already in the
repo, `../2026-10-25_replication/papers/boekholt_pz_2015_ComAC_2_2_arXiv1411.6671.pdf`. Page numbers below are the
version of record's where given.

## What each paper establishes

**Boekholt & Portegies Zwart 2015 (BPZ).** Hermite, f64, against Brutus, N = 3, four sets of 10,000. **No f32 run**: the
word "single-precision" appears in passing in the introduction.
- Abstract **[paper]**: "the error due to divergence is unbiased, as long as the time-step parameter, η ≤ 2⁻⁵ and when
  simulations which violate energy conservation by more than 10% are excluded."
- Fig. 6's caption **[paper]**, the KS result, which is a different and stricter condition: "For η < 2⁻⁵, the
  distributions are not significantly different."; § 5.2 **[paper]**: "the distributions of the cold systems become
  significantly different for η > 2⁻⁶".
- § 6.1 **[paper]**: "If we filter out simulations with a relative energy conservation |∆E/E| > 1, or |∆E/E| > 0.1, we
  observe that the bias in the average error of the semimajor axis of the binaries vanishes." The count removed is not
  given. Fig. 14 shows the whole ensemble beside the two cuts.
- § 6.3 **[paper]**: "In our experiment, a maximum level of relative energy conservation of |∆E/E| = 0.1 was sufficient to
  remove the biases."
- § 7 **[paper]**: "possible differences on the long term are not visible in this experiment."
- The 3-body cold collapse "is essentially a two-dimensional problem" **[paper]**.

**Portegies Zwart 2026.** The only 32-bit numbers in the set, each from **one run**: Table 4 at η = 10⁻⁴, 32-bit float,
|dE/E₀| = 1.3×10⁻⁴ (figure-eight), 5.7×10⁻¹ (Pythagorean). No ensemble, no KS.
- Abstract **[paper]**: "Single precision (fp32, and Posits<32,2>) could be used for statistical ensemble calculations,
  but lead to relatively large errors in any individual strong encounter."
- § 4.2.1 **[paper]**: "None of the 32-bit solutions … is sufficiently reliable for any scientific computation"; "dominated
  by round-off errors over the entire range of η". § 5 **[paper]**: "32-bit precision tends to lead to unacceptable errors."
- **[inference]** The abstract's fp32 sentence has no experiment behind it in this paper; it is not evidence.

**Portegies Zwart & Boekholt 2023.** The fraction of irreversible triples against tolerance, by forward-and-back Brutus
runs; a clause citing BPZ on statistics, no new test. **No f32.**

**Portegies Zwart & Boekholt 2014 (`_local`).** The same four sets, f64 against Brutus; its energy bound is, in its own
abstract, "we conjecture". **No f32.**

**Boekholt, Portegies Zwart & Heggie 2023 (`_local`).** A model of punctuated chaos. It calls the statistical validity of
approximate solutions "an article of faith". **No f32.**

**Rein, Brown & Kanda 2025 (`_local`).** The Solar System, f64, a step scan: not three bodies, and **no f32**. A KS test on
secular frequencies passes where instability rates differ, and it keeps every run. A caution on KS and on energy, not a
result for this problem.

## Against never discard

Our rule: "a failed or budget-exhausted run is shown as such, with its end time (R-563)"
(`docs/design/principia_honesty_precision.md` § 4).

| BPZ | Verdict |
|---|---|
| Fig. 14: the ensemble shown cut and uncut | Compatible as a method, once the count removed is given. BPZ omit it |
| "unbiased … when … excluded" | Not compatible as a claim: it holds only after the cut. Principia can say "after this cut, n removed", beside the uncut result, never "unbiased" |
| Long-lived runs: last state kept, fraction measured | Compatible |
| All-zero placeholder rows: no time, no reason, no flag | Conflicts on "with its end time". R-661: undetermined or censored, never dropped |

## The records, measured

Each file: a header, 10,000 rows, 46 fields: index; `m x y z vx vy vz` for each body, initial then final; `t e Lw`, where
`e` is the Bulirsch-Stoer tolerance and `Lw` the word length. **Only Brutus's side is there. The Hermite results are not
published**, so BPZ's KS tests cannot be recomputed, and any f64 ensemble on these starts is ours.

**[ran]** `tools/measure_records.py`, output in `tools/measure_records_output.txt`. Out of plane is measured from the plane
through the three bodies at t = 0.

| | MOESM3 cold equal | MOESM4 cold 1:2:4 | MOESM1 warm equal | MOESM2 warm 1:2:4 |
|---|---|---|---|---|
| Largest start speed | 0 | 0 | 0.991 | 1.70 |
| \|L₀\|, median | 0 | 0 | 0.172 | 0.143 |
| **Largest final out-of-plane distance** | **4.29×10⁻⁹** | **3.41×10⁻⁹** | **84.9** | **132** |
| Same, median over the runs | 4.4×10⁻¹⁶ | 6.1×10⁻¹⁶ | 2.93 | 5.63 |
| Largest final out-of-plane speed | 2.0×10⁻⁹ | 2.1×10⁻¹⁰ | 4.22 | 6.70 |
| All-zero finals (t, e, Lw zero too) | 98 | 16 | 95 | 98 |
| Stored t = 501, last state kept | 1,364 | 851 | 3,497 | 3,047 |
| Undissolved at 501, placeholders counted so | 14.6% | 8.7% ("∼10%", BPZ § 7) | 35.9% | 31.4% |
| Ended at t ≤ 200 Hénon | 6,682 | 8,202 | 4,376 | 5,162 |

**The cold sets are planar as measured.** The cause, from the owner: from rest means planar. The cold records hold every
body's motion in its starting plane to the digits their convergence keeps.

**The warm files are 3D**, by the same measure, and start with velocities out of the plane and non-zero L: the BZ-1
objection stands for them, and **nothing is proposed from them**. They were fetched for completeness, as the owner asked, and committed per R-369: each is CC BY 4.0.

**The horizon.** R-610's cap of 200 is in the integrator's units, `M = G = I = 1` (integrator contract Part 5). BPZ's are
`M = G = 1`, `E = −0.25`, so a start with inertia `I` has `t_Hénon = t_int · I^{3/4}`. **[ran]** T = 200 is, in Hénon units,
135 to 297 (median 166) for cold equal masses, and 95.5 to 257 (median 113) for cold 1:2:4. Runs ended within it: **6,257
and 7,366 of 10,000**. The others are censored at T. Reaching the records' own 500 is past the cap for every cold start, and
R-610 stands.

**[inference]** Three different "0.1"s must not be read across: BPZ's relative energy cut, BPZ's phase-space distance
δ ≈ 10⁻¹, and Principia's parted mark, a shape-sphere chord (R-574).

## The power calculation

**[ran]** `tools/power.py`, output in `tools/power_output.txt`. α = 0.05, n per sample.

Two-sample KS: `D_c = c(α) √(2/n)`, `c(0.05) = 1.358`. A Monte Carlo of the test with the true distance set to D_c gives
power 0.83 to 0.90 (n = 256 and 10⁴; a normal shift and mass moved to a tail), so D_c reads as the distance detected.

| n | D_c | D_c, Bonferroni over 12 tests | Fraction difference at 80% power, p = 0.5 / 1/3 / 0.1 |
|---|---|---|---|
| 256 | **0.120** | 0.155 | 0.124 / 0.117 / 0.074 |
| 1,000 | 0.061 | 0.079 | 0.063 / 0.059 / 0.038 |
| 4,096 | 0.030 | 0.039 | 0.031 / 0.029 / 0.019 |
| 6,257 (cold equal, ended by T = 200) | 0.024 | 0.031 | 0.025 / 0.024 / 0.015 |
| 7,366 (cold 1:2:4, ended by T = 200) | 0.022 | 0.029 | 0.023 / 0.022 / 0.014 |
| **10,000** | **0.019** | 0.025 | **0.020 / 0.019 / 0.012** |

- `D_c = 0.02` needs **n = 9,222**, the owner's n ~ 10⁴; 15,434 with Bonferroni over 12 tests (four statistics, three
  comparisons). A fraction difference of 0.02 at p = 0.5 needs n = 9,811, of 0.05 n = 1,570, of 0.01 n = 39,244.
- **Pairing helps little where runs have parted.** McNemar's `n = (z_{α/2} + z_β)² d/Δ²`, `d` the discordant fraction:
  `d = 0.33` gives n = 6,475 at Δ = 0.02. BPZ find about half their f64 runs diverged, so `d` is not expected to be small at
  f32. It is measured, not assumed; n stays ~10⁴. Two-sample KS on paired samples is conservative where pairs
  coincide, so the paired tests run beside it.
- **Lifetimes, by log-rank**, total events `4 (z_{α/2} + z_β)² / ln²HR`: at T = 200 with 10⁴ a side, 12,514 and 14,732
  events detect a hazard ratio of 1.051 and 1.047.
- **So:** n = 10⁴ per set, all the records hold, gives a sensitivity of 2% in a whole-ensemble fraction, 2 to 3% in KS
  distance on end states, and 5% in hazard. Going below 2% needs starts beyond the records, whose converged side would be
  ours as well.

## What cannot be claimed

- "f32 ensemble statistics match f64's, or the true ones", at any N. No paper here tests it.
- "fp32 is fine for ensembles (Portegies Zwart 2026)". The sentence exists; the experiment does not.
- "|ΔE/E| < 0.1 is sufficient", or "η ≤ 2⁻⁵", as a rule for Principia. BPZ say "in our experiment", for Hermite.
- That f32 on a GPU behaves like f32 here: every 32-bit number above is one code on a CPU.

Scope: this set and the library's own triage notes, not an exhaustive literature search, so the gap stays worded as
`PLAN.md` § 7 words it: "not found in these records".
