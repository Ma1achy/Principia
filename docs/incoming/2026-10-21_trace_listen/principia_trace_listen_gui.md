# The trace and Listen in the dev GUI

*21 Oct 2026. New. It places the trace (`principia_trace_markers.md` and its follow-up) and Listen
(`principia_listen.md`) in the egui dev GUI's Explore mode. The art boards in `artboards/` are the layout reference;
this file is the rule where they differ. It amends the dev-GUI sketch: the Trajectory panel, the menu bar, the left
panel and the bottom band. Quote §0 verbatim in the ruling.*

---

## 0. My words

> "improve the gui of the trace sound and trace artefacts to better match the gui of principia (you made an artefact a
> while back that was a load of the art boards of the different gui elements / windows for principia (not the webpage
> version.. the dev gui) i need the trace and trace sound stuff to be properly integrated in and designed well with the
> rest of the gui"

> "I don't know, with the like artboard stuff, should this be integrated with the trajectory viewer stuff potentially?
> … we already have that like trajectory viewer thing where like on hover it shows you the shape sphere in of the
> trajectory, and it also shows you the trajectory in real space in those two preview windows next to each other. So
> should we have the stuff to do with the trace below? Maybe either you click on it and it like expands, and then also
> as well we could have the sound thing as well as like a button, and when you click on that it then opens up a window
> where you can like do all the sound stuff, and then maybe as well it like changes the it like opens up another thing
> where like you can see like the spectrogram or something."

> "Yeah, please redo the artboards along these lines and also make them look consistent with how the trace should look,
> along with also how, and also make sure it has all the features for the trace and all the features for the sound
> stuff."

> "Just maybe it needs a slightly bit more work in just considering how it's going to fit in the explore mode with the
> rest of the GUI, such as the NavCube compass and the legend and play bar at the bottom."

---

## G1. The principle

**Everything about one trajectory lives in one place: the Trajectory panel.** That covers its previews, its trace and
its sound. The view shows the trace, and the bottom band's Time column shows the trajectory over time.

**Every setting appears in at most two places**: an everyday control in the panel, and the full set in a window
(`Trace…`, `Listen…`), opened like `Profiler…` and `Export…`. Nothing about a trajectory is in the left panel, which
is about the chart.

## G2. Menus and keys

- **`Overlays ▾` gains a Trace group**, after Quadtree, Integration, Chart and Stain, with these items:
  - trace under cursor (`T`);
  - kept traces (`⇧T`);
  - syzygies (`Y`);
  - end event icon;
  - off-screen icons, pinned to the edge;
  - repeat in wrapped copies;
  - outline the chart's square;
  - a `Trace…` button.

  A trace is an overlay, so it lives with the overlays.
- **No `Lines ▾` and no `Listen ▾`.** An earlier art-board pass had both; they are withdrawn.
- **Keys**: `T` trace, `⇧T` kept traces, `Y` syzygies, `K` keep the current trajectory, `L` sound on/off. Add them to
  the keyboard sheet.

## G3. The left panel

Chart, Navigate, Centre `z₀`, Slice & tilt. **No trace controls.** The Compass moves back to the bottom band (G7),
where the sketch has it.

## G4. The Trajectory panel

From the top:

1. **Header**: `Trajectory`, then tabs `under cursor`, `#1`, `#2`, …, `+ keep`. **Each kept trajectory is a tab**, with
   its own previews, trace and sound. The kept list moves here and into `Trace…`, and out of the left panel.
2. **Status line**: running / pinned, the end event with its time, `|ΔE/E|`.
3. **Previews**: real space (bodies coloured 1 yellow, 2 magenta, 3 cyan) and the shape sphere, side by side as in the
   sketch, with `sphere / unwrapped` and `turn`. **While Listen is on, the stretch of the shape-sphere path inside the
   analysis window is drawn brighter** (`#5ab4ff`, width 2.4), so what is being heard is visible.
4. **Three collapsible sections**, each a single row when collapsed.

| Section | Collapsed row | Expanded |
|---|---|---|
| **Trace** (collapsed by default) | switch · the line's colour swatch · summary (`to the head · ticks 2 · left 0×`) · `⋯` opens `Trace…` | show (up to the head / tail only, `Δt`), colour + swatch, ramp, width, time ticks + labels, arrows, then **marker chips**, then icon set |
| **Listen** (collapsed until first used, then remembered) | `♪` on/off · instrument · `⋯` opens `Listen…` | instrument, volume, listen mode, loudness, event and effect chips, a small spectrum at the head, the two strongest lines as notes with their sides, interval, entropy |
| **Numbers** (collapsed) | — | F₂ word, substeps, min sep (the sketch's existing numbers) |

The marker chips are toggle buttons that show the icon they draw: end event, syzygies, leaving the chart, off-screen
pins, wrapped copies, head marker. Event and effect chips are the same idea, with short glyphs.

## G5. The `Trace…` window

**Every trace setting** in `demo/trace_markers.html` and `demo/trace_sound.html`, with their defaults, under a
preview strip drawn with the current settings. It applies to `all traces` or to the current tab only.

| Group | Controls (default **bold**) |
|---|---|
| Line | colour **Fixed** `#fff27a` / Ramp along time; ramp **Sunset** · Viridis · Magma · Ice · Ember · Aurora · Mono · Custom… (three stops), `rev`; width **2.2**; **fade off the slice** (momentum proxy, R-509, floor 0.18); time ticks Off · Every 1 · **Every 2** · Every 5, **labels** |
| Direction arrows | shape Off · **Filled triangle** · Open chevron · Half barb · Notched · Dot; size **8 px**; every N px of path · **Δt of time** · N per trace; value (**Δt 1.0**) |
| Animation | mode Static · **With the playhead** · Own clock; repeat Once · **Loop** · Back and forth; speed **3 t/s**; tail only (off); tail Δt **3.0**; show the rest faintly (off); **head marker, labelled with its time** |
| Markers | **end event icon**; syzygies, coloured by the middle body (**off**) with style From the icon set · o–o–o · Three in a row · Line and bead · Numeral tag · Crossing tick · Eclipse, and **fade stutters**; **leaving / re-entering the chart**; leaving / entering the window (off); seams (off); join jumps dotted (off) |
| At the edges | **off-screen icons pinned (faint)**; **repeat in wrapped copies**; **outline the chart's square**; forbidden hatch (off, R-470) |
| Icons | icon set **Principia** · Arrows · Pictures · Orbits, `key…` opens the icon key; icon size **1.4×** |
| Kept traces | one row each with its swatch, end and `×`; `clear kept` |

Footer: `reset`, `save as default`. When nothing is playing or scrubbing, the whole trace to the horizon is drawn
(follow-up F3).

## G6. The `Listen…` window

**Every sound setting** in `principia_listen.md`, in two columns:
- left: Sound, Loudness, Listen;
- right: Pitch & analysis, Events, Effects.

A readout row runs under both.

| Group | Controls (default **bold**) |
|---|---|
| Sound | **♪ sound on**, `sweep a row` + speed (**10 s**), known orbit (— · figure-eight · figure-eight 1% · Burrau 3:4:5 · Lagrange), instrument (**Warm sine**), voices (**16**), strikes on (**each syzygy** · every crossing time ÷ · only when a line changes) with ÷ **2** |
| Loudness | **follows the shape's movement** / normalised; min level **15%**; max **100%**; volume **60%** |
| Listen | **follow the head** / whole run; horizon t ≤ 20 · **60** · 150 · 400 |
| Pitch & analysis | signal **angles + rotation** · angles only · shape vector; channels **stereo by winding** · stereo θ/φ · mono; window **fixed, centred** · grows · grows, last half; fixed × t_c **6.0**; 1/t_c is 110 · **220** · 440 Hz; **fit to hearing**; **spectrogram strip in the Time column** |
| Events | **close approach**, **collision by pair**, **escape / ejection**, syzygy tick (off); event level **80%** |
| Effects | each a switch + amount slider + value: **reverb ← spread (×1.40)**, Doppler ← expansion (off, ×1.50), bit-crush ← entropy (off, ×0.60), **width ← angular momentum (×1.50)** |

**The instrument menu is grouped**: pure (5), keys (3), struck (10), bowed and blown (7), voices (4), synths (8),
textures (2), one instrument per row in the build. The art board compresses rows to fit.

## G7. The bottom band

The sketch's band stays: **Compass · Time · Legend**, 196 px, with the Compass at the Explore left panel's width and
the Legend at 360 px. Listen adds to the Time column and nothing else.

| State | Band | Time column | Compass | Legend column |
|---|---|---|---|---|
| Sound off | 196 px | transport, scrubber, axis, as now | full | Legend |
| **Sound on** | **196 px, unchanged** | an **event lane** and a **mini spectrogram** (≈54 px) fill its spare height; `♪ strip ⏶` | full | Legend + the trace section (G8) |
| **Strip expanded (⏶)** | **grows upward to 420 px**; the view gives up the height | the full spectrogram on the scrubber's own time axis, with the window, signal and channels shown, and `Listen…` | **cube only** | **the spectrum at the head**, until `⏷` |

- The spectrogram and the scrubber **share one time axis**, so the playhead is one line through both. The spectrogram
  is blank past the run's end, which is labelled.
- `F3` still hides the whole band.

## G8. The Legend's trace section

While a trace is shown, the Legend gains an `on the trace` block under the stain's classes: the line (and each kept
trace's colour), the start star, and the end icon for this run. With sound on it adds `♪ ◀ left ▶ right`. The full key
is `Trace… › key…`.

## G9. The trace's look (consistency)

Every board and the build draw the trace one way, the demo's defaults:

- line `#fff27a`, width 2.2, over a dark under-stroke `w + 2.2` (the demo's `pass` halo), so it reads on any stain;
- time ticks every 2, labelled;
- filled-triangle arrows every Δt 1.0, 8 px;
- the start a yellow star;
- the head a white disc labelled with its time;
- icons at 1.4×.

Syzygies are off. Kept traces take distinct colours: kept #2 is `#5ab4ff` in the boards. Spectrum and spectrogram
colours are `principia_listen.md` S11's.

## G10. Tests

| Test | Passes when | Fires on |
|---|---|---|
| One place | no trace or sound control exists outside the Trajectory panel, `Overlays ▾ › Trace`, `Trace…`, `Listen…` and the Time column | a stray control (e.g. a surviving `Lines ▾`) |
| Band height | turning ♪ on changes no pixel of the view's size; expanding the strip changes it by exactly the band's growth | the strip pushing the view on sound-on |
| Shared axis | the spectrogram's playhead x equals the scrubber thumb's x at every `t` | two axes |
| Window highlight | the bright arc on the shape sphere covers exactly the samples in `windowAt(t)` | an off-by-window highlight |
| Tabs | switching tabs switches the trace, previews and sound together | the sound following the cursor while a kept tab is selected |

## G11. Queue, don't rule

- **Q6. Listen's default collapsed state.** Recommendation (a): collapsed until first used, then remembered per user.
  (b) Always collapsed.
- **Q7. The expanded strip's spectrum in the Legend's column.** Recommendation (a): borrow the column, as G7. (b) Keep
  the Legend and stack the spectrum under the spectrogram, making the band ≈520 px.
- **Q8. Kept-trace colours.** Recommendation (a): a fixed cycle of six, distinct from the body colours and the stain
  classes; to be chosen against the stain palettes.
