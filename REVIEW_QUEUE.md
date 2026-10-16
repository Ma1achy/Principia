# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

## RQ-278: §3.7 grounds `running_mean_divergence`'s and `coherence`'s (−∞, ∞) in the default reading R-426 retires, and `alpha_energy`'s declared (−∞, ∞) `lin` would centre its view on zero where §3.7 says 1.0 *(conflict, R-426, R-400, R-403, R-71, REQ-PAY-093, REQ-PAY-094, REQ-GEN-035; context: TASK-M1-21, TASK-M5-01, the M5 gate)*

*Found by TASK-M1-21's fix round (code review 5511539171 F3), once an uncited (−∞, ∞) became a gap on every ledger.
Nothing is chosen.*

- **File, section:**
  - `docs/design/principia_dd_generation_root.md` § "3.7" "Layout and §3.8 entries", "The §3.8 entries", line 488:
    "an f32 member §3.7 gives no range is `(−∞, ∞)`, as §3.8's worked entries are". The table below it (:500–520)
    writes `(−∞, ∞)` for `alpha_area`, `alpha_energy`, `running_mean_divergence` and `coherence`.
  - The same section, "Scales (REQ-PAY-094)", line 527: "`alpha_energy` is `lin`, centred on its analytic 1.0, not on
    zero."
  - `decisions.md` § "R-426 — …", the human's words: "an undeclared range is a ledger gap, not "spans zero"; a field
    is diverging only when its declared range spans zero (RQ-260)". R-400: a numeric view whose field's declared range
    spans zero is drawn on the diverging ramp, centred at zero.
- **Conflict:**
  - Line 488 makes `running_mean_divergence`'s and `coherence`'s (−∞, ∞) the range a member gets when §3.7 gives it
    none: the default R-426 says declares nothing. Whether the table's row is then a declaration, or a gap the row
    merely writes, the corpus does not say.
  - `alpha_energy`'s row declares (−∞, ∞) and its scale is `lin`, so under R-400 its view would be on the diverging ramp
    centred at zero; line 527 says it is centred on 1.0, not on zero. `alpha_area` (`diverging`, its range spanning
    zero by R-403) does not conflict.
  - The same holds in kind for `running_mean_divergence` and `coherence`. Option (a), applied to the ledger meanwhile
    (`crates/ledger/src/reduction.rs`, the two `.cited(SECTION_3_7)` members at the end of `entries()`), makes both
    `lin` over a declared (−∞, ∞) that spans zero, so under R-400 any view of them is drawn on cool-warm centred at
    zero. The corpus points to both being one-signed:
    - `coherence`: dd_generation_root.md:444 reads "| `coherence` | f32 | priority ordering
      (`complexity = 1 − coherence`) |"; `docs/contracts/principia_scheduler_contract.md` § "Priority" (:133) reads
      "complexity (`1 − coherence`) drives adaptive refinement". `docs/design/principia_debug_tooling_plan.md`:121
      draws it as "| coherence / impurity | scalar heatmap | = reduction output; high where boundaries |";
      the render contract's `RenderQuad` (:15) carries it as `coherence_score`.
    - `running_mean_divergence`: dd_generation_root.md:406 reads "| `running_mean_divergence` | f32 | diagnostic, not
      a split input (R-99) |": the running mean of the per-footprint divergence whose latched running maximum (:410,
      "`running_max_divergence` (f32, max-updated,") is compared with `eps` as a spread (:413, "a footprint is
      unresolved if its spread exceeds `eps` now, or its latched"). A spread is ≥ 0.
    - Neither has a formula in the corpus, so no line fixes either range.
- **Options seen:**
  - (a) Keep the table's rows as declarations: the four are declared (−∞, ∞) citing §3.7's table, and line 488 is
    reworded to cite the rows, not a default; `alpha_energy`'s view is then on cool-warm centred at zero, and line 527
    is reworded.
  - (b) Treat line 488's members as gaps until a requirement declares their ranges, as R-426 does for the payload;
    `running_mean_divergence` and `coherence`, `lin` scalars, then fail generation until it does.
  - (c) For `alpha_energy`, declare a range that does not span zero, or define a placement centred on 1.0 (R-72),
    with REQ-PAY-093's proposal at the M5 gate.
  - (d) For `running_mean_divergence` and `coherence`, make their ranges a definition requirement (R-72): it gives
    each a formula and declares a range, with its citation, that does not span zero.
- **Applied meanwhile:** applied per R-369 in TASK-M1-21's PR (#208): option (a)'s ledger half. The four members cite
  §3.7's "The §3.8 entries" (`crates/ledger/src/reduction.rs`, `SECTION_3_7`), `alpha_area` and `alpha_energy` also
  REQ-PAY-093's proposal (R-71, R-418) until the M5 gate; line 488 and line 527 are unchanged. No numeric view draws
  a `QuadReduction` member in M1, `running_mean_divergence` and `coherence` included (`ledger::layout()` is the
  payload's), so no ramp follows from it yet.
- **Waits:** nothing in M1; TASK-M1-21's PR does not wait. REQ-PAY-093's confirmation at the M5 gate is where
  `alpha_energy`'s range is settled; it is named, not marked.

## RQ-280: The addressing channel convention names two channels per position and "grey" for an index, but not the third channel of TL = RB and centre = BG, which index, or how it maps to [0, 1] *(silence, R-72, REQ-COL-054; context: TASK-M7-01, TASK-M7-17, TASK-M7-18)*

*Found by TASK-M7-01, writing render GUI spec §16's standing gaps into colour_composition §9. Nothing is chosen.*

- **File, section:**
  - `docs/gui/principia_render_gui_spec.md` § "16. Open / next": "the addressing channel convention (UV=RG / TL=RB /
    centre=BG / index=grey)".
  - `docs/design/principia_colour_composition.md` § "6. Debug views as presets": "**UV view** = `colour =
    ctx.screen.uv → RG` (fragment addressing) or `ctx.quad.uv → RG` (structural addressing)".
  - `docs/design/principia_debug_tooling_plan.md` § "F. Structural views", the "UV passthrough" row: "raw post-flip
    `(u,v)` as a gradient (`u→red, v→green`), + quad-local `δ` as a second mode: … `u`'s in red and `v`'s in green,
    blue 0 (REQ-TOOL-153; R-72, written by TASK-M1-07; in exact arithmetic it draws `t`, as `ctx.quad.uv → RG` does,
    so the two agree unless the δ path is wrong)".
  - `docs/design/principia_colour_composition.md` § "3. The `ctx` contract", the quad lane: "`index`, `depth`, `tl`
    (slice coords), `centre` (slice coords), …"; the tile/sample lane: "`tile_index` (within quad), `sample_index`".
- **Silence:**
  - UV = RG's third channel is written: §F's δ mode draws blue 0 and agrees with `ctx.quad.uv → RG`, which holds
    only with that view's blue 0 too, so colour_composition §9.7 writes B = 0 for UV = RG (both sources, §6's one UV
    view). For TL = RB and centre = BG the passages name two channels; the third (G and R; 0, or something else) is
    not written. colour_composition §9.7 writes the pairs' order (`x` → first, `y` → second) by the pattern of
    `u → red, v → green`.
  - "index = grey" does not say which index: `ctx.quad.index`, `ctx.tile.tile_index` or `ctx.tile.sample_index`.
  - An index is an integer, and grey takes a value in [0, 1]; no mapping is written (divided by a count, a hash, or
    a cyclic modulus).
- **Options seen:**
  - (a) TL = RB's G and centre = BG's R are 0, as UV = RG's B is.
  - (b) "index" is `ctx.quad.index`, mapped by a stated normalisation (for example over the visible quads), or a
    hash to grey as `word`-hash's categorical view.
  - (c) One preset per index (quad, tile, sample), each with its own mapping.
- **Applied meanwhile:** nothing; colour_composition §9.7 writes only what the passages name.
- **Waits:** the addressing presets' payloads in TASK-M7-17's preset library, and any map preset that uses the
  convention (TASK-M7-18). TASK-M7-01 does not wait.

## RQ-281: Render GUI spec §11 names six debug groups, but the passages settle the membership of four; diagnostic's and profiling's, and where ROUNDTRIP, the agreement presets, the §B–§D payload field views and §E's live-shape and accumulator views go, are not written, and the telemetry doc says profiling is "not a debug mode" *(silence and possible conflict, R-72, REQ-COL-054; context: TASK-M7-01, TASK-M7-17)*

*Found by TASK-M7-01, writing render GUI spec §16's "the debug taxonomy" into colour_composition §9.6. Nothing is
chosen.*

- **File, section:**
  - `docs/gui/principia_render_gui_spec.md` § "11. Presets = whole graphs": "the debug groups — addressing /
    quad-structure / decoded-IC / validity / diagnostic / profiling — below".
  - `docs/design/principia_colour_composition.md` § "6. Debug views as presets": the §B–§E payload field views
    (`f_edrift`, `f_wordhash`, `f_state`), the §F `ctx.quad` presets (`s_depth`, `s_impurity`, `s_state`), and the §A
    views (UV, DECODE, ROUNDTRIP, "Agreement presets").
  - `docs/design/principia_debug_tooling_plan.md` § "E. Payload field views — `ICDescriptor` (64 B) & the live-state
    block": "*ICDescriptor views certify the **decoder**, independent of any integration.*" Its table also holds the
    "**live shape views** `u_mode`" and "accumulator views" rows, which are live state, not the `ICDescriptor`.
  - `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode":
    "`stage_ms` and the frame record are part of the render loop's contract, always present, with the *reporting*
    toggleable rather than the *measurement*."
- **Silence:** addressing (§6's UV views), quad-structure (§6's `ctx.quad` presets), decoded-IC (the decoded-IC payload
  quantities, §E's `ICDescriptor` views by §E's line above, and DECODE) and validity (§3's validity lane) follow from
  the passages, and colour_composition §9.6 writes them. No passage names a preset of the diagnostic or the
  profiling group, or places ROUNDTRIP, the agreement presets, the §B–§D payload field views (such as `f_edrift`,
  `f_wordhash` and `f_state`), or §E's live-shape and accumulator views.
- **Possible conflict:** a "profiling" group of locked debug colour presets beside §5.5's "Profiling is FIRST-CLASS, not
  a debug mode". The two may not disagree (§5.5 is about measurement always being on, not about how a view of it is
  filed), but no passage says what a profiling preset colours.
- **Options seen:**
  - (a) diagnostic holds the §B–§D payload field views, §E's live-shape and accumulator views, and the cross-checks
    (ROUNDTRIP, agreement); profiling holds views of per-sample cost (`total_substeps`, the substep-log proxy) and
    quad cache age.
  - (b) ROUNDTRIP and the agreement presets go in decoded-IC, as checks of the decode; diagnostic holds the §B–§D
    payload field views and §E's live-shape and accumulator views.
  - (c) The membership is the render-modes prototype's grouping, transcribed into a contract by a ruling (R-159).
- **Applied meanwhile:** nothing; colour_composition §9.6 writes the four groups the passages settle and points here.
- **Waits:** TASK-M7-17's preset library, for the diagnostic and profiling rows. TASK-M7-01 does not wait.

## RQ-282: "Gradient-unifies-the-ramp" may mean the GUI's gradient editor ("built-in LUT or custom stops") unifies §1.2's ramps, and colour_composition has no ramp of custom stops *(silence, R-72, REQ-COL-054; context: TASK-M7-01, TASK-M7-11)*

*Found by TASK-M7-01. Nothing is chosen.*

- **File, section:**
  - `docs/gui/principia_render_gui_spec.md` § "16. Open / next": "gradient-unifies-the-ramp".
  - `plan/tasks/M7/TASK-M7-01.md`, Deliverables: "`FieldRamp{gradient_magnitude}` vs the map post node (§1.2,
    render_contract Part 4)" — the reading colour_composition §9.4 writes.
  - `docs/gui/principia_render_gui_spec.md` § "9. Node inspector — morphing editors": "`colour` ← `scalar` |
    **gradient editor** (built-in LUT or custom stops, cyclic)"; "`brightness` ← any | **gradient / compaction
    editor** (greyscale + polarity)".
  - `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp": "**Ramp** (scalar → colour):
    `lut(name)`, `lerp(c0,c1)`, `diverging(c−,c0,c+)` (through a neutral), `bands(…)`".
- **Silence:** the item's name fits two readings. The task file's, written in colour_composition §9.4: gradient
  magnitude is a ramp over a derived field, distinct from the map's post-occupant gradient. The other: one "gradient" (a stop list) is the GUI's
  single form of §1.2's `lut`, `lerp` and `diverging`. Under the second, §1.2 has no ramp of custom stops: their
  interpolation space (OKLab or sRGB), the stops' positions, and how `cyclic` (a compaction in §1.2, a gradient
  flag in render GUI spec §9) applies are not written.
- **Options seen:**
  - (a) The task file's reading alone; render GUI spec §9's custom stops are a separate definition, written by the
    task that needs it (R-72).
  - (b) Both: §1.2 gains `stops([(t_i, c_i)], space)`, with `lut`, `lerp` and `diverging` as its named instances.
- **Applied meanwhile:** colour_composition §9.4 writes the task file's reading and points here.
- **Waits:** TASK-M7-11's graph model, for the gradient editor's ramp type. TASK-M7-01 does not wait.

## RQ-283: chart_reference §0.3 names `Σpᵢ = 0` "the test that catches a swap", but the full swap of the crossed factors keeps `Σpᵢ = 0` at every mass *(corpus error, REQ-DEC-019, PIT-9; context: TASK-M2-03)*

*Found by TASK-M2-03's review (physics review 5526305435 E1; qa review 5527243917). Nothing is chosen; TASK-M2-03's PR
does not wait.*

- **File, section:**
  - `docs/design/principia_chart_reference.md` § "0.3 Momentum — free Jacobi momenta", the "Transcription hazard"
    note (:85–88): "Transcribe it, then verify by asserting `Σpᵢ = 0` to machine precision — the test that catches a
    swap."
  - `plan/tasks/M2/TASK-M2-03.md` § "Acceptance tests", `decode_momentum`: "Σpᵢ = 0 to machine precision on fuzzed z
    with unequal masses (catches the crossed m₀/m₁ factors)"; `plan/requirements.yaml` REQ-DEC-019's acceptance
    detail (:383) repeats it.
- **Why it is wrong:** the map is `p₀ = −p_ρ − w₀p_λ`, `p₁ = p_ρ − w₁p_λ`, `p₂ = p_λ` with `w₀ = m₀/M₀₁`,
  `w₁ = m₁/M₀₁`. Swapping both factors (`p₀` taking `w₁`, `p₁` taking `w₀`, the positions' form) gives
  `Σp = −(w₁ + w₀)p_λ + p_λ = 0` at every mass. `Σpᵢ = 0` catches only a one-sided swap, so PIT-9's "draw unequal
  masses" is necessary but not sufficient.
- **What catches it:** the conjugacy of the map, `μ_ρ(p₁/m₁ − p₀/m₀) = p_ρ` (TASK-M2-03's `decode_momentum`, its
  `swapped_factors` control), or the kinetic-energy identity `Σ‖pᵢ‖²/2mᵢ = ‖p_ρ‖²/2μ_ρ + ‖p_λ‖²/2μ_λ` (qa's
  `qa_m2_03_momentum`). Both fail under the full swap at unequal masses.
- **Options seen:**
  - (a) Reword §0.3's note, and REQ-DEC-019's detail and the task line with it, to name the conjugacy check (or the
    kinetic-energy identity) as the test that catches the swap, `Σpᵢ = 0` as the one that catches a one-sided swap.
  - (b) Keep the note and add the conjugacy check beside it as a second required assertion.
- **Applied meanwhile:** TASK-M2-03's tests assert both `Σpᵢ = 0` and the conjugacy, with a control for each swap; no
  doc line is changed.
- **Waits:** nothing.

## RQ-286: profiler schema v1's frame record has no per-pixel substep field, so nothing records what the Profiler's substeps-per-pixel histogram draws *(silence, REQ-TOOL-098; context: TASK-M8-28, TASK-M6-28)*

*Found by TASK-M6-28's fix round (gui review 5529720235, finding 3 of PR #212), drawing the histogram on the mock.
Nothing is chosen.*

- **File, section:**
  - `docs/gui/principia_render_gui_spec.md` § "G5" › "Profiler", line 237: "a substeps-per-pixel histogram, with the
    cap marked (the long tail near close encounters is the GPU divergence);". GUI_DESIGN_NOTES § "04 Windows" (:69)
    lists it too, and REQ-TOOL-098 asks for it.
  - `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the
    sender", "The frame record" (:340–354): "**The frame record** is §2's, key for key, followed by the five stages'
    nested sections and the memory live at the frame's end"; its one substep key is line 346's
    "quads_computed  quads_reused  samples  substeps_total". § "2. What to record" (:46) reads "substeps_total     the
    honest cost measure; steps are not comparable across steppers": a frame's total only.
- **Silence:** the histogram needs a distribution of substeps over pixels, but the frame record carries only the
  frame's total, and no other field of schema v1 (header, stages, events) carries per-pixel substeps. The corpus
  does not say whether the histogram is a new frame-record field (its bins, their width or edges, and whether the cap
  is its own bin), a stage section's payload, or something the GUI reads elsewhere (`total_substeps` per pixel,
  render contract :384), nor what readback carries it (R-288 keeps `QuadReduction` unchanged).
- **Options seen:**
  - (a) Add a frame-record field, e.g. `substep_histogram: [u32; n]` with fixed bin edges up to `N_max` and the cap
    hits as the last bin, read back with the telemetry counters (R-288), a schema v1 change by ruling.
  - (b) Compute it on demand from the per-pixel `total_substeps` (a sanctioned pull), outside the trace, so a saved
    trace does not carry it.
  - (c) Drop the histogram from §G5 and REQ-TOOL-098.
- **Applied meanwhile:** applied per R-369 in TASK-M6-28's PR (#212): the mock's Profiler draws placeholder bins of
  4 substeps up to `N_max` 64 (`crates/gui/src/mock/profile.rs`, `SUBSTEPS_PER_PIXEL`), the cap marked at 64, with a
  note naming this entry; the real engine's side has no trace, so draws nothing.
- **Waits:** TASK-M8-28 (REQ-TOOL-098's histogram on real data). TASK-M6-28 does not wait.

## RQ-340: The compass's tilt and the decoder's basis tilt q₁ and q₂ toward one shared hidden axis, the contract tilts each toward its own target, and the human's nav-cube reference maps (τ₁, τ₂) jointly by an axis-angle map; which plane a tilt means differs between them *(conflict, physics, R-451; context: TASK-M6-33)*

*Found by R-451's port (16 Oct 2026), which the human asked to compare the three. Nothing is chosen; the decoder's
geometry is not changed, and the compass draws the decoder's mapping. For the human (physics: it changes which plane a
tilt means).*

- **File, section:**
  - `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)": "`q'(τ) = normalise( cos τ · q + sin τ ·
    d_target )        τ ∈ [−90°, +90°]`"; "The 8D has **6 directions orthogonal** to the current plane; the tilt selector
    offers those". `docs/design/principia_chart_reference.md` §1.1: "A 2-plane in 8D has 12 tilt axes (6 hidden
    dimensions × 2 basis vectors). Raw tilts **replace** rather than compose".
  - The code, `crates/gui/src/explore/manifold_view/mod.rs`, `View::basis`: `u₁ = cos τ₁ e_a + sin τ₁ e_d`,
    `u₂ = cos τ₂ e_b + sin τ₂ e_d` (one hidden axis `e_d`, `slice_axis`), then `q₁ = σ(cos γ u₁ + sin γ u₂)`,
    `q₂ = σ(cos γ u₂ − sin γ u₁)`, with no normalisation and no orthogonalisation; `crates/gui/src/explore/compass.rs`,
    `plane_frame`, draws the same in the frame `(e_a, e_b, e_d)`.
  - `docs/gui/reference/principia_nav_cube.html`, `planeSpec`: "axis-angle ("joystick") map — rotate the up-normal by
    angle m toward (τ₁,0,τ₂) … no gimbal lock (the naïve Euler form freezes q₂ at τ₁=±90°)", `n = (sin m · τ₁/m, cos m,
    sin m · τ₂/m)`, `m = √(τ₁² + τ₂²)`, toward two targets (Width = q₁'s, Depth = q₂'s).
- **The difference:** the code's two tilts share one target, so `u₁ · u₂ = sin τ₁ sin τ₂` (the basis is not orthogonal
  with both tilts non-zero, and the window's shape changes with it), and at `τ₁ = ±τ₂ = ±90°` the plane degenerates to
  the line `e_d`; the contract allows each vector its own target and normalises it; the reference's joint map never
  degenerates inside ±90° and moves the plane with both angles always.
- **Options seen:**
  - (a) Keep the code's mapping (one shared slice axis), and say so in the contract; the compass draws it (as applied).
  - (b) The contract's form as written: each of q₁, q₂ tilted toward its own chosen hidden direction, normalised, then
    orthonormalised; the compass's tilt labels become the reference's two targets.
  - (c) The reference's joint axis-angle map for the GUI's (τ₁, τ₂), written into the contract as the tilt pair's
    meaning.
- **Applied meanwhile:** the compass's cube draws the decoder's mapping (`View::basis` in the cube's frame), and its
  tilt mode names the one shared target (R-451 B1).
- **Waits:** nothing builds on the choice yet; TASK-M6-33 does not wait. A ruling for (b) or (c) changes the decoder's
  basis, which re-integrates every tilted view, and goes to a follow-up task.

## RQ-341: The physical compass views need the decoder's Jacobian with respect to z along all eight latent directions, into Jacobi or shape-sphere coordinates, and its Gram area factor; the corpus's J_D is the derivative with respect to the chart's (u, v) into the particle frame, not yet built, and no Gram factor is defined for it *(silence and conflict with the ruling's words, R-72, R-452; context: TASK-M6-35)*

*Found by R-452's port (16 Oct 2026). Nothing is chosen. For the human: R-452 says "use the existing J_D machinery"
and "the Gram-determinant area factor already defined for J_D", and neither exists in that form.*

- **File, section:**
  - `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision": "compute `J_D = ∂D/∂(u,v)` at the
    centre by **central differences in f64** (two decode evals per axis)"; § "Couplings to the contract", a: "The
    linearised object is the full chart→IC composite, not the decoder `D` alone", and b: "`|det J_D|` is the shared
    measure weight … `J_D = ∂physical/∂(u,v)`". `docs/contracts/principia_integrator_contract.md`: `(m, r, p)` "comes from
    `x₀ + J_D·δ`". It is built by REQ-DEC-033 (M6); `crates/kernel/src/stage/decode.rs` says the linearised path lands
    in M6, and nothing in `crates/` computes a Jacobian of the decode today.
  - `docs/design/principia_dd_generation_root.md` gives `√det(JᵀJ)` as "the log of the area element" for the links'
    forward Jacobians only; nothing defines `|det J_D|` for a non-square `J_D`, nor the Gram factor
    `√(|J q₁|²|J q₂|² − (J q₁ · J q₂)²)` that R-452 quotes.
- **Silence:** (1) the views' `J` is `∂(Jacobi or shape coordinates)/∂z`, applied to `q₁`, `q₂` (and, for the steer,
  RQ-344, its pseudo-inverse over all eight directions); the corpus's `J_D` has two columns (∂/∂u, ∂/∂v) and the
  particle frame's 15 rows. On an affine chart `J_D`'s columns are `2·(∂D/∂z)·q₁` and `2·(∂D/∂z)·q₂` through
  `(m, r, p)`, so the views' pushforward could be read off `J_D` up to that factor and the change of coordinates, but
  the steer cannot. (2) Which stage the views differentiate: before or after canonicalisation, the scale gauge and
  `η_E`. (3) The derivative's method and step (central differences in f64, as deep_zoom's, or analytic). (4) The Gram
  factor's definition and the factor of 2 from the chart's `(2s−1)`.
- **Options seen:**
  - (a) A definition requirement (R-72): a CPU, f64, central-difference Jacobian `∂x/∂z` (8 columns) of the decoder's
    output in the view's coordinates at a latent point, the views reading `J q₁`, `J q₂` from it, the Gram factor
    defined as R-452 writes it; a new requirement in M6 beside REQ-DEC-033, sharing its method.
  - (b) Read the views off REQ-DEC-033's `J_D` once built (two columns, through `(m, r, p)` to Jacobi or shape
    coordinates by the chain rule), and drop the physical steer or give it another construction.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199, REQ-GUI-200).

## RQ-342: The Jacobi view's eight coordinates are not eight degrees of freedom of the decoder's image: ρ̃ᵧ is identically 0 and the four position components carry two, the masses move none of them, and the momenta are not mass-weighted *(physics review, R-452; context: TASK-M6-35)*

*Found by R-452's port (16 Oct 2026), the physics review the human asked for before the set is built. Nothing is
chosen. For the physics reviewer, then the human (the set is the human's proposal).*

- **File, section:**
  - `decisions.md` § R-452 (the human's words, 3.2): "Jacobi view, 8 coordinates: the mass-weighted Jacobi vectors and
    their momenta, ρ̃ₓ, ρ̃ᵧ, λ̃ₓ, λ̃ᵧ, p_ρₓ, p_ρᵧ, p_λₓ, p_λᵧ, as the decoder writes them."
  - `docs/design/principia_dd_decoder.md` §3.2 and `docs/design/principia_chart_reference.md` §0.2: the canonical frame
    `ρ̃ = R̃ cos α · (1, 0)`, `λ̃ = R̃ sin α · (cos β, sin β)`, `R̃ = 1`; §0.3: `p_ρ = (q0, q1)`, `p_λ = (q2, q3)`,
    physical Jacobi momenta (`docs/notes/ic_inspector_scratchpad.md`: "*no* `√μ` unweighting").
  - `docs/contracts/principia_chart_decoder_contract.md` Part 1: the eight latent coordinates are "2 config ⊕ 4
    momentum ⊕ 2 mass", rotation and scale fixed.
- **The points:** (1) `ρ̃ᵧ ≡ 0` on the decoder's image, so every cell in its row and column is always 0; (2) `ρ̃ₓ, λ̃ₓ,
  λ̃ᵧ` carry the two configuration degrees of freedom (α, β) with `R̃ = 1`, so they are redundant; (3) `z_μ1`, `z_μ2` move
  none of the eight, so a plane tilted into the mass directions pushes forward degenerately (`J q = 0` for `q` in
  them) and the view cannot draw it; (4) the positions are mass-weighted and the momenta are not, so `Σ`-type
  comparisons mix weightings; (5) whether the decoder's output is read before or after the scale gauge and `η_E`.
- **Options seen:**
  - (a) Keep the eight as proposed; the view shows the dead row and the degenerate cases (readout "J q = 0: the plane
    moves only the masses").
  - (b) Eight coordinates that chart the image: e.g. `α, β` (or `ρ̃ₓ, λ̃` in polar form), the four momenta, and the two
    masses (`m₀/M`, `m₁/M`), each named as the decoder writes it.
  - (c) The momenta mass-weighted as the positions are (`p̃ = p/√μ`), so the eight share one weighting.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199).

## RQ-343: The Shape view's eight coordinates are redundant and incomplete: n̂ is three numbers for two degrees of freedom, R is 1 on the decoder's image, no doc defines ṅ or Ṙ, and the masses and the overall rotation rate are missing *(physics review, silence, R-72, R-452; context: TASK-M6-35)*

*Found by R-452's port (16 Oct 2026), the physics review the human asked for. Nothing is chosen. For the physics
reviewer, then the human (the set is the human's proposal).*

- **File, section:**
  - `decisions.md` § R-452 (the human's words, 3.2): "Shape view, 8 coordinates: the shape-sphere vector n̂ = (n₁, n₂,
    n₃), the hyperradius R, and their rates ṅ₁, ṅ₂, ṅ₃, Ṙ."
  - `docs/design/principia_chart_reference.md` § "3. Shape-sphere chart (θ, φ)", §3.1: `n = ((a−b)/I, 2p/I, 2q/I)`,
    `I = a + b`; the full reduced phase space is `T*S²`. The scale gauge fixes `I = 1` (`principia_dd_decoder.md` §3.5).
- **The points:** (1) `|n̂| = 1`, so `n₁, n₂, n₃` carry two degrees of freedom and their pushforward is constrained
  (`n̂ · ṅ = 0`); (2) `R = √I = 1` on the image, so its row is always 0; (3) no doc defines `ṅ` or `Ṙ` (from the
  momenta through which masses, in which frame); (4) the set has no mass coordinate and no overall rotation rate
  (`L_z`, the fibre of `T*S²`), so planes in those directions push forward degenerately; (5) the 2-to-1 cover of the
  shape sphere (chart_reference §3.3).
- **Options seen:**
  - (a) Keep the eight; define `ṅ`, `Ṙ` from the decoded state (`ṅ = dn/dt` along the flow at `t = 0`), and show the
    dead `R` row.
  - (b) A chart of the image: `θ, φ` (or two of `n̂`), their rates, `L_z` (or the fibre rate), `Ṙ` and the two
    masses.
  - (c) Drop the Shape view until the set is settled.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199).

## RQ-344: The physical steer's latent preimage q ∝ J_D⁺ e_i needs a Jacobian over all eight latent directions, a metric, a rank test and a rule for a coordinate outside the Jacobian's range; none is written *(physics review, silence, R-72, R-452; context: TASK-M6-35)*

*Found by R-452's port (16 Oct 2026), the physics review the human asked for on this construction. Nothing is
chosen. For the physics reviewer, then the human.*

- **File, section:** `decisions.md` § R-452 (the human's words, 3.4): "clicking physical cell (i, j) sets the slice
  plane to the latent preimage of that physical plane at z₀: q₁ ∝ J_D⁺ e_i, q₂ ∝ J_D⁺ e_j, Gram–Schmidt
  orthonormalised, with J_D⁺ the pseudo-inverse. If J_D is rank-deficient there, so the preimage is not a 2-plane,
  refuse the click and log why to the console." No doc has a pseudo-inverse or a latent preimage
  (`principia_inverse_encode_contract.md` inverts by closed-form blocks after canonicalisation and rejects a "smallest
  latent norm" argmin for invariant charts).
- **The points:** (1) it needs RQ-341's eight-column `∂x/∂z`, not the chart's two-column `J_D`; (2) the
  Moore–Penrose inverse assumes the Euclidean metric on `z` (and on the physical coordinates, whose units differ:
  positions and momenta); (3) by RQ-342 and RQ-343 the Jacobian is rank-deficient on the whole image (at most 6 of 8),
  so "rank-deficient there" needs reading as "the two preimages do not span a 2-plane", with a threshold (a singular
  value or the Gram factor of `J⁺e_i`, `J⁺e_j` below a tolerance); (4) where `e_i` is not in the Jacobian's range
  (`ρ̃ᵧ`, `R`), `J⁺e_i = 0` and the click is always refused, and where it is partly in range the steered plane's
  pushforward lights cell (i, j) below ±1, so R-452's test "lights that physical cell at ±1 (to tolerance)" holds only
  for `e_i`, `e_j` in the range; (5) the steered basis is a general 8D plane, not a preset with tilts, so how the
  Manifold view's sliders follow needs saying (as the Chart builder's Apply does, §G7, is the nearest).
- **Options seen:**
  - (a) As written, with the rank test "the smaller singular value of `[J⁺e_i, J⁺e_j]` above a tolerance", the test
    restricted to coordinates in the range, and the sliders following as after Chart builder's Apply.
  - (b) A weighted pseudo-inverse with stated metrics on `z` and on the physical coordinates.
  - (c) Only the latent steer (the Plücker view's); the physical cells are not clickable.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-200).
