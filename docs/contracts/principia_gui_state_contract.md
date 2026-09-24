# Principia — GUI & state-interface contract

*The load-bearing wall for "tear out the dev GUI later." Everything customisable, GUI pluggable, dev tooling swappable for a polished UI without touching the sim. The mechanism is one rule applied three ways: the GUI is a pure reader/emitter of a typed state schema; the *fragment* shader registry is the scanned filesystem (compute occupants are Rust build variants — §3); occupants are typed only by output signature. Corrects the colouring drill-down's implied "shape dictates channel" claim.*

---

## 1. The one-way dependency rule

> **The engine never imports the GUI. The dependency arrow points one way: GUI → state → engine.**

The dev GUI is allowed to be ugly and throwaway *precisely because* it is a thin renderer over the same state objects the engine consumes. The test is brutal and concrete:

> **You could drive the entire engine from a JSON file with no GUI attached. The GUI is just one possible editor of that JSON.**

**Under the substrate decision (`principia_spike_brief.md`: whole engine in Rust → wasm) this stops being an import-lint and becomes structural.** The GUI-facing surface — the typed `SimConfig`/`RenderState`/`ViewUI` schema, `set_field`, and the state snapshot — is defined **once**, in Rust, in the engine crate. Two bindings call that one surface, and only the firewall's *enforcement* differs between them:
>
> - **The egui dev binding (now):** a Rust immediate-mode **debug menu** (toggled with `F3`), compiled into the same wasm binary and calling the surface *natively* — a typed `SetField` enum + snapshot, no marshalling. The firewall is a fact of the **crate graph**: the engine crate marks *only* the state surface `pub` (scheduler, cache, kernel, payload buffers stay crate-private), so the gui crate depends on the engine crate yet *cannot reach past the surface* — a stray "egui callback into a sim object" is unexpressible because the internals aren't visible. Language-boundary strength, in-process.
> - **The TS production binding (later):** a thin TS GUI (React/HTML/whatever) across the wasm↔JS membrane, calling the *same* surface through a wasm-bindgen layer. The firewall is a fact of the **binary**: no shared linker, so the engine *cannot* import the GUI.
>
> **The dev binding earns no privilege for being in-process.** egui reads a snapshot and emits `SetField` exactly as the TS GUI will, and must *not* mutate engine state through a `&mut` borrow even though the same binary makes that physically possible. Exercising `set_field`/snapshot from day one is precisely what makes the eventual TS swap a drop-in rather than a cliff — the marshalled path is never left untested.

The boundary must stay a **data** boundary, not an **object** one — under *both* bindings; only the seductive failure differs. wasm-bindgen makes it easy to hand JS a **live handle to a Rust struct with callable methods** (the moment the TS GUI calls `engine.scheduler.request_refine(quad)` the firewall is decorative); in-process, the egui equivalent is holding a `&mut` into engine state. Both are barred — the rule is identical:
- **In:** `set_field(path, value)` — plain serialised data. **Out:** a state snapshot — plain serialised data. The TS side holds **no Rust handles**, only the last snapshot it was handed. That is the whole of §1's subscribe/emit rule, now with teeth.
- **The canvas is the one sanctioned exception** — and its handling is itself binding-specific. Under the TS binding the surface is browser-owned, so the GUI transfers the `OffscreenCanvas` to the engine's worker **once at startup and never again**, and the wasm engine drives `wgpu` directly against it (handle crosses once; no state crosses with it). Under the egui binding there is nothing to transfer: egui-wgpu shares the engine's `wgpu` context and paints the debug overlay onto the same surface in-process. Either way, no *state* rides on the canvas.
- **The snapshot carries GUI-*sized* state only** (view state, current tier, a few scalars — what the panels display), **never engine-sized** state (the payload, the quad tree, the reductions). Those stay wasm-side and are only ever *summarised* across. This is the identical law as the GPU→CPU membrane (systems-architecture §3): big data never crosses a membrane — and there are now **two** membranes (wasm↔JS and GPU↔CPU), same rule on both. A snapshot the GUI renders every frame is a serialise-copy-deserialise, cheap if it is GUI-sized and ruinous if someone reaches for the tree.

No GUI element ever holds sim logic, caches sim data, or computes anything the engine needs; it reads a snapshot and emits field edits, nothing more.

---

## 2. The editable state is the entire coupling surface

Everything customisable = every knob is a typed field on the state, already split by the two keys:

```
SimConfig    (sim key)     chart id + params · z₀,q₁,q₂ · slice values · lock flag · z_locked anchor · δ excursion
                          (lock is chart construction, R-69) · link ids ·
                          integrator occupant · T/dt/thresholds/eps · collision radius r_coll · quality settings (§6 — NB not every quality field is sim-key: `render_scale`/`lock_to_native`/`MAX_REL_DEPTH`/`checkerboard_mode` invalidate nothing; the GUI's re-integrate warning keys per-FIELD off the caching blast-radius table, not off the struct's home) · playback transport (play/pause/speed/loop)
RenderState (render key)  the stain graph (nodes · wires · per-node params, §5) · overlay set ·
                          palette/compaction params · playhead t
ViewUI       (pure UI)    backdrop ref · debug category visibility · keyboard focus scope · selection ·
                          kept orbits · inspector t_cursor · open windows
                          — never read by the engine
```

The GUI requirement adds **no new state** — it says *expose all of it*. Consequences that come free from the key split:

- The GUI knows, per control, whether editing it re-integrates (sim key) or is live (render key), and greys/warns accordingly.
- **Navigation and the sim key (R-92).** `SimConfig` holds `z₀`, `q₁`, `q₂` and the lock, but the sim key holds only the slice plane (`z₀`'s out-of-plane part, span{q₁, q₂}, the in-plane orientation): in-plane pan and zoom re-address; slicing out of the plane, tilting and rotating re-integrate; lock changes neither key.
- Provenance already serialises `SimConfig + RenderState`; the GUI is an editor of that serialisable object, so **two GUIs are two editors of one schema** and can coexist during the transition.
- `ViewUI` is the firewall line: it is the GUI's own scratch state (blur, which debug category is visible, focus, open panels; lock is `SimConfig` since R-69) and the engine never reads it — so the polished GUI can define its own `ViewUI` entirely.

**The interface is subscribe/emit:** the GUI subscribes to state (re-renders on change) and emits typed edits (`setField(path, value)`). It never mutates engine internals directly. That is the whole contract between the two — small, and the only thing a replacement GUI must honour.

**Undo and redo live in the contract (R-52).** The contract keeps one undo/redo history of the typed `setField` edits it has applied, shared by every GUI: a GUI shows the depth (the dev GUI's top bar, `principia_render_gui_spec.md` §G2) and sends undo / redo as requests, and keeps no history of its own. A replacement GUI inherits the history for free. **What is undoable (R-69):** every `SimConfig` and `RenderState` edit — including navigation (it edits `z₀` and the basis) and lock / unlock (chart construction). `ViewUI`-only state — open panels, focus, selection, the kept-orbit list — is not.

**The snapshot carries the events the GUI reports (R-54).** The precision warning is raised by events, not fixed depths: the snapshot carries, GUI-sized, whether `DECODE_SWITCHOVER` has fired on visible quads and whether `AT_F32_FLOOR` has been hit (`principia_deep_zoom.md` §2; scheduler contract Part 4). The console (render_gui_spec §G12) reads the same telemetry stream the profiler does.

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

**`debug/` is a peer directory, but a filter tag — not a different mechanism.** Debug occupants satisfy the same signatures as their slot; they just read `ctx.sample`/`ctx.quad` raw fields. The scanner tags anything under `debug/` `category: debug`. The polished GUI hides that category by **filtering the list** (`ViewUI.debugVisible = false`); the dev GUI shows it. "Hide debug in the nice GUI" is a filter predicate over a tagged registry, never a structural change.

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

**Debugging payoff (the case that proves the freedom matters):** `‖n‖` as a Viridis map should render **flat uniform colour** if normalisation is correct; any deviation lights up instantly — a normalisation check impossible to express under "vectors must be colour." This is a *generated* debug occupant (the ledger knows `n` is a vector → auto-offers "‖·‖ as scalar" beside "as direction-cosines"), paired with the test asserting `‖n‖ = 1` to tolerance. Same four-surface pattern, one more reduction offered.

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

The polished GUI, whenever it arrives, must satisfy exactly and only:

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
