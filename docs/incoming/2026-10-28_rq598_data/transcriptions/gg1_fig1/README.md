# GG-1, route 2: Fig. 1 of Boekholt, Portegies Zwart & Valtonen (2020), read from its vector paths

A record, not authority (R-1, R-159). **These are the paper's figure, read by us.** Every value here is the authors'
result as their Fig. 1 draws it, recovered from the PDF's vector paths. Nothing in it was computed by us, and none of
it is a Brutus run of ours. Route 2 of `../../README.md` §4, task 2, under the human's ruling of 28 Oct 2026 quoted in
§3 there.

## Source

- T. C. N. Boekholt, S. F. Portegies Zwart & M. Valtonen, "Gargantuan chaotic gravitational three-body systems and
  their irreversibility to the Planck length", *MNRAS* 493(3), 3932–3937 (2020), doi:10.1093/mnras/staa452, Fig. 1.
- File read: the arXiv copy (arXiv:2002.04029v1) in the repo,
  `docs/incoming/2026-10-25_replication/papers/boekholt_pz_valtonen_2020_MNRAS_493_3932_arXiv2002.04029.pdf`,
  sha256 `2ded4252b31b10cc1a7f64112fe9946d41c4ee3dffbb19c6e9f604a712c0dbfc`, page index 2 (the paper's p. 3).
- The publisher copy `staa452.pdf` (sha256 `9577338175d8210122d672300c2b34ae9949ecd1c2be3ca17e374a3dc3ab3a0f`) is
  not in the repo, and has not been run through `build.py`. Its Fig. 1, as `../../README.md` §2 describes it, has the
  same structure as the arXiv copy's: 2469 filled circular paths about 2.9 pt across, 8 Bézier segments each, 62
  x-centres at a 4.61 pt pitch, 26 black on the left and 19 on the right, and `jet` fills. `build.py` reads its ticks
  and frames from the page rather than hard-coding them, so `python3 -I build.py staa452.pdf` would check it.

## Files

| File | What |
|---|---|
| `build.py` | Reads the PDF, reconciles both panels, and writes the two CSVs. It stops before writing anything if a check fails |
| `build_output.txt` | The output of `python3 -I build.py > build_output.txt`, run from this folder (pymupdf 2.0, matplotlib 3.11.2, numpy 2.4.6) |
| `gg1_fig1_left_eps1e-6.csv` | Left panel, ε = 10⁻⁶: 1212 rows |
| `gg1_fig1_right_eps1e-70.csv` | Right panel, ε = 10⁻⁷⁰: 1212 rows |

Columns:

| Column | Meaning |
|---|---|
| `grid_index` | 1–1212, in (i, j) order, x-major; the same point has the same index in both panels |
| `i`, `j` | Grid integers: x = i/64, y = j/64 |
| `x`, `y` | i/64 and j/64, exact |
| `lut_index` | The `jet` entry (0–255) the dot's fill matches. `29-31` where those three identical entries match. `black` for T > 1000 |
| `log10_T` | The midpoint of the LUT bin, or `censored >1000` |
| `log10_T_lo`, `log10_T_hi` | The bin, [lo, hi). For a black dot, lo = 3 (T > 1000) and hi is empty |
| `colour_residual_L1` | The L1 distance, in RGB on 0–1, from the fill to the matched entry (to (0, 0, 0) for black) |
| `draw_order` | The dot's index among the page's 2469 dot paths, in content-stream order |
| `covered_lut_index` | For a black dot, the LUT entry of the coloured dot drawn beneath it |

## Reconciliation: exactly 1212 per panel

| Panel | Paths | Black | Distinct grid points | Coloured dots per point | Black over a coloured dot |
|---|---|---|---|---|---|
| left, ε = 10⁻⁶ | 1238 | 26 | **1212** | exactly 1 at each | 26 |
| right, ε = 10⁻⁷⁰ | 1231 | 19 | **1212** | exactly 1 at each | 19 |

- **The repeats** are all one kind: a black dot drawn after, and on top of, a coloured dot at the same grid point. Its
  centre is within 0.0007 grid steps (about 0.003 pt) of the coloured one. Every coloured dot under a black one is
  LUT 255, the top of the scale, which is what clipping log₁₀ T > 3 to the colour bar's maximum gives. So the plot drew
  every point in colour, then redrew those with T > 1000 in black. 1238 = 1212 + 26 and 1231 = 1212 + 19.
- **The extra positions**: there are none. Snapped to the 1/64 grid, every dot is within 0.0007 grid steps of its
  grid point, and each panel occupies exactly 1212 points. The counts in `../../README.md` §2 (1215 and 1217 distinct
  positions) count distinct raw page coordinates. Rounding the arXiv copy's centres to 0.01 pt gives 1216 and 1217:
  the black-over-coloured pairs, a few thousandths of a point apart, sometimes fall either side of a rounding boundary.
  Rounding to 0.1 pt gives 1212 for each. The exact procedure behind 1215 is not recorded, so this is the likely
  cause, not a proven one.
- **The grid**: both panels hold the same 1212 points, i = 1…31 and j = 1…54. They are exactly the 1/64 grid points
  with 0 < x < 1/2, y > 0 and (x + 1/2)² + y² < 1, with no point on any boundary. That is an observation about the
  figure. The paper says only "a uniform grid on the map with a grid-spacing of 0.015625" giving 1212 points (§2,
  Appendix A).
- **The axes**: the ticks are 0.0–0.5 on x (six) and 0.0–0.8 on y (nine) in each panel. They are linear to 6 × 10⁻¹⁶
  (x) and 1.3 × 10⁻⁶ (y), in data units. One grid step is 4.6106 pt in x and 4.4826 pt in y.
- **The colours**: every non-black fill matches a `jet` entry to an L1 distance of 5.3 × 10⁻⁸. These are the exact
  table values, written at PDF precision, not approximations. LUT indices run from 23 (left) or 26 (right) to 255.

## Precision

- **log₁₀ T is known to one LUT step, 3/256 ≈ 0.0117 dex**, the width of the bin. The midpoint in `log10_T` is within
  ±0.0059 dex of the value the authors plotted. That is about ±1.4% in T.
- **Exception**: `jet` entries 29, 30 and 31 are the same colour. Three dots per panel read as that range (left:
  (i, j) = (23, 11), (25, 5), (26, 9); right: (23, 11), (26, 9), (31, 11)). Their bin is three steps, 0.0352 dex wide,
  log₁₀ T in [0.3398, 0.3750), with the midpoint ±0.0176 dex.
- **Black is exact as a class**: T > 1000, censored. No number is given for it.
- **LUT 255, not black** (one dot, left panel, the only one): log₁₀ T in [2.98828, 3], that is T between 973 and 1000.
  Whether T = 1000 exactly is drawn black or red is not stated.
- The bin edges assume matplotlib's mapping, Normalize(vmin = 0, vmax = 3), then bin k = floor(256 · log₁₀ T / 3).
  The colour bars run from 0.0 to 3.0, and the fills are `jet`'s own entries. If the authors' plotting code binned
  differently, the bins could move by up to one step. The reading's uncertainty would not change.
- The positions are exact grid points. The snap tolerance is not close to binding (0.0007 against the stop at 0.05).
