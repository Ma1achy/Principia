/-
§3 of `chart-space-model.md`: the operations as paths, and whether each is well defined on the domains §1 gives it.

Slice is. Pan is not, for any `a ≠ 0`; zoom is not, for any `k > 1`. Both evaluate `φ` outside `U`, where a chart has no
value. What holds: zoom for `0 < k ≤ 1` on a chart itself, and pan and zoom on a chart that comes with a continuous
extension to the plane. Such an extension exists, by Tietze, but it isn't unique, and pan's frames depend on it.
-/
import ChartSpace.Morph

open scoped unitInterval

noncomputable section

namespace ChartSpace

/-! ### Slice: well defined -/

/-- §3 **Slice (excursion `δ`)**: `H_s(u) = φ(u) + s δ`, the end chart. -/
def sliced (φ : Chart) (δ : Z) : Chart := ⟨fun u => φ u + δ, by fun_prop⟩

@[simp] theorem sliced_apply (φ : Chart) (δ : Z) (u : U) : sliced φ δ u = φ u + δ := rfl

/-- §3 **Slice**, as a path. -/
def slicePath (φ : Chart) (δ : Z) : EditPath φ (sliced φ δ) where
  toFun p := φ p.2 + (p.1 : ℝ) • δ
  continuous_toFun := by fun_prop
  map_zero_left u := by simp
  map_one_left u := by simp

@[simp] theorem slicePath_apply (φ : Chart) (δ : Z) (s : I) (u : U) :
    slicePath φ δ (s, u) = φ u + (s : ℝ) • δ := rfl

/-! ### Pan and zoom leave the square (finding F1) -/

/-- §3 **Pan** is not well defined on `𝒞` (finding F1). For every `a ≠ 0` and every `s > 0`, some pixel `u ∈ U` has
`u + s a ∉ U`, so `φ(u + s a)` is not a value of `φ : U → Z`. -/
theorem pan_leaves_square {a : Plane} (ha : a ≠ 0) {s : ℝ} (hs : 0 < s) :
    ∃ u ∈ squareSet, u + s • a ∉ squareSet := by
  obtain ⟨i, hi⟩ : ∃ i, a i ≠ 0 := by
    by_contra h
    push Not at h
    exact ha (funext h)
  rcases lt_or_gt_of_ne hi with hneg | hpos
  · refine ⟨Function.update 0 i (-1), mem_squareSet.mpr fun j => ?_, fun h => ?_⟩
    · by_cases hj : j = i
      · subst hj; simp
      · simp [Function.update_of_ne hj]
    · have := (mem_squareSet.mp h i).1
      simp only [Pi.add_apply, Function.update_self, Pi.smul_apply, smul_eq_mul] at this
      nlinarith
  · refine ⟨Function.update 0 i 1, mem_squareSet.mpr fun j => ?_, fun h => ?_⟩
    · by_cases hj : j = i
      · subst hj; simp
      · simp [Function.update_of_ne hj]
    · have := (mem_squareSet.mp h i).2
      simp only [Pi.add_apply, Function.update_self, Pi.smul_apply, smul_eq_mul] at this
      nlinarith

/-- §3 **Zoom** with `k > 1` is not well defined on `𝒞` (finding F1). For every `s > 0`, the corner pixel has
`k^s u ∉ U`. -/
theorem zoom_leaves_square {k : ℝ} (hk : 1 < k) {s : ℝ} (hs : 0 < s) :
    ∃ u ∈ squareSet, k ^ s • u ∉ squareSet := by
  refine ⟨1, mem_squareSet.mpr fun j => by simp, fun h => ?_⟩
  have := (mem_squareSet.mp h 0).2
  simp only [Pi.smul_apply, Pi.one_apply, smul_eq_mul, mul_one] at this
  exact absurd (Real.one_lt_rpow hk hs) (not_lt.mpr this)

/-- Zoom with `0 < k ≤ 1` stays in the square. -/
theorem zoom_stays_in_square {k : ℝ} (hk0 : 0 < k) (hk1 : k ≤ 1) (s : I) (u : U) :
    k ^ (s : ℝ) • (u : Plane) ∈ squareSet := by
  have hle : k ^ (s : ℝ) ≤ 1 := Real.rpow_le_one hk0.le hk1 s.2.1
  have hpos : 0 < k ^ (s : ℝ) := Real.rpow_pos_of_pos hk0 _
  refine mem_squareSet.mpr fun j => ?_
  have hu := mem_squareSet.mp u.2 j
  simp only [Pi.smul_apply, smul_eq_mul, Set.mem_Icc]
  constructor <;> nlinarith [hu.1, hu.2]

/-- §3 **Zoom** for `0 < k ≤ 1`, on a chart itself: the only case the document's domains allow. -/
def zoomInEnd (φ : Chart) (k : ℝ) (hk0 : 0 < k) (hk1 : k ≤ 1) : Chart :=
  ⟨fun u => φ ⟨k • (u : Plane), by simpa using zoom_stays_in_square hk0 hk1 1 u⟩, by fun_prop⟩

/-- §3 **Zoom** for `0 < k ≤ 1`, as a path on a chart itself. -/
def zoomInPath (φ : Chart) (k : ℝ) (hk0 : 0 < k) (hk1 : k ≤ 1) : EditPath φ (zoomInEnd φ k hk0 hk1) where
  toFun p := φ ⟨k ^ (p.1 : ℝ) • (p.2 : Plane), zoom_stays_in_square hk0 hk1 p.1 p.2⟩
  continuous_toFun := by
    refine φ.continuous.comp (Continuous.subtype_mk ?_ _)
    exact ((continuous_const.rpow continuous_subtype_val.fst' fun _ => Or.inl hk0.ne').smul
      continuous_subtype_val.snd')
  map_zero_left u := by simp
  map_one_left u := by simp [zoomInEnd]

/-- Theorem 6, the zoom clause on a chart itself: `φ(k^s · 0) = φ(0)`. -/
theorem zoomInPath_centre (φ : Chart) (k : ℝ) (hk0 : 0 < k) (hk1 : k ≤ 1) (s : I) :
    zoomInPath φ k hk0 hk1 (s, centre) = φ centre := by
  change φ _ = φ centre
  congr 1
  apply Subtype.ext
  simp [centre]

/-! ### Extended charts: the hypothesis pan and zoom need -/

/-- **Added hypothesis A1** (finding F1): a chart carried with a continuous extension to the plane. Every operation
in §3 that evaluates off the square is stated for one of these. -/
abbrev ExtChart : Type := C(Plane, Z)

/-- The chart an extended chart restricts to. -/
def ExtChart.view (Φ : ExtChart) : Chart := ⟨fun u => Φ u, by fun_prop⟩

@[simp] theorem ExtChart.view_apply (Φ : ExtChart) (u : U) : Φ.view u = Φ u := rfl

/-- The end of a pan by `a`. -/
def ExtChart.panned (Φ : ExtChart) (a : Plane) : ExtChart := ⟨fun u => Φ (u + a), by fun_prop⟩

/-- The end of a zoom by `k`. -/
def ExtChart.zoomed (Φ : ExtChart) (k : ℝ) : ExtChart := ⟨fun u => Φ (k • u), by fun_prop⟩

/-- §3 **Pan by `a`**: `H_s(u) = φ(u + s a)`, with hypothesis A1. -/
def panPath (Φ : ExtChart) (a : Plane) : EditPath Φ.view (Φ.panned a).view where
  toFun p := Φ ((p.2 : Plane) + (p.1 : ℝ) • a)
  continuous_toFun := by fun_prop
  map_zero_left u := by simp
  map_one_left u := by simp [ExtChart.panned]

/-- §3 **Zoom by `k > 0` about the centre**: `H_s(u) = φ(k^s u)`, with hypothesis A1. -/
def zoomPath (Φ : ExtChart) (k : ℝ) (hk : 0 < k) : EditPath Φ.view (Φ.zoomed k).view where
  toFun p := Φ (k ^ (p.1 : ℝ) • (p.2 : Plane))
  continuous_toFun := by
    refine Φ.continuous.comp ?_
    exact ((continuous_const.rpow continuous_subtype_val.fst' fun _ => Or.inl hk.ne').smul
      continuous_subtype_val.snd')
  map_zero_left u := by simp
  map_one_left u := by simp [ExtChart.zoomed]

/-- A1 can always be met: every chart has a continuous extension to the plane (Tietze). -/
theorem exists_extension (φ : Chart) : ∃ Φ : ExtChart, Φ.view = φ := by
  obtain ⟨g, hg⟩ := ContinuousMap.exists_restrict_eq isClosed_squareSet φ
  refine ⟨g, ?_⟩
  ext1 u
  exact congrArg (fun f : C(U, Z) => f u) hg

/-- **Pan isn't a function of the chart** (finding F1). Every chart has two extensions which disagree on some frame of
the pan by any `a ≠ 0`. So A1's extension is data: it can't be recovered from `φ ∈ 𝒞`. -/
theorem pan_depends_on_extension (φ : Chart) {a : Plane} (ha : a ≠ 0) :
    ∃ Φ₁ Φ₂ : ExtChart, Φ₁.view = φ ∧ Φ₂.view = φ ∧
      ∃ (s : I) (u : U), panPath Φ₁ a (s, u) ≠ panPath Φ₂ a (s, u) := by
  obtain ⟨Φ₁, h₁⟩ := exists_extension φ
  obtain ⟨u, hu, hout⟩ := pan_leaves_square ha one_pos
  set e : Z := EuclideanSpace.single 0 (1 : ℝ) with he
  have he0 : e ≠ 0 := by simp [he]
  let Φ₂ : ExtChart :=
    ⟨fun v => Φ₁ v + Metric.infDist v squareSet • e, by
      have : Continuous fun v : Plane => Metric.infDist v squareSet :=
        Metric.continuous_infDist_pt _
      fun_prop⟩
  have hne : squareSet.Nonempty := ⟨0, zero_mem_squareSet⟩
  have hview : Φ₂.view = φ := by
    rw [← h₁]
    ext1 v
    have : Metric.infDist (v : Plane) squareSet = 0 := Metric.infDist_zero_of_mem v.2
    simp [Φ₂, ExtChart.view, this]
  refine ⟨Φ₁, Φ₂, h₁, hview, 1, ⟨u, hu⟩, fun h => ?_⟩
  have hpos : Metric.infDist (u + (1 : ℝ) • a) squareSet ≠ 0 := by
    intro h0
    exact hout ((isClosed_squareSet.mem_iff_infDist_zero hne).mpr h0)
  have h' : Φ₁ (u + (1 : ℝ) • a) = Φ₁ (u + (1 : ℝ) • a) + Metric.infDist (u + (1 : ℝ) • a) squareSet • e := h
  have : Metric.infDist (u + (1 : ℝ) • a) squareSet • e = 0 := by
    simpa using h'.symm
  exact (smul_ne_zero hpos he0) this

/-! ### Affine charts extend canonically -/

/-- An affine chart's own formula is its extension, which is why pan and zoom are unproblematic on affine charts. -/
def affineExt (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ) : ExtChart :=
  ⟨fun u => z₀ + matVec Q u, continuous_const.add (continuous_matVec Q)⟩

@[simp] theorem affineExt_apply (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ) (u : Plane) :
    affineExt z₀ Q u = z₀ + matVec Q u := rfl

theorem affineExt_view (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ) : (affineExt z₀ Q).view = affine z₀ Q := rfl

end ChartSpace
