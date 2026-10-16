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

## RQ-330: R-444 to R-448 change what merged M1 tasks built and what their goldens hold; which goldens, fixtures and tests are re-baselined, under what decision, and which gate confirms the new swatches and their evidence *(the human's question, R-444 to R-448, R-110, R-71, REQ-COL-002, REQ-COL-053, REQ-COL-062, REQ-COL-064, REQ-COL-067, REQ-COL-068, REQ-COL-104, REQ-COL-105; context: TASK-M2-32)*

*Filed at the human's request in their message of 16 Oct 2026 (R-444 quotes it): "Whether any task already in flight
or merged (M1-10's goldens, the outcome palette fixtures, REQ-COL-053/064 tests) needs re-baselining, and which
milestone gate confirms the new swatches." Nothing is chosen. For the human: yes (a gate and a re-baseline, R-110).*

- **File, section:**
  - `docs/design/principia_colour_composition.md` § "1.4 Categorical colour-assignment — the outcome-state default
    palette" and § "4.1 Backbone & `Option` occupants", as R-444 to R-448 leave them.
  - `plan/WORKFLOW.md` § "Conventions: crates and runners (R-146)", the golden-image row: "no baseline is re-baselined
    without a gate decision (R-110)". § "Human checkpoints: the milestone gates": every calibration (R-71) is confirmed
    by the human at its milestone's gate.
- **What changes, by owner** (merged unless marked):
  - **Goldens** (`fixtures/golden/`, rows in `BASELINES.md`): `m1-outcome/outcome` and `m1-outcome/outcome_edited`
    (TASK-M1-10; last re-rendered by R-434, confirmed by R-435) draw every class, the white "degenerate" and the orange
    t = 0 rows, the triple swatches `#D6A1FF` and `#000097`, and `running` in the stipple; `m1-synthetic/known_escaper`
    (TASK-M1-14; R-434, R-435) draws a triple ejection in navy and a running sample in the stipple. Each changes under
    R-444 to R-446. `m1-outcome/state_view`, `detail_view`, `m1-synthetic/set_state` and `forced_failure` are debug
    views and do not change. No other fixture names a removed class or navy (`fixtures/cvd/goldens.json` holds the
    simulation matrices, not the palette).
  - **Test data asserted against** (`crates/render/tests/`): `outcome_state.rs` (TASK-M1-10, TASK-M1-17: the eleven
    swatches, navy, a triple collision at `t = 0` in orange); `not_yet_grey.rs` (TASK-M1-09: the grey's separation from
    "degenerate" `#ECECF0` and "collision @ t=0"); `colour_independence.rs` (TASK-M1-14: the eleven classes, `K = 11`);
    `support/palettes.rs` (TASK-M1-10, TASK-M1-13, TASK-M7-20: the outcome entries every hatch, stipple and
    colour-vision measurement runs over, REQ-COL-055, REQ-COL-061 and REQ-COL-064 among them); and qa's files, which only
    qa changes (R-290): `qa_TASK-M1-03.rs`, `qa_TASK-M1-10.rs` (white, navy, "collision at start"),
    `qa_TASK-M1-14_independence.rs` (`K = 11`, the orange) and `qa_TASK-M1-18_r434.rs` (navy against the dot).
  - **Code**: `crates/render/src/colour/outcome.rs` and `crates/render/shaders/wgsl/frag/colour/outcome_state.wgsl`
    (TASK-M1-10, TASK-M1-17), `crates/validation/src/golden_scene.rs`'s outcome scenes, and the GUI mock's Inspector
    labels "collision @ t=0" and "degenerate" (`crates/gui/src/tests/mock_content.rs`, TASK-M6-27). The Legend reads
    `SWATCHES` (`crates/engine/src/contract/legend.rs`, TASK-M6-27, TASK-M6-29), so it follows the code.
  - **Requirements claimed by merged tasks whose statements change**: REQ-COL-002 and REQ-COL-062 (TASK-M1-10),
    REQ-COL-053 (TASK-M1-09), REQ-COL-064 and REQ-TOOL-162 (TASK-M1-17), REQ-COL-067 and REQ-COL-068 (TASK-M1-14). They
    stay closed by those tasks and in the M1 gate's list; none is reopened. `main` meets the changed statements once
    TASK-M2-32 merges.
  - **In flight**: PR #223 (the Chazy rulings) quotes the old §4.1 text (`K = 11`, "degenerate", "collision @ t=0")
    as context and offers, in its entry on what the Chazy stains draw for `decode_failed`, "the outcome palette's
    degenerate white `#ECECF0`" as an option, which R-446 removes; it draws B as "the event palette's bounded" `#141418`,
    unchanged. PR #225 (TASK-M2-09) adds `DEGENERATE(INVARIANT_INFEASIBLE)`, `decode_failed`/3, which the outcome view
    draws black or, where the chart hatches it, with the hatch (R-446). PR #224 (TASK-M6-30) and PR #219 (TASK-M7-05) do
    not use the outcome palette's classes.
- **Options seen, the re-baseline:**
  - (a) The swatches are the human's (R-444 to R-446), as R-434's dot was, so TASK-M2-32 re-renders the three goldens
    under those rulings, each `BASELINES.md` row leading with the ruling, confirmed at it (as R-434's seven were).
  - (b) TASK-M2-32 re-renders them as proposals, marked for the gate the human names below, and they are confirmed
    there (R-110).
- **Options seen, the gate that confirms the new swatches' evidence** (REQ-COL-104, REQ-COL-105, both M2, calibration):
  - (a) The M2 gate, TASK-M2-32's milestone and the requirements' as filed.
  - (b) None: R-444 to R-447 are the confirmation, and the evidence is checked in review, so REQ-COL-104 and
    REQ-COL-105 become ordinary requirements (`kind` cleared).
  - (c) The M7 gate, with the other palette calibrations (REQ-COL-061's colour-vision check, which runs over the same
    entries, and PR #223's Chazy palettes, if it merges), the requirements moved to M7.
- **Options seen, the M1 gate's record:** (i) the M1 gate stays passed, and the changed M1 requirements are re-checked
  as "every earlier gate still green" at the next gate; (ii) the M1 gate is re-run once TASK-M2-32 has merged.
- **Applied meanwhile:** REQ-COL-104 and REQ-COL-105 are filed in M2, TASK-M2-32's milestone. TASK-M2-32 waits on this
  entry for its goldens' `BASELINES.md` decision only; its code and tests do not wait.
- **Waits:** the `BASELINES.md` rows of TASK-M2-32's re-rendered goldens; REQ-COL-104's and REQ-COL-105's gate.

## RQ-331: the sRGB outcome palette's closest pair is 0.175 (triple collision against escape body 1), not the 0.185 R-444's message gives, because the orange stays as triple collision *(measurement against a ruled figure, R-445, R-71, REQ-COL-104; context: TASK-M2-32)*

*Found porting R-444 to R-448, measuring as REQ-COL-055 measures. Nothing is chosen; no swatch is changed. For the
human: yes (a calibration's evidence, R-71).*

- **File, section:**
  - `decisions.md` § "R-444 — …", the human's message, "What to change": "the sRGB palette's closest pair rises from
    0.175 to 0.185 with the t = 0 class gone (collision 0–2 / escape 2)".
  - The same message, item 2: "Its orange #F29620 is reused for triple collision."
  - `docs/design/principia_colour_composition.md` § "1.4 …", the triple outcomes' paragraph, where both values are
    recorded.
- **Measured** (Euclidean OKLab distance from 8-bit sRGB through dd_colouring §3.1's transforms; the classes named as
  shown, R-449, the human's "collision 0–2 / escape 2" being collision 1–3 / escape body 3): in the nine-class palette
  of R-444, the closest pair is triple collision `#F29620` against escape body 1 `#F0DE32`, **0.1751**; next,
  collision 1–3 against escape body 3, **0.1847**; then collision 2–3 against triple ejection, 0.1926. The old
  palette's 0.175 was collision @ t=0 against escape body 1, the same two colours. 0.185 is the closest pair only with the orange left
  out of the palette.
- **Options seen:**
  - (a) Record 0.175 (triple collision / escape body 1) as the palette's closest pair, the swatches as ruled.
  - (b) Change a swatch so that the closest pair is 0.185 or more (for instance triple collision's orange), which is a
    new ruling.
- **Applied meanwhile:** both figures are recorded, the human's and the measured; REQ-COL-104's evidence gives both.
- **Waits:** REQ-COL-104's confirmation (RQ-330's gate).

## RQ-332: R-447's C = 0.123 is in gamut at the eight listed hues but is not "the largest chroma in gamut at every hue at that L" (0.1224), and three of its swatches differ by one or two 8-bit steps from (0.72, 0.123, hue) *(measurement against a ruled figure, R-447, R-71, REQ-COL-103, REQ-COL-105; context: TASK-M2-32)*

*Found porting R-447, measuring with dd_colouring §3.1's transforms. Nothing is chosen; no swatch is changed. For the
human: yes (a calibration's evidence, R-71).*

- **File, section:**
  - `decisions.md` § "R-447 — …", the human's words: "each chromatic class keeps its own OKLab hue at L = 0.72 and C
    = 0.123 (the largest chroma in gamut at every hue at that L)", and the table's swatches.
  - `docs/design/principia_colour_composition.md` § "1.4 …", the variant's paragraph, where both values are recorded.
- **Measured:**
  - The largest chroma in the sRGB gamut at every hue at L = 0.72 is **0.1224**, at a hue near 199.8°: at C = 0.123 a
    hue there is out of gamut. At each of the eight listed hues C = 0.123 is in gamut; the narrowest is escape body 3's 208° (R-449's name),
    whose largest in-gamut chroma is 0.1238.
  - The swatches as ruled measure L 0.719 to 0.721, C 0.122 to 0.124, each hue within 0.5° of its row. Rounding
    (0.72, 0.123, hue) to 8 bits gives the ruled swatch for five rows and, for three, `#6DB976` (ruled `#6EB976`),
    `#D9934A` (`#D99349`) and `#07B9CD` (`#09B9CD`).
  - The variant's closest pair is 0.063 (escape body 2 / triple ejection, R-449's names); the human said it would be closer than the sRGB
    palette by construction, and it is. Recorded, not a departure.
- **Options seen:**
  - (a) Keep C = 0.123 and the swatches as ruled; the parenthesis reads "in gamut at every listed hue", and the
    swatches are the ruled hex values, not a computed rounding.
  - (b) Take C = 0.1224 (in gamut at every hue) and recompute the eight swatches.
  - (c) Keep C = 0.123 and replace the three swatches with the computed rounding.
- **Applied meanwhile:** the ruled values are transcribed, the measured beside them; REQ-COL-103 pins the ruled hex
  values and REQ-COL-105's evidence gives the measurements.
- **Waits:** REQ-COL-105's confirmation (RQ-330's gate). TASK-M2-32 builds the ruled swatches meanwhile.

## RQ-333: R-446 sends the η_E tags to "the η_E debug view", which the corpus does not define *(silence, R-446, R-441, R-72, REQ-COL-101; context: TASK-M2-32)*

*Found porting R-446. Nothing is chosen. For the human: no; decidable per R-369, unless the human meant a new view.*

- **File, section:**
  - `decisions.md` § "R-446 — …", the human's words: "it now draws black in the outcome view, and the η_E debug view
    is where the tags show".
  - `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)":
    the `state` view draws `decode_failed` in its `dbg_cat` colour, and the `detail` view its failure category,
    `decode_failed`'s codes 0 to 3 (payload §2). No view is named for `η_E`.
  - `docs/design/principia_dd_decoder.md` § "3.7 Energy normalisation (optional; keep-or-drop is decision B9)" (the tags `ENERGY_UNSCALABLE` and
    `ENERGY_INFEASIBLE`); the payload records each as `decode_failed` with a `detail` code, so no stored field tells
    the two tags apart from each other or from `M01_TINY` beyond that code.
- **Silence:** which debug view "the η_E debug view" is, and whether it must tell `ENERGY_UNSCALABLE`,
  `ENERGY_INFEASIBLE` and the mass block's tags apart, which the payload's `detail` alone may not.
- **Options seen:**
  - (a) The existing `state` and `detail` views are where the tags show; nothing new is built.
  - (b) A decoded-IC debug preset (fragment-side, from `ctx.chart.z`, as DECODE is, R-75) that re-runs the decode's
    `η_E` branch and draws each tag in its own colour.
  - (c) A further `detail` code for the energy tags, a payload change by ruling.
- **Applied meanwhile:** render_gui_spec §G6's reworded line names the human's view and says the `state` and `detail`
  views show the failure today, pointing here.
- **Waits:** nothing built; REQ-COL-101's outcome-view behaviour does not depend on it.

## RQ-334: the shape sphere's Euler landmarks `E₀, E₁, E₂` have no defined index, so R-449 cannot say how they are shown *(silence, R-449, R-14, R-50, R-72; context: TASK-M8-11, which draws the landmarks)*

*Found porting R-449. For the human: no; decidable per R-369 (naming).*

- **File, section:**
  - `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep,
    ratified)": the table's symbols "$BC_{01}, BC_{12}, BC_{20}$" and "$E_0, E_1, E_2$", and "$\hat{\mathbf e}_j =
    -\hat{\mathbf b}_j$ (equal masses)", where $\hat{\mathbf b}$ is indexed by two bodies and $\hat{\mathbf e}$ by one
    index; "The table's labels are 0-based (R-22)".
  - `decisions.md` § "R-449 — …": "Everything a person reads is 1-based … GUI labels"; "A pair is always named by its
    bodies, smaller first."
- **Silence:** `BC₀₁` names a pair by its bodies, so it is shown `BC₁₂` (applied in dd_integrator §3.7). `E_j`'s index
  is not defined: it may be the body in the middle of the collinear configuration (the usual reading, the antipode of
  the other two bodies' collision at equal masses), or the antipode's pair id, which R-449 forbids showing.
- **Options seen:**
  - (a) Define `E_j` as the configuration with body `j` in the middle, and show it `E_{j+1}`.
  - (b) Name each Euler landmark by its two outer bodies, as the collision it is antipodal to (`E₁₂` opposite `BC₁₂`).
- **Applied meanwhile:** nothing built; TASK-M8-11, which draws the landmarks' labels, writes the definition (R-72)
  and its reviewers decide per R-369.
- **Waits:** the overlay's landmark labels only.

## RQ-335: R-449's shown body numbers are not the Burrau literature's: at `ν = 1/2` the shown body 1 is the mass-5 body *(possible confusion, R-449, R-22, R-73, REQ-SYS-016; context: TASK-M2-33)*

*Found porting R-449. For the human: no; decidable per R-369 (R-449's table is explicit), recorded so that it is
seen.*

- **File, section:**
  - `docs/design/principia_chart_reference.md` § "4.2 Positions and masses — Burrau convention": "`m0 = c/(a+b+c)` …
    `(m0,m1,m2) = (5,4,3)/12`"; the indexing note: "The Burrau family is conventionally written 1-based (bodies 1–3,
    inner pair 1–2), and §4.2 translates it."
  - `decisions.md` § "R-449 — …": "body 0 / 1 / 2 | body 1 / 2 / 3".
- **The point:** R-449 shows internal body `k` as body `k + 1`. In the Burrau family internal body 0 has the mass
  opposite the hypotenuse, 5/12 at the classical point; in the usual literature labelling of the Pythagorean problem
  (masses 3, 4, 5 as bodies 1, 2, 3) that body is body 3. A reader comparing "body 1 escapes" with the literature reads
  a different body.
- **Options seen:**
  - (a) R-449's map as ruled, everywhere; the Burrau presets' legends and the Inspector show each body's mass beside its
    number.
  - (b) A Burrau-only relabelling, which R-449's "one conversion function" does not allow without a ruling.
- **Applied meanwhile:** (a)'s first half, R-449 as ruled; chart_reference's indexing note says the shown numbers are
  not the literature's.
- **Waits:** nothing.

## RQ-336: R-449 names `.raw` and `.prnq` dumps, which the live corpus does not define *(silence, R-449, R-159, REQ-GEN-036; context: TASK-M2-33)*

*Found porting R-449. For the human: no; decidable per R-369.*

- **File, section:**
  - `decisions.md` § "R-449 — …", item 5: "Machine-readable exports (CSV columns, JSON fields, .raw and .prnq dumps)
    stay 0-based, and their headers or schema say so explicitly".
  - `.prnq` appears only in `docs/experiments/briefs/principia_ARCHIVE_brief_structure_criterion.md` (prin-rs's tree
    dumps, a record) and `workbench/`; no live contract defines a `.raw` or `.prnq` format.
- **Silence:** which dumps of this project the two names mean.
- **Applied meanwhile:** REQ-GEN-036 covers every machine-readable export the corpus defines (the export decoder of
  generation root seam 13, the Measure tool's CSV) and any dump defined later, `.raw` and `.prnq` included when they
  are; nothing more is built for them now.
- **Waits:** nothing.
