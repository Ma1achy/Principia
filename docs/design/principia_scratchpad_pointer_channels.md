# SCRATCHPAD — the pointer channels: trace, sound, inspector

*Working notes, not ratified. Updates `principia_trajectory_viewing.md` where the two disagree —
that doc's §1 mechanism stands; its §1 responsiveness paragraph is superseded by §3 below.*

---

## 1. There is ONE mechanism, and it now has THREE consumers

`principia_trajectory_viewing.md` already establishes the rule: **hover or click integrates that IC
on the CPU (f64, `computeIC`); the features differ only in what you project onto.** Sonification is
a third projection, not a new subsystem.

| consumer | projection | drawn/played where |
|---|---|---|
| **hover trace** | onto the current chart's `(q₁, q₂)` plane | overlaid on the survey |
| **sonification** | `θ(t), φ(t)` → spectrum → audible | audio out |
| **click inspector** | absolute shape sphere + real space | separate three-panel surface |

**One integration, three lenses.** All three optional and independently toggleable; none is a
prerequisite for another. The IC inspector is the same system with a longer, higher-precision run
and a persistent panel.

---

## 2. Why the trace and the sound are the SAME measurement

**The loop drawn on the shape sphere *is* `θ(t), φ(t)` traced out. The sound is the spectrum of that
same path.** A closed loop and a clean note are the same fact rendered twice.

Measured on the figure-eight against a chaotic IC, spectrum of `θ(t)`:

| | spectral entropy | top-5 peak ratios |
|---|---|---|
| figure-eight (periodic) | **0.076** | 1, 0.40, 0.15, 0.09, 0.08 |
| chaotic | **0.177** | 1, 0.92, 0.57, 0.53, 0.19 |

Entropy 0 is one line — **a note with timbre**. Entropy 1 is white noise. The periodic case has one
dominant line with harmonics falling away; the chaotic case has four peaks of comparable height,
broadening toward noise at longer `t`.

**And they fail in complementary ways, which is the argument for having both.** A trace drawn on a
2-D slice is a *projection*: a loop can look closed while hidden components disagree, and dense
winding turns to visual mush past a few dozen turns. **The ear does not care about projection and
stays sharp exactly where the drawing gives up** — a hundred tight windings look like a smudge and
sound like a clear high note.

**Human pitch perception beats visual inspection at detecting a periodic component in noise.** For
periodicity specifically, sonification is not a second-rate view of the picture; it is the better
instrument.

---

## 3. UPDATE TO THE RESPONSIVENESS MODEL — hover is affordable, and the earlier caution was wrong

`principia_trajectory_viewing.md` §1 says *"one integration per settled hover, not per mouse-pixel"*.
**Too conservative. The arithmetic:**

```
one trajectory, t = 50 : 1.06e5 substeps × 4 evals × ~72 flop = 30.6 MFLOP
                          6.1 ms on ONE CPU core
one trajectory, t = 13 : 7.9 MFLOP
                          1.6 ms on ONE CPU core,  0.4 ms on four
```

**One trajectory fits inside a 60 fps frame budget on a single core, at any production horizon.** It
can run *per frame* during motion, not only on settle.

**So the async machinery is for the TAIL, not the average.** `total_substeps` is bimodal with a ~100×
p1→p99 spread, so a hover landing on a near-collision can cost 50× the typical pixel. The budget
exists to stop one pathological pixel stalling the pointer.

**Which gives a better rule than "reduce quality until it fits":**

> **Give the hover trajectory a STEP BUDGET. If it exhausts it, report what it got and mark it
> incomplete.**

That is the no-discard rule applied to the pointer: a truncated hover is a **partial answer, not a
failure**. For the trace it draws a shorter curve; for audio a shorter trajectory is a **coarser
spectrum**, so you *hear* it become less certain. Honest, and it degrades continuously.

**Dwell releases the bound.** Moving pointer → bounded budget and whatever that buys. Pointer stops →
budget lifted, run converges, trace and spectrum sharpen. **Same shape as the coarse-ancestor fill
during camera motion: degrade during movement, converge on rest.** The renderer already uses this
pattern; this is the second channel adopting it.

Cancel-on-move by generation counter still applies — latest position wins, stale results dropped.

---

## 4. Audio-specific constraints, which are NOT the frame budget

**The floor on trajectory length is set by frequency resolution, not by compute.** A spectrum needs
enough oscillation periods to resolve a line — roughly 20+. So there is a minimum `t` below which
the sound is meaningless *even though the integration was cheap*. **Measure it; do not guess it.**

**Scale to audible range from the CROSSING TIME, not an arbitrary factor.** Map `1/t_c` to a fixed
reference pitch. Then frequency *ratios* are preserved across initial conditions, two ICs with the
same dynamical ratio sound like the same interval, and the mapping is **derived rather than chosen** —
which is the standing rule for constants.

**Cross-fade, never retrigger.** Hovering across a basin boundary is the most interesting moment
available — you would *hear* one basin become another — but only if the sound is continuous.
Retriggering per pixel turns the best part into clicking.

---

## 5. The prediction worth testing early, because it is the demo

**Hover slowly across a fractal boundary and listen for the note dissolving into noise and re-forming
as a different note.**

If that works it is the most direct perception of the manifold's structure anything in this project
has produced, and **it is testable on existing inspector data without building the interaction** —
take a line of ICs crossing a known boundary, integrate them, and play the spectra in sequence.

**Second prediction, sharper and also testable now:** a `p:q` resonance gives commensurate
frequencies, which is a rational interval — **a 3:2 resonance should literally sound like a fifth.**
If that holds you can *hear* which resonance you are near.

---

## 6. What this reuses rather than adds

- **`computeIC`** — the Precision-ring path, already built for parity. Trajectory viewing was its
  third consumer; sonification is its fourth.
- **Frequency diffusion is already a computed field**, which means an FFT of the shape angle already
  exists somewhere in the pipeline. The sonification spectrum is that machinery with a different
  output stage.
- **Closure → 0 and spectral entropy → 0 are the same statement.** The periodic-orbit detector and
  the "sounds like a note" condition are one quantity.
- **The cursor-bias refinement idea** (slippy-map notes §18) composes directly: **the thing you are
  pointing at is the thing you are hearing and the thing being refined.** One attention signal,
  three consumers.
  **Ratified as cursor bias (R-55):** `P_focus` centres on the pointer while it is in view (scheduler contract Part 6).

---

## 7. Open

- The minimum `t` for a usable spectrum (§4) — measure.
- Whether `φ(t)` adds anything over `θ(t)` alone, or whether two channels is better as *stereo*.
- Whether the whole-slice version is anything but a meme. Averaging is mush; **sweeping a path and
  hearing it change** is the version worth trying, and it is the same code as hover with a scripted
  pointer.
- Whether the hover trace should show the *time cursor* position audibly — i.e. whether the sound is
  the whole trajectory's spectrum or a windowed spectrum tracking the playhead.
