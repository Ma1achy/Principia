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
