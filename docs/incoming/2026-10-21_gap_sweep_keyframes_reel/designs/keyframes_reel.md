# Keyframed view animations and the spotlight reel: the native UI

*Draft proposal against main @45caf8be.*

## 1. What exists today

The export contract defines three segment kinds and their costs (`principia_export_animation_contract.md` Part 2,
:28-37), the keyframe spec `{keyframes: [(t_wall, ViewState, RenderState, playhead policy)], easing, fps, duration}`
and per-field interpolation (Part 3, :40-64), link parameters eased in a blocking export only, flagged (R-520,
:51-64), the blocking export job (Part 4, :68-79), and a reel of curated specs whose segments load live when clicked
(Part 7, :107-111). REQ-TOOL-074…076 and 084 require them; TASK-M8-31 builds the engine side and `cargo xtask reel`.
The GUI has only Record a time sweep (`principia_render_gui_spec.md` §G9, :1229-1233), and the window list has no
animation window (`principia_gui_state_contract.md` §2, :87).

## 2. The gap

No screen to make keyframes, see a timeline, choose interpolation, price or preview a sequence, or export it; no
gallery for the reel; no rule for what a reel click loads. Separately, low-bit pxpack does not survive GIF, MP4 or WebM
(embedding §7, :363-373); a carrier test is running, so that is left as a slot.

## 3. Proposed design

### The Sequence window

Docked along the bottom of Explore (a splitter, R-462).

- **Add keyframe (K)** captures the whole `SimConfig` and `RenderState` at the timeline cursor; a keyframe is always
  whole, as the spec requires.
- **The timeline**: wall time across, keyframes as diamonds. Read-only **lanes** beneath (Camera, Plane, Chart, Links,
  Playhead, Stain, Print, Lines) light where fields differ between neighbours, so the user sees why a segment costs
  what it does.
- **Segments are coloured by class**: (a) time playback, (b) view animation, (c) recolour only, with a frame count;
  chart and link-id changes show a cut mark.
- **A keyframe**: show values, **Go to** (one undo entry), **Update** from the view, drag in time, and set its playhead
  policy (hold / advance with wall time / set to a value).
- **A segment**: easing; cut or dissolve (with length) at a chart change; at a link-parameter change, **cut** or
  **interpolate (export only)**, showing R-520's "every frame a different decoder". The fixed rules of REQ-TOOL-075
  (log zoom, Grassmannian basis, componentwise `z₀`) are shown, not offered.
- **Undo** is the window's own (Q4). **Saved** as `.principia-seq.json`, the spec object in canonical serialisation (R-309) with build hash, schema
  version and link entry hashes (R-521).

### Preview

**Play preview** runs the sequence through the live frame loop, link parameters and dissolves as cuts (R-520), under a
banner "Preview: not converged; cuts where the export interpolates". **Proof strip** renders one small frame per
keyframe and per segment midpoint through the blocking exporter at the lowest tier: exact frames before paying for all.

### The planner and export

**Export…** opens the planner first. One row per segment: class, frames, integration cost ((a) one march to the
segment's end; (b) `0 → t_frame` per frame; (c) none), flags (link parameters interpolated; dissolve is two renders),
and estimated time.

- **An estimate carries its basis**: cost over this machine's measured throughput from the profiler's frame records,
  shown as a range; with no measurement it reads "no measurement on this machine yet", never a number. Proof frames
  calibrate it.
- Then Part 4's parameters (size, fps, quality tier, format, Plain or styled); **Start** runs in the background with
  progress, ETA, last-frame thumbnail, Cancel.
- **What travels**: each PNG frame carries its own pxpack (R-520); the `.principia-seq.json` is written beside every
  export.
- **Slot — how a video carries its spec: open, pending the carrier test.** Meanwhile the dialog says a GIF/MP4/WebM
  alone does not carry it.

### The reel and Spotlights

- **The reel** is a curated list `reel/specs/NN_name.principia-seq.json` plus a manifest of segment start and end
  times, rendered by `cargo xtask reel` through the blocking exporter at release.
- **Spotlights** (a window, and the cold-start screen) plays the reel as ambient background with a caption cycling
  each segment's provenance (chart, horizon, integrator, build, date), beside a gallery of poster tiles.
- **A click** maps the clicked time through the manifest to a segment and frame, recomputes that frame's state from
  the spec as the exporter did, and loads it live, paused at that playhead (import is the identity, R-484). **Open
  the sequence** loads the whole spec into Sequence.
- **The reel build refuses** a spec whose link entry hashes do not match the build ("Never substitute", embedding §6).

## 4. Fit with existing rulings

- Implements Parts 2, 3, 7 and REQ-TOOL-074…076, 084 as written; R-520 appears as the segment's link choice.
- **Amends** state contract §2's window names (adds Sequence, Spotlights) and the GUI spec (a new §G9a).
- Proof frames are exported frames, not scrub over stored data (Part 1, R-66).
- **Amends REQ-TOOL-076** only if easing goes per segment (Q1).

## 5. Open questions

1. **Easing**: (i) per segment, amending REQ-TOOL-076; (ii) one per sequence, as written; (iii) per keyframe.
   *Recommend (i).*
2. **What a reel click loads**: (i) the clicked frame, live and paused; (ii) the whole sequence; (iii) the segment's
   first keyframe. *Recommend (i), (ii) one click away.*
3. **Reel storage in the native app**: (i) GIF, decoded by the existing `gif` crate, labelled "ambient preview";
   (ii) MP4/WebM, adding a video decoder; (iii) poster frames only. *Recommend (i)*: a click loads the exact view.
4. **Keyframe undo**: (i) own history; (ii) the contract's; (iii) none. *Recommend (i).*

## 6. Tests (each with what makes it fail)

- **lanes_light_on_change**: keyframes differing only in a stain uniform light only Stain; segment is (c). *Fails if*
  Plane lights. Control: a tilt lights Plane and makes it (b).
- **keyframe_is_whole**: capture, edit every field, Go to: bitwise equal. *Fails on* any field missed.
- **planner_no_guess**: no frame records → "no measurement". *Fails if* a number appears.
- **preview_cuts_links**: an interpolating link segment previews as a cut and exports flagged. *Fails* otherwise.
- **proof_equals_export**: a proof frame is bitwise the exported frame. *Fails if* it came from the live loop.
- **reel_click_identity**: segment k at time τ loads that frame's record. Control: an off-by-one manifest fails it.
- **reel_build_refuses**: a changed link entry hash stops `xtask reel`. *Fails if* it renders.


## Decided (Malachy, 21 Oct 2026)

- **Easing is set per segment, between keyframes** ("The easing functions is set between key frames of the
  animation"). Amends REQ-TOOL-076. Open question 1 is closed.
- **What clicking a reel clip opens is not decided** ("We need to deisgn"): designed separately, `reel_click.md`.
