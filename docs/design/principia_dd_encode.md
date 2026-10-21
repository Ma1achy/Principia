# Drill-down — Encode (the Question rung's upward door)

*Fourth drill-down. The inverse-encode contract is already maths-dense; this drill-down's value is the assembly: every forward map (decoder dd §3) placed beside its inverse **with the closure verified**, the canonicalisation as an exact ordered procedure, the gauge group action written explicitly (so T1 is a concrete sweep, not a slogan), and the projection metric for curve axes pinned. Consolidation where the contract sufficed; new maths only where the two *precisions* of the one shared kernel could otherwise diverge (encode runs the decode inverted, in the same single source — `principia_spike_brief.md`).*

---

## 1. What it is

The **only upward edge on the ladder**: `physical state → z`, the quotient map onto the decode's canonical section. Consumers: the lock's re-centre, direct physical IC entry, Burrau `(m,n)` entry, invariant-pair lookup, **literature IC import** (how the validation programme's Anosova/Lehto/Burrau starts get loaded), the T-tests, and the `ROUNDTRIP` debug view (a fragment preset, `encode(decode(z))` recomputed fragment-side — `principia_colour_composition.md` §6; R-75).

---

## 2. Consolidated contract

From the **inverse-encode contract** (all of it binds; headline clauses): `E = (block/chart inverses) ∘ C`; **T1** gauge-invariance (deterministic tie-breaks), **T2** right-inverse on the interior, **T3** left-inverse modulo gauge; rigid operations act on the **full phase-space state**; the canonicalisation order is **1a CoM → 0 scale → 1b boost → 2 rotate → 3 mirror** (R-23); the invariant fibre point is the chart's own forward construction (**encode reuses decode** — pending change 3 supersedes "smallest latent norm"); tolerances asserted in **physical units**; everything discarded is **reported** (`lookup_rescaled`, `lookup_mirrored`, `lookup_clamped`).

From the **chart contract / generation root**: block inverses come from the **link registry** (never inlined); ε clamps are registry data (`ε_μ = ε_z = ε_q = 10⁻⁶`, `μ_max = 5`, `q_max = 2`; the last two the defaults of tunable parameters, the active value read, R-481).

From the **decoder dd**: decoded states are already canonical (§3.3 no-op property), so T2 exercises the block inverses and numerics only; body 0 is the softmax reference.

From **navigation (Part 4)**: the lock on nonlinear charts prefers CPU forward replication; encode is the path for *entered* states, not clicked pixels.

---

## 3. The maths

### 3.1 Forward ↔ inverse pairing table — closures verified

| Block | Forward (decoder dd §3) | Inverse (encode) | Closure check |
|---|---|---|---|
| **Mass** | `mₖ = e^{μₖ}/(1+e^{μ₁}+e^{μ₂})`, `μ₀ = 0`; `μₖ = μ_max·tanh(z)` | `μₖ = log(mₖ/m₀)`; `z = artanh(μₖ/μ_max)`, clamp `(1−ε_μ)μ_max` | `m₁/m₀ = e^{μ₁}` ⟹ log-ratio recovers `μ₁` exactly ✓ |
| **Config** | `ρ̃ = cos α (1,0)`, `λ̃ = sin α (cos β, sin β)`; `α = (π/2)σ(z_α)`, `β = πσ(z_β)` | `α = atan2(‖λ̃‖, ‖ρ̃‖)`, `β = atan2(|λ̃_y|, λ̃_x)` (R-438); `z = logit(s)` with the range maps inverted | `atan2(sin α, cos α) = α` on `(0, π/2)` ✓; `β ∈ [0,π]` because `β` reads `|λ̃_y|`, not by the mirror alone (R-438) ✓ |
| **Momentum** | `p₀ = −p_ρ − (m₀/M₀₁)p_λ`, `p₁ = p_ρ − (m₁/M₀₁)p_λ`, `p₂ = p_λ`; `qₖ = q_max(2σ(z)−1)` | `p_λ = p₂`; `p_ρ = p₁ + (m₁/M₀₁)p₂`; `s = ½(q/q_max + 1)`, `z = logit(s)`, clamp | substitute forward into inverse: `p₁ + (m₁/M₀₁)p₂ = p_ρ` identically ✓; `Σpᵢ = 0` consumed, not assumed |
| **Invariant fibre** *(momentum stage of (L_z,E)/(L_z,K) charts)* | `v = L_z·Jr + a·w`, `a = √(2(K*−L_z²/2))` (I = 1) | chart coords = evaluate `L_z(x)`, `E(x)` directly; latent = **run the same construction**, then free-momentum-invert the constructed `p = m·v` | closure is *by construction* — one code path, so lookup and clicked pixel land on the same z |
| **Curve (Burrau)** | `γ: ν ↦ (α, β, μ₁, μ₂)`; `θ(ν)` monotone | on-curve: `ν(θ) = sec θ − tan θ` from the canonical leg ratio; masses cross-check `μ₁ = log(b/c)`, `μ₂ = log(a/c)` | `sec θ − tan θ` inverts `θ(ν)` in closed form on `θ ∈ (0, π/2)` ✓ |

**`β` at the `α = 0` pole (R-437).** At `λ̃ = 0`, `β = atan2(|λ̃_y|, λ̃_x)` is at its pole: there `β` is 0, whatever the
signs of the zeros, chosen by a select on `λ̃_x = 0` and `λ̃_y = 0`, not obtained from `atan2(0, 0)`, which GLSL.std.450
and WGSL leave undefined (`principia_dd_decoder.md` §3.2, "Every `atan2` at a possible pole"). `α = atan2(‖λ̃‖, ‖ρ̃‖)`
has no pole: after step 0, `‖ρ̃‖² + ‖λ̃‖² = I = 1`.

**`β` reads `|λ̃_y|` (R-438).** `β = atan2(|λ̃_y|, λ̃_x)` reads the mirror's quotient for `β` only; R-82's mirror
test and deadband are unchanged. `β ∈ [0, π]` holds because `β` reads `|λ̃_y|`, not by the mirror alone: after §3.2's
step 3, `λ̃_y ≥ −δ_λ`, not `λ̃_y ≥ 0`, and with `λ̃_y` read as it is, a deadband `λ̃_y ≤ 0` with body 2 on the `−x` side
gives `β` near `−π` (`atan2(−0, x < 0) = −π` exactly), which `s_β` would clamp to the `β = 0` end. `β ≈ 0` and
`β ≈ π` are different collinear shapes, not mirror images: the mirror fixes the x-axis. Near `α = 0`, where
`‖λ̃‖ ≲ δ_λ`, the whole lower half of the `λ̃` disc lies inside the deadband, and `|λ̃_y|` still gives a `β` in range,
continuous across the deadband's edge. The code reads `|λ̃_y|` by `abs`, which clears the sign bit, never by `max`:
GLSL.std.450 `FMax` returns `−0` for `max(−0, +0)`, and Rust's `f64::max` may return either zero. At `λ̃ = 0` the
select above gives `β = 0`.

**`β` on the `−x` half-axis selects `π̂` (R-438; physics review 5532062124 P1, applied per R-369).** `abs` pins the
zero's sign only as far as `atan2`'s operand. WGSL may ignore the sign of a zero, under the same IEEE-754 deviations
that let it assume there are no infinities (R-436), and so may Vulkan without `SignedZeroInfNanPreserve` and Metal's
fast-math. So the generated encode's `atan2(+0, x < 0)` can evaluate as `atan2(−0, x < 0) = −π`, which `s_β` would
clamp to the `β = 0` end, at every exactly collinear input with body 2 on the `−x` side. As `rho_angle`'s antiparallel
case does (`principia_dd_decoder.md` §3.6), `β` is defined case by case, each case chosen by a comparison with the
constant 0, selecting a value already computed, with no branch:
- `λ̃_x = 0` and `λ̃_y = 0` (the `α = 0` pole): 0, above (R-437);
- `λ̃_y` compares equal to 0, true for `+0` and `−0` alike, and `λ̃_x < 0`: `π̂`, the image of π in
  `principia_dd_decoder.md` §3.2 (rounded toward zero at f32, the nearest value at f64; R-417);
- `atan2(|λ̃_y|, λ̃_x)` compares above `π̂`: `π̂` (code review C8 and qa Q5 of PR #228, applied per R-369 under R-438
  and R-417). Near the `−x` half-axis the true `β` is just below π, and an f32 `atan2` that rounds to nearest (glibc's
  `atan2f`) can return f32's nearest π, which is above `π̂`; the select keeps `β` in `[0, π̂]`, the range `s_β` reads;
- elsewhere: `atan2(|λ̃_y|, λ̃_x)`.

At f64, `π̂` is what the CPU's `atan2(+0, x < 0)` returns, so the select pins the GPU to the CPU's value, and CPU-f64 and
GPU-f32 agree on the half-axis, `±0` operands included. `λ̃_y = 0` with `λ̃_x > 0` needs no select: either zero gives
`β = ±0`, and the clamp sets `s_β` to `ε_z` for both.

**The curve's closed form** (defined per R-72, applied per R-369; TASK-M2-18, REQ-ENC-011). `C(x)`'s triangle is read
in the Burrau convention (`principia_chart_reference.md` §4.2: the right angle at body 0, each side opposite the body of
its mass): the legs `a = ‖r₁ − r₀‖`, `b = ‖r₂ − r₀‖` and the side `c = ‖r₂ − r₁‖`. The acute angle is
`θ = atan2(a, b)` (`tan θ = a/b`, the angle at `r₂`), and `ν(θ) = sec θ − tan θ`, evaluated as `cos θ/(1 + sin θ)` as
the acute-angle charts evaluate it (§4.5). The mass coupling `μ₁ = log(b/c)`, `μ₂ = log(a/c)` is checked against
`C(x)`'s `μₖ = log(mₖ/m₀)`, and its two differences are reported. `x` is on the curve where `ν(θ)` lies in the chart's
`ν` span and `d(ν(θ)) ≤ tol`, §3.4's metric and tolerance: the metric's mass term is 0 exactly where the coupling holds
on `γ(ν)`'s triangle, so the one test decides both. There the argmin is bypassed. On a chart's curve axis the
coordinate is the closed form's, the chart's other axis read by its own kind, and Part 6 step 4's comparison decides
the landing, as for every chart. On the δm strip, whose masses leave the curve's off its `v = 0` row, `θ` is read off
the triangle alone and `v` is the blend's least squares, `v = (m − m_B)·(m_t − m_B)/‖m_t − m_B‖²`; the projection,
which is the curve's, does not apply there.

**The fold: an input at `θ > π/4`** (defined per R-72, REQ-ENC-022; TASK-M2-18). On the
acute-angle charts, `θ ∈ (0, π/4]` and `ν ∈ [√2 − 1, 1)` (§4.5), an input whose closed form gives `θ > π/4` (`a > b`,
so `ν(θ) < √2 − 1`) is the leg-swapped twin of a fold shape: bodies 1 and 2 exchanged, which under the Burrau mass
convention exchanges two masses, a body relabelling and a distinct labelled system (`principia_chart_reference.md`
§4.5; R-27). Encode does **not relabel** it: the gauge group it quotients by (§3.3) holds no body permutation, and the
relabelled pixel's decode is the other labelled system, which Part 6 step 4's comparison refuses. Nor does it refuse
the input: latent `z` represents it (Part 6 case 3). It **falls back to latent `z`** and raises the flag `past_fold`,
decided as `a² > b²` in §3.4's branch-input form (equal legs, the isosceles triangle, are not past it), so that a lookup
can say the state is the fold's twin, at `θ′ = π/2 − θ` with bodies 1 and 2 exchanged. Its `ν(θ)` is past the fold's
end, so it is no chart point (§3.4, "Past the span"), and the projection over the chart's span finds the chart's edge
nearest it; only a twin within `tol` of that edge, which can happen only next to the isosceles triangle where the two
labellings meet, is projected there, as any point within `tol` of the chart is. On a full-range `ν` chart
(`DoubleCover`, R-27) the twin is its own chart point, at `ν′ = (1 − ν)/(1 + ν)`, and lands there.

### 3.2 Canonicalisation `C` — the exact ordered procedure

Every operation acts on the **full state** (all `rᵢ` and all `pᵢ`). Order is load-bearing.

```
1a COM        r ← r − R_com                            (translations gauged; I is CoM-frame-defined)
0  SCALE      I_in = Σ mᵢ‖rᵢ‖²  (CoM frame)          → r ← r/√I_in ;  p ← I_in^{1/4}·p
              record λ = I_in^{−1/2} ; notice lookup_rescaled
              (E_canon = √I_in·E_in ;  L_canon = L_in/I_in^{1/4} ;  t_canon = t_in/I_in^{3/4})
1b BOOST      p ← p − mᵢ·P_tot/M                       (boosts gauged)
2  ROTATE     φ = atan2(ρ_y, ρ_x) ;  apply R(−φ) to every rᵢ AND every pᵢ ;  ρ̃ lands on +x
              ρ = 0 → reject (coincident inner pair)
3  MIRROR     if λ̃_y < −δ_λ : apply diag(1,−1) to every rᵢ AND every pᵢ ; notice lookup_mirrored   (λ̃ = √μ_λ·λ)
              |λ̃_y| ≤ δ_λ = 10⁻¹² : deterministic no-mirror (T1 depends on this tie-break; R-82)
4  INVERT     per §3.1, into the active chart where possible, else latent z (always possible)
5  FIBRE      chart's own construction first ; smallest-latent-norm only where none exists
6  VALIDATE   hypercube → chart feasibility → decode sanity ; project / clamp / reject ; surface flags
```

**Step 2's `φ` at the pole (R-437).** At `ρ = 0`, `φ` is 0, whatever the signs of the zeros, so `R(−φ)` is the
identity, on CPU-f64 and GPU-f32 alike: a select on `ρ_x = 0` and `ρ_y = 0`, not `atan2(0, 0)`
(`principia_dd_decoder.md` §3.2, §3.3). The guard (`lookup_clamped`, R-13) still catches the pixel; the defined `φ`
makes the rotation there the same on both, the identity.

**After step 3, `λ̃_y ≥ −δ_λ`, not `λ̃_y ≥ 0`.** The deadband `|λ̃_y| ≤ δ_λ` is left unmirrored (R-82), so a state can
leave `C` with `λ̃_y ∈ [−δ_λ, 0]`, `−0` included. §3.1's `β` reads `|λ̃_y|` for that reason (R-438).

The order is normative (R-23): **1a** subtract CoM → **0** rescale → **1b** subtract boost → **2** rotate → **3** mirror. These steps commute with nothing; *CoM subtraction precedes the `I` computation* because `I` is CoM-frame-defined. The inverse-encode contract (Part 6) uses the same numbering.

### 3.3 The gauge group action, explicit (this is what T1 sweeps)

For random `g = (a, u, θ, s, Π)` — translation `a ∈ ℝ²`, boost `u ∈ ℝ²`, rotation `θ`, scale `s > 0`, mirror `Π ∈ {𝟙, diag(1,−1)}`:

```
rᵢ' = s · R(θ) · Π · rᵢ + a
pᵢ' = s^{−1/2} · R(θ) · Π · pᵢ + mᵢ · u
```

**T1 asserts `E(g·x) = E(x)` for all such g** — including g's that land `λ̃_y` inside the mirror deadband (the tie-break sweep). The scaling law on momenta (`s^{−1/2}`) is the similarity symmetry; using `s^{+1/2}` or forgetting `Π` on `p` are the two errors this explicit form exists to prevent.

### 3.4 Curve projection (off-curve input) — metric pinned

Generic input is not on the Burrau embed. Projection:

```
ν* = argmin_ν  d²(ν) = ‖ n(α,β) − n(α_γ(ν), β_γ(ν)) ‖²  +  ‖ m − m_γ(ν) ‖²
```

— shape-sphere **chordal** distance ⊕ mass-simplex **Euclidean** distance, unit weights (both O(1)-normalised). 1-D minimisation over a smooth curve (Brent/golden-section grade); report `d(ν*)`; beyond tolerance → the curve chart cannot represent the point → fall back to latent z. **The metric and weights are confirmed by R-24.** On-curve inputs bypass the argmin via the closed form (§3.1) — and the two paths agreeing *is a test* (5.7).

**As built** (defined per R-72, applied per R-369; TASK-M2-18, REQ-ENC-017, REQ-ENC-020). The one source is
`kernel::encode::curve` beside the curve embeds; every consumer calls its `nearest` (R-542), never a second
implementation.
- **The points.** `n(α, β)` is `C(x)`'s shape point, the shape map of its mass-weighted Jacobi vectors
  (`principia_chart_reference.md` §3.1), its `w` read as `|w|` as `β` reads `|λ̃_y|` (R-438), so it lies on the upper
  hemisphere where the embed's points do; `n(α_γ(ν), β_γ(ν))` is the embed's `γ(ν)` through the same weighting and
  map, and `m_γ(ν)` its masses.
- **Past the span** (defined per R-72, applied per R-369). The search runs over the chart's own `ν` span, the `ν` its
  decode gives at its curve axis's two ends, not over the family's `(0, 1)`. The curve axis (`ν`, or `θ` on the
  acute-angle charts) of every Burrau chart is bounded (`principia_chart_reference.md` §5.4; `K` continues past
  `t = 1`, R-508, and `L_z` and δm are affine): past its edge the chart shows no system, so a `ν` past the span is no
  chart point. A closed form past the span does not land; the search does not leave the span, and where the curve's
  nearest point lies past an edge, `ν*` is that edge (flagged), and the landing is decided by `d(ν*)` as anywhere else.
- **The minimiser.** A grid of 65 points, 64 cells, the span's edges included, then 80 golden-section steps inside the
  best grid point's two neighbouring cells, each step one evaluation, a fixed count; `ν*` is the better of the grid's
  best and the golden section's, so an edge minimum is the edge itself. A tie keeps the earlier point; a NaN start is
  replaced.
- **The landing.** On the curve (§3.1, the argmin bypassed); else projected to `ν*` where `d(ν*) ≤ tol`; else latent
  `z`, `E(x)`'s. `d` is reported in every case. `tol = k·ε_R`, `ε_R` the precision's machine epsilon, one `k` for both
  precisions as `ε_phys` takes it (Part 4): REQ-ENC-026's calibration (R-71), proposed at `k = 128` by
  `fixtures/gates/curve_projection/`, provisional until the M2 gate.
- **The branch inputs** (the human's ruling on RQ-298, 21 Oct 2026: a branch input that must agree across backends is
  built only from operations every IEEE backend rounds alike, with no intermediate that can be subnormal). Every
  comparison reads a squared distance or a squared length, each square formed as `IeeeBits::product(x, x)` of a
  difference `x` taken as 0 below `2^⌊e/4⌋`, `e` the precision's least normal exponent (`2⁻³¹` at f32), so that its
  partial products are normal, and the squares added in one fixed order, no `fma` written: the search's comparisons,
  `d² ≤ tol²` with `tol² = product(tol, tol)`, the fold's `a² > b²`, and the guards of the selects. The values the
  distances are formed from are the decode's own (the embed's divisions, the weighting's `sqrt`) and `C`'s, which a
  backend rounds within its own bounds; two backends can differ on a comparison only where two distances lie within
  those roundings of each other.

### 3.5 Conditioning (numbers, and the tolerance rule)

`d logit/ds = 1/(s(1−s)) ≤ 1/ε = 10⁶` at the clamps; `artanh` likewise at saturation. Therefore every assertion is in **physical space**: `‖D(z) − D(E(D(z)))‖_phys ≤ ε_phys`, never a z-residual near saturation. Clamp events are *expected and flagged*, not failures.

---

## 4. Seams (obligations → integration tests)

| Seam | Obligation | Test |
|---|---|---|
| **8** physical → manifold | quotient onto the section; full-state rigid ops; everything discarded reported | T1/T3 suites (§5); the three `lookup_*` notices fire iff their operation did |
| **2/13** link registry | block inverses are registry inverses, never re-derived inline | swap a link → encode's inverse swaps with it; analytic vs numeric Jacobian agreement inherited from the registry tests |
| **fibre / (L_z,E) chart** | encode reuses decode's construction — one code path | enter `(L_z, E)` numerically vs click the corresponding pixel → identical z to precision (the pending-change-3 test) |
| **Observation** | `ROUNDTRIP` preset = `D → E → D`, physical residual, recomputed fragment-side (R-75) | residual view dark except clamps / feasibility edges / mirror tie — each *tagged expected* |
| **validation import** | literature ICs enter through this door | Anosova region-D and Burrau rest starts encode → decode → invariants match the papers' stated values after the recorded rescale |

---

## 5. Unit tests

Golden pairs: **z = 0** (equal-mass golden IC) and **Burrau** (physical `(3,4,5)` construction ↔ its known ν and mass point).

1. **T2, per chart:** interior z samples → `D → E`, assert physical residual ≤ `ε_phys`; clamp-adjacent samples assert `lookup_clamped`, not a tight residual.
2. **T1 gauge sweep:** random `g` per §3.3 (including deadband-straddling rotations) → `E(g·x) = E(x)` bit-comparably after float tolerance; the mirror tie produces the identical output twice.
3. **T3 with the half-mirror catch:** random-gauge x → `E → D` → equals `C(x)`: shapes match, masses match, `E` matches after the recorded rescale, `|L_z|` matches **with sign consistent with `lookup_mirrored`** — a config-only mirror fails exactly this check.
4. **Scale step:** feed the Burrau state at 2× scale → `lookup_rescaled` with `λ = 1/2`... precisely `I_in = 4·I_canon` ⇒ recorded `λ = 1/2`, `E_canon = 2·E_in`, timing conversion factor `I_in^{3/4}` — assert the whole table row.
5. **Burrau closure:** physical `(3,4,5)` → encode → masses `(5,4,3)/12`, right angle recovered, `ν` matches `ν(θ) = sec θ − tan θ`; decode back → same triangle to `ε_phys`.
6. **Infeasible invariant entry:** `(L_z, E)` with `K* < L_z²/2` → projected/clamped/rejected per the validation ladder, flagged, never silent, never NaN.
7. **Curve projection consistency:** exact on-curve input — argmin and closed form agree on `ν` to tolerance; off-curve input — `d(ν*)` reported, latent fallback taken beyond tolerance, and `E ∘` fallback still satisfies T2. The distance is §3.4's metric (R-24).
8. **Conditioning formulation:** a near-clamp input where the z-residual is ~10⁵× the physical residual — asserting that the *physical* tolerance passes while a naive z-tolerance would fail (the test that keeps future contributors honest about §3.5).
9. **Coincident-pair rejection:** `ρ → 0` inputs reject at step 2 with the correct reason, both precisions, same branch.
10. **Collinear `β` in the deadband (R-438):** encode collinear shapes with `λ̃_y ∈ {−1e-13, −0, +0, +1e-13}`, body 2
    on either side and at each Euler landmark, requiring `β = π` (body 2 on the `−x` side) or 0 (on the `+x` side) as
    appropriate, never near `−π`, and a round trip within the deadband.
    - **The pole landmark** (physics review 5532062124 P2, applied per R-369). With `m₀ = m₁`, the landmark with body 2
      in the middle has body 2 exactly at the inner pair's centre of mass, so `λ̃ = 0` and `α = 0`: body 2 is on
      neither side. There, at `λ̃_y = ±0`, `β = 0` by R-437's select (R-438 condition (1)), never `atan2(0, 0)`. At
      `λ̃_y = ±1e-13` with `λ̃_x = 0`, the ruling's formula gives `β = π/2` (`π̂/2` as computed), and the test asserts
      that value. The side rule applies to the landmarks with `λ̃_x ≠ 0`. The test's masses are chosen so that no other
      landmark sits on the pole, and the pole case is tested on its own.
    - **The `−x` half-axis** (P1): at `λ̃_y = ±0` with `λ̃_x < 0`, `β = π̂` by §3.1's select, on CPU-f64 and GPU-f32 alike.
    - **What the round trip compares** (P3; R-72, applied per R-369; R-438, "Contradiction flagged, applied"). An
      exactly collinear input clamps `s_β` to `ε_z` or `1 − ε_z`, so the decode returns `|λ̃_y| ≈ π·ε_z·‖λ̃‖`, about
      10⁶ × `δ_λ`: a deadband `λ̃_y` cannot come back (inverse-encode Part 7: the chart cannot cleanly reach an exactly
      degenerate configuration). So, at each landmark off the pole, compared in mass-weighted Jacobi coordinates (the
      masses, `ρ̃`, `λ̃`, `p_ρ`, `p_λ`; physics review 5532166783 P1): the four `λ̃_y` inputs encode to the same `z`;
      `decode(encode(x))` keeps the input's side, with `|Δλ̃_y| ≤ π·ε_z·‖λ̃‖`, plus the input's own `|λ̃_y|` and rounding;
      `|Δλ̃_x| ≤ ‖λ̃‖(1 − cos(π·ε_z))` plus rounding, since the clamp moves `β` by `π·ε_z` and so moves `λ̃_x` too, which
      `ε_phys`, calibrated over interior `z`, does not cover; the remaining components (the masses, `ρ̃`, `p_ρ`, `p_λ`)
      are within `ε_phys`; `lookup_clamped` is set. At the pole landmark the test asserts `β` and
      `lookup_clamped`; its round trip is REQ-DEC-014's exact-pole ingest (no NaN, deterministic,
      `lookup_clamped` set).

---

## 6. Deferred / flagged

- ~~**Projection metric pin (§3.4)**~~ — **settled by R-24:** chordal-⊕-Euclidean, unit weights, confirmed; in the shared source with the curve embeds (`kernel::encode::curve`, TASK-M2-18).
- ~~**Operational order of steps 0/1 (§3.2)**~~ — **settled by R-23:** 1a → 0 → 1b → 2 → 3 is normative, in the contract's numbering as here; pinned in the **shared source** so both instantiations (CPU-f64, GPU-f32) inherit the identical sequence.
- **Batch import ergonomics** (CSV of literature ICs → encode → tagged report per row) — a validation-phase tool, not contract; noted so the import seam test has a harness to live in.

---

*Every forward has its inverse beside it, and the closure is checked on paper before it is checked in code. The group action is written out so T1 is a loop, not a hope. One construction serves the pixel and the lookup. And the residual that matters is measured in the world, not in z.*
