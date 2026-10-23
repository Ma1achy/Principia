# TASK-M8-79 — The spotlight reel's machinery, natively: the reel builder and its manifest, the film played without compute, the takeover that keeps playing, the Spotlights window (R-578)

- **Milestone:** M8
- **Closes:** REQ-TOOL-084, REQ-TOOL-198, REQ-TOOL-199, REQ-GUI-382, REQ-GUI-383, REQ-GUI-384, REQ-GUI-385
- **Depends on:** TASK-M8-78, TASK-M5-26
- **Needs (earlier milestones):** REQ-RENDER-045
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~500 lines, the tests included

## Goal
`cargo xtask reel` renders the curated specs in `reel/specs/` once per release through the blocking exporter into one GIF film and a manifest with an entry for every film frame, refusing a spec whose link entry hashes do not match. Windows › Spotlights… lists the segments with their provenance and plays the film in the Explore canvas, computing nothing, the badge "preview" and the provenance in the header strip. Any interaction takes over at the displayed frame, loading its state bitwise as one undoable entry, and keeps playing live; the film frame is the backdrop until the live baseline cover is complete (R-578).

## References
- `docs/design/principia_spotlight_reel.md` § "3.1 The reel builder and its manifest"
- `docs/design/principia_spotlight_reel.md` § "3.2 Playing the film"
- `docs/design/principia_spotlight_reel.md` § "3.3 The takeover"
- `docs/design/principia_spotlight_reel.md` § "3.4 The native presentation: the Spotlights window"
- `docs/design/principia_spotlight_reel.md` § "3.5 What is v2 web's"
- `docs/gui/principia_render_gui_spec.md` § "Spotlights — the reel, natively (R-578)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 7 — The spotlight reel (attract mode)"
- `docs/contracts/principia_caching_contract.md` § "Part 5 — The stale backdrop: blur means loading"
- `decisions.md` § "R-578 — The spotlight reel passes for Explore: a v2 browser product feature whose machinery is built and tested natively, its native presentation rough; the film rendered once per release, offline, by the blocking exporter, with a manifest for every frame, and only played by the app; any interaction takes over live at that exact frame and keeps playing *(amends REQ-TOOL-084; queues RQ-561)*"
- `decisions.md` § "R-571 — The browser: the native app first, then v1 web, the whole native app compiled to run in the browser on the GitHub Pages site, then v2 web, a separate product designed later *(replaces the browser item of the gap sweep's `INSTRUCTION_now.md` backlog; queues RQ-550 and RQ-551)*"
- `decisions.md` § "R-521 — Each recorded link carries its own registry-entry hash beside the build hash and the schema version: a mismatch refuses, naming exactly that link, and a match recreates across an unrelated schema change *(closes RQ-385; amends R-483)*"
- `decisions.md` § "R-523 — Badges go in the chrome or the status line, never over the figure: the decoder badge sits in the figure's header strip, above the plot *(closes RQ-387; amends R-479)*"
- `decisions.md` § "R-484 — The round-trip rule: importing any export is the identity on `SimConfig` and `RenderState`, bitwise in the canonical serialisation, on every path but the image's low-bit embedding, which drops whole tiers from the bottom (custom source, then colour) and never tier 1, physics, writing nothing rather than a partial record; the degrading is reported by the writer, the record and the reader, and badged; every field is assigned a tier in one table, exhaustive at compile time; four tests keep it from drifting; splitting tier 2 is queued (RQ-386)"
- `decisions.md` § "R-155 — One GIF encoder for both builds *(closes RQ-125)*"

## Deliverables
- `xtask/src/reel.rs` — `cargo xtask reel` builds the film (GIF, the `gif` crate) and `manifest.json` from `reel/specs/`, refusing a spec whose link entry hashes do not match; moved here from TASK-M8-31 (R-578).
- `reel/specs/` with at least the fixture specs the tests use; the film and manifest are build outputs, not checked in.
- `crates/gui/src/windows/spotlights.rs` — the Spotlights window; the film player into the compositor's snapshot layer; the takeover (manifest lookup, recompute and hash check, one history entry, the live sequence player of TASK-M8-78), the backdrop until the baseline cover, the header-strip badge.
- Tests: `reel_manifest`, `reel_build_refuses`, `reel_plays_without_compute`, `spotlights_window`, `takeover_identity`, `no_blank_handover`, `preview_marker`.

## Acceptance tests
- `cargo test -p xtask reel_manifest` — a reel built from three fixture specs has one manifest entry per film frame, each entry's segment, frame within the spec and state hash equal to those the exporter rendered that frame from, and each segment's provenance (chart, horizon, integrator, build, date) taken from its spec (control: a manifest shifted by one frame at a segment boundary fails) (REQ-TOOL-084).
- `cargo test -p xtask reel_build_refuses` — a fixture spec with one link entry hash changed makes cargo xtask reel exit non-zero, its message naming the spec and the link, and leaves no film and no manifest (control: the same specs with matching hashes build a film and a manifest) (REQ-TOOL-198).
- `cargo test -p gui reel_plays_without_compute` — playing 120 frames of a fixture film counts 0 integrations, 0 march dispatches and 0 SetFields, and Stop leaves SimConfig, RenderState and the undo depth bitwise as before (control: a player that renders each frame's spec live fails) (REQ-TOOL-199).
- `cargo test -p gui spotlights_window` — with a fixture reel of three segments the window lists three rows with each spec's provenance; Play reel with segment 2 selected shows segment 2's first film frame in the Explore canvas; Open the sequence puts segment 2's spec into the Sequence window bitwise; with no reel it reads "No reel in this build" and Play reel is disabled; a launch shows the live view, not the film (controls: a Play that starts at frame 0 with segment 2 selected, and a launch that opens on the reel, fail) (REQ-GUI-382).
- `cargo test -p gui takeover_identity` — a click at film frame i of segment k loads SimConfig and RenderState whose canonical hash equals manifest entry i's, with that frame's playhead, the transport playing and the following frames following the spec live; one undo restores the view from before the reel; the click neither pans nor locks; a manifest entry with a wrong hash logs one error naming segment k and frame i; a spec played to its end with an advance policy keeps playing and with hold pauses (controls: a manifest off by one frame at a segment boundary, a takeover that pauses, and a takeover that adds one history entry per field, fail) (REQ-GUI-383).
- `cargo test -p gui no_blank_handover` — with the live render held back by a test clock, every displayed frame from the click to the complete baseline cover has no pixel of the canvas background, and the first frame after it has no backdrop (control: a handover that clears to the background before the first live cover fails) (REQ-GUI-384).
- `cargo test -p gui preview_marker` — the badge is present on every film frame of a fixture reel, inside the header strip's rect and outside the plot's, and absent from the first frame after the backdrop goes; the provenance shown changes at each segment boundary (controls: a film frame without the badge, and a badge drawn over the plot, fail) (REQ-GUI-385).

## Notes
- R-578 (21 Oct 2026, gap sweep PR 2b), from `reel_click.md` and `keyframes_reel.md`'s reel section; the presentation is rough natively and v2 web's otherwise (R-571).
- **RQ-561 (a takeover inside a segment the film interpolates) is open:** meanwhile its option (a), the takeover frame exact and live continuing under R-520 F3; nothing waits.
- GIF as the native film format, the per-frame manifest with its state hash, what counts as an interaction, the spec's end and the Spotlights window are applied per R-369 (R-578).
