# The spotlight reel: a film that passes for Explore

**What exists.** The spotlight reel is a pre-rendered film of curated views, made by the blocking exporter from a
list of saved specs. It plays at cold start, when nothing can be computed live yet, and as the gallery's ambient mode.
"Clicking through loads the spec live — the reel is a menu of shareable objects, not just a film" (export contract
Part 7; REQ-TOOL-084). Nothing says what that looks like.

**Scope (Malachy, 21 Oct).** This is a feature of the **v2 browser product**, not of the native app or its first
browser port. "We could build all the machinery for it and test it natively but yeah it can be kinda rough around the
edges ig design wise." So: build and test the machinery natively; keep the native presentation rough.

## Proposal

**The reel plays inside the Explore view and looks like Explore running.** "I feel like the showreel should almost
fake being the explore view."

1. **Same canvas, same chrome.** The film is drawn in the Explore canvas, with the same header, legend and
   overlays a live view would have at that frame, so a first-time visitor sees what looks like the app already
   exploring.
2. **Any interaction takes over at that exact frame.** Click, drag, scroll or a key: the frame's spec is loaded live
   (chart, view, stain, playhead, and any keyframed motion it was in the middle of), and the live engine continues
   from there. The film frame stays as the backdrop until the live render has resolved over it, coarse to fine, so
   the handover has no blank or jump.
3. **The one tell.** Until the handover, a small, quiet marker reads "preview". It's kept so a screenshot of the film
   is never mistaken for a computed result; its styling can be minimal.
4. **Nothing is lost.** At cold start there is no previous view. Anywhere else, the takeover is one undoable step.
5. **Exact by construction.** The reel is built per release, and the build refuses any spec whose link hashes don't
   match, so the live view always recreates what the film showed. Live and film disagreeing is a bug, reported.

## What to build natively

- The reel builder: a curated spec list goes through the blocking exporter to a film plus a manifest of segment
  times and spec ids.
- The takeover: frame time → segment → spec → live load at that playhead, with the film frame as backdrop.
- A plain native window that plays the reel and takes over on click. No polish.

## Decided

- **After takeover it keeps playing** whatever the clip was doing, now live ("Yeah keep playing").
- **The film is not computed in the app.** It is rendered once per release, offline, by the blocking exporter, and
  shipped as a video file. The app only plays it; the engine starts computing at the takeover.

## Tests

- **takeover_identity**: an interaction at film time τ loads exactly the spec and playhead of that frame. *Fails on*
  an off-by-one in the segment manifest.
- **no_blank_handover**: from the interaction until live has resolved, every displayed frame is either the film
  backdrop or a live render over it. *Fails if* any frame shows nothing.
- **preview_marker**: the marker is present on every film frame and gone after handover. *Fails if* a film frame
  lacks it.
