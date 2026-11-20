/-
§6 of `chart-space-model.md`: time as an axis.

§6 writes `τ(u) = (φ(u₁), t(u₂))`, applying `φ` to one coordinate, while §1 gives charts a two-dimensional domain, and
`ι(φ) = (φ, t_play)` then uses a chart of §1 (finding F6). This file types the two senses apart: the IC axis of a time
chart is a curve `[-1,1] → Z`, and `curveChart` is the chart of §1 it gives.
-/
import ChartSpace.Morph

open scoped unitInterval

noncomputable section

namespace ChartSpace

/-- The closed interval one pixel coordinate ranges over. -/
abbrev Axis : Type := Set.Icc (-1 : ℝ) 1

/-- The coordinate `u_i` of a pixel, in its axis. -/
def axisOf (i : Fin 2) (u : U) : Axis := ⟨(u : Plane) i, mem_squareSet.mp u.2 i⟩

theorem continuous_axisOf (i : Fin 2) : Continuous (axisOf i) := by
  unfold axisOf
  exact Continuous.subtype_mk ((continuous_apply i).comp continuous_subtype_val) _

/-- §6: the target `Z × [0,T]` of a time chart, and a map from `U` into it. -/
abbrev TimeChart (T : ℝ) : Type := C(U, Z × Set.Icc (0 : ℝ) T)

/-- §6: `τ(u) = (φ(u₁), t(u₂))`, with `φ` typed as the curve the formula needs: **added hypothesis A6** (finding F6). -/
def timeChart {T : ℝ} (φ : C(Axis, Z)) (t : C(Axis, Set.Icc (0 : ℝ) T)) : TimeChart T :=
  ⟨fun u => (φ (axisOf 0 u), t (axisOf 1 u)),
    (φ.continuous.comp (continuous_axisOf 0)).prodMk (t.continuous.comp (continuous_axisOf 1))⟩

/-- §6: the chart of §1 a time chart's IC axis gives: `u ↦ φ(u₁)`. It ignores `u₂`, so it isn't an affine chart. -/
def curveChart (φ : C(Axis, Z)) : Chart := ⟨fun u => φ (axisOf 0 u), φ.continuous.comp (continuous_axisOf 0)⟩

/-- §6: `ι(φ) = (φ, t_play)`, constant time. Here `φ` is a chart of §1. -/
def embedIC {T : ℝ} (φ : Chart) (tp : Set.Icc (0 : ℝ) T) : TimeChart T :=
  ⟨fun u => (φ u, tp), by fun_prop⟩

/-- `[0,T]` is convex: the blend of two times is a time. -/
def timeBlend {T : ℝ} (s : I) (a b : Set.Icc (0 : ℝ) T) : Set.Icc (0 : ℝ) T :=
  ⟨(1 - (s : ℝ)) * a + s * b, by
    have hs := s.2
    have ha := a.2
    have hb := b.2
    constructor
    · nlinarith [hs.1, hs.2, ha.1, hb.1]
    · nlinarith [hs.1, hs.2, ha.2, hb.2]⟩

/-- §6: `Z × [0,T]` is convex, so any two maps into it are joined by their straight-line blend. Theorem 9's two paths
are this one. -/
def timeMorph {T : ℝ} (f g : TimeChart T) : ContinuousMap.Homotopy f g where
  toFun p := ((1 - (p.1 : ℝ)) • (f p.2).1 + (p.1 : ℝ) • (g p.2).1, timeBlend p.1 (f p.2).2 (g p.2).2)
  continuous_toFun := by
    refine Continuous.prodMk (by fun_prop) (Continuous.subtype_mk ?_ _)
    fun_prop
  map_zero_left u := by
    exact Prod.ext (by simp) (Subtype.ext (by simp [timeBlend]))
  map_one_left u := by
    exact Prod.ext (by simp) (Subtype.ext (by simp [timeBlend]))

/-- §6 **Theorem 9**, first clause, as stated once §6's `φ(u₁)` is typed: between time charts `(φ, t)` and `(ψ, t)` with
the same time axis, `(L_s, t)` is a path: the time axis is fixed and the IC axis morphs. -/
theorem theorem9_time_to_time {T : ℝ} (φ ψ : C(Axis, Z)) (t : C(Axis, Set.Icc (0 : ℝ) T)) :
    ∃ H : ContinuousMap.Homotopy (timeChart φ t) (timeChart ψ t),
      ∀ (s : I) (u : U), H (s, u) =
        (lineMorph (curveChart φ) (curveChart ψ) (s, u), t (axisOf 1 u)) := by
  refine ⟨timeMorph _ _, fun s u => Prod.ext rfl (Subtype.ext ?_)⟩
  change (1 - (s : ℝ)) * (t (axisOf 1 u) : ℝ) + s * (t (axisOf 1 u) : ℝ) = _
  ring

/-- §6 **Theorem 9**, second clause, as stated once §6's `φ(u₁)` is typed: between a time chart and `ι(ψ)` the hybrid
`((1−s)φ + sψ, (1−s) t(u₂) + s t_play)` is a path. So a cut isn't forced by the mathematics. -/
theorem theorem9_time_to_IC {T : ℝ} (φ : C(Axis, Z)) (t : C(Axis, Set.Icc (0 : ℝ) T)) (ψ : Chart)
    (tp : Set.Icc (0 : ℝ) T) :
    ∃ H : ContinuousMap.Homotopy (timeChart φ t) (embedIC ψ tp),
      ∀ (s : I) (u : U),
        (H (s, u)).1 = (1 - (s : ℝ)) • φ (axisOf 0 u) + (s : ℝ) • ψ u ∧
          ((H (s, u)).2 : ℝ) = (1 - (s : ℝ)) * (t (axisOf 1 u) : ℝ) + s * (tp : ℝ) :=
  ⟨timeMorph _ _, fun _ _ => ⟨rfl, rfl⟩⟩

end ChartSpace
