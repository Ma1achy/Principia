/-
§6b of `chart-space-model.md`: initial-condition, dynamical and shared quantities, and Theorem 11.

The flow is abstract, as the owner ordered: a map `F t` on a state space, with the trajectory's survival a predicate,
and the reversibility identity `F_{−T} = R ∘ F_T ∘ R` a stated hypothesis, not derived from the equations of motion.
The app's approximate inverse (a non-reversible integrator) stays out of Lean, as §8 does.
-/
import ChartSpace.Signed

open scoped unitInterval

noncomputable section

namespace ChartSpace

/-- **Added hypotheses A9** (the abstract flow, by order): a flow `F` on a state space `X`, defined where the trajectory
survives (`Survives t x`: no triple collision or escape on the time interval between `0` and `t`), with the momentum
flip `R`. Each field is a hypothesis the document uses without stating, except `rev`, which it states as a fact of
Newtonian dynamics. -/
structure RevFlow (X : Type*) [TopologicalSpace X] where
  /-- The state after time `t`. -/
  F : ℝ → X → X
  /-- The trajectory from `x` survives to time `t`. -/
  Survives : ℝ → X → Prop
  /-- The momentum flip. -/
  R : X → X
  R_invol : ∀ x, R (R x) = x
  /-- `F_0` is the identity. -/
  zero : ∀ x, F 0 x = x
  /-- A surviving trajectory can be run back: `F_{−t} ∘ F_t = id`, and the way back survives. -/
  inv : ∀ t x, Survives t x → Survives (-t) (F t x) ∧ F (-t) (F t x) = x
  /-- Survival backwards from `x` is survival forwards from `R x`. -/
  rev_survives : ∀ t x, Survives (-t) x ↔ Survives t (R x)
  /-- **The reversibility identity**, `F_{−t} = R ∘ F_t ∘ R`, where the trajectory survives. -/
  rev : ∀ t x, Survives t (R x) → F (-t) x = R (F t (R x))
  /-- Each `F t` is continuous where the trajectory survives. -/
  cont : ∀ t, ContinuousOn (F t) {x | Survives t x}

variable {X : Type*} [TopologicalSpace X]

/-- §6b: `φ_T(u) = F_{−T}(ψ(u))`, in §0's form: the chart `ψ` maps into the state space itself, so no encode. -/
def finalChart (Fl : RevFlow X) (T : ℝ) (ψ : C(U, X)) : U → X := fun u => Fl.F (-T) (ψ u)

/-- **Theorem 11**, §0 form, with where the trajectory must survive. For the pixels whose backward trajectory survives
`[0, T]`, the set `dom`:
- `dom` is the set where the forward trajectory of the flipped state survives `[0, T]`;
- `φ_T` is computed by the forward flow: flip the momenta, run forward by `T`, flip back;
- the start `φ_T(u)` survives to `T` and is at `ψ(u)` then;
- it is the only start that survives to `T` and is at `ψ(u)` then ("which starts lead there");
- `φ_T` is continuous on `dom` (a chart there, masked off it);
- `T = 0` gives `ψ`;
- a conserved quantity reads the same at `φ_T(u)` and `ψ(u)`. -/
theorem theorem11 (Fl : RevFlow X) (T : ℝ) (ψ : C(U, X)) :
    let dom := {u | Fl.Survives (-T) (ψ u)}
    dom = {u | Fl.Survives T (Fl.R (ψ u))} ∧
      (∀ u ∈ dom, finalChart Fl T ψ u = Fl.R (Fl.F T (Fl.R (ψ u)))) ∧
      (∀ u ∈ dom, Fl.Survives T (finalChart Fl T ψ u) ∧ Fl.F T (finalChart Fl T ψ u) = ψ u) ∧
      (∀ u ∈ dom, ∀ x, Fl.Survives T x → (Fl.F T x = ψ u ↔ x = finalChart Fl T ψ u)) ∧
      ContinuousOn (finalChart Fl T ψ) dom ∧
      finalChart Fl 0 ψ = ψ ∧
      ∀ q : X → ℝ, (∀ t x, Fl.Survives t x → q (Fl.F t x) = q x) →
        ∀ u ∈ dom, q (finalChart Fl T ψ u) = q (ψ u) := by
  intro dom
  have back : ∀ u ∈ dom, Fl.Survives T (finalChart Fl T ψ u) ∧ Fl.F T (finalChart Fl T ψ u) = ψ u := by
    intro u hu
    have := Fl.inv (-T) (ψ u) hu
    rw [neg_neg] at this
    exact this
  refine ⟨Set.ext fun u => Fl.rev_survives T (ψ u), fun u hu => Fl.rev T (ψ u) ((Fl.rev_survives T _).mp hu), back,
    fun u hu x hx => ⟨fun h => ?_, fun h => h ▸ (back u hu).2⟩, ?_, ?_, ?_⟩
  · have := (Fl.inv T x hx).2
    rw [h] at this
    exact this.symm
  · exact (Fl.cont (-T)).comp ψ.continuous.continuousOn fun u hu => hu
  · funext u
    simp [finalChart, Fl.zero]
  · intro q hq u hu
    have := hq T _ (back u hu).1
    rw [(back u hu).2] at this
    exact this.symm

/-- §6b over `Z`: `φ_T(u) = Enc(F_{−T}(D(ψ(u))))`. Its pixel shows `D(φ_T(u))`, which reaches `D(ψ(u))` at `T` only if the
decode undoes the encode on the start: **added hypothesis A10**, which §0.1 says fails for Principia's decode (it
canonicalises and saturates). -/
theorem theorem11_latent (Fl : RevFlow X) (T : ℝ) (D : Z → X) (Enc : X → Z) (ψ : Chart) (u : U)
    (hu : Fl.Survives (-T) (D (ψ u))) (hDE : D (Enc (Fl.F (-T) (D (ψ u)))) = Fl.F (-T) (D (ψ u))) :
    Fl.F T (D (Enc (Fl.F (-T) (D (ψ u))))) = D (ψ u) := by
  rw [hDE]
  have := (Fl.inv (-T) _ hu).2
  rwa [neg_neg] at this

/-! ### §6b item 2: what the gauge keeps -/

/-- §6b item 2: under the scale gauge `g` (to `I = 1`), which divides `E` by `c > 0` and multiplies `L_z²` by `c`,
`L_z² E` and the sign of `E` are kept. -/
theorem gauge_keeps {Y : Type*} (E L : Y → ℝ) (g : Y → Y) (c : Y → ℝ) (hc : ∀ x, 0 < c x) (hE : ∀ x, E (g x) = E x / c x)
    (hL : ∀ x, L (g x) ^ 2 = c x * L x ^ 2) (x : Y) :
    L (g x) ^ 2 * E (g x) = L x ^ 2 * E x ∧ SignType.sign (E (g x)) = SignType.sign (E x) := by
  have hcx := (hc x).ne'
  refine ⟨by rw [hL, hE]; field_simp, ?_⟩
  rw [hE]
  rcases lt_trichotomy (E x) 0 with h | h | h
  · rw [sign_neg h, sign_neg (div_neg_of_neg_of_pos h (hc x))]
  · rw [h, zero_div]
  · rw [sign_pos h, sign_pos (div_pos h (hc x))]

/-- §6b item 2: the gauge doesn't keep `E` itself, unless `c = 1` or `E = 0`. -/
theorem gauge_moves_E {Y : Type*} (E : Y → ℝ) (g : Y → Y) (c : Y → ℝ) (hE : ∀ x, E (g x) = E x / c x) (x : Y) (hc : c x ≠ 1)
    (hc0 : c x ≠ 0) (h0 : E x ≠ 0) : E (g x) ≠ E x := by
  rw [hE]
  intro h
  rw [div_eq_iff hc0] at h
  exact hc (mul_left_cancel₀ h0 (h.symm.trans (mul_one _).symm))

/-- §0.5: the encode's mirror `M` keeps `E` and flips `L_z`, so it keeps `L_z² E` and doesn't keep `L_z`'s sign. **Finding
F13**: §6b's limits bullet, "the signs of `E` and `L_z`" independent of `T`, contradicts §0.5 and §6b item 2, where
only `L_z² E` and the sign of `E` are kept: the sign of `L_z` is kept only where no mirror is applied. -/
theorem mirror_flips_Lz {Y : Type*} (E L : Y → ℝ) (M : Y → Y) (hE : ∀ x, E (M x) = E x) (hL : ∀ x, L (M x) = -L x) (x : Y)
    (h0 : L x ≠ 0) :
    L (M x) ^ 2 * E (M x) = L x ^ 2 * E x ∧ SignType.sign (L (M x)) ≠ SignType.sign (L x) := by
  refine ⟨by rw [hL, hE]; ring, ?_⟩
  rw [hL, Left.sign_neg]
  rcases lt_or_gt_of_ne h0 with h | h
  · rw [sign_neg h]; decide
  · rw [sign_pos h]; decide

end ChartSpace
