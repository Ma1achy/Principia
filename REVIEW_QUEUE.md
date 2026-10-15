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
