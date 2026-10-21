# Principia — Listen: the trajectory heard as sound

*Adopted by R-540 (21 Oct 2026) from the human's `principia_listen.md` (kept as delivered at
`docs/incoming/2026-10-21_trace_listen/`), transcribed from the code of its reference implementation,
`docs/gui/reference/trace/trace_sound.html` (version 9), which the human's message makes the transcription source:
where the file and the code disagree, this document carries the code and the disagreement is queued (RQ-454 to RQ-458).
It defines pointer_channels §4's open points (R-109, R-144; extended by R-540) and is normative. Listen is a feature of
the trajectory viewer (`principia_trajectory_viewing.md`): it reads the same `computeIC` trajectory as the trace. Its GUI
is `principia_render_gui_spec.md` §G2, §G3, §G5 and §G6 (R-541). Bodies are shown 1, 2, 3 (R-449); the formulas index
them 0, 1, 2. Units: `G = 1`, every system's total mass 1. Listen's build waits on RQ-440 (the audio backend) and RQ-442
(the floor against prin-rs's integrator).*

---

## 1. What is heard

**The sound is the spectrum of the motion of the triangle's shape, plus its rotation in space.** Nothing else drives
pitch. A periodic orbit is a set of lines, so a note or a chord; a chaotic run spreads its power, so it is noise. The
masses, the energy and the overall size only set the time unit (§3), so two systems that differ only by scale sound the
same.

**The shape vector**, per sample, from the state with no canonical frame (a canonical frame mirrors at every syzygy and
would put a jump into the signal):

```
M01 = m0 + m1,   μ_ρ = m0 m1 / M01,   μ_λ = m2 M01            (total mass 1)
ρ = r1 − r0,     λ = r2 − (m0 r0 + m1 r1) / M01
ρ̃ = √μ_ρ ρ,      λ̃ = √μ_λ λ,      A = |ρ̃|²,  B = |λ̃|²,  I = A + B
n1 = (A − B)/I,  n2 = 2 ρ̃·λ̃ / I,  n3 = 2 (ρ̃_x λ̃_y − ρ̃_y λ̃_x) / I
θ = atan2(n2, n1),  φ = acos(clamp(n3, −1, 1)),  (cos ψ, sin ψ) = ρ / |ρ|      (|ρ| = 0: ρ itself)
```

`ψ` is the triangle's orientation in space. `n3` is R-14's `w` over `I`.

**The signal**, `Signal`, one of three:

| `Signal` | Channel A (complex) | Channel B (real) | Rotation R (complex) | Pole weight `w` |
|---|---|---|---|---|
| **Angles + rotation (default)** | `w·e^{iθ}` | `φ` | `e^{iψ}` | `min(1, sin φ / 0.1)` |
| Angles only | `w·e^{iθ}` | `φ` | — | `min(1, sin φ / 0.1)` |
| Shape vector | `w·e^{iθ} = n1 + i n2` | `n3 = cos φ` | — | `sin φ` |

**The wrap is handled by using `e^{iθ}`, not `θ`**: `θ + 2π` is the same point, so `e^{iθ}` is exactly wrap-free, where
`θ` itself would drift and put a sawtooth's harmonics into the spectrum. `φ` never wraps. **The pole weight** fades `θ`
within 0.1 rad of a pole, where it is undefined and swings fast (a coordinate singularity, not motion). Under *Angles
only* a shape that does not move is silent even while it rotates (Lagrange's rotating triangle); the rotation channel is
what makes it heard under the default. Whether the default may add `ψ` to R-109's `θ(t), φ(t)` is RQ-451.

**The spectrum is complex, so it carries direction.** A forward FFT (`X[k] = Σ x_n e^{−2πi kn/NF}`) of `e^{iθ}` puts
anticlockwise winding (θ increasing) at positive frequencies and clockwise at negative ones; that sign is what *Stereo by
winding* uses (§6).

## 2. The spectrum

For a window `[t₀, t₁]` of the run, `D = t₁ − t₀`:

- **Resample** the run's `computeIC` samples to `N` uniform times `t₀ + D·k/(N−1)`, by linear interpolation of the whole
  state between the two samples around each time (`N = 4096` for the analysis shown and read out, `1024` per frame of
  the follow track, §4).
- **Each series:** subtract its mean (a complex series its complex mean), multiply by the Hann window
  `0.5 − 0.5 cos(2πi/(N−1))`, zero-pad to `NF` (`8192` shown, `2048` per frame), FFT. Bin `k` is the frequency
  `k·df`, `df = (N−1)/(D·NF)` cycles per unit time; bins `0 … NF/2 − 1` are kept.
- **The channel magnitudes:** for A, `pos[k] = |X[k]|` and `neg[k] = |X[NF−k]|` (`neg[0] = pos[0]`); under the default
  signal R's `pos` and `neg` are summed in power into them, `pos[k] = hypot(pos_A[k], pos_R[k])`, likewise `neg`;
  `fold[k] = hypot(pos[k], neg[k])`; B's real magnitude `b[k] = |X_B[k]|`; `mono[k] = hypot(fold[k], b[k])`. §6 says
  which reach the left and right.
- **Peaks** of a magnitude `m` (`Voices` = `K`, 1 to 32, default 16): `k₀ = max(2, ⌈1.5 / (D·df)⌉)`, so **bins below
  1.5 periods per window are ignored**; `mx` = the largest `m[k]` for `k₀ ≤ k`, raised to the shared scale `ref` where one
  is given; every `k₀ ≤ k < len − 1` with `m[k] > m[k−1]`, `m[k] ≥ m[k+1]` and `m[k] > 0.01·mx` is a peak, refined by the
  parabola through its three bins, `d = ½(a − c)/(a − 2b + c)` (a zero denominator read as 1), at `f = (k + d)·df` with
  weight `b/mx`; sorted by weight, the top `K` kept.
- **Left and right share one scale:** their peaks are found with `ref` = the larger of the two channels' maxima above
  `k₀`, so a side with almost nothing stays almost silent instead of being normalised up; the mono peaks take their own
  maximum.
- **Entropy** of the mono magnitude: `H = −Σ_{k≥1} p_k ln p_k / ln(len − 1)`, `p_k = mono[k]² / Σ_{k≥1} mono[k]²`: 0 for a
  pure line, 1 for white noise. It is read out and drives the bit-crush (§9).
- **Interval** of the two strongest mono peaks: `r = f_hi / f_lo`, halved while `r ≥ 4`, named as the nearest of 13 just
  ratios by `c = 1200 log₂(r/(p/q))` if `|c| ≤ 25` cents: 1:1 unison, 2:1 an octave, 3:2 a fifth, 4:3 a fourth, 5:4 a
  major third, 6:5 a minor third, 5:3 a major sixth, 8:5 a minor sixth, 9:8 a whole tone, 7:4 a harmonic seventh, 3:1 a
  twelfth, 5:2 a major tenth, 7:5 a tritone (7:5); otherwise "no simple ratio within ±25¢".
- **Periods** per window: the strongest mono peak's `f·D`.

**No run is too short to hear.** Where no mono peak clears the cut, the **mean rates** are played instead, in cycles per
unit time over the window: `θ`'s winding rate, its steps wrapped into `(−π, π]` and weighted by the pole weight; `ψ`'s
(the default signal only), wrapped likewise; both signed; and `φ`'s, `Σ|Δφ|/(2πD)`, unsigned. A rate with `|f|·D ≤ 10⁻⁶`
is dropped. Each plays at weight `max(0.3, |f|/f_max)`, panned `−0.8·sign(f)` (anticlockwise left), `φ`'s at the centre;
the left takes the rates panned at most 0.01, the right the others, the left all of them if it would be empty. A run
that moves at all makes a sound. A run with `t_end ≤ 10⁻⁶`, or fewer than three samples, has no analysis and is silent.
Against pointer_channels §4's "roughly 20+" periods, RQ-450.

## 3. Pitch

```
Hz = f · t_c · f_ref · 2^shift
t_c = (2|E|)^{−3/2}  if E < 0,   else  I^{3/4}       (G = 1, total mass 1; E and I = Σ m_i |r_i|² at t = 0,
                                                      the centre of mass at the origin)
```

- `f` is in cycles per unit time, so `f·t_c` is cycles per crossing time and **scale-free**: a system and its rescaled
  copy play the same notes.
- `f_ref` (`1/t_c is`) is 110, **220 (default)** or 440 Hz; the value is REQ-GUI-143's calibration, confirmed at the M8
  gate, 220 Hz the bundle's proposal.
- **Fit to hearing** (default on): `shift` is the whole number of octaves, from −8 to 8, that puts the whole run's
  strongest mono line into 110–880 Hz: from `shift = 0`, down an octave while above 880 Hz, then up while below 110 Hz.
  **It is chosen once per run, from the whole run**, so the pitch does not jump as the head moves, and an octave keeps
  every ratio. Off, `shift = 0`.
- **Only lines strictly between 30 Hz and 6 kHz are voiced.**
- `t_c` grows without bound as `E → 0⁻` while the fit stops at ±8 octaves, so a moving bound run with small `|E|` can
  voice nothing, and the pitch jumps by about 19 octaves across `E = 0`; against S2's "A run that moves at all makes a
  sound" this is RQ-467, the code applied meanwhile (physics review 5595386853 of PR #267, P2).

Measured in the demo (its own integrator, Dormand–Prince 5(4) at relative tolerance 10⁻¹¹; the figure-eight from its
standard ICs, `t ≤ 60`, the defaults): 262.6, 131.3, 393.9 and 525.2 Hz, in 2:1, 3:2 and 4:3 against the 131.3 line; the
1 %-perturbed figure-eight reads the same four to 0.1 Hz; Burrau 3:4:5 ends in a collision at `t = 54.84`, its top two in
an unnamed 1.549 and its entropy 0.352 against the figure-eight's 0.309; Lagrange plays one line at 181.7 Hz under the
default signal. These are evidence, not values (R-540 H12).

## 4. Follow the head, or the whole run

| `Listen` | When | What plays |
|---|---|---|
| **Follow the head (default)** | while the trace is being played or scrubbed, or animates on its own clock (R-538 TF3) | the frame at the head, re-voiced as the head moves (the follow track, §5) |
| Follow the head | otherwise | **the whole run as one chord**, as the trace of the default length is drawn whole (R-538 TF3; a `Window` or `LastDt` trace is drawn as chosen and the chord is still the whole run, RQ-427 (a), RQ-465 (a), applied meanwhile) |
| Whole run | always | the whole run as one chord |

*Was:* "**the whole run as one chord**, as the trace is drawn whole (R-538 TF3)" in the second row (merging main's PR #263, which applied RQ-427 (a); RQ-465; applied per R-369).

The head's time is the playhead's, or the trace's own clock under *Own clock*, never past the run's end.

**Windows** (`Window`, follow mode), at the head's time `t`, the run ending at `t_end`:

| `Window` | Span |
|---|---|
| **Fixed, centred (default)** | `W = min(Fixed·t_c, t_end)`, `Fixed` from 1 to 30 in steps of 0.5, default 6; `t₀ = max(0, min(t_end − W, t − W/2))`, the span `[t₀, t₀ + W]`: shifted inside the run near its ends, so every moment has a full window |
| Grows | `[0, t₁]` |
| Grows, last half | `[t₁ − W′, t₁]`, `W′ = max(min(t₁, ½t_c), t₁/2)`: never shorter than half a crossing time (RQ-455) |

`t₁ = min(t_end, max(t, min(t_end, ½t_c)))`, so a growing window is never empty.

**Frames.** In follow mode the run is analysed beforehand into frames at times `0, h, 2h, … ≤ t_end`, the hop
`h = min(t_end/8, max(W/16, t_end/900))` with `W` the fixed window's span whatever the mode. Each frame holds its mono
magnitude, its peaks of the mono magnitude (or its mean rates, §2) in Hz, each with a pan `(R − L)/(R + L)` of the left and
right channels' magnitudes at the peak's bin (`(R − L)/1` where both are 0), and its shape rms (§7). The head reads the
last frame at or before it, the first before the first.

**Past the run's end** (its event, or the horizon) follow mode is **silent**, never a held chord. The display's analysis
at the head (§11) is redone at most every 120 ms.

## 5. Voices

- **The chord** (the whole run): one voice per peak between 30 Hz and 6 kHz. In stereo the left channel's peaks are voiced
  at pan −0.9 and the right's at +0.9; in mono the mono peaks at 0. Each voice's level is `0.22·a/Σa` over its set,
  ×1.3 in stereo; each starts at its pitch at once, a struck instrument struck (§8), any other faded in at a time
  constant of 10 ms (or its `att/3`, §8). The voices form a bank whose gain goes from 0 to the run's loudness (§7) at a time constant of a
  third of the cross-fade; the bank it replaces falls to 0 at the same time constant and is stopped after six of them.
  The cross-fade is 140 ms on a new run, 260 ms when the dwelt run replaces the moving one (§10), 160 ms on a setting
  change, 120 ms on entering or leaving follow and 200 ms on sound on. **A chord is never retriggered.** Against
  REQ-GUI-050 for a struck instrument, RQ-452.
- **The follow track**: a pool of **32 persistent voices** of the instrument on one bus (rebuilt when the instrument
  changes, the old bus falling to 0 at 30 ms). At each frame, with the frame's peaks between 30 Hz and 6 kHz:
  1. Each sounding voice takes the nearest unused peak **within a semitone** (`|log₂(p/f)| < 1/12`) and glides to it
     (time constant 35 ms), its level `0.22·a/Σa × loudness`; a voice with none fades out at 35 ms.
  2. Each unclaimed peak takes the first silent voice: its pitch is set while silent, then it fades in at 70 ms (a struck
     one is struck). A peak with no silent voice left is not voiced.
  3. The pan follows the peak's pan × 0.9 in stereo, 0 in mono, at the same time constant.

  This is partial tracking: a line that drifts is heard as a glide, not as a sequence of re-attacks. A voice that is
  struck or retriggered sounds again when its line has moved by more than a quarter-tone since it last sounded (§8).
- **Silence** (the run ended, nothing playable, sound off): every voice falls to 0 at 40 ms.

## 6. Channels

| `Channels` | Left | Right |
|---|---|---|
| **Stereo by winding (default)** | `pos`: anticlockwise winding (positive frequencies), the rotation's included | `neg`: clockwise winding (negative frequencies) |
| Stereo | `fold`: `θ` (and the rotation), both directions folded | `b`: `φ` |
| Mono | `mono`: everything summed in power | — |

So in the default a system that winds one way round the sphere is heard on one side. Under *Stereo by winding* channel
B (`φ`) reaches no side of the chord; in the follow track a line is found in `mono`, `φ` included, and panned by its
bin's `pos` and `neg` (RQ-458).

## 7. Loudness

```
rms   = hypot(rms(Re A), rms(Im A), rms(B)/π [the two angle signals] or rms(B) [shape vector],
              hypot(rms(cos ψ), rms(sin ψ)) [the default signal only])         rms: the population deviation over the window
level = 0                                   if not rms > RMS_FLOOR (10⁻⁷, RQ-442)
      = clamp(√min(1, rms/0.1), min, max)   Loudness: follows the shape's movement (default)
      = clamp(1, min, max)                  Loudness: normalised, every system at full level
```

`Min level` (0 to 100 %) defaults to 15 %, `Max level` (5 to 100 %) to 100 %, `Volume` (the master, 0 to 100 %) to 60 %,
reached at a time constant of 50 ms, and 0 while the sound is off. `Volume` scales the voices; the events take their own
level (§9).

**The floor is there so that a system that does not move is silent rather than normalised up to its rounding noise.**
The demo's floor is set against its own integrator; the human's file says the shipped integrator's drift is quieter
and asks for the floor to be set against it (Q3, RQ-442). The physics review of PR #267 found that the equal-mass
Lagrange triangle is linearly unstable, its shape rms growing with the horizon under any integrator, with an f64 floor of
4.9e-8 under *Angles only* (RQ-442 records the measurements); the value waits on RQ-442. *Was:* "prin-rs's is far
quieter, and the floor is set against it (RQ-442)." (physics review 5595386853 of PR #267, P1; applied per R-369).

## 8. Instruments

**The spectrum decides the pitches and their weights; the instrument only decides the timbre each partial is voiced
with**, so changing instrument changes nothing that is measured or displayed. There are 39 instruments; the default is
**Sine (pure)** in the code, Warm sine in the human's file (RQ-454). Their recipes, transcribed from the demo's
`INSTRUMENTS` as written:

```
sine:      { name: 'Sine (pure)', a: 'sine', lvl: 1 }
warm:      { name: 'Warm sine', a: [1, 0.35, 0.12, 0.05], lvl: 0.9 }
organ:     { name: 'Organ', a: [1, 0.8, 0.6, 0.0, 0.45, 0.0, 0.3, 0.25], lvl: 0.55, vib: [6, 6] }
pad:       { name: 'Soft pad', a: 'sawtooth', b: 'sawtooth', mix: 1, ratio: 1, det: 9, lp: 2.2, q: 0.6, att: 0.35, lvl: 0.45 }
strings:   { name: 'Strings', a: 'sawtooth', b: 'sawtooth', mix: 0.7, ratio: 1, det: -6, lp: 3.5, q: 0.8, vib: [5.5, 12], att: 0.12, lvl: 0.4 }
cello:     { name: 'Cello', a: 'sawtooth', lp: 2.2, q: 2.5, vib: [4.8, 16], att: 0.08, lvl: 0.5, oct: -1 }
choir:     { name: 'Choir (aah)', a: 'sawtooth', b: 'sawtooth', mix: 0.8, ratio: 1, det: 7, formant: [[730, 9], [1090, 11], [2440, 14]], vib: [5, 10], att: 0.2, lvl: 1.4 }
flute:     { name: 'Flute', a: [1, 0.12, 0.05], breath: 0.06, vib: [5, 8], att: 0.06, lvl: 0.9 }
clarinet:  { name: 'Clarinet', a: [1, 0, 0.55, 0, 0.35, 0, 0.2, 0, 0.12], lp: 6, q: 0.7, att: 0.04, lvl: 0.6 }
brass:     { name: 'Brass', a: 'sawtooth', lp: 5, q: 1.4, att: 0.05, lvl: 0.45, vib: [5, 5] }
glass:     { name: 'Glass bell (FM)', a: 'sine', b: 'sine', fm: 2.2, ratio: 3.5, strike: 1.6, sus: 0.25, lvl: 0.8 }
epiano:    { name: 'Electric piano (FM)', a: 'sine', b: 'sine', fm: 1.1, ratio: 1, strike: 0.9, sus: 0.35, lvl: 0.9 }
marimba:   { name: 'Marimba', a: [1, 0, 0, 0.25, 0, 0, 0, 0, 0, 0.08], strike: 0.35, sus: 0.2, lvl: 1.1 }
musicbox:  { name: 'Music box', a: [1, 0, 0.4, 0, 0.2], strike: 0.8, sus: 0.25, oct: 1, lvl: 0.8 }
harp:      { name: 'Harp', a: 'sawtooth', lp: 4, q: 0.5, strike: 0.7, sus: 0.3, lvl: 0.55 }
chip:      { name: 'Chiptune', a: 'square', lvl: 0.3 }
theremin:  { name: 'Theremin', a: 'sine', vib: [6.5, 28], glide: 0.12, lvl: 1 }
sub:       { name: 'Sub drone', a: 'sine', b: 'sine', mix: 0.8, ratio: 0.5, lvl: 0.9, att: 0.3 }
saw:       { name: 'Raw saw', a: 'sawtooth', lvl: 0.35 }
kalimba:   { name: 'Kalimba', a: 'sine', b: 'sine', mix: 0.35, ratio: 5.4, strike: 0.6, sus: 0.15, lvl: 1 }
steeldrum: { name: 'Steel drum', a: [1, 0.55, 0.3, 0.12], b: 'sine', mix: 0.3, ratio: 2.02, strike: 0.55, sus: 0.22, lvl: 0.8 }
vibes:     { name: 'Vibraphone', a: 'sine', b: 'sine', mix: 0.22, ratio: 4, strike: 1.3, sus: 0.4, trem: [5.5, 0.45], lvl: 1 }
celesta:   { name: 'Celesta', a: [1, 0, 0.3, 0, 0.1], strike: 0.6, sus: 0.2, oct: 1, lvl: 0.9 }
glock:     { name: 'Glockenspiel', a: 'sine', b: 'sine', mix: 0.4, ratio: 2.76, strike: 0.8, sus: 0.15, oct: 2, lvl: 0.7 }
tubular:   { name: 'Tubular bells', a: 'sine', b: 'sine', fm: 1.3, ratio: 2.76, strike: 2.6, sus: 0.35, lvl: 0.8 }
gong:      { name: 'Gong', a: 'sine', b: 'sine', fm: 0.1, ratio: 1.41, fmenv: [4, 0.8, 1.2], retrig: true, strike: 3.2, sus: 0.4, oct: -1, lvl: 0.9 }
bowglass:  { name: 'Bowed glass', a: 'sine', b: 'sine', mix: 0.15, ratio: 2, att: 0.6, trem: [0.35, 0.35], lvl: 1 }
tanpura:   { name: 'Tanpura / sitar drone', a: 'sawtooth', b: 'sawtooth', mix: 0.3, ratio: 2, lp: 3, q: 7, fsweep: [0.22, 2.2], vib: [4, 4], att: 0.15, lvl: 0.35 }
wind:      { name: 'Wind (tuned noise)', a: 'sine', oscOff: true, breath: 1.6, breathQ: 28, att: 0.25, lvl: 1 }
whistle:   { name: 'Whistle', a: 'sine', breath: 0.25, breathQ: 40, vib: [5.5, 10], lvl: 0.8 }
ooh:       { name: 'Voice: ooh', a: 'sawtooth', b: 'sawtooth', mix: 0.8, ratio: 1, det: 7, formant: [[300, 10], [870, 12], [2240, 14]], vib: [5, 10], att: 0.2, lvl: 1.6 }
ee:        { name: 'Voice: ee', a: 'sawtooth', b: 'sawtooth', mix: 0.8, ratio: 1, det: 7, formant: [[270, 10], [2290, 14], [3010, 16]], vib: [5, 10], att: 0.2, lvl: 1.6 }
oh:        { name: 'Voice: oh', a: 'sawtooth', b: 'sawtooth', mix: 0.8, ratio: 1, det: 7, formant: [[570, 10], [840, 12], [2410, 14]], vib: [5, 10], att: 0.2, lvl: 1.5 }
grain:     { name: 'Grain cloud (approximation)', a: 'sine', b: 'sine', mix: 0.5, ratio: 1, det: 13, trem: [23, 0.85], vib: [9, 18], lvl: 0.9 }
supersaw:  { name: 'Synth: supersaw lead', a: 'sawtooth', unison: [-18, -9, 9, 18, -4], umix: 0.55, lp: 6, q: 0.9, att: 0.02, lvl: 0.22 }
anabass:   { name: 'Synth: analogue bass', a: 'sawtooth', b: 'square', mix: 0.6, ratio: 0.5, lp: 1.2, q: 4, fenv: [9, 1.4, 0.18], retrig: true, oct: -1, lvl: 0.5 }
acid:      { name: 'Synth: acid (resonant sweep)', a: 'sawtooth', lp: 2.5, q: 14, fsweep: [0.6, 2], fenv: [7, 2.5, 0.12], retrig: true, lvl: 0.35 }
pwm:       { name: 'Synth: PWM pad', a: 'square', b: 'square', mix: 0.9, ratio: 1, det: 6, bvib: [0.35, 22], lp: 3, q: 0.6, att: 0.4, lvl: 0.3 }
fmbass:    { name: 'Synth: FM bass', a: 'sine', b: 'sine', fm: 0.6, ratio: 1, fmenv: [3.2, 0.6, 0.2], retrig: true, oct: -1, lvl: 0.95 }
```

The demo's menu lists the five synths under "Synths" without the "Synth: " prefix. The fields:

| Field | Meaning |
|---|---|
| `a` | the main oscillator: `sine`, `square`, `sawtooth`, `triangle`, or a harmonic list `[h₁, h₂, …]`, a periodic wave whose harmonic `n` has sine amplitude `h_n` and no cosine part |
| `b`, `mix`, `ratio`, `det` | a second oscillator at `ratio × f` (default 1), detuned `det` cents (default 0), mixed in at `mix` |
| `fm` | `b` instead modulates `a`'s frequency, at a depth `fm × f` Hz |
| `fmenv: [from, to, τ]` | on each note the FM depth jumps to `from × f` and settles to `to × f` at time constant `τ` |
| `lp`, `q` | a low-pass at `min(18 kHz, lp × f)`, its Q `q` (default 0.7) |
| `formant: [[Hz, Q], …]` | fixed band-passes in parallel instead of the low-pass (vowels) |
| `fenv: [from, to, τ]` | on each note the low-pass jumps to `from × f` and settles to `to × f` at `τ` |
| `fsweep: [rate, depth]` | an LFO at `rate` Hz sweeping the low-pass by `depth × f` |
| `vib: [rate, cents]` | vibrato on `a`, `b` and the unison copies |
| `bvib: [rate, cents]` | vibrato on `b` alone |
| `trem: [rate, depth]` | tremolo: the gain `1 − depth/2 + (depth/2)·sin` |
| `unison: [cents, …]`, `umix` | detuned copies of `a`, each at gain `umix` (default 0.5) |
| `breath`, `breathQ` | white noise through a band-pass at `f`, Q `breathQ` (default 6), at gain `breath` |
| `oscOff` | the oscillators silent: only the breath sounds |
| `att` | the attack: a level change is reached at a time constant `max(τ, att/3)` |
| `glide` | the minimum glide time constant |
| `oct` | an octave offset: the voice sounds at `f × 2^oct` |
| `lvl` | the voice's level (default 1) |
| `strike`, `sus` | **struck**: on a strike the gain rises to the level at 4 ms and, from 20 ms on, decays at a time constant `strike/3` **to a held floor `sus` × the level** (`sus` default 0.25), never to zero (RQ-456) |
| `retrig` | a note event (the `fenv` and `fmenv` envelopes) when the partial moves on, the gain held |

**A voice** (`makeVoice`), for an instrument `I` and the output it feeds:
1. The output gain starts at 0 and feeds a stereo panner. A tremolo, if any, sits before it; the voice's sum (`dest`)
   is the tremolo's input, else the output.
2. `a` feeds a pre-gain of `lvl`; the pre-gain feeds the formant band-passes, summed, or else the low-pass, or else
   `dest` directly. With `oscOff` the pre-gain is 0.
3. The unison copies feed the pre-gain; `b` feeds `a`'s frequency through an FM depth gain if `fm`, else the pre-gain at
   `mix`; the breath feeds `dest`.
4. The vibrato, `b`'s vibrato and the sweep are LFOs shared across voices, one oscillator per rate; the Doppler offset
   (§9), in cents, is added to `a`, `b` and the unison copies' detune.
5. **Setting the pitch** `f` (glide time constant `τ`, or at once): `x = f·2^oct`; `a` and the unison copies at `x`, `b`
   at `x·ratio`, the FM depth at `x·fm` (without `fmenv`), the low-pass at `min(18 kHz, x·lp)` (without `fenv`), the
   breath's band-pass at `x`, the sweep's depth at `x·fsweep[1]`, each at the time constant `max(τ, glide)` where the
   instrument has a `glide`; set at once, it is also a note event.
6. **A note event** starts the `fenv` and `fmenv` envelopes at the current pitch; it leaves the gain alone.
7. **Setting the level** of an instrument that is not struck moves the output gain at `max(τ, att/3)`; a struck
   instrument's level changes only by its strikes. **A strike** of one that is not struck is a level change at 20 ms.
   **Silence** cancels what is scheduled and falls to 0 at the given time constant.

**Struck instruments** (the twelve with `strike`: glass bell, electric piano, marimba, music box, harp, kalimba, steel
drum, vibraphone, celesta, glockenspiel, tubular bells, gong; and, with `retrig` instead, analogue bass, acid and FM
bass; the gong has both):
- They are **struck once and ring at their floor**. The first build re-struck them on a timer and they sounded like a
  loop; the second decayed to zero and they played once and died. The floor is the fix for both.
- **They sound again on the trajectory's own beats** (`Strikes on`), in follow mode: each syzygy the head passes
  (**default**), or every crossing time ÷ 1, 2 or 4 (default 2), the beats between the last tick and this one counted;
  a beat strikes every sounding voice at its level, or gives a `retrig` voice a note event. *Only when a line changes*
  adds no beats. In every mode a struck or `retrig` voice also sounds again when its line has moved by more than a
  quarter-tone (`1/48` octave) since it last sounded (RQ-457).

The menu groups them in families (§G6): pure (5), keys (3), struck (10), bowed and blown (7), voices (4), synths (8),
textures (2). Which instrument is in which family is RQ-459.

## 9. Events and effects

**Events** are one-shot sounds fired when the head passes them in follow mode, each with its switch (`Events`), at `Event
level` (0 to 150 %, default 80 %), panned by the bodies involved, bodies 1, 2, 3 at −0.8, 0 and +0.8:

| Event (default) | Detected | Sound |
|---|---|---|
| Close approach (on) | at a sample `k` (not the first or last) where a pair's separation `d_k` is a local minimum (`d_k < d_{k−1}`, `d_k ≤ d_{k+1}`) below `0.2 R_k`, `R = √I` about the centre of mass, before the run's end (`t_k < t_end − 10⁻⁶`); its depth `min(1, log₁₀(0.2 R_k / d_k)/2)` (RQ-444) | a band-passed noise whoosh, panned between the pair, its centre rising from 280 Hz to `900 + 3200·depth` Hz over 0.33 s, Q 5, 0.55 s, rising to `0.35 + 0.4·depth` in 0.18 s |
| Collision (on) | the run's terminal collision | 1–2 a low tom: a sine 190 → 70 Hz over 0.25 s, 0.5 s, 0.9, and a 30 ms low-passed (2.5 kHz) noise tick at 0.4; 1–3 a snare: high-passed (1.2 kHz, Q 0.7) noise, 0.22 s, 0.55, and a triangle 200 → 170 Hz, 0.12 s, 0.45; 2–3 a rimshot: band-passed (2.6 kHz, Q 9) noise, 70 ms, 0.8, and a 1750 Hz square, 50 ms, 0.35; each panned between its pair; triple: a crash, centred: high-passed (3.5 kHz, Q 0.7) noise, 1.8 s, 0.6, and a triangle 420 → 300 Hz, 0.6 s, 0.25 |
| Escape / ejection (on) | the run's terminal escape | a falling glide into the escaping body's ear: a sine 950 → 110 Hz over 1.2 s, 1.3 s, 0.45, and band-passed noise 2.4 kHz → 300 Hz, Q 2, 1 s, 0.12; a triple ejection glides to all three, each 0.12 s after the last and 120 Hz lower |
| Syzygy tick (off) | each syzygy of the trace (R-538 I9) | a click panned by the middle body: 1350 Hz for 60 ms at 0.55 and 2650 Hz for 35 ms at 0.25 |

A noise burst is the shared white noise from a random offset through its filter, the filter's frequency ramped
exponentially over 0.6 of its length, its gain rising linearly to its peak over its attack (3 ms unless given) and
falling exponentially to 10⁻⁴ at its end; a tone starts at its peak and falls the same way, its frequency ramped
exponentially over its bend (its length unless given). Each fires 5 ms after the tick that passes it, into the effects
chain's event input.

**Effects** each have a switch and an amount; in follow mode they read the run at the head (the sample at or before
it), for a chord its mean (`R` and `L_z` averaged over the samples, `Ṙ` taken as 0); the entropy is the frame's or the
whole run's. Along the run: `R = √I` and `L_z = Σ m_i (x_i v_{y,i} − y_i v_{x,i})`, both about the centre of mass, `Ṙ`
by central differences (one-sided at the ends), `E` and `t_c` at `t = 0`, `R₀` the first sample's `R` (1 if 0).

| Effect (switch, amount range, default) | Driven by | Mapping |
|---|---|---|
| Reverb (off in the code, on in the file, RQ-454; 0 to 2.5, ×1.40) | the spread `R/R₀` | wet `= min(1.6, a·(0.12 + 0.9·clamp(log₂(R/R₀)/3, 0, 1)))` at 150 ms, through a 3.6 s impulse: a tight system dry, a wide one in a hall; off, 0 |
| Doppler (off; 0 to 3, ×1.50) | the scale-free expansion rate `Ṙ t_c / R` | detune `= −400·a·Ṙ t_c/R` cents, clamped to `±1200·clamp(a, 0.25, 2)`, at 80 ms: expanding bends down, contracting up; off, 0 |
| Bit-crush (off; 0 to 2, ×0.60) | the spectral entropy `H` | `bits = round(clamp(15 − 40a·max(0, H − 0.12), 3, 16))`: a clean line stays clean, noise frays; off, 16 (no crush) |
| Stereo width (off in the code, on in the file, RQ-454; 0 to 3, ×1.50) | the scale-free angular momentum `√(L_z²·|E|)` | `w = clamp(a·√(L_z²|E|)/0.08, 0.15, 2)` at 100 ms, mixed as `L′ = (1+w)/2·L + (1−w)/2·R`, `R′ = (1−w)/2·L + (1+w)/2·R`: released from rest is narrow, a rotating system wide; off, `w = 1` |

All four inputs are scale-free, so the effects keep §1's scale invariance. The crush is a 4096-point wave-shaper on
`[−1, 1]`, `round(x·2^{bits−1})/2^{bits−1}`, the identity at 16 bits. The impulse is stereo white noise ×
`(1 − i/len)^{2.6}`. **The chain:** the voices (through `Volume`) and the event input → bit-crush → the width matrix →
dry, and through the convolver and the wet gain → a dynamics compressor at the platform's defaults → the output. The
readout reads "spread ×R/R₀ · Doppler ±n¢ · b bits · width w %".

## 10. Horizon and dwell

- While the hover is moving the run is integrated to `t ≤ 20` (the demo's maps' horizon), so the sound is coarse.
- When a moving hover's run has reached that horizon without an event, and Listen's `Horizon` (20, **60**, 150, 400)
  is longer, after a dwell of 280 ms (at once for a pin) the same run is integrated on to the horizon, if the pointer
  has not moved on, and the trace and the sound both switch to it (cross-fade 260 ms) and sharpen. Changing the
  horizon re-extends the current run; a known orbit (§12) runs to it at once.
- A run ended early by its event is a short run, so a coarse spectrum; that is information, not a fault.

The hover's own horizon and step budget in the app, and Listen's horizon against the Run window's, are RQ-464.

## 11. Display

**The spectrum** (at the head, or the whole run; the analysis at `N = 4096`):
- log frequency 30 Hz to 6 kHz, gridded at the A of each octave and labelled "A1 55" to "A7 3520";
- dB rows at 0, −20, −40 and −60 relative to the strongest bin (above bin 1) of the three magnitudes;
- the 110–880 Hz fit band shaded (`rgba(26,124,163,0.10)`) and labelled "fit band";
- left channel blue `#5ab4ff` (anticlockwise), right amber `#e0a030` (clockwise), mono `#dcdcdc`, each a curve over bins
  2 and up, stroked 2 px and filled at 12 % opacity, the right drawn first;
- the top four mono peaks dotted on the curve (the first white, larger), with chips showing the note with its cents
  (`m = 69 + 12 log₂(f/440)`, the nearest semitone named C … B and octave `⌊n/12⌋ − 1`, then `±c¢`) and its Hz;
- the strongest line's harmonics 2f to 8f, up to 6 kHz, as dashed ticks labelled "2f" to "8f";
- a header naming the window ("whole run" or "at the head · t a–b"), the entropy, the pitch mapping and any octave
  shift, and the two sides ("anticlockwise · left / clockwise · right", or "θ · left / φ · right").

**The spectrogram** (the whole run, on the run's own time axis; frames, §4):
- 240 log-spaced rows from 30 Hz to 6 kHz, each frame a column, in the magma palette (`#000004 #120d31 #331067 #59157e
  #7e2482 #a3307e #c83e73 #e95462 #fa7d5e #fea973 #fed395 #fcfdbf`, interpolated linearly);
- **each frame shown in dB above its own mean over ±½ octave** (±16 rows), with a range of 14 dB: `v = ((dB − mean +
  3)/14)^{0.8}`, clamped to `[0, 1]`, each column's values floored at its maximum − 90 dB. On raw dB the broad slope every
  run carries fills the panel and hides the lines; this keeps the lines and drops the slope. **Display only: the sound is
  untouched**, and the panel's header says "dB above the local mean over ±½ octave";
- bins below the fourth (indices 0 to 3), and bins past the last, are left blank: they hold the window's own leakage, not
  lines;
- **an event lane** above: syzygies as ticks in the middle body's colour, close approaches as triangles, the end event
  as a diamond in its outcome colour (colour_composition §1.4);
- time ticks at a 1, 2 or 5 × 10ⁿ step near an eighth of the span; the playhead, white, 2 px, with a chip "t x", while
  following, scrubbing or short of the run's end. In the GUI the strip is on the scrubber's time axis, blank past the
  run's end (R-541 G7).

**The readout**: the pitch mapping (`1/t_c → f_ref Hz`, and the octave shift); the window; `t_c`; the entropy ("0 a
pure line, 1 noise"); the movement, shape rms → loudness with the clamps, or "the shape does not move, so there is
nothing to play"; the periods per window of the strongest line (with "under 20: too short to resolve a note" below 20,
RQ-450); the top four peaks as notes with Hz; the named interval of the top two; the run (its horizon, its end event,
"truncated: coarser" for an incomplete run); and the reason for any silence ("silence: not a system", "silence: the run
ended at t = … by …").

## 12. Sweep and known orbits

- **Sweep a row**: the pointer moves along the current run's row of the chart (the middle row without one, kept 2 % in
  from the edges) from 2 % to 98 % of the view's width over `Sweep` seconds (3 to 30, default 10), each step a pinned
  hover, every 60 ms; pressed again, it stops. So the sound of a slice is heard as a gesture (pointer_channels §7).
- **Known orbit** loads a named IC as the trace, pinned, integrated to the horizon at once, with total mass 1 and
  `G = 1`:
  - **figure-eight**: masses ⅓ each; `r₁ = (−0.97000436, 0.24308753)`, `r₂ = −r₁`, `r₃ = 0`; with
    `v = (−0.93240737, −0.86473146)` and `s = 1/√3`, `v₁ = v₂ = −v·s/2`, `v₃ = v·s`;
  - **figure-eight perturbed 1 %**: the same with body 1's `v_x` × 1.01;
  - **Burrau 3:4:5**: masses `(3, 4, 5)/12` at `(1, 3)`, `(−2, −1)`, `(1, −1)`, less their centre of mass, at rest;
  - **Lagrange rotating**: equal masses at `(cos 2πi/3, sin 2πi/3)`, each moving at `ω(−y, x)`, `ω = √(1/(3√3))`.

  These are the calibration set for the tests. Lagrange rotating is linearly unstable with equal masses (Routh's
  27Σm_im_j/M² = 9 > 1) and breaks up by t ≈ 150 under any integrator, so it is calibration only before that: its tests
  state t ≤ 60 or wait on RQ-442 (physics review 5595386853 of PR #267, P1; applied per R-369).

## 13. Tests (each with the arm that fires)

| Test | Passes when | Fires on | Requirement |
|---|---|---|---|
| Scale invariance | a system and its copy scaled by `α` (times by `α^{3/2}`) give the same peaks in Hz to 0.1 Hz | a `t_c` or `f_ref` slip; an effect input that is not scale-free | REQ-GUI-313 |
| Winding sign | the figure-eight run forwards and time-reversed swap its lines between L and R under *Stereo by winding* | a real FFT, or the sign dropped | REQ-GUI-310 |
| Wrap | the `θ` signal's spectrum has no lines at multiples of the winding rate that are absent from `e^{iθ}`'s | `θ` used instead of `e^{iθ}` | REQ-GUI-309 |
| Figure-eight | lines in exact 2:1 / 3:2 / 4:3 ratios to ±5 cents | a windowing or peak-refinement error | REQ-GUI-311 |
| Lagrange (default signal) | one line, level above the min clamp | the rotation channel dropped | REQ-GUI-309 |
| Lagrange (angles only) | level 0, because the shape rms is below the floor | the floor removed, or set above prin-rs's drift (RQ-442) | REQ-GUI-316 |
| No silent run | every run with `t_end > 0` that moves has at least one voiced line (fallback included) | the fallback missing | REQ-GUI-312 |
| Struck floor | a struck voice 10 s after its strike is at `sus × level`, not 0, and was struck exactly once without a beat | a timer re-strike, or decay to zero | REQ-GUI-317 |
| Silence after the end | follow mode past `t_end` voices nothing | a held chord | REQ-GUI-314 |
| Spectrogram is display-only | the voiced peaks are identical with the spectrogram on and off and under any display mode | the contrast step leaking into analysis | REQ-GUI-322 |

The requirements carry three test details the physics review of PR #267 added (applied per R-369): Winding sign
asserts the side levels and that the sides differ (P3); Scale invariance gains a known-answer arm, Lagrange's line at
181.94 Hz at shift 0 within ±5 cents (P4); and the Lagrange tests state their horizon or wait on RQ-442 (P1). No silent
run excepts the silences RQ-467 queues (P2).

## 14. What the demo is not

The demo is Web Audio in a browser: its time constants are `setTargetAtTime`'s, which the app's backend reproduces
(RQ-440); its integrator (Dormand–Prince 5(4), relative tolerance 10⁻¹¹, steps at most 0.004, a 400 000-step budget), its
escape test and its precomputed maps are its own, and the app's are `computeIC`'s and the run's. Where the analysis runs
is RQ-441. The questions the human queued are RQ-440 to RQ-447; the conflicts and silences the port found, RQ-448 to
RQ-467 (R-540, R-541; RQ-465 from merging main, RQ-466 and RQ-467 from PR #267's review). *Was:* "RQ-448 to RQ-464"
(code review 5595371779 of PR #267, F4; applied per R-369).
