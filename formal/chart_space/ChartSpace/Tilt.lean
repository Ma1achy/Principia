/-
§3's tilt and §2's Theorem 2 of `chart-space-model.md`.
-/
import ChartSpace.Ops

open scoped unitInterval InnerProductSpace

noncomputable section

namespace ChartSpace

/-- An orthonormal pair of `Z`: the plane of a rotation. -/
structure RotPlane where
  /-- The first axis. -/
  e : Z
  /-- The second axis. -/
  f : Z
  norm_e : ‖e‖ = 1
  norm_f : ‖f‖ = 1
  inner_ef : ⟪e, f⟫_ℝ = 0

namespace RotPlane

variable (P : RotPlane)

theorem inner_ee : ⟪P.e, P.e⟫_ℝ = 1 := by simp [P.norm_e]

theorem inner_ff : ⟪P.f, P.f⟫_ℝ = 1 := by simp [P.norm_f]

theorem inner_fe : ⟪P.f, P.e⟫_ℝ = 0 := by rw [real_inner_comm]; exact P.inner_ef

/-- §3's `R(θ)`: the rotation by `θ` in the plane of `(e, f)`, turning `e` towards `f`, fixing the plane's orthogonal
complement. -/
def rot (θ : ℝ) (z : Z) : Z :=
  z + (Real.cos θ - 1) • (⟪P.e, z⟫_ℝ • P.e + ⟪P.f, z⟫_ℝ • P.f) +
    Real.sin θ • (⟪P.e, z⟫_ℝ • P.f - ⟪P.f, z⟫_ℝ • P.e)

theorem continuous_rot : Continuous fun p : ℝ × Z => P.rot p.1 p.2 := by
  unfold rot
  fun_prop

@[simp] theorem rot_zero_angle (z : Z) : P.rot 0 z = z := by simp [rot]

/-- The defining formula, with the two inner products named. -/
theorem rot_eq (θ : ℝ) (z : Z) {a b : ℝ} (ha : ⟪P.e, z⟫_ℝ = a) (hb : ⟪P.f, z⟫_ℝ = b) :
    P.rot θ z = z + (Real.cos θ - 1) • (a • P.e + b • P.f) + Real.sin θ • (a • P.f - b • P.e) := by
  rw [rot, ha, hb]

theorem rot_e (θ : ℝ) : P.rot θ P.e = Real.cos θ • P.e + Real.sin θ • P.f := by
  rw [P.rot_eq θ _ P.inner_ee P.inner_fe]
  ext j
  simp only [PiLp.add_apply, PiLp.sub_apply, PiLp.smul_apply, smul_eq_mul]
  ring

theorem rot_f (θ : ℝ) : P.rot θ P.f = Real.cos θ • P.f - Real.sin θ • P.e := by
  rw [P.rot_eq θ _ P.inner_ef P.inner_ff]
  ext j
  simp only [PiLp.add_apply, PiLp.sub_apply, PiLp.smul_apply, smul_eq_mul]
  ring

theorem rot_add (θ : ℝ) (x y : Z) : P.rot θ (x + y) = P.rot θ x + P.rot θ y := by
  simp only [rot, inner_add_right, add_smul, smul_add, sub_eq_add_neg, neg_add, add_assoc]
  abel

theorem rot_smul (θ c : ℝ) (x : Z) : P.rot θ (c • x) = c • P.rot θ x := by
  simp only [rot, inner_smul_right, mul_smul, smul_add, smul_sub]
  simp only [smul_comm c]

@[simp] theorem rot_zero_vec (θ : ℝ) : P.rot θ 0 = 0 := by simp [rot]

/-- `R(θ)` on the plane's coordinates. -/
theorem rot_plane (θ a b : ℝ) :
    P.rot θ (a • P.e + b • P.f) =
      (Real.cos θ * a - Real.sin θ * b) • P.e + (Real.cos θ * b + Real.sin θ * a) • P.f := by
  rw [rot_add, rot_smul, rot_smul, rot_e, rot_f]
  simp only [smul_add, smul_sub, smul_smul, sub_smul, add_smul]
  simp only [mul_comm a, mul_comm b]
  abel

theorem inner_e_rot (θ : ℝ) (z : Z) :
    ⟪P.e, P.rot θ z⟫_ℝ = Real.cos θ * ⟪P.e, z⟫_ℝ - Real.sin θ * ⟪P.f, z⟫_ℝ := by
  simp only [rot, inner_add_right, inner_sub_right, inner_smul_right, P.inner_ee, P.inner_ef]
  ring

theorem inner_f_rot (θ : ℝ) (z : Z) :
    ⟪P.f, P.rot θ z⟫_ℝ = Real.cos θ * ⟪P.f, z⟫_ℝ + Real.sin θ * ⟪P.e, z⟫_ℝ := by
  simp only [rot, inner_add_right, inner_sub_right, inner_smul_right, P.inner_ff, P.inner_fe]
  ring

/-- §2, the proof of Theorem 2: "Rotations preserve inner products." -/
theorem inner_rot (θ : ℝ) (x y : Z) : ⟪P.rot θ x, P.rot θ y⟫_ℝ = ⟪x, y⟫_ℝ := by
  obtain ⟨a, ha⟩ : ∃ a, ⟪P.e, x⟫_ℝ = a := ⟨_, rfl⟩
  obtain ⟨b, hb⟩ : ∃ b, ⟪P.f, x⟫_ℝ = b := ⟨_, rfl⟩
  obtain ⟨c, hc⟩ : ∃ c, ⟪P.e, y⟫_ℝ = c := ⟨_, rfl⟩
  obtain ⟨d, hd⟩ : ∃ d, ⟪P.f, y⟫_ℝ = d := ⟨_, rfl⟩
  have ha' : ⟪x, P.e⟫_ℝ = a := by rw [real_inner_comm]; exact ha
  have hb' : ⟪x, P.f⟫_ℝ = b := by rw [real_inner_comm]; exact hb
  rw [P.rot_eq θ x ha hb, P.rot_eq θ y hc hd]
  simp only [inner_add_left, inner_add_right, inner_sub_left, inner_sub_right, real_inner_smul_left,
    real_inner_smul_right, P.inner_ee, P.inner_ff, P.inner_ef, P.inner_fe, hc, hd, ha', hb']
  linear_combination (a * c + b * d) * Real.sin_sq_add_cos_sq θ

/-- `R` is a one-parameter group, which is the sense of `R(sθ)` in §3. -/
theorem rot_rot (α β : ℝ) (z : Z) : P.rot α (P.rot β z) = P.rot (α + β) z := by
  rw [P.rot_eq α (P.rot β z) (P.inner_e_rot β z) (P.inner_f_rot β z)]
  rw [P.rot_eq β z rfl rfl, P.rot_eq (α + β) z rfl rfl, Real.cos_add, Real.sin_add]
  generalize ⟪P.e, z⟫_ℝ = a
  generalize ⟪P.f, z⟫_ℝ = b
  ext j
  simp only [PiLp.add_apply, PiLp.smul_apply, PiLp.sub_apply, smul_eq_mul]
  ring

end RotPlane

/-- **Added hypothesis A4** (finding F3), as the theorem that discharges it. §3's "a rotation in the plane of `(q_i, t)`"
needs `q_i` and `t` to span a plane. When they do, Gram-Schmidt gives the rotation's axes, and `f` has a positive
component of `t`, which is the sense of "towards `t`". -/
theorem exists_plane_of_tilt {q t : Z} (h : LinearIndependent ℝ ![q, t]) :
    ∃ (P : RotPlane) (α β : ℝ), q = ‖q‖ • P.e ∧ 0 < ‖q‖ ∧ 0 < β ∧ t = α • P.e + β • P.f := by
  have hq : q ≠ 0 := by
    have := h.ne_zero 0
    simpa using this
  have hqn : ‖q‖ ≠ 0 := norm_ne_zero_iff.mpr hq
  set e : Z := ‖q‖⁻¹ • q with he
  have hne : ‖e‖ = 1 := by
    rw [he, norm_smul, norm_inv, norm_norm, inv_mul_cancel₀ hqn]
  set w : Z := t - ⟪e, t⟫_ℝ • e with hw
  have hw0 : w ≠ 0 := by
    intro hw0
    have ht : t = (⟪e, t⟫_ℝ * ‖q‖⁻¹) • q := by
      rw [mul_smul, ← he]
      exact sub_eq_zero.mp hw0
    have hli := LinearIndependent.pair_iff.mp h (⟪e, t⟫_ℝ * ‖q‖⁻¹) (-1) (by
      rw [neg_smul, one_smul, ← ht]
      simp)
    norm_num at hli
  have hwn : ‖w‖ ≠ 0 := norm_ne_zero_iff.mpr hw0
  have hew : ⟪e, w⟫_ℝ = 0 := by
    rw [hw, inner_sub_right, real_inner_smul_right, real_inner_self_eq_norm_sq, hne]
    ring
  refine ⟨⟨e, ‖w‖⁻¹ • w, hne, ?_, ?_⟩, ⟪e, t⟫_ℝ, ‖w‖, ?_, norm_pos_iff.mpr hq, norm_pos_iff.mpr hw0, ?_⟩
  · rw [norm_smul, norm_inv, norm_norm, inv_mul_cancel₀ hwn]
  · rw [real_inner_smul_right, hew, mul_zero]
  · simp only [he, smul_smul, mul_inv_cancel₀ hqn, one_smul]
  · change t = ⟪e, t⟫_ℝ • e + ‖w‖ • (‖w‖⁻¹ • w)
    rw [smul_smul ‖w‖, mul_inv_cancel₀ hwn, one_smul]
    exact (add_sub_cancel _ _).symm

/-! ### Columns, `R Q`, and area -/

/-- The `8×2` matrix `[q₀ q₁]` with the given columns. -/
def ofCols (q₀ q₁ : Z) : Matrix (Fin 8) (Fin 2) ℝ := Matrix.of fun j i => ![q₀, q₁] i j

@[simp] theorem col_ofCols_zero (q₀ q₁ : Z) : col (ofCols q₀ q₁) 0 = q₀ := by
  ext j; simp [col, ofCols]

@[simp] theorem col_ofCols_one (q₀ q₁ : Z) : col (ofCols q₀ q₁) 1 = q₁ := by
  ext j; simp [col, ofCols]

/-- A matrix is its columns. -/
theorem matrix_ext_col {Q Q' : Matrix (Fin 8) (Fin 2) ℝ} (h : ∀ i, col Q i = col Q' i) : Q = Q' := by
  ext j i
  simpa [col] using congrArg (fun z : Z => z j) (h i)

/-- `R Q`: the rotation applied to each column, which is the matrix product. -/
def rotMat (P : RotPlane) (θ : ℝ) (Q : Matrix (Fin 8) (Fin 2) ℝ) : Matrix (Fin 8) (Fin 2) ℝ :=
  Matrix.of fun j i => P.rot θ (col Q i) j

@[simp] theorem col_rotMat (P : RotPlane) (θ : ℝ) (Q : Matrix (Fin 8) (Fin 2) ℝ) (i : Fin 2) :
    col (rotMat P θ Q) i = P.rot θ (col Q i) := by
  ext j; simp [col, rotMat]

theorem matVec_rotMat (P : RotPlane) (θ : ℝ) (Q : Matrix (Fin 8) (Fin 2) ℝ) (u : Plane) :
    matVec (rotMat P θ Q) u = P.rot θ (matVec Q u) := by
  simp [matVec_eq, P.rot_add, P.rot_smul]

/-- `(Qᵀ Q') i j` is the inner product of column `i` of `Q` and column `j` of `Q'`. -/
theorem gram_apply (Q Q' : Matrix (Fin 8) (Fin 2) ℝ) (i j : Fin 2) :
    (Q.transpose * Q') i j = ⟪col Q i, col Q' j⟫_ℝ := by
  simp [Matrix.mul_apply, col, PiLp.inner_apply, mul_comm]

/-- §2: the frame's area, `area = sqrt(det(Qᵀ Q))`. -/
def area (Q : Matrix (Fin 8) (Fin 2) ℝ) : ℝ := Real.sqrt (Q.transpose * Q).det

/-- §2, the proof of Theorem 2: `Q_sᵀ Q_s = Qᵀ R(sθ)ᵀ R(sθ) Q = Qᵀ Q`. -/
theorem gram_rotMat (P : RotPlane) (θ : ℝ) (Q : Matrix (Fin 8) (Fin 2) ℝ) :
    (rotMat P θ Q).transpose * rotMat P θ Q = Q.transpose * Q := by
  ext i j
  simp only [gram_apply, col_rotMat, P.inner_rot]

/-- Rank 2 is `det(Qᵀ Q) ≠ 0`. -/
theorem rank_two_iff_det (Q : Matrix (Fin 8) (Fin 2) ℝ) : Q.rank = 2 ↔ (Q.transpose * Q).det ≠ 0 := by
  rw [← Matrix.rank_transpose_mul_self Q]
  set A := Q.transpose * Q
  constructor
  · intro h
    have hs : Function.Surjective A.mulVec := by
      have htop : LinearMap.range A.mulVecLin = ⊤ := by
        apply Submodule.eq_top_of_finrank_eq
        rw [Module.finrank_fin_fun]
        exact h
      exact LinearMap.range_eq_top.mp htop
    have hu := Matrix.mulVec_surjective_iff_isUnit.mp hs
    exact ((Matrix.isUnit_iff_isUnit_det A).mp hu).ne_zero
  · intro h
    have hu : IsUnit A := (Matrix.isUnit_iff_isUnit_det A).mpr (isUnit_iff_ne_zero.mpr h)
    simpa using Matrix.rank_of_isUnit A hu

/-! ### Theorem 2: the tilt -/

/-- §3 **Tilt of an affine chart**: `H_s(u) = z₀ + R(sθ) Q u`. -/
def tiltPath (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ) (P : RotPlane) (θ : ℝ) :
    EditPath (affine z₀ Q) (affine z₀ (rotMat P θ Q)) where
  toFun p := z₀ + P.rot ((p.1 : ℝ) * θ) (matVec Q p.2)
  continuous_toFun := by
    have h : Continuous fun p : I × U => (((p.1 : ℝ) * θ), matVec Q p.2) :=
      (by fun_prop : Continuous fun p : I × U => (p.1 : ℝ) * θ).prodMk
        ((continuous_matVec Q).comp (continuous_subtype_val.comp continuous_snd))
    exact continuous_const.add (P.continuous_rot.comp h)
  map_zero_left u := by simp
  map_one_left u := by simp [matVec_rotMat]

@[simp] theorem tiltPath_apply (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ) (P : RotPlane) (θ : ℝ) (s : I) (u : U) :
    tiltPath z₀ Q P θ (s, u) = z₀ + P.rot ((s : ℝ) * θ) (matVec Q u) := rfl

/-- §3 **latent plane geodesic**, the family, as the table defines it: paths with the closed form
`H_s(u) = z₀ + R(sθ) Q u`. The document never defines "geodesic" beyond this table (finding F3). -/
def IsLatentPlaneGeodesic {φ ψ : Chart} (H : EditPath φ ψ) : Prop :=
  ∃ (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ) (P : RotPlane) (θ : ℝ),
    ∀ s u, H (s, u) = z₀ + P.rot ((s : ℝ) * θ) (matVec Q u)

/-- §2 **Theorem 2**, the positive part, as stated: the tilt path is a path between `φ` and `ψ = z₀ + R(θ) Q u`, it is
exactly a member of the latent plane geodesic family, each frame is the affine chart of `Q_s = R(sθ) Q`, and it preserves
`Q_sᵀ Q_s`, the frame's area and its rank. -/
theorem theorem2_tilt (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ) (P : RotPlane) (θ : ℝ) :
    IsLatentPlaneGeodesic (tiltPath z₀ Q P θ) ∧
      (∀ s : I, frame (tiltPath z₀ Q P θ) s = affine z₀ (rotMat P ((s : ℝ) * θ) Q)) ∧
      (∀ s : I, (rotMat P ((s : ℝ) * θ) Q).transpose * rotMat P ((s : ℝ) * θ) Q = Q.transpose * Q) ∧
      (∀ s : I, area (rotMat P ((s : ℝ) * θ) Q) = area Q) ∧
      (∀ s : I, (rotMat P ((s : ℝ) * θ) Q).rank = Q.rank) := by
  refine ⟨⟨z₀, Q, P, θ, fun _ _ => rfl⟩, fun s => ?_, fun s => gram_rotMat _ _ _, fun s => ?_, fun s => ?_⟩
  · ext1 u
    simp [matVec_rotMat]
  · rw [area, area, gram_rotMat]
  · rw [← Matrix.rank_transpose_mul_self, gram_rotMat, Matrix.rank_transpose_mul_self]

/-- A tilt of an affine chart is an affine chart. -/
theorem tilt_frame_isAffine {z₀ : Z} {Q : Matrix (Fin 8) (Fin 2) ℝ} (hQ : Q.rank = 2) (P : RotPlane) (θ : ℝ) (s : I) :
    IsAffineChart (frame (tiltPath z₀ Q P θ) s) :=
  ⟨z₀, _, ((theorem2_tilt z₀ Q P θ).2.2.2.2 s).trans hQ, (theorem2_tilt z₀ Q P θ).2.1 s⟩

/-! ### Affine representations are unique -/

theorem pi_single_mem_square (i : Fin 2) : (Pi.single i 1 : Plane) ∈ squareSet := by
  refine mem_squareSet.mpr fun j => ?_
  by_cases h : j = i
  · subst h; simp
  · simp [Pi.single_eq_of_ne h]

theorem matVec_single (Q : Matrix (Fin 8) (Fin 2) ℝ) (i : Fin 2) : matVec Q (Pi.single i 1) = col Q i := by
  fin_cases i <;> simp [matVec_eq]

/-- A chart determines its `z₀` and its `Q`. -/
theorem affine_inj {z z' : Z} {Q Q' : Matrix (Fin 8) (Fin 2) ℝ} (h : affine z Q = affine z' Q') : z = z' ∧ Q = Q' := by
  have hc : z = z' := by
    simpa [centre] using congrArg (fun χ : Chart => χ centre) h
  refine ⟨hc, matrix_ext_col fun i => ?_⟩
  have := congrArg (fun χ : Chart => χ ⟨_, pi_single_mem_square i⟩) h
  simpa [matVec_single, hc] using this

/-! ### Theorem 2: the straight-line blend -/

/-- The blend `(1−s) Q + s Q'`. -/
def blendMat (Q Q' : Matrix (Fin 8) (Fin 2) ℝ) (s : ℝ) : Matrix (Fin 8) (Fin 2) ℝ := (1 - s) • Q + s • Q'

theorem col_blendMat (Q Q' : Matrix (Fin 8) (Fin 2) ℝ) (s : ℝ) (i : Fin 2) :
    col (blendMat Q Q' s) i = (1 - s) • col Q i + s • col Q' i := by
  ext j; simp [col, blendMat]

/-- Each frame of `L` between two affine charts with the same `z₀` is the affine form of the blended matrix. The form
may have rank below 2, so it needn't be an affine chart of §1. -/
theorem frame_line_affine (z₀ : Z) (Q Q' : Matrix (Fin 8) (Fin 2) ℝ) (s : I) :
    frame (lineMorph (affine z₀ Q) (affine z₀ Q')) s = affine z₀ (blendMat Q Q' s) := by
  ext1 u
  simp only [frame_apply, lineMorph_apply, affine_apply, matVec_eq, col_blendMat]
  ext j
  simp only [PiLp.add_apply, PiLp.smul_apply, smul_eq_mul]
  ring

/-- The standard basis vector `e_k` of `Z`. -/
def bas (k : Fin 8) : Z := EuclideanSpace.single k (1 : ℝ)

theorem inner_bas (k l : Fin 8) : ⟪bas k, bas l⟫_ℝ = if k = l then 1 else 0 := by
  rw [bas, EuclideanSpace.inner_single_left]
  by_cases h : k = l
  · simp [bas, h]
  · simp [bas, h]

/-- §2's `Q = [e₁ e₂]`. -/
def frameE : Matrix (Fin 8) (Fin 2) ℝ := ofCols (bas 0) (bas 1)

/-- §2's `Q' = [e₂ e₁]`. -/
def frameS : Matrix (Fin 8) (Fin 2) ℝ := ofCols (bas 1) (bas 0)

theorem det_gram (Q : Matrix (Fin 8) (Fin 2) ℝ) :
    (Q.transpose * Q).det = ⟪col Q 0, col Q 0⟫_ℝ * ⟪col Q 1, col Q 1⟫_ℝ - ⟪col Q 0, col Q 1⟫_ℝ * ⟪col Q 1, col Q 0⟫_ℝ := by
  rw [Matrix.det_fin_two]
  simp only [gram_apply]

theorem det_gram_frameE : (frameE.transpose * frameE).det = 1 := by
  rw [det_gram]
  simp only [frameE, col_ofCols_zero, col_ofCols_one, inner_bas]
  norm_num

theorem det_gram_frameS : (frameS.transpose * frameS).det = 1 := by
  rw [det_gram]
  simp only [frameS, col_ofCols_zero, col_ofCols_one, inner_bas]
  norm_num

theorem rank_frameE : frameE.rank = 2 := (rank_two_iff_det _).mpr (by rw [det_gram_frameE]; exact one_ne_zero)

theorem rank_frameS : frameS.rank = 2 := (rank_two_iff_det _).mpr (by rw [det_gram_frameS]; exact one_ne_zero)

/-- The blend's midpoint: both columns are `(e₁ + e₂)/2`. -/
theorem col_blend_half (i : Fin 2) : col (blendMat frameE frameS (1 / 2)) i = (1 / 2 : ℝ) • (bas 0 + bas 1) := by
  rw [col_blendMat, smul_add]
  fin_cases i
  · simp [frameE, frameS]; norm_num
  · simp [frameE, frameS, add_comm]; norm_num

/-- A matrix with both columns equal to a non-zero `w` has rank 1. -/
theorem rank_of_equal_columns {Q : Matrix (Fin 8) (Fin 2) ℝ} {w : Z} (hw : w ≠ 0) (h : ∀ i, col Q i = w) :
    Q.rank = 1 := by
  have hc : ∀ j i, Q j i = w j := fun j i => by
    simpa [col] using congrArg (fun z : Z => z j) (h i)
  have hrange : LinearMap.range Q.mulVecLin = Submodule.span ℝ {WithLp.ofLp w} := by
    ext v
    rw [LinearMap.mem_range, Submodule.mem_span_singleton]
    constructor
    · rintro ⟨u, rfl⟩
      refine ⟨u 0 + u 1, ?_⟩
      ext j
      simp [Matrix.mulVec, dotProduct, Fin.sum_univ_two, hc]
      ring
    · rintro ⟨c, rfl⟩
      refine ⟨Pi.single 0 c, ?_⟩
      ext j
      simp [Matrix.mulVec, dotProduct, Fin.sum_univ_two, hc]
      ring
  rw [Matrix.rank, hrange]
  have hw' : WithLp.ofLp w ≠ 0 := fun h0 => hw (by
    ext j
    simpa using congrFun h0 j)
  exact finrank_span_singleton hw'

/-- `½`, as a parameter. -/
def halfI : I := ⟨1 / 2, by norm_num, by norm_num⟩

/-- §2 **Theorem 2**, the negative part, as stated: `Q = [e₁ e₂]` and `Q' = [e₂ e₁]` each have area 1, and at `s = ½`
both columns of the blend are `(e₁+e₂)/2`, so it has rank 1 and area 0, and the frame isn't an affine chart of §1. -/
theorem theorem2_swap :
    area frameE = 1 ∧ area frameS = 1 ∧
      (∀ i, col (blendMat frameE frameS (1 / 2)) i = (1 / 2 : ℝ) • (bas 0 + bas 1)) ∧
      (blendMat frameE frameS (1 / 2)).rank = 1 ∧
      area (blendMat frameE frameS (1 / 2)) = 0 ∧
      ¬ IsAffineChart (frame (lineMorph (affine 0 frameE) (affine 0 frameS)) halfI) := by
  have hw : (1 / 2 : ℝ) • (bas 0 + bas 1) ≠ 0 := by
    intro h
    have := congrArg (fun z : Z => z 0) h
    simp [bas] at this
  have hrank := rank_of_equal_columns hw col_blend_half
  refine ⟨by simp [area, det_gram_frameE], by simp [area, det_gram_frameS], col_blend_half, hrank, ?_, ?_⟩
  · rw [area, det_gram, col_blend_half, col_blend_half]
    simp
  · rintro ⟨z₀, Q, hQ, h⟩
    have hf : frame (lineMorph (affine 0 frameE) (affine 0 frameS)) halfI =
        affine 0 (blendMat frameE frameS (1 / 2)) := by
      simpa [halfI] using frame_line_affine 0 frameE frameS halfI
    obtain ⟨-, rfl⟩ := affine_inj (hf.symm.trans h)
    omega

/-- A tilt in the document's own sense also loses area under the straight line (finding F2): `Q` with orthonormal
columns, a rotation in the plane of `(q₁, t)` with `t` orthogonal to the columns. Then
`det(Q_sᵀ Q_s) = 1 − 2 s (1−s) (1 − cos θ)`, where `Q_s = (1−s) Q + s R(θ) Q`. -/
theorem det_blend_tilt {Q : Matrix (Fin 8) (Fin 2) ℝ} {P : RotPlane} (h0 : col Q 0 = P.e) (h1 : ‖col Q 1‖ = 1)
    (he : ⟪P.e, col Q 1⟫_ℝ = 0) (hf : ⟪P.f, col Q 1⟫_ℝ = 0) (θ s : ℝ) :
    ((blendMat Q (rotMat P θ Q) s).transpose * blendMat Q (rotMat P θ Q) s).det =
      1 - 2 * s * (1 - s) * (1 - Real.cos θ) := by
  have hr1 : P.rot θ (col Q 1) = col Q 1 := by
    rw [P.rot_eq θ _ he hf]; simp
  have c0 : col (blendMat Q (rotMat P θ Q) s) 0 = (1 - s + s * Real.cos θ) • P.e + (s * Real.sin θ) • P.f := by
    rw [col_blendMat, col_rotMat, h0, P.rot_e, smul_add, smul_smul, smul_smul, add_smul]
    abel
  have c1 : col (blendMat Q (rotMat P θ Q) s) 1 = col Q 1 := by
    rw [col_blendMat, col_rotMat, hr1, ← add_smul]; simp
  have h11 : ⟪col Q 1, col Q 1⟫_ℝ = 1 := by rw [real_inner_self_eq_norm_sq, h1]; norm_num
  have he' : ⟪col Q 1, P.e⟫_ℝ = 0 := by rw [real_inner_comm]; exact he
  have hf' : ⟪col Q 1, P.f⟫_ℝ = 0 := by rw [real_inner_comm]; exact hf
  rw [det_gram, c0, c1]
  simp only [inner_add_left, inner_add_right, real_inner_smul_left, real_inner_smul_right, P.inner_ee, P.inner_ff,
    P.inner_ef, P.inner_fe, h11, he, hf, he', hf']
  linear_combination (s ^ 2) * Real.sin_sq_add_cos_sq θ

/-- Theorem 2's negative claim for a genuine tilt (finding F2): for `0 < s < 1` and `cos θ ≠ 1`, the straight line loses
area strictly. At `θ = π`, `s = ½` it collapses, as the swapped pair does. -/
theorem line_loses_area {Q : Matrix (Fin 8) (Fin 2) ℝ} {P : RotPlane} (h0 : col Q 0 = P.e) (h1 : ‖col Q 1‖ = 1)
    (he : ⟪P.e, col Q 1⟫_ℝ = 0) (hf : ⟪P.f, col Q 1⟫_ℝ = 0) {θ s : ℝ} (hs0 : 0 < s) (hs1 : s < 1)
    (hθ : Real.cos θ ≠ 1) :
    area (blendMat Q (rotMat P θ Q) s) < 1 ∧ area (blendMat Q (rotMat P Real.pi Q) (1 / 2)) = 0 := by
  constructor
  · rw [area, det_blend_tilt h0 h1 he hf]
    have hc : 0 < 1 - Real.cos θ := sub_pos.mpr (lt_of_le_of_ne (Real.cos_le_one θ) hθ)
    have hp : 0 < 2 * s * (1 - s) * (1 - Real.cos θ) := by
      have : 0 < 1 - s := sub_pos.mpr hs1
      positivity
    rw [Real.sqrt_lt' one_pos]
    linarith
  · rw [area, det_blend_tilt h0 h1 he hf]
    simp only [Real.cos_pi]
    norm_num

/-- Theorem 2's negative claim for the tilt towards the other column (`t = q₂`, a rotation inside the plane of `Q`'s
orthonormal columns): `det(Q_sᵀ Q_s) = (1 − 2s(1−s)(1 − cos θ))²`, not `det_blend_tilt`'s formula. -/
theorem det_blend_inplane (P : RotPlane) (θ s : ℝ) :
    ((blendMat (ofCols P.e P.f) (rotMat P θ (ofCols P.e P.f)) s).transpose *
        blendMat (ofCols P.e P.f) (rotMat P θ (ofCols P.e P.f)) s).det =
      (1 - 2 * s * (1 - s) * (1 - Real.cos θ)) ^ 2 := by
  have c0 : col (blendMat (ofCols P.e P.f) (rotMat P θ (ofCols P.e P.f)) s) 0 =
      (1 - s + s * Real.cos θ) • P.e + (s * Real.sin θ) • P.f := by
    rw [col_blendMat, col_rotMat, col_ofCols_zero, P.rot_e]
    ext j
    simp only [PiLp.add_apply, PiLp.smul_apply, smul_eq_mul]
    ring
  have c1 : col (blendMat (ofCols P.e P.f) (rotMat P θ (ofCols P.e P.f)) s) 1 =
      (-(s * Real.sin θ)) • P.e + (1 - s + s * Real.cos θ) • P.f := by
    rw [col_blendMat, col_rotMat, col_ofCols_one, P.rot_f]
    ext j
    simp only [PiLp.add_apply, PiLp.sub_apply, PiLp.smul_apply, smul_eq_mul]
    ring
  rw [det_gram, c0, c1]
  simp only [inner_add_left, inner_add_right, real_inner_smul_left, real_inner_smul_right, P.inner_ee, P.inner_ff,
    P.inner_ef, P.inner_fe]
  linear_combination (s ^ 2 * ((1 - s + s * Real.cos θ) ^ 2 + (s * Real.sin θ) ^ 2 +
    (1 - 2 * s * (1 - s) * (1 - Real.cos θ)))) * Real.sin_sq_add_cos_sq θ

/-! ### The swap, against the theorem's hypothesis -/

/-- Build a rotation plane from inner products. -/
def RotPlane.ofInner (e f : Z) (he : ⟪e, e⟫_ℝ = 1) (hf : ⟪f, f⟫_ℝ = 1) (hef : ⟪e, f⟫_ℝ = 0) : RotPlane where
  e := e
  f := f
  norm_e := by
    have : ‖e‖ ^ 2 = 1 := by rw [← real_inner_self_eq_norm_sq, he]
    nlinarith [norm_nonneg e, sq_nonneg (‖e‖ - 1)]
  norm_f := by
    have : ‖f‖ ^ 2 = 1 := by rw [← real_inner_self_eq_norm_sq, hf]
    nlinarith [norm_nonneg f, sq_nonneg (‖f‖ - 1)]
  inner_ef := hef

/-- §2's swap does satisfy `Q' = R(θ) Q`, for a rotation by `π` in the plane of `(e₁ − e₂, e₃)`, which is not a plane of
any `(q_i, t)`. So a tilt-like path from `[e₁ e₂]` to `[e₂ e₁]` exists that keeps the area: it passes through `e₃`. -/
theorem swap_is_rotation_by_pi : ∃ P : RotPlane, rotMat P Real.pi frameE = frameS := by
  set c : ℝ := (Real.sqrt 2)⁻¹ with hc
  have hc2 : c * c = 1 / 2 := by
    rw [hc, ← mul_inv, Real.mul_self_sqrt (by norm_num)]
    norm_num
  set e : Z := c • (bas 0 - bas 1) with he
  have he0 : ⟪e, bas 0⟫_ℝ = c := by
    simp only [he, inner_sub_left, real_inner_smul_left, inner_bas]
    norm_num
  have he1 : ⟪e, bas 1⟫_ℝ = -c := by
    simp only [he, inner_sub_left, real_inner_smul_left, inner_bas]
    norm_num
  have hee : ⟪e, e⟫_ℝ = 1 := by
    simp only [he, inner_sub_left, inner_sub_right, real_inner_smul_left, real_inner_smul_right, inner_bas]
    norm_num
    linarith
  have he3 : ⟪e, bas 2⟫_ℝ = 0 := by
    simp only [he, inner_sub_left, real_inner_smul_left, inner_bas]
    norm_num
  refine ⟨RotPlane.ofInner e (bas 2) hee (by rw [inner_bas]; simp) he3, matrix_ext_col fun i => ?_⟩
  have hb0 : ⟪bas 2, bas 0⟫_ℝ = 0 := by rw [inner_bas]; simp
  have hb1 : ⟪bas 2, bas 1⟫_ℝ = 0 := by rw [inner_bas]; simp
  have key0 : (2 * c) • e = bas 0 - bas 1 := by
    rw [he, smul_smul, show 2 * c * c = 1 by nlinarith, one_smul]
  fin_cases i
  · simp only [Fin.zero_eta, col_rotMat, frameE, frameS, col_ofCols_zero]
    rw [RotPlane.rot_eq _ _ _ he0 hb0]
    simp only [RotPlane.ofInner, Real.cos_pi, Real.sin_pi, zero_smul, add_zero]
    rw [show ((-1 : ℝ) - 1) • c • e = -((2 * c) • e) by
      rw [smul_smul, ← neg_smul]; congr 1; ring]
    rw [key0]
    abel
  · simp only [Fin.mk_one, col_rotMat, frameE, frameS, col_ofCols_one]
    rw [RotPlane.rot_eq _ _ _ he1 hb1]
    simp only [RotPlane.ofInner, Real.cos_pi, Real.sin_pi, zero_smul, add_zero]
    rw [show ((-1 : ℝ) - 1) • (-c) • e = (2 * c) • e by
      rw [smul_smul]; congr 1; ring]
    rw [key0]
    abel

/-- **Finding F2**: §2's swap is no tilt. If `x, y` are orthonormal and `x` lies in the plane of a rotation `R`, then `R`
doesn't exchange `x` and `y`. A rotation in the plane of `(q_i, t)` has `q_i` in that plane. -/
theorem no_exchange (P : RotPlane) (θ : ℝ) {x y : Z} (hx : ‖x‖ = 1) (hxy : ⟪x, y⟫_ℝ = 0)
    (hin : x ∈ Submodule.span ℝ {P.e, P.f}) (h1 : P.rot θ x = y) (h2 : P.rot θ y = x) : False := by
  obtain ⟨a, b, rfl⟩ := Submodule.mem_span_pair.mp hin
  have hab : a ^ 2 + b ^ 2 = 1 := by
    have : ⟪a • P.e + b • P.f, a • P.e + b • P.f⟫_ℝ = 1 := by
      rw [real_inner_self_eq_norm_sq, hx]; norm_num
    simp only [inner_add_left, inner_add_right, real_inner_smul_left, real_inner_smul_right, P.inner_ee, P.inner_ff,
      P.inner_ef, P.inner_fe] at this
    linarith
  have hy : y = (Real.cos θ * a - Real.sin θ * b) • P.e + (Real.cos θ * b + Real.sin θ * a) • P.f := by
    rw [← h1, P.rot_plane]
  have hc : Real.cos θ = 0 := by
    rw [hy] at hxy
    simp only [inner_add_left, inner_add_right, real_inner_smul_left, real_inner_smul_right, P.inner_ee, P.inner_ff,
      P.inner_ef, P.inner_fe] at hxy
    linear_combination hxy - Real.cos θ * hab
  have hs : Real.sin θ ^ 2 = 1 := by
    have := Real.sin_sq_add_cos_sq θ
    rw [hc] at this
    linarith
  rw [hy, P.rot_plane, hc] at h2
  have hxx : ⟪a • P.e + b • P.f, a • P.e + b • P.f⟫_ℝ = 1 := by
    rw [real_inner_self_eq_norm_sq, hx]; norm_num
  have hneg : (0 * ((0 * a - Real.sin θ * b)) - Real.sin θ * (0 * b + Real.sin θ * a)) • P.e +
      (0 * (0 * b + Real.sin θ * a) + Real.sin θ * (0 * a - Real.sin θ * b)) • P.f = -(a • P.e + b • P.f) := by
    have ea : Real.sin θ * Real.sin θ = 1 := by nlinarith
    have e1 : (0 * ((0 * a - Real.sin θ * b)) - Real.sin θ * (0 * b + Real.sin θ * a)) = -a := by
      linear_combination (-a) * ea
    have e2 : (0 * (0 * b + Real.sin θ * a) + Real.sin θ * (0 * a - Real.sin θ * b)) = -b := by
      linear_combination (-b) * ea
    rw [e1, e2, neg_smul, neg_smul]
    abel
  rw [hneg] at h2
  have hz : a • P.e + b • P.f = 0 := by
    ext j
    have := congrArg (fun z : Z => z j) h2
    simp only [PiLp.neg_apply] at this
    simp only [PiLp.zero_apply]
    linarith
  rw [hz] at hxx
  simp at hxx

/-- **Finding F2**, as stated of the example: `Q' = [e₂ e₁]` is not `R(θ) Q` for any rotation in the plane of any
`(q_i, t)`. So the example's two charts aren't "the same two charts" as a tilt's: they don't meet the theorem's
hypothesis as §3 defines `R`. -/
theorem swap_is_not_a_tilt (P : RotPlane) (θ : ℝ) (i : Fin 2) (hin : col frameE i ∈ Submodule.span ℝ {P.e, P.f}) :
    rotMat P θ frameE ≠ frameS := by
  intro h
  have h0 : P.rot θ (bas 0) = bas 1 := by
    have := congrArg (fun M => col M 0) h
    simpa [frameE, frameS] using this
  have h1 : P.rot θ (bas 1) = bas 0 := by
    have := congrArg (fun M => col M 1) h
    simpa [frameE, frameS] using this
  have n0 : ‖bas 0‖ = 1 := by simp [bas]
  have n1 : ‖bas 1‖ = 1 := by simp [bas]
  fin_cases i
  · exact no_exchange P θ n0 (by rw [inner_bas]; simp) (by simpa [frameE] using hin) h0 h1
  · exact no_exchange P θ n1 (by rw [inner_bas]; simp) (by simpa [frameE] using hin) h1 h0

/-! ### "Geodesic": what the plane does -/

/-- The product of the cosines of the principal angles between the planes of frames `a` and `b` of a tilt, for `Q` with
orthonormal columns and `q₁ = e`, in terms of `c = ⟪f, q₂⟫`:
`det(Q_aᵀ Q_b) = cos α + c² (1 − cos α)`, `α = b − a`. -/
theorem det_frames {Q : Matrix (Fin 8) (Fin 2) ℝ} {P : RotPlane} (h0 : col Q 0 = P.e) (h1 : ‖col Q 1‖ = 1)
    (he : ⟪P.e, col Q 1⟫_ℝ = 0) (a b : ℝ) :
    ((rotMat P a Q).transpose * rotMat P b Q).det =
      Real.cos (b - a) + ⟪P.f, col Q 1⟫_ℝ ^ 2 * (1 - Real.cos (b - a)) := by
  obtain ⟨c, hc⟩ : ∃ c, ⟪P.f, col Q 1⟫_ℝ = c := ⟨_, rfl⟩
  have hb : ∀ z, P.rot b z = P.rot a (P.rot (b - a) z) := fun z => by
    rw [P.rot_rot]; congr 1; ring
  have hg : ∀ i j, ((rotMat P a Q).transpose * rotMat P b Q) i j = ⟪col Q i, P.rot (b - a) (col Q j)⟫_ℝ := by
    intro i j
    rw [gram_apply, col_rotMat, col_rotMat, hb, P.inner_rot]
  have h11 : ⟪col Q 1, col Q 1⟫_ℝ = 1 := by rw [real_inner_self_eq_norm_sq, h1]; norm_num
  have he' : ⟪col Q 1, P.e⟫_ℝ = 0 := by rw [real_inner_comm]; exact he
  have hc' : ⟪col Q 1, P.f⟫_ℝ = c := by rw [real_inner_comm]; exact hc
  rw [Matrix.det_fin_two, hg, hg, hg, hg, h0, hc, P.inner_e_rot, P.inner_e_rot, P.rot_e,
    P.rot_eq _ (col Q 1) he hc]
  simp only [inner_add_right, inner_sub_right, real_inner_smul_right, P.inner_ee, P.inner_fe, h11, he, he', hc, hc']
  linear_combination (c ^ 2) * Real.sin_sq_add_cos_sq (b - a)

/-- With `t` orthogonal to the columns (`c = 0`), the principal angle between frames is `|b − a|`: the plane moves at
constant speed. This is the sense in which a tilt is a geodesic of planes. **Added hypothesis A5** (finding F3). -/
theorem tilt_constant_speed {Q : Matrix (Fin 8) (Fin 2) ℝ} {P : RotPlane} (h0 : col Q 0 = P.e) (h1 : ‖col Q 1‖ = 1)
    (he : ⟪P.e, col Q 1⟫_ℝ = 0) (hf : ⟪P.f, col Q 1⟫_ℝ = 0) (a b : ℝ) :
    ((rotMat P a Q).transpose * rotMat P b Q).det = Real.cos (b - a) := by
  rw [det_frames h0 h1 he, hf]
  ring

/-- Finding F3: without A5, that fails. If `0 < c² < 1`, then `cos α + c² (1 − cos α)` is not `cos (κ α)` for any
constant `κ`: the frames' cosine is not that of an angle growing at constant speed. -/
theorem tilt_not_constant_speed {c : ℝ} (h0 : c ^ 2 ≠ 0) (h1 : c ^ 2 ≠ 1) :
    ¬ ∃ κ : ℝ, ∀ α : ℝ, Real.cos (κ * α) = Real.cos α + c ^ 2 * (1 - Real.cos α) := by
  rintro ⟨κ, h⟩
  have hpi := h Real.pi
  have h2pi := h (2 * Real.pi)
  rw [Real.cos_pi] at hpi
  rw [Real.cos_two_pi, show κ * (2 * Real.pi) = 2 * (κ * Real.pi) by ring, Real.cos_two_mul, hpi] at h2pi
  have : c ^ 2 * (c ^ 2 - 1) * (2 * c ^ 2 - 1 + 1) = 0 := by nlinarith
  have hne : c ^ 2 * (c ^ 2 - 1) ≠ 0 := mul_ne_zero h0 (sub_ne_zero.mpr h1)
  have : c ^ 2 = 0 := by
    have h3 := (mul_eq_zero.mp this).resolve_left hne
    linarith
  exact h0 this

/-- Finding F3, for the tilt itself: with `t` neither orthogonal to `q₂` nor along it, no constant `κ` has
`det(Q_aᵀ Q_b) = cos (κ (b − a))`. -/
theorem tilt_frames_not_constant_speed {Q : Matrix (Fin 8) (Fin 2) ℝ} {P : RotPlane} (h0 : col Q 0 = P.e)
    (h1 : ‖col Q 1‖ = 1) (he : ⟪P.e, col Q 1⟫_ℝ = 0) (hc0 : ⟪P.f, col Q 1⟫_ℝ ^ 2 ≠ 0) (hc1 : ⟪P.f, col Q 1⟫_ℝ ^ 2 ≠ 1) :
    ¬ ∃ κ : ℝ, ∀ a b : ℝ, ((rotMat P a Q).transpose * rotMat P b Q).det = Real.cos (κ * (b - a)) := by
  rintro ⟨κ, h⟩
  refine tilt_not_constant_speed hc0 hc1 ⟨κ, fun α => ?_⟩
  have := h 0 α
  rw [det_frames h0 h1 he, sub_zero] at this
  exact this.symm

/-- Finding F3: tilting a column towards the other column (`t = q₂`, `c = 1`) is a member of §3's family, it moves every
frame, and the plane doesn't move: each frame's columns span the same plane. So the family's frames aren't determined
by the two planes. -/
theorem tilt_inside_plane (P : RotPlane) {θ : ℝ} (hθ : Real.sin θ ≠ 0) :
    Submodule.span ℝ (Set.range (col (rotMat P θ (ofCols P.e P.f)))) =
        Submodule.span ℝ (Set.range (col (ofCols P.e P.f))) ∧
      rotMat P θ (ofCols P.e P.f) ≠ ofCols P.e P.f := by
  have r0 : col (rotMat P θ (ofCols P.e P.f)) 0 = Real.cos θ • P.e + Real.sin θ • P.f := by
    simp [P.rot_e]
  have r1 : col (rotMat P θ (ofCols P.e P.f)) 1 = Real.cos θ • P.f - Real.sin θ • P.e := by
    simp [P.rot_f]
  have hrange : ∀ Q : Matrix (Fin 8) (Fin 2) ℝ, Set.range (col Q) = {col Q 0, col Q 1} := fun Q => by
    ext z
    simp only [Set.mem_range, Set.mem_insert_iff, Set.mem_singleton_iff, Fin.exists_fin_two, eq_comm]
  constructor
  · rw [hrange, hrange, r0, r1]
    simp only [col_ofCols_zero, col_ofCols_one]
    apply le_antisymm
    · refine Submodule.span_le.mpr ?_
      rintro z (rfl | rfl)
      · exact Submodule.add_mem _ (Submodule.smul_mem _ _ (Submodule.subset_span (by simp)))
          (Submodule.smul_mem _ _ (Submodule.subset_span (by simp)))
      · exact Submodule.sub_mem _ (Submodule.smul_mem _ _ (Submodule.subset_span (by simp)))
          (Submodule.smul_mem _ _ (Submodule.subset_span (by simp)))
    · refine Submodule.span_le.mpr ?_
      set A := Real.cos θ • P.e + Real.sin θ • P.f
      set B := Real.cos θ • P.f - Real.sin θ • P.e
      have hA : A ∈ Submodule.span ℝ {A, B} := Submodule.subset_span (by simp)
      have hB : B ∈ Submodule.span ℝ {A, B} := Submodule.subset_span (by simp)
      have hcs : Real.cos θ * Real.cos θ + Real.sin θ * Real.sin θ = 1 := by
        nlinarith [Real.sin_sq_add_cos_sq θ]
      rintro z (rfl | rfl)
      · have : P.e = Real.cos θ • A - Real.sin θ • B := by
          ext j
          simp only [A, B, PiLp.add_apply, PiLp.sub_apply, PiLp.smul_apply, smul_eq_mul]
          linear_combination (-(P.e j)) * hcs
        rw [this]
        exact Submodule.sub_mem _ (Submodule.smul_mem _ _ hA) (Submodule.smul_mem _ _ hB)
      · have : P.f = Real.sin θ • A + Real.cos θ • B := by
          ext j
          simp only [A, B, PiLp.add_apply, PiLp.sub_apply, PiLp.smul_apply, smul_eq_mul]
          linear_combination (-(P.f j)) * hcs
        rw [this]
        exact Submodule.add_mem _ (Submodule.smul_mem _ _ hA) (Submodule.smul_mem _ _ hB)
  · intro h
    have h' := congrArg (fun M => ⟪P.f, col M 0⟫_ℝ) h
    simp only [r0, col_ofCols_zero, inner_add_right, real_inner_smul_right, P.inner_fe, P.inner_ff, mul_zero, zero_add,
      mul_one] at h'
    exact hθ h'

end ChartSpace
