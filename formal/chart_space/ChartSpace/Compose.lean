/-
§4 of `chart-space-model.md`: composition, waypoints and easing.
-/
import ChartSpace.Morph

open scoped unitInterval

noncomputable section

namespace ChartSpace

/-! ### Theorem 4 -/

/-- §4 **Theorem 4 (concatenation)**, as stated. If `H` goes from `φ` to `ψ` and `K` from `ψ` to `χ`, the map which is
`H_{2s}` for `s ≤ ½` and `K_{2s−1}` for `s > ½` is a path from `φ` to `χ`. Mathlib's `Homotopy.trans` is that map. -/
theorem theorem4 {φ ψ χ : Chart} (H : EditPath φ ψ) (K : EditPath ψ χ) :
    ∃ J : EditPath φ χ,
      (∀ (s t : I) (u : U), (s : ℝ) ≤ 1 / 2 → (t : ℝ) = 2 * s → J (s, u) = H (t, u)) ∧
      (∀ (s t : I) (u : U), 1 / 2 < (s : ℝ) → (t : ℝ) = 2 * s - 1 → J (s, u) = K (t, u)) := by
  refine ⟨H.trans K, fun s t u hs ht => ?_, fun s t u hs ht => ?_⟩
  · rw [ContinuousMap.Homotopy.trans_apply, dite_eq_left hs]
    congr 2
    exact Subtype.ext ht.symm
  · rw [ContinuousMap.Homotopy.trans_apply, dite_eq_right (not_le.mpr hs)]
    congr 2
    exact Subtype.ext ht.symm

/-- §4 **Keyframes are waypoints**. A Sequence with keyframes `φ₀ … φ_n` and segments `H¹ … Hⁿ` has a concatenation,
the left-nested `(…(H¹·H²)·…)·Hⁿ`, which passes through every keyframe, in order. The document's `H¹ · … · Hⁿ` doesn't
say how to bracket, and the bracketing fixes the keyframes' times (finding F8); the keyframes visited and their order
don't depend on it. -/
theorem sequence_concat (φ : ℕ → Chart) (n : ℕ) (seg : ∀ i, i < n → EditPath (φ i) (φ (i + 1))) :
    ∃ (J : EditPath (φ 0) (φ n)) (t : ℕ → I),
      StrictMonoOn t (Set.Iic n) ∧ ∀ i ≤ n, frame J (t i) = φ i := by
  induction n with
  | zero =>
    refine ⟨ContinuousMap.Homotopy.refl _, fun _ => 0, ?_, ?_⟩
    · intro a ha b hb hab
      simp only [Set.mem_Iic, Nat.le_zero_eq] at ha hb
      omega
    · intro i hi
      obtain rfl : i = 0 := by omega
      simp
  | succ n ih =>
    obtain ⟨J, t, ht, hJ⟩ := ih fun i hi => seg i (by omega)
    let half : I → I := fun x => ⟨(x : ℝ) / 2, by have := x.2.1; positivity, by have := x.2.2; linarith⟩
    refine ⟨J.trans (seg n (by omega)), fun i => if i ≤ n then half (t i) else 1, ?_, ?_⟩
    · intro a ha b hb hab
      simp only [Set.mem_Iic] at ha hb
      have ha' : a ≤ n := by omega
      simp only [ha', ite_true]
      by_cases hbn : b ≤ n
      · simp only [hbn, ite_true]
        have := ht (Set.mem_Iic.mpr ha') (Set.mem_Iic.mpr hbn) hab
        change (t a : ℝ) / 2 < (t b : ℝ) / 2
        have h' : (t a : ℝ) < t b := this
        linarith
      · simp only [hbn, ite_false]
        change (t a : ℝ) / 2 < 1
        have := (t a).2.2
        linarith
    · intro i hi
      by_cases hin : i ≤ n
      · simp only [hin, ite_true]
        ext1 u
        have h1 : ((half (t i) : I) : ℝ) ≤ 1 / 2 := by
          change (t i : ℝ) / 2 ≤ 1 / 2
          have := (t i).2.2
          linarith
        simp only [frame_apply]
        rw [ContinuousMap.Homotopy.trans_apply, dite_eq_left h1, ← hJ i hin, frame_apply]
        congr 2
        apply Subtype.ext
        change 2 * ((t i : ℝ) / 2) = t i
        ring
      · obtain rfl : i = n + 1 := by omega
        simp [show ¬ n + 1 ≤ n by omega]

/-- **Finding F8**: concatenation isn't associative as a path, so `H¹ · … · Hⁿ` is under-specified. Moving along one line
of `Z` at unit steps, the two bracketings of three segments are different charts at `s = ¼`: the keyframe `φ₁` is reached
at `¼` by one and at `½` by the other. -/
theorem concat_not_associative :
    ∃ (φ₀ φ₁ φ₂ φ₃ : Chart) (H : EditPath φ₀ φ₁) (K : EditPath φ₁ φ₂) (J : EditPath φ₂ φ₃) (q : I),
      frame ((H.trans K).trans J) q ≠ frame (H.trans (K.trans J)) q := by
  let e : Z := EuclideanSpace.single 0 (1 : ℝ)
  have he : e 0 = 1 := by simp [e]
  let c : ℝ → Chart := fun r => ContinuousMap.const U (r • e)
  have hquarter : ((⟨1 / 4, by norm_num, by norm_num⟩ : I) : ℝ) = 1 / 4 := rfl
  refine ⟨c 0, c 1, c 2, c 3, lineMorph _ _, lineMorph _ _, lineMorph _ _, ⟨1 / 4, by norm_num, by norm_num⟩, ?_⟩
  intro h
  have h' := congrArg (fun χ : Chart => χ centre 0) h
  simp only [frame_apply] at h'
  have left : ((lineMorph (c 0) (c 1)).trans (lineMorph (c 1) (c 2))).trans (lineMorph (c 2) (c 3))
      (⟨1 / 4, by norm_num, by norm_num⟩, centre) = c 1 centre := by
    rw [ContinuousMap.Homotopy.trans_apply, dite_eq_left (by rw [hquarter]; norm_num)]
    rw [ContinuousMap.Homotopy.trans_apply, dite_eq_left (by norm_num [hquarter])]
    have : (⟨2 * (((⟨2 * ((⟨1 / 4, by norm_num, by norm_num⟩ : I) : ℝ), by norm_num [hquarter]⟩ : I)) : ℝ),
        by norm_num [hquarter]⟩ : I) = 1 := Subtype.ext (by norm_num [hquarter])
    rw [this]
    simp
  have right : (lineMorph (c 0) (c 1)).trans ((lineMorph (c 1) (c 2)).trans (lineMorph (c 2) (c 3)))
      (⟨1 / 4, by norm_num, by norm_num⟩, centre) = (1 / 2 : ℝ) • e := by
    rw [ContinuousMap.Homotopy.trans_apply, dite_eq_left (by rw [hquarter]; norm_num)]
    simp only [lineMorph_apply, c, ContinuousMap.const_apply]
    ext j
    simp only [PiLp.add_apply, PiLp.smul_apply, smul_eq_mul]
    ring
  rw [left, right] at h'
  simp only [c, ContinuousMap.const_apply, PiLp.smul_apply, smul_eq_mul, he] at h'
  norm_num at h'

/-! ### Theorem 5 -/

/-- §4: an **easing**: a monotone continuous `e : [0,1] → [0,1]` with `e(0) = 0` and `e(1) = 1`. Monotone is meant
weakly: plateaus are allowed. -/
structure Easing where
  /-- The function. -/
  e : C(I, I)
  mono : Monotone e
  map_zero : e 0 = 0
  map_one : e 1 = 1

/-- The proof's first step: `e` is a surjection. This uses continuity and the two endpoint conditions. -/
theorem Easing.surjective (E : Easing) : Function.Surjective E.e := fun t =>
  mem_range_of_exists_le_of_exists_ge E.e.continuous
    ⟨0, by rw [E.map_zero]; exact unitInterval.nonneg'⟩
    ⟨1, by rw [E.map_one]; exact unitInterval.le_one'⟩

/-- §4: the eased path `H_{e(s)}`. -/
def eased {φ ψ : Chart} (H : EditPath φ ψ) (E : Easing) : EditPath φ ψ where
  toFun p := H (E.e p.1, p.2)
  continuous_toFun := by fun_prop
  map_zero_left u := by simp [E.map_zero]
  map_one_left u := by simp [E.map_one]

@[simp] theorem eased_apply {φ ψ : Chart} (H : EditPath φ ψ) (E : Easing) (s : I) (u : U) :
    eased H E (s, u) = H (E.e s, u) := rfl

theorem frame_eased {φ ψ : Chart} (H : EditPath φ ψ) (E : Easing) (s : I) :
    frame (eased H E) s = frame H (E.e s) := rfl

/-- "Visits the charts `c₀, c₁, …, c_{n−1}` in that order", for any finite list of charts, repeats allowed: there are
parameters `t₀ ≤ t₁ ≤ …` with `γ(tᵢ) = cᵢ`. A path may revisit a chart, so the order of a path's charts is the set of
all such lists, not a relation on charts alone. -/
def VisitsInOrder (γ : I → Chart) {n : ℕ} (c : Fin n → Chart) : Prop :=
  ∃ t : Fin n → I, Monotone t ∧ ∀ i, γ (t i) = c i

/-- `e` has a strictly increasing right inverse. -/
theorem Easing.exists_section (E : Easing) : ∃ g : I → I, StrictMono g ∧ ∀ t, E.e (g t) = t := by
  refine ⟨Function.surjInv E.surjective, fun a b hab => ?_, Function.surjInv_eq E.surjective⟩
  by_contra h
  have := E.mono (not_lt.mp h)
  rw [Function.surjInv_eq E.surjective, Function.surjInv_eq E.surjective] at this
  exact absurd hab (not_lt.mpr this)

/-- §4 **Theorem 5 (easing changes timing, never the frames visited)**, as stated, unweakened: the eased path visits
exactly the same set of charts, and every finite list of charts is visited in order by the one iff by the other, and
it has the same endpoints. -/
theorem theorem5 {φ ψ : Chart} (H : EditPath φ ψ) (E : Easing) :
    Set.range (frame (eased H E)) = Set.range (frame H) ∧
      (∀ (n : ℕ) (c : Fin n → Chart), VisitsInOrder (frame (eased H E)) c ↔ VisitsInOrder (frame H) c) ∧
      frame (eased H E) 0 = φ ∧ frame (eased H E) 1 = ψ := by
  obtain ⟨g, hg, hge⟩ := E.exists_section
  refine ⟨?_, fun n c => ⟨?_, ?_⟩, frame_zero _, frame_one _⟩
  · ext χ
    constructor
    · rintro ⟨s, rfl⟩
      exact ⟨E.e s, rfl⟩
    · rintro ⟨t, rfl⟩
      refine ⟨g t, ?_⟩
      rw [frame_eased, hge]
  · rintro ⟨t, ht, hc⟩
    exact ⟨fun i => E.e (t i), E.mono.comp ht, hc⟩
  · rintro ⟨t, ht, hc⟩
    refine ⟨fun i => g (t i), hg.monotone.comp ht, fun i => ?_⟩
    rw [frame_eased, hge]
    exact hc i

/-- §4 **Corollary** to Theorem 5: "Easing can never move a keyframe." Every list of keyframes the path passes through,
at parameters `t₀ ≤ t₁ ≤ …`, is passed through by the eased path, in the same order, and each keyframe is unchanged.
Only its parameter changes, to a preimage under `e`: this is the "timing" of Theorem 5's title. -/
theorem easing_keeps_keyframes {φ ψ : Chart} (H : EditPath φ ψ) (E : Easing) {n : ℕ} (t : Fin n → I)
    (ht : Monotone t) :
    ∃ s : Fin n → I, Monotone s ∧ (∀ i, E.e (s i) = t i) ∧ ∀ i, frame (eased H E) (s i) = frame H (t i) := by
  obtain ⟨g, hg, hge⟩ := E.exists_section
  exact ⟨fun i => g (t i), hg.monotone.comp ht, fun i => hge _, fun i => by rw [frame_eased, hge]⟩

/-- §4 **Corollary**, the per-segment reading: easing each of two segments on its own keeps the shared keyframe at
exactly the concatenation's time `½`. -/
theorem easing_segments_keeps_midpoint {φ ψ χ : Chart} (H : EditPath φ ψ) (K : EditPath ψ χ) (E₁ E₂ : Easing)
    (half : I) (hhalf : (half : ℝ) = 1 / 2) : frame ((eased H E₁).trans (eased K E₂)) half = ψ := by
  ext1 u
  have h1 : (half : ℝ) ≤ 1 / 2 := hhalf.le
  simp only [frame_apply]
  rw [ContinuousMap.Homotopy.trans_apply, dite_eq_left h1]
  have : (⟨2 * (half : ℝ), by rw [hhalf]; norm_num⟩ : I) = 1 := Subtype.ext (by simp [hhalf])
  simp [this]

end ChartSpace
