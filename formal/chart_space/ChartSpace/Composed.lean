/-
§3b of `chart-space-model.md`: axes, composed axes and physical-quantity projections, and Theorem 10.
-/
import ChartSpace.TimeAxis

open scoped unitInterval

noncomputable section

namespace ChartSpace

/-- §3b: a composed chart `φ(u) = A(p(x₁(u₁), x₂(u₂)); h)`, for warped axes `x_i`, a pair map `p` and an assembly `A`
with held set `h`. The target `Y` is any space: `Z`, or §0's `S`. -/
def composed {H Y : Type*} (x₁ x₂ : Axis → ℝ) (pair : ℝ × ℝ → ℝ × ℝ) (A : ℝ × ℝ → H → Y) (h : H) : U → Y :=
  fun u => A (pair (x₁ (axisOf 0 u), x₂ (axisOf 1 u))) h

/-- **Theorem 10 (continuity of a composed chart)**, as stated: continuous warped axes, a continuous pair map and an
assembly continuous on the region the chart visits make `φ` a chart. As a chart, every theorem of §2–§6 applies to it
(over `Z`, those of the files before; over any path-connected target, those of `Signed.lean`). -/
theorem theorem10 {H Y : Type*} [TopologicalSpace Y] (x₁ x₂ : Axis → ℝ) (hx₁ : Continuous x₁) (hx₂ : Continuous x₂)
    (pair : ℝ × ℝ → ℝ × ℝ) (hpair : Continuous pair) (A : ℝ × ℝ → H → Y) (h : H) (K : Set (ℝ × ℝ))
    (hK : ∀ u, pair (x₁ (axisOf 0 u), x₂ (axisOf 1 u)) ∈ K) (hA : ContinuousOn (fun v => A v h) K) :
    Continuous (composed x₁ x₂ pair A h) :=
  hA.comp_continuous (hpair.comp ((hx₁.comp (continuous_axisOf 0)).prodMk (hx₂.comp (continuous_axisOf 1)))) hK

/-- **Theorem 10**, the seam clause, as stated: where `A` is continuous only on the region the pixels off a declared
seam set `seam` visit, `φ` is continuous on `U \ Σ`. -/
theorem theorem10_seams {H Y : Type*} [TopologicalSpace Y] (x₁ x₂ : Axis → ℝ) (hx₁ : Continuous x₁)
    (hx₂ : Continuous x₂) (pair : ℝ × ℝ → ℝ × ℝ) (hpair : Continuous pair) (A : ℝ × ℝ → H → Y) (h : H)
    (seam : Set U) (K : Set (ℝ × ℝ)) (hK : ∀ u ∉ seam, pair (x₁ (axisOf 0 u), x₂ (axisOf 1 u)) ∈ K)
    (hA : ContinuousOn (fun v => A v h) K) :
    ContinuousOn (composed x₁ x₂ pair A h) seamᶜ :=
  hA.comp (hpair.comp ((hx₁.comp (continuous_axisOf 0)).prodMk (hx₂.comp (continuous_axisOf 1)))).continuousOn
    fun u hu => hK u hu

/-- Theorem 10's "every theorem holds on `U \ Σ`", for Theorem 1: the straight-line morph is a path between charts on
any domain, so on `U \ Σ` too. -/
def lineMorphOn {X : Type*} [TopologicalSpace X] (φ ψ : C(X, Z)) : ContinuousMap.Homotopy φ ψ where
  toFun p := (1 - (p.1 : ℝ)) • φ p.2 + (p.1 : ℝ) • ψ p.2
  continuous_toFun := by fun_prop
  map_zero_left u := by simp
  map_one_left u := by simp

/-- **Finding F12**: "every theorem holds on `U \ Σ`" fails for zoom. For every zoom in, `0 < k < 1`, there is a
one-point seam and a pixel off it which zoom by `k` sends onto it, so zoom's frames are not maps on `U \ Σ`. The proof
uses only `k < 1`; `0 < k` restricts the statement to zooms. -/
theorem zoom_enters_seam {k : ℝ} (_hk0 : 0 < k) (hk1 : k < 1) :
    ∃ (seam : Set Plane) (u : Plane), u ∈ squareSet ∧ u ∉ seam ∧ k • u ∈ seam := by
  refine ⟨{k • (1 : Plane)}, 1, mem_squareSet.mpr fun _ => by simp, fun h => ?_, rfl⟩
  have := congrFun (Set.mem_singleton_iff.mp h) 0
  simp only [Pi.one_apply, Pi.smul_apply, smul_eq_mul, mul_one] at this
  linarith

/-- §3b **per-axis morph in quantity space**: `x₁ˢ = (1−s) x₁ + s y₁`, axis 2 fixed, then `A`. If `A` realises the
axis values (`q₂ (A v) = v₂` on the region, which is §3b's definition of the assembly, "find the latent point with
these axis values") and is continuous on the region the morph visits, this is a path from the chart of `x₁` to that of
`y₁`, and every frame keeps axis 2's value at every pixel exactly. No pair map: with one, the axis values are those
after it. -/
theorem axis_morph_keeps_meaning {Y : Type*} [TopologicalSpace Y] (A : ℝ × ℝ → Y) (q₂ : Y → ℝ) (K : Set (ℝ × ℝ))
    (hq : ∀ v ∈ K, q₂ (A v) = v.2) (hA : ContinuousOn A K) (x₁ y₁ x₂ : Axis → ℝ) (hx₁ : Continuous x₁)
    (hy₁ : Continuous y₁) (hx₂ : Continuous x₂)
    (hK : ∀ (s : I) (u : U), ((1 - (s : ℝ)) * x₁ (axisOf 0 u) + s * y₁ (axisOf 0 u), x₂ (axisOf 1 u)) ∈ K) :
    ∃ G : C(I × U, Y), (∀ u, G (0, u) = A (x₁ (axisOf 0 u), x₂ (axisOf 1 u))) ∧
      (∀ u, G (1, u) = A (y₁ (axisOf 0 u), x₂ (axisOf 1 u))) ∧
      ∀ s u, q₂ (G (s, u)) = x₂ (axisOf 1 u) := by
  have hc : Continuous fun p : I × U =>
      ((1 - (p.1 : ℝ)) * x₁ (axisOf 0 p.2) + p.1 * y₁ (axisOf 0 p.2), x₂ (axisOf 1 p.2)) := by
    have h0 := continuous_axisOf 0
    have h1 := continuous_axisOf 1
    refine Continuous.prodMk ?_ (hx₂.comp (h1.comp continuous_snd))
    exact ((continuous_const.sub (continuous_subtype_val.comp continuous_fst)).mul
      (hx₁.comp (h0.comp continuous_snd))).add
      ((continuous_subtype_val.comp continuous_fst).mul (hy₁.comp (h0.comp continuous_snd)))
  refine ⟨⟨_, hA.comp_continuous hc fun p => hK p.1 p.2⟩, fun u => by simp, fun u => by simp,
    fun s u => hq _ (hK s u)⟩

end ChartSpace
