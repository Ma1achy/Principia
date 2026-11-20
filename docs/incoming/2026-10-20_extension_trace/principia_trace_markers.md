# Trace markers and icons: an update to `principia_lines.md` §8

*Written 20 Oct 2026 for the orchestrator. It is the human's instruction from one design session on the trace demo
(`demo/trace_markers.html` in this bundle, version 25). Quote §0 verbatim in whatever rulings it becomes. It amends
`principia_lines.md` §8 ("Traces") as adopted by R-476, and R-509's answers stand: the trace fade uses the proxy where
it is defined, else no fade and the GUI says so; trace labels win. Bodies are numbered 1, 2, 3 on screen; the corpus's
ids stay 0-based (R-22).*

---

## 0. My words

These are the session's messages that decide something, in order. "…" marks text that was not preserved word for word.

> "Idea, if the trace line leaves the current slice and re enters… … special icons for those and colour them based on
> the collision / ejection detail :)"

> "Oh make it so it draws everything up to the current t by default rather than having a fixed tail length"

> "Make the icons for the events more graphical and better convey the event? Maybe? But again be minimalist. Custom
> icons would be cool"

> "Try these also: Collision == two arrows pointing at an explosion shape? Triple collision == three arrows … Single
> Escape == arrow leaving pointing away from circle; Double scape == two arrows …; Triple escape == threee arrows …
> Plus maybe other ideas. Wtf is a syzygy btw"

> "Maybe syzygy is just like o-o-o or something?"

> "Yeah but color them differently LOL and have the lines be a different colour."

> "Icons for the start of the trace, the end and when it enters and leaves?"

> "Start icon should be a star?"

> "Remove double escape. Also would incomplete be used?"

> "Incomplete should be a "?""

> "Circled what I want the default icon set to be" *(a screenshot circling the Arrows set's event icons and the Orbits
> set's leave and re-enter rings; that combination is the default, "Principia", §4)*

> "Don't think the syzygy icon is centred on the trace properly btw"

> "Btw with the trace stuff I'd turn off sygzgy by default since it adds extra noise to the image and also add an
> option to not draw icons at the edge if they aren't actually in the view port or whatever. Plus the trace system
> needs to work with the axes extension when we turn off f3 :)"

> "Okay but now the icons don't show when the trace and icons go off screen/view port? You can't see them at the
> edges?ans there's no option to enable / disable that"

> "I don't understand the shape one hemisphere chart? Why is that a thing also surely the latent charts all have valid
> extensions? A better one without an extension would be the mass simplex, also the trace system needs to handle
> forbidden regions etc I.e. not actually going through forbidden regions or being able to start in one. Maybe if you
> try start in a forbidden region a red cross / X is shown where you click?"

> "Also on things like the shape sphere with the domain wrap all of the icons etc should repeat"

---

## 1. What changes in `principia_lines.md` §8

| §8 today | Becomes |
|---|---|
| Markers: "Start ●, collision ×, escape →, horizon ▪" | The icon sets of §3 and §4. Start is a star, each end event has its own icon coloured by its outcome's detail, and incomplete is "?". |
| Length: "To the playhead (the default), a fixed window, or the last `Δt`" | Unchanged. The default draws everything from `t = 0` to the head. The tail (a bright stretch behind the head) is **off** by default ("Tail only"), and "show the rest faintly" is **off** by default. |
| Breaks: "never joined across the outside" | Unchanged by default; an option joins a jump with a dotted line (§6). |
| (absent) | Syzygy markers, off by default (§5) |
| (absent) | Leaving and re-entering the chart's reach (§3) |
| (absent) | Off-screen icons pinned to the view's edge, with an option to hide them (§6) |
| (absent) | Repeats in every image of an extended or wrapped chart (§7) |
| (absent) | Not-a-system handling: never drawn through, never started in (§8) |

---

## 2. Defaults

| Setting | Default |
|---|---|
| Icon set | Principia (§4) |
| Icon size | 1.4× |
| Syzygy markers | off |
| Syzygy style | from the icon set (o–o–o in Principia) |
| Fade stutters | on |
| Leaving / re-entering the chart's reach | on |
| End event icon | on |
| Off-screen icons pinned to the edge, faint | on |
| Leaving / entering the window (edge chevrons) | off |
| Seams (θ wrap) | off |
| Join jumps with a dotted line | off |
| Length | to the head, everything from `t = 0` |
| Tail only | off (tail Δt 3 when on) |
| Show the rest faintly | off |
| Head marker, labelled with its time | on |
| Time ticks | every 2, labelled |
| Direction arrows | filled triangle, 8 px, every Δt 1.0 |
| Width | 2.2 px |
| Colour | fixed `#FFF27A` for the hover or pinned trace. Kept traces take the trace palette in turn (`#7FE3FF #FF9EC4 #B7F28A #FFB86B #C9A7FF #9EF0D2`) |

---

## 3. The markers

Every colour below is the outcome palette's (the outcome-view ruling's nine classes), so an icon and the pixel under
it agree:

- collisions: 1–2 `#DE2D2D`, 1–3 `#2EBC4E`, 2–3 `#3462E0`; triple collision `#F29620`;
- escapes: body 1 `#F0DE32`, body 2 `#E034C6`, body 3 `#30C8DC`; triple ejection `#611597`.

Icons are drawn over a dark halo (`#0B0B0E`) with light ink (`#F5F4F0`), so they read on any field.

| Marker | When | Principia (default) icon |
|---|---|---|
| **Start** | `t = 0` | a five-pointed star in the trace's colour |
| **Collision** | the run ends in a pair collision | two arrows meeting at a burst, the burst in the pair's colour, oriented along the trace |
| **Triple collision** | two or more pairs inside `r_coll` (the ≥2-pair rule, `detail = 3`) | three arrows into a burst, `#F29620` |
| **Escape** | one body escapes | a disc in the escaping body's colour with an arrow leaving it, pointing along the trace |
| **Triple ejection** | all three unbound (`detail = 3`) | three discs `#611597`, each with an arrow pointing out |
| **Horizon** | nothing happened by `t_max` | an hourglass |
| **Incomplete** | the hover's step budget ran out (the pointer-channels note) | "?" ; the last stretch of the line is dashed |
| **Leaves the chart's reach** | the state leaves what the chart can show (on the latent chart: a momentum past `q_max`, or α past its limits) | a dotted ring; the line is dashed until it re-enters |
| **Re-enters the chart's reach** | it comes back | a ring with a centre dot |
| **Leaves / enters the window** (optional) | the line crosses the view's edge | a chevron at the edge |
| **Seam** (optional) | θ wraps | a short tick at both ends of the jump |
| **Syzygy** (off by default) | the three bodies are collinear (§5) | o–o–o |

**There is no double escape.** It is not an outcome class: two bodies leaving a third is the third and the pair
unbound, which classifies as triple ejection (`detail = 3`). The icon was drawn and removed.

**"Incomplete" is used.** The hover integration has a step budget, and a run that exhausts it is marked rather than
drawn as if it were whole (§8 today; the demo's 400 000-step budget).

---

## 4. Icon sets

One selector, **Icon set**. Each set gives every marker in §3.

| | Principia (default) | Arrows | Pictures | Orbits |
|---|---|---|---|---|
| Start | star | star | the initial triangle in body colours | star |
| Leaves reach | dotted ring | arrow into a bar | open ring | dotted ring |
| Re-enters reach | ring with a dot | bar, then an arrow out | filled ring | ring with a dot |
| Syzygy | o–o–o | o–o–o | three in a row | eclipse |
| Collision | arrows into a burst | arrows into a burst | two discs touching | a merger: the pair as one point in a flash ring |
| Triple collision | three arrows into a burst | the same | three discs touching | three discs in a flash ring |
| Escape | disc with an arrow out | the same | a streaking disc | a bound pair (ellipse) left behind, the body leaving |
| Triple ejection | three discs, arrows out | the same | three discs flying apart | three dotted streaks out |
| Horizon | hourglass | hourglass | hourglass | a closed orbit (ellipse with a body on it) |
| Incomplete | ? | ? | ? | ? |

Principia is Arrows with Orbits' two rings for leaving and re-entering the reach, which is what was circled. The
demo's "Compare icon sets" panel draws this table.

---

## 5. Syzygy

**What it is.** A syzygy is an instant at which the three bodies are collinear: the configuration passes through the
equator of the shape sphere (`w = 0`). Each syzygy has a **middle body**. The sequence of middle bodies is the
trajectory's syzygy word (Montgomery), whose letters name which side of the collinear set the shape crossed.
Repeating the same letter twice running is a **stutter**: in the reduced (free-group) word the two cancel, since the
shape crossed and came back.

**Off by default.** On a chaotic trajectory there are many (75 on Burrau to `t = 70`), and they add noise.

**The marker, o–o–o** (Principia's and Arrows' style):
- Three discs on a short line. The discs are in the **body colours**, in the order the bodies actually lie on the
  line: the outer pair lowest-numbered first, the middle body in the centre.
- The line is a different colour, `#D6D3CB`, and the discs have dark halos.
- No other dots and no rings.
- Five other styles stay selectable (Three in a row, Line and bead, Numeral tag, Crossing tick, Eclipse). The
  "Syzygy" selector defaults to "from the icon set".

**Placed on the trace.** The marker sits where the line actually crosses `w = 0`, at that point interpolated between
the two samples on either side of the crossing. It is not placed at either sample.

**Stutters** are drawn faded, with an option to switch that off.

**On charts that carry the y-momenta** (the latent momenta chart): the canonical frame mirrors at every syzygy, so
the trace jumps there (§9). A marker is drawn at both ends of the jump.

---

## 6. Off-screen icons, jumps and the window

- **Pin to the edge, by default.** An icon whose point is outside the view is drawn faint, pinned to the nearest
  point just inside the view's edge, so an event that happened off screen is still seen.
- **An option hides them:** "Off-screen icons: pin to the edge (faint). Untick to hide them."
- **Join jumps** (option, off): a dotted line across a break, where the line leaves the chart and comes back
  elsewhere.
- **Window chevrons** (option, off): where the line crosses the view's edge.

---

## 7. Extension, repeats and the window past the square

The trace works wherever the view goes (`principia_extension_interaction.md`).

- **Affine charts:** the trace runs on past the square like the field.
- **Periodic, pole-crossing and fold charts** (the shape sphere, its hemisphere): the trace is drawn **in every image
  of the chart in view**, with every marker and icon (start, events, rings, ticks, arrows, head), at half strength.
- **Repeats are never pinned to the edge.** Only the primary copy pins (§6); a repeat's icon that is off screen is
  simply not drawn, so the edge does not fill with duplicates.
- **A hover or click in a repeat** integrates the canonical system (the same IC as the primary pixel), so the trace is
  identical in every image.
- **The hemisphere is a toggle on the shape sphere** (R-412), not a chart of its own. With it on, the trace folds at
  the equator and its repeats continue into the mirror hemisphere.

---

## 8. Not a system

- **Never drawn through.** Every sample of the trajectory is encoded into the chart. A sample whose chart point is not
  a system (fails `validate` layers 2 and 3) is not drawn; the line breaks there.
  - A real trajectory cannot enter such a region: masses are constant and energy is conserved, so its own state stays
    valid.
  - The readout counts such samples ("in forbidden: N samples"). It reads 0 on every run tried. A non-zero count is a
    bug in the encode or the chart map, not physics, and is reported as such.
- **Never started in.**
  - **Click:** a click on a not-a-system pixel integrates nothing and draws a red ✕ where it was clicked.
  - **Readout:** the readout says "forbidden: not integrated", why (for example "a mass is ≤ 0, so this is not a
    three-body system"), the label `decode_failed`, detail 2, and the masses.
  - **Hover:** hovering there shows the same readout and no trace.
- **On the mass simplex the trace is a point.** A run's masses never change, so its chart point never moves. What
  changes is the off-slice distance: the shape and momenta leave the slice's fixed Burrau triangle at rest. So the
  trace is a fading dot. That is correct, and it is the general dot → line → curve rule (`trajectory_viewing.md` §3)
  for a conserved axis.

---

## 9. What was measured on the way

- **Encode jumps are the chart, not the sampling.** On Burrau 3:4:5 to `t = 70`, with the same encode, in Python:
  - 75 syzygies, and the canonical frame mirrors at every one.
  - The latent y-momenta jump by up to 3.4 against a median step of 1e-4, and the jump does not shrink at 100× finer
    sampling.
  - `z_β` runs to ±∞ at each syzygy (β = π·σ(z_β) ∈ (0, π)).
  - The momentum leaves `q_max = 2` on 19 excursions.
- **A non-finite step size hangs the loop.** A start with bodies 1 and 2 coincident (the centre of the shape sphere,
  θ = π, φ = π/2) gave a NaN step. Every comparison with NaN is false, so the loop never advanced and never ended.
  `computeIC` needs both guards:
  - **(a)** a start already inside `r_coll` ends as a collision at `t = 0`. That is an ordinary collision in its
    pair's colour, per the outcome-view ruling that removed "collision @ t = 0" as a class;
  - **(b)** a step size that is not finite ends the run as incomplete.

  This is the standing "Test `is_finite` explicitly".
- **Map agreement.** The sphere map integrated directly matches the latent chart's map, looked up through the shape
  inverse, on **100%** of pixels where both fields are smooth (5×5-uniform). The disagreement, 20% of the rest, is all
  in the fractal regions, sampled on different grids.
- **Holes were a lookup, not physics.** Grey holes on the earlier sphere were where the lookup through the latent map
  fell outside its ±3 window: near collinear (`z_β → ±∞`) and near the 1–2 collision, which the demo's α limit of
  0.05 cannot reach (prin-rs's GLSL value; Principia's `α_min` is 0, R-21).

---

## 10. The demo and the app

The demo is not the app's code. Where they differ:

| | Demo | App |
|---|---|---|
| Integrator | DOP5(4), adaptive, 1e-11 | `computeIC`, f64, the shipped integrator |
| Escape test | separation > 4, positive two-body energy, receding (the "demo test") | the closure criterion |
| Triple ejection | in the key only; the demo test does not detect it | classified |
| Fade | latent distance from the slice, floor 0.18 | R-509: the proxy where defined, else no fade, and the GUI says so |
| Latent α limit | 0.05 (prin-rs, GLSL) | `α_min = 0` (R-21) |
| Backgrounds | prin-rs outcome maps at fixed resolution (`reference/prin_rs_examples/`) | the live field |
| Mass simplex | chart_reference §4.5's ternary plot, Burrau triangle fixed at rest | the same chart |

---

## 11. Tests (each with the arm that fires)

| # | Test | The arm that fires |
|---|---|---|
| T1 | Defaults: a new trace draws from `t = 0` to the head with no tail, no ghost, no syzygy markers | a fixed tail length or syzygies on by default |
| T2 | Each end class draws its own icon in its outcome colour; triple ejection and triple collision use `detail = 3` | one icon for every outcome, or a double-escape icon |
| T3 | Syzygy placement: the marker's centre lies on the drawn line within 0.5 px at every syzygy of Burrau to `t = 70` | placing it at a sample point is off the line by up to a segment |
| T4 | o–o–o order: discs are in actual collinear order, middle body in the centre | a fixed 1-2-3 order |
| T5 | Pinning: an event off screen draws one faint pinned icon; with the option off it draws none | no icon off screen, or an icon at the true (invisible) point |
| T6 | Repeats: on the sphere at a zoom with 4 images, each image carries the start star and the end icon | lines only in repeats |
| T7 | Repeats never pin: no repeat icon is drawn at the view's edge | duplicates stacked at the edge |
| T8 | Not-a-system: across 1000 random runs on the simplex the forbidden-sample count is 0, and a constructed sample past the edge breaks the line | a line drawn through blank pixels |
| T9 | Start in a not-a-system pixel: ✕ drawn, `computeIC` not called | a fallback integration |
| T10 | Simplex trace: the chart point's spread over a run is 0 (masses constant) | an encode that drifts the masses |
| T11 | `computeIC` on coincident bodies returns within its step budget, with a collision at `t = 0` | the NaN loop |
| T12 | A pick in a repeat integrates the canonical IC bitwise | an unfolded decode |

---

## 12. Queue, don't rule

- **Q1.** Repeats of a trace at half strength or full. *Recommendation:* half, so the primary reads first.
- **Q2.** Start icon: the star in every set, or Pictures' initial-triangle glyph as an option elsewhere.
  *Recommendation:* star by default, the triangle as Pictures'.
- **Q3.** Where a syzygy and an end event coincide in time, which draws on top? *Recommendation:* the end event.
- **Q4.** Should "Leaves / enters the window" chevrons and edge-pinned icons merge into one edge treatment?
  *Recommendation:* keep them separate; pins are events, chevrons are the line.
