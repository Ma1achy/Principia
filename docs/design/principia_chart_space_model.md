# The chart-space model: views, edits and paths

*A short formal model of Principia's navigation: charts, slicing, tilting, zoom, the lock, chart switching,
morphing and time-axis charts. Written 7 Nov 2026 to settle RQ-721 (chart paths) and to give Claude Code one
statement of how it works. Exact arithmetic throughout; §8 says what changes in floating point.*

---

## 1. Objects

**Latent space.** `Z = R^8`, the latent chart (`z_α, z_β, z_q0..z_q3, z_μ1, z_μ2`).

**Decoder.** `D : Z → Sys`, from a latent point to a canonical three-body system. Two facts from the existing
contracts are all the model needs:

- **(D1) Total:** `D` is defined on all of `Z` ("the decode accepting any z"; the links saturate rather than
  refuse).
- **(D2) Continuous:** `D` is a composition of continuous maps (sigmoid, tanh, trigonometric functions, the
  Jacobi map).

**Square.** `U = [-1,1]^2`, the view's pixel coordinates. The centre pixel is `u = 0`.

**Chart (as a view).** A continuous map `φ : U → Z`. A pixel `u` shows the system `D(φ(u))`. Every chart
Principia has fits this, including the curved and physical ones: those are `φ = E ∘ Φ`, where `Φ` builds a
physical state and `E` encodes it to `Z`. Masks (blank, uncounted pixels) are a subset `M_φ ⊆ U` carried
alongside. They don't affect anything below.

**Chart space.** `𝒞 = C(U, Z)`, all continuous maps `U → Z`.

**Affine charts.** `φ(u) = z₀ + Q u`, where `Q = [q₁ q₂]` is an 8×2 matrix of rank 2. Its column space is
the slice plane, and its scale is the zoom.

**Path (edit).** A continuous `H : [0,1] × U → Z`, written `H_s = H(s,·)`, from `H₀` to `H₁`. That is, a
**homotopy** of charts. Every frame `H_s` is itself a chart.

---

## 2. The central theorem: every edit is a path, and a path always exists

**Theorem 1.** For any two charts `φ, ψ ∈ 𝒞` there is a path from `φ` to `ψ`. In particular the
straight-line morph

    L(s,u) = (1−s)·φ(u) + s·ψ(u)

is one, and every frame decodes to a valid system.

*Proof.* `Z = R^8` is convex, so `L(s,u) ∈ Z` for all `s, u`. `L` is continuous, being built from continuous
maps by continuous operations, and `L₀ = φ`, `L₁ = ψ`. By (D1), `D(L(s,u))` exists for every `(s,u)`. By
(D2), `s ↦ D(L(s,u))` is continuous, so each pixel's system moves continuously. ∎

**Why this matters.** The morph is possible *only because* the decoder is total on all of `R^8` and the blend
is done in `Z`. Blending in physical space would not have this property: bodies pass through collisions, and
masses can leave the simplex. **Rule: always blend in `Z`, never in physical coordinates.**

**Corollary 1 (no frame is invalid).** A morph never produces a "not a system" pixel from the decoder side.
Blank pixels mid-morph come only from a chart's own mask (`M_φ`, `M_ψ`), interpolated as a display choice.

**Corollary 2 (held quantities are not conserved).** If `φ` holds energy at `E₀` and `ψ` holds `L_z` at `L₀`,
the frames in between generally hold neither. The set `{z : E(D(z)) = E₀}` isn't convex in `Z`, so the
straight line leaves it. This is a theorem, not a defect. Mid-morph frames must not claim a held value.

---

## 3. The existing operations are paths of named families

Each operation is a path `H` with a closed form. Here `s ∈ [0,1]`.

| Operation | Path | Family |
|---|---|---|
| Pan by `a` | `H_s(u) = φ(u + s a)` | reparametrisation |
| Zoom by factor `k > 0` about the centre | `H_s(u) = φ(k^s u)` | reparametrisation |
| Tilt of an affine chart by angle `θ` towards `t` | `H_s(u) = z₀ + R(sθ) Q u`, with `R` a rotation in the plane of `(q_i, t)` | latent plane geodesic |
| Tilt of a curved chart | the same rotation, applied to the chart's own parameters (R-473), then `Φ`, `E` | chart-parameter geodesic |
| Slice (excursion `δ`) | `H_s(u) = φ(u) + s δ` | latent translation |
| Chart switch (today) | a jump `φ → ψ`: the path's interior is skipped | — |
| Morph | any path, default `L` or a geodesic | general |

**Theorem 2 (tilt is a geodesic morph, exactly).** Let `φ(u) = z₀ + Q u` and `ψ = z₀ + Q' u` with
`Q' = R(θ) Q`. The tilt path *is* the morph along the latent plane geodesic between the two planes, and it
preserves the frame's area at every `s`:

    area(H_s) = sqrt(det(Q_sᵀ Q_s)) = sqrt(det(Qᵀ Q))    (since R(sθ) is orthogonal)

The straight-line morph `L` between the same two charts does **not** preserve area. Take `Q = [e₁ e₂]` and
`Q' = [e₂ e₁]`, which is the same plane with its axes swapped (a 90° rotation in the plane plus a mirror). At
`s = ½` both columns of `Q_s` are `(e₁+e₂)/2`, so it has rank 1 and **area 0**: the picture collapses to a line.

*Proof.* Rotations preserve inner products, so `Q_sᵀ Q_s = Qᵀ R(sθ)ᵀ R(sθ) Q = Qᵀ Q`. The swapped case is
direct computation. ∎

**Consequence.** "The morph is the general form" is true **with the right path family**. The general morph
is *any* homotopy, and tilt is the member of the geodesic family, exactly, not approximately. A straight-line
blend is a different member with the same endpoints. So the rule from 6 Nov, "rotate, never squash", isn't
a preference: it's the condition under which tilt is literally a special case.

**Theorem 3 (curved tilt is its own family; the 0.197 is not rounding).** For a curved chart, tilt rotates
the *chart's parameters*, so its frames are `E(Φ(R(sθ)·params))`. The latent geodesic between the endpoints
is a different path with the same ends. The two agree to first order in `θ` (same endpoints and same tangent
at `s = 0`), and differ at second order. That matches the Mac's measurement: 0.197 at `τ = 1.2`, falling as
`τ²`.

*Proof sketch.* Both paths are smooth in `θ` with the same value and first derivative at `θ = 0`, so their
difference is `O(θ²)`. ∎

**Consequence for RQ-721.** No tolerance between curved tilt and the latent morph is needed or meaningful:
they're different paths, both valid. The general morph must therefore include **three canonical families**:

1. the **latent line** `L`;
2. the **latent plane geodesic** (affine tilt);
3. the **chart-parameter geodesic** (curved tilt, and any morph between two charts of the same parametric
   family).

Each operation is *exactly* its family's formula. The per-operation test compares the implementation with its
own family's formula, never with a different family's.

---

## 3b. Axes, composed axes and physical-quantity projections

**Axis model.** A composed chart is `φ(u) = A(x₁(u₁), x₂(u₂); h)`. Here `x_i : [-1,1] → R` is axis *i*'s
quantity after its warp (linear, log, power): a primitive or a combination of primitives (E, L_z, an angle, a
ratio and so on). `h` is the held set (the coordinates not on an axis, plus any held constraint), and
`A : R² × H → Z` is the **assembly**: "find the latent point with these axis values and these held values".
Assembly is closed-form for registered inverses (Tier 2a), a numerical solve otherwise (Tier 2c, RQ-733), and
it optionally includes a pair map (polar, curved) applied to `(x₁, x₂)` first.

**Theorem 10 (continuity of a composed chart).** If each warped axis `x_i` is continuous, the pair map is
continuous, and `A` is continuous on the region the chart visits, then `φ` is a chart (continuous), and all of
§2–§6 applies to it unchanged. Where `A` is discontinuous (a numerical solve switching branch, the encode's
mirror deadband, a collision degeneracy), `φ` is continuous **away from a declared seam set** `Σ_φ ⊂ U`, and
every theorem holds on `U \ Σ_φ`.

*Proof.* A composition of continuous maps is continuous; restrict to the complement of `Σ_φ`. ∎

So "projection into energy, angular momentum, ..." is a chart like any other: it's the assembly `A` that
turns `(E, L_z)` back into a latent point. Its continuity is a property of `A`, and its seams are declared,
never hidden.

**Changing an axis.** Replacing axis 1's quantity `x₁` by `y₁`, or changing its warp or its combination,
gives a new chart `φ'`. By Theorem 1 there's always a path `φ → φ'`. Two canonical forms:

- **Per-axis morph, in quantity space:** `x₁ˢ = (1−s) x₁ + s y₁` (or a geodesic for angle-like
  quantities), axis 2 fixed, then `A`. This is a path whenever `A` is continuous along it. It keeps the
  *meaning* of the untouched axis exactly, because axis 2's value per pixel never changes.
- **Per-axis morph, in latent space:** `(1−s) φ + s φ'`. Always a path (Thm 1), but axis 2's displayed
  quantity may wander mid-morph, since latent blending doesn't respect what the axes mean.

The first is the default for "change one axis"; the second is the fallback where `A` isn't continuous along
the quantity path.

**Pair maps.** Flat to polar (or log-polar) is the path `P_s = (1−s)·id + s·polar` on `(x₁, x₂)`. It's
continuous except where polar itself is singular (the origin, and the angle's branch line), which are declared
seams.

**Continuous, not merely smooth.** Every theorem here proves *continuity*: small steps make small changes.
*Smoothness* (no kinks in the motion) also holds where every map involved is differentiable. `D` is (sigmoid,
tanh and trigonometric functions), and so are geodesics and linear blends. Warps with kinks, `|·|`, clamps
and numerical solves near a branch point are not. Easing curves should be at least C¹ for visually smooth
playback; that's a property of the chosen curve, not of the model.

---

## 4. Composition: Sequence, waypoints and easing

**Theorem 4 (concatenation).** If `H` goes from `φ` to `ψ` and `K` goes from `ψ` to `χ`, then

    (H·K)_s = H_{2s}       for s ≤ ½
            = K_{2s−1}     for s > ½

is a path from `φ` to `χ`.

*Proof.* The two pieces agree at `s = ½` (both equal `ψ`), so the glued map is continuous. ∎

**Keyframes are waypoints.** A Sequence with keyframes `φ₀, φ₁, …, φ_n` and segments `H¹ … Hⁿ` is the
concatenation `H¹ · … · Hⁿ`.

**Theorem 5 (easing changes timing, never the frames visited).** For a monotone continuous
`e : [0,1] → [0,1]` with `e(0) = 0` and `e(1) = 1`, the eased path `H_{e(s)}` visits exactly the same set of
charts in the same order, and has the same endpoints.

*Proof.* `e` is a surjection of `[0,1]` onto itself that preserves order. ∎

**Corollary.** Easing can never move a keyframe. That's the guarantee to state in Sequence's spec.

**Spline paths (waypoint or control charts)** are paths by Theorem 1 for latent splines (`Z` is convex, so
any spline in `Z` stays in `Z`). For the geodesic families, use rotation splines, as in Theorem 2, to keep
area.

---

## 5. The lock: based homotopy, excursions and the sliding anchor

**Lock state.** An anchor `a ∈ Z` and an excursion `δ ∈ Z`. The centre shows `a + δ`, meaning
`φ(0) = a + δ`.

**Theorem 6 (which operations keep the anchor).** With `δ = 0`:

- tilt (affine, about the centre) keeps `φ(0) = z₀ = a` for every `s`;
- zoom about the centre keeps `φ(0)` (`φ(k^s · 0) = φ(0)`);
- pan does **not** (`φ(s a') ≠ φ(0)` in general);
- slice changes `δ` (an excursion), by definition.

*Proof.* Substitute `u = 0`. ∎

This is the contract's "anchor-preserving" against "excursion" split, now proved rather than asserted.

**Snap-back.** Setting `δ ← 0` restores `φ(0) = a` exactly. Excursions compose additively
(`δ₁ + δ₂`), so any order of slice steps followed by snap-back returns the same state.

**Based morph.** A path `H` with `H_s(0) = a` for all `s` (a *based homotopy*).

**Theorem 7 (a based morph exists iff both ends contain the anchor).** If `φ(0) = ψ(0) = a`, then

    B(s,u) = a + (1−s)(φ(u) − a) + s(ψ(u) − a)

is a based path. Conversely, if `ψ` can't be centred on `a` (no chart of `ψ`'s family has `a` at its centre),
no based path ends in that family.

*Proof.* At `u = 0` both differences vanish, so `B(s,0) = a`. The converse is immediate, since the endpoint
must have `ψ(0) = a`. ∎

**The constrained-chart case.** Some families only contain points of a subset `C_ψ ⊂ Z`. For example, a rest
chart requires `q = 0`, and region D or Burrau fix the masses and velocities. If `a ∉ C_ψ`, Theorem 7 forbids
a based morph. That's CC's "the anchor may not exist in chart B".

**Theorem 8 (the sliding anchor, the continuous fix).** Let `π : Z → C_ψ` be a continuous retraction (for
coordinate constraints, set the constrained coordinates to `C_ψ`'s values). Define the anchor path
`a_s = (1−s) a + s π(a)`, and

    S(s,u) = a_s + (1−s)(φ(u) − a) + s(ψ_{π(a)}(u) − π(a))

where `ψ_{π(a)}` is `ψ` re-centred on `π(a)`. Then `S` is a path from `φ` to `ψ_{π(a)}`, and its centre
moves continuously from `a` to `π(a)`. No jump.

*Proof.* `S` is continuous as a composition of continuous maps. `S(0,·) = φ` and `S(1,·) = ψ_{π(a)}`. At
`u = 0`, `S(s,0) = a_s`, which is continuous in `s`. ∎

When the constraint is a set of latent coordinates, `π` is "keep everything else, set those coordinates", so
the pinned system keeps its shape and has its velocity eased to rest. That's the RQ-721 rider.

---

## 6. Time as an axis

A time chart is `τ : U → Z × [0, T]`, with `τ(u) = (φ(u₁), t(u₂))`, and a pixel shows the trajectory of
`D(φ(u₁))` at time `t(u₂)`. An IC chart embeds as `ι(φ) = (φ, t_play)` (constant time).

**Theorem 9 (time-to-time morphs exist; time-to-IC is a choice, not a necessity).**

- Between time charts `(φ, t)` and `(ψ, t)` with the same time axis, `(L_s, t)` is a path: the time axis is
  fixed and the IC axis morphs.
- Between a time chart and `ι(ψ)`, a path also exists, for example the "hybrid"
  `((1−s)φ + sψ, (1−s) t(u₂) + s t_play)`, because `Z × [0,T]` is convex.

So the 6 Nov decision to **cut** between them is a *product choice* (hybrid frames are hard to read and
costly), not a mathematical impossibility. The design should say so.

---

## 6b. Initial-condition, dynamical and shared quantities

Let `F_t : Sys → Sys` be the flow (the system after time `t`), defined where the trajectory survives to `t`.
There are three kinds of quantity. The same function can belong to more than one.

1. **State functions** `q(x)`, such as shape-sphere position, interior angles, momenta and K. Evaluated at
   `t = 0` they're IC axes. Evaluated at `t` they're **dynamical** values, `q(F_t(x))`. Shape-sphere position
   is the example: one function, read at two times.
2. **Conserved quantities**, `E` and `L_z` (plus anything fixed by the gauge). `q(F_t(x)) = q(x)` for all
   `t`, so their IC value *is* their dynamical value: these are exactly the overlap. That's why an `(L_z, E)`
   chart means the same thing at every playhead. Approximately conserved quantities (a hierarchical binary's
   orbital elements, the Hill-stability quantity `L_z²E`) are approximately in the overlap, and should be
   labelled as approximate.
3. **Trajectory functionals**, such as minimum separation, lifetime `t_end`, outcome class, Lyapunov exponent
   and the syzygy word. These depend on the whole trajectory and are many-to-one, with no inverse even
   locally. **They can never be axes**, only colourings and plots of results.

**Theorem 11 (a state function at time `T` can be an axis).** A chart whose axes are state functions
evaluated at time `T` (for example "shape-sphere position at `T = 5`") is the IC chart

    φ_T(u) = Enc( F_{−T}( D(ψ(u)) ) ),

where `ψ` is an ordinary chart describing the state *at time `T`*. Because Newtonian dynamics is
time-reversible, `F_{−T} = R ∘ F_T ∘ R`, with `R` flipping all momenta. So the same forward integrator
computes it: flip the momenta, integrate forward by `T`, flip back.

*Proof.* Reversibility of the equations of motion under `t → −t, p → −p`. ∎

**What that buys:** "final-condition" charts. Pick where the system *is* at time `T`, on the shape sphere or
in `(L_z, K)` or any state axes, and see which starts lead there and what happened before and after. All of
§2–§6 applies, because `φ_T` is a chart.

**Its limits, stated:**
- `φ_T` is continuous only where the backward trajectory survives `[0, T]` without a triple collision or an
  escape. Those pixels are masked and declared, never dropped.
- Chaos is symmetric in time: states that are close at `T` can come from starts far apart. So `φ_T` can be
  wildly folded even when `ψ` is smooth. That's real structure, not error.
- Its cost is one extra integration of length `T` per pixel before the usual forward run.
- `T = 0` recovers the ordinary chart. For conserved axes `φ_T` is independent of `T` in those coordinates,
  which makes a built-in check.
- Trajectory functionals stay excluded at any `T`.

---

## 7. What the model settles

| Claim | Status |
|---|---|
| Any chart can morph into any other | **Proved** (Thm 1): requires blending in `Z` and the decoder being total |
| Composed and physical-quantity axes (E, L_z, angles, ratios) are charts like any other | **Proved** (Thm 10), continuous away from declared seams of the assembly |
| Changing one axis can keep the other's meaning exactly | **Proved** (§3b): per-axis morph in quantity space |
| Conserved quantities are both IC and dynamical | **Proved** (§6b): E and L_z are the exact overlap |
| A state function at a later time can be an axis | **Proved** (Thm 11), via time reversal: flip momenta, integrate forward, flip back |
| Trajectory functionals (lifetime, outcome, Lyapunov, minimum distance) as axes | **Impossible**: many-to-one; colourings and plots only |
| Continuous vs smooth | Continuity proved everywhere; smoothness wherever the maps are differentiable (§3b) |
| Tilt, zoom, pan and slice are special cases of the path model | **Proved** (Thm 2, 6; table in §3), each as an exact member of a named family |
| "Never squash" is necessary for tilt to be a special case | **Proved** (Thm 2): the straight-line blend collapses swapped planes to a line |
| Curved tilt vs latent morph tolerance | **Not a tolerance**: different families (Thm 3). Test each operation against its own family |
| Easing can't move keyframes | **Proved** (Thm 5) |
| Snap-back is exact, and excursions compose | **Proved** (§5) |
| A based morph exists iff both ends contain the anchor | **Proved** (Thm 7) |
| No jumps under the lock | **Proved possible** (Thm 8, sliding anchor) |
| Held values survive a morph | **False in general** (Cor. 2): frames must not claim them |
| Time ↔ IC morph impossible | **False**: a cut is a choice (Thm 9) |

---

## 8. Floating point: what the proofs don't cover

Everything above is exact. In f32 and f64:

- **Exact family membership becomes "same formula, same evaluation order".** Tilt is bitwise its family's
  formula only if the code *is* that formula. So "each operation is evaluated by its own code" (the RQ-721
  amendment) is the floating-point form of "each operation is its family's member". The test is: the
  implementation of family *k* reproduces the *family k formula*. Bitwise where it's the same code path;
  otherwise within a measured per-operation tolerance (R-71). Never compare across families.
- **The anchor's bits.** Theorems 6–8 hold exactly for `φ(0)`. In floating point, a re-centred nonlinear chart
  doesn't return the anchor's bits (`E(D(z)) ≠ z` bitwise: the Mac measured 0 of 20,000 samples). So the
  anchor is carried as **stored words**, which is §17.8's design. The model says *what* must be constant, and
  stored words are *how*.
- **Continuity becomes "small step, small change except at declared seams".** The encode `E` has genuine
  discontinuities: the mirror deadband (`λ̃_y = 0`) and collision degeneracies. These affect only how `φ` is
  *built* for a nonlinear chart, never the morph itself, which is continuous in `Z` (Thm 1). Seams belong to
  charts, not to paths.

---

## 9. One-paragraph version for the agents

A view is a continuous map from the unit square into the 8D latent space, and every edit is a continuous path
of such maps: a homotopy. A path always exists, because the latent space is convex and the decoder is defined
everywhere on it; blend in latent space, never in physical space. Pan, zoom, tilt and slice are exact members
of named path families: reparametrisation, the latent plane geodesic, the chart-parameter geodesic and latent
translation. Morph is the general case, with the latent line and the geodesics as canonical families, and
each operation is tested against its own family, never another's. Composed and physical-quantity axes are
charts through an assembly map, continuous away from declared seams; changing one axis morphs in quantity
space, so the other axis keeps its meaning. Easing reparametrises time and can't move
keyframes. Under the lock, a morph keeps the anchor exactly when both charts contain it. When the target
chart can't, the anchor slides continuously to the target's nearest admissible point, so there are no jumps.
Held quantities aren't preserved mid-morph. Time-to-IC morphs are possible but cut by choice. In floating
point, family membership means "same formula, same evaluation order", the anchor is carried as stored words,
and seams belong to charts, not paths.
