# Principia — GUI & state-interface contract

*The load-bearing wall for "tear out the dev GUI later." Everything customisable, GUI pluggable, dev tooling swappable for a polished UI without touching the sim. The mechanism is one rule applied three ways: the GUI is a pure reader/emitter of a typed state schema; the *fragment* shader registry is the scanned filesystem (compute occupants are Rust build variants — §3); occupants are typed only by output signature. Corrects the colouring drill-down's implied "shape dictates channel" claim.*

---

## 1. The one-way dependency rule

> **The engine never imports the GUI. The dependency arrow points one way: GUI → state → engine.**

The dev GUI is allowed to be ugly and throwaway *precisely because* it is a thin renderer over the same state objects the engine consumes. The test is brutal and concrete:

> **You could drive the entire engine from a JSON file with no GUI attached. The GUI is just one possible editor of that JSON.**

**Under the substrate decision (`principia_spike_brief.md`: whole engine in Rust → wasm) this stops being an import-lint and becomes structural.** The GUI-facing surface — the typed `SimConfig`/`RenderState`/`ViewUI` schema, `set_field`, and the state snapshot — is defined **once**, in Rust, in the engine crate. Two bindings call that one surface, and only the firewall's *enforcement* differs between them:
>
> - **The egui dev binding (now; the native app's and v1 web's, R-571):** a Rust immediate-mode **debug menu** (toggled with `F3`), compiled into the same wasm binary and calling the surface *natively* — a typed `SetField` enum + snapshot, no marshalling. The firewall is a fact of the **crate graph**: the engine crate marks *only* the state surface `pub` (scheduler, cache, kernel, payload buffers stay crate-private), so the gui crate depends on the engine crate yet *cannot reach past the surface* — a stray "egui callback into a sim object" is unexpressible because the internals aren't visible. Language-boundary strength, in-process.
> - **The TS production binding (v2 web's, if there is one; R-571, *was:* "(later)"):** a thin TS GUI (React/HTML/whatever) across the wasm↔JS membrane, calling the *same* surface through a wasm-bindgen layer. The firewall is a fact of the **binary**: no shared linker, so the engine *cannot* import the GUI.
>
> **The dev binding earns no privilege for being in-process.** egui reads a snapshot and emits `SetField` exactly as a TS GUI would (R-571: v2 web's, if there ever is one; *was:* "exactly as the TS GUI will"), and must *not* mutate engine state through a `&mut` borrow even though the same binary makes that physically possible. Exercising `set_field`/snapshot from day one is precisely what would make a TS swap, v2 web's if it ever comes (R-571; *was:* "the eventual TS swap"), a drop-in rather than a cliff — the marshalled path is never left untested.

The boundary must stay a **data** boundary, not an **object** one — under *both* bindings; only the seductive failure differs. wasm-bindgen makes it easy to hand JS a **live handle to a Rust struct with callable methods** (the moment a TS GUI, v2 web's if there ever is one (R-571; *was:* "the TS GUI"), calls `engine.scheduler.request_refine(quad)` the firewall is decorative); in-process, the egui equivalent is holding a `&mut` into engine state. Both are barred — the rule is identical:
- **In:** `set_field(path, value)` — plain serialised data. **Out:** a state snapshot — plain serialised data. The TS side holds **no Rust handles**, only the last snapshot it was handed. That is the whole of §1's subscribe/emit rule, now with teeth.
- **The canvas is the one sanctioned exception** — and its handling is itself binding-specific. Under the TS binding the surface is browser-owned, so the GUI transfers the `OffscreenCanvas` to the engine's worker **once at startup and never again**, and the wasm engine drives `wgpu` directly against it (handle crosses once; no state crosses with it). Under the egui binding there is nothing to transfer: egui-wgpu shares the engine's `wgpu` context and paints the debug overlay onto the same surface in-process. Either way, no *state* rides on the canvas. (v1 web, the whole native app in the browser, runs the egui binding (R-571); where egui runs there, and whether a canvas is transferred, is RQ-551.)
- **The snapshot carries GUI-*sized* state only** (view state, current tier, a few scalars — what the panels display), **never engine-sized** state (the payload, the quad tree, the reductions). Those stay wasm-side and are only ever *summarised* across. This is the identical law as the GPU→CPU membrane (systems-architecture §3): big data never crosses a membrane — and there are now **two** membranes (wasm↔JS and GPU↔CPU), same rule on both. The engine posts the snapshot throttled to ~10 Hz (caching contract Part 6a, R-94), and the GUI (egui) redraws at frame rate from the latest one; each snapshot is a serialise-copy-deserialise, cheap if it is GUI-sized and ruinous if someone reaches for the tree.

No GUI element ever holds sim logic, caches sim data, or computes anything the engine needs; it reads a snapshot and emits field edits, nothing more.

**The link catalogue and evaluator (R-480, F2; decided per R-369).** The Decoder window's previews evaluate the
generated link functions, the same code the kernel runs (`principia_render_gui_spec.md` §G7a, P4). The surface offers
them as a pure function, `engine::contract::links`: the registry's catalogue (the entries onto each block's codomain,
their display names, their tunable parameters with defaults and ranges) and an evaluator of an entry's forward,
inverse and log-det at f64 for given parameter values. It is data in and data out, holds no engine state and reads none,
so it is no handle into the engine; the mock calls the same function, and the TS binding reaches it as a plain call
across the membrane.

**The mock engine and the conformance suite (R-390).** The dev GUI is built first against a mock engine: a test double of this surface, in the gui crate, always compiled, the crate's `mock` feature choosing only that `main` runs on it (RQ-252 as amended per R-369), which serves plausible snapshots, applies `SetField` with undo and redo (R-69), emits events and runs a fake clock so the playhead moves. One conformance suite, defined beside the surface in the engine crate, runs every case against both the mock and the real engine, and both pass it; a change to the surface goes through review and re-runs it for both. The mock earns no privilege: the GUI reaches it only through this surface, as it reaches the real engine.

---

## 2. The editable state is the entire coupling surface

Everything customisable = every knob is a typed field on the state, already split by the two keys:

```
SimConfig    (sim key)     chart id + params · z₀,q₁,q₂ · slice values · lock flag · z_locked anchor · δ excursion
                          (lock is chart construction, R-69) · link ids ·
                          integrator occupant · kernel variant (physics, or colour_composition Appendix A's bring-up mode, "the only sim-key item", RQ-221) · compute fast-math setting (off by default, R-297) · T/dt/thresholds/eps · collision radius r_coll · quality settings (§6 — NB not every quality field is sim-key: `render_scale`/`lock_to_native`/`MAX_REL_DEPTH`/`checkerboard_mode` invalidate nothing; the GUI's re-integrate warning keys per-FIELD off the caching blast-radius table, not off the struct's home)
RenderState (render key)  the stain graph (nodes · wires · per-node params, §5) · overlay set ·
                          palette/compaction params · playhead t · the Print graph (print, R-491)
ViewUI       (pure UI)    backdrop ref · debug category visibility · keyboard focus scope · selection ·
                          kept orbits · inspector t_cursor · open windows · linked views for side by side (v2, R-106) ·
                          playback transport (play/pause/speed/loop — not undoable, not on the sim key, R-96; the
                          GUI's clock reads it and advances RenderState's playhead t each frame through a SetField
                          marked "no history", R-101) · the mode switch's mode, Explore or Stain
                          (render_gui_spec §G2; RQ-249)
                          — never read by the engine
```

The GUI requirement adds **no new state** — it says *expose all of it*. Consequences that come free from the key split:

- The GUI knows, per control, whether editing it re-integrates (sim key) or is live (render key), and greys/warns accordingly.
- **Navigation and the sim key (R-92).** `SimConfig` holds `z₀`, `q₁`, `q₂` and the lock, but the sim key holds only the slice plane (`z₀`'s out-of-plane part, span{q₁, q₂}, the in-plane orientation): in-plane pan and zoom re-address; slicing out of the plane, tilting and rotating re-integrate; lock changes neither key.
- Provenance already serialises `SimConfig + RenderState`; the GUI is an editor of that serialisable object, so **two GUIs are two editors of one schema** and can coexist during the transition.
- **One canonical serialisation (R-309).** `SimConfig` and `RenderState` serialise one way everywhere, with a stable key order and exact number formatting, so equal state is equal text: the profiler header's `config` (`principia_dd_telemetry_and_tiers.md` §5), snapshot JSON, share links and pxpack all carry it. The key order and the number formatting are JCS's (R-318), below.
- **The canonical serialisation is JCS, RFC 8785 (R-318).** Object members are sorted by their names' UTF-16 code units (RFC 8785 §3.2.3), with no insignificant whitespace; strings are escaped as §3.2.2.2 gives; numbers take its format (ECMAScript's Number-to-String, §3.2.2.3). −0.0 is written `0`. A u64 field (a seed, say) is always written as a JSON string of its decimal digits, whatever its value; every other number is a JSON number. A field's type never depends on its value (R-322, amending R-318's per-value reading). No serialised type has a `usize` field: a count or size is an explicit u32 or u64, so neither its width nor its JSON type depends on the platform (R-329). RFC 8785's published test vectors are the acceptance test (REQ-TOOL-145).
- `ViewUI` is the firewall line: it is the GUI's own scratch state (blur, which debug category is visible, focus, open panels; lock is `SimConfig` since R-69) and the engine never reads it — so a replacement GUI, if there ever is one (v2 web's, R-571), can define its own `ViewUI` entirely (*was:* "so the polished GUI can define").

**The interface is subscribe/emit:** the GUI subscribes to state (re-renders on change) and emits typed edits (`setField(path, value)`). It never mutates engine internals directly. That is the whole contract between the two — small, and the only thing a replacement GUI must honour.

**Undo and redo live in the contract (R-52).** The contract keeps one undo/redo history of the typed `setField` edits it has applied, shared by every GUI: a GUI shows the depth (the dev GUI's top bar, `principia_render_gui_spec.md` §G2) and sends undo / redo as requests, and keeps no history of its own. A replacement GUI inherits the history for free. **What is undoable (R-69):** every `SimConfig` and `RenderState` edit — including navigation (it edits `z₀` and the basis) and lock / unlock (chart construction). `ViewUI`-only state — open panels, focus, selection, the kept-orbit list, playback transport — is not. **A drag coalesces into one history entry** (R-96). **Playback never enters undo (R-101):** the GUI's clock advances the playhead each frame through a `SetField` marked "no history"; a manual scrub is one coalesced entry.

**The snapshot carries the events the GUI reports (R-54).** The precision warning is raised by events, not fixed depths: the snapshot carries, GUI-sized, whether `DECODE_SWITCHOVER` has fired on visible quads and whether `AT_F32_FLOOR` has been hit (`principia_deep_zoom.md` §2; scheduler contract Part 4). The console (render_gui_spec §G12) reads the same telemetry stream the profiler does.

**The shell's fields (RQ-243, decided per R-369).** `RenderState`'s playhead is `Playhead { t: f64 }`, and its `SetField` path is `RenderField::Playhead`: the GUI's no-history clock advance (R-101) and a scrub (R-96) both write it. The snapshot's history is `History { undo_depth: u32, redo_depth: u32 }` (R-329). The snapshot carries a GUI-sized frame summary, named from the frame record (`principia_dd_telemetry_and_tiers.md` §5): `frame_ms`, `fps`, `quad_count` and `live_memory { heap_bytes, gpu_bytes }`, each optional and absent until the real engine's frame loop fills it, the GUI drawing "—" for an absent value. Their definitions against the frame record are written here by the task that adds them (R-72).

**The frame summary's definitions (R-72; REQ-GUI-176; TASK-M6-24).** Each is read from the frame record (`principia_dd_telemetry_and_tiers.md` §5) and posted GUI-sized, never per frame:
- `frame_ms` is the frame record's `frame_ms` at the latest frame: its wall-clock milliseconds.
- `fps` is 1000 / the mean `frame_ms` of the frames since the previous snapshot. When no frame has completed since the previous snapshot, or that mean `frame_ms` is 0, `fps` is absent (the GUI draws "—"); it is not carried forward from an earlier snapshot.
- `quad_count` is the frame record's `leaf_count` at the latest frame, a u64 as `leaf_count` is.
- `live_memory { heap_bytes, gpu_bytes }` is the frame record's `live_memory` at the latest frame: `heap_bytes` its `heap` pool's `bytes`, `gpu_bytes` its `gpu` pool's `bytes`, each a u64. The `tile_cache` pool is not added to either: `heap` and `gpu` are the tracked memory outside the tile cache. The footer's memory readout, GPU and heap, shows these two.

**The log entry (R-72; REQ-GUI-177; TASK-M6-24).** A log entry is `{ severity, at, source, message }`, plain data: `severity` is one of `error`, `warn` and `info`; `at` is the wall-clock time it was logged, in milliseconds since the Unix epoch, UTC, a u64 (R-329); `source` is one of `stain`, `integrator`, `quadtree`, `contract` and `app` (`principia_render_gui_spec.md` §G12's sources); `message` is one line of text. The snapshot carries, GUI-sized, the entries accumulated since the previous snapshot, oldest first, so each entry rides in exactly one snapshot, and the GUI reads them from each snapshot it receives: they add no crossing of the membrane, which carries only GUI-sized snapshots and single `set_field` edits (`principia_systems_architecture.md` §6, invariant 10). R-54's precision flags stay in the snapshot as flags. Every engine logs each `SetField` it applies as one `info` entry from `contract`, naming the field's path and its values before and after ("SetField Playhead.t 0 → 1.5", "(no history)" added for an edit so marked); the conformance suite checks it on every engine (R-390). The footer counts the `warn` and `error` entries since the session began, until the console's "clear" (`principia_render_gui_spec.md` §G12) resets them. Carrying the entries in profiler schema v1's stream is not part of this definition: schema v1 is unchanged.

**Time, the overlay set, the hover, the legend's keys and the sample (R-72; decided per R-369; TASK-M6-27).** The fields these screens read and write, named here as the definitions they need:
- `SimConfig`'s horizon is `Horizon { t: f64 }`, the `T` of "T/dt/thresholds/eps" above: where playback stops and the scrubber ends (`principia_dd_simstate_payload.md` §2). Its `SetField` path is `SimField::Horizon`, the whole `Horizon` in one edit, a sim key and undoable (R-69), its `t` in `[50, 200]` (`principia_render_gui_spec.md` §G5, R-132), logged as "SetField Horizon.t before → after"; the Run window's `T_horizon` sends it (R-390, "Contract fields"; TASK-M6-28).
- `ViewUI`'s open windows are `Windows { open }`, each a `Window` by name, in the order opened; the names are Run, Chart builder, Export & share, Display, Profiler, Console and Inspector (`principia_render_gui_spec.md` §G5, §G7, §G8, §G12), and Sequence (R-577) and Spotlights (R-578) (`principia_render_gui_spec.md` §G9a; the sequence being edited is the GUI's own document, beside `ViewUI`, which the engine reads only as the spec a preview, proof or export request carries, and whose undo is RQ-559), and the dev GUI keeps the console's open state as a flag of its own, beside the list. Like the rest of `ViewUI`, never read by the engine, never undoable (R-69; TASK-M6-28).
- **The Sequence's preview (R-577; fix round 1 of PR #286, gui review 5631060675, F2; applied per R-369).** Play preview (`principia_render_gui_spec.md` §G9a) is engine state set by a request, as undo and redo are, never by `SetField`: a **preview** request carries the spec (`principia_export_animation_contract.md` Part 3) and the wall time to start from, and a **stop-preview** request, the spec's end, a `SetField` that enters the history, or an undo or redo request ends it; a no-history `SetField` (the hover's, the clock's advance) does not (*was:* "or any `SetField` ends it"; fix round 2 of PR #286, gui review 5631227164, G1, applied per R-369). While it plays the engine draws the spec's frames through the live frame loop in place of the view and owns the playhead by the keyframes' policies; `SimConfig` and `RenderState` are not changed, so nothing enters the history. The snapshot carries `preview: Option<{ t_wall }>`. Such an edit, undo or redo ends the preview first and then applies to `SimConfig` or `RenderState` as usual (*was:* "A `SetField` that arrives while it plays ends the preview first"). While it plays the GUI sends no hover, and the Legend, decoder badge and `t` readout describe nothing (`principia_sequence.md` §3.6). Logged as "Preview start <t_wall>" and "Preview stop".
- `ViewUI`'s playback transport is `Transport { playing: bool, speed: f64 }`: `speed` scales the GUI clock's time source, so the clock's no-history advance (R-101) is the source's elapsed time, in units of `t`, times `speed`, stopping at `T`. Loop is not yet a field. `ViewUI`'s kept orbits are `KeptOrbits { orbits }`, each a copy of the snapshot's sample when it was kept; like the rest of `ViewUI`, never read by the engine, never undoable (R-69).
- `RenderState`'s overlay set is `Overlays`, one bool per toggle of Overlays ▾ (`principia_render_gui_spec.md` §G2), in its order: `tile_bounds`, `depth_colouring`, `priority_heatmap`, `refine_flagged`, `split_merge_activity`; `substep_density`, `substep_cap_hits`, `nan_invalid_pixels`, `energy_drift`; `forbidden_region`, `grid`, `burrau_lattice`; `class_edges`, `t_end_contours`. Its `SetField` path is `RenderField::Overlays`, the whole set in one edit, undoable (R-69), logged as "SetField Overlays [before] → [after]" with the names of the toggles on. The engine draws what is on into the figure; the GUI draws nothing over it.
- `RenderState`'s hover is `Hover { at: Option<(s, t)> }`: the figure point under the cursor, `(s, t)` in `[0, 1]²` with `t` up, or none when the cursor is off the figure or the layer hidden. The engine reads it, to integrate and draw the hover path and to sample the IC under the cursor, so it is `RenderState`'s, not `ViewUI`'s; its `SetField` path is `RenderField::Hover`, always marked "no history" like the playhead's advance (R-101), so a hover never enters undo, and it is logged as "SetField Hover.at before → after (no history)". The GUI sends it only when it changes.
- The snapshot carries the legend's keys, `Legend { keys }`, evaluated from the stain walking back from OUT, one key per dimension in OUT's order (`principia_render_gui_spec.md` §G6): a categorical key `{ title, entries }`, each entry `{ label, swatch, share }`, its swatch a flat sRGB colour or the "not yet" style (R-399) and its share in percent of the view; or a bar key `{ title, lo, hi, lo_note, hi_note, samples }`, the channel's sRGB samples at equal steps over its declared domain `[lo, hi]`. The GUI draws exactly these keys, in order, and holds none of its own.
- The snapshot carries the sample under the hover, `Sample`, or none with no hover: its figure point `at`, its IC, its fate (the outcome swatch's label) and `t_end` once decided at the playhead, else none for both; its FTLE, `E₀`, F₂ word, substep count, minimum separation and `|ΔE/E|`; its path's span `path_t`; its shape-sphere path as `(θ, φ)` pairs, `θ` the azimuth and `φ` the polar angle (`principia_chart_reference.md` §3.3); and the three bodies' paths in the centre-of-mass frame. The Trajectory panel's "under cursor" tab and the hover label read it.

**The slice frame, live drags and the GUI's own settings (R-457 to R-462; decided per R-369).**
- `SimConfig`'s slice frame is `SliceFrame { normal, tilt_x, tilt_y }`, three latent dimension indices: the slice normal (the compass cube's D axis, the dimension the slice moves through) and the dimensions `τ₁` and `τ₂` tilt into (`principia_render_gui_spec.md` §G2, the Tilt section and the compass's axis buttons). Like the lock it is chart construction, so it is undoable (R-69); no kernel reads it (the kernel reads `z₀`, `q₁`, `q₂`), so an edit of it alone re-integrates nothing. `normal` is never one of the plane's own axes, and `tilt_x`, `tilt_y` are never `e_a` or `e_b`; its defaults are the GUI's slice axis before R-459 (z_q3, or the first axis the plane does not show) for all three, which draws what the GUI drew before. A pick that changes both the basis and the slice frame is one history entry.
- **A drag coalesces into one history entry (R-96), its edits applied live (R-458).** A drag (a slider, a number box, the compass, the scrubber, a pan) sends its edit every frame the value changes, at most one per frame, each marked with the drag's key; the history keeps one entry per key, its before the value at the drag's start and its after the last value sent, and the console logs it once, when the drag ends. An edit with a new key, or none, closes the entry.
- **A command of several edits is one history entry (R-466).** A section reset or Home (`principia_render_gui_spec.md`
  §G2) sends its edits, `z₀` and the basis, in one frame under one fresh key, as a drag does, so the history keeps one
  entry and one undo restores every field it set; a command with nothing to change sends nothing. Its values are the
  chart preset's (its base point, centre and window, R-72, REQ-GUI-224), read by the GUI; the engine reads only the
  `z₀` and basis it is sent.
- **The debug view and F3's fill mode (R-467, R-468; decided per R-369).** `RenderState`'s `debug_view: bool` is View ›
  Debug view (`principia_render_gui_spec.md` §G2): the engine reads it to draw the figure's non-system, failed and
  non-finite pixels in the debug styles and to count them for the legend's "Debug" key. Its `SetField` path is
  `RenderField::DebugView`, undoable as an Overlays ▾ toggle is (R-69: every `RenderState` edit is undoable), one
  history entry per toggle, a render key that re-integrates nothing, logged as "SetField DebugView before → after"; it
  is off at launch (applied per R-369 on gui review 5585209592 of PR #253, G4). *Was (R-467 D5 as first ported):*
  "always marked 'no history' as the hover's is, so it never enters undo". `ViewUI`'s
  `F3Fill { Extend, Fit }` is View › "F3 fills by", Extend at each launch, the GUI's own state: never undoable, never
  read by the engine, which reads only the figure's viewport, its size and scale (R-406), not kept across sessions.
  The canvas background the presentation views draw for a pixel that is not part of the chart (R-467) is a render
  uniform the GUI sets to the colour its window is cleared to, not a contract field.
- **The tilt map, the energy-axis mappings, saved charts, the stop and the lines (R-469 to R-476; decided per R-369).**
  - `SimConfig`'s slice frame gains `tilt_map: Sheet | Lean` (`SliceFrame { normal, tilt_x, tilt_y, tilt_map }`),
    the map from `τ₁`, `τ₂` to the basis when both angles tilt into one target (R-474; `principia_chart_decoder_contract.md`
    Part 4), default `Sheet`. It is chart construction, as the targets are: undoable, one history entry, sending the
    basis with it where the basis changes; serialised as `tilt_map=sheet|lean`; a state, saved view or export with no
    such field, from before R-474, reads as `Lean`, the map it was made with.
  - A curved chart's tilt (R-473) is part of `SimConfig`'s chart params: the angles, the targets (indices of its hidden
    coordinates) and the map, from which the CPU computes the rotation `R` the kernel reads. A sim key.
  - Each energy-type axis's mapping, `warped | linear` (R-471), is part of `SimConfig`'s chart params, a sim key,
    `warped` by default.
  - A saved chart (R-475) is, while active, `SimConfig`'s chart id and params: the latent chart with the saved
    `(z₀, q₁, q₂)`, or the parent chart's id and params with the saved slice centre and tilt. Saving is one `SetField`
    on them, one history entry. The list of saved charts (the chart picker's "Saved" group, each with its name and
    provenance) is the GUI's own settings, beside the Chart builder's presets, kept across sessions and never undoable.
  - **The navigation stop (R-469).** The contract applies the stop to every navigation `SetField` (`z₀`, the basis,
    the lock, a chart parameter that moves the centre) before it records the edit: the edit applied, the history's
    after and the console's "SetField … before → after" line are the clamped value, never the requested one, and the
    snapshot says which check stopped it, for the readout.
  - `RenderState` gains `Lines` (R-476; `principia_lines.md` §3): the styles a view uses, its contour sets, its traces'
    settings and its axes, exported with the view, each edit one undoable `SetField` (`RenderField::Lines`) that
    re-integrates nothing (a render key). `Overlays` loses `grid` and `t_end_contours`, which become the Lines presets
    Grid and t_end (R-476, L2), and `class_edges`, which becomes the preset Class edges (R-509). The personal style library is the GUI's own settings. A hover is never recorded.
  - **The sidecar.** An export's provenance record (the pxpack, the snapshot JSON, the share link) carries, beside
    `SimConfig` and `RenderState` in their canonical serialisation, `ViewUI`'s F3 fill mode as `f3_fill=extend|fit`
    (R-472); `tilt_map`, the curved chart's tilt, the axis mappings and a saved chart's definition ride in `SimConfig`.
- The keyboard navigation flag (on or off, `principia_render_gui_spec.md` §G3, R-460; turned off by any mouse click or drag and on by the next key press, R-464), the windows' positions and sizes (R-461) and the panel layout (R-462) are the GUI's own state, never the contract's: never undoable and never read by the engine. The positions, sizes and layout are kept across sessions in the GUI's own settings.

**The links, the Decoder window and the round-trip rule (R-478 to R-484; decided per R-369).**
- `SimConfig`'s links are `Links { mass, alpha, beta, momenta }`, one choice per row of the Decoder window
  (`principia_render_gui_spec.md` §G7a), each a `LinkChoice { id, params }`: `id` the registry entry's name, `params` a
  map from each tunable parameter of that entry to its value (`{"mu_max":5}`; `{}` for an entry with none). It is the
  one place the links and their values live: the ViewState record, the sim key, the payload compatibility signature,
  every export and every sidecar read it through the one canonical serialisation, and no second copy is kept (R-483,
  X1). Every block's id and every parameter is written even at its default, with no `skip_serializing_if` and no serde
  default on a link field: a record that omitted a default would read back under the receiver's default (X2). The
  four momentum controls share the Momenta row's choice (R-479, E1). Its `SetField` path is `SimField::Links`, the
  whole `Links` in one edit, a sim key that re-integrates and an undoable history entry (R-69), logged as "SetField
  Links before → after" with each block's id and values; the Decoder window's staged edits are the GUI's own until its
  Apply sends it (R-479, E4). No chart edit sends it (R-478, L2), but for one: loading a chart preset or a saved chart
  that pins links sends the chart and the pinned `Links` together, a command of several edits, one history entry
  (R-466's rule above), and the GUI posts a notice naming each block it changed (R-516); neither Home nor a section
  reset sends it (R-479, D3). A preset's or saved chart's pin, the whole `Links` it stores, is that entry's, the GUI's
  own settings beside the preset, never `SimConfig`'s, so no export carries a pin: it carries the links active
  (R-516). *Was:* "No chart edit sends it (R-478, L2), and neither Home nor a section reset does (R-479, D3)." (stale
  value, replaced by R-516). `ViewUI`'s open windows gain `Decoder`, listed after Chart builder (R-478, L4).
- **The decoder's identity on import (R-483, X4, X5; R-521).** Every export carries, beside `SimConfig` and
  `RenderState`, the build hash, the schema version and, per block, the recorded link's entry hash
  (`principia_dd_image_embedding.md` §6; `principia_dd_generation_root.md` §3.9, "Each entry's own hash", which also
  covers the registry constants the entry's decode reaches). One function of the
  contract reads every path before anything renders: a link id the build's registry lacks is "state present, cannot
  recreate", naming the link and its block; a block whose entry hash differs from the build's entry of that id is
  refused the same way, naming exactly that link (R-521); when every block's hash matches, a schema version other than
  the build's refuses nothing on the links' account, the schema version staying the whole-ledger check of what a
  stored payload's bits mean, so a payload the record holds is not reused across it and the view re-integrates; the
  build-hash component that detects a decoder or chart change (REQ-TOOL-112) still refuses what it detects. A record
  that carries no entry hashes is read as before R-521: a schema version other than the build's is refused, naming its
  links as possibly changed. No default link or default value ever stands in. *Was:* "the build hash and the schema
  version (`principia_dd_image_embedding.md` §6)" and "meanwhile a schema version other than the build's is refused
  the same way (whether each link carries its own entry hash is RQ-385); no default link or default value ever stands
  in." (stale value, replaced by R-521). A record whose `links` is
  `{}`, written before R-483, recreates under this build's defaults if its build hash is this build's, and otherwise
  only on the user's explicit acceptance, labelled "links assumed".
- **The round-trip rule (R-484).** Importing an export is the identity on `SimConfig` and `RenderState`, field for field
  and bitwise in the canonical serialisation, on every path: snapshot JSON, share links, pxpack (a saved view's
  snapshot), the profiler header's `config`, the animation spec object and the PNG `tEXt` chunk. `ViewUI` is not
  exported; R-472's `f3_fill` rides beside the two as a sidecar field, outside the identity (R-484, J3). The one lossy
  path is the image's low-bit embedding, which drops whole tiers, custom source first, then colour, never tier 1
  (`principia_dd_image_embedding.md` §6). Every field of `SimConfig` and `RenderState` is assigned a tier in one table,
  `crates/engine/src/contract/tiers.rs`: one function destructuring both structs and every group with no `..`, and
  every enum by a `match` with no wildcard, so a field added without a tier does not compile; the embedding's writer
  and the tests read it (R-484, J8). No struct of either type serialises to `{}` in a state whose every field is off
  its default (R-484, T2; maps and lists may be empty).

**The Print graph (R-485 to R-492; `principia_print_graph.md` §4, §9).**
- `RenderState` gains `print: PrintGraph`, the Print graph the figure is printed through (R-491): the design doc §4.1's
  graph object (`format` `principia.print/1`, `name`, `settings`, `nodes`, each `{ type, params, in }`, and `out`),
  its preset, when it came from one, recorded **by value**, the graph it expanded to, with the preset's id as a label
  (R-491). The default is Plain, the identity. It serialises with `RenderState` in the one canonical serialisation
  (R-309, R-318), node ids sorted by JCS and carrying no meaning, so it travels everywhere `RenderState` goes and
  round-trips bitwise (R-484, RT1). Its `SetField` path is `RenderField::Print`, the whole graph in one edit, a render
  key that re-integrates nothing and never touches the sim key; it is undoable (R-69), a drag on one of its settings
  coalescing into one entry (R-96), and Reset to Plain one entry. In R-484's tier table the graph with its built-in
  nodes is tier 2 and a custom WGSL node's source tier 3 (R-491).
- **Editor positions are `ViewUI`'s** (R-491; design doc §4.1): where each Print graph node sits on the graph editor's
  canvas, its pan and zoom and the selection are the editor's own, never the graph's, so moving a node is no undo entry
  and two equal graphs serialise equal, as the stain graph's are (below).

**The stain graph (R-72; decided per R-369; TASK-M6-29).** `RenderState`'s stain graph is §5's graph object, the one the engine checks and lowers: its nodes, each `{ id, kind, occupant, params }`, `kind` one of `source`, `colour`, `brightness`, `combiner`, `post` and `out`, `occupant` none, a source's field, a built-in's id or a custom occupant's WGSL, and `params` its uniforms' values by name, one number per component; and its wires, each `{ from, to, port }`, `port` the in-port's position, a u32 (R-329). It serialises with `RenderState` as `{ nodes, wires }`, in id order; the default is the backbone alone, the combiner wired to `OUT`. Its `SetField` path is `RenderField::StainGraph`, the whole graph in one edit, undoable (R-69): picking a node's occupant or setting a param is one edit (REQ-GUI-016, which TASK-M7-11 keeps). It is logged as "SetField StainGraph [before] → [after]", each graph as its nodes in id order, `id kind occupant` (a custom occupant written `custom`) with its params, if any, as `{name=value, …}`, then its wires, each `from→to.port`. A node's place on the editor's canvas, the canvas's pan and zoom, and which nodes are selected are the editor's own, not the graph's (R-390, "Contract fields": the corpus names nodes, wires and per-node params only).

**A param of several components in the log line (R-72; decided per R-369; TASK-M6-29).** In "SetField StainGraph"'s `{name=value, …}`, a param of one component is written as its number, and a param of several components as its numbers in order, comma-separated, in parentheses: `{tint=(0.2, 0.4, 0.6)}`.

---

## 3. The registry is the scanned filesystem — for the *fragment* side; compute occupants are Rust build variants

**Under the substrate split (lowering Part 2) the two pipelines register differently, because they are different mechanisms.** The **fragment/colour side is the scanned filesystem**: colour/brightness/combiner/post occupants are standalone **WGSL** source files in known directories, and **the registry is not a hand-maintained list — it is generated by scanning those directories** (and runtime-authored customs join the same way — §5). Add a `.wgsl` file to `frag/colour/` → it appears in the GUI, selectable → no code edited. This is what keeps colour occupants *runtime-registerable* and the devkit alive. Same "generate from one source" discipline as the layout table, applied to fragment shaders.

The **compute side does not work this way**: the physics kernel is one Rust source monomorphised over chart/occupant (lowering Part 2), so an occupant like KDK/Yoshida/RK4/Euler or a chart's Φ is a **build-time Rust variant selected by a type/enum**, *not* a scanned file and *not* runtime-authored. There is no browser-time compilation of user Rust. So the "scanned filesystem" registry is a **fragment-side** mechanism; the compute-side "registry" is the finite set of compiled variants the Rust build produced.

```
# Fragment side — WGSL, scanned at build time, runtime-registerable:
shaders/wgsl/
  lib/          generated + shared: unpack accessors (from the Rust layout def), vMF, oklab, colour-space, prelude
  frag/
    colour/     colour(ctx) -> vec3
    brightness/ brightness(ctx) -> f32
    combiner/   combine(rgb, b) -> vec3
    post/       post(rgb) -> vec3
    debug/      colour/brightness occupants that read raw payload/quad fields (tagged category:debug)
  compositor/   fixed passes (blur, composite) — NOT occupants, not scanned

# Compute side — Rust, monomorphised at build time, NOT scanned:
kernel/  (Φ maps · decode · canonicalise · wrapper · occupants KDK/Yoshida/RK4/Euler)
         — chart & occupant are type parameters; the "registry" is the set of compiled variants
```

The scanner produces registry entries `{id, slot, source, category, uniformSchema, inputDomains}` keyed by directory (slot) — an occupant is valid only in its slot (its signature). **`inputDomains` (R-53):** each node declares the domain of each input it maps from; by default it inherits the manifest's per-field domain (the fixed `[lo, hi]` of `range_norm`, render_gui_spec §10.1), and a node that transforms its field declares its own. The generated legend samples each node over these domains (render_gui_spec §G6). The slot dropdowns read this registry; the assembler splices from it. **A file appearing in `frag/colour/` is the act of registering a shader.**

**The declaration format (R-72; REQ-GEN-027; TASK-M1-04).** An occupant's `.wgsl` file declares its `uniformSchema`
and `inputDomains` in line comments, so the file stays plain WGSL; the scanner and the assembler read the same lines.
A declaration is a line whose text, leading white space removed, starts `// @`, one to a line, anywhere in the file.
The word after `@` is `uniform` or `input`; any other is refused, so a misspelt declaration is an error, never a
silent default.
- `// @uniform <name>: <type> = <default>`, optionally followed by `[<lo>, <hi>]`: one `uniformSchema` entry. `<type>`
  is `f32`, `i32`, `u32`, `vec2<f32>`, `vec3<f32>` or `vec4<f32>`. `<default>`, the value a new node starts with, is a
  number for a scalar and `(<x>, <y>, …)` for a vector, one number per component. `[<lo>, <hi>]`, `lo < hi`, is the
  range of the control the GUI offers, for a scalar only. A number is a decimal literal without a suffix; an `i32` or
  `u32` value is an integer in its type's range; an `f32` value, and each component of a vector, is a number that
  rounds to a finite `f32`, so none beyond `f32::MAX` but its rounding. The default, and every value a node is
  given, is a value of the type
  within the range, or it is refused. The occupant reads the value as `uniforms.<name>`: the assembler declares
  `uniforms` for it, a uniform block holding the entries in declaration order, in group 0 with the per-frame uniforms
  (R-343). Each node that declares a uniform has its own block, and the blocks take consecutive bindings, the first
  the binding after the prelude's, in the nodes' canonical order (lowering contract Part 5): the first such node's
  block at the prelude's binding + 1, the next at + 2, and so on.
- `// @input <name>`, optionally followed by `[<lo>, <hi>]`: one `field` in-port of the node, in declaration order.
  The occupant reads the field wired to it as `ctx.inputs[k]`, `k` its position, a `vec4<f32>` holding a scalar or a
  category in `.x` and a vector in `.xyz`. The bracket is the input's `inputDomains` entry, the domain the node maps
  from (R-53); without it the input inherits the manifest's per-field domain, the fixed `[lo, hi]` of `range_norm`. A
  colour occupant declares one to four inputs and a brightness occupant one; one that declares none has one, named
  `field`, its domain inherited (render_gui_spec Part II §3.1). A post's inputs are its optional field ins, up to
  four, after its `vec3` in. A source and a combiner declare none. (Four, the length of `ctx.inputs`, is applied per
  R-369: the corpus gives no bound.)
- Names are ASCII WGSL identifiers, each declared once among a file's uniforms and once among its inputs: an ASCII
  letter or `_`, then ASCII letters, digits or `_`, and neither `_` alone nor beginning `__`, which WGSL does not take
  as identifiers. WGSL's non-ASCII identifiers are not names here.

**Worked example of the declaration format.** A colour occupant, `frag/colour/banded.wgsl`:

```wgsl
// @uniform gain: f32 = 1.0 [0.0, 4.0]
// @uniform tint: vec3<f32> = (1.0, 0.5, 0.25)
// @input t [0.0, 1.0]
// @input mask
fn colour(ctx: Ctx) -> vec3<f32> {
    return uniforms.tint * clamp(ctx.inputs[0].x * uniforms.gain, 0.0, 1.0) * ctx.inputs[1].x;
}
```

Its `uniformSchema` is `gain`, an `f32` starting at 1.0 with a control over [0, 4], then `tint`, a `vec3<f32>`
starting at (1.0, 0.5, 0.25) with no range; the assembler declares its block holding `gain` then `tint`, read as
`uniforms.gain` and `uniforms.tint`. Its node has two `field` in-ports: `t`, read as `ctx.inputs[0]`, whose
`inputDomains` entry is [0, 1], and `mask`, read as `ctx.inputs[1]`, which inherits the manifest's per-field domain.

**`debug/` is a peer directory, but a filter tag — not a different mechanism.** Debug occupants satisfy the same signatures as their slot; they just read `ctx.sample`/`ctx.quad` raw fields. The scanner tags anything under `debug/` `category: debug`. A replacement GUI, if there ever is one (v2 web's, R-571; *was:* "The polished GUI"), hides that category by **filtering the list** (`ViewUI.debugVisible = false`); the dev GUI shows it. "Hide debug in the nice GUI" is a filter predicate over a tagged registry, never a structural change.

**Generated field views mount into the same tree.** The ledger-derived per-field debug shaders (tooling plan §B–E) are emitted into `frag/debug/generated/` (or the ledger is scanned as a virtual directory beside the real files). Hand-written debug shaders (quad-depth heatmap, quadtree overlay) are real files in `debug/`. Both surface through one scan; both are registry occupants selectable in a slot.

---

## 4. The occupant model — typed by signature, free inside

> **All that matters is the return type of each pipeline stage. What the shader does inside is entirely its own business.**

This corrects a category error in the colouring drill-down. *Data shape* (scalar / vector / categorical) and *visual channel* (colour / brightness) are **independent**. "Vector → colour, scalar → brightness" is a sensible **default**, never a law. Three independent axes; any signature-compatible combination is allowed:

1. **Source + reduction** — the occupant chooses what field it reads and how it reduces it. A vector `n` may be viewed as the vector, its magnitude `‖n‖`, a component `n_z`, an angle — the shape displayed is what the occupant asks for, not a fixed property of the field.
2. **Channel** — colour or brightness, independent of source shape.
3. **Mapping** — a scalar → a matplotlib LUT (Viridis/Magma/…), → brightness, → one colour channel's intensity; a vector → vMF, → direction-cosines. Free.

The **only** constraint is the slot's output signature:

- `colour` occupant must return `vec3` — it may read a vector and vMF it, *or* read a vector, take the magnitude, and Viridis it, *or* read a scalar and Viridis it, *or* jam a scalar into one channel and zero the others. All legal.
- `brightness` occupant must return `f32` — it may read a scalar directly *or* read a vector and emit its magnitude.

**LUTs are therefore available to any scalar quantity**, including a scalar extracted from a vector — which makes the seamless-LUT system far broader than "sphere colouring": any scalar field anywhere can wear any LUT.

**What survives from the drill-down (kept, because it is real): meaning-preservation, not shape.**
- **Never interpolate a category.** Averaging outcome-`state` 1 and `state` 3 into "state 2" is a lie. Mixed categorical pixels are resolved by the **per-sample colour-then-SSAA-resolve** blend (each sample classified and coloured independently, then colours averaged — sampling/SSAA note), NOT by averaging `state` *indices* and NOT by the old majority+desaturation rule (vetoed, dd_colouring §3.7). (This is about *meaning*, not channel: you *may* map `state` → brightness levels if you want; you may not *average* the `state` value.)
- **Never silently claim a measure.** A quantity displayed for quantitative reading still owes its Jacobian (Part 2.5); the display freedom does not suspend measure honesty.
- **L-ownership** (when a brightness metric is bound it owns L) survives as the *combiner's* concern — unchanged.

Legibility guidance (5 hues usually beat 5 brightness levels for classes) is **advice surfaced in the GUI**, never an enforced constraint.

**Debugging payoff (the case that proves the freedom matters):** `‖n‖` as a Viridis map should render **flat uniform colour** if normalisation is correct; any deviation lights up instantly — a normalisation check impossible to express under "vectors must be colour." This is a *generated* debug occupant (the ledger knows `n` is a vector → auto-offers "‖·‖ as scalar" beside "as direction-cosines"), paired with the test asserting `‖n‖ = 1` to tolerance. Same four-surface pattern, one more reduction offered. The tolerance is 64 U = 2⁻¹⁸ ≈ 3.81e-6 on `‖n‖ − 1` at f32, U = 2⁻²⁴ the f32 unit roundoff, confirmed at the M1 gate and held as the constants register's `n_norm_tol` (R-433; REQ-VAL-122).

---

## 5. The stain editor — a free, typed node graph (R-64)

**The stain is a graph object** (`principia_render_gui_spec.md` Part II §3–§4): typed `source` / `colour` / `brightness` /
`post` nodes wired freely, subject only to port types and acyclicity, over a fixed `combiner` + `OUT` backbone; fan-out
from one source; multi-input nodes; a variable-length post chain `… → combiner → (post)* → OUT`. Node ids, wires and
per-node params are the stain's data, and they serialise with `RenderState`. *(Was, before R-64: a four-slot object
`{colour_id, brightness_id, combiner_id, post_id, uniforms}` edited by a fixed-wiring four-slot inspector. The graph
supersedes it; a four-slot pipeline is one graph among many.)*

The three edit modes are three edit-actions on that one object:

1. **Pick a node.** A node's occupant is chosen from the scanned registry (§3; filtered by slot, debug category
   shown/hidden per `ViewUI`). Selecting writes an id. One field edit.
2. **Custom code per node.** The node's occupant becomes `custom`; the editor exposes a WGSL text field for that node's
   source — the render contract's custom-occupant path verbatim: schema-driven uniforms, async compile,
   **last-valid-pipeline fallback on error**, per-node failure isolation. The editor adds a compile-status indicator and an
   error surface (render_gui_spec Part II §9, §10).
3. **Presets.** A preset is a whole serialised graph — pure data (lowering contract; render_gui_spec Part II §11).
   Selecting one replaces the stain wholesale; "swap the entire pipeline" is one assignment.

All three edit the same graph; they differ only in whether a node holds a built-in id or a custom source string, or the
whole graph is set from a preset. This is *why* custom is "an occupant, not a node kind" — it collapses the three modes into
one uniform mechanism.

**Custom compute occupants are NOT runtime-authored (substrate change, lowering Part 2).** A user integrator or experimental Φ is a **build-time Rust variant** — you cannot compile user Rust in the browser, so there is no compute-side text-box occupant with compile/fallback rails. This is the accepted loss of the Rust-kernel move (niche — few users write their own symplectic integrator; the parity-critical side is single-sourced in exchange). The colour-side custom path (WGSL text field, §5 mode 2) is the one that stays runtime-authored, because colour has no parity stakes. So the three-mode editor of §5 is a *fragment-side* affordance; the compute side offers a *choice among compiled variants*, not free authoring.

---

## 6. Quality settings — preset selector over one struct (see `principia_quality_device_note.md`)

Quality is a **preset selector populating one settings struct** (same pattern as the colour graph editor, §5 — one object, several ways to fill it):

```
QualitySettings = { N (samples per quad axis), max_rel_depth, render_scale, lock_to_native,
                    E, ftle, word, e_motion_gating, checkerboard_mode, cache_cap, ... }
```
*(Field-level authority: `principia_quality_device_note.md` §2 — this listing is a pointer, not a second definition.)*

- **Auto** — runs **device characterisation** (limits gate → probe measures throughput → adapter.info sanity-checks → thermal headroom), *solves* every knob from one measurement. Re-runnable ("re-detect"). Auto is a *measurement*, not a fixed tier — and its **steady-state behaviour is a live controller** (below).
- **Named tiers** (Potato/Low/Medium/High/Ultra/Extreme — pinned rungs of the quality ladder, `principia_memory_tiers.md`) — fixed values, skip the probe.
- **Custom** — exposes the knobs directly, **pre-filled with the current preset's values** (read-only until touched, so the user *sees* what auto chose; touching any field flips to custom *from that baseline* — "auto's answer, now editable," not a blank slate).

**Hard device limits clamp EVERY preset including custom** — allocation feasibility (`adapter.limits`) is authoritative; a slider maxes at the device ceiling. Named tiers clamp too (a "High" preset on a weak device clamps to what fits rather than crashing — graceful-degradation-not-hard-failure).

**Auto is a continuous controller (Auto-only).** One arbiter, many sensors; model-based (estimate device throughput, solve knobs) not reactive; adjusts on a **quality ladder** (one integer = the rung); a resting rung plus a motion offset (drop rungs while the playhead/camera moves, restore at rest); changes only under cover (motion/blur/natural-invalidation); hysteresis + failed-rung memory; senses sim-time-debt and resync-time (felt quantities). **Named/custom switch the arbiter OFF** — "give me exactly this" is honoured verbatim; adaptivity is what "Auto" *means*. Full architecture in the quality/device note.

**Sim-key vs not (SETTLED):** `N`/`ftle`/`word` are **sim-key** (change *what's computed* → invalidate → re-boot, masked by the staleness backdrop). **Depth and `E` are not on the sim key (R-89):** `MAX_REL_DEPTH` is a scheduler knob, and `E` changes live — copies are cached per `copy_index` and the nominal's key excludes `E`, so copies drop and respawn under motion gating without invalidating the nominal. **`render_scale` is NOT sim-key and invalidates nothing** (payload purity — it moves the refinement target only; quality/device note, caching Part 2), which is exactly why the controller uses it as the continuous motion-time lever; `lock_to_native` likewise invalidates nothing. Sim-key components move only at natural-invalidation moments.

**Persistence:** the resolved model persists (`{throughput, resting rung, motion-offset curve, failed-rung memory, provenance signature}`) — reload with matching provenance (adapter info + limits + probe version) starts correct instantly; mismatch re-probes; the live controller + explicit re-detect keep it from ever being a stale trap. The user's *intent* (preset/custom values) is remembered verbatim; the auto *measurement* is a signature-validated cache.

**GUI/state placement:** the preset selector and custom fields are GUI surfaces editing `SimConfig.quality` (in the dev GUI, the Run window — render_gui_spec §G5); the arbiter is engine-side (it writes `SimConfig.quality` knobs during settled periods). `ViewUI` carries the arbiter's debug overlay visibility (estimated throughput, current rung, recent decisions — magic you can inspect).

## 7. What a replacement GUI must honour (the teardown contract)

A replacement GUI, if there ever is one (v2 web's, R-571; the native app and v1 web keep this spec's GUI), must satisfy exactly and only (*was:* "The polished GUI, whenever it arrives, must"):

- read state via the subscribe interface; emit edits via `setField` — never touch engine internals (§1).
- treat the scanned registry as its source of selectable occupants; respect the `category` tag for filtering (§3).
- edit the stain graph object for colour (§5); the three modes are optional UI affordances, not requirements.
- expose the quality preset selector (auto/named/custom) editing `SimConfig.quality`; the arbiter is engine-side and independent of the GUI (§6), so a replacement GUI inherits adaptive quality for free — it need only offer the preset choice and the custom fields.
- define its own `ViewUI`; the engine reads none of it (§2).
- use the contract's undo/redo history (§2, R-52); keep none of its own.

Nothing else about the dev GUI is contractual. It can be Tweakpane, a hand-rolled panel, or thrown away entirely — the engine cannot tell, because it only ever sees state edits arriving through one typed door.

---

## 8. Amendment to the colouring drill-down

Strike the implication that data shape dictates channel. Keep: **categorical never averaged** (§4 — a meaning rule, not a channel rule), measure honesty, L-ownership. Add: slots are typed by output signature; occupants freely choose source, reduction, channel, and mapping; LUTs apply to any scalar. (The drill-down's vMF/OKLAB/CVD maths is unaffected — it describes *available* occupants, not *mandatory* channel assignments.)

---

*One typed door between GUI and engine, and the arrow points one way. The registry is the folder. The slot cares about the return type and nothing else. A scalar may wear any colourmap; a category may never be averaged. And the dev GUI is disposable because the engine was never looking at it — only at the state it edits.*
