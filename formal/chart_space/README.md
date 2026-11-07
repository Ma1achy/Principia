# The chart-space model in Lean 4

A machine-checked formalisation of `chart-space-model.md` (7 Nov 2026, held outside the repository at
`orch/charts/`), by the owner's order of 7 Nov 2026: Theorems 1, 2, 4, 5, 6, 7, 8 and 9, over
`Z = EuclideanSpace ℝ (Fin 8)` and the compact square `U = [-1,1]^2`, with `D : Z → Sys` for an abstract topological
space `Sys`, assumed continuous, and total because it is a function. Floating point (§8) is out of it, and so is
Theorem 3.

This directory is outside the Rust workspace. It is a measurement of the model, not a contract: nothing in Principia
builds on it, and its workflow is never a required check.

## Build

Lean is pinned in `lean-toolchain`, and Mathlib and its dependencies in `lake-manifest.json`, at Mathlib's `v4.35.0`
commit. Mathlib is fetched, never vendored. With [elan](https://github.com/leanprover/elan) installed:

    lake exe cache get    # prebuilt Mathlib
    ./check.sh            # build, refuse escape hatches, print and check every main theorem's axioms

`check.sh` fails on `sorry`, `admit`, `native_decide`, a new axiom or a new opaque constant in the source, and on any
theorem listed in `ChartSpace/Axioms.lean` depending on an axiom beyond `propext`, `Classical.choice` and `Quot.sound`.
`.github/workflows/formal-chart-space.yml` runs it nightly and on manual dispatch.

## Files

| File | Document | Contents |
|---|---|---|
| `ChartSpace/Basic.lean` | §1 | `Z`, `U`, `Chart`, `EditPath` (Mathlib's `Homotopy`), `frame`, affine charts |
| `ChartSpace/Morph.lean` | §2 | Theorem 1, Corollary 1, charts with seams |
| `ChartSpace/Ops.lean` | §3 | slice, pan, zoom; what is and isn't well defined |
| `ChartSpace/Tilt.lean` | §2, §3 | the rotation, area, Theorem 2 and its negative parts |
| `ChartSpace/Compose.lean` | §4 | Theorems 4 and 5, Sequences, the Corollary |
| `ChartSpace/Lock.lean` | §5 | lock state, snap-back, Theorems 6, 7 and 8 |
| `ChartSpace/TimeAxis.lean` | §6 | Theorem 9 |
| `ChartSpace/Axioms.lean` | | `#check_axioms` on every main theorem; not part of the library |

## Result

| Document | Status | Lean |
|---|---|---|
| Theorem 1 | proved as stated | `theorem1` |
| Corollary 1 | proved; `D` is a function, so it has no further content | `corollary1` |
| Theorem 2, area and rank | proved as stated | `theorem2_tilt` |
| Theorem 2, "is the geodesic" | proved for §3's family; as a statement about planes it needs A5 | `theorem2_tilt`, `tilt_constant_speed`, F3 |
| Theorem 2, swapped axes | the computation is proved as stated; the pair isn't a tilt (F2) | `theorem2_swap`, `swap_is_not_a_tilt` |
| Theorem 4 | proved as stated | `theorem4` |
| Theorem 5 | proved as stated, in the strong sense below | `theorem5` |
| §4 Corollary | proved | `easing_keeps_keyframes`, `easing_segments_keeps_midpoint` |
| Theorem 6, tilt and slice | proved as stated | `theorem6_tilt`, `theorem6_slice` |
| Theorem 6, zoom | proved for `k ≤ 1`; A1 for `k > 1` | `zoomInPath_centre`, `theorem6_zoom` |
| Theorem 6, pan | counterexample proved, strengthened to every affine chart; A1 for the formula | `theorem6_pan_counterexample`, `theorem6_pan_affine` |
| §5 snap-back, additive excursions | proved, with A7 | `snap_restores_anchor`, `slice_slice`, `slices_any_order`, `snap_after_slices` |
| Theorem 7, forward, iff, converse | proved as stated, with "contain" read as "at the centre" (F10) | `theorem7_forward`, `theorem7_iff`, `theorem7_converse` |
| Theorem 8 | proved with A2 and A3; `S(s,0) = a_s` is false without them | `theorem8`, `theorem8_pointwise`, `sliding_centre_iff` |
| Theorem 9 | proved once §6's types are fixed (A6) | `theorem9_time_to_time`, `theorem9_time_to_IC` |
| Theorem 3, §8 | out of the order | |
| Corollary 2 | not formalised: it is a claim about the concrete `D` and `E`, which `D` abstract can't state | |

"The same set of charts in the same order" in Theorem 5 is stated without reduction: the ranges are equal, and for every
`n` and every list of `n` charts, repeats allowed, one path visits the list in order iff the other does
(`VisitsInOrder`). A path may revisit a chart, so order isn't a relation on charts alone.

## Hypotheses the document doesn't state

Each is marked in the source by its name.

- **A1** (`ExtChart`): pan, and zoom with `k > 1`, need a chart with a continuous extension to the plane.
- **A2** (`theorem8_pointwise`, `hφ`): Theorem 8 needs `φ(0) = a`, that is, a lock with `δ = 0`.
- **A3** (`theorem8_pointwise`, `hψ`): Theorem 8 needs `ψ_{π(a)}(0) = π(a)`, and a definition of re-centring.
- **A4** (`exists_plane_of_tilt`): `q_i` and `t` must be linearly independent for "the plane of `(q_i, t)`" to exist.
- **A5** (`tilt_constant_speed`): `t` orthogonal to the frame's columns, for a tilt to be a geodesic of planes.
- **A6** (`timeChart`): in §6, `φ` is a curve `[-1,1] → Z`, not a chart of §1, and `t_play ∈ [0,T]`.
- **A7** (`Locked.snap`): snap-back also translates the chart by `−δ`. Setting `δ ← 0` alone breaks `φ(0) = a + δ`.

## Findings

- **F1. Pan and zoom-out are not well defined on `𝒞` (§1 against §3).** `φ(u + s a)` and `φ(k^s u)` evaluate `φ` outside
  `U`, where it has no value: for every `a ≠ 0` and `s > 0`, and for every `k > 1` and `s > 0`, some pixel leaves
  (`pan_leaves_square`, `zoom_leaves_square`). Every chart has a continuous extension (`exists_extension`, Tietze), but
  not a unique one, and pan's frames depend on it (`pan_depends_on_extension`), so the extension is data. Affine charts
  have a canonical one (`affineExt`). The likely repair is `𝒞 = C(R², Z)` with `U` as the viewport; every theorem here
  survives it.
- **F2. Theorem 2's example doesn't meet its hypothesis (§2 against §3).** `[e₂ e₁]` is not `R(θ) [e₁ e₂]` for any rotation
  in a plane containing a column (`swap_is_not_a_tilt`), so "the same two charts" is false under §3's `R`. It is
  `R(π) Q` in the plane of `(e₁ − e₂, e₃)` (`swap_is_rotation_by_pi`), a path that keeps the area by leaving the plane
  through `e₃`. The conclusion survives, and strengthens, for true tilts: `det(Q_sᵀ Q_s) = 1 − 2s(1−s)(1 − cos θ)`
  (`det_blend_tilt`), so the line loses area at every `0 < s < 1` unless `cos θ = 1`, and collapses at `θ = π`, `s = ½`
  (`line_loses_area`). §7's "never squash is necessary" should cite this.
- **F3. "Geodesic" is undefined, and the plane reading needs A5.** The document defines the family only by the formula
  (`IsLatentPlaneGeodesic`). With `t` orthogonal to the columns, `det(Q_aᵀ Q_b) = cos(b − a)`: the plane moves at constant
  speed (`tilt_constant_speed`). With `t = q₂` the frames turn and the plane doesn't move (`tilt_inside_plane`), so §3's
  family holds paths which aren't "between the two planes" in any sense. In between, no constant speed fits
  (`det_frames`, `tilt_frames_not_constant_speed`). Mathlib has no Grassmannian metric, so this is stated through the
  product of principal cosines, and no claim about Riemannian geodesics is proved.
- **F4. Theorem 8 omits two hypotheses, and `S(s,0) = a_s` is false without them (A2, A3).** `sliding_centre_iff` is
  the exact condition. With `δ ≠ 0` the centre starts at `a + δ`, not at `a`. `ψ_{π(a)}` is never defined; `recentre` is a
  definition that works, and stays in `C` (`recentre_spec`).
- **F5. Theorem 8 uses nothing of `π` but the point `π(a)`, and §9 asks more than it needs.** `slidingMorph` takes any
  `b`. Continuity and retraction matter only for keeping the re-centred chart in `C`. A continuous retraction exists only
  onto a closed connected set (`IsRetraction.isClosed`, `IsRetraction.isPreconnected`), so §9's "nearest admissible
  point" isn't available for every `C_ψ`. Coordinate constraints are fine (`pin_isRetraction`).
- **F6. §6 is ill-typed against §1 (A6).** `φ(u₁)` applies a function of two coordinates to one, and `(φ, t_play)` then
  uses a chart of §1. `curveChart` is the bridge. It ignores `u₂`, so a time chart's IC axis is never an affine chart.
- **F7. §1 and §8 disagree about continuity.** §1 says every chart is continuous, `E ∘ Φ` included; §8 says `E` has
  genuine discontinuities, which aren't a floating-point effect. A chart with a seam isn't a frame of any path of §1
  (`seam_is_not_a_frame`). §8's "the morph is continuous in `Z`" holds pixelwise in `s` (`blend_continuous_in_s`), which is
  weaker than Theorem 1. Either `𝒞` must allow seams or those charts are outside it.
- **F8. `H¹ · … · Hⁿ` is under-specified.** Concatenation isn't associative as a path (`concat_not_associative`), so the
  bracketing fixes each keyframe's time, though not which keyframes are passed, or their order (`sequence_concat`).
- **F9. `B` and `S` are `L`.** `basedMorph_eq_line` and `slidingMorph_eq_line`: the based morph and the sliding anchor are
  the straight-line morph, to the original and to the re-centred chart. They add no path family, and Theorem 7 and 8's
  whole content is what happens at `u = 0`.
- **F10. "Contain" means "at the centre".** Theorem 7's title and §9 say a chart contains the anchor; the theorem needs
  `ψ(0) = a`. A chart showing `a` off-centre has no based morph (`contains_is_not_enough`).

Smaller notes: Theorem 1's path needs neither (D1) nor (D2), only the claim about decoding does (`lineMorph` has no `D`),
so "only because the decoder is total" is about the decoded frames, not the path. Theorem 9 is about paths in
`Z × [0,T]` only: what a pixel shows is a trajectory of `D(z)` at a time, which this model doesn't define and which needs
a flow that can collide, so no continuity of pixels is claimed for time charts.
