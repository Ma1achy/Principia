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

## RQ-284: the corpus does not say how the descriptor's infinite values at the poles are stored or consumed, and differences against `E₀ = −∞` read NaN *(silence, R-79, REQ-COL-006, REQ-COL-060, REQ-VAL-027; context: TASK-M2-03 and the tasks of those requirements)*

*Found by TASK-M2-03's review (physics review 5526305435 E2). Nothing is chosen; TASK-M2-03's PR does not wait.*

- **File, section:**
  - `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities", "The collision pole reaches 0"
    (written by TASK-M2-03): "`V_0` reads `−∞` and `virial_ratio` 0"; and `rho_ratio` "reads `+∞` only where `‖λ̃‖`
    is exactly 0", which TASK-M2-03 measures at `α = 0`. So `E₀ = K₀ + V₀` reads `−∞` at the f64 collision pole.
  - `docs/design/principia_colour_composition.md` § "6. Debug views as presets" (:489): "**Agreement presets** (new,
    and better than the originals) = `|E(fragment-decode) − ctx.payload.E₀|`" (REQ-COL-006; its tolerance
    REQ-COL-060).
  - `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)" (:424): "| Energy agreement |
    `E_0` (SimState) vs `K_0 + V_0` (ICDescriptor), log \|Δ\| |" (REQ-VAL-027).
  - `decisions.md` § "R-79 — NaN and sentinels": "Storage never holds NaN." It says nothing of ±∞.
  - `docs/design/principia_dd_decoder.md` § 3.2 (:95): "exact-pole `atan2(0,0)=0` yields a deterministic canonical
    value rather than a NaN", which also speaks only to NaN.
  - `docs/notes/principia_gpu_determinism_note.md` § "The discipline", rule 4: "WGSL cannot express it" (an infinite
    literal). The WGSL specification lets an implementation assume no infinities in floating-point evaluation.
  - The generated debug views that read a stored ±∞ (render_gui_spec §10.1's numeric template; physics review
    5529362163 F3): `crates/render/frag/debug/generated/rho_ratio.wgsl`, whose log compaction
    `1 − 1/(1 + log(1 + abs(raw)/ε))` reads `+∞` at `α = 0` (raw 1 on the CPU);
    `crates/render/frag/debug/generated/V_0.wgsl`, `lin` with `RANGE_AUTO = 1`, whose measured range takes `−∞` as
    its lower end at the collision pole; and by extension `crates/render/frag/debug/generated/E_0.wgsl` (`E₀ = K_0 + V_0`, also `RANGE_AUTO = 1`).
- **The silence:** at such a pixel `|E − E₀|` and `|E_0 − (K_0 + V_0)|` are `(−∞) − (−∞) = NaN` on the CPU, and on the
  GPU the result is not pinned at all, so the agreement presets' and the energy-agreement view's parity on those
  pixels is undefined. Nothing says whether an infinite descriptor value may be stored, whether it is a sentinel, or
  how a difference against it is coloured. The same holds for the debug views above: their GPU result at a stored ±∞,
  and an auto range whose measured end is infinite, are unpinned.
- **Options seen:**
  - (a) Allow ±∞ in storage at the poles, and have each consumer of a difference against `E₀` route a non-finite
    operand to the invalid colour (R-79's NaN rule extended to ±∞), on the CPU and the GPU alike.
  - (b) Store a finite sentinel for an infinite descriptor value, declared in the ledger, and have consumers test it.
  - (c) Exclude the poles from the agreement checks by the collision flag the pipeline already raises there.
- **Applied meanwhile:** TASK-M2-03 records the infinite values in §3.6 and stores them as computed; no consumer of a
  difference against `E₀` exists yet.
- **Waits:** the tasks of REQ-COL-006, REQ-COL-060 and REQ-VAL-027, at the point where they consume `E₀` or
  `K_0 + V_0`; and the debug views of `rho_ratio`, `V_0` and `E_0` (generated now; their goldens' showcase values are
  finite, so nothing renders ±∞ yet), whose GPU reading of a stored ±∞ waits on the ruling. TASK-M2-03 does not wait.

## RQ-285: dd_decoder §3.2 calls the exact-pole `atan2(0,0)=0` "a deterministic canonical value", which holds on the CPU only: GLSL.std.450 `Atan2` is undefined at (0, 0) *(conflict, R-116, R-417, REQ-COL-006; context: TASK-M2-03, the encode and canonicalisation tasks, the fragment-side decode/encode)*

*Found by TASK-M2-03's fix round (code review 5527358480 C2). Nothing is chosen; TASK-M2-03's PR does not wait: its
`rho_angle` defines the pole value without `atan2` (dd_decoder §3.6).*

- **File, section:**
  - `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi" (:95): "A config
    at a pole, reached at underflow or ingested there, is caught by those flags, and exact-pole `atan2(0,0)=0` yields
    a deterministic canonical value rather than a NaN."
  - The same doc, § "3.3 Canonicalise" (:114): "rotate all rᵢ, pᵢ by R(−φ),  φ = atan2(ρ_y, ρ_x)", which reads
    `atan2(0, 0)` at the `α = π/2` collision pole (`ρ = 0`); and `docs/design/principia_chart_reference.md` § "0.4
    Canonicalisation `C`" (:96), the same line.
  - REQ-COL-006 (`docs/design/principia_colour_composition.md` § "3. The `ctx` contract", § "6. Debug views as
    presets"; R-116): the chart decode and encode are generated as WGSL from the one Rust source, so fragment presets
    recompute `encode(decode(z))` on the GPU.
  - `docs/notes/principia_gpu_determinism_note.md` § "The discipline": "a branch is frozen or a comparison, never
    backend-elective floating point".
- **The conflict:** `atan2(0, 0) = 0` is the CPU libm's value (IEEE 754's recommended `atan2(±0, +0) = ±0`). On the
  GPU, GLSL.std.450 `Atan2` (what SPIR-V and naga lower `atan2` to) gives an undefined result when both operands are
  0, and WGSL's `atan2` inherits it. §3.2's "deterministic canonical value rather than a NaN" therefore holds on the
  CPU-f64 path only; on the fragment-side encode, and on any GPU-compiled canonicalisation, the pole's `φ` (and the
  rotation and mirror test that follow it) is backend-elective, possibly NaN, so the two paths need not agree there.
  The sign of a zero also matters: `atan2(−0, x < 0) = −π` (TASK-M2-03 measured it for `rho_angle`).
- **Options seen:**
  - (a) Define the pole's value in the corpus and have every `atan2` at a possible pole chosen branch-free by a select
    on its operands against 0, as dd_decoder §3.6 now does for `rho_angle`; reword §3.2:95 to say the value is
    defined, not obtained from `atan2`.
  - (b) Keep `atan2` and restrict §3.2:95 to the CPU path, with the GPU parity checks excluding the poles by the flags
    that already catch them.
- **Applied meanwhile:** nothing outside `rho_angle`; no canonicalisation or encode code exists yet.
- **Waits:** the tasks that implement §3.3's canonicalisation and the encode (and their WGSL generation, REQ-COL-006).
