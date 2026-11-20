/-
§5 of `chart-space-model.md`: the lock, based homotopies, excursions and the sliding anchor.
-/
import ChartSpace.Tilt

open scoped unitInterval

noncomputable section

namespace ChartSpace

/-! ### The lock, snap-back and additive excursions -/

/-- §5 **Lock state**: an anchor `a`, an excursion `δ`, and a chart with `φ(0) = a + δ`. -/
@[ext]
structure Locked where
  /-- The chart in view. -/
  chart : Chart
  /-- The anchor. -/
  anchor : Z
  /-- The excursion. -/
  δ : Z
  centre_eq : chart centre = anchor + δ

/-- §5: a slice step by `d` moves the chart by `d` and adds `d` to the excursion. -/
def Locked.slice (st : Locked) (d : Z) : Locked where
  chart := sliced st.chart d
  anchor := st.anchor
  δ := st.δ + d
  centre_eq := by rw [sliced_apply, st.centre_eq, add_assoc]

/-- §5 **Snap-back**: `δ ← 0`. The document names only the assignment. **Added hypothesis A7**: a lock state's chart must
follow it, or `φ(0) = a + δ` breaks, so the chart is translated back by `δ`. -/
def Locked.snap (st : Locked) : Locked where
  chart := sliced st.chart (-st.δ)
  anchor := st.anchor
  δ := 0
  centre_eq := by rw [sliced_apply, st.centre_eq]; simp

/-- §5: "Setting `δ ← 0` restores `φ(0) = a` exactly." -/
theorem snap_restores_anchor (st : Locked) : st.snap.chart centre = st.anchor := by
  rw [st.snap.centre_eq]; simp [Locked.snap]

/-- §5: "Excursions compose additively (`δ₁ + δ₂`)." -/
theorem slice_slice (st : Locked) (d₁ d₂ : Z) : (st.slice d₁).slice d₂ = st.slice (d₁ + d₂) := by
  ext1
  · ext1 u; simp [Locked.slice, add_assoc]
  · rfl
  · simp [Locked.slice, add_assoc]

/-- §5, slice steps commute. -/
theorem slice_comm (st : Locked) (d₁ d₂ : Z) : (st.slice d₁).slice d₂ = (st.slice d₂).slice d₁ := by
  rw [slice_slice, slice_slice, add_comm]

/-- §5: "any order of slice steps followed by snap-back returns the same state." Two reorderings of the same steps
reach the same state, before the snap as well as after it. -/
theorem slices_any_order (st : Locked) {l l' : List Z} (h : l.Perm l') :
    l.foldl Locked.slice st = l'.foldl Locked.slice st := by
  have hcomm : ∀ (b : Locked) (x y : Z), (b.slice x).slice y = (b.slice y).slice x := slice_comm
  have : RightCommutative Locked.slice := ⟨hcomm⟩
  exact h.foldl_eq st

/-- §5: more than the document asks: after snap-back the state doesn't depend on the slice steps at all. -/
theorem snap_after_slices (st : Locked) (l : List Z) : (l.foldl Locked.slice st).snap = st.snap := by
  induction l generalizing st with
  | nil => rfl
  | cons d l ih =>
    rw [List.foldl_cons, ih]
    ext1
    · ext1 u
      simp [Locked.snap, Locked.slice]
    · rfl
    · rfl

/-! ### Theorem 6 -/

/-- Rank 2 makes `u ↦ Q u` injective. -/
theorem matVec_ne_zero {Q : Matrix (Fin 8) (Fin 2) ℝ} (hQ : Q.rank = 2) {v : Plane} (hv : v ≠ 0) : matVec Q v ≠ 0 := by
  intro h
  have hdet : (Q.transpose * Q).det ≠ 0 := (rank_two_iff_det Q).mp hQ
  have h0 : Q.mulVec v = 0 := by
    have := congrArg (WithLp.ofLp) h
    simpa [matVec] using this
  have h1 : (Q.transpose * Q).mulVec v = 0 := by
    rw [← Matrix.mulVec_mulVec, h0, Matrix.mulVec_zero]
  exact hv (Matrix.eq_zero_of_mulVec_eq_zero hdet h1)

/-- §5 **Theorem 6**, the tilt clause, as stated: tilt of an affine chart about the centre keeps `φ(0) = z₀`. -/
theorem theorem6_tilt (z₀ : Z) (Q : Matrix (Fin 8) (Fin 2) ℝ) (P : RotPlane) (θ : ℝ) (s : I) :
    tiltPath z₀ Q P θ (s, centre) = z₀ := by
  simp [tiltPath_apply, centre]

/-- §5 **Theorem 6**, the zoom clause, as stated, with hypothesis A1 for `k > 1`: `φ(k^s · 0) = φ(0)`. For `k ≤ 1` no
hypothesis is needed: `zoomInPath_centre`. -/
theorem theorem6_zoom (Φ : ExtChart) (k : ℝ) (hk : 0 < k) (s : I) : zoomPath Φ k hk (s, centre) = Φ 0 := by
  change Φ (k ^ (s : ℝ) • (0 : Plane)) = Φ 0
  rw [smul_zero]

/-- §5 **Theorem 6**, the pan clause, in the strong form: for *every* affine chart, every non-zero offset and every
`s > 0`, pan moves the centre. So "`φ(s a') ≠ φ(0)` in general" holds with no exceptions among affine charts. -/
theorem theorem6_pan_affine (z₀ : Z) {Q : Matrix (Fin 8) (Fin 2) ℝ} (hQ : Q.rank = 2) {a' : Plane} (ha : a' ≠ 0) (s : I)
    (hs : (s : ℝ) ≠ 0) : panPath (affineExt z₀ Q) a' (s, centre) ≠ z₀ := by
  change affineExt z₀ Q ((0 : Plane) + (s : ℝ) • a') ≠ z₀
  rw [zero_add, affineExt_apply]
  intro h
  exact matVec_ne_zero hQ (smul_ne_zero hs ha) (by simpa using h)

/-- §5 **Theorem 6**, the pan clause, as an explicit counterexample to "pan keeps the anchor": the rank-2 affine chart
`u ↦ [e₁ e₂] u`, centred on 0, and the pan offset `(1, 1)`: at `s = 1` its centre has left 0. -/
theorem theorem6_pan_counterexample :
    ∃ (Φ : ExtChart) (a' : Plane), IsAffineChart Φ.view ∧ Φ.view centre = 0 ∧ panPath Φ a' (1, centre) ≠ 0 := by
  refine ⟨affineExt 0 frameE, fun _ => 1, ⟨0, frameE, rank_frameE, rfl⟩, by simp [centre], ?_⟩
  exact theorem6_pan_affine (0 : Z) rank_frameE (fun h => one_ne_zero (congrFun h 0)) 1 one_ne_zero

/-- §5 **Theorem 6**, the slice clause, as stated: with `δ = 0` at the start, a slice moves the centre to `a + s δ'`,
which is the excursion: it changes `δ`, and keeps `a` only if `s δ' = 0`. -/
theorem theorem6_slice (φ : Chart) (a δ' : Z) (ha : φ centre = a) (s : I) :
    slicePath φ δ' (s, centre) = a + (s : ℝ) • δ' ∧ (slicePath φ δ' (s, centre) = a ↔ (s : ℝ) • δ' = 0) := by
  rw [slicePath_apply, ha]
  exact ⟨rfl, by simp⟩

/-! ### Theorem 7 -/

/-- §5 **Based morph**: a path with `H_s(0) = a` for all `s`. -/
def IsBased {φ ψ : Chart} (H : EditPath φ ψ) (a : Z) : Prop := ∀ s, H (s, centre) = a

/-- §5: `B(s,u) = a + (1−s)(φ(u) − a) + s(ψ(u) − a)`. -/
def basedMorph (φ ψ : Chart) (a : Z) : EditPath φ ψ where
  toFun p := a + (1 - (p.1 : ℝ)) • (φ p.2 - a) + (p.1 : ℝ) • (ψ p.2 - a)
  continuous_toFun := by fun_prop
  map_zero_left u := by simp
  map_one_left u := by simp

@[simp] theorem basedMorph_apply (φ ψ : Chart) (a : Z) (s : I) (u : U) :
    basedMorph φ ψ a (s, u) = a + (1 - (s : ℝ)) • (φ u - a) + (s : ℝ) • (ψ u - a) := rfl

/-- Finding F9: `B` is `L`. The two formulas are the same map, for any `a`. -/
theorem basedMorph_eq_line (φ ψ : Chart) (a : Z) (s : I) (u : U) :
    basedMorph φ ψ a (s, u) = lineMorph φ ψ (s, u) := by
  simp only [basedMorph_apply, lineMorph_apply, smul_sub, sub_smul, one_smul]
  abel

/-- §5 **Theorem 7**, the forward direction, as stated: if `φ(0) = ψ(0) = a` then `B` is a based path. -/
theorem theorem7_forward {φ ψ : Chart} {a : Z} (hφ : φ centre = a) (hψ : ψ centre = a) :
    IsBased (basedMorph φ ψ a) a := fun s => by
  simp [hφ, hψ]

/-- §5 **Theorem 7**, the title, as stated: a based morph exists iff both ends have the anchor at the centre. -/
theorem theorem7_iff {φ ψ : Chart} {a : Z} :
    (∃ H : EditPath φ ψ, IsBased H a) ↔ φ centre = a ∧ ψ centre = a := by
  constructor
  · rintro ⟨H, hH⟩
    exact ⟨by simpa using hH 0, by simpa using hH 1⟩
  · rintro ⟨h1, h2⟩
    exact ⟨_, theorem7_forward h1 h2⟩

/-- §5 **Theorem 7**, the converse, as stated: if no chart of `ψ`'s family `F` has `a` at its centre, no based path ends
in that family. -/
theorem theorem7_converse {F : Set Chart} {a : Z} (hF : ∀ χ ∈ F, χ centre ≠ a) (φ : Chart) :
    ¬ ∃ χ ∈ F, ∃ H : EditPath φ χ, IsBased H a := by
  rintro ⟨χ, hχ, H, hH⟩
  exact hF χ hχ (theorem7_iff.mp ⟨H, hH⟩).2

/-- §5, the constrained-chart case: "If `a ∉ C_ψ`, Theorem 7 forbids a based morph." -/
theorem constrained_forbids_based {C : Set Z} {a : Z} (ha : a ∉ C) (φ : Chart) :
    ¬ ∃ χ : Chart, (∀ u, χ u ∈ C) ∧ ∃ H : EditPath φ χ, IsBased H a := by
  rintro ⟨χ, hχ, H, hH⟩
  have := (theorem7_iff.mp ⟨H, hH⟩).2
  exact ha (this ▸ hχ centre)

/-- Finding F10: Theorem 7's title and §9 say "contain"; the theorem needs "at the centre". A chart can show `a` at a
pixel off the centre and still have no based morph. -/
theorem contains_is_not_enough (a : Z) (φ : Chart) :
    ∃ χ : Chart, a ∈ Set.range χ ∧ ¬ ∃ H : EditPath φ χ, IsBased H a := by
  let v : Plane := fun _ => 1
  have hv : v ∈ squareSet := mem_squareSet.mpr fun _ => by simp [v]
  let χ : Chart := affine (a - matVec frameE v) frameE
  refine ⟨χ, ⟨⟨v, hv⟩, by simp [χ]⟩, fun ⟨H, hH⟩ => ?_⟩
  have h := (theorem7_iff.mp ⟨H, hH⟩).2
  have h' : a - matVec frameE v = a := by simpa [χ, centre] using h
  have : matVec frameE v = 0 := by simpa using h'
  exact matVec_ne_zero rank_frameE (fun h => one_ne_zero (congrFun h 0)) this

/-! ### Theorem 8 -/

/-- §5: `π : Z → C` is a continuous retraction onto `C`. -/
structure IsRetraction (π : Z → Z) (C : Set Z) : Prop where
  continuous : Continuous π
  maps : ∀ z, π z ∈ C
  fixes : ∀ z ∈ C, π z = z

/-- §9's "slides continuously to the target's nearest admissible point", and Theorem 8's `π`, ask for something of `C`
(finding F5): a continuous retraction of `Z` exists only onto a closed, connected set. -/
theorem IsRetraction.isClosed {π : Z → Z} {C : Set Z} (hπ : IsRetraction π C) : IsClosed C := by
  have : C = {z | π z = z} := Set.ext fun z => ⟨hπ.fixes z, fun h => h ▸ hπ.maps z⟩
  rw [this]
  exact isClosed_eq hπ.continuous continuous_id

/-- Finding F5, as above. -/
theorem IsRetraction.isPreconnected {π : Z → Z} {C : Set Z} (hπ : IsRetraction π C) : IsPreconnected C := by
  have : C = Set.range π :=
    Set.Subset.antisymm (fun z hz => ⟨z, hπ.fixes z hz⟩) (Set.range_subset_iff.mpr hπ.maps)
  rw [this]
  exact isPreconnected_range hπ.continuous

/-- §5: the anchor path `a_s = (1−s) a + s b`. -/
def anchorPath (a b : Z) (s : I) : Z := (1 - (s : ℝ)) • a + (s : ℝ) • b

/-- §5: `S(s,u) = a_s + (1−s)(φ(u) − a) + s(ψ'(u) − b)`, where `b = π(a)` and `ψ' = ψ_{π(a)}`. -/
def slidingMorph (φ ψ' : Chart) (a b : Z) : EditPath φ ψ' where
  toFun p := anchorPath a b p.1 + (1 - (p.1 : ℝ)) • (φ p.2 - a) + (p.1 : ℝ) • (ψ' p.2 - b)
  continuous_toFun := by
    unfold anchorPath
    fun_prop
  map_zero_left u := by simp [anchorPath]
  map_one_left u := by simp [anchorPath]

@[simp] theorem slidingMorph_apply (φ ψ' : Chart) (a b : Z) (s : I) (u : U) :
    slidingMorph φ ψ' a b (s, u) = anchorPath a b s + (1 - (s : ℝ)) • (φ u - a) + (s : ℝ) • (ψ' u - b) := rfl

/-- Finding F9: `S` is `L` too, for any `a`, `b`: the sliding anchor is the straight-line morph to the re-centred
chart, not a new path. -/
theorem slidingMorph_eq_line (φ ψ' : Chart) (a b : Z) (s : I) (u : U) :
    slidingMorph φ ψ' a b (s, u) = lineMorph φ ψ' (s, u) := by
  simp only [slidingMorph_apply, lineMorph_apply, anchorPath, smul_sub]
  abel

/-- **Finding F4**: the claim "`S(s,0) = a_s`" holds exactly when `φ(0) = a` and `ψ'(0) = b`. The document's Theorem 8
states neither: its proof uses both. -/
theorem sliding_centre_iff (φ ψ' : Chart) (a b : Z) :
    (∀ s, slidingMorph φ ψ' a b (s, centre) = anchorPath a b s) ↔ φ centre = a ∧ ψ' centre = b := by
  constructor
  · intro h
    exact ⟨by simpa [anchorPath] using h 0, by simpa [anchorPath] using h 1⟩
  · rintro ⟨h1, h2⟩ s
    simp [h1, h2]

/-- §5, the re-centring Theorem 8 leaves undefined (finding F4): translate `ψ` so its centre is at `b`, and then apply
`π` so it stays in `C`. -/
def recentre (π : Z → Z) (hπ : Continuous π) (ψ : Chart) (b : Z) : Chart :=
  ⟨fun u => π (ψ u - ψ centre + b), hπ.comp (by fun_prop)⟩

@[simp] theorem recentre_apply (π : Z → Z) (hπ : Continuous π) (ψ : Chart) (b : Z) (u : U) :
    recentre π hπ ψ b u = π (ψ u - ψ centre + b) := rfl

/-- The re-centred chart stays in the constraint set and has `b` at its centre. -/
theorem recentre_spec {π : Z → Z} {C : Set Z} (hπ : IsRetraction π C) (ψ : Chart) {b : Z} (hb : b ∈ C) :
    (∀ u, recentre π hπ.continuous ψ b u ∈ C) ∧ recentre π hπ.continuous ψ b centre = b := by
  refine ⟨fun u => hπ.maps _, ?_⟩
  simp [hπ.fixes b hb]

/-- A chart already in `C` and already centred on `b` is its own re-centring. -/
theorem recentre_self {π : Z → Z} {C : Set Z} (hπ : IsRetraction π C) {ψ : Chart} (hψ : ∀ u, ψ u ∈ C) :
    recentre π hπ.continuous ψ (ψ centre) = ψ := by
  ext1 u
  simp [hπ.fixes _ (hψ u)]

/-- §5 **Theorem 8 (the sliding anchor)**, with the two hypotheses the document's proof uses and its statement omits
(finding F4), marked **A2** (`hφ`: the lock has `δ = 0`) and **A3** (`ψ'` is centred on `π(a)`). `S` is a path from `φ` to
`ψ'`, and its centre is `a_s`, which moves continuously from `a` to `π(a)`. The hypotheses "continuous" and "retraction"
on `π` are not used here: they are what produces a `ψ'` staying in `C` (`theorem8`). -/
theorem theorem8_pointwise {φ ψ' : Chart} {a b : Z} (hφ : φ centre = a) (hψ : ψ' centre = b) :
    (∀ s u, slidingMorph φ ψ' a b (s, u) =
      anchorPath a b s + (1 - (s : ℝ)) • (φ u - a) + (s : ℝ) • (ψ' u - b)) ∧
      (∀ s, slidingMorph φ ψ' a b (s, centre) = anchorPath a b s) ∧
      Continuous (anchorPath a b) ∧ anchorPath a b 0 = a ∧ anchorPath a b 1 = b := by
  refine ⟨fun _ _ => rfl, (sliding_centre_iff φ ψ' a b).mpr ⟨hφ, hψ⟩, ?_, by simp [anchorPath], by simp [anchorPath]⟩
  unfold anchorPath
  fun_prop

/-- §5 **Theorem 8**, the document's setting: `π` a continuous retraction onto `C`, a lock with `δ = 0` (A2), and the
re-centring `ψ_{π(a)}` taken to be `recentre` (A3 is then a theorem). -/
theorem theorem8 {π : Z → Z} {C : Set Z} (hπ : IsRetraction π C) {φ : Chart} {a : Z} (hφ : φ centre = a) (ψ : Chart) :
    let ψ' := recentre π hπ.continuous ψ (π a)
    (∀ u, ψ' u ∈ C) ∧ ψ' centre = π a ∧ ∃ S : EditPath φ ψ', ∀ s, S (s, centre) = anchorPath a (π a) s := by
  intro ψ'
  obtain ⟨h1, h2⟩ := recentre_spec hπ ψ (hπ.maps a)
  exact ⟨h1, h2, slidingMorph φ ψ' a (π a), (sliding_centre_iff _ _ _ _).mpr ⟨hφ, h2⟩⟩

/-- §5, Theorem 8's parenthesis: for coordinate constraints, `π` sets the constrained coordinates to `C`'s values. -/
def pin (I' : Finset (Fin 8)) (c : Z) (z : Z) : Z :=
  WithLp.toLp 2 fun i => if i ∈ I' then c i else z i

/-- The set of points with the pinned coordinates at their values. -/
def pinnedSet (I' : Finset (Fin 8)) (c : Z) : Set Z := {z | ∀ i ∈ I', z i = c i}

/-- §5: the coordinate pin is a continuous retraction, so Theorem 8 applies to every coordinate constraint. -/
theorem pin_isRetraction (I' : Finset (Fin 8)) (c : Z) : IsRetraction (pin I' c) (pinnedSet I' c) where
  continuous := by
    unfold pin
    refine (PiLp.continuous_toLp 2 _).comp (continuous_pi fun i => ?_)
    by_cases hi : i ∈ I' <;> simp only [hi, ite_true, ite_false]
    · exact continuous_const
    · fun_prop
  maps z i hi := by simp [pin, hi]
  fixes z hz := by
    ext i
    by_cases hi : i ∈ I'
    · simp [pin, hi, hz i hi]
    · simp [pin, hi]

/-- §5, the RQ-721 rider: "the pinned system keeps its shape". Along the anchor path, every coordinate not pinned keeps
its value, at every `s`. -/
theorem pin_keeps_free_coordinates (I' : Finset (Fin 8)) (c a : Z) (s : I) {i : Fin 8} (hi : i ∉ I') :
    anchorPath a (pin I' c a) s i = a i := by
  simp only [anchorPath, pin, PiLp.add_apply, PiLp.smul_apply, hi, ite_false, smul_eq_mul]
  ring

end ChartSpace
