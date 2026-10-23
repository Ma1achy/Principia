# Principia: Sequence — keyframed view animations in the native GUI

*Adopted by R-576 and R-577 (gap sweep PR 2b, 21 Oct 2026), from the human's design `keyframes_reel.md`
(`docs/incoming/2026-10-21_gap_sweep_keyframes_reel/designs/`), its "Decided" section his words. The reel half of that
draft, and what a reel click opens, is `principia_spotlight_reel.md` (R-578). The engine side (the spec object,
interpolation, the planner's classes, the blocking export) is `principia_export_animation_contract.md` Parts 2 to 6,
built by TASK-M8-30 and TASK-M8-31; this document is the screen that makes a spec, and what it adds to the export.
Queued from it: RQ-559 (the Sequence's undo) and RQ-560 (a cut offered at a link-parameter change).*

## 1. What exists

The export contract defines the three segment kinds and their costs (Part 2), the keyframe spec and its per-field
interpolation (Part 3), link parameters eased in a blocking export only, flagged (R-520, Part 3), the blocking export
job (Part 4), determinism and the round trip (Part 5, R-484), sharing (Part 6) and the reel (Part 7).
REQ-TOOL-074 to REQ-TOOL-076 require them, and TASK-M8-31 builds the engine side. The GUI has only Record a time sweep
(`principia_render_gui_spec.md` §G9), and the open windows (`principia_gui_state_contract.md` §2) have no animation
window.

## 2. The gap

There is no screen to make keyframes, see a timeline, choose interpolation, price or preview a sequence, or export it.
The spec object had one `easing` for the whole animation; the human set easing per segment (R-576).

## 3. The design

### 3.1 The Sequence window

**Sequence** opens from Windows › Sequence… and is docked along the bottom of Explore: below the bottom row (Compass,
Time, Legend) and above the footer, its top edge a splitter (R-462), its height remembered with the layout and reset by
a double-click or View › reset layout. The console, docked over the bottom row (R-463), docks over the Sequence too
when both are open. It has a × to close it. Like every window it is `ViewUI`'s, never read by the engine
(`principia_gui_state_contract.md` §2). The sequence being edited is the GUI's own document, beside `ViewUI`: the engine
reads it only as the spec a preview, proof or export request carries (applied per R-369; whether its edits enter any
history is RQ-559). Outside the preview, the only `SetField`s the Sequence sends are Go to's (§3.2) and, after a reel
takeover, the live sequence player's (`principia_spotlight_reel.md` §3.3).

Its keyboard model is §G3's for a window, with three sections in order: the **header bar** (Add keyframe, Play preview,
Proof strip, Export…, Open…, Save…, ×), the **timeline** and the **inspector** (the selected keyframe's or segment's
controls). In the timeline, ← and → move between keyframes and the segments between them, in time order, and Enter
selects. Its scope's shortcuts (R-409): **K** adds a keyframe and **Delete** removes the selected one. The Figure's K
stays the lock (§G4, R-556); a scope's letter acts only with the focus in that scope, so the two never meet. Esc from
the window's top level returns the focus to the Figure and leaves the window open.

While a preview plays, Esc in the Sequence stops it and nothing else (§3.6).

**In a result view** (fix round 9 of PR #286, gui review 5631987230, G1; applied per R-369, as `principia_results.md`
§3.4 disables sim-key edits there "with the reason"). While the view source is `Result` (`principia_gui_state_contract.md`
§2), the figure draws a stored payload and not the snapshot's state, so every Sequence control that writes or captures
the sim key is disabled: Go to, Add keyframe, Update and Play preview are disabled, each with the reason "result view — switch to Live" shown
in the Sequence beside it, never over the figure (§G1). The way back is the existing view-source request (*Explore from here*,
or closing the result, which sets `Live`); nothing switches the source implicitly. Opening a result while a preview
plays ends the preview first, as a user edit does. The document edits that send nothing (selecting, dragging in time,
Delete, a segment's easing and transition, Open…, Save…, the Proof strip and Export…, which run through the blocking
exporter from the spec) stay enabled.

**Getting back in by keyboard.** The Sequence is a window, not a big scope: §G3's Tab order of big scopes (1 top bar …
8 footer, R-405) is unchanged. **Windows › Sequence…** on an open Sequence gives it the focus, landing on its header
bar in navigation mode, as opening it does; the top bar's menus are reached by keyboard as big scope 1 (§G3), so the
way back is Tab to the top bar, Enter into Windows, Sequence…. A click anywhere in the Sequence gives it the focus as a
click does (R-456). (Fix round 1 of PR #286, gui review 5631060675, F3; applied per R-369.)

### 3.2 Keyframes

- **Add keyframe** captures the whole `SimConfig` and the whole `RenderState` at the timeline cursor. A keyframe is
  always whole (Part 3: a keyframe's state is its `SimConfig`, links included, R-483 X7). **At an occupied frame**,
  where the cursor's own frame `⌊t_wall · fps + ½⌋` is an existing keyframe's, Add keyframe is refused with the reason
  ("frame n already holds a keyframe"), shown in the Sequence's header bar beside the button, and nothing is added, so
  keyframes' frames stay distinct and the window never builds a spec the exporter refuses (export contract Part 3; fix
  round 9 of PR #286, gui review 5631987230, G2 (a); applied per R-369).
- A selected keyframe shows its values (each field that differs from its neighbours first), and offers:
  - **Go to**: the view takes the keyframe's `SimConfig` and `RenderState`, bitwise, as one undoable history entry
    (as Home is one, R-466);
  - **Update**: the keyframe takes the view's state, whole;
  - **drag in time**, its `t_wall` clamped strictly between its neighbours', so keyframes never reorder, and so that
    its own frame stays strictly between theirs (export contract Part 3's frame grid; fix round 6 of PR #286, physics
    review 5631678451, P1): at the spec's fps a drag stops one frame short of a neighbour's frame;
  - **playhead policy**: **hold**, **advance with wall time** (with its rate, in `t` per wall second, set from the
    Transport's speed when chosen and editable) or **set to a value** (Part 3's "playhead policy"; its semantics are
    export contract Part 3's, fix round 6 of PR #286, P1, the rest REQ-TOOL-197's definition).

### 3.3 The timeline and its lanes

Wall time runs across; keyframes are diamonds on it and the timeline cursor a vertical line. Beneath, read-only
**lanes** light over a segment where a field in that lane differs between the segment's two keyframes, so the user
sees why a segment costs what it does:

| lane | what it holds |
|---|---|
| Navigate | pan and zoom within the plane (they only re-address, R-92): the in-plane part of `z₀`, the in-plane scale of the basis, and the lock |
| Plane | the slice plane, the sim key's navigation part (R-92): `z₀`'s out-of-plane part, span{q₁, q₂} and the in-plane orientation |
| Chart | the chart and its construction |
| Links | the decoder's links and their parameters (R-483) |
| Playhead | the playhead and the keyframe's playhead policy |
| Stain | the stain graph and its uniforms |
| Print | the Print graph (R-485) |
| Lines | the line system (R-476) |
| Run | every other `SimConfig` field: the occupant, the horizon, `E`, the thresholds |

**`z₀`, `q₁` and `q₂` are split as R-92 splits them** (fix round 6 of PR #286, physics review 5631678451, P3; applied
per R-369). A pan and an off-plane move both change `z₀`, and a zoom and a tilt both change `q₁, q₂`, so no lane can
take those fields whole. Between a segment's two keyframes they are decomposed by **the same function that computes the
sim key's slice plane** (R-92), never a second copy: Plane lights where the slice plane differs, and Navigate where the
in-plane part of `z₀`, the in-plane scale or the lock differs. A segment that lights only Navigate therefore never
changes the sim key, and one that changes the slice plane always lights Plane; a tilt may light both. *Was:* "| Navigate
| pan and zoom within the plane (they only re-address, R-92) |" and "| Plane | the slice and the basis: the slice centre
off the plane, the tilt, the slice frame |".

Every other field of `SimConfig` and `RenderState` is in exactly one lane (*was:* "Every field of `SimConfig` and
`RenderState` is in exactly one lane."). The table assigning each field is a definition
(R-72, REQ-GUI-375), written here by the task that builds the lanes and kept exhaustive at compile time, as the tier
table is (R-484). The draft's "Camera" lane is **Navigate** here, since no control is named after a camera
(`principia_render_gui_spec.md` §G1); "Run" is added so that every field has a lane (both applied per R-369).

**Segments are coloured by their class**, (a) time playback, (b) view animation or (c) recolour only, as TASK-M8-31's
planner classifies them (Part 2, REQ-TOOL-074), each labelled with its frame count at the spec's fps. A segment whose
chart or any link id changes shows a **cut mark** at its second keyframe.

### 3.4 Segments: easing per segment, and transitions

**Easing is set per segment, between keyframes (R-576).** The spec object carries one segment between each pair of
neighbouring keyframes, each with its own easing curve, which gives the segment's progress `s ∈ [0, 1]` at each frame
for every eased field (Part 3), the link parameters of R-520 included. The curves offered and the default are a
definition (R-72, REQ-TOOL-197), written into export contract Part 3 by TASK-M8-31. One `s` per frame, from the
segment-local fraction `u` on the frame grid, drives every interpolated field, the zoom and the basis included; linear
is among the curves; the playhead follows its keyframe's policy (export contract Part 3; fix round 6 of PR #286,
physics review 5631678451, P1; applied per R-369).

A selected segment offers:

- **its easing**, from REQ-TOOL-197's curves;
- **at a chart change, cut or dissolve**, a dissolve with its length (two renders composited, Part 3); live playback
  shows a dissolve as a cut at the second keyframe, as it shows every chart switch;
- **at a link-parameter change**, R-520 as written: interpolated in a blocking export, every frame a different decoder,
  and a cut in live playback, shown with R-520's words "link parameters interpolated: every frame a different decoder".
  The draft offers the user a choice here, cut or interpolate (export only); a cut in a blocking export is not R-520's,
  so the choice is RQ-560, and meanwhile the segment shows R-520's rule, not a choice;
- **the fixed rules**, shown, not offered: zoom log-lerped, the basis along the Grassmannian geodesic, `z₀` eased
  componentwise (REQ-TOOL-075).

### 3.5 Saving: the `.principia-seq.json` file

**Save…** writes the spec object, its keyframes and segments, fps and duration, in the one canonical serialisation
(R-309), with the build hash, the schema version and each link's registry-entry hash (R-521), to a file named
`<name>.principia-seq.json`. **Open…** reads one back: the identity on every keyframe's `SimConfig` and `RenderState`
and on every segment, bitwise (R-484). A file naming a link the build lacks, or whose entry hash differs, opens as
"state present, cannot recreate", naming the link, and is never recreated under another link (Part 6, R-521).

**Undo.** Whether an edit to the sequence (add, delete, Update, a drag in time, a playhead policy, a segment's easing
or transition) is undoable, and in which history, is RQ-559: the draft gives the window its own history, the contract
keeps one history and a GUI none of its own (R-52), and RQ-559 also offers no undo. Nothing is applied meanwhile: if
RQ-559 rules a history, each edit is one entry in it, a drag one entry; if it rules none, no sequence edit enters the
contract's history. Go to is the contract's own entry either way. *Was:* "Every edit to the sequence (add, delete,
Update, a drag in time, a playhead policy, a segment's easing or transition) is undoable and redoable, one entry per
edit, a drag one entry. The draft gives the window its own history; the contract keeps one history and a GUI none of
its own (R-52), so which history is RQ-559." (fix round 1 of PR #286, gui review 5631060675, F5).

### 3.6 Preview and the proof strip

- **Play preview** runs the sequence through the live frame loop from the timeline cursor, with link-parameter changes
  and dissolves shown as cuts (R-520 F3), under a banner in the Sequence's header bar: "Preview: not converged; cuts
  where the export interpolates". While it plays the timeline cursor follows it. A preview started mid-way through a
  link-parameter ease shows the first keyframe's value up to the second keyframe (say `μ_max` 5 between 5 and 4),
  wherever it starts: it is ordinary live playback (export contract Part 3). A reel takeover at the same frame would
  instead hold the frame's eased value (4.6), RQ-561's interim (`principia_spotlight_reel.md` §3.3); the two differ
  under RQ-561's interim; whether they differ once it is ruled is RQ-561's second question (fix round 9 of PR #286,
  gui review 5631987230, G3; fix round 11, physics review 5632129472, P7; *was:* "the two differ until RQ-561 is
  ruled").
  - **A transient view, not an edit.** The GUI sends one **preview request** carrying the spec and the cursor's wall
    time; the engine plays the spec from there through its live frame loop and draws the preview's frames in place of
    the view. No `SetField` is sent and the contract's `SimConfig` and `RenderState` are never changed, so nothing
    enters the undo history and R-69 is not engaged; the GUI clock's playhead advance (R-101) is held while it plays,
    the preview owning its playhead by each keyframe's policy. The snapshot carries the preview's state (on or off, its
    wall time), which the banner and the cursor read (`principia_gui_state_contract.md` §2).
  - **When it stops or ends** (Stop, Play preview again, Esc in the Sequence, or the spec's end), the preview goes and
    the view is the user's view from before it, bitwise, since it was never changed. To keep a preview's frame the user
    moves the cursor there and uses a keyframe's Go to, or Update.
  - **The user's cover stays pinned** (fix round 12 of PR #286, perf review 5632950982, P3; applied per R-369). While a
    preview runs, the user's view's baseline cover is held as a **third pinned identity** beside caching Part 5's two
    (current and backdrop), so the preview's identities never evict it. Returning shows that cover at once, never
    blank, and costs no recompute of it; only quads the user's view had not yet completed continue as they would have.
    It costs one more viewport-bounded baseline cover of memory, under the hard cap like the other pins (caching Part 4,
    Part 5 and Part 7), so pinned bytes are at most three baseline covers; when the cap binds the preview's own
    non-pinned work is evicted first, never the user's cover (fix round 13 of PR #286, perf review 5634346116, P3').
  - **Esc in the Sequence while a preview plays** stops the preview and does nothing else: the focus and the keyboard's
    mode stay where they were. The next Esc acts as §G3 has it (§3.1) (fix round 2 of PR #286, gui review 5631227164, G4; applied per R-369).
  - **A user edit while it plays** (a `SetField` that enters the history: a pan, a slider, Home, a stain edit) or an
    undo or redo request first ends the preview, and then applies to the user's view as usual; the preview never
    overwrites it. A no-history `SetField` (the hover's, the clock's advance) does not end it. *Was:* "(any `SetField`
    the GUI would send: a pan, a slider, Home, a stain edit, undo or redo) first stops the preview, and then applies to
    the user's view, an ordinary entry; the preview never overwrites it." (fix round 2 of PR #286, gui review 5631227164, G1; applied per R-369: undo and
    redo are requests, not `SetField`s, and the hover is a no-history `SetField`.)
  - **The rest of Explore while it plays** (fix round 2 of PR #286, gui review 5631227164, G2; applied per R-369). The figure shows the preview's frames, but
    the snapshot's `SimConfig` and `RenderState` are the user's, so nothing that reads them may describe the figure: the
    header strip shows a badge **preview** (R-523) and no decoder badge; the Legend shows no keys, only the line
    "Preview: the legend returns when it stops"; the top bar's status and Time's readout show "—" for `t`; the hover is
    held off (no `Hover` is sent and no hover trace, label or sample is drawn, since its sample would name an IC of the
    user's chart, not the preview's); the Inspector and Trajectory panel show nothing new. The Manifold view, Compass,
    Time and the other controls keep showing the user's view, which is what they edit: touching one is a user edit,
    which ends the preview first.
  - *Was:* "It is playback, and playback never enters undo (R-101): each frame's state is sent with "no history", and
    stopping it leaves the undo depth as it was." (fix round 1 of PR #286, gui review 5631060675, F2; applied per
    R-369: R-101 covers only the clock's playhead, and R-69 makes every `SimConfig` and `RenderState` edit undoable, so
    the preview edits nothing.)
- **Proof strip** renders one small frame per keyframe and one per segment midpoint through the blocking exporter
  (Part 4) at its lowest export quality tier, so the user sees exact frames before paying for all of them. A proof
  frame is an exported frame, not the live loop's (Part 1, R-66): at the same time, size and tier it is the exported
  frame bitwise. It runs as a **lower-priority background job off the GUI thread** (`principia_dd_telemetry_and_tiers.md`
  § "CPU": background work runs at lower priority and yields; caching Part 6), with progress and **Cancel**, live work
  first. The proof frame's size ("small") is a calibration requirement (R-71, REQ-GUI-388), provisional until M8;
  no corpus value gives it (fix round 12 of PR #286, perf review 5632950982, P6; applied per R-369).

### 3.7 The planner and export

**Export…** in the Sequence's header bar opens the **planner** before anything runs. One row per segment: its class,
its frames, its integration cost ((a) one march to the segment's end; (b) `0 → t_frame` per frame; (c) none, Part 2),
its flags (link parameters interpolated, R-520; a dissolve is two renders) and its estimated time.

- **An estimate carries its basis** (fix round 12 of PR #286, perf review 5632950982, P1, P2; applied per R-369). A segment's cost is the samples it
  marches (the export's pixels times its copies, `E + 1`) times the steps each is marched (the physical time of the
  march over `dt`). The planner prices it with **the one throughput model the app keeps**,
  `principia_quality_device_note.md` § "2. Model-based, not reactive" ("device throughput (cost-per-dt-per-sample)"),
  read under its provenance signature (§ "9. Persist the model, not just the rung"): the estimate prices the cost
  with that scalar, as defined below, one value, shown with its provenance key, since the model gives one value, not a range. No second estimator
  is built from frame records. With no model under a matching provenance it reads "no measurement on this machine
  yet", never a number.
  **The estimate's unit, defined** (fix round 13 of PR #286, perf review 5634346116, P1', P2'; R-72 form; applied per
  R-369). The model's "sample" is one IC sample run at the probe's real trajectory count, `2(E_probe + 1)`
  trajectories, shadows and ensemble
  (`principia_quality_device_note.md`, the device-characterisation section's step 2, "PROBE"; REQ-PERF-051), so:
  - the cost per trajectory-step is `c_traj = c_model / (2(E_probe + 1))`, `c_model` the model's
    cost-per-dt-per-sample and `E_probe` the `E` it was measured at;
  - the export's trajectory-steps are `S_export × 2(E_export + 1) × steps`, where `S_export` is the export tier's
    samples per quad times the quads its refinement puts over the frame (Part 4: the export tier's extra samples per
    quad and depth are "priced into the estimate"), `E_export` the export's full `E` (no motion gating in export,
    scheduler Part 9), and `steps` the march's physical time over `dt` (Part 2's class: (a) one march to the
    segment's end, (b) `0 → t_frame` per frame, (c) none);
  - the sustained derating: the estimate divides by the same thermal headroom the solve applies to the probe (step 4,
    "apply thermal headroom ... of raw cold measurement"; its value is REQ-PERF-088's calibration, no new number), since
    a multi-minute export is the sustained load that headroom is for;
  - **estimate = `S_export × 2(E_export + 1) × steps × c_traj / headroom`**, one value, with the model's provenance key.
  *Was:* "**An estimate carries its basis.** A segment's cost is the samples it marches times the
  physical time each is marched (the export's pixels times its copies, `E + 1`, times the march's length); the
  throughput is the same quantity per wall second, measured from this machine's frame records
  (`principia_dd_telemetry_and_tiers.md` §5). The estimate is shown as a range, the cost over the fastest and over the
  slowest throughput measured. With no measurement it reads "no measurement on this machine yet", never a number. The
  proof strip's frames are timed and enter the measurement, so they calibrate it."
- Then Part 4's parameters: size, fps, quality tier, format, and Plain or styled (R-489). An fps that would give two
  keyframes one frame (export contract Part 3) is refused inline beside the fps field, naming both keyframes ("at 12
  fps keyframes 2 and 3 share frame 7"), never over the figure (§G1), and the spec keeps its fps (fix round 9 of PR
  #286, gui review 5631987230, G2 (b); applied per R-369). **Start** runs the export as
  Part 4's background job, with progress, ETA, the last frame's thumbnail and Cancel.
- **What travels.** Each PNG frame carries its own record, that frame's `SimConfig` with its link values (R-520 F5),
  and the `.principia-seq.json` (§3.5) is written beside every export, whatever its format. How a GIF, MP4 or WebM
  carries the spec in itself is the video carrier's design (`video_carrier_test.md`, gap sweep PR 2c, ported
  separately); this design adds only the file beside it.

## 4. Fit with rulings

- Implements export contract Parts 2, 3, 4 and 6 and REQ-TOOL-074 to REQ-TOOL-076 as written, with REQ-TOOL-076
  amended by R-576 (easing per segment). R-520 appears as the segment's link rule (§3.4).
- Amends `principia_gui_state_contract.md` §2's window names (adds Sequence) and the GUI spec (§G9a).
- Proof frames are exported frames, not scrub over stored data (Part 1, R-66).
- Undo: R-52 holds the contract's one history; the draft's own history for the window is RQ-559.
- R-520 F3 holds: live playback, the preview included, cuts link parameters and dissolves; the export eases them.
- R-69 and R-101 hold: the preview edits no `SimConfig` or `RenderState` field and sends no `SetField` (§3.6).

## 5. Decided

The human, 21 Oct 2026 (`keyframes_reel.md`, "Decided", verbatim):

> - **Easing is set per segment, between keyframes** ("The easing functions is set between key frames of the
>   animation"). Amends REQ-TOOL-076. Open question 1 is closed.
> - **What clicking a reel clip opens is not decided** ("We need to deisgn"): designed separately, `reel_click.md`.

The draft's open questions 3 (the reel's storage) and 4 (keyframe undo) were not ruled: 3 is decided per R-369 in
`principia_spotlight_reel.md` §3.1, and 4 is RQ-559.

## 6. Tests

Each with what makes it fail.

- **keyframe_is_whole**: capture, edit every field of `SimConfig` and `RenderState`, Go to: the state equals the
  captured one bitwise, and Go to adds exactly one history entry. *Fails on* any field missed.
- **lanes_light_on_change**: keyframes differing only in a stain uniform light only Stain, and the segment is (c).
  *Fails if* Plane lights. Control: a tilt lights Plane and makes the segment (b); a chart change shows a cut mark.
- **lane_table_exhaustive**: every field of `SimConfig` and `RenderState` has exactly one lane, `z₀`, `q₁` and `q₂`
  through R-92's split. *Fails* to compile when a field is added without one.
- **lanes_follow_r92**: a pan-only pair lights Navigate and not Plane, its sim keys equal; an off-plane move and a tilt
  light Plane; across random pairs, a pair lighting only Navigate has equal sim keys. *Fails if* a pan-only pair lights
  Plane, an off-plane move lights only Navigate, or the split is computed apart from the sim key's function (fix round 6
  of PR #286, physics review 5631678451, P3).
- **segment_easing**: two segments with different curves: each segment's frames follow its own curve. *Fails if* one
  curve is applied to both.
- **seq_file_roundtrip**: save then open is the identity, bitwise. Control: a file with one changed link entry hash
  opens as "cannot recreate", naming the link, and a reader that substitutes the build's link fails.
- **preview_cuts_links**: a segment interpolating a link parameter previews as a cut at its second keyframe and is
  flagged in the planner; the preview sends no `SetField`, and when it stops or ends the state is bitwise the state
  before it and the undo depth unchanged; a pan during the preview stops it and pans the user's view; a cursor moved
  over the figure does not stop it; Esc stops it and leaves the focus. *Fails* if a frame is sent as an edit, if the
  view stays at the preview's frame, if the pan lands on the preview's state, or if a hover or an Esc does more.
- **preview_holds_explore**: during a preview of another stain and chart, the header strip shows the preview badge, the
  Legend shows no keys, the decoder badge is absent, `t` reads "—", no hover is sent and the Manifold view shows the
  user's values. *Fails if* the preview badge is missing, the Legend or the decoder badge describes the user's view, `t`
  shows a number, a hover is sent or its sample drawn, or the Manifold view shows the preview frame's values.
- **proof_equals_export**: a proof frame is bitwise the exported frame at the same time, size and tier. *Fails if* it
  came from the live loop.
- **planner_no_guess**: with no frame records the estimate reads "no measurement on this machine yet" and shows no
  number. *Fails if* a number appears. Control: with records, the range's ends are the cost over the fastest and the
  slowest measured throughput.
- **export_writes_seq**: every export, PNG frames and GIF, has its `.principia-seq.json` beside it, equal to the saved
  spec bitwise. *Fails if* one is missing.
