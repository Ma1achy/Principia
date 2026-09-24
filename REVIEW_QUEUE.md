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
- **Ruling:** R-60 (decisions.md): confirmed, no t = 0 escape; the only valid t = 0 terminal is a collision. Closed in step 5.

## RQ-20: The stain editor — a free node graph, or a four-slot inspector *(step 6, GUI)*

The corpus disagrees with itself, and the notes side with one half. Nothing is chosen. The rewritten render_gui_spec keeps the node
graph (its existing §3–§11), and gui_state_contract §5 is left as it is, marked with this entry.
- `principia_gui_state_contract.md` §5: "The editor is a **four-slot inspector** (fixed wiring — stage order is
  constitutional, so there is no free-form topology to build or mis-wire)"; the occupants are
  `{colour_id, brightness_id, combiner_id, post_id, uniforms}`; §7 (teardown): "edit the four-slot object for colour".
- `principia_render_gui_spec.md` §3–§4, §15 items 1–6: source nodes, a "free-ish graph with a fixed OUT + combiner
  backbone", fan-out, multi-input nodes, and a variable-length post chain.
- GUI_DESIGN_NOTES 02: "Stain — the plain node-graph editor … Graph with typed pins (field / colour / brightness) and wires."
- **Needed:** whether the stain is a graph (and gui_state_contract §5/§7 are rewritten to a graph object, with post as a
  chain rather than one `post_id`), or the four-slot object (and the graph editor is a view over it).
- **Ruling:** R-64 (decisions.md): the free, typed node graph. Closed in step 6.

## RQ-21: One inspector window, or the click inspector plus a separate IC Inspector *(step 6, GUI)*

Nothing is chosen; the spec writes the notes' window and marks the difference.
- GUI_DESIGN_NOTES 05: "The IC Inspector and the trajectory viewer are ONE window." Pane 2 is "bodies or shape sphere
  (turning, with axes, or unwrapped)", a toggle; the sphere "turns slowly".
- `principia_trajectory_viewing.md` §4: the click inspector shows **four** things at once — "3D shape sphere
  (rotatable)", "2D UV unwrap", "Real space", "Scalar readout" — and "The 3D orbit control rotates the **camera, not the
  data**". `ic_inspector_scratchpad.md` Build notes: the IC Inspector is its own tool whose "eventual home is an **egui**
  panel in the F3 debug menu".
- **Needed:** whether the merged window supersedes trajectory_viewing §4's panel set (sphere and unwrap as a toggle, and an
  auto-turning sphere), or keeps both sphere views on screen together.
- **Ruling:** R-65 (decisions.md): one Inspector window; the panels are hosted there and in Explore's Trajectory panel. Closed in step 6.

## RQ-22: A time scrubber that re-integrates, against "there is no scrub" *(step 6, GUI)*

Nothing is chosen.
- GUI_DESIGN_NOTES 01, Time: "play, step, a scrubber. **Scrubbing back re-integrates** to that time, so the figure
  refines progressively. It is not instant, and says so."
- `principia_export_animation_contract.md` Part 1: "There is no scrub — the playhead is a clock, not a slider over stored
  data." Its transport controls are play/pause, restart, loop and speed. `principia_temporal_architecture_note.md`: "No
  scrub; playback only."
- The notes' scrubber stores nothing (it re-marches), so it may be compatible with lockstep, but the contract names no
  seek control and says there is no scrub.
- **Needed:** whether a seek-by-re-march transport control joins export_animation Part 1 (and its catch-up rules), or the
  Time panel has no scrubber.
- **Ruling:** R-66 (decisions.md): the scrubber stays; "no scrub" applies to exported animations only. Closed in step 6.

## RQ-23: The display stage — the style stage, and where gamut clamp, display scale and the controls sit *(step 6, GUI)*

Nothing is chosen.
- GUI_DESIGN_NOTES 04, Display: "fixed order — SimResult → stain → style → colour-vision simulation → screen. Style is
  optional and applies to the figure only; scientific checks run with plain." The display settings are a **window**, and
  the overlays are a top-bar **Overlays ▾** menu (01).
- `principia_render_gui_spec.md` §12: the display stage is "**gamut clamp**, **CVD simulation** …, **render→display
  scale**", in "the top display bar"; §1: "Global display bar (top) — gamut / CVD / render-scale / boundary-overlay";
  §15 item 11. There is no style stage in the corpus.
- **Needed:** (a) where gamut clamp and render→display scale sit in the notes' order (before or after style; the notes
  don't list them); (b) whether the "global display bar" placement is replaced by the Display window and Overlays menu.
  The spec keeps both corpus settings, and keeps the display stage global and outside the pipeline.
- **Ruling:** R-67 (decisions.md): stain → style → display scale → gamut clamp → colour-vision simulation → screen; the
  Display window and Overlays menu replace the top display bar. Closed in step 6.

## RQ-24: Artboard details that differ from the corpus *(step 6, GUI)*

The pictures differ from the corpus in these details, and the notes say nothing about them. The spec follows the corpus on each;
please confirm.
- **Outcome palette** (02, 06): e.g. bounded `#3f4652`, body-escape `#d6a83c`. colour_composition §3's canonical
  palette is bounded `#141418`, body 0 escape `#F0DE32`, and so on, with golden tests pinned to it (R-16).
- **Substep cap** (04 Run, and the profiler's "100k (cap)"): 100 000. integrator_contract Part 3: `N_max` default 64.
- **"tolerance ε 1e-9"** under Integration (04 Run): integrator_contract has no integrator tolerance. The step is
  `dt_macro` (fixed) with substepping, and `eps` is the refinement tolerance (`Policy::Tolerance`, R-15).
- **Sonification mapping** (01, 05: "separations → pitch", a selector): scratchpad_pointer_channels and trajectory_viewing
  define one mapping, `θ(t), φ(t)` → spectrum.

Settled by a ruling or the notes, so not questions: the 1-based labels (R-22); the escape "persistence 8" (R-29, change
11); the profiler's top-level categories (R-56: telemetry §2's five stages); "linked camera", "CameraZoom" and "fate
edges" (notes: no camera object; "Legend", never "Fate").
- **Ruling:** R-68 (decisions.md): artboard values are illustrative; corpus values win; the Run window uses contract
  names. Closed in step 6.

## RQ-25: Is the logH experiment and a refinement re-take milestone work, or settled? *(step 7, build plan)*

Nothing is chosen.
- `principia_00_philosophy.md` §7.8 "Sequencing — what is next, and why in this order" (:444): (1) "**logH experiment.** The
  falsification test for the re-registration mechanism"; (2) "**The refinement mechanism, from scratch.**" — "**What must be
  re-taken:** everything else, *including* "nothing beats breadth-first""; only then (3) "The GUI, and actually building the thing."
- `principia_canonical_spec.md` §11, "Milestone/implementation build plan" (:153): "**The vertical slice has since collapsed the
  research phases** — integrator, step control, escape criterion and refinement policy are settled with evidence — so the plan
  is now a *build* plan."
- **Needed:** whether the build plan carries the logH experiment and a refinement re-take as milestone work (and before which
  milestone), or treats both as settled by the vertical slice (and §7.8 is marked as superseded by §11).
- **Ruling:** R-74 (decisions.md). Closed in step 7.

## RQ-26: Kernel debug modes — four baked kernel variants, or fragment presets plus one bring-up mode *(step 7, render)*

Nothing is chosen. R-41 settles only baked variants vs flag bits. It doesn't say which kernel modes exist.
- `principia_render_contract.md` Part 6, Cross-check views (:202): "**Kernel debug dispatch modes** (a `DEBUG_MODE` enum,
  selected as a baked kernel variant …): `NORMAL`, `UV_PASSTHROUGH`, `DECODE_PASSTHROUGH`, `ROUNDTRIP`".
  `principia_lowering_contract.md` compute-side table (:52): "Kernel debug modes (UV / DECODE / ROUNDTRIP) | **BAKED**".
  `principia_debug_tooling_plan.md` §A (:26) lists the same four.
- `principia_colour_composition.md` §6 (:379): "**§A kernel modes → mostly presets, via fragment-side recompute (§3).**"
  Appendix A (:508): "The **only** debug item that is *not* a render-key preset and *does* touch the kernel. A single
  minimal mode". Its header (:6) says it supersedes "the mode-enumeration in `principia_debug_tooling_plan.md` §B–§G".
- **Needed:** whether the kernel keeps the four `DEBUG_MODE` variants, or only Appendix A's bring-up mode (UV / DECODE /
  ROUNDTRIP become fragment presets). If the latter, render_contract Part 6, lowering :52 and debug_tooling_plan §A are rewritten.
- **Ruling:** R-75 (decisions.md). Closed in step 7.

## RQ-27: Stability × Hue — deleted, or the house pattern *(step 7, colour)*

Nothing is chosen.
- `principia_colour_composition.md` §4.1 (:288): "This **replaces the deleted "Stability × Hue"**".
- The same file's §7 table (:424) still lists "| **Stability × Hue** | pipeline preset: … `brightness = FieldRamp{stability, lin}`",
  and §7.1 (:480) lists "Stability × Hue | the house encoding, dd_colouring §3.4". Golden tests are pinned to §7.1 (R-16).
- `principia_render_contract.md` Part 4 (:71): "Stability×Hue is the house pattern: hue = shape-sphere position, L = metric,
  default BC proximity". `principia_dd_colouring.md` §3.4 heading (:88): "the house encoding (stability × hue)", with
  `L = 0.25 + 0.55 · ½(1 − maxⱼ n̂·b̂ⱼ)`.
- **Needed:** whether Stability × Hue is a preset (and on the golden list), or deleted (and §7, §7.1, render_contract Part 4 and
  dd_colouring §3.4 drop it). If kept, what `stability` is as a field.
- **Ruling:** R-76 (decisions.md). Closed in step 7.

## RQ-28: dd_colouring vs colour_composition — the Replace-L mapping and the outcome palette *(step 7, colour)*

Nothing is chosen. colour_composition §8 (:494) says dd_colouring's "combine L-ownership rules (Replace-L / Multiply) are
unchanged and referenced by §4.1", and that dd_colouring's "mode-by-mode presentation is superseded".
1. **Replace-L.** `principia_dd_colouring.md` §3.5 (:103): "`L ← L_min + (L_max − L_min)·b`". `principia_colour_composition.md`
   §4.1 truth table (:272–274): "`OKLab(L=B, a=Cₐ, b=C_b)`" and "`OKLab(L=B, 0, 0)`". These agree only if `L_min = 0`,
   `L_max = 1`. Neither file gives default `L_min` / `L_max`.
2. **The outcome palette.** dd_colouring §3.7: "State → palette index (Okabe–Ito cycle ≤ 8, golden-angle beyond". colour_composition
   §1.4 (:150–162): the `state` field has "a **canonical default palette**" of nine fixed sRGB classes. R-68 says the corpus's
   palette hex codes win over the artboards, but doesn't say which corpus palette.
- **Needed:** (1) which Replace-L formula, and the default `L_min` / `L_max` if the range form stays; (2) whether §1.4's
  nine-class palette replaces dd_colouring §3.7's index rule for `state`.
- **Ruling:** R-77 (decisions.md). Closed in step 7.

## RQ-29: The CVD matrices are not the Viénot/Brettel forms the text names *(step 7, colour)*

Nothing is chosen.
- `principia_colour_composition.md` §4.3 (:329): "The CVD matrices and linear-sRGB path are the Viénot/Brettel forms already in
  the reference artefacts." §8 (:497) points to them: "its Eq. 5 and CVD matrices are in `principia_dd_colouring.md` §3.2 and §3.8".
- `principia_dd_colouring.md` §3.8 (:136): single 3×3 matrices on linear RGB, e.g. deutan
  `(0.625 0.375 0 / 0.700 0.300 0 / 0 0.300 0.700)`. Viénot (1999) and Brettel (1997) work through LMS space; these matrices
  are not those forms.
- **Needed:** which is authoritative: the §3.8 matrices as written, or real Viénot/Brettel (and the reference to replace them
  with, for the golden tests).
- **Ruling:** R-78 (decisions.md). Closed in step 7.

## RQ-30: Invalid values — NaN or sentinel, in storage and on screen *(step 7, render)*

Nothing is chosen. Three passages pull different ways.
1. **An absent field.** `principia_render_contract.md` Part 3 (:65): "reading anyway shows the sentinel/0 with the suspect
   styling". Part 2 and the unpack layer (:15 area): an absent feature reads NaN (at E = 0, `ensemble_spread` → NaN).
2. **A blown-up sample.** render_contract Field views, SimState row (:175): "**NaN is a deliberate sentinel** for a tier-absent
   feature or a blown-up sample". Part 4 (:79): "**never NaN in storage buffers**". A tier-absent value is derived, not stored;
   a blown-up sample's stored fields are.
3. **Debug fields.** `principia_render_gui_spec.md` §13 (:714): "every colouring has an explicit invalid-pixel colour — a NaN /
   sentinel must read as "no data", not as a value". §10.1 (:600): "**Debug fields are raw:** apart from the NaN guard there is
   no validity masking — a failed-state sentinel (e.g. `0.0`) is shown as its literal value".
- **Needed:** (1) whether an absent field reads NaN or sentinel/0; (2) whether a blown-up sample may store NaN (payload §2's
  "Failed-state contents are defined" may already answer it); (3) whether debug fields are a stated exception to §13.
- **Ruling:** R-79 (decisions.md). Closed in step 7.

## RQ-31: How many samples per footprint, and where copy 0 sits *(step 7, sampling)*

Nothing is chosen.
- `principia_sampling_msaa_note.md` "The sampling pattern" (:95): "**copy_index** — `0 … E−1`: which of the E ensemble copies …
  Copy 0 = the un-jittered nominal (offset 0, i.e. the pixel centre); copies 1…E−1 are Halton points 1…E−1." That is E
  samples in all, E−1 of them jittered.
- The same note (:29, :44, :68), render_contract and dd_colouring: "the resolve averages the E+1 colours"; "(E+1) full sims".
  That is a nominal plus E jittered copies.
- Also: Halton index 0 is `(0, 0)`, and Halton points lie in `[0,1)²`. "offset 0, i.e. the pixel centre" holds only if offsets
  are centred (e.g. minus ½). The note doesn't say.
- **Needed:** whether a footprint has E or E+1 samples (and the copy_index range to match), and whether the Halton offsets are
  centred on the nominal.
- **Ruling:** R-80 (decisions.md). Closed in step 7.

## RQ-32: The embedded "What travels" block uses the prototype's names and a 10-D latent *(step 7, image embedding)*

Nothing is chosen.
- `principia_dd_image_embedding.md` §6 (:106–109): "slice  z0[10], dimH, dimV, mag, zoom, pan, tilt, gamma"; "sim  horizon,
  dtMacro, maxSteps, rColl, rEsc, eta, nSync"; "ensemble  E, N, jitter_frac".
- `principia_colour_composition.md` §3 (:224): "`z` (the full 8-D latent"; `principia_canonical_spec.md` :48: "`z ∈ ℝ⁸`". The
  contracts name `T`, `dt_macro`, `N_max`, `r_coll`, … ; `jitter_frac` has no counterpart in the fixed Halton-offset model (RQ-31).
- **Needed:** whether the embedded record is rewritten against the contract names and the 8-D latent (and what replaces
  `jitter_frac`), or keeps the prototype's fields with a mapping table.
- **Ruling:** R-81 (decisions.md). Closed in step 7.

## RQ-33: Two decode rules that differ between files — the mirror deadband and seed selection *(step 7, decode)*

Nothing is chosen. T1 (bit-determinism) depends on both.
1. **The mirror boundary and its variable.**
   - `principia_dd_decoder.md` §3.3 (:90–91): "mirror … if λ_y < 0, with deadband |λ_y| < δ_λ = 10⁻¹² → fixed no-mirror choice".
   - `principia_chart_reference.md` §0.4 (:97): "if λ_y < −δ_λ:          mirror". `principia_dd_encode.md` §3.2 (:48–49):
     "if λ_y < −δ_λ" and "|λ_y| ≤ δ_λ = 10⁻¹² : deterministic no-mirror".
   - `principia_inverse_encode_contract.md` Part 6 (:151): "Mirror if `λ̃_y < 0` … Tie `|λ̃_y| < δ_λ`", on λ̃ = √μ_λ·λ, not λ.
   - At λ_y = −δ_λ exactly, the first and last mirror and the middle two don't; and the threshold differs by √μ_λ.
2. **The momentum seed.** chart_reference §2.2 (:256–257): "take the first seed with `‖w⁽²⁾‖²_m > ε_w` (default `1e−10`); if
   several qualify, **choose the largest `‖w⁽²⁾‖_m` for conditioning**". "First" and "largest" pick different seeds. dd_decoder
   §3.4 (:121): "all seeds < ε_w → DEGENERATE" puts the threshold on the norm, not its square.
- **Needed:** (1) one mirror test: the variable (λ or λ̃), and `<` vs `≤` at `−δ_λ`; (2) first-qualifying or largest seed, and
  whether `ε_w` bounds the norm or its square.
- **Ruling:** R-82 (decisions.md). Closed in step 7.

## RQ-34: The affine slice — a common scale in `q`, or per-axis `s_u`, `s_v` *(step 7, chart)*

Nothing is chosen.
- `principia_chart_decoder_contract.md` Part 3 (:113): "z(s,t) = z₀ + (2s−1) q₁ + (2t−1) q₂", with zoom as a common scale on `q`
  (Part 4). `principia_coordinate_conventions_note.md` (:38): "same map, one authoritative form."
- `principia_chart_reference.md` §1.1 (:142): "z(u, v) = z0 + (2u − 1)·s_u·q_1 + (2v − 1)·s_v·q_2". `s_u`, `s_v` are not defined.
- **Needed:** whether the scale lives in `q` (chart_reference drops `s_u`, `s_v`) or in separate per-axis factors (and where they
  live in the view state and the lock formula).
- **Ruling:** R-83 (decisions.md). Closed in step 7.

## RQ-35: Must branch decisions match across precisions along a trajectory? *(step 7, parity)*

Nothing is chosen.
- **Yes:** `principia_integrator_contract.md` (:299): "branch decisions still match, by the comparison-only rule — Part 4";
  Part 4 (:343): "values may diverge by precision; **branch decisions in the wrapper may not**". `principia_dd_integrator.md`
  seam 1 (:258): "branch-trace equality over fuzzed ICs"; §3.3: "`N_sub`, `state`, `total_substeps`, and terminal labels are
  **bit-identical across CPU-f64, CPU-f32, …** on every golden input". `principia_gpu_determinism_note.md`: branch words "are
  asserted **bit-exact**".
- **No:** `principia_parity_contract.md` Tier L (:47): "The previous wording said *"branch decisions must be identical across
  precisions"*. **That is false and cannot be made true.**" Tier B table (:101): "a **label on a real trajectory** | **none — it
  will differ**".
- **And, within the parity contract:** Tier S (:120): "**do** assert: same **outcome class** (Tier L, exact)" — against Tier B's
  "none — it will differ" for a label on a real trajectory.
- **Needed:** the guarantee: branch decisions equal on identical inputs only (the integrator and determinism texts are
  reworded), or along whole golden trajectories (and on which inputs); and, following from it, whether Tier S asserts the
  outcome class.
- **Ruling:** R-84 (decisions.md). Closed in step 7.

## RQ-36: Which backends pin the Tier-N tolerances — CI Dawn, or native `wgpu` *(step 7, parity)*

Nothing is chosen.
- `principia_parity_contract.md` §4 (:144): "run the same kernel on the CI Dawn backend and on a real browser … the matrix is
  small — CI Dawn + one or two real browsers".
- §6 (:173): the sim-parity runner is native, in-process via `wgpu`: "Native in-process parity is *simpler* than the old
  Dawn-in-Node harness".
- **Needed:** whether CI still runs Dawn, and which backend pair sets the Tier-N tolerances.
- **Ruling:** R-85 (decisions.md). Closed in step 7.

## RQ-37: Payload descriptions that disagree with the payload doc *(step 7, payload)*

Nothing is chosen. `principia_dd_simstate_payload.md` is the consolidated doc; the ledger's own canonical clause
(`principia_dd_generation_root.md` :29) covers only its §3.1–3.3a. These are outside that clause.
1. **The `times` word.** payload §2 (:192): "**Exact unsigned 16-bit macro-step indices** — the *only* format (no Q0.16
   fallback". `principia_render_contract.md` unpack layer (:111): "requires horizon_steps = ceil(T/dt) <= 65535 (dispatch
   invariant); else these revert to Q0.16 normalised". Ledger §5 test 4 (:443): "**Fixed-point:** `t_end`/`t_dmin` (in `times`)
   round-trip with ≤ 1/65535 error". Also unstated: what dispatch does when `ceil(T/dt) > 65535` (refuse the config?).
2. **Phase-state grouping.** Ledger §3.5 (:151): "`r, p` — 12 × f32 (vec4-grouped for alignment)". payload §1 (:36):
   `array<vec2<f32>, 3>`, and §6: 8-byte aligned, vec2 groupings.
3. **The debug catalogue's struct tables.** `principia_debug_tooling_plan.md` §D (:70): "`SimState` scalars (11 × f32)" over a
   10-row table that lists `t_end_step` as a scalar (§C puts it in `times`) and `d_min` as an f32 (§B: "`d_min`, `dE_max`,
   `dLz_max` (f16, packed)"). §E (:89): "`ICDescriptor` (12 × f32)" = 48 B, against `principia_canonical_spec.md` :79
   "`ICDescriptor` (64 B)"; the padding isn't stated.
4. **The `ICDescriptor` field set.** `principia_dd_decoder.md` §3.6 (:135–136) lists "E₀ = K₀ + V₀ ;   virial_ratio = 2K₀ / |V₀| ;   ρ-magnitudes,
   ρ_ratio, ρ_angle, r_min_pair₀" as ICDescriptor quantities. Ledger §3.6 (:162) has `m0 m1 m2`, `q_mass`, …, `K_0`, `V_0`,
   `virial_ratio`, `r_min_pair_0` — `q_mass` and no `E₀` (render_contract :175 puts `E_0` among the `SimState` scalars).
5. **Descriptor bits 8–15.** payload §2 (:163): "**`last_symbol` (bits 8–9) is a deliberate, versioned assignment**", reserved
   10–15. `principia_canonical_spec.md` :79: "Descriptor: 8 bits used, 8–15 reserved." render_contract (:91, :96, :115, :120):
   "bits 8–15 are reserved"; (:176) "`sample_descriptor` sub-fields — 8 bits". (Ledger :41–43 says the same, under its clause.)
6. **The `saturated` condition.** payload §2: set when "`N_sub == N_max` occurred". render_contract (:94): "substep exponent ever
   hit ⌈log2 N_max⌉"; `principia_debug_tooling_plan.md` §B (:45): "set iff the substep exponent hit `⌈log2 N_max⌉`". Ledger :39
   agrees with the exponent form. The two agree only for power-of-two `N_max` (R-68's default 64 is).
7. **The complexity proxy's rounding.** payload §6 (:396): `select(0u, 31u - countLeadingZeros(max(total,1u)), total > 1u)` and
   render_contract (:97) `31u - countLeadingZeros(max(n, 1u))` are ⌊log₂⌋ (and differ at `total = 1`). debug_tooling_plan §B
   (:47): "matches `⌈log2 Σ N_sub⌉`".
8. **Accessor names.** payload §3 (:286) `fgw_prefix_length` and §6 (:352) `fgw_retained_prefix_length` for the same clamp.
   render_contract uses `fgw_reduced_length` (:155) and `tm_t_dmin` (:109) beside `fgw_length_raw` (:132); only the last is in
   its unpack layer.
- **Needed:** confirm the payload doc governs all eight (and the others are conformed), or rule each; one name per accessor;
  the `ICDescriptor` field list and size with its padding; floor or ceiling for the proxy.
- **Ruling:** R-86 (decisions.md). Closed in step 7.

## RQ-38: `failed_fraction` vs "there is no failed category" *(step 7, payload)*

Nothing is chosen.
- `principia_dd_generation_root.md` §3.7 Refinement: `worst_energy_drift` is "input to the per-copy classifier that sets
  `failed_fraction`"; (:323) "**`failed_fraction > 0.10` detects estimator failure**"; "Open" (:391): with `failed_fraction` as a
  contributor, "a pixel whose copies cannot be integrated reads **indeterminate**".
- The same section, "Ensemble spread" (:246, :253): "**There is also no "failed" category.**" … "**`error_ratio` replaces any
  notion of a failure count**". `failed_fraction` is in no member table.
- **Needed:** whether `failed_fraction` is a `QuadReduction` member (with a row, type and classifier), or retired in favour of
  `error_ratio` (and the references are removed).
- **Ruling:** R-87 (decisions.md). Closed in step 7.

## RQ-39: What stops in-view refinement — the screen-floor veto and `MAX_REL_DEPTH` vs policy §0.1 *(step 7, scheduler)*

Nothing is chosen. R-15 gives the split decision to `Policy::Tolerance`; the stop rules around it still disagree.
- `principia_scheduler_contract.md` Part 4 (:94): "Refinement happens iff the policy splits **AND** no veto has fired", with the
  screen floor as the "everyday … view-relative veto" — yet the same Part says "Whether the criterion supersamples below it is
  the refinement policy's call". (:86): "`MAX_REL_DEPTH` is a *voluntary* tighter cap … that may stop refinement *before* the
  screen floor … `MAX_REL_DEPTH ≤ screen floor` always."
- `principia_dd_refinement_policy.md` §0.1 (:58): "| **in view, below screen floor** | the **criterion** — supersampling where
  unresolved | decides depth |"; (:51–52): "every in-view quad above the screen floor must split … **The in-view tree is complete
  at screen resolution**".
- `principia_memory_tiers.md` §2 (:58): sample density "is *fully determined* by the quadtree's screen-space floor" — one real
  sample per render pixel.
- **Needed:** (a) whether the screen floor is a hard veto or the criterion may supersample below it (and memory_tiers §2 with
  it); (b) whether `MAX_REL_DEPTH` may stop an in-view quad above the screen floor, against §0.1's "must split".
- **Ruling:** R-88 (decisions.md). Closed in step 7.

## RQ-40: The quality device note vs memory_tiers — sim-key knobs and the rung count *(step 7, quality)*

Nothing is chosen.
1. **Depth.** `principia_scheduler_contract.md` (:70): "**`MAX_REL_DEPTH` is not on the sim key.** … Lowering it while zoomed
   invalidates *no payload*". `principia_quality_device_note.md` (:109): "**Sample count and depth are sim-key parameters**", and
   §6: "sim-key rungs (samples, depth) trigger a re-boot". Its own struct (:14) marks `max_rel_depth` "scheduler knob".
2. **`E`.** quality_device_note (:17): "`E, // ensemble/SSAA copies per nominal sample — sim key`"; §6 (:148): "sim-key knobs
   adjust only at natural invalidation moments". `principia_memory_tiers.md` §5 (:187): "**E and the refinement floor move live
   under motion** (copies drop/respawn without invalidating the nominal". quality_device_note itself (:117) has the heuristic
   set "`e_motion_gating` to reduce E (→0/1) during an active march".
3. **Rungs.** quality_device_note §3 (:132): "an ordered ladder of **~8–12 rungs**". memory_tiers §5 (:191): "The controller
   subdivides each named tier into ~8–12 unnamed internal steps" (~48–72 in all).
- **Needed:** (1) whether depth is a sim-key knob; (2) whether `E` can change live, and if so how that squares with its sim-key
  status; (3) ~8–12 rungs in total, or per named tier.
- **Ruling:** R-89 (decisions.md). Closed in step 7.

## RQ-41: The decoder switchover trigger *(step 7, deep zoom)*

Nothing is chosen.
- `principia_deep_zoom.md` §2 (:57): "Depth `≤ ℓ_switch` (default 20) → full decoder; depth `> ℓ_switch` → linearised … The more
  robust trigger is *adaptive* … **switch when the energy-drift diagnostic sees adjacent samples collapsing to identical results.**"
- `principia_scheduler_contract.md` Part 4 (:90): "When the *full nonlinear decoder's* adjacent samples collapse to
  bitwise-identical ICs … switch to the linearised decoder".
- `principia_lowering_contract.md` Part 5 (:137): "`decodeMode: quad.depth > SWITCH ? LIN : FULL`" (a depth threshold; `SWITCH`
  is not defined there).
- **Needed:** depth threshold, adaptive trigger, or both; and, if adaptive, whether collapse is detected by the energy-drift
  diagnostic or by bitwise IC comparison.
- **Ruling:** R-90 (decisions.md). Closed in step 7.

## RQ-42: Temporal accumulators as a second split trigger, beside `Policy::Tolerance` *(step 7, refinement)*

Nothing is chosen.
- `principia_scheduler_contract.md` Part 8 (:180): "**Split fires on either of two orthogonal signals:**" — spatial coherence
  ("wide state spread", not `spread_shape > eps`) and "**Temporal accumulators** … **running max divergence** … **running mean
  divergence**, **divergence trend** … **first-divergence time**". The latch "persists across visits".
  `principia_temporal_architecture_note.md` (:75): "split if spatial_incoherence(now) > θ_s OR running_max_divergence > θ_max OR
  divergence_trend(now) > θ_trend".
- `principia_dd_refinement_policy.md` §1 and R-15: `split(quad) ⟺ any footprint f in quad is unresolved`, one knob `eps`.
- **Needed:** whether the temporal accumulators (and the latch) survive under `Policy::Tolerance` — as a split trigger, as an
  input to "unresolved", or not at all — and what `θ_s`, `θ_max`, `θ_trend` become.
- **Ruling:** R-91 (decisions.md). Closed in step 7.

## RQ-43: Navigation — "neither key", but it edits sim-key inputs *(step 7, caching)*

Nothing is chosen.
- `principia_systems_architecture.md` "The two keys as ladder geometry" (:124–130): "SIM KEY … chart id+params · z₀/basis/warps
  … ⇒ re-integrate" and "NAVIGATION   pan/slice/tilt/zoom/lock ⇒ NEITHER — re-addresses which quads are asked for".
  `principia_canonical_spec.md` §8 (:99): navigation "(re-addresses which quads are asked for; neither key)".
- `principia_canonical_spec.md` §4 (:60): "pan/slice edit `z₀`, zoom/tilt edit the basis, the lock pins the centre". R-69:
  navigation "edits `z₀` and the basis".
- **Needed:** which navigation edits re-address (quads keyed in a fixed chart frame) and which re-integrate (a new slice plane or
  tilt changes every quad's ICs), and the sim key's `z₀/basis` entry stated to match.
- **Ruling:** R-92 (decisions.md). Closed in step 7.

## RQ-44: The f32 predictability horizon — the cross-check gate and refinement *(step 7, validation)*

Nothing is chosen.
- `principia_dd_predictability_horizon.md` §4.1 (:135): "The cross-check should be gated on `t < t_max(f32)`". §1 gives ~16
  crossing times; the §7 banner (:27): "`t_f64 ≈ 52`, `t_f32 ≈ 23`"; §6 item 3 (:231): "The f32 figure (~16) is derived, not
  measured". §7.3's superseded box (:357) says §4.1's heading claim "is wrong in its reasoning", and that box is itself superseded
  by the ✅ resolution box above it.
- §4.2 (:150): a quad whose playhead exceeds its own `t_max` "should not be refined"; §6 item 2 (:228): "**Whether `t_max` should
  gate refinement**, or merely annotate it" — open.
- **Needed:** whether §4.1's gate stands (the value comes from the R-35 re-run), and whether `t_max` gates refinement or only
  annotates it.
- **Ruling:** R-93 (decisions.md). Closed in step 7.

## RQ-45: How often the engine posts the GUI snapshot *(step 7, membrane)*

Nothing is chosen.
- `principia_caching_contract.md` Part 6a (:134): the engine posts a GUI-sized snapshot "**throttled to ~10 Hz, never per-frame**".
- `principia_systems_architecture.md` §3, the membrane table (:90): "| **State snapshot** | GUI-*sized* state … | wasm → JS | per
  displayed frame; **never engine-sized**".
- **Needed:** ~10 Hz or once per displayed frame (the other file is conformed).
- **Ruling:** R-94 (decisions.md). Closed in step 7.

## RQ-46: Decoder labels and degenerate cases the corpus doesn't name *(step 7, decode)*

The values or definitions below are missing. Each item gives the file, the section and the silence.
- **The `M01_TINY` ε.** `principia_dd_decoder.md` §3.1 (:42): "if M₀₁ < ε  →  DEGENERATE(M01_TINY)"; `principia_chart_reference.md`
  §0.1 (:32): "If `M01 < ε` emit `DEGENERATE(M01_TINY)`". None of `ε_μ`, `ε_z`, `ε_q`, `ε_w` is said to be it. **Needed:** its value.
- **DEGENERATE reasons vs `decode_failed` codes.** dd_decoder §2 (:27): "a narrow, enumerated cause set", naming only `M01_TINY`;
  chart_reference §2.2: "Emit `DEGENERATE` only if all four fail" (no reason name). payload §2 (:169) has `decode_failed` detail
  codes 0 non-finite, 1 degenerate configuration, 2 invalid mass construction, 3 other. **Needed:** the full reason list and its
  map onto those codes.
- **The infeasible invariant pixel.** chart_reference §2.2 (:264): "if K* < K_min:  terminal"; `principia_chart_decoder_contract.md`
  Part 5 (:237): "infeasible pixels are *tagged labelled outputs*". **Needed:** the label (state and detail).
- **Skipped quads.** `principia_inverse_encode_contract.md` Chart-aware validation: "skip quads entirely outside the feasible
  region"; chart_reference §0.7 (:123): "No pixel is ever rejected." **Needed:** what a skipped quad's pixels carry.
- **`η_E` at `K₀ = 0`, and infeasible `E*`.** dd_decoder §3.7 (:143–145): "check feasibility $E^* \ge U$", then
  $\eta_E = \sqrt{(E^* - U)/K_0}$. The rest start sits at the momentum origin (chart_decoder_contract :25), so `K₀ = 0` there; the
  failed check has no outcome. R-25 keeps `η_E` with an off switch and says neither. **Needed:** both behaviours.
- **R-27's new `system_image` value and R-26's `ValidationResult`.** R-27: "A `system_image` value for "covers each shape twice, as
  two labelled systems" is added"; R-26 adds `validate(u, v) -> ValidationResult`. **Needed:** the value's name; the result's
  variants.
- **Encoding into the folded Burrau chart.** `principia_dd_encode.md` §3.1 (:35) inverts on "θ ∈ (0, π/2)"; R-27 keeps the folded
  chart (θ ≤ π/4). **Needed:** what encode does with θ > π/4 (relabel, fall back to latent, refuse).
- **The hypercube check's space.** inverse_encode Chart-aware validation, layer 1 (:163): "check $z_k \in [0,1]$ for every $k$",
  but `z ∈ ℝ⁸` passes through σ/tanh. **Needed:** whether the check is on `s = σ(z)` (or the clamp space).
- **Ruling:** R-71 (values) and R-72 (definitions) (decisions.md). Closed in step 7.

## RQ-47: Encode, decode and chart tolerances and defaults with no number *(step 7, decode)*

- **ε_phys.** `principia_inverse_encode_contract.md` Part 4 (:93): "‖D(z) − D(E(D(z)))‖_phys ≤ ε_phys"; also dd_encode §3.5,
  dd_decoder test 7. **Needed:** its value.
- **κ(z).** inverse_encode Part 1 (:24): "Residual bounded by the conditioning `κ(z)` (Part 4)"; Part 7 (:211): "**A conditioning
  number** `κ(z)`". Part 4 gives only `d logit/ds = 1/(s(1−s))`. **Needed:** its definition.
- **Curve-projection distance.** dd_encode §3.4 (:76): "beyond tolerance → … fall back to latent z"; inverse_encode Part 5 (:118):
  "If the distance exceeds tolerance". **Needed:** the tolerance.
- **Decode sanity.** inverse_encode Chart-aware validation, layer 3: "CoM at the origin and total momentum zero, both within
  tolerance." **Needed:** the tolerance.
- **E₀.** dd_decoder §3.6 (:139): "`E₀` here must agree with the kernel's `E_0` at t=0"; `principia_dd_generation_root.md` §3.4
  (:138): "must equal `K₀+V₀` (cross-check view)". **Needed:** the agreement tolerance.
- **dd_decoder tests.** Test 12 (:184): "f32 eps-scaled tolerance" (no scale factor); test 11 (:183): error "shrinks `∝ h²`" (no slope
  tolerance). **Needed:** both.
- **Burrau at ν = 1/2.** chart_reference §5.2: reproduces "`(3,4,5)`, and the classical configuration to a stated tolerance".
  **Needed:** the tolerance.
- **Invariant-chart and Burrau-axis defaults.** chart_reference §2.1 (:205): "choose `K_max > 0`, exponent `γ_K ≥ 1`"; §4.5
  (:457): "θ(u) = θ_min + (θ_max − θ_min)u"; `Φ_{θ,L_z}` "(fix $K$, sweep $L_z$)". **Needed:** `K_max`, `γ_K`, `θ_min`, `θ_max`,
  the fixed `K` and the `L_z` range.
- **BodyPlane.** chart_reference §5.1 (:522): "`BodyPlane` (today's slice) stays as a chart and must reproduce **bit-for-bit** —
  it is the Python cross-check's anchor." **Needed:** its map and the reference artefact it must match.
- **Validation imports.** dd_encode §4 (:92): Anosova region-D and Burrau rest starts "match the papers' stated values after the
  recorded rescale". **Needed:** the values and the tolerance.
- **The Inspector's gates.** `ic_inspector_scratchpad.md` "Status — as built" (:23) gives only measured bounds ("max `‖z−z'‖ =
  1.2×10⁻¹³`"); "Degeneracy routing" (:27, :113): "past threshold → flip chart-encode ▸ **direct-physical-inject**" and "run both
  paths and diff". `principia_gui_state_contract.md` §4 (:115): "the test asserting `‖n‖ = 1` to tolerance". **Needed:** the
  round-trip gate, the conditioning threshold, the two-path agreement tolerance and the `‖n‖ = 1` tolerance.
- **Ruling:** R-71 (values) and R-72 (definitions) (decisions.md). Closed in step 7.

## RQ-48: Escape and termination — what the corpus doesn't give *(step 7, integrator)*

- **The state after escape fires.** R-31: "Specify what `state` holds when escape has fired but the march continues." No file
  does. `principia_integrator_contract.md` Part 1 (:23) still has `done ← detect_terminal(state, params)` for every terminal.
  **Needed:** which fields freeze at the escape step and which keep advancing, and what `state` reads meanwhile.
- **Time averages past escape.** `principia_01_pitfalls.md` §2.4 (:190–191): continuing past escape "is actively wrong" for
  time-averaged fields — "`FTLE = S/T` past escape adds nothing to `S` while growing `T`, diluting the measurement". R-31 keeps
  the march going. **Needed:** how FTLE and other time averages are treated after escape.
- **Checks 2 and 3.** pitfalls §2.4 names "Independent ground truth — separation growing without bound" (:203) and "deep interior"
  (:205), with no pass threshold, horizon or fixture. **Needed:** all three for each.
- **The sampling grid for the window.** integrator_contract Part 7 (:392) and `principia_dd_integrator.md` §3.6 (:165): "`|Δn̂|` is
  taken over 0.4 time units, sampled at sync boundaries". Sync boundaries exist only for the regularised (AZ) occupant
  (integrator_contract :170); KDK/Yoshida have macro-steps, and escape is evaluated per `STEP`. **Needed:** the grid for
  unregularised occupants.
- **The re-validation fixture.** pitfalls §2.2 (:152) uses the config chart with ground truth "unbound and receding at `t = 30`".
  **Needed:** whether R-29's re-validation keeps that ground truth or uses check 2's independent one.
- **Ruling:** R-95, with R-71 and R-72 for the rest (decisions.md). Closed in step 7.

## RQ-49: Integrator values the corpus doesn't give *(step 7, integrator)*

- **Where the re-registration count lives.** `principia_integrator_contract.md` "The profile gains a field" (:186):
  "`re_registrations: u32` — or at minimum a per-trajectory count in the payload." **Needed:** profile field, payload field, or both
  (and its payload placement — a ledger change, R-36).
- **"Report which would have been better."** integrator_contract "One thing deliberately NOT decided": "expose the choice, report
  which would have been better, never switch silently." **Needed:** how "better" is judged, when, and where it is reported.
- **The `r_coll` / `N_max` coupling.** integrator_contract Part 7 (:385): "Either surface the coupling in the UI, or let very-small
  `r_coll` raise `N_max` at a cost the quality controller accounts for." **Needed:** one of the two.
- **Benettin.** `principia_dd_integrator.md` §3.8 (:234): "`x' = x₀ + δ₀` (arbitrary direction, ‖δ₀‖ small)", "every n_renorm
  steps"; neither is in integrator_contract Part 3's parameter table. **Needed:** `‖δ₀‖`, its direction and `n_renorm`.
- **Order → complexity proxy and FTLE confidence.** integrator_contract Part 2 (:93): "Order sets the meaning of the
  `total_substeps_log2` complexity proxy … and FTLE confidence". **Needed:** the mapping.
- **The `τ` tie tolerance.** `principia_dd_simstate_payload.md` §3 (:294): "fixed cut-ID priority only as a tie-break when `τ`
  values are equal within a defined tolerance … Specify the `τ`-sort, tolerance, and sign convention in the integrator
  contract." Not specified there. **Needed:** all three.
- **Ruling:** R-71 (values) and R-72 (definitions) (decisions.md). Closed in step 7.

## RQ-50: Payload and `QuadReduction` layout silences *(step 7, payload)*

- **The Welford `y`.** payload §4 (:305) and dd_integrator §3.5 (:142): "Slope of spread `y` on time `t`". **Needed:** what the
  per-sample `y` is.
- **The closure floor.** payload (:112): "The floor is precision-dependent, and this must be reported." **Needed:** where (readout,
  export, legend).
- **`class_histogram[N]`.** `principia_dd_generation_root.md` §3.7 (:190): "`class_histogram[N]` | u8 × N". `N` isn't defined, and a
  u8 bin overflows at 256 samples (a 16×16 grid, R-43's N = 16). **Needed:** `N` and the bin width.
- **`outcome_impurity`.** Ledger (:192): "`1 − max(class fraction)`", under a heading that says all fields are at joint
  `class ⊕ detail` grain. **Needed:** which fraction.
- **The struct itself.** Ledger §3.7 (:176): "Size the struct from the member list, then align, then update the figure." **Needed:**
  member order, packing of the 2-bit and 5-bit members, and the final size.
- **`roundtrip_error`.** Ledger (:208): "time-reversal round-trip displacement". **Needed:** its horizon, when it runs, its cost.
- **`error_ratio`.** Ledger (:207): "**Boolean flag only**" with no threshold for "departing from 1.0"; (:212):
  "`ensemble_spread = max(spread_shape, spread_event)  -- how error_ratio enters is OPEN`". **Needed:** the threshold and its role.
- **The gate / tolerance pair.** Ledger "Open" (:395): "gate threshold and integrator tolerance must be specified **as a pair**".
  **Needed:** the pair.
- **Measurement gates.** payload §8: word truncation rate "drives whether 76 symbols suffices" (:475); the crossing distribution
  decides whether `S_word` is additive; "switch to displacement only if FTLE accuracy meaningfully improves". **Needed:** the
  threshold each decision uses.
- **Ruling:** R-71 (values) and R-72 (definitions) (decisions.md). Closed in step 7.

## RQ-51: Scheduler, cache and quality values *(step 7, scheduler)*

- **Baseline cover.** `principia_caching_contract.md` Part 4 (:55): "the viewport-covering quads a few levels above camera depth".
  **Needed:** how many levels.
- **Debounces.** caching_contract Part 5 (:81): "At-rest (debounce fired)"; `principia_scheduler_contract.md` Part 7 (:174):
  "At-rest resumes on debounce". Only the bake's ~120 ms is given. **Needed:** the gesture debounce.
- **Budgets and ceilings.** caching_contract Part 6 (:106): "time-budgeted per frame"; Part 7: cost-weighted LRU whose "cost model now includes `t_cached`" (:13). `principia_dd_telemetry_and_tiers.md` §6.3 (:284):
  "small enough to return"; §6.4 (:324): "Keep the in-flight depth shallow"; CPU: leave at least one core. **Needed:** the per-frame
  budget, the LRU cost formula, the chunk bound, the queue depth and the core reserve.
- **The integration floor.** scheduler_contract Part 4 (:92): "a quad whose `suspect_fraction` stays high and whose samples are
  **substep-saturated**". **Needed:** "high" and "stays" (frames or levels).
- **Neighbour agreement.** `principia_dd_refinement_policy.md` §2.1 (:130): "land within **~11°** on the shape sphere". **Needed:**
  the exact angle.
- **Priority (R-44, R-55).** scheduler_contract Part 6 (:132): "Visibility dominates (never compute off-screen)" and (:134)
  "offscreen → stop", against policy §0.1's "off screen | the criterion | decides depth". R-44 orders them reconciled and "how the
  order in view enters" said; R-55 leaves `P_focus`'s "weight and decay are still to be written". **Needed:** the reconciled rule,
  and `w_f`'s decay law.
- **The frontier margin (R-45).** refinement_policy §7 (:271): "The margin must be **derived from the refill rate**". **Needed:**
  the formula, and the widened-margin baseline's size.
- **Quad-skip (R-26).** R-26 lists "scheduler_contract (a quad-skip rule)"; the contract has none. **Needed:** the rule, including
  quads partly inside the chart's domain.
- **`QualitySettings` and `eps` (R-40).** The struct (quality_device_note :12–20) has no `eps`, frame-budget or hard-cap field.
  **Needed:** the fields, and whether `eps` varies per internal rung or per named tier.
- **Frame cadence.** `principia_temporal_architecture_note.md` (:171): "`steps_per_frame = ⌈T / (60 × fps × dt_macro)⌉`".
  **Needed:** whether `fps` is measured or nominal.
- **The render key for a graph.** R-64 replaces the four slots with a node graph. `principia_lowering_contract.md` (:125)
  `fragmentKey = hash(rc.colourSrc, rc.brightnessSrc, rc.combinerSrc, rc.postSrc)`; render_contract (:61) "hash of 4 slot-source
  hashes". **Needed:** how the fragment / render key hashes a graph (canonical node order, post chain).
- **Checkerboard.** `principia_checkerboard_contract.md` §7 (:63): "per-frame playhead advance exceeds ~`0.05`". **Needed:** the
  units (the value itself is measured, §8).
- **`sea_fraction`.** refinement_policy §5.1 names the estimator (built after the tier controller, R-48). **Needed:** its accuracy
  target.
- **Telemetry size.** telemetry §5 (:186): "Either downsample on write (keep every frame during motion, every Nth while idle) or roll
  up idle stretches". **Needed:** which, and `N`.
- **Ruling:** R-71 (values) and R-72 (definitions) (decisions.md). Closed in step 7.

## RQ-52: Colour and render silences *(step 7, colour)*

- **Tolerances.** `principia_colour_composition.md` §7 (:433): "agreement to tolerance certifies the port". `principia_dd_colouring.md`
  §5 tests 1, 4 (:161, :164): "within tolerance", "within blend tolerance"; test 9 (:169): "a minimum OKLab hue separation".
  `principia_render_contract.md` cross-check views (energy agreement, Welford, invariant-chart gradient): no numbers. **Needed:**
  each tolerance and the minimum hue separation.
- **Palette classes.** colour_composition §1.4 (:150–162): "degenerate" (white) and "collision @ t=0" (orange) aren't `state` values
  (payload §2: escape, bounded, collision, running, sim_failed, decode_failed). **Needed:** how each is read (e.g. `decode_failed`;
  collision with `t_end_step == 0`), and default colours for `running`, `sim_failed` and `decode_failed`.
- **Monomorphisation.** `principia_lowering_contract.md` Part 4 (:101): "bound it to the active/plausible set, not the full
  cross-product". **Needed:** the set.
- **Symbolic spread.** `principia_sampling_msaa_note.md` (:123): "shared-prefix-length / edit-distance / distinct-word-count across
  the E+1 words". **Needed:** which metric, and its split threshold.
- **Ruling:** R-96 (definitions), with R-71 for the values (decisions.md). Closed in step 7.

## RQ-53: Image-embedding format silences *(step 7, image embedding)*

- **Header.** `principia_dd_image_embedding.md` §2 (:40): "header = magic(4) ‖ version(1) ‖ flags(1) ‖ payload_len(4) ‖ n_records(2)
  ‖ crc32(header)(4)". **Needed:** the magic, version, flag bits, byte order, bit order within a tile, the payload serialisation
  (§7 measures "596 B JSON → 382 B deflated", :131) and the tEXt keyword.
- **Hybrid redundancy.** §5: no default `k` (measurements used `k=9` and `k=25`, :90). **Needed:** the default.
- **Decoder change.** §6 (:103): "build hash — REFUSE to recreate silently across a version where the decoder changed". **Needed:**
  which hash component detects a decoder change.
- **Resolutions.** §9: "at every supported resolution". **Needed:** the list (§7 covers 64² to 1024²).
- **Ruling:** R-71 (values) and R-72 (definitions) (decisions.md). Closed in step 7.

## RQ-54: GUI silences *(step 7, GUI)*

- **Hover budget.** `principia_scratchpad_pointer_channels.md` §3 (:75): "Give the hover trajectory a STEP BUDGET." **Needed:** the
  budget, and the rest time before it lifts.
- **Sonification.** pointer_channels §4 (:97–98): "Map `1/t_c` to a fixed reference pitch." §7 leaves open "Whether `φ(t)` adds
  anything over `θ(t)` alone, or whether two channels is better as *stereo*" and "whether the sound is the whole trajectory's
  spectrum or a windowed spectrum tracking the playhead" (:147). R-68 calls this mapping the corpus's. **Needed:** the reference
  pitch and both answers.
- **The note's standing.** pointer_channels (:3): "*Working notes, not ratified.*" — yet trajectory_viewing §1, render_gui_spec §G2
  and R-55 / R-68 rely on it. **Needed:** which parts are normative.
- **The spectral-entropy anchor.** pointer_channels §2 (:36): "figure-eight (periodic) | **0.076**", "chaotic | **0.177**". **Needed:**
  the chaotic IC and a tolerance.
- **Readouts under scale-all.** `ic_inspector_scratchpad.md` (:106): "rotate or scale the whole system and none of these numbers
  move", over separations, `R`, `|pᵢ|`, `L`, `E`, `I`, which all change under scaling. **Needed:** the frame they're shown in
  (canonical R̃ = 1?).
- **Deferred Inspector features.** The scratchpad designs a right-click properties popover and discs with radius ∝ ∛mass;
  render_gui_spec §G8 lists neither, and the scratchpad says (:3) "*Not spec yet.*" **Needed:** in or out.
- **Profiler.** `principia_render_gui_spec.md` Profiler (:171): "a **leak detector** that flags steady growth while idle"; "Leak
  flags and hot-path summaries are precomputed". **Needed:** "idle", the growth threshold and window; what a hot-path summary holds.
- **Keys.** §G3 (:136): "delay, then repeat (the DAS / ARR model)". **Needed:** the delay and rate.
- **Tier 3 buffer.** §12.1 (:676): "~16 KB for a typical viewport". **Needed:** the typical visible-quad count and the per-quad record.
- **Measure.** §G10: **Needed:** a fixture of known dimension, the tolerance on `α ± error`, and defaults for the ε range, samples
  per ε and the classifier.
- **Research (v2).** §G11 (:297): "Newton-refine from each; residual and period per seed; compared with the Šuvakov–Dmitrašinović
  catalogue". **Needed:** the convergence residual and the catalogue-match tolerance.
- **§16 open items.** §16 (:756, :761): "**Node palette contents**", "**Preview** — sphere vs illustrative-slice toggle; which is
  default", and "**Standing composition-spec gaps**", which "This GUI spec assumes". **Needed:** each.
- **Undo granularity.** R-69 makes every `SimConfig` / `RenderState` edit undoable; `RenderState` holds the playhead, `SimConfig`
  "playback transport (play/pause/speed/loop)" (gui_state_contract §2, :38). **Needed:** whether playback, scrubbing and slider
  drags coalesce into one history entry.
- **Transport and the blast radius.** gui_state_contract §2 puts transport in `SimConfig (sim key)`;
  `principia_export_animation_contract.md` Part 1 (:15): "pause freezes the *playhead*, not the compute". **Needed:** transport's row
  in caching's blast-radius table (it shouldn't re-integrate).
- **Persistence.** §G2 (:65): "**all off** and **save as default**"; §G9 (:276): "Saved views: pxpack snapshots". **Needed:** where
  each is stored.
- **Link ids.** gui_state_contract §2 (:37) lists "link ids" in `SimConfig`; §G11: "Side by side with a linked cursor and
  navigation". **Needed:** what a link id holds and how linked views share navigation.
- **Ruling:** R-96 (definitions), with R-71 for the values (decisions.md). Closed in step 7.

## RQ-55: Validation fixtures, gates and tolerances *(step 7, validation)*

- **Aggregate survey.** `principia_parity_contract.md` §5 (:158): "The exact statistic and threshold — Q3 in the working note — is
  pinned in the validation phase". **Needed:** the statistic and threshold (or the milestone that pins them).
- **Ground truth.** `principia_validation_ground_truth_note.md` "Open sub-questions (settle at implementation)" (:117): the analytic
  ICs, which periodic orbits beyond the figure-eight, the Burrau feature set, the `r_coll` to pin ("Pin `r_coll` to match the
  regularisation the reference computation assumed", :121), the Path B harness API, and the exact vs structural tolerances. "Four
  tiers" (:20): "**Lehto et al.** specific numbers" — not named. **Needed:** each.
- **The independent convergence reference.** `principia_canonical_spec.md` §11 (:151): "its full *treatment* (protocol, when it runs,
  how it plugs into the harness) is the gap still to close." R-33 settles what it is. **Needed:** the protocol.
- **Burrau smoke test.** `principia_dd_integrator.md` test 12 (:278): "outcome-class + coarse `t_end` window". **Needed:** the window,
  given `principia_dd_validation_orbits.md` §1.5 (:135): "**The classical result sits past our horizon**".
- **Closure.** validation_orbits §3 (:163): "`|dr|` after one period, and **that it falls at the occupant's stated order**". §0.1:
  AZ+RK4 converges at "roughly third … Not yet RK4's fourth". **Needed:** the `|dr|` threshold, the order tolerance, and whether
  AZ+RK4 failing it is expected.
- **`xi`.** validation_orbits (:96): "the natural test bed for the reversibility measure `xi`". `xi` is defined nowhere. **Needed:**
  its definition.
- **Pitfall gates.** `principia_01_pitfalls.md` §1.6 (:100): "`stop_on_escape` on and off give near-identical images" (no tolerance,
  nor for the patchwork golden test); §3 / philosophy §4.5a (:208): convergence under refinement (no threshold); §4.1: the
  re-registration controls (2.5e-6, 7.5e-5, 4.4e-1 decades) with no acceptance level for the default occupant. **Needed:** each
  threshold.
- **Debug catalogue.** `principia_debug_tooling_plan.md` §G (:127): "`|E_0 − (K_0+V_0)|` log | ≤ tol"; §A (:31): "to Tier-N tol";
  §F (:109): "loaded / pending / refinable / **terminal** / stale … | state transitions legal". **Needed:** the tolerances and the
  legal-transition table.
- **Ruling:** R-71 (values) and R-72 (definitions) (decisions.md). Closed in step 7.

## RQ-56: Ruling follow-ups not yet folded into the docs *(step 7, cleanup)*

No choice is needed for most of these; a ruling already decides them. Line numbers are current at `6002ac2`.

**Missed by a ruling already applied** (the ruling's file list didn't name the line):
- [ ] `principia_render_contract.md:26` "The graph is **not** a graph. It is a fixed four-slot pipeline"; `:61` "render key: hash of 4
  slot-source hashes"; `:232` "One payload, four slots" — R-64 (free, typed node graph).
- [ ] `principia_render_contract.md:37` post occupants "none, CVD preview, …"; `:77` "→ CVD → **render→display scale**" — R-67
  (display scale → gamut clamp → CVD, outside the pipeline).
- [ ] `principia_lowering_contract.md:12` "// four-slot render side"; `:28` "The four-slot colour side stays exactly the render doc's
  flow"; `:125` `fragmentKey = hash(rc.colourSrc, …, rc.postSrc)` — R-64 (the graph hash itself is RQ-51).
- [ ] `principia_lowering_contract.md:65` "there is no scrub" — R-66 ("no scrub" applies to exported animations only).
- [ ] `principia_systems_architecture.md:25` "four slots"; `:64` "fragment pipeline (4 slots, fixed wrapper)"; `:126` "4 slot
  sources" — R-64.
- [ ] `principia_systems_architecture.md:65` "backdrop ▸ blur ▸ composite ▸ CVD"; `:112` "▸ CVD ▸ render→display scale ▸ SCREEN" —
  R-67.
- [ ] `principia_systems_architecture.md:145` seam 4 "`STEP(state, dt, params)`" — R-19 (`ADVANCE` is the occupant seam).
- [ ] `principia_systems_architecture.md:233` "the lock is UI state" — R-69 (lock is chart construction, in `SimConfig`).
- [ ] `principia_integrator_contract.md:32` "The occupant is `STEP(state, dt, params) → state'` and nothing else." — R-19 (:231
  "ADVANCE … # the occupant seam"; :235 KDK/Yoshida implement `ADVANCE` as their loop).
- [ ] `principia_integrator_contract.md:34` "the escape persistence counter `c_esc` therefore ticks per `STEP`" — R-29 / change 11
  (:340 "The old persistence counter … is gone"); R-59 D1 listed :318, :340, :357 but not this line.
- [ ] `principia_dd_colouring.md:15` "composite order baked base → combine → overlays → CVD → render→display scale"; `:134` "pipeline
  order **pixel function → physics overlay → CVD → render→display scale → canvas write**" — R-67. `:15` "the four slots" — R-64.
- [ ] `principia_dd_colouring.md:86` "Equirect mapping is **(φ, n_z)** ∈ [−π,π]×[−1,1]"; `principia_trajectory_viewing.md:79`
  "equirect (φ, n_z)" — R-14 (θ is the azimuth on the horizontal axis, φ the polar angle).
- [ ] `principia_dd_generation_root.md:37` "a valid IC already escaping is `escape` at step 0" (§3.1) — R-60 (no t = 0 escape).
- [ ] `principia_canonical_spec.md:155` "the Burrau leg-swap quotient and body-index naming remain open decisions" — R-22 (applied)
  and R-27 (recorded).

**Scheduled by the ruling's own status line** (listed so nothing is lost; not due yet):
- [ ] 136/88 B widths — R-40 / R-59 D6. Payload :448 already has "Recomputed at 144 / 96 B". Named in R-40:
  `principia_canonical_spec.md:79`, `principia_systems_architecture.md:63`, `:163` ("512 trajectories × 136 B = 68 KB"; the §5.5
  workgroup arithmetic rests on it). **Not named in R-40:** `principia_render_contract.md:13` ("136 B (FTLE-on) / 88 B (FTLE-off)"),
  `principia_memory_tiers.md:90` ("`bytes` = 136/88"), `principia_dd_generation_root.md:60` ("136 B effective (FTLE-on) / 88 B").
- [ ] `has_redundant_hemisphere` — R-59 D5 (with R-27): `principia_inverse_encode_contract.md:199` (named) and
  `principia_chart_reference.md:345` "The chart sets `has_redundant_hemisphere = true`." (not named).
- [ ] "scale rescale is step 0" `principia_canonical_spec.md:52`; "scale rescaled first" `principia_systems_architecture.md:149` — R-23.
- [ ] "a separate double-double/arbitrary integrator" `principia_canonical_spec.md:151` — R-33 (double-double is a fast screen only).
- [ ] The ledger's `alpha` row `principia_dd_generation_root.md:273` — R-42 / R-59 D4.

**Overridden by the ledger's own canonical clause** (`principia_dd_generation_root.md:29`: "the consolidated doc is canonical"):
- [ ] `:83` "**Truncation is bit-occupancy:**" (payload §3: the normative rule is the length cap).
- [ ] `:74`, `:79` the pop doesn't decrement `length`, and decode is "while W ≥ 4" (payload §3 decrements on pop and decodes by depth).
- [ ] `:60` "the bandwidth-bound march" (payload §0/§3: the bottleneck is unconfirmed).
- (The ledger's descriptor bits 8–15 at `:41–43` and `saturated` at `:39` are in RQ-37, items 5 and 6, since other files
  carry the same text.)

**Doc defects, no ruling:**
- [ ] `principia_INDEX.md:14` "Eight named patterns" — `principia_01_pitfalls.md` has nine pattern sections (§1–2, §4–10; §3 is
  standing rules).
- [ ] `principia_canonical_spec.md:150` "the full GUI *design* (Malachy's ideas) is unwritten" — step 6 wrote
  `principia_render_gui_spec.md`.
- **Ruling:** R-73 (decisions.md). Closed in step 7.
