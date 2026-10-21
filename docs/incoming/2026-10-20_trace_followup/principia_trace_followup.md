# Follow-up to the extension and trace bundle: where a trace can leave a chart, playback, the time ramp

*20 Oct 2026, after `principia_extension_trace_bundle.zip` was sent. It amends that bundle's
`principia_trace_markers.md`: F1 replaces its §3a, F2 adds two bug classes, F3 and F4 change §1 and §2, and F6's tests
join its §11. Quote §0 verbatim in the ruling. The demo, `demo/trace_markers.html` (version 27), replaces the bundle's
copy; it adds four odd charts that exercise F1 and F2.*

---

## 0. My words

> "btw how could the trace ever leave the shape sphere chart? n hat is always defined along with theta and phi ?"

> "yeah but what about charts that have only one of the angular compents as an axis or some combination of theta plus
> something else for an axis etc? means there is a bug with how you are calculating the trace ?"

> "yeah a follow up, also update the artefact to have more weird / whacky charts so i can see how it behaves, then we
> can give the updated version to claude code. already sent the previous bundle off"

> "also allow the user to change the colour ramp along time, i.e. the gradient used"

> "also if the trace isn't being animated/played the full trace up to the horizon time should be drawn always"

---

## F1. Where a trace can leave a chart (replaces §3a)

**The trace point is the state projected onto the chart's actual axes** (`principia_trajectory_viewing.md` §3). Two
rules follow.

1. **Hidden coordinates never make a trace leave.** Anything orthogonal to the chart's plane feeds only the
   off-slice distance, so it shows as the fade. Where that distance has no value, R-509 applies: no fade, and the GUI
   says so.
2. **A trace leaves only where a coordinate entering a shown axis with non-zero weight has no value.** That means:
   - ±∞ from a saturated link;
   - NaN;
   - a bounded axis's inverse outside its domain.

   Moving off the slice is never leaving.

So whether a trace can leave depends on the chart's axes, not the chart's name:

| Shown axis | Can it make the trace leave? |
|---|---|
| θ or φ of the shape sphere, alone or in any mix | **No.** `n̂` exists for every shape with `I > 0`; `I = 0` is a total collision, which has already ended the run. θ is undefined at the poles, a coordinate singularity the line passes as a jump. |
| A mass or mass ratio | **No.** Masses are constant along a run. |
| A physical quantity (`E`, `L_z`, `K`, a momentum, the dilation rate) | **No**, unless its warp's inverse has no value. `K ≥ 0` always, and R-508 continues `t > 1`. |
| A curve axis (Burrau's ν) | **No.** The encode projects onto the nearest point of the curve (inverse_encode_contract Part 5(b)), and the distance to the curve is fade. If the nearest point lies past a bounded edge, the point is not drawn there: those are real systems the bounded axis does not show (§8, the domain's end). They are counted apart from the not-a-system count, whose non-zero value is a bug; this one is not. |
| A latent coordinate behind a saturating link (`z_q` past `q_max`, `z_α` past the decoder's α limits) | **Yes**, exactly when that coordinate has weight in the shown axis. |
| `z_β` at a syzygy | It runs to ±∞ and comes back. That is a jump at infinity, not a sample off the chart. |

**A tilted axis is a mix.** The shown coordinate is a weighted sum of the chart's parameters and the tilt target
(R-473's rotation). It can leave only through a term with non-zero weight:
- **Tilted toward a latent momentum** `z_q`: the axis carries `z_q` with weight `sin τ`, so past `q_max` it has no
  value and the trace leaves.
- **Tilted toward the physical momentum:** the same tilt never leaves.

The demo's tabs E and F are that pair.

---

## F2. Two bug classes this rule exposes (new, with tests)

1. **Whole-encode validity.** The inverse encode contract's validation (Part 6, the three layers) is written for
   lookup and lock: it clamps, projects or rejects the whole state if any component fails. **The trace must not
   reuse it.**
   - A trace that does so vanishes whenever any hidden component is out of range, for example a momentum past
     `q_max` on a shape chart.
   - The trace checks each shown axis on its own (F1).
   - The demo had this bug, and drew leave and re-enter rings on the shape sphere.
2. **θ inside a mix.**
   - If θ is taken in `[0, 2π)` at each sample and then rotated together with another coordinate, the tilted
     coordinate jumps by `2π·cos τ` at every wrap. A pole passage does the same, through θ + π. Both jumps are
     artefacts.
   - The correct order is three steps: take the state's chart parameters on the branch nearest the previous sample
     (θ ± 2π; θ + π with φ reflected over a pole); then mix; then draw in every image by the extension types.
   - On a tilted chart those branches are **not** repeats of one image. θ wraps but the target does not, so a shift
     of `2π / cos τ` along the tilted axis is a different system. Such a chart is affine along that axis and does not
     repeat.

3. **The tilt target read in a different frame from the chart's parameters.** Found building the demo's tilted
   sphere.
   - The encode returns momenta in the **canonical** frame, which mirrors whenever the shape is in the lower
     hemisphere (`β ∈ (0, π)`, the latent chart's section).
   - The full shape sphere keeps both hemispheres, so its `(θ, φ)` are read **unmirrored**.
   - Mixing the two flipped the sign of `p_y` at every syzygy, so the tilted trace jumped there: up to ~2000× the
     median step.
   - Read in the sphere's own frame (`ρ` along `+x`, no mirror), the worst jump falls to 5–97× the median, the size
     of genuine fast motion at close approach.
   - **Rule:** every coordinate in a mix is read in the frame the chart's own parameters are defined in.

---

## F3. A trace that is not being played is drawn whole (amends §1 and §2)

- **Not playing or scrubbed:** when the playhead is not playing and is not being scrubbed, every trace is drawn from
  `t = 0` to its end (its event, or the horizon). Under §2 a paused trace was cut at the playhead.
- **While playing, or while the playhead is dragged:** the trace is drawn to the head, as §2 says, with the head
  marker.
- **Static mode** draws the whole trace, as now.
- **The own-clock mode** is always animated.

---

## F4. The colour ramp along time is selectable (amends §2's Colour row)

- **Presets:** Sunset (the default, `#FFF27A #FF8A5B #E0457B #7A4CFF #3BC7FF`), Viridis, Magma, Ice, Ember, Aurora,
  Mono.
- **Custom:** three colour stops.
- **Reverse:** reverses the ramp.
- **Preview:** a preview bar shows the ramp.
- It applies to the line and its time ticks.
- It is part of the trace style, so it is kept with kept orbits, saved with the view, and travels in exports (the
  round-trip rule's tier 2). The demo holds one ramp for every trace.

**Not the field.** It is not the field's palette. A Chazy or outcome palette is a stain (the node graph); this ramp is
the line system's.

---

## F5. The odd charts in the demo

Each was chosen to exercise F1 or F2. Each has a prin-rs outcome map integrated with the demo's own decode
(`reference/prin_rs_examples/demo_maps.rs`, modes `tiltz`, `tiltp`, `thetah`, `nuh`).

| Tab | Axes | What it shows |
|---|---|---|
| **E. Sphere tilted into z_q3** | `s`: `θ = π + s·cos 30°`, `z_q3 = (0.6/π)·s·sin 30°`; φ untilted | A tilt toward a **latent** momentum. A trace leaves (dotted ring, dashed line) while `|p_λ,y| > q_max` and comes back. Branch continuity keeps wraps and pole passages from jumping. |
| **F. Sphere tilted into p_λ,y** | the same with the **physical** momentum | Never leaves. Where E loses the point, F runs far out and the edge pins its icons. |
| **G. θ × dilation rate** | θ (φ fixed at 1.1) × the scale-free homologous rate `h = (Σ m r·v)/I · I^¾` | A mixed chart: one angle and one physical quantity. Periodic in θ, so it repeats sideways; affine in `h`. Never leaves; the shape leaving the circle and the non-homologous motion are fade. |
| **H. Right triangles ν × dilation** | the Burrau family's right triangles (equal masses) × `h` | A curve axis. The trace point is the nearest ν and the distance to the curve is fade. ν is bounded: past its edges the chart ends, blank, and a point projected there is not drawn. |

The four original tabs stay: latent `z_α × z_β`, shape sphere (with the hemisphere toggle), mass simplex and latent
momenta.

---

## F6. Tests (each with the arm that fires)

| # | Test | The arm that fires |
|---|---|---|
| T14 | Shape-axis charts never lose a trace to a hidden coordinate: on the latent `z_α × z_β` chart, a run whose momentum passes `q_max` keeps every sample's point | reusing the lookup/lock validation drops those samples |
| T15 | A latent-momentum tilt leaves exactly while the target's `|q| ≥ q_max`; the physical-momentum tilt never does, over 100 runs | a whole-state check (leaves on any component) or none (never leaves) |
| T16b | Frame: on the sphere tilted toward `p_y`, the trace's screen step at a syzygy is within 3× the median | reading `p_y` in the canonical (mirrored) frame flips it at every syzygy |
| T16 | No false jumps: on the tilted chart, the trace's screen step at a θ wrap and at a pole passage is within 3× the median step | per-sample `[0, 2π)` θ gives a jump of `2π·cos τ` |
| T17 | The tilted chart does not repeat: the systems at `s` and `s + 2π/cos τ` differ | drawing it with periodic repeats |
| T18 | The curve chart's trace is the nearest ν: perturb a run's shape off the curve, the point stays and the fade grows | dropping or clamping the point |
| T19 | Paused: a pinned trace draws to its end; playing or scrubbing, it draws to the head | cutting a paused trace at the playhead |
| T20 | The ramp round-trips: a kept orbit's ramp survives save and load, and Reverse reverses it | a ramp held only in GUI state |

---

## F7. Queue, don't rule

- **Q5.** On a tilted chart with a periodic parameter, should the other θ branches (θ ± 2π) also be drawn, each faded
  by its own out-of-plane distance, since they are the same system projected at different distances?
  *Recommendation:* no by default; an option. The demo draws the continuous branch only.
- **Q6.** The ramp's default: Sunset, as the mock-ups, or a perceptually uniform one (Viridis)? *Recommendation:*
  Sunset, which reads on the dark field; Viridis's dark end vanishes on it.
