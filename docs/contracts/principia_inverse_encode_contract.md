# Principia — inverse encode contract (encode: physical → z)

*Ninth doc. The mirror image of the chart & decoder contract. Three docs already lean on this seam without specifying it: the lock's anchor preservation, physical-frame charts (Anosova), and direct IC entry — including how literature ICs (Lehto, Anosova, Burrau triples) get loaded for the validation programme. This doc holds the block formulae (Part 3), the lookup forms and the validation ladder (Part 6), and the mathematics that makes the whole thing well-defined: what encode* is *as a map, its theorems, its two gaps, and its tests.*

---

## Part 1 — What encode is: a quotient map onto a section

The forward decode `D: z → (m, r, p)` lands on a **canonical section** of physical state space: CoM at origin, total momentum zero, `ρ̃` on +x, `λ̃_y ≥ 0`, `I = 1`. Encode `E` is the composition

```
E  =  (block/chart inverses)  ∘  C
```

where `C` is **canonicalisation**: the quotient map from an arbitrary physical state onto that section. (`C`'s mirror test leaves its deadband unmirrored, so `C` lands on `λ̃_y ≥ −δ_λ`, R-82; Part 3's `β` reads `|λ̃_y|` for that reason, R-438.) The gauge group being quotiented:

```
G  =  T(2) translations × T(2) boosts (CoM frame) × SO(2) rotation × ℝ₊ scale × Z₂ mirror-convention
```

**The three theorems (the contract):**

- **T1 (well-defined).** `E(g·x) = E(x)` for all `g ∈ G`. Encode is constant on gauge orbits. This *requires* deterministic tie-breaking (mirror iff `λ̃_y < −δ_λ`; the tie `|λ̃_y| ≤ δ_λ = 10⁻¹²` resolves to no mirror, R-82; `ρ̃ = 0` is excluded — exactly coincident bodies can't be represented, and the lookup range check catches them with `lookup_clamped`, R-13). T1 holds for every `g` off the mirror band. On `|λ̃_y| ≤ δ_λ`, collinear states included, it holds for translations, boosts, rotations and scalings, and `x` and `Π·x` encode as two distinct `z`, the mirror's two sides both kept; R-82 is unchanged (R-547, RQ-360).
- **T2 (right inverse).** `E(D(z)) = z` up to float, for all `z` in the hypercube interior away from clamps. Decoded states are already canonical, so this exercises only the block inverses and their numerics. Residual bounded by the conditioning `κ(z)` (Part 4).
- **T3 (left inverse modulo gauge).** `D(E(x)) = C(x)` — you get back the **canonical representative of x's orbit, never x itself** (unless x was already canonical). Position, orientation, scale, and possibly parity of the input are deliberately discarded; the physics is preserved up to the corresponding transformation of the trajectory.

**The classic bug, called out once, inherited everywhere:** every rigid operation in `C` acts on the **full phase-space state — positions *and* momenta**. The rotation taking `ρ̃` to +x rotates every `p_i` identically; the mirror reflects every `p_i` through the x-axis. Mirroring configuration only (the bug an agent will write) silently changes the physics: it flips the shape but not `L_z`, producing a state on the manifold that is *not* gauge-equivalent to the input. Under the correct full-state mirror, `L_z → −L_z` and prograde/retrograde flip — that is parity acting on the whole system, dynamically consistent (the trajectory is the mirror of the original). Encode surfaces a **`lookup_mirrored`** notice when it fires, because the user's entered `L_z` sign has changed frame.

---

## Part 2 — The scale gauge gap (missing from the original inverse policy)

The original canonical inverse policy (Part 6, steps 1–5) translates to CoM, rotates, and mirrors — **but never rescales**. Input at `I ≠ 1` therefore breaks T1 and T3: two similarity-equivalent inputs encode to different points, and the round trip does not land on the section. The fix is the similarity transform the whole design is built on (`r → λr, t → λ^{3/2}t`), applied as **step 0 of canonicalisation** — after the CoM subtraction (step 1a) that defines `I`, before the boost removal (step 1b); R-23:

Given input with moment of inertia `I_in = Σ mᵢ‖rᵢ‖²` (CoM frame), set `λ = I_in^{−1/2}` and

```
r  ←  λ r        =  r / √I_in
p  ←  λ^{−1/2} p =  I_in^{1/4} · p
```

Then `I = 1` exactly. Derived quantities transform as

```
E_canon   =  E_in / λ      =  √I_in · E_in
L_z,canon =  λ^{1/2} L_z,in =  L_z,in / I_in^{1/4}
t_canon   =  λ^{3/2} t_in   =  t_in / I_in^{3/4}
```

so `t_end` and all timing outputs are in canonical units; to express them in the user's original units, multiply by `I_in^{3/4}`. Encode surfaces a **`lookup_rescaled`** notice carrying `λ`, in the same family as `lookup_clamped`, exactly when the rescale ran: it is skipped (`λ = 1`, the state unchanged) where `|I_in − 1| ≤ τ_I`, a calibration (R-71) in eps of the precision, so a canonical input raises none (R-547, RQ-295). (The simpler alternative — reject unless `I ≈ 1` — is admissible but strictly weaker; the rescale is the mathematically complete behaviour and matches the project's clamp-and-notify pattern.)

---

## Part 3 — Block inverses (closed forms, with their forward mates)

All three are exact closed forms on the canonical section. Quoted with the forward map each inverts, so an agent can verify the pairing:

**Mass** (forward: `μ_k = μ_max·tanh(z_raw)`, softmax over `(0, μ₁, μ₂)`):

```
μ₁ = log(m₁/m₀),   μ₂ = log(m₂/m₀),   z_μk = artanh(μ_k / μ_max)
```
clamp `|μ_k| ≤ (1−ε_μ)μ_max`, `ε_μ = 10⁻⁶`. Burrau shortcut at shape ν: `μ₁ = log(b/c)`, `μ₂ = log(a/c)`.

**Configuration** (forward: `α = (π/2)·σ(z_α)`, `β = π·σ(z_β)`): from physical positions, translate to CoM,
form $\boldsymbol\rho = \mathbf r_1 - \mathbf r_0$ and $\boldsymbol\lambda = \mathbf r_2 - (m_0\mathbf r_0 + m_1\mathbf r_1)/M_{01}$, mass-weight,
canonicalise, then

```
α = atan2(‖λ̃‖, ‖ρ̃‖),   β = atan2(|λ̃_y|, λ̃_x) ∈ [0, π]
s_α = 2α/π,   s_β = β/π,   z = logit(s)
```

At `λ̃ = 0`, the `α = 0` pole, `β` is 0, whatever the signs of the zeros, chosen by a select on `λ̃_x = 0` and
`λ̃_y = 0`, not obtained from `atan2(0, 0)`, which GLSL.std.450 and WGSL leave undefined; `α`'s operands are never both
0, since `‖ρ̃‖² + ‖λ̃‖² = I = 1` (R-437; `principia_dd_decoder.md` §3.2, "Every `atan2` at a possible pole").

`β` reads `|λ̃_y|`, the mirror's quotient, for `β` only (R-438): `β ∈ [0, π]` holds because it does, not by the mirror
alone, since the mirror's deadband leaves `λ̃_y ∈ [−δ_λ, 0]` unmirrored (R-82, unchanged). `|λ̃_y|` is read by `abs`,
never by `max`, whose `max(−0, +0)` can return `−0` (`principia_dd_encode.md` §3.1).

On the `−x` half-axis, where `λ̃_y` compares equal to 0 (true for `±0`) and `λ̃_x < 0`, `β` is `π̂`, R-417's image of π,
chosen by a select, not taken from `atan2` (R-438; physics review 5532062124 P1, applied per R-369). `abs` pins the
zero's sign only as far as `atan2`'s operand: WGSL may ignore the sign of a zero, as may Vulkan without
`SignedZeroInfNanPreserve` and Metal's fast-math, so the generated encode's `atan2(+0, x < 0)` can return `−π`, which
`s_β` would clamp to the `β = 0` end. With the pole's select above, `β` is 0 at `λ̃ = 0`, `π̂` on the `−x` half-axis and
`atan2(|λ̃_y|, λ̃_x)` elsewhere, each case chosen by a comparison with 0, as `rho_angle`'s antiparallel case is
(`principia_dd_decoder.md` §3.6); CPU and GPU agree there, `±0` operands included (`principia_dd_encode.md` §3.1).

`s_α = 2α/π` is the $\alpha_{\min} = 0$ case of the general inverse
$s_\alpha = (\alpha - \alpha_{\min})/(\pi/2 - 2\alpha_{\min})$. $\alpha_{\min} = 0$
(R-21). `μ_max = 5` and `q_max = 2` are settled (R-10), as the defaults of two tunable link parameters since R-481; the inverses and the clamps read the active value.
`s` clamped into `[ε_z, 1−ε_z]`, `ε_z = 10⁻⁶`.

**Free momentum** (forward: `q_k = q_max·(2σ(z)−1)`): invert Jacobi, `p_λ = p₂`, `p_ρ = p₁ + (m₁/M₀₁)p_λ`, then

```
s_k = ½(q_k/q_max + 1),   z_qk = logit(clamp(s_k, ε_q, 1−ε_q))
```
`|q_k| > q_max` → clamp + `lookup_clamped`.

---

## Part 4 — Conditioning: assert in physical units, never in z

`d logit/ds = 1/(s(1−s))` → up to `1/ε = 10⁶` at the clamps; `artanh` likewise blows up at saturation. Consequences, stated as rules:

- **Near chart edges, encode amplifies physical input error by up to 10⁶ in z.** Lock fidelity degrades near saturation; this is intrinsic to compactified controls, not a bug.
- **Therefore every round-trip tolerance is asserted in *physical* space**, where the composed map is well-conditioned: `‖D(z) − D(E(D(z)))‖_phys ≤ ε_phys`. A z-space residual near saturation is meaningless and must not be a test criterion.
- The clamp flags are the honest boundary: inside the clamps, T2 holds tightly; at the clamps, the inverse is by construction lossy and *says so*.

**The norm `‖·‖_phys`** (defined per R-72, REQ-ENC-029; TASK-M2-16). Between a reference state `x` and a state `x′`, both
on the canonical section (`G = M = I = 1`), over the masses, the positions and the momenta of all three bodies:

```
‖x′ − x‖_phys  =  maxᵢ max( |m′ᵢ − mᵢ|,  ‖r′ᵢ − rᵢ‖ / R_x,  ‖p′ᵢ − pᵢ‖ / P_x )
R_x = max(1, maxⱼ ‖rⱼ‖)        P_x = max(1, maxⱼ ‖pⱼ‖)
```

- **Unweighted by mass, each block against the state's own extent.** A position or momentum difference is absolute
  where the reference state's extent in that block is at most 1, and relative to that extent above it; the masses are
  absolute (`Σmᵢ = 1`). On the section a light body sits far out (`‖rᵢ‖` up to `1/√mᵢ`), and the decode builds the
  bodies from the Jacobi vectors (`r₁ = r₀ + ρ`), so a near body's position carries the far body's rounding: measured
  against its own size it would show a conditioning the map does not have. A mass-weighted norm (`√mᵢ‖Δrᵢ‖`,
  `‖Δpᵢ‖/√mᵢ`) is not used: it divides a light body's momentum rounding by `√mᵢ`.
- **One norm for every round trip.** T2 measures `D(E(D(z)))` against `D(z)`, T3 `D(E(x))` against `C(x)`; the
  ground-truth ingestion residual and the `ROUNDTRIP` preset (Part 8) use it unchanged.
- **`ε_phys = k·ε_R`**, `ε_R` the precision's machine epsilon (2⁻⁵² at f64, 2⁻²³ at f32), one `k` for both precisions:
  the interior residual is rounding, the same count of `ε_R` at f64 and f32. `k` is REQ-ENC-024's calibration (R-71),
  the constants register's `eps_phys`, proposed at 64 by the `encode-eps-phys` gate and provisional until the M2 gate.
- **Interior and clamp-adjacent** (applied per R-369, TASK-M2-16). Each control is read off the state by Part 3's own
  formulas. A sample is interior when every control is clear of its clamp by a factor of two: `s_α`, `s_β` and `s_q`
  in `[2ε, 1 − 2ε]` (their clamp `[ε, 1 − ε]`) and `|μₖ| ≤ (1 − 2ε_μ)·μ_max` (its clamp `(1 − ε_μ)·μ_max`); T2 holds
  it to `ε_phys`. A control past its clamp by the same factor, `s` within `ε/2` of 0 or 1 or `|μₖ| ≥ (1 − ε_μ/2)·μ_max`,
  is clamp-adjacent and must set its `lookup_clamped` bit; a control clear of its clamp must leave its bit unset.

---

## Part 5 — Inverse by axis kind (completing the four kinds)

**Kind 1 — raw latent.** Read the component. Exact, unconditional.

**Kind 2 — derived-in-block.** Block inverse (Part 3) **plus the same residual convention as the forward axis**. The convention is part of the axis's identity: encoding "m₀ = 0.4" under the hold-`m₁:m₂` convention and under the hold-`m₁=m₂` convention give different z, and the round trip only closes if encode uses the axis's own convention. One convention, declared once, used both directions. **The shape sphere chart's point is the state's signed `n̂`** (R-544): its lookup reads the shape from `C(x)` with step 3's mirror undone where `lookup_mirrored` is set, so `x` and its mirror `Π·x` land at `(θ, φ)` and `(θ, π − φ)`, two pixels, two systems. T1 stands for `z` and for every other chart; only the shape sphere's chart point keeps the mirror's sign.

**Kind 3 — invariant.** Two directions, sharply different:

- *(a) state → chart coords:* trivial and always exact — evaluate the invariants `L_z(x)`, `E(x)` or `K(x)` directly. No solve.
- *(b) chart coords → latent (the lookup direction):* the fibre `{p : L_z = L*, K = K*}` at fixed `(m, config)` is generically **2-dimensional** (a linear constraint ∩ an ellipsoid in the 4D momentum space), so a canonical point must be chosen. **There are two candidate definitions, and they conflict.** This contract resolves it:
  - The earlier lookup rule: "choose the canonical representative with smallest latent-space norm."
  - The chart's own forward decode says: rigid rotation `v^(L) = (L_z/I)·J r` (the *minimal-kinetic-energy* realisation of `L_z` — a theorem: minimising `K = ½‖v‖²_m` subject to `⟨Jr, v⟩_m = L_z` gives exactly `v ∝ Jr`), plus `a·w` along the deterministic seeded direction field, `a = √(2(K* − L_z²/2I))`.
  
  These are **not the same functional** (kinetic-energy-minimal ≠ latent-norm-minimal after the logit pullback), and if encode ran an independent argmin the round trip would not close. **Resolution — the single-source-of-truth rule: encode reuses decode.** Entering an `(L_z, E)` pair means running the chart's own forward construction (i)–(iii) at the current frozen configuration, exactly as a pixel would, then recovering `z_mom` via the free-momentum inverse of the constructed `p`. The "smallest latent norm" phrasing survives only as the general fallback for case-3 charts with no canonical construction. This supersedes the smallest-latent-norm rule for invariant charts (pending change 3).
  - Feasibility applies before construction: `|L_z| ≤ √(2I(E−U))`, `K ≥ L_z²/2I`; infeasible pairs → project / clamp / reject per the validation ladder (Part 6). Note at the canonical scale `I = 1`, so `ω = L_z` and `K_min = L_z²/2` — the constants simplify because encode already normalised scale (Part 2).
  - The seeded direction family can degenerate at special configurations (no seed has `‖w⁽²⁾‖²_m > ε_w`; otherwise the largest-norm qualifying seed is taken, ties by seed order — R-82); encode then fails with the same `DEGENERATE` label the pixel path would emit — consistent by construction, since it *is* the pixel path.

**Kind 4 — coupled curve.** Two directions:

- *(a) on-curve encode:* exact closed form — recover the acute angle from the canonical config, then `ν(θ) = sec θ − tan θ`; verify the Burrau mass coupling via `μ₁ = log(b/c)`, `μ₂ = log(a/c)`.
- *(b) is x on the curve at all?* Generic input isn't. Project: `ν* = argmin_ν d²(ν)`, `d²(ν) = ‖n(α,β) − n(α_γ(ν), β_γ(ν))‖² + ‖m − m_γ(ν)‖²`, the shape sphere's chordal distance ⊕ the mass simplex's Euclidean one at unit weights (R-24; `principia_dd_encode.md` §3.4) (1D minimisation over a smooth curve — cheap and robust), report distance-to-curve `d(ν*)`. If the distance exceeds tolerance (REQ-ENC-026), the curve chart *cannot represent the point*: fall back to latent z (case 3 of Part 6), which is always available.
- *On the curve and past its span* (defined per R-72, applied per R-369; TASK-M2-18). `x` is on the curve where the closed form's `ν` lies in the chart's `ν` span and `d` there is within the tolerance; then (b) is bypassed. The search runs over the chart's own `ν` span: the curve axis (`ν`, or `θ` on the acute-angle charts) of every Burrau chart is bounded (`principia_chart_reference.md` §5.4), so a `ν` past it is no chart point. An input at `θ > π/4` on a folded chart is not relabelled; it falls back to latent z, flagged (`principia_dd_encode.md` §3.1, REQ-ENC-022).
- *Into the chart: kind 4, then Part 6 step 4* (defined per R-72, applied per R-369; TASK-M2-18; `principia_dd_encode.md` §3.1, "The composition"). Kind 4 supplies the curve axis's coordinate of `c(x)`: the closed form's on the curve (a), `ν*`'s where `d(ν*) ≤ tol` (b), and none where `d(ν*) > tol`, which falls back to latent z (b). The chart's other axis is read by its own kind, and Part 6 step 4's `‖D(Φ(c(x))) − C(x)‖_phys ≤ ε_phys` then decides chart against latent, as for every chart. A chart landing needs both.

**Annotation axes.** Nothing to invert — no IC content by definition.

---

## Part 6 — The canonical inverse policy, completed

**Lookup.** Two forms:

- **Exact IC (full lock).** The user gives a Pythagorean pair $(m, n)$, a triple, a raw $\mathbf z$, or physical
  coordinates $(m_i, \mathbf r_i, \mathbf p_i)$. Encode into the latent chart by the steps below, then lock.
- **Partial specification (constrained slice).** The user gives fewer coordinates than needed. Fill the rest
  with canonical defaults: masses → equal (unless the chart implies Burrau); geometry in a Burrau chart →
  the current locked $\nu$, or $\nu = 1/2$; momentum → rest start; latent coordinates → 0. Then set the slice
  basis to span the unspecified degrees of freedom.

**Three inverse cases.**

1. **Exact inverse available**: latent chart, mass simplex, Burrau geometry, free Jacobi momenta. Part 3's closed forms apply.
2. **Constrained inverse available**: the invariant charts $(L_z, E)$ and $(L_z, K)$. The chart coordinates can be recovered,
   but the latent point is not unique. The fibre point is the chart's own forward construction (Part 5, kind 3).
3. **No canonical inverse**: charts that need projection or optimisation, such as arbitrary physical coordinates
   that don't align with any active chart. Fall back to latent $\mathbf z$ via Part 3. Smallest-latent-norm
   is the tie-break here only.

**The canonical inverse policy.** The original five steps were: translate to CoM; canonicalise; invert into the
current chart or else latent z; among several valid inverses take the smallest latent norm; surface any clamping.
Completed, with two additions (step 0; step 5's resolution) and the full-state rule made explicit. The order is
normative (R-23): **1a → 0 → 1b → 2 → 3**, then 4–6. CoM subtraction precedes the `I` computation, because `I` is
CoM-frame-defined.

- **1a. Subtract the CoM** — positions to the CoM frame, so `I_in` is defined.
- **0. Rescale to `I = 1`** via the similarity transform (Part 2). Record `λ`; notice `lookup_rescaled`. Skipped where `|I_in − 1| ≤ τ_I` (`λ = 1`, `r` and `p` unchanged, no notice); elsewhere in two steps, first by a power of two (exact), then by the remainder, so `I_in` neither underflows nor overflows for any finite state with the bodies apart; `I_in = 0`, the bodies coinciding, keeps the select: divisor 1, `λ = 1`, `p = 0`, `lookup_clamped`, `φ = +0` (R-547, RQ-295, RQ-296).
- **1b. Subtract the boost** — the momentum frame (`p_i ← p_i − m_i P_tot/M`; total momentum zero).
- **2. Rotate `ρ̃ → +x`** — the same rotation applied to **all positions and all momenta**. `ρ̃ = 0` (exactly coincident inner pair) can't be represented → `lookup_clamped` (R-13).
- **3. Mirror iff `λ̃_y < −δ_λ`** (`λ̃ = √μ_λ·λ`, `δ_λ = 10⁻¹²`) — reflect the **full state** (every `r_i` and every `p_i`) through the x-axis. Tie `|λ̃_y| ≤ δ_λ` → no mirror, deterministic (R-82); T1 depends on it. Notice `lookup_mirrored` (the user's `L_z` sign has flipped frame).
- **4. Invert into the active chart** where possible (Part 5, by axis kind), otherwise into latent z (always possible via Part 3).
- **5. Fibre choice**: the chart's own forward construction is the canonical representative (encode reuses decode); smallest-latent-norm only as the documented fallback where no construction exists.
- **6. Validate** — the three layers below, in order, with project / clamp / reject and every flag surfaced (`lookup_clamped`, `lookup_rescaled`, `lookup_mirrored`).

**Where possible, at step 4** (R-72, applied per R-369; REQ-ENC-008). The active chart represents `x` when the
chart point `c(x)`, read off `C(x)` axis by axis (Part 5), decodes back to it: `‖D(Φ(c(x))) − C(x)‖_phys ≤ ε_phys`,
Part 4's norm and tolerance (REQ-ENC-029, REQ-ENC-024), both states on the canonical section. A chart sweeps two DOF
and its slice and conventions pin the other six (chart_decoder_contract Part 5), so a generic state is not on it: its
frozen blocks differ from the slice's, or, on an invariant chart, its momenta are another point of the fibre than the
construction's. Such a state lands in latent `z` (case 3), which represents every state. An annotation axis gives no
coordinate, and nothing is compared for it.
On a curve chart (Part 5, kind 4) `c(x)`'s curve coordinate is kind 4's at REQ-ENC-026's curve tolerance
(`principia_dd_encode.md` §3.1, "The composition"; code review 5606284432 of PR #275, C4, applied per R-369), and that
tolerance is the lookup's to pass, as `‖·‖_phys` and `ε_phys` are. At a tolerance of 0 such a chart places essentially
no state, rounding alone putting `d > 0` on its own curve, so every state would land in latent `z`.

**On the shape sphere chart** (R-544; physics review 5603309678 of PR #273, P1, applied per R-369) the decode runs `C`
with step 3 not taken, so its states lie on that chart's own section, `λ̃_y` keeping its sign. The comparison there is
against `C(x)` with step 3's mirror undone on the **full state**, every `r_i` and every `p_i` (`Π·C(x)` where
`lookup_mirrored` is set, `C(x)` otherwise), and `c(x)` is the signed `n̂` (Part 5, kind 2). So a lower-hemisphere `x`
whose masses and unmirrored body-frame momenta are the slice's lands on the chart at its signed point and decodes back
to itself, `L_z`'s sign kept; its mirror `Π·x` lands at `(θ, π − φ)` only when the slice holds `Π·x`'s momenta, and
otherwise goes to latent `z`. Comparing against the mirrored `C(x)` would send every lower-hemisphere state to latent
`z`. Every other chart compares against `C(x)` as above.

### Chart-aware validation

Not every point of the latent hypercube $[0,1]^8$ is valid in every chart, and not every entered IC lands inside
the hypercube. Lookup and lock validate against the **active chart's** constraints before committing. A trace does
not: it never clamps, projects or rejects a sample's whole state by these layers, but encodes the chart's shown
coordinates and checks each shown axis on its own (R-538 TF2.1; `principia_lines.md` §8).

**Layers, in order.**

1. **Hypercube bounds.** After encoding, check $z_k \in [0,1]$ for every $k$. Outside: clamp (`lookup_clamped`) or
   reject. This catches physical inputs beyond the decoder's range, such as momenta above `q_max` or mass
   ratios beyond `μ_max`.
2. **Chart feasibility.** What each chart adds beyond the hypercube:
   - $(L_z, E)$: inside the parabola $|L_z| \le \sqrt{2I(E - U)}$. A pair outside it has no physical realisation at the current configuration.
   - $(L_z, K)$: $K \ge 0$ and $K \ge L_z^2/2I$.
   - Mass simplex: every $m_i > 0$ and $\sum m_i = 1$. The interior buffer $\varepsilon_m$ narrows it further.
   - Shape sphere: $\theta \in [0, 2\pi)$ (azimuth) and $\varphi \in [0, \pi]$ (polar, from $+w$), with no polar buffer (`principia_chart_reference.md` §3.3, R-12 and R-14).
   - Burrau charts: $\nu \in (0,1)$. The triangle degenerates as $\nu \to 0$ or $\nu \to 1$.
   - Mixed-axis charts inherit both axes' constraints.
3. **Decode sanity.** After decoding: all masses positive; CoM at the origin and total momentum zero, both within
   tolerance. This catches individually valid coordinates that combine into a degenerate configuration.
   **There is no coincident-bodies rejection (R-13).** A looked-up IC takes the same path as any pixel: bodies
   within $r_{\mathrm{coll}}$ give a $t = 0$ collision outcome, which is a real outcome. Exactly coincident bodies
   can't be represented, and layer 1's range check catches them with `lookup_clamped`.

**On failure**, in order of preference:

- **Project**: move to the nearest valid point in the chart's feasible region (for $(L_z, E)$, onto the parabola), lock there, and notify.
- **Clamp**: snap each out-of-range coordinate to its boundary, set `lookup_clamped`, and show which coordinates moved.
- **Reject**: refuse with an error naming the failed constraint. Only when projecting or clamping would give a
  qualitatively different IC from the one intended.

**The nearest valid point** (R-72; REQ-ENC-030). On the invariant charts, a projection keeps the energy and moves
`L_z` alone. At the frozen configuration and masses, the kinetic energy is `K* = E − U` on $(L_z, E)$ and `K* = K` on
$(L_z, K)$, and the points feasible at that energy are the segment $|L_z| \le \sqrt{2I K^*}$. The nearest valid point is
the one at the same `K*`, and so at the same `E`, on the parabola, on the side of the entered `L_z`:
$L_z \leftarrow \operatorname{sign}(L_z)\sqrt{2I K^*}$. Distance is $|\Delta L_z|$ at fixed energy. Each chart's vertical
coordinate is its energy, so the projection moves only the horizontal coordinate, as the charts' own domain function
does. The root is held so that $L_z^2 \le 2I K^*$ exactly, so the projected pair passes the feasibility test it failed.
Where `K* < 0` (`E < U`; on $(L_z, K)$, `K < 0`), no `L_z` is feasible at that energy. The nearest feasible point is
then the parabola's apex, `L_z = 0` at `K* = 0`, `E = U`: the rest start. The lock re-centres to the nearest feasible
point in the same metric. The other charts' layer-2 failures are clamps in the chart's own coordinates, each
out-of-range coordinate snapped to its boundary, which needs no metric.

**A qualitatively different IC** (R-72; REQ-ENC-030). A projection or clamp gives a qualitatively different IC, and
lookup rejects instead, when it changes the class of either constant of motion that classifies the motion:

- **The sense of rotation**, `sign(L_z)` ∈ {−, 0, +}. `L_z = 0` is a class of its own: a triple collision needs zero
  angular momentum (Sundman), so only that class can reach one.
- **The energy class**, `sign(E)` ∈ {−, 0, +}. With `E ≥ 0`, the Lagrange–Jacobi identity
  $\ddot I = 4E - 2U > 0$ ($U < 0$) makes `I` grow without bound, so only `E < 0` admits motion that stays bounded.

Under the metric above, a projection at `K* > 0` keeps `E` and the sign of `L_z`, so it is taken. At `K* = 0` with
`L_z ≠ 0` the parabola's only point is `L_z = 0`, so the pair is rejected (`L_z`'s class would change). At `K* < 0` the
apex keeps `L_z`'s class only where `L_z = 0`. It keeps `E`'s, since `E < U < 0`. So that pair is projected to the
rest start, and every other pair there is rejected. A non-finite entry is rejected, naming its constraint, never
carried on as a NaN. A chart point below the invariant charts' apex, `t < 0`, has no entered `L_z`: `L_z(s, t)` has
no real value there. So projecting it to `t = 0` changes no class, and the projection stands.

**Lock validation.** A clicked pixel is valid by construction, but a tilt can carry the lock's neighbourhood
into an infeasible region. Detect it, then re-centre to the nearest feasible point or switch to a chart where
the lock is valid.

**Implementation.** Each chart type registers `validate(u, v) -> ValidationResult`, a CPU-side check: the GPU
never sees an invalid IC. The quadtree uses the same function to skip quads entirely outside the feasible
region (for example above the parabola in $(L_z, E)$).

**Chart compatibility flags**, read by lookup and lock:

| flag | meaning |
|---|---|
| `forbids_energy_normalisation` | the chart fixes energy itself; any $E^*$ override — every `Some(E*)`, including `Some(0)` (R-25) — is refused at validation (`principia_dd_decoder.md` §3.7). True for $(L_z, E)$ and $(L_z, K)$. |
| `system_image` | how the address map folds onto distinct systems — bijective, n-to-1, `DoubleCover` or ray-degenerate (`principia_chart_decoder_contract.md` Part 5; R-59 D5, R-141, R-157). For an n-to-1 chart or a `DoubleCover` chart (the full-range Burrau chart, R-157), show the canonical half, or label the redundant one. The shape sphere is bijective, the whole sphere, nothing hidden or labelled redundant (R-544; *was:* "the shape sphere: 2-to-1 over the φ hemispheres, R-141"). |
| `requires_per_pixel_mass` | mass varies per pixel, so the integrator must not use `SimUniforms.m[3]`. True for the ternary mass plot, the Burrau family, and mixed-axis charts with a mass axis. |

## Part 7 — Ground-truth ingestion (the validation programme's demand)

*Encode was specified above for the self-round-trip property (T2/T3). The ground-truth validation suite (`principia_validation_ground_truth_note.md`) places a **harder demand**: ingest **foreign** physical ICs — analytic solutions and published orbits (Lagrange, figure-eight, Burrau, Lehto) — that Principia did **not** generate, and route them into the general pipeline. This is the same encode map, but its job and its outputs widen.*

**Everything valid is representable, so a bad round-trip is a located bug — never "out of manifold."** The 8D reduced manifold is the complete space of planar three-body ICs modulo the quotiented symmetries; there is no *physically valid* planar IC outside it. So a foreign IC that fails to round-trip has a bug in one of: **canonicalisation** (the gauge-fix `C`, Parts 1–2 — the most dangerous, because a frame/convention mismatch can round-trip fine yet fail the *forward-behaviour* check, pointing at the wrong subsystem — the gauge-mismatch trap), **convention translation** (a paper's Jacobi weighting / body order / scale normalisation misidentified → encodes to a valid-but-wrong `z`), or **conditioning** (near boundaries/symmetric configs). Ingesting foreign ICs therefore **independently tests `C` itself** — the canonicalisation, otherwise only checked by self-consistency, is exercised on external points.

**Three outputs encode must expose for ingestion** (beyond the `z` it already returns):
1. **The canonicalisation must accept foreign frames** — run `C` (Parts 1–2: CoM, orientation, mirror tie-break, scale `λ`) on an *arbitrary* input, not just decode output. Already specified; now load-bearing for external ICs.
2. **A physical-space round-trip residual** `‖D(z) − C(x)‖_phys` (Part 4 — asserted in physical units, never z) as the **bug signal**: small ⇒ faithfully ingested; large ⇒ a located defect in `C`/convention/conditioning.
3. **A conditioning number** `κ(z)` at the point (Part 4) — distinguishing "hard but representable" (ill-conditioned, often link-fixable — see below) from "fundamental." An implementer building encode for the round-trip property *alone* would expose neither (2) nor (3); the validation suite requires both.

**Two entry paths (routed by interior-vs-degenerate):**
- **Chart-encode (generic ICs):** the full `E` above → `z` → general forward pipeline → compare behaviour. The end-to-end test (encode + decode + `C` + integrator + shape, as integrated). Figure-eight, generic orbits, Burrau.
- **Direct physical inject (exactly-degenerate analytic configs — Euler collinear):** bypass encode entirely; feed the closed-form physical `(m,r,p)` to the integrator+shape via a lower harness entry point. The chart cannot cleanly reach an *exactly*-degenerate config (it sits at a coordinate/parametrisation boundary the open `[0,1]` approaches but never lands on — validation note), so validate the *dynamics* directly. This is a harness concern, not a widening of encode.

**Link variation is a conditioning diagnostic, not a default change.** If a foreign IC is ill-conditioned under the default compactification, re-encoding under a different registry link (the same registry the measure-robustness programme uses) may make it well-conditioned — bisecting "conditioning problem (link-fixable)" from "fundamental." Encode may be run per-IC under the best link *with that link recorded in provenance*; the **default link is chosen for sampling-measure quality, never for validation reachability** (changing it would corrupt the measure the science reports). See the validation note's firewall.

---

## Part 8 — Round-trip contracts and the new debug view

**Generated tests, per chart** (same discipline as the layout-table catalogue):

- **T2 test:** sample z in the interior → `D` → `E` → assert **in physical space**: `‖D(z) − D(E(D(z)))‖_phys ≤ ε_phys` (Part 4's rule). Clamp-adjacent samples are asserted with the clamp flag expected, not with a tight residual.
- **T3 test:** random physical x (random scale, orientation, offset, parity) → `E` → `D` → assert the result equals `C(x)`: shape angles match, `|L_z|` matches with sign consistent with the mirror flag, `E` matches after the recorded rescale, masses match. This is the test that catches a half-applied rigid transform — the Part 1 bug — because a config-only mirror preserves shape but breaks the `L_z` consistency check.
- **T1 test:** `E(g·x) = E(x)` over random `g ∈ G` — the gauge-invariance sweep. Cheap, brutal, and the only test that exercises the tie-breaks.

**New debug view: `ROUNDTRIP`** — a fragment preset, recomputed fragment-side from `ctx.chart.z`, not a kernel mode (`principia_colour_composition.md` §6; R-75). Per pixel: `z → D → E → D → physical residual`, log-scaled. One glance certifies the block inverses, the numerics, and (on invariant charts) the encode-reuses-decode rule across the whole visible chart — the encode-side sibling of the invariant-chart-gradient view. Legitimate bright regions: clamp boundaries, feasibility edges, the mirror tie — *tagged expected*, not bugs.

---

*Encode is the quotient map onto the decode's section: constant on gauge orbits, right-inverse to decode, left-inverse modulo gauge. Rigid operations act on the whole phase-space state — positions and momenta together, always. Scale is canonicalised by the similarity rescale — step 0, after the CoM subtraction that defines `I` (R-23). The fibre point on an invariant chart is whatever the chart's own decode constructs — encode never runs its own optimiser. Tolerances live in physical units; z-space residuals near saturation mean nothing. Everything the inverse discards, it reports.*
