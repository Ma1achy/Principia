# Principia — inverse encode contract (encode: physical → z)

*Ninth doc. The mirror image of the chart & decoder contract. Three docs already lean on this seam without specifying it: the lock's anchor preservation, physical-frame charts (Anosova), and direct IC entry — including how literature ICs (Lehto, Anosova, Burrau triples) get loaded for the validation programme. The spec has the block formulae and a validation ladder (§lookup); this doc supplies the mathematics that makes the whole thing well-defined: what encode* is *as a map, its theorems, its two gaps, and its tests.*

---

## Part 1 — What encode is: a quotient map onto a section

The forward decode `D: z → (m, r, p)` lands on a **canonical section** of physical state space: CoM at origin, total momentum zero, `ρ̃` on +x, `λ̃_y ≥ 0`, `I = 1`. Encode `E` is the composition

```
E  =  (block/chart inverses)  ∘  C
```

where `C` is **canonicalisation**: the quotient map from an arbitrary physical state onto that section. The gauge group being quotiented:

```
G  =  T(2) translations × T(2) boosts (CoM frame) × SO(2) rotation × ℝ₊ scale × Z₂ mirror-convention
```

**The three theorems (the contract):**

- **T1 (well-defined).** `E(g·x) = E(x)` for all `g ∈ G`. Encode is constant on gauge orbits. This *requires* deterministic tie-breaking (mirror tie `|λ̃_y| < δ_λ = 10⁻¹²` resolves to a fixed choice; `ρ̃ = 0` is excluded — coincident bodies, caught by decode sanity).
- **T2 (right inverse).** `E(D(z)) = z` up to float, for all `z` in the hypercube interior away from clamps. Decoded states are already canonical, so this exercises only the block inverses and their numerics. Residual bounded by the conditioning `κ(z)` (Part 4).
- **T3 (left inverse modulo gauge).** `D(E(x)) = C(x)` — you get back the **canonical representative of x's orbit, never x itself** (unless x was already canonical). Position, orientation, scale, and possibly parity of the input are deliberately discarded; the physics is preserved up to the corresponding transformation of the trajectory.

**The classic bug, called out once, inherited everywhere:** every rigid operation in `C` acts on the **full phase-space state — positions *and* momenta**. The rotation taking `ρ̃` to +x rotates every `p_i` identically; the mirror reflects every `p_i` through the x-axis. Mirroring configuration only (the bug an agent will write) silently changes the physics: it flips the shape but not `L_z`, producing a state on the manifold that is *not* gauge-equivalent to the input. Under the correct full-state mirror, `L_z → −L_z` and prograde/retrograde flip — that is parity acting on the whole system, dynamically consistent (the trajectory is the mirror of the original). Encode surfaces a **`lookup_mirrored`** notice when it fires, because the user's entered `L_z` sign has changed frame.

---

## Part 2 — The scale gauge gap (missing from the spec's policy)

The spec's canonical inverse policy translates to CoM, rotates, and mirrors — **but never rescales**. Input at `I ≠ 1` therefore breaks T1 and T3: two similarity-equivalent inputs encode to different points, and the round trip does not land on the section. The fix is the similarity transform the whole design is built on (`r → λr, t → λ^{3/2}t`), applied as **step 0 of canonicalisation**:

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

so `t_end` and all timing outputs are in canonical units; to express them in the user's original units, multiply by `I_in^{3/4}`. Encode surfaces a **`lookup_rescaled`** notice carrying `λ`, in the same family as `lookup_clamped`. (The simpler alternative — reject unless `I ≈ 1` — is admissible but strictly weaker; the rescale is the mathematically complete behaviour and matches the project's clamp-and-notify pattern.)

---

## Part 3 — Block inverses (spec formulae, confirmed, with their forward mates)

All three are exact closed forms on the canonical section. Quoted with the forward map each inverts, so an agent can verify the pairing:

**Mass** (forward: `μ_k = μ_max·tanh(z_raw)`, softmax over `(0, μ₁, μ₂)`):

```
μ₁ = log(m₁/m₀),   μ₂ = log(m₂/m₀),   z_μk = artanh(μ_k / μ_max)
```
clamp `|μ_k| ≤ (1−ε_μ)μ_max`, `ε_μ = 10⁻⁶`. Burrau shortcut at shape ν: `μ₁ = log(b/c)`, `μ₂ = log(a/c)`.

**Configuration** (forward: `α = (π/2)·σ(z_α)`, `β = π·σ(z_β)`): after canonicalisation,

```
α = atan2(‖λ̃‖, ‖ρ̃‖),   β = atan2(λ̃_y, λ̃_x) ∈ [0, π]
s_α = 2α/π,   s_β = β/π,   z = logit(s)
```
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

---

## Part 5 — Inverse by axis kind (completing the four kinds)

**Kind 1 — raw latent.** Read the component. Exact, unconditional.

**Kind 2 — derived-in-block.** Block inverse (Part 3) **plus the same residual convention as the forward axis**. The convention is part of the axis's identity: encoding "m₁ = 0.4" under the hold-`m₂:m₃` convention and under the hold-`m₂=m₃` convention give different z, and the round trip only closes if encode uses the axis's own convention. One convention, declared once, used both directions.

**Kind 3 — invariant.** Two directions, sharply different:

- *(a) state → chart coords:* trivial and always exact — evaluate the invariants `L_z(x)`, `E(x)` or `K(x)` directly. No solve.
- *(b) chart coords → latent (the lookup direction):* the fibre `{p : L_z = L*, K = K*}` at fixed `(m, config)` is generically **2-dimensional** (a linear constraint ∩ an ellipsoid in the 4D momentum space), so a canonical point must be chosen. **Here the spec contains two conflicting definitions**, now resolved:
  - The lookup section says "smallest latent-space norm."
  - The chart's own forward decode says: rigid rotation `v^(L) = (L_z/I)·J r` (the *minimal-kinetic-energy* realisation of `L_z` — a theorem: minimising `K = ½‖v‖²_m` subject to `⟨Jr, v⟩_m = L_z` gives exactly `v ∝ Jr`), plus `a·w` along the deterministic seeded direction field, `a = √(2(K* − L_z²/2I))`.
  
  These are **not the same functional** (kinetic-energy-minimal ≠ latent-norm-minimal after the logit pullback), and if encode ran an independent argmin the round trip would not close. **Resolution — the single-source-of-truth rule: encode reuses decode.** Entering an `(L_z, E)` pair means running the chart's own forward construction (i)–(iii) at the current frozen configuration, exactly as a pixel would, then recovering `z_mom` via the free-momentum inverse of the constructed `p`. The "smallest latent norm" phrasing survives only as the general fallback for case-3 charts with no canonical construction. This supersedes the lookup section's wording for invariant charts (→ pending-changes note).
  - Feasibility applies before construction: `|L_z| ≤ √(2I(E−U))`, `K ≥ L_z²/2I`; infeasible pairs → project / clamp / reject per the spec's ladder. Note at the canonical scale `I = 1`, so `ω = L_z` and `K_min = L_z²/2` — the constants simplify because encode already normalised scale (Part 2).
  - The seeded direction family can degenerate at special configurations (all seeds `< ε_w`); encode then fails with the same `DEGENERATE` label the pixel path would emit — consistent by construction, since it *is* the pixel path.

**Kind 4 — coupled curve.** Two directions:

- *(a) on-curve encode:* exact closed form — recover the acute angle from the canonical config, then `ν(θ) = sec θ − tan θ`; verify the Burrau mass coupling via `μ₁ = log(b/c)`, `μ₂ = log(a/c)`.
- *(b) is x on the curve at all?* Generic input isn't. Project: `ν* = argmin_ν dist(C(x), γ(ν))` (1D minimisation over a smooth curve — cheap and robust), report distance-to-curve. If the distance exceeds tolerance, the curve chart *cannot represent the point*: fall back to latent z (the spec's case 3), which is always available.

**Annotation axes.** Nothing to invert — no IC content by definition.

---

## Part 6 — The canonical inverse policy, completed

The spec's five steps, with the two additions (step 0; step 5's resolution) and the full-state rule made explicit:

0. **Rescale to `I = 1`** via the similarity transform (Part 2). Record `λ`; notice `lookup_rescaled`.
1. **Translate to CoM** — positions and the momentum frame (`p_i ← p_i − m_i P_tot/M`; total momentum zero).
2. **Rotate `ρ̃ → +x`** — the same rotation applied to **all positions and all momenta**. `ρ̃ = 0` → reject (coincident inner pair).
3. **Mirror if `λ̃_y < 0`** — reflect the **full state** (every `r_i` and every `p_i`) through the x-axis. Tie `|λ̃_y| < δ_λ = 10⁻¹²` → fixed deterministic choice (no-mirror), documented; T1 depends on it. Notice `lookup_mirrored` (the user's `L_z` sign has flipped frame).
4. **Invert into the active chart** where possible (Part 5, by axis kind), otherwise into latent z (always possible via Part 3).
5. **Fibre choice**: the chart's own forward construction is the canonical representative (encode reuses decode); smallest-latent-norm only as the documented fallback where no construction exists.
6. **Validate** — the spec's three layers in order (hypercube bounds → chart feasibility → decode sanity), with project / clamp / reject and every flag surfaced (`lookup_clamped`, `lookup_rescaled`, `lookup_mirrored`).

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

**New kernel debug mode: `ROUNDTRIP`** (joining `NORMAL / UV_PASSTHROUGH / DECODE_PASSTHROUGH` in the render contract's dispatch enum). Per pixel: `z → D → E → D → physical residual`, log-scaled. One glance certifies the block inverses, the numerics, and (on invariant charts) the encode-reuses-decode rule across the whole visible chart — the encode-side sibling of the invariant-chart-gradient view. Legitimate bright regions: clamp boundaries, feasibility edges, the mirror tie — *tagged expected*, not bugs.

---

*Encode is the quotient map onto the decode's section: constant on gauge orbits, right-inverse to decode, left-inverse modulo gauge. Rigid operations act on the whole phase-space state — positions and momenta together, always. Scale is canonicalised by the similarity rescale, and it is step zero. The fibre point on an invariant chart is whatever the chart's own decode constructs — encode never runs its own optimiser. Tolerances live in physical units; z-space residuals near saturation mean nothing. Everything the inverse discards, it reports.*
