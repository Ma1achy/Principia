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

## RQ-295: When `lookup_rescaled` is raised: step 0 always rescales and records `λ`, but a notice must fire only if its operation did, and canonical inputs raise none *(silence, R-72, REQ-ENC-006, REQ-ENC-014; context: TASK-M2-07, TASK-M2-15)*

*Found by TASK-M2-07, building `C`. Nothing is chosen.*

- **File, section:**
  - `docs/design/principia_dd_encode.md` § "3.2 Canonicalisation `C` — the exact ordered procedure", step 0 (:73–75):
    "`0  SCALE      I_in = Σ mᵢ‖rᵢ‖²  (CoM frame)          → r ← r/√I_in ;  p ← I_in^{1/4}·p`" and
    "`record λ = I_in^{−1/2} ; notice lookup_rescaled`": the step runs, and the notice is issued, on every input.
  - `docs/contracts/principia_inverse_encode_contract.md` § "Part 2 — The scale gauge gap (missing from the original
    inverse policy)" (:50): "Encode surfaces a **`lookup_rescaled`** notice carrying `λ`"; § "Part 6 — The canonical
    inverse policy, completed" (:167): "**0. Rescale to `I = 1`** via the similarity transform (Part 2). Record `λ`;
    notice `lookup_rescaled`."
  - `docs/design/principia_dd_encode.md` § "4. Seams (obligations → integration tests)", seam 8 (:128): "the three
    `lookup_*` notices fire iff their operation did"; REQ-ENC-014's statement, "lookup_rescaled, lookup_mirrored and
    lookup_clamped must each fire if and only if their operation did", and its verify detail, "canonical inputs raise
    no notice".
- **Silence:** a canonical-frame decode has `I = 1` only to rounding (TASK-M2-07 measures `|I − 1|` up to 3 eps at
  f64 and f32 on its output), so step 0 computes `λ = 1 ± a few ulps` and moves `r` and `p` by an ulp or so. Read
  literally, Part 2 and step 0 raise `lookup_rescaled` on every input; read with seam 8 and REQ-ENC-014, a canonical
  input raises none. The corpus does not say what counts as "the rescale's operation happened": `λ ≠ 1` bit for bit, a
  tolerance on `|I_in − 1|` (and so a calibration), or something else; nor whether step 0 is skipped where the notice is
  not raised, which would make `C` an exact no-op on canonical decodes (dd_decoder §5 test 5 asks only "rotation angle
  ≈ 0, mirror not taken").
- **Options seen:**
  - (a) Raise it iff `λ ≠ 1` bit for bit; step 0 always runs. Canonical decodes then raise it at most rounding
    rescales, against REQ-ENC-014's "canonical inputs raise no notice".
  - (b) Raise it iff `|I_in − 1| > τ_I`, a new calibration constant (R-71) in eps of the precision, and still run step 0
    everywhere; the notice reports a rescale the user would notice, not rounding.
  - (c) As (b), and skip step 0 (select `λ = 1`, `r`, `p` unchanged) where `|I_in − 1| ≤ τ_I`, so `C` is an exact
    no-op in scale on canonical input and the notice fires exactly when the state was rescaled.
  - (d) Always raise it (every lookup reports `λ`), and amend REQ-ENC-014's "canonical inputs raise no notice" for
    `lookup_rescaled`.
- **Applied meanwhile:** `kernel::canon::canonicalise` (TASK-M2-07) runs step 0 on every input and records `λ` in its
  output (`Canonical::lambda`, the notice's value); it forms no raised/not-raised flag for `lookup_rescaled`.
  `lookup_clamped` and `lookup_mirrored` are flags, each raised iff its operation did. `scale_step` asserts `λ = 1/2`
  is carried on the 2× Burrau state, which every option raises.
- **Waits:** REQ-ENC-006's and REQ-ENC-014's notice (`rq: RQ-295`): TASK-M2-07's closing of REQ-ENC-006, and
  TASK-M2-15's of REQ-ENC-014. TASK-M2-07's PR does not wait: it merges with REQ-ENC-006 held, not closed.

## RQ-296: What `C` outputs where `I_in` is 0 or rounds to 0 or `+∞`: all three bodies coinciding, an underflowing or an overflowing state, where step 0 divides by `√I_in` before step 2's guard *(silence, R-72, R-23, R-13, R-436, REQ-DEC-002; context: TASK-M2-07, TASK-M2-15)*

*Found by TASK-M2-07, building `C`. Nothing is chosen.*

- **File, section:**
  - `decisions.md` § "R-23 — Encode order: CoM subtraction precedes `I` *(CD-3)*": "The operational order 1a subtract
    CoM → 0 rescale → 1b subtract boost → 2 rotate → 3 mirror is normative." So step 0's `r ← r/√I_in` runs before
    step 2's `ρ = 0` guard.
  - `docs/design/principia_dd_encode.md` § "3.2 Canonicalisation `C` — the exact ordered procedure", step 0:
    "`r ← r/√I_in ;  p ← I_in^{1/4}·p`", "`record λ = I_in^{−1/2}`"; step 2: "`ρ = 0 → reject (coincident inner
    pair)`", which R-13 makes `lookup_clamped`.
  - `decisions.md` § "R-13 — Lookup has no coincident-bodies rejection *(closes RQ-11)*": "Exactly coincident bodies
    can't be represented, and the lookup ladder's range check catches them with `lookup_clamped`."
    `docs/contracts/principia_inverse_encode_contract.md` § "Part 6", step 2: "`ρ̃ = 0` (exactly coincident inner pair)
    can't be represented → `lookup_clamped` (R-13)."
  - `decisions.md` § "R-436 — …", *Applied*, "The producers select the pole values": a producer never forms `±∞` by
    `1/0`, and chooses a pole's value by comparing its divisor with 0 and a select.
- **Silence:** with positive masses, `I_in = 0` exactly when the three bodies coincide. Then `ρ = 0`, and R-13 and
  Part 6 give the notice, `lookup_clamped`; R-437 gives step 2's `φ = 0`. But step 0 runs first and divides by
  `√I_in = 0`, and nothing defines `C`'s output there: `λ = I_in^{−1/2}` is `+∞`, `r/√I_in` is `0/0`, and
  `I_in^{1/4}·p` is `0` (a state with no kinetic energy). No section point has `I = 1` and all bodies at one place.
  In floating point `I_in` also reaches 0 and `+∞` with the bodies apart (physics review 5542989202 P1), where the
  corpus is as silent:
  - **Underflow:** `I_in` rounds to 0 while the bodies are still apart: at f64 below about `1e-162` per coordinate
    (e.g. `r₀,₁ = ∓1e-170`, masses `(¼, ¼, ½)`), at f32 below about `4e-23` (about `1e-19` where a GPU
    flushes subnormals). There `ρ ≠ 0`, so no `lookup_clamped` fires, and step 0 cannot reach `I = 1`.
  - **Overflow:** `I_in` rounds to `+∞`, at f64 above about `1e154` per coordinate and at f32 above about `1e19`. Then
    `λ = 0`, `r/√I_in` is `±0`, and `I_in^{1/4}·p` is `±∞`, or `0·∞` = NaN for a zero component; and WGSL may assume
    there are no infinities (R-436), so the GPU's arithmetic there is not pinned.
- **Options seen:**
  - (a) Skip step 0 at `I_in = 0` (select divisor 1, so `r` stays as it is, `p` unscaled, `λ = 1`), and let
    `lookup_clamped` carry the coincident case: the state passes through un-normalised and flagged. Underflow and
    overflow are not covered by this alone.
  - (b) Record `λ = +∞` by its bits (R-436's producer rule), `r = 0`, `p = 0`, flagged `lookup_clamped`.
  - (c) Define `C`'s output there as a fixed sentinel state, or as undefined with the flag the only output a consumer
    may read (the validation ladder rejects the pixel before decode).
  - (d) For underflow and overflow: rescale in two steps (by a power of two first, exact, then by the rest), so `I_in`
    is formed from scaled coordinates and neither rounds to 0 nor `+∞` for any finite state with the bodies apart.
  - (e) For underflow and overflow: a range check on `I_in` (against calibrated bounds, R-71) that raises
    `lookup_clamped`, or a notice of its own, with the output defined as in (a), (b) or (c).
- **Applied meanwhile:** `kernel::canon::similarity` (TASK-M2-07) selects the divisor `√I_in` as 1 where `I_in = 0`,
  so no `1/0` is formed (R-436). What the code then does in each case (`crates/kernel/src/canon.rs`, the module's
  note):
  - **`I_in = 0`, bodies coincident:** `r` stays 0, `p ← I_in^{1/4}·p = 0`, `λ = 1`, `lookup_clamped`, `φ = +0`.
  - **`I_in` rounded to 0, bodies apart:** the same select, so `r` is left unscaled (`I ≈ 0`, not 1), every momentum
    becomes 0 (`√√0 = 0`), `λ = 1`, and, since `ρ ≠ 0`, no `lookup_clamped`: the state leaves `C` off the section,
    unflagged.
  - **`I_in = +∞`:** `r` becomes `±0`, `λ = 0`, every momentum NaN on the CPU (`±∞` or `0·∞`, then step 1b's sums and
    the rotation). Step 2's guard and angle and step 3's decision are read from the input's pair differences before
    step 0's division (`kernel::canon`, "The branch inputs, read before step 0"), so `lookup_clamped` is raised iff
    `ρ = r₁ − r₀` is 0, `φ` is that `ρ`'s angle, and the mirror is R-82's on the state's own `λ̃_y`, which the
    decision's power-of-two rescale keeps in range; on the GPU the arithmetic on `∞` is unpinned.

  That only keeps the arithmetic free of `1/0`. The tests assert `lookup_clamped` and `φ = +0` at the coincident case
  (R-13, R-437) and that the GPU forms no non-finite value there; at underflow, that the case is reached and no `1/0` is
  formed (every output finite, `canon_inertia_extremes`); at overflow, only that the case is reached. Nothing about the
  state or `λ`.
- **Waits:** REQ-DEC-002 (`rq: RQ-296`), TASK-M2-07's closing of it. TASK-M2-07's PR does not wait: it merges with
  REQ-DEC-002 held, not closed.

## RQ-297: a ledger `scale` change moves the payload schema version, which invalidates every cached payload, but seam 5 says it re-styles the view "with **zero** recompute" *(conflict, R-36, REQ-COL-032; context: TASK-M7-05)*

*Found by TASK-M7-05's review (qa, PR #219, `qa_ledger_scale_restyle_keeps_the_sim_key`, since restated; see
"Waits"). Nothing is chosen.*

- **File, section:**
  - `docs/design/principia_dd_colouring.md` § "4. Seams (obligations → integration tests)", seam 5 (:243): "read
    fields only through generated accessors + ledger metadata | field views sane; a ledger scale change re-styles the
    view with **zero** recompute". § "2. Consolidated contract" (:17): "colour **compaction** is the third
    compactification role — render-key, free, never re-integrates; per-field scale comes from the **ledger
    metadata**". REQ-COL-032 repeats it, its verify detail "change a field's ledger scale; the view re-styles and the
    dispatch counter is unchanged".
  - `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract" (:21): "From the
    **caching/integrator contracts**: the **payload compatibility signature** includes the payload schema version — a
    layout change must invalidate every cached payload. **The schema version is a content hash of the canonicalised
    §3 ledger, not a hand-bumped integer (R-36).**", and § "4. Seams" (:994): "schema version (the
    ledger's content hash, R-36) ∈ payload compatibility signature | any ledger change invalidates every cached
    payload (cache serves nothing stale-schema'd)".
  - `docs/contracts/principia_caching_contract.md` § "Part 1 — Two-level keying: identity vs validity", "Validity"
    (:14): the payload compatibility signature includes the "payload schema version (a content hash of the ledger,
    R-36). Same identity, different signature → different payload".
  - The code: `crates/ledger/src/version.rs` (:199–209) hashes each entry's `scale` into the schema version, as R-36's
    "canonicalised §3 ledger" reads; `scale` is one of the §3 ledger's columns.
- **Conflict:** an edit to a field's ledger `scale` is a ledger change, so under R-36 and generation_root :994 it
  moves the schema version, the compatibility signature changes, and every cached payload is recomputed. Seam 5 and
  dd_colouring §2 say the same edit re-styles the view with zero recompute: `scale` is render-key metadata, which
  changes nothing the kernel computes or the payload's layout. The corpus also differs on scope: generation_root :21
  states the requirement as "a layout change must invalidate every cached payload", and then makes the schema version
  a hash of the whole canonicalised §3 ledger; :994 states it as "any ledger change invalidates every cached payload".
  A `scale` edit is a ledger change but not a layout change.
- **Options seen:**
  - (a) The schema version hashes only what decides the payload's layout and the kernel's output, not the colour
    metadata: `scale` leaves the hash, with any other render-only column a ruling names; R-36 and generation_root
    :21/:994 are reworded to say which columns, and seam 5 holds as written. It rests on :21's "a layout change".
  - (b) Keep the whole-ledger hash and amend seam 5, dd_colouring §2 and REQ-COL-032: a ledger scale change re-styles
    the view with zero *recompute of the view's own pass* but invalidates cached payloads, as any ledger edit does;
    the run-time re-style without recompute is then a view override (a GUI choice of compaction), not a ledger edit.
    It rests on :994's "any ledger change" and R-36's hash of the whole canonicalised ledger.
- **Applied meanwhile:** nothing is chosen. TASK-M7-05's seam-5 test builds a view from an edited clone of `E_0`'s
  entry without regenerating the ledger, so it shows the render side only (the view re-styles, the sim buffer is
  unchanged, one fragment compile); it says so in its doc. `ledger::version` is unchanged.
- **Waits:** TASK-M7-05's PR does not wait; REQ-COL-032 stays held until the ruling. qa has restated its test as
  `qa_ledger_scale_restyle_leaves_the_sim_buffer`, which asserts neither side of this entry: it covers the render side
  only (the frame re-styles, the sim buffer reads back byte for byte), and the schema-version side waits on this
  entry.

## RQ-298: An explicit `fma` is not fused on every backend: lavapipe evaluates it as a rounded product and an add, against the determinism note's rule 6; and a GPU that flushes subnormals rounds differently from one that keeps them, which the corpus does not address *(conflict and silence: corpus premise against measurement, R-34, R-84, REQ-INT-064; context: TASK-M2-07)*

*Found by TASK-M2-07's CI (qa review 5543656052 R1). TASK-M2-07 no longer relies on the premise; other branch inputs do.*

- **File, section:**
  - `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)", rule 2:
    "an **explicitly written** `fma(dy, dy, dx*dx)` (explicit `fma` forces every backend onto the identical fused op
    instead of choosing)"; rule 6: "Written as `a*b + c`, one compiler contracts and another does not; written as
    `fma(a, b, c)`, every backend takes the fused path."
  - `docs/design/principia_dd_decoder.md` § "3.1 Mass", "Its premise": "The form holds only on a backend that keeps an
    explicit `fma` fused and does not fold the `×1`. … Whether the kernel's SPIR-V keeps GLSL.std.450 `Fma` fused, which
    TASK-M2-02's qa review observes needs the `NoContraction` decoration, and whether rust-gpu emits that decoration, is
    a check for the second-backend parity run, not settled here."
  - `decisions.md` § R-34: "Rule 6 extends to every branch input that consumes multi-op arithmetic", and
    REQ-INT-064: "must use explicit `fma` in a fixed written form".
- **Conflict:** CI's `gpu-lavapipe` and mutants shards (Mesa 25.2.8 lavapipe, Vulkan, `PRIN_GPU_BACKEND=vulkan`; run
  38857393629, jobs 116831834076 and 116831834067) failed `canonicaliser_seam_sweep` at 1c1037ae: "the backends'
  mirror branches differ at seam state 3". At that head the decision was built from explicit `fma`s. qa recomputed it
  on the CPU with every `fma` as an unfused `a*b + c`, and that reproduces lavapipe's branch at state 3 and flips
  states 4, 6, 7, 11 and 15 too. The same code gives CPU-f32's branch on Metal. So on lavapipe an explicit `fma` is not
  the single fused rounding that rule 6 says every backend takes. (Whether WGSL's `fma` or SPIR-V's GLSL.std.450 `Fma`
  must be fused is not settled here: the specifications were not checked from this machine, and dd_decoder §3.1
  already leaves it open.) Every
  branch input built on rule 6's premise can therefore fork between Metal and lavapipe at its rounding edges:
  dd_decoder §3.1's `M₀₁ = fma(m₁, 1, m₀)` and rule 2's quantised `d²` among them.
- **Silence (a second hazard): subnormal flushing.** Metal flushes subnormal results; the CPU keeps them. A branch
  input with an intermediate below the smallest normal (f32: about `1.2e-38`) can therefore fork on identical inputs,
  fused or not. The six rules in the note do not mention it. Measured on this PR (physics review 5544295057 P3): at
  66974583 the mirror decision's products, with `δ_λ² ≈ 2^−80` among their factors, went subnormal in their low parts on
  hierarchical states, and over 20 000 states per ratio CPU-f32 and Metal forked on 16 at an inner/outer ratio of
  `1e-3`, 482 at `1e-4` and 10 262 at `1e-6`. At `λ̃_y = −2δ_λ` Metal missed the mirror on 8 093 at `1e-7` and on all
  at `1e-8`.
- **Options seen:**
  - (a) Correct rules 2 and 6, R-34 and REQ-INT-064. A branch input that must agree everywhere is built only from
    operations every IEEE backend rounds alike, whether it fuses or flushes. For example, TASK-M2-07's
    `IeeeBits::product` (operands split at their bits so the partial products are exact) is used on mantissas kept in
    `[1, 2)` with integer exponents, so no intermediate is subnormal. This costs more operations per branch input, and
    each input's form has to be rewritten and re-proved.
  - (b) Keep rule 6 and require it of the toolchain: `NoContraction` and a fused `Fma` through rust-gpu → naga → each
    driver, and subnormals kept (or flushed) alike, checked per backend at the parity gate. A backend that fails the
    check is out of scope for branch parity. The existing forms stay, and the cost moves into toolchain checks; whether
    every target driver can meet it is unknown.
  - (c) Keep rule 6 for Metal, the build machine's backend, and record lavapipe, and any backend that flushes or does
    not fuse differently from Metal, as one whose branch parity is not claimed. Nothing in the code changes, and the
    parity claim narrows.
- **Applied meanwhile (TASK-M2-07, per R-369 on qa R1 and physics P3):** `C`'s mirror decision and guard use no
  `fma`. The guard is a lone subtraction. The decision carries mantissas in `[1, 2)` with integer exponents, and its
  products are `IeeeBits::product` (`kernel::canon`, "The branch inputs, read before step 0"). **This departs from
  R-34's letter and from REQ-INT-064** ("explicit `fma` in a fixed written form"), and option (b) would undo it, so
  REQ-DEC-017's form is among what this entry holds. Nothing else changes: `M₀₁` and the other `fma`-built branch
  inputs stay as they are, pending a ruling.
- **Holds** (`rq: RQ-298`, applied per R-369 from physics review 5544295057; every branch input built with an explicit
  `fma`, or this PR's departure from that):

  | Requirement | Why |
  |---|---|
  | REQ-DEC-011 | `crates/kernel/src/decode/mass.rs:73`: `M₀₁ = fma(m₁, 1, m₀)` feeds `DEGENERATE(M01_TINY)` |
  | REQ-VAL-017 | the tags must be bit-identical across backends, `M01_TINY` among them |
  | REQ-VAL-079 | the no-fork test's domain |
  | REQ-INT-087 | `crates/kernel/src/shape.rs` `disc_input` (every term an explicit `fma`) feeds `in_disc`; the result must match exactly across backends |
  | REQ-INT-088 | the march's disc test, through `point_disc_input` |
  | REQ-EVT-021 | collision `d² < r_coll²`, specified with an explicit `fma`, not yet built (TASK-M3-11) |
  | REQ-INT-066 | the `N_sub` bucket's `d² = fma(dy, dy, dx·dx)`, not yet built (TASK-M4-01) |
  | REQ-INT-064 | every integrator branch input in explicit `fma`, not yet built (TASK-M4-03) |
  | REQ-INT-058 | the compile-discipline checklist's "explicit `fma`" |
  | REQ-DEC-017 | this PR's mirror decision, which departs from R-34's letter (above) |
  | REQ-DEC-002 | `crates/kernel/src/canon.rs`'s `fma` CoM and `I_in` feed `similarity`'s `I_in = 0` select (also held by RQ-296) |
  | REQ-DEC-022 | TASK-M2-09's feasibility input (`fma` per R-34), not yet built |

  REQ-INT-089 is not held: it is same-backend, split dispatches.
- **Waits:** the twelve requirements above. TASK-M2-07's PR does not wait.

## RQ-299: The pinned nightly miscompiles plain Rust at every optimisation level above 0: two calls of a by-value closure on one array lose the first call's writes, in rustc's MIR GVN pass, and the workspace's native build, the kernel's f64 instantiation included, is built on that compiler *(conflict: toolchain against the native build, R-169, canonical_spec §1 item 2; context: TASK-M2-07)*

*Found by TASK-M2-07's code review (5544375930). Nothing is chosen.*

- **File, section:**
  - `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)", item 2: "to **f64**
    for the CPU via ordinary `rustc`"; item 6: "The native build can also run the survey headless and generate the
    Paper-2 numbers".
  - `decisions.md` § "R-169 — The GPU CI jobs, and an install step for every toolchain *(closes G1, H5)*":
    "`rust-toolchain.toml` for the rust-gpu pin (TASK-M0-14)". `rust-toolchain.toml` pins `nightly-2026-04-11`
    (rustc 1.96.0-nightly 02c7f9bec 2026-04-10), and the whole workspace builds on it.
- **Conflict:** `stage_canon_native` (`crates/kernel/tests/canon.rs`) fails under
  `cargo test -p kernel --release --test canon`, and under the dev profile at opt-level 1, 2 and 3; it passes at
  opt-level 0, which every CI test job builds at, so CI does not see it. In `notice_states` (canon.rs:675)
  `pair(flip(plain))` returns `pair(plain)`: the flip is lost. The kernel's code is not involved. A standalone file
  with no dependencies reproduces it:
  ```rust
  fn main() {
      let plain: [[f32; 2]; 3] = [[-1.0, -0.5], [1.0, -0.5], [0.0, 0.5]];
      let flip = |mut x: [[f32; 2]; 3]| { for v in x.iter_mut() { v[1] = -v[1]; } x };
      let pair = |mut x: [[f32; 2]; 3]| { x[1] = x[0]; x };
      let s = [flip(plain), pair(flip(plain))];
      println!("{} (want 0.5)", s[1][0][1]);
  }
  ```
  Measured on this machine (`rustc +<toolchain> -C opt-level=<n>`):

  | Toolchain | opt-level 0 | 1 | 2 | 3 |
  |---|---|---|---|---|
  | nightly-2026-04-11 (the pin) | 0.5 | −0.5 | −0.5 | −0.5 |
  | stable 1.99.0 (b940084d7 2026-09-28) | 0.5 | 0.5 | 0.5 | 0.5 |

  On the pin, `-Zmir-opt-level=0` or `-Zmir-enable-passes=-GVN` at opt-level 1 gives 0.5, and opt-level 0 with
  `-Zmir-opt-level=2` gives −0.5: the fault is in rustc's MIR GVN pass, not LLVM. Close variants pass: the same
  functions as `#[inline(always)] fn`s; `[flip(plain), pair(flip(plain))]` without the first `flip(plain)`, or with
  `pair(plain)` in its place; a two-element `i32` array. So the trigger is not characterised, and no search can show
  that shipped code is free of it.
- **Shipped code searched:** every by-value `mut` closure parameter outside the test files (`git grep` on
  `crates/*/src` and `xtask/src`, the patterns `|mut x: T` and `|mut x|`): `gui/src/keyboard/scopes.rs:279` (a
  `usize` loop counter), `engine/src/compute/tests.rs:380` (a test module), and six closures over a writer, a pipe or
  an iterator item, each called once per item. None takes a `Copy` aggregate and is called twice on the same value,
  as the reproducer does. Not checked: whether rust-gpu's SPIR-V build of the kernel runs the same MIR pass.
- **Where it can matter:** release builds, which no CI job builds and the human's Mac uses for benchmarks (R-186),
  and the native survey build (canonical_spec §1 item 6).
- **Options seen:**
  - (a) Bump the pin to a nightly without the fault. It must be one `rustc_codegen_spirv` 0.10.0-alpha.1 builds
    against, or rust-gpu moves with it; whether such a nightly exists was not checked.
  - (b) Keep the pin and turn the pass off for the native build: `-Zmir-enable-passes=-GVN` in the workspace's
    rustflags. This is a nightly flag, changes build configuration (a ruling), and costs some optimisation.
  - (c) Keep the pin and the passes; report the fault upstream, and add a CI job that runs the native tests at the
    release profile, so a miscompile a test covers is seen. Code no test covers stays exposed.
- **Applied meanwhile:** nothing. `stage_canon_native` is unchanged, not restructured around the fault.
- **Waits:** nothing held: no shipped code was found with the pattern. TASK-M2-07's PR does not wait.

## RQ-310: `I_max` and the "when decided" latch want descriptor bits 10–15, which payload §2 holds reserved and "never opportunistically reused", beside R-37's departed bit, whose place is open *(conflict and budget, R-442, R-37, R-36, REQ-PAY-105, REQ-PAY-106, REQ-COL-082, REQ-COL-106, REQ-VAL-055; context: TASK-M7-35, TASK-M7-40)*

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
  and REQ-COL-106's lightness by time (TASK-M7-40). The local, per-step latch and the classifier's tests do not wait.

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
- **Waits:** REQ-COL-074 (`rq: RQ-317`), TASK-M7-38's B test; REQ-COL-087's confirmation (`rq: RQ-317`), since
  option (c) changes B's swatch in its palettes. The flat stains (TASK-M7-37) draw `#141418` meanwhile.

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

## RQ-320: the "what just changed" view needs the previous frame's class, but a stain node reads the payload at the playhead *(silence, R-442, REQ-COL-083, REQ-COL-106; context: TASK-M7-40)*

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
- **Waits:** REQ-COL-083 and REQ-COL-106's dimming (`rq: RQ-320`), TASK-M7-40. The dimming's value is REQ-COL-106's
  calibration (R-71), proposed by TASK-M7-40 under whichever option rules.

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
- **Waits:** REQ-COL-104's confirmation (RQ-330's gate, the M2 gate by R-454).

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
- **Waits:** REQ-COL-105's confirmation (RQ-330's gate, the M2 gate by R-454). TASK-M2-32 builds the ruled swatches meanwhile.

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
- **Waits:** the Euler landmarks' shown labels only, in TASK-M8-11's shape-sphere widget. No requirement carries
  `rq: RQ-334` (applied per R-369, code review 5547986569 on PR #226, F2): REQ-GUI-054, the landmarks TASK-M8-11
  closes, names the binary collisions and the Lagrange poles, whose labels R-449 settles, and REQ-GUI-188's names;
  the Euler labels are the definition TASK-M8-11 writes, so neither requirement waits.

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

## RQ-350: chart_reference §2 says the invariant charts "fix geometry and mass", but no passage says where the frozen configuration and masses come from *(silence, R-72, REQ-CHART-017; context: TASK-M2-12, gap G8)*

*Found by TASK-M2-12 (its Notes, gap G8). Nothing is chosen for the docs; TASK-M2-12's PR does not wait.*

- **File, section:**
  - `docs/design/principia_chart_reference.md` § "2. Invariant-momentum charts `(Lz, E)` and `(Lz, K)`": "These fix
    geometry and mass, and use the two axes for **conserved quantities** rather than raw controls."
  - § "2.1 Feasibility, and the warp that makes every pixel valid": "`E = U + K`", `U` the potential at that
    configuration, with no source named for the configuration.
  - `docs/contracts/principia_chart_decoder_contract.md` Part 5: "The usual case is 2 swept + 6 frozen", and Design
    axiom 6: "the lock pins the centre"; neither names which six controls an invariant chart freezes or where they
    are read from (`z₀`'s blocks, the lock, or the chart's own parameters).
- **What the task file gives:** `plan/tasks/M2/TASK-M2-12.md` § "Deliverables": "frozen configuration and mass as
  chart params".
- **Options seen:**
  - (a) The chart's parameters are the four frozen controls `(z_α, z_β, z_μ₁, z_μ₂)` in the decoder's own terms, set
    from the slice centre `z₀`'s configuration and mass blocks when the chart is entered (or from the lock where one
    is set), and edited as any chart parameter (Design axiom 6).
  - (b) The parameters are physical: the masses `mᵢ` and the configuration `(α, β)` (or `rᵢ`), encoded to controls by
    the encode path.
- **Applied meanwhile** (per R-369, TASK-M2-12): (a)'s representation, the four controls as chart parameters
  (`kernel::chart::invariant::Invariant`), decoded by the shared mass and configuration blocks, so the construction
  reads the decoded `z_α` its seed rule needs; where the GUI takes them from (`z₀` or the lock) is not written.
- **Waits:** nothing.

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
  the steer cannot. (2) Which stage the views differentiate: before or after canonicalisation and the scale gauge
  (whether before or after `η_E` is RQ-345). (3) The derivative's method and step (central differences in f64, as
  deep_zoom's, or analytic). (4) The Gram factor's definition: `√(|J q₁|²|J q₂|² − (J q₁ · J q₂)²)` is the
  pushforward's area factor only for an orthonormal pair, and the decoder's `q₁`, `q₂` carry the zoom σ (`|q| = σ`,
  R-83) and, with both tilts set, the shear `u₁ · u₂ = sin τ₁ sin τ₂` (RQ-340), so on them the "stretch" picks up σ²
  and `√(1 − sin²τ₁ sin²τ₂)` and changes with the zoom; and the factor of 2 from the chart's `(2s−1)`. (5) The metric
  on the physical coordinates, which the grid depends on as the steer does, is RQ-346. (Points 2, 4 and 5 added in
  the fix round of PR #227, physics review P7 to P9.)
- **Options seen:**
  - (a) A definition requirement (R-72): a CPU, f64, central-difference Jacobian `∂x/∂z` (8 columns) of the decoder's
    output in the view's coordinates at a latent point, the views reading `J q₁`, `J q₂` from it, the Gram factor
    read on the orthonormalised pair, `√(|J q̂₁|²|J q̂₂|² − (J q̂₁ · J q̂₂)²)` with `q̂₁`, `q̂₂` orthonormal (the physics
    review's proposed reading of R-452's formula); a new requirement in M6 beside REQ-DEC-033, sharing its method.
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
  λ̃ᵧ` carry the two configuration degrees of freedom (α, β) with `R̃ = 1`, so they are redundant; (3) with `η_E` off,
  `z_μ1`, `z_μ2` move none of the eight, so a plane tilted into the mass directions pushes forward degenerately
  (`J q = 0` for `q` in them) and the view cannot draw it; the physics review measured the set's rank at exactly 6 at
  every sample, its kernel the two mass directions; (4) the positions are mass-weighted and the momenta are not, so
  `Σ`-type comparisons mix weightings; (5) whether the decoder's output is read before or after the scale gauge;
  `η_E` is RQ-345: with it on, `η_E` depends on the masses and the configuration, so the masses do move the momenta,
  and point 3 holds only with it off; (6) a non-redundant 8-set exists: the decoder `z ↦ (α, β, q₀…q₃, m₀, m₁)` is a
  diffeomorphism onto its image (each block a monotone link of its own latents, the masses through softmax∘tanh), so
  the decoder's own physical set, `(α, β)` or a chart of the hemisphere, the four Jacobi momenta and two masses, gives
  an 8 × 8 `J` of full rank (the physics review: rank 8 at 200 of 200 samples, `η_E` off). With such a set
  `J⁺ = J⁻¹`, the steer is metric-independent and lights cell (i, j) at exactly ±1 at `z₀` for every cell, and its
  only refusals are chart singularities (RQ-344). (Point 6, and the rank in point 3, added in the fix round of
  PR #227, physics review P6 and P7.)
- **Options seen:**
  - (a) Keep the eight as proposed; the view shows the dead row and the degenerate cases (readout "J q = 0: the plane
    moves only the masses"). This fails the criterion the human set in 3.2 ("no redundant coordinate"): a row and
    column always 0, and a steer refused or lighting below ±1 on most cells.
  - (b) Eight coordinates that chart the image, the full-rank set of point 6: e.g. `α, β` (or `ρ̃ₓ, λ̃` in polar form),
    the four momenta, and the two masses (`m₀/M`, `m₁/M`), each named as the decoder writes it.
  - (c) The momenta mass-weighted as the positions are (`p̃ = p/√μ`), so the coordinates share one weighting. This
    complements (a) or (b) rather than replacing them: mass-weighting the momenta alone leaves the rank at 6.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199, REQ-GUI-200: the steer's cells are this set's).

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
  (`L_z`, the fibre of `T*S²`) as coordinates. The masses do move its rates: `ṅ` and `Ṙ` are `d/dt` of the
  mass-weighted `ρ̃`, `λ̃`, with `ρ̃̇ = p_ρ/√μ_ρ` and `λ̃̇ = p_λ/√μ_λ`, e.g.
  `Ṙ = cos α · p_ρx/√μ_ρ + sin α (cos β p_λx + sin β p_λy)/√μ_λ` (the physics review: the `z_μ` columns' norms about
  19 and 17, not 0). The Shape Jacobian has rank 5 at every sample (`n̂` 2, `ṅ` 2, `Ṙ` 1), and its 3-dimensional
  kernel is mixed: the rotation (`L_z`-only) direction in the momentum block, and two mass directions each compensated
  by a momentum change. So a plane in the `L_z` direction pushes forward degenerately; a mass-direction tilt does not
  push forward to 0. The masses enter Shape's rates but not Jacobi's coordinates (RQ-342 point 3); (5) the 2-to-1
  cover of the shape sphere (chart_reference §3.3); (6) a non-redundant 8-set exists (RQ-342 point 6): (b)'s set
  `(θ, φ, θ̇, φ̇, L_z, Ṙ, m₀, m₁)` has full rank (the physics review: rank 8 at 299 of 300 samples, the one other at
  8e-8 relative being a saturated mass logit). (Point 4 corrected, and point 6 added, in the fix round of PR #227,
  physics review P4 and P6.)
- **Options seen:**
  - (a) Keep the eight; define `ṅ`, `Ṙ` from the decoded state (`ṅ = dn/dt` along the flow at `t = 0`), and show the
    dead `R` row. This fails the criterion the human set in 3.2 ("no redundant coordinate"): a row and column always
    0, rank at most 5, and a steer refused or lighting below ±1 on most cells.
  - (b) A chart of the image, the full-rank set of point 6: `θ, φ` (or two of `n̂`, naming which: `(n₁, n₂)` is
    singular on `n₃ = 0`, the collinear configurations, which is the decoder's mirror boundary `β ∈ {0, π}`), their
    rates, `L_z` (or the fibre rate), `Ṙ` and the two masses.
  - (c) Drop the Shape view until the set is settled.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199, REQ-GUI-200: the steer's cells are this set's).

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
  positions and momenta; the physical metric is RQ-346); (3) by RQ-342 and RQ-343 the proposed sets' Jacobians are
  rank-deficient on the whole image (rank at most 6 for Jacobi, at most 5 for Shape), so "rank-deficient there" needs
  reading as "the two preimages do not span a 2-plane", with a threshold (a singular
  value or the Gram factor of `J⁺e_i`, `J⁺e_j` below a tolerance); (4) where `e_i` is not in the Jacobian's range
  (`ρ̃ᵧ`, `R`), `J⁺e_i = 0` and the click is always refused, and where it is partly in range the steered plane's
  pushforward lights cell (i, j) below ±1, so R-452's test "lights that physical cell at ±1 (to tolerance)" holds only
  for `e_i`, `e_j` in the range; (5) the steered basis is a general 8D plane, not a preset with tilts, so how the
  Manifold view's sliders follow needs saying (as the Chart builder's Apply does, §G7, is the nearest); (6) the
  coupling to RQ-342 and RQ-343: with a full-rank 8-set (their option (b), `η_E` off), `J⁺ = J⁻¹`, the steer is
  metric-independent and lights (i, j) at exactly ±1 at `z₀` for every cell, its only refusals chart singularities
  (link saturation, the α and β poles, the chosen sphere chart's poles), and points 2 to 4 largely dissolve; options
  (a) and (b) below matter only if the human keeps a rank-deficient set. With `η_E` on, no 8-set has full rank
  (RQ-345). (Point 3 corrected, and point 6 added, in the fix round of PR #227, physics review P5 and P6.)
- **Options seen:**
  - (a) As written, the test restricted to coordinates in the range, and the sliders following as after Chart
    builder's Apply, with one of two rank tests: (a1) the smaller singular value of `[J⁺e_i, J⁺e_j]` above a
    tolerance, a quantity in `J`'s inverse units that scales with `J`, so a fixed tolerance on it is not dimensionless;
    or (a2) the smaller singular value of `[P e_i, P e_j]`, with `P = J J⁺` the projector onto `J`'s range, which lies
    in [0, 1], is scale-free, and also predicts how close to ±1 cell (i, j) lights (the physics review, P10).
  - (b) A weighted pseudo-inverse with stated metrics on `z` and on the physical coordinates (the latter RQ-346's).
  - (c) Only the latent steer (the Plücker view's); the physical cells are not clickable.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-200).

## RQ-345: With the energy normalisation E* set, every physical Jacobian of the decoder has rank at most 7, whatever the coordinates; the physical views and the steer must read the state before or after η_E, and the steer needs a rule for the direction normal to the energy shell *(physics review, silence, R-72, R-452; context: TASK-M6-35)*

*Found by PR #227's physics review (P7, 16 Oct 2026), filed in its fix round as a new entry rather than folded into
RQ-341, because it bounds every coordinate set of RQ-342 and RQ-343 and the steer of RQ-344 alike. Nothing is chosen.
For the human (physics: it changes what the physical views show).*

- **File, section:** `docs/design/principia_chart_reference.md` § "0.6 Energy normalisation — optional, and forbidden
  on some charts": "`η_E = √((E* − U)/K0)          pᵢ ← η_E·pᵢ          feasibility: E* ≥ U`". `decisions.md` § R-452
  (the human's words, 3.2) pushes the plane "through the decoder's Jacobian at z₀" and names no stage.
- **The points:** (1) with `E*` set, `p ← η_E p` puts the decoder's whole image on the level set `E = E*`, so every
  physical Jacobian has rank at most 7, whatever the eight coordinates (the physics review: rank 7 at every sample,
  even for RQ-342's full-rank set); no 8-coordinate set is non-degenerate there. (2) `η_E` depends on the masses and the
  configuration, so with it on the masses move Jacobi's momenta (RQ-342 point 3 holds only with it off). (3) The
  steer on an `E*`-normalised chart has a direction it cannot reach: the one normal to the energy shell.
- **Options seen:**
  - (a) The physical views and the steer read the state before `η_E` (the full-rank sets stay full rank; the views show
    a state the integrator does not start from when `E*` is set).
  - (b) They read it after `η_E`, as the integrator starts from it; on an `E*`-normalised chart the steer refuses the
    one direction normal to the energy shell, and a cell whose plane needs it is refused with that reason logged.
  - (c) After `η_E`, with the Jacobi and Shape views replaced on an `E*`-normalised chart by a 7-coordinate set on the
    level set.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199, REQ-GUI-200).

## RQ-346: The physical views' grid and the physical steer both depend on a metric on the physical coordinates, whose units differ (positions, momenta, angles, masses); no metric is written *(physics review, silence, R-72, R-452; context: TASK-M6-35)*

*Found by PR #227's physics review (P8, 16 Oct 2026), filed in its fix round as one question for the view and the
steer; RQ-344 point 2 keeps the metric on `z`. Nothing is chosen. For the human (physics).*

- **File, section:** `decisions.md` § R-452 (the human's words, 3.2): "u₁ = J_D q₁, u₂ = J_D q₂, orthonormalised";
  3.4: "J_D⁺ the pseudo-inverse". `docs/design/principia_chart_reference.md` §0.2, §0.3: the positions mass-weighted,
  the momenta physical.
- **The points:** (1) R-452's view orthonormalises `u₁`, `u₂` and normalises `Σ p² = 1` in the physical coordinates'
  Euclidean metric; rescaling one coordinate (the units of momenta against positions, angles against masses) rescales
  its row and column of `p_ij` and can change the dominant cell, so the grid and the dominant pair depend on the
  metric. (2) The steer's `J⁺` depends on the same metric (RQ-344 point 2), unless the set has full rank (RQ-342 and
  RQ-343 point 6, where `J⁺ = J⁻¹`). (3) One choice should serve both, so the grid shows the planes the steer reaches.
- **Options seen:**
  - (a) The Euclidean metric in the coordinates as named, stated as a convention, with the help text saying the grid
    depends on the units.
  - (b) The canonical pairing for Jacobi, `(ρ̃, p_ρ/√μ_ρ)` and `(λ̃, p_λ/√μ_λ)`, which gives a natural metric there
    (the physics review's suggestion), and a stated one for Shape and the masses.
  - (c) Each coordinate scaled by a stated reference range (e.g. its spread over the chart's image), so every row is
    dimensionless.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199, REQ-GUI-200).

## RQ-355: The corpus puts the Lagrange (equilateral) configurations at the shape sphere's poles, `w = ±1`, for any masses, but R-14's mass-weighted map puts them there only at equal masses *(conflict: corpus statement against its own map, R-14, R-50, REQ-INT-003, REQ-CHART-018, REQ-CHART-036; context: TASK-M2-08)*

*Found by TASK-M2-08 while building the landmarks from the shape map. Nothing is chosen; TASK-M2-08's PR does not wait.*

- **File, section** (line numbers at TASK-M2-08's fix-round head):
  - `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's": "φ = polar angle from +w, on the vertical
    axis, 0..π; L⁺ (w = +1) at the top"; and, of the R-12 note, "under this convention the poles are the Lagrange
    points (regular) and every binary collision lies on the equator (w = 0)". R-14's own words name the pole `L⁺`.
  - `docs/design/principia_dd_integrator.md` § "5" test 9 (:297): "equilateral configs → `n_w = ±1` (poles)"; and
    "`L⁺` (bodies 0 → 1 → 2 anticlockwise, equilateral) `→ (0, 0, +1)`", beside "`BC₀₁ → (−1, 0, 0)` for any masses".
  - The same file, § "3.7 The shape readout and winding" (:213): "`w = ±1` at the Lagrange (equilateral) poles"; its
    table (:223) "`L⁺, L⁻` | Lagrange equilateral | north and south poles", headed "(Montgomery's shape sphere, equal
    masses)".
  - `docs/design/principia_chart_reference.md` § "3.1" (:356): "`L⁺` (`w = +1`) is the equilateral triangle with bodies
    0 → 1 → 2 anticlockwise"; § "3.3" (:415): "Under this convention the poles are the Lagrange configurations";
    § "3.3" "Projection" (:429): "the Lagrange configurations sit at the poles and can be compressed to invisibility
    in equirectangular"; § "3.3" (:446): "the Lagrange configurations are the poles"; § "3.4" (:455): "**Lagrange
    configurations** (equilateral) are the two poles. Their exact coordinates depend on the mass ratios — compute them
    from §3.1 rather than hard-coding."
  - `plan/requirements.yaml` REQ-INT-003: "`L⁺` (bodies 0→1→2 anticlockwise) at `(0, 0, +1)`"; REQ-CHART-018's detail:
    "L⁺ (bodies 0→1→2 anticlockwise equilateral) → w = +1"; REQ-CHART-036's verify detail: "L⁺ → (0, 0, +1)".
- **Why they disagree:** under §3.1's map `w = 2(ρ̃ ∧ λ̃)/I`, with `ρ̃ ∧ λ̃ = √(m₀m₁m₂)·2·Area` and
  `I = Σ_{i<j} mᵢmⱼ rᵢⱼ²` (`M = 1`), the unit equilateral triangle has
  `w = √(3m₀m₁m₂)/(m₀m₁ + m₀m₂ + m₁m₂)`: 1 at equal masses only, and below 1 otherwise (0.968 at masses
  (0.2, 0.3, 0.5)). The poles are the triangles of largest area for their `I` in the mass metric, equilateral only when
  the masses are equal. §3.4's own "their exact coordinates depend on the mass ratios" agrees with the map; its "are
  the two poles", and test 9's `n_w = ±1`, hold at equal masses.
- **Options seen:**
  - (a) Qualify the pole statements to equal masses: `L±` are the equilateral (Lagrange) configurations through §3.1
    with the current masses, the poles at equal masses, off them otherwise; test 9's `n_w = ±1` and REQ-INT-003's `L⁺`
    line read "at equal masses", with the general value above.
  - (b) Make `L±` the poles `(0, 0, ±1)` for any masses, the overlay's marks, and say they are the Lagrange
    configurations only at equal masses.
- **Applied meanwhile:** (a), as §3.4 says to compute them: `kernel::shape::landmarks` gives `L±` as `n` of the unit
  equilateral triangle under the current masses. TASK-M2-08's tests assert `L⁺ = (0, 0, +1)` at equal masses and
  `w = √(3m₀m₁m₂)/(m₀m₁ + m₀m₂ + m₁m₂)` for every mass set; no doc line is changed.
- **R-12's conclusion survives either option.** No polar buffer is needed under (a) or (b): at a pole `a = b` and
  `ρ̃ ⊥ λ̃`, a nondegenerate triangle for any masses, a regular point of the flow, and every binary collision stays on
  the equator. Only which configuration the pole is named changes, not the map.
- **Waits:** REQ-INT-003, REQ-CHART-018 and REQ-CHART-036 carry `rq: [RQ-355]`, held, not closed, by TASK-M2-08's PR
  (applied per R-369); the code waits for nothing, option (a) applied. Option (b)
  would change two lines of `landmarks` and the tests' expectation.

## RQ-360: T1 says `E(g·x) = E(x)` for every `g`, the mirror `Π` among them, but on a state `C` leaves unmirrored on both sides of `Π`, every `|λ̃_y| ≤ δ_λ` and every exactly collinear one, `E(Π·x)` and `E(x)` differ in the momenta's `y` components *(conflict, R-82, REQ-ENC-003; context: TASK-M2-15)*

*Found by TASK-M2-15, building T1's sweep. Nothing is chosen.*

- **File, section:**
  - `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
    (:23): "**T1 (well-defined).** `E(g·x) = E(x)` for all `g ∈ G`. Encode is constant on gauge orbits. This
    *requires* deterministic tie-breaking (mirror iff `λ̃_y < −δ_λ`; the tie `|λ̃_y| ≤ δ_λ = 10⁻¹²` resolves to no
    mirror, R-82; …)"; and (:27): "the mirror reflects every `p_i` through the x-axis".
  - `docs/design/principia_dd_encode.md` § "3.3 The gauge group action, explicit (this is what T1 sweeps)" (:106):
    "**T1 asserts `E(g·x) = E(x)` for all such g** — including g's that land `λ̃_y` inside the mirror deadband (the
    tie-break sweep)."
  - `decisions.md` § "R-82 — One mirror test, one seed rule *(closes RQ-33)*": "mirror iff λ̃_y < −δ_λ (λ̃ = √μ_λ·λ),
    δ_λ = 1e-12; |λ̃_y| ≤ δ_λ → no mirror".
  - REQ-ENC-003's statement: "E(g·x) = E(x) for every g = (translation a, boost u, rotation θ, scale s, mirror Π)".
- **Conflict:** `Π = diag(1, −1)` is in `G`, and the mirror acts on the full state, positions and momenta (Part 1).
  Take `x` canonical with `|λ̃_y| ≤ δ_λ`. Then `Π·x` is canonical too, with `λ̃_y` of the other sign, inside the
  deadband, so `C` mirrors neither: `C(Π·x) = Π·C(x)`. The two configurations agree to `2δ_λ`, and `β` reads `|λ̃_y|`
  (R-438), so `z_α` and `z_β` agree; but `p_ρ,y` and `p_λ,y` change sign, so `z_q1` and `z_q3` do, and `L_z` flips.
  `E(Π·x) ≠ E(x)` unless both `y` momenta are 0. Exactly collinear states (`λ̃_y = 0`) are such states for any
  momenta: there `Π` fixes the configuration, and nothing in `C` reads which of `x`, `Π·x` was given. A deterministic
  tie-break makes `E` a function, as T1's sentence asks, but not one constant on the orbit `{x, Π·x}`. Measured at
  f64 on the collinear Euler landmark `ρ̃ = (0.8, 0)`, `λ̃ = (−0.6, 0)`, masses `(0.2, 0.3, 0.5)`, `p_ρ = (0.3, −0.2)`,
  `p_λ = (−0.4, 0.25)`: `E(x)` has `z_q1 = −0.2007`, `z_q3 = 0.2513`; `E(Π·x)` has `+0.2007`, `−0.2513`; the other six
  controls are equal bit for bit, and neither encode mirrors.
- **Options seen:**
  - (a) Restrict T1 and REQ-ENC-003 to `g` that do not mirror where `|λ̃_y| ≤ δ_λ`: on that band (collinear states
    included) `E` is a deterministic function constant on the orbit's turns, translations, boosts and scalings, and
    the mirror's two sides are both kept, as two `z`. The docs say so; the code is unchanged.
  - (b) Break the tie on the band by the momenta, so `E` is constant on the whole orbit: inside the deadband, mirror
    iff a mirror-odd momentum quantity is negative (for one, `L_z`, or `p_λ,y`, then `p_ρ,y` where it is 0), and no
    mirror where it too is 0, the state being then `Π`-invariant. R-82 changes (a ruling), `C`'s decision changes
    (`kernel::canon`, TASK-M2-07's), and that quantity's own tie needs the cross-backend discipline R-82's has (R-34,
    RQ-298).
  - (c) Keep R-82 and read T1 on the band as holding "after float tolerance" in configuration only, the momenta's
    `y` sign left to the input: weaker than T1's sentence, and REQ-ENC-003 reworded.
  - Bearing on all three (physics review of PR #228): inverse-encode Part 8 (:248) already lists "the mirror tie"
    among ROUNDTRIP's "Legitimate bright regions: clamp boundaries, feasibility edges, the mirror tie — *tagged
    expected*, not bugs", which reads with (a) and (c); and under (b) the seam of `E` moves to the band's edges for
    states whose mirror-odd quantity is negative, so `E` is discontinuous somewhere on the band whichever is chosen.
- **Applied meanwhile:** `encode_t1_gauge` (`crates/kernel/tests/encode_core.rs`) sweeps random `g` per §3.3, mirror
  included, on states off the band (`|λ̃_y| ≥ about 1.5e-3`), and the band with `g` that turn, move, boost and rescale
  but do not mirror, which is what REQ-ENC-003's verify detail names ("deadband-straddling rotations"). The mirror tie
  is encoded twice, with the same bits. No state on the band is swept under `Π`.
- **Waits:** REQ-ENC-003 (`rq: RQ-360`): TASK-M2-15's closing of it. TASK-M2-15's PR does not wait: it merges with
  REQ-ENC-003 held, not closed.

## RQ-365: REQ-SYS-015's acceptance scan finds a hand-written WGSL copy of dd_decoder §3.2's Jacobi map, the read side's `shape()`, which RQ-235's R-369 decision put there; does the scan reach read-side derived fields, and if so is REQ-SYS-015 closed before TASK-M2-25 moves it onto the translation? *(conflict, R-116, REQ-SYS-015, RQ-235; context: TASK-M2-15)*

*Found in review of PR #228 (qa's observation, code review 5554105081 C5, physics review 5554127777 P2). Nothing is
chosen. For the human: whether the scan's reach excludes read-side derived fields narrows the task's acceptance
(R-369).*

- **File, section:**
  - `decisions.md` § "R-116 — The fragment decode and encode are generated from the one source *(closes RQ-84)*":
    "The fragment WGSL decode and encode are generated from the one Rust source (rust-gpu → SPIR-V → WGSL translation),
    never hand-written."
  - `plan/requirements.yaml` REQ-SYS-015, statement: "there must be no separate transcriptions of the decode logic and
    no hand-written WGSL decode or encode"; verify detail: "no second implementation of any §3 formula exists (grep for
    the mass/config/momentum formulae outside the shared source, including .wgsl)".
  - `plan/tasks/M2/TASK-M2-15.md` § "Acceptance tests" (:49): "a scan (including `.wgsl` files) finds no second
    implementation of any dd_decoder §3 formula outside the shared source, so any fragment decode/encode can only be
    the translation's output (REQ-SYS-015)".
  - `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi": the formulae
    `M₀₁ = m₀ + m₁`, `μ_ρ = m₀m₁/M₀₁`, `μ_λ = m₂M₀₁`, `ρ = r₁ − r₀`, `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`.
  - `docs/archive/review_queue/M0.md` § "RQ-235" (record only), its decision: "**Decided:** per R-369 (7 Oct 2026), by
    the orchestrator: TASK-M1-12 adds `n` to `ledger::payload` as §3.8's worked entry (`derived`, `vector(f32, 3)`).
    The read side generates its WGSL from the same source as `kernel::shape`, with a twin test, as it does
    `hamiltonian`."
  - `crates/render/frag/generated/read_side.wgsl:146–162`, `fn shape(r, m)`: "`kernel::shape::shape`, operation for
    operation. `ρ = r₁ − r₀`, `λ = r₂ − r₀₁`, `μ_ρ = m₀m₁/M₀₁`, `μ_λ = m₂M₀₁`"; its body
    `let m01 = m.x + m.y; let mu_rho = m.x * m.y / m01; let mu_lambda = m.z * m01; …`. It is a string literal emitted by
    `crates/ledger/src/gen/read.rs:1086–1102`, not an output of the rust-gpu → SPIR-V → WGSL translation.
- **Conflict:** `shape()` is a second, hand-transcribed implementation of §3.2's mass and configuration formulae in a
  `.wgsl` file, which is what REQ-SYS-015's verify detail and the task's acceptance scan look for. RQ-235's decision
  made it "the same source as `kernel::shape`" by a twin test, not by the translation R-116 names. It is not a decode
  or an encode (REQ-SYS-015's statement names those), it is the shape map (R-14) of the read side's derived field `n`,
  and it predates TASK-M2-15 (TASK-M1-12). The physics review confirmed it agrees with dd_decoder §3.2 formula for
  formula, `n = (‖ρ̃‖² − ‖λ̃‖², 2ρ̃·λ̃, 2ρ̃∧λ̃)/I` per R-14, and with `kernel::shape::shape`
  (`crates/kernel/src/shape.rs:43–61`) operation for operation; neither uses an explicit `fma`, so their roundings may
  differ by a backend's contraction, as any Tier-N value does. TASK-M2-15's first scan (`atanh|jacobi|atan2|artanh`)
  missed it; the formula scan (`mu_rho|mu_lambda|m01`) finds it.
- **Options seen:**
  - (a) The scan reaches it: REQ-SYS-015 stays open until the read side's `shape()` is the translation's output, the
    fragment-snippet work of TASK-M2-25 (R-116; the task's Notes: "the fragment snippets are built in TASK-M2-25").
  - (b) The scan's reach is decode and encode only, as REQ-SYS-015's statement words it: read-side derived fields
    with a twin test (RQ-235) are outside it, REQ-SYS-015 closes with TASK-M2-15, and its verify detail is reworded.
  - (c) As (a), but REQ-SYS-015 is split: decode and encode close now, the derived fields' shape map moves to a
    requirement TASK-M2-25 closes.
- **Applied meanwhile:** nothing in the code changes. TASK-M2-15's PR records the hit in its scan section and holds
  REQ-SYS-015 (`rq: RQ-365`).
- **Waits:** REQ-SYS-015 (`rq: RQ-365`): TASK-M2-15's closing of it. TASK-M2-15's PR does not wait: it merges with
  REQ-SYS-015 held, not closed.

## RQ-372: R-467 says every chart is valid inside `[0,1]²`, "charts built in the Chart builder too", and makes an in-square validity failure a decoder bug with a console warning; a builder physical-quantity axis has a range that can leave the slice's reach inside the square, which the Domain preview exists to show *(conflict, R-467, R-407, R-408 B1, R-26, REQ-GUI-109, REQ-GUI-228; context: TASK-M8-18, TASK-M8-47)*

*Found porting R-467 (F2). Nothing is chosen. For the human: yes, it changes what the Chart builder must build.*

- **File, section:**
  - `decisions.md` § "R-467 — …", the human's words: "Inside [0,1]², every chart is valid by construction: the
    feasibility warp, the simplex map, and the decode accepting any z. That holds for charts built in the Chart builder
    too." and clause 4: "Inside [0,1]², a validity failure is a decoder bug: debug view only, plus a console warning
    naming the chart and pixel. The real engine should produce zero."; clause 6: "The Chart builder's Domain preview
    "forbidden in view: N%" (R-408, §G7) stays: it is a diagnostic for building charts, not a presentation view."
  - `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)": "physical quantity (energy,
    `L_z`, virial ratio, mass ratio…, with a range and what is **held fixed** …)"; "a physical-quantity axis mapped
    linearly onto its range is affine, the fallback hatching what fails validation"; "**The Domain preview:** the
    chart's admissible region in its own coordinates, the forbidden region hatched, the current view as a rectangle, the
    boundary's formula, and "forbidden in view: N%"."
  - `plan/requirements.yaml` REQ-GUI-109, verify detail: "N% equals the fraction of a uniform grid over the view that
    validate rejects"; its note: "With the egui layer shown the Chart builder's view is [0,1]²".
  - `decisions.md` § "R-408 — …", B1: "Under this ruling its primary range is unbounded, so its unreachable pixels fail
    validity and count as forbidden."
- **Conflict:** nothing in the corpus makes a builder axis's range feasible on the slice. An energy axis whose range
  reaches below the slice's potential `U` fails `validate`'s layer 2 at those pixels inside `[0,1]²`, and the Domain
  preview's N% over `[0,1]²` is non-zero exactly then. Under clause 4 each such chart would log a "decoder bug" warning
  for a chart the person built that way, while the context says such a chart cannot exist.
- **Options seen:**
  - (a) The builder keeps every chart valid inside the square: a physical-quantity axis's range is clamped or warped to
    the slice's reach (as the invariant charts' warp is, chart_reference §2.1), so N% is 0 inside `[0,1]²` and a
    failure there is a decoder bug, warned; the Domain preview then shows forbidden regions only past the square or
    before Apply.
  - (b) A builder chart may be invalid inside the square: its in-square failures draw the background (presentation)
    and the hatch (debug view), as R-467 settles for every chart, but log no decoder-bug warning; the warning is for the
    gallery charts (R-467 D11's list) only.
  - (c) As (b), with a different, informational console line for a builder chart ("chart <name>: N% of the view is
    outside its domain").
- **Applied meanwhile:** R-467's drawing applies to every chart (R-467 D3, F6); the warning is built for the gallery
  charts and the mock (REQ-GUI-227, TASK-M8-46; REQ-GUI-226, TASK-M6-46). Nothing in the builder changes.
- **Waits:** REQ-GUI-228 (`rq: RQ-372`), a Chart-builder chart's in-square warning, closed by TASK-M8-47, a leaf task
  nothing depends on.
- **R-471 bears on it (20 Oct 2026, a note; the entry stays open, narrowed).** The human ruled that energy-type axes
  are warped by default, every pixel of `[0,1]²` feasible, with a linear mapping as an option whose infeasible pixels
  are "not a system" (E1, E2). For an energy-type builder axis (energy, `L_z`) that is option (a) under the default and
  option (b) under the linear option: no decoder-bug warning for its in-square pixels (R-471 H5). The question stays
  open for the builder's other physical quantities, the virial ratio and the mass ratio, for which nothing is ruled;
  REQ-GUI-228 waits for those alone.

## RQ-373: R-467 changes two of the three outcome goldens R-454 re-baselined under RQ-330, and two field-ramp goldens; which are re-baselined, under what gate decision *(the human's question, R-467 clause 8, R-110, R-454; context: TASK-M2-34)*

*Filed porting R-467 (D15), as the human asked: "The outcome goldens re-baselined under RQ-330 may need another
re-baseline; list them, don't re-baseline without a gate decision." Nothing is re-baselined.*

- **File, section:**
  - `fixtures/golden/BASELINES.md`, the rows `m1-outcome/outcome`, `m1-outcome/outcome_edited` and
    `m1-synthetic/known_escaper` (each "R-454 (re-baselined under R-444 to R-446, RQ-330 …)"), and the rows
    `m1-numeric/length_ramp` and `m1-numeric/d_min_ramp`.
  - `crates/validation/src/golden_scene.rs`, the `outcome` scene: "a sample whose chart point is forbidden
    ([`OUTCOME_FORBIDDEN`]), a `decode_failed` one, drawn with the hatch (REQ-COL-101, REQ-COL-102)", tile 22; the
    `length_ramp` and `d_min_ramp` scenes: "127 in the invalid pattern", "NaN in the invalid pattern".
  - `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*": no re-baselining without
    a gate decision.
- **What changes under R-467:**
  - `m1-outcome/outcome` and `m1-outcome/outcome_edited`: tile 22, the forbidden sample, from the hatch to the canvas
    background (R-467 D2, D3). No other tile changes.
  - `m1-synthetic/known_escaper`: no forbidden sample and no hatched pixel; expected unchanged.
  - `m1-numeric/length_ramp` and `m1-numeric/d_min_ramp`: a field ramp is a presentation occupant (R-467 D1), so its
    invalid value draws `#141418` (D4) where the hatch was; their `_override` twins draw the node's override colour and
    are expected unchanged.
  - The debug catalogue's goldens do not change (D1).
- **Options seen:**
  - (a) Re-baseline the changed goldens under R-467 as TASK-M2-34 renders them, each `BASELINES.md` row leading with the
    ruling, and confirm the canvas background's value and the changed tiles at the M2 gate, with REQ-COL-110's NaN /
    Inf style.
  - (b) As (a), but `length_ramp` and `d_min_ramp` keep the hatch: a field ramp is read as a debug occupant, which
    narrows R-467 D1.
  - (c) Hold the re-baseline for the M2 gate itself, TASK-M2-34 waiting until then.
- **Applied meanwhile:** nothing re-baselined. TASK-M2-34 is built and reviewed; its PR reports the hashes that
  change and waits for this ruling before it merges.
- **Waits:** TASK-M2-34's merge (REQ-COL-108's golden line, `rq: RQ-373`).

## RQ-375: T4's units: mixing a chart parameter with a hidden coordinate under a tilt needs a common scale; the human proposes each screen axis in units of the window's half-width and the hidden coordinate in units of its own slider span, and on the latent chart that proposal departs from T2's "exactly the decoder's basis edit" *(the human's queued question, physics, R-473 T4, R-469 §11; context: TASK-M2-36, TASK-M2-39)*

*Queued by the human (R-469 §11: "T4's units for mixing a chart parameter with a hidden coordinate"; R-473 T4: "Queue
it as an RQ with this as the proposal; do not settle it silently"). For the human: yes (physics, ⚑).*

- **File, section:**
  - `decisions.md` § "R-473 — …", the human's words: "**T4. Units.** Mixing a chart parameter with a hidden coordinate
    needs a common scale. Proposed: each screen axis in units of the window half-width; the hidden coordinate in units
    of its own slider span. This makes `τ = 45°` mean the same visual lean on every chart."
  - The same, T2: "A pixel at window offset `(u, v)` maps to `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`" and "On the latent
    chart, T2 is exactly the decoder's basis edit."
  - `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)" ("Tilt on every chart (R-473)"): the
    rotation acts on the chart's own parameters, the scale queued here.
- **The question:** in what units `a`, `b` (the chart's own parameters, in their axes' units) and `d` (a hidden latent
  control, in its own) are compared, so that a rotation between them means something. The latent chart's basis edit
  compares `q₁`, `q₂` and a unit latent direction in the latent metric; under the proposal the hidden coordinate is
  scaled by its slider span, which on the latent chart changes what a given `τ` does there.
- **Options seen:**
  - (a) The proposal on every chart: each screen axis in half-widths of the window, the hidden coordinate in its slider
    span. `τ = 45°` is the same visual lean everywhere; on the latent chart the basis edit is no longer the contract's
    unit-vector form.
  - (b) The proposal on curved charts only, the latent chart keeping the decoder's basis edit exactly (its own units,
    the latent metric), so T2's latent sentence holds as written.
  - (c) One metric for all charts, the latent metric pulled back through each chart map (needs the decoder's Jacobian
    and a metric, which RQ-341 and RQ-346 raise).
- **Applied meanwhile:** (b), per R-369 (R-473 J3): the proposal on the curved charts, the latent chart unchanged.
- **Waits:** REQ-CHART-064 (`rq: RQ-375`), the units' requirement, closed by TASK-M2-39, a leaf task nothing depends
  on. The tilt itself (TASK-M2-36) is built under (b).

## RQ-376: Whether an energy chart's `t > 1` side (`K > K_max`, feasible) should be declared affine rather than bounded *(the human's queued question, physics, R-469 §11, R-407 clause 3, R-471; context: TASK-M2-37)*

*Queued by the human (R-469 §11: "Whether an energy chart's `t > 1` side (`K > K_max`, feasible) should be declared
affine rather than bounded. chart_reference §2.1 leaves this for physics review."). For the human: yes (physics).*

- **File, section:**
  - `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid": "Its
    `t > 1` side (`K > K_max`) stays feasible; declaring it would be a later declaration, needing physics review
    (R-407, clause 3), and is not made."
  - The same, § "5.4 …": "Its `t > 1` side (`K > K_max`, feasible at `L_z = 0`) is left undeclared, as on the warp
    (R-407, A5)."
  - `decisions.md` § "R-471 — …": energy-type axes are warped by default, so every energy-type axis has this side.
- **The question:** past `t = 1` the warp's `K = K_max·t^γ_K` is still feasible (`K > K_max ≥ 0`), so the axis could
  continue as affine there, while past `t = 0` it cannot. Bounded draws that side blank (R-470) and leaves it out of the
  count (R-408).
- **Options seen:**
  - (a) Bounded on both sides, as today: past `t > 1` the domain's end, blank.
  - (b) Affine past `t = 1`, bounded past `t = 0`: a one-sided declaration, `K` continuing beyond `K_max` (for `E` and
    `L_z` the same through their lines of the warp, R-471 H1), counted as R-408 counts an affine axis.
  - (c) Affine past `t = 1` for `K` and `E` only, `L_z`'s `±√(2 I K_max)` range staying bounded.
- **Applied meanwhile:** (a), the corpus as written. Nothing waits; a ruling for (b) or (c) is a follow-up change to
  REQ-CHART-067's extension type and REQ-CHART-056's statistic.

## RQ-377: `principia_lines.md` §11's six open questions: the expression language, the playhead quantities, the trace fade's off-plane distance, label collisions, class edges, and new contour sets' default style *(the human's queued questions, R-476 L5, R-469 §11; context: TASK-M7-42, TASK-M7-43, TASK-M8-49, TASK-M8-50, TASK-M8-51)*

*Queued by the human (R-469 §11: "`principia_lines.md` §11's six questions"; R-476 L5: "Queue them as RQ entries; do
not rule on them"). One entry with six parts, one per question (R-476 M10). For the human: yes, each part, as each
changes what the line system builds.*

- **File, section:** `docs/design/principia_lines.md` § "11. Open questions", quoted in each part below; read with
  § "4.1 The expression", § "7. Labels" and § "8. Traces".
- **(1) Expression language.** "A purpose-built mini-language (proposed; total, sandboxed, GPU-compilable), or reuse the
  stain graph's Code view syntax?"
  - Options: (a) the purpose-built language (§4.1, proposed); (b) the stain graph's Code view syntax.
  - Applied meanwhile: contour expressions are held as §4.1's tree (its variables, operators, functions and totality),
    built and tested without a surface syntax; the contour-set editor picks a variable or one of §4.1's examples from a
    list.
  - Waits: REQ-RENDER-097's typed expression (the parser and its error reporting), closed by TASK-M8-51.
- **(2) Playhead quantities.** "Allow any payload field, or a curated list?"
  - Options: (a) any payload field; (b) a curated list.
  - Applied meanwhile: the fields §4.1's table names (`t_end`, `state`, `d_min`, `drift`, `ftle`).
  - Waits: REQ-RENDER-097's playhead clause, closed by TASK-M8-51.
- **(3) Off-plane distance for the trace fade.** "The general definition is the distance in z from the trajectory's
  encoded point to the slice plane, which needs the encode at every step (and the metric RQ-341 raises). The mock-up uses
  a proxy: the scale-normalised Jacobi momentum magnitude, which this shape chart holds at zero."
  - Options: (a) the general definition, the encode per step and a metric (RQ-341, RQ-346); (b) the mock-ups' proxy;
    (c) a per-chart distance, each chart naming its own.
  - Applied meanwhile: no fade; every trace draws at full opacity (`fade: None`).
  - Waits: REQ-GUI-249 (`rq: RQ-377`), closed by TASK-M8-51.
- **(4) Label collisions.** "Between contour labels and the trace's tick labels: which wins?"
  - Options: (a) contour labels win; (b) the trace's tick labels win; (c) the later in the Lines list wins, as §7 orders
    contour sets.
  - Applied meanwhile: §7's rule within the contour sets; between contour labels and a trace's ticks nothing is
    suppressed.
  - Waits: REQ-RENDER-098 (`rq: RQ-377`), closed by TASK-M8-51.
- **(5) Class edges.** "Should they move onto the line system as "contours of a categorical field", or stay in the
  stain?"
  - Options: (a) move them onto the line system; (b) keep them in the stain, an Overlays ▾ toggle.
  - Applied meanwhile: (b), as today (R-476 M3 keeps class edges in Overlays ▾). Nothing waits.
- **(6) Default style for new contour sets.** "Screen contour (white with outline), or smart ink on by default?"
  - Options: (a) Screen contour; (b) smart ink on.
  - Applied meanwhile: (a), Screen contour, the library's first style (§3).
  - Waits: REQ-GUI-250 (`rq: RQ-377`), the default's confirmation, closed by TASK-M8-51.
- **Waits, in all:** REQ-RENDER-097, REQ-RENDER-098, REQ-GUI-249 and REQ-GUI-250, each closed by TASK-M8-51, a leaf
  task nothing depends on; the rest of the line system is built meanwhile.

## RQ-378: Whether "Save view as chart" on a curved chart should also offer a flattened oblique-latent version, where one exists locally *(the human's queued question, R-469 §11, R-475; context: TASK-M2-40, TASK-M8-48)*

*Queued by the human (R-469 §11: "Whether "Save view as chart" on a curved chart should also offer a flattened
oblique-latent version, where one exists locally."). For the human: yes, it changes what Save view as chart builds.*

- **File, section:**
  - `decisions.md` § "R-475 — …", the human's words, S2: on a curved chart the saved chart is "the parent chart, its
    slice centre, and the tilt (map, angles, targets)"; on a latent view, "`(z₀, q₁, q₂)` as an oblique chart".
  - `docs/gui/principia_render_gui_spec.md` § "G2. …", the Chart bullet's Save view as chart.
- **The question:** a curved view near its centre is approximated by its tangent plane, an oblique latent chart
  `(z₀, J q̂₁, J q̂₂)`; whether saving should also offer that flattened chart, and what "exists locally" requires (a
  full-rank Jacobian at the centre, the window small enough).
- **Options seen:** (a) no, the parent chart and its tilt only (S2 as written); (b) yes, a second option "Save as
  flat chart", offered where the decoder's Jacobian at the centre has rank 2 (it needs the derivative RQ-341 raises);
  (c) yes, with the flattened chart shown only as provenance, not offered.
- **Applied meanwhile:** (a). Nothing waits.

## RQ-379: A `decode_failed` sample the chart does not classify as not a system (a non-finite decode, η_E's `ENERGY_UNSCALABLE` and `ENERGY_INFEASIBLE`, the mass block's `M01_TINY`): blank, as P1 suggests, or `#141418`, as R-467 clause 2 has it *(conflict, R-469 P1, R-470 N1 and N3, R-467 clause 2, REQ-COL-108; context: TASK-M2-34)*

*Found porting R-470 (G2). For the human: yes, it changes what a presentation view draws.*

- **File, section:**
  - `decisions.md` § "R-469 — …", the human's words, P1: "Every pixel a chart draws is either a valid three-body system
    or explicitly "not a system"."
  - `decisions.md` § "R-470 — …", N1: "A pixel that is not a system draws **blank** … This covers: a bounded axis past
    its edge; layer-2 infeasibility; a failed `Φ`; a layer-3 failure." and N3: "NaN, Inf, `sim_failed`,
    `decode_failed`, and the classification behind the hatch are shown **only in the debug views**".
  - `decisions.md` § "R-467 — …", the human's words, clause 2: "Non-finite values (NaN, Inf), sim_failed and
    decode_failed likewise never appear as a category in a presentation view. They keep R-446's black #141418."
  - `docs/design/principia_colour_composition.md` § "1.4 …": "the decoder's own outcomes, its `DEGENERATE` tags … and a
    non-finite decode, never mark a pixel forbidden; such a pixel draws black unless the types or `validate` hatch it".
- **The conflict:** such a sample has no decoded state to integrate, so it is no system, which P1 makes "explicitly
  not a system", blank; N1's list does not name it, and R-467 clause 2 draws `decode_failed` `#141418`, which N3's
  "shown only in the debug views" is consistent with (black is not a category).
- **Options seen:** (a) `#141418`, R-467 D4 as it stands; (b) blank, as a not-a-system pixel, the forbidden-region
  overlay then drawing over it too; (c) blank for the decoder's `DEGENERATE` tags, `#141418` for a non-finite decode.
- **Applied meanwhile:** (a), per R-369 (R-470 G2). Nothing waits; a ruling for (b) or (c) is a follow-up change to
  REQ-COL-108 and REQ-COL-111.

## RQ-380: Whether a chart preset may optionally pin links ("designed under stick-breaking"), and what loading it does if the current links differ *(the human's queued question, R-478 W4, the link-functions instruction §6; context: TASK-M6-50, TASK-M8-52)*

*Queued by the human (the link-functions instruction, §6: "Queue, don't rule"). For the human: yes, it changes what a
preset carries and what loading one does.*

- **File, section:**
  - `decisions.md` § "R-478 — …", the human's words, W4: "Chart presets do **not** carry links. A preset is a pair of
    axes. Whether a preset may optionally pin a link is queued (§6)." and §6: "Whether a chart preset may optionally pin
    links ("designed under stick-breaking"), and what loading it does if the current links differ."
  - The same, W1: "Switching chart must not silently switch links, because the robustness test Part 2.5 asks for is
    exactly "same chart, different link"."
  - `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)": "Presets are saved pairs of
    axes and are editable".
- **The question:** a preset designed under a non-default decoder (the masses near an edge, where only
  `stick_breaking` reaches) may want to say so; W1 forbids switching links silently.
- **Options seen:**
  - (a) No pin: presets carry no links (W4 as written); a preset's note may name the decoder it was designed under, as
    text.
  - (b) An optional pin, recorded with the preset; loading it with other links active offers "Switch to the pinned
    links" beside the load, never switching on its own, and the Decoder window's row shows the pinned choice.
  - (c) An optional pin that switches the links on load, one undo entry with the chart, the amber row and the badge
    showing the change (R-479).
  - Saved charts (R-475) are charts and are covered by the same answer.
- **Applied meanwhile:** (a), W4 as written. Nothing waits; a ruling for (b) or (c) is a follow-up change to
  REQ-GUI-254 and REQ-GUI-108.

## RQ-381: Whether the Decoder window should offer a one-click robustness comparison, rendering the current view under the default links and under the active ones side by side *(the human's queued question, R-478, the link-functions instruction §6, R-106; context: TASK-M8-52, TASK-M8-36)*

*Queued by the human (§6). For the human: yes, it adds a view to the GUI.*

- **File, section:**
  - `decisions.md` § "R-478 — …", the human's words, §6: "Whether the Decoder window should offer a one-click
    **robustness comparison**, rendering the current view under the default links and under the active ones side by
    side. This overlaps the v2 side-by-side view (R-106)."
  - `docs/contracts/principia_chart_decoder_contract.md` § "Integrity: the link is part of the experiment", item 3:
    "**Falsifiable by re-measurement.** The real test: does a finding *survive* a link swap?"
  - `decisions.md` § "R-106 — The link ids are the chart's link functions *(closes RQ-66)*": "Linked views for side by
    side are a separate `ViewUI` item, specified with the v2 research tools."
- **The question:** whether the comparison is a Decoder-window command now, or waits for the v2 side-by-side view
  (render_gui_spec §G11, TASK-M8-36) and is one of its uses.
- **Options seen:**
  - (a) Not now: the comparison is made by hand (Apply, look, Revert), and the v2 side-by-side view gains "compare
    links" when it is built.
  - (b) A Decoder-window button now, opening a two-pane comparison of the current view under the defaults and the
    active links, both integrated (twice the cost, priced before it runs).
  - (c) As (b), but built as the first use of the v2 side-by-side view, moving that view's linked-camera part forward.
- **Applied meanwhile:** (a). Nothing waits.

## RQ-382: Whether α's range (`α_min`) should become a tunable link parameter *(the human's queued question, physics, R-481 A1, R-21, the link-functions instruction §6; context: TASK-M2-41)*

*Queued by the human (§6; A1: "The α range (`α_min`, R-21) stays fixed unless physics review says otherwise"). For the
human: yes (physics).*

- **File, section:**
  - `decisions.md` § "R-481 — …", the human's words, A1: "The α range (`α_min`, R-21) stays fixed unless physics review
    says otherwise." and §6: "Whether α's range (`α_min`) should become a parameter."
  - `decisions.md` § "R-21 — `α_min = 0` *(CD-1)*": "Full sphere; the poles are represented and fenced by the
    collision detector, the conditioning readout and the SAT flags. `α_min` is not a numerical guard."
  - `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part
    2.5)": the α entries declare and read `alpha_min`; α's codomain is `[α_min, π/2 − α_min]`.
- **The question:** a tunable `α_min` would let a view exclude the poles' neighbourhoods by a choice of measure; R-21
  ruled the full sphere and that `α_min` is no guard.
- **Options seen:** (a) fixed at 0 (R-21, A1 as written); (b) tunable on the α entries, default 0, its range under
  physics review (the poles' conditioning, the collision fence at `α = 0`); (c) tunable, but only on a chart that
  declares it, recorded in the chart's params rather than the links.
- **Applied meanwhile:** (a). Nothing waits; the previews draw α's codomain from its bounds (R-480), so they follow any
  ruling.

## RQ-383: Whether new link registry entries are wanted, links that disagree enough to stress a claim, such as a heavier-tailed bounded map *(the human's queued question, the link-functions instruction §6, chart_decoder_contract Part 2.5; context: TASK-M2-41)*

*Queued by the human (§6). For the human: yes, each new entry is a definition (R-72) with a physics review.*

- **File, section:**
  - `decisions.md` § "R-478 — …", the human's words, §6: "Whether new registry entries are wanted. Part 2.5 asks for
    links that **disagree enough to stress a claim**, such as a heavier-tailed bounded map, not three flavours of the
    same bias."
  - `docs/contracts/principia_chart_decoder_contract.md` § "Integrity: the link is part of the experiment", "What makes
    an alternative worth offering": "The registry should span genuinely different sampling densities (softmax vs. a
    simplex map that reaches the edges; `t^γ` at spread-apart γ; sigmoid vs. a heavier-tailed bounded map)".
  - `docs/design/principia_dd_generation_root.md` § "3.9 …": the registry's entries, among them `softsign_*` (R-72),
    "heavier-tailed than σ", and "Temperature-softmax is not registered. The corpus gives no formula or parameter for
    it".
- **The question:** today each block has two or three entries (REQ-GEN-014); `tanh_*` is "interchangeable with σ up to
  reparam", so the bounded blocks have two genuinely different densities, σ and softsign. Whether more are wanted, and
  which.
- **Options seen:** (a) none now; (b) a temperature-softmax on the masses, with its parameter tunable under R-481; (c) a
  bounded map heavier-tailed than softsign (an algebraic `x/(1 + |x|^k)^(1/k)` family, `k` tunable); (d) a symmetric
  Dirichlet-measure simplex map, uniform over the simplex.
- **Applied meanwhile:** (a). Nothing waits.

## RQ-384: Whether link parameters may be interpolated across keyframes in a blocking export, re-integrating every frame *(the human's queued question, R-483 X7; context: TASK-M8-31)*

*Queued by the human (the export follow-up, "Add to §6": "Whether link parameters may be interpolated across
keyframes in a blocking export (X7)."). For the human: yes, it changes what an animation can describe.*

- **File, section:**
  - `decisions.md` § "R-483 — …", the human's words, X7: "A keyframe's `SimConfig` carries `links`, so a link change
    between keyframes is a sim-key change at that keyframe (a cut, not an interpolation). Whether link parameters may
    be **interpolated** across keyframes is queued (§6). It would re-integrate every frame, which the blocking exporter
    could afford and live playback could not."
  - `docs/contracts/principia_export_animation_contract.md` § "Part 3 — The keyframe system (retained)": "chart
    *switches* are cuts or dissolves (two renders composited), never parameter interpolation across chart types", and §
    "Part 2 — The two playback types and their costs", (b): when a sim-key parameter interpolates per frame, "**each
    frame is a different family of ICs**".
- **Options seen:** (a) no: a parameter change is a cut, as X7 has it for links; (b) yes in a blocking export only,
  eased per its scale (linear in `μ_max` or `q_max`), each frame re-integrated, refused in live playback; (c) yes, and
  live playback shows it as a dissolve between the two end values.
- **Applied meanwhile:** (a). Nothing waits; a ruling for (b) is a follow-up change to REQ-TOOL-175 and REQ-TOOL-075.

## RQ-385: Whether each recorded link carries its own registry-entry hash, or the schema hash suffices to detect a changed link definition *(the human's queued question, R-483 X4; context: TASK-M7-31, TASK-M7-45)*

*Queued by the human (the export follow-up, "Add to §6": "Whether each recorded link carries its own registry-entry
hash, or the schema hash suffices (X4)."; X4: "say in the plan whether that is enough"). For the human: yes, it
changes what a record holds and what a reader refuses.*

- **File, section:**
  - `decisions.md` § "R-483 — …", the human's words, X4: "**Changed definition.** A link id whose registry entry's
    content hash differs from the writer's is refused the same way. The schema hash already covers registry entries by
    semantic content (`crates/ledger/src/version.rs`); say in the plan whether that is enough or whether each recorded
    link should carry its own entry hash." and C11: "A record naming an unknown link id, or one whose entry hash
    differs, reads as "state present, cannot recreate", naming the link."
  - `docs/design/principia_dd_generation_root.md` § "3.9 …", "The hash (R-340)": the schema version hashes every entry,
    used or not, and the chart constants; "Changing a hashed member of any entry, or adding or removing an entry,
    changes the schema version".
  - `docs/design/principia_dd_image_embedding.md` § "6. What travels": the build hash's line, "REFUSE to recreate
    silently across a version where the decoder changed."
- **The question:** the schema version is one hash of the whole ledger: payload layout, register constants and every
  link entry. It changes when any of them changes, so a reader that compares it cannot tell a record whose own links
  changed from one whose links are untouched while an unrelated entry or the payload layout moved, and cannot name the
  changed link, which C11 asks for.
- **Options seen:**
  - (a) The schema hash suffices: a record whose schema version differs from the reader's is refused, naming the
    record's links as possibly changed. No new field. It over-refuses: every record crosses a schema change refused,
    whatever changed.
  - (b) Each recorded link carries its entry's hash: the hash of that entry's canonical bytes (dd_generation_root §3.9's
    form, the entry and the declarations of its tunable parameters), a u64 written as a string (R-322), in the sidecar
    beside the build hash and schema version (not in `SimConfig`, since it derives from the build's registry and the
    ids, not from the view). The reader compares each block's against its own registry's entry: a mismatch names the
    link; a match recreates, even across an unrelated schema change, if nothing else in the record's tier 1 changed
    meaning. About 100 B of JSON per record, four hashes.
  - (c) Both: (b)'s per-link check, and (a)'s refusal across a schema change unless every recorded link's hash matches
    and the payload layout is unchanged.
- **Recommended:** (b), with the schema version kept as the record's whole-ledger check for the payload's meaning. It
  is the only option that satisfies C11 as written ("naming the link"), refuses exactly the records whose decoder
  changed, and costs four short strings; (a) is safe but refuses every old record at each ledger change.
- **Applied meanwhile:** (a), the conservative reading, which never recreates silently across a change (R-483 H4 (b)).
  **Waits:** REQ-TOOL-172 (`rq: RQ-385`), the per-link arm and C11's "entry hash differs" arm, closed by TASK-M7-45, a
  leaf task nothing depends on; the unknown-id arm and "never substitute" (REQ-TOOL-171) are built meanwhile
  (TASK-M7-31).

## RQ-386: Whether tier 2 of the embedding should split further, the ramp window and colour mode above the overlays and lines, if measurement shows tier 2 itself often does not fit at small sizes *(the human's queued question, R-484 RT2; context: TASK-M7-44)*

*Queued by the human (the round-trip rule, "Queue, don't rule"). For the human: yes, it changes what a small image
carries.*

- **File, section:**
  - `decisions.md` § "R-484 — …", the human's words: "Whether tier 2 should split further, for example ramp window and
    colour mode above overlays and lines, if measurement shows tier 2 itself often does not fit at small sizes." and
    RT2's table: "**2. Colour** | `RenderState` as built-ins: colour mode enum and every parameter, the ramp window,
    overlays and the line system."
  - `docs/design/principia_dd_image_embedding.md` § "6. What travels": "**The ramp window is not optional.** An
    auto-ranged ramp manufactures or hides the difference it is meant to show".
  - The same, § "7. Measured capacity and behaviour", to be re-measured with the tiers (R-483 H6, TASK-M7-44).
- **Options seen:** (a) tier 2 whole, as RT2 has it; (b) tier 2a (the stain graph's built-ins, their parameters and the
  ramp window) above tier 2b (overlays, lines, the playhead and hover, `f3_fill`), 2b dropping first; (c) decided by
  TASK-M7-44's measurement: split only if tier 2 fails to fit at 64² or 128² for a typical stain.
- **Applied meanwhile:** (a). Nothing waits; TASK-M7-44's measurement reports tier 2's size so the human can rule with
  it.

## RQ-387: D4's "canvas corner badge beside the chart name": the figure has no chart-name badge, and §G1 says the figure is never covered, the Home button its one control; whether the decoder badge is a second exception *(conflict, R-479 D4, R-466 K7, render_gui_spec §G1, GUI_DESIGN_NOTES; context: TASK-M6-50, TASK-M8-52)*

*Found porting R-479 (E7). For the human: yes, it changes what the figure shows.*

- **File, section:**
  - `decisions.md` § "R-479 — …", the human's words, D4: "A non-default decoder shows in the canvas corner badge beside
    the chart name, for example "z_α × z_β · stick-breaking masses", and in every sidecar and export."
  - `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere": "**The figure is never covered.** No
    toasts or labels over the plot. Warnings go to the footer console (§G12). The hover label and the lock reticle are
    the figure's own marks, not notifications. The one control over the figure is the **Home** button in its
    top-right corner, by the human's words (§G2's Figure, R-466 K7)."
  - `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere": "**The figure is never covered.** No toasts or
    labels over the plot."
- **The conflict:** no canvas badge names the chart today (the chart is named in the Manifold view's Chart section);
  D4 places the decoder's name in one, over the figure, which §G1 forbids for a label.
- **Options seen:**
  - (a) D4 as written: a badge in the canvas's top-left corner naming the chart and any non-default block, hidden with
    the layer, a second exception to §G1 beside the Home button.
  - (b) The badge outside the figure: in the figure's frame above the plot, or in the top bar's status, the figure
    uncovered; the amber Chart row (W2) stays.
  - (c) (a) only while the decoder is off its default, the chart named alone nowhere on the figure.
- **Applied meanwhile:** (a), D4 being the human's later and more specific words (R-479 E7). Nothing waits; a ruling
  for (b) or (c) is a follow-up change to REQ-GUI-258.
