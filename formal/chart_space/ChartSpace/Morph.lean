/-
§2 of `chart-space-model.md`: every edit is a path, and a path always exists.
-/
import ChartSpace.Basic

open scoped unitInterval

noncomputable section

namespace ChartSpace

/-- §2 **the straight-line morph** `L(s,u) = (1−s)·φ(u) + s·ψ(u)`, as a path from `φ` to `ψ`. -/
def lineMorph (φ ψ : Chart) : EditPath φ ψ where
  toFun p := (1 - (p.1 : ℝ)) • φ p.2 + (p.1 : ℝ) • ψ p.2
  continuous_toFun := by fun_prop
  map_zero_left u := by simp
  map_one_left u := by simp

@[simp] theorem lineMorph_apply (φ ψ : Chart) (s : I) (u : U) :
    lineMorph φ ψ (s, u) = (1 - (s : ℝ)) • φ u + (s : ℝ) • ψ u := rfl

/-- §2 **Theorem 1**, as stated. For any two charts there is a path; the straight-line morph is one; and, with `D`
total (a function) and continuous, every pixel's system moves continuously along it. Joint continuity in `(s,u)` is
stated too: the document's proof gives it, though its statement only needs the pixelwise form. -/
theorem theorem1 {Sys : Type*} [TopologicalSpace Sys] (D : Z → Sys) (hD : Continuous D) (φ ψ : Chart) :
    Nonempty (EditPath φ ψ) ∧
      (∃ L : EditPath φ ψ, ∀ s u, L (s, u) = (1 - (s : ℝ)) • φ u + (s : ℝ) • ψ u) ∧
      (∀ u, Continuous fun s : I => D (lineMorph φ ψ (s, u))) ∧
      Continuous fun p : I × U => D (lineMorph φ ψ p) :=
  ⟨⟨lineMorph φ ψ⟩, ⟨lineMorph φ ψ, fun _ _ => rfl⟩,
    fun u => hD.comp ((lineMorph φ ψ).toContinuousMap.continuous.comp (by fun_prop)),
    hD.comp (lineMorph φ ψ).toContinuousMap.continuous⟩

/-- §1, a consequence of Theorem 1: `𝒞` is path-connected in its compact-open topology. -/
theorem chartSpace_pathConnected : PathConnectedSpace Chart where
  nonempty := ⟨ContinuousMap.const U 0⟩
  joined φ ψ := ⟨(lineMorph φ ψ).toPath⟩

/-- §2 **Corollary 1**: no frame is invalid. In this model `D` is a function, so the claim is the statement that
every frame's every pixel has a decoded system. It is true by totality and has no further content here; the masks are
not modelled. -/
theorem corollary1 {Sys : Type*} (D : Z → Sys) (φ ψ : Chart) (s : I) (u : U) :
    ∃ y : Sys, y = D (lineMorph φ ψ (s, u)) := ⟨_, rfl⟩

/-- §8 against §1 (finding F7). A chart with a seam (a discontinuous `φ`, which §8 says `E ∘ Φ` can be) is not a chart
of §1, and it can't be a frame of any path of §1: frame 0 of a jointly continuous map is continuous. -/
theorem seam_is_not_a_frame (φ : U → Z) (hφ : ¬ Continuous φ) :
    ¬ ∃ H : C(I × U, Z), ∀ u, H (0, u) = φ u := by
  rintro ⟨H, hH⟩
  apply hφ
  have : φ = fun u => H (0, u) := funext fun u => (hH u).symm
  rw [this]
  fun_prop

/-- §8 (finding F7): what survives for charts with seams. For arbitrary functions `φ, ψ`, each pixel of the blend
moves continuously in `s`. This is the true reading of §8's "the morph … is continuous in `Z`": continuity in `s` at
each pixel, not the joint continuity of §1. -/
theorem blend_continuous_in_s (φ ψ : U → Z) (u : U) :
    Continuous fun s : I => (1 - (s : ℝ)) • φ u + (s : ℝ) • ψ u := by
  fun_prop

end ChartSpace
