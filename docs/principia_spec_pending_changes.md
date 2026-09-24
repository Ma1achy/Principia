# Principia — pending spec changes (LaTeX edit queue)

*Scratch note holding ALL pending edits to the LaTeX implementation spec — seven so far, each surfaced by a contract and resolved at contract level where applicable; the LaTeX wording updates are batched for the next spec pass. (File renamed from `principia_spec_change_tilereduction.md` — the old name carried the retired tile-as-quad vocabulary and named only change 1.)*

> **Change 7 is not like 1–6.** Those are LaTeX wording fixes with the `.md` corpus already correct. **Change 7 alters payload semantics** (what `detail = 3` means) and adds two detectors, so it must land before outcome data is generated at scale.

---

# Pending spec change 1: `QuadReduction` majority state

> **RESOLVED — dissolved, not decided.** This change asked whether `dominant_outcome` (stored at
> `class|detail`) needed a class-only companion so it would match `outcome_impurity` (defined on the
> class fraction), since the proposed "masked accessor" is unsound — **argmax does not commute with
> masking** (three escape-details at 20% each against bounded at 30% gives `argmax(class|detail)` =
> bounded but `argmax(class)` = escape at 60%).
>
> **Defining *every* event-derived reduction field at the joint `class ⊕ detail` grain removes the
> question**: one grain, so no commuting problem and no companion field. Storing the **class
> histogram** and deriving both dominant and impurity from it makes disagreement impossible by
> construction. Empirically the joint grain also strictly dominates either grain alone (+0.148 over
> the better single grain) while collapsing to the class value where there is nothing extra to see —
> `principia_dd_refinement_criterion.md` §6.5.
>
> Landed in `principia_dd_generation_root.md` §3.7. **The original analysis below is retained for the
> argmax argument, which is the reason the obvious shortcut fails.**

*A small, deferred edit to the LaTeX implementation spec (its `§tile_summary` / `§par:tile_reduction` sections — which become `QuadSummary`/`QuadReduction` when the LaTeX is brought in line with the markdown rename), surfaced by the impurity-mask cross-check view in the render contract. Not urgent; record so it isn't lost.*

## What surfaced it

The debug catalogue's **impurity-mask cross-check** renders, per fragment, whether that sample's outcome class differs from the quad's majority class. Its spatial mean must then equal the quad's `outcome_impurity` — that identity is the assertion (sample↔quad reduction agreement). To evaluate it in the fragment shader, the shader needs the quad's majority class available per pixel.

## The finding: the field already exists, under another name

`QuadReduction` already carries:

```
uint  dominant_outcome;   // most common class|detail
float outcome_impurity;   // 1 - max(class_fraction)
```

So `dominant_outcome` **is** the majority `state`. No new reduction field is required. The gap is only that:

1. `dominant_outcome` packs `class | detail` together, whereas the per-sample comparison for the impurity mask is on **`state` alone** (`sd_state`, bits 0–2, and only the terminal values escape/bounded/collision). Comparing against the full `class|detail` would over-count — two escapes of different bodies would read as "impure" even where `outcome_impurity` (which is defined on the reduced class fraction) counts them together. **Decide and document which grain `dominant_outcome` and `outcome_impurity` use, and make them consistent** — if `outcome_impurity` is class-only, the mask must compare class-only, and `dominant_outcome` needs a class-only companion (or a documented accessor that masks off detail).
2. `RenderQuad` (the render-facing quad struct in the render contract) must expose whichever field the mask reads, so `ctx.quad` carries it to the fragment shader.

## The edit, when convenient

- In the LaTeX `QuadReduction`: either confirm `dominant_outcome` is class-only, or add `uint dominant_class;` (class bits only, matching the grain of `outcome_impurity`). One u32 if added.
- State explicitly, next to both fields, the grain (`state` vs `state|detail`) each uses — this is the actual ambiguity; the field count is secondary.
- Thread the chosen field into `RenderQuad` → `RenderContext.quad`.
- Nothing changes on the GPU reduction cost — the majority is already computed to produce `dominant_outcome` and `outcome_impurity`.

## Status

Deferred, same bucket as literature validation and floor-trust. The cross-check view is what makes the ambiguity matter; until the impurity mask is built, `dominant_outcome` as-is is fine for rendering. Flagged here so the grain question is decided when the spec is next touched, not silently papered over.

---

# Pending spec change 2: Burrau leg-swap range mismatch

*Surfaced while reconciling the Burrau charts against the chart contract. Two Burrau charts survey different quotients of shape space without saying so.*

## What surfaced it

The two shape-axis conventions in `§burrau_charts` use inconsistent ranges:

- **Acute-angle axis:** `θ ∈ (0, π/4]` — explicitly "removes the leg-swap redundancy" (θ and π/2−θ are the same triangle with legs swapped).
- **Euclid plane / default ν:** `ν ∈ (1/32, 31/32)`, which maps to `θ ∈ (0, π/2)` — i.e. **both** leg orderings.

So the acute-angle chart folds the leg swap out; the Euclid plane keeps it in. They are quietly surveying different sets.

## The subtlety (why it's a decision, not just a bug)

Under the **Burrau mass convention** (mᵢ = opposite side), swapping legs `a ↔ b` swaps two of the three masses — it is a body *relabelling*, which produces a genuinely distinct labelled system, not a duplicate. So keeping both orderings (Euclid plane) is defensible: it surveys both mass assignments. Folding them out (acute-angle) is also defensible: it surveys each *shape* once. Neither is wrong; they answer different questions. The bug is only that the two charts disagree silently.

## The edit, when convenient

- Decide which quotient the survey wants: shape-only (fold the swap, `θ ≤ π/4`) or shape×labelling (keep it, full range).
- Make the two chart ranges consistent with that decision, or — if both are wanted — label each chart with which quotient it covers, and note it in the `system_image` descriptor (the full-range Euclid plane is then an *n-to-1*-flavoured relabelling cover, not a clean bijection over shapes).

## Status

Deferred. Matters when quantitative per-shape statistics are computed (double-counting risk if the full-range chart is used for shape fractions). Flagged so the ranges are reconciled deliberately.

---

# Pending spec change 3: constrained-inverse wording in §lookup

*Surfaced by the inverse-encode contract. The spec contains two conflicting definitions of the canonical fibre point for invariant charts.*

## The conflict

- **§inverse_encode (case 2):** "Choose the canonical representative with **smallest latent-space norm**."
- **§LE_view (the chart's own forward decode):** the deterministic construction — minimal-kinetic-energy rigid rotation `v^(L) = (L_z/I)·J r` realising `L_z`, plus `a·w` along the seeded direction field.

These are different functionals (kinetic-energy-minimal ≠ latent-norm-minimal after the logit pullback). If lookup ran an independent smallest-norm optimisation, entering an `(L_z, E)` pair and clicking the corresponding pixel would land on *different* z — the round trip would not close.

## The resolution (inverse-encode contract, Part 5)

**Encode reuses decode.** For invariant charts, the canonical fibre point *is* whatever the chart's forward construction produces at the current frozen configuration; lookup runs that construction and recovers `z_mom` via the free-momentum inverse of the constructed `p`. "Smallest latent norm" survives only as the general fallback for case-3 charts with no canonical construction.

## The edit, when convenient

In §inverse_encode case 2, replace "choose the canonical representative with smallest latent-space norm" with "run the chart's own deterministic momentum construction at the frozen configuration (§LE_view) and invert the result; smallest latent norm applies only to case 3."

## Status

Resolved at contract level; LaTeX wording update deferred to the next spec pass.

---

# Pending spec change 4: stale reduction-cascade figure caption

*Surfaced during the COM-projection audit. A figure caption near the top of the spec contradicts the spec's own dimension accounting.*

## The problem

The symmetry-reduction cascade figure (around line 108) captions the result as: "The final **5-DOF** reduced space (**2 mass + 2 shape + 1 momentum orientation**) is what the decoder parameterises."

This is wrong under the spec's own accounting (§the 8D latent chart) and the chart contract Part 1: the decoder parameterises **8 DOF = 2 config ⊕ 4 momentum ⊕ 2 mass**. The canonical gauge (rotation + scale) consumes 2 *configuration* DOF; the momentum sector keeps all 4. There is no reading under which the decoder's space is 5-dimensional — the caption looks like a leftover from an earlier reduction scheme, or a description of a restricted family (e.g. a one-momentum-parameter slice) mislabelled as the general decoder.

Given the entire 10D→8D history, this caption is precisely the sentence that would re-teach a code agent the wrong dimension count. Minor secondary wording: the caption lists reflection as a step that "removes degrees of freedom" — the mirror is a discrete Z₂ quotient; it halves the space but removes no continuous DOF.

## The edit, when convenient

Either recount the caption to the correct 8 (2 config + 4 momentum + 2 mass, with rotation/scale consuming 2 configuration DOF and reflection a discrete halving), or — if the figure genuinely depicts a restricted slice — re-caption it as that slice and remove "is what the decoder parameterises."

## Status

Deferred to the next spec pass. High priority among the pending edits: it sits in the introductory material an agent reads first.

---

# Pending spec change 5: body-index naming (0-indexed decode vs `m1/m2/m3` descriptor)

*Surfaced by the decoder drill-down (§6) and cross-referenced by the ledger (§3.6) and generation-root §6. Three docs flag it as a pending-changes item; the queue was missing the entry they point to — recorded here so the references resolve.*

## The conflict

- **Decode** (decoder dd §3): bodies are **0, 1, 2** — the softmax reference is body 0 (logit 0), and the mass-weighted Jacobi construction, canonicalisation, and detector pair/body labels all run on 0-indexed bodies.
- **`ICDescriptor`** (payload §2 / ledger §3.6): the mass fields are named **`m1, m2, m3`** (1-indexed).

The same three bodies are 0-indexed in the physics and 1-indexed in the descriptor — the off-by-one that survives silently until a collision-pair or escaper label is wrong on screen (an escape of "body 0" mislabelled, or a `dmin_pair`/`detail` pair-id decoded against the wrong index base).

## The resolution direction (decoder dd §6 recommendation)

Pick one convention project-wide — **recommend 0-indexed internally** (matching the decode/softmax reference and the descriptor bit unions), and **rename the `ICDescriptor` fields** `m1/m2/m3 → m0/m1/m2`. Whichever wins, the ledger's `ICDescriptor` names change with it — a **schema-version change** (generation-root §6), correctly, since it is layout-visible.

## The edit, when convenient

- In the LaTeX `ICDescriptor` (and the ledger §3.6 field table): rename the mass fields to the chosen convention, stated once next to the decode's body-indexing so the base is unambiguous.
- Confirm the `detail`-union pair/body ids (escape → body, collision → pair) and `dmin_pair` decode against the same index base.

## Status

Deferred to the next spec pass, batched with the others. Not urgent for rendering; matters the moment a pair/body label is surfaced on screen. Flagged by three docs — added here so the queue is complete.

---

# Pending spec change 6: remove `α_min` from the shape-sphere config link

*Chart-definition change, decided and already propagated through the `.md` corpus (dd_decoder §3.2, dd_encode, inverse_encode) and the IC Inspector. The `.tex` still carries `α_min`; this brings it into line.*

## The change

`α_min` was a polar-cap excision on the shape sphere (`α ∈ [α_min, π/2−α_min]`), **not** a numerical guard — nothing in decode or encode divides by α, and exact poles give `atan2(0,0)=0`, not NaN. It was removed for full-sphere coverage.

- **Config link (forward):** `α = α_min + (π/2 − 2α_min)·σ(z_α)`  →  **`α = (π/2)·σ(z_α)`**, giving `α ∈ (0, π/2)` (finite `z` never reaches the exact poles).
- **Config inverse (encode):** `s_α = (α − α_min)/(π/2 − 2α_min)`  →  **`s_α = 2α/π`**.
- **Range statements:** `α ∈ [α_min, π/2 − α_min]`  →  **`α ∈ (0, π/2)`**.

## Why it's safe

Both `α`-poles are now *represented*, not carved out. `α→π/2` (`ρ̃→0`) is a binary collision (caught by collision detection) with the rotation pin undefined; `α→0` (`λ̃→0`) leaves the azimuth `β` undefined at the point. These are coordinate/collision degeneracies the pipeline already fences via the collision detector, the conditioning readout, and the saturation flags — verified in the IC Inspector: exact-pole ingestion yields finite, SAT-flagged `z`, no NaN.

## The edit, in the `.tex`

Wherever the chart config defines the `α` link, its inverse, and the shape-sphere `α`-range: delete `α_min` and its buffering, per the three substitutions above. Confirm no other `.tex` section reads `α_min` (e.g. a coverage/sampling figure caption citing the excised band).

## Status

Deferred to the next spec pass, batched with the others. The `.md` corpus and the tool are already consistent at `α_min = 0`; only the `.tex` lags.

---

> **LANDED in the vertical slice.** `State::is_triple`, `triple_ejection` and the **≥2-pair rule**
> (with its triangle-inequality argument) are implemented and on `main` in prin-rs — present at
> `0114be4`, predating the closure work. Not a prototype, not a gap.
>
> **One correction to the wording below:** the count must be tested **before** classification.
> `>=2 pairs -> triple`, `exactly 1 -> binary`. Testing "any pair below `r_coll`" first steals genuine
> triples into the binary arm — the same precedence bug found on the escape arm.

# Pending spec change 7: triple collision and triple ejection have no event class

*Surfaced by the regularisation work (`principia_dd_refinement_criterion.md` §7.7). Unlike changes
1–6 this is a **payload semantics** change, not a LaTeX wording fix — it alters what `detail` means
and adds two detectors. Cheap in bits (zero), but it must be decided before outcome data is
generated at scale, because it changes how existing samples would have been classified.*

## What surfaced it

Levi-Civita regularisation handles binary collision at machine precision, but **triple collision is
provably non-regularisable** — the singularity is essential and solutions cannot be continued
uniquely through it. Systems with non-zero angular momentum are protected (triple collision requires
`L = 0`); **Burrau is released from rest, so `L = 0` and the protection does not apply.**

The `deep interior` region, which destroyed every integrator tried (drift 3.7e+01 unsoftened,
7.7e+01 with LC, `d_min` reaching exactly 0), is plausibly a near-triple-collision. **There is
currently nowhere in the enum to record that.**

## The gap

Per `principia_dd_simstate_payload.md` §2:

```
state  (bits 0-2, enum6):  0 escape · 1 bounded · 2 collision · 3 running
                           4 sim_failed · 5 decode_failed   (6-7 reserved)
detail (bits 3-4, union keyed by state, 4 codes):
        escape    -> body id (0-2, 3 = invalid)
        collision -> pair id (0-2, 3 = invalid)
```

`collision` names **which pair**; `escape` names **which body**. Neither can express "all three".
The terms *triple collision*, *triple ejection* and *ionisation* appear **nowhere** in the design
corpus — this has not previously been raised.

Both are physically real:

- **Triple ejection (ionisation)** — all three bodies mutually unbound. Requires `E > 0`, so it is
  impossible for free-fall-from-rest ICs, but the 8D chart has free momenta and `E > 0` regions are
  reachable. A standard scattering outcome, currently unrepresentable: `escape` + body id encodes
  only "one body left, a binary remains".
- **Triple collision** — measure-zero exactly, but near-triple-collision passages are the organising
  centre of the dynamics (the shape-sphere / McGehee formalism exists to study them), and it is the
  one case the integrator cannot pass through.

## The change

**Encoding — `detail = 3` (`11`) means "all three", in both arms of the union:**

| state | detail 0–2 | **detail 3 (`11`)** |
|---|---|---|
| `collision` | pair id | **triple collision** |
| `escape` | body id | **triple ejection (ionisation)** |

One rule — *"3 means all three"* — symmetric across both arms, so a reader needs one mental model,
not two. **Costs zero bits.**

**Detector — reuse `r_coll`, count the pairs.** The march already computes pairwise distances every
step (for `d_min`, for the binary collision test, and for the tightest-pair event class), so this
adds no tracked quantity and no new constant:

- **exactly 1 pair** below `r_coll` → `collision`, `detail` = that pair
- **2 or more pairs** below `r_coll` → `collision`, `detail = 3`

> **The rule is on the count, not on "all three", because of the triangle inequality.** If
> `|AB| < r_coll` and `|AC| < r_coll` then `|BC| < 2·r_coll` automatically, so *exactly two pairs
> below threshold* is a reachable state and is already a near-triple. Requiring all three would
> leave the two-pair case silently classified as a binary collision — whichever pair happened to be
> tested first.

(A hyperradius test, `R < r_triple` with `R = sqrt(I/M)`, expresses the same condition and is
elegant in the canonical frame — `√I = 1` is the pinned quantity — but it needs a new threshold and
a new tracked quantity. The count-based rule is strictly less machinery.)

**Ionisation gate — genuinely different from the existing escape gate.** The current detector tests
*one body against the remaining binary* (outward, plus positive outer-energy). Ionisation requires
**all three pairwise relative energies positive and all three separations growing**. This is new
logic, not a re-parameterisation of the existing gate.

**Sentinel removal.** `detail = 3` currently means "invalid". Dropping that is safe: the spec already
states `detail` is only meaningful when `state ∈ {escape, collision, sim_failed, decode_failed}`, and
the writer sets `detail` in the same operation that sets `state`. An unwritten `detail` therefore
cannot occur without a wrong `state`, which the state field's own gating already catches. **The
sentinel was defending a case that is unreachable** given how the field is written.

> Considered and rejected: preserving a validity marker in spare bits elsewhere. It would only guard
> "field never written", which the above already covers. Considered and rejected: taking a reserved
> `state` code (6/7) for `triple`. It preserves the sentinel and degrades safely on old decoders
> (unknown states read as *finished and untrusted*), but burns one of two spare codes for something
> that fits inside `detail`.

## Consequences to check

1. **`sd_*` accessors** — `sd_detail` consumers that treat `3` as invalid must be updated; grep for
   the invalid-detail branch.
2. **Binary parity** — `state`/`detail` are in the exact-match set (payload spec §0), so CPU and GPU
   detectors must agree bit-for-bit, including the ≥2-pair rule.
3. **Existing outcome data** — any samples already classified would have recorded a two-pair
   collision as a binary one. Regenerate, or treat pre-change data as a different `state` grain.
4. **Basin fractions and quad impurity** — gain up to two new classes; anything enumerating outcome
   codes (palettes, histograms, the `class_histogram` in `QuadReduction`, ML label sets) widens.
5. **Triple collision is terminal and non-continuable** — unlike binary collision, which is
   regularisable and where stopping is a *choice*. Worth stating next to the enum, since the two look
   alike in the data but differ in whether continuation was possible.

## Amendment from the Rust kernel — `r_coll` is a recorded parameter, not a physical constant

Measured on a 64×64 near-field sweep: the collision fraction runs **0.0000 → 0.0242 → 1.0000 across
three decades of `r_coll`**, because the whole grid's `d_min/R` spans **less than one decade**. No
threshold in that range is a physical event boundary — the label is a *readout of the `d_min`
distribution*.

**So the detector is restated:** `d_min` (over all three pairs) is the **primary stored quantity**;
the collision label is **derived** from it; and `r_coll` is a **recorded parameter carried on every
output**, not a physical constant. The count-based ≥2-pair rule for triple collision is unchanged —
it is about *which* label, not *whether* one fires.

The default `1e-3` of `R` has the honest justification and no more: it separates tail from bulk **on
the slices measured**.

Also corrected: **`deep interior` is an ordinary binary collision**, not the near-triple this
document's motivation implied. Pair (0,2) closes to 2.28e-5 while the other two never register even
at `r_coll = R`. The near-triple reading came from an unregularised run with energy drift of 37.

## Status

Open — **decided in principle** (`detail = 3` = all three; count-based detector; sentinel dropped),
not yet written into the payload spec or the LaTeX. Should land **before** outcome data is generated
at scale, since it changes classification of samples that already exist. The ionisation gate needs
its own definition pass; the collision half is implementable immediately from constants already in
the march.

---

> **LANDED, and went further than proposed.** Regularisation is now a **second swappable axis**,
> independent of the stepper — `principia_integrator_contract.md` Part 2b. Four occupants: `none`
> (the independent control), AZ, **Heggie (the default)**, logH.
>
> **Heggie is the default**: 31 of 32 cases, `err>10` 3916 → 73, AZ's worst decile fixed on 100% of
> pixels. AZ retains exactly one win — `far` — **because sustained hierarchy means it never
> re-registers**, which is the mechanism confirming itself.
>
> **The measured reason:** doubling the sync-boundary **re-registration count** at fixed step size
> moves the drift field **0.444 decades**, against 2.5e-6 for the LC branch and 7.5e-5 for the
> reference-body rule. *It is not which chart is chosen; it is how often the state is passed through
> one.*

# Pending spec change 8: widen the integrator occupant slot for regularisation

*Surfaced by the Rust kernel work. Like change 7 this is a **contract semantics** change rather than
a LaTeX wording fix, and it has a dependency: the reversibility diagnostic cannot be built until it
lands.*

## What surfaced it

`principia_integrator_contract.md` Part 1 specifies the occupant as `STEP(state, dt, params)`, called
`N_sub` times by the wrapper at `dt_macro/N_sub`, with **adaptive substepping owned by the wrapper**.

That model was measured failing on close encounters. On `deep interior` an unregularised adaptive
integrator returned **`|dE/E| = 2.8e+02`** in 35k steps against a 2e6 budget — it did not exhaust
the budget, it returned **a wrong number that looks like a right one.**

Aarseth–Zare regularisation fixes it (passage through exact collision at `d_min = 1.35e-11`,
drift **6.2e-15**) but carries its own time transformation `dt = |R1||R2| dtau` and integrates in
**fictitious time**. The occupant, not the wrapper, then owns the mapping to physical time — which
the current slot shape does not permit.

## The change

- `STEP(state, dt, params)` becomes **`ADVANCE(state, t_now, t_target, params)`**. The wrapper owns
  the target; the occupant owns how it gets there. KDK/Yoshida implement it as the loop they already
  run.
- The capability profile gains **`owns_time_mapping: bool`**, joining
  `{order, force_evals, symplectic, reversible}`.
- The per-substep cadence (projection, invariant accumulation, terminal detection) becomes a
  **callback passed in**, not something the wrapper does after — otherwise an occupant can silently
  skip it.
- Two new occupant rows: **AZ + RK4** (available now, `symplectic: false`) and
  **AZ + time-transformed leapfrog** (Mikkola–Tanikawa — the entry the table wants, and the one that
  unblocks the reversibility diagnostic). `Gamma` is not separable, so plain KDK does not apply.
- Canonical spec law 18's count-bound relocates from fixed `dt` to a fixed **`tau`-schedule**. Same
  guarantee.

**Full rationale, costs and the preserved-invariants list: `principia_integrator_contract.md` §2a.**

## Consequences to check

1. **`total_substeps` becomes occupant-reported** and the wrapper can no longer verify it. It is
   load-bearing — measured bimodal with a **100× p1→p99 range** across 68,685 leaves — and
   cost-aware priority depends on it meaning the same thing across occupants. Assert, do not assume.
2. **Caching** already hashes "integrator occupant + config"; confirm `owns_time_mapping` is inside
   that config.
3. **The reversibility diagnostic (ξ) is blocked on this.** It cannot be measured under RK4, which
   is neither symplectic nor reversible — any ξ computed there measures the integrator's asymmetry,
   not the system's.

## Status

Open — **decided in principle**, not yet written into the LaTeX. Should land before the reversibility
measure is attempted, and before any occupant table is treated as complete.

---

# Pending spec change 9: closure field — periodic-orbit detection in `SimState`

*A payload change. Landed in `principia_dd_simstate_payload.md` §1 but **not yet reflected in the
tier table, the caching signature, or the debug catalogue** — those are the open items.*

## What it is

Two scalars appended to `SimState`, updated once per step as a **running minimum** (the `d_min`
pattern — two registers, no array, no history):

```
closure_min  : f32   // min over t > t_min of |n̂(t) − n̂(0)| — SHAPE-sphere closure
closure_step : u16   // exact step index of the minimum — the period label
_reserved    : u16   // free under alignment
```

**Why it exists.** A periodic orbit returns to its starting configuration, so this quantity goes to
the integration floor there and stays O(1) everywhere else — **~7 decades of contrast at f32.**
Colouring by it makes periodic orbits appear as holes punched in the field, *without searching for
them*: every published family is found by targeted numerical continuation, whereas this renders the
manifold they live in and they appear because they cannot not.

**Shape-sphere closure, not Cartesian** — rotation is quotiented out, so **relative** periodic orbits
count too, a strictly larger family. `shape_vec` is already computed, so it costs three subtracts and
a compare on registers already loaded.

## Consequences, in order of how much work they are

1. **`SimStateFTLE` 136 → 144 B, `SimStateBase` 88 → 96 B.** Both stay 8-byte aligned, nothing
   repacks. **The tier table in §7 of the payload spec is stale and flagged as such** — recompute at
   the new widths. ~+4 KB per quad at N=8/E+1=8; ~+8 MB against the quoted ~140 MB.
2. **Caching signature.** A payload width change must invalidate cached payloads — confirm the
   struct version is inside the compatibility signature.
3. **Two new debug/production colour sources** — `closure` and `period`. The second is a genuinely
   new channel: families with related periods should band together.
4. **`t_min` must be defined gauge-covariantly** (after the state has moved by a stated fraction of
   the system size), never as an absolute time, or the field inherits a scale.

## The limit that must be reported, not hidden

**The closure floor is precision-dependent**: ~1e-7 on the f32 GPU path, ~1e-16 on the f64 CPU path,
because the floor is set by the *trajectory* representation, not the storage. An orbit closing to
1e-9 in the reference reads as 1e-7 on the GPU — **saturation, not a lost orbit.** Same class of
reportable limit as `t_max = ln(1/eps)/lambda`.

**Consequence for use:** at f32 every genuine orbit saturates to the same value, so the GPU path
**finds** orbits but cannot **rank** them. Ranking is the f64 inspector's job. State the division of
labour rather than letting someone read a saturated field as a null result.

## Status

Open — **specced in the payload doc, not yet propagated.** Tier recompute, caching signature and
debug catalogue are the outstanding work.

---

> **LANDED — but it was NOT the wedge fix, and that matters.**
>
> The ablation (`FINDINGS.md`): `dtau` alone took non-finite 1153 → 32 and left **wedge density
> unchanged at 0.0026**. The **predictive step limit** alone took non-finite to **0** and wedge
> density to **0.0001**, reproducing the all-three arm on every column. The landing clamp *alone*
> made non-finite **worse** (1531).
>
> **Two artefacts, never one defect.** The magenta and the wedges were treated as one phenomenon for
> a long stretch and they were not.

# Pending spec change 10: the AZ sync-boundary overshoot fix

*Small, mechanical, and it changes results — so it is a spec change rather than a tidy-up.*

## What surfaced it

The Chenciner–Montgomery figure-eight closure test (`principia_dd_validation_orbits.md` §0). One
period, one trajectory: **energy conserved to 1e-12 while the orbit missed closure by 6e-3**, and
convergence was **first order** where RK4 should give fourth.

**Mechanism, confirmed analytically and by measurement.** `dtau = eta·dt_left/(A·B)` is fixed per
sub-interval, so the last step **overshoots** by up to one step. Total overshoot `= eta·dt_left ×
n_sync = eta·T` — **independent of `n_sync` by construction** (which is why varying it did not help)
and **linear in `eta`** (which is why convergence looked first-order).

## The change

Shrink the final step to land exactly on the boundary:

```
rem      = max(dt_left - t_so_far, 0)
dtau_eff = min(dtau, rem/(A·B))       // dt = A·B·dtau
```

| `eta` | before | after | improvement |
|---|---|---|---|
| 0.02 | 6.163e-02 | 8.454e-06 | 7,291× |
| 0.002 | 5.759e-03 | 3.113e-08 | 185,007× |
| 0.001 | 2.818e-03 | **4.232e-09** | **665,795×** |

Convergence goes **first order → roughly third**. Not yet RK4's fourth; a residual lower-order term
remains, most plausibly the LC re-registration at sync boundaries. **Worth isolating, but 4.2e-9 is
already six orders better and sufficient to validate against published orbits.**

## Consequences

1. **It changes results.** Re-run the Python cross-check and the divergence-vs-horizon table — both
   were measured with the overshoot present.
2. **The NumPy reference has the same defect.** Fix both sides or the cross-check disagrees for the
   right reason. Same pattern as the LC branch cut.
3. **Nothing else in the test suite could have found this.** The error is pure **phase** —
   displacement *along* the trajectory — which conserves energy exactly. Every existing gate is an
   energy or an agreement test. **That is the argument for the closure suite as a standing gate**,
   not just a one-off.

Patch: `tb_az_overshoot_fix.py`.

## Status

Open — measured and confirmed, not yet adopted. Should land before any further integrator
measurement, since every existing accuracy figure was taken with the overshoot present.

---

> ### ⚠ ESCAPE CRITERION SUPERSEDED — see `principia_01_pitfalls.md` §2
> The old test (`E_rel > 0 ∧ receding`, optionally `∧ d > r_esc`) **fires on transients**: measured
> 0 of 895 escapes still unbound 8 boundaries later. It is replaced by
>
> ```
> ESCAPE  ⟺  |Δn̂| over a window < tau    AND    E_rel > 0
> ```
>
> **100% precision, 96.3% recall**, against 97.9% for the old test. `receding` and `d > r_esc` are
> **redundant** once both hold (identical to the digit), so three tuned constants are eliminated.
> `tau` sits in a **383× gap** and is not tuned. Fires at `t≈10` rather than `t≈1.5` — **late rather
> than wrong**, which is correct for a *stored* `t_end`.
>
> **And escape must not terminate integration until §2.4's three checks pass.** Freezing a
> trajectory whose displayed quantity is still moving is what produced the patchwork artefact
> (§1). Collision stays terminal — it is a singularity, not a heuristic.


---

# Pending spec change 11: the escape criterion is closure + energy

**LANDED in the vertical slice.** Recorded here because the register is where the spec's deltas live.

```
ESCAPE  ⟺  |Δn̂| over a window < tau    AND    E_rel > 0
```

**100% precision, 96.3% recall**, against 97.9% for `E>0 ∧ receding`. `receding` and `d > r_esc` are
**redundant** once both hold — measured identical to the digit — so **three tuned constants are
eliminated**. `tau` sits in a **383× gap** and is not tuned.

**Why the old test failed:** it fired on transients. **0 of 895 escapes were still unbound eight
boundaries later.** The escape fraction was about to be overstated **5.8×**.

**It fires late — `t≈10` rather than `t≈1.5` — and that is correct.** `t_end` is a *stored, reported*
quantity feeding the escape-time gradient, the outcome label and the colouring. Late is right; wrong
is not.

## Status

Landed. **Termination on escape remains a separate question** — under this criterion the trajectory
has already converged when it fires, so freezing is nearly a no-op, but the three checks in
`principia_01_pitfalls.md` §2.4 have not all been run.

---

# Pending spec change 12: the refinement policy is `Policy::Tolerance`

**LANDED.** Full design: `principia_dd_refinement_policy.md`.

- **Split iff any footprint unresolved.** No exponent in the split test.
- **`eps` replaces `tau_display`, `alpha_hi` and `k_frac`** as the single quality knob, in
  `spread_shape`'s own units.
- **`alpha_area` is a dimension threshold**, not a tuned constant — a line reads 1, a sea 0, a
  boundary of dimension `d` reads `2 − d`.
- **The metric is payload-space.** `error(B)` scored in OKLab is **void** — auto-ranged lightness made
  breadth-first optimal by construction.
- **`Decision::Merged` and `QuadTree::resident`** are new, for the live playhead.

## Status

Landed at **v0.5**. Two open defects in `alpha_area` (§2.2 of the policy doc), and the camera is not
wired into priority — until it is, **a pan measures an identity.**
