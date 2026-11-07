/-
§1 of `chart-space-model.md`: the objects. Every other file of this library builds on these definitions.
-/
import Mathlib.Analysis.Complex.Tietze
import Mathlib.Analysis.InnerProductSpace.PiL2
import Mathlib.Analysis.Normed.Module.FiniteDimension
import Mathlib.Analysis.SpecialFunctions.Pow.Continuity
import Mathlib.Analysis.SpecialFunctions.Trigonometric.Deriv
import Mathlib.LinearAlgebra.Matrix.Rank
import Mathlib.LinearAlgebra.Matrix.ToLinearEquiv
import Mathlib.Topology.CompactOpen
import Mathlib.Topology.Homotopy.Basic
import Mathlib.Topology.Instances.Matrix
import Mathlib.Topology.Order.IntermediateValue
import Mathlib.Topology.TietzeExtension

open scoped unitInterval

noncomputable section

namespace ChartSpace

/-- §1 **Latent space**: `Z = R^8`. -/
abbrev Z : Type := EuclideanSpace ℝ (Fin 8)

/-- The plane the pixel coordinates live in. The document only names its square `U`; the plane is needed to say that
`u + s a` (pan) and `k u` (zoom) leave it. -/
abbrev Plane : Type := Fin 2 → ℝ

/-- §1 **Square**, as a set: `[-1,1]^2`. -/
def squareSet : Set Plane := Set.Icc (-1) 1

/-- §1 **Square**: `U = [-1,1]^2`, the view's pixel coordinates. -/
abbrev U : Type := squareSet

theorem mem_squareSet {u : Plane} : u ∈ squareSet ↔ ∀ i, u i ∈ Set.Icc (-1 : ℝ) 1 := by
  simp only [squareSet, Set.mem_Icc, Pi.le_def, Pi.neg_apply, Pi.one_apply]
  exact ⟨fun h i => ⟨h.1 i, h.2 i⟩, fun h => ⟨fun i => (h i).1, fun i => (h i).2⟩⟩

theorem isClosed_squareSet : IsClosed squareSet := isClosed_Icc

theorem isCompact_squareSet : IsCompact squareSet := isCompact_Icc

instance : CompactSpace U := isCompact_iff_compactSpace.mp isCompact_squareSet

theorem zero_mem_squareSet : (0 : Plane) ∈ squareSet :=
  mem_squareSet.mpr fun _ => by norm_num

/-- §1: "The centre pixel is `u = 0`." -/
def centre : U := ⟨0, zero_mem_squareSet⟩

/-- §1 **Chart space**: `𝒞 = C(U, Z)`, and a **chart (as a view)** is a point of it. -/
abbrev Chart : Type := C(U, Z)

/-- §1 **Path (edit)**: a continuous `H : [0,1] × U → Z` from `H₀` to `H₁`, "a homotopy of charts". This is Mathlib's
`ContinuousMap.Homotopy` with nothing added. -/
abbrev EditPath (φ ψ : Chart) : Type := ContinuousMap.Homotopy φ ψ

/-- §1: "Every frame `H_s` is itself a chart." -/
def frame {φ ψ : Chart} (H : EditPath φ ψ) (s : I) : Chart :=
  ⟨fun u => H (s, u), by fun_prop⟩

@[simp] theorem frame_apply {φ ψ : Chart} (H : EditPath φ ψ) (s : I) (u : U) : frame H s u = H (s, u) := rfl

@[simp] theorem frame_zero {φ ψ : Chart} (H : EditPath φ ψ) : frame H 0 = φ := by
  ext1 u; simp

@[simp] theorem frame_one {φ ψ : Chart} (H : EditPath φ ψ) : frame H 1 = ψ := by
  ext1 u; simp

/-- §1: a path of the document's form *is* a continuous path in `𝒞` (compact-open topology), one way. -/
def EditPath.toPath {φ ψ : Chart} (H : EditPath φ ψ) : Path φ ψ where
  toFun := frame H
  continuous_toFun := by
    have : Continuous fun s : I => (ContinuousMap.curry H.toContinuousMap) s :=
      (ContinuousMap.curry H.toContinuousMap).continuous
    exact this
  source' := frame_zero H
  target' := frame_one H

/-- §1: a continuous path in `𝒞` is a path of the document's form, the other way. `U` is compact, so the evaluation
map is continuous. -/
def EditPath.ofPath {φ ψ : Chart} (γ : Path φ ψ) : EditPath φ ψ where
  toFun p := γ p.1 p.2
  continuous_toFun := by
    have : Continuous fun p : C(U, Z) × U => p.1 p.2 := continuous_eval
    exact this.comp ((γ.continuous.comp continuous_fst).prodMk continuous_snd)
  map_zero_left u := by simp
  map_one_left u := by simp

/-- §1 **Affine charts**, the coordinate action: `Q u` for `u` in the plane, as a point of `Z`. -/
def matVec (Q : Matrix (Fin 8) (Fin 2) ℝ) (u : Plane) : Z := WithLp.toLp 2 (Q.mulVec u)

/-- `Q u` is continuous in `u`. -/
theorem continuous_matVec (Q : Matrix (Fin 8) (Fin 2) ℝ) : Continuous (matVec Q) := by
  unfold matVec
  exact (PiLp.continuous_toLp 2 _).comp (Continuous.matrix_mulVec continuous_const continuous_id)

/-- The columns of an `8×2` matrix, as points of `Z`. -/
def col (Q : Matrix (Fin 8) (Fin 2) ℝ) (i : Fin 2) : Z := WithLp.toLp 2 fun j => Q j i

theorem matVec_eq (Q : Matrix (Fin 8) (Fin 2) ℝ) (u : Plane) : matVec Q u = u 0 • col Q 0 + u 1 • col Q 1 := by
  ext j
  simp [matVec, col, Matrix.mulVec, dotProduct, Fin.sum_univ_two]
  ring

@[simp] theorem matVec_zero (Q : Matrix (Fin 8) (Fin 2) ℝ) : matVec Q 0 = 0 := by
  simp [matVec_eq]

/-- §1 **Affine charts**: `φ(u) = z₀ + Q u`. The rank is the separate predicate `IsAffineChart`, because a frame of a
straight-line morph has this form with a rank below 2 (Theorem 2). -/
def affine (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ) : Chart :=
  ⟨fun u => z₀ + matVec Q u, continuous_const.add ((continuous_matVec Q).comp continuous_subtype_val)⟩

@[simp] theorem affine_apply (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ) (u : U) :
    affine z₀ Q u = z₀ + matVec Q u := rfl

/-- §1: a chart is an **affine chart** when it is `z₀ + Q u` with `Q` of rank 2. -/
def IsAffineChart (φ : Chart) : Prop :=
  ∃ (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ), Q.rank = 2 ∧ φ = affine z₀ Q

end ChartSpace
