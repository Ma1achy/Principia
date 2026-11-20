# Listen: the trajectory heard as sound

*20 Oct 2026. New. It specifies the sound system demonstrated in `demo/trace_sound.html` (version 9), which is the
reference implementation for every rule here: where this file and the demo disagree, the demo's code is the
transcription source and the disagreement is a bug in this file, to be queued. Its GUI is specified separately, in
`principia_trace_listen_gui.md`. Quote §0 verbatim in the ruling.*

---

## 0. My words

> "With this trace thing, can you create a second version of this, like a second version of this artifact that is like
> the sort of sound system? … converting the trajectory into sound and then potentially having that play as well as we
> hover and show the trac"

> "Explain to me how you're producing the sound. Also, it'd be cool as well if you could listen to the sound of it
> evolve over time as you, the sort of like trace is drawn out…"

> "Oh, also … the Lagrange one, you can't hear… maybe need to like clamp the volume / normalise it ? yeah maybe options
> to clamp min and max volume and or normalise all the volumes would be nice. also ability to change the instrument
> would be sick, have a fat collection of instruments"

> "shouldn't you use the angles? i think those are more correct for you to use, you can probs account for the wrap.
> maybe need a larger window size idk think this could be improved."

> "stereo by winding should be default... still can't here Lagrange btw. also maybe the window should get longer as the
> trajectory gets longer in time? idk could be a cool option? ah i don't like how some are too short to hear, that
> shouldn't be a thing."

> "glass bell, marimba, music box, harp, analogue bass, acid, fm bass, and electric piano i don't think play correctly?
> they sound like a loop rather than a note?"

> "the instruments that i listed that are broken now just play once and fade out"

> "sound effect for close approach, collision types, ejection types, syzygy (optional can be disabled individually).
> yes again optional reverb on system spread, optional doppler pitch shift, option bit cruch, optional streo width.
> add all those other instruments"

> "maybe the reverb should be increased along with the doppler shift and stero width effect ? make it adjustable."

> "improve the appearance of the spectrograph and the graph above it."

---

## S1. What is heard

**The sound is the spectrum of the motion of the triangle's shape, plus its rotation in space.** Nothing else drives
pitch. A periodic orbit is a set of lines, so it is a note or a chord; a chaotic run spreads its power, so it is
noise. The masses, the energy and the overall size only set the time unit (S3), so two systems that differ only by
scale sound identical.

The **signal** is built from the shape vector `n̂ = (n₁, n₂, n₃)` on the shape sphere, computed straight from the
mass-weighted Jacobi vectors with no canonical frame (a canonical frame mirrors at every syzygy and would put a jump in
the signal):

```
ρ̃ = √μ_ρ ρ,  λ̃ = √μ_λ λ,   A = |ρ̃|², B = |λ̃|², I = A + B
n₁ = (A − B)/I,   n₂ = 2 ρ̃·λ̃ / I,   n₃ = 2 (ρ̃ × λ̃) / I
θ = atan2(n₂, n₁),   φ = acos(n₃),   ψ = atan2(ρ_y, ρ_x)     (ψ: the triangle's orientation in space)
```

Three signals are selectable (`Signal`):

| Signal | Channel A (complex) | Channel B | Notes |
|---|---|---|---|
| **Angles + rotation (default)** | `w · e^{iθ}` and, summed in power, `e^{iψ}` | `φ` | Lagrange's shape is fixed and it still sounds, through `ψ` |
| Angles only | `w · e^{iθ}` | `φ` | Lagrange is silent: the shape does not move |
| Shape vector | `sin φ · e^{iθ} = n₁ + i n₂` | `cos φ = n₃` | the `sin φ` weight quietens `θ` near the poles |

**The wrap is handled by using `e^{iθ}`, not `θ`.** `θ + 2π` is the same point, so `e^{iθ}` is exactly wrap-free; `θ`
itself would drift and put a sawtooth's harmonics into the spectrum. `φ` never wraps. **The pole weight**
`w = min(1, sin φ / 0.1)` fades `θ` within 0.1 rad of a pole, where it is undefined and swings fast (a coordinate
singularity, not motion).

**The spectrum is complex, so it carries direction.** A complex FFT of `e^{iθ}` puts anticlockwise winding at positive
frequencies and clockwise at negative ones. That sign is what `Stereo by winding` uses (S6).

## S2. The spectrum

For a window `[t₀, t₁]` of the run, `D = t₁ − t₀`:

- **Resample** the run's samples to `N` uniform times by linear interpolation of the state (4096 for the displayed
  analysis, 1024 per frame of the follow track).
- **Detrend** (subtract the mean), **Hann-window**, **zero-pad** to `NF` (8192; 2048 per frame), FFT. `df = (N−1)/(D·NF)`
  cycles per unit time.
- **Peaks**: local maxima above `0.01 ×` the largest bin, refined by a parabola through the three bins, sorted by
  height, the top `K` kept (`Voices`, default 16). **Bins below 1.5 periods per window are ignored** (`k₀ =
  ⌈1.5/(D·df)⌉`, at least 2): a line needs one and a half cycles inside the window to be a line.
- **Left and right share one scale** (`ref` = the larger of the two channels' maxima), so a side with almost nothing
  stays almost silent instead of being normalised up to full level.
- **Entropy**: Shannon entropy of the normalised power spectrum over `log(bins)`, 0 for a pure line and 1 for white
  noise. It is read out and drives the bit-crush (S9).
- **Interval**: the ratio of the two strongest lines, folded below 4 and named against 13 just ratios if within 25
  cents (`7:5` is named, as a tritone).

**No run is too short to hear.** Where no line clears the 1.5-period cut, the **mean rates** are played instead: the
winding rates of `θ` and `ψ` (unwrapped step by step, signed) and the rate of `φ` (unsigned), in cycles per unit time,
each at level `max(0.3, |f|/f_max)`, panned by sign. A run that moves at all makes a sound.

## S3. Pitch

```
Hz = f · t_c · f_ref · 2^shift
t_c = (2|E|)^{-3/2}   if E < 0,   else  I^{3/4}       (G = 1, total mass 1; evaluated at t = 0)
```

- `f` is in cycles per unit time, so `f · t_c` is cycles per crossing time and is **scale-free**: a system and its
  rescaled copy play the same notes.
- `f_ref` (`1/t_c is`) is 110, **220 (default)** or 440 Hz.
- **Fit to hearing** (default on): `shift` is the whole number of octaves that puts the whole run's strongest line in
  110–880 Hz. **It is chosen once per run, from the whole run**, so the pitch does not jump as the head moves, and an
  octave shift keeps every ratio.
- Lines outside 30 Hz–6 kHz are not voiced.

Measured in the demo (its own integrator, figure-eight released from its standard ICs, `t ≤ 60`, defaults):
**262.6, 131.3, 393.9 and 525.2 Hz** — 2:1, 3:2 and 4:3 against the 131.3 line. The 1%-perturbed figure-eight reads
the same four to 0.1 Hz; Burrau 3:4:5 ends in a collision at `t = 54.84` with an unnamed ratio of 1.549 and entropy
0.352 against the figure-eight's 0.309; Lagrange plays **one line at 181.7 Hz** under the default signal.

## S4. Follow the head, or the whole run

| `Listen` | When | What plays |
|---|---|---|
| **Follow the head (default)** | while the trace is playing or being scrubbed | the spectrum of the window around the head, re-voiced as the head moves |
| Follow the head | when nothing is playing or scrubbing | **the whole run as one chord**, matching the trace being drawn whole (follow-up F3) |
| Whole run | always | the whole run as one chord |

**Windows** (`Window`, in follow mode):

| Window | Span at head time `t` |
|---|---|
| **Fixed, centred (default)** | `W = min(Fixed × t_c, t_end)` (default `6 t_c`), centred on `t`, shifted inside the run near its ends so every moment has a full window |
| Grows | `[0, t₁]`, everything drawn so far |
| Grows, last half | `[t₁/2, t₁]` |

For the growing windows, `t₁ = min(t_end, max(t, min(t_end, 0.5 t_c)))`, so the window is never empty.

**Frames.** In follow mode the run is pre-analysed into frames at a hop of `min(t_end/8, max(W/16, t_end/900))`. Each
frame holds its spectrum, its peaks in Hz with a pan (`(R−L)/(R+L)` at that bin) and its shape rms. The head reads the
frame at or before it.

**After the run ends** (its event, or the horizon), follow mode is **silent**, not a held chord.

## S5. Voices

- **Chord** (whole run): one voice per peak, cross-faded in as a bank over ~140 ms, and cross-faded out when the run
  changes. Level `0.22 × a / Σa`, ×1.3 in stereo. **Never retriggered.**
- **Follow track**: a pool of **32 persistent voices**. At each frame:
  1. Each sounding voice takes the nearest unused peak **within a semitone** and glides to it (35 ms); a voice with none
     fades out.
  2. Each unclaimed peak takes a silent voice: its pitch is set while silent, then it fades in.
  3. Pan follows the peak's pan × 0.9 in stereo, 0 in mono.

  This is partial tracking: a line that drifts is heard as a glide, not as a sequence of re-attacks.

## S6. Channels

| `Channels` | Left | Right |
|---|---|---|
| **Stereo by winding (default)** | anticlockwise winding (positive frequencies) | clockwise winding (negative frequencies) |
| Stereo | `θ` (both directions folded) | `φ` |
| Mono | everything summed in power | — |

So in the default a system that winds one way round the sphere is heard on one side.

## S7. Loudness

```
rms = |(rms(Re A), rms(Im A), rms(B)/π [angles] or rms(B) [vector], |rms(e^{iψ})| [rotation signal only])|
level = 0                                    if rms ≤ RMS_FLOOR (1e-7)
      = clamp(√min(1, rms/0.1), min, max)    Loudness: follows the shape's movement (default)
      = clamp(1, min, max)                   Loudness: normalised, every system at full level
```

`Min level` defaults to 15%, `Max level` to 100%, `Volume` (the master) to 60%.

**The floor is there so that a system that does not move is silent rather than normalised up to its rounding noise.**
Measured in the demo under *Angles only*: Lagrange's shape rms reads **2.65e-7**, just above the floor, so it plays at
the min level with lines at 181.5 Hz and above — that is the demo integrator's error, not motion. The floor was set
against the demo's integrator, and prin-rs's is far quieter (Q3).

## S8. Instruments

**The spectrum decides the pitches and their weights; the instrument only decides the timbre each partial is voiced
with**, so changing instrument changes nothing that is measured or displayed. 39 instruments, default **Warm sine**.
Transcribe `INSTRUMENTS` and `makeVoice` from the demo; a recipe has:

| Field | Meaning |
|---|---|
| `a` | main oscillator: sine, square, sawtooth, triangle, or a harmonic list (a periodic wave) |
| `b`, `mix`, `ratio`, `det` | a second oscillator, mixed at `mix`, at `ratio × f`, detuned in cents |
| `fm`, `fmenv` | `b` as an FM modulator at depth `fm × f`; `fmenv` makes the index fall on each note |
| `lp`, `q`, `formant` | low-pass at `lp × f`, or fixed-Hz band-passes (vowels) |
| `fenv`, `fsweep` | filter opening on each note and settling; an LFO sweeping the filter |
| `vib`, `bvib`, `trem` | vibrato (rate, cents), vibrato on `b` alone, tremolo |
| `unison` | detuned copies of `a` |
| `breath` | band-passed noise tracking `f` |
| `att`, `glide`, `oct`, `lvl` | attack, minimum glide, octave offset, level |
| `strike`, `sus` | **struck**: decays with time constant `strike` **to a held floor `sus`**, never to zero |
| `retrig` | a note event (filter or FM envelope) when the partial moves on, gain held |

**Struck instruments** (glass bell, electric piano, marimba, music box, harp, kalimba, steel drum, vibraphone, celesta,
glockenspiel, tubular bells, gong; and, with `retrig` rather than `strike`, analogue bass, acid and FM bass):

- They are **struck once and ring at their floor**. The first build re-struck them on a timer and they sounded like a
  loop; the second decayed to zero and they played once and died. The floor is the fix for both.
- **They are struck again on the trajectory's own beats** (`Strikes on`):
  - each syzygy the head passes (**default**);
  - every crossing time ÷ 1, 2 or 4;
  - only when a line changes, i.e. a voice's partial has moved more than a quarter-tone since it last sounded.

Families, for the menu (GUI file G6): pure (5), keys (3), struck (10), bowed and blown (7), voices (4), synths (8),
textures (2).

## S9. Events and effects

**Events** are one-shot sounds fired when the head passes them, each optional (`Events`), at `Event level` (80%),
panned by the bodies involved (bodies 1, 2, 3 at −0.8, 0, +0.8):

| Event | Detected | Sound |
|---|---|---|
| Close approach (on) | a pair's separation at a local minimum below `0.2 R` (`R = √I`), before the run's end | a band-passed noise whoosh, brighter and louder the deeper (`depth = min(1, log₁₀(0.2R/d)/2)`) |
| Collision (on) | the run's terminal collision | by pair: 1–2 low tom, 1–3 snare, 2–3 rimshot; triple: a crash |
| Escape / ejection (on) | the run's terminal escape | a falling glide towards the escaping body's ear; a triple ejection glides to all three, staggered |
| Syzygy tick (off) | each syzygy | a short click panned by the middle body |

**Effects** each have a switch and an amount (×, default in brackets); they read the run at the head (follow) or its
mean (chord):

| Effect | Driven by | Mapping |
|---|---|---|
| Reverb (on, ×1.40) | spread `R/R₀` | wet `= min(1.6, a · (0.12 + 0.9 · clamp(log₂(R/R₀)/3, 0, 1)))`, 3.6 s impulse: a tight system dry, a wide one in a hall |
| Doppler (off, ×1.50) | scale-free expansion rate `Ṙ t_c / R` | detune `= −400 a · Ṙ t_c/R` cents, clamped to `±1200 · clamp(a, 0.25, 2)`: expanding bends down, contracting up |
| Bit-crush (off, ×0.60) | spectral entropy `H` | `bits = round(clamp(15 − 40 a · max(0, H − 0.12), 3, 16))`: a clean line stays clean, noise frays |
| Stereo width (on, ×1.50) | scale-free angular momentum `√(L_z² |E|)` | `w = clamp(a · √(L_z²|E|)/0.08, 0.15, 2)`, mixed as `L' = (1+w)/2 L + (1−w)/2 R`: released from rest is narrow, a rotating system wide |

All four inputs are scale-free, so the effects obey S1's scale invariance too. Chain: voices and events → bit-crush →
width matrix → dry + convolver → compressor → output.

## S10. Horizon and dwell

- While the hover is moving, the run is integrated to `t ≤ 20` (the maps' horizon), so the sound is coarse.
- On dwell (280 ms) or pin, the same run is integrated on to the **horizon** (`Horizon`: 20, **60**, 150, 400), and the
  trace and the sound both switch to it and sharpen.
- A run ended early by its event is a short run, so a coarse spectrum; that is information, not a fault.

## S11. Display

**Spectrum** (at the head, or the whole run):
- log frequency 30 Hz–6 kHz, gridded at the A of each octave (A1 55 … A7 3520);
- dB from 0 to −60 relative to the strongest bin;
- the 110–880 Hz fit band shaded;
- left channel blue `#5ab4ff` (anticlockwise), right amber `#e0a030` (clockwise), mono `#dcdcdc`, each a filled
  curve;
- the top peaks dotted, with chips showing note, cents and Hz;
- the strongest line's harmonics 2f–8f as dashed ticks.

**Spectrogram** (the whole run, on the run's own time axis):
- the frames, magma palette;
- **each frame shown in dB above its own mean over ±½ octave**, range 14 dB. On raw dB the broad slope every run
  carries fills the panel and hides the lines; this keeps the lines and drops the slope. **Display only — the sound is
  untouched**, and the panel says so;
- bins below the fourth are left blank: they hold the window's own leakage, not lines;
- an **event lane** above: syzygies as ticks in the middle body's colour, close approaches as triangles, the end event
  as a diamond in its outcome colour;
- the playhead with a time chip; blank after the run's end.

**Readout**:
- pitch mapping;
- window;
- top peaks as notes;
- the named interval;
- shape rms → loudness;
- entropy;
- periods per window;
- the reason for any silence ("not a system", "the run ended at t = … by …").

## S12. Sweep and known orbits

- **Sweep a row**: the cursor moves along the current row of the chart over `Sweep` seconds (default 10), so the
  sound of a slice is heard as a gesture.
- **Known orbit** loads a named IC as the trace, with total mass 1 and `G = 1`: figure-eight, figure-eight perturbed
  1%, Burrau 3:4:5, Lagrange rotating. These are the calibration set for the tests.

## S13. Tests (each with the arm that fires)

| Test | Passes when | Fires on |
|---|---|---|
| Scale invariance | a system and its copy scaled by `α` (times by `α^{3/2}`) give the same peaks in Hz to 0.1 Hz | a `t_c` or `f_ref` slip; an effect input that is not scale-free |
| Winding sign | the figure-eight run forwards and time-reversed swap its lines between L and R under *Stereo by winding* | a real FFT, or the sign dropped |
| Wrap | the `θ` signal's spectrum has no lines at multiples of the winding rate that are absent from `e^{iθ}`'s | `θ` used instead of `e^{iθ}` |
| Figure-eight | lines in exact 2:1 / 3:2 / 4:3 ratios to ±5 cents | a windowing or peak-refinement error |
| Lagrange (default signal) | one line, level above the min clamp | the rotation channel dropped |
| Lagrange (angles only) | level 0, because the shape rms is below the floor | the floor removed, or set above prin-rs's drift (Q3) |
| No silent run | every run with `t_end > 0` that moves has at least one voiced line (fallback included) | the fallback missing |
| Struck floor | a struck voice 10 s after its strike is at `sus × level`, not 0, and was struck exactly once without a beat | a timer re-strike, or decay to zero |
| Silence after the end | follow mode past `t_end` voices nothing | a held chord |
| Spectrogram is display-only | the voiced peaks are identical with the spectrogram on and off and under any display mode | the contrast step leaking into analysis |

## S14. Queue, don't rule

- **Q1. Audio backend for prin-rs.** The demo is Web Audio. Recommendation (a): a Rust synthesis graph on
  `cpal` for output, with the voice and effects chain transcribed node for node. (b) Keep sound web-only.
- **Q2. Where the analysis runs.** Recommendation (a): on the CPU, from the trajectory viewer's own samples, at the
  sizes in S2; it is two FFTs per frame. (b) On the GPU with the integrator.
- **Q3. `RMS_FLOOR` against prin-rs's integrator.** Recommendation (a): measure Lagrange's shape rms under *Angles
  only* with the shipped integrator over `t ≤ 400`, and set the floor at 10× its maximum. The demo's 1e-7 is its own
  RK4's drift.
- **Q4. The superseded sketch row.** The dev-GUI sketch's "listen · separations → pitch" is replaced by this file's
  signal. Recommendation (a): mark it stale, replaced by this ruling. (b) Keep separations as a fourth `Signal`.
- **Q5. Close-approach threshold** `0.2 R` and the event depth scale are demo choices. Recommendation (a): keep them,
  stated as parameters.
