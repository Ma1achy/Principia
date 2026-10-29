# RQ-598 Brutus cost pilot: report

**Brutus, run here.** Every number below was computed by us, in one cloud session, with Brutus as pinned (`pgbreen/NvMgeneration` @ `98906f4`, GPL-3.0).
None is the papers' result. Machine: Intel Xeon @ 2.80 GHz, 4 vCPU, g++ 13.3 `-O1`, three jobs side by side. Wall times will differ on the Mac.
**M** = measured here, **E** = estimated (a model applied to measured numbers). Nothing was run below ε = 10⁻⁶. Nothing was scaled: 16 GG-1 points and 12 BR-1 trajectories.
Files: `docs/incoming/2026-10-28_rq598_data/brutus_run_here/` on branch `data/rq598-brutus-pilot` (no PR).

## Decisions needed

1. **Lifetime definition or unit convention (GG-1).** Our T and the paper's Fig. 1 disagree at the only points where the paper's own two panels agree, and ours does not move with ε there (below). Neither the analytic nor the history-based reading of Appendix A closes it, and the ratios are not one time-unit factor. This needs a ruling (a definition requirement, R-72) before any point-by-point comparison with route 2 means anything.
2. **Scaling.** The numbers below are what the pilot supports. Going below ε = 10⁻⁶ to calibrate the 10⁻⁷⁰ estimate (one point each at 10⁻⁸ and 10⁻¹⁰ would do) needs your go-ahead.
3. Still open from the README: may a self-computed ensemble score a target, or is it a cross-check only?

## Findings that change how to read everything else

- **The pinned Brutus never applies the word length.** `main.cpp` reads L_w, echoes it, and nothing sets MPFR's precision. The stock executable runs at 53 bits for any L_w (identical output for L_w = 20, 53, 128, 1024: `stock_lw_test.txt`). All our "L_w = 1024 / 128 / 88" runs use our own driver (`src/pilot.cpp`), which sets it, linked to the unmodified Brutus objects. If the authors' NvM data came from this `main.cpp` unchanged, their "L_w = 128" did not take effect. The repo cannot show which they used. Worth asking in the emails.
- The stock source needs two build-flag deviations on current Ubuntu, no source edits (`INSTALL_RECORD.md`).
- **p_max is not stated in the GG-1 paper and moves the cost a lot** (M, ε = 10⁻⁶, L_w = 1024, two points): p_max = 2 is about 10× slower than 4 or worse (one point hit the 300 s cap); p_max = 8 and 16 cost about 0.5–0.6× of 4. We used 4, the value in the repo's sample inputs.
- A wall cap is checked between Bulirsch–Stoer steps, not inside one. One BR-1 run at ε = 10⁻² ran over 10 minutes inside a single step and was killed by hand (abandoned; its row is in `results_br1.csv`). ε = 10⁻³ on GG-1 point (27, 24) took 59,040 steps by t = 0.25 and hit the 600 s cap. Cost is not monotonic in ε: very loose tolerances break the step control.

## 1. Grid (gate passed)

The 1/64 grid on the Agekyan–Anosova map has **exactly 1212 points** (i = 1–31, j = 1–54; bodies 1 and 2 at (∓½, 0), body 3 at (i/64, j/64), 0 < x < ½, y > 0, (x+½)² + y² < 1; tested in integers). It is identical, as a set, to the Fig. 1 extraction on `data/rq598-gg1-fig1-v2` (`gg1/make_grid_output.txt`). Units are m = 1, G = 1, r₁₂ = 1 (the repo's Anosova convention); the paper's "time units (Heggie & Mathieu)" may differ (see decision 1).

## 2. GG-1 at ε = 10⁻⁶, L_w = 1024: 16 points (M)

Sample: stratified on the paper's own Fig. 1 lifetimes and spread over the map (including its 3 black T > 1000 points), 16 of 1212. Fig. 1 was used only to choose points. T uses the paper's lifetime definition: first time the single is > 10 units from the binary centre of mass, moving away, with positive energy; then retraced to the closest binary–single approach (Kepler retrace; the integrated-history reading agrees to 0.1–1%, `gg1/pilot_fwdonly_final_driver_*.csv`). T is capped at 1000. "Convergence" is the paper's reversibility test: reverse velocities at the escape time, integrate on to twice that time, pass if log₁₀Δ < −3. p_max = 4.

| (i, j) | x, y | T (ours) | escape detected at | steps | forward wall s | reverse leg: steps, wall s | log10 Δ (pass if < −3) | paper Fig. 1 left, log10 T |
|---|---|---|---|---|---|---|---|---|
| 1, 1 | 0.0156, 0.0156 | 69.42 | 73.03 | 10882 | 139.0 | 9697, 63 | 1.57 | 1.27 |
| 1, 23 | 0.0156, 0.3594 | 15.33 | 17.56 | 1814 | 13.6 | 1737, 10 | 0.17 | 2.29 |
| 1, 54 | 0.0156, 0.8438 | 0.6329 | 2.383 | 877 | 6.2 | 870, 6 | -0.84 | 0.52 |
| 2, 34 | 0.0312, 0.5312 | 45.39 | 49.47 | 4038 | 31.0 | 4540, 41 | 1.14 | 1.93 |
| 3, 43 | 0.0469, 0.6719 | 0.5706 | 9.144 | 567 | 3.4 | 568, 3 | -1.44 | 2.71 |
| 4, 13 | 0.0625, 0.2031 | 6.996 | 10.17 | 1846 | 21.5 | 2128, 22 | 0.99 | 2.07 |
| 8, 37 | 0.1250, 0.5781 | 31.09 | 32.96 | 4056 | 63.1 | 81176, 1200 (hit 1200 s cap) | n/a | >3 (black) |
| 9, 3 | 0.1406, 0.0469 | 13.48 | 15.25 | 3018 | 38.2 | 6249, 67 | 1.56 | >3 (black) |
| 12, 30 | 0.1875, 0.4688 | 14.73 | 19.72 | 1462 | 12.3 | 1528, 7 | 1.04 | 2.61 |
| 14, 20 | 0.2188, 0.3125 | 1.339 | 3.617 | 1340 | 15.1 | 1317, 15 | 0.08 | 0.69 |
| 15, 6 | 0.2344, 0.0938 | 95.64 | 99.92 | 19837 | 204.8 | 16705, 96 | 1.98 | 1.67 |
| 17, 41 | 0.2656, 0.6406 | 19.77 | 23.81 | 1982 | 26.6 | 2628, 20 | 1.42 | 1.25 |
| 23, 29 | 0.3594, 0.4531 | 15.88 | 20.25 | 1683 | 15.2 | 1229, 7 | 0.22 | 2.12 |
| 27, 24 | 0.4219, 0.3750 | 4.135 | 6.476 | 1024 | 9.7 | 1031, 8 | 0.03 | >3 (black) |
| 28, 1 | 0.4375, 0.0156 | 1.555 | 4.272 | 5274 | 80.2 | 3913, 22 | 0.45 | 1.04 |
| 29, 17 | 0.4531, 0.2656 | 14.31 | 19.84 | 2587 | 17.2 | 2498, 14 | -0.09 | 1.58 |

- **All 16 escaped; none was censored.** T runs from 0.57 to 95.6. Even the three points the paper draws black at ε = 10⁻⁶ escaped (T = 4.1, 13.5, 31.1). That is expected: the paper says the two panels differ point by point.
- **Cost per point, forward leg:** wall median **19 s** (mean 44, p10 8, p90 110, range 3–205); steps median 1914 (567–19,837); 9.9 ms per step (6.0–15.6); **1.7 s per unit of simulated time** (median; p10 0.6, p90 3.4, max 18.8 for the close-encounter point (28, 1)).
- **Reverse leg:** median 17 s. One of 16 (point (8, 37)) ran 81,176 steps and hit the 1200 s cap: the reverse leg has a heavy tail the forward leg does not.
- **Reversibility: 0 of 15 passed** (log₁₀Δ from −1.4 to +2.0). At ε = 10⁻⁶ nothing is converged, as the paper's Fig. 2 has it (f_irr near 1). Energy error at escape: median 3.9×10⁻³.
- **L_w** (M): point (10, 10): 128 bits 21 s, 1024 bits 44 s, same steps (×2.1, run alone). Point (1, 54) side by side with other jobs: 5.8 / 6.1 / 6.5 / 8.1 s at 128 / 256 / 512 / 1024 (×1.4). Steps are identical at every L_w, as they should be at ε = 10⁻⁶.
- **ε** (M, 1024 bits, ms per step; point (1, 54) / (14, 20)): 10⁻³ 2.0 / 1.8; 10⁻⁴ 3.4 / 6.6; 10⁻⁵ 7.2 / 11.7; 10⁻⁶ 7.0–9.2 / 11.2 (the spread is load: the same point ran in two different mixes). It rises, then flattens near 10 ms.
- Determinism: a forward-only re-run with the final driver reproduced steps, T and status on 16 of 16.

### Where ours and the paper's Fig. 1 disagree (M; context for decision 1)

At (1, 54), (3, 43), (14, 20) the paper's ε = 10⁻⁶ and ε = 10⁻⁷⁰ panels give the same lifetime (T = 3.3, 515, 4.85). Ours is T = 0.633, 0.571, 1.339, and does not change with ε over 10⁻⁴ … 10⁻⁶ (0.6329359 / 0.6329355 / 0.6329359; 0.57059 at all three; 1.3376 / 1.3389 / 1.3393; `gg1/robustness_three_points_Lw128_norev.csv`). The ratios (5.2, 900, 3.6) are not one time-unit factor. (1, 54) is near the equilateral corner; our T = 0.63 is a close triple approach (minimum separation 0.01). At the other points the two panels differ from each other and from ours, which the paper expects. I did not run more points to test conventions: that would exceed the stated sample.

### Projection: the full ε = 10⁻⁶ panel at L_w = 1024 (E)

Weights are the lifetimes in the paper's own Fig. 1 left panel (statistically the same as ours, K–S p = 0.72): Σ T over 1212 points = 85,690, of which 26,000 is the 26 black points counted at the cap of 1000. Our measured rate (1.7 s per unit time median, 2.7 mean) and integrated-time / T ratio (median 1.31) give:

- **Forward leg only: about 40–105 core-hours (central 65–85).**
- **With the reversal leg (+0.82× the forward cost, median): about 75–190 core-hours (central 115–150).** About 1–2 days of wall time on four cores like these.
- Weak points: our longest T is 96, so the T = 100–1000 tail is extrapolated from the rate; and the reverse leg's heavy tail (1 of 16 capped) could add to it.

### Projection: ε = 10⁻⁷⁰ at L_w = 1024 (E; order of magnitude only)

Not extrapolable from ε ≥ 10⁻⁶, and I do not trust a single number. Brackets:

- The code's own step law (`Brutus::get_eta`) shrinks steps by η(10⁻⁶)/η(10⁻⁷⁰) = 0.337/0.0263 = **12.8×**. That alone gives 12.8 × (40–105) = 500–1,300 core-hours forward.
- Cost per step rises with tightness and may flatten (we see it flatten near 10 ms from 10⁻⁵ to 10⁻⁶). A further 1–10× on top gives 500–13,000 core-hours.
- The paper (Gargantuan, §2) says the average simulation took about 7 hours and the longest one month, on unstated hardware. If that is per 10⁻⁷⁰ run, 1212 × 7 h ≈ 8,500 core-hours.
- So **order 10³–10⁴ core-hours forward (a similar amount again for the reversal leg), uncertain by an order of magnitude either way, and p_max could move it by more.** Two measurements, one point each at ε = 10⁻⁸ and 10⁻¹⁰, would turn this into a fit. They need your go-ahead.

## 3. BR-1: the authors' own initial conditions, ε = 10⁻⁶, L_w = 128 (M)

Per your update: no random draw. Twelve of the authors' runs from `pgbreen/NVM` @ `a35c378`, from `trainex/data` and `validex/data` (180 files; 136 have the full 2561 rows, 44 stop early): 9 full-length runs (three from each tertile of minimum pair separation along their trajectory) and 3 of the truncated ones. Row 1 gives x₁ = (1, 0) and x₂; x₃ from the barycentre; equal masses at rest; outputs every 2⁻⁸. Our trajectory is compared with theirs row by row. Their ε = 10⁻¹¹ / 10⁻¹⁰ pairs were not run.

| run | set | authors' rows | steps | wall s (to authors' last row) | wall s to t = 10 | first t where our position differs from theirs by > 1e-3 / 1e-2 / 0.1 | mean δ (positions) < 0.1 |
|---|---|---|---|---|---|---|---|
| run9941 | validex | 2561 | 2679 | 1.42 | same | 5.39453 / – / – | yes (0.000423) |
| run9968 | validex | 2561 | 2682 | 1.74 | same | 3.50391 / 7.46875 / 9.42188 | yes (0.0191) |
| run9956 | validex | 2561 | 2661 | 1.53 | same | 3.31641 / 4.75 / 6.33594 | no (0.104) |
| run28 | trainex | 2561 | 2608 | 1.12 | same | 2.52734 / 6.72266 / 8.92969 | yes (0.0318) |
| run14 | trainex | 2561 | 2653 | 1.60 | same | 3.8125 / 9.14453 / – | yes (0.000921) |
| run53 | trainex | 2561 | 2626 | 1.18 | same | 3.03125 / 9.85547 / – | yes (0.00146) |
| run86 | trainex | 2561 | 2694 | 2.06 | same | 0.75 / 4.23828 / 5.12109 | no (0.276) |
| run9946 | validex | 2561 | 2592 | 1.08 | same | 5.09766 / – / – | yes (0.00167) |
| run99 | trainex | 2561 | 2563 | 1.45 | same | – / – / – | yes (7.46e-05) |
| run9992 | validex | 33 | 33 | 0.03 | 6.67 | – / – / – | yes (9.14e-10) |
| run9994 | validex | 403 | 408 | 0.23 | 1.72 | – / – / – | yes (5.66e-06) |
| run9959 | validex | 2298 | 2445 | 2.03 | 2.44 | 2.75 / 4.83594 / 7.95312 | yes (0.0215) |

- **Our build tracks theirs.** At t = 0 the difference is exactly 0 (the initial condition is identical). Over t ≤ 1 the largest position difference is below 3×10⁻⁵ in 11 of 12 runs (median 4×10⁻⁶; run86: 2.5×10⁻³), the size ε = 10⁻⁶ allows against their ε = 10⁻¹¹ solution. It grows by roughly 3–4× per decade of ε, not uniformly (10⁻⁶, 10⁻⁵, 10⁻⁴: `compare_to_authors.csv`). Runs then separate after close encounters, as chaos requires: first difference above 10⁻³ at t = 0.75–5.4 (median 3.3) in 9 runs, and 3 runs stay within 10⁻³ for the whole of their rows.
- By the authors' criterion (average phase distance < 0.1; ours uses the 6 position terms, their files hold no velocities) **10 of 12 pass at ε = 10⁻⁶** against their solution (δ_pos 9×10⁻¹⁰ … 0.28).
- **Cost to t = 10 (M):** full-length runs at ε = 10⁻⁶, L_w = 128: wall median **1.45 s** (1.1–2.1), about 2650 steps (the 2561-output floor), 0.57 ms per step. All 12 run to t = 10: median 1.6 s, mean 2.0, max 6.7 (run9992, minimum pair separation 9×10⁻⁸). Energy error is 5×10⁻⁵ … 9×10⁻³ at t = 10.
- **ε dependence (M, 9 full runs, L_w = 128, median wall / ms per step):** 10⁻³ 0.61 s / 0.24; 10⁻⁴ 0.89 / 0.34; 10⁻⁵ 1.19 / 0.45; 10⁻⁶ 1.45 / 0.57. Linear in −log₁₀ε, about +0.11 ms per step per decade. **L_w = 88 vs 128 at 10⁻⁶: 1.42 vs 1.45 s, the same** (both fit in two 64-bit limbs).

### Projection: a BR-1 ensemble at the authors' pairs (E)

Model: steps × ms per step, with ms per step extrapolated linearly in −log₁₀ε (4–5 decades beyond the fit, the weak step), steps 2650–3050, and a tail factor 1.28 (mean / median of our 12 runs). Typical run: **(10⁻¹¹, 128 bits) 2.9–3.4 s; (10⁻¹⁰, 88 bits) 2.6–3.0 s.**

| N | one pair | both pairs (the authors' convergence test needs two) |
|---|---|---|
| 1,000 | 1.0–1.2 core-hours | 2.1–2.4 |
| 10,000 (the authors' size) | 10–12 core-hours | 21–24 |

The authors' paper says generating its data "required over 10 days of computer time". Our figure is 10–20× lower. Reasons it may be low: it is the typical run; the 12-run sample cannot show the hard tail (their 44 truncated files mark where convergence failed for them); their tuning iterated over several (ε, L_w) pairs; and their hardware is unknown. Treat it as the cost of a typical run, not of the ensemble.

## Not done, by design

No run below ε = 10⁻⁶; no scale-up of GG-1 (1196 points remain) or BR-1; no AMUSE copy of Brutus; V-11 and the REVIEW_QUEUE/ruling bookkeeping of §4 task 1 (not part of this brief). Nothing from `pgbreen/NVM` is committed. The commit trailers name Claude Sonnet 5.5 and this session, not the Opus line in the brief, because that is the model that wrote them.
