# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

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
- **Parked:** R-510 (20 Oct 2026), the human: "RQ-378: parked until RQ-341." A deferral by ruling: the entry stays
  open, blocked on RQ-341, and returns to the human when RQ-341 is ruled; (a) stays applied and nothing waits.
- **21 Oct 2026:** RQ-341 is ruled (R-550), so R-510's parking has ended and the entry is with the human again. The
  human's batch of 21 Oct leaves it open (R-545); (a) stays applied and nothing waits.

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
- **Parked:** R-522 (20 Oct 2026), the human: "RQ-386 after the capacity re-measure." A deferral by ruling: the entry
  stays open, waiting on TASK-M7-44's capacity re-measure (REQ-TOOL-174), which reports tier 2's own size and, at
  64² and 128², whether tiers 1 and 2 fit; it then returns to the human with those numbers. Meanwhile (a), tier 2
  whole, stands, and nothing else waits.

## RQ-391: Whether the Export Styles' frames (Polaroid border, banknote, poster) become a later "Frame" stage after the Print graph *(the human's queued question, R-485, §0 "separate, later", the message's point 6; context: none until ruled)*

*Queued by the human (point 6, as RQ-388), and §0: "On presentation frames (Polaroid, banknote, posters): separate,
later." For the human: yes, whether there is a Frame stage at all.*

- **File, section:** `docs/design/principia_print_graph.md` § "12. Open questions (for the review queue, not the agent
  to settle)", question 4: "Should the **Export Styles** frames (Polaroid border, banknote, poster) become a later
  "Frame" stage after the Print graph? Ruled out of scope for this bundle."
- **Options:** (a) a Frame stage after the Print graph, its own design later; (b) frames as Print graph presets, a
  sub-graph like any other; (c) frames as an export option only, outside `RenderState`.
- **Applied meanwhile:** nothing is built (§0: "separate, later"); the lab's Polaroid medium (a Photographic node and
  preset) is a medium, not a frame, and is ported with the rest (R-486). Nothing waits.
- **Parked:** R-527 (20 Oct 2026), the human: "RQ-391 later." A deferral by ruling, with no target: the entry
  stays open and returns to the human when the human raises it. Nothing is built; the lab's Polaroid medium stays a
  medium (R-486).

## RQ-520: The frame record's `substep_histogram` is ruled `[u32; 16]` with "equal bins of width `N_max/16`" and "cap hits as their own last bin", but sixteen bins of width `N_max/16` fill `[0, N_max]` and leave no entry for a separate cap bin *(conflict within the human's ruling on RQ-286; R-545, R-288, REQ-TOOL-098, REQ-TOOL-187; context: TASK-M5-28, TASK-M8-28)*

*Found porting R-545 (the human's batch of 21 Oct 2026). Nothing is chosen.*

- **File, section:**
  - `docs/incoming/2026-10-21_rulings/rulings_2026-10-21.md` line 17, the human's ruling on RQ-286: "The frame record
    gains `substep_histogram: [u32; 16]`: equal bins of width `N_max/16`, with cap hits as their own last bin. It is
    read back with R-288's counters on the existing readback. This is a schema v1 change made by this ruling."
  - `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful", the
    `substep_histogram` line (R-545); `docs/gui/principia_render_gui_spec.md` § "Profiler": "a substeps-per-pixel
    histogram, with the cap marked".
  - The mock (`crates/gui/src/mock/profile.rs`, `SUBSTEPS_PER_PIXEL`, TASK-M6-28's PR #212) draws placeholder bins of 4
    substeps up to `N_max` 64, the cap marked at 64: sixteen bins of width `N_max/16`, with no separate cap bin.
- **The conflict:** sixteen equal bins of width `N_max/16` cover `[0, N_max]`, so a cap hit (a count of `N_max`) falls
  in the sixteenth with every count in `[15·N_max/16, N_max)`, not in a bin of its own; a bin of cap hits alone needs a
  seventeenth entry, or bins of another width. The words do not say which, nor which side of each edge a count on it
  falls.
- **Options seen:**
  - (a) `[u32; 17]`: sixteen equal bins of width `N_max/16` over `[0, N_max)`, half-open, and the cap hits (`N_max`) in
    a seventeenth.
  - (b) `[u32; 16]`: fifteen equal bins of width `N_max/15` over `[0, N_max)`, half-open, and the cap hits in the
    sixteenth.
  - (c) `[u32; 16]`: sixteen equal bins of width `N_max/16`, the last closed at `N_max`, so the cap hits are counted in
    the last bin with the counts just below the cap, and the Profiler marks the cap at its top edge (the mock's
    layout).
- **Applied meanwhile:** telemetry §2 and §5 carry `substep_histogram: [u32; 16]` as ruled; REQ-TOOL-187 (TASK-M5-28)
  builds the field, its GPU counting and its readback, and waits on this entry for the bin edges only.
- **Waits:** REQ-TOOL-187's bin edges (TASK-M5-28) and REQ-TOOL-098's drawing of them (TASK-M8-28).

## RQ-521: Under R-548's E ≥ 0 guard, an E < 0 state that passes the dispersal step's geometric tests goes on to the hierarchy step, where it may read HE, PE or excursion, but the human's words say it "falls through to B-stable, OS-candidate or B-so-far" *(physics, silence: the human's ruling against the option it adopts; R-548, R-442; REQ-PAY-099; context: TASK-M7-35)*

*Found porting R-548 (the human's batch of 21 Oct 2026). Nothing is chosen. For the human: yes, as physics where the
choice changes results (R-369): it decides which label a rare E < 0 state reads, and the texts do not settle it.*

- **File, section:**
  - The human's ruling on RQ-325 (`docs/incoming/2026-10-21_rulings/rulings_2026-10-21.md`, line 37; R-548): "(b). "H,
    HP, P and the two-stage H each require `E ≥ 0`. An E < 0 state that passes their geometric tests falls through to
    B-stable, OS-candidate or B-so-far. …""
  - RQ-325's option (b), which the ruling adopts: "Guard H, HP, P and the two-stage H with `E ≥ 0`, so an E < 0 state
    that passes their geometric tests falls through to the later steps (B-stable, OS-candidate or B-so-far)."
  - `docs/design/principia_dd_chazy_final_motion.md` § "3.2 The rule, in order", the order `classify` evaluates:
    step 1 (dispersal) ends "Otherwise go on"; step 2 (hierarchy) reads HE, PE, B-stable or excursion when its gate
    passes and the outer body recedes with the pair bound (`K_pair < U_pair`); steps 3 to 6 are transient, B-stable,
    OS-candidate and B-so-far.
- **The gap:** the two-stage H's guard sends a guarded state on to step 3, whose outcomes at E < 0 are exactly the
  three the words list. Step 1's guard sends a guarded state on to step 2. Step 1's geometric tests (every separation
  above `h_far · R_0`, every pair receding, no pair's band `−1`) leave a pair with `1 − ε ≤ K/U < 1`, bound by §3.4's
  sign test, so step 2 can read HE or PE (or B-stable or excursion) for it when its hierarchy gate passes. That label
  is not in the words' list, though PE and HE are allowed at E < 0 (§2's table). The homothetic fixture (R-548's
  §12.2 item 11) reads B-so-far under either reading (its hierarchy ratio is about 0.87, below `h_far`), and no fixture
  reaches the case.
- **Options seen:**
  - (a) The later steps from step 2 on: a guarded dispersal state goes on to the hierarchy step, as `classify`'s
    "Otherwise go on" and option (b)'s "the later steps" read; it may read HE, PE, B-stable or excursion there.
  - (b) The words' list: a guarded dispersal state skips the hierarchy step and goes on to step 3, so it reads
    B-stable, OS-candidate or B-so-far only.
- **Applied meanwhile:** (a), the drill-down's §3.2 as R-548 applies it (decided per R-369, R-548 C2); REQ-PAY-099
  carries this entry for that clause only. Nothing else of TASK-M7-35 waits.

## RQ-522: The trace fade's general off-plane distance is "the distance in z … in the metric RQ-341 rules", and RQ-341's ruling gives the latent Jacobian but no metric on `z` *(silence: R-509 Y3 against R-550; physics; REQ-GUI-266; context: TASK-M8-54)*

*Found porting R-550 (the human's 21 Oct batch). Nothing is chosen. For the human: yes (physics: it changes which
trajectories fade, and how fast).*

- **File, section:**
  - `decisions.md` § "R-509 — …", the human's words: "3 proxy where defined, else no fade and say so, the real distance
    waits on RQ-341"; Y3: "The general definition, the encode per step and the metric RQ-341 raises, is
    **REQ-GUI-266**".
  - `docs/design/principia_lines.md` § "11. Open questions", 3: "The general definition is the distance in z from the
    trajectory's encoded point to the slice plane, which needs the encode at every step (and the metric RQ-341
    raises)."
  - `plan/requirements.yaml` REQ-GUI-266: "the distance in z from the trajectory's encoded point to the slice plane,
    encoded per step, in the metric RQ-341 rules".
  - `decisions.md` § "R-550 — …", the human's words on RQ-341: "(a). Add an M6 definition requirement beside
    REQ-DEC-033 that shares its method: a CPU, f64, central-difference Jacobian `∂x/∂z` with 8 columns."; on RQ-344:
    "so `J⁺ = J⁻¹` and no metric on `z` enters"; on RQ-346: "Each coordinate is divided by a stated, fixed reference
    range: angles by their domain, masses by 1, and momenta by the decoder's momentum saturation bound." (a metric on
    the physical coordinates, not on `z`).
- **The silence:** the fade needs a distance from a point in `z` to a 2-plane in `z`, so a metric on `z`. RQ-341's
  ruling defines a Jacobian, not a metric; RQ-344's steer was ruled so that no metric on `z` enters; RQ-346's metric is
  on the compass's physical coordinates. None of the human's texts says which metric the trace fade uses.
- **Options seen:**
  - (a) The Euclidean metric in the latent coordinates `z`, the chart's own (the distance to `span(q̂₁, q̂₂)` through
    `z₀`).
  - (b) RQ-346's physical metric pulled back to `z` through R-550's latent Jacobian at the slice centre,
    `G = Jᵀ W J` with `W` the fixed reference ranges' diagonal, so a fade means the same physical distance on every
    chart.
  - (c) The distance taken in the physical coordinates themselves, the trajectory's decoded state against the slice
    plane's image, under RQ-346's metric.
- **Applied meanwhile:** none; nothing changes: on a chart with no proxy the trace does not fade and its settings say
  so (REQ-GUI-249, R-509). What waits: REQ-GUI-266 (`rq`), TASK-M8-54, a leaf.
