# Known-solution validation orbits

**Why these matter:** a periodic orbit **is its own reference.** Integrate one period, compare to the
start. No second implementation, no analytic formula, no tolerance chosen by eye — the orbit closes
or it does not. **That is a test that can fail**, which is the standard everything else in this
project is held to.

And they are cheap: one trajectory each, not 10⁶.

---

## 0. The headline: the figure-eight already found a first-order error

Run before writing this document, on the current AZ + RK4 integrator.

**Chenciner–Montgomery figure-eight** (equal masses, `E = −1.2871419918`, `L_z = 0` exactly,
`Σp = 0` exactly, period `T = 6.32591398`). Integrate one full period and compare to the start:

| `eta` | closure `|dr|` | `|dE/E|` |
|---|---|---|
| 0.02 | 6.163e-02 | 4.47e-12 |
| 0.01 | 3.100e-02 | 4.69e-13 |
| 0.005 | 1.432e-02 | 4.03e-12 |
| 0.002 | **5.759e-03** | 5.83e-12 |

**Energy is conserved to 1e-12. The orbit misses closure by 6e-3.**

And the convergence rate is the finding: a 10× reduction in `eta` improves closure by **10.7×** —
**first order**. RK4 should give `eta⁴`. Something O(`eta`) dominates completely.

**The likely culprit is already documented as a deliberate transcription choice:** the sync-boundary
overshoot — *"the reference clips the time accumulator only, leaving state overshot by up to one
step. Transcribe; don't interpolate back."* That is an O(`dtau`) = O(`eta`) error per boundary.
(Varying `n_sync` at fixed `eta` gives 4.8e-2 → 3.0e-2 across 8→128 boundaries — weakly dependent,
so overshoot is a contributor but may not be the whole story. **Worth isolating.**)

**The general lesson, and it is the one that justifies the suite:** this error is invisible to every
diagnostic currently in the payload. It is **phase error** — displacement *along* the trajectory —
which conserves energy exactly and therefore cannot be seen by `error_ratio` or `worst_energy_drift`.
It is the same blind spot identified independently as `roundtrip_error`. **A closure test sees it in
one run.**

### 0.1 The mechanism, and the fix — measured, not proposed

**Why the error is `n_sync`-independent but `eta`-proportional.** `dtau = eta·dt_left/(A·B)` is fixed
per sub-interval, so the first physical step is `~eta·dt_left` and the last step **overshoots by up
to that**. Total overshoot `= eta·dt_left × n_sync = eta·T` — **independent of `n_sync` by
construction**, and linear in `eta`. That predicts 6.33e-2 at `eta = 0.01`; measured 4.8e-2 to
3.0e-2 across `n_sync` 8→128. Mechanism confirmed.

**The fix is one clause: shrink the final step to land exactly on the boundary.**

```
rem      = max(dt_left - t_so_far, 0)
dtau_eff = min(dtau, rem/(A·B))       # dt = A·B·dtau, so this consumes exactly the remainder
```

| `eta` | current | **corrected** | improvement |
|---|---|---|---|
| 0.02 | 6.163e-02 | **8.454e-06** | 7,291× |
| 0.01 | 3.100e-02 | **2.215e-06** | 13,993× |
| 0.005 | 1.432e-02 | **5.625e-07** | 25,466× |
| 0.002 | 5.759e-03 | **3.113e-08** | 185,007× |
| 0.001 | 2.818e-03 | **4.232e-09** | **665,795×** |

**Convergence goes from first order to roughly third** (3.8×, 3.9×, 18.1×, 7.4× per reduction). Not
yet RK4's fourth — a residual lower-order term remains, most plausibly the LC re-registration at
sync boundaries — but **closure at 4.2e-9 is six orders better than before and comfortably good
enough to validate against published orbits.**

**So the answer to "is the integrator good enough to reproduce these solutions?" is: the method
yes, the current implementation no — and the gap is one clause.** The validation orbit found it,
quantified it, and confirmed the fix, in a single session. Nothing else in the test suite could
have: the error is pure phase, and energy stayed at 1e-12 throughout.

**Patch kept at `tb_az_overshoot_fix.py`** for reference. Note it changes results, so it is a
deliberate spec change rather than a tidy-up: re-run the cross-check and the horizon table after
adopting it.

---

## 1. The families, and what each is good for

### 1.1 Free-fall periodic orbits — the best fit for this project

Li & Liao 2018 (234 unequal-mass), Hristov–Hristova–Dmitrašinović–Tanikawa 2023 (**12,409**
equal-mass). *"The free-fall formulation starts with all three bodies at rest… the masses do not
orbit in a closed loop, but travel forward and backward along an open track."*

**Why these are the right family here:**

- **All three bodies start at rest** — so `p = 0`, which is *exactly* the `z_q = 0` plane of the
  latent chart. **They are points we can locate on our own chart.**
- **Zero angular momentum**, like Burrau — so no `L ≠ 0` assumption is smuggled in.
- **They are brake orbits**: out and back. That makes them **time-symmetric by construction**, so
  they are the natural test bed for the reversibility measure `xi` — and the one place a
  non-reversible integrator will show itself immediately.
- **12,409 of them.** A validation *suite*, not a validation *case*.

### 1.2 Figure-eight — the canonical closure test

Moore 1993, Chenciner–Montgomery 2000. Equal masses, `L_z = 0`, stable to small perturbations
(so not knife-edge). ICs and period above; already run, already productive (§0).

### 1.3 Lagrange and Euler central configurations — the only analytic ones

*"These solutions are valid for any mass ratios, and the masses move on Keplerian ellipses. These
four families are the only known solutions for which there are explicit analytic formulas."*

**And we already know where they are on the chart.** From
`principia_config_chart_diagram.png`: **Lagrange (equilateral) is at `z = 0`, the chart centre**;
**Euler (collinear) is at `beta → 0` and `beta → pi`, the top and bottom edges.**

So these are landmarks with **known position and known dynamics** — a test of the decoder and the
integrator at once. Give the equilateral configuration the right circular velocity and it must stay
equilateral forever; that is a closure test with an analytic answer at every `t`, not just at `T`.

### 1.4 Šuvakov–Dmitrašinović (13 families, 2013) and Broucke–Hénon–Hadjidemetriou

Equal-mass, zero-angular-momentum. Already in the project's reference list
(Šuvakov–Dmitrašinović AmJPhys 2014). Šuvakov and Dmitrašinović maintain a **"Three-body Gallery"**
of ICs, which is the practical source.

Worth noting these are classified by **free-group word / braid class** — the same `F₂` word calculus
the symbolic-dynamics contract uses. **So they validate the word machinery too**, not just the
integrator: a known orbit has a known word, and if the encoder produces a different one, that is a
bug with a named right answer.

### 1.5 Burrau itself

Szebehely & Peters 1967: *"established eventual escape of the lightest body… while at the same time
finding a nearby periodic solution."*

**Use with care.** Disruption is around `t ≈ 60`, and the measured f64 predictability horizon is
`ln(1/eps)/lambda ≈ 52` at `lambda ≈ 0.7`. **The classical result sits past our horizon**, so it is
a qualitative sanity check, not a quantitative test. The *nearby periodic solution* they found is
the more useful object.

---

## 2. What the suite tests that the current gates do not

| existing gate | what it sees |
|---|---|
| two-body radial collision | the singular regime, one pair |
| gauge invariance under rescaling | the scale gauge |
| Burrau constants | the decoder's arithmetic |
| Python cross-check | agreement with a second implementation |
| `error_ratio`, `worst_energy_drift` | **energy** error only |

**None of them sees phase error.** §0 is the proof: energy at 1e-12 while the orbit misses by 6e-3.

**And closure needs no second implementation** — which matters because the Python reference shares
source lineage with the Rust, so a shared-derivation bug survives the cross-check. A periodic orbit
does not care where the code came from.

---

## 3. Proposed suite

| test | orbit | assertion |
|---|---|---|
| **closure** | figure-eight, `T = 6.32591398` | `|dr|` after one period, and **that it falls at the occupant's stated order** — the order check is what caught §0 |
| **closure, free-fall** | 2–3 from Hristov et al. 2023 | same, on brake orbits: returns to rest in the same configuration |
| **analytic, all `t`** | Lagrange circular | stays equilateral at every sampled `t`, not just at `T` |
| **chart landmark** | Lagrange at `z = 0`, Euler at `beta → 0, pi` | the decoder puts them where the geometry says |
| **reversibility (`xi`)** | free-fall brake orbits | time-symmetric by construction — **the natural test bed, and blocked on a reversible occupant** |
| **word calculus** | Šuvakov–Dmitrašinović families | the encoder reproduces the published braid class |

**Report the convergence order, not just the error.** A pass/fail at one `eta` would have missed §0
entirely; the order is what exposed it.

---

## 4. What the suite cannot do

**Periodic orbits are measure-zero and non-chaotic.** They validate the integrator, the decoder and
the word encoder — they say nothing about behaviour in the chaotic sea, which is where the
instrument actually operates. **Necessary, not sufficient.**

**Most published families are collisionless by construction**, so they do not exercise the
regularisation. The existing two-body radial collision gate remains the test for that, and the two
suites are complementary rather than overlapping.

**And the figure-eight is stable**, which is a virtue for a test (not knife-edge) and a limitation
(it does not probe sensitivity). The free-fall orbits are typically less stable and therefore the
sharper test.

---

## 5. Immediate action

1. **~~Isolate the first-order error~~ — done (§0.1). ~~Adopt the overshoot fix~~ — adopted (pending change 10, landed; it was not the wedge fix).** Confirmed
   analytically and by measurement: closure improves **665,795×** at `eta = 0.001` and convergence
   goes first-order → ~third. It changes results, so re-run the Python cross-check and the
   divergence-vs-horizon table after adopting it — both were measured with the overshoot present.
2. **Add the figure-eight closure test with an order check**, now. It is one trajectory, it already
   found something, and it costs nothing to keep.
3. **Fetch free-fall ICs** from Hristov et al. 2023 / the Three-body Gallery and add 2–3.
4. **Record Lagrange at `z = 0` and Euler at the `beta` edges as chart landmarks** in the chart
   reference — they are decoder tests with analytic answers, currently unused.


---

## 6. The closure field — from validation to discovery

The suite above validates. **The same quantity, stored per sample, turns the atlas into a search.**

Specced as `closure_min: f32` + `closure_step: u16` in `SimState`
(`principia_dd_simstate_payload.md` §1; pending change 9). The minimum starts once the shape has departed from
`n̂(0)` by a measured threshold `δ_dep` (R-37).

### 6.1 Why rendering beats searching

Every published family — Šuvakov–Dmitrašinović's 13, Li–Liao's 669, Hristov et al.'s 12,409 — was
found by **targeted numerical continuation**: pick a family, follow it. **Nobody renders the whole IC
manifold with a closure field and looks at where they sit**, because nobody has a picture of the
manifold.

**And the geometry is already in place.** Free-fall periodic orbits start at rest, so `p = 0` —
**exactly the `z_q = 0` plane the Config α×β chart already renders.** The 12,409 known equal-mass
solutions are points on a chart we are already drawing.

That makes it **self-validating**: render the plane and check they appear where they should. If they
light up, the field works *and* the integrator is validated. If they do not, one of those two is
wrong — and either answer is worth having.

### 6.2 Stability is in the gradient, not the value

The better question is not *where* the orbits are but **what happens around them.**

| | closure rises | shape | reading |
|---|---|---|---|
| **stable (elliptic)** | gently | broad soft basin | perturbations stay near-periodic — these are the ones that could exist physically |
| **unstable (hyperbolic)** | steeply | thin filament | the near-periodic set is a curve, not a region |

**So the width of the closure well is a stability measure**, and it is not something a list of orbits
can give you — it needs the surrounding manifold.

Two ways to make that quantitative with machinery that already exists:

- **`alpha` on the closure field.** A smooth basin gives `alpha ≈ 1` (linear response); a filament
  gives `alpha ≈ 0`. The refinement exponent, pointed at a new field, distinguishes elliptic from
  hyperbolic **as a number** rather than an impression.
- **Well width has a Lyapunov reading.** Around an unstable orbit the width over which closure stays
  small scales as `e^(−λT)`. With `closure_step` giving `T` and FTLE giving `λ` **on the same
  pixels**, that is a closed check rather than an assertion.

**And the structure around unstable orbits is the point.** Their stable and unstable manifolds *are*
the fractal boundaries — so the render should show the filaments **emanating from** the orbits. If it
does, that is the skeleton and the chaos in one image, with the causal relationship visible rather
than asserted.

### 6.3 The Burrau connection

Szebehely & Peters 1967 established the escape of the lightest body *"while at the same time finding
a **nearby periodic solution**."*

**Burrau has a known periodic orbit next door, and nobody has pictured the relationship.** Burrau is
released from rest, so it sits in the same `z_q = 0` plane as all 12,409 free-fall orbits — not near
*a* periodic orbit but a single point in a plane densely populated with them.

**The question this upgrades the Burrau neighbourhood study to:** is Burrau on the **unstable
manifold** of a nearby periodic orbit? If so that is not a description of its chaos, it is the
**cause** — and it is a different kind of claim from a fractal dimension. Dimension says *how*
tangled; the skeleton says *what it is tangled around*.

Testable: the Szebehely–Peters orbit is a **named target with a published location**, so the field
either shows it there or it does not. `closure_step` and the well width are directly comparable
against the 1967 paper.

**Horizon caveat holds.** Disruption at `t ≈ 60` is past the f64 limit of `~52`, so the long-term
Burrau outcome stays qualitative. But **neighbourhood structure is a `t ≈ 13` question**, comfortably
inside — and that is the part this addresses.

### 6.4 Two cautions

**At f32 every genuine orbit saturates to the same ~1e-7 floor.** The GPU path **finds** orbits; it
cannot **rank** them. Ranking is the f64 inspector's job — the architecture already has the division,
but it should be stated so a saturated field is not read as a null result.

**Near-periodic is not periodic.** A pixel reading 1e-7 shows a *neighbourhood*, not a solution.
Anything that looks like a discovery needs the f64 path and a proper closure test before it is called
one — the same discipline that made the figure-eight productive rather than decorative.
