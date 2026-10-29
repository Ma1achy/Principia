# RQ-598 Brutus cost pilot: report

**Brutus, run here.** Every number below was computed by us, with Brutus as pinned (`pgbreen/NvMgeneration` @ `98906f4`, GPL-3.0), on a
Linux machine (Intel Xeon @ 2.80 GHz, 4 vCPU, g++ 13.3 `-O1`, three jobs side by side; `INSTALL_RECORD.md`). None is the papers' result. Wall
times will differ on the Mac. **M** = measured here, **E** = estimated (a model applied to measured numbers). Nothing was run below ε = 10⁻⁶
except one point at ε = 10⁻⁸ and 10⁻¹⁰ (§ 2b). Nothing was scaled: 16 GG-1 points and 12 BR-1 trajectories.

The Fig. 1 values used here are route 2's reading of the paper's figure, `../transcriptions/gg1_fig1/`, the one record of Fig. 1 in the repo.
The units conversion, the comparison with Fig. 1 in the paper's units, and the projections of § 2 and § 2c come from `hm_units.py` (output:
`hm_units_output.txt`), run from this folder as `python3 -I hm_units.py gg1 ../transcriptions/gg1_fig1/gg1_fig1_left_eps1e-6.csv
../transcriptions/gg1_fig1/gg1_fig1_right_eps1e-70.csv`.

## Open questions (`REVIEW_QUEUE.md`)

1. **GG-1 lifetime: definition, units and the (3, 43) cell** (RQ-612). In the paper's units, two of the three points where Fig. 1's two
   panels agree come within 0.3 dex of ours; (3, 43) is still 2.4 dex off. Its neighbours are to be run before any definition ruling (§ 2).
2. **`p_max`, the word length, and the ε = 10⁻⁷⁰ cost** (RQ-613). The cost is undetermined to orders of magnitude (§ 2c).
3. **May a self-computed ensemble score a target, or is it a cross-check only?** (RQ-614).
4. **BR-1's 180 public trajectories**, which have no licence (RQ-615).
5. **BR-1's convergence criterion: δ or δ²** (RQ-616, a definition requirement, R-72; § 3).
6. **Scaling.** Anything beyond the one-point probes below ε = 10⁻⁶ (the other 1196 GG-1 points, a BR-1 ensemble, more ε) needs the owner's
   go-ahead (`../README.md` § 5).

## Not verified from committed material

The stock Brutus sources (`main.cpp`, `Brutus.cpp`, `Bulirsch_Stoer.cpp`) are not committed, and the authors' NVM files are local only. These
claims rest on the pilot's own record and stay **unverified** until the Brutus source at `98906f4` is read or the runs are repeated:

- that the stock `main.cpp` never applies L_w, and that nothing in the source calls `set_default_prec` (the md5 test in `stock_lw_test.txt`
  is a recorded result, not reproducible from the repo);
- that the step loop in `src/pilot.cpp` is a line-for-line copy of `Brutus::evolve`;
- what `p_max` does (that it caps the extrapolation at `p_max` points), and the ×11.5 step factor attributed to `get_eta` below 10⁻⁵⁰;
- the NVM counts (180 files, 136 with the full 2561 rows, 44 truncated; only the 12 sampled run names and row counts are committed), and that
  these files are the paper's converged ε = 10⁻¹¹ solutions;
- every timing and every lifetime: none was re-run in review.

What review verified from the committed files: every statistic and projection below is reproduced by the scripts from the CSVs; the grid is
the paper's 1212 points; the initial conditions, escape test, Kepler retrace and reversibility test coded in `src/pilot.cpp` follow Appendix A
in the code's own units; route 2's Fig. 1 values equal the ones used here; our T at (1, 54) equals the equilateral free-fall collapse time.

## Findings that change how to read everything else

- **Units.** The paper gives times and distances in Heggie–Mathieu (HM) N-body units: G = 1, M = 1, E = −1/4 (Appendix A). The pilot ran in
  m = 1 per body, G = 1, r₁₂ = 1. At rest the conversion depends on the point: T_HM = T_ours · √(3λ³) and R_HM = R_ours · λ, with
  λ = (4/9) Σ 1/r_ij (`hm_units.py` derives it). Over the 1212 grid points the time factor runs from **2.72 to 167**, median 5.05. Check: an
  equilateral triangle of side 1 at rest collapses at T = 0.641 ours, 1.710 HM. Because the factor varies from point to point, ratios of our T
  to the paper's are not expected to be one constant; #334's argument that "the ratios are not one time-unit factor" is withdrawn.
- **The escape radius and the cap were applied in our units, not the paper's.** Our escape test used 10 of our length units, which is 13.5 to
  210 HM units (median 20.4) depending on the point; the paper's is 10 HM units (0.48 to 7.4 of ours). Our cap T ≤ 1000 is 2,719 to 166,791
  HM units; the paper integrates to 10⁴ HM units and draws T > 1000 HM black, which is T_ours above 6.0 to 368 depending on the point. So our
  censored class is not Fig. 1's black class.
- **The pinned Brutus may never apply the word length** (unverified, above). As recorded, the stock executable gives identical output for
  L_w = 20, 53, 128 and 1024 (`stock_lw_test.txt`). All our "L_w = 1024 / 128 / 88" runs use our own driver (`src/pilot.cpp`), which calls
  `mpreal::set_default_prec(L_w)` before any `mpreal` is built and links the unmodified Brutus objects. The L_w timings in
  `gg1/scaling_gg1_lw.csv` (5.8 → 8.1 s from 128 to 1024 bits, same steps) are indirect evidence that the precision takes effect.
- The stock source needs two build-flag deviations on current Ubuntu, no source edits (`INSTALL_RECORD.md`).
- **p_max is not stated in the GG-1 paper and decides the cost at tight ε** (M, L_w = 1024). At ε = 10⁻⁶ on two points: p_max = 2 is about 10×
  slower than 4 or worse (one point hit the 300 s cap); 8 and 16 cost about 0.5–0.6× of 4. At ε = 10⁻¹⁰ on point (1, 54) it is 147 s at
  p_max = 4 and 10 s at 8 or 16 (§ 2b). We used 4 for the 16-point sample, the value in the repo's sample inputs.
- A wall cap is checked between Bulirsch–Stoer steps, not inside one. One BR-1 run at ε = 10⁻² ran over 10 minutes inside a single step and was
  killed by hand. ε = 10⁻³ on GG-1 point (27, 24) took 59,040 steps by t = 0.25 and hit the 600 s cap. Cost is not monotonic in ε: very loose
  tolerances break the step control.

## 1. Grid

The 1/64 grid on the Agekyan–Anosova map has **exactly 1212 points** (i = 1–31, j = 1–54; bodies 1 and 2 at (∓½, 0), body 3 at (i/64, j/64),
0 < x < ½, y > 0, (x+½)² + y² < 1; tested in integers). It is identical, as a set, to route 2's grid (`gg1/make_grid_output.txt`). The grid
positions are in our units (r₁₂ = 1), as the map is defined; the conversion above applies to times and to the escape radius.

## 2. GG-1 at ε = 10⁻⁶, L_w = 1024: 16 points (M)

Sample: stratified on the paper's own Fig. 1 lifetimes (route 2) and spread over the map, including 3 points drawn black at ε = 10⁻⁶; 16 of
1212. Fig. 1 was used only to choose points. T: the first time the single is more than 10 **of our** length units from the binary centre of
mass, moving away, with positive energy; then retraced to the closest binary–single approach (Kepler retrace; the integrated-history reading
agrees to 0.1–1%, `gg1/pilot_fwdonly_final_driver_*.csv`). T is capped at 1000 of our units. "Convergence" is the paper's reversibility test:
reverse velocities at the escape time, integrate on to twice that time, pass if log₁₀Δ < −3. p_max = 4. The last three columns are in HM
units (`gg1/ours_vs_fig1.csv`).

| (i, j) | x, y | T (ours, our units) | escape detected at | steps | forward wall s | reverse leg: steps, wall s | log10 Δ (pass if < −3) | factor T_HM / T_ours | ours, log10 T_HM | Fig. 1 left (route 2), log10 T |
|---|---|---|---|---|---|---|---|---|---|---|
| 1, 1 | 0.0156, 0.0156 | 69.42 | 73.03 | 10882 | 139.0 | 9697, 63 | 1.57 | 5.74 | 2.60 | 1.27 |
| 1, 23 | 0.0156, 0.3594 | 15.33 | 17.56 | 1814 | 13.6 | 1737, 10 | 0.17 | 4.50 | 1.84 | 2.29 |
| 1, 54 | 0.0156, 0.8438 | 0.6329 | 2.383 | 877 | 6.2 | 870, 6 | -0.84 | 2.72 | 0.24 | 0.52 |
| 2, 34 | 0.0312, 0.5312 | 45.39 | 49.47 | 4038 | 31.0 | 4540, 41 | 1.14 | 3.72 | 2.23 | 1.93 |
| 3, 43 | 0.0469, 0.6719 | 0.5706 | 9.144 | 567 | 3.4 | 568, 3 | -1.44 | 3.20 | 0.26 | 2.71 |
| 4, 13 | 0.0625, 0.2031 | 6.996 | 10.17 | 1846 | 21.5 | 2128, 22 | 0.99 | 5.30 | 1.57 | 2.07 |
| 8, 37 | 0.1250, 0.5781 | 31.09 | 32.96 | 4056 | 63.1 | 81176, 1200 (hit 1200 s cap) | n/a | 3.54 | 2.04 | >3 (black) |
| 9, 3 | 0.1406, 0.0469 | 13.48 | 15.25 | 3018 | 38.2 | 6249, 67 | 1.56 | 6.29 | 1.93 | >3 (black) |
| 12, 30 | 0.1875, 0.4688 | 14.73 | 19.72 | 1462 | 12.3 | 1528, 7 | 1.04 | 4.07 | 1.78 | 2.61 |
| 14, 20 | 0.2188, 0.3125 | 1.339 | 3.617 | 1340 | 15.1 | 1317, 15 | 0.08 | 5.15 | 0.84 | 0.69 |
| 15, 6 | 0.2344, 0.0938 | 95.64 | 99.92 | 19837 | 204.8 | 16705, 96 | 1.98 | 7.36 | 2.85 | 1.67 |
| 17, 41 | 0.2656, 0.6406 | 19.77 | 23.81 | 1982 | 26.6 | 2628, 20 | 1.42 | 3.31 | 1.82 | 1.25 |
| 23, 29 | 0.3594, 0.4531 | 15.88 | 20.25 | 1683 | 15.2 | 1229, 7 | 0.22 | 4.32 | 1.84 | 2.12 |
| 27, 24 | 0.4219, 0.3750 | 4.135 | 6.476 | 1024 | 9.7 | 1031, 8 | 0.03 | 5.09 | 1.32 | >3 (black) |
| 28, 1 | 0.4375, 0.0156 | 1.555 | 4.272 | 5274 | 80.2 | 3913, 22 | 0.45 | 37.86 | 1.77 | 1.04 |
| 29, 17 | 0.4531, 0.2656 | 14.31 | 19.84 | 2587 | 17.2 | 2498, 14 | -0.09 | 7.02 | 2.00 | 1.58 |

- **All 16 escaped under our test; none was censored.** T runs from 0.57 to 95.6 of our units (1.7 to 704 HM). The three points the paper
  draws black at ε = 10⁻⁶ escaped here (T = 110, 85 and 21 HM), but our escape radius and cap are not the paper's (above), so this is not a
  like-for-like comparison of the black class.
- **Cost per point, forward leg:** wall median **19 s** (mean 44, p10 8, p90 110, range 3–205); steps median 1914 (567–19,837); 9.9 ms per step
  (6.0–15.6); **1.7 s per unit of our simulated time** (median; p10 0.6, p90 3.4, max 18.8 for the close-encounter point (28, 1)).
- **Reverse leg:** median 17 s. One of 16 (point (8, 37)) ran 81,176 steps and hit the 1200 s cap: the reverse leg has a heavy tail the forward
  leg does not.
- **Reversibility: 0 of 15 passed** (log₁₀Δ from −1.4 to +2.0). At ε = 10⁻⁶ nothing is converged, as the paper's Fig. 2 has it (f_irr near 1).
  Energy error at escape: median 3.9×10⁻³.
- **L_w** (M): point (1, 54) side by side with other jobs: 5.8 / 6.1 / 6.5 / 8.1 s at 128 / 256 / 512 / 1024 (×1.4). Steps are identical at
  every L_w, as they should be at ε = 10⁻⁶. (#334 also quoted a timing at point (10, 10), 128 against 1024 bits. It has no committed raw
  output, so it is dropped.)
- **ε** (M, 1024 bits, ms per step; point (1, 54) / (14, 20)): 10⁻³ 2.0 / 1.8; 10⁻⁴ 3.4 / 6.6; 10⁻⁵ 7.2 / 11.7; 10⁻⁶ 7.0–9.2 / 11.2 (the spread
  is load: the same point ran in two different mixes). It rises, then flattens near 10 ms.
- Determinism: a forward-only re-run with the final driver reproduced steps, T and status on 16 of 16.

### Ours against Fig. 1 in the paper's units (M; RQ-612)

In the 16-point sample, three points have the same lifetime in both of the paper's panels (ε = 10⁻⁶ and ε = 10⁻⁷⁰). That holds only within the
sample: over the whole grid, 242 of 1212 points share a LUT index in both panels. Ours does not move with ε at these three points over
10⁻⁴ … 10⁻⁶ (`gg1/robustness_three_points_Lw128_norev.csv`: (1, 54) 0.6329355 / 0.6329359 / 0.6329359; (3, 43) 0.57059 at all three;
(14, 20) 1.3376 / 1.3389 / 1.3393).

| (i, j) | ours, our units | factor | ours, HM | log₁₀ ours, HM | Fig. 1, both panels, log₁₀ T | ours − Fig. 1 |
|---|---|---|---|---|---|---|
| 1, 54 | 0.633 | 2.72 | 1.72 | 0.24 | 0.52 | −0.29 dex |
| 14, 20 | 1.339 | 5.15 | 6.90 | 0.84 | 0.69 | +0.15 dex |
| 3, 43 | 0.571 | 3.20 | 1.83 | 0.26 | 2.71 | −2.45 dex |

- In HM units, (1, 54) and (14, 20) come within 0.3 dex of Fig. 1. (3, 43) is still off by 2.4 dex.
- At (1, 54), next to the equilateral corner, our T is a close triple approach (minimum separation 0.01) and equals the equilateral free-fall
  collapse time (0.633 against 0.641 ours; 1.72 against 1.71 HM). Fig. 1's 3.3 HM is 1.9 times that collapse time, which points at the
  paper's definition of T rather than at our integration.
- **(3, 43) sits at the edge of a short-lifetime band.** In Fig. 1 left its neighbours (2, 42), (2, 43) and (2, 44) are log₁₀ T = 0.92, 0.90
  and 0.87, and (3, 44) is 1.11, while (3, 43) is 2.71 (route 2; `hm_units_output.txt` lists all eight neighbours). Ours puts (3, 43) in the
  short band, robustly over ε. A one-cell offset in the initial condition or in the grid snap would explain it. **The four neighbours of
  (3, 43), that is (2, 43), (4, 43), (3, 42) and (3, 44), must be run before any definition ruling is sought**; RQ-612 lists it as an option.
- Over all 16 points, ours in HM units minus Fig. 1 left runs from −2.45 to +1.33 dex (13 uncensored points). At the other points the two
  panels differ from each other, as the paper expects, so no point-by-point agreement is expected at ε = 10⁻⁶.
- Our escape radius and cap were in our units. Rerunning with R_esc = 10 HM units (0.48 to 7.4 of ours, by point) and the cap at 10⁴ HM units
  would match Appendix A as written. It is not done here: no new runs were made for this record.

### Projection: the full ε = 10⁻⁶ panel at L_w = 1024 (E; RQ-613)

The weights are the lifetimes in the paper's Fig. 1 left panel (route 2). That panel and ours have not been tested against each other, and the
section above shows they disagree point by point. (The K–S p = 0.72, `../README.md` § 2, is the paper's own test between its two panels; it
is not a test of ours.) The rates are measured in our time units, so the weights must be converted: each Fig. 1 lifetime, read as HM units,
is divided by its point's factor. #334 counted the 26 black points at 1000; the paper's runs go to 10⁴ HM units, so both are shown.

| Weights | Σ T, our units | forward leg, core-hours | with the reversal leg (+0.82×) |
|---|---|---|---|
| #334's (Fig. 1 read as our units, black at 1000) | 85,690 | 40–83 | 74–151 |
| HM-converted, black at 1000 HM | 18,974 | 9–18 | 16–33 |
| HM-converted, black at 10⁴ HM | 70,211 | 33–68 | 60–124 |

Low end: the median rate (1.7 s per unit) × Σ T; high end: the mean rate (2.7) × the median integrated-time / T ratio (1.31) × Σ T
(`hm_units.py`; `summary_output.txt` gives #334's six variants on its own weights). Weak points: our longest T is 96 of our units, so the long
tail is extrapolated from the rate; the reverse leg's heavy tail (1 of 16 capped) could add to it. These are p_max = 4 numbers; at p_max = 8
the 10⁻⁶ cost is about 0.5–0.6× of them (two points).

### 2b. Below ε = 10⁻⁶: one point at ε = 10⁻⁸ and 10⁻¹⁰ (M)

Point (1, 54), L_w = 1024, T capped at 1000 (our units), the paper's reversibility test, run side by side with one other job. Files:
`gg1/below_1e6_*.csv`, `gg1/pilot_*` (the ε = 10⁻⁶ row from the sample), `gg1/below_1e6_pmax_probe.csv`.

| p_max | ε | steps (forward) | forward wall | reverse wall | log₁₀Δ (pass if < −3) | T (our units) |
|---|---|---|---|---|---|---|
| 4 | 10⁻⁶ | 877 | 6.2–8.1 s | 6 s | −0.84 (fail) | 0.63294 |
| 4 | 10⁻⁸ | 1290 | 27 s | 27 s | −2.40 (fail) | 0.632936 |
| 4 | 10⁻¹⁰ | 3448 | 147 s | 146 s | −4.57 (**pass**) | 0.632936 |
| 8 | 10⁻⁸ | 917 | 6.8 s | 7.2 s | −4.18 (**pass**) | |
| 8 | 10⁻¹⁰ | 973 | 10.1 s | 9.3 s | −5.60 (**pass**) | |
| 16 | 10⁻⁸ | 916 | 6.6 s | 6.9 s | −4.63 (**pass**) | |
| 16 | 10⁻¹⁰ | 972 | 9.9 s | 9.9 s | −5.60 (**pass**) | |

- **At p_max = 4 the local slope steepens over 10⁻⁶ … 10⁻¹⁰.** Wall ×3.4–4.4 from 10⁻⁶ to 10⁻⁸ (the range is the 10⁻⁶ run's two loads), then
  ×5.4 to 10⁻¹⁰: d log₁₀(wall) / d log₁₀(1/ε) is 0.26–0.32, then 0.37. Steps ×1.5 then ×2.7; ms per step 7–9, 21, 43. Three ε values at one
  grid point show this much and no more. They do not show growth faster than any power of ε, and #334's claim that they do is withdrawn.
  Held at the last local slope (about ×4.6 per two decades), 10⁻⁷⁰ is of order 10²² s, so p_max = 4 is not usable there.
- **At p_max = 8 or 16 it is nearly flat over these two decades**: 917 → 973 steps, 6.8 → 10.1 s from 10⁻⁸ to 10⁻¹⁰, and the reversibility
  test already passes at 10⁻⁸ (−4.2, −4.6; at p_max = 4 it needs 10⁻¹⁰). So **p_max is also an accuracy setting**: the same ε converges at a
  different level.
- T (the lifetime) is 0.63294 at every ε and p_max here: this point is not chaotic enough to move it.

### 2c. Projection: ε = 10⁻⁷⁰ at L_w = 1024 (E; RQ-613): undetermined to orders of magnitude

Only p_max ≥ 8 is considered (for 4, see § 2b). Inputs, point (1, 54): ms per step at p_max 8 is 7.46 at 10⁻⁸ and 10.35 at 10⁻¹⁰ (p_max 16:
7.17 and 10.14); 973 steps over 2.38 units of our time at 10⁻¹⁰. Two models fitted to **the same two points** give results about 2,000 times
apart:

- **Linear in −log₁₀ε:** +1.44 ms per decade (p_max 8; 1.49 at 16), so about 97–99 ms per step at 10⁻⁷⁰.
- **Geometric:** ×1.386 per two decades (p_max 8; ×1.414 at 16), so 1.8×10⁴ to 3.3×10⁴ times the 10⁻¹⁰ per-step cost, 186–333 s per step.
- Bulirsch–Stoer with its extrapolation order capped at p_max should eventually cost a power of ε: once ε is far below what p_max points can
  reach at a given step, the step must shrink as a power of ε. That is the p_max = 4 behaviour of § 2b, and it applies at 8 and 16 too once 60
  decades are crossed. Neither model above includes it, so neither is a bound.
- **Steps.** #334 took steps × 11.5 from 10⁻¹⁰ to 10⁻⁷⁰ from Brutus's `get_eta`. That cannot be checked here: `Brutus.cpp` is not committed,
  and the step law `projections.py` records (log₁₀ η = −0.012 |log₁₀ ε| − 0.40) is stated only for ε > 10⁻⁵⁰, so the factor below 10⁻⁵⁰ is
  unverified. `hm_units_output.txt` gives the results with × 11.5 and with × 1.
- **Weights.** As in § 2: #334's Σ T × 1.31 (Fig. 1 read as our units, black at 1000), or the HM-converted Σ T with black at 1000 or 10⁴.

| p_max 8, forward leg, core-hours | #334's weights | HM, black at 1000 | HM, black at 10⁴ |
|---|---|---|---|
| linear, steps × 11.5 | 1.4×10⁴ | 3.1×10³ | 1.2×10⁴ |
| geometric, steps × 11.5 | 2.7×10⁷ | 6.0×10⁶ | 2.2×10⁷ |
| linear, steps × 1 | 1.2×10³ | 2.7×10² | 1.0×10³ |
| geometric, steps × 1 | 2.4×10⁶ | 5.2×10⁵ | 1.9×10⁶ |

p_max 16 is within a factor of 2 of these (`hm_units_output.txt`); the reversal leg adds about 0.82×. On #334's own weights and step factor,
the two models give **about 14,000 core-hours (linear) against about 3×10⁷ (geometric)**. #334's lower figure, 6,000, rested on a "+0.5 ms
per decade" slope that has no source; it is dropped.

- **The paper's "about 7 hours".** The paper's § 2 gives "the average wall-clock time of our simulations was about 7 hours". That averages over
  every ε of the iterative convergence runs, on hardware the paper does not give. It is not a per-10⁻⁷⁰ cost and does not check any figure
  here.
- To pin the model down, the next probe would be p_max 8 and 16 at, say, 10⁻²⁰ and 10⁻³⁰ on the same point. It needs the owner's go-ahead.
- Further weak points: one easy point (T = 0.63); the T-dependence of cost at 10⁻⁷⁰ is assumed to follow the 10⁻⁶ rate.

## 3. BR-1: the authors' own initial conditions, ε = 10⁻⁶, L_w = 128 (M)

No random draw. Twelve of the authors' runs from `pgbreen/NVM` @ `a35c378`, from `trainex/data` and `validex/data`: 9 full-length runs
(three from each tertile of minimum pair separation along their trajectory) and 3 of the truncated ones. Row 1 gives x₁ = (1, 0) and x₂; x₃
from the barycentre; equal masses at rest; outputs every 2⁻⁸. Our trajectory is compared with theirs row by row. Their ε = 10⁻¹¹ / 10⁻¹⁰
pairs were not run. No coordinate from their files, and none of our trajectories, is committed (`README.md`, "Not committed"); the comparison
is committed as differences only. The file counts (180 files, 136 full, 44 truncated) are the pilot's record and unverified.

| run | set | authors' rows | steps | wall s (to authors' last row) | wall s to t = 10 | first t where our position differs from theirs by > 1e-3 / 1e-2 / 0.1 | √(time-mean MSE), positions | time-mean MSE, positions |
|---|---|---|---|---|---|---|---|---|
| run9941 | validex | 2561 | 2679 | 1.42 | same | 5.39453 / – / – | 0.000423 | 1.79e-07 |
| run9968 | validex | 2561 | 2682 | 1.74 | same | 3.50391 / 7.46875 / 9.42188 | 0.0191 | 3.64e-04 |
| run9956 | validex | 2561 | 2661 | 1.53 | same | 3.31641 / 4.75 / 6.33594 | 0.104 | 0.0108 |
| run28 | trainex | 2561 | 2608 | 1.12 | same | 2.52734 / 6.72266 / 8.92969 | 0.0318 | 1.01e-03 |
| run14 | trainex | 2561 | 2653 | 1.60 | same | 3.8125 / 9.14453 / – | 0.000921 | 8.49e-07 |
| run53 | trainex | 2561 | 2626 | 1.18 | same | 3.03125 / 9.85547 / – | 0.00146 | 2.13e-06 |
| run86 | trainex | 2561 | 2694 | 2.06 | same | 0.75 / 4.23828 / 5.12109 | 0.276 | 0.0761 |
| run9946 | validex | 2561 | 2592 | 1.08 | same | 5.09766 / – / – | 0.00167 | 2.79e-06 |
| run99 | trainex | 2561 | 2563 | 1.45 | same | – / – / – | 7.46e-05 | 5.56e-09 |
| run9992 | validex | 33 | 33 | 0.03 | 6.67 | – / – / – | 9.14e-10 | 8.35e-19 |
| run9994 | validex | 403 | 408 | 0.23 | 1.72 | – / – / – | 5.66e-06 | 3.20e-11 |
| run9959 | validex | 2298 | 2445 | 2.03 | 2.44 | 2.75 / 4.83594 / 7.95312 | 0.0215 | 4.64e-04 |

- **Our build tracks theirs.** At t = 0 the difference is exactly 0 (the initial condition is identical). Over t ≤ 1 the largest position
  difference is below 3×10⁻⁵ in 11 of 12 runs (median 4×10⁻⁶; run86: 2.5×10⁻³), the size ε = 10⁻⁶ allows against their solution. It grows by
  roughly 3–4× per decade of ε, not uniformly (10⁻⁶, 10⁻⁵, 10⁻⁴: `compare_to_authors.csv`). Runs then separate after close encounters, as
  chaos requires: first difference above 10⁻³ at t = 0.75–5.4 (median 3.3) in 9 runs, and 3 runs stay within 10⁻³ for the whole of their rows.
- **The "< 0.1" count depends on how δ is read (RQ-616).** Breen et al., Appendix A, define δ² = (1/12) ΣΣ (Δq)² over positions and
  velocities, then say "δ is equivalent to the mean squared error", with the threshold "average phase distance < 0.1". On the √(mean MSE)
  reading, positions only (`compare.py`), **10 of 12** are below 0.1 (run9956 at 0.104 and run86 at 0.276 are not). On the "δ is the mean
  squared error" reading, **all 12** are (the largest is 0.076). Either way this is not the authors' convergence test, which compares two of
  their own (ε, L_w) runs with each other rather than a run of ours with theirs; and their files hold no velocities, so ours uses the 6
  position terms.
- **Cost to t = 10 (M):** full-length runs at ε = 10⁻⁶, L_w = 128: wall median **1.45 s** (1.1–2.1), about 2650 steps (the 2561-output floor),
  0.57 ms per step. All 12 run to t = 10: median 1.6 s, mean 2.0, max 6.7 (run9992; our minimum pair separation 9×10⁻⁸). Energy error is
  5×10⁻⁵ … 9×10⁻³ at t = 10.
- **ε dependence (M, 9 full runs, L_w = 128, median wall / ms per step):** 10⁻³ 0.61 s / 0.24; 10⁻⁴ 0.89 / 0.34; 10⁻⁵ 1.19 / 0.45; 10⁻⁶ 1.45 /
  0.57. Linear in −log₁₀ε, about +0.11 ms per step per decade. **L_w = 88 vs 128 at 10⁻⁶: 1.42 vs 1.45 s, the same** (both fit in two 64-bit
  limbs).

### Projection: a BR-1 ensemble at the authors' pairs (E)

Model: steps × ms per step, with ms per step extrapolated linearly in −log₁₀ε (4–5 decades beyond the fit, the weak step), steps 2650–3050,
and a tail factor 1.28 (mean / median of our 12 runs). Typical run: **(10⁻¹¹, 128 bits) 2.9–3.4 s; (10⁻¹⁰, 88 bits) 2.6–3.0 s.** The caution
of § 2c about a linear extrapolation applies here too, over a shorter span.

| N | one pair | both pairs (the authors' convergence test needs two) |
|---|---|---|
| 1,000 | 1.0–1.2 core-hours | 2.1–2.4 |
| 10,000 (the authors' size) | 10–12 core-hours | 21–24 |

The authors' paper says generating its data "required over 10 days of computer time". Our figure is 10–20× lower. Reasons it may be low: it
is the typical run; the 12-run sample cannot show the hard tail (their truncated files mark where convergence failed for them); their tuning
iterated over several (ε, L_w) pairs; and their hardware is unknown. Treat it as the cost of a typical run, not of the ensemble.

## Not done, by design

No run below ε = 10⁻⁶ except the one-point probes of § 2b (p_max 4, 8 and 16); no run in HM units, of (3, 43)'s neighbours, or at 10⁻²⁰ and
10⁻³⁰; no scale-up of GG-1 (1196 points remain) or BR-1; no AMUSE copy of Brutus. Nothing from `pgbreen/NVM` is committed.
