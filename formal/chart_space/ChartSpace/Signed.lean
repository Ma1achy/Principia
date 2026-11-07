/-
§0 of `chart-space-model.md` (revised 7 Nov 2026), where it changes a theorem: charts map into the signed state space
`S`, the product of each block's own space (masses on the simplex, configuration on the shape sphere, momenta in
`R^4`), and paths blend block by block in each block's own geometry. The results over `Z` in the other files stand;
this file adds the `§0` form.

The first section holds for charts into *any* space: `U` is a square, so every chart contracts to its centre. That
gives Theorem 1's existence, Theorem 7 and Theorem 8 for every path-connected target, `S` included, with no formula.
-/
import ChartSpace.Lock

open scoped unitInterval InnerProductSpace

noncomputable section

namespace ChartSpace

/-! ### Charts into any space -/

theorem smul_mem_squareSet {c : ℝ} (h0 : 0 ≤ c) (h1 : c ≤ 1) {u : Plane} (hu : u ∈ squareSet) : c • u ∈ squareSet := by
  refine mem_squareSet.mpr fun j => ?_
  have := mem_squareSet.mp hu j
  simp only [Pi.smul_apply, smul_eq_mul, Set.mem_Icc]
  constructor <;> nlinarith [this.1, this.2]

/-- Every chart, into any space, contracts to its centre: `(s,u) ↦ φ((1−s) u)`, which keeps the centre fixed. -/
def contract {X : Type*} [TopologicalSpace X] (φ : C(U, X)) : ContinuousMap.Homotopy φ (ContinuousMap.const U (φ centre)) where
  toFun p := φ ⟨(1 - (p.1 : ℝ)) • (p.2 : Plane),
    smul_mem_squareSet (by have := p.1.2.2; linarith) (by have := p.1.2.1; linarith) p.2.2⟩
  continuous_toFun := by
    refine φ.continuous.comp (Continuous.subtype_mk ?_ _)
    fun_prop
  map_zero_left u := by simp
  map_one_left u := by
    simp only [Set.Icc.coe_one, sub_self, zero_smul, ContinuousMap.const_apply]
    rfl

theorem contract_centre {X : Type*} [TopologicalSpace X] (φ : C(U, X)) (s : I) : contract φ (s, centre) = φ centre := by
  change φ _ = φ centre
  congr 1
  apply Subtype.ext
  simp [centre]

/-- The path between constant charts given by a path of points. -/
def constPath {X : Type*} [TopologicalSpace X] {a b : X} (γ : Path a b) :
    ContinuousMap.Homotopy (ContinuousMap.const U a) (ContinuousMap.const U b) where
  toFun p := γ p.1
  continuous_toFun := by fun_prop
  map_zero_left u := by simp
  map_one_left u := by simp

/-- **Theorem 1, §0 form, general**: for charts into any path-connected space, a path joins any two charts. Its
centre follows any chosen path between the two centres. -/
theorem theorem1_general {X : Type*} [TopologicalSpace X] (φ ψ : C(U, X)) (γ : Path (φ centre) (ψ centre)) :
    ∃ H : ContinuousMap.Homotopy φ ψ, ∀ s, H (s, centre) ∈ Set.range γ := by
  refine ⟨((contract φ).trans (constPath γ)).trans (contract ψ).symm, fun s => ?_⟩
  rw [ContinuousMap.Homotopy.trans_apply]
  split_ifs
  · rw [ContinuousMap.Homotopy.trans_apply]
    split_ifs
    · exact ⟨0, by rw [contract_centre]; simp⟩
    · exact ⟨_, rfl⟩
  · rw [ContinuousMap.Homotopy.symm_apply, contract_centre]
    exact ⟨1, by simp⟩

/-- A path of charts, from a homotopy, in the compact-open topology. -/
def homToPath {X : Type*} [TopologicalSpace X] {φ ψ : C(U, X)} (H : ContinuousMap.Homotopy φ ψ) : Path φ ψ where
  toFun s := ContinuousMap.curry H.toContinuousMap s
  continuous_toFun := (ContinuousMap.curry H.toContinuousMap).continuous
  source' := by ext u; simp
  target' := by ext u; simp

/-- **Theorem 1, §0 form**: the chart space of a path-connected target is path-connected. -/
theorem chartSpace_pathConnected_general {X : Type*} [TopologicalSpace X] [PathConnectedSpace X] :
    PathConnectedSpace C(U, X) where
  nonempty := ⟨ContinuousMap.const U (Classical.arbitrary X)⟩
  joined φ ψ := by
    obtain ⟨H, -⟩ := theorem1_general φ ψ (PathConnectedSpace.somePath _ _)
    exact ⟨homToPath H⟩

/-- **Theorem 7, §0 form**: for charts into any space, a based morph exists iff both ends have the anchor at the centre.
No formula is needed: contract each end to its centre. -/
theorem theorem7_general {X : Type*} [TopologicalSpace X] {φ ψ : C(U, X)} {a : X} :
    (∃ H : ContinuousMap.Homotopy φ ψ, ∀ s, H (s, centre) = a) ↔ φ centre = a ∧ ψ centre = a := by
  constructor
  · rintro ⟨H, hH⟩
    exact ⟨by simpa using hH 0, by simpa using hH 1⟩
  · rintro ⟨h1, h2⟩
    obtain ⟨H, hH⟩ := theorem1_general φ ψ ((Path.refl a).cast h1 h2)
    refine ⟨H, fun s => ?_⟩
    obtain ⟨t, ht⟩ := hH s
    rw [← ht]
    rfl

/-- **Theorem 8, §0 form**: for charts into any space, with `φ` centred on `a`, `ψ'` centred on `b` and any path `γ`
from `a` to `b` (A2 and A3, as over `Z`), a path from `φ` to `ψ'` exists whose centre stays on `γ`: no jump. -/
theorem theorem8_general {X : Type*} [TopologicalSpace X] {φ ψ' : C(U, X)} {a b : X} (γ : Path a b)
    (hφ : φ centre = a) (hψ : ψ' centre = b) :
    ∃ H : ContinuousMap.Homotopy φ ψ', ∀ s, H (s, centre) ∈ Set.range γ := by
  obtain ⟨H, hH⟩ := theorem1_general φ ψ' (γ.cast hφ hψ)
  exact ⟨H, fun s => by obtain ⟨t, ht⟩ := hH s; exact ⟨t, by rw [← ht]; rfl⟩⟩

/-! ### The signed state space -/

/-- §0.1: the simplex of three masses. -/
def simplex3 : Set (Fin 3 → ℝ) := {m | (∀ i, 0 ≤ m i) ∧ ∑ i, m i = 1}

theorem convex_simplex3 : Convex ℝ simplex3 := by
  intro m hm m' hm' a b ha hb hab
  refine ⟨fun i => ?_, ?_⟩
  · simp only [Pi.add_apply, Pi.smul_apply, smul_eq_mul]
    exact add_nonneg (mul_nonneg ha (hm.1 i)) (mul_nonneg hb (hm'.1 i))
  · simp only [Pi.add_apply, Pi.smul_apply, smul_eq_mul, Finset.sum_add_distrib, ← Finset.mul_sum, hm.2, hm'.2]
    linarith

/-- §0.1: the mass block, the simplex. -/
abbrev Masses : Type := simplex3

/-- The plane the shape sphere sits in. -/
abbrev E3 : Type := EuclideanSpace ℝ (Fin 3)

/-- §0.1: the configuration block, the signed shape sphere (both mirror halves). -/
abbrev Shape : Type := Metric.sphere (0 : E3) 1

/-- §0.1: the momentum block, `R^4`. -/
abbrev Mom : Type := EuclideanSpace ℝ (Fin 4)

/-- §0.1: the signed state space `S`, the product of the blocks. -/
abbrev S : Type := Masses × Shape × Mom

instance : PathConnectedSpace Masses :=
  isPathConnected_iff_pathConnectedSpace.mp
    (convex_simplex3.isPathConnected ⟨fun _ => 1 / 3, fun _ => by norm_num, by simp⟩)

instance : PathConnectedSpace Shape := by
  have h : Module.rank ℝ E3 = 3 := by
    rw [← Module.finrank_eq_rank, finrank_euclideanSpace_fin]
    norm_num
  exact isPathConnected_iff_pathConnectedSpace.mp (isPathConnected_sphere (by rw [h]; norm_num) 0 zero_le_one)

instance : PathConnectedSpace Mom :=
  pathConnectedSpace_iff_univ.mpr (convex_univ.isPathConnected Set.univ_nonempty)

/-- **Theorem 1, §0 form, as stated in §0.1**: `S` is path-connected, so its chart space is, and a path joins any two
charts into `S`. -/
theorem theorem1_signed : PathConnectedSpace S ∧ PathConnectedSpace C(U, S) ∧
    ∀ φ ψ : C(U, S), Nonempty (ContinuousMap.Homotopy φ ψ) :=
  ⟨inferInstance, chartSpace_pathConnected_general,
    fun φ ψ => ⟨(theorem1_general φ ψ (PathConnectedSpace.somePath _ _)).choose⟩⟩

/-! ### Blending block by block -/

/-- The blend on the simplex, which stays on it. -/
def massBlend (s : I) (m m' : Masses) : Masses :=
  ⟨(1 - (s : ℝ)) • (m : Fin 3 → ℝ) + (s : ℝ) • (m' : Fin 3 → ℝ),
    convex_simplex3 m.2 m'.2 (by have := s.2.2; linarith) s.2.1 (by ring)⟩

/-- The great-circle blend on the sphere: the normalised chord. Away from antipodes it runs along the shorter arc. -/
def sphereBlend (s : ℝ) (x y : E3) : E3 := ‖(1 - s) • x + s • y‖⁻¹ • ((1 - s) • x + s • y)

/-- On the unit sphere the chord vanishes only between antipodes, at `s = ½`. -/
theorem chord_ne_zero {x y : E3} (hx : ‖x‖ = 1) (hy : ‖y‖ = 1) (hxy : x ≠ -y) {s : ℝ} (h0 : 0 ≤ s) (h1 : s ≤ 1) :
    (1 - s) • x + s • y ≠ 0 := by
  intro h
  have he : (1 - s) • x = -(s • y) := eq_neg_of_add_eq_zero_left h
  have hn : 1 - s = s := by
    have := congrArg norm he
    rwa [norm_neg, norm_smul, norm_smul, hx, hy, Real.norm_of_nonneg (by linarith), Real.norm_of_nonneg h0,
      mul_one, mul_one] at this
  have hs : s = 1 / 2 := by linarith
  subst hs
  apply hxy
  have : (1 / 2 : ℝ) • x = (1 / 2 : ℝ) • -y := by rw [smul_neg]; norm_num at he ⊢; exact he
  exact smul_right_injective E3 (by norm_num : (1 / 2 : ℝ) ≠ 0) this

theorem norm_sphereBlend {x y : E3} (hx : ‖x‖ = 1) (hy : ‖y‖ = 1) (hxy : x ≠ -y) {s : ℝ} (h0 : 0 ≤ s) (h1 : s ≤ 1) :
    ‖sphereBlend s x y‖ = 1 := by
  rw [sphereBlend, norm_smul, norm_inv, norm_norm, inv_mul_cancel₀ (norm_ne_zero_iff.mpr (chord_ne_zero hx hy hxy h0 h1))]

/-- §0.1's "geodesic on the sphere": every point of the blend is a non-negative combination of the ends, so it lies on
the shorter arc of the great circle through them. -/
theorem sphereBlend_on_arc (s : ℝ) (h0 : 0 ≤ s) (h1 : s ≤ 1) (x y : E3) :
    ∃ α β : ℝ, 0 ≤ α ∧ 0 ≤ β ∧ sphereBlend s x y = α • x + β • y :=
  ⟨‖(1 - s) • x + s • y‖⁻¹ * (1 - s), ‖(1 - s) • x + s • y‖⁻¹ * s,
    mul_nonneg (inv_nonneg.mpr (norm_nonneg _)) (by linarith), mul_nonneg (inv_nonneg.mpr (norm_nonneg _)) h0,
    by rw [sphereBlend, smul_add, smul_smul, smul_smul]⟩

/-- **Theorem 1, §0 form, block by block**, with the condition the sphere needs (**added hypothesis A8**, finding F11):
no pixel of `ψ` is antipodal to the same pixel of `φ` on the sphere. Then the path that is linear on the simplex and on
`R^4` and the great-circle blend on the sphere is a path from `φ` to `ψ`. -/
def blockPath (φ ψ : C(U, S)) (hA : ∀ u, ((φ u).2.1 : E3) ≠ -((ψ u).2.1 : E3)) : ContinuousMap.Homotopy φ ψ where
  toFun p := (massBlend p.1 (φ p.2).1 (ψ p.2).1,
    ⟨sphereBlend p.1 (φ p.2).2.1 (ψ p.2).2.1, mem_sphere_zero_iff_norm.mpr
      (norm_sphereBlend (by simp) (by simp) (hA p.2) p.1.2.1 p.1.2.2)⟩,
    (1 - (p.1 : ℝ)) • (φ p.2).2.2 + (p.1 : ℝ) • (ψ p.2).2.2)
  continuous_toFun := by
    have hφ := φ.continuous
    have hψ := ψ.continuous
    refine Continuous.prodMk (Continuous.subtype_mk (by fun_prop) _) (Continuous.prodMk ?_ (by fun_prop))
    refine Continuous.subtype_mk ?_ _
    have hw : Continuous fun p : I × U =>
        (1 - (p.1 : ℝ)) • ((φ p.2).2.1 : E3) + (p.1 : ℝ) • ((ψ p.2).2.1 : E3) := by fun_prop
    exact ((continuous_norm.comp hw).inv₀ fun p => norm_ne_zero_iff.mpr
      (chord_ne_zero (by simp) (by simp) (hA p.2) p.1.2.1 p.1.2.2)).smul hw
  map_zero_left u := by
    refine Prod.ext (Subtype.ext ?_) (Prod.ext (Subtype.ext ?_) ?_) <;> simp [massBlend, sphereBlend]
  map_one_left u := by
    refine Prod.ext (Subtype.ext ?_) (Prod.ext (Subtype.ext ?_) ?_) <;> simp [massBlend, sphereBlend]

/-- `blockPath`'s frames are, block by block, the linear blend and the great-circle blend. -/
theorem blockPath_apply (φ ψ : C(U, S)) (hA : ∀ u, ((φ u).2.1 : E3) ≠ -((ψ u).2.1 : E3)) (s : I) (u : U) :
    ((blockPath φ ψ hA (s, u)).1 : Fin 3 → ℝ) = (1 - (s : ℝ)) • ((φ u).1 : Fin 3 → ℝ) + (s : ℝ) • ((ψ u).1 : Fin 3 → ℝ) ∧
      ((blockPath φ ψ hA (s, u)).2.1 : E3) = sphereBlend s (φ u).2.1 (ψ u).2.1 ∧
      (blockPath φ ψ hA (s, u)).2.2 = (1 - (s : ℝ)) • (φ u).2.2 + (s : ℝ) • (ψ u).2.2 :=
  ⟨rfl, rfl, rfl⟩

/-! ### The antipodal case -/

/-- The antipodal case, stated: between antipodes the chord passes through `0` at `s = ½`, so the great-circle blend is
undefined there. -/
theorem antipodal_chord (x : E3) : (1 - (1 / 2 : ℝ)) • x + (1 / 2 : ℝ) • (-x) = 0 := by
  rw [smul_neg]; norm_num

/-- The half great circle from `x` to `−x` through `w`, for `w` a unit vector orthogonal to `x`. -/
def halfCircle (x w : E3) (s : ℝ) : E3 := Real.cos (Real.pi * s) • x + Real.sin (Real.pi * s) • w

/-- The antipodal case, stated: each unit `w ⊥ x` gives a great-circle path from `x` to `−x`, on the sphere, and two
choices of `w` give different paths. So between antipodes a great-circle path exists, but it is a choice. -/
theorem antipodal_halfCircle {x w : E3} (hx : ‖x‖ = 1) (hw : ‖w‖ = 1) (hxw : ⟪x, w⟫_ℝ = 0) :
    halfCircle x w 0 = x ∧ halfCircle x w 1 = -x ∧ (∀ s, ‖halfCircle x w s‖ = 1) ∧
      Continuous (halfCircle x w) ∧ halfCircle x w (1 / 2) ≠ halfCircle x (-w) (1 / 2) := by
  refine ⟨by simp [halfCircle], by simp [halfCircle], fun s => ?_, by unfold halfCircle; fun_prop, ?_⟩
  · have hxx : ⟪x, x⟫_ℝ = 1 := by rw [real_inner_self_eq_norm_sq, hx]; norm_num
    have hww : ⟪w, w⟫_ℝ = 1 := by rw [real_inner_self_eq_norm_sq, hw]; norm_num
    have hwx : ⟪w, x⟫_ℝ = 0 := by rw [real_inner_comm]; exact hxw
    have : ‖halfCircle x w s‖ ^ 2 = 1 := by
      rw [← real_inner_self_eq_norm_sq, halfCircle]
      simp only [inner_add_left, inner_add_right, real_inner_smul_left, real_inner_smul_right, hxx, hww, hxw, hwx]
      linear_combination Real.sin_sq_add_cos_sq (Real.pi * s)
    nlinarith [norm_nonneg (halfCircle x w s)]
  · intro h
    simp only [halfCircle, show Real.pi * (1 / 2) = Real.pi / 2 by ring, Real.cos_pi_div_two, Real.sin_pi_div_two,
      zero_smul, zero_add, one_smul] at h
    have : (2 : ℝ) • w = 0 := by rw [two_smul]; nth_rewrite 2 [h]; simp
    have hw0 : w = 0 := by simpa using this
    rw [hw0, norm_zero] at hw
    exact zero_ne_one hw

/-- The point `(a, 0)` of the square, for `a` clamped to `[-1, 1]`. -/
def onAxis (a : ℝ) : U :=
  ⟨fun i => if i = 0 then (Set.projIcc (-1) 1 (by norm_num) a : ℝ) else 0, mem_squareSet.mpr fun i => by
    split_ifs
    · exact (Set.projIcc (-1) 1 (by norm_num) a).2
    · norm_num⟩

theorem continuous_onAxis : Continuous onAxis := by
  refine Continuous.subtype_mk (continuous_pi fun i => ?_) _
  split_ifs
  · exact continuous_subtype_val.comp continuous_projIcc
  · exact continuous_const

/-- A point of `E3` from its coordinates. -/
def v3 (a b c : ℝ) : E3 := WithLp.toLp 2 ![a, b, c]

theorem norm_sq_v3 (a b c : ℝ) : ‖v3 a b c‖ ^ 2 = a ^ 2 + b ^ 2 + c ^ 2 := by
  rw [EuclideanSpace.real_norm_sq_eq]
  simp [v3, Fin.sum_univ_three]

/-- The pixel `u ↦ ((t²−1)/(1+t²), 2t/(1+t²), 0)`, `t = u₀`: the unit circle, at `(−1,0,0)` only for `t = 0`. -/
def circ (t : ℝ) : E3 := v3 ((t ^ 2 - 1) / (1 + t ^ 2)) (2 * t / (1 + t ^ 2)) 0

theorem norm_circ (t : ℝ) : ‖circ t‖ = 1 := by
  have hp : 0 < 1 + t ^ 2 := by positivity
  have : ‖circ t‖ ^ 2 = 1 := by
    rw [circ, norm_sq_v3]
    field_simp
    ring
  nlinarith [norm_nonneg (circ t)]

/-- **Finding F11**: §0.1's "geodesic on the sphere", block by block, is not always a path, and A8 is needed. With `φ`
the constant chart `(1,0,0)` and `ψ` sweeping the circle with `u₀`, which is antipodal to `φ` on the line of pixels
`u₀ = 0` and nowhere else, the frame at `s = ½` of the great-circle blend has no continuous extension across that line:
its second coordinate is near `+1` on one side and near `−1` on the other. A path between the two charts exists (`theorem1_signed`); a block-wise
shortest-arc path does not. -/
theorem sphere_blend_jumps :
    ∃ x : E3, ∃ y : U → E3, ‖x‖ = 1 ∧ (∀ u, ‖y u‖ = 1) ∧ Continuous y ∧ x = -y centre ∧
      ¬ ∃ g : C(U, E3), ∀ u, x ≠ -y u → g u = sphereBlend (1 / 2) x (y u) := by
  refine ⟨v3 1 0 0, fun u => circ ((u : Plane) 0), ?_, fun u => norm_circ _, ?_, ?_, ?_⟩
  · have h : ‖v3 1 0 0‖ ^ 2 = 1 := by rw [norm_sq_v3]; norm_num
    nlinarith [norm_nonneg (v3 1 0 0)]
  · unfold circ v3
    refine (PiLp.continuous_toLp 2 _).comp (continuous_pi fun i => ?_)
    have hp : ∀ u : U, (1 + ((u : Plane) 0) ^ 2) ≠ 0 := fun u => by positivity
    have hc : Continuous fun u : U => (u : Plane) 0 := (continuous_apply 0).comp continuous_subtype_val
    fin_cases i <;> simp only [Fin.zero_eta, Fin.mk_one, Matrix.cons_val_zero, Matrix.cons_val_one,
      Matrix.cons_val_two, Matrix.head_cons, Matrix.tail_cons, Fin.reduceFinMk]
    · exact Continuous.div ((hc.pow 2).sub continuous_const) (continuous_const.add (hc.pow 2)) hp
    · exact Continuous.div (continuous_const.mul hc) (continuous_const.add (hc.pow 2)) hp
    · exact continuous_const
  · ext i
    fin_cases i <;> simp [circ, v3, centre]
  rintro ⟨g, hg⟩
  -- The frame's second coordinate at `(t, 0)`.
  have key : ∀ t : ℝ, t ≠ 0 → -1 ≤ t → t ≤ 1 →
      (0 < t → 1 / 2 ≤ g (onAxis t) 1) ∧ (t < 0 → g (onAxis t) 1 ≤ -1 / 2) := by
    intro t ht h1 h2
    have hpt : ((onAxis t : U) : Plane) 0 = t := by
      simp [onAxis, Set.projIcc_of_mem _ ⟨h1, h2⟩]
    have hp : 0 < 1 + t ^ 2 := by positivity
    set w : E3 := (1 - (1 / 2 : ℝ)) • v3 1 0 0 + (1 / 2 : ℝ) • circ t with hw
    have hw1 : w 1 = t / (1 + t ^ 2) := by
      simp [hw, circ, v3]; field_simp
    have hw0 : w 0 = t * w 1 := by
      rw [hw1]; simp [hw, circ, v3]; field_simp; try ring
    have hw2 : w 2 = 0 := by simp [hw, circ, v3]
    have hn : ‖w‖ ^ 2 = w 1 ^ 2 * (1 + t ^ 2) := by
      rw [EuclideanSpace.real_norm_sq_eq, Fin.sum_univ_three, hw0, hw2]
      ring
    have hne : v3 1 0 0 ≠ -circ t := by
      intro h
      have := congrArg (fun z : E3 => z 1) h
      simp [circ, v3] at this
      rcases this with h' | h'
      · exact ht h'
      · linarith
    have hgv : g (onAxis t) 1 = ‖w‖⁻¹ * w 1 := by
      have hgu := hg (onAxis t) (by simp only [hpt]; exact hne)
      rw [hgu]
      simp only [sphereBlend, hpt, ← hw]
      rfl
    have hw1ne : w 1 ≠ 0 := by rw [hw1]; exact div_ne_zero ht hp.ne'
    have hnpos : 0 < ‖w‖ := by
      have h2 : 0 < ‖w‖ ^ 2 := by rw [hn]; positivity
      exact lt_of_le_of_ne (norm_nonneg w) (fun h => by rw [← h] at h2; simp at h2)
    have hsq : (g (onAxis t) 1) ^ 2 ≥ 1 / 2 := by
      rw [hgv, mul_pow, inv_pow, hn]
      have ht2 : t ^ 2 ≤ 1 := by nlinarith
      have hw1sq : 0 < w 1 ^ 2 := by positivity
      rw [ge_iff_le, ← sub_nonneg]
      have : (w 1 ^ 2 * (1 + t ^ 2))⁻¹ * w 1 ^ 2 = 1 / (1 + t ^ 2) := by field_simp
      rw [this, div_sub' (by positivity), le_div_iff₀ (by positivity)]
      nlinarith
    constructor
    · intro tpos
      have : 0 < g (onAxis t) 1 := by
        rw [hgv, hw1]; positivity
      nlinarith
    · intro tneg
      have : g (onAxis t) 1 < 0 := by
        rw [hgv, hw1]
        exact mul_neg_of_pos_of_neg (inv_pos.mpr hnpos) (div_neg_of_neg_of_pos tneg hp)
      nlinarith
  have hf : Continuous fun t : ℝ => g (onAxis t) 1 :=
    (continuous_apply 1).comp ((PiLp.continuous_ofLp 2 _).comp (g.continuous.comp continuous_onAxis))
  have hcentre : onAxis 0 = centre := by
    apply Subtype.ext; funext i
    simp only [onAxis, centre, Set.projIcc_of_mem _ (⟨by norm_num, by norm_num⟩ : (0 : ℝ) ∈ Set.Icc (-1) 1)]
    split_ifs <;> rfl
  have hpos : 1 / 2 ≤ g (onAxis 0) 1 := by
    have hcl : closure (Set.Ioc (0 : ℝ) 1) ⊆ {t | 1 / 2 ≤ g (onAxis t) 1} :=
      (isClosed_le continuous_const hf).closure_subset_iff.mpr fun t ht =>
        (key t ht.1.ne' (by linarith [ht.1]) ht.2).1 ht.1
    exact hcl (by rw [closure_Ioc (by norm_num)]; exact ⟨le_refl _, by norm_num⟩)
  have hneg : g (onAxis 0) 1 ≤ -1 / 2 := by
    have hcl : closure (Set.Ico (-1 : ℝ) 0) ⊆ {t | g (onAxis t) 1 ≤ -1 / 2} :=
      (isClosed_le hf continuous_const).closure_subset_iff.mpr fun t ht =>
        (key t ht.2.ne ht.1 (by linarith [ht.2])).2 ht.2
    exact hcl (by rw [closure_Ico (by norm_num)]; exact ⟨by norm_num, le_refl _⟩)
  linarith

end ChartSpace
