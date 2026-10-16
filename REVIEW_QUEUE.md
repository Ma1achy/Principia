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

## RQ-310: `I_max` and the "when decided" latch want descriptor bits 10–15, which payload §2 holds reserved and "never opportunistically reused", beside R-37's departed bit, whose place is open *(conflict and budget, R-442, R-37, R-36, REQ-PAY-105, REQ-PAY-106, REQ-COL-082, REQ-VAL-055; context: TASK-M7-35, TASK-M7-40)*

*Found porting the Chazy stain spec (R-442). Nothing is chosen. **For the human: yes**, as the human asked ("Two need
me: the descriptor-bit budget …").*

- **File, section:**
  - `docs/design/principia_dd_chazy_final_motion.md` § "6. The latch: `I_max`", transcribing spec 02 §3: "A
    log-quantised value fits the reserved descriptor bits 10–14 (32 half-octave levels up to 2¹⁶), leaving bit 15 for
    the open departure flag. That is a versioned format change and needs a ruling (`04`)."
  - Spec 04 §3 item 1: "**Descriptor bits.** `I_max` in reserved bits 10–14 claims space next to the open departure
    flag (bit 15). The "when decided" view (`03` §7.1) needs a second latch and more bits. Is that budget available,
    and is it worth it?"
  - Spec 03 §7.1 (drill-down § "10.8.1 When it was decided"): "a second forward-only latch, the time the current
    coarse class was first committed, reset whenever it changes. Same shape as `I_max`, and it needs its own storage
    bits. Without that latch this view is offline only."
  - `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest
    reserved", line 226: "*reserved* | 10–15 | 6 | … Reserved, decode as zero, never opportunistically reused"; line 228:
    `last_symbol`'s bits 8–9 were "a deliberate, versioned assignment — a binary-format version bump … (automatic: the
    version is the ledger's content hash, R-36)".
  - `open-questions.md` item 9, line 71: "**Still open:** the value of `δ_dep` (by measurement), and where the
    per-sample "departed" bit lives" (R-37; carried by REQ-VAL-055 and REQ-PAY-056).
- **Conflict:** the spec spends five of the six reserved bits on `I_max` and names bit 15 for the departed bit, whose
  placement R-37 left open; payload §2 holds the six reserved. The "when decided" latch needs more storage that the
  descriptor does not have.
- **Options seen:**
  - (a) Spend bits 10–14 on `I_max` (log, half-octave, 5 bits), a versioned assignment as `last_symbol`'s was; bit 15
    for the departed bit; "when decided" offline only.
  - (b) As (a), and give the "when decided" latch storage outside the descriptor (a new `SimState` member, a size
    change with its memory cost), so the view is live.
  - (c) Keep the descriptor's bits reserved and store `I_max` elsewhere (a new `SimState` member), at its own memory
    cost.
  - (d) Do not store `I_max`: OS-candidate is then computed from no history (it never fires), or dropped.
- **Waits:** REQ-PAY-105's storage and REQ-PAY-106 (`rq: RQ-310`), TASK-M7-35's persisting the latch, and REQ-COL-082
  (TASK-M7-40). The local, per-step latch and the classifier's tests do not wait.

## RQ-311: the default seven-class final-motion palette sits below τ within itself (H / HP 0.083; HP / P 0.002 under deuteranopia); keep it, or move the default to Okabe–Ito *(decision at the gate, R-442, R-132, R-429, REQ-COL-075, REQ-COL-086, REQ-COL-087; context: TASK-M7-37, the M7 gate)*

*Found porting the Chazy stain spec (R-442). Nothing is chosen. **For the human: yes**, as the human asked ("whether the
default seven-class palette should stay below τ (04 §2.1) or move to Okabe–Ito").*

- **File, section:**
  - `docs/design/principia_dd_chazy_final_motion.md` § "11.2.1 Palette separation, measured", transcribing spec 04
    §2.1: "**The seven-class palette is under τ by design.** H, HP and P are steps of one cool family, so H and HP sit
    0.083 apart, and under deuteranopia HP and P collapse to the same colour. That is the cost of the family logic. The
    Okabe–Ito variant (`03` §3) is the colour-vision-safe alternative. Whether the default should change is a decision
    for the gate."
  - R-442, ruling 7: "The seven-class palette in 03 §1 is the default."
  - `decisions.md` § "R-429 — …": "τ is the OKLab distance from `#FF00FF` to `#E034C6`, 0.101", the distance R-132
    ruled a collision (REQ-COL-055, REQ-COL-061).
- **Context:** no corpus rule holds a palette's own entries τ apart (R-442's note); the seven-class palette clears
  every style unsimulated (0.137). The question is the human's choice of default, not a corpus conflict. Neither
  candidate clears τ within itself: the seven-class family's tightest pair is 0.083 unsimulated and 0.002 under
  deuteranopia (HP / P); Okabe–Ito's is 0.041 everywhere (P / collision).
- **Options seen:**
  - (a) Keep the seven-class palette as the default (R-442 as given), the shortfall recorded and asserted by
    REQ-COL-086; Okabe–Ito one click away.
  - (b) Make the Okabe–Ito variant (drill-down § "10.4 Final motion: Okabe–Ito") the default, the seven-class family
    a preset. Its cost, measured in the review of PR #223 (drill-down § "11.2.1 Palette separation, measured"): with
    collision kept magenta, Okabe–Ito's P `#CC79A7` sits 0.041 from collision `#DA85B6`, unsimulated and under each
    simulation, its tightest pair; without collision its seven clear 0.170 unsimulated and at least 0.079 simulated.
  - (c) Keep the family logic but respace H, HP and P (and re-measure) until they clear τ unsimulated, at the cost of
    the shared lightness.
- **Waits:** REQ-COL-087's confirmation at the M7 gate (`rq: RQ-311`). TASK-M7-37 builds R-442's default meanwhile and
  does not wait.

## RQ-312: REQ-GUI-076 bans "Chazy subtitles" from the dev GUI, which could be read as banning the Chazy stains R-442 adds *(scope, R-442, REQ-GUI-076, REQ-COL-075, REQ-GUI-185; context: TASK-M8-19, TASK-M8-45)*

*Found porting the Chazy stain spec (R-442). Nothing is chosen. For the human: no; decidable per R-369 (the scope of a
requirement's wording).*

- **File, section:**
  - `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere", line 69: "**Not in the dev GUI:**
    Chazy subtitles, and the object-based stain desk." `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold
    everywhere", line 17, the same.
  - REQ-GUI-076: "The dev GUI must not include Chazy subtitles or the object-based stain desk …"; verify: "no Chazy
    subtitle or stain-desk code path exists" (TASK-M8-19).
  - Spec 04 §3 item 4: "**REQ-GUI-076** bans "Chazy subtitles" from the dev GUI. That is a different feature from a
    Chazy stain. Record a ruling so the ban is not read as covering this."
- **Silence:** the corpus does not say what "Chazy subtitles" were; read broadly, REQ-GUI-076's check ("no Chazy
  subtitle … code path exists") would flag the Chazy fields, stains and legends.
- **Options seen:**
  - (a) Record that the ban covers subtitles only (a caption naming the final-motion class over the figure), not the
    Chazy fields, stains or legends; REQ-GUI-076 gains a note, its check scoped to subtitles.
  - (b) Reword REQ-GUI-076 and render_gui_spec §G1 to say "Chazy subtitles (captions over the figure)", with the same
    effect.
  - (c) Read the ban as covering any Chazy surface in the dev GUI, so the stains are browser-only (contradicting
    R-442's request for stains).
- **Waits:** REQ-GUI-185, REQ-GUI-186 and REQ-GUI-187 (`rq: RQ-312`), the Chazy legends and the three panels the ban
  could be read to cover, so TASK-M8-45 opens no PR before this is decided. REQ-GUI-076 carries no `rq:` (TASK-M8-19
  does not wait).

## RQ-313: the Chazy HE gate and Principia's escape detector are different tests, so a pixel can read "escape" in the event stain and excursion or B in the Chazy stain at the same playhead *(design, R-442, R-29, R-61, R-103, REQ-EVT-027; context: TASK-M7-36)*

*Found porting the Chazy stain spec (R-442). Nothing is chosen. For the human: no (per R-369), unless the choice is
read as physics.*

- **File, section:**
  - `docs/design/principia_dd_chazy_final_motion.md` § "9. Gaps between the prototype and Principia", item 3,
    transcribing spec 02 §6.3: "Principia's escape event uses its own rule and stops the run at fire. The classifier's
    HE gate (hierarchy ratio > 5, receding, outer `K/U > 1 + ε`) is not the same test. A pixel can read "escape" in the
    event stain and "excursion" or B here at the same playhead. Decide whether that is acceptable (they answer different
    questions) or whether one should gate the other."
  - Spec 04 §3 item 2: "**Escape detector alignment.** … Acceptable, or should one gate the other?"
  - `decisions.md` § "R-103 — Escape ends the production loop; …"; the escape rule, `principia_01_pitfalls.md` §
    "2.2 The criterion" and R-29 (closure over the window and `E_rel > 0`, no receding test).
  - Spec 02 §4 (drill-down § "7. Validity and styles"): "**Escape-terminated**: the production loop stops at escape
    fire, so the class freezes at the fire-time state."
- **Options seen:**
  - (a) Accept it: the two answer different questions; the legend help says so.
  - (b) An escape-terminated sample reads at least HE (or PE) in the Chazy fields from fire on: the event gates the
    class.
  - (c) The Chazy HE gate gates the escape event, a change to R-29's rule that would need its own ruling.
- **Waits:** REQ-EVT-027 (`rq: RQ-313`), TASK-M7-36's escape-freeze test. The rest of TASK-M7-36 does not wait.

## RQ-314: the f32 disagreement rate near `K = U` is unmeasured, and the spec's "Tier N" for a categorical label sits uneasily with the parity contract's tiers *(measurement and terminology, R-442, R-85, REQ-VAL-183; context: TASK-M7-39)*

*Found porting the Chazy stain spec (R-442). Nothing is chosen. For the human: no. The measurement is REQ-VAL-183's,
carried by TASK-M7-39; this entry holds how the result is read.*

- **File, section:**
  - Spec 04 §3 item 3: "**f32 parity.** Measure the disagreement rate at f32 near the `K = U` sign changes before
    setting a tolerance."
  - Spec 02 §5 (drill-down § "8. Parity"): "It is **Tier N**: CPU and GPU agree to tolerance, not bit for bit. A pixel
    within ulps of a sign change may differ between backends. The parity test measures that disagreement rate rather
    than asserting zero."
  - prin-rs PR #32, `results/chazy/README.md` §6: "it is Tier N (numerical), not Tier B: CPU and GPU agree to
    tolerance, and pixels within the band of a sign change read as the marginal labels rather than as confident ones."
  - `docs/contracts/principia_parity_contract.md` § "Tier N — tight numerical (checkable because nothing amplifies)",
    line 110, lists continuous quantities compared elementwise; the scope table, line 103: "a **label on a real
    trajectory** | **none — it will differ** | reported by the inspector, not suppressed"; Tier B's comparison-only rule:
    "A branch decision derived from a runtime transcendental **cannot** be made bit-exact across backends".
- **Silence and tension:** a label is not "within tolerance" of another; the classifier's branches read divisions and
  square roots (§3.1's `K`, `U`, `d`), so on identical inputs they can fork, which Tier B excludes and Tier N does not
  describe. The PR's sentence that pixels near a sign change "read as the marginal labels" holds for the parabolic band
  but not for the pair's binding, which is a sign (drill-down § "3.4 Two choices that are physics, not tuning"); the
  spec does not repeat it.
- **Options seen:**
  - (a) Record the class as "a label", reported not asserted (the scope table's row), with REQ-VAL-183's disagreement
    rate and its band check the gate.
  - (b) Treat the label as Tier N with a tolerance on the disagreement rate, set by REQ-VAL-183's measurement (a
    calibration).
  - (c) Make the class's branch inputs comparison-only (Tier B's rule) where they can be, and report the rest.
- **Waits:** REQ-VAL-183 (`rq: RQ-314`), TASK-M7-39's threshold. The measurement itself does not wait.

## RQ-315: OS-candidate and B-stable are labels, not predictions; the spec proposes shipping them as provisional and saying so in the legend help *(design, R-442, REQ-PAY-099, REQ-GUI-185; context: TASK-M7-35, TASK-M8-45)*

*Found porting the Chazy stain spec (R-442). Nothing is chosen. For the human: no; decidable per R-369.*

- **File, section:**
  - Spec 04 §3 item 5: "**OS-candidate and B-stable** are labels, not predictions: OS-candidate is unvalidated and
    B-stable has never fired on random starts. Ship them as provisional and say so in the legend help."
  - Drill-down § "4.2 Where it is not perfect": "**OS-candidate is not validated.** 2–3 per seed later escaped as HE";
    "**B-stable never fired** on random starts" (prin-rs PR #32).
- **Options seen:**
  - (a) Ship both, marked provisional in the status legend's help, as the spec proposes.
  - (b) Ship both unmarked.
  - (c) Withhold OS-candidate (reading B-so-far) until it is validated; keep B-stable, which a fixture covers.
- **Waits:** REQ-GUI-185's legend help (`rq: RQ-315`), TASK-M8-45. The classifier (TASK-M7-35) does not wait: it
  computes both labels under every option.

## RQ-316: where the Chazy tasks land: after the event machinery (M3) and the composition backbone (M7) *(placement, R-442, R-415; context: TASK-M7-35 to TASK-M7-40, TASK-M8-45)*

*Found porting the Chazy stain spec (R-442). For the human: no; decided per R-369 meanwhile.*

- **File, section:**
  - Spec 04 §3 item 6: "**Where it lands.** Needs the event machinery (M3) and the composition backbone (M7)."
  - The human's request, item 5: "Create tasks for the implementation, sequenced after the event machinery (M3) and
    the composition backbone (M7); place them in whichever milestone fits once those land."
- **Options seen:**
  - (a) M7 for the classifier, fields, stains and parity, M8 for the legends, each depending on the M3, M4 and M7 tasks
    it uses.
  - (b) All in M8, after the M7 gate.
  - (c) A milestone of its own after M7.
- **Applied meanwhile:** (a), per R-369: TASK-M7-35 to TASK-M7-40 in M7, TASK-M8-45 in M8. The calibrations are
  confirmed at the M7 gate.
- **Waits:** nothing; this records the placement for the human's veto.

## RQ-317: B's swatch `#141418` has OKLab chroma ≈ 0.008, but the spec's B rule and test 05 §2.5 say it has none, so that Replace-L turns it neutral grey *(conflict within the ruled spec, R-442, R-77, REQ-COL-074; context: TASK-M7-38)*

*Found porting the Chazy stain spec (R-442), measuring the swatch. Nothing is chosen. For the human: only if option (c)
is wanted, since the swatch is a ruling.*

- **File, section:**
  - R-442, ruling 3: "B is black #141418".
  - Spec 03, "Two rules hold in every stain" (drill-down § "10.1 The two rules that hold in every stain"): "**B is black
    `#141418`**, the same swatch as the event palette's "bounded". It has no hue (a = b = 0 in OKLab), so under
    Replace-L it becomes neutral grey at the brightness field's lightness".
  - Spec 05 §2.5: "**B has no hue.** The B swatch has OKLab a = b = 0 to within quantisation, so Replace-L gives
    neutral grey. Control: a chromatic swatch fails the same check."
  - Measured here, with dd_colouring §3.1's transforms: `#141418` is OKLab L 0.193, a 0.0022, b −0.0077, chroma 0.0080.
    A neutral 8-bit grey has a = b = 0 to rounding; `#141418`'s blue channel is four steps above its red and green.
- **Conflict:** under Replace-L at a light `L` (§10.5.1's transient, `L` near 0.9) B keeps a faint blue cast; the test
  as written fails unless "within quantisation" admits chroma 0.008.
- **Options seen:**
  - (a) Keep `#141418` (R-442); the stain takes B's hue as it is, and test 05 §2.5 asserts chroma ≤ a stated bound
    (0.008 measured) with the control's chroma well above it.
  - (b) Keep `#141418` as the flat swatch, and have the Chazy stains zero B's `a` and `b` under Replace-L, so that B is
    exactly neutral there.
  - (c) Change B to a neutral near-black (e.g. `#151515`), which changes R-442's swatch and the outcome palette's
    bounded link: the human's.
- **Waits:** REQ-COL-074 (`rq: RQ-317`), TASK-M7-38's B test. The flat stains (TASK-M7-37) draw `#141418` meanwhile.

## RQ-318: the spec's rule puts B-stable before transient; prin-rs PR #32's `classify` tests transient first; they differ for a Mardling–Aarseth-stable hierarchy with E > 0, a case no state has been shown to reach *(spec against reference code, R-442, REQ-PAY-099; context: TASK-M7-35; sibling of RQ-324)*

*Found porting the Chazy stain spec (R-442). Nothing is chosen (the human's follow-up: where the spec and the PR's code
disagree, file an RQ). For the human: only if a state reaching the case is shown. While none is, the three options
give the same label on every state found, so the choice changes no result shown; the ordering question that does
change results is RQ-324.*

- **File, section:**
  - Spec 01 §2's table (drill-down § "3.2 The rule, in order"), in order: "… **B-stable** | a bound hierarchy that
    passes Mardling & Aarseth (2001) | yes … **transient** | none of the above, and `E > 0` | no".
  - prin-rs PR #32, `src/chazy.rs::classify`: `if e > 0.0 { return Motion::Transient; }` comes before
    `if cfg.mardling_aarseth && qp == -1 && ko < uo && ma_stable(…) == Some(true) { return Motion::BStable; }`.
  - Drill-down § "2. Chazy's classes": "**E > 0 rules out B and OS.** Lagrange–Jacobi, `I'' = 4E − 2V > 4E > 0`".
- **Conflict:** B-stable's test reads the two-body energies of the inner pair and of the outer body against the pair's
  centre of mass. Their sum is not the total energy (the outer potential is the monopole's), so it does not follow
  from B-stable's test that `E < 0`. Were a state to pass it with `E > 0`, the spec would label it B-stable and the
  code transient.
- **Reachability unshown (the physics review of PR #223).** The total energy splits as `E = E_pair + E_out − ΔU`, with
  `ΔU` the outer potential less its monopole. B-stable needs `E_out < 0`; the osculating orbits bound the geometry,
  `r_in < 2 a_in` and `d ≥ R_p^out > crit · a_in`; at quadrupole order that gives `|ΔU| ≲ 0.11 |E_pair|`, so `E` stays
  below about `−0.9 |E_pair|`. Across 400,000 random states and a hill-climb maximising `E / |E_pair|` subject to
  `ma_stable` true, the pair bound and `K_out < U_out` (any orientation, retrograde included, log-uniform masses), the
  largest value found was −0.9986; no state reached `E > 0`. This is evidence, not a proof: the case is unshown, not
  shown unreachable.
- **Options seen:**
  - (a) The code's order: transient first, so E > 0 never reads B-stable, as the energy certificate says.
  - (b) The spec's order: B-stable first.
  - (c) The code's order, with B-stable also requiring `E < 0` explicitly (the same as (a), stated in the rule).
- **Waits:** REQ-PAY-099's B-stable-against-transient ordering clause (`rq: RQ-318`), in TASK-M7-35. REQ-VAL-185 does
  not wait on it: its E > 0 half holds by the code's order, which TASK-M7-35 builds, and (b) would differ only at a
  state not shown to exist. The fixtures do not reach this case, so TASK-M7-35's other work does not wait.

## RQ-319: what the Chazy fields read and the Chazy stains draw for a `decode_failed` sample, or a reserved `state` code, is not given *(silence, R-442, R-96, R-441, REQ-COL-072; context: TASK-M7-37)*

*Found porting the Chazy stain spec (R-442). Nothing is chosen. For the human: no; decidable per R-369.*

- **File, section:**
  - Spec 02 §4 (drill-down § "7. Validity and styles") gives running, `sim_failed` or NaN, collision-terminated and
    escape-terminated; it names neither `decode_failed` nor a reserved `state` code.
  - `docs/design/principia_colour_composition.md` § "1.4 …", line 185: "**How the table reads the payload (R-96):**
    "degenerate" is `decode_failed`"; § "4.1 …", "What every mode keeps": "a `sim_failed` sample, a reserved `state`
    code or a NaN the invalid pattern".
  - `docs/gui/principia_render_gui_spec.md` § "G6. …" (R-441): with `η_E` on, a free-fall slice is almost all
    `decode_failed`, drawn degenerate's white.
- **Silence:** a `decode_failed` sample has no physical state to classify.
- **Options seen:**
  - (a) The invalid hatch, as for `sim_failed`.
  - (b) The outcome palette's degenerate white `#ECECF0`, passed through as collision is, so a free-fall slice under
    `η_E` reads the same in both systems.
  - (c) A further value, "no IC", in `chazy_final` and `chazy_status`.
  - A reserved `state` code takes the invalid hatch under every option (colour_composition §4.1).
- **Waits:** REQ-COL-072's `decode_failed` clause (`rq: RQ-319`), TASK-M7-37.

## RQ-320: the "what just changed" view needs the previous frame's class, but a stain node reads the payload at the playhead *(silence, R-442, REQ-COL-083; context: TASK-M7-40)*

*Found porting the Chazy stain spec (R-442). Nothing is chosen. For the human: no; decidable per R-369.*

- **File, section:**
  - Spec 03 §7.2 (drill-down § "10.8.2 What just changed"): "White where the coarse class changed since the previous
    playhead, the rest dimmed. Needs the previous frame's class: a render-side comparison between frames, not a payload
    field."
  - `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots", lines 36–39:
    `colour(ctx) → vec3`, `brightness(ctx) → f32`, `post(rgb) → vec3`: every node reads the context at the playhead.
- **Silence:** no node kind or stage reads a previous frame, the corpus names no previous-frame buffer, and how much
  "dimmed" is is not given.
- **Options seen:**
  - (a) A render-side previous-class texture written each frame and bound to a post node, with the dimming a
    calibration.
  - (b) A latch in the payload (the coarse class at the previous playhead), against the spec's "not a payload field".
  - (c) Offline only: the view is computed from two exported frames, outside the live stain graph.
- **Waits:** REQ-COL-083 (`rq: RQ-320`), TASK-M7-40.

## RQ-321: the one-body views' escaping body: the classifier's `chazy_escaper`, or the event's escape detail where the run has an escape event *(open question in the ruled spec, R-443, R-442, R-29, R-103, REQ-COL-090, REQ-COL-092, REQ-COL-093; context: TASK-M7-41; sibling of RQ-313)*

*Found in the human's follow-up to the Chazy spec, which asks for it to be filed (R-443). Nothing is chosen. For the
human: yes, as physics where the choice changes results (R-369): it decides which body the views draw as escaping on
the pixels where the two rules disagree, and the docs do not settle it.*

- **File, section:**
  - The block's 04 addition, verbatim: "Open question for REVIEW_QUEUE: the escaper definition. chazy_escaper is the
    classifier's most-separated-body choice; the event stain's escape detail is set by Principia's own escape rule.
    They can name different bodies on the same pixel at the same playhead (the same gap as 02 §6.3). Decide whether
    the one-body views should read the event detail when the run has an escape event, or chazy_escaper always."
  - `docs/design/principia_dd_chazy_final_motion.md` § "5.6 `chazy_escaper` and `chazy_qk` (R-443)" (the field: the
    argmax of the hierarchy ratio, the lowest index on a tie) and § "10.11 One body at a time (R-443)" (the role
    table keys the escaping body on `chazy_escaper`).
  - `docs/design/principia_dd_simstate_payload.md` § "2. Bit layouts (the packed u32s)", `detail`: "escape → body id
    (0–2), `3` = **triple ejection**"; the escape rule, `principia_01_pitfalls.md` § "2.2 The criterion" and R-29.
  - RQ-313 (the same gap for the class: an escape-terminated pixel can read excursion or B here).
- **Options seen:**
  - (a) `chazy_escaper` always. The views agree with `chazy_class`'s HE/PE by construction; the body can differ from
    the event stain's on the pixels where the rules disagree.
  - (b) The event's escape detail where the run has an escape event naming one body (0–2), `chazy_escaper` otherwise.
    The views agree with the event stain on those pixels; a triple ejection (`detail` 3) names no single body, so it
    needs its own reading under this option.
  - (c) As (a), with the pixels where the two name different bodies counted in the legend, so the disagreement shows
    without changing what is drawn.
  - How RQ-313 is ruled bears on this one: if the event gates the class (its option (b)), (b) here follows naturally;
    under its (a), either option here stands.
- **Waits:** the escaping-body clause of REQ-COL-090, REQ-COL-092 and REQ-COL-093 (`rq: RQ-321`). TASK-M7-41 builds
  them on `chazy_escaper`, as the block gives it, meanwhile.

## RQ-322: the grey `#8A8A90` of the one-body palettes has OKLab chroma ≈ 0.009, where §10.11.2 says grey has no hue; its 0.020 from the stipple dot `#909090` is the dot distance, and by R-434's criterion it clears the stipple *(measured against the ruled spec, R-443, R-442, R-132, R-429, R-434, REQ-COL-091, REQ-COL-098, REQ-COL-099; context: TASK-M7-41, the M7 gate; sibling of RQ-317)*

*Found porting the follow-up, re-measuring its palettes (R-443). Nothing is chosen. For the human: yes, at the M7 gate,
with the palette calibrations it belongs to (REQ-COL-098, REQ-COL-099), since a calibration value is the human's
(R-369).*

- **File, section:**
  - The block's 04 addition: "Body-lens palette: 0.084, escaping hyperbolic against escaping parabolic … Every other
    pair, and every entry against the hatch and stipple, clears tau".
  - The block's 03 §10.2 (drill-down § "10.11.2 Body lens with energy"): "Grey and black have no hue, so under
    Replace-L they become neutral greys at body k's energy".
  - `docs/design/principia_colour_composition.md` § "1.4 Categorical colour-assignment — the outcome-state default
    palette", R-434's paragraph: the stipple's dots changed to `#909090` "so that no flat palette colour comes within
    the hatch's τ = 0.101 of every pixel of a 4 × 4 block: the nearest is 0.115 from its farthest pixel" (REQ-COL-064).
  - `docs/design/principia_dd_chazy_final_motion.md` § "11.2.2 One-body palettes, measured (R-443)".
  - Measured here, with dd_colouring §3.1's transforms: `#8A8A90` is OKLab L 0.635, a 0.0025, b −0.0085, chroma
    0.0089. Against the stipple: 0.020 from the dot `#909090` (spec 04 §2.1's method, the distance to the dot colour)
    and 0.212 from the block's farthest pixel, the not-yet grey `#4E4E4E` (R-434's criterion), τ = 0.101 (R-429). The
    block's other values (0.084, 0.177, 0.076, 0.102) reproduce, so the method agrees with the block's.
- **The stipple: no conflict.** By R-434's criterion, which is the corpus's, the grey clears the stipple (0.212 > τ),
  so the block's claim that every entry clears the hatch and stipple holds. The 0.020 is the dot distance, recorded and
  asserted as measured beside the R-434 figure (REQ-COL-097); it needs no ruling. (Corrected in the gui review of
  PR #223: this entry first read the dot distance as a conflict.)
- **Conflict (open):** the grey, like `#141418` (RQ-317), is not exactly neutral, so under Replace-L it keeps a faint
  blue cast where §10.11.2 says the grey roles become neutral greys.
- **Options seen:**
  - (a) Keep `#8A8A90`; for its chroma, take RQ-317's ruling for `#141418` (its (a), a stated chroma bound, or its (b),
    zeroing `a` and `b` under Replace-L).
  - (b) Make the grey exactly neutral at about the same lightness (e.g. `#8C8C8C`), re-measured against every entry and
    style by both methods; only the chroma changes.
  - (c) Move the grey to a neutral well above or below L 0.64, re-measured, if the human also wants the dot distance
    above τ.
- **Waits:** REQ-COL-091's neutral-grey clause and REQ-COL-098 and REQ-COL-099's confirmation (`rq: RQ-322`).
  REQ-COL-097 does not wait: it asserts both stipple figures as measured. TASK-M7-41 draws `#8A8A90` meanwhile.

## RQ-323: where §10.11.3's three body-lens panels live: the GUI spec has no view of one figure as several panels in v1, and its "side by side" is a v2 research tool *(silence, R-443, R-106, REQ-GUI-186; context: TASK-M8-45)*

*Found porting the follow-up (R-443). Nothing is chosen. Option (a) is the orchestrator's call per R-369 (a GUI
layout); options (b) and (c) are the human's, since each defers or drops part of R-443's 03 §10.3 (CLAUDE.md § "When to
ask the human": dropping or deferring a requirement).*

- **File, section:**
  - The block's 03 §10.3 (drill-down § "10.11.3 All three bodies"): "The 10.1 lens for k = 0, 1, 2 as three small
    panels side by side, at one playhead. A layout over 10.1, not a new stain."
  - `docs/gui/principia_render_gui_spec.md` § "G11. Research — first pass (v2)": "**Side by side** with a linked cursor
    and navigation, and a difference view. The linked views are a separate `ViewUI` item, specified with these v2
    research tools (R-106)".
  - The render contract's stain graph renders one composition per view (§ "Part 2 — Fixed pipeline, swappable
    slots").
- **Silence:** no v1 view shows one figure as several panels at one playhead, so the layout has no host.
- **Options seen:**
  - (a) One figure: the view's canvas is split into three panels, each the body lens at its own `k`, from the one
    payload at one playhead; a GUI layout, no new stain or node.
  - (b) Three linked views: the v2 side-by-side (R-106), so §10.11.3 moves to v2 with it (a deferral: the human's).
  - (c) An export only: the three lenses rendered and composed into one image for the record, with no live layout
    (drops the live layout: the human's).
- **Waits:** REQ-GUI-186 (`rq: RQ-323`), its part of TASK-M8-45.

## RQ-324: the spec's rule puts B-stable before excursion; prin-rs PR #32's `classify` returns excursion inside the hierarchy step, before B-stable; they differ for a bound, Mardling–Aarseth-stable hierarchy whose outer body is past `h_far` and receding, which is reachable and common *(spec against reference code, R-442, REQ-PAY-099, REQ-PAY-101; context: TASK-M7-35, TASK-M7-36; sibling of RQ-318)*

*Found in the physics review of PR #223 (the Chazy port). Nothing is chosen (the human's follow-up to R-442: where the
spec and the PR's code disagree, file an RQ). For the human: yes, as physics where the choice changes results (R-369):
it decides `chazy_class` and `chazy_status` on a large share of stable hierarchies.*

- **File, section:**
  - Spec 01 §2's table (drill-down § "3.2 The rule, in order"), in order: "… **B-stable** | a bound hierarchy that
    passes Mardling & Aarseth (2001) | yes | **excursion** | as HE but the outer body is bound to its pair
    (`K/U < 1 − ε`) | no …".
  - prin-rs PR #32, `src/chazy.rs::classify`, as the drill-down's § "3.2 The rule, in order" transcribes it: step 2
    (hierarchy), "pair bound and outer band `−1`: **excursion**", comes before step 4, "**B-stable.** If
    Mardling–Aarseth is on, the pair is bound, the outer body is bound to it (`K_out < U_out`) and `ma_stable(i, j, b)`
    holds".
  - Drill-down § "10.6 Status": B-stable `#2F6DB5` "bound, and passes Mardling–Aarseth: predicted to stay bound";
    excursion `#E08A2E` "bound, one body far out but still bound to the pair".
  - The port's first text, drill-down § "3.2" and R-442's note: "They differ only for a Mardling–Aarseth-stable
    hierarchy with `E > 0`" (RQ-318), which missed this case; both now name it.
- **Conflict:** a bound, Mardling–Aarseth-stable hierarchy whose outer body has ratio `ρ > h_far` and is receding
  passes the hierarchy gate with the outer band `−1`, so the code returns excursion; the table's order would reach
  B-stable first. The review generated 23,675 random Mardling–Aarseth-stable bound hierarchies, of which 11,066 fall in
  that branch. A stable triple then reads excursion on the outgoing half of each outer orbit and B-stable on the
  incoming half, so it flickers on the status map. Both fold to B in `chazy_final` (drill-down § "5.3"), so the
  final-motion maps and the one-body views do not change; `chazy_class` and `chazy_status` do. The §12.1 fixture
  `a_mardling_aarseth_stable_hierarchy_is_b_stable…` passes under both orders only because its outer `v_r` is exactly
  0. PE, which the code also returns in step 2, comes before B-stable in both orders, so they agree there.
- **Options seen:**
  - (a) The code's order: excursion before B-stable. A Mardling–Aarseth-stable hierarchy reads excursion whenever its
    outer body is past `h_far` and receding, B-stable otherwise; prin-rs PR #32's measurements (drill-down § "4") were
    taken under it.
  - (b) The table's order: B-stable before excursion, so a Mardling–Aarseth-stable bound hierarchy reads B-stable
    throughout its outer orbit, and excursion means "far out, bound, and not certified stable". The fixtures pass
    unchanged; PR #32's status-map shares (drill-down § "10.6") would move from excursion to B-stable where this
    applies.
  - (c) The code's order, with the status map latching B-stable once fired (a further latch, against the
    live-playhead reading of a label from the current state only).
- **Waits:** REQ-PAY-099's B-stable-against-excursion ordering clause (`rq: RQ-324`), in TASK-M7-35, and REQ-PAY-101's
  status for such a state (TASK-M7-36). The fixtures do not reach the case, so TASK-M7-35's other work does not wait.

## RQ-325: the drill-down says the classifier relies on "E < 0 rules out H, HP and P", but `classify` never tests E in the dispersal step or the two-stage H, so an E < 0 state can read H or P *(spec against reference code, R-442, REQ-PAY-099, REQ-VAL-185, REQ-VAL-186; context: TASK-M7-35, TASK-M7-36)*

*Found in the physics review of PR #223 (the Chazy port). Nothing is chosen (the human's follow-up to R-442: where the
spec and the PR's code disagree, file an RQ). For the human: yes, as physics where the choice changes results (R-369):
it decides whether an E < 0 sample can read H, HP or P.*

- **File, section:**
  - Spec 01 (drill-down § "2. Chazy's classes"), as first transcribed: "**The energy certificates the classifier
    relies on:** … **E < 0 rules out H, HP and P.** Total dispersal needs non-negative energy."
  - Drill-down § "4.3 On real charts (48 × 48)", as first transcribed: "No H and no transient appear: free fall has
    E < 0, and the certificate holds pixel by pixel."
  - Spec 05 §2.4 (drill-down § "12.2 New, for Principia", item 4): "On a free-fall chart (E < 0 everywhere) no pixel
    reads H, HP, P or transient at any playhead"; item 1 (Burrau): "E < 0 holds the certificate (no H, HP, P at any
    playhead)".
  - prin-rs PR #32, `src/chazy.rs::classify` (drill-down § "3.2 The rule, in order"): E is read only in the tidal
    gate's dispersal test and in the transient step; under the default hierarchy gate, step 1 (dispersal) and step 2's
    two-stage H do not test E.
- **Conflict:** the E > 0 half is enforced by the order (transient before B-stable, OS-candidate and B-so-far); the
  E < 0 half is not. The physics review constructed a counter-example: an equal-mass homothetic equilateral expansion
  with `E = −0.1 U_0`. At 2.9 times its initial size, `d_min / R_0 = 5.02`, every pair recedes and every pair's
  `K/U = 1.065`, so it reads **H**; at 3.2–3.5 times, pair `K/U` = 1.02–0.975, so it reads **P**; `E` stays below 0
  throughout and the system turns back at 10 times. For equal-mass free fall and for Burrau (3, 4, 5) the dispersal
  step provably cannot fire (`U(t) ≥ U_0` and `d0_ij ≤ M R_0 / √(m_i m_j) ≤ 3.46 R_0`, so the three separations
  cannot all exceed `5 R_0`), but the two-stage H is not excluded, nor are free-fall charts with extreme mass ratios.
  prin-rs PR #32's "0 wrong commits on E < 0" (drill-down § "4.1") is a measurement, not a guarantee.
- **Options seen:**
  - (a) Keep the code as it is. REQ-VAL-185's E < 0 arm and REQ-VAL-186's "no H, HP or P" become empirical checks on
    the charts they run: a failure is a physics result to report, not a bug.
  - (b) Guard H, HP, P and the two-stage H with `E ≥ 0`, so an E < 0 state that passes their geometric tests falls
    through to the later steps (B-stable, OS-candidate or B-so-far). The certificate then holds by construction, and
    the counter-example reads B-so-far.
  - (c) As (b), but the E < 0 state that passes the geometric tests reads a working state of its own, so the status
    map shows where the geometry and the energy disagree (an eleventh label).
- **Waits:** REQ-PAY-099's dispersal and two-stage-H clauses (`rq: RQ-325`), in TASK-M7-35; REQ-VAL-185's E < 0 arm and
  REQ-VAL-186 (TASK-M7-36). The fixtures do not reach the case (the two-stage H fixture and the three-flying-apart
  fixture both have E > 0), so the rest of TASK-M7-35 does not wait.
