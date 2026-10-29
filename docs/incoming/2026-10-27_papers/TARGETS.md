# Replication targets from the 27 Oct 2026 papers

> **Note added 27 Oct 2026.** Every target below is folded, with its id kept, into the one target table of
> `../2026-10-27_validation_plan/PLAN.md` §3, proposed for one ruling with the other records' targets. Its §5 replaces
> "Five to do first" below as the first batch, and its §2.1 resolves README §5's "311 against 234" (LL-1). The
> transcriptions here stay the reference; nothing below is changed.

A record, not authority (R-1, R-159). Every target is read from the paper itself, and its page, table or figure is
given. Numbers are transcribed, not paraphrased. Where a value can only be read off a plot, it says so and gives the
reading's precision; where the paper does not state something, that is said instead of supplying a value. Rules 1–8 of
`../2026-10-25_replication/INSTRUCTION_replication_suite_2026-10-25.md` apply to every target: the source is in the
repo, tables are transcribed twice, tolerances are calibrations, nothing is tuned to match, coordinates are remapped by
a tested function, each comparison states the escape criterion, and chaotic results are compared as distributions.

**Charts named below** are those of `docs/design/principia_chart_reference.md`:

- `BodyPlane`: one body's position swept over a plane, the others held. The Burrau-centred slice and the Python
  cross-check's anchor.
- `Latent`: affine slices of the 8D latent chart, including slices with momentum axes.
- `ShapeSphere`: shape varied, masses and momenta held; at rest this is the free-fall problem.
- `Burrau { nu, K }`, and `MassSimplex`.
- The **Anosova physical-frame** entry (`principia_lowering_contract.md`, "region D bounds"), which brings
  Agekyan–Anosova coordinates into the canonical pipeline.
- **The Šuvakov velocity plane** is the `Latent` slice with equal masses, collinear positions (−1, 0), (1, 0), (0, 0),
  and the two free velocity components (ẋ₁, ẏ₁) (catalogue §2.3).

**Difficulty**:

- **E**: exact. A deterministic value matched to a tolerance.
- **S**: statistical. A distribution or summary compared as such.
- **Q**: qualitative. Morphology, the existence of a feature, or a sign.

"1990s double precision" means the authors integrated in 16-digit arithmetic. Those papers' long-lived runs are not
ground truth, and where they say so themselves it is noted.

**Counts:**

| Paper | Targets |
|---|---|
| Aarseth, Anosova, Orlov & Szebehely 1994a | 6 (P-1 to P-6) |
| Aarseth, Anosova, Orlov & Szebehely 1994b | 4 (Q-1 to Q-4) |
| Lehto et al. 2008 | 5 (L-1 to L-5) |
| Agekyan & Anosova I and II | 8 (AA-1 to AA-8) |
| Boekholt, Moerman & Portegies Zwart 2021 | 3 (B-1 to B-3) |
| Li & Liao 2019 | 4 (LL-1 to LL-4) |
| Hudomal 2015 | 5 (HU-1 to HU-5) |
| Heggie 1974 | 3 (H-1 to H-3) |
| Valtonen et al. 2016 | 11 (V-1 to V-11; V-6 to V-11 added with `valtonen2016/`) |
| **Total** | **49** |

---

## Aarseth, Anosova, Orlov & Szebehely 1994a, *CeMDA* 58, 1–16 ("Global chaoticity in the Pythagorean three-body problem")

File: `../2026-10-25_replication/papers/aarseth_anosova_orlov_szebehely_1994_CeMDA_58_1.pdf`. Catalogue §3.2.

**Common set-up** (§§1–3, pp. 2–4):

- **System.** Masses m₁ = 3, m₂ = 4, m₃ = 5, at rest at (1, 3), (−2, −1) and (1, −1), so r₁₂ = 5, r₂₃ = 3 and
  r₃₁ = 4 (Fig. 1). G = 1.
- **Displacements (eq. 1).** One body is displaced at a time; the other two stay at their Pythagorean positions:
  - xᵢ = x̄ᵢ + δᵢ cos αᵢ and yᵢ = ȳᵢ + δᵢ sin αᵢ;
  - δᵢ = 5×10⁻ᵏ with k = 8, 7, …, 3 (six circles);
  - αᵢ = 3.6n° with n = 0, 1, …, 99 (100 azimuths).
  - That makes 600 experiments per body and **1800** in all, numbered N = 1, 2, …, starting from α₁ = 0 at δ₁ = 5×10⁻⁸.
- **Integrator.**
  - Aarseth–Zare regularisation (Aarseth & Zare 1974; Aarseth 1976) with Bulirsch–Stoer, tolerance 1×10⁻¹³.
  - Repeated at 1×10⁻¹⁵ if the time-reversed run fails the accuracy test below.
  - Double precision ("16 figures").
- **Escape.**
  - Standish (1971) criterion for the relative velocity outside 3R_grav, where R_grav = 3.67 (G = 1). The paper
    quotes the standard crossing time as T_cr = 3.84.
  - **Conditional escape** (Anosova 1986) when a separation exceeds 5R_grav.
  - Upper time limit 200 time units; runs reaching it are "incomplete".
- **Reliability.**
  - Every run is time-reversed. It is **"predictable"** if the r.m.s. deviations between initial and final
    coordinates, and separately between velocities, are both < 0.001; otherwise **"non-predictable"** and excluded.
  - The authors attribute the non-predictable runs to numerical error, chaos or both, and say they cannot separate the
    two (p. 4).
- **Outcome.**
  - The escaper's identity, and its escape angle β to the x-axis.
  - The escape time T: the moment either escape criterion fires (p. 8).
  - Also the binary's semi-major axis A and eccentricity E, the minimum perimeter P at the final triple approach, the
    moment of inertia I then, the minimum two-body distance R, and the reversal errors ε (coordinates) and ν
    (velocities).

**Mapping onto Principia.**

- Each circle family is a **polar sampling of a `BodyPlane` centred on body i's Burrau position**: radius δ on a
  logarithmic ladder, azimuth α. The paper's figures draw the six circles as equally spaced rings, one per decade of
  δ, so a renderer reproducing Figs. 2–4 should plot ring index, not δ.
- At δ ≤ 5×10⁻³ every sample is within one pixel of the Burrau point on any full-chart view. This is a deep-zoom target.
- **Horizon.** Escapes run to T ≈ 45–120 time units, past the project's `near-field` horizon of 13. RQ-555 caps the
  horizon at 200, the paper's own cut.

**Caveats.**

- 1990s double precision, with an unregularised third pair under Aarseth–Zare. The authors' own predictability filter
  removes a large share: 831 of 1800 runs are predictable escapers (Table VIII), and the paper does not break down the
  other 969.
- The escape angle β depends on when and where the criterion fires. Compare it under their criterion.

### P-1. The regular ring around each body at δ = 5×10⁻⁸ (Figs. 2 and 3, pp. 5–8; Table II, p. 11)

- **Compare:**
  - At δ₁ = 5×10⁻⁸ the β values form "a continuous closed ring … where the angles β are close to 70 degrees" (p. 8).
  - For m₂ and m₃ "the central rings are incomplete; they have a 'horseshoe' form" (p. 8).
  - Table II's δ = 5×10⁻⁸ row gives ⟨β₁⟩ = 71.3 (σ 0.2), ⟨β₂⟩ = 70.2 (σ 0.3) and ⟨β₃⟩ = 66.8 (σ 0.9). The text names
    "β = 71.4 degrees, corresponding to the original Pythagorean triangle" (p. 4).
  - At 5×10⁻⁸, Table I counts 100, 83 and 74 escapers (N₁, N₂, N₃) among the 100 runs displacing m₁, m₂ and m₃.
    The voids are non-predictable runs. Table I does not split the counts by which body escaped; Table VIII pools
    them over δ.
- **Chart:** a 100-sample ring on `BodyPlane` around each body. The pass criterion is qualitative: a complete ring for
  m₁ and gaps for m₂ and m₃, with β continuous along the regular arcs.
- **Difficulty:** Q for the ring shapes; S for ⟨β⟩ against Table II.

### P-2. Escape time by displacement radius (Table I, p. 10, transcribed in full)

Escape times of triple systems. Nᵢ is the number of escapers among the 100 runs displacing body i; ⟨Tᵢ⟩ and σᵢ are in
time units (G = 1).

| δ | N₁ | ⟨T₁⟩ | σ₁ | N₂ | ⟨T₂⟩ | σ₂ | N₃ | ⟨T₃⟩ | σ₃ |
|---|---|---|---|---|---|---|---|---|---|
| 5×10⁻³ | 47 | 64.8 | 2.4 | 35 | 65.9 | 2.6 | 46 | 58.2 | 2.6 |
| 5×10⁻⁴ | 42 | 62.1 | 2.1 | 37 | 65.8 | 2.5 | 55 | 57.8 | 1.8 |
| 5×10⁻⁵ | 47 | 62.3 | 1.2 | 34 | 72.7 | 2.7 | 30 | 64.9 | 1.8 |
| 5×10⁻⁶ | 28 | 79.0 | 3.1 | 36 | 67.0 | 1.8 | 38 | 62.1 | 1.6 |
| 5×10⁻⁷ | 34 | 71.4 | 0.8 | 30 | 73.3 | 1.3 | 35 | 70.9 | 1.2 |
| 5×10⁻⁸ | 100 | 66.0 | 0.2 | 83 | 65.4 | 0.3 | 74 | 64.6 | 0.3 |

- **Note.** The paper calls σ an r.m.s. deviation (§5). Its size, for example 0.2 on N = 100 with escape times spread
  over 45–120, looks like a standard error of the mean. This is not resolved; compare against both readings.
- **Text (p. 4).** "the regular motions have escape times within the interval [45, 120] scaled time units, with a mean
  escape time of 65 (or 17 crossing times)"; the non-predictable runs have escape times of "typically 60–200 units or
  more".
- **Chart:** as P-1. The counts Nᵢ are themselves a target: the predictable fraction per ring, a direct analogue of the
  project's `error_ratio` flagging.
- **Difficulty:** S.

### P-3. Escape time against azimuth (Fig. 4, pp. 9–10)

- **Compare:** T(α) on each ring, in four classes: 40–50, 50–60, 60–70 and > 70 time units (p. 8). Voids are
  non-predictable runs. "The regions of regular behaviour of β coincide with similar regions for T" (p. 8). Only the
  classes are published.
- **Chart:** as P-1, coloured by T in their four bins.
- **Difficulty:** Q.

### P-4. Binary elements, perimeters, distances and reversal errors (Tables II–VII, pp. 11–12, transcribed in full)

Columns: Table II ⟨β⟩ (degrees) and σ; Table III ⟨A⟩ and σ; Table IV ⟨E⟩ and σ.

| δ | ⟨β₁⟩ | σ | ⟨β₂⟩ | σ | ⟨β₃⟩ | σ | ⟨A₁⟩ | σ | ⟨A₂⟩ | σ | ⟨A₃⟩ | σ | ⟨E₁⟩ | σ | ⟨E₂⟩ | σ | ⟨E₃⟩ | σ |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 5×10⁻³ | 172.4 | 14.1 | 196.4 | 14.4 | 188.0 | 13.9 | 0.523 | 0.025 | 0.566 | 0.027 | 0.442 | 0.029 | 0.788 | 0.028 | 0.825 | 0.035 | 0.740 | 0.028 |
| 5×10⁻⁴ | 235.9 | 12.3 | 200.8 | 15.3 | 130.6 | 11.9 | 0.435 | 0.022 | 0.439 | 0.024 | 0.616 | 0.024 | 0.754 | 0.034 | 0.739 | 0.039 | 0.715 | 0.023 |
| 5×10⁻⁵ | 205.8 | 17.9 | 182.3 | 17.2 | 201.1 | 17.1 | 0.480 | 0.024 | 0.505 | 0.025 | 0.490 | 0.033 | 0.815 | 0.034 | 0.761 | 0.044 | 0.725 | 0.049 |
| 5×10⁻⁶ | 96.3 | 16.4 | 105.3 | 11.3 | 189.5 | 15.0 | 0.506 | 0.027 | 0.414 | 0.038 | 0.573 | 0.033 | 0.918 | 0.029 | 0.931 | 0.016 | 0.899 | 0.025 |
| 5×10⁻⁷ | 143.3 | 16.8 | 130.2 | 18.9 | 134.8 | 18.3 | 0.530 | 0.023 | 0.530 | 0.030 | 0.510 | 0.027 | 0.784 | 0.048 | 0.864 | 0.039 | 0.884 | 0.033 |
| 5×10⁻⁸ | 71.3 | 0.2 | 70.2 | 0.3 | 66.8 | 0.9 | 0.545 | 0.015 | 0.483 | 0.020 | 0.414 | 0.025 | 0.990 | 0.000 | 0.991 | 0.000 | 0.952 | 0.011 |

Columns: Table V ⟨P⟩ and ⟨I⟩ (no σ printed); Table VI ⟨R⟩ as log₁₀, with σ; Table VII ⟨ε⟩ and ⟨ν⟩ as log₁₀.

| δ | ⟨P₁⟩ | ⟨P₂⟩ | ⟨P₃⟩ | ⟨I₁⟩ | ⟨I₂⟩ | ⟨I₃⟩ | ⟨R₁⟩ | σ | ⟨R₂⟩ | σ | ⟨R₃⟩ | σ | ⟨ε₁⟩ | ⟨ε₂⟩ | ⟨ε₃⟩ | ⟨ν₁⟩ | ⟨ν₂⟩ | ⟨ν₃⟩ |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 5×10⁻³ | 1.36 | 1.71 | 1.13 | 1.13 | 1.70 | 0.79 | −3.51 | 0.04 | −3.64 | 0.14 | −3.51 | 0.04 | −5.39 | −5.56 | −5.99 | −5.90 | −6.08 | −6.44 |
| 5×10⁻⁴ | 1.12 | 1.08 | 1.76 | 0.65 | 0.68 | 1.78 | −3.66 | 0.13 | −3.66 | 0.12 | −3.44 | 0.03 | −5.04 | −4.71 | −6.58 | −5.50 | −5.17 | −7.05 |
| 5×10⁻⁵ | 1.51 | 1.56 | 1.33 | 1.08 | 1.16 | 0.87 | −3.58 | 0.08 | −3.88 | 0.19 | −3.49 | 0.05 | −4.60 | −4.26 | −4.46 | −5.06 | −4.72 | −4.92 |
| 5×10⁻⁶ | 1.63 | 1.43 | 1.89 | 1.51 | 1.48 | 1.62 | −3.80 | 0.25 | −3.50 | 0.06 | −3.42 | 0.01 | −3.78 | −3.79 | −4.70 | −4.26 | −4.25 | −5.16 |
| 5×10⁻⁷ | 1.50 | 1.47 | 1.61 | 1.29 | 1.57 | 1.31 | −3.50 | 0.06 | −3.64 | 0.16 | −3.45 | 0.03 | −3.85 | −3.72 | −3.79 | −4.31 | −4.18 | −4.26 |
| 5×10⁻⁸ | 1.88 | 1.68 | 1.44 | 1.50 | 1.16 | 1.05 | −3.48 | 0.02 | −3.76 | 0.14 | −3.60 | 0.10 | −3.66 | −3.80 | −3.86 | −4.13 | −4.26 | −4.32 |

- **Definitions (p. 13).** P is the minimum perimeter at the final triple approach. I is the moment of inertia at that
  epoch, defined as the minimum of I with the perimeter below the virial size. R is the minimum two-body distance
  (osculating). ε and ν are the r.m.s. coordinate and velocity errors after time reversal.
- **Text (pp. 13–14).**
  - ⟨P⟩ ≈ 1.5 and ⟨A⟩ ≈ 0.5; ⟨E⟩ = 0.85, with about 60% of binaries at E ∈ [0.9, 1.0].
  - ⟨R⟩ = 10^−3.5, with an r.m.s. of 0.5 in the exponent; "almost in all cases" R < 10⁻³.
  - For δ > 5×10⁻⁸ the β distribution is close to uniform.
- **Chart:** as P-1. These are per-ring ensemble statistics of outcome fields the engine already produces (binary
  elements, `d_min`).
- **Difficulty:** S. Table VII is about *their* integrator, so it is not a target for ours, but it is the reliability
  floor their predictable runs sit on.

### P-5. Escaper statistics (Table VIII, p. 14, transcribed in full)

Escaper statistics. ⟨Tᵢ⟩ and σ_T are in units of the crossing time, T_cr = 3.84. By the paper's description, rows 1–3
are escapes of m₁ for displacements of m₁, m₂ and m₃, each combined over all δ; rows 4–6 are for m₂ and 7–9 for m₃.

| Escaper | N_esc | ⟨Tᵢ⟩ | σ_T | ⟨Pᵢ⟩ | σ_P | ⟨Aᵢ⟩ | σ_A | ⟨Eᵢ⟩ | σ_E |
|---|---|---|---|---|---|---|---|---|---|
| m₁ | 219 | 17.4 | 0.18 | 1.73 | 0.04 | 0.54 | 0.01 | 0.90 | 0.01 |
| m₁ | 190 | 17.5 | 0.19 | 1.66 | 0.05 | 0.52 | 0.01 | 0.91 | 0.01 |
| m₁ | 211 | 16.4 | 0.18 | 1.72 | 0.05 | 0.55 | 0.01 | 0.85 | 0.01 |
| m₂ | 49 | 17.7 | 0.50 | 1.22 | 0.06 | 0.46 | 0.01 | 0.75 | 0.04 |
| m₂ | 47 | 18.1 | 0.54 | 1.15 | 0.08 | 0.40 | 0.02 | 0.76 | 0.04 |
| m₂ | 33 | 17.2 | 0.64 | 1.10 | 0.08 | 0.39 | 0.02 | 0.78 | 0.04 |
| m₃ | 30 | 16.2 | 0.66 | 0.91 | 0.04 | 0.37 | 0.01 | 0.79 | 0.04 |
| m₃ | 18 | 17.8 | 0.99 | 0.96 | 0.08 | 0.33 | 0.02 | 0.86 | 0.03 |
| m₃ | 34 | 14.6 | 0.72 | 0.74 | 0.05 | 0.32 | 0.02 | 0.76 | 0.03 |

- **Escape shares.** The paper gives 75, 15 and 10% for m₁, m₂ and m₃, against 80, 16 and 4 for the general problem
  (Anosova & Orlov 1985; Anosova 1986).
- **This record's arithmetic, not the paper's.** The N_esc sum to 831: 620 for m₁, 129 for m₂, 82 for m₃, giving
  74.6, 15.5 and 9.9%. Grouped by displaced body, the sums are 298, 255 and 278. Each equals the corresponding column
  sum of Table I (Σ N₁ = 298, Σ N₂ = 255, Σ N₃ = 278), which confirms the row order. **831 is the predictable-escaper
  count that 1994b uses** (Q-3).
- **Chart:** an escaper-identity colouring on the P-1 rings, aggregated.
- **Difficulty:** S. The lightest-escapes-most share is robust. The individual numbers carry their integrator's
  predictability filter: compare our predictable fraction too, not only the shares.

### P-6. The fine scans of three regular regions (p. 8)

- **Set-up:** step 0.36°, three radial displacements δ = 3.0×10⁻⁷, 5.0×10⁻⁷ and 7.0×10⁻⁷, within sectors of central
  angle 36°. One region "includes the central point and elongates along the straight line connecting the bodies m₁ and
  m₂".
- **Compare:** "large oscillations of T at the boundaries of regular regions", attributed to conditional escape (very
  distant ejection without escape) at the boundaries.
- **Not published:** which body was displaced in each region, the sectors' azimuths, the number of runs, and any figure
  or table. Only the qualitative statement can be compared.
- **Chart:** a zoomed polar `BodyPlane` sector.
- **Difficulty:** Q.

---

## Aarseth, Anosova, Orlov & Szebehely 1994b, *CeMDA* 60, 131–137 ("Close triple approaches and escape in the three-body problem")

File: `../2026-10-25_replication/papers/aarseth_anosova_orlov_szebehely_1994b_CeMDA_60_131.pdf`. New catalogue entry
§3.5.

**Common set-up** (§2, pp. 132–133):

- **Three ensembles, all from zero velocity.**
  - **(i) Region D:** equal masses, the third body uniform at random in Agekyan–Anosova region D (bodies at (∓0.5, 0),
    the quadrant inside the unit circle centred on the first body). 2500 systems.
  - **(ii) Circle R:** equal masses, "all the positions are distributed uniformly within a circle R of unit radius".
    2500 systems.
  - **(iii) Problem 𝒫:** masses in ratio 3:4:5, positions "in the vicinity of the Pythagorean triangle (Aarseth et al.,
    1994)". These are the 831 predictable runs of 1994a.
- **Integration:** "the same numerical algorithm as before", i.e. 1994a's.
- **Escape.**
  - Standish (1971), or conditional escape when the third body is beyond 10d from the binary.
  - d is the virial radius, d = −G(M₁M₂ + M₁M₃ + M₂M₃)/(2E).
  - **This threshold differs from 1994a's 5R_grav.**
- **Triple approach.** The epoch of minimum moment of inertia, provided the perimeter there is < 2d. "Hence a small
  proportion of the examples may not be included."
- **Parameters.**
  - T: time of the last triple approach.
  - P: minimum perimeter. I: minimum moment of inertia.
  - Y: impact parameter of the escaper about the centre of mass.
  - Z = |r₁ − r₂|/(r₁ + r₂), where r₁ and r₂ are the escaper's distances to the binary components.
  - Ψ: angle between the escaper's velocity and the binary's line.
  - V_b: radial relative velocity of the binary components. V_e: the escaper's velocity relative to the binary.
  - A and E: the final binary's semi-major axis and eccentricity.
  - Z, Ψ, V_b and V_e are taken at the minimum of I.
- **Units.** d = 1, masses 1 (or 3, 4, 5). The crossing time is τ = G M_t^{1/2}(M₁M₂ + M₁M₃ + M₂M₃)/(−2E)^{3/2},
  with M_t the total mass, and the velocity unit is d/τ. T is in units of τ.

**Caveats.**

- 1990s double precision.
- The D and R samples include non-predictable runs; the 𝒫 sample excludes them (p. 136).
- Circle R is not a 2D slice: positions uniform in a disc are a 6-parameter draw, before quotienting.

### Q-1. Triple-approach statistics, region D and circle R (Table I, p. 134, transcribed in full)

Statistics of triple-approach parameters. Mean ± the paper's error figure.

| Parameter | Region D | Circle R |
|---|---|---|
| T | 72.6 ± 2.6 | 79.5 ± 1.4 |
| P | 0.655 ± 0.012 | 0.746 ± 0.010 |
| I | 0.469 ± 0.046 | 0.271 ± 0.006 |
| Y | 0.058 ± 0.003 | 0.050 ± 0.001 |
| Z | 0.332 ± 0.010 | 0.330 ± 0.007 |
| Ψ | 50.4° ± 0.9° | 51.9° ± 0.7° |
| V_b | 1.2 ± 0.2 | 1.2 ± 0.3 |
| V_e | 5.8 ± 0.3 | 6.1 ± 0.4 |
| A | 0.280 ± 0.003 | 0.277 ± 0.003 |
| E | 0.720 ± 0.009 | 0.715 ± 0.007 |

- **Chart:** region D is the Anosova physical-frame entry (at rest, equal masses); Circle R has no chart. Both are
  ensemble statistics over random draws, not maps.
- **Difficulty:** S. The T values depend on the conditional-escape distance 10d.

### Q-2. Parameter distributions (Table II, p. 134, transcribed in full)

Distributions of triple-approach parameters.

- For each parameter, the first row is region D and the second is circle R.
- The ten bins split the range into tenths; the 11th column is the share above the range.
- Percentages are "truncated" (p. 135). The last column is the median, with T in units of τ.

| Param. | Range | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | > | Median |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| T (D) | 0–100 | 23 | 12 | 10 | 6 | 6 | 4 | 4 | 3 | 2 | 2 | 21 | 40 |
| T (R) | | 17 | 10 | 9 | 8 | 6 | 5 | 4 | 4 | 3 | 3 | 28 | 50 |
| P (D) | 0–1.5 | 4 | 12 | 18 | 17 | 15 | 12 | 8 | 5 | 1 | 1 | 1 | 0.6 |
| P (R) | | 3 | 8 | 11 | 15 | 17 | 14 | 10 | 8 | 4 | 3 | 4 | 0.7 |
| I (D) | 0–1.0 | 27 | 20 | 15 | 11 | 7 | 3 | 1 | 2 | 1 | 2 | 4 | 0.22 |
| I (R) | | 27 | 23 | 16 | 10 | 7 | 3 | 3 | 2 | 2 | 1 | 2 | 0.20 |
| Y (D) | 0–0.1 | 23 | 12 | 10 | 8 | 6 | 6 | 4 | 3 | 4 | 2 | 17 | 0.04 |
| Y (R) | | 22 | 12 | 11 | 8 | 7 | 7 | 4 | 3 | 3 | 3 | 15 | 0.04 |
| Z (D) | 0–1.0 | 24 | 19 | 13 | 9 | 7 | 6 | 6 | 4 | 4 | 4 | 0 | 0.25 |
| Z (R) | | 22 | 19 | 13 | 9 | 7 | 6 | 4 | 4 | 3 | 5 | 0 | 0.27 |
| Ψ (D) | 0–90° | 5 | 6 | 7 | 9 | 11 | 11 | 12 | 11 | 11 | 12 | 0 | 54° |
| Ψ (R) | | 5 | 6 | 7 | 8 | 9 | 11 | 12 | 12 | 12 | 13 | 0 | 56° |
| V_b (D) | −10–10 | 2 | 0 | 2 | 7 | 15 | 35 | 23 | 6 | 1 | 1 | 2 | 1.4 |
| V_b (R) | | 1 | 0 | 2 | 8 | 14 | 35 | 25 | 6 | 2 | 0 | 2 | 1.4 |
| V_e (D) | 0–10 | 0 | 1 | 15 | 29 | 19 | 10 | 6 | 3 | 2 | 2 | 9 | 4.2 |
| V_e (R) | | 0 | 0 | 14 | 29 | 20 | 12 | 6 | 3 | 1 | 1 | 8 | 4.3 |
| A (D) | 0–0.5 | 1 | 3 | 5 | 8 | 11 | 16 | 26 | 25 | 0 | 0 | 0 | 0.3 |
| A (R) | | 0 | 3 | 6 | 8 | 13 | 16 | 23 | 25 | 0 | 0 | 0 | 0.3 |
| E (D) | 0–1.0 | 1 | 3 | 4 | 5 | 5 | 7 | 9 | 10 | 13 | 37 | 0 | 0.8 |
| E (R) | | 1 | 4 | 4 | 6 | 6 | 8 | 8 | 10 | 13 | 36 | 0 | 0.8 |
| Theory (E) | | 1 | 2 | 3 | 4 | 5 | 7 | 9 | 11 | 16 | 44 | 0 | 0.87 |

- **The "Theory" row** is Monaghan's (1976) planar eccentricity distribution, f(E) = E/(1 − E²)^{1/2} (p. 136).
  **It is exact and needs no integration.** Its CDF is 1 − (1 − E²)^{1/2}. Integrated over the ten bins it gives
  0.5, 1.5, 2.6, 3.7, 5.0, 6.6, 8.6, 11.4, 16.4 and 43.6%, and median √0.75 = 0.866. Rounded half-up, that is the
  printed row exactly. This is this record's arithmetic, and it checks the transcription.
- **Text (p. 135).** About ¾ of binaries are receding at the closest approach. About a quarter of Y values are below
  0.01. The A distribution has a sharp cut-off at A ≈ 0.4, "connected with an assumed value of 10d for the conditional
  escape distance".
- **Difficulty:** S. The A cut-off is criterion-made, so compare it only under the 10d rule.

### Q-3. Pythagorean vicinity against equal masses (Table III, p. 136, transcribed in full)

Comparison of statistics for different problems. "Problem 𝒫" is the 831 predictable 1994a runs, "re-scaled to the same
units as used for equal masses".

| Parameter | Regions D + R | Problem 𝒫 |
|---|---|---|
| T | 76.0 ± 1.7 | 52.4 ± 0.8 |
| P | 0.700 ± 0.008 | 0.805 ± 0.018 |
| Y | 0.054 ± 0.001 | 0.051 ± 0.007 |
| Z | 0.331 ± 0.006 | 0.213 ± 0.028 |
| Ψ | 51.2° ± 0.6° | 60.4° ± 2.8° |
| V_b | 1.2 ± 0.2 | 2.0 ± 0.4 |
| V_e | 6.0 ± 0.2 | 4.9 ± 0.5 |
| A | 0.278 ± 0.002 | 0.272 ± 0.008 |
| E | 0.718 ± 0.006 | 0.858 ± 0.018 |

- **Text (p. 136).** For 𝒫, ⟨Y⟩ ≈ 0.05, ⟨Z⟩ ≈ 0.2, V_b > 0 in ≈ 90% of cases, ⟨V_e⟩ ≈ 5 and ⟨Ψ⟩ ≈ 60°.
- **Units caveat (this record's arithmetic).** With τ as defined above, the Burrau masses (M_t = 12, ΣMᵢMⱼ = 47,
  E = −12.8167) give τ ≈ 1.255. 1994a's ≈ 65 time units is then ≈ 51.8τ, close to 52.4. So 1994b's T is in τ units,
  while 1994a's "17 crossing times" uses T_cr = 3.84, a different crossing time. State which is in use.
- **Chart:** the P-1 rings (`BodyPlane` around each body), with the triple-approach parameters measured at min I.
- **Difficulty:** S.

### Q-4. The random-geometry control for Z (p. 135): no integration

- **Set-up:** "A separate study of such deviations based on a total of 2500 random samples inside a circle", with Z
  determined "where the maximum distance represents the binary components as in the dynamical case". In other words:
  draw three points uniformly in the unit disc, call the two farthest apart the binary, and compute Z for the third.
- **Compare:** the bins of Z over [0, 1] in tenths are **17, 16, 14, 14, 12, 10, 8, 5, 4, 1** (%).
- **Chart:** none; a pure geometric Monte Carlo. It is the null against which the dynamical Z of Q-2 is "more
  isosceles".
- **Difficulty:** S. It is cheap, needs no integrator, and its only ambiguities are in the wording. One is whether
  "maximum distance represents the binary" means the two bodies at the largest separation; read literally, that makes
  the third body the one *off* the longest side. Pin the reading before claiming agreement.

---

## Lehto, Kotiranta, Valtonen, Heinämäki, Mikkola & Chernin 2008, *MNRAS* 388, 965–970

File: `../2026-10-25_replication/papers/lehto_etal_2008_MNRAS_388_965_arXiv0805.2844.pdf`, the arXiv v1 preprint.
**Page numbers below are the preprint's (1–8), not MNRAS's.** Catalogue §1.2.

**Common set-up** (§2, pp. 1–2):

- **System.** Equal masses, at rest, zero angular momentum, G = 1. Bodies at (0, 0) and (1, 0), the third at (x, y)
  with "abscissa from 0.0 to 1.0 and … ordinate from −√3/2 to √3/2".
  - The "shield shaped area" is not defined by an equation in the text. That it is the lens |P| ≤ 1, |P − (1, 0)| ≤ 1
    (the side from (0, 0) to (1, 0) being the longest) is **inferred** from the stated ranges and from the time unit's
    "R is the longest separation of the initial triangle".
- **Time unit.** t = (1/2π)(R/1 AU)^{3/2}(M/M_☉)^{−1/2} yr, i.e. the G = 1 unit with R = 1 and M = 1. The colour
  scales call it "crossing times".
- **Integrator.** Chain-regularised Bulirsch–Stoer (Mikkola & Aarseth 1990, 1993, 1996). The reversal analysis
  prefers logH leapfrog (Mikkola & Tanikawa 1999) for its constant step (p. 5).
- **Breakup time.** The escaper has non-negative energy and is at r ≥ 1 from both other bodies (p. 2).
- **Escape** (p. 6).
  - A semi-major-axis ratio, escaper's orbit to binary's, ≥ 100, together with a hyperbolic, receding escaper;
  - or a ratio ≥ 2000 alone.
  - Following Anosova (1986). "Smaller values … with no apparent effect".

**Mapping.** This is the Agekyan–Anosova region-D chart in a different frame:

- The longest side runs (0, 0)–(1, 0), not (−½, 0)–(½, 0).
- The lens covers region D four times, by the two reflections.
- A tested remap into the Anosova entry, or into `ShapeSphere` at rest, is required (rule 6).

### L-1. The lifetime map (Fig. 1, p. 2)

- **Set-up:**
  - Pixel 1.25×10⁻³, about 200,000 systems over the lens.
  - Colour: red if the system decays within 1 crossing time, orange within 2, "etc."; dark blue over 13.
- **Compare** (morphology):
  - The vertical red 1:1 band on the symmetry line x = ½.
  - Bands of odd resonances 1:3, 1:5 and 1:7 thickening towards the left and right corners; no 1:2 or 1:4 bands.
  - Long orange arcs from second-encounter breakups.
  - A "sea of apparent tranquillity" around (0.7, 0.5) and its mirror points.
  - Deep trenches at about (0.70, 0.64) and (0.65, 0.15).
  - Long-lived configurations around the upper branch of the 1:7 resonance at (0.90, 0.23) and below the "yellow eye"
    at (0.75, 0.60) (pp. 3–4).
  - "The image of the complete shield is symmetric" (Fig. 1 caption).
- **Not published:** the intermediate colour bins between 2 and 13 ("etc.").
- **Chart:** region D or `ShapeSphere` at rest, coloured by `t_end`, binned at integer crossing times and capped at 13.
- **Difficulty:** Q.

### L-2. The zooms (Figs. 2 and 3, pp. 3–4)

- **Fig. 2: pixel 2.5×10⁻⁴.**
  - The window is not stated in the text. Read from its axis labels: x from 0.70 to 0.80, y from 0.45 to about 0.62,
    with the lens edge cutting the upper right. The lower-left corner (0.70, 0.45) is fixed by Fig. 3.
  - **Compare:** the "large yellow area breaks into three major parts", one per escaping body, meeting at a triple
    collision point. Its boundary is "sharp from inside".
- **Fig. 3: pixel 3.125×10⁻⁶.**
  - "A 5 by 5 pixel area in the lower left corner of Figure 2". Its axis labels (x 0.7005, 0.701; y 0.4505, 0.451) put
    it at x ∈ [0.700, 0.70125], y ∈ [0.450, 0.45125], i.e. 400 × 400 pixels.
  - Colour linear to 20 crossing times. A light-green band in the upper right decays in about 7.
- **Chart:** as L-1, zoomed. Fig. 3 is a deep-zoom test at a chart coordinate of order 1.
- **Difficulty:** Q.

### L-3. Local homogeneity and the fractal dimension (§2.3, p. 4)

- **Set-up:**
  - For each point, compare its ejection time t_o with the median t_med of 8 neighbours on a square stencil offset by
    ε = ±1.25×10⁻³ in x and y (total width 3.75×10⁻³).
  - The point is **locally homogeneous** if |t_med − t_o| ≤ 0.5.
- **Compare:**
  - Of **21,699** points, **6,212** (p_ε = 28.6%) are homogeneous and 71.4% are not.
  - At bin sizes 1, 2, 3, 4, 5 and 10 times the original, p_ε = **28.6, 16.8, 14.9, 13.8, 13.5 and 8.6%**.
  - "To within measurement errors p_ε ∝ ε^−0.5", and the same exponent holds in the zoomed areas. Hence D_H = 1.5.
- **Chart:** as L-1. The statistic is computable from any `t_end` raster.
- **Difficulty:** S. The resolution at which the 21,699 points sit is not stated; the stencil defines it.

### L-4. Reversibility (§3 and Figs. 4–9, pp. 5–7)

- **Set-up:**
  - Integrate forward to escape, reverse the velocities, integrate back to the free-fall moment, then continue to
    breakup.
  - "Recovered" means the initial state is regained to δ = ±5.0×10⁻⁵ (five significant digits; p. 6).
  - ξ is the time ratio of the recovered part to the disruption time t_tot.
- **Compare:**
  - Fig. 4: the recovered fraction against δ, with a floor near δ = 10⁻⁵, the precision of their result files.
  - Fig. 7: the ξ map. Pink is ξ = 1. Red is 0.45–0.60, yellow 0.6–0.9, blue 0.9–1.0. White means either ξ < 0.45 or
    not recovered.
  - Fig. 8: ξ ≤ 0.45, with no grouping.
  - Fig. 9: ξ against t_tot. Population I has lower edge ξ₁ = a₁t^−n₁ + b₁ with a₁ = 0.5, n₁ = 0.7, b₁ = 0.47.
    Population II has a₂ = 2.5, n₂ = 0.9, b₂ = 0.
  - Exactly time-symmetric orbits (ξ = 1) cluster in the resonance bands.
- **Chart:** as L-1. Needs the reversible occupant (logH's TTL mode), as the replication instruction's tier C says.
- **Difficulty:** S/Q. ξ is a property of *their* double-precision integration. A converged integrator would give ξ = 1
  wherever it recovers, so compare the ξ map to an f64 run only.

### L-5. The step-halving and unregularised controls (§4, p. 7)

- **Compare:**
  - Halving the step "noticed only a slight sharpening in the boundaries".
  - A "naïve simulation with non-regularized equations of motion" with short variable steps reproduced "the main
    resonant structures" and "similar fractal-like patterns" on small scales.
- **Chart:** as L-1. This is a direct analogue of running the map under the `none` regularisation occupant.
- **Difficulty:** Q.

---

## Agekyan & Anosova I (*Sov. Astron.* 11, 1006, 1968) and II (*Astrophysics* 4, 11, 1968 volume)

Files: `../2026-10-24_anosova/papers/`. Transcriptions: this folder's CSVs. Catalogue §1.1 and §2.5. Region D and the
near-rest starts are already ported (R-592, `dd_validation_orbits.md` §1.6, `fixtures/anosova/`). These targets add the
**outcomes**.

**Common set-up.**

- **System.** Equal masses 1 at A₁ = (−½, 0) and A₂ = (+½, 0), with A₃ = (ξ, η) in region D: ξ ≥ 0, η ≥ 0,
  (ξ + ½)² + η² ≤ 1 (I eq. 8).
- **Units.** G = 6.67×10⁻⁸ in their units.
- **Starting velocities.**
  - **Paper I:** near rest, ẋ₁ = ẋ₂ = +5×10⁻⁵, ẋ₃ = −10⁻⁴, all ẏ = 0 (eq. 3). Kinetic energy L₀ = 7.5×10⁻⁹.
  - **Paper II:** exact rest ("the initial velocities of the components were taken to be zero", p. 11).
- **Time unit.** τ, the mean time for a component to cross the system, **defined per initial condition** by I eq. (7):
  τ = G^{−1/2}[(2/3)(1 + 1/r₁₃ + 1/r₂₃ − L₀/G)]^{−3/2}, with r₁₃ = √((ξ + ½)² + η²) and r₂₃ = √((ξ − ½)² + η²).
  Every T below is in that row's own τ. A comparison must convert each pixel's `t_end` by that pixel's τ.
- **Integrator.**
  - **Paper I:** Runge–Kutta with an automatic step, 0.1 < |δx/ε_p| < 1. ε_p = 0.25×10⁻⁶ for most runs and
    0.5×10⁻⁷ for the rest. Energy, angular momentum and centre of mass checked at each step, twice (p. 1008).
  - **Paper II:** the same, but linearised (Duboshin 1964) whenever any separation < 0.01 (p. 11).
  - Unregularised. 1960s arithmetic.
- **Decay.** Tevzadze's criterion (I eq. 10): "exact" decay. Otherwise a **nominal** (I) or **conditional** (II)
  decay when a component recedes to 10 units without the criterion holding. T is the epoch of the closest approach
  after which the criterion holds or the 10-unit recession occurs (I p. 1009).

**Caveats.**

- Rule 5 applies: an old, unregularised integrator. In paper I, 21 runs were rejected for |ΔE/E| > 5% and 17 stopped
  at r < 3×10⁻⁷ (p. 1009).
- Agreement is expected only for short-lived rows. A long-lived disagreement is information about their integration.

### AA-1. Region D and its starts: already ported

- R-592 ports region D, eq. (3) and Table 1's ξ and η as the fixture. Listed here so the targets are complete.
- **Difficulty:** E (done).

### AA-2. Paper I, decay time per near-rest start (Table 1 cols. 4–8, pp. 1010–1012; `agekyan_anosova_I_table1_cols5to8.csv`)

- **Compare:** T (in τ) for the **59** rows that print one.
  - Also p, the perimeter of the triple approach leading to decay (in units of the mean size a, I eq. 5), and p_min
    where an earlier approach was closer.
  - The outcome class is col. 4 (`fixtures/anosova/table1.csv`): disintegration, nominal disintegration, close
    approach (stopped), or ΔE/E > 0.05 (rejected).
  - Shortest decays: N = 6, 17, 26, 37, 44 and 72 at T = 1.9; N = 29 and 98 at 2.0; N = 8 at 2.1.
- **Chart:** the Anosova physical-frame entry, with the eq. (3) velocities; the import already exists. Per row: convert
  T to time by eq. (7) with L₀ = 7.5×10⁻⁹ and G = 6.67×10⁻⁸ in their units, or rescale to G = 1.
- **Difficulty:** E for the short-lived rows, S for the rest.

### AA-3. Paper I, the decay-time statistics (pp. 1012–1014)

- **Compare:**
  - Mean T̄ = 61.6 (eq. 12), σ = 74.4 (eq. 13), T̄ = 62 ± 10 (eq. 14), "from the 59 values".
  - **Recorded discrepancy:** the 59 tabulated values give mean 64.63 and sd 74.52 (README §5).
  - Fig. 3: the normalised distribution of T. Fig. 4: the 62 completed points in D, each labelled with its T.
  - "Sharp changes in the value of T for comparatively small changes in ξ and η", and their conjecture of an
    unbounded number of regions where T is continuous (p. 1014).
- **Chart:** the Anosova entry. Fig. 4 is a 62-point "map" to overlay on a `t_end` render.
- **Difficulty:** S. The mean depends on which rows complete, so compare over the same 59.

### AA-4. Paper I, the N = 3 trajectory (Figs. 2a–2h, p. 1013)

- **Set-up:** ξ = 0.269722548, η = 0.476622422, with eq. (3)'s velocities.
- **Compare:** eight panels of the three trajectories with time marks in τ, from 0 to about 28. Table 1 gives T = 26.0
  and p = 0.1145 for this row.
- **Chart:** a single-trajectory view, not a map.
- **Difficulty:** Q. The figure is a hand-drawn reproduction of 1960s output.

### AA-5. Paper I, run accounting (pp. 1006, 1008–1009)

- **Compare:**
  - The abstract: 62 completed, 59 used for the mean.
  - p. 1008: "Of the 100 computations performed, 64 have been extended to … disintegration". Table 1 has 62 rows
    printing E, so 62 is consistent with the table.
  - 21 rejected for ΔE/E > 5%; 17 stopped at r < 3×10⁻⁷.
  - Of the 62: ΔE/E < 0.03 in 56 and < 0.01 in 43; Δ′E > ΔE in 33 of 48; decay after a very close triple approach in
    49; λ ≈ 0.3 in p < λa (eq. 11).
- **Difficulty:** S. This is a check of our classifier's rates against theirs, not of physics.

### AA-6. Paper II, decay time per exact-rest start (Table 1, pp. 12–14; `agekyan_anosova_II_table1_exact_rest.csv`)

- **The single most useful region-D target in this batch.** All 100 configurations at **exact rest**, which is
  Principia's free-fall surface exactly, each with:
  - T (in τ, eq. 7 with L₀ = 0) and a conditional-decay flag;
  - p, with a flag that a closer approach came first;
  - −E·10⁻⁷, ΔE/−E and Δ′E/−E;
  - n₁ and n₂, the number of triple approaches with p ≤ 1.0a and ≤ 0.6a before decay;
  - k, the number of binary approaches with r ≤ 0.2a;
  - s, the number of switches to the linearised equations.
- **Compare:**
  - T per row. Shortest: N = 2 and 18 at T = 1.2; N = 51 and 74 at 1.9; N = 7, 85 and 100 at 2.0.
  - The ensemble: mean 110.1τ, r.m.s. 125.0τ, T̄ = (110.1 ± 12.5)τ (II eq. 2).
  - Table 2 (p. 15), counts and mean Δ′E/−E by p bin: 0 < p ≤ 0.1, 15 and 2.45; 0.1 < p ≤ 0.2, 35 and 0.412;
    0.2 < p, 50 and 0.205.
  - "All 100 decayed after a triple approach". 81 decayed after the closest triple approach; 19 had a closer one
    earlier (p. 16).
- **Chart:** the Anosova entry, or `ShapeSphere` at rest, with velocities exactly zero. **Here the remap is the whole
  job: G = 6.67×10⁻⁸ against G = 1, and τ per row.** No integration choice enters the remap.
- **Difficulty:** E for the short-lived rows (T ≲ 10τ, 13 rows), S otherwise. The integrator is unregularised and its
  |ΔE/E| reaches 0.019.
- **Caveat:** the energy column has the same discrepancy as RQ-570. II's −E is not I's eq. (4) on the printed (ξ, η)
  either, so record it and do not assert it.
- *Note added 27 Oct 2026 (R-632).* RQ-570 is ruled with its option (a): paper I's column 8 stays transcribed and
  recorded, never asserted. The human's ruling notes that this transcription of paper II found the same discrepancy,
  the caveat above. Nothing above is changed.

### AA-7. Paper II, the 38 completed near-rest starts (Table 3, p. 15; `agekyan_anosova_II_table3_near_rest_completed.csv`)

- **Compare:**
  - T, p, −E, ΔE, Δ′E, M, ΔM/M, n₁, n₂, k and s for the 38 rows paper I left at "—".
  - With I's 59: T̄ = 80.7τ, σ = 83.0τ, T̄ = (80.7 ± 8.4)τ (II eq. 3). The record reproduces 80.67 and 82.99 from 97
    values, not 100 (README §5).
  - Pooled with AA-6: T̄ = (95.4 ± 6.9)τ for the 200 systems (II eq. 4). The authors attribute the 110.1 against 80.7
    difference to chance.
- **Chart:** as AA-2.
- **Difficulty:** as AA-2. The M column departs from η·10⁻⁴ on three rows (N = 13, 27, 52); see README §5.

### AA-8. Exact rest against near rest, per row (I Table 1 and II Tables 1 and 3)

- **Set-up:** the same 100 (ξ, η) twice, once with eq. (3)'s velocities (|v| ≤ 10⁻⁴) and once at rest.
- **Compare:** T_rest against T_near-rest per row. They are "dynamically new triple systems" (II p. 11): N = 1 decays
  at 148.4τ from rest and at 24.7τ from near rest. The scatter of that difference against T is a direct, published
  measure of sensitivity to a 10⁻⁴ velocity perturbation.
- **Chart:** a `Latent` slice with one momentum axis through the region-D point, or two runs per pixel.
- **Difficulty:** S. The two sets of runs used different integrators (linearised in II only).

---

## Boekholt, Moerman & Portegies Zwart 2021, *Phys. Rev. D* 104, 083020 (arXiv:2109.07013)

File: `../2026-10-25_replication/papers/boekholt_moerman_pz_2021_PRD_104_083020_arXiv2109.07013.pdf`. **Page numbers are
the arXiv preprint's.** Catalogue §3.3.

**Common set-up** (§§II–IV, Appendix C):

- **System.** Masses 3, 4, 5 at rest at (1, 3), (−2, −1), (1, −1). G = 1.
- **Integrator.** Brutus: Bulirsch–Stoer with arbitrary-precision words. The tolerance ε and word length L_w are
  decreased and increased until converged. With post-Newtonian terms, the Auxiliary-Vector-Algorithm replaces leapfrog
  (App. B).
- **ζ.** The dimensionless speed of light. The **Newtonian regime is ζ > 10⁶** (p. 5: "the dimensionless lifetime is
  constant and consistent with the Newtonian solution").
- **Escape** (p. 5):
  - at least 5 distance units from the **centre of mass**;
  - moving away from it;
  - positive energy;
  - all three held for at least 100 time units.
- **Merger:** separation below the sum of the gravitational radii.

### B-1. The Newtonian lifetime of Burrau's problem (Fig. 3, bottom panel, p. 6)

- **Compare:** the lifetime T_l (N-body units) as ζ → ∞.
  - **The paper prints no value.** It is plotted only.
  - The bottom panel shows a flat plateau for log₁₀ ζ ≳ 6 at **log₁₀ T_l ≈ 1.8**, i.e. T_l ≈ 63. That is a figure
    reading, good to about ±0.05 in log₁₀, which is 56–71.
  - Whether T_l is the time the escape conditions are first met, or that time plus 100, is not stated.
- **Chart:** the `BodyPlane` Burrau pixel. Szebehely & Peters 1967 (catalogue §3.1) is the sharper reference for the
  same quantity.
- **Difficulty:** E in principle, since the solution is converged, but only to the figure's resolution here.

### B-2. The Lyapunov time (Eq. 1, Fig. 5, p. 6)

- **Set-up:**
  - The lightest body is perturbed in x in the tenth decimal (App. C).
  - The amplification factor A is the ratio of final to initial phase-space distance (6N-dimensional, after Miller
    1964).
  - T_A is the minimum of the lifetime and the time at which δ = 0.1.
- **Compare:** log₁₀ A = λ₁₀ T_A + γ with **λ₁₀ = 0.101 ± 0.005** and **γ = 1.1 ± 0.3**, giving λ = 0.23 ± 0.01 and
  **τ_L = 4.3 ± 0.2**.
- **Caveat:** the fit is over the triples "in both our experiments" across the whole ζ range, relativistic included. It
  is not a Newtonian-only measurement. The Newtonian triples "lie in the top right" of Fig. 5.
- **Chart:** an FTLE at the Burrau pixel. It calibrates an FTLE colouring at one point only.
- **Difficulty:** S.

### B-3. Time reversal to t = 100 (§III and Table I, pp. 4 and 10)

- **Compare:**
  - The Newtonian benchmark quoted from Portegies Zwart & Boekholt (2018): the forward–backward run to t = 100 and back
    converges to the initial perturbation "only for ε = 10⁻²⁴, and L_w = 128" (p. 4).
  - Table I gives the initial and final states of the unperturbed and perturbed reversal runs at **ζ = 10⁶ with the 1PN
    conservative terms on**, to about 40 digits. For example, the unperturbed final x₃ is 0.999999999999877092833786140987105719279.
- **Caveat:** Table I is **not Newtonian**, so a Newtonian engine cannot reproduce its digits. What transfers is the
  tolerance figure, 10⁻²⁴ and 128-bit for a clean reversal to t = 100. That is a target for the extended-precision
  ladder (`../2026-10-26_precision_ladder/`), not for f64.
- **Difficulty:** E (above double precision only).

---

## Li & Liao 2019, *New Astron.* 70, 22–26 (arXiv:1805.07980)

File: `../2026-10-24_anosova/papers/li_liao_2018_arXiv_1805.07980.pdf`, the arXiv v1 preprint of 21 May 2018. **Page
numbers are the preprint's.** Catalogue §6.3.

**Common set-up** (§II, p. 2):

- **System.**
  - Body 1 at A = (−0.5, 0) and body 2 at B = (0.5, 0), body 3 at C = (x, y) in region D, all at rest. G = 1.
  - Masses (m₁, m₂, m₃) in six sets: (1,1,1), (1,0.8,0.8), (1,0.8,0.6), (1,0.8,0.4), (1,0.8,0.2) and (0.6,0.8,1).
- **Search.**
  - (x, y) scanned at Δx = Δy = 10⁻⁴ with dop853 (Hairer, Nørsett & Wanner).
  - Candidates where the return-proximity function, the 12-dimensional distance ‖y(t) − y(0)‖ of eq. (1), is < 0.05.
  - Refined by Newton–Raphson, then verified with clean numerical simulation (arbitrary-order Taylor, arbitrary
    precision, confirmed by a second, cleaner run).
  - Accepted at return proximity < 10⁻⁶.
- **Cuts** (p. 2): period < 200, and minimum separation > 10⁻⁶ (collisionless).

### LL-1. Closure of the 234 orbits (Tables A1–A5, pp. 5–9)

- **Format.** Each row is "Fᵢ (m₁, m₂, m₃)", x, y, T, printed to **10 decimals with trailing zeros dropped** (for
  example "0.068494294" is 0.0684942940).
  - Table A1 holds (1,1,1), F1–F30, and (1,0.8,0.8), F1–F29.
  - Tables A2 and A3 hold (1,0.8,0.6), 69 rows, and (1,0.8,0.4), F1–F44.
  - Table A4 holds (1,0.8,0.2), F1–F32.
  - Table A5 holds (0.6,0.8,1), F1–F30.
  - Counts per set (Table I, p. 2): 30, 29, 69, 44, 32 and 30; **234** in all.
- **Compare:** each IC returns to itself after T. The published ICs are good to their ten printed decimals, so closure
  is limited by those digits times the orbit's instability, not by the authors' CNS. Short periods first: the shortest
  is F2 (1,1,1) at T = 1.6896364928.
- **Chart:** the Anosova physical-frame entry for (1,1,1); for unequal masses, the same frame on a `MassSimplex` point.
  - **Caveat:** for unequal masses the labelling matters. Region D covers every *shape* only up to permuting the
    bodies, so each mass set's points lie in one labelled sector. Li & Liao say one of Yasko & Orlov's orbits
    ("Orbit 19") is missing because its IC "is not in the region-D" (p. 3).
- **Difficulty:** E. `../2026-10-26_orbit_catalogues/` already carries a free-fall family attributed to this paper
  (README §5); test the paper's rows, not the atlas's.

### LL-2. Brake-orbit symmetry: all velocities vanish at T/2 (p. 3)

- **Compare:** "all bodies of these periodic orbits have zero velocities at time t = T/2", after which the bodies
  retrace their paths. It holds for every row, so it is a per-orbit exact check at half the period. It is cheaper and
  stricter than full closure.
- **Chart:** as LL-1.
- **Difficulty:** E.

### LL-3. Known orbits, recovered (p. 3)

- **Compare:**
  - F1 (0.6, 0.8, 1) is Standish's (1970) orbit.
  - F1 (1,1,1) and F2 (1,1,1) are Yasko & Orlov's Orbits 16 and 17 (*Astron. Rep.* 59, 404, 2015).
  - Table II (p. 2) highlights six new orbits, with trajectories in Fig. 2 (p. 3): F10 (1,1,1), F1 and F24
    (1,0.8,0.8), F5 (1,0.8,0.6), F27 (1,0.8,0.4) and F6 (1,0.8,0.2).
- **Difficulty:** E.

### LL-4. Scale-invariant period per letter (p. 3, Fig. 3)

- **Compare:**
  - T̄* = (T/L_f)|E|^{3/2} ≈ **2.363** for the equal-mass free-fall orbits, against 2.433 for Li & Liao's 2017
    collinear set. L_f is the word length; Tables A6–A10 (pp. 10–14) give the free-group words.
  - For m₁ = 1 and m₂ = 0.8: **1.663, 1.191, 0.776 and 0.527** at m₃ = 0.8, 0.6, 0.4 and 0.2. Linear fit
    T*/L_f = **1.912m₃ + 0.084** (Fig. 3).
- **Chart:** none needed. It needs the encounter-word reader (R-125).
- **Difficulty:** E per orbit; the fit is S.

---

## Hudomal 2015, MSc thesis, Belgrade

File: `../2026-10-26_orbit_catalogues/hudomal_2015_msc_belgrade.pdf`. The thesis's printed page numbers equal its PDF
pages. **Already used elsewhere:**

- Tables 1 and 16–22 (`../2026-10-26_belgrade_gallery/crosscheck.py`);
- the word convention of §4.1.

The targets below are the parts not used yet. Catalogue §2.3.

**Common set-up** (§5.1, pp. 53–56; Table 1 caption, p. 16):

- **System.** Equal masses, G = m = 1, L = 0. Positions collinear, with the outer bodies at ∓1 and the middle body at
  0. Velocities: the two outer bodies have (p₁, p₂), the middle one −2(p₁, p₂).
  - **The labelling differs between the thesis's two statements.** Table 1 puts body 3 in the middle; eq. (18) and
    Fig. 32 put body 2 there. Same system; state which.
- **Integrator.**
  - Runge–Kutta–Fehlberg with adaptive step (Šuvakov's code, p. 15).
  - The energy drift ranges from below 10⁻¹⁰ to below 10⁻⁵, depending on the orbit (p. 53).
- **Return proximity.**
  - The minimum over the run of the distance from X₀, by linear interpolation within each step (eq. 19).
  - Maximal period T₀ = 100 (p. 25).
  - Candidates: local minima < 10⁻², about 200 of them.
  - Refined by a 5 × 5 gradient-descent grid, halving from d_p = 10⁻⁴.
  - Rejected above 10⁻⁴ (pp. 25 and 56).

### HU-1. The return-proximity map over the velocity plane (Fig. 8, p. 26)

- **Set-up:**
  - (p₁, p₂) ∈ [0.3, 0.5] × [0.1, 0.5], at **2000 × 4000** points, step 10⁻⁴.
  - Colour is −log₁₀ of the return proximity, with T₀ = 100.
  - The search was later extended to [0.2, 0.3] × [0.4, 0.5], [0.5, 0.6] × [0.34, 0.46] and
    [0.38, 0.44] × [0.50, 0.54], and around dragonfly (0.0806, 0.5888) and bumblebee (0.1843, 0.5872) (p. 25).
- **Compare** (morphology): "Bright spots represent candidates for periodic orbits. Dark lines probably correspond to
  trajectories which lead to collision of two bodies." The candidates' coordinates are Table 23's (HU-3).
- **Chart:** **the Šuvakov velocity plane**, coloured by a return-proximity scalar. No numbers are printed off the
  figure.
- **Difficulty:** Q for the map; E at the tabulated minima (HU-3).

### HU-2. The butterfly III zoom (Fig. 34, p. 55)

- **Set-up:** −log₁₀ d over [0.40, 0.42] × [0.22, 0.24].
- **Compare:** "Three candidate solutions are visible. The middle one is the previously known butterfly III orbit". Its
  IC is (0.405916, 0.230163), T = 13.866 (Table 23). The other two are not identified.
- **Chart:** as HU-1, zoomed.
- **Difficulty:** Q, with one E point.

### HU-3. Initial conditions, periods and return proximities of the 38 thesis orbits (Table 23, p. 59)

- **Format.** Label, ẋ₁(0), ẏ₁(0) to six decimals, T to three, and the return-proximity value from 9.75×10⁻⁸ to
  4.50×10⁻⁴. Not rescaled; contrast Tables 16–22, which are rescaled to E = −½.
- **Compare:** closure after T. It is limited by six-digit ICs, so the gallery's longer digits
  (`../2026-10-26_belgrade_gallery/belgrade_gallery.csv`) are the better closure source. Table 23's return-proximity
  column is the *published* quality of each IC, a fair bar for our closure at the same digits.
  - Five labels changed later (`../2026-10-26_belgrade_gallery/README.md`, cross-check item 4): thesis IVa.6.A,
    IVa.8.A, IVa.8.B, IVa.8.C and IVb.15.A.
- **Chart:** points on the Šuvakov velocity plane.
- **Difficulty:** E.

### HU-4. Period against word length, per sequence (§4.1, pp. 28–41; Tables 3, 5, 7, 9, 11, 13, 15)

- **Compare:** linear fits T|E|^{3/2} = a(n + n̄)/2 + b.

| Sequence | a | b | page |
|---|---|---|---|
| I, butterfly I | 9.924 ± 0.015 | 0.04 ± 0.11 | 28 |
| II, dragonfly | 9.20 ± 0.01 | −0.02 ± 0.07 | 30 |
| III, yin-yang | 9.8662 ± 0.0004 | 0.005 ± 0.004 | 32 |
| IVa, moth I | 9.259 ± 0.005 | 1.09 ± 0.03 | 35 |
| IVb, butterfly III | 9.9695 ± 0.0008 | −0.091 ± 0.013 | 36 |
| IVc, moth III | 9.84 ± 0.04 | −0.3 ± 0.5 | 39 |
| V, figure-eight | 9.2312 ± 0.0015 | 0.003 ± 0.017 | 40 |

  - The sequence tables give each orbit's T|E|^{3/2} to six significant figures. Butterfly I and II, I.2.A and I.2.B,
    are 19.9325 and 19.9313 (Table 3, p. 27).
  - The thesis's text cites the moth III fit as "Figure 13" (p. 39); the moth III fit is Figure 21.
- **Chart:** none. It needs the encounter-word reader (R-125) and E per orbit.
- **Difficulty:** E for T|E|^{3/2}, S for the fits.

### HU-5. Period ratios from Ref. [22] (Table 2, p. 17)

- **Compare:** periods rescaled to E = −0.5, with their ratios to Moore's figure-eight T_M8 = 26.1281.
  - S8 26.1268 (ratio 0.999951).
  - Moth I 68.4636; yarn 205.469, giving a yarn/moth I ratio of **3.00114** against a word ratio of 3.
  - Butterfly I 56.3776, II 56.3746, goggles 112.129, dragonfly 104.005, bumblebee 286.192.
  - Yin-yang I a and b 83.7273; yin-yang II a 334.876.
- **Chart:** none.
- **Difficulty:** E.

The gravitational-wave luminosities (Tables 16–22, Fig. 24) are outside Principia's Newtonian scope and are not
targets.

---

## Heggie 1974, *Celest. Mech.* 10, 217–241

File: `../2026-10-25_replication/papers/heggie_1974_CeMech_10_217.pdf`. The batch's copy was an ADS image scan of the
same paper. These are integrator targets. The default occupant is Heggie × RK4 (R-560), and prin-rs already transcribed
the equations from this paper.

**Common set-up** (§3, p. 223):

- Runge–Kutta–Fehlberg 7(8), about 16 significant figures, the same machine for both methods.
- Heggie's per-step cost is about 1.6× Aarseth–Zare's with eqs. (20)–(21), and a further ~15% with eqs. (22)–(24).

### H-1. Burrau to t = 16: steps and energy error (Table I, p. 223, transcribed in full)

Pythagorean problem.

| Tolerance | Eqs. (21): steps | Eqs. (21): \|δE/E\| | Aarseth–Zare: steps | Aarseth–Zare: \|δE/E\| |
|---|---|---|---|---|
| 10⁻⁸ | 147 | 2×10⁻¹⁰ | 129 | 2×10⁻¹¹ |
| 10⁻¹⁰ | 257 | 7×10⁻¹³ | 225 | 2×10⁻¹² |
| 10⁻¹² | 455 | 2×10⁻¹³ | 397 | 1×10⁻¹³ |

- **Text.** In each case the error at t = 16 was not below those at t = 4, 8 and 12, except AZ at 129 steps, which
  reached 7×10⁻¹¹ at t = 4.
- **Chart:** the `BodyPlane` Burrau pixel.
- **Difficulty:** E for step counts under RKF7(8), which we do not ship; S for "comparable accuracy at about equal
  steps". Steps depend on the error-control details, so compare the *ratio* of steps between the two regularisations
  under one stepper.

### H-2. Aarseth–Zare's "realistic" example (Table II, p. 224, transcribed in full)

The ICs are Aarseth & Zare 1974's Example II (`../2026-10-25_replication/papers/aarseth_zare_1974_CeMech_10_185.pdf`).
|δE/E| is after 0.002 time units.

| Tolerance | Eqs. (21): steps | Eqs. (21): \|δE/E\| | Aarseth–Zare: steps | Aarseth–Zare: \|δE/E\| |
|---|---|---|---|---|
| 10⁻⁸ | 48 | 3×10⁻⁹ | 42 | 4×10⁻⁹ |
| 10⁻¹⁰ | 83 | 3×10⁻¹¹ | 70 | 3×10⁻¹¹ |
| 10⁻¹² | 149 | 3×10⁻¹³ | 124 | 2×10⁻¹³ |

- **Text.** With eqs. (22), (23) and (25) at tolerance 10⁻¹⁰: 1×10⁻¹¹ in 63 steps (p. 224).
- **Difficulty:** as H-1.

### H-3. Close triple encounters and their reversal (Table III, p. 226, transcribed in full)

- **Set-up:**
  - Masses 1, at rest at (−1, 0), (1, 0) and (Δx, √3 + Δy). √3 is entered rounded to 15 decimals.
  - Eqs. (22)–(24), tolerance 10⁻¹².
  - After N_steps the momenta are reversed and the run continued for the same physical time.
  - C/3 is a third of the minimum perimeter.
  - R_f/a_f is the receding body's distance over the binary's semi-major axis at reversal.
  - The last column is the r.m.s. position error {⅓Σ(δx_i² + δy_i²)}^{1/2} on return.

| Δx | Δy | N_steps | C/3 | R_f/a_f | error |
|---|---|---|---|---|---|
| 0 | 10⁻⁴ | 250 | 4×10⁻⁴ | 32 | 3×10⁻¹¹ |
| 0 | 10⁻⁶ | 250 | 7×10⁻⁶ | 30 | 5×10⁻¹⁰ |
| 0 | 10⁻⁸ | 250 | 1×10⁻⁷ | 28 | 3×10⁻⁹ |
| 10⁻⁴ | 0 | 250 | 3×10⁻⁴ | 31 | 6×10⁻¹¹ |
| 10⁻⁶ | 0 | 250 | 6×10⁻⁶ | 29 | 7×10⁻¹⁰ |
| 0 | 0 | 139 | 2×10⁻¹³ | – | 4×10⁻⁶ |

- **Note.** The last row is the equilateral (Lagrange) configuration at rest: a homothetic collapse to triple
  collision, reversed at the step after the minimum C. Heggie (p. 226) says the reversal's success "does not depend on
  any judicious choice for the initial labelling", unlike Aarseth & Zare's.
- **Chart:** `ShapeSphere` at rest through the Lagrange point, the engine's z = 0 landmark, or a `Latent` slice
  displacing body 3. C/3 against Δ is a d_min-scaling test toward a triple collision.
- **Difficulty:** E for C/3 against Δ: 4×10⁻⁴, 7×10⁻⁶ and 1×10⁻⁷ at Δy = 10⁻⁴, 10⁻⁶ and 10⁻⁸. That is not
  proportional, which is itself a target for d_min scaling toward a triple collision. S for the reversal errors, which
  are their integrator's.

---

## Valtonen, Anosova, Kholshevnikov, Mylläri, Orlov & Tanikawa 2016, *The Three-body Problem from Pythagoras to Hawking*

Springer, doi:10.1007/978-3-319-22726-9. © Springer; **not in the repo** (README §2). Book page numbers are given. PDF
page = book page + 12 in Chapters 1–2, + 11 in Chapters 3–7, + 10 in Chapter 8 (the PDF omits the blank versos book
pp. 50 and 146; corrected 27 Oct 2026, it previously read "+ 11 from Chapter 1 on"). The working reference built
from the book, with digitised figures, is `valtonen2016/`.

The book is a popular history. It gives **no integrator, resolution, horizon or escape criterion** for any figure, and
no numerical table. Its figures are reproductions of research maps, mostly by its authors' groups. Its replicable
content is therefore qualitative, and a number from it is a pointer to a primary paper, not a reference value. The
chapters asked about:

- the Agekyan–Anosova map and Maasai shield: Chapter 4, "The Agekian-Anosova Map", "Fractals" and "Chaos in the
  three-body Problem", pp. 85–94;
- symbolic dynamics and entropy: Chapter 4, "Symbolic Dynamics", pp. 93–97, and Chapter 7, "New Frontiers",
  pp. 140–142;
- the Pythagorean problem: Chapter 3, "Calculation of Orbits", pp. 74–76;
- statistical lifetimes and outcomes: Chapter 3, "Statistical Description", pp. 76–81, and "Hierarchical Systems",
  pp. 82–84.

### V-1. The Agekyan–Anosova map with its L/M/A/H regions (Fig. 4.1, p. 86)

- **Content.**
  - Region D drawn **at twice Agekyan & Anosova's scale**: bodies a and b at (−1, 0) and (1, 0), "two units apart" and
    the most distant pair, with the third body in the curved triangle b–c–d.
  - Read from the axes: c at the origin, d at (0, √3).
  - Dashed curves split it into L (near-Lagrangian), M (middle), A (aligned) and H (hierarchical, near b).
  - The text attributes the four categories to Chernin and colleagues (p. 141). It does **not** define the boundaries
    quantitatively.
- **Chart:** the Anosova entry, scaled ×2. The L/M/A/H partition is a candidate symbolic colouring, once Chernin et
  al.'s definitions are in hand (Heinämäki et al. 1999 is in the repo, catalogue §1.9).
- **Difficulty:** Q. The boundaries are not replicable from the book.

### V-2. The full Maasai shield and its zooms (Figs. 4.5–4.7, pp. 94–95)

- **Content.**
  - Fig. 4.5: lifetime over the full lens. Read from the axes: x ∈ [−1, 1], y ∈ [−1.73, 1.73], i.e. Lehto's lens at
    twice the scale. Blue is short-lived, yellow long-lived. The caption: "reflection symmetry about both the vertical
    and horizontal axes", arising from equal masses at rest.
  - Fig. 4.6: one quarter, region D, and a magnification.
  - Fig. 4.7: deeper magnifications ("the stripy structures with dotted (fully chaotic) regions between them").
  - No resolution, bins or zoom windows are given.
- **Chart:** as L-1. The two-axis symmetry is a cheap exact property of the rest surface: a render of the full lens
  must be symmetric to the integrator's rounding.
- **Note (27 Oct 2026):** these are **not** Lehto et al. 2008's Figs. 1–3. The colour key is reversed (book: blue
  short-lived; Lehto: red short, dark blue long) and the zoom windows differ; the book credits no source. Windows in
  both coordinate systems: `valtonen2016/data/fig4_5to4_9_axis_windows.csv`; see V-8.
- **Difficulty:** E for the symmetry; Q for the morphology.

### V-3. The entropy map over the four-region partition (Fig. 4.9, p. 97; text pp. 96 and 141)

- **Content.**
  - Entropy of the H/A/M/L symbol sequence per pixel over one quarter. Read from the axes: x ∈ [0, 1], y ∈ [0, 1.73].
    Blue is low, light brown high.
  - "The short-lived systems in the Maasai shield correspond to low entropy. Low entropy orbits are in principle
    reversible. … Roughly one half of the map is covered by non-reversible systems" (p. 96).
  - A new symbol is appended "when the three-body shape enters a new configuration" (p. 141).
- **Chart:** the Anosova entry, coloured by a symbol-sequence entropy.
- **Difficulty:** Q. It needs the partition (V-1) and the entropy estimator from the primary paper.

### V-4. The Pythagorean orbit in three panels, and Mikkola's near-Pythagorean bounce orbit (Figs. 3.12–3.16, pp. 75–79)

- **Content.**
  - Figs. 3.12–3.15 reprint Szebehely & Peters's positions, the first 10 time units, and the intervals 50–60 and
    60–70. At the end, "the body of mass 3 units escaped" and the other two form "a binary system of very high
    eccentricity" (p. 76).
  - Fig. 3.16 is Seppo Mikkola's periodic orbit with masses 3, 4, 5 from positions "a little different from the corners
    of the Pythagorean triangle". It has an elastic bounce at a collision of the two heavy bodies, and the third body
    stops at the collision (p. 76). **Its initial conditions are not given.**
- **Chart:** the `BodyPlane` Burrau pixel. Szebehely & Peters (catalogue §3.1, in the repo) is the source to replicate,
  not the book.
- **Difficulty:** Q, and E via the primary source. Fig. 3.16 is not replicable without Mikkola's ICs; approximate ICs
  digitised from the figure are V-6.

### V-5. Statistical statements (pp. 76, 80, 84, 98, 140–142)

- **Content.**
  - Samples of "usually … 1000 solutions for each type of a triple system"; "about 95%" escape "within the practical
    time limit of computations", usually the lightest body (p. 76).
  - Valtonen's Cambridge sample of 25,000 orbits (p. 80).
  - Forward–reverse runs fail "in about 50% of cases" (p. 98).
  - "line-dance" and other long-lasting formations appear "about once in a thousand cases" (p. 141).
  - Hierarchical stability: 3.5 binary radii (Harrington 1972, p. 84); 4.8 prograde against 2.6 retrograde, for a
    circular inner binary and an eccentric coplanar outer orbit (Valtonen & Karttunen, p. 140).
- **None states its set-up** (masses, velocities, horizon or criterion). Each is a pointer to the primary paper, not a
  target value.
- **Difficulty:** S, conditional on finding the primary source.
- **Note (27 Oct 2026):** "about 50 %" is consistent with Boekholt, Portegies Zwart & Valtonen 2020 at double
  precision, but Lehto et al. 2008's Fig. 4 shows ≈ 65 % unrecovered at their δ = 5×10⁻⁵; the lifetime-shape claim
  ("radioactive decay", p. 77) conflicts with Orlov et al. 2010 (V-9). Details: `valtonen2016/ch3_*.md`, `ch4_*.md`.

### V-6. Mikkola's near-Pythagorean bounce orbit, from digitised starts (Fig. 3.16, p. 79; text p. 76)

- **Content.** Masses 3, 4, 5. Digitised start points (±0.018 in x, ±0.014 in y;
  `valtonen2016/data/fig3_16_mikkola_markers.csv`): m₃ (0.926, 3.021), m₄ (−1.984, −1.057), m₅ (1.034, −0.977).
  Digitised bounce: m₄–m₅ collision at (−0.619, −0.291), m₃ at (1.841, 0.876). Both sets have their mass-weighted
  centre at the origin within the read-off error. Text: the heavy pair collides, the light body stops at that
  moment, and with an elastic bounce the orbit retraces itself.
- **Assumption, ours:** the starts are a zero-velocity (brake) state; the book does not say so.
- **Test.** From rest at the digitised starts, integrate to the first m₄–m₅ collision and measure |v₃| there and the
  positions against the rings; then shoot on the start positions (four free numbers with the centre of mass fixed)
  for v₃ = 0 at collision. A converged orbit within the digitisation box confirms the reading; none would be a finding.
  Check Szebehely & Peters 1967b (AJ 72, 1187, doi:10.1086/110398; not in the repo) as a possible primary.
- **Chart:** `BodyPlane` around the Burrau pixel, or a `Latent` slice through Burrau's masses.
- **Difficulty:** E once shot (a periodic orbit closes or it does not); Q against the figure.

### V-7. The L/M/A/H partition as drawn, and the entropy share (Figs. 4.1 and 4.9, pp. 86, 97; text pp. 96, 141)

- **Content.** The book never defines the boundaries. As drawn in Fig. 4.1 (a–b = 2; digitised,
  `valtonen2016/data/fig4_1_*.csv`): L/M is a circle about b of radius **1.3852 ± 0.0023**; the H boundary a circle
  about b of radius **0.535 ± 0.010**; M/A the line **y = 0.4246 − 0.5026x**. Example starts 1–4 at (0.498, 0.963),
  (0.049, 1.502), (0.075, 0.219), (0.749, 0.249). Fig. 4.9's high-entropy (brown) share of the quarter is
  **0.66–0.83** depending on the colour threshold, against the text's "roughly one half" non-reversible.
- **Test.** Colour region D by the partition; generate L/M/A/H sequences on free-fall runs; compute a transition
  entropy; compare the high-entropy share with both numbers, and the map's morphology with Fig. 4.9. Our fits are
  one explicit choice of thresholds, not Chernin et al.'s definition (V-1, V-3).
- **Chart:** the Anosova entry, scaled ×2.
- **Difficulty:** Q (thresholds and estimator unknown); S for the share once an estimator is fixed.

### V-8. The shield's mirror symmetry and the zoom ladder (Figs. 4.5–4.7, pp. 94–95)

- **Content.** The book's Fig. 4.5 is mirror-symmetric about both axes to better than a pixel (mirror correlation
  0.909 / 0.916 against neighbour correlation 0.78 / 0.84; measured on its raster). Zoom windows (book coordinates,
  bodies at (∓1, 0)): x 0.40–0.60 × y 0.90–1.10; x 0.500–0.520 × y 0.910–0.930; x 0.5080–0.5100 × y 0.9140–0.9160
  (Lehto coordinates in `valtonen2016/data/fig4_5to4_9_axis_windows.csv`).
- **Test.** (a) Full-lens render: assert the two mirror symmetries to rounding, pixel for pixel (E). (b) Render the
  three windows and check that banded structure with speckled gaps persists at 10× and 100× (Q), alongside Lehto's
  ε-ladder (L-3).
- **Chart:** as L-1.
- **Difficulty:** E for (a), Q for (b).

### V-9. Lifetime distribution shape on the free-fall surface (book p. 77; Orlov, Rubinov & Shevchenko 2010)

- **Content.** Book: decay "resembles radioactive decay, with a known half-life". Orlov et al. 2010 (in the repo):
  heavy-tailed, differential power-law index about −1.7 to −1.4 depending on the virial coefficient. Manwadkar et al.
  2020 (in the repo): exponential for ergodic interactions, power-law tail from Lévy flights.
- **Test.** Histogram t_end over a region-D render (and over P-1's rings); fit exponential and power-law tails;
  report which describes the tail, with the escape criterion stated.
- **Chart:** the Anosova entry.
- **Difficulty:** S.

### V-10. Post-Newtonian Pythagorean problem and the relativistic triple (pp. 147–149; Fig. 8.1)

- **Content.** Book (distance unit 1 pc): 10⁵ M☉ like Newton; ×10 qualitatively the same; ×100 escaper 900 km/s,
  binary 300 km/s; ×1000 all three merge. Chitan, Mylläri & Haque 2022 (in the repo): mergers at 10⁵ M☉ in their
  standard run, one merger in 2000 crossing times at 10⁹ M☉, transition 10^5.5–10^5.6 M☉ over sixteen Pythagorean
  triangles. Valtonen 1976 (Fig. 8.1): masses 0.5, 0.5, 1.0; digitised markers at t = 1–4.56
  (`valtonen2016/data/fig8_1_valtonen1976_positions.csv`). Momentum checks: 0.5 × 24,000 = 1.5 × 8000;
  3 × 900 = 9 × 300.
- **Test.** Out of scope while Principia is Newtonian. If a post-Newtonian force is ever added, the target is Chitan
  et al. 2022, not the book; the momentum identities are cheap checks of any port.
- **Difficulty:** S, conditional on a post-Newtonian integrator.

### V-11. Hierarchical stability limits (pp. 82–84, 114, 140)

- **Content.** Harrington 1972: ≥ 3.5 inner semi-major axes, "doubled" for safety (p. 84). Valtonen & Karttunen:
  circular inner binary, coplanar eccentric outer orbit, 4.8 (prograde) and 2.6 (retrograde) units (p. 140). Bohlin
  1923: pericentre 4.7 major axes, stable for three outer revolutions (p. 82). Innanen 1979: 70 % / 144 % of the Hill
  radius (p. 114). **No masses or outer eccentricities are given.**
- **Test.** A `Latent` or hierarchical slice scanning outer pericentre for prograde and retrograde outer orbits; the
  number to compare comes from the primary (Valtonen & Karttunen 2006, doi:10.1017/CBO9780511616006; Mylläri et al.
  2018, MNRAS 476, 830, doi:10.1093/mnras/sty237; neither in the repo), not the book.
- **Difficulty:** S, conditional on the primary source.

---

## Five to do first

1. **AA-6, Agekyan & Anosova II exact-rest decay times.**
   - It is the only per-IC outcome table in the batch on Principia's own free-fall surface. It is already transcribed
     twice and self-consistent with the paper's statistics.
   - It reuses the region-D import that R-592 built, so the work is the τ conversion and a `t_end` comparison.
   - Its short-lived rows are an exact test of the classifier and the integrator on cheap runs.
2. **LL-2 and LL-1, Li & Liao brake-orbit symmetry and closure.**
   - Exact, cheap, on the same surface, and the T/2 zero-velocity check needs half a period per orbit.
   - The unequal-mass sets exercise the `MassSimplex` axis with a deterministic answer.
3. **Q-2's Theory row and Q-4, the no-integration controls of AAOS 1994b.**
   - Both are pure arithmetic or geometric Monte Carlo. They pin the definitions of Z, and the Monaghan eccentricity
     histogram the outcome classifier will be graded against, before any dynamics is trusted.
4. **P-1 with P-2, the regular ring around Burrau.**
   - It is the direct test of the `BodyPlane` chart at the project's namesake point.
   - The m₁ ring at δ = 5×10⁻⁸ is predicted complete, with m₂ and m₃ broken. Either a complete ring for m₂ or a broken
     one for m₁ would be a finding.
   - The horizon (T ≈ 65) fits inside RQ-555's cap.
5. **L-3, Lehto's local-homogeneity fraction and its ε-ladder.**
   - A number with a stated estimator (8 neighbours, ±1.25×10⁻³, |Δt| ≤ 0.5) that any `t_end` raster of the lens
     yields directly. Its six-rung ladder (28.6 → 8.6%) tests the fractal scaling, not one value.
   - It also exercises the coordinate remap (rule 6) that every region-D map comparison will need.
