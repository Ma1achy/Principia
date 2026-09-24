# Drill-down — Colouring (Meaning rung)

*Fifth drill-down. This drill-down holds the spherical colour-map maths, consolidated with the render contract's slot rules, and pins the exact forms (colour-space matrices, compaction functions, the mixed-pixel colour-per-sample → SSAA resolve). Colour is data: every equation here is a measurement display, not decoration.*

---

## 1. What it is

The **Meaning rung**: `payload → colour, per pixel, at the playhead`. Fixed topology `post(combine(colour(ctx), brightness(ctx)))`; occupants are data (built-in / debug / custom); everything below the waist, governed by the render key — **no equation in this document can ever trigger a re-integration**.

---

## 2. Consolidated contract

From the **render contract**: the four slots and their signatures; **L-ownership** (a bound brightness metric owns L; the colour occupant contributes hue/chroma; monotone-L LUTs × Replace-L is a semantic conflict the combiner choice resolves, surfaced in UI); **averaging** (colour-per-sample → render-side SSAA resolve, so category-averaging is structurally impossible; continuous S² occupants may consume means of current shape vectors — the reason `n` is handled Cartesian); bake tier for pure-`f(n̂)` occupants (the GUI preview canvas *is* the uploaded texture — zero drift by construction); composite order baked base → combine → overlays → CVD → render→display scale (when `render_scale ≠ 1`) → canvas.

From the **chart contract (Part 2.5)**: colour **compaction** is the third compactification role — render-key, free, never re-integrates; per-field scale comes from the **ledger metadata** (lin | log | cyclic | diverging | categorical | flag).

From the **GUI design doc**: the **two-rotations rule** — `sph_uv` eats the config-space normal; view orbit is display-only; pattern auto-rotate is a UV offset inside the colour function.

From the **generation root**: unpack accessors and the debug catalogue are generated; the `detail` union field switches legend by `state` (the three-colours-bug guard).

---

## 3. The maths

### 3.1 Colour spaces (exact transforms — transcription-check against Ottosson before entering the shared source)

**sRGB transfer** (per channel): `c_lin = c/12.92` if `c ≤ 0.04045` else `((c+0.055)/1.055)^2.4`; inverse threshold `0.0031308`.

**Linear sRGB → OKLab**: `lms = M₁·rgb`, cube-root each, `Lab = M₂·lms^{1/3}`:

```
M₁ = [ 0.4122214708  0.5363325363  0.0514459929
       0.2119034982  0.6806995451  0.1073969566
       0.0883024619  0.2817188376  0.6299787005 ]

M₂ = [ 0.2104542553  0.7936177850 −0.0040720468
       1.9779984951 −2.4285922050  0.4505937099
       0.0259040371  0.7827717662 −0.8086757660 ]
```

**Inverse**: `lms' = M₂⁻¹·Lab` (closed coefficients: `l' = L + 0.3963377774a + 0.2158037573b`; `m' = L − 0.1055613458a − 0.0638541728b`; `s' = L − 0.0894841775a − 1.2914855480b`), cube, then

```
M₁⁻¹ = [ 4.0767416621 −3.3077115913  0.2309699292
        −1.2684380046  2.6097574011 −0.3413193965
        −0.0041960863 −0.7034186147  1.7076147010 ]
```

OKLCH is the polar form: `C = √(a²+b²)`, `h = atan2(b, a)`.

### 3.2 The vMF engine

The von Mises–Fisher density on $S^2$ is the spherical analogue of a Gaussian,
$f(\hat{\mathbf n}; \hat{\mathbf p}, \kappa) \propto \exp(\kappa\, \hat{\mathbf n}\cdot\hat{\mathbf p})$: largest at $\hat{\mathbf n} = \hat{\mathbf p}$, decaying with geodesic
distance, with spread set by the concentration $\kappa$. The colour at $\hat{\mathbf n}$ is a vMF-weighted mean of the pole
hues in OKLab:

$$a(\hat{\mathbf n}) = C\,\frac{\sum_{i=1}^{6} w_i\cos\theta_i}{\sum_{i=1}^{6} w_i}, \qquad
b(\hat{\mathbf n}) = C\,\frac{\sum_{i=1}^{6} w_i\sin\theta_i}{\sum_{i=1}^{6} w_i}, \qquad
w_i = \exp(\kappa\,\hat{\mathbf n}\cdot\hat{\mathbf p}_i).$$

| pole | direction | full OKLab | $\theta_i$ | Okabe–Ito $\theta_i$ |
|---|---|---|---|---|
| $\hat{\mathbf p}_1$ | $+\hat x$ | red | 0° | 250° (blue) |
| $\hat{\mathbf p}_2$ | $-\hat x$ | cyan | 180° | 70° (orange) |
| $\hat{\mathbf p}_3$ | $+\hat y$ | green | 120° | 30° (vermillion) |
| $\hat{\mathbf p}_4$ | $-\hat y$ | magenta | 300° | 210° (sky blue) |
| $\hat{\mathbf p}_5$ | $+\hat z$ | blue | 240° | 170° (bluish green) |
| $\hat{\mathbf p}_6$ | $-\hat z$ | yellow | 60° | 350° (reddish purple) |

The same, in the code form:

```
wᵢ = exp( κ · n̂ · p̂ᵢ )
a(n̂) = C · Σwᵢcosθᵢ / Σwᵢ        b(n̂) = C · Σwᵢsinθᵢ / Σwᵢ
```

Six poles at `{±x̂, ±ŷ, ±ẑ}`, opposing poles complementary. Hue tables: **Full OKLAB** (0° 180° 120° 300° 240° 60° for +x −x +y −y +z −z) and **Okabe–Ito CB-safe** (250° 70° 30° 210° 170° 350°). Properties (all tests, §5): seamless (no `atan2` anywhere in the blend), opposing poles cancel to neutral grey at midpoints, linear in OKLab, fully parameterised by `(L, C, κ)`.

**Seamless LUT sphere**: `N_e = 16` colours sampled uniformly from the LUT placed as equatorial poles `p̂ᵢ = (cos 2πi/N_e, sin 2πi/N_e, 0)`, LUT endpoints at the N/S poles; the identical blend evaluated in RGB. Cyclic LUTs (Twilight) close exactly by construction.

### 3.3 Sphere sampling conventions

Widget projection: `s_x = (p_x−c_x)/R`, `s_y = −(p_y−c_y)/R`, `s_z = √max(0, 1−s_x²−s_y²)`, `R = W/2−4`. Equirect mapping is **(φ, n_z)** ∈ [−π,π]×[−1,1] — `v` **linear in `n_z`**, not in latitude angle (a subtle mismatch source if assumed spherical-uniform). `sph_uv` input is always the **config-space** normal (two-rotations rule).

### 3.4 Physics overlay (blob blend) and the house encoding (stability × hue)

Special configurations are computed from the shape map with the current masses, not hard-coded (R-14, `principia_dd_integrator.md` §3.7). With equal masses: `b̂₀₁ = (−1, 0, 0)`, `b̂₁₂ = (½, √3/2, 0)`, `b̂₂₀ = (½, −√3/2, 0)`; Euler `êⱼ = −b̂ⱼ`; Lagrange `l̂± = (0,0,±1)`. With unequal masses, whether the overlay uses the mass-weighted positions or fixed 120° spacing is audit decision B18.

```
Blob blend:   c_out = c_base + Σⱼ wⱼ(cⱼ − c_base),
              wⱼ = s · 4 · max(0, exp(κⱼ(n̂·p̂ⱼ − 1)) + 0.01)      κ = 11 (BC), 9 (Euler/Lagrange)

Stability×Hue:  L = 0.25 + 0.55 · ½(1 − maxⱼ n̂·b̂ⱼ)
```

— hue identifies basin/position; darkness marks proximity to binary-collision directions (L = 0.25 at a BC point, 0.80 antipodal).

### 3.5 Combiners

**Replace-L (Principia default)**: base RGB → OKLab; `L ← L_min + (L_max − L_min)·b`; `(a, b_ab)` untouched; → RGB. Preserves hue and chroma exactly. **Multiply**: `rgb·b` in linear space — preserves the base's own L structure (the escape hatch for monotone-L LUTs).

### 3.6 Compaction (payload scalar → b ∈ [0,1]; forms per ledger `scale`)

```
lin        b = clamp( (x − lo)/(hi − lo) )
log        b = clamp( (ln x − ln lo)/(ln hi − ln lo) ),  x ≤ 0 → 0 with sentinel styling
cyclic     b = frac( x / period )                          (phase, ρ_angle)
diverging  b = ½ + ½·sign(x)·ln(1+|x|/x₀)/ln(1+x_max/x₀)   ★ symlog — PIN, confirm/veto
flag       b ∈ {0, 1}
```

★ The signed drifts (`energy_drift`, `Lz_drift`) need *some* symmetric compression around 0 with the ε floor as `x₀`; symlog is the pin this drill-down introduces.

### 3.7 Categorical colour, and how mixed pixels resolve (colour-per-sample → SSAA)

State → palette index (Okabe–Ito cycle ≤ 8, golden-angle beyond: `θᵢ = 2π·frac(i·φ_g)`, `φ_g = (√5−1)/2` — adjacent indices ≈ 137.5° apart). `detail` is a **union field**: legend and palette segment switch on `state` (escape → body id; collision → pair id).

**The mixed-pixel question is anti-aliasing, not semantics** (sampling/SSAA note, ratified). Samples ≠ pixels: at a fractal boundary several disagreeing samples fall under one display pixel. Resolution: **each sample is coloured independently through the full pipeline, then a render-side resolve pass averages the sample *colours* into the pixel colour** (`post(combine(colour, brightness))` runs per sample; only then are colours averaged). Consequences:

- **Category-averaging is structurally impossible** — classification → colour happens per-sample *before* any averaging, so a boundary pixel that's 75% escape-samples / 25% bounded-samples renders 75/25 blended sRGB (honest spatial AA of the footprint), never an invented "class 2" from a colormap position.
- **The ensemble copies ARE the SSAA samples** — the same E copies computed for the spread metric feed the colour resolve; AA is free wherever ensemble is enabled and scales with tier. Their offsets are a **fixed low-discrepancy Halton (2,3) prefix** (fixed because lockstep marches the scene → no Monte-Carlo accumulation; low-discrepancy for best coverage at low E).
- **Point vs footprint quantities** — a *point* quantity every copy computes (class, shape `n`, FTLE) **anti-aliases** (copies differ, colours blend at edges); a *footprint* quantity defined over the copies collectively (ensemble spread) is one value per nominal sample, **shared by its copies, so it does not sharp-edge AA** — correct, because there is nothing sub-footprint to resolve.
- **The resolve is render-side and terminal** — it averages colours only, never reads back as data; the spread/impurity statistics are computed on the *data* side from the copies' classified outcomes (two firewalls, sampling note). 

**Deleted (was a pin, now vetoed):** the `C' = C·(1 − H/ln k)` majority-class-plus-entropy-desaturation. It baked a display choice into a data occupant and solved a problem the SSAA resolve doesn't have. If a user *wants* an explicit uncertainty marker, that's an **optional independent slot binding** reading the exposed `ensemble_spread`/entropy field (a post occupant, the fuzziness overlay) — swappable and off-able, never welded into the class colour.

### 3.8 Palettes and CVD

**Cubehelix** (analytic, CB-tolerant by monotone L): `φ = 2π(s/3 − λt)`, `a = h·t(1−t)/2`, `s = 0.5, λ = 1.5, h = 1`; `R = t + a(−0.14861cosφ + 1.78277sinφ)`, `G = t + a(−0.29227cosφ − 0.90649sinφ)`, `B = t + a(1.97294cosφ)`.

**CVD simulation** — post-process, **linear sRGB**, after all pixel computation; pipeline order **pixel function → physics overlay → CVD → render→display scale → canvas write** (the scale stage is a no-op at native; render contract Part 4). $M_{\mathrm{cvd}}$ multiplies the linear $(R_\ell, G_\ell, B_\ell)$ triplet:

$$M_{\mathrm{deutan}} = \begin{pmatrix} 0.625 & 0.375 & 0 \\ 0.700 & 0.300 & 0 \\ 0 & 0.300 & 0.700 \end{pmatrix}, \qquad
M_{\mathrm{protan}} = \begin{pmatrix} 0.567 & 0.433 & 0 \\ 0.558 & 0.442 & 0 \\ 0 & 0.242 & 0.758 \end{pmatrix},$$

$$M_{\mathrm{tritan}} = \begin{pmatrix} 0.950 & 0.050 & 0 \\ 0 & 0.433 & 0.567 \\ 0 & 0.475 & 0.525 \end{pmatrix}, \qquad
M_{\mathrm{achrom}} = \begin{pmatrix} 0.299 & 0.587 & 0.114 \\ 0.299 & 0.587 & 0.114 \\ 0.299 & 0.587 & 0.114 \end{pmatrix}.$$

Under deuteranopia the full-OKLab map loses the red–green distinction (two poles collapse to near-identical
orange-brown). The Okabe–Ito scheme keeps all six poles because it avoids the red–green axis.

---

## 4. Seams (obligations → integration tests)

| Seam | Obligation | Test |
|---|---|---|
| **5** payload → colour | read fields only through generated accessors + ledger metadata | field views sane; a ledger scale change re-styles the view with **zero** recompute |
| **6** meets compute at the payload only | no colour equation references chart, integrator, or scheduler state | grep-level + the render-key isolation test (below) |
| **7** compositor above the slots | occupants emit layer content; blur/CVD-final are compositor business | per-layer CVD ≡ post-composite CVD for opaque quads (linearity check) |
| **render key** | nothing here re-integrates | change every slot, uniform, palette, playhead → sim-buffer hash unchanged; dispatch counter static across render-mode changes at a paused playhead |
| **13 / Observation** | debug views are ordinary occupants; catalogue generated | new ledger field → view appears (or generation fails); union-legend switching live |

---

## 5. Unit tests

1. **Transform round trips:** sRGB↔linear↔OKLab↔back within tolerance across a gamut lattice; anchors — white → `(L,a,b) = (1,0,0)`, primaries against Ottosson's published test values.
2. **Seam-free guarantee, automated:** for every mode declared continuous, sample dense pairs straddling the antimeridian and both poles → colour difference → 0. **Continuous:** all vMF modes (smooth weights, smooth weighted mean); the seamless LUT sphere (the same argument in RGB); latitude stripes, $\cos(f\arccos n_z)$; longitude stripes, $\sin(f\,\mathrm{atan2}(n_y, n_x))$, continuous at the antimeridian for integer $f$; 3-D Cartesian Perlin noise. **Intentionally discontinuous**, *excluded by name*, not by failure: Octant, Voronoi 6, Hemispheres, Icosahedral, Fibonacci (hard), Checkerboard, Truchet.
3. **vMF properties:** opposing-pole midpoints → `a = b = 0` (grey); `κ → large` → nearest-pole colour; **rotation equivariance** — `blend(Rn̂, R·poles) = blend(n̂, poles)`.
4. **LUT sphere:** an equator longitude sweep reproduces the 1-D LUT within blend tolerance; Twilight closes exactly at the wrap.
5. **Physics overlay:** blob maxima exactly at `b̂/ê/l̂`; strength `s = 0` is the identity; Stability×Hue L endpoints `0.25 / 0.80` exact.
6. **Combiner:** Replace-L leaves `(a, b_ab)` bit-stable; Multiply preserves channel ratios; the monotone-L-LUT × Replace-L pairing raises the UI conflict flag (a wiring test, not a colour test).
7. **Compaction:** each form monotone on its domain; log handles the −1.0 sentinel via styling, never via the ramp; symlog symmetric (`b(x) + b(−x) = 1`) with `b(0) = ½` exactly.
8. **Categorical discipline:** a synthetic mixed quad renders the **per-sample colour-then-SSAA-resolve blend** per §3.7 (e.g. 75% escape / 25% bounded → 75/25 blended sRGB) — **never an RGB average of class *indices*, and never the vetoed majority+desaturation**; the `detail` legend switches per `state` (the regression test the three-colours bug earns).
9. **Golden-angle adjacency:** consecutive palette indices exceed a minimum OKLab hue separation for n up to the Fibonacci-lattice counts.
10. **CVD stage:** achrom output has `R = G = B` exactly; applying any matrix pre-linearisation produces a detectable difference — asserting the *stage*, not just the matrix.
11. **Bake equivalence:** for every pure-`f(n̂)` occupant, texture-sampled vs directly-evaluated colour agree within texture quantisation over a sphere lattice — the preview-is-the-texture guarantee, executable.
12. **Render freedom at a paused playhead:** with the frame loop paused, cycling every render mode issues zero compute dispatches and leaves the sim-buffer hash unchanged (there is no scrub — the playhead is a live clock; temporal note).

---

## 6. Deferred / flagged

- **Symlog pin (§3.6)** — ratified (standard for signed wide-range data). **Entropy-desaturation (§3.7) — VETOED**: replaced by colour-per-sample SSAA resolve; uncertainty marking, if wanted, is an optional independent slot binding on the exposed spread/entropy field.
- **Equirect v-linear-in-`n_z`** — the convention is recorded because "obviously it's latitude" is the natural wrong assumption.
- **OKLab coefficients** — transcription-check against Ottosson's reference implementation before entering the shared source (same discipline as the Yoshida-6 w's).
- **Custom-occupant safety rails** — schema-driven uniforms, async compile, last-valid fallback: already fully specified in the render/lowering contracts; owned there, not re-stated here.

---

*No atan2 in a blend, no average of a category, no L that two owners claim. The sentinel is styled, never scaled. The preview is the texture. And nothing on this rung can ever cost an integration.*
