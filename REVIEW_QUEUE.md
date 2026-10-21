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

## RQ-445: Listen's section in the Trajectory panel: collapsed until first used, then remembered per user, or always collapsed *(the human's queued question, R-541 Q6, G4; REQ-GUI-326; context: TASK-M8-61)*

*Queued by the human (`principia_trace_listen_gui.md` G11, Q6; this bundle's Q-numbers, distinct from the trace
follow-up's F7 Q5 and Q6, RQ-421 and RQ-422). For the human: no for (a).*

- **File, section:** `decisions.md` § "R-541 — …", G11: "**Q6. Listen's default collapsed state.** Recommendation (a):
  collapsed until first used, then remembered per user. (b) Always collapsed."; G4's table: "**Listen** (collapsed until
  first used, then remembered)".
- **Options seen:** (a) collapsed until first used, then remembered per user (the file's recommendation); (b) always
  collapsed.
- **Applied meanwhile:** (a) (R-541 P11): remembered in the GUI's own settings, as a window's place is (R-461), never the
  contract. Nothing waits.
- **See also:** since PR #267's split (applied per R-369) the Listen section is REQ-GUI-334's, TASK-M8-64's.

## RQ-446: The expanded strip's spectrum takes the Legend's column, or the Legend stays and the spectrum stacks under the spectrogram (the band ≈ 520 px) *(the human's queued question, R-541 Q7, G7; REQ-GUI-330; context: TASK-M8-63)*

*Queued by the human (`principia_trace_listen_gui.md` G11, Q7). For the human: no for (a).*

- **File, section:** `decisions.md` § "R-541 — …", G11: "**Q7. The expanded strip's spectrum in the Legend's column.**
  Recommendation (a): borrow the column, as G7. (b) Keep the Legend and stack the spectrum under the spectrogram, making
  the band ≈520 px."; G7: "| **Strip expanded (⏶)** | … | **the spectrum at the head**, until `⏷` |".
- **Options seen:** (a) borrow the Legend's column (the file's recommendation); (b) keep the Legend, the band ≈ 520 px.
- **Applied meanwhile:** (a) (R-541 P11). Nothing waits.

## RQ-447: Kept-trace colours: a fixed cycle of six, distinct from the body colours and the stain classes, to be chosen against the stain palettes, where R-538 TM2 has already fixed a six-colour trace palette *(the human's queued question, R-541 Q8, G9; conflict with R-538 TM2; REQ-GUI-271, REQ-GUI-333; context: TASK-M8-58, TASK-M8-61)*

*Queued by the human (`principia_trace_listen_gui.md` G11, Q8). Found by the port: the recommendation reads as if no
cycle were chosen. For the human: yes, if the cycle is to change.*

- **File, section:**
  - `decisions.md` § "R-541 — …", G11: "**Q8. Kept-trace colours.** Recommendation (a): a fixed cycle of six, distinct
    from the body colours and the stain classes; to be chosen against the stain palettes."; G9: "Kept traces take
    distinct colours: kept #2 is `#5ab4ff` in the boards."
  - `decisions.md` § "R-538 — …", TM2's Colour row: "fixed `#FFF27A` for the hover or pinned trace. Kept traces take
    the trace palette in turn (`#7FE3FF #FF9EC4 #B7F28A #FFB86B #C9A7FF #9EF0D2`)"; `principia_lines.md` § "8. Traces",
    Colour, and REQ-GUI-271 carry it.
  - `#5ab4ff` is also Listen's left channel and the shape sphere's window highlight (R-540 S11, R-541 G4).
- **The conflict:** R-538's six are a fixed cycle already, not checked against the stain palettes; the boards' kept #2
  is not one of them and is a Listen colour.
- **Options seen:** (a) a fixed cycle of six, distinct from the body colours and the stain classes, chosen against the
  stain palettes (the file's recommendation), replacing R-538's six if they fail that check; (b) R-538's six stand; the
  boards' `#5ab4ff` illustrative.
- **Applied meanwhile:** R-538's six (the ruling), the boards' `#5ab4ff` illustrative (R-541 P10). Nothing waits.

## RQ-448: R-541 G2 withdraws `Lines ▾` ("An earlier art-board pass had both; they are withdrawn"), where `Lines ▾` is R-476's menu for the whole line system: contours, axes, grids, presets and class edges as well as the traces *(conflict: the human's words against a ruling, R-541 G2, R-476 L2, R-509 Y5, R-538 I2; REQ-GUI-247, REQ-GUI-271, REQ-GUI-324; context: TASK-M8-50, TASK-M8-58, TASK-M8-61)*

*Found by the port. Nothing is chosen. For the human: yes, it decides whether the line system keeps its menu.*

- **File, section:**
  - The human's message (R-540): "The GUI file supersedes the earlier art-board pass's `Lines ▾` and `Listen ▾`
    menus".
  - `decisions.md` § "R-541 — …", G2: "**No `Lines ▾` and no `Listen ▾`.** An earlier art-board pass had both; they are
    withdrawn." and "A trace is an overlay, so it lives with the overlays."; G10's One place test: "a stray control
    (e.g. a surviving `Lines ▾`)".
  - `decisions.md` § "R-476 — …", L2, and `principia_lines.md` § "2. Where it sits": "One menu: **Lines ▾** beside
    Overlays ▾, with a count, `Alt+L` to show or hide all."; `principia_render_gui_spec.md` § "G2. …": "**Lines ▾
    (R-476).** … The menu lists the contour sets, traces and axes with their visibility, opens the contour-set editor
    and the style library, and holds the presets, among them **Grid** …, **t_end** … and **Class edges** (… R-509),
    which were Overlays toggles."; REQ-GUI-247 (TASK-M8-50).
  - `decisions.md` § "R-538 — …", I2: "They are the trace's entry in Lines ▾ (render_gui_spec §G2, R-476), in TM2's
    order".
  - `Listen ▾` is in no corpus file: the port looked in `docs/` (less `docs/archive/` and `docs/reference/`),
    `decisions.md`, `REVIEW_QUEUE.md` and `plan/`; it is only in the demos' mock menu bars
    (`docs/gui/reference/trace/trace_markers.html` version 28, `trace_sound.html` version 9) and the earlier art-board
    canvases under `workbench/`, which are not normative.
- **The conflict:** the human's words and G2 withdraw a menu "an earlier art-board pass had"; that pass's `Lines ▾` was
  adopted by R-476 as the line system's one menu, and G2 rehomes only the trace. Withdrawing it whole leaves contours,
  axes, grids, the presets and class edges (R-509) without a menu.
- **Options seen:**
  - (a) The trace leaves `Lines ▾` for `Overlays ▾ › Trace`, the Trajectory panel and `Trace…` (G2, G4, G5); `Lines ▾`
    stays for the line system's other lines; G10's One place test reads "a surviving `Lines ▾`" as a `Lines ▾` that holds
    a trace control.
  - (b) `Lines ▾` withdrawn whole: contours, axes, grids, presets and class edges move to `Overlays ▾` (a Lines group),
    their editors staying windows, `Alt+L` kept.
  - (c) `Lines ▾` withdrawn and the line system's controls placed by a later design pass.
- **Applied meanwhile:** (a) (R-541 P2): the trace's place in `Lines ▾` is stale, replaced by R-541; the rest of
  R-476's menu stands. Nothing waits; a ruling for (b) or (c) is a follow-up change to REQ-GUI-247 and TASK-M8-50.
- **See also:** PR #263's gui review G3 gave REQ-GUI-271 a keyboard reach and order arm on the trace's `Lines ▾` entry;
  merging main into rulings/R-540 moved it, with its presence screenshot, to REQ-GUI-328 (`Trace…`, TASK-M8-62), read
  on R-541 G5's table order (applied per R-369).

## RQ-449: R-541 G2's `K` "keep the current trajectory" is the Figure's lock key (R-409, §G4), where Space keeps the orbit; and G2 does not say in which scope `T`, `⇧T`, `Y`, `K` and `L` act *(conflict and silence, R-541 G2, R-409, R-460; REQ-GUI-097, REQ-GUI-325; context: TASK-M8-61)*

*Found by the port. Nothing is chosen. For the human: yes, for `K`.*

- **File, section:**
  - `decisions.md` § "R-541 — …", G2: "**Keys**: `T` trace, `⇧T` kept traces, `Y` syzygies, `K` keep the current
    trajectory, `L` sound on/off. Add them to the keyboard sheet."; the art board `Main.dc.html`'s top bar: "♪ L listen ·
    T trace · K keep".
  - `principia_render_gui_spec.md` § "G3. …": "Figure — arrows pan; + / − zoom, Space keeps the orbit, L listens, K locks
    (§G4), P pins the sample under the pointer in the Trajectory panel (R-453)."; § "G4. …": "Locking (K, a shortcut with
    the focus in the Figure, in either keyboard mode (§G3, R-409); …"; REQ-GUI-097.
- **The conflict:** `K` cannot both lock and keep. `L` agrees (the Figure's L listens). `T`, `⇧T` and `Y` are bound
  nowhere today; §G3 has no global letter keys but `Alt+L` and `?`.
- **Options seen:** (a) `K` stays lock and Space stays keep, G2's "K keep" read as Space; `T`, `⇧T`, `Y` and `L` are the
  Figure's scope shortcuts, as L is (R-409); (b) `K` becomes keep, lock moving to another key; (c) the five global, in
  either mode wherever the focus is, as `Alt+L`.
- **Applied meanwhile:** (a) (R-541 P3). Nothing waits.

## RQ-459: The instrument menu's families and their counts (pure 5, keys 3, struck 10, bowed and blown 7, voices 4, synths 8, textures 2) name no instrument, and the code has none *(silence: a definition, R-540 S8, R-541 G6, R-72; REQ-GUI-329; context: TASK-M8-62)*

*Found by the port. Nothing is chosen. For the human: no; the task that builds the menu can write it (R-72), unless the
human wants to.*

- **File, section:** `decisions.md` § "R-540 — …", S8: "Families, for the menu (GUI file G6): pure (5), keys (3), struck
  (10), bowed and blown (7), voices (4), synths (8), textures (2)."; § "R-541 — …", G6: "**The instrument menu is
  grouped**: pure (5), keys (3), struck (10), …, one instrument per row in the build."; the demo's menu groups 19
  instruments ungrouped, 15 under "More" and 5 under "Synths"; S8 lists twelve `strike` instruments, against "struck
  (10)", and five named synths, against "synths (8)".
- **Options seen:** (a) TASK-M8-62 writes the assignment (R-72), meeting the counts (the electric piano among keys, the
  chiptune, raw saw and sub drone among synths, say), reviewed by gui; (b) the human gives it.
- **Applied meanwhile:** (a) (R-541 P8). Nothing waits.
- **See also:** since PR #267's split (applied per R-369) the assignment is TASK-M8-65's (`Listen…`).

## RQ-460: R-541 G1's "Every setting appears in at most two places", where G2, G4 and G5 put the syzygies, the end event icon, the off-screen pins and the wrapped copies in three: `Overlays ▾ › Trace`, the Trajectory panel's marker chips and `Trace…` *(conflict: the applied text against itself, R-541 G1, G2, G4, G5; REQ-GUI-324; context: TASK-M8-61, TASK-M8-62)*

*Found by the port. Nothing is chosen. For the human: no for (a), which keeps every control the file names.*

- **File, section:** `decisions.md` § "R-541 — …", G1: "**Every setting appears in at most two places**: an everyday
  control in the panel, and the full set in a window (`Trace…`, `Listen…`)"; G2: "`Overlays ▾` gains a Trace group … :
  trace under cursor (`T`); kept traces (`⇧T`); syzygies (`Y`); end event icon; off-screen icons, pinned to the edge;
  repeat in wrapped copies; outline the chart's square; a `Trace…` button."; G4: "The marker chips are toggle buttons
  that show the icon they draw: end event, syzygies, leaving the chart, off-screen pins, wrapped copies, head marker.";
  G5's Markers and At the edges rows.
- **Options seen:** (a) all three stay, one state behind them: G1 is read as settings, `Overlays ▾ › Trace` as the view's
  toggles of the same fields, a third route to the same `SetField`; (b) the panel's chips lose the items `Overlays ▾`
  has; (c) `Overlays ▾ › Trace` keeps only the trace, kept traces and `Trace…`.
- **Applied meanwhile:** (a) (R-541 P4). Nothing waits.
- **See also:** RQ-466, the Inspector's listen, a third place for sound (gui review 5595359543 of PR #267, G1).

## RQ-461: R-541 G2 and G5 turn "outline the chart's square" on by default in `Overlays ▾ › Trace` and `Trace…`, where RQ-408 (R-537 Q3) is open, nothing built meanwhile, its recommendation an Overlays › Chart toggle, off by default *(conflict: the applied text against an open entry, R-541 G2, G5, R-537 Q3; REQ-GUI-324, REQ-GUI-328; context: TASK-M8-61, TASK-M8-62, TASK-M8-55)*

*Found by the port. Nothing is chosen. For the human: yes, with RQ-408.*

- **File, section:** `decisions.md` § "R-541 — …", G2: "outline the chart's square"; G5's At the edges row:
  "**outline the chart's square**" (bold the default); the art board `Main.dc.html`'s Trace group shows it ticked;
  RQ-408: "(a) an Overlays › Chart toggle, off by default and on in the debug view (the file's recommendation); (b)
  always drawn while the view shows past the square; (c) none." and "**Applied meanwhile:** nothing is built: no outline
  is drawn and Overlays › Chart gains no toggle until this is ruled".
- **Options seen:** (a) RQ-408 governs: the outline's entries in the Trace group and `Trace…` are built only once RQ-408
  is ruled, as it rules; (b) R-541 answers RQ-408: an outline toggle, on by default, in the Trace group and `Trace…`.
- **Applied meanwhile:** (a) (R-541 P5). Only the outline's two entries wait.

## RQ-462: R-541 G4 lists the Trajectory panel from the top and leaves out the summary's FTLE and `E₀`, the panel's own playhead and the Open full viewer… and IC Inspector… buttons, which §G2 and REQ-GUI-088 have *(silence, R-541 G4, R-65, R-453; REQ-GUI-088, REQ-GUI-326; context: TASK-M8-11, TASK-M8-61)*

*Found by the port. Nothing is chosen. For the human: yes, it decides what the panel shows.*

- **File, section:** `decisions.md` § "R-541 — …", G4: "1. **Header** … 2. **Status line**: running / pinned, the end
  event with its time, `|ΔE/E|`. 3. **Previews** … 4. **Three collapsible sections**"; "The kept list moves here and into
  `Trace…`, and out of the left panel."; G1: "the bottom band's Time column shows the trajectory over time"; the art
  board `TrajectoryPanel.dc.html`'s state A ends with "Open full viewer…" and "IC Inspector…".
  `principia_render_gui_spec.md` § "G2. …", the Trajectory panel: "a one-line summary (fate, `t_end`, FTLE, `E₀`)", "a
  playhead for this orbit", "**Open full viewer…** and **IC Inspector…** (both open §G8)", "**Kept orbits** below, each
  with its fate and time, removable"; REQ-GUI-088.
- **Options seen:** (a) a port adds: the status line carries the summary (fate and `t_end`, FTLE, `E₀`) with G4's items;
  the panel's playhead and the two buttons stay, after Numbers, as art board A has them; the kept list moves as G4 says;
  (b) G4 is the whole panel: FTLE, `E₀` and the panel's playhead go, the buttons stay; (c) G4 is the whole panel, the
  buttons going too.
- **Applied meanwhile:** (a) (R-541 P6). Nothing waits.

## RQ-463: R-541 G7 fixes the bottom band's sizes (196 px, 420 px expanded, the Compass at the left panel's width, the Legend 360 px), where every panel boundary is a remembered splitter (R-462); and "Compass: cube only" does not say what becomes of the compass's buttons, which are keyboard siblings *(conflict and silence, R-541 G7, R-462, R-409, R-452, R-459; REQ-GUI-213, REQ-GUI-330; context: TASK-M8-63)*

*Found by the port. Nothing is chosen. For the human: no for (a), which reads the sizes as R-462 reads the artboards'.*

- **File, section:**
  - `decisions.md` § "R-541 — …", G7: "**Compass · Time · Legend**, 196 px, with the Compass at the Explore left panel's
    width and the Legend at 360 px"; "| **Strip expanded (⏶)** | **grows upward to 420 px**; the view gives up the
    height | … | **cube only** | …"; G10's Band height test: "expanding the strip changes it by exactly the band's
    growth".
  - `principia_render_gui_spec.md` § "G2. …", "Layout: every panel boundary is a splitter (R-462)": "Each panel has a
    minimum size; a double-click on a splitter resets that splitter to its default; the layout is remembered across
    sessions"; § "G13. …": "The artboards' panel sizes and window positions are the defaults".
  - § "G3. …": "The Compass scope holds the compass, then its Tilt and Slice buttons, then its view buttons Cube,
    Plücker, Jacobi and Shape, then the cube's axis buttons H, V and D and, while it is shown, the "4D: see Plücker"
    badge".
- **Options seen:** (a) the sizes are defaults (R-462): the band's splitter keeps its height, 196 px by default; the
  expanded strip is the band's second height, 420 px by default, its own splitter position, remembered; while "cube
  only" the compass draws its cube in its current view and mode, its buttons hidden and out of the keyboard tree until
  `⏷`, a drag on it working as before; (b) fixed sizes, the band's splitter disabled while the strip shows; (c) the
  compass keeps its buttons, the cube shrunk.
- **Applied meanwhile:** (a) (R-541 P9). Nothing waits.

## RQ-465: R-541 G5's footer and R-540 S4 restate R-538 TF3 for every trace, "the whole trace to the horizon is drawn" when nothing plays, where PR #263 (merged to main after this port began) applies RQ-427 (a): only a trace of the default length is drawn whole while paused, a `Window` or `LastDt` length kept as chosen *(conflict: the human's words against an applied option, R-541 G5, R-540 S4, R-538 TF3, RQ-427; REQ-GUI-281, REQ-GUI-314, REQ-GUI-328; context: TASK-M8-58, TASK-M8-12, TASK-M8-62)*

*Found merging main (7daf689b) into rulings/R-540. Nothing is chosen. For the human: yes if G5 was meant to answer
RQ-427; RQ-427 itself said the orchestrator may rule it.*

- **File, section:**
  - `decisions.md` § "R-541 — …", G5 (`principia_trace_listen_gui.md` §G5): "Footer: `reset`, `save as default`. When
    nothing is playing or scrubbing, the whole trace to the horizon is drawn (follow-up F3)."
  - `decisions.md` § "R-540 — …", S4 (`principia_listen.md` S4): "| Follow the head | when nothing is playing or
    scrubbing | **the whole run as one chord**, matching the trace being drawn whole (follow-up F3) |".
  - `docs/design/principia_lines.md` § "8. Traces", Animation, as PR #263 left it: "every trace of the default length
    (to the head) is drawn from `t = 0` to its end (its event, or the horizon), with no head marker, and a `Window` or
    `LastDt` length is kept as chosen (RQ-427 (a), applied meanwhile)"; `principia_render_gui_spec.md` § "G2. …", the
    same; REQ-GUI-281's arm "paused, a Window(t0, t1) trace still draws exactly [t0, t1] and a LastDt trace its last Δt
    (RQ-427 (a)); control: either drawn whole while paused fails" (gui review 5594407176 of PR #263, G2).
  - RQ-427: "(a) TF3 is the default length's (`ToPlayhead`): a fixed window or the last `Δt` is kept as chosen, … (b)
    TF3 for every length: paused, every trace is drawn whole".
- **The conflict:** the bundle's two files, written after TF3, say the whole trace is drawn whenever nothing plays, with
  no word on a chosen length; main now draws a `Window` or `LastDt` trace as chosen while paused, and tests that it
  does. If the files mean every trace, they are RQ-427's option (b) and REQ-GUI-281's new arm fails correct code; if
  they mean the default trace, they agree with (a). S4's chord is the whole run either way, so under (a) a paused
  `Window` trace and Listen's chord no longer "match".
- **Options seen:** (a) RQ-427 (a) stands: G5's footer and S4's "matching" read as the default length's; a paused
  `Window` or `LastDt` trace is drawn as chosen while Listen plays the whole run; (b) the files answer RQ-427 as (b):
  paused, every trace is drawn whole, REQ-GUI-281's `Window` / `LastDt` arm reversed; (c) as (a) for the trace, and
  Listen's paused chord follows the trace's drawn span.
- **Applied meanwhile:** (a): main's text and tests stand; `principia_render_gui_spec.md`'s `Trace…` footer and
  `principia_listen.md` S4 carry "of the default length (RQ-427 (a), RQ-465)". Nothing waits.

## RQ-466: render_gui_spec §G8 puts listen in the Inspector ("**listen** is here too") and REQ-GUI-090 says "listen (in the Trajectory panel and the Inspector)", where R-541 G1 puts everything about one trajectory, its sound included, in the Trajectory panel, and REQ-GUI-324's One place test fails a sound control anywhere else *(conflict: the applied text against the GUI spec and a requirement, R-541 G1, R-65, R-109; REQ-GUI-090, REQ-GUI-324; context: TASK-M8-12, TASK-M8-61)*

*Found by gui review 5595359543 of PR #267, G1. Nothing is chosen. For the human: no; the orchestrator may decide it
as a routine design choice, unless the human wants it.*

- **File, section:**
  - `principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)":
    "Integration is `computeIC` on the CPU in f64 (trajectory_viewing §1); the engine is reached only through it (§6).
    **listen** is here too."
  - REQ-GUI-090's statement: "listen (in the Trajectory panel and the Inspector) must sonify the corpus's mapping".
  - `decisions.md` § "R-541 — …", G1 (`principia_trace_listen_gui.md` §G1): "**Everything about one trajectory lives in
    one place: the Trajectory panel.** That covers its previews, its trace and its sound." and "**Every setting appears
    in at most two places**: an everyday control in the panel, and the full set in a window (`Trace…`, `Listen…`)".
  - REQ-GUI-324's verify (R-541 G10 One place): "walking the widget tree finds no trace or sound control outside the
    Trajectory panel, Overlays ▾ › Trace, Trace…, Listen… and the Time column".
- **The conflict:** the Inspector's listen either fails the One place test or is dropped against REQ-GUI-090.
- **Options seen:** (a) R-541 G1 holds: listen in the Trajectory panel only; the Inspector holds no sound control, and
  REQ-GUI-090's "and the Inspector" and §G8's sentence are stale; (b) the Inspector keeps listen as a view of the same
  Listen state, and the One place test's list adds the Inspector window (a third place for sound, see RQ-460).
- **Applied meanwhile:** (a) (applied per R-369, PR #267's fix round): §G8 and REQ-GUI-090's note carry it, and
  REQ-GUI-324's control fails a listen control in the Inspector. A ruling for (b) is a follow-up change to REQ-GUI-324,
  TASK-M8-61 and §G8.

## RQ-478: R-526 rules a field input "the stain's normalisation", and the design doc §4.4 and §12 q3 take that to be "over the visible range", rescaling while zooming; but the stain normalises by `range_norm`, a field's declared range fixed (`t_end` → lin [0, T]) and only an unbounded field measured over the draw; and which range applies when the stain does not colour by that field *(conflict: the human's words and the bundle against docs on origin/main, render_gui_spec §10.1 and render contract Part 6, R-526, R-486, R-536; context: the plan PR's field-inputs task)*

*Found by physics review 5596610796 of PR #268 (P2). For the human: yes, it decides whether a field-driven medium
rescales while zooming; physics, since it is the field's normalisation.*

- **File, section:**
  - `decisions.md` § "R-526 — …", the human's words: "RQ-390 the stain's normalisation."
  - `docs/design/principia_print_graph.md` § "4.4 Payload fields as inputs": "The field is the same value the stain
    reads, normalised over the visible range with the stain's own window rule." § "12. Open questions …", question 3:
    "A field normalised over the visible range changes its normalisation while zooming, so a "Relief of lifetime"
    relief rescales as you zoom."
  - `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library", `range_norm`: "**`auto = false`** uses
    the **fixed `[lo,hi]`** — a per-field domain **declared in the manifest** … (a `u16` step index
    `[0, horizon_steps]`; …). **`auto = true`** ignores the declared range and uses the **measured min/max** of the
    field over the current draw".
  - `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library", the debug fields' `colour()`:
    "`RANGE_AUTO` is the fixed↔auto flag, editable identically in the node inspector, on the node in the graph, and in
    the code": a setting of the stain's node that colours by the field.
  - `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)": "`t_end` → lin
    [0, T]".
- **The conflict:** under the stain's own rule `t_end` has a declared range and does not rescale while zooming, so
  Relief of lifetime would not rescale, against §4.4's "visible range" and §12 q3's premise; the ensemble spread, with
  no declared bound, would. And the stain's `RANGE_AUTO` belongs to the field the stain colours by: when the stain does
  not bind the Print graph's field, there is no stain window to share.
- **Options seen:**
  - (a) Each field's own `range_norm` rule: fixed over its ledger-declared range where it has one, measured over the
    draw where it does not; a field the stain does not bind takes its ledger default.
  - (b) The visible range for every field input (measured over the draw), as §4.4 words it, whatever the stain does.
  - (c) The stain's window when the stain binds the same field, (a) otherwise.
- **Applied meanwhile:** the human's words literally: each field's own `range_norm` rule, fixed where the ledger
  declares a range and measured over the draw where it does not (option (a)). R-526's text no longer decides "visible
  range". What waits: the plan PR's field-inputs task writes the normalisation from the ruling.

## RQ-479: R-488 counts the Print graph's post stage in the tier controller's accounting, and the Auto arbiter's rungs set N, E, depth and FTLE gating, so a heavy Print graph changes numbers the instrument reports, against R-489 and the design doc §3 *(conflict: the human's P4 against the human's P5 and `principia_quality_device_note.md` §3 on origin/main, R-488, R-489; context: the plan PR's post-stage budget task)*

*Found by physics review 5596610796 of PR #268 (P3). For the human: yes, it decides whether a look can change a
measurement.*

- **File, section:**
  - `decisions.md` § "R-488 — …", what it decides (P4): "The post stage is a stage of the telemetry frame record
    (`stage_ms.post`), the tier controller's accounting includes it". `docs/design/principia_print_graph.md` § "8.2 The
    budget": "It is not exempt from the tier controller's accounting; the tier simply has a post-stage share."
  - `docs/design/principia_quality_device_note.md` § "3. The quality ladder": "each a complete preset (N, E, depth,
    render_scale, FTLE/word gating, live-set size) … **The policy's whole output is one integer: the rung.**"
  - `decisions.md` § "R-489 — …", the human's P5: "measurement never reads the print". `principia_print_graph.md` § "3.
    What never changes because of it": "A Print graph cannot change any number the instrument reports."
- **The conflict:** under Auto, a heavy Print graph's cost lowers the resting rung, which changes E (and so the
  ensemble spread's value and presence), N and depth (the Inspector's quad statistics and the area statistics) and can
  gate FTLE out: the print feeds back into measurement through the cost loop.
- **Options seen (the physics review's):**
  - (a) The post stage's cost is excluded from the arbiter's rung solve; only §8.1's own degrade (reduced, then Plain)
    answers it.
  - (b) The post stage counts only toward render-key rungs (render_scale), never E, N, depth or FTLE.
  - (c) As written, with §3's sentence amended to allow it.
- **Applied meanwhile:** none; both texts stand as written. What waits: the plan PR's post-stage budget task and the
  arbiter's accounting of `post`.

## RQ-480: A Chart position target other than the view's own chart: allowed, with the target's inverse generated into the fragment pipeline, or the view's own chart only? *(question, the state map's Q1, R-542, R-116; context: TASK-M7-47)*

*Filed by R-542's port from `principia_state_map.md` SM13 (the file's "Queue, don't rule"). Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q1. A target other than the
  view's chart.** Recommendation (a): allowed, with the target's inverse generated into the fragment pipeline beside the
  view's. (b) The view's own chart only." SM2: "A target other than the view's chart needs that chart's inverse in the
  fragment pipeline as well (Q1)."
- **Options seen:**
  - (a) Allowed, with the target's inverse generated into the fragment pipeline beside the view's (R-116's one Rust
    source).
  - (b) The view's own chart only.
- **Applied meanwhile:** (a), as the prototype has it (its target menu lists every chart). Reading any chart at all from a
  colour occupant is RQ-489, which TASK-M7-47 waits on.

## RQ-481: Tilted targets: the branch nearest the slice, as prototyped, or not offered as a target? *(question, the state map's Q2, R-542, R-473; context: TASK-M7-47)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q2. Tilted targets.**
  Recommendation (a): the branch nearest the slice, as prototyped. (b) Not offered as a target." § "SM4. Destination
  side": "a tilted axis (E, F) | not periodic and multi-valued in θ | take the branch nearest the slice (smallest
  out-of-plane `w`); this is a choice, Q2".
- **Options seen:**
  - (a) The branch nearest the slice (the prototype's `smWhere`: `k ∈ [−4, 4]`, the smallest `|w|`).
  - (b) Not offered as a target.
- **Applied meanwhile:** (a).

## RQ-482: The state map and the refinement criterion: never a criterion input, or a spread over it as a candidate signal, measured first? *(question, the state map's Q3, R-542; scheduler contract Part 1; context: TASK-M8-66)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q3. The state map and the
  criterion.** Recommendation (a): never a criterion input (the firewall). (b) A spread over it as a candidate signal,
  measured first." SM3: "It is a colouring: the scheduler's criterion does not read it (the firewall, scheduler contract
  Part 1)."
- **Options seen:**
  - (a) Never a criterion input (the firewall).
  - (b) A spread over it as a candidate signal, measured first.
- **Applied meanwhile:** (a); SM12's Firewall test holds it (TASK-M8-66).

## RQ-483: The vMF shape colouring is not one-to-one: keep it as the hue carrier it is and say so in colour composition §4.1, or also offer SM6's sphere map as a shape colouring? *(question, the state map's Q4, R-542; dd_colouring §3.2; context: none, recorded only)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled, and nothing in the vMF colouring changes
(the human's message: "Q4 concerns the existing vMF shape colouring: record it, change nothing in it").*

- **File, section:**
  - `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q4. The vMF shape colouring.** It is not
    one-to-one, by SM6's theorem. Recommendation (a): keep it as the hue carrier it is, and state that in colour
    composition §4.1. (b) Offer SM6's sphere map as an alternative shape colouring."
  - SM6: "the vMF blend at fixed L puts **antipodal shapes** (chord 2.000) in one bin at κ = 1, 3 and 6 — the pair
    `±(1,1,1)/√3`, both neutral grey".
  - `docs/design/principia_dd_colouring.md` § "3.2 The vMF engine": "fully parameterised by `(L, C, κ)`", and now its
    paragraph "Not one-to-one (R-542; RQ-483)", which records it.
- **Options seen:**
  - (a) Keep it as the hue carrier it is, and state that in colour composition §4.1.
  - (b) Offer SM6's sphere map as an alternative shape colouring.
- **Applied meanwhile:** recorded only, in dd_colouring §3.2. Colour composition §4.1 is not changed until this is ruled;
  the vMF colouring is unchanged.

## RQ-484: Ensemble colour: the nominal copy, or the OKLab mean over the copies, labelled as not one-to-one? *(question, the state map's Q5, R-542, R-102; context: TASK-M7-49)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q5. Ensemble colour.**
  Recommendation (a): the nominal copy. (b) The OKLab mean over copies, labelled as not one-to-one." SM10: "An OKLab
  mean over the `E+1` copies would turn undetermined regions grey, which is informative but not one-to-one (Q5)."
- **Options seen:**
  - (a) The nominal copy (copy 0).
  - (b) The OKLab mean over the copies, labelled as not one-to-one.
- **Applied meanwhile:** (a).

## RQ-485: After its run ends: hold the end state, or paint the outcome palette? *(question, the state map's Q6, R-542, R-95, R-103; context: TASK-M7-47)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q6. After its end.**
  Recommendation (a): hold the end state. (b) The outcome palette." SM5a: "**Ended runs hold their end state
  (absorbing).** … Option: *after its end*, paint the outcome palette instead."
- **Options seen:**
  - (a) Hold the end state (the default), the outcome palette an option.
  - (b) The outcome palette.
- **Applied meanwhile:** (a), as the prototype has it (its "after its end" control defaults to hold; "outcome colour"
  paints the outcome palette at 0.78 of its value).

## RQ-486: History fields (winding on any chart, time in a tile, first passage): out of scope, or specified as accumulators, measured for cost first? *(question, the state map's Q7, R-542; context: none)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q7. History fields**
  (winding on any chart, time in a tile, first passage). Recommendation (a): out of scope; each needs its own
  accumulator, which is a struct change. (b) Specify the accumulators, measured for cost first." SM11: "**Fields that
  need history stay out of scope without a struct change** (Q7)."
- **Options seen:**
  - (a) Out of scope; each needs an accumulator, which is a struct change (SM2).
  - (b) Specify the accumulators, measured for cost first.
- **Applied meanwhile:** (a). No task builds a history field; SM2's constraint stands either way until a ruling changes
  it.

## RQ-487: The tiling defaults and the state map's constants are calibration values: adopt the prototype's as the starting values? *(question, the state map's Q8, R-542, R-71; REQ-COL calibration; context: TASK-M7-48)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q8. Tiling defaults**
  (SM11): pattern, tiles across (6) and the index palette chroma (0.08) are calibration values (R-71). Recommendation
  (a): adopt the prototype's as the starting values. Chart units as the default is settled by §0, not queued."
- **Added by the port (applied per R-369):** the compaction scales `κ` of SM6 (latent 3, momenta 0.3, the tilt's `s` π,
  dilation 0.6) are values the file gives no source for, so they join the same calibration requirement as starting
  values (R-71), as do the layer line width 0.045, the dot radius 0.13 and the plane gradient's `L = 0.75` and chroma
  0.095.
- **Options seen:**
  - (a) Adopt the prototype's values as the starting values, confirmed at the M7 gate.
  - (b) Other values, given at the gate.
- **Applied meanwhile:** (a), the prototype's values, provisional under the calibration requirement.

## RQ-488: The plane's 2D gradient is grey at the chart origin: accept it, or offset the map so its achromatic point falls outside the home square? *(question, the state map's Q9, R-542; context: TASK-M7-48)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q9. The plane's grey
  origin.** The linear map is achromatic at the chart origin. Recommendation (a): accept it — on the latent charts the
  origin is the Lagrange landmark, a named place, and it is far in L from the undetermined and no-value colours. (b)
  Offset the map so the achromatic point falls outside the home square."
- **Options seen:**
  - (a) Accept it.
  - (b) Offset the map so the achromatic point falls outside the home square.
- **Applied meanwhile:** (a).

## RQ-489: Chart position reads the target chart and its inverse, and SM11's "a fixed t" reads a state other than the playhead's, where seam 6 and render contract Part 4 keep every chart identifier from a colour occupant, a source included, and SM2 allows only data the fragment already has *(conflict: the human's design against a contract and a seam, and a need for state SimState does not hold; R-542, R-116, REQ-COL-005, REQ-RENDER-015; context: TASK-M7-47)*

*Found by R-542's port. Nothing is ruled. SM2 is the human's hard constraint ("If anything turns out to need a stored
field, stop and queue it; do not add one"): no field is added.*

- **File, section:**
  - `docs/design/principia_dd_colouring.md` § "4. Seams": "| **6** meets compute at the payload only | no colour
    equation references chart, integrator, or scheduler state |".
  - `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules", "The occupant rules `cargo xtask lint
    wgsl` checks": "**A colour occupant** is the occupant of any stain node but `OUT` (Part 2): a source, colour,
    brightness, combiner or post"; "**Chart, integrator and scheduler identifiers** (seam 6) are, in the fragment's
    context: the chart's, the chart lane (`ctx.chart`; …)"; the named exceptions are views "when they are built, the
    DECODE view, its agreement presets and ROUNDTRIP, which read `ctx.chart.z`", each "excepted by the task that builds
    it".
  - `docs/design/principia_colour_composition.md` § "3. The `ctx` contract", "Fragment-side recompute": the fragment can
    "recompute cheap quantities (decode `z` → shape/energy; `encode(decode(z))` residual)". The encode is
    `E = (block/chart inverses) ∘ C` (`principia_dd_encode.md` § "2. Consolidated contract"), so ROUNDTRIP already runs
    the view chart's inverse per pixel.
  - `docs/design/principia_state_map.md` § "SM2": "the encode, including the chart inverses | WGSL generated from the
    one Rust source (R-116); `ROUNDTRIP` already runs it per pixel"; "It is live-compatible: only the playhead state is
    read." § "SM11" and § "SM15": "at (playhead · t = 0 · a fixed t)". The human's message: "position on a chosen chart
    at the playhead (or t = 0, or a fixed t)" and "**SM2 is a hard constraint** … The field is derived at read in the
    fragment stage from data it already has (`r, p` at the playhead, the masses, the `state` lane, the WGSL encode)."
- **The conflict:**
  - Chart position is a source, so a colour occupant; its projection `P_T` reads the target chart (its id, params and
    inverse), which seam 6 forbids an occupant and the lint fails. No exception names it. The view chart's inverse is
    already in the encode `ROUNDTRIP` runs (dd_encode §2); only a target other than the view's chart (Q1, RQ-480) needs
    a second inverse in the fragment. *Was (before PR #270's physics review, P5):* "and the chart inverses are not yet
    in fragment WGSL (`ROUNDTRIP` runs the encode)".
  - "A fixed t" other than `0` and the playhead needs the state at that t. `SimState` holds the playhead's only (payload
    §1), the prototype offers no fixed t (its menu: playhead, `t = 0`), and the state there is not data the fragment
    already has: it needs a stored field (which SM2 forbids) or a second march.
- **Options seen:**
  - (a) A stated seam-6 exception for Chart position, as for ROUNDTRIP and DECODE (render contract Part 4's list and the
    lint's), with the target chart's inverse generated to WGSL from the one Rust source (R-116); a fixed t dropped.
  - (b) The target chart and its parameters as the node's own uniforms (render-side, not `ctx.chart`), so seam 6 is kept
    as written; a fixed t only while the playhead is at it.
  - (c) Chart position not offered until the seam is re-ruled; a fixed t as a second march, measured for cost first.
- **Applied meanwhile:** the playhead and `t = 0` only, no fixed t; no field is added. TASK-M7-47, which builds the
  source, waits on this.

## RQ-495: R-488's whole-graph fallback reads the measured post-stage time each frame, and the quality device note gives every knob one writer, model-based, with no per-frame setting change, and switches the arbiter off under a named tier: who owns the post level, and does the fallback run under a pinned tier *(silence and conflict: the human's P4 against `principia_quality_device_note.md` §1, §2, §5 and §10 on origin/main, R-488, R-499; context: the plan PR's degrade task)*

*Found by perf review 5596612647 of PR #268 (F2). For the human: yes, it decides whether a pinned tier can still be
overridden while moving.*

- **File, section:**
  - `decisions.md` § "R-488 — …", the human's P4: "a whole-graph fallback when over budget".
    `docs/design/principia_print_graph.md` § "8.1 Degrade while moving", step 2: "If the post stage still exceeds its
    share of the frame budget, the **whole graph** drops a level: reduced, then Plain." `docs/design/principia_benchmark.md`
    § "8. Determinism of the work": degrade-while-moving "uses its policy, not a measured budget" in a swept row.
  - `docs/design/principia_quality_device_note.md` § "1. One arbiter, many sensors": "a single policy owns every knob
    write"; § "2. Model-based, not reactive"; § "5. Timescale cascade", the per-frame layer: "**No settings change,
    ever.**"; § "10. Two sanctities": "Named tier or custom → the arbiter is OFF, entirely."
- **The gap:** neither R-488 nor the design doc says whether the post level is the arbiter's lever or a second,
  reactive writer, nor whether the over-budget fallback runs when the user has pinned a named tier or Custom.
- **Options seen:**
  - (a) The post level is one of the arbiter's levers: the fallback runs only under Auto, model-based, and never under a
    named tier or Custom, where only the per-node motion policies (step 1) and settle (step 3) apply.
  - (b) The fallback is part of the Print graph itself, outside the arbiter, and runs under every tier, as §8.1 words
    it; the quality device note's rules are read as covering the tier's knobs only.
  - (c) (b) under Auto, and under a pinned tier the fallback runs only while moving and is recorded, the tier untouched.
- **Applied meanwhile:** none; §8.1 stands as written. What waits: the plan PR's degrade task writes the owner from the
  ruling.

## RQ-496: SM2's "the brightness slot is None" against L-ownership's combiner choice, and SM15's Replace-L chroma scaling against Replace-L as ruled, "`(a, b_ab)` untouched" *(conflict: the human's design against the render contract, a design doc's tests and a ruling's truth table; R-542, R-77, R-64; REQ-COL Replace-L requirements; context: TASK-M7-50, TASK-M7-48)*

*Found by R-542's port. Nothing is ruled.*

- **File, section:**
  - `docs/design/principia_state_map.md` § "SM2", item 3: "**The colour map owns lightness** (SM6), so under
    L-ownership (render contract) it is a full-colour occupant: the brightness slot is None." § "SM11": "The plane's 2D
    gradient … hold L fixed, so a brightness node can take L without losing position", and its Displacement preset puts
    a brightness node after the 2D gradient. § "SM15": "Replace-L scales chroma by `min(1, L/0.75, (1 − L)/0.35)` along
    with L"; "**Backwards compatibility.** Nothing to migrate: every saved stain loads and renders as before"; test (1)
    "Every saved stain renders bitwise the same". § "SM12" Shape n̂ unchanged: "Replace-L now scales chroma (below),
    which changes a sphere stain *with* a brightness node".
  - `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules": "**L-ownership (reserved lightness).**
    When a brightness metric is bound, L belongs to brightness; the colour occupant contributes hue/chroma only. …
    a semantic conflict, not a bug, resolved by combiner choice (Replace default; Multiply preserves base L)."
    "Full-colour occupant" appears nowhere on main.
  - `docs/design/principia_dd_colouring.md` § "3.5 Combiners": "`(a, b_ab)` untouched; → RGB. Preserves hue and chroma
    exactly." § "5. Unit tests", 6: "Replace-L leaves `(a, b_ab)` bit-stable".
  - `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants": "| C | B |
    `OKLab(L=B, a=Cₐ, b=C_b)` |", and "Replace-L here is `L = B`: the range form … (R-77)".
- **The conflict:**
  - SM2 item 3 makes the brightness slot None for the whole occupant; L-ownership and SM11/SM15 let a brightness node
    follow an L-fixed base, the conflict surfaced as a Problems warning.
  - The chroma scaling changes Replace-L for every stain, not only the state map's: it breaks dd_colouring §3.5 and test
    6, the §4.1 truth table, and SM15's own "renders bitwise the same" for any saved stain with a brightness node
    (SM12 says so).
- **Options seen:**
  - (a) SM2 item 3 read as "an L-spending base takes no brightness node without a Problems warning; Multiply keeps its
    L"; the chroma scaling a Replace-L option (or only on Chart position's bases), the default unchanged, so saved stains
    render bitwise as before.
  - (b) The chroma scaling for every stain, as SM15 has it, with dd_colouring §3.5, its test 6 and the §4.1 truth table
    re-ruled, and SM15's backwards-compatibility test narrowed to stains without a brightness node.
  - (c) No chroma scaling; the 0.30 level's near-black left to the Problems warning.
- **A consequence for the ruling** (PR #270's gui review, note): under main's Replace-L the Mixing preset's "stayed"
  level (`B = 0.30`) clips toward near-black, close to the undetermined `#141418`; TASK-M7-51 builds Mixing before this
  is ruled.
- **Applied meanwhile:** main's Replace-L, unscaled, for every stain; SM2 item 3 read as (a)'s first half (colour
  composition §4.1's R-542 paragraph). The scaling requirement carries `rq: RQ-496`, and TASK-M7-50, which builds it,
  waits on this.

## RQ-497: The stain graph's types and nodes: Shape n̂ is a `vector` field with one colour editor on main, where the state map calls it `point⟨sphere⟩` with Position, Site blend and Bands nodes to swap with; a field subtype is no hard gate on main, where SM15 offers only what the source takes; a source has one param, and band-masks are posts; and the Print graph (PR #268) uses the same words *(conflict: the human's design against the render GUI spec, colour composition and an open branch; R-542, R-64; PR #268; context: TASK-M7-51, TASK-M7-48)*

*Found by R-542's port and its check of PR #268 (`rulings/print-graph`, 1b7ca0ed). Nothing is ruled.*

- **File, section:**
  - `docs/gui/principia_render_gui_spec.md` Part II § "3. The node graph — model": "`field` — a ctx field, **tagged
    with a subtype**: `scalar` / `vector` / `categorical`"; § "9. Node inspector — morphing editors": "| `colour` ←
    `vector` (n̂) | **shape-sphere / SiteBlend editor** |"; § "3.1 Node kinds": "| `source` | — | `field` | one param:
    **which ctx field** (dropdown) |" and "| `post` | … | one op (overlay / band-mask / tone); chained |"; § "6. Port
    typing & wire rules": "**Subtype within `field` is *not* a hard gate:** … the node's inspector **morphs** to the new
    subtype's editor".
  - `docs/design/principia_colour_composition.md` § "7.1 The complete map list": no Shape n̂ node "Position (linear in
    n̂)" or "Bands over n̂"; §1.2's `bands` is a ramp on (θ, φ).
  - `docs/design/principia_state_map.md` § "SM15": "Shape n̂ is a `point⟨sphere⟩`, and its nodes accept exactly that, as
    now"; "Swapping the source between Shape n̂ and Chart position puts each base's counterpart in its place (Position ↔
    Gradient, Site blend ↔ Site blend, Bands ↔ Palette)"; "The library and the drag-off-a-pin menu offer only what the
    wired source takes"; test (3) "A mixed quotient on Displacement is an error". § "SM12" Shape n̂ unchanged: "every one
    of its colour nodes (Position, Site blend …, Bands)". § "SM11": layers are "a `bandmask` … or for site dots a
    `site_overlay`" in the colour slot.
  - Board 4 (`docs/gui/reference/state_map/boards/Ports.dc.html`): "Chart position's chart from Latent to Shape sphere
    and the wire becomes point⟨sphere⟩. The flat base takes its sphere counterpart's place (Gradient → Position, Site
    blend → Site blend, Palette → Bands); the layers have no sphere form and are kept, ghosted". SM11 and the prototype
    instead give Chart position on the sphere its own bases and layers.
  - PR #268 (`rulings/print-graph`): the Print graph's nodes `palette`/`palslot` ("Palette"), `flow` ("Flow"), a field
    input node of type `"field"`, and its stage "post" (the profiler's `stage_ms.post`); print_graph §4.4: "A graph may read a payload field as a
    `scalar` input", by its `range_norm`, which that branch queues. SM14 adds "chart position at the playhead" to payload §5's derived
    table, a `point⟨q⟩` with no ledger range.
- **The conflict:** main's Shape n̂ is `vector` with one colour editor, so the Position and Bands counterparts SM15 swaps
  with do not exist on main (they are the prototype's); SM15's gate against §6's morphing; §3.1's one-param source and
  band-masks as posts against Chart position's four params and its layers inside the colour slot; board 4 against SM11;
  and, on the open branch, two graphs sharing "Palette", "Flow", "field" and "post", and a derived payload quantity that
  could read as a Print graph field input without a range.
- **Options seen:**
  - (a) `point⟨q⟩` a fourth subtype carried by Chart position only, Shape n̂ staying `vector`; Site blend ↔ Site blend
    the only swap until Shape n̂'s Position and Bands nodes are ruled in; §6's morph kept between subtypes and SM15's
    filter applied to menus only; Chart position a source with its own params, its layers combinators inside the
    occupant (colour composition §1.3); board 4's sphere paragraph superseded by SM11; the state map's terms qualified
    in the GUI where both graphs show ("Palette base", "Picture flow"), and Chart position not a Print graph field input.
  - (b) Shape n̂ re-typed `point⟨sphere⟩`, with its Position and Bands nodes added to §7.1 and the golden suite (a change
    to Shape n̂, which §0 keeps "exactly as it is").
  - (c) Each part ruled separately.
- **Applied meanwhile:** (a), in render GUI spec Part II §3, §3.1, §6 and §9 (R-542), except the qualified names, which
  wait for the ruling: the GUI uses the file's names ("Palette") and the code's preset names (RQ-499) meanwhile (PR #270's
  gui review, G2). TASK-M7-51 builds to it; the Shape n̂ unchanged requirement carries `rq: RQ-497`. Whether the graph
  draws the layers as their own node is RQ-502.

## RQ-498: Where Chart position draws a system that is not at a plain chart point: SM1 folds the mirror where SM4 and the prototype's code read the signed n̂, and R-141's decode gauge puts a lower-hemisphere pixel's system on the upper one; mirror pixels inside the square are equal to rounding, not bitwise; the trace draws a left or past-the-end sample as no value, where SM5a clamps; and on an affine chart there is no navigation edge *(conflict: the file against itself and its code, and against rulings, a design doc and an open branch; R-542, R-141, R-412, R-469, R-470, R-503, R-537 EX3 and EX7, R-538 TF1; context: TASK-M7-47, TASK-M7-48, TASK-M7-52, TASK-M8-66)*

*Found by R-542's port and its check of `task/TASK-M2-24` (PR #262), `task/TASK-M2-28` and main; the mirror part
restated and R-141's gauge added after PR #270's physics review (5598441029, P1 and P2). Nothing is ruled.*

- **File, section:**
  - **The mirror, in the file and the code.** `docs/design/principia_state_map.md` § "SM1. Definition": "`E` is the
    encode …: scale and rotation are removed, mirror images identified." § "SM4. Destination side": "| the shape sphere
    (θ, φ with pole crossing) | the sphere, via `n̂` | be a function of `n̂`, never of (θ, φ) |", the unfolded row; only
    "| the hemisphere (R-412) | the sphere mod the mirror |" folds. The prototype's code
    (`docs/gui/reference/state_map/state_flow.html`, `encode`, "shape sphere, before any canonicalisation (§3.1)", and
    `smWhere` case 'B'): `(θ, φ)` from the signed `n̂`, over both hemispheres. Main: `principia_chart_reference.md`,
    "encode's `β = atan2(|λ̃_y|, λ̃_x)` reads `|λ̃_y|`" (the encode's chart point folds); payload §5's derived `n`,
    "`shape(r)` or `shape(r, masses)`", keeps its sign.
  - **R-141's decode gauge.** `decisions.md` § R-141: "the canonical decode gauges λ̃_y → −λ̃_y, so both hemispheres
    decode to the same system". On main a lower-hemisphere pixel decodes to a system with `n₃ ≥ 0`; the prototype's
    `decodeShape` does not gauge (`ψ = atan2(n₃/2, n₂/2)`), so its lower hemisphere decodes to the true mirror system,
    which is where SM7's "sphere 1.5e-6" was measured.
  - **Mirror pixels in the square.** R-141 (the spherical map's two hemispheres decode to one system) and R-503 (the
    exponential map's cap: "a pixel counts once unless its mirror `w → −w` is in view"): each pixel is decoded from its
    own coordinate. R-537 EX7(b)'s canonical address covers repeats outside the primary range. `principia_state_map.md`
    § "SM3": "**only if a copy reuses its canonical quad's samples (EX7's canonical addressing)**"; § "SM12" Copies agree:
    "its IC is **bitwise** equal".
  - **Left the chart.** `docs/design/principia_lines.md` § "8. Traces", "Where a trace can leave (R-538 TF1)": "A
    trace leaves only where a coordinate entering a shown axis with non-zero weight has no value"; the curve axis: "A
    point whose nearest ν lies past a bounded edge is not drawn there, the line broken, with no ring … counted in the
    readout's "past the domain's end: N samples""; a left sample is drawn as a dotted ring, the line dashed until it
    re-enters. `principia_state_map.md` § "SM5": "**at the edge it left by** (default, SM5a)"; § "SM9": "the trace is
    this field for one pixel"; § "SM12" Four results: "neither is drawn as a colour | left drawn as a clamped colour".
    R-470 N1 and R-472 use "past a bounded edge" for a source pixel drawn blank.
  - **The navigation edge.** `decisions.md` § R-469: the centre "accepted only while the chart's centre passes
    `validate`"; R-537 EX3: "The only navigation limit is R-469's … **Affine latent axes:** no stop at all."
    `principia_state_map.md` § "SM3": "on an affine chart it is the navigation limit"; § "SM11": "A system that has left
    the chart takes the cell at the navigation edge it left by".
- **The conflict:**
  - SM1's fold against SM4's unfolded shape-sphere row and the code's signed `n̂`. Under the fold, Chart position ⟨shape
    sphere⟩ and Shape n̂ (signed `n`) read mirror points where `n₃ < 0`, and SM11's "the two read the same point" and
    SM12's Same site blend (for `axes6`, not mirror-symmetric) fail; under the code's reading both hold as written.
  - R-141's gauge: on a both-hemispheres view a lower-hemisphere pixel's system lies on the upper hemisphere, so its
    Chart position at `t = 0` is the mirror of its own pixel's point under either reading, and SM7's identity holds there
    only modulo the mirror; the prototype, which does not gauge, does not show it.
  - SM3's and SM12's bitwise identity holds for EX7 repeats only.
  - SM5a's default draws as a colour what the trace draws as no value, and SM12's Four results test says the opposite
    of SM5a.
  - The latent charts have no navigation edge to clamp to.
- **Options seen:**
  - (a) The code's reading: Chart position ⟨shape sphere⟩ reads the signed `n̂` (payload §5's `n`, no stored field) over
    the full sphere, SM1's "mirror images identified" read for the encode's other coordinates; the `t = 0` identity on
    the sphere stated modulo the mirror (a lower-hemisphere pixel compared with its mirror point, R-141); bitwise copies
    for EX7 repeats, rounding-level for in-square mirrors; the clamp kept as the default, SM9 read for on-chart samples,
    Four results run under "when it leaves: none", where its arm holds as written; the clamp's edge the compacted edge
    `s = ±1` (where main's clamped encode puts it, `|z| ≈ logit(1 − 10⁻⁶)`), and the prototype's base grid a demo device
    (the app's fill is EX4's coarse ancestor and EX7(a)'s roots).
  - (b) SM1's fold: the shape-sphere target is the hemisphere (`E`'s fold), the identity and site-blend tests compare
    folded points and SM11's sentence is read for `n₃ ≥ 0`; "no value" the default, as the trace has it, the clamp an
    option.
  - (c) Each part ruled separately.
- **Applied meanwhile:** (a), the code's reading (the human's message: "Where the md and the code disagree, the code
  wins and the disagreement is queued"). *Was (before PR #270's physics review):* the meanwhile folded the shape-sphere
  target onto the hemisphere, the identity and site-blend tests measured there. The requirements on the identity, the
  copies, the clamp, the four results and the sphere's site blend carry `rq: RQ-498`; their tests run as (a) reads them.
  The hemisphere view's target is RQ-504.

## RQ-499: The prototype's code and the state map's text disagree: preset names, the triangle palette's lightness, Picture under a brightness node, the fade's function, and the Fibonacci cells' swatches *(conflict: the file against its transcription source; R-542; context: TASK-M7-48, TASK-M7-49, TASK-M7-51)*

*Found by R-542's port, reading `docs/gui/reference/state_map/state_flow.html` as source (no browser was available).
The human's message: "Where the md and the code disagree, the code wins and the disagreement is queued, not resolved."
Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` against the prototype's code (each transcribed in the
  section's *Transcribed from the prototype (R-542)* paragraph):
  - **Preset names** (§ "SM15"): "Where …; Graph paper …; Basins …; Displacement …; Flow …; Mixing", and board 5;
    the code's `smPreset` buttons read "Classic shape colour", "Where now", "Graph paper", "Basin tiles",
    "Displacement", "Picture flow", "Mixing".
  - **The triangle palette** (§ "SM11"): "**by index** — … nine OKLab swatches at `L = 0.72`"; the code gives triangles
    `L = 0.75` or `0.69` by orientation.
  - **Picture and brightness** (§ "SM8", § "SM11"): the brightness slot is "outside the colour slot" and "unchanged";
    the code applies no brightness under a Picture base.
  - **The fade** (§ "SM10"): "R-509's distance, as the trace"; the code's fade is `max(0.18, 1/(1 + (d/0.35)²))`, the
    trace's `max(0.18, e^{−d/1.2})`.
  - **Fibonacci cells** (§ "SM11"): "**by index** … or **golden-angle** hues by tile"; the code colours Fibonacci cells
    golden-angle always.
- **Options seen:**
  - (a) The code, as transcribed: these are the behaviour, and the file's words are read through the transcriptions.
  - (b) The file's words, the prototype's code a bug.
  - (c) Each ruled separately.
- **Applied meanwhile:** (a), the code (the human's message). The presets requirement carries `rq: RQ-499` for the
  names; TASK-M7-48, TASK-M7-49 and TASK-M7-51 build the code's behaviour.

## RQ-500: Board 6's "Edit graph…" in Explore's top bar is the label PR #268 gives the Print panel's button *(conflict: the human's board against an open branch; R-542, PR #268; context: TASK-M8-66)*

*Found by PR #270's gui review (5598419574, G1). Nothing is ruled.*

- **File, section:** `docs/gui/reference/state_map/boards/` board 6 (Explore): the top bar's "Stain [Chart position]"
  and "Edit graph…", which opens the stain editor. PR #268 (`rulings/print-graph`), the Print panel: its button "Edit
  graph…" opens the Print graph's editor.
- **The conflict:** one label, two editors, both reachable from Explore.
- **Options seen:**
  - (a) Each qualified by its graph: "Edit stain…" in the stain's place, the Print panel's "Edit graph…" as PR #268 has
    it (or "Edit print graph…").
  - (b) Board 6's label kept, PR #268's renamed.
  - (c) Board 6's button not built: the stain editor is reached as it is on main (§G2, `02_stain.png`).
- **Applied meanwhile:** (c) until ruled: the shell follows `01_main.png` (render GUI spec §G13, R-542's bullet), so no
  "Edit graph…" is added to Explore's top bar by the state map.

## RQ-501: Board 6 draws a ring on the hovered pixel with no hover trace, and its hover label has no "L listen", where R-541 and §G2 keep both *(conflict: the human's board against rulings; R-542, R-540, R-541, §G2; context: TASK-M8-66)*

*Found by PR #270's gui review (5598419574, G1). Nothing is ruled.*

- **File, section:** board 6 (Explore): the figure shows a ring at the hovered pixel and no trace; the hover label
  reads the system without "L listen". `principia_state_map.md` § "SM9": "The prototype draws no trace, only a ring on
  the hovered pixel" (it does not say the app drops the trace). `docs/gui/principia_render_gui_spec.md` § "G2. Explore —
  the everyday view (`01_main.png`)": the hover trace and its label; R-541 G1–G4 and R-540: the label's listen hint.
- **The conflict:** the board, read as a layout, would drop the hover trace and the listen hint under a Chart position
  stain.
- **Options seen:**
  - (a) The hover trace and its label stay as R-541 and §G2 give them, and the figure gets no ring while the trace is
    shown: the trace's start star marks the pixel, and SM9's marker is the legend's ring (render GUI spec §G13's R-542
    bullet; TASK-M8-66).
  - (b) Board 6's ring on the hovered pixel: under a Chart position stain the trace is hidden and the ring replaces it,
    as board 6 draws.
  - (c) Both: the ring added on the figure beside the trace, written into §G2 with its key entry (a figure mark under
    R-539, in R-541 P13's trace block).
- **Applied meanwhile:** (a). *Was (before PR #270's gui re-review, G7):* "(a) The hover trace and its label stay as
  R-541 and §G2 give them; the ring is added on the hovered pixel beside them, and the legend's ring marks the system
  (SM9)."

## RQ-502: Whether the stain graph draws Chart position's layers as their own LAYERS node, as boards 1 and 4 do, or inside the colour occupant, as render GUI spec §3.1 and §9 now say *(conflict: the human's boards against the applied text; R-542, R-64; context: TASK-M7-51)*

*Found by PR #270's gui review (5598419574, G4). Nothing is ruled.*

- **File, section:** board 1 (editor) draws BASE and LAYERS as two graph nodes, LAYERS with in-ports `base` (vec3) and
  `point`; board 4 (ports) lists "Layer … base: vec3 · point → vec3". `docs/gui/principia_render_gui_spec.md` Part II
  § "3.1 Node kinds" (R-542 paragraph): "Its layers are combinators inside the colour occupant (colour composition
  §1.3, §1.5), not `post` nodes"; § "9. Node inspector — morphing editors": one base-and-layers editor (board 3). §3.1's
  table has no kind with a `vec3` in-port ahead of the combiner. `principia_state_map.md` § "SM15": "the inspector shows
  them as a stack under the base, reorderable, each removable."
- **The conflict:** the graph either shows one colour node with the stack in its inspector, or a base node feeding a
  layers node with a `vec3` in-port, a node kind §3.1 does not have.
- **Options seen:**
  - (a) One colour node; the base and the layer stack in its inspector (§3.1, §9, board 3, SM15's text), boards 1 and
    4's LAYERS node read as a drawing of that stack.
  - (b) A LAYERS node kind with a `vec3` in-port ahead of the combiner, added to §3.1 (an extension of R-64).
- **Applied meanwhile:** (a): the graph draws one colour node for the base and its layers; REQ-GUI-335's screenshot
  is read against board 3's inspector and board 1's library and Problems rows, not board 1's two nodes.

## RQ-503: The prototype's latent chart uses `AMIN = 0.05`, where R-21 sets `α_min = 0`, so SM5a's latent-chart figures and "it leaves because α passes the decoder's limit" describe the prototype, not the app *(conflict: the transcription source against a ruling; R-542, R-21; context: TASK-M7-47, TASK-M7-52)*

*Found by PR #270's physics review (5598441029, P3). Nothing is ruled.*

- **File, section:** `docs/gui/reference/state_map/state_flow.html`: `AMIN = 0.05`, its latent decode and encode
  `α = AMIN + (π/2 − 2·AMIN)·σ(z_α)`. `decisions.md` § R-21 and `docs/design/principia_chart_reference.md` § 0.2: `α_min
  = 0` ("full-sphere coverage"); main's latent α axis runs out only at α = 0 or π/2 (λ̃ = 0 or ρ̃ = 0), within
  `ε_z = 10⁻⁶`. `principia_state_map.md` § "SM5a": "On the latent chart it leaves because α passes the decoder's limit";
  "On the latent chart at `t = 8`, 400 random systems, 129 of them left"; "On the latent chart, 3186 of 9216 have
  **left**".
- **The conflict:** on main almost no system leaves the latent chart off a collision, so SM5a's latent figures and a
  test built on them (a population of left systems on the latent chart) would be empty and could not fire.
- **Options seen:**
  - (a) Main's `α_min = 0` stands; SM5a's latent figures are the prototype's, reference only; the edge-clamp test runs
    on a target that leaves on main (the latent momenta at the clamp, or the `z_q3` tilt), its figures measured on the
    app; the latent α case is tested on a constructed state (λ̃ or ρ̃ within `ε_z` of 0).
  - (b) Chart position's latent target uses the prototype's `α_min = 0.05` (a second latent chart, a chart change).
- **Applied meanwhile:** (a). REQ-COL-116's verify is on the latent momenta; REQ-COL-113 constructs the α case.

## RQ-504: The one-hemisphere view's Chart position target: the prototype targets the full sphere, SM4 the sphere mod the mirror *(conflict: the file against its code; R-542, R-412; context: TASK-M7-47)*

*Found by PR #270's physics review (5598441029, P1). Nothing is ruled.*

- **File, section:** `principia_state_map.md` § "SM4. Destination side": "| the hemisphere (R-412) | the sphere mod the
  mirror | be a function of the canonical (folded) `n̂` |". The prototype's `smTarget`: "the view's own" on the
  one-hemisphere view resolves to the full sphere (transcribed in SM1's *Transcribed from the prototype* paragraph).
- **The conflict:** the code colours a hemisphere view's systems by their signed `n̂` on the full sphere; the file folds
  them.
- **Options seen:**
  - (a) The code's: the one-hemisphere view targets the full sphere, signed `n̂` (with RQ-498 (a)).
  - (b) The file's: the sphere mod the mirror, the canonical folded `n̂`.
- **Applied meanwhile:** (a), the code (the human's message).

## RQ-505: "'none' must be available as a debug option" (the bundle's draft requirement) against SM5, SM11 and SM15, which offer "none" as an ordinary setting of the node *(conflict: the human's draft against the file and its code; R-542; REQ-COL-116; context: TASK-M7-47, TASK-M7-51)*

*Found by PR #270's code review (5598464229, 3). Nothing is ruled.*

- **File, section:** `docs/incoming/2026-10-21_state_map/requirements_patch.yaml`, the edge-clamp draft: "A system that
  has left the chart must be drawn at the edge it left by, with no stored history; 'none' must be available as a debug
  option." `principia_state_map.md` § "SM5": "**at the edge it left by** (default, SM5a); or no value, in its own
  style"; § "SM11" and § "SM15": "when it leaves the chart (clamp to the edge · none)", a node setting; the prototype's
  "When it leaves" control offers both.
- **The conflict:** debug-only, or an ordinary node setting.
- **Options seen:**
  - (a) An ordinary setting of the node, as the file and the code have it (which includes the debug use).
  - (b) Debug only: offered in the debug views, not in the node's inspector.
- **Applied meanwhile:** (a), the code. REQ-COL-116 keeps the draft's words, its note citing this entry.

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
