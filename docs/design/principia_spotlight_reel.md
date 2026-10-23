# Principia: the spotlight reel — a film that passes for Explore

*Adopted by R-578 (gap sweep PR 2b, 21 Oct 2026), from the human's designs `reel_click.md` and the reel half of
`keyframes_reel.md` (`docs/incoming/2026-10-21_gap_sweep_keyframes_reel/designs/`), their "Decided" sections his words.
The specs it plays are made in the Sequence window (`principia_sequence.md`, R-577). Queued from it: RQ-561 (a takeover
inside a segment the film interpolates and live playback cuts).*

**Scope (the human, 21 Oct 2026).** The reel is a feature of the **v2 browser product**, not of the native app or its
first browser port: "We could build all the machinery for it and test it natively but yeah it can be kinda rough around
the edges ig design wise." So the machinery is built and tested natively, and the native presentation is rough
(R-571: the reel's machinery native, its presentation v2 web's).

## 1. What exists

The reel is a pre-rendered film of curated views, made by the blocking exporter from a list of saved specs. It covers
cold start, when nothing can be computed live yet, and is the gallery's ambient mode. "Clicking through loads the spec
live — the reel is a menu of shareable objects, not just a film" (`principia_export_animation_contract.md` Part 7;
REQ-TOOL-084). The caching contract names its handoff: a video frame has nothing behind it to live-render, so the
snapshot backdrop applies (`principia_caching_contract.md` Part 5, "Fallback — the snapshot"). Nothing says what the
reel looks like, how a frame maps back to its spec, or what a click loads.

## 2. The gap

No manifest from film time to spec, no rule for what a click loads or what happens after it, no handover from the film
to the live render, and no native way to build or try any of it.

## 3. The design

**The reel plays inside the Explore view and looks like Explore running.** The human: "I feel like the showreel should
almost fake being the explore view." **Any interaction takes over live at that exact frame and keeps playing.**

### 3.1 The reel builder and its manifest

- **The curated list** is `reel/specs/NN_name.principia-seq.json`, each a spec saved from the Sequence window
  (`principia_sequence.md` §3.5), played in `NN` order.
- **The film is not computed in the app.** `cargo xtask reel` renders it once per release, offline, through the
  blocking exporter (Part 4), each spec at its own fps, at the size and quality tier given to the command and recorded
  in the manifest. The app only plays it; the engine starts computing at the takeover.
- **The film's format natively is GIF**, encoded and decoded by the one `gif` crate (R-155; R-131 lists GIF among the
  native encoders), labelled as a preview. The draft's open question 3, (i), decided per R-369: it needs no new decoder,
  and a click loads the exact view whatever the film's colours. Which format v2 web ships is v2 web's.
- **The manifest** (`manifest.json`, beside the film) gives, for each segment, its spec's file name and hash and its
  first and last film frame; and for **every film frame**, its segment, its frame within the spec, its wall time within
  the spec, and the hash of the `SimConfig` and `RenderState` the exporter rendered it from, in their canonical
  serialisation (R-309). The takeover maps a frame by its index, never by a time read back from the film's delays.
- **The build refuses** a spec whose link entry hashes do not match the build's (R-521; "never substitute",
  `principia_dd_image_embedding.md` §6): `cargo xtask reel` stops before rendering anything, exits non-zero, and names
  the spec and the link. The reel is built per release, so the live view can always recreate what the film showed.
- The specs are checked in; the film and the manifest are build outputs, shipped with the release and not checked in
  (applied per R-369). A build with no reel says so in the Spotlights window (§3.4).

### 3.2 Playing the film

- The film is drawn in the Explore canvas, through the compositor's snapshot layer (caching Part 5), frame by frame at
  its own delays, looping until it is stopped or taken over. While it plays the live view beneath is paused and hidden,
  and nothing is integrated or dispatched for the film: no `SetField` is sent, so stopping it leaves the view exactly as
  it was.
- **The one tell.** Until the handover a small, quiet badge reads **preview**, so a screenshot of the film is never
  taken for a computed result. It sits in the figure's header strip, as every badge that says what the figure is does
  (R-523), never over the plot. Beside it the header strip shows the segment's provenance, cycling with the segments:
  chart, horizon, integrator, build and date.

### 3.3 The takeover

- **Any interaction takes over**: a pointer press, drag or scroll on the figure, or any key with the focus in the
  Figure scope (a modifier pressed alone excepted). The interaction is consumed by the takeover; it does not also pan,
  zoom or lock (applied per R-369).
- **At that exact frame.** The displayed frame's index gives, through the manifest, its segment, its spec and its frame
  within the spec. The app recomputes that frame's state from the spec as the exporter did (Part 3's interpolation,
  TASK-M8-31) and loads it live: chart, view, stain, links, playhead, every field of `SimConfig` and `RenderState`,
  bitwise the exporter's. Import is the identity (R-484). If the recomputed state's hash is not the manifest's, live and
  film disagree, which is a bug: it is logged as an error (source `app`) naming the segment and the frame, and the
  takeover goes ahead with the recomputed state.
- **It keeps playing** whatever the clip was doing, now live (the human: "Yeah keep playing"): the transport plays, and
  the spec is loaded into the live sequence player (`principia_sequence.md` §3.6) at that frame's wall time, so any
  keyframed motion it was in the middle of continues live. That playback is playback: its frames enter no undo (R-101).
  At the spec's end the player stops at its last keyframe, and the transport plays on to the horizon if that keyframe's
  playhead policy is advance, and is paused otherwise (applied per R-369).
- **Inside a segment the film interpolates and live playback cuts** (link parameters, R-520, and dissolves), live
  cannot continue what the film showed. Which way that goes is RQ-561. Meanwhile, its option (a): the takeover frame is
  exact, as above, and live playback continues under R-520 F3, cutting at the segment's second keyframe; the difference
  after the takeover frame is expected and not reported as a disagreement.
- **No blank, no jump.** The film frame stays as the backdrop, the snapshot of caching Part 5 (blurred, under its
  coherence discipline), until the live identity's baseline cover is complete (caching Part 4); then it and the
  preview badge go. From the interaction to then, every displayed frame is the film backdrop or a live render over it,
  never the canvas background.
- **Nothing is lost.** The takeover is one undoable history entry: one undo returns the view to what it was before the
  reel played. (At cold start there is no previous view; cold start is v2 web's, §3.5.)

### 3.4 The native presentation: the Spotlights window

A plain window, no polish. **Spotlights** opens from Windows › Spotlights…, a window under §G5's rules (R-461). It lists
the reel's segments in order, each with its provenance (chart, horizon, integrator, build, date), and offers **Play
reel** (from the selected segment's first frame, or the first segment's), **Stop**, and **Open the sequence**, which
loads the selected segment's whole spec into the Sequence window (`principia_sequence.md`). With no reel in the build
it reads "No reel in this build" and Play reel is disabled.

### 3.5 What is v2 web's

The presentation: playing the reel at cold start (the app opening on it, the window `principia_lowering_contract.md`'s
startup budget names), the gallery of poster tiles, and the film wearing the same header, legend and overlays a live
view would have at that frame. Natively the film plays only from the Spotlights window, in the Explore canvas, with
the preview badge and the provenance in the header strip (R-571; the human's scope, above; applied per R-369).

## 4. Fit with rulings

- Implements export contract Part 7 and REQ-TOOL-084, which R-578 amends: the click is any interaction, at that exact
  frame, and playback continues.
- R-571: the machinery here is native and tested natively; §3.5 is v2 web's.
- R-521 and image embedding §6: the build refuses, never substitutes. R-484: a takeover is the identity.
- R-523 and §G1: the preview badge and the provenance sit in the header strip, never over the figure.
- Caching Part 5's snapshot fallback is the takeover's backdrop, as it already names it.
- R-520 F3 against "live and film disagreeing is a bug": RQ-561.
- Amends `principia_gui_state_contract.md` §2's window names (adds Spotlights) and the GUI spec (§G9a).

## 5. Decided

The human, 21 Oct 2026 (`reel_click.md`, "Decided", verbatim):

> - **After takeover it keeps playing** whatever the clip was doing, now live ("Yeah keep playing").
> - **The film is not computed in the app.** It is rendered once per release, offline, by the blocking exporter, and
>   shipped as a video file. The app only plays it; the engine starts computing at the takeover.

## 6. Tests

Each with what makes it fail.

- **reel_manifest**: a three-spec reel from fixtures: the manifest has one entry per film frame, and each entry's
  segment, frame within the spec and state hash are those the exporter rendered. *Fails on* an off-by-one in the
  segment manifest.
- **reel_build_refuses**: a spec with one changed link entry hash stops `cargo xtask reel`, naming it, and nothing is
  written. *Fails if* it renders. Control: matching hashes build.
- **reel_plays_without_compute**: playing the film integrates nothing and dispatches no march, and stopping it leaves
  the view bitwise as it was. *Fails if* the player renders the spec live.
- **takeover_identity**: an interaction at film frame `i` loads exactly the spec, state and playhead of that frame, and
  playing continues. *Fails on* an off-by-one in the segment manifest, or a takeover that pauses.
- **no_blank_handover**: from the interaction until live has resolved, every displayed frame is the film backdrop or a
  live render over it. *Fails if* any frame shows nothing.
- **preview_marker**: the badge is present on every film frame and gone after the handover. *Fails if* a film frame
  lacks it.
