# The node SDK: built-in and custom stain nodes, made and shown the same way

The owner, 24 Nov 2026: "if a custom node follows our SDK/api display it with the same features / buttons etc as an
inbuilt node... also means our nodes are built in the same way." And: "Entire point is the nodes are a visual
abstraction of the shader api/sdk, so custom node == custom shader."

Custom stays what the render contract Part 2 says it is: an occupant of a node, not a node kind. Below, "a custom" is a
node whose occupant is a source someone wrote in the app or imported.

## 1. The rule

1. **A node is a shader, drawn.** One source file is one node: its WGSL, its inputs, its parameters, and everything the
   editor shows about it. Writing or changing a node's code makes a custom shader, there and then. Changing its
   settings never does ("Settings, code and saving", §3).
2. **The editor reads only the source.** No code in the editor, or in the description it reads, asks whether a node is
   built-in or custom, or names a built-in's id. A node carries no mark of it: where its shader comes from (shipped,
   yours, whose) is said where shaders are listed and on the inspector's source line, from the source's own `@author`
   and where the file is.
3. **A source chooses from our controls and glyphs; it never brings UI code.** A new kind of control is added to the
   app once, and then any node can use it.
4. **Built-ins have no private door.** Whatever a built-in needs (a gradient, a list of sites, a measured range, the
   bake), a custom can ask for with the same words. If a built-in cannot be written with the SDK, the SDK is what is
   extended.

The test that holds all four: paste each built-in's source in as a custom, and the node, its inspector, its legend and
its picture come out the same. Its control arm is a branch on the id.

## 2. What exists

- **Shader path, shared.** `render::assemble` takes a built-in's file and a custom's text alike, with `// @uniform` and
  `// @input` lines (gui_state_contract §3).
- **Description, shared (TASK-M7-76).** `render::assemble::declare`: `@name`, `@group`, `@about`, `@maps`,
  `@glyph`, `@control`, `@version`; every built-in declares itself, the generated ones through their generators (the
  debug catalogue, the field ramps, a field's source); declaration lines are outside the fragment and render keys, so
  a label edit changes no picture and compiles nothing. `engine::contract::stain::describe::describe` gives the editor
  one description, and each library entry carries its own. The occupant algebra's expression trees
  (`render::codegen`) do not declare yet: they come with gap 2.
- **The editor, shared (TASK-M7-77).** `crates/gui/src/stain` takes titles, glyphs, groups, the "maps" line, the
  choices and every control from the description (`stain::controls` draws them, `stain::catalogue` is the library), on
  the mock and the real side alike. The special editors of gap 3 are not controls yet.
- **Not shared: the colour algebra.** Site-blend, field-ramp and the combinators are to be Rust operations
  (`Primitive`) that generate WGSL, with inspectors written by hand. As planned they are exactly the private door rule
  4 forbids (gap 1).

## 3. Where the plan has to grow

Each row: what is missing, and what to do. The task that takes it is in brackets.

### Parameters and controls

| # | Gap | What to do |
|---|---|---|
| 1 | Parameters are numbers and short vectors only. A gradient with any number of stops, a list of sites with colours, a swatch set, a lookup table and a named field cannot be declared. The custom-stop ramp is a special field on colour nodes, outside the schema. | Add parameter types: `gradient`, `list<…>` with a declared maximum, `lut`, `field`, `image`. The ramp becomes the first gradient. (M7-78) |
| 2 | The algebra's nodes would have their own inspectors. | Each generates a source with declarations, so its inspector is drawn from them like any other. Structure (which kernel, how many sites) is a `choice` or a list; changing it may recompile, but the node is still the same shader. (M7-06, -07, -09, -10, built on M7-78) |
| 3 | The sphere editor, gradient bar and palette grid are special editors, so far reachable only by node kind and wired subtype (render_gui_spec Part II §9). | They become controls in the fixed set: any node with a direction list may ask for the sphere, any with a gradient for the bar. The bar is one (`gradient`) and the palette is its colours; the sphere is left. (M7-23) |
| 4 | Relations between controls: shown only when another is on, linked lo/hi pairs, a soft range inside the hard one, reset to default, units and number format. | Add `pair`, `soft [lo, hi]`, `unit`, `format` to `@control`. When a control matters is not declared: it is proved (gap 26). Reset and copy/paste of values come with every control, for free. (M7-78) |
| 5 | The editor draws the first eight parameters and drops the rest silently. | Done: controls scroll and fold by `group`; no cap. (M7-77) |

### What a node reads

| # | Gap | What to do |
|---|---|---|
| 6 | A colour node's inspector morphs with the wired subtype (§9), but nothing lets a node say which subtypes it takes. | `@input` gains a label, the accepted subtypes (scalar, vector, categorical, position), optional or required, and a unit. A wire it does not accept still attaches (§6's rule), with a Problems note and the nodes that do accept it offered. (M7-78, M8-20) |
| 7 | Needs are not declared: FTLE (tier-gated), ensemble spread, the word, a measured range, time, the previous frame's class, quad data. | Derived, not declared: the proof of gap 26 gives what the source can read. The editor says what is missing from it; the legend and the bake use it. (M7-79) |
| 8 | "Pure function of n̂" decides whether a node bakes to a texture (M7-16), with no way to say it. | Derived from what the source reads, never claimed by the author. (M7-79, M7-16) |
| 9 | Each occupant writes the invalid, not-a-system and not-yet-run treatment by hand. A custom that forgets shows bad pixels as data. | The assembler wraps every node with the standard treatment unless the source says it handles validity itself. Built-ins drop their copies; pictures bit-identical. (M7-79) |
| 10 | The legend samples a node over its input domains, but has no categories, units or tick names for a custom. | The legend reads `@input` units and `@maps`, and a categorical input's names from the ledger. (M8-22) |

### Writing one

| # | Gap | What to do |
|---|---|---|
| 11 | Eject greys the node's controls (colour_composition §5, render_gui_spec Part II §9; M8-21). Under rule 2 that is wrong: an edited node still declares them. | Controls stay live after an edit. One that the edited code provably cannot use is dropped (gap 26), its value kept in case the code uses it again. Revert to generated stays. (M7-14, M7-89, M8-21) |
| 12 | Editing a declaration: what happens to values already set. | Matched by name. A value outside a new range is clamped and said so; a changed type takes the default; a removed one is remembered until the edit is undone or saved. `was <old name>` carries a value over a rename. (M7-14) |
| 13 | While a declaration is half typed, the node must not lose its controls. | Last valid description as well as last valid pipeline, per node. Declaration errors and WGSL errors go to the same Problems list, with the author's own line numbers. (M7-14) |
| 14 | No way to start one. | New from template (one per node kind), and Duplicate on any node, which copies its real source. Either makes the shader at once, named, in your shaders, and the node is that shader. (M7-14, M7-80) |
| 15 | No guide, and no check outside the app. | The guide's reference section is generated from the grammar's own tables, with the complete example (the fixture) beside it. `prin node check <file>` runs the same parser and compiler, for an author's editor. (M7-82) |

### Settings, code and saving

A node in a stain is a shader and its settings. They are kept apart.

| What you do | What happens | A new shader? |
|---|---|---|
| Move a slider, pick a colour, flip a toggle, edit a gradient or a list | The stain's value for that node changes. A plain uniform recompiles nothing; a structural setting (a list's length, a kernel) may recompile, its shader still the same one. | No |
| Undo, or Reset | The value before, or the shader's default | No |
| Save the stain | The stain keeps the values | No |
| **Save as a node…**, on a node's menu | A new shader of yours: the same code, your values its defaults, a name you give | Yes, because you asked |
| New, or Duplicate | A new shader of yours, named | Yes |
| Edit the code of a node that is not yours | The stain holds an edited copy, unsaved and marked so in the shaders list. Save puts it among your shaders; Revert goes back | Yes, at the first edit, in this stain only until saved |
| Edit the code of your own shader | It changes when you save it, not as you type. Other stains that use it are offered the update (gap 17) | No |

### Keeping and sharing

| # | Gap | What to do |
|---|---|---|
| 16 | A custom is text inside one stain. It cannot be reused, named or updated. | **Your nodes**: files in a user folder, scanned as the built-in folder is, listed by `@group` beside ours. `@id`, `@author`, `@licence`, `@node_version`. Import, export, show in Finder. (M7-80) |
| 17 | What a saved stain holds when the node's file later changes. | The source by value, always, with the id and version as a label, so a picture never changes by itself. When a newer file is found the node offers Update. (M7-80) |
| 18 | The library lists whole stains and is "not a node source" (§7); nothing lists nodes to add. | The right-click palette and the drag-off-a-pin menu list ours and yours by group, with search. The library stays for stains. (M8-20) |
| 19 | Two nodes with one id, or a name of ours. | Ours are under a reserved prefix. Yours may share a name, never an id: the second import is asked to replace or keep both. (M7-80) |
| 20 | Someone else's stain can carry a shader. WGSL cannot touch files or the network, but it can hang the GPU, and no document covers that. | A source from outside does not run until the owner of the machine allows it; until then the node says so and its slot's default is drawn. Loops are bounded by a check on the parsed module, and a lost device brings the app back with that node off. (M7-81) |
| 21 | The picture's embedded record drops custom sources first (image embedding, tier 3). | Unchanged. The record keeps the node's id, name and controls' values, so the fallback can say which node is missing and a machine that has it can put it back. (M7-31, M7-80) |
| 22 | The grammar will grow. | `@version`, read by every build. An older source is always read; a newer one is refused with the version named. The read-side names a source may use are part of the version. (M7-76, M7-82) |

### Proving what matters

| # | Gap | What to do |
|---|---|---|
| 26 | Nothing knows which controls, inputs, fields and nodes can change the picture, so dead ones are drawn, compiled and computed. | A static proof over each shader and the graph, below. What provably cannot change the picture is dropped. (M7-89) |
| 27 | Code view is a dead end once someone edits the whole pipeline (render_gui_spec Part II §10). If nodes are a view of the shader, a shader should open as nodes. | The assembled text keeps each node's function and declarations, so an edit inside a function comes back as that node, and a pasted shader made of declared functions opens as a graph. Only text with no such form stays code. (M7-90) |

The proof reads naga's IR, which the assembler already builds to find the fields a stain reads. Three questions, each
answered for a shader once and kept under its code key:

1. **Can it reach the output at all?** For every uniform, input, stored field, and time, camera and pixel position:
   whether any path of data or of control (a branch, a loop bound, a discard, a write through a pointer, a call) leads
   from it to the returned value or a side effect. If none does, its control, its port's wire and the work behind it
   are dropped: the uniform leaves the block, the node feeding only dead ports is not emitted, and a field nothing
   reaches (FTLE, say) is not asked for.
2. **Can it reach the output with these settings?** The same, with every `choice` and `toggle` held at its value and
   branches on them pruned. A control that cannot matter now is dimmed, with the setting that would make it matter.
   Nobody has to declare when a control is shown.
3. **What can make the picture change?** The union, over the graph: whether it can change with the playhead, the
   camera, the zoom or nothing. This gives the bake (pure in n̂), what has to be redrawn, and what a tier must compute.

It is one-sided, on purpose. **"Cannot change the picture" is proved; "may change it" is not a promise that it does**:
`x - x` counts as using `x`. Where the code cannot be followed, the answer is "may". No algebra is used: `0 * x` is
not 0 when `x` is NaN, so a term is never dropped for being multiplied by zero, only for being unreachable.

Tests: every uniform, input and field judged dead is set to random values, NaN and infinity included, and the picture
does not change by a bit; every shipped node with a control that can never matter fails the build. Controls: an
analysis that forgets branches; one that forgets writes through a pointer; one that folds `0 * x`.

### Shaders that are not nodes

Every node is a shader; not every shader is nodes. A shader that declares nothing, or cannot be taken apart, still runs.
The graph is a view of a shader where there is one, never the price of running it.

| What it is | How it shows | What it still gets |
|---|---|---|
| **Declared functions** | A graph of nodes (gap 27). | Everything. |
| **One of our function signatures, declaring nothing** | One node. Its controls are its uniforms, with default controls and their own names. | The proof, the standard validity treatment, inputs by wire. |
| **A whole `shade(ctx)`** that cannot be taken apart | One box feeding `OUT`, code inside. Post nodes may follow it. | The proof, the validity treatment, its uniforms as controls. |
| **A foreign fragment shader**: its own entry point, WGSL, or GLSL and SPIR-V through naga's front ends | One box, code inside, run in its own module through an adapter that maps resolution, time, pointer and, where it asks, our fields. | The validity treatment, applied around it. The proof, as far as its module parses. |

| # | Gap | What to do |
|---|---|---|
| 28 | The assembler takes only our signatures: names it reserves, its own prelude, one module. A foreign shader's names, bindings and entry point clash with it. | A foreign shader is never spliced in. It is compiled as its own module and drawn as its own pass, its output the box's colour. The adapter's bindings are the whole of what it can read. (M7-91) |
| 29 | What must hold even then. | Invalid and not-a-system pixels are masked after it, from the data, so no shader can paint over them. Picking, the Inspector, measures and exports of data read the data, never the picture. The legend says "this shader has no legend" unless it declares one. The proof says when a picture reads no data at all, and the status line repeats it. The safety of gap 20 applies: asked once, loops bounded, a lost GPU recovered. (M7-91, M7-81) |
| 30 | What is refused, with the reason in Problems and the last good picture kept. | Compute shaders, storage writes, several passes feeding each other, and textures or buffers the adapter does not have. Several passes may come later, as a Print graph. (M7-91) |

### The same idea elsewhere

| # | Gap | What to do |
|---|---|---|
| 23 | The Print graph reuses the Stain's editor (M8-69) with nodes of its own. | The same declarations and the same description. `temporally_stable` becomes one of its words. (M8-69) |
| 24 | Chart position's base and layers (M7-51) are planned as a special inspector. | A list control whose items are nodes' descriptions (gap 1). (M7-51) |
| 25 | Integrators and charts are Rust, built in advance, and cannot be written in the app (gui_state_contract §5). | That stands. Their options can be described with the same control set, so Run and the Chart builder draw them the same way. Later, and not part of this. |

## 4. Moving the built-ins onto the path

The owner, 24 Nov 2026: all the built-in nodes use the same path as custom ones. This is where each stands in the code
and what moves it.

| Built-in | Its path today | Moves to | Task |
|---|---|---|---|
| The registry's 87 sources | Scanned from folders, and the only table: a graph names any of them by its id (`registry::named`), a file under a slot's directory by its name there (`pass_through`), any other by its whole id (`debug/generated/state`). The zero source the debug views read is a shipped file, `source/zero`. | Done. | M7-83 |
| Keys and saved stains | Keyed by code, whoever wrote it. Saved as id, version and hash, plus the text (`engine::contract::stain::saved`); a node whose shipped file has changed draws its saved text and is offered the update. | Done. | M7-84 |
| Source nodes (`ctx.<field>`) | WGSL written inside the assembler, one per field. | One shipped source, `field`, with a `field` parameter. | M7-85 |
| Field ramps | Built by Rust (`colour/field_ramp.rs`), one shader per field, 42 of them, each handling its own invalid pixels, with a CPU mirror the goldens check. Placed in graphs as custom text, and not in the app's library at all. | One shipped source, `field_ramp`, with input, gradient, curve, polarity and range as parameters, so the ramps appear in the app. The generator goes; the CPU mirror stays as the check. | M7-86 |
| The outcome palette | Its colours are its source's declared defaults: `SWATCHES`, the sets, the field's image and the legend read them there (`render::colour::outcome::Palette`), and the source is found by its `@maps outcome class` line, never by its id. | Done. | M7-87 |
| Combiners, margin, boundary lines, fallback tint, pending hatch | Plain source files, named by id from the registry. | Done. | M7-83 |
| Generated debug views | Generated from the field ledger, which is right: one source of truth. They carry declarations, and the library names them by id. | Done. | M7-76, M7-83 |
| Site-blend, gradients, combinators, overlays | Planned as Rust operations with their own inspectors. Not built. | Shipped sources on M7-78's parameters. Where a shader must be generated (a chosen kernel, a list's length), the generator takes a declared source and is open to a custom too. | M7-06, -07, -09, -10 |
| Presets | Graphs built in Rust (`presets.rs`, `structural.rs`), with small shaders as strings inside. | Saved stains as files, read as a user's are. Their inline shaders become shipped sources. | M7-17 |
| The mock engine's stain | The shipped registry's names, descriptions and source text. Its assembled code and compile times are still invented: the mock assembles nothing. | Done. | M7-77 |
| Print graph nodes | Declared as data already, in JSON, in a format of their own. | The same words and controls, so one inspector draws both. | M8-69 |
| Legend line for FTLE | A special case on one field. | A declared note on the input. | M7-79, M8-22 |

**What stays off the path, and why.** `OUT` and the empty (None) node are the graph's structure, with no code to write.
The shared library (colour spaces, ramps, the read side) is what sources call, not a node. The compositor, the display
chain and colour-vision simulation run after the stain on finished pixels. View › Debug view is the instrument's check
on the stain, so it must not be something a stain can replace. Physics is Rust, built in advance.

**How each step is proved.**

- The code every library row assembles to is pinned (`declared_code.txt`). M7-83 to -85 and -87 may not change a byte of
  it, and no golden may move. M7-86 replaces many files by one, so its pictures are compared pixel by pixel with the
  old ones, on the zero view and on fixtures holding every invalid, unset and infinite value. A difference is explained
  or it blocks.
- **The final proof (M7-88).** Start the app with its shipped sources removed, and the same files given as the user's
  node folder. The library, inspectors, legends, pictures and presets are the same. If anything needs a source to be
  shipped, this fails.
- **The lock (M7-88).** A lint refuses a shipped id or a match on "built-in" outside the registry and the saved-form
  reader, so a private path cannot come back.

Old saved stains keep opening. The reader takes the three old forms (`builtin`, `field`, `custom`) and writes the new.

## 5. Order

1. **M7-76**, the description.
2. **M7-83**, one registry. Before the editor work, so the titles are right the first time.
3. **M7-77**, the editor draws every node from its description. The visible step.
4. **M7-84**, keys and saved form, then **M7-80**, your shaders. Before anyone can write one.
5. **M7-14**, writing and editing WGSL in the app, with gaps 11 to 14. **M7-89**, the proof of what matters, beside it.
6. **M7-78**, richer parameters; **M7-79**, the standard validity treatment and the needs the proof gives.
7. **M7-85**, **M7-86**, **M7-87**: sources, ramps and the palette. Then the algebra (M7-06, -07, -09, -10) and the
   presets (M7-17), built on the path from the start.
8. **M7-90**, a shader opens as a graph. **M7-81**, other people's shaders, then **M7-91**, foreign ones. **M7-82**, the
   guide and the checker.
9. **M7-88**, the final proof and the lock.

## 6. Settled

The owner, 24 Nov 2026:

- The colour algebra goes through the SDK (gaps 1 and 2).
- Controls stay live after a code edit, and what provably does nothing is dropped (gaps 11 and 26).
- Someone else's shader asks once before it runs (gap 20).
- Saved stains hold the source by value; an update is offered, never applied (gap 17).
- Changing a node's settings never makes a shader; saving it as one does.
- Shaders that do not support nodes, or cannot be turned into them, are still supported.
- No badge on a node: a custom node is a custom shader, and the shader is what says whose it is.

Calls still open to change:

- A wire a node does not accept still attaches, with a note (gap 6), as the editor's rule says today.
- A share link and a picture's low-bit record are too small for sources, so they hold a shipped node's id and hash, and
  say so when the source has since changed.
- The field ramps become one node (M7-86). It is the only step that replaces shaders instead of moving them.
- A pasted shader opens as a graph where it can (gap 27), which goes further than the older design.
- A foreign shader runs as one box through an adapter (gaps 28 to 30); GLSL and SPIR-V come in through naga, HLSL does
  not.
