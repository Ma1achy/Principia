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
- **21 Oct 2026 (evening):** RQ-341 is ruled (R-550), and the human parked RQ-378 again, with no target (R-568): "yeah
  ok we can park that for now then". The entry stays open, (a) stays applied, nothing waits, and it returns when he
  raises it. *Was:* "RQ-341 is ruled (the 21 Oct batch, PR #274)", before R-550 was on main.

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
- **21 Oct 2026, R-554 (PR #274):** R-554 records the human's ruling on RQ-472. Its pole-map clause is applied (the
  pole maps on a tilted view only where `N_a = 0` and `N_b = 0`, decided on the stored angles); its equator clause and
  its last sentence (§3.3's pixel fold for the statistics) are left unapplied pending this entry. Only the equator arms
  on a tilted view wait; the options above are unchanged (physics re-check 5606508368 of PR #274, P6).

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

## RQ-524: R-565 makes the regularised occupants' sync grid a fixed quantum independent of the horizon, where the unregularised occupants' macro-step grid, `dt_macro = max(1e-3, T/65535)`, depends on the horizon above `T = 65.535` *(silence: the ruling names the sync grid only; R-565, R-132, R-86; REQ-INT-081, REQ-INT-100; context: TASK-M3-06, TASK-M3-40)*

*Found by R-565's port. Nothing is ruled.*

- **File, section:**
  - The human's `INSTRUCTION.md` (`docs/incoming/2026-10-21_gap_sweep_integrator/`), § "PR 1": "**The sync grid is a
    fixed quantum of physical time**, the same for every run and independent of the horizon and of frame timing.
    Steps are sized from it, the march stops only on it, and the playhead reads boundary states."
  - `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)", `dt_macro`'s
    row: "`max(1e-3, T/65535)` (**fixed** for a given `T`, §5; R-132) — so `⌈T/dt_macro⌉ ≤ 65535` and the u16 step
    indices stay exact (R-86)"; § "Part 5": "With `dt_macro` fixed for a given `T`, this length is identical CPU/GPU".
  - `decisions.md` § R-132: "REQ-INT-081: `dt_macro = max(1e-3, T/65535)`. T keeps [50, 200], with a default of 50".
- **The silence:** a "sync boundary" belongs to the regularised occupants (Part 2b); the unregularised ones step on
  the macro-step grid, which at the default `T = 50` is the fixed `1e-3` but above `T = 65.535` is `T/65535`, a
  different discretisation per horizon, the trap R-565 rules out for the sync grid. The ruling does not say whether
  it applies to the macro-step grid, and the u16 step index (R-86) is what ties `dt_macro` to `T`.
- **Options seen:**
  - (a) R-565 is the sync grid only; the macro-step grid stays R-132's, unchanged.
  - (b) The macro-step grid is fixed too, `dt_macro = 1e-3` for every `T`, and a horizon above 65.535 marches in
    epochs (Part 5's "march epochs") so the u16 index stays exact.
  - (c) The macro-step grid is fixed at a value that keeps `⌈200/dt_macro⌉ ≤ 65535` for every `T` in `[50, 200]`.
- **Applied meanwhile:** (a): the contracts' `dt_macro` is unchanged; integrator contract Part 2a names this entry.
- **Waits:** nothing built waits; the answer changes REQ-INT-081 if it is not (a).

## RQ-525: R-561's warning "in the Problems panel": the corpus's only Problems pane is the stain editor's, for compile errors and colour-graph warnings *(silence: which panel; R-561, R-542; REQ-GUI-352; context: TASK-M8-24)*

*Found by R-561's port. Nothing is ruled.*

- **File, section:**
  - The human's `INSTRUCTION.md`, § "PR 1": "A pair measured not to work carries a warning beside the selector, in the
    Problems panel and in each export's settings."
  - `docs/gui/principia_render_gui_spec.md` Part II (the stain editor), § "1. Layout — four surfaces": "the assembled
    code and a **Problems** pane (compile"; its "Problems for the state map (R-542; state map SM15)": "A warning where
    a brightness node replaces L".
    No other Problems panel is defined; the Run window (§G5) and the Profiler are the integrator's surfaces.
- **The silence:** an integrator pair's warning is not a stain problem, and no app-wide Problems panel exists.
- **Options seen:**
  - (a) The stain editor's Problems pane gains a row for the active occupant's warning.
  - (b) A Problems panel for the whole app (a window or a status-line count opening a list), holding the stain
    editor's problems and the integrator's warnings; the stain editor's pane shows its own subset.
  - (c) The warning shows in the status line and the Run window only, "the Problems panel" read as those.
- **Applied meanwhile:** none: the selector's warning (REQ-GUI-351) and the export settings' (REQ-GUI-353) are built
  without it.
- **Waits:** REQ-GUI-352 (TASK-M8-24; the rest of the task does not wait), `rq: RQ-525`.

## RQ-526: How a regularised copy's budget exhaustion and its two counted signals are stored, whether an undetermined-by-budget footprint splits, and which signal stops a quad at the integration floor *(silence: R-563, R-310's descriptor bits, payload §2, refinement policy §1 and §6, scheduler Part 4; REQ-INT-101, REQ-SCHED-063; context: TASK-M3-41, TASK-M6-10)*

*Found by R-563's port. Nothing is ruled.*

- **File, section:**
  - The human's `INSTRUCTION.md`, § "PR 1": "Regularised: running out of the step budget makes the result
    undetermined, never discarded. The separation floor and the entry-cap hits are counted, non-terminal signals.
    Re-point REQ-SCHED-063 and -077."
  - The source design (`designs/integrator_findings.md` §2, not ruled text): "The `saturated` bit's meaning is
    restated in those terms".
  - `docs/contracts/principia_integrator_contract.md` § "Part 7", the terminal states: "`escape`/`bounded`/`collision`
    are the physical outcomes; `sim_failed` … and `decode_failed` … are lifecycle states"; the sticky `saturated`
    bit is "descriptor bit 5"; RQ-310 holds descriptor bits 10–15 contested.
  - `docs/design/principia_dd_refinement_policy.md` § "1. The split rule": "unresolved(f) ⟺ … ∨ the footprint is
    undetermined (non-finite spread, or an unusable copy)"; § "6. Decision variants, and the second budget line":
    "**`Undetermined` means FINER `eta`, NOT FINER CELLS.**"
  - `docs/contracts/principia_scheduler_contract.md` § "Part 4", the integration floor: "a quad whose `suspect_fraction`
    stays high and whose samples are **substep-saturated** … **stop refining**".
- **The silence:** (1) a budget-exhausted copy has no `state` value (it is not `sim_failed`, which is NaN or Inf, and
  not `running` once it stops) and no bit; (2) the two counts have no payload field; (3) §1 splits an undetermined
  footprint, which the design measured as manufactured work, while §6 says undetermined calls for finer `eta`, not
  finer cells; (4) scheduler Part 4's floor reads substep saturation, which a regularised occupant has none of.
- **Options seen:**
  - (a) A sticky descriptor bit `budget_exhausted` beside `saturated`, the copy's `state` latched as it stood and its
    footprint undetermined at read; the `saturated` bit set when either counted signal fires, the two counts kept in a
    payload word (a schema change); an undetermined-by-budget footprint takes §6's `Undetermined` decision, never a
    split; the floor stop reads `saturated` and `suspect_fraction` for every occupant.
  - (b) As (a), but the budget's outcome a new `state` value, `undetermined`, a lifecycle state like `sim_failed`.
  - (c) As (a) or (b), but an undetermined-by-budget footprint splits as §1 says, the budget sized so that it does not
    bind (R-564's calibration).
- **Applied meanwhile:** none in the payload; the contracts say "undetermined, never discarded" and "counted,
  non-terminal" and name this entry.
- **Waits:** REQ-INT-101 (TASK-M3-41) and the regularised arm of REQ-SCHED-063 (TASK-M6-10; its regularisation-off arm
  does not wait), each `rq: RQ-526`.

## RQ-527: How R-564's rate is accounted along a run: against the elapsed grid time, per sync interval, or with an allowance *(silence: R-564; REQ-INT-102, REQ-INT-103; context: TASK-M3-41)*

*Found by R-564's port. Nothing is ruled.*

- **File, section:**
  - The human's `INSTRUCTION.md`, § "PR 1": "**The step budget is a rate**, steps per unit physical time. The
    calibration value starts at about 4e4 per unit time per copy."
  - The source design §2: "The budget is a **rate**, steps per unit physical time, so it is defined for a live playhead
    with no fixed end." Its measurement is a whole-run mean, total steps over `t_max`.
- **The silence:** a rate becomes a bound only through a rule for when it is exceeded, and the rules give different
  outcomes on the same run: a close encounter at `t = 0.5` that needs `3·10⁴` steps exceeds a cumulative bound
  `rate × t` (2·10⁴ there) but not a whole-run mean.
- **Options seen:**
  - (a) Cumulative: a copy is exhausted when its steps exceed `rate × t` at a sync boundary, `t` its grid time.
  - (b) Per interval: each `Δ_sync` interval gets `rate × Δ_sync` steps, unspent steps lost.
  - (c) Cumulative with an allowance: steps exceed `rate × max(t, t_min)`, `t_min` a calibration, so an early
    encounter is not cut.
  - (d) Cumulative, unspent steps carried forward, checked at every step rather than at boundaries.
- **Applied meanwhile:** none; Part 3's `step_budget_rate` row names this entry.
- **Waits:** REQ-INT-102 and REQ-INT-103 (TASK-M3-41), `rq: RQ-527`.

## RQ-528: Where the regularised energy's drift is stored, and how a drift view draws a run that stopped at an event, under R-566 *(silence: R-566, payload §5, integrator contract Part 1; REQ-VAL-195, REQ-INT-085, REQ-INT-106; context: TASK-M3-41, TASK-M3-05)*

*Found by R-566's port. Nothing is ruled.*

- **File, section:**
  - The human's `INSTRUCTION.md`, § "PR 1": "The two-body collision gate (REQ-VAL-045) and any per-step suspect flag
    under a regularised occupant read the regularised energy. The Cartesian energy is for diagnostics, read only from
    runs that did not stop at an event."
  - `docs/design/principia_dd_integrator.md` § "3.5": "accumulate: ΔE_final, max|ΔE|, ΔL_z final, max|ΔL_z|"; "suspect
    predicates (READ-TIME, over the stored drift latches — no stored suspect bits, payload §5)".
- **The silence:** the payload has one energy drift, the Cartesian one; a suspect flag derived at read from it cannot
  read the regularised energy, so a second drift latch, or a different one per occupant, is needed and has no place.
  And every pixel's drift is drawn today, including those of runs that stopped at a collision or an escape.
- **Options seen:**
  - (a) Under a regularised occupant the stored drift latches are the regularised energy's, and the Cartesian drift
    is computed only in the harness and the inspector (the diagnostic runs, termination off).
  - (b) Both drifts stored, the regularised one in new payload fields (a schema change), the suspect predicate reading
    it under a regularised occupant.
  - (c) For the display: a drift view draws an event-stopped run as "no data" (the not-yet style) under a regularised
    occupant; or it draws it with a badge; or it draws it as today.
  - (d) *Added after physics review 5605806092 of PR #277, F6:* the energy-suspect threshold (REQ-INT-085) is one
    value for every occupant, and its calibration measured only the occupants with regularisation `none`, asking that
    Euler light SUSPECT_ENERGY on dd test 2's long orbit and the symplectic occupants not. The default occupant is now
    Heggie × RK4 (R-560), not symplectic, its regularised energy drifting secularly (REQ-VAL-031), and SUSPECT_ENERGY
    reads it (R-566). If it lights on test 2's orbit it lights across the field. Either (d1) the threshold must leave
    Heggie × RK4 unlit on test 2's orbit at the default step, as it does the symplectic occupants; or (d2) Heggie ×
    RK4 is measured and recorded with no requirement, as RK4 is, and a lit default reads as integrator drift
    (integrator contract Part 2, "the suspect flag … means different things per occupant"); or (d3) the threshold is
    per occupant (a change to the one-threshold rule).
- **Applied meanwhile:** none in the payload; the contracts state the rule and name this entry. For (d): (d2),
  REQ-INT-106 measures and records Heggie × RK4's regularised drift on tests 2 and 7 and which predicates it lights
  under REQ-INT-085's proposed thresholds. REQ-INT-106 waits with TASK-M3-41, which builds nothing before RQ-526 to
  RQ-528 are ruled (applied per R-369, physics review 5606990734 of PR #277).
- **Waits:** REQ-VAL-195 and REQ-INT-106 (TASK-M3-41), `rq: RQ-528`. *Was:* "REQ-VAL-195 (TASK-M3-41), `rq: RQ-528`.
  Nothing waits on (d): the M3 gate reads REQ-INT-106's record."

## RQ-529: R-560's default occupant, Heggie × RK4, against integrator contract Part 2's tier column, which binds KDK at Potato–Low, Yoshida-4 at Medium–High and Yoshida-6 at Ultra–Extreme *(conflict: R-560 against R-4 and the tier table; REQ-INT-014, REQ-INT-006; context: TASK-M3-02, TASK-M3-08)*

*Found by R-560's port. Nothing is ruled.*

- **File, section:**
  - The human's `INSTRUCTION.md`, § "PR 1": "**Default occupant: Heggie × RK4.**"; `DECISIONS_2026-10-21.md`: "Yep
    Heggie x RK4 as default".
  - `docs/contracts/principia_integrator_contract.md` § "Part 2", the table: "the last column gives the *default*
    binding; the user overrides": KDK "Potato–Low (fast, motion)", Yoshida-6 "Ultra–Extreme (accuracy)", RK4
    "Validation cross-check".
  - `decisions.md` § R-4: "spec-keyed defaults" means the markdown's tier tables.
- **The conflict:** the ruling sets one default with no tier; the table binds a stepper per tier, and with Heggie the
  table's KDK and Yoshida rows compose into R-561's measured-bad pair (KDK) or unmeasured pairs (Yoshida).
- **Options seen:**
  - (a) Heggie × RK4 at every tier; the tier column is the per-stepper binding with regularisation `none` only.
  - (b) Heggie × RK4 at Medium and above; the low tiers keep a cheaper default (`none` × KDK at Potato–Low).
  - (c) The tier column is redrawn for Heggie once R-561's table measures Heggie's other pairs.
- **Applied meanwhile:** (a), the ruling's words: Part 2 says so and names this entry. TASK-M3-02's per-stepper
  binding is unchanged under any option.
- **Waits:** nothing built; the default config of TASK-M3-08 follows (a) until ruled.

## RQ-530: With R-562's "v2" text removed, whether Kustaanheimo–Stiefel regularisation, the one method that text named, has a place *(silence: R-562; dd_decoder §6; context: none)*

*Found by R-562's port. Nothing is ruled.*

- **File, section:**
  - The human's `INSTRUCTION.md`, § "PR 1": "**Regularisation is v1, not v2.** Remove the "v2" text (integrator
    contract Part 6) and REQ-INT-033."
  - `docs/contracts/principia_integrator_contract.md` § "Part 6", as it stood: "Kustaanheimo–Stiefel (KS)
    regularisation is **v2**. … it changes the state representation and ripples through
    decoder/canonicaliser/observables, so it is not a drop-in occupant".
  - `docs/design/principia_dd_decoder.md` § "6. Deferred / flagged": "**KS-regularised state representation (v2)** —
    changes the decoder's output type; explicitly out of scope until then."
- **The silence:** the v2 text was about KS, which is not among Part 2b's regularisation occupants; R-562 makes
  regularisation v1 but does not say whether KS joins them, stays a later method, or is dropped.
- **Options seen:**
  - (a) KS is not built: Part 2b's four are the regularisations, and a later method is added by its own ruling;
    dd_decoder §6's item is reworded to say so, with no version.
  - (b) KS stays a later version's method, dd_decoder §6 unchanged.
  - (c) KS joins v1 as a fifth regularisation occupant.
- **Applied meanwhile:** the v2 text is removed from integrator contract Part 6 and dd_integrator §6, each naming this
  entry; dd_decoder §6 is unchanged.
- **Waits:** nothing.

## RQ-531: REQ-EVT-013's re-bind test counts "+1, +2, +3, +4 and +8 sync boundaries" past firing, a count whose physical time R-565's fixed quantum changes *(silence: R-565 against a count of boundaries; R-31, R-103; REQ-EVT-013; context: TASK-M3-33)*

*Found by R-565's port. Nothing is ruled.*

- **File, section:**
  - The human's `INSTRUCTION.md`, § "PR 1": "Time windows such as the closure window are stated in physical time."
  - `plan/requirements.yaml`, REQ-EVT-013's test: "integrate +1, +2, +3, +4 and +8 sync boundaries (2–3x the firing
    time) past firing with the original discretisation"; `docs/contracts/principia_integrator_contract.md` § "Part
    7": "0 of 895 escapes were still unbound eight sync boundaries later".
- **The silence:** the 0-of-895 measurement's boundaries were prin-rs's `t_max / n_sync`; on a fixed `Δ_sync` the same
  counts are a different, and if `Δ_sync` is small a much shorter, time past firing, and "(2–3x the firing time)" no
  longer follows from them.
- **Options seen:**
  - (a) The offsets are stated in physical time, the times the original boundaries stood at (to be read from the
    prin-rs measurement's grid), marched on `Δ_sync`.
  - (b) The offsets are 1, 2, 3, 4 and 8 closure windows (0.4 time units each).
  - (c) The offsets are 1×, 2× and 3× the firing time, as the parenthesis says.
- **Applied meanwhile:** none; REQ-EVT-013 is unchanged.
- **Waits:** REQ-EVT-013's test (TASK-M3-33), `rq: RQ-531`.

## RQ-532: R-567 says Heggie × the generalised leapfrog "stays allowed, with the warning, like every other pair", but the generalised leapfrog is not one of the contract's steppers, and the port of R-561 recorded the two implicit steppers as not occupants *(conflict: R-567 against R-561's port and integrator contract Part 2; REQ-INT-017, REQ-INT-098; context: TASK-M3-39)*

*Found by R-567's port. Nothing is ruled. For the human: yes, it changes what is built (one or two implicit steppers).*

- **File, section:**
  - The human's follow-up of 21 Oct 2026 (evening), item 1: "In the warning table of REQ-INT-006/-017, the pair Heggie
    × generalised leapfrog is **measured bad**, with that reason in one line … It stays allowed, with the warning, like
    every other pair."
  - `docs/contracts/principia_integrator_contract.md` § "Part 2", the occupants' table: Euler, KDK leapfrog,
    Yoshida-4, Yoshida-6, RK4; § "Every stepper × regularisation pair runs; …": "Two implicit steppers were measured
    under Heggie on the same fixture and are not occupants"; "A symplectic regularised occupant, if one is wanted, is
    an implicit stepper and gets its own row in the table below."
  - The gap sweep's design §1 (the bundle's record): "A symplectic regularised occupant, if ever wanted, is an
    implicit stepper and gets its own entry in the table."
- **The conflict:** "stays allowed … like every other pair" reads as a selectable pair, so a selectable stepper; the
  contract has no generalised-leapfrog stepper (nor implicit midpoint), and "not occupants" was the port's reading,
  applied per R-369, not the human's words.
- **Options seen:**
  - (a) Neither implicit stepper is selectable; the table records Heggie × the generalised leapfrog as measured bad
    for the record (note 4), and "stays allowed" means no pair of the built steppers is blocked.
  - (b) The generalised (Störmer–Verlet) leapfrog becomes a stepper (implicit, fixed-point iteration, its profile and
    its pairs), selectable with its warning under Heggie; implicit midpoint stays out.
  - (c) Both implicit steppers become steppers, the implicit midpoint rule being the symplectic regularised occupant
    the contract and the design name.
- **Applied meanwhile:** (a): the pair is in Part 2b's table as note 4, measured bad with its warning; neither
  implicit stepper is built.
- **Waits:** nothing built; TASK-M3-39 builds the pair table with note 4 under (a).

## RQ-533: R-569's words and its demos differ in four places: an icon larger than the view, a click that pins the trace the hover already shows, the legend's key line, and a hover into a repeat of the shown trace *(conflict: the ruling against its transcription source; R-569, R-538; REQ-GUI-354, REQ-GUI-356, REQ-GUI-332; context: TASK-M8-58, TASK-M8-63)*

*Found by R-569's port, reading `docs/incoming/2026-10-21_trace_playback/demo/trace_markers.html` (version 29) and
`trace_sound.html` (version 10) as text; neither was run. The two demos agree with each other in all three places.
Nothing is ruled.*

- **File, section:** `docs/incoming/2026-10-21_trace_playback/INSTRUCTION_trace_playback_2026-10-21.md` § "The rulings"
  (R-569; `docs/design/principia_lines.md` § "8. Traces", `docs/gui/principia_render_gui_spec.md` § "G6. Legend —
  generated by evaluating the stain (`06_legend.png`)") against the demos' code:
  - **An icon larger than the view** (ruling 3): "Where an icon is larger than the view on an axis (a tiny view), it is
    centred on that axis and not clamped." The demos' `withIcon` clamps an axis only `if ((a1 - a0) * s < W)` (and the
    same for `H`); otherwise it leaves the icon at its point, unclamped and not centred.
  - **A click that pins the hovered trace** (ruling 1): "a hover or click that integrates a different IC sets the
    playhead to 0 and keeps playing". The demos' `pointerup` calls `compute` on the clicked pixel, which is the pixel the
    hover already shows when the pointer has not moved, and `compute` calls `restartOnNewRun` whenever the pixel is a
    system, the same IC or not; so pinning the shown trace while playing restarts it in the demos.
  - **The legend's key line** (ruling 2): "The legend gains one key line: "Dotted: paused at t = 0, not yet played"".
    The demos' key reads "Dotted: paused at t = 0, the whole trace, not yet played".
- **Options seen:**
  - (a) The words in all three: centred on that axis; no restart when the click integrates the shown trace's IC; the
    key line as the words give it.
  - (b) The demos in all three: left at its point; every click on a system restarts; the demos' longer key line.
  - (c) Each ruled separately.
- **Applied meanwhile:** (a). `principia_lines.md` §8 and render_gui_spec §G6 say so, citing this entry; REQ-GUI-354's
  control and REQ-GUI-356's statement carry it, each with `rq: RQ-533`, and REQ-GUI-332's key line.
- **A fourth place (added by PR #278's fix round 6, gui re-check 5624466118 G2, applied per R-369; the three above
  unchanged):**
  - **A hover into a repeat of the shown trace** (ruling 1): "a hover or click that integrates a different IC sets the
    playhead to 0". A pixel in a repeat integrates the canonical system, the shown trace's IC bitwise (R-537, R-538
    TM7; REQ-GUI-275), so by the words moving the hover into it does not restart. In the demos every `pointermove`
    while unpinned schedules `compute` (`trace_markers.html:838`, `:853`; `trace_sound.html:903`), and `compute` calls
    `restartOnNewRun` for any pixel that is a system, the same IC or not (`trace_markers.html:837`;
    `trace_sound.html:887`), so they restart there.
  - **Options seen:** (a) the words: no restart, the IC being the same; (b) the demos: every hover onto a system pixel
    while playing restarts.
  - **Applied meanwhile:** (a), the words. REQ-GUI-354's repeat arm carries it (`rq: RQ-533`).
  - *Was (before PR #278's fix round 8, qa re-check 5624560597 Q5, applied per R-369):* the citation "(`trace_markers.html:838`, `:851`;
    `trace_sound.html:903`)", `:851` being the pan; and the heading "R-569's words and its demos differ in three places:
    an icon larger than the view, a click that pins the trace the hover already shows, and the legend's key line".

## RQ-534: R-569's restart sets "the playhead" to 0, and the trace's playhead is the Time panel's, which is the figure's one global playhead, so every hover onto a new pixel while playing restarts the whole field's live march *(conflict: the ruling against the canonical spec and the export and animation contract; R-569, R-538 TF3, R-476; REQ-GUI-354; context: TASK-M8-58)*

*Found by R-569's port. The demos' field is a precomputed image, so they cannot show it. Nothing is ruled.*

- **File, section:**
  - `docs/incoming/2026-10-21_trace_playback/INSTRUCTION_trace_playback_2026-10-21.md` § "The rulings", 1 (R-569):
    "Under `animate: WithPlayhead`, with the transport playing, a hover or click that integrates a different IC sets the
    playhead to 0 and keeps playing."
  - `docs/design/principia_lines.md` § "8. Traces", Animation: "With the playhead: a head marker sits at the playhead
    and moves as time is scrubbed or played, using the Time panel's own play and loop."
  - `docs/contracts/principia_canonical_spec.md` item 14: "**Single playhead + checkerboard bounded exception** — one
    global barrier-synced playhead; never present mixed time".
  - `docs/contracts/principia_export_animation_contract.md`, the transport table: "**restart** | playhead → 0, live-set
    states discarded and re-decoded."
- **The conflict:** the words restart "the playhead", and the only playhead "With the playhead" has is the global one.
  Restarting it while playing discards and re-decodes every live pixel's state and moves every playhead-read layer (a
  Chart position stain at the playhead, `t_end` contours, Listen's head) back to 0. While playing, a hover moves to a new
  pixel many times a second, so the field would restart continually while the pointer moves over it. The words do not
  say whether the field restarting with the trace is meant.
- **Options seen:**
  - (a) The words as written: the Time panel's global playhead goes to 0, the field's live march restarting with it.
  - (b) The trace's head only: under "With the playhead" a new trace starts its own head at 0 and runs it at the
    transport's rate while the transport plays, the figure's playhead and the field unaffected (as the own clock does).
  - (c) (a), but only for a click that pins a trace; a hover draws the new trace without restarting.
- **Applied meanwhile:** (a), the words. REQ-GUI-354 carries it with `rq: RQ-534`.

## RQ-536: R-570's "No hatch, no blank, and no 'not a system' on these charts at any pan or zoom" against the decoder's `DEGENERATE(M01_TINY)` fence, which the far field of a latent chart reaches under the edge-reaching simplex link, or under `softmax_tanh` with a large enough `μ_max`, and which R-511 draws blank *(conflict: the human's ruling against the decoder's definitions; R-570, R-511, R-481, dd_decoder §3.1, REQ-GEN-026; REQ-CHART-087; context: TASK-M8-57)*

*Found by R-570's port. Nothing is ruled. For the human: yes (physics: what a far-field pixel is under a non-default
link).*

- **File, section:**
  - `docs/incoming/2026-10-21_latent_unbounded/INSTRUCTION_latent_unbounded_2026-10-21.md` § "The behaviour", item 1:
    "every latent `z` decodes to a valid system"; item 3: "**Every pixel is a system, everywhere.** No hatch, no blank,
    and no "not a system" on these charts at any pan or zoom."
  - `docs/design/principia_dd_decoder.md` § "3.1 Mass": "`M₀₁ = m₀ + m₁ ;   if M₀₁ < ε  →  DEGENERATE(M01_TINY)`",
    with `ε_M01 = 10⁻⁵` (REQ-DEC-039) and "the edge-reaching simplex link's corner `m₂ → 1` piles up at
    `ε_μ − ε_μ²/4`, below it".
  - `docs/design/principia_dd_generation_root.md` § 3, the edge-reaching simplex link (REQ-GEN-026): "Near the corner
    `m₂ → 1`, `m₀ + m₁` falls to about `ε_μ`, where dd_decoder's `DEGENERATE(M01_TINY)` fence (`M₀₁ < ε`) can fire."
  - `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type
    (R-407)", "Blank, never a colour (R-470)": "a `decode_failed` sample the chart does not classify (a non-finite
    decode, η_E's tags, the mass block's `M01_TINY`) blank, as a pixel that is not a system (R-511)".
- **The conflict:** at the default links (`softmax_tanh` at `μ_max = 5`) the far field's `M₀₁` stays at or above
  `(1 + e^−5)/(1 + e^−5 + e^5) ≈ 6.7×10⁻³`, far above `ε_M01`, so no pixel is tagged and R-570 holds. Under the
  edge-reaching simplex link, which the Decoder window may select for the masses (R-478), the latent far field saturates toward the corner
  `m₂ → 1`, where the fence fires, so those pixels are `decode_failed` and draw blank (R-511); under `softmax_tanh` the
  floor falls below `ε_M01` once `μ_max` is above about 11.5, and `μ_max`'s allowed range is an open calibration
  (REQ-GEN-038). Either way a latent chart has "not a system" pixels far out, which item 3 rules out.
- **Options seen:**
  - (a) R-570's item 3 holds at the default links; under a link or a parameter whose saturation reaches the fence, the
    tag stands and those far-field pixels are not a system, blank, as the decoder defines them.
  - (b) The links' ranges are bounded so that saturation never reaches the fence: `μ_max`'s range capped below the
    value where `softmax_tanh`'s floor meets `ε_M01`, and the edge-reaching link's `ε_μ` set above `ε_M01`.
  - (c) R-570 governs on latent and flat charts: a far-field `M01_TINY` there is a labelled system, drawn and
    integrated as any other, the fence becoming a label rather than "not a system" on these charts.
- **Applied meanwhile:** nothing changes in the decoder; R-570's tests run at the default links, where no pixel is
  tagged.
- **Waits:** REQ-CHART-087's arm under a non-default mass link or `μ_max` (TASK-M8-57), `rq: RQ-536`. The default-link
  arms do not wait.

## RQ-538: A Print graph length in `device_px` is "true screen pixels" and the sheet is "anchored to the screen", but the Print graph runs before the display scale, where every stage runs at render resolution, and under Auto `render_scale` changes while the camera moves *(silence: the design doc against the render contract; R-490, R-487, R-531, R-557; REQ-RENDER-110; context: TASK-M7-69)*

*Found writing the Print graph plan (the human's request of 21 Oct 2026 to add the bundles' tasks). Nothing is
chosen. For the human: no physics; it changes how a screen medium (a CRT's scanlines, an LCD's subpixels, an LED's
pitch) and the paper's tooth look whenever `render_scale ≠ 1`, so it is a design question, not a routine one.*

- **File, section:**
  - `docs/design/principia_print_graph.md` § "6.2 Units: a medium must look the same at every export size":
    "**`device_px`**: true screen pixels. Only for effects that model the display itself: CRT scanlines, LCD
    subpixels, LED pitch." and "**`ref_px`** …: pixels on a 1024-wide figure, scaled by `output_width / 1024` at render
    time."
  - `docs/design/principia_print_graph.md` § "6.3 Anchoring and stability under motion and playback": "**The sheet is
    anchored to the screen.** Paper tooth, canvas weave, grain and the ground stay fixed while the figure pans and
    zooms beneath them." and "Noise-driven marks … are seeded from screen-anchored coordinates and the graph's seed".
  - `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules", the composite order: "**display
    scale** (when `render_scale ≠ 1`: bilinear upscale below native, box downsample above — the step onto the
    display-sized swap chain; all prior stages run at render resolution)", and the Print graph stage: "SimState →
    stain → [the line sets marked `through_medium`] → **Print graph** → display scale → gamut clamp → …" (R-487,
    R-531).
  - `docs/design/principia_quality_device_note.md` § "5. Timescale cascade (each layer absorbs what's too fast for the
    one above)": the motion regime's levers, `render_scale` among them, beside the post level (R-557).
- **The silence:** the Print graph writes a render-resolution buffer, which the display scale then resamples to the
  screen. A `device_px` length measured in that buffer is a screen pixel only at `render_scale = 1`; at any other
  scale a CRT's scanline pitch, drawn at one render pixel, is resampled to `1/render_scale` screen pixels, blurred
  below native and averaged away above. The same holds for "screen-anchored" coordinates: a sheet seeded from render
  pixels rescales on the screen when Auto moves `render_scale` during a pan, so the sheet the design keeps still would
  change grain at every motion-regime step. Neither the design doc nor the render contract says which pixels
  `device_px` and the screen anchoring are counted in, or what a `device_px` node does at export, where no screen
  exists (§6.2 exempts `device_px` nodes from the scale-invariance test, but does not say what width they use).
- **Options seen:**
  - (a) `device_px` and the screen anchoring are counted in display pixels mapped back into the render buffer: a
    length of `n` device pixels is `n · render_scale` render pixels, and the anchoring coordinate is the pixel's
    screen position; the effect is still resampled by the display scale, so below native a one-pixel scanline is
    approximated, never exact. At export, a display pixel is an export pixel.
  - (b) As (a), and nodes flagged `device_px` (and the sheet) run after the display scale, at display resolution,
    a second print pass, so a scanline is exactly one screen pixel at every `render_scale`; the chain gains a stage.
  - (c) `device_px` is counted in render pixels as written, and the design doc §6.2's "true screen pixels" is read as
    "true pixels of the buffer the Print graph writes"; a screen medium coarsens while Auto lowers `render_scale`.
- **Applied meanwhile:** none. REQ-RENDER-110 (`rq`) and its leaf task TASK-M7-69 wait; every other Print graph task
  proceeds, testing `device_px` nodes and the anchored sheet at `render_scale = 1`, where the three options agree.

## RQ-550: The membrane rule's "exactly three crossings" (REQ-SYS-059) against v1 web, the whole native app in the browser, which needs more *(conflict: R-571 against REQ-SYS-059, systems_architecture §3 and canonical_spec §1 item 5; REQ-SYS-059, REQ-SYS-051, REQ-SYS-086; context: TASK-M8-38, TASK-M8-41)*

*Queued by R-571, which names it: "The membrane rule is the one place I expect a real conflict. … Do not redesign it.
Measure what v1 web actually needs to cross, write that down beside the rule, and **queue** it with options for me."
Nothing is ruled. For the human: yes, it changes what v1 web builds or offers.*

- **File, section:**
  - `plan/requirements.yaml`, REQ-SYS-059: "The wasm↔JS membrane must have exactly three crossings: set_field(path,
    value) as the GUI's only write, a GUI-sized (never engine-sized) state snapshot throttled to about 10 Hz, and the
    OffscreenCanvas handle transferred once at startup"; its verify, "the wasm exports are set_field, snapshot and the
    one-time canvas init".
  - `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not
    diminished)", the wasm↔JS table (Field edit, State snapshot, Canvas transfer): "It is thin and enumerable in the
    same way"; `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)", item 5:
    "**wasm↔JS** (`set_field` edits in, GUI-sized snapshots out, canvas transferred once)".
  - R-571: "REQ-SYS-059's "exactly three crossings" cannot hold for the whole native app in a browser: downloads, file
    drops, the clipboard and audio all cross."
- **The conflict:** the rule was drawn for a thin TS GUI across the membrane. v1 web is the whole native app, whose
  specified features need the browser for files, the clipboard, sound, storage and more. Even the design as written
  crossed more than three: the input deltas (SharedArrayBuffer or postMessage, caching_contract Part 6a), the resize
  message, the inspector worker's messages, and `wgpu`'s own calls into the browser's WebGPU; REQ-SYS-059's verify
  counts only the wasm exports.
- **Measured** (systems_architecture §3, "v1 web, and what it crosses", fourteen rows, each with the doc and section
  of the native feature): (1) input, text entry and the cursor; (2) resize; (3) the inspector worker's messages;
  (4) GPU commands to WebGPU; (5) downloads: images with pxpack, snapshots, sweep frames and GIFs, the benchmark's
  file, profiler captures; (6) video frames to WebCodecs; (7) file drops and picks: Import picture, load snapshot,
  custom colour occupants; (8) the clipboard: copy snapshot, share link, Copy summary, the console's Copy; (9) the
  page's URL, for share links; (10) audio, Listen; (11) persistent storage: presets, Your stains, warning
  suppressions, saved views, "save as default", the quality model, the benchmark's first launch; (12) device and page
  facts: adapter info and limits, the browser, a phone's viewport, visibility, clocks; (13) a wake lock; (14) the live
  profiler query (REQ-TOOL-128, its browser transport undefined). None carries the payload, the quad tree or the
  reductions.
- **Options seen:**
  - (a) **Two lists.** The three stay the engine↔GUI membrane's (state in, state out, the canvas once), its law
    unchanged; v1 web's platform crossings are a second, enumerated list (the measured rows), each plain data, none
    engine state, none engine-sized; review checks the wasm module's exports and imports against both lists.
  - (b) **One allow-list.** "Exactly three" becomes "exactly these": the three plus the measured rows, enforced in CI
    against the wasm module's imports and exports, as TASK-M5-30 enforces the GPU↔CPU membrane's five.
  - (c) **The law, not the count.** The count was the TS GUI's, so v2 web's; v1 web keeps the law alone (big data
    never crosses, data boundaries not object boundaries, the GUI holds no engine handles), with no count.
  - (d) **The rule as written.** Every native feature that needs another crossing is disabled in v1 web and says so in
    the app (R-571's own fallback): by the table, downloads, imports, the clipboard, Listen, the saved stores and the
    rest.
- **Its ruling must also settle the gaps with no native control to disable** (REQ-SYS-086 covers only gaps that have
  one; gui review 5608379011 of PR #282, applied per R-369): (i) runtime-authored colour occupants, the fragment
  registry being "the scanned filesystem" (row 7), where the browser has no folder to scan; (ii) persistence across
  reloads, the stores and the quality model (row 11); (iii) `prin profile query --live` (row 14). For each: crossed,
  replaced, or absent, and if absent, what v1 web shows.
- **Applied meanwhile:** the rule stands as written and nothing is built (M8); the measured list sits beside it, a
  record, not a rule. REQ-SYS-059 and REQ-SYS-051 carry `rq: RQ-550`, and REQ-SYS-086's list of what v1 web says it
  cannot do waits on it.
- **Waits:** TASK-M8-38 (REQ-SYS-051, REQ-SYS-059) and TASK-M8-41 (REQ-SYS-086).

## RQ-551: Where the GUI runs in v1 web, and what the main thread holds, now that v1 web's GUI is the native app's egui and not a TS DOM shell *(conflict: R-571 against caching_contract Part 6a, systems_architecture §3, §5 seam 12 and §6 invariant 5, canonical_spec §1 and §6; REQ-SYS-039, REQ-SYS-040, REQ-SYS-041, REQ-SYS-043, REQ-SYS-044, REQ-SYS-045, REQ-SYS-047, REQ-SYS-049, REQ-SYS-050, REQ-SYS-056; context: TASK-M8-37, TASK-M8-38)*

*Found by R-571's port, and queued as R-571 says: "Queue anything else that conflicts rather than resolving it."
Nothing is ruled. For the human: yes, it changes how v1 web is built.*

- **File, section:**
  - `docs/contracts/principia_caching_contract.md` § "Part 6a — The threading model: the render loop lives in a worker
    (and that worker is the wasm engine)": "The main thread is a thin TS shell: an input pump and a DOM-GUI host, a
    *different binary*, and it owns no simulation state."; and Part 6, item 3: "Time-budgeting this on the main thread
    was tried and **failed under real load — the scheduler hitched the gesture and the DOM GUI froze.**"
  - `plan/requirements.yaml`, REQ-SYS-049: "with the main thread only a TS input pump and DOM-GUI host, and a second
    wasm context as the f64 inspector worker"; REQ-SYS-047: "the GUI must be a thin TypeScript shell owning only DOM
    and view-scratch state" (its GUI clause now R-571's).
  - `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule": the egui binding is "compiled
    into the same wasm binary", and "Under the egui binding there is nothing to transfer: egui-wgpu shares the
    engine's `wgpu` context and paints the debug overlay onto the same surface in-process."
  - The native split, caching Part 6a (R-113): "the thread that takes input and draws the GUI never runs the
    scheduler, the cache or the GPU work."
- **The conflict:** R-571 makes the browser's GUI the native app's egui GUI. The corpus's browser layout puts a DOM GUI
  on the main thread, in a separate TS binary, with the engine in a worker; egui cannot live in that TS binary, so
  either it moves into a wasm context of its own on the main thread, or into the worker, or the worker goes.
- **Options seen:**
  - (a) **egui in the engine's worker**, painting onto the transferred OffscreenCanvas through the engine's `wgpu`
    context (gui_state_contract §1's egui binding as written); the main thread a small loader and input pump, no GUI
    and no state. Cost: in the browser the GUI shares the frame loop's thread, so R-113's split (the thread that
    draws the GUI never runs the scheduler, the cache or the GPU work) would not hold for GUI drawing in the browser;
    a long frame delays the GUI's redraw, and the mitigation, the scheduler time-budgeted per frame on that thread, is
    the one Part 6, item 3, quoted above, records as having "failed under real load — the scheduler hitched the
    gesture and the DOM GUI froze". The other way, egui's layout, tessellation and paint take from the worker's frame
    budget, and the frame record's `stage_ms` has only its fixed stages (integrate, reduce, colour, upload, present,
    and `post` for the Print graph; `principia_dd_telemetry_and_tiers.md` §2, R-56, R-488), so that time would show
    in none of them. Text entry and the clipboard need the main thread's help (RQ-550's rows 1 and 8).
  - (b) **egui on the main thread, in a wasm context of its own**: the gui crate against the engine surface, with
    `set_field` and the snapshot posted between the two contexts as serialised data; the engine and the frame loop stay
    in the worker. R-113's split holds as natively; REQ-SYS-059's three become messages between two wasm contexts.
    Cost: two surfaces (the figure on the worker's OffscreenCanvas, the GUI on a main-thread canvas above it) or a
    compositing step; the main-thread bundle holds the GUI but no engine state.
  - (c) **No worker**: the whole app on the main thread, as egui's own web target runs, the frame loop time-budgeted.
    Against Part 6, item 3, quoted above. With no worker at all, the inspector worker goes too, and in the browser
    REQ-SYS-039, -040, -041, -044, -045, -049 and -050 would go, as would the browser form of REQ-SYS-031 (the main
    thread never waits on scheduler work; here it runs it), REQ-SYS-060 (trajectory integration and sonification in
    the inspector worker) and REQ-PERF-072 (the hover integration in an inspector worker); those three are closed by
    native tasks and carry no `rq`.
- **Applied meanwhile:** none; the layout stands as written in the corpus and nothing is built (M8). The requirements
  above carry `rq: RQ-551`; REQ-SYS-047's GUI clause is changed by R-571 itself (the GUI is the native app's; a TS GUI
  shell is v2 web's), and only its main-thread clause waits. REQ-SYS-042 (the coi-serviceworker) is unaffected under
  (a) and (b).
- **Waits:** TASK-M8-37 and TASK-M8-38.

## RQ-552: R-570's "No hatch, no blank, and no 'not a system' on these charts at any pan or zoom" against energy normalisation η_E, which dd_decoder §3.7 allows on the latent chart and whose tags R-511 draws blank, at the default links, near the origin as well as far out *(conflict: the human's ruling against the decoder's definitions; R-570, R-511, R-25, dd_decoder §3.7; REQ-CHART-087, REQ-GUI-357; context: TASK-M8-55, TASK-M8-57)*

*Found by physics review 5624655553 of PR #279, P1. Nothing is ruled. For the human: yes (physics: what a pixel whose
energy target cannot be met is on a latent chart).*

- **File, section:**
  - `docs/incoming/2026-10-21_latent_unbounded/INSTRUCTION_latent_unbounded_2026-10-21.md` § "The behaviour", item 3:
    "**Every pixel is a system, everywhere.** No hatch, no blank, and no "not a system" on these charts at any pan or
    zoom."
  - `docs/design/principia_dd_decoder.md` § "3.7 Energy normalisation (optional; keep-or-drop is decision B9)",
    "Where it is allowed": "Only on charts that decode arbitrary momenta without enforcing an energy: the
    free-Jacobi-momentum chart and the latent chart."; its cases: "**$E^* < U$:** $\eta_E$ is not real, since the
    kinetic energy would be negative. The decode is tagged `DEGENERATE(ENERGY_INFEASIBLE)`." and "At $E^* > U$ the
    decode is tagged `DEGENERATE(ENERGY_UNSCALABLE)`; the golden IC, `z = 0` at rest, is so tagged under any
    $E^* > U$."
  - `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type
    (R-407)", "Blank, never a colour (R-470)": "a `decode_failed` sample the chart does not classify (a non-finite
    decode, η_E's tags, the mass block's `M01_TINY`) blank, as a pixel that is not a system (R-511)".
- **The conflict:** with η_E off (`None`, R-25) no latent pixel carries an η_E tag. With `Some(E*)`, which §3.7 allows on
  the latent chart, a pixel whose potential is above a bound target is `ENERGY_INFEASIBLE` and a rest start under
  `E* > U` is `ENERGY_UNSCALABLE`; each is `decode_failed`, drawn blank as not a system (R-511), near the origin as
  well as far out and at the default links. Item 3 rules that out, and the port's qualifier "at the decoder's default
  links" does not cover it, η_E not being a link.
- **Options seen:**
  - (a) R-570's item 3 holds with η_E off; under `Some(E*)` the tags stand and those pixels are not a system, blank, as
    the decoder defines them.
  - (b) η_E is forbidden on the latent chart and the flat and body-plane slices (`forbids_energy_normalisation`
    true), so item 3 holds on them always; the free-Jacobi-momentum chart keeps it.
  - (c) R-570 governs: on these charts an η_E-tagged pixel is a labelled system, drawn and integrated as decoded,
    without the normalisation, the tag a label rather than "not a system".
- **Applied meanwhile:** the port's statements and tests read "at the decoder's default links, η_E off (`None`)":
  far_field_saturates and latent_unbounded_navigation pin `E* = None`. Nothing changes in the decoder.
- **Waits:** REQ-CHART-087's arm under `Some(E*)` (TASK-M8-57), `rq: RQ-552`. The η_E-off arms do not wait, and
  neither does REQ-GUI-357, which states the η_E-off case.
- **Also (physics review 5624721489 of PR #279, an observation):** dd_decoder §3.7 tags the collision pole too:
  "**The collision pole, $U = -\infty$** (a pair separation is 0, §3.6, R-436): $E^* - U$ is infinite and no finite
  $\eta_E$ exists. The decode is tagged `DEGENERATE(ENERGY_UNSCALABLE)`." It does not change the conflict or the
  options.

## RQ-554: A band quad on a latent or flat chart, saturated along one in-plane axis only, collapses, switches, collapses again on the linear path and stops terminally at `AT_F32_FLOOR` above the screen floor, so the strip that "stays structured" stops refining, against R-108's at-rest target *(physics, conflict: the ladder's terminal stop against R-108 and R-570's port; R-570, R-108, R-88, R-90; REQ-SCHED-062, REQ-DEC-033; context: TASK-M6-08, TASK-M8-57)*

*Found by perf review 5624936602 of PR #279, P2. Nothing is ruled. For the human: yes (physics: whether a quad whose
collapse is saturation along one axis keeps refining, which changes what is drawn).*

- **File, section:**
  - `docs/design/principia_deep_zoom.md` § "4. Sliding depth bound": "(iv) the **decode floor proper**
    (`AT_F32_FLOOR`) — only if the *linearised* decoder's samples collapse (~depth 50+), genuinely terminal, a
    deep-zoom backstop"; and R-570's port, as it stood: "Stopping refinement there is harmless, the field being flat."
  - `plan/requirements.yaml` REQ-SCHED-062: "sample collapse on the linearised decoder must fire AT_F32_FLOOR, a
    terminal stop cached as a quad fact".
  - `decisions.md` § "R-108 — Must-split above the floor is the at-rest target *(closes RQ-68)*": "During a gesture
    the frame budget governs: ancestors show, so there are never blanks, and completeness resumes at rest."
  - `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)":
    "a view goes flat only where every coordinate its plane varies is saturated, so along each in-plane axis a strip a
    few units of `z` wide stays structured out to any distance".
- **The conflict:** at `z₀ = 10⁴·ê_k` a quad's full-decoder pairs along `ê_k` collapse, so `quad.collapsed` is set and
  it switches (REQ-DEC-033); its linear samples along `ê_k` collapse again, `J_D`'s `ê_k` column being 0, so
  `AT_F32_FLOOR` fires, a terminal stop never re-queued (REQ-SCHED-062). The quad stops near depth 0 while its other
  in-plane axis has structure above the screen floor, which at rest stays at the terminal quad's `N × N` resolution,
  against R-108's "completeness resumes at rest" and the strip's "stays structured". "Harmless" holds only where
  every in-plane `J_D` column is 0.
- **Options seen:**
  - (a) A quad every collapsed pair of which is a saturation collapse (RQ-553's per-direction test) is not terminal:
    it keeps refining along its unsaturated direction to the screen floor, its cost still screen-bounded
    (REQ-SCHED-104's at-rest bound).
  - (b) The stop is accepted: the band stays at the terminal quad's resolution, and the readout says so.
  - (c) Other: for example, the switchover's collapse test reads only the directions whose `J_D` column is non-zero.
- **Applied meanwhile:** the terminal stop stands as REQ-SCHED-062 defines it; deep_zoom §4's "harmless" is limited
  to quads whose in-plane `J_D` columns are all 0, and the band quad's case points here.
- **Waits:** REQ-SCHED-062's behaviour on band quads (TASK-M6-08), `rq: RQ-554`. Its other arms, and the rest of
  TASK-M6-08, do not wait.

## RQ-555: The horizon's range now that it can be lowered: the lowest `T_horizon`, and whether 200 still caps it *(found by R-575's port; R-575 against R-132 and RQ-524; REQ-INT-081, REQ-INT-026, REQ-GUI-102; context: TASK-M3-06, TASK-M8-24)*

*Found by R-575's port. The human ruled that the horizon "shouldn't be capped to 50 should be able to raised or
lowered", and gave no lowest value. For the human: yes, it sets what the Run window accepts.*

- **File, section:**
  - `decisions.md` § "R-132 — The R-71/R-72 classification is accepted, with three changes *(closes RQ-110)*":
    "REQ-INT-081: `dt_macro = max(1e-3, T/65535)`. T keeps [50, 200], with a default of 50; u16 indices stay exact
    (R-86)."
  - The human, 21 Oct 2026 (`designs/honesty_precision.md`, "Decided"): "**Horizon**: default stays 50, and it can be
    **raised or lowered**, not floored at 50 ("Keep horizon to 50. (Also horizon shouldn't be capped to 50 should be
    able to raised or lowered)"). Amends the Run window's `T_horizon ∈ [50, 200]` (render_gui_spec §G5) and R-132."
  - `docs/gui/principia_render_gui_spec.md` § "Run — from the top bar"; `docs/contracts/principia_integrator_contract.md`
    § "Part 3 — Parameter ownership (who owns what)" and § "Part 5 — Units and the horizon (inherited scale gauge)".
- **The question:** the floor at 50 goes; nothing says what replaces it. Below 50 nothing breaks arithmetically
  (`dt_macro = max(1e-3, T/65535)` gives `⌈T/dt_macro⌉ ≤ 65535` at any `T > 0`), but a short horizon leaves most
  pixels without an outcome, and the escape test needs its window (0.4 time units, R-29) and the sync grid its quantum
  (R-565). "Raised" is already possible up to 200; whether the ruling means past 200 is not said. RQ-524 (the
  unregularised `dt_macro` depending on `T` above 65.535) is unaffected by lowering.
- **Options seen:** the floor: (a) any `T > 0`; (b) a floor that lets every detector act, the escape window or one
  sync quantum `Δ_sync`, whichever is larger; (c) a calibrated floor (R-71), proposed with evidence. The cap: (i) 200
  stays; (ii) it is raised (the u16 rule holds at any `T`, so the cap is not needed for exactness).
- **Applied meanwhile:** (a) and (i): `T_horizon ∈ (0, 200]`, default 50, a value at or below 0 refused with its reason.
  Nothing waits.
  Above 200 the field clamps to 200, as before R-575. A press of ↓ (with or without Shift or Alt) or a drag that would
  take T to 0 or below sets T to the Alt step, 0.2, if T is above it, and otherwise leaves T unchanged; ↓ never raises
  T (fix rounds 1 and 3 of PR #283, gui reviews 5627122883 G8 and 5627280803 N3, applied per R-369).

## RQ-556: What the header strip states about the horizon, against REQ-SYS-061's "The interactive atlas must state its predictability horizon" *(found by R-574's port; R-574 against REQ-SYS-061 and philosophy §4.1; context: TASK-M8-41)*

*Found by R-574's port, and queued as the bundle's instruction says: "Where anything here conflicts with a ruling or a
queue entry these texts don't address, queue it rather than resolving it." For the human: yes, it changes what the
atlas's chrome says.*

- **File, section:**
  - The human, 21 Oct 2026 (`designs/honesty_precision.md`, "Decided"): "A single global "f32 trustworthy to ~23" is
    right pooled and wrong for any given point, which is why the readout is per point."; "**Nothing changes in the
    atlas**: it stays f32 on the GPU, coloured as now"; "The header states the horizon; the viewer states each point's
    parting."
  - The draft's §3, "The atlas statement": "The header strip reads: "Horizon `T = 50` · f32 trustworthy to about
    `t ≈ 23` (derived, not yet measured on this GPU)", changing to "measured" once REQ-VAL-071 runs. A stain preset,
    **How far f32 sees**, draws `ln(1/eps)/ftle`."
  - `plan/requirements.yaml`, REQ-SYS-061: "The interactive atlas must state its predictability horizon, and nothing
    downstream may treat an atlas pixel past the horizon as a result."; `docs/read_first/principia_00_philosophy.md`
    § "4.1 The instrument must say where it stops knowing".
- **The question:** "the horizon" the header states may be `T` (the run's horizon) or the predictability horizon
  REQ-SYS-061 asks the atlas to state; the human's reason for the per-point readout argues against a global "≈ 23",
  which is how the draft's header met REQ-SYS-061. The stain preset is not made either way (R-574: nothing changes in
  the atlas).
- **Options seen:** (a) the header strip states `T` only ("Horizon T = 50"), and REQ-SYS-061's predictability horizon
  is stated per point, by the viewer's trust line and its FTLE estimate; REQ-SYS-061 restated so. (b) The header
  states `T` and the draft's derived f32 horizon ("f32 trustworthy to about t ≈ 23 (derived …)", "measured" once
  REQ-VAL-071 runs); REQ-SYS-061 unchanged. (c) Nothing is added to the header; REQ-SYS-061 is met as its verify
  reads, by labelling exports and statistics built from atlas data as exploratory.
- **Applied meanwhile:** none; nothing is built before TASK-M8-41. REQ-SYS-061 carries `rq: RQ-556`.
- **Waits:** TASK-M8-41 (REQ-SYS-061).

## RQ-557: The ensemble ratio's expected value: `ln eps₃₂ / ln eps₆₄ ≈ 0.44`, against the probe's readings *(found by R-574's port; physics; REQ-VAL-199; context: TASK-M8-76)*

*Found by R-574's port. The draft's test is the human's; the measurement since does not settle its value. Physics,
where the choice changes what passes: for the human.*

- **File, section:**
  - `docs/design/principia_honesty_precision.md` § "6. Tests (each with what makes it fail)", the draft's
    **ensemble_ratio**: "over ≥ 256 chaotic ICs, median `t_part(f32)/t_part(f64)` near `ln eps₃₂/ln eps₆₄ ≈ 0.44`.
    Never read off one trajectory."
  - `docs/incoming/2026-10-21_gap_sweep_results_honesty/evidence/parting_probe_output.txt`, "deep interior", at
    `θ = 0.1`: f32 vs f64 median 12.1, 95 % parted by 50; f64 vs f64′ median 39.4, **19 %** parted by 50.
- **The question:** exponential growth from a starting gap `δ₀` predicts `t_part ≈ ln(θ/δ₀)/λ`, so the ratio is
  `ln(θ/δ₃₂)/ln(θ/δ₆₄)`, not `ln eps₃₂ / ln eps₆₄`, unless `θ` is ignored: at `θ = 0.1` with the probe's deep-interior starting
  chords (about 2e-7 and 1.6e-16 at `t = 0.1`) it is about 0.38; the draft's value is 0.44. The probe's medians give
  12.1 / 39.4 ≈ 0.31, but only over the 19 % whose f64 runs parted by 50, the earliest, so biased low. At `T = 50` most
  f64 runs do not part, so the gate needs a longer horizon (up to 200) or a conditioning rule.
- **Options seen:** (a) the draft's value, 0.44, the tolerance calibrated (R-71); (b) the predicted ratio from each
  IC's measured starting chords and the mark, `ln(θ/δ₃₂)/ln(θ/δ₆₄)`, the tolerance calibrated; (c) no pass/fail: the
  ratio measured and recorded, with the predicted value beside it. Under any: the ICs from the chaotic regions, at
  `T = 200`, over the ICs where both pairs part, the count reported.
- **Applied meanwhile:** none. REQ-VAL-199 carries `rq: RQ-557`.
- **Waits:** REQ-VAL-199 (TASK-M8-76's gate).

## RQ-558: Whether the viewer's four runs carry on past their first terminal event, as the marks were measured, or stop there as the atlas does *(found by R-574's port; physics; REQ-VAL-196, REQ-VAL-197; context: TASK-M8-76)*

*Found by R-574's port. Physics the texts do not settle: for the human.*

- **File, section:**
  - `docs/incoming/2026-10-21_gap_sweep_results_honesty/evidence/parting_probe.rs`, its header: "four Heggie runs to
    the horizon (termination off, so every run reaches it)"; `stop_on_event: false`, each run's fate its first event.
  - `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer": a terminal
    event latches, and the copy's march ends there.
  - `docs/design/principia_honesty_precision.md` § "3.1 Four runs of one IC", "Past a terminal event".
- **The question:** the marks (10⁻³, 0.1) and the agreement figures were measured on runs that carry on past their
  first event. The atlas stops at it, so does `f32_run_is_the_atlas`'s comparison of `t_end`. After a collision under
  regularisation off, carrying on integrates through a singularity the substep machinery is capped against.
- **Options seen:** (a) as measured: each run's fate and `t_end` are its first event, latched as the atlas latches it,
  and the run carries on to the horizon for the chord; a run that fails or runs out of its budget ends there, its chords
  read "ended at `t = …`" after it; (b) each run stops at its first event as the atlas does; the chord ends at the
  earlier end, and a pair that ends before parting reads "settled before the runs parted"; (c) (a) under a regularised
  occupant, (b) with regularisation off.
- **Applied meanwhile:** (a). REQ-VAL-196 and REQ-VAL-197 carry `rq: RQ-558`, so a ruling for (b) or (c) traces to
  them (fix round 1 of PR #283, physics review 5627073272, F2). *Was:* "(a). Nothing waits."
- **Waits:** nothing built waits; a ruling other than (a) changes REQ-VAL-196 and REQ-VAL-197 (TASK-M8-76) (physics
  review 5627216712 of PR #283, F6). *Was:* "**Waits:** TASK-M8-76 (REQ-VAL-196, REQ-VAL-197)."

## RQ-559: The Sequence window's undo: its own history, as the draft gives it, against R-52's one history kept by the contract *(found by R-577's port; R-577 against R-52 and gui_state_contract §2; REQ-GUI-378; context: TASK-M8-77)*

*Found by R-577's port, and queued as the bundle's instruction says: "Where anything here conflicts with a ruling or a
queue entry these texts don't address, queue it rather than resolving it." For the human: yes, it sets what Ctrl+Z
does in the Sequence window.*

- **File, section:**
  - `docs/incoming/2026-10-21_gap_sweep_keyframes_reel/designs/keyframes_reel.md` § "The Sequence window": "**Undo** is
    the window's own (Q4)."; § "5. Open questions", 4: "**Keyframe undo**: (i) own history; (ii) the contract's; (iii)
    none. *Recommend (i).*" The human's "Decided" section rules questions 1 and 2 only.
  - `decisions.md` § "R-52 — Undo lives in the state contract *(GU-1 (a))*": "Undo/redo is a history of typed
    `setField` edits, shared by every GUI."
  - `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface": "The
    contract keeps one undo/redo history of the typed `setField` edits it has applied, shared by every GUI: a GUI
    shows the depth … and sends undo / redo as requests, and keeps no history of its own."
- **The question:** the sequence being edited is not `SimConfig` or `RenderState`, so its edits (add, delete, Update, a
  drag in time, a playhead policy, a segment's easing or transition) are not the contract's `SetField`s. The draft
  gives the window its own history; R-52 lets a GUI keep none.
- **Options seen:** (i) the window's own history, Ctrl+Z in its scope undoing a sequence edit, the contract's history
  untouched (the draft's recommendation; an exception to R-52 for a document that is not contract state); (ii) the
  sequence becomes contract state, its edits typed `SetField`s in the one history, Go to and sequence edits
  interleaved; (iii) no undo for sequence edits.
- **Applied meanwhile:** none of them. Go to is one entry in the contract's history under every option.
- **Waits:** REQ-GUI-378, `rq: RQ-559` (TASK-M8-77). The rest of TASK-M8-77 does not wait.

## RQ-560: A cut offered at a link-parameter change in a blocking export, as the draft's segment control gives it, against R-520's export that always eases *(found by R-577's port; R-577 against R-520; REQ-GUI-376; context: TASK-M8-77)*

*Found by R-577's port, and queued as the bundle's instruction says. For the human: yes, it sets what an export of a
link-parameter change can be.*

- **File, section:**
  - `docs/incoming/2026-10-21_gap_sweep_keyframes_reel/designs/keyframes_reel.md` § "The Sequence window": "**A
    segment**: easing; cut or dissolve (with length) at a chart change; at a link-parameter change, **cut** or
    **interpolate (export only)**, showing R-520's "every frame a different decoder"."; § "4. Fit with existing
    rulings": "R-520 appears as the segment's link choice."
  - `decisions.md` § "R-520 — Link parameters interpolate across keyframes in a blocking export only, eased linearly
    with every frame re-integrated, and the export is flagged; live playback keeps the cut, and a changed link id is a
    cut everywhere *(closes RQ-384; amends R-483)*": "In a blocking export, link parameters interpolate across
    keyframes, each frame re-integrated at its own values".
  - `docs/contracts/principia_export_animation_contract.md` § "Part 3 — The keyframe system (retained)": "a blocking
    export (Part 4) eases each differing parameter linearly in its value".
- **The question:** R-520 makes the blocking export always ease a differing link parameter; the draft lets the user
  choose a cut instead, per segment.
- **Options seen:** (a) no choice: R-520 as written, the segment showing its rule; (b) a per-segment choice, cut or
  interpolate (export only), interpolate the default, the spec's segment recording it and the planner's flag naming
  only interpolating segments; (c) a choice, cut the default.
- **Applied meanwhile:** (a). Nothing waits: REQ-GUI-376 shows R-520's rule and offers no choice until this is ruled.

## RQ-561: A takeover inside a segment the film interpolates and live playback cuts (link parameters, dissolves): "Live and film disagreeing is a bug" against R-520 F3 *(found by R-578's port; R-578 against R-520; REQ-GUI-383; context: TASK-M8-79)*

*Found by R-578's port, and queued as the bundle's instruction says. For the human: yes, it sets what the live view
does after a takeover in such a segment, and which specs the reel may hold.*

- **File, section:**
  - `docs/incoming/2026-10-21_gap_sweep_keyframes_reel/designs/reel_click.md` § "Proposal", 2: "the frame's spec is
    loaded live (chart, view, stain, playhead, and any keyframed motion it was in the middle of), and the live engine
    continues from there"; 5: "the build refuses any spec whose link hashes don't match, so the live view always
    recreates what the film showed. Live and film disagreeing is a bug, reported."; "Decided": "**After takeover it
    keeps playing** whatever the clip was doing, now live".
  - `decisions.md` § "R-520 — Link parameters interpolate across keyframes in a blocking export only, eased linearly
    with every frame re-integrated, and the export is flagged; live playback keeps the cut, and a changed link id is a
    cut everywhere *(closes RQ-384; amends R-483)*", F3: "live playback shows the change and never interpolates it."
  - `docs/contracts/principia_export_animation_contract.md` § "Part 3 — The keyframe system (retained)": chart switches
    are "cuts or dissolves (two renders composited)"; live playback composites no dissolve.
- **The question:** the film is a blocking export, so in a segment that eases link parameters or dissolves between
  charts it shows what live playback cannot. A takeover there is exact at its frame, but the live view then continues
  differently from the film.
- **The second question (added by fix round 6 of PR #286, physics review 5631678451, P4 (b)):** after a takeover
  mid-way through a link-parameter ease, what does the live player hold until the segment's second keyframe? R-520 F3:
  "live playback shows the change and never interpolates it"; REQ-TOOL-175: "live playback of the same spec gives a cut
  at the second keyframe, the frames before it at 5 and from it at 4, none between"; `principia_spotlight_reel.md` §3.3:
  "the frame's spec is loaded live … and the live engine continues from there" and "It keeps playing … now live". The
  takeover loads the eased value (say 4.6); R-520 F3 as written then puts the next frame at the first keyframe's value
  (4.6 → 5, re-integrating, then 5 → 4 at the keyframe). Options seen: (i) hold the takeover's value until the second
  keyframe, then cut; (ii) R-520 F3 as written, back to the first keyframe's value; (iii) whatever (c) above gives,
  the export's ease continued live.
- **A dissolve frame (P4 (a), applied per R-369, not a question):** a dissolve frame was rendered from two states and a
  weight, so its manifest entry carries both hashes, the weight and which state live shows at that frame; the takeover
  loads that state.
- **Options seen:** (a) the takeover frame exact, live continuing under R-520 F3 (cut at the segment's second
  keyframe), the difference after that frame expected and not reported; (b) the reel build refuses a spec with such a
  segment, so live always continues what the film shows; (c) live playback after a takeover interpolates link
  parameters and composites dissolves as the export does (amending R-520 F3 for this case).
- **Applied meanwhile:** (a), made determinate without deciding either question: in a dissolve the takeover loads the
  state live shows at that frame; after a takeover in a link-parameter ease the live player holds the takeover's value
  until the second keyframe, then cuts (option (i) of the second question, as an interim only). Nothing waits.
  *Was:* "(a). Nothing waits." (fix round 6 of PR #286, physics review 5631678451, P4; applied per R-369).

## RQ-570: Table 1's column 8 (E·10⁻⁷) of Agekyan & Anosova (1968) does not equal eq. (4) on columns 2 and 3 *(found by TASK-M2-20; REQ-VAL-019; context: TASK-M2-20, the RQ-566 ruling's item 5)*

*Filed as the RQ-566 ruling's item 5 asks. Physics, for the human; not resolved here. TASK-M2-20 records column 8
without asserting it until this is ruled.*

- **File, section:** `docs/incoming/2026-10-24_anosova/papers/agekyan_anosova_1968_SvA_11_1006.pdf`, p. 1007 eq. (4),
  "E = −G(1 + 1/√((ξ+½)²+η²) + 1/√((ξ−½)²+η²)) + L₀", and Table 1, pp. 1010–1012, column 8 "E·10⁻⁷"; the RQ-566
  ruling (24 Oct 2026), item 5: "Table 1's column 8 (E·1e-7) does not equal eq. (4) evaluated on columns 2–3."
- **The comparison, all 100 rows** (G = 6.67·10⁻⁸, L₀ = K₀ = 7.5·10⁻⁹, m = 1; the 62 rows that print a value): the
  ratio column 8 / eq. (4) runs from 0.029 (N = 24: eq. (4) −7.228, printed −0.207) to 1.571 (N = 26: −2.056,
  printed −3.231), wider than the ruling's 0.23–1.25, which came from a sample. N = 1: −2.464 against −2.254; N = 3:
  −2.589 against −2.041; N = 10: −3.858 against −0.887, as the ruling found. The 38 rows that print "—" have nothing to
  compare. Every row's figures are in `fixtures/anosova/col8_comparison.csv`, committed beside the fixture.
- **What paper II says** (`agekyan_anosova_1968_Ap_4_11_part2.pdf`, p. 11 and Table 1, pp. 12–14): the same 100
  configurations ("exactly the same as in [1]"), units "the same as in [1]", started at exact rest; its column 6 is
  "the total energy E (with minus sign)". Its values sit K₀ = 0.075·10⁻⁷ from paper I's column 8, not from eq. (4):
  N = 1, II 2.329, I −2.254; N = 3, II 2.116, I −2.041 (both differ by exactly 0.075); N = 10, II 0.971, I −0.887
  (0.084). Eq. (4) at rest gives −2.539, −2.664 and −3.933. So both papers print one energy that is not eq. (4) on the
  printed (ξ, η): a scan or OCR error in one paper alone does not explain it. II's Table 3 gives −E·10⁻⁷ for the 38
  rows I left at "—".
- **Possible causes:** (a) column 8 (and II's column 6) is a different E from eq. (4): another configuration than
  the printed (ξ, η), another normalisation, or another quantity; (b) a print error common to both tables (unlikely,
  since II was set separately and agrees with I to K₀); (c) the printed (ξ, η) are not the configurations the
  energies were computed from, though column 11, printed on 62 rows, matches η·10⁻⁴ on 57 (RQ-571).
- **Also recorded here, as dispatched:** N = 17's ξ in paper I has a broken fifth decimal. Transcription A read
  0.115219340 (a damaged "1", flagged uncertain: "it could be a '7'"), B 0.115279340: the one difference between the
  two transcriptions. At 600 dpi the glyph is a slanted stem with no top bar, the slant of row 16's "7", while every
  "1" in the cell is upright with a flat foot, so it is read "7"; paper II prints 0.115279340 cleanly. The fixture
  carries 0.115279340 and both readings.
- **Applied meanwhile:** column 8 is transcribed and recorded, never asserted.
- **Options seen:** (a) column 8 stays recorded and unasserted at M2, and the Anosova chart's milestone decides; (b)
  the human identifies column 8's definition and it is asserted as that.

## RQ-574: R-482 C3's mass-varying chart is the mass simplex chart, whose masses `μ_max` cannot move *(found by the physics re-check 5643056937 of PR #289, TASK-M2-41; R-482 C3 against `crates/kernel/src/chart/mass_simplex.rs`; REQ-CHART-074; context: TASK-M4-21)*

*Found in TASK-M2-41's fix round 2. C3's choice was "applied per R-369", not the human's words, so a reviewer can
settle it; queued because the requirement's verify names the chart, and a test written to it cannot pass.*

- **File, section:**
  - `decisions.md` § "R-482 — The tests of R-478 to R-481, …", C3: "REQ-CHART-074 (TASK-M4-21). The mass-varying chart
    is the mass simplex chart; the constant-mass control is a latent configuration view with both mass logits at 0, the
    equal masses `(⅓, ⅓, ⅓)`, which `μ_max` does not move (`μₖ = μ_max·tanh 0 = 0`). … (applied per R-369)."
  - `plan/requirements.yaml` REQ-CHART-074: "`mu_max = 4` against 5 must give different payloads on the mass simplex
    chart and identical payloads on a latent configuration view with both mass logits at 0"; verify: "cargo test -p
    engine link_param_moves: mu_max 4 vs 5 on the mass simplex chart, payloads differ".
  - `crates/kernel/src/chart/mass_simplex.rs`, the module's note (lines 1–17): "the masses swept over the simplex, the
    bodies at rest"; "the masses through the shared entry for a chart that sets masses
    ([`crate::decode::mass::from_masses`] …), … every momentum 0". The chart sets its masses directly: no mass link and
    no momentum link is read, so `μ_max` (and `q_max`) cannot reach its payloads (TASK-M2-41's `Chart::decode_params`
    keeps the default, which ignores the parameters, for this chart).
- **The question:** `link_param_moves`' first arm, "payloads differ" on the mass simplex chart, cannot pass; which
  chart is the mass-varying one?
- **Options seen:** (a) a latent view at a fixed non-zero mass logit (say `z_μ = (0.5, −0.3)`), whose masses
  `softmax(0, μ_max·tanh z_μ)` move with `μ_max`, or a mixed chart with a mass axis (`z_μ₁`), whose masses move at
  every pixel off `z_μ₁ = 0`; the constant-mass control unchanged; (b) other.
- **Applied meanwhile:** none: TASK-M4-21 is not built. Waits: REQ-CHART-074 (TASK-M4-21).

## RQ-576: Under which equivalence the computed free-group word is compared with the published one: the Belgrade gallery and the 2013 PRL write the same orbits' words with different representatives *(definition, R-72; the human's queued question on the 26 Oct incoming record; R-125; REQ-VAL-043, REQ-PAY-070; context: TASK-M3-30, TASK-M3-16, TASK-M8-35)*

*Queued by the human, 27 Oct 2026, on `docs/incoming/2026-10-26_belgrade_gallery/` (PR #296): "So the reader's
comparison must declare its equivalence: free and cyclic reduction, plus which symmetries count. That is a definition
requirement (R-72): queue it in REVIEW_QUEUE.md with the crosscheck.txt numbers. Don't pick it yourself." The options
are listed with no choice made.*

- **File, section:**
  - `decisions.md` § "R-125 — The branch-cut convention is M3's, transcribed from the literature *(closes RQ-96)*": "Its
    convention (the a/b branch-cut assignment and the crossing sign) is transcribed from the literature (Montgomery;
    Šuvakov–Dmitrašinović) and verified by REQ-VAL-043 against the published braid classes."
  - `plan/requirements.yaml` REQ-VAL-043: "A word-calculus test must assert the free-group word encoder reproduces the
    published braid class of Šuvakov–Dmitrašinović orbits."; verify: "for each gallery orbit used, computed word (under
    REQ-PAY-070's transcribed convention) equals the published class". TASK-M3-30's acceptance line:
    "`cargo test -p validation word_braid_class` — for each gallery orbit used, the computed word, under REQ-PAY-070's
    transcribed convention, equals the published class (REQ-VAL-043)."
  - `docs/design/principia_dd_validation_orbits.md` § "1.4 Šuvakov–Dmitrašinović (13 families, 2013) and
    Broucke–Hénon–Hadjidemetriou": "a known orbit has a known word, and if the encoder produces a different one, that is
    a bug with a named right answer."
  - `docs/incoming/2026-10-26_belgrade_gallery/crosscheck.txt` § "3. Šuvakov & Dmitrašinović 2013, Table 1", and the
    record's `README.md` § "The cross-check (`crosscheck.txt`)", item 3: "**Their words agree only up to a symmetry, and
    which symmetry varies by orbit**"; "So the gallery and the PRL write the same conjugacy classes with different
    representatives. The encounter-word reader (R-125) must declare which equivalence it compares under, and a comparison
    up to rotation alone would fail 6 of these 15 for a reason that is not an error."
- **The numbers (`crosscheck.txt` § 3).** `crosscheck.py` freely and cyclically reduces each PRL word, then lists every
  variant of it whose cyclic rotations contain the gallery's word: `cyclic` (the word itself), `inverse` (reversed and
  each letter inverted), `reverse` (reversed), `a<->b`, and `(a,b)<->(A,B)`. Of the 15 orbits of the PRL's Table 1
  (`p1`, `p2`, `T` agreeing within the PRL's printed precision, largest difference 4.1e-5):
  - **9 match up to cyclic rotation alone:** butterfly I (`cyclic, (a,b)<->(A,B)`), butterfly II (`cyclic,
    (a,b)<->(A,B)`), bumblebee (`cyclic, (a,b)<->(A,B)`), moth II (`cyclic, reverse, a<->b`), moth III (`cyclic,
    reverse, a<->b`, after reduction, below), butterfly IV (`cyclic, reverse, a<->b`), yarn (`cyclic, reverse, a<->b`),
    yin-yang 2a (`cyclic`), yin-yang 2b (`cyclic`).
  - **6 match only under a symmetry:** moth I (`inverse, (a,b)<->(A,B)`; PRL `ba(BAB)ab(ABA)`, gallery `abaBAbabAB`),
    butterfly III (`inverse, (a,b)<->(A,B)`; PRL `(ab)2(ABA)(ba)2(BAB)`, gallery `ABabaBABAbabAB`), goggles (`inverse,
    (a,b)<->(A,B)`; PRL `(ab)2ABBA(ba)2BAAB`, gallery `abbaBABAbaabABAB`), dragonfly (`inverse, (a,b)<->(A,B)`; PRL
    `(b2(ABabAB))(a2(BAbaBA))`, gallery `abABabAAbaBAbaBB`), and, under the exchange only, yin-yang 1a and yin-yang 1b
    (`(a,b)<->(A,B)`; PRL `(ab)2(ABA)ba(BAB)` for both, gallery `AbabABABabaB` and `ABabaBAbabAB`).
  - **Moth III's published word is not freely reduced:** `(babABA)²a(abaBAB)²b` contains `A·a`, 26 letters against the
    gallery's 22 (`AbabABAbabABabaBABabaB`); after free and cyclic reduction it matches up to rotation.
  - The PRL words above are as `crosscheck.py`'s table `T1` transcribes them from the PRL (a record, not checked here).
- **The question:** REQ-VAL-043's "equals the published class" does not say under which equivalence a computed word
  equals a published one. Which reduction is applied to each side (free reduction; cyclic reduction), and which
  symmetries count as the same class: none (cyclic rotation only), the inverse word, the reversed word, the exchange
  `a ↔ b`, the exchange `(a, b) ↔ (A, B)`, or a set of these. A second part: whether the encoder's own word (REQ-PAY-070's
  transcribed convention, not yet written) is compared with the gallery's representatives or the PRL's, which differ by
  the symmetries above, and whether the published side is the gallery, the PRL, or both.
- **Options seen** (none chosen, as the human asked):
  - (a) free and cyclic reduction of both words, then equality up to cyclic rotation only (the conjugacy class); 6 of
    the 15 fail against the PRL's words, none against a source written in the encoder's own convention.
  - (b) (a), plus the inverse word counted as the same class.
  - (c) (a), plus the exchange `(a, b) ↔ (A, B)` counted as the same class.
  - (d) (a), plus both the inverse and the exchange: every one of the 15 matches.
  - (e) (a), plus every variant `crosscheck.py` lists (inverse, reverse, `a ↔ b`, `(a, b) ↔ (A, B)`) and their
    compositions.
  - (f) a per-orbit declaration: each reference row names the symmetry under which its published word relates to the
    encoder's, and the comparison applies exactly that one.
  - (g) other.
- **Recorded, not resolved (the human: "Record, don't resolve"):**
  - **Two periods for butterflies I and II.** The record's `README.md` § "One internal discrepancy, recorded": "the main
    page has the PRL's 6.235641 and 7.003907, the sequence pages (I.2.A, I.2.B) have 6.234671 and 7.003707, with the same
    `p1`, `p2` and energy. The sequence values agree with the thesis's scale-invariant `T|E|^{3/2}` (19.9325 and 19.9313,
    Table 3) and with the Three Body Orbits atlas's refined periods (6.23489 at E = −2.170144, 7.00370 at E = −2.008195),
    so the PRL's are the outliers, at about 1e-4." Both values are in `belgrade_gallery.csv` as the pages give them.
  - **Five thesis labels renamed later** (`crosscheck.txt` § 4; `README.md` item 4): thesis IVa.6.A, IVa.8.A, IVa.8.B,
    IVa.8.C and IVb.15.A are gallery IVa.14.C, IVa.8.F, IVa.8.G, IVa.8.H and IVb.15.D (matching to 1.0e-6, 1.5e-6,
    1.7e-6, 2.0e-6 and 5.1e-7), and thesis V.4.A is gallery V.4.A.α (3.5e-6). The human: "the gallery's labels are
    current"; the README: "The later labels are the gallery's and the 2019 table's; the thesis's are earlier."
- **The reference, as the human named it:** "belgrade_gallery.csv is the reference for the encounter-word reader
  (R-125), together with Hudomal 2015 and the 2013 and 2018 papers. Transcribe twice, as usual." The sources:
  `docs/incoming/2026-10-26_belgrade_gallery/belgrade_gallery.csv` (sha256 `3558b26b…a839335b`); A. Hudomal's 2015
  thesis; Šuvakov & Dmitrašinović, *Phys. Rev. Lett.* 110, 114301 (2013)
  (`docs/incoming/2026-10-25_replication/papers/suvakov_dmitrasinovic_2013_PRL_110_114301_arXiv1303.0181.pdf`); and
  Dmitrašinović, Hudomal, Shibayama & Sugita, *J. Phys. A* 51, 315101 (2018)
  (`…/papers/dmitrasinovic_etal_2018_JPhysA_51_315101_arXiv1705.03728.pdf`). The 2019 table
  (`archive/initial-conditions.pdf`) agrees with all 206 sequence orbits to every printed digit (`crosscheck.txt` § 1).
  **Credit line**, from the gallery's own terms (`archive/ref.php.html`: "You may copy and use any images or data from
  this web gallery. Please, credit us by citing this page and our paper [8]."): every use cites
  `http://three-body.ipb.ac.rs/` and *Phys. Rev. Lett.* 110, 114301 (2013).
- **Unclear in the records, said here, not resolved:**
  - The "encounter-word reader" is the human's and the record's term; the corpus's are R-125's convention
    (REQ-PAY-070, TASK-M3-16) and REQ-VAL-043's "free-group word encoder" (TASK-M3-30). This entry reads them as the
    same comparison.
  - The Hudomal 2015 MSc thesis is not on `main` yet: it arrives with open PR #295 (`docs/orbit-catalogues`) as
    `docs/incoming/2026-10-26_orbit_catalogues/hudomal_2015_msc_belgrade.pdf`. `crosscheck.txt`'s command takes it as a
    path argument (`<Hudomal 2015 thesis>`).
  - R-125 says the convention is "transcribed from the literature (Montgomery; Šuvakov–Dmitrašinović)"; the gallery and
    the PRL, both Šuvakov–Dmitrašinović, already differ by the symmetries above, so which of their representatives
    REQ-PAY-070's transcription follows is part of the question.
- **Applied meanwhile:** none; TASK-M3-30 and TASK-M3-16 are not built. No requirement is added or changed by this
  entry (the human's instruction for this PR is the queue only); the definition requirement R-72 asks for, and
  REQ-VAL-043's `rq: RQ-576`, are the port's next step.
- **Waits:** REQ-VAL-043's comparison (TASK-M3-30, `word_braid_class`). The rest of TASK-M3-30, and TASK-M3-16's
  convention, do not wait on the equivalence.

## RQ-577: The precision-ladder proposal's seven open questions (§6), and the amendments it proposes to R-265, R-572, R-90, canonical spec §1 item 3, deep zoom §2 and its layer table, and integrator contract Part 2 and Part 2b *(the human's queued questions on the 26 Oct incoming proposal; R-265, R-572, R-90, R-561, R-547, R-33; context: none built)*

*Queued by the human, 27 Oct 2026, on `docs/incoming/2026-10-26_precision_ladder/` (PR #297): "A PROPOSAL. Nothing in
it is in force and nothing in it is built until I rule on its §6." and "Do not amend R-265, R-572, R-90, the canonical
spec, deep_zoom or the integrator contract from it." **Nothing in the proposal is in force and nothing in it is built
until the human rules on its §6.** For the human: yes, every question changes what is built. No option is recommended
here.*

- **File, section:** `docs/incoming/2026-10-26_precision_ladder/PROPOSAL_precision_ladder_and_taylor_2026-10-26.md`
  (sha256 `57eca498…852b192d7`), § "6. Rulings touched and open questions"; its design in § "3. The design" (3.1 to
  3.10), its build order in § "4. Build order" and its twelve tests in § "5. Tests (each with what makes it fail)". Its
  header: "**Draft design note, 26 Oct 2026. Not authority.** It proposes changes to rulings in force (R-265, R-572,
  canonical spec §1 item 3, R-90). Nothing here is built until the human rules on §6."
- **The amendments it proposes** (§6, "Rulings it would amend", quoted):
  - **Canonical spec §1 item 3:** "**Canonical spec §1 item 3**, "The GPU is f32-locked". Proposed reading: the GPU's
    native arithmetic is f32 (and f64 where the hardware offers it); extended precision on the GPU is emulated, opt-in,
    and guarded by the canary." In force: `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the
    defining decision)", item 3: "**The GPU is f32-locked** (WebGPU has no f64)."
  - **R-265:** "**R-265**, the kernel is f32 and f64 only and `Dd` is parked. Proposed: the kernel's `Real` gains `df32`,
    `qf32`, `dd64` and `mp(bits)`. R-265's condition for unparking has two clauses, "a precision question needs it and
    Brutus is too slow". The case made here is the Burrau neighbourhood mapped past t ≈ 52 (§2), where a per-pixel Brutus
    run is too slow; the single trajectory does not meet the second clause." In force: `decisions.md` § "R-265 — The
    shared kernel is f32 and f64 only; double-double is parked *(closes RQ-161, amends R-33)*".
  - **R-572:** "**R-572**, "f32 or f64 only" in a Result. Proposed: the ladder, with f64 still the default."; and "**R-572
    and results §3.2**: a Result's uniform grid is a complete tree at one level, run in quads rather than rows (§3.10)."
  - **Integrator contract Part 2 and Part 2b:** "**Integrator contract Part 2 and Part 2b**: Taylor joins the stepper
    axis; precision becomes a third axis; R-561's warn-never-block extends to it." (`docs/contracts/principia_integrator_contract.md`
    § "Part 2 — Occupants and the capability profile" and § "Part 2b — Regularisation is a SECOND swappable axis, not a
    property of the stepper".)
  - **R-90:** "**R-90**, `ℓ_switch = 20` as a fixed bound: per precision (§3.9)." Its proposed rule (§3.9): "`ℓ_switch`
    becomes a per-precision backstop, `ℓ_switch(P)`, never below `b(P)/2`, and is a frozen value of the sim key (Part 2c
    rule 1)."; and "`AT_F32_FLOOR` generalises to `AT_PRECISION_FLOOR(P)`."
  - **Deep zoom §2:** "**Deep zoom §2**, "Floor moves from ~23 to ~50+" with an f32 sum on the GPU: contradicted by the
    prin-rs measurement (§3.9 item 4). This one stands on its own and goes to `REVIEW_QUEUE.md` even if the ladder is not
    adopted." That finding is RQ-578, queued separately as the human asked.
  - **Deep zoom's layer table:** "**Deep zoom's layer table**: the linearised decoder belongs to depth, not to layer 2's
    adaptivity (§3.10)." In force: `docs/design/principia_deep_zoom.md` § "3. Three-layer quadtree", layer 2: "CPU→GPU
    adds the linearised-decoder uniforms".
  - Held, not amended, by the proposal's own account: "**Integrator contract Part 2c** is held, not amended"; "**R-547**
    governs the emulated arithmetic's branch inputs and forbids an FMA-based TwoProd on this project's backends (§3.5)";
    "**R-33** is unchanged; §3.8 states why."
- **The seven open questions** (§6, "Open questions for the human", verbatim):
  1. "Unpark extended precision now, or only after M3 (the current integrator milestone) closes?"
  2. "Is the live view allowed the ladder by choice (§3.6), or is it Result-only?"
  3. "Taylor's default order rule: Jorba & Zou's, or a fixed order per tier?"
  4. "What does the profile's `force_evals` mean for Taylor: a measured equivalent, or a new field?"
  5. "Event location by dense output (§3.4): adopt it for Taylor only, or keep events where every stepper samples them,
     so steppers stay comparable?"
  6. "Should a Result above f64 be able to store raw extended state, or rounded fields only?"
  7. "Per-quad precision in the tree: may a deep quad run at a higher rung than its parent, so that only the deep part of
     a view pays for it? It costs the scheduler a precision field per quad and makes neighbouring quads differ in
     arithmetic, which R-547's determinism rules would have to cover."
- **Beside the seven, implied by the amendments:** whether the ladder is adopted at all (the amendments above, each
  accepted, changed or refused), and whether uniform mode becomes "a complete tree at one level" (§3.10), which the
  proposal lists as an amendment but not as a question.
- **Unclear in the record, said here, not resolved:**
  - The proposal's header names four rulings it changes (R-265, R-572, canonical spec §1 item 3, R-90); its README and
    §6 add deep zoom §2 and its layer table, integrator contract Part 2 and Part 2b, and §6 adds results §3.2
    (`docs/design/principia_results.md` § "3.2 Where it runs: the job"). This entry lists every one §6 names.
  - Test 6 (§5) reads `atlas-params.json`, which "**is held outside the repo** until its data terms are confirmed
    (`docs/incoming/2026-10-26_orbit_catalogues/`, unmerged)".
  - §7's sources are "not yet checked against the papers".
- **Applied meanwhile:** none. R-265, R-572, R-90, the canonical spec, deep_zoom and the integrator contract stand as
  written; no requirement or task is added from the proposal.
- **Waits:** nothing built waits; no task builds the ladder. RQ-578 (deep zoom §2's floor) does not wait on this entry.

## RQ-578: Deep zoom §2's "Floor moves from ~23 to ~50+" does not hold for an f32 sum on the GPU: the ICs are absolute O(1) numbers, so the integration's precision sets the floor *(conflict: deep_zoom §2 and §4 (iv), scheduler_contract Part 3 and Part 4, dd_refinement_policy §0 against the prin-rs measurement the precision-ladder proposal reports; R-90, R-265, canonical_spec §1 item 3; REQ-DEC-036, REQ-TOOL-159, REQ-SCHED-062; context: TASK-M5-04, TASK-M6-07, TASK-M6-08, TASK-M6-02, TASK-M6-21, TASK-M6-01, TASK-M8-57)*

*Queued by the human, 27 Oct 2026: "One finding stands on its own, ladder or not: deep_zoom §2's "Floor moves from ~23
to ~50+" does not hold for an f32 sum on the GPU. The ICs are absolute O(1) numbers, so the integration precision sets
the floor (proposal §3.9 item 4; prin-rs measured x0 + J·δ all in f32 collapsing with plain f32). Queue it separately
in REVIEW_QUEUE.md, with file, section and quoted text. It is not RQ-554. It affects deep-zoom tasks now, so flag which
tasks depend on that floor." For the human: yes, it sets what depth the deep-zoom gates assert and what the linearised
decoder is for. It is not RQ-554, which is the band quad's terminal stop at `AT_F32_FLOOR` on a saturated axis; this
entry is where that floor sits at all on the f32 path.*

- **File, section (the claim):**
  - `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision": "**Why the precision survives.** The
    nonlinear precision lives in `x₀` (computed in f64, so its f32 cast is the best possible representation of the true
    centre IC). `J_D` is *rates of change* — `O(1)`, no precision lost in f32. Floor moves from ~23 to **~50+**."; and
    "**GPU kernel (f32):** `x = x₀ + J_D · δ`, where `δ` is the quad-local UV offset."
  - `docs/design/principia_deep_zoom.md` § "4. Sliding depth bound": "(iv) the **decode floor proper** (`AT_F32_FLOOR`) —
    only if the *linearised* decoder's samples collapse (~depth 50+), genuinely terminal, a deep-zoom backstop".
  - `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)", its table: "GPU
    (kernel) | f32 | quad-local sample positions, IC via `x₀ + J_D·δ`, integration".
  - `docs/contracts/principia_scheduler_contract.md` § "The absolute floor is emergent": "the **decode floor**
    (`AT_F32_FLOOR`, pushed to ~depth 50 by the linearised decoder)"; § "Part 4 — Terminal vs refinable, tied to the two
    floors": "Refinement **continues** on the linear path (depth ~23 → ~50)." and "Only when the *linearised* decoder's
    samples collapse to bitwise-identical ICs (~depth 50+) is f32 genuinely exhausted — the quad-local offsets `δ` are
    too small for f32 to represent distinct neighbours".
  - `docs/design/principia_dd_refinement_policy.md` § "0. What the mechanism is actually for", item 2: "The adaptive tree
    plus the linearised decoder (`principia_deep_zoom.md`) reaches depth ~50. **This is the feature; the tree exists for
    it.**"
  - `plan/requirements.yaml` REQ-DEC-036, verify: "the linear path distinguishes adjacent samples to depth ≥ 50".
- **File, section (the finding):** `docs/incoming/2026-10-26_precision_ladder/PROPOSAL_precision_ladder_and_taylor_2026-10-26.md`
  § "3.9 Deep zoom: the decoder switchover depends on the precision", item 4: "**The floor that matters is the
  integration's precision, not the decoder's.** The ICs are absolute numbers of order 1, and the integrator starts from
  them. If two pixels' ICs differ by less than `eps(P)` relative to O(1), they are the same IC to an integrator at
  precision `P`, whatever decoder produced them. prin-rs measured exactly this: "the literal formula `x₀ + J_D·δ` all in
  f32 collapses on exactly the same curve as plain f32", and the linearised split bought ~24 levels only by summing in
  f64 ("~24 levels over f32 and none over f64"). So deep zoom §2's "~50+" holds only if the sum and the integration run
  at f64. On the GPU at f32 the floor stays near 16–23 with either decoder. **That looks like a conflict in the corpus as
  it stands and should go to `REVIEW_QUEUE.md`** with this measurement, whether or not the ladder is adopted. (It is not
  RQ-554, which is a different issue.)"
- **The prin-rs measurement, as the proposal reports it** (§3.9; prin-rs is not authority, and the measurement is quoted
  from the proposal only): "the literal formula `x₀ + J_D·δ` all in f32 collapses on exactly the same curve as plain
  f32"; the linearised split "bought ~24 levels only by summing in f64 ("~24 levels over f32 and none over f64")";
  item 1: "prin-rs measured f32 collapsing at 16–23 and every f64 path at exactly rung 48"; its floor table: "f32 |
  ~16–23 (measured)", "f64 | ~50 (measured 48)"; and "At the chart origin there is no such floor (the standing prin-rs
  result: "the deep-zoom floor is a property of where you zoom")."
- **The conflict:** the corpus places the precision loss in `δ` ("the quad-local offsets `δ` are too small for f32 to
  represent distinct neighbours") and the floor at ~50+; but `x = x₀ + J_D·δ` is an absolute f32 number of order
  `|x₀|`, and the integrator, f32 on the GPU (canonical spec §1 item 3, R-265), starts from it. Adjacent ICs that differ
  by less than f32's ulp at `|x₀|` round to the same f32 IC whichever decoder produced them, so on the GPU at O(1)
  coordinates the linear path's samples collapse where the full decoder's do, ~16–23 by the measurement, and
  `AT_F32_FLOOR` fires a few levels past the switchover, not at ~50+. Deep zoom §1 already says "The deep-zoom floor is a
  property of where you zoom: a floor depth is quoted with the coordinate magnitude"; §2's and the scheduler contract's
  "~50+" quote none. The deep-zoom note's § "Reserved: the integration floor" covers ICs "*distinct* in physical
  coordinates" whose integration error exceeds their difference; this case is ICs not distinct at f32 at all.
- **Options seen:**
  - (a) The floor is restated: on the f32 GPU path the linearised decoder's floor is about the full decoder's (~16–23
    at O(1) coordinates, quoted with the coordinate magnitude); the linear path is kept for speed and for the
    f64-anchored decode (deep zoom § "Couplings to the contract", c); the ~50+ text above and REQ-DEC-036's and test
    11's "depth ≥ 50" are rewritten to the measured floor.
  - (b) The ~50+ floor is kept by running the sum and the integration past the switchover at a precision above f32:
    on the CPU at f64 for deep quads, or emulated on the GPU (the proposal's `df32`, which is RQ-577's ladder and needs
    R-265 and canonical spec §1 item 3 amended).
  - (c) Measured first: a gate measures the f32 GPU path's distinguishable depth, as integrated, at stated coordinate
    magnitudes (as the proposal's test 12's f32 arm, "it must collapse at 16–23"), and the docs and the gates take the
    measured value (a calibration, R-71).
  - (d) Other.
- **Applied meanwhile:** none; the docs and requirements stand as written. This PR adds the queue entry only; the
  requirements below do not yet carry `rq: RQ-578`, which is the port's next step.
- **Waits** (the arms that assert the ~50+ floor, or need distinct f32 ICs past ~23 at O(1) coordinates):
  - TASK-M5-04: `cargo xtask gate linear-decode-depth`'s arm "the linear path distinguishes adjacent samples to depth ≥
    50" (REQ-DEC-036's verify). Its other arms (the O(h²) agreement, no nonlinear decode on the GPU) and the rest of
    TASK-M5-04 do not wait.
  - TASK-M6-07: decoder test 11's "depth ≥ 50 distinguishability" (`cargo xtask gate linear-decode`, its Deliverables).
    Test 11's O(h²) slope (REQ-DEC-034, REQ-DEC-042) does not wait.
  - TASK-M6-08: `cargo test -p engine past_switch_routing` (REQ-TOOL-159), "quads at depths 21 and 30 with distinct
    adjacent ICs (no collapse)", a fixture that exists at O(1) coordinates only if the floor is past 30; and where
    REQ-SCHED-062's `AT_F32_FLOOR` (the linear path's collapse, "~depth 50+") fires. The switchover itself (REQ-DEC-033,
    REQ-DEC-037) does not wait.
- **Read the floor, do not wait** (their tests hold wherever the floor falls; what they show moves with it):
  - TASK-M6-02: REQ-SCHED-057's `infinite_zoom_no_absolute_cap`, "drive a scripted zoom to depth > 30 … until a floor
    fires": past 30 only where no floor fires first, so the fixture's coordinates decide whether it reaches 30.
  - TASK-M6-21: REQ-GUI-010 and REQ-GUI-012, the precision warning at `AT_F32_FLOOR` ("zoom a visible region through the
    switchover and to the linear floor"); under the measurement it fires a few levels after the switchover.
  - TASK-M6-01: REQ-REF-011 and REQ-SCHED-064, `AT_F32_FLOOR` as a terminal floor.
  - TASK-M8-57: REQ-TOOL-182's T10 arm, "adjacent samples give distinct ICs to the same depth as at the root", and
    REQ-TOOL-183's tolerance, the floor quoted with `|x₀|`.
  - The proposal's §3.10 (uniform mode, the layer table) and §3.9's per-precision switchover are RQ-577's, not this
    entry's.

## RQ-579: Redistribution terms for the high-precision IC databases *(found while pulling the reference data for the precision ladder's test 6, PR #299; `docs/incoming/2026-10-27_hp_ics/README.md` § "Terms: why the data is not committed"; context: the precision-ladder proposal, §5 test 6)*

*Found on 27 Oct 2026. Licensing third-party data is outside the repo, so it is the human's call. Nothing is
committed meanwhile.*

- **File, section:**
  - `docs/incoming/2026-10-27_hp_ics/README.md` § "Terms: why the data is not committed": "None of the sources states
    a licence or terms of use: the five pages of `db2.fmi.uni-sofia.bg` (its root, `3body123`, `3bodychor462`,
    `3bodychor164`, `3bodystab971`), Masson's page and site, the SJTU pages, and the `sjtu-liao/three-body` GitHub
    repository (no licence file; the API returns 404 for it). The three Hristov arXiv papers carry arXiv's
    non-exclusive distribution licence, which covers the papers, not the databases."
  - The precedent: `docs/incoming/2026-10-26_orbit_catalogues/README.md`: "**The extracted initial conditions are
    committed under the site's own terms.**"; `docs/incoming/2026-10-26_belgrade_gallery/README.md`: "**Terms.** The
    gallery's own "Use and Copyright" page (`archive/ref.php.html`): "You may copy and use any images or data from
    this web gallery. …"". Both folders committed third-party data only under stated terms.
  - `docs/incoming/2026-10-26_precision_ladder/PROPOSAL_precision_ladder_and_taylor_2026-10-26.md` § 5, test 6, which
    needs the values above f64.
- **The question:** the Hristov et al. databases (123 solutions at 150 digits, 462 and 164 + 259 choreographies at
  180, 971 stable i.c.s at 100), and the Simó and Li & Liao tables, state no terms. May their values be committed?
- **Options seen:** (a) commit the values with attribution (the citations in `docs/incoming/2026-10-27_hp_ics/README.md`);
  (b) ask the authors for permission first, and commit under whatever they grant; (c) keep fetch-and-verify only, as
  now.
- **Applied meanwhile:** (c). `fetch.sh` downloads all 599 files listed in `sources.tsv` and checks each against its
  recorded sha256, so the reference data is reproducible byte for byte without being committed. Waits: the precision
  proposal's test 6 reference data, in the repo.

## RQ-580: `hristov2024nc`'s title in the orbit catalogues' CITATIONS.md is not the journal's *(found while checking the citations for PR #299; `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md` § "References", `threebodyorbits_sources.json`; context: the precision-ladder proposal, §5 test 6)*

*Found on 27 Oct 2026. A citation's correctness is a routine decision, but CITATIONS.md is generated from the earlier
batch's transcription of the atlas's reference list, so the fix is that batch's, not PR #299's.*

- **File, section:**
  - `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md` § "References": "`hristov2024nc`: Hristov, Hristova,
    Puzynin, Puzynina, Sharipov & Tukhliev (2024), *Nontrivial choreographies of the planar three-body problem*,
    Physics of Particles and Nuclei 55, 495–497, https://doi.org/10.1134/S1063779624030444."
  - Its source, `docs/incoming/2026-10-26_orbit_catalogues/threebodyorbits_sources.json`, `hristov2024nc`: "title":
    "Nontrivial choreographies of the planar three-body problem".
  - The publisher's record of the DOI, deposited with Crossref (`https://api.crossref.org/works/10.1134/S1063779624030444`,
    publisher "Pleiades Publishing Ltd", fetched 27 Oct 2026): title "Searching for New Nontrivial Choreographies for
    the Planar Three-Body Problem", Physics of Particles and Nuclei 55, issue 3, pages 495–497, June 2024. The
    authors' page `https://db2.fmi.uni-sofia.bg/3bodychor164/` gives the same title. The publisher's article page
    (link.springer.com) returned a bot-check page and could not be read.
  - `docs/incoming/2026-10-27_hp_ics/README.md` § "Sources", the notes, records this.
- **The question:** CITATIONS.md carries the atlas's wording of the title, not the journal's. Should the generator
  correct it (and the transcribed JSON stay as the site published it), or should CITATIONS.md note the site's wording
  beside the journal's, as it already does for Li, Jing & Liao 2018?
- **Options seen:** (a) correct the title in `citations.py`'s output and leave `threebodyorbits_sources.json` as
  transcribed; (b) add a note under CITATIONS.md's "Notes", as for `lijingliao2018`, giving both titles; (c) other.
- **Applied meanwhile:** none in CITATIONS.md. `docs/incoming/2026-10-27_hp_ics/README.md` cites the Crossref title.
  Nothing waits.
