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

---

*Found while applying R-70 to R-96 (step 7). Each is a place where applying a ruling literally met text the ruling
doesn't settle. The text was left as it was; nothing is chosen.*

## RQ-57: R-92 — in what frame are quads addressed? *(step 7, scheduler)*

- `principia_deep_zoom.md` §1 (:21): "This is what `(depth, tx, ty)`, the hash seed, and the quadtree key on — an *index into
  the current view*."
- R-92: "In-plane pan and zoom re-address" without re-integrating. For a cached quad to survive an in-plane pan, its address must
  be fixed in the slice plane, not in the current view.
- **Needed:** the frame `(depth, tx, ty)` is taken in (e.g. slice-plane coordinates at a fixed origin and scale), and whether
  deep_zoom §1's "index into the current view" is reworded.
- **Ruling:** R-97 (decisions.md). Closed in step 7.

## RQ-58: R-88 — does `MAX_REL_DEPTH` cap off-screen policy splits? *(step 7, scheduler)*

- R-88: "`MAX_REL_DEPTH` caps **only** that supersampling depth" (in view, below the screen floor).
- `principia_scheduler_contract.md` Part 3 (:66): `∨ policy_splits(C) ∧ ℓ < camera_depth + MAX_REL_DEPTH` — applied to every
  policy split, including quads off screen (the frontier).
- **Needed:** whether off-screen policy splits are also capped by `MAX_REL_DEPTH` (Part 3 stands) or by something else.
- **Ruling:** R-98 (decisions.md). Closed in step 7.

## RQ-59: R-91 — four points it leaves open *(step 7, refinement)*

- **(a) Granularity.** R-91: "a **footprint** is unresolved if … its latched running maximum ever did". scheduler Part 8
  (:184): the running max is "per quad, in `QuadReduction`". refinement_policy §1 (:91) now tests it per footprint. Is the
  latch per footprint (a new per-footprint member) or per quad?
- **(b) The other accumulators.** scheduler Part 8 (:184) keeps "**running mean divergence**, **first-divergence time**
  (write-once)"; temporal note :70, :80. R-91 drops θ_s, θ_max, θ_trend and the trend, and says nothing of these two. Kept
  as telemetry, or dropped?
- **(c) `S_word`.** scheduler Part 8 (:183): spatial coherence "**Includes symbolic spread** (`S_word`)". refinement_policy §1's
  `unresolved(f)` (:86–91) has no word term. Does `S_word` feed "unresolved"?
- **(d) The latch and merging.** A latched maximum keeps a footprint unresolved for ever, so its parent can never merge
  (refinement_policy §3, :156: merged "when the parent has become resolved"), and the measured result "the final tree is
  **bitwise the static tree at the horizon**" (:161) can no longer hold under a live playhead. Is that intended?
- **Ruling:** R-99 (decisions.md). Closed in step 7.

## RQ-60: R-80 — the optional per-cell Halton rotation *(step 7, sampling)*

- R-80: copies 1..E are "Halton points 1..E, centred (minus ½) and scaled to the footprint"; R-81: "the footprint fixes the
  offsets".
- `principia_sampling_msaa_note.md` :89–90 still offers an "optional decorrelation per cell (Cranley–Patterson rotation,
  deterministic)", now limited to copies 1…E; :110 hashes it.
- **Needed:** whether the rotation is dropped (offsets fully fixed) or kept as an option (and then how it composes with the
  centring).
- **Ruling:** R-100 (decisions.md). Closed in step 7.

## RQ-61: R-96 — transport in `ViewUI`, which is "never read by the engine" *(step 7, GUI)*

- R-96: "Transport (play / pause / speed / loop) moves from `SimConfig` to `ViewUI`: not undoable, not on the sim key."
- `principia_gui_state_contract.md` §2 (:43–44): the `ViewUI` block now lists "playback transport (play/pause/speed/loop …)"
  and ends "— never read by the engine". The frame loop reads play/pause and speed.
- **Needed:** whether transport is a `ViewUI` field the engine reads (the "never read by the engine" line is reworded), or a
  third category (engine-read, not undoable, not on either key).
- **Ruling:** R-101 (decisions.md). Closed in step 7.

## RQ-62: R-89 — "ensemble on/off" as a baked variant *(step 7, integrator)*

- R-89: "E changes live: copies are cached per `copy_index`, and the nominal's key excludes E."
- `principia_integrator_contract.md` Part 3, item 3 (:323): "the co-computation selections (`FTLE_ENABLED`, ensemble on/off …).
  These select **baked variants**" — a variant change re-dispatches.
- **Needed:** whether turning the ensemble on or off is still a baked variant (only E's value changes live), or the variant is
  dropped.
- **Ruling:** R-102 (decisions.md). Closed in step 7.

## RQ-63: R-95 — the loop's `done` flag after escape fires *(step 7, integrator)*

- R-95: "Any further march exists only to run the pitfall §2.4 checks and writes nothing else."
- `principia_integrator_contract.md` Part 1 (:23): `done ← detect_terminal(state, params)   # WRAPPER — per STEP; sets the flag`.
  The prose now states R-95; the pseudocode is unchanged.
- **Needed:** whether `detect_terminal` sets `done` on escape (and a separate "checks pending" state carries the march), or
  `done` is set only when the §2.4 checks pass.
- **Ruling:** R-103 (decisions.md). Closed in step 7.

## RQ-64: R-27 — the shape-sphere chart's `system_image` *(step 7, charts)*

- `principia_lowering_contract.md` chart table (:158): shape sphere (α, β) — "`system_image: 2-to-1`".
- R-27 adds "a `system_image` value for 'covers each shape twice, as two labelled systems'" and lists lowering :160–162, but
  gives the value no name. D5 (R-59) replaced `has_redundant_hemisphere` with `system_image`.
- **Needed:** the name of R-27's new value, and whether the shape sphere's `2-to-1` is that value or a different one.
- **Ruling:** R-104 (decisions.md). Closed in step 7.

## RQ-65: R-93 — which re-run gives `t_max(f32)`? *(step 7, validation)*

- R-93: "with the value from the **R-84** re-run". R-84 (branch decisions across precisions) has no re-run; R-35 is "The
  change-10 cross-checks are re-run and the NumPy reference patched", and its file list includes dd_predictability_horizon.
- `principia_dd_predictability_horizon.md` §4.1 now reads "the value of `t_max(f32)` comes from the re-run of the change-10
  cross-checks (R-35)". decisions.md records R-93 as given.
- **Needed:** confirm R-35 (or correct the doc).
- **Ruling:** R-105 (decisions.md). Closed in step 7.

## RQ-66: R-96 — which "link ids"? *(step 7, GUI)*

- R-96: "Link ids are specified when the v2 research tools are built" — the linked views of `principia_render_gui_spec.md` §G11
  (:304), where it is now applied.
- `principia_gui_state_contract.md` §2 (:37) lists "link ids" in `SimConfig`, and `principia_render_contract.md` :61 puts link ids
  on the sim key. These read as the chart's link functions (lowering's `sk.links`), not linked views.
- **Needed:** confirm the two are different things (and the contract's "link ids" means the chart links), or say otherwise.
- **Ruling:** R-106 (decisions.md). Closed in step 7.

## RQ-67: Further follow-ups the step-7 rulings leave in the text *(step 7, cleanup)*

Like RQ-56: each has a ruling behind it, and none is applied yet.
- [ ] `principia_canonical_spec.md` :174 (escape banner): "`tau` sits in a **383× gap** and is not tuned" — R-29 marks the gap "to
  re-measure"; canonical isn't in R-29's file list.
- [ ] `principia_dd_refinement_policy.md` :136, heading "2.2 TWO KNOWN DEFECTS IN `alpha_area` — both open": the body now gives
  R-42's fixes. (A heading change: requirements citing it are re-pointed in the same commit.)
- [ ] `principia_INDEX.md` "Known open items" (:166–171 and near): items ruled before step 7 still listed as open — N = 16 vs 8
  (R-43), frontier scoping (R-45), relevance arithmetic (R-46), `Decision::Undetermined` (R-47), SimState widths "pending
  redefinition" (R-40).
- [ ] `principia_colour_composition.md` §7.1: the checkerboard formula names θ = arccos n_z and φ = atan2, the reverse of R-14's
  names (internal to the pattern).
- [ ] `docs/gui/design/GUI_DESIGN_NOTES.md` :72: display order "stain → style → colour-vision simulation → screen" omits display
  scale and gamut clamp (R-67). The notes are the reviewer's source material, so they are not edited without your say.
- **Ruling:** R-107 (decisions.md). Closed in step 7.

## RQ-68: R-88 — does "must split above the screen floor" hold during a gesture? *(step 7, scheduler)*

- R-88: "Above the screen floor, in-view quads must split (policy §0.1)."
- `principia_memory_tiers.md` §5 (:181): the **refinement floor** lever — "stop subdividing coarser than pixel-size (tiles 2×
  pixels etc.) … *under motion only*; snaps back to the pixel floor at rest". `principia_caching_contract.md` Part 6 (:82): in
  motion "the only thing dispatched is the **full-canvas coarse cover**".
- **Needed:** whether R-88's must-split applies at rest only (the motion lever and the in-motion regime stand), or also in
  motion.
- **Ruling:** R-108 (decisions.md). Closed in step 7.

## RQ-69: R-95 — where the post-escape march keeps its state *(step 7, integrator)*

- R-95: "Any further march exists only to run the pitfall §2.4 checks and writes nothing else." The same words are now in
  `principia_integrator_contract.md` (:400), `principia_01_pitfalls.md` §2.4 (:202) and `principia_dd_simstate_payload.md` (:505).
- The march advances a phase state. If that state is `SimState`'s own `r`/`p` (the live-state block), the march writes it; if
  not, the corpus names no other place for it. The requirements that every stored field is bit-identical after escape fires
  depend on the answer.
- **Needed:** whether the live-state block keeps advancing after escape (and "writes nothing else" means no other field), or
  the check march runs on a copy outside `SimState` (and where that lives).
- **Ruling:** R-103 (decisions.md). Closed in step 7.

## RQ-70: R-96 — how much of pointer_channels is normative? *(step 7, GUI)*

- R-96: "pointer_channels is normative only where render_gui_spec, trajectory_viewing or a ruling cites it."
- `principia_render_gui_spec.md` :103 cites the whole file for **listen**: "sonification (`principia_scratchpad_pointer_channels.md`;
  `θ(t), φ(t)` → spectrum)". Its §1 (the three channels independently toggleable), §5 (the sonification predictions) and §6
  (the FFT's reuse) are cited by nothing more specific.
- **Needed:** whether the "listen" citation makes all of the sonification part normative, or only the mapping named there.
- **Ruling:** R-109 (decisions.md). Closed in step 7.

---

*Found while applying R-97 to R-109 (step 7), and for checkpoint B. Nothing is chosen.*

## RQ-71: R-104 — is the shape sphere a double cover or a 2-to-1 fold? *(step 7, charts)*

- R-104: `DoubleCover` "covers each shape twice, as two **labelled systems**"; the docs now name the shape sphere
  `DoubleCover` (`principia_chart_decoder_contract.md` Part 5 :234, lowering :159, chart_reference :349, inverse_encode :202).
- `principia_chart_decoder_contract.md` :28: "the canonical decode gauges the `λ̃_y → −λ̃_y` reflection … The φ hemispheres
  are **reflection-equivalent** — the chart is a 2-to-1 cover"; chart_reference :346 the same. Two pixels giving the *same*
  system is n-to-1 (:233: "a fixed finite number of pixels share each system"), not two labelled systems.
- **Needed:** whether the shape sphere is `DoubleCover` (and the two hemispheres are distinct labelled systems, so the
  reflection is not gauged for this chart), or n-to-1 (and R-104 applies only to the full-range Burrau chart).

## RQ-72: R-99 — how the per-footprint latch reaches the split decision *(step 7, refinement)*

- R-99: the latch is per footprint and lives with the resident quad. The ledger no longer lists `running_max_divergence` in
  `QuadReduction` (§3.7, its layout "defined by the task that builds it").
- `principia_systems_architecture.md` :80: `QuadReduction` is "the **sole automatic** return" GPU → CPU. The split decision
  is made on the CPU (refinement_policy §1).
- **Needed:** whether a per-footprint "latched" bit (or count) travels in `QuadReduction`, or the latch is evaluated on the
  GPU and only its verdict returns.

## RQ-73: R-99 — does the merge still reproduce the static tree? *(step 7, refinement)*

- RQ-59 (d) asked it; R-99 answers where the latch lives but not this. `principia_dd_refinement_policy.md` :161: under a live
  playhead "the final tree is **bitwise the static tree at the horizon**" — measured before the latch existed. With a
  latch, a footprint that ever exceeded `eps` stays unresolved, so its parent never merges while resident.
- **Needed:** whether the measured claim is withdrawn (and the latch's cost recorded), or a merge may drop a latch.

## RQ-74: R-109 — pointer_channels §3 supersedes a trajectory_viewing paragraph *(step 7, GUI)*

- R-109: only pointer_channels §4 is normative; "the rest of the file stays working notes".
- `principia_scratchpad_pointer_channels.md` :4–5: "Updates `principia_trajectory_viewing.md` … its §1 responsiveness
  paragraph is superseded by §3 below." `principia_trajectory_viewing.md` :23 carries "**SUPERSEDED — see
  `principia_scratchpad_pointer_channels.md` §3.**"
- **Needed:** whether trajectory_viewing §1's responsiveness paragraph stands again (the banner goes), or pointer_channels
  §3 is normative too.

## RQ-75: R-102 — the fragment side's baked `has_ensemble` *(step 7, render)*

- R-102: "The ensemble isn't a baked variant. `copy_index` is a uniform" — the compute kernel.
- `principia_render_contract.md` :46 and `principia_lowering_contract.md` :82, :90: the fragment assembler bakes `const bool
  has_ensemble`; E = 0 → `ensemble_spread` is NaN.
- **Needed:** whether the fragment side keeps a baked `has_ensemble` (so E = 0 ↔ E > 0 re-bakes the fragment, a render-key
  change), or reads it as a uniform too.

## RQ-76: The crate layout the tasks use *(step 7, checkpoint B)*

- The corpus names two crates: the engine crate and the gui crate (`principia_canonical_spec.md` §1, item 5). The tasks need
  names for the rest, to state acceptance commands.
- Proposed in `plan/WORKFLOW.md` and used by every task: `crates/kernel` (the physics source compiled twice),
  `crates/ledger` (the generation root), `crates/engine` (also holding the typed contract surfaces, "defined once, in Rust,
  in the engine crate": `principia_gui_state_contract.md` §1), `crates/render` (the fragment side),
  `crates/gui`, `crates/validation` (the harness, R-103), `crates/prin` (the CLI, `prin profile`), `xtask` (the runners:
  `cargo xtask golden | gate | bench | screenshot | plan-check`), `web/` (the browser product), `fixtures/`.
- Not named by the corpus either: the test runner for `web/` (the M8 tasks use `npm --prefix web test -- <filter>` as a
  placeholder).
- **Needed:** confirm the layout and the `web/` runner, or give the ones to use (the tasks' commands are renamed
  mechanically).

## RQ-77: Follow-ups the R-97 to R-109 pass left *(step 7, cleanup)*

Like RQ-56 and RQ-67.
- [ ] `principia_sampling_msaa_note.md` :81, heading "The sampling pattern: coordinate-seeded deterministic offsets" — no seed
  or hash remains after R-100. (A heading change: requirements citing it are re-pointed in the same commit.)
- [ ] `principia_chart_reference.md` :468 "(pending change 2, open)" and "Until one quotient is chosen" — R-27 ruled both
  charts kept; R-27's status line defers its application to "before any Burrau statistic".

## RQ-78: R-103 — where the `stop_on_escape` "off" image comes from *(step 7, validation)*

- `principia_01_pitfalls.md` §1.6 (:100): under the escape criterion "`stop_on_escape` on and off give near-identical images"
  — the control that settled the patchwork, carried as a regression requirement (REQ-EVT-014; its tolerance REQ-EVT-023).
- R-103: "In production, `done` is set when escape fires and the loop ends. The post-escape march … runs only in the
  validation harness." Production has no "off" setting any more.
- **Needed:** whether the "off" image is rendered by the validation harness (its own march, continued past escape), and the
  regression stands there; or the regression is retired.

## RQ-79: CI frequency, GPU hardware and browsers the corpus doesn't schedule *(step 7, checkpoint B, CI)*


- `principia_parity_contract.md` § "6. The harness" (:166–171) gives four frequencies: sim parity "every commit / CI";
  codegen "every commit"; aggregate survey "nightly / pre-release (heavier)"; colour / visual (Playwright + headless
  Chrome) "pre-release (out of parity scope — Q2)".
- No frequency is given for the other suites the plan runs: the numerical gates (`cargo xtask gate`), the benchmarks
  (`cargo xtask bench`), the GUI screenshots (`cargo xtask screenshot`), and the golden images other than the colour
  suite (the M1 debug views, the fragment and bake goldens).
- GPU in CI (M0-15): the GPU self-tests (REQ-GEN-004/006, REQ-PAY-011, REQ-TOOL-003), parity and the rust-gpu build need
  an adapter; parity §6 (:173) "Run the gate on more than one GPU backend once available … (Vulkan/D3D12) … the standing
  pre-Paper-2 action item"; R-58 "The non-Metal parity run gates Paper 2, not the build"; R-85 "Dawn CI is dropped".
  Nothing names the CI hardware or backend (a software adapter such as lavapipe, or a self-hosted Metal / non-Metal GPU).
- Second backend and browsers (M4-13c, M8-14): REQ-VAL-079 (M4, TASK-M4-18) verifies "on two backends" without naming the
  second (the spike used lavapipe); § "4. Tolerance — and the cross-backend reality" (:144) "one or two real browsers";
  REQ-VAL-116 (M8, TASK-M8-40) names none.
- Goldens before the browser (M0-14a): the colour/visual suite runs under Playwright + headless Chrome, but the browser
  build is M8 (R-85), and goldens are asserted from M1 (TASK-M0-06 builds `cargo xtask golden`, native wgpu). Nothing says
  which backend renders the goldens from M1 to M8, or whether they are re-baselined when the browser runner arrives.
- *Gaps:* M0-14a, M0-15, M4-13c, M8-14. *Tasks:* TASK-M4-18, TASK-M8-40.
- **Needed:** (a) the frequency of each unscheduled suite (every commit / nightly / pre-release / at the milestone gate);
  (b) the CI GPU (software adapter, self-hosted machine) and whether a second backend gates M4 or only Paper 2; (c) the
  browsers for REQ-VAL-116; (d) the golden renderer before M8 (native wgpu offscreen, then Playwright at M8, or both).
- **Ruling:** R-110 (decisions.md). Closed in step 7.

## RQ-80: Retired terms still live in the docs, and the vocabulary lint's doc scope *(step 7, vocabulary)*

- `principia_canonical_spec.md` §8 (:101) retires `TileID`/`computeTile`/`samples_per_tile`, the `M` checkpoint count
  (→ `n_renorm`), …; `principia_temporal_architecture_note.md` § "The rename" (:26): "`SimResult` → `SimState` … a
  mechanical global rename once this note is ratified".
- Still live: `principia_render_gui_spec.md` :199 "Order is fixed (R-67): SimResult → stain → style → …";
  `GUI_DESIGN_NOTES.md` :73 (§ "04 Windows") the same; REQ-COL-043's statement copies it. `principia_render_contract.md`
  :192 (§ "Field views (one per field, every struct)"): "Uniform echo — flat swatches of `quality_tier`, `M`,
  thresholds" (REQ-TOOL-011 ticks this row at M1).
- REQ-SYS-002 verify: "a grep over code and docs (excluding archive) finds none of the retired identifiers" — but the
  passages that retire them name them (canonical_spec :101, temporal note :26 and :188, parity :17 "was computeTile",
  dd_integrator :104 on the `N_sub` rule, memory_tiers §1).
- **Needed:** (a) whether `SimResult` in the display chain is renamed `SimState` (render_gui_spec :199, GUI_DESIGN_NOTES
  :73, REQ-COL-043) and `M` in the uniform echo becomes `n_renorm`; (b) the lint's doc scope — code only, or docs with an
  exclusion for the retirement passages (named, or marked in the text).
- *Gaps:* M0-2, M1-10b. *Tasks:* TASK-M0-16, TASK-M1-14, TASK-M7-21.
- **Ruling:** R-111 (decisions.md). Closed in step 7.

## RQ-81: The archived briefs' standing parts aren't citable *(step 7, plan)*

- `principia_INDEX.md` § "Archived — record only, do not implement" (:137, :142): the structure-criterion brief's
  "§4–4.6, the slippy map, still stands"; the kernel-build brief's "§5 gates still stand". REQ-SYS-008 carries both
  exceptions.
- `plan/tools/sections.py` (CORPUS_GLOBS) does not index `docs/experiments/`, so neither part is citable: no requirement
  is sourced from them, and TASK-M0-02's plan-check (test (c)) fails any task reference to an `ARCHIVE_` brief.
- **Needed:** whether the two standing parts are admitted to the citable index (and their obligations extracted into
  requirements — slippy map: breadth-first, frame budget; kernel build §5: verification gates), or are taken as
  superseded by the consolidated docs (deep_zoom §3, scheduler, parity), with the INDEX rows and REQ-SYS-008 conformed.
- *Gaps:* M0-3. *Tasks:* TASK-M0-02.
- **Ruling:** R-112 (decisions.md). Closed in step 7.

## RQ-82: The generated debug NaN guard vs the bitcast rule *(step 7, render)*

- `principia_render_gui_spec.md` §10.1 (:603–604): each numeric debug field generates `if (raw != raw) { return
  DEBUG_NAN; }` then the ramp (REQ-RENDER-022's two-line template).
- `principia_render_contract.md` Part 2 (:45, :47): fast-math may assume no-NaN, so no correctness logic may rely on NaN
  or `isnan()`; the absence test "is an exact **bitcast comparison** against the canonical quiet-NaN bits — reliable where
  `isnan()` is not". Lowering Part 3a (:90) the same; render_contract Part 4 (:81) "`isNan` is unreliable under fast-math".
  `raw != raw` is the same self-comparison `isnan` makes (REQ-RENDER-015: no correctness logic on NaN).
- **Needed:** whether the generated guard becomes the bitcast test (any NaN pattern, or the canonical one only — a stored
  value is never NaN, R-79), or `raw != raw` stands as best-effort debug garnish (render_contract :81 "bitcast pattern
  tests in debug views are best-effort garnish").
- *Gaps:* M1-6. *Tasks:* TASK-M1-09, TASK-M1-15.
- **Ruling:** R-114 (decisions.md). Closed in step 7.

## RQ-83: The raw `state` debug view's palette: six states or §1.4's nine classes? *(step 7, debug views)*

- `principia_debug_tooling_plan.md` §B (:43): the `state` field view is a "categorical palette, **6 states**
  (escape/bounded/collision/running/sim_failed/decode_failed)" with the failure/lifecycle states rendered distinctly
  (REQ-TOOL-021, M1).
- R-77 (:576–577): "The state palette is `colour_composition` §1.4's nine canonical classes. The Okabe–Ito / golden-angle
  rule stays for other categorical fields." §1.4's classes are at `state ⊕ detail` grain (escape by body, collision by
  pair), `running` is neutral grey and `sim_failed` the invalid colour (:173).
- A raw 3-bit `state` view has no body or pair, so it can't show §1.4's escape and collision colours.
- **Needed:** whether the raw `state` field view uses §1.4's palette (and which colour stands for escape and collision
  without `detail`), or R-77 governs the outcome palette only and the raw debug view keeps a six-colour categorical palette
  (dbg_cat).
- *Gaps:* M1-11. *Tasks:* TASK-M1-10.
- **Ruling:** R-115 (decisions.md). Closed in step 7.

## RQ-84: One decode source vs "the two decode ports" *(step 7, decoder)*

- `principia_dd_decoder.md` §1 (:9): the decoder "runs from **one Rust source** in three roles" (f32 kernel, f64 CPU path, encode) — "not three transcriptions of one definition but one source monomorphised/instantiated three ways, so decode-logic drift between them is *structurally impossible*." REQ-SYS-015: "there must be no separate transcriptions of the decode logic." No fragment (WGSL) role is named.
- `principia_colour_composition.md` §6 (:384–393): "With `ctx.chart.z` present and the decode/encode ported to WGSL … Agreement presets … WGSL-decode vs Rust-decode … a **live cross-implementation check** between the two decode ports — the project's two-references discipline"; §3 (:238) "the decode/encode are portable WGSL"; `principia_debug_tooling_plan.md` :158 "once the WGSL decode port lands".
- The one-source generator emits WGSL only for layouts, accessors and the debug catalogue (`principia_dd_generation_root.md` :9; `principia_render_contract.md` :87); lowering Part 2 (:26) compiles Φ/decode to SPIR-V and the CPU only.
- **Needed:** whether the fragment WGSL decode/encode is generated from the one Rust source (a generator target, or a translation of the rust-gpu output), so REQ-SYS-015 stands and the agreement presets check translation rather than transcription; or a hand-written second port, kept deliberately as a second reference (REQ-SYS-015 and dd_decoder §1 then name it as the exception).
- *Gaps:* M2-G1. *Tasks:* TASK-M2-15, TASK-M2-25, TASK-M2-26.
- **Ruling:** R-116 (decisions.md). Closed in step 7.

## RQ-85: The shape sphere in the lowering appendix: (α, β) or (θ, φ)? *(step 7, charts)*

- `principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set" (:161): "**Shape sphere (α, β)** | derived-in-block × derived-in-block (config) | block inverse-free direct: (s,t)→(α,β) ranges | … `system_image: DoubleCover` (2-to-1 over the φ hemispheres, R-104)".
- `principia_chart_reference.md` § "3.3 The chart map" (:325–329), by R-14: "θ = 2π·s — azimuth in the (u, v) plane … φ = π·(1 − t) — polar angle from +w … n = (sin φ cos θ, sin φ sin θ, cos φ)".
- These are different maps. By §0.2 (:49–50) `‖ρ̃‖ = cos α`, `λ̃ = sin α (cos β, sin β)`, so `n = (cos 2α, sin 2α cos β, sin 2α sin β)`: α is a polar angle from +u, not from +w, and `β ∈ [0, π]` keeps `w ≥ 0` (§3.3 :347 "the canonical decode's β ∈ [0, π] keeps w ≥ 0") — one hemisphere, while (θ, φ) covers both (the `DoubleCover` the same row declares).
- R-14 calls §3.1/§3.3's convention "the project's one shape-sphere convention"; the corpus leans to conforming the appendix row.
- **Needed:** whether the appendix row becomes "(θ, φ) … (s,t)→(θ,φ) → n → (ρ̃, λ̃) by §3.2" (conformed to chart_reference §3.3), or the shape sphere lowers as (α, β) ranges (and §3.3 and REQ-CHART-019 change).
- *Gaps:* M2-G3b. *Tasks:* TASK-M2-08, TASK-M2-14, TASK-M2-25, TASK-M4-06.
- **Ruling:** R-117 (decisions.md). Closed in step 7.

## RQ-86: The Chart trait's f64 `map` vs Φ generic over the float type *(step 7, charts)*

- `principia_chart_reference.md` § "5.1 One trait, one dispatch" (:509): `fn map(&self, u: f64, v: f64) -> ChartOut;`.
- `principia_lowering_contract.md` Part 2 (:26): "The chart map Φ, decode, canonicalise, wrapper, and occupant … are one Rust kernel **generic over the float type** and over chart/occupant … compiled by rust-gpu to SPIR-V and — the *same source* — to the CPU-f64 reference."
- An f64-only `map` can't be the Φ the f32 kernel monomorphises. The corpus leans to lowering (the later consolidated contract, R-70's rule); chart_reference §5.1 would then read `map<F: Float>(&self, u: F, v: F)`, with `validate(u, v)` staying CPU-side f64 (inverse_encode "Chart-aware validation").
- **Needed:** confirm that chart_reference §5.1 is conformed to lowering Part 2 (Φ generic over the float type), or say where the f64 trait sits.
- *Gaps:* M2-G13. *Tasks:* TASK-M2-05, TASK-M2-06.
- **Ruling:** R-118 (decisions.md). Closed in step 7.

## RQ-87: Which measurement gives `t_max(f32)`? *(step 7, validation)*

- `principia_dd_predictability_horizon.md` § "4.1 The two kernels have different horizons" (:136–137): "The gate stands
  (R-93); the value of `t_max(f32)` comes from the re-run of the change-10 cross-checks (R-35, confirmed by R-105)".
  R-93 (decisions.md :681): "the cross-check runs only for t < t_max(f32), with the value from R-35's change-10
  re-run". REQ-VAL-070 (M4, TASK-M4-16) reads "the recorded re-run value".
- R-35's re-run (REQ-VAL-036, M3, TASK-M3-36) is of the change-10 cross-checks with "the NumPy reference patched"
  (files: dd_validation_orbits §0.1, §5; `workbench/tb_az.py`) — f64 CPU runs. The same doc's § "6. Open" (:232–234):
  "The f32 figure (~16) is derived, not measured — all experiments here were f64. It should be confirmed against the
  GPU kernel directly". REQ-VAL-071 (M4, TASK-M4-16) measures it against the GPU kernel.
- Two values can result, and nothing says whether the M3 re-run has an f32 or GPU leg.
- **Needed:** which value REQ-VAL-070's gate reads — the change-10 re-run's (then say how an f64 re-run yields an f32
  horizon), or REQ-VAL-071's GPU measurement (then R-93's "from R-35's re-run" and §4.1 are conformed).
- *Gaps:* M4-5. *Tasks:* TASK-M3-36, TASK-M4-16.
- **Ruling:** R-119 (decisions.md). Closed in step 7.

## RQ-88: Eviction order: deepest first, or cost-weighted resistance? *(step 7, scheduler)*

- `principia_dd_telemetry_and_tiers.md` § "Eviction order, and the trap in it" (:253): "Drop the **deepest** cached quads
  first — cheapest to lose, easiest to recompute." The same in § "6.2a Budgeting before allocating" (:274) and in the
  pressure states of § "Three states, not two" (:245): "reclaiming — over it -- evict, deepest quads first".
  Carried as REQ-PERF-022.
- `principia_scheduler_contract.md` § "Part 6 — The settled policy" (:138): "Cost-weighted LRU: eviction resistance ∝
  `computeCostMs`. Expensive (deep, close-encounter, high-substep) quads resist eviction; high-coherence smooth quads are
  cheap to recompute and evicted first." `principia_caching_contract.md` § "Part 7 — The current-state cache (resume
  points, hard-capped)" (:151) keeps cost-weighted LRU and adds `t_cached`. Carried as REQ-SCHED-032 and REQ-SCHED-079.
- All three are closed by TASK-M5-09; REQ-PERF-022 and REQ-SCHED-032 cannot both hold for a deep, expensive quad.
  R-70's named pairs do not include this one.
- **Needed:** which order governs reclaiming (deepest first, or lowest cost-weighted resistance first), and whether the
  other text is conformed; the pinned classes (coarse ancestors, baseline cover, backdrop leaf cover) are common to both.
- *Gaps:* M5-1. *Tasks:* TASK-M5-09, TASK-M5-11.
- **Ruling:** R-120 (decisions.md). Closed in step 7.

## RQ-89: Is the physics overlay baked? *(step 7, render)*

- `principia_render_contract.md` Part 3 (:62): the baked-texture tier holds "colour occupants that are pure `f(n̂)` (vMF, LUTs, patterns, **physics blobs**)", bake key "colour-node source + its uniforms"; Part 4 (:79): "baked base (physics blobs already in) → combine (L-override) → …".
- `principia_colour_composition.md` §2 (:197–202): physics generators are "functions of the **decoded IC** (the mass point), evaluated per pixel from `ctx.payload` masses; **not bakeable** … there is no per-slice constant to bake even in principle"; the hoist optimisation (:204–206) moves them to uniforms only when no axis or tilt touches a mass dimension. REQ-RENDER-070 (M7): the bake texture "must be chart- and IC-independent".
- R-70 names colour_composition over dd_colouring, not over render_contract.
- **Needed:** whether the physics overlay leaves the bake tier (render_contract Part 3 and Part 4 conformed: a post occupant evaluated per fragment), or is baked when hoisted (masses constant across the slice, a bake keyed on the mass point).
- *Gaps:* M7-2. *Tasks:* TASK-M7-10, TASK-M7-16.
- **Ruling:** R-121 (decisions.md). Closed in step 7.

## RQ-90: The blob-blend weight and blend: markdown vs reference artefacts *(step 7, colour)*

- `principia_dd_colouring.md` §3.4 (:95–96): `wⱼ = s · 4 · max(0, exp(κⱼ(n̂·p̂ⱼ − 1)) + 0.01)`. exp(·) > 0, so the max never clips and every blob adds a weight of at least 0.04·s at every point of the sphere; with 8 sites and s = 1 that is a 0.32 pull towards the site colours everywhere.
- The reference HTML differs: `principia_gui_mock.html` :299 has `exp(k(n·p − 1) + 0.01)` (the constant inside the exponent); `principia_colour_explorer.html` :189 has `max(0, exp(…) + 0.005)·s·4` and blends sequentially with `mix(c, colᵢ, min(1, w))`, not the additive sum. colour_composition §7 makes the reference artefacts the golden oracle; R-1 makes the markdown the authority.
- dd_colouring test 5 (:168): "blob maxima exactly at b̂/ê/l̂; strength s = 0 is the identity" — holds for all three forms.
- **Needed:** the weight as intended (as written; `− 0.01`, which makes the max clip; or the constant inside the exponent) and the blend (additive sum or sequential clamped mix), so the golden image and the formula agree.
- *Gaps:* M7-6. *Tasks:* TASK-M7-10.
- **Ruling:** R-122 (decisions.md). Closed in step 7.

## RQ-91: Achromatopsia: specified, but not offered in the Display window *(step 7, colour)*

- `principia_dd_colouring.md` §3.8 (:140–143) specifies achromatopsia (`M_achrom`, luma rows) and test 10 (:173) asserts "achrom output has `R = G = B` exactly"; the checkpoint-A reviewer accepted "the achromatopsia matrix stays" (decisions.md :712).
- `principia_render_gui_spec.md` § "Display — the last stages" (:204): "**Colour-vision simulation:** off, deuteranopia, protanopia, tritanopia." — no achromatopsia. REQ-COL-045 carries the four; REQ-COL-042 carries M_achrom.
- **Needed:** whether achromatopsia is offered in the Display window (render_gui_spec and REQ-COL-045 gain a fifth mode), or kept as a test-only stage.
- *Gaps:* M7-10b. *Tasks:* TASK-M7-20.
- **Ruling:** R-123 (decisions.md). Closed in step 7.

## RQ-92: Rulings not yet applied to some passages *(step 7, cleanup)*

Like RQ-56, RQ-67 and RQ-77: each has a ruling behind it, and none is applied yet.
- [ ] R-25: "Energy normalisation stays, gated by `forbids_energy_normalisation`. 'Off' is an explicit `Option`/flag, never `E* = 0`, which is a real physical target" (status "applied during the build"). Still reading "non-zero E*" (so E* = 0 would pass as "off"): `principia_dd_decoder.md` §3.7 (:156–157) "refuses any view that combines such a chart with a non-zero $E^*$ override"; `principia_chart_reference.md` §0.6 (:118–119) "refuses a chart with the flag set combined with a non-zero `E*`" and §5.2 (:545) "a config combining `(Lz,E)` with `E* ≠ 0` is **refused**"; `principia_chart_decoder_contract.md` Part 5 (:239) "a non-zero `E*` override"; `principia_inverse_encode_contract.md` flag table (:201) "a non-zero $E^*$ override is refused". Conform to: the refusal covers every `Some(E*)`, including `Some(0)`.
- [ ] R-50: "The shape-sphere collision landmarks are mass-weighted" (status "applied before the physics-overlay occupant"; files dd_integrator §3.7, test 9; dd_colouring :90; colour_composition :197–209, :481; trajectory_viewing :78, :84; `principia_gui_mock.html` :784, :937; `principia_colour_presets.html` :131–139). Still open in the text: `principia_dd_integrator.md` §3.7 (:228) "Whether the overlay marks them at their mass-weighted positions or at fixed 120° spacing is audit decision B18, still open."; `principia_dd_colouring.md` §3.4 (:92) "With unequal masses, whether the overlay uses the mass-weighted positions or fixed 120° spacing is audit decision B18."; `principia_colour_composition.md` §7 (:488) "landmark positions per R-14 and decision B18". Conform to R-50. TASK-M2-08 builds the landmarks (REQ-INT-003) and TASK-M7-09 the overlay.
- [ ] `principia_systems_architecture.md` :161 heading "5.5 THE DISPATCH SHAPE — one thread per texel, ensemble copies
  serial" and :182 ("each thread loops over its E+1 ensemble copies SERIALLY, folding as it goes"), with :185–187
  ("The ensemble becomes a time cost (8 copies takes 8× as long)") — R-102: "`copy_index` is a uniform, and each copy is
  the same kernel dispatched again (R-89)". REQ-PERF-012 (M4, TASK-M4-05) copies the old text and contradicts
  REQ-INT-065 / REQ-INT-069 (TASK-M4-06); it is conformed in the same commit. (A heading change: requirements citing
  it are re-pointed.) Blocks TASK-M4-05 and TASK-M4-06.
- *Gaps:* M2-G18, M2-G25, M7-R50, M4-1. *Tasks:* TASK-M2-04, TASK-M2-08, TASK-M2-12, TASK-M4-05, TASK-M4-06, TASK-M7-09.
- **Ruling:** R-124 (decisions.md). Closed in step 7.

## RQ-93: M0 requirements that need things M0 doesn't have *(step 7, plan)*

**The screenshot runner.** (M0-1)

- `plan/MILESTONES.md` M0 (:58–59): "the golden-image runner, the numerical-gate runner and the benchmark runner. Each
  requirement's `verify.method` has a runner here, before the first requirement of that kind exists." `GUI screenshot`
  is a verify method (`plan/requirements.yaml` header) with no runner in the list and no M0 task building one.
- The first `GUI screenshot` requirement is REQ-TOOL-010 (M1; TASK-M1-12 runs `cargo xtask screenshot debug-views`).
- **Proposed fix:** add the screenshot runner to TASK-M0-06's deliverables (beside the golden-image runner) and to the
  MILESTONES M0 list; no new requirement (the runners carry none today).
- **Needed:** confirm, or place the runner in M1 with TASK-M1-12.

**`deep_zoom_03` and the first frame loop.** (M0-4)

- render_gui_spec § "Profiler" (:183): `prin profile --scenario deep_zoom_03 --frames 600` — "a fixed scenario,
  headless"; nothing in the corpus says what `deep_zoom_03` runs. R-56 (:436): "the measurement struct lands with the
  first frame loop"; M0 has no physics and no frame loop.
- REQ-TOOL-006 (M0, TASK-M0-18) verify runs `deep_zoom_03` for 600 frames; REQ-TOOL-008 (M0, TASK-M0-17) carries R-56's
  "lands with the first frame loop".
- **Proposed fix:** split REQ-TOOL-006 — M0: `prin profile --scenario NAME` runs a registered deterministic scenario
  headless (a synthetic scenario that emits frame records) and writes v1 JSON; M5 (the quadtree and deep zoom exist): the
  `deep_zoom_03` scenario is defined (R-72, render_gui_spec § "Profiler") and runs for 600 frames with a stable scope and
  event sequence. Move REQ-TOOL-008's "lands with the first frame loop" clause to the milestone of the first frame loop
  (M1's fragment pipeline).
- **Needed:** confirm, or move REQ-TOOL-006 whole.

**The profiler file's readers and config.** (M0-6)

- REQ-TOOL-002 (M0, TASK-M0-18) verify: "the dev GUI profiler and prin profile both read it"; telemetry §5 (:180) "It must
  carry the build hash and the full config". The Profiler window is REQ-TOOL-098 (M8); `SimConfig + RenderState`
  serialising as the provenance object is REQ-GUI-039 (M8).
- **Proposed fix:** split REQ-TOOL-002's verify — M0: the file parses against schema v1, the header holds the build hash
  and the config as the M0 contract skeleton (TASK-M0-16) serialises it, and `prin profile` reads it; M8: the dev GUI
  profiler reads the same file and the header's config is REQ-GUI-039's provenance object (added to REQ-TOOL-098's verify).
- **Needed:** confirm the split, or move REQ-TOOL-002 to M8.

**QuadReduction's size.** (M0-9)

- `principia_dd_generation_root.md` §3.7 (:182): "Size the struct from the member list, then align"; the list has
  `class_histogram[N]` (u8 × N, N not given), `dominant_outcome` (5 bits, packed) and `spread_winner` (2 bits).
- REQ-PAY-001 and REQ-PAY-006 (M0, TASK-M0-11) record the size from the member list; the member order, packing and aligned
  size (REQ-PAY-077) and N and the bin width (REQ-PAY-075) are definitions placed in M5 (TASK-M5-01).
- **Proposed fix (one of):** (a) move REQ-PAY-075 and REQ-PAY-077 to M0, closed by TASK-M0-11; or (b) split REQ-PAY-001 —
  M0: ICDescriptor 64 B and the descriptor bits; M5 (new requirement, closed by TASK-M5-01): QuadReduction sized from its
  member list and aligned — and move REQ-PAY-006 to M5.
- **Needed:** which.

**The caching signature.** (M0-12)

- REQ-GEN-008 (M0, TASK-M0-12) verify ends "caching signature carries it"; the compatibility signature is REQ-GEN-017
  (M5, TASK-M5-06), which already asserts the schema version is part of it.
- **Proposed fix:** drop "caching signature carries it" from REQ-GEN-008's verify (REQ-GEN-017 holds it at M5).
- **Needed:** confirm.

**REQ-VAL-135's evidence.** (M0-16)

- REQ-VAL-135 (M0, TASK-M0-05) is a calibration: the convergence-under-refinement threshold. The corpus's only evidence is
  the failing sequence 0.0947 → 0.2153 → 0.4423 → 0.5494 (philosophy §4.5a; pitfalls §3). A threshold also has to pass a
  quantity that does converge, and M0 runs no physics.
- **Proposed fix (one of):** keep it at M0, with the proposal evidenced by the recorded failing sequence and a synthetic
  converging series, and re-checked on the first real survey; or move REQ-VAL-135 to M3, where the march gives a real
  converging aggregate (the runner and the gate stay in M0 with a placeholder threshold).
- **Needed:** which.
- *Gaps:* M0-1, M0-4, M0-6, M0-9, M0-12, M0-16. *Tasks:* TASK-M0-05, TASK-M0-06, TASK-M0-11, TASK-M0-12, TASK-M0-17, TASK-M0-18, TASK-M1-12, TASK-M5-01, TASK-M5-06, TASK-M8-01, TASK-M8-28.
- **Ruling:** R-113 (decisions.md). Closed in step 7.

## RQ-94: M1 requirements that need M2, M3, M5 or M8 *(step 7, plan)*

- REQ-INT-001 (M1, TASK-M1-11) verify is dd_integrator test 8 "on a circulating bounded orbit" — a real march (M3). M1 can
  test the accumulator on a synthetic `n(t)` path only.
- REQ-TOOL-010 (M1, TASK-M1-12) verify: "the |n|−1 view is flat zero on a real march" (M3); the live current-substep
  heatmap reads the live march (render_contract :186).
- REQ-TOOL-011 (M1, TASK-M1-14) ticks every row of render_contract § "Field views": the DECODE preset (:181) needs the M2
  WGSL decode (REQ-RENDER-027, REQ-TOOL-029); the ensemble views (:188) are "derived at resolve from the footprint's E+1
  samples" (M5: REQ-RENDER-051, REQ-INT-072); the live effort heatmap (:186) needs M3.
- REQ-RENDER-022 (M1, TASK-M1-09): RANGE_AUTO "editable identically in the node inspector, on the graph node and in the
  code" — the node inspector is M8 (REQ-GUI-136/137).
- **Proposed fix:** split each — REQ-INT-001: M1 keeps the accumulator on a synthetic path; a new M3 requirement holds dd
  test 8 on a real orbit. REQ-TOOL-010: M1 renders every view on a synthetic payload; the real-march |n|−1 check and the
  live effort heatmap go to a new M3 requirement. REQ-TOOL-011: M1 ticks the rows a synthetic payload can show; the DECODE
  row is held by REQ-RENDER-027 (M2), the ensemble rows move to a new M5 requirement, the live effort row to the M3 one.
  REQ-RENDER-022: M1 keeps the template and RANGE_AUTO as node parameter ↔ code; the node-inspector leg goes into
  REQ-GUI-136's verify (M8).
- **Needed:** confirm the splits, or move the four requirements whole (INT-001 → M3, TOOL-010 → M3, TOOL-011 → M5,
  RENDER-022 → M8).
- *Gaps:* M1-8, M1-9, M1-10a, M1-13, M2-Plan. *Tasks:* TASK-M1-09, TASK-M1-11, TASK-M1-12, TASK-M1-14, TASK-M2-25.
- **Ruling:** R-113 (decisions.md). Closed in step 7.

## RQ-95: M2 requirements that need M3, M4, M5 or an artboard *(step 7, plan)*

**The appendix charts built in M4.** (M2-G3a)

- REQ-RENDER-027 (M2, TASK-M2-25): "every chart in the lowering appendix must decode through [the DECODE preset] without special-casing". REQ-CHART-014 (M2, TASK-M2-14) verify: "the int (m,n) lattice is bijective".
- `principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set" (:167–168) includes the **Burrau int lattice** (per-cell dispatch) and the **Anosova physical-frame** chart. Both are built only by REQ-CHART-037 (M4, TASK-M4-06); no M2 requirement builds them.
- **Proposed fix:** split REQ-RENDER-027 — M2: "every chart of the lowering appendix built by M2 (latent affine slice, shape sphere, (L_z, E), (L_z, K), ternary mass, Euclid ν plane, θ × K strip) must decode through the DECODE preset without special-casing"; new M4 requirement (closed by TASK-M4-06 beside REQ-CHART-037): "the Burrau int lattice and the Anosova physical-frame chart must decode through the DECODE preset without special-casing". Move REQ-CHART-014's lattice clause ("the int (m,n) lattice is bijective") into REQ-CHART-037's verify.
- **Needed:** confirm the split, or move REQ-RENDER-027 and REQ-CHART-014 whole to M4.

**The shape-sphere controls with no artboard.** (M2-G5)

- REQ-CHART-002 (M2, TASK-M2-28), verify GUI screenshot: "shape-sphere chart shows either one labelled hemisphere or both with a redundancy flag". REQ-RENDER-026 (M2, TASK-M2-28), verify GUI screenshot: "both projections selectable on the shape-sphere chart".
- `principia_chart_reference.md` § "3.3 The chart map": "Draw one hemisphere and say so, or draw both and flag the redundancy"; "offer an equal-area alternative". Neither `principia_render_gui_spec.md` nor any artboard in `docs/gui/design/` shows the hemisphere label or a projection control; the dev GUI is M8 (REQ-TOOL-098).
- **Proposed fix:** at M2 verify both by golden image (the render carries the hemisphere label or redundancy flag; one golden per projection) plus the descriptor unit test; add an M8 GUI requirement for the projection selector and hemisphere toggle, whose placement the human decides (no artboard).
- **Needed:** confirm, or name where the control lives.

**The link swap and the sim key.** (M2-G23)

- REQ-CHART-033 (M2, TASK-M2-21): "link ids must be recorded in provenance, and a link swap must swap both directions, recompile and re-integrate"; verify "swapping a link changes provenance and the payload signature".
- The payload compatibility signature carrying link ids (`principia_caching_contract.md` § "Part 1 — Two-level keying: identity vs validity" :12) is the sim key: REQ-SCHED-007 (M4, TASK-M4-08), REQ-SCHED-048 and REQ-GEN-017 (M5). There is no integrator at M2 (M3) to re-integrate.
- **Proposed fix:** split REQ-CHART-033 — M2: "decode and encode must consume only registry links and inverses; link ids must be recorded in provenance; a link swap must swap both directions"; M4 (closed by TASK-M4-08 beside REQ-SCHED-007): "a link swap must change the sim key, recompile and re-integrate from t = 0".
- **Needed:** confirm the split.

**The t = 0 collision label.** (M2-G24)

- REQ-ENC-019 (M2, TASK-M2-19): "bodies within r_coll give a t = 0 collision outcome"; verify "bodies within r_coll → t = 0 collision; exactly coincident → lookup_clamped; no separate rejection branch".
- `principia_integrator_contract.md` (:357): "at dispatch, before the first step, evaluate the terminal detectors on the decoded IC. A valid IC already inside `r_coll` → `state=collision, t_end_step=0`" — REQ-EVT-002 (M3, TASK-M3-09). The label is written by the integrator dispatch, which M2 doesn't have.
- **Proposed fix:** split REQ-ENC-019 — M2: "lookup must have no coincident-bodies rejection branch; exactly coincident bodies are caught by the range check as `lookup_clamped`"; M3 (closed by TASK-M3-09 beside REQ-EVT-002): "a looked-up IC with bodies within r_coll must reach dispatch and be labelled `collision`, `t_end_step = 0`".
- **Needed:** confirm the split.
- *Gaps:* M2-G3a, M2-G5, M2-G23, M2-G24. *Tasks:* TASK-M2-14, TASK-M2-19, TASK-M2-21, TASK-M2-25, TASK-M2-28, TASK-M3-09, TASK-M4-06, TASK-M4-08.
- **Ruling:** R-113 (decisions.md). Closed in step 7.

## RQ-96: The branch-cut convention is needed in M3, required in M6, and has no author *(step 7, integrator)*

- `principia_symbolic_dynamics_contract.md` § "1. Generator ↔ branch-cut convention" (:12–14): "Fix, normatively: which
  two branch cuts correspond to generators `a` and `b` … the crossing-direction sign convention (which crossing
  direction is the generator vs its inverse)". The contract's status (:7) is "OPEN — specification required before
  per-pair quantities are trusted".
- REQ-PAY-070 (M6, TASK-M6-13) requires §1 "before any per-pair view ships"; REQ-PAY-071 and REQ-PAY-072 require §2
  (the punctured-sphere relation) and §3 (the attribution algorithm). None is `kind: definition`, so no requirement
  says whether a task writes them (R-72, physics reviewer) or the human supplies them.
- But M3 already writes the word: REQ-INT-031 and REQ-INT-046…048 (TASK-M3-16) append `a/A/b/B` per crossing, and
  REQ-VAL-043 (TASK-M3-30) asserts "the encoder reproduces the published braid class" of Šuvakov–Dmitrašinović orbits —
  impossible without §1's a/b assignment and direction sign.
- Proposed fix: move REQ-PAY-070 to M3, closed by TASK-M3-16 (REQ-VAL-043 then checks it against the published
  classes); REQ-PAY-071 and REQ-PAY-072 stay in M6 (per-pair views, R-38).
- **Needed:** whether REQ-PAY-070 moves to M3; and whether §1–§3 are written by the tasks under R-72 (making
  REQ-PAY-070…072 `kind: definition`, physics reviewer) or supplied by the human (§2–§3 are topology, which R-72 must
  not be used to invent).
- *Gaps:* M3-7, M6-4. *Tasks:* TASK-M3-16, TASK-M3-30, TASK-M6-13.
- **Ruling:** R-113 and R-125 (decisions.md). Closed in step 7.

## RQ-97: GPU and browser legs before the GPU kernel or the browser exists *(step 7, parity)*

- M3 is "CPU, native" (MILESTONES M3); the shared kernel is first compiled to f32 SPIR-V / WGSL in M4 (TASK-M4-01).
  The M0 GPU self-test dispatch (REQ-GEN-007) runs the codegen unpack, not the kernel. Yet:
  - REQ-INT-028 (M3, TASK-M3-03) verify: "`N_sub` bit-identical on CPU-f64, CPU-f32, native GPU and
    browser-GPU-via-WGSL" (from dd_integrator § "5. Unit tests" test 4, :276).
  - REQ-INT-029 (TASK-M3-17) "CPU and GPU take the same capped step"; REQ-INT-030 (TASK-M3-05) "CPU-f32 and GPU-f32
    matches bit-for-bit on Metal"; REQ-INT-031 (TASK-M3-16) "word identical CPU/GPU"; REQ-INT-007 (TASK-M3-04)
    "branch-exact GPU output".
- M4 has native wgpu only; the browser is M8 (R-85: "Real browsers are checked against those tolerances with the
  browser build (M8)"; `principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality", :144). Yet:
  - REQ-VAL-059 (M4, TASK-M4-03) verify: "identical across CPU-f64, CPU-f32, native GPU, browser GPU".
  - REQ-INT-059 (M4, TASK-M4-09) verify: "SPIR-V->MSL and SPIR-V->WGSL->Tint" (`principia_gpu_determinism_note.md`
    § "The discipline (each rule = one measured failure)", :50). Tint is Dawn's compiler; Dawn CI is dropped (R-85),
    so the WGSL→Tint leg exists only in a browser.
- Proposed fix:
  - REQ-INT-028: M3 half = CPU-f64 and CPU-f32 identical; the native-GPU leg is REQ-VAL-059's `N_sub` (M4); the
    browser leg joins the M8 browser check (below).
  - REQ-INT-007, 029, 030, 031: drop the GPU arm from the M3 verify; it is covered in M4 by REQ-VAL-059 (Tier L),
    REQ-VAL-061 (one step) and REQ-VAL-072 (integer fields and the word's arithmetic).
  - REQ-VAL-059 and REQ-INT-059: M4 half = native GPU (Metal, and a second native backend per RQ-79); new M8
    requirement: "Parity Tier L's branch decisions and the 100-macro-step `done`-flag dispatch must match the CPU
    branch words through the browser build's WGSL path (SPIR-V → WGSL → the browser's compiler)", closed with
    REQ-VAL-116.
- **Needed:** confirm the splits (or give the milestone each leg belongs to).
- *Gaps:* M3-8, M4-3, M4-4. *Tasks:* TASK-M3-03, TASK-M3-04, TASK-M3-05, TASK-M3-16, TASK-M3-17, TASK-M4-03, TASK-M4-09.
- **Ruling:** R-113 (decisions.md). Closed in step 7.

## RQ-98: M3 and M4 requirements that name later surfaces *(step 7, plan)*

**M3 → M4 / M8.** (M3-12, M3-14)

- REQ-GUI-008 (M3, TASK-M3-19): "The IC inspector must be the shared kernel at f64 on CPU for a single IC (not a
  separate viewer), hosted in the one Inspector window." The one Inspector window is REQ-GUI-112 (M8, TASK-M8-14, R-65).
- REQ-INT-026 (M3, TASK-M3-06) verify: "default SimUniforms match the table; changing any one changes the sim key". The
  sim key is built by REQ-SCHED-007 (M4, TASK-M4-08), which lists "integrator config, T, event thresholds" and verifies
  "changing each sim-key component changes the key".
- Proposed fix: drop "hosted in the one Inspector window" from REQ-GUI-008 (REQ-GUI-112 carries it in M8); drop "be on
  the sim key" and its verify half from REQ-INT-026 (REQ-SCHED-007 carries it in M4, and its fixture includes each
  SimUniforms field).
- **Needed:** confirm (the clauses are removed from the M3 requirements, not from the docs).

**M4 → M5.** (M4-2, M4-11, M4-13a)

- REQ-INT-072 (M5, TASK-M5-18): "copy 0 the un-jittered centre, copies 1..E at Halton (2,3) points 1..E centred …
  scaled to the footprint — and compute outcome entropy H … and spread σ²_T … at resolve". But M4 already dispatches
  E ≥ 1 copies: REQ-RENDER-031 ("(E+1) full uniform samples", TASK-M4-06), REQ-RENDER-033 / REQ-RENDER-035
  (TASK-M4-11), REQ-INT-065 / REQ-INT-069 ("each copy the same kernel dispatched again", TASK-M4-06). Un-jittered M4
  copies would be E+1 identical trajectories.
- REQ-PERF-005 (M4, TASK-M4-11) verify: "GPU allocation is identical with checkerboard on and off; the estimator's
  figure is unchanged" — the memory estimator is REQ-PERF-018 (M5, TASK-M5-10).
- REQ-RENDER-031 verify "(E+1) below Medium and 2(E+1) from Medium up" and REQ-RENDER-035 "Potato (E = 0)" name tiers
  whose table is REQ-PERF-014 (M5, TASK-M5-02).
- Proposed fix:
  - split REQ-INT-072: the copy offsets (copy 0 at the centre, copies 1..E at the centred, footprint-scaled Halton
    points) move to M4, closed by TASK-M4-06; H and σ²_T at resolve stay in M5 (TASK-M5-18).
  - REQ-PERF-005: M4 keeps "no extra or half-size buffer, every stale SimState resident" (allocation identical);
    "the estimator must credit it with no memory saving" moves to M5, joined to REQ-PERF-018.
  - REQ-RENDER-031 / REQ-RENDER-035: the M4 verify is parameterised by (E, FTLE on/off) — "(E+1) with FTLE off, 2(E+1)
    with FTLE on"; "E = 0 with checkerboard on" — and the tier names are checked by REQ-PERF-014 in M5.
- **Needed:** confirm the splits.
- *Gaps:* M3-12, M3-14, M4-2, M4-11, M4-13a. *Tasks:* TASK-M3-06, TASK-M3-19, TASK-M4-06, TASK-M4-08, TASK-M4-11, TASK-M5-02, TASK-M5-10, TASK-M5-18, TASK-M8-14.
- **Ruling:** R-113 (decisions.md). Closed in step 7.

## RQ-99: M5 requirements that need M6, M7 or M8 *(step 7, plan)*

**The linearised decoder (M6).** (M5-2)

- REQ-SCHED-024 (M5, TASK-M5-04): per-quad uniforms carry "c, h, x₀, J_D; kernel computes only x₀ + J_D·δ".
  REQ-SCHED-040 (M5, TASK-M5-03), third assertion: "linearised decode within a quad is not mirrored relative to the full
  decode". REQ-SCHED-023 (M5, TASK-M5-24): "deep-zoom gesture landing with dozens of Jacobian quads".
- x₀ = D(c_u, c_v) and J_D by central differences in f64 are REQ-DEC-036 (M6, TASK-M6-07); the switchover is REQ-DEC-033/037
  (M6, TASK-M6-08). `principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)".
- The M5 tasks use fixture x₀/J_D computed by finite differences of the full decoder — which is REQ-DEC-036's computation.
- **Needed:** one of: (a) move REQ-DEC-036 (x₀ and J_D by central differences, CPU f64) to M5, closed by TASK-M5-04, leaving
  the switchover (REQ-DEC-033/037) and the error-fit tests (REQ-DEC-034, REQ-DEC-042) in M6; or (b) split REQ-SCHED-040 —
  M5: addresses and the shared Y-up convention; M6: "linearised decode within a quad is not mirrored relative to the full
  decode" (closed by TASK-M6-07) — and state that REQ-SCHED-023/024 are met at M5 with fixture Jacobians.

**The real colour mapping (M7).** (M5-3)

- `principia_checkerboard_contract.md` § "8. Build-time settles (measure on the real system)" (:119): "§7's ~0.05 is from
  the proxy; the real VMF/OKLAB colour mapping could amplify small state differences into more levels (a more sensitive
  mapping → lower ceiling) … Confirm where the mean crosses ~2 levels on the real render."
- REQ-VAL-081 (M5, TASK-M5-23) asks for the ceiling "confirmed on the real colour mapping and integrator"; the VMF/OKLab
  colour mapping is built in M7 (dd_colouring, colour_composition).
- **Needed:** split REQ-VAL-081 — M5: the gate exists and the ceiling is measured on the M5 render (integrator real, colour
  mapping the M1 debug/ramp views) where mean reconstruction error crosses ~2 8-bit levels; M7 (new requirement): the
  ceiling re-confirmed on the real VMF/OKLab mapping, lowered if the mapping is more sensitive. Or move REQ-VAL-081 to M7
  whole (checkerboard then ships in M5 at the proxy 0.05).

**The wasm worker (M8).** (M5-5)

- `principia_caching_contract.md` § "Part 6a — The threading model: the render loop lives in a worker (and that worker is
  the wasm engine)" (:116): "the entire frame loop — WebGPU device, scheduler, cache, quadtree, all compute and render — is
  the wasm engine, running in a Web Worker via `OffscreenCanvas`."
- REQ-SYS-034 (M5, TASK-M5-24): "must run in the render-loop worker"; REQ-RENDER-045 (M5, TASK-M5-26): "the frame loop runs
  in the wasm-engine worker". The worker itself is REQ-SYS-039 and REQ-SYS-049 (M8, TASK-M8-37); M5 has only the native
  build, whose loop host the corpus does not state.
- **Needed:** split both — M5 halves: "the frame loop must never await GPU work and must run on a dedicated render-loop
  thread, off the input/GUI thread" (SYS-034) and REQ-RENDER-045 without its worker clause; the worker clause is already
  REQ-SYS-039/049 (M8), so no new M8 requirement is needed.
- *Gaps:* M5-2, M5-3, M5-5. *Tasks:* TASK-M5-03, TASK-M5-04, TASK-M5-23, TASK-M5-24, TASK-M5-26, TASK-M6-07, TASK-M8-37.
- **Ruling:** R-113 (decisions.md). Closed in step 7.

## RQ-100: Existing requirements closed after the task that needs them *(step 7, plan)*

The gaps are covered by existing requirements, but those requirements are closed in a later milestone (or by a task
not on the needing task's `depends_on` path).
- REQ-PERF-074 (M8, TASK-M8-25; not `kind: calibration`): the memory-fit margin, `principia_memory_tiers.md` § "8. Caveats"
  (:236) "Budget the process estimate with margin" and § "6. Auto-mode tier selection" (:200) "fits with margin". Needed by
  TASK-M5-10 (REQ-PERF-031), TASK-M5-11 and TASK-M6-16 (REQ-PERF-048). (M5-8a, M6-5b)
- REQ-REF-045 (M6, TASK-M6-03, definition, waits on RQ-72): the per-footprint latch's layout. TASK-M5-19 builds the latch in
  M5 under REQ-PAY-065. (M5-11)
- REQ-GUI-151 and REQ-GUI-152 (M8, TASK-M8-20 / TASK-M8-19, definitions): the node palette and the Stain preview default.
  TASK-M7-22 (REQ-GUI-031) builds the canvas and preview in M7. (M7-13)
- REQ-CHART-044 (M2, TASK-M2-12): K_max, γ_K, which the Burrau (θ, K) and (ν, K) charts use (chart_reference § "4.5 The
  Burrau-family chart maps" `K(v) = K_max v^{γ_K}`); TASK-M2-10 and TASK-M2-11 don't depend on TASK-M2-12. (M2-G22)
- **Proposed fix:** REQ-PERF-074's margin half becomes an M5 calibration requirement closed by TASK-M5-10 (the rest stays
  M8); REQ-REF-045 moves to M5, closed by TASK-M5-19 (or REQ-PAY-065's latch half moves to M6); REQ-GUI-151/152 move to M7,
  closed by TASK-M7-22; TASK-M2-10 and TASK-M2-11 gain `depends_on: TASK-M2-12`.
- **Needed:** confirm each, or give the milestone.
- **Ruling:** R-113 (decisions.md). Closed in step 7.

## RQ-101: The colour golden oracle and the LUT data live outside the corpus *(step 7, colour)*

- `principia_colour_composition.md` §7 (:434–436): "The **two React reference artefacts are the oracle** (`ColourSphere`
  = Artefact 1, `PatternSphere` = Artefact 2 …). Every recreated preset ships with a **golden-image test** against the
  corresponding reference output" (REQ-COL-030, REQ-COL-046, REQ-VAL-096, tolerance REQ-COL-052; M7). Neither artefact is
  in this repo.
- §7.1 (:456) names the LUTs (Viridis, Cividis, Plasma, Magma, Inferno, Twilight, Cool-warm, Principia, Cubehelix) without
  their data; the M1 debug views already need Viridis (`dbg_lin`, render_contract :152) and Twilight (REQ-TOOL-010).
- `docs/gui/reference/principia_colour_explorer.html` and `principia_colour_presets.html` carry similar maps and LUT
  tables, but no corpus passage names them as the oracle or as the LUT source.
- The same holds for the Principia palette ("indigo → teal → gold") and Cool-warm ("diverging"): §7.1 (:456) gives no
  stops; they exist only in `principia_colour_explorer.html` :108 and `principia_colour_presets.html` :110–111 (M7-5).
- **Needed:** where the oracle comes from — the React artefacts added to the repo, or the two reference HTML files named
  as the oracle (a doc change to §7) — and the LUT data source (those files, or the published matplotlib tables for the
  matplotlib maps).
- *Gaps:* M1-16, M7-4, M7-5. *Tasks:* TASK-M1-03, TASK-M1-12, TASK-M7-07, TASK-M7-18.
- **Ruling:** R-122 (decisions.md). Closed in step 7.

## RQ-102: The regularisation occupants and step control that live only in prin-rs *(step 7, integrator)*

- `principia_integrator_contract.md` § "The split" (:106): the step / deriv / Hamiltonian layer "PORTS AS-IS … Porting is
  transcription" (REQ-INT-016) — from prin-rs, which is not in this repo.
- § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper" (:168–172) names four occupants;
  only Aarseth–Zare has an in-repo reference (`workbench/tb_az.py`, `tb_az_overshoot_fix.py`, outside the corpus). Heggie
  1974 (the default, REQ-INT-051) and logH (REQ-INT-018, the R-74 falsification check) have no equations of motion,
  time transformation or step control anywhere in `docs/`.
- § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature" (:289–295): "Time-transformed
  leapfrog (Mikkola–Tanikawa) … is required, not optional" — no equations. REQ-VAL-044 (TASK-M3-29) is "blocked on a
  reversible occupant"; nothing says whether AZ + Mikkola–Tanikawa is built in M3 or the slot is only specified
  (":292: Specify the slot now even if RK4 fills it initially").
- `principia_INDEX.md` § "The evidence base — where settled defaults were measured" (:27): the **predictive step
  limit** "lives there" (prin-rs) and "is what removed the wedges"; REQ-INT-052 requires it on by default. pitfalls §8
  (:371–375) names its ablation arms (`dtau only`, `clamp only`, `limit only`) and "wedge density" without defining
  the clamp, the limit or the metric.
- R-72 must not be used to write physics equations.
- **Needed:** the source for the Heggie, logH and Mikkola–Tanikawa equations and their step control, and for the
  predictive step limit, the clamp arm and the wedge-density metric (import the prin-rs text into the corpus, or
  name the papers and let the task transcribe them for physics review); and whether the reversible AZ +
  Mikkola–Tanikawa occupant is built in M3 (else REQ-VAL-044 moves to the milestone that builds it).
- *Gaps:* M3-1, M3-5, M3-9. *Tasks:* TASK-M3-07, TASK-M3-08, TASK-M3-29, TASK-M3-31.

## RQ-103: The prin-rs fixtures and slices the M3 re-runs need *(step 7, validation)*

- The M3 numerical gates re-run measurements made in prin-rs, on inputs defined only there:
  - REQ-INT-051 (TASK-M3-32): "31 of 32 cases, `err>10` 3916 → 73" (`principia_integrator_contract.md` § "Part 2b —
    Regularisation is a SECOND swappable axis, not a property of the stepper", :171) — the 32 cases and the `err>10`
    metric are not defined in `docs/`.
  - REQ-INT-050: "+0.305" / "−0.082" (:169) — the control's fixture ICs.
  - REQ-INT-052 (TASK-M3-31): the ablation "on config_stability" (pitfalls § "8. TWO ARTEFACTS ARE NOT ONE DEFECT");
    the slices `config_stability`, `near-field`, `far`, `deep interior`, "the config slice" are named
    (dd_refinement_policy :70, :141, :205–206) but their chart, z₀, q₁, q₂ and extent are not given.
  - REQ-VAL-040 (TASK-M3-34): "the legacy `t = 30` set kept as a comparison" (pitfalls § "2.2 The criterion", :153).
  - REQ-VAL-028 (TASK-M3-36): BodyPlane "must reproduce bit-for-bit … and the Python cross-check green"
    (`principia_chart_reference.md` § "5.2 Tests that can fail", :548). REQ-VAL-119 has the task define the map, but the
    reference dump and the Python cross-check are prin-rs artefacts.
- **Needed:** whether the prin-rs slice definitions, case matrix, control ICs, legacy set and BodyPlane dump are
  imported into the repo (and where: `fixtures/`), or the tasks define new fixtures and the gates compare against the
  recorded numbers only (then "bit-for-bit" in REQ-VAL-028 has no reference to match).
- *Gaps:* M3-6, M3-10. *Tasks:* TASK-M3-31, TASK-M3-32, TASK-M3-33, TASK-M3-34, TASK-M3-35, TASK-M3-36.

## RQ-104: Checkerboard at the dt ceiling: ramp or hard gate ("a feel call") *(step 7, checkerboard)*

- `principia_checkerboard_contract.md` § "8. Build-time settles (measure on the real system)" (:120): "optionally *ramp*
  the stale fraction down as `dt` approaches the ceiling … Whether a ramp is worth the complexity over a hard gate is a
  feel call." REQ-VAL-081 (TASK-M5-23) leaves it open: "above it checkerboard is off (or ramped)".
- A feel call is a product decision, not a value R-71 can calibrate or a definition R-72 can write.
- **Needed:** hard gate or ramp (the task can supply captures of both at the ceiling as evidence).
- *Gaps:* M5-3b. *Tasks:* TASK-M5-23.
- **Ruling:** R-128 (decisions.md). Closed in step 7.

## RQ-105: GUI surfaces with no artboard *(step 7, GUI)*

- REQ-GUI-014 (M6, TASK-M6-22): Custom mode exposes render_scale (0.25–2.0), N, MAX_REL_DEPTH, E, FTLE, motion gating and
  lock-to-native (`principia_memory_tiers.md` § "5. Controller levers, ranked by impact"); verify: GUI screenshot, "the
  Custom quality panel shows each control". REQ-TOOL-058 (TASK-M6-22): the arbiter debug overlay shows throughput, rung,
  headroom and recent decisions (`principia_quality_device_note.md` § "10. Two sanctities: the user, and observability");
  verify: GUI screenshot.
- `docs/gui/design/04_windows.png` (render_gui_spec § "G5. Windows (`04_windows.png`)" :164; GUI_DESIGN_NOTES § "04 Windows" :64): the Run window shows
  quality, budget, max depth and ensemble only; no artboard shows the Custom fields or the arbiter overlay.
  `plan/WORKFLOW.md`:79: a GUI screenshot is compared against `docs/gui/design/NN_*.png` for layout.
- **Needed:** an artboard (or a sketch) for the Custom panel and the arbiter overlay; or a ruling that these two are checked
  by presence of their controls/items only (no layout comparison) until the M8 dev GUI.
- `principia_dd_telemetry_and_tiers.md` § "A deliberate ceiling, user-visible" (:343–348): "Offer a **target utilisation** … 'Use up to N cores' and 'cap at 30 fps'" (REQ-GUI-044, TASK-M8-25). No window in `principia_render_gui_spec.md` and no artboard (`04_windows.png` Run window: quality, budget, max depth, ensemble) carries it.
- § "8. What this is not" (:414–415): "passive logging is a mode with a **visible indicator**" (REQ-TOOL-088, TASK-M8-26). Neither the spec nor `GUI_DESIGN_NOTES.md` places the mode's switch or its indicator.
- R-68: artboard values are illustrative, but the artboards are the approved layout; placing a new control is a design decision.
- **Needed:** where each lives (e.g. the Run window, the footer, the top bar) — or leave it to the GUI reviewer at the M8 gate.
- *Gaps:* M6-7, M8-7b, M8-17. *Tasks:* TASK-M6-22, TASK-M8-25, TASK-M8-26.
- **Ruling:** R-129 (decisions.md). Closed in step 7.

## RQ-106: The Euler landmarks for unequal masses *(step 7, colour)*

- `principia_dd_integrator.md` §3.7 (:218): the Euler landmarks are the "Euler collinear" configurations, "equator, between the collisions"; the only formula is "ê_j = −b̂_j (equal masses)" (:225). `principia_colour_composition.md` §2 (:197): "`Euler(m)` — collinear configs", which "move with (m₀,m₁,m₂)".
- R-50 makes the **collision** landmarks mass-weighted; it says nothing of the Euler points. With unequal masses the antipode of b̂_j and the Euler central configuration (the collinear relative equilibrium, a root of Euler's quintic in the mass ratios) are different points on the equator.
- `principia_colour_explorer.html` :126 places Euler blobs at the antipodes of its (heuristically skewed) BC points; REQ-COL-021's verify gives only the equal-mass values.
- **Needed:** which points the Euler landmarks are for unequal masses (antipodes of the mass-weighted b̂, or the Euler central configurations mapped through the shape map) — a physics definition the corpus doesn't give.
- *Gaps:* M7-7. *Tasks:* TASK-M7-09.
- **Ruling:** R-126 (decisions.md). Closed in step 7.

## RQ-107: The style presets are named, not defined *(step 7, colour)*

- `principia_render_gui_spec.md` § "Display — the last stages" (:202–203): "**Style** is optional and applies to the figure only. Scientific checks run with **plain**. Presets: plain, watercolour & pencil, print · Poster78, more; with paper grain and press misregistration." REQ-COL-044 (M7, TASK-M7-26) requires them.
- `GUI_DESIGN_NOTES.md` § "04 Windows" (:73–75) and `04_windows.png` show the Display window; no doc, artboard or reference HTML defines what any style computes, its parameters, or what "Poster78" is.
- A product/design decision (the look), not a definition R-72 can supply.
- **Needed:** each style's look (a reference image or description and its parameters: paper grain, misregistration), or REQ-COL-044 reduced to plain for v1 with the styles deferred.
- *Gaps:* M7-12. *Tasks:* TASK-M7-26.
- **Ruling:** R-130 (decisions.md). Closed in step 7.

## RQ-108: The MP4 / GIF encoders *(step 7, export)*

- `principia_render_gui_spec.md` §G9 (:283): Record a time sweep offers "GIF / PNG frames / MP4"; `principia_export_animation_contract.md` Part 4 gives the blocking frame loop but no encoder.
- The browser build (R-85, M8) has no built-in MP4 or GIF encoder: the choice (WebCodecs, a wasm encoder, or PNG frames only in the browser) sets browser support, bundle size and codec licensing.
- **Needed:** the encoders for native and browser, or which formats the browser offers in v1.
- *Gaps:* M8-10. *Tasks:* TASK-M8-30, TASK-M8-33.
- **Ruling:** R-131 (decisions.md). Closed in step 7.

## RQ-109: Research v2: the fold stability measure and the Poincaré sections *(step 7, research)*

- `principia_render_gui_spec.md` §G11 (:304): continuation "marking folds where stability changes" (REQ-GUI-123) — the corpus defines no stability measure for a periodic orbit (monodromy / Floquet multipliers or another) and no fold test.
- §G11 (:305): "Poincaré return map on a chosen section" (REQ-GUI-124) — which sections are offered (a coordinate hyperplane in phase space, a shape-sphere great circle, a syzygy crossing) is not given.
- Physics the corpus doesn't hold; R-72 must not invent it.
- **Needed:** the stability measure and fold criterion, and the section family offered — or a source (prin-rs, literature) the task transcribes.
- *Gaps:* M8-13a, M8-13b. *Tasks:* TASK-M8-36.
- **Ruling:** R-127 (decisions.md). Closed in step 7.

## RQ-110: Silences classified under R-71 and R-72 at checkpoint B *(step 7, checkpoint B)*

Each item is a new requirement in , closed by the task named. Tick any you want reclassified —
a value that is really a design decision, or a definition that is really physics the corpus must supply (R-72 must not invent it).

- [ ] REQ-TOOL-119 · definition (M0) · TASK-M0-18 · `prin profile diff … --threshold P%`: which statistic and which scopes the P% regression compares. (M0-5)
- [ ] REQ-TOOL-120 · definition (M0) · TASK-M0-17 · Profiler schema v1's concrete JSON: key names, the nesting of scopes, GPU passes, allocations and events, and the session header's layout. (M0-7)
- [ ] REQ-TOOL-121 · definition (M0) · TASK-M0-19 · Session header: 'reported f64 rate' has no source (the graphics API reports f64 support, not rate), and the display fields have no value in a headless run. (M0-8)
- [ ] REQ-GEN-024 · definition (M0) · TASK-M0-07 · The §3.8 metadata schema has no `location` kind for fields derived at read (ftle, energy_drift, Lz_drift, diffusion, n) and no type for a vector field, though 'the ledger knows n is a vector'. (M0-10, M1-14)
- [ ] REQ-SYS-063 · definition (M0) · TASK-M0-08 · The constants register: §3.8 has no citation or admissibility-class field, and nothing says whether the register is inside the R-36 hashed ledger. (M0-11)
- [ ] REQ-PAY-087 · definition (M0) · TASK-M0-14 · The f64 (and DoubleF64 stub) instantiation of the payload layout: which fields widen with Real (f16 latches, packed u32 words, u16 steps) is unspecified. (M0-13)
- [ ] REQ-VAL-138 · calibration (M0) · TASK-M0-06 · The golden-image runner's diff metric and tolerance: none is given, and goldens are asserted from M1; REQ-COL-052 covers only the M7 preset-vs-reference tolerance. (M0-14b)
- [ ] REQ-COL-055 · calibration (M1) · TASK-M1-03 · The invalid colour's value: the corpus says 'a fixed magenta, a plain default, no source claimed'; the outcome palette already uses magenta #E034C6 for body-1 escape. (M1-1)
- [ ] REQ-TOOL-122 · definition (M1) · TASK-M1-03 · The debug presentation helpers are signatures only: dbg_sentinel's hatch, dbg_cat's golden-angle lightness/chroma, dbg_log's form, dbg_hash_u32's hash, dbg_flag's green/red. (M1-2, M1-3)
- [ ] REQ-RENDER-077 · definition (M1) · TASK-M1-01 · The tier-absent sentinels' exact values: the canonical quiet-NaN bit pattern the bitcast test compares against, and the 'empty/sentinel word' an unbound word buffer returns (length 0 or the 127 truncation sentinel?). (M1-4, M1-5)
- [ ] REQ-TOOL-123 · definition (M1) · TASK-M1-11 · The kernel bring-up mode's known pattern ('e.g. ctx-derived UV or a fixed ramp') and which payload slots it writes are not given. (M1-7)
- [ ] REQ-COL-056 · definition (M1) · TASK-M1-06 · `ctx.tile.uv` (within-tile coordinate) is used by render_gui_spec §12.1's boundary overlay but is missing from colour_composition §3's tile/sample lane. (M1-12)
- [ ] REQ-TOOL-124 · definition (M1) · TASK-M1-13 · Structural overlay styling: the pending-hatch pattern and the fallback-tint colour are not given. (M1-15)
- [ ] REQ-COL-057 · definition (M2) · TASK-M2-25 · ctx.chart.z ('the full 8-D latent at this pixel') is undefined on charts whose Φ does not pass through z (shape sphere, invariant, Burrau, Anosova) (M2-G2)
- [ ] REQ-CHART-047 · definition (M2) · TASK-M2-28 · Which equal-area projection (Mollweide or Hammer–Aitoff), and is the projection a chart map (sim key) or a display remap? (M2-G6)
- [ ] REQ-PAY-088 · definition (M2) · TASK-M2-03 · ICDescriptor derived fields q_mass, rho_mag, lambda_mag, rho_ratio, rho_angle, r_min_pair_0 have no formulas (M2-G7)
- [ ] REQ-CHART-048 · definition (M2) · TASK-M2-08 · Frozen values of the nonlinear charts: shape sphere's m_fixed/p_fixed (source, frame relative to the fibre-phase rotation) and φ_f; invariant charts' fixed geometry and masses (M2-G8)
- [ ] REQ-DEC-044 · calibration (M2) · TASK-M2-02 · dd_decoder unit tests 1, 6 and 8 give no tolerance ('precision-appropriate', 'to tolerance') (M2-G9a)
- [ ] REQ-GEN-025 · calibration (M2) · TASK-M2-01 · Generation-root test 8 (c) log-det vs numeric Jacobian and (d) C¹ check give no tolerance (M2-G9b)
- [ ] REQ-CHART-049 · calibration (M2) · TASK-M2-08 · Tolerance of the shape_vec cross-check against the IC Inspector's JS is not given (M2-G9c)
- [ ] REQ-GEN-026 · definition (M2) · TASK-M2-01 · REQ-GEN-014 needs ≥2 links per block; the alternative links (edge-reaching simplex map, heavier-tailed bounded map, temperature-softmax) have no formulas and §3.9 has one simplex link (M2-G10)
- [ ] REQ-CHART-050 · definition (M2) · TASK-M2-05 · chart_reference §5.2's 'a direct (α, β) sweep' is undefined (through the links, or linear in angle?) (M2-G12)
- [ ] REQ-ENC-029 · definition (M2) · TASK-M2-16 · ‖·‖_phys (the physical-units norm of the T2 round trip) is undefined; REQ-ENC-024's ε_phys presupposes it (M2-G15)
- [ ] REQ-ENC-030 · definition (M2) · TASK-M2-17 · Chart-aware validation: the metric of 'the nearest valid point' / 'nearest feasible point' and the 'qualitatively different IC' criterion are undefined (M2-G16)
- [ ] REQ-ENC-031 · definition (M2) · TASK-M2-19 · Partial-specification lookup: when more than two DOF are unspecified, which two the slice basis spans (M2-G17)
- [ ] REQ-CHART-051 · definition (M2) · TASK-M2-22 · The named compound directions (energy at fixed L_z, mass away from Burrau, the morph) are not given as latent vectors (M2-G19)
- [ ] REQ-INT-080 · calibration (M3) · TASK-M3-07 · The regularised occupants' sync schedule (the number of sync boundaries n_sync, AZ's eta in dtau = eta·dt_left/(A·B)) has no default (M3-2)
- [ ] REQ-EVT-024 · calibration (M3) · TASK-M3-12 · The escape settling threshold tau has no value ('—', 'to re-measure'); the detector (TASK-M3-12) needs one before TASK-M3-34 measures the gap (M3-3)
- [ ] REQ-INT-081 · calibration (M3) · TASK-M3-06 · T_horizon has a range [50, 200], no default — and with dt_macro fixed at 10⁻³ the enforced ⌈T/dt_macro⌉ ≤ 65535 admits only T ≤ 65.535 (M3-4)
- [ ] REQ-TOOL-125 · definition (M3) · TASK-M3-07 · dt_max (REQ-TOOL-039) is carried by no ledger or kernel field: where it lives is undefined (M3-11)
- [ ] REQ-VAL-139 · calibration (M3) · TASK-M3-26 · No tolerance on the fitted order-scaling slope (dd_integrator test 1) or on 'no secular trend' (test 2) (M3-13a)
- [ ] REQ-PERF-085 · calibration (M3) · TASK-M3-20 · REQ-PERF-004's benchmark 'typical IC' is undefined (and the tail is what the budget exists for) (M3-13b)
- [ ] REQ-TOOL-126 · definition (M4) · TASK-M4-13 · The horizon map's eps in t_max(IC) = ln(1/eps)/ftle(IC) is not said (f32's, f64's, or the measurement horizon's jitter δ) (M4-6)
- [ ] REQ-TOOL-127 · definition (M4) · TASK-M4-16 · The on-demand single-IC f32 GPU trace: its output buffer (dense n(t) layout), how the survey kernel writes it, the 'shared times' sampling, and the divergence-time criterion are undefined (M4-7, M8-15)
- [ ] REQ-VAL-140 · definition (M4) · TASK-M4-04 · Tier N's 'growing-but-bounded envelope' for short pre-divergence trajectories: its form and the divergence-onset time are not given (M4-8)
- [ ] REQ-VAL-141 · calibration (M4) · TASK-M4-04 · The numerical-ambiguity band outside which CPU↔GPU word parity is tested has no width (M4-9)
- [ ] REQ-RENDER-078 · definition (M4) · TASK-M4-12 · Spread-extrapolation's `rate` — the local rate of change of what, from which integration by-product — is undefined (M4-10)
- [ ] REQ-PERF-086 · calibration (M4) · TASK-M4-19 · The tier table's N = 24 / 32 (Ultra, Extreme) break N² ≤ 256; R-43 says fix them but gives no replacement values (M4-12)
- [ ] REQ-SCHED-087 · calibration (M5) · TASK-M5-14 · The in-flight job limit: scheduler Part 6 gives the range 2–4, not the value (M5-6)
- [ ] REQ-SCHED-088 · calibration (M5) · TASK-M5-25 · Depth of the in-motion coarse cover ("a few levels above camera depth"), and whether it is the baseline cover's depth (M5-7)
- [ ] REQ-PERF-087 · calibration (M5) · TASK-M5-11 · Where 'pressured' begins: telemetry's three memory states give 'approaching the cap' with no threshold (M5-8c)
- [ ] REQ-RENDER-079 · calibration (M5) · TASK-M5-26 · The stale backdrop's blur radius (M5-9)
- [ ] REQ-REF-047 · definition (M6) · TASK-M6-01 · Which Decision variant each stop records (AT_F32_FLOOR, the integration floor) and what produces BalanceForced (M6-1)
- [ ] REQ-SCHED-089 · calibration (M6) · TASK-M6-02 · The motion refinement floor's offset: 'one or two levels above pixel-size' (M6-2)
- [ ] REQ-REF-048 · definition (M6) · TASK-M6-05 · 'Acquire a third scale' — which scale, and when (M6-3)
- [ ] REQ-PERF-088 · calibration (M6) · TASK-M6-16 · Device-characterisation probe values: quad set, duration, percentile, thermal headroom, the info-leg inconsistency test (M6-5a)
- [ ] REQ-REF-049 · definition (M6) · TASK-M6-19 · The sea_fraction(eps) estimator's method (M6-6)
- [ ] REQ-VAL-142 · calibration (M6) · TASK-M6-23 · REQ-VAL-085's 'without over-degrading' has no threshold (M6-8)
- [ ] REQ-REF-050 · calibration (M6) · TASK-M6-23 · tau's value: REQ-REF-032 says 'set by a calibrated grid measurement' but is not kind: calibration (M6-9a)
- [ ] REQ-PERF-089 · calibration (M6) · TASK-M6-18 · The arbiter's tunables (REQ-PERF-066) are 'set by recorded measurement' but not kind: calibration (M6-9b)
- [ ] REQ-PERF-090 · calibration (M6) · TASK-M6-16 · Quality-controller thresholds/ladder and the device budget heuristics (REQ-PERF-038) are 'set by measurement' but not kind: calibration (M6-9c)
- [ ] REQ-SCHED-090 · calibration (M6) · TASK-M6-11 · P_focus decay constant(s) (M6-10)
- [ ] REQ-RENDER-080 · calibration (M7) · TASK-M7-16 · Equirect bake texture resolution and texel format (sets REQ-RENDER-067's 'one texel quantisation step') (M7-3)
- [ ] REQ-RENDER-081 · calibration (M7) · TASK-M7-12 · Invalid-graph fallback: 'flat grey / error tint' — which one and its colour (M7-8)
- [ ] REQ-COL-058 · definition (M7) · TASK-M7-08 · Categorical filter: how the muted (filtered-out) classes are drawn (M7-9)
- [ ] REQ-COL-059 · definition (M7) · TASK-M7-21 · Gamut-clamp method (per-channel clip vs chroma-reducing clamp) (M7-11)
- [ ] REQ-GEN-027 · definition (M1) · TASK-M1-04 · How a fragment .wgsl file declares its uniformSchema and inputDomains (registry declaration format) (M7-14)
- [ ] REQ-PERF-091 · calibration (M8) · TASK-M8-01 · Fixed bound on the GUI snapshot's serialised size ('GUI-sized') (M8-1)
- [ ] REQ-GUI-155 · definition (M8) · TASK-M8-07 · What the scrubber's 'refining · 72%' percentage measures (M8-2)
- [ ] REQ-GUI-156 · definition (M8) · TASK-M8-10 · Hover path fade: the distance measure from the slice and the opacity fall-off (M8-3)
- [ ] REQ-GUI-157 · calibration (M8) · TASK-M8-11 · Rotation rate of the turning shape sphere (M8-4)
- [ ] REQ-GUI-158 · calibration (M8) · TASK-M8-13 · Keyboard base step per adjustable field/scope before Shift ×10 / Alt ×0.1 (M8-5)
- [ ] REQ-ENC-032 · definition (M8) · TASK-M8-16 · Formula of the Inspector's canonicalisation conditioning number (ρ → 0, R → 0) (M8-6)
- [ ] REQ-PERF-092 · calibration (M8) · TASK-M8-25 · Default headroom of the target-utilisation ceiling (M8-7a)
- [ ] REQ-TOOL-128 · definition (M8) · TASK-M8-28 · Transport for `prin profile query --live` to a running app (M8-8)
- [ ] REQ-TOOL-129 · definition (M8) · TASK-M8-31 · URL encoding of the spec object and the principia://view?… query format (M8-9)
- [ ] REQ-VAL-143 · definition (M8) · TASK-M8-34 · Measure's 'threshold sweep': which threshold is swept (M8-11)
- [ ] REQ-GUI-159 · definition (M8) · TASK-M8-36 · Research side-by-side 'difference view': what is differenced and how it is shown (M8-13c)
- [ ] REQ-GUI-160 · definition (M8) · TASK-M8-18 · Storage of user-side stores: chart-builder presets, 'Your stains', per-choice warning suppressions (M8-16)
- **Needed:** confirm the classification, or name the items to move to an RQ of their own.
- **Ruling:** R-132 (decisions.md). Closed in step 7.

## RQ-111: Interpretations taken at checkpoint B *(step 7, checkpoint B)*

Gaps the task agents raised that the corpus answers, as read here. Nothing in `plan/` or `docs/` was changed for them.
Tick any you don't accept.
- [ ] SimConfig's full field list (M0 gap 17): `principia_gui_state_contract.md` §2 lists the groups and says "the GUI
  requirement adds **no new state**"; each group's fields come from the contract that owns them (e.g. integrator
  Part 3). TASK-M0-16 declares surfaces and groups only.
- [ ] M0 scaffolding with no requirement id (M1 gap 17): `depends_on` names earlier-milestone tasks directly; a task
  with no other dependency depends on TASK-M0-01.
- [ ] REQ-DEC-014's "saturation flag" (M2 gap G14): at an exact α-pole the encode clamp fires
  (`principia_inverse_encode_contract.md` Part 3–4, `principia_dd_encode.md` §3.5 `lookup_clamped`); R-21's "SAT flags"
  is read as that clamp flag, not the payload's `saturated` bit (the substep cap). TASK-M2-15 tests `lookup_clamped`.
- [ ] REQ-CHART-044/045/046's evidence renders (M2 gap G21): their verify details ask for coverage of the chart domain
  (feasibility, K, L_z), which decode-time renders show; no integrated field is needed at M2.
- [ ] REQ-SCHED-045 (M5 gap 4): its check ("no blank pixels" during a fast pan and zoom) compares no layout, so no
  artboard is needed; its verify method reads better as `property test` than `GUI screenshot` (not changed).
- [ ] The Research "winding number" (M8 gap 12, `principia_render_gui_spec.md` § G11): read as the payload's winding —
  the unwrapped phase θ̃ and `orbit_count = ⌊|θ̃|/2π⌋` (`principia_dd_integrator.md` §3.7,
  `principia_dd_simstate_payload.md`). If another winding is meant (about the collision points, symbolic), it needs a
  definition.
- [ ] The M2 decode agreement tolerance (M2 gaps G4, G20): before REQ-VAL-064 (M4) sets Tier N, "Tier-N tolerance" in
  REQ-TOOL-029 and "f32 noise" in REQ-COL-006 are read as REQ-DEC-043's calibrated f32 decode factor (TASK-M2-06);
  the preset compares against the decode stage's E₀ = K₀ + V₀ (R-86), not SimState.E_0.
- **Needed:** accept, or rule otherwise on any item.
- **Ruling:** R-133 (decisions.md). Closed in step 7.

---

*Found while applying R-110 to R-133 (step 7, checkpoint B rulings). Nothing is chosen.*

## RQ-112: R-110 — the Playwright suite's and the aggregate survey's frequency *(step 7, CI)*

- `principia_parity_contract.md` §6 (:173): the colour row now reads "native `wgpu` offscreen from M1; at M8 the
  **Playwright** browser suite checks against the same baselines within tolerance (R-110)", scheduled "every commit
  (native goldens, R-110)". R-110 gives no frequency for the M8 Playwright suite.
- The same table (:172): "Aggregate survey (§5) … nightly / pre-release (heavier)". R-110 names the unit, property,
  numerical-gate, native golden, benchmark and screenshot cadences, not this one; "pre-release" has no other meaning in
  the plan now (the gates are the milestone gates).
- **Needed:** the Playwright suite's cadence (every commit once `web/` exists, nightly, or at the gates), and whether the
  aggregate survey stays "nightly / pre-release" or becomes "nightly and at each gate".
- **Ruling:** R-134 (decisions.md). Closed in step 7.

## RQ-113: R-102 — what's left of systems_architecture §5.5's in-thread copies *(step 7, dispatch)*

- `principia_systems_architecture.md` §5.5, now "5.5 THE DISPATCH SHAPE — one thread per texel, one dispatch per ensemble
  copy" (R-124): the code block keeps "shared memory holds the REDUCTION ACCUMULATORS, not live states" (:183). With one
  copy per dispatch, the copies are no longer folded in-thread; where the across-copy fold happens (a later dispatch, the
  resolve stage, atomics) isn't stated.
- :201–209, "### And serial copies probably IMPROVE load balance": "With copies **serial within a thread**, each thread's
  cost is a **sum of 8 draws** … The serial arrangement is likely better balanced". Its premise is gone under R-102.
- **Needed:** where the across-copy reduction lives; and whether the load-balance subsection is marked superseded (R-102)
  or rewritten for per-dispatch copies (and if rewritten, with what argument — none is measured).
- **Ruling:** R-135 (decisions.md). Closed in step 7.

## RQ-114: R-132 — the hatched invalid pattern in the fragment code *(step 7, render)*

- `principia_render_gui_spec.md` §10.1 (:620): "**`DEBUG_NAN : vec3<f32>`** — the reserved invalid-pixel treatment …
  the hatched pattern". A hatch depends on the pixel position; a `vec3<f32>` constant can't carry it.
- `principia_render_contract.md` (:156): `fn dbg_sentinel(x: f32) -> vec3f // −1.0 sentinel and absence-NaN … → the
  hatched invalid pattern (R-132)`. Before R-132 the view told the two apart ("−1.0 sentinel → magenta", "absence-NaN →
  hatched"); now both render the same.
- **Needed:** `DEBUG_NAN`'s type (a function of the fragment position, e.g. `fn debug_nan(p: vec2f) -> vec3f`, or a
  pattern the calibration REQ-COL-055 defines); and whether `dbg_sentinel` keeps a distinct treatment for the −1.0
  sentinel (a second pattern, or a tint of the hatch) or both are "no data".
- **Ruling:** R-136 (decisions.md). Closed in step 7.

## RQ-115: R-132 — the tier table's Ultra and Extreme rows *(step 7, quality)*

- `principia_memory_tiers.md` §4 (:111–115): N is 16 for Ultra and Extreme, and "their `E` and `render_scale` values are
  calibrated (REQ-PERF-086, R-132)"; the table still shows the old E = 7 / 15 and 1.0×, and the paragraph "**Extreme is
  `1.0×` native, not supersampled.** 16× ensemble SSAA already handles …" and the Total-GB table assume those values.
- **Needed:** whether the table keeps the old E / render_scale as placeholders (marked "calibrated, REQ-PERF-086") and the
  Extreme paragraph and Total-GB rows are marked provisional, or the cells are emptied until the calibration lands.
- **Ruling:** R-137 (decisions.md). Closed in step 7.

## RQ-116: R-132 — is `dt_macro` still a Run-window parameter? *(step 7, GUI)*

- `principia_render_gui_spec.md` (:226): the Run window lists `dt_macro` among integrator_contract Part 3's parameters.
- R-132: "`dt_macro = max(1e-3, T/65535)`" — derived from T.
- **Needed:** whether `dt_macro` leaves the Run window (shown read-only as derived), or stays editable as an override
  (and then how the u16 bound of R-86 is kept).
- **Ruling:** R-138 (decisions.md). Closed in step 7.

## RQ-117: R-122 — Turbo's data, and the §7.1 artefact labels *(step 7, colour)*

- `principia_colour_composition.md` §7.1 (:462): "| Turbo | a 1-D colour LUT, shown among the additional colour map
  modes |". R-122 names the source for viridis, cividis, plasma, magma, inferno, twilight, cubehelix (matplotlib),
  cool-warm (Moreland) and the Principia stops (the explorer); not Turbo. The explorer carries a Turbo table
  (`principia_colour_explorer.html` :108).
- The same section's group labels, "**Artefact 1 — colour maps (`ColourSphere`).**" (:456) and "Artefact 2 … (`PatternSphere`)"
  (:467), still name the React artefacts R-122 replaced as the oracle.
- **Needed:** Turbo's source (Google's published Turbo table, or the explorer's); and whether the labels drop the React
  names (they are only group names now).
- **Ruling:** R-139 (decisions.md). Closed in step 7.

## RQ-118: Readings taken while applying R-110 to R-133 *(step 7, checkpoint B rulings)*

Tick any you don't accept.
- [ ] R-118: `principia_chart_reference.md` §5.1 (:510) reads `fn map<F: Float>(&self, u: F, v: F) -> ChartOut<F>`; the
  ruling gives the parameters, the generic return type is inferred.
- [ ] R-116: "hand-WGSL" still describes the colour side (parity §6 :173 "colour side stays hand-WGSL";
  `principia_systems_architecture.md` :49, :83, :149). Read as: the colour occupants stay hand-written; only the decode
  and encode are generated (the exception is stated in lowering Part 2 and canonical_spec §2 item 4).
- [ ] R-111: the retired term `TIMEOUT` is the case-sensitive identifier; the prose "no separate timeout state"
  (canonical_spec :117, systems_architecture :239, dd_integrator :194, integrator_contract :341) is not wrapped, and the
  lint matches identifiers case-sensitively.
- [ ] R-113: caching_contract Part 6a keeps its heading ("the render loop lives in a worker"), true for the browser; the
  native build's dedicated render thread is a paragraph under it.
- **Needed:** accept, or rule otherwise on any item.
- **Ruling:** R-140 (decisions.md). Closed in step 7.
