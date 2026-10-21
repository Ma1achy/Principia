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

## RQ-506: The prototype's code and R-543's text disagree: the legend states no Torus period, the hovered-pixel readout does not name the wrap mode, and the period counts the code uses go past the text's *(conflict: the ruling against its transcription source; R-543, R-542; context: TASK-M7-48, TASK-M7-51, TASK-M8-66)*

*Found by R-543's port, reading `docs/gui/reference/state_map/state_flow.html` (version 9) as source (no browser was
available). R-542's rule, which R-543 keeps: where the text and the code disagree, the code is the transcription source
and the disagreement is queued. Nothing is ruled.*

- **File, section:** `docs/incoming/2026-10-21_state_map_tiling/INSTRUCTION.md` (the human's ruling, R-543) against
  the prototype's code, each carried into `docs/design/principia_state_map.md`:
  - **The legend's period** (§2.1; state map § "SM6"): "The legend states the period in chart units". The code's
    `smLegend` draws the colour slot over the target's square and writes no period anywhere; only the comment above
    `SM_TORUS` says "(the legend states the period; …)".
  - **The hovered-pixel readout** (§2.2; state map § "SM8"): "The hovered-pixel readout and the graph strip name the
    wrap mode ("Picture · repeat")". The code names it in the graph strip (`smSyncUI`'s "Picture · " + the wrap); its
    `readout()` gives the pixel, the masses and the end, and no base or wrap mode.
  - **The period counts** (§2.1; state map § "SM6", its *Transcribed* paragraph): "on the plane the y period count is
    round(n · H / W)"; the code's is `max(1, round(n·H/W))`, round half up, which differs only where `n·H/W < ½`
    (no prototype chart). "On the cylinder, X is θ itself once round …, and only s repeats" gives no count for `s`;
    the code repeats `s` `n` times over the square's height whatever its aspect.
- **Options seen:**
  - (a) The ruling's text where the code is silent (the legend states the period in chart units, the readout names the
    wrap mode), and the code's counts, which only fill what the text leaves open.
  - (b) The code throughout: no period in the legend, the wrap mode in the graph strip only.
  - (c) Each ruled separately.
- **Applied meanwhile:** (a). REQ-GUI-339 carries the legend's period and the readout (TASK-M8-66), REQ-GUI-340 the
  node's label (TASK-M7-51), each with `rq: RQ-506`; REQ-COL-136's verify uses the code's counts.

## RQ-507: Under a Torus gradient, "Left its starting tile" compares the Palette's tile, so it separates systems one Torus period apart only where that tile ends within one period ahead of the system, in the move's direction, which depends on the Palette's tiling, counts, spacing and the chart's aspect, and the brightness node it needs replaces the L the Torus spends *(conflict: the ruling against R-542 and itself; R-543, R-542; REQ-COL-128; context: TASK-M7-48)*

*Found by R-543's port. Nothing is ruled.*

- **File, section:** `docs/incoming/2026-10-21_state_map_tiling/INSTRUCTION.md` §2.1 (R-543; state map § "SM6"): "The
  honest cost: a system exactly one period away gets the same colour. The legend states the period in chart units;
  "Left its starting tile" brightness already separates such systems." and "It spends L on position, so a brightness
  node after it gets the existing "no longer one-to-one" warning." `docs/design/principia_state_map.md` § "SM11",
  "Brightness, unchanged in meaning": "*left its starting tile*, which under Chart position compares the Palette's tile
  now and at `t = 0`"; the prototype's graph strip: ""tile" is the Palette's tiling"; its Palette offers 4, 6, 8, 12, 18
  or 24 tiles across and the Torus 1, 2, 3, 4, 6 or 8 periods.
- **The conflict:** the tile is the Palette's, from its own tiling, its count `m` and its spacing setting, not the
  Torus's Repeats `n`. **The exact condition (applied per R-369, the coordinator's decision on PR #272's physics
  review 5602225811):** a one-period move along an axis changes the Palette tile at a position exactly when the Palette
  tile containing that position, measured in chart units along that axis, ends within one Torus period ahead of it, in
  the move's direction (a move of −P tests the tile's other end). Both
  counts per axis are `smTileXY`'s: along x `m` and `n`; along y `max(1, round(m·H/W))` and `max(1, round(n·H/W))`;
  on the cylinder, along `s`, the Palette's own count `max(1, round(n_θ(m)·H/W))` against the Torus's `n`; under the
  squashed-square spacing the Palette's tile is measured through the compaction, the Torus's period never. The
  consequences, each measured on a transcription of the prototype's `smTile`, `smPattern`, `smTileXY`, `smHexRound`
  and `smTorusXY` (a one-period move from each of 200² positions in the home square unless stated; the survey and its
  output are in R-543's PR; "every offered pair" means every `m` in {4, 6, 8, 12, 18, 24} with every `n` in {1, 2, 3,
  4, 6, 8}), are examples of that condition, not further rules:
  - **square and triangles, square chart (latent A):** at every offered pair with `m ≥ n`, 0 of 10,000 positions (100²)
    stay on either axis (and 0 of 40,000 at `m = n` = 4, 6, 8); at `m = 6`, `n = 8`, squares 10,000 of 40,000 stay on
    each axis, triangles 2,442 along x and 724 along y.
  - **hexagons** (pointy-top, one unit wide, `2/√3 ≈ 1.155` tall), chart A: along x, at `m = n`, 0 of 40,000 stay at 4
    and 6, 14 at 8, on tile edges; along y the move must be at least `2/√3` tile units, and at `m = n` = 4, 6, 8, 1,032,
    968 and 936 of 40,000 stay; 0 at `m = 6`, `n = 4`, at `m = 8`, `n = 4` and at `m = 12`, `n = 6`.
  - **brick** (2 units wide, 1 tall), chart A: along y, 0 of 40,000 stay at `m = n` = 4, 6, 8 and at `m/n` = 6/4, 8/4
    and 12/6; along x the move must be at least 2 tile units: 20,000 of 40,000 stay at `m = n` = 4, 6, 8, 10,000 at
    `m = 6`, `n = 4`, and 0 at `m = 8`, `n = 4` and `m = 12`, `n = 6`.
  - **stripes:** the tile has no end across the stripes: on chart A, at every offered pair, stripes along x leave
    10,000 of 10,000 positions in their tile under a move along y, and stripes along y under a move along x.
  - **squashed-square spacing** (`sm_tspace = square`; latent chart A, square tiles, a move along x). Inside the square
    (2,001 starts on a line, both ends in the square): positions stay at 8 offered pairs, each with `m < 2n`: 132,
    801 and 1,144 at `m = 4` with `n` = 4, 6, 8; 331 and 698 at `m = 6` with `n` = 6, 8; 216 and 512 at `m = 8` with
    `n` = 6, 8; 12 at `m = 12`, `n = 8` (at `m = n = 8`, `X(2.0) = 6.331` and `X(2.75) = 6.897`, one tile). At every
    other offered pair with `n ≥ 2`, 0 of 2,001 stay (at `n = 1` the period is the whole square). Past the square
    (100² starts over `[3, 30]²`, a move of chart A's Torus period `6/n`), positions stay at every offered pair: 9,400
    to 10,000 of 10,000 (10,000 at every `n` for `m` = 4, 6, 8; 9,800 for 12; 9,500 to 9,700 for 18; 9,400 to 9,600
    for 24); at `m = 6`, `n = 2`, the move from `u = 0` to 3 changes the tile and those from 3 to 6, 6 to 9 and 9 to
    12 do not.
  - **a non-square flat chart:** the y move is `round(m·H/W)/round(n·H/W)` tile units, not `m/n`. On the tilted charts
    E and F (720 × 360), hexagons at `m = 4`, `n = 3` have both y counts 2, so the y move is 1 tile unit and 824 of
    40,000 stay, though `m/n = 1.33 ≥ 2/√3`; at `m = n = 4` likewise 824. On ν × dilation H (600 × 420), hexagons at
    `m = n = 4` (y counts 3 and 3): 1,080 stay. Squares and triangles on E and H, at every offered pair with `m ≥ n`:
    0 of 10,000 stay on either axis. Brick along x on E and H: 13,200 of 40,000 stay at `m = 4`, `n = 3`, 20,000 at
    `m = n = 4`, 10,000 at `m = 6`, `n = 4`, 13,600 at `m = 8`, `n = 6`, 0 at `m = 6`, `n = 3`.
  - **the cylinder** (G, 720 × 360): round θ there is nothing to separate (the Torus's θ is once round and θ + 2π is
    the same system), so the cost is in `s` only, where the Palette's `s` count is 3 for every `m` in {4, 6, 8} (then
    6, 9, 12 for 12, 18, 24). The cylinder's tiles are in chart units, and it offers six tilings (square, hexagons,
    triangles, brick, stripes round θ, bands along `s`); all six were measured at every offered pair (a one-period move
    in `s`, 100² positions, stays of 10,000):
    - squares, brick (one row tall) and bands along `s` alike: stay at 10 of the 36 pairs: 2,500 at `m = 4`, `6` or `8`
      with `n = 4`; 5,000 at `m = 4`, `6` or `8` with `n = 6`; 6,200 at `m = 4`, `6` or `8` with `n = 8`; 2,400 at
      `m = 12`, `n = 8`;
    - triangles: stay at the same 10 pairs: 204 with `n = 4`, 2,012 with `n = 6` and 3,468 with `n = 8` (each at
      `m = 4`, `6` or `8`), 232 at `m = 12`, `n = 8`;
    - hexagons: stay at 15 pairs: 264, 1,492, 3,764 and 5,212 at `m = 4`, `6` or `8` with `n` = 3, 4, 6, 8; 236 at
      `m = 12`, `n = 6`; 1,656 at `m = 12`, `n = 8`; 8 at `m = 18`, `n = 8`;
    - stripes round θ: the tile has no end along `s`, and 10,000 of 10,000 stay at all 36 pairs;
    - the other five tilings, at every pair not listed for them: 0 of 10,000 stay.

  And the separation is a brightness node, which replaces the L the Torus spends on position, so it costs the
  one-to-one map within a period that the Torus is for (the warning §2.1 names). It also tells a system that moved one
  period from one that stayed, but not one period from two.
- **Options seen:**
  - (a) As the code has it: the Palette's tile, its tiling, counts and spacing the Palette's; a one-period move is
    separated only where the exact condition above holds. In the measured cases above, every offered tiling has
    positions that stay at some offered pair: hexagons, brick, stripes and squashed spacing on chart A, and each of the
    cylinder's six tilings along `s`, at `m ≥ n` too. Squares and triangles on the flat charts A, E and H are
    the measured exception: 0 stay at every offered pair with `m ≥ n`. The legend says which tile is compared.
  - (b) Under a Torus base, Left its starting tile compares the Torus's own period cell, `(⌊X⌋, ⌊Y⌋)` in periods.
  - (c) The Palette's count is held to a whole multiple of the Torus's Repeats while both are in the graph. In the
    measured cases above: squares and triangles on A, E and H then stay separated (0 stay at every offered pair with
    `m ≥ n`); hexagons along y and brick along x on chart A still have positions that stay at `m = n` = 4, 6, 8;
    stripes leave 10,000 of 10,000 in their tile across their direction at every offered pair; the y and `s` counts
    are rounded from `m` and `n`, so on the cylinder squares, triangles, brick and bands along `s` still stay at the
    multiples `m = n` = 4, 6, 8 and at `m = 8`, `n = 4`, hexagons at those and at `m = 6`, `n = 3` and `m = 12`,
    `n = 6`, and stripes round θ at every multiple; and under squashed spacing, inside the square, positions stay at
    `m = n` = 4, 6, 8 (132, 331 and 512 of 2,001) and at no other multiple with `n ≥ 2`, while past the square they
    stay at every offered pair. Of the three options, (b) is the one that changes the compared cell on every
    one-period move, by construction (`⌊X + 1⌋ = ⌊X⌋ + 1`).
- **Applied meanwhile:** (a), the code. REQ-COL-128 is unchanged; REQ-COL-136's note cites this entry.

## RQ-508: The human's ruling on RQ-472 keeps the equator fold on a tilted shape sphere view where `N_b = 0`, where R-544 has no folding of any kind on the shape sphere chart *(conflict: the human's batch ruling against the human's later ruling; R-544, R-537 EX7(b), RQ-472; REQ-CHART-062, REQ-SCHED-101, REQ-REF-054, REQ-CHART-079, REQ-GUI-278; context: TASK-M2-36, TASK-M8-56, TASK-M8-58)*

*Found by R-544's port. Nothing is ruled.*

- **File, section:**
  - The human's `rulings_2026-10-21.md` (not yet recorded in `decisions.md`; a batch rulings PR records it), RQ-472:
    "(a). "On a tilted view the equator fold is kept iff `N_b = 0`, and the pole maps iff `N_a = 0` and `N_b = 0`, each
    decided exactly on the stored angles as §3.3 decides. Otherwise the view is affine along that axis and draws no
    image (TF2.2). §3.3's pixel fold governs the statistics (EX7(g)).""
  - `docs/incoming/2026-10-21_shape_sphere_whole/REPLY.md`, R-544: "No folding of any kind on the shape sphere chart,
    visible or not."; `PLAIN_SUMMARY_2.md`: "There is no hemisphere mode and no folding."; `INSTRUCTION.md` §2 item 4:
    "The mirror is no longer a repeat. R-537's repeats on the shape sphere are θ's period and the pole crossing only".
  - `docs/design/principia_chart_reference.md` § "3.3 The chart map", the tilted fold (#255, applied per R-369): "the
    pair keeps its fold, exactly when the tilt adds no hidden offset that depends on the mirrored axis: **`N_b = 0`**",
    which R-544's port replaces.
- **The conflict:** RQ-472's ruling concerns the shape sphere chart: its subject is "Tilted shape sphere: EX7(b) folds
  only where §3.3 allows", the tilted views of the shape sphere chart, and `N_b = 0` includes the untilted view. Its
  equator clause keeps a mirror fold there, and its last sentence keeps §3.3's pixel fold for the statistics; R-544 has
  none, at any tilt. The pole maps under a tilt (its `N_a = 0` and `N_b = 0` clause) are not a fold of the mirror:
  R-544 keeps the pole crossing as a repeat, and that clause is not in conflict.
- **Options seen:**
  - (a) R-544 governs on the shape sphere chart: no equator fold and no pixel fold on any view, tilted or not; RQ-472's
    ruling keeps its pole maps, decided as it says, and its equator clause and its last sentence no longer apply to the
    shape sphere.
  - (b) RQ-472's equator clause stands for a tilted view with `N_b = 0`, the untilted view excepted, so the fold
    returns on a tilted view only.
- **Applied meanwhile:** (a), R-544, the later ruling and the one that says "of any kind". chart_reference §3.3's
  tilted fold is replaced by R-544's one-to-one tilted view.
- **Waits:** the tilted shape sphere's fold arm of REQ-CHART-062 (TASK-M2-36; the rest of the task does not wait),
  and the tilted canonical maps of REQ-SCHED-101 (TASK-M8-56), with the neighbours and the statistics read through
  them, REQ-REF-054 and REQ-CHART-079, and their trace images, REQ-GUI-278 (TASK-M8-58), which already wait on RQ-472.
  Each carries `rq: RQ-508`.

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
