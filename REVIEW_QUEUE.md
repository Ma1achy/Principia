# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

---

## RQ-1: The display-adequacy evidence exists only on one machine *(step 1)*

- **Where:** `principia_INDEX.md`, "Known open items": "Display adequacy landed at prin-rs `52caf14`".
- **Finding:** `52caf14` (15 Sep) is **not on prin-rs `main`** (`8600d45`, 8 Sep). It sits on the local
  `lowering-spike` branch of `~/src/principia-rs-test`, which is 9 commits ahead of `origin/lowering-spike`.
  Those commits have never been pushed, so a fresh clone can't see the evidence the index cites.
- **Needed:** push `lowering-spike` or merge it into `main`, or tell me what the index should cite instead.
- **Ruling:** R-2 (decisions.md) — applied in step 3

## RQ-2: μ_max: the LaTeX says 5, the audit says 4 *(step 1)*

- **LaTeX** (`principia_spec_revised.tex` ~2991): "Mass logit saturation `μ_max = 5`; … `α_min = 0.05`".
- **Audit B25** (IC Inspector open items): "chart constants μ_max = 4, α_min = 0.05, q_max = 2".
- **Also:** pending change 6 and the `.md` corpus say `α_min = 0` ("the `.md` corpus and the tool are
  already consistent at `α_min = 0`"), and B25 says 0.05.
- **Needed:** the real chart config's values. This joins B25 on the decision sheet in step 5.
- **Ruling:** R-5 (decisions.md) — applied in step 5

## RQ-3: The sphere colour-map PDF: is it retired too? *(step 1)*

- The handoff retires the LaTeX. It says nothing about `spec_sources/sphere_colour_map_spec.pdf`.
- `principia_dd_colouring.md` line 3 calls the PDF "already publication-grade" and takes "Eq. 5, verbatim" from it (§3.2).
  `principia_dd_integrator.md` 185 and 246 take the shape-sphere axis convention from its §8.1.
- **Needed:** a ruling. If the PDF is retired too, step 3 ports its §8 and Eq. 5 in the same way. If it stays
  authoritative, the markdown has two sources of truth and `canonical_spec` should say so.
  `com_projection_mini_spec.pdf` raises the same question on a smaller scale.
- **Ruling:** R-3 (decisions.md) — applied in step 3

## RQ-4: The event priority order has nothing in the LaTeX to check against *(step 1)*

- `principia_dd_integrator.md` §3.6 (line 244): "confirm against the spec's event-detection section or veto and re-pin".
- `.tex` `sec:events` (987–993) has only "Collision: `min‖rᵢ−rⱼ‖ < r_coll`. Record pair." and then "Escape".
  There is **no priority order in it**.
- So the pin in dd_integrator §3.6 stands with no cross-check, and audit B4 has no LaTeX backing for either choice.
  I'm recording this, not choosing.
- **Ruling:** R-6 (decisions.md) — applied in step 5

## RQ-5: Which defaults does "spec-keyed defaults" mean? *(step 1)*

- `principia_dd_telemetry_and_tiers.md` 398: "Until then the spec-keyed defaults are placeholders".
- It could mean the `.tex` quality tiers (`sec:quality_tiers`, 1959–1980) or the tier tables in the markdown.
  Step 3 needs to know which one before it repoints the reference.
- **Ruling:** R-4 (decisions.md) — applied in step 3

## RQ-6: Step 3's new done-check can't pass as written *(step 1, after R-1..R-6)*

- **Check** (HANDOFF step 3): `grep -rniE "latex|spec\.tex|the spec'" --exclude-dir=spec_sources --exclude-dir=archive .`
  must return "only the retirement ruling in `principia_canonical_spec.md`".
- **Why it can't:** it scans the whole repo, so it also matches:
  - the root working docs, whose job is to talk about the LaTeX: the handoff (which contains the grep
    string itself), `REVIEW_QUEUE.md`, `decisions.md` and `open-questions.md`;
  - every HTML file with CSS `translateX`, because the case-insensitive `latex` matches `transLATEX`. That
    includes `principia_dev_gui.html`, `poster_both_sides.html` and `stain_*.html`, which step 2 moves to
    `workbench/` or `docs/gui/`;
  - `.git/` (it isn't excluded);
  - the pending-changes register, which step 4 archives only *after* step 3.
- **Options:** scope the check to `docs/` (the plan's original form), or add `--exclude-dir={workbench,.git}`
  and `--include='*.md'`, and exclude the root working docs. Either way, run it after the register is
  archived, or exempt the register by name. **Applied verbatim in the handoff until you rule.**
- **Ruling:** R-7 — applied in step 3

## RQ-7: `findings.md` was archived on a false premise *(step 2, INDEX)*

- **Where:** `docs/archive/findings.md` (it was `spec_sources/findings.md`).
- **The premise:** in step 1 I called it "an older 17 Jul copy of prin-rs `FINDINGS.md`". The reviewer's
  step-2 layout sent it to `archive/` on that basis.
- **What it actually is:** it is byte-identical to `~/src/principia-spike/FINDINGS.md`, the **toolchain spike findings**
  ("Verdict: rust-gpu … bit-identical to native on every golden input"). It isn't a copy of prin-rs.
  It's the evidence for the substrate decision, and `principia_spike_brief.md` says the spike's findings
  "are propagated across the corpus".
- **Options:** keep it archived (the spike was delivered and its findings have been absorbed), or move it to
  `docs/experiments/results/` next to the spike brief, as the evidence for a settled decision. Either way,
  the INDEX row's "why archived" is written as "unclear" until you rule.
- **Ruling:** R-9 — applied in step 3

## RQ-8: R-5's μ_max premise doesn't match the IC Inspector notes *(step 3, re-sweep)*

- **R-5 says:** "`μ_max` is 5 in the LaTeX and 4 in the IC Inspector notes."
- **The IC Inspector notes** (`docs/notes/ic_inspector_scratchpad.md` line 20) say: "**`μ_max` was 4 → corrected
  to 5** (dd_decoder §3.1, chart_decoder §87/96, inverse_encode §63; at 4 the mass-saturation range was too
  narrow and `MASS_SAT` fired at the wrong `z_μ`)". The same passage records "**`α_min` REMOVED** … `α = (π/2)·σ(z_α)`".
- **The rest of the corpus:** `principia_dd_decoder.md:38` has `[μ_max = 5]`, and `principia_chart_reference.md:31` has
  "`μ_max = 4` is the recorded default (an open …)". Audit B25 has 4.
- **So** the 4 appears only in `chart_reference` and in the audit. The IC Inspector notes argue *against* 4.
  I'm not choosing. Under R-5 the step-5 decision sheet should carry this evidence, and the step-3 ports use
  the named symbol.
- **Ruling:** R-10 (decisions.md) — applied in step 3

## RQ-9: The per-body momentum cap exists only in the LaTeX *(step 3, dd_decoder port)*

- **LaTeX** (`sec:jacobi_mom`, `.tex` 247–255): after the Jacobi-to-particle map, "Apply same rotation/mirror as
  positions. **Optional per-body cap with COM re-enforcement.**"
- **Markdown** (`principia_dd_decoder.md` §3.4): the momentum decode and the full-state rotation/mirror rule are there.
  No per-body cap is mentioned anywhere in the corpus.
- **I can't tell** whether the cap was dropped on purpose (the `q_max` saturation already bounds the Jacobi momenta)
  or lost in transcription. It isn't ported. Rule on it: port it as an optional step, or record it as dropped.
- **Ruling:** R-11 (decisions.md) — applied in step 3

## RQ-10: The shape-sphere chart map: axis assignment and polar buffer *(step 3, chart_reference port)*

- **Markdown** (`principia_chart_reference.md` §3.3, spherical option): $\theta = \pi v$, $\varphi = 2\pi u$. θ is on the
  **vertical** axis and there's **no buffer**.
- **LaTeX** (`sec:shape_sphere_view`, `.tex` 333–383): $\theta(u) = \varepsilon + (\pi - 2\varepsilon)u$, $\varphi(v) = 2\pi v$.
  θ is on the **horizontal** axis, with a buffer $\varepsilon > 0$ "to avoid the collision-singularity poles".
- **Also:** the LaTeX's reason for the buffer is itself doubtful. Under the markdown's own §3.4 the collision points
  sit **on the equator**, not at the poles (the poles are the Lagrange configurations).
- **Not ported. The markdown is unchanged.** Rule on the axis assignment and on whether θ gets a buffer.
- **Ruling:** R-12 (decisions.md) — applied in step 3

## RQ-11: Lookup's decode-sanity check vs "a t = 0 collision is a real outcome" *(step 3, inverse_encode port)*

- **Ported, as the LaTeX wrote it** (`sec:chart_validation`, `.tex` 771–815 → `principia_inverse_encode_contract.md`,
  Chart-aware validation, layer 3): decode sanity requires "no two bodies coincident ($r_{ij} > r_{\mathrm{coll}}$)".
  A failure means project, clamp or reject.
- **The markdown elsewhere:** `principia_chart_reference.md` §0.7 and the LaTeX's own `sec:no_holes_impl` say
  "`COLLISION_T0(pair)` with `t_event = 0` if `r_min(0) < r_coll`. **No pixel is ever rejected.**"
  `principia_integrator_contract.md` 348 calls a t = 0 terminal "a real outcome".
- **The tension:** for a *rendered pixel* $r < r_{\mathrm{coll}}$ at t = 0 is a labelled outcome. For a *typed-in lookup*
  the ported rule refuses or moves it. That may be intended (lookup is user entry, not rendering), or it may be stale.
  I'm not choosing. Rule on whether lookup should accept and label a t = 0 collision.
- **Ruling:** R-13 (decisions.md) — applied in step 3

## RQ-12: The shape-sphere axis convention doesn't line up, and it undercuts R-12 *(step 3, port 6)*

R-12 asked me to check the colour PDF §8 axis convention against the chart map and flag any mismatch. Three turn up.
Nothing is chosen. The PDF convention is ported into `principia_dd_integrator.md` §3.7 as it stands.

1. **The first axis is reversed against the overlay.** The overlay (colour PDF §8, now dd_integrator §3.7) puts
   `b̂₁ = (1,0,0)`. dd_integrator's shape map has $u = \|\tilde\rho\|^2 - \|\tilde\lambda\|^2$, so the inner-pair collision
   ($\tilde\rho = 0$) sits at **$u = -1$** and $u = +1$ is $\tilde\lambda = 0$ (body 2 at the inner CoM, an Euler point for equal
   masses). Either the first axis is $x = -u$, or `b̂₁` is a different pair than the inner one.
2. **Two markdown files disagree on the third axis's sign.** dd_integrator §3.7 has $w = +2(\tilde\rho \wedge \tilde\lambda)$.
   chart_reference §3.1 has $q = \tilde\rho_y\tilde\lambda_x - \tilde\rho_x\tilde\lambda_y$ ("NEGATIVE of the standard 2D cross") as its
   third component. So the two files put $L^+$ and $L^-$ at opposite poles.
3. **R-12's premise is false for the markdown's own chart map.** chart_reference §3.3's spherical map is
   $n = (\cos\theta, \sin\theta\cos\varphi, \sin\theta\sin\varphi)$ with $\theta = \pi v$, so θ's poles are on the **first** component,
   $n_0 = (a-b)/I$. Verified numerically: $\theta = \pi$ gives $\|\tilde\rho\|^2 = 0$, **the binary collision of bodies 0 and 1**.
   $\theta = 0$ gives $\tilde\lambda = 0$. So in this chart map a collision point *is* at a pole, and the LaTeX's reason for a polar
   buffer holds here. R-12 assumed the overlay convention (Lagrange at the poles), and in that convention
   the premise is indeed false. **The R-12 note now in chart_reference §3.3 ("Its premise is wrong …") is
   therefore itself wrong as written.** I left it in place because it's your ruling, and I'm flagging it rather than editing it.
- **Needed:** one component → axis convention for `n` (the order and the signs), used by the chart map, the
  shape readout and the overlay alike. Then a re-ruling on the polar buffer under that convention.
- **Ruling:** R-14 (decisions.md) — applied in step 3

## RQ-13: The scheduler contract's refinement rule predates `Policy::Tolerance` *(step 3, port 8)*

`principia_scheduler_contract.md` Parts 4 and 6 describe the refinement rule the LaTeX had (Part 6 was headed "The settled
policy (from the spec…)"). `principia_dd_refinement_policy.md` (landed, pending change 12) replaces it. They disagree in three
places. Nothing is chosen. Part 6 now carries a note pointing here, and its text is otherwise unchanged.

1. **What triggers a split.**
   - Scheduler Part 6: "Split if any spread/impurity threshold is exceeded (`outcome impurity`, `S_n`, `S_t`, `S_L`, `S_f` when
     `FTLE_VALID`, `S_D`, low `ensemble_outcome_agreement`, persistent parent-child disagreement …) **and**
     `ℓ < camera_depth + MAX_REL_DEPTH`." Part 4: "Refinement happens iff `S_quad > τ(ℓ)` **AND** no veto has fired."
   - Refinement policy §1: "`split(quad) ⟺ any footprint f in quad is unresolved`", where unresolved is `spread_shape(f) > eps`,
     or the copies disagree on event class, or the footprint is undetermined. One knob, `eps`, replaces the per-metric thresholds.
2. **In view, above the screen floor.**
   - Scheduler Part 4: complexity is the sole trigger, so a smooth in-view quad above pixel size may stay coarse. Part 6: "Keep
     coarse if dominant purity high, all spreads low … **Default is keep.**"
   - Refinement policy §0.1: "`Keep ⟺ n_unresolved == 0 AND tile_size_px ≤ 1`" — "every in-view quad above the screen floor must
     split"; there, "the camera decides depth and the criterion decides ORDER".
3. **Below the screen floor.**
   - Scheduler Part 4: the screen floor is a veto: "`tile_size(quad, zoom) ≤ pixel_size` → stop refining".
   - Refinement policy §0.1 table: "in view, below screen floor | the **criterion** — supersampling where unresolved | decides depth".
- **Needed:** which rule the scheduler contract states. If it's `Policy::Tolerance`, Parts 4 and 6 are rewritten from the
  refinement policy doc in step 4, when pending change 12 is folded. Priority weights, eviction and cancellation aren't
  affected.
- **Ruling:** R-15 (decisions.md) — applied in step 4

## RQ-14: colour_composition's golden images are pinned to the retired colour PDF *(step 3, port 10)*

R-3 retires `sphere_colour_map_spec.pdf`. `principia_colour_composition.md` still uses it as a test reference, and that
can't be ported as text:
- §1.1: "Fidelity to the PDF is pinned by golden-image tests (§7), not by a special type."
- §7: "Every currently-specified map (the PDF's Artefact-1 colour maps, Artefact-2 patterns, special modes, the physics
  overlay) and every debug view is **recreated as a composition preset**."

The PDF is the only full list of the Artefact-1 maps and Artefact-2 patterns. It is also the only source of the images a golden test would
compare against. I've marked the PDF as retired in colour_composition's header and in its supersession list (§8). The two
sentences above are left as they are. Nothing is chosen.
- **Needed:** where the golden images come from once the PDF is archived. For example, the archived PDF stays the test reference, or the
  first reviewed renders of the preset library become the goldens and the map list moves into §7's preset table.

Also recorded here, not chosen: colour_composition §3 gave the default invalid colour as "(spec: a fixed magenta)". I
found no source that says so. The `.tex` and the colour PDF use magenta only as a hue in a colour scheme (the PDF's
`−ŷ` pole). The attribution is removed, and "a fixed magenta" stays as the markdown's own default.
- **Ruling:** R-16 (decisions.md) — applied in step 3

## RQ-15: When the diffusion sentinel fires *(step 3, port 10; found by the removed-lines audit)*

Two markdown accounts of `diffusion` disagree. B10 had changed the render contract to match the second one. That changed a
decision without a ruling, so the change is reverted and the conflict is flagged here instead. Nothing is chosen.
- `principia_render_contract.md` Part 4, "Sentinels, not NaN": "`diffusion = −1.0` when the **two-window fit** is invalid".
- `principia_dd_integrator.md` §3.5: "**Diffusion (Welford streaming, per macro-step).** Slope of spread `y` on time `t` via
  centered co-moments". `principia_dd_simstate_payload.md`: "`diffusion_slope` … **Invalid for `n < 2`** (`C_tt=0`) → sentinel
  slope". `principia_dd_generation_root.md` §3.4: "`diffusion` | lin | **sentinel −1.0** = fit invalid".
- **Needed:** which fit the sentinel belongs to. If it's the streaming slope, the render contract's "two-window" is reworded.
- **Ruling:** R-17 (decisions.md) — applied in step 3

## RQ-16: The ensemble agreement scalar has three accounts *(step 3, port 9; found by the removed-lines audit)*

B9 had renamed the sampling note's `ensemble_outcome_agreement` to the ledger's `spread_event`. That changed a decision
without a ruling, so the change is reverted and the conflict is flagged here instead. Nothing is chosen.
- `principia_sampling_msaa_note.md` ("Ensemble copies ARE the SSAA samples") and `principia_scheduler_contract.md` Part 6
  (split on "low `ensemble_outcome_agreement`") name a tile-level **agreement** scalar.
- `principia_dd_generation_root.md` §3.7 stores `spread_event` = "`disagreement(class⊕detail) / (1 − 1/(E+1))`", a
  normalised **disagreement**, in `QuadReduction`.
- `principia_render_contract.md` Part 6: "outcome agreement, spread — **derived at resolve** from the footprint's E+1 samples
  (not a stored field)".
- **Needed:** whether `ensemble_outcome_agreement` is `spread_event` (same quantity, opposite sense) or a separate field, and
  whether it's stored or derived. RQ-13 may also settle the scheduler's use of it.
- **Ruling:** R-18 (decisions.md) — applied in step 3

## RQ-17: Did change 8's `ADVANCE` signature land? *(step 4, register)*

The register, the index and the integrator contract disagree on change 8. Nothing is chosen, and Part 2a is left as it is.
- Register banner above change 8: "**LANDED, and went further than proposed.** Regularisation is now a **second swappable
  axis**, independent of the stepper — `principia_integrator_contract.md` Part 2b." The index lists 8 as landed.
- Register change 8 status: "Open — **decided in principle**, not yet written into the LaTeX."
- Integrator contract Part 2a: "**PROPOSED CHANGE** … `STEP(state, dt, params) -> state' # current` /
  `ADVANCE(state, t_now, t_target, params) -> state' # proposed`", with `owns_time_mapping`, the per-substep cadence as a
  callback, and two AZ rows ("AZ + RK4", "AZ + time-transformed leapfrog").
- Part 2b (DECIDED) covers the regularisation axis but doesn't mention `ADVANCE`, `owns_time_mapping` or the callback.
- **Needed:** whether the landed form includes `ADVANCE` / `owns_time_mapping` / the cadence callback (Part 2a becomes
  current), or whether Part 2b replaced them (Part 2a is marked superseded). Either way, which AZ rows are in the table.
- **Ruling:** R-19 (decisions.md) — applied in step 4

## RQ-18: The impurity mask's field — `majority_class`, or change 1's joint grain *(step 4, register)*

Change 1 is resolved, and render_contract Part 6 still asks for the field the resolution says isn't needed. Nothing is chosen.
- Register change 1 (RESOLVED, landed in `principia_dd_generation_root.md` §3.7): "Defining *every* event-derived reduction
  field at the joint `class ⊕ detail` grain removes the question: one grain, so no commuting problem and **no companion
  field**. Storing the class histogram and deriving both dominant and impurity from it makes disagreement impossible."
- `principia_render_contract.md` Part 6, impurity mask: "per-sample `state` ≠ quad majority `state` (**requires
  `majority_class` added to `QuadReduction`/`RenderQuad`** — one u32, do it)".
- **Needed:** whether the mask compares at the joint grain against the existing `dominant_outcome` (no new field; the mask
  definition changes from `state` to `class ⊕ detail`), or keeps a class-only `majority_class`.
- **Ruling:** R-20 (decisions.md) — applied in step 4

## RQ-19: No t = 0 escape outcome under R-29 *(step 5, applying R-29)*

R-29's settling test needs `|Δn̂|` over 0.4 time units, so it can't be evaluated on the decoded IC before the first step.
The corpus had a t = 0 escape outcome under the old gate:
- integrator_contract Part 5 (was): "a valid IC already satisfying the *complete* escape gate (outward + positive
  outer-energy, not merely beyond `R_esc`) → `state=escape, t_end_step=0`."
- dd_simstate_payload §2 (was): "an IC that at t=0 genuinely satisfies the complete escape detector … is an **escape at
  step 0** (`state=escape`, `detail=body`, `t_end_step=0`)."

Applying R-29, both now say escape has no t = 0 case, and an IC that is already escaping is classified when its window
completes. That follows from the rule as written; it is not a separate choice. The t = 0 collision outcome is unchanged.
- **Needed:** confirm, or rule a t = 0 escape test (for example `E_rel > 0` alone at t = 0, which is the "energy alone
  flickers" failure the criterion exists to avoid).
