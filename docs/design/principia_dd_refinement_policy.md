# Refinement policy — `Policy::Tolerance`

**Status: v0.5. Works, failure modes measured, two known defects.** Settled enough to build on;
expected to improve in the real build.

**Evidence base:** `prin-rs` — `FINDINGS.md` §5 (the policy), `README.md` §7 (the scheduler),
`results/tolerance_scaling/README.md` (where the saving does and does not generalise).

**Supersedes** `principia_ARCHIVE_dd_refinement_criterion_v0.md` and
`principia_ARCHIVE_dd_tree_dump_analysis_v0.md`. Read those only for method and for the record of
how the answer was reached; **do not implement from them.**

---

## 0. What the mechanism is actually for

**Progressive display, and arbitrary zoom. The compute saving is a bonus and is not the argument.**

This was misread for a long stretch, including in this corpus. The quadtree's value is:

1. **Time to first frame.** The root quad is `N² = 64` footprints — effectively instant. You see the
   slice immediately and it sharpens. That is what makes tilting and slicing feel responsive, and it
   is **independent of what the total saving turns out to be**.
2. **Arbitrary zoom.** A uniform grid cannot zoom past its own resolution. The adaptive tree plus the
   linearised decoder (`principia_deep_zoom.md`) reaches depth ~50. **This is the feature; the tree
   exists for it.**
3. **Total compute**, which varies enormously by slice and is the subject of §5.

> A frame does not have to converge within the frame budget. **The application has to stay
> responsive** — show something immediately, keep improving. Those are different requirements and
> only the second is a hard constraint.

---

### 0.1 In view, the camera decides depth and the criterion decides ORDER

**Landed at prin-rs `52caf14`.** A quad may stop only if its texel is at or below one screen pixel:

```
Keep   ⟺  n_unresolved == 0      AND  tile_size_px ≤ 1
Floor  ⟺  area floor fired       AND  tile_size_px ≤ 1
```

**Why.** `alpha_area` measures unresolved area — a *physics* quantity — and cannot see that the
display still wants pixels. It was flooring quads the camera needed, so zooming in made texels
**grow** (39 → 181 px, a 4.63× climb) and `max_depth` went **backwards**, 12 → 8, while the camera's
target depth rose to 14. **The floor was not wrong about physics; it was being asked a question
outside its domain.**

**The consequence, stated rather than inferred.** `adequate` is `tile ≤ 1` and the `ScreenFloor` veto
is `tile ≤ 1` — **they meet on the same value**. So every in-view quad above the screen floor must
split and every one at it stops. **The in-view tree is complete at screen resolution**, which is what a
slippy map displays. **Must-split is the at-rest target (R-108):** during a gesture the frame budget governs —
ancestors show, so there are never blanks — and completeness resumes at rest.

| region | who decides depth | what the criterion does |
|---|---|---|
| **in view, above screen floor** | the **camera** — must reach `tile ≤ 1` | decides **order** only |
| **in view, below screen floor** | the **criterion** — supersampling where unresolved | decides depth |
| **off screen** | the **criterion** | decides depth; adequacy is read as *met*, stored nowhere |

**Off-screen is adequate by definition.** `tile_size_px` is a ratio of sizes with no position term —
the same property that keeps `veto` free of `cx`/`cy` — so taken literally it would force the whole
plane to screen depth. Off-screen quads read adequate at query time.

**This subsumes the missing `target_depth` driver.** A quad shallower than the camera wants simply
cannot stop, so no separate wiring is needed.

**It does not reopen the degeneration `alpha_area` exists to close.** Measured on the cases that
matter: `config_stability` +5.5% quads with the floor still stopping 343 leaves, against `alpha_lo =
0` at +49% stopping none; `preset_shape_h1` +18% vs +222%. `cap frac` 0.69 / 0.52 against the
degeneration's ~0.99. **Bounded by the screen; the hole stays closed.**

**Converged, the line is flat:** 1.51, 1.38, 1.53, 1.50 px across the last two octaves, ±5%, `p50`
1.00–1.41.

**So the role of this document shifts.** §0 says the quadtree is for progressive display and arbitrary
zoom. Both still hold — but **in view, "arbitrary zoom" is now delivered by the camera, and the
criterion's contribution there is the order in which the complete tree arrives.** Its depth decisions
govern supersampling and off-screen work, both capped by `MAX_REL_DEPTH` (scheduler Part 3, R-98).

---

## 1. The split rule — no exponent in the split test

```
split(quad)  ⟺  any footprint f in quad is unresolved

unresolved(f)  ⟺  spread_shape(f) > eps          the payload has not settled
                ∨  the copies disagree on event class
                ∨  the footprint is undetermined  (non-finite spread, or an unusable copy)
                ∨  its latched running maximum of spread_shape ever exceeded eps   (R-91; per footprint, R-99)
```

**`eps` is a tolerance in `spread_shape`'s own units** — the same units in every region. The previous
policy's threshold had to sit *inside a distribution whose median moves six orders between regions*,
which is why it needed per-region tuning and never got it right.

**`eps` is the user-facing quality knob.** Tier tables key off it, not off resolution.

---

## 2. The stop rule — `alpha_area` is a DIMENSION, not a tuned constant

A fractal region is unresolved at every level and would descend forever. The stop is an exponent on
the quantity the policy actually optimises — the **unresolved area**:

```
alpha_area = log2( unresolved_area(coarse) / unresolved_area(children) )
```

**A line reads 1. A sea reads 0. A boundary of box dimension `d` reads `2 − d`.** So `alpha_lo` is a
*dimension threshold*, which satisfies the derive-or-absent rule — and it means **the scheduler can
report a measured box dimension as a by-product**, which is a Paper 2 quantity falling out of the
renderer for free.

Shipped default **`alpha_lo = 0.005`** (it stays, R-42): floor only where the children resolved essentially nothing,
judged as no-gain at a noise margin rather than as a dimension cut. At `0.2` the same floor costs
**11% of `config_stability`'s resolvable pixels** and puts its tree *above* uniform at its own error.

### 2.1 Two structural details, each of which cost a measurement

**The exponent is judged over TWO levels, from the grandparent's quadrant.** A footprint cell on a
quad boundary is shared by both quads at half weight, while the finer grid below locates the same
structure on one side at full weight — so a shore in an edge cell reads *no gain* on one side and
*infinite* gain on the other, and its children floor. From the grandparent's quadrant, whose cells
split cleanly at the midlines: **min 1.00, max 1.05 over 15 splits, against 0 under the one-level
form.** This is a constraint on the tree walk, not an optimisation.

**Noise is told from structure by NEIGHBOUR AGREEMENT, per footprint** — not by a coherence statistic
with a base rate in it. An unresolved footprint counts as structure iff **at least two of its eight
neighbours share its class and land within ~11° on the shape sphere**. A sea's neighbours are
independent draws, so two agreeing by chance is a few in ten thousand. Calibrated on white noise and
nearly inert on real charts, where seas are coherent sponges rather than white: it floors **2–37
quads where the dimension floor floors 262–334**.

### 2.2 TWO KNOWN DEFECTS IN `alpha_area` — both ruled (R-42)

**It cannot tell an empty mask from a full one.** Both return exactly `0.0000`, so any positive
`alpha_lo` floors on either. This is a **can't-fail test inside the floor itself**, and it costs
`near-field` its `t = 50` descent: 21 quads at error 0.103 with 93% of the frame resolvable, against
829 quads at error 0.00000 with `alpha_lo = 0`. **Fix (R-42):** tell the empty mask from the full one by
`n_unresolved`.

**The exponent goes negative on sea charts at tight `eps`** (−0.020 to −0.076 at `eps = 1e-3`): the
children found *more* unresolved area than the parent, so `d = 2 − α` reads **above 2 — impossible in
the plane**. The dimension interpretation has lapsed and **the floor fires anyway, hardest where
refinement is discovering structure.** Worst possible failure direction. **Fix (R-42):** refuse the floor on a
negative exponent.

---

## 3. Merging — the split rule read backwards

Under a live playhead the field changes as `t` advances, so the tree must **coarsen**:

> A parent whose four children are all leaves that did not split this boundary is **merged back** when
> the parent has become resolved (`Keep`) or its split shows no gain (`Floor`).

Children become `Decision::Merged`, and their footprints' latches go with them: the latch lives with the resident
quad and never pins memory (R-99). **`QuadTree::resident` — what a live design actually holds — is
tracked separately from `quads_computed`.** On the moving pulse, resident runs 37 → 85 → 37 → 69
while 181 quads are computed. **The moving pulse is a synthetic field, not a chart slice (R-166):** a step with an
unresolved band around it whose half-width is `w(t) = w_max sin(πt/t_max)` (prin-rs `src/testing.rs:162`), defined in
`fixtures/moving_pulse.toml`. *Was: "the final tree is **bitwise the static tree at the horizon**", measured before the
latch existed (R-99); withdrawn by R-143.* **The live tree contains the static tree at the horizon, and they are equal
when footprint spreads are monotone in time (R-143).** A merge doesn't drop a latch while the quad is resident (R-99): a
footprint that ever exceeded `eps` stays unresolved. The latch's cost, the extra resident quads, is measured on the named
slices, §5's `near-field`, `deep interior`, `config_stability` and `tilt_plambda` (a calibration, R-71, R-143).

A merged parent remembers its exponents so it is not re-split into the same four children every
boundary; the memory expires when the quad's structured weight leaves a factor of two of where the
split was judged.

**Two subtleties, both measured:**

- **Expiring a memory by DELETING it is the opposite of expiring it.** `decide` reads *no memory* as
  *never merged for no gain*, so clearing reverts to first-time behaviour and floors **more**, not
  less — 421 quads against 645. A separate expiry flag is required, and **an `assert_ne!` cannot tell
  the two apart.**
- **A capped leaf must stay re-decidable.** A leaf stopped by the camera floor originally left the
  frontier permanently, so a parent that later resolved could never merge it — **149 quads against a
  static 69, finer rather than coarser.** Caps are re-tested every boundary, because a resolved quad
  is decided *ahead* of the caps.

---

## 4. THE METRIC MUST BE PAYLOAD-SPACE. THIS IS NOT OPTIONAL.

**`error(B)` scored in OKLab is void**, and every conclusion drawn from it with it.

The shipping colouring auto-ranges lightness **per region**. So a smooth region's `1e-8` residual
counted as error at every depth, and **breadth-first came out near-optimal by construction of the
metric**. That is the entire explanation for the long-standing "nothing beats uniform" result.

**Score on the payload:** `shape_vec` and event class, against a fixed `eps` in `spread_shape`'s own
units. Shader-agnostic — which is what the architecture wanted anyway, and it resolves the tension
between "refinement must be shader-independent" and "the metric is a colour distance".

**Any future criterion work scored in render space is void before it starts.**

---

## 5. WHERE THE SAVING GENERALISES — and where it does not

**The saving is a property of the FIELD, not of the policy.** Measured across four charts, three
horizons, three tolerances:

| | tolerance vs uniform |
|---|---|
| Burrau regions (`near-field`, `deep interior`) | **5–37×** |
| `config_stability`, `tilt_plambda` (arbitrary latent slices) | **~1×**, sometimes below |

**Read `dp/u` before `tol/u`. This is the methodological result the rest depends on.** A low saving
has two causes — *a field with nothing to find*, and *a policy failing to find it* — and **only the
exact optimum separates them.** Both occur on the same chart one decade of `eps` apart.

On the sea charts the **exact-optimum ceiling is 1.08–1.44×** over breadth-first in all 24 fixed-target
cells, at every horizon, and the policy sits at 1.10–1.45× of that. **It is not underperforming. The
optimum is breadth-first.**

**And that is not bad news for interactivity.** Breadth-first is exactly what progressive display
does — it improves the whole frame evenly, which is what you want while someone tilts. On those charts
the best refinement order and the best *display* order coincide.

### 5.1 `sea_fraction` is the regime test, and it is computable before any descent

`sea_fraction(eps)` is the footprint-spread CDF. Its sensitivity says whether the threshold sits **in
the bulk or in a gap** — and *failing to cut the bulk is what makes the policy simultaneously good and
untunable* on those charts.

**A cheap estimator that avoids needing a full cache is the concrete unbuilt next step.** It would let
the system know which regime a slice is in *before* descending, which affects tiering, budget
prediction, and possibly the UI.

### 5.2 The tolerance is inert exactly where the policy works

`deep interior` returns **the same quad count at all three tolerances at all three horizons** —
225/225/225, 313/313/313, 309/309/309 — while delivering the grid's best saving, 16.3–18.6×. The sea
charts, where the policy delivers least, are the **most** tolerance-sensitive.

**Two causes of an inert `eps`, separated by the sea span:** a frozen tree with a *frozen*
`sea_fraction` is a gap in the field; a frozen tree with a *moving* `sea_fraction` is **another stop
overriding the tolerance** — `near-field` at `t = 50`, bitwise one tree while `sea_fraction` runs
0.0479 → 0.1011.

**`tau` is a second threshold and is not `eps` times a constant.** The working value ran `eps`,
`eps/10`, `eps/100` across `t = 13, 23, 50` on one chart, and `eps/3.3` over-refines `deep interior`
by 2.4× for no error gain. **Calibration moves cells by an order** — `near-field` at `t=13, eps=1e-3`
runs 0.30× at `tau=eps` and **15.26×** at `tau=eps/3.3`.

---

## 6. Decision variants, and the second budget line

`Keep`, `Split`, `Floor`, `ScreenFloor`, `Undetermined`, `Collapsed`, `Merged`, `BalanceForced`.

> **`Undetermined` means FINER `eta`, NOT FINER CELLS.** That is a **second refinement axis** and
> nothing in the architecture currently has one. It measured **exactly 0.0000 in all 36 cells** of the
> tolerance grid — but that is a property of the **shipped step control**, not of the policy. A chart
> that still integrates badly reintroduces it, and the budget model would then need two lines.

**Report the stop-reason breakdown WEIGHTED BY THE SUBTREE EACH STOP FORECLOSES.** Only **4 leaves**
read `floor` at level 2 in the `near-field` case — each foreclosing a quarter of the frame, suppressing
a descent that reaches 829 quads at zero error. **A decisive stop reads as a rounding error in a leaf
count.** This extends the standing rule (never quote a leaf count without its stop breakdown).

---

## 7. Open

- **The two `alpha_area` defects** (§2.2). Both are bugs with reproductions; the fixes are ruled (R-42) and land
  with the refinement task.
- **A cheap `sea_fraction` estimator** (§5.1) — the named next step.
- **Depth beyond level 6.** Every tree in the tolerance study is capped; nothing is known past it.
- **A calibrated grid.** All 36 cells ran shipped defaults; a calibrated `tau` moves several by an
  order.
- **The live playhead.** The tolerance study is static descents. Merging is measured on a pulse
  fixture but not under real camera motion.
- **Frontier scoping.** The camera now reaches depth (§0.1) but the frontier is still unbounded —
  29 → 585 quads while in-view work stays ~130, so the camera is a tie-break rather than a filter. A
  naive cull evicts faster than the descent refills and **stalls worse than the unfixed version**
  (170 px, 4 in view). The margin must be **derived from the refill rate**, which §0.1 has made
  computable: the in-view descent is now bounded by the screen floor rather than unbounded.
