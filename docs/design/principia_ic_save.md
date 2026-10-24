# Principia: saving, sharing and exporting an initial condition

*Adopted by R-586 and R-587 (gap sweep PR 2c, 21 Oct 2026), from the human's design `ic_save.md`
(`docs/incoming/2026-10-21_gap_sweep_exports_picture_ic/designs/`), its "Decided" section his words: "Accepted as
proposed, with one change: **CSV body labels are 1-based**". Where the draft gave a choice with a recommendation (its
§5), the recommendation is what was accepted, but for that change. Queued from it: RQ-563 (the frame the constructor
integrates, against R-544) and RQ-564 (a kept orbit's `Trace`, against R-553).*

## 1. What exists

The Inspector edits one IC; bodies are shown 1, 2, 3 and stored 0, 1, 2 (R-449). *Create & locate* builds one and
offers lock, centre, mark or **keep** (`principia_render_gui_spec.md` §G8). Kept orbits are `ViewUI`
(`principia_gui_state_contract.md` §2), but a kept orbit's `Trace`, its IC and style, is `Lines` state and round-trips
with `RenderState` (R-553, RQ-428 (a)). `Trace.ic` is "the decoded IC (chart, uv, sim key)" (`principia_lines.md` §3),
a chart point, so it cannot hold an off-slice built IC. Integration enters only through `computeIC(chart, uv, simKey)`
(`principia_parity_contract.md` §1; §G8, "the engine is reached only through it"). The prototype edits velocity and
shows momentum (`docs/notes/ic_inspector_scratchpad.md`); its mirror test has the deadband `δ_λ = 1e-12` (R-82). The
constructor's sphere shows both halves and the mirror twin as a faint marker (R-544; §G8). R-449 keeps
machine-readable output 0-based; JCS writes −0 as `0` (state contract §2).

## 2. The gap

An IC cannot be copied, saved, shared or reopened. An off-slice IC has no carrier, and there is no trajectory export.

## 3. The design

### 3.1 The IC record

`principia.ic/1`, one JSON object in JCS (R-318):

```
format:    "principia.ic/1"
bodies:    "0-based; shown as bodies 1, 2, 3 (R-449)"
as_built:  { m: [3], r: [3][2], v: [3][2] }      the constructor's own variables and frame
canonical: { z: [8], mirrored, hemisphere, links, entry_hashes }
source:    "built" | { pixel: { chart, uv } }
sim:       SimConfig (canonical) ; build, schema, label, made   (made outside the identity)
```

The **primary** is `as_built` for a built IC, and `source.pixel` with `sim` for a picked one; everything else is
re-derived on load and compared. The primary velocity variable is `v`, `p = m·v` derived (the draft's open question 2,
(a): `p = m·v` is not bitwise invertible, so only one variable can be primary). **Bitwise at f64**: JCS's shortest
decimals are exact for every finite f64 but −0, so one function normalises −0 to +0 as values are written (open
question 4, (a)). A non-finite value is refused at entry, in the constructor and on load. `mirrored` is the lookup's
`lookup_mirrored` (R-544), and `hemisphere` comes from the same decision, R-82's one mirror test as the kernel makes it
(exactly, on the input, never a second test on the rounded `λ̃_y`): `−1` iff `lookup_mirrored`; `+1` iff the same test
on the reflected state; `0` otherwise, on the seam. So `mirrored ⇔ hemisphere = −1` holds by construction (applied
per R-369, fix round 4 of PR #287, physics review 5638148080, P1). *Was:* "`hemisphere` is the sign of `λ̃_y` past `δ_λ`, `+1`, `−1`, or `0` on the seam;
`mirrored` is the lookup's `lookup_mirrored` (R-544)."

**−0 at entry** (applied per R-369, fix round 5 of PR #287, physics review 5638172877, P5). The one function that turns −0 into +0 runs at entry, in the
constructor and on paste and open, as well as on write, so the in-memory `as_built`, `E(as_built)` and
`computeICState`'s input are always the values the record holds. The reflection the `+1` test runs on is `Π`, the
full state reflected through the x-axis, every `rᵢ` and every `pᵢ`, as `C`'s step 3 reflects it
(`principia_inverse_encode_contract.md` Part 6).

JSON has no literal for a non-finite number, so on load a number token that overflows f64 (for example `1e999`) is
read as non-finite and refused, naming the field, and a bare `NaN` or `Infinity` does not parse. `made` is when the IC
was made: a re-save of an opened record keeps the record's `made`, so save, open and re-save give the same bytes
(applied per R-369, fix round 2 of PR #287, qa review 5638084688, Q6).

### 3.2 Copy, save, open

- **Copy**: Ctrl+C in the Inspector copies the record as its JSON text; *Copy as text* gives a 1-based table, read by a
  person (bodies 1, 2, 3; R-449). **Ctrl+V** in the Inspector pastes a record, as *Open IC…* does.
  While a text field is being typed in (a mass, a position, a velocity), Ctrl+C and Ctrl+V copy and paste its text,
  never the record, as every chord in §G3's table does there (R-409; applied per R-369, fix round 1 of PR #287, gui review 5638052211, G2).
- **Save**: *File › Save IC…* writes `<label>.principia-ic.json`.
- **Open**: *File › Open IC…*, or a drop of the file, opens it in *Create & locate*, in its as-built frame, with one of
  three outcomes:
  - **recovered**;
  - **physics only**, when a link is unknown or its entry hash differs (R-521): `as_built` loads, since it does not
    depend on the decoder, and only "locate on this chart" is refused, naming the link and why;
  - **corrupt**, when the JSON does not parse, a field is missing or non-finite, or the re-derived values contradict
    the stored ones beyond what the residual reports.
  The stored `z` against the recomputed `z` is shown as a residual.

**Corrupt or a residual** (applied per R-369, fix round 2 of PR #287, qa review 5638084688, Q6). A record is corrupt when its JSON does not parse, a field
is missing, a value is non-finite, or a field re-derived exactly from the primary contradicts the stored one
(`mirrored`, `hemisphere`, or `source`'s kind against the fields present). `canonical.z = E(as_built)` is a
deterministic f64 function, so reopened on the build and platform that saved it the recomputed `z` is bitwise the
stored one and the residual is exactly 0; a residual arises only from another platform's maths library (`atan2` in
`C`'s step 2, the chart inverses) or another build. The stored `z` against the recomputed is that residual, measured
as `‖Δz‖`, the norm REQ-VAL-022 gates `‖z − z′‖` by: within its bound the record is recovered with the residual
shown, beyond it corrupt (fix round 4 of PR #287, physics review 5638148080, P4, applied per R-369). *Was:* "`canonical.z` is re-derived through the decoder,
which is not bitwise, so the stored `z` against the recomputed is a residual: within its bound the record is
recovered with the residual shown, beyond it corrupt." The residual is 0 on the saving machine because `as_built` is
normalised at entry (§3.1): a −0 left in the constructor's `as_built` would turn `atan2(−0, ρ̃_x)`, `ρ̃_x < 0`, into
`−π` before the save and `+π` after it (fix round 5 of PR #287, physics review 5638172877, P5). The bound is a calibration value (R-71, REQ-TOOL-215), proposed
with its evidence by the task that builds the record and confirmed by the human at the M8 gate; a link that is
unknown or whose entry hash differs is physics only, never corrupt (R-521).

### 3.3 The mirror twin

The constructor shows both hemispheres with a faint marker at the twin (R-544). `as_built` keeps the hemisphere it was
built in; `canonical` is folded, recording `mirrored` and `hemisphere`. A reopened IC sits where it was built, with its
twin opposite, and on the seam (`hemisphere = 0`) there is no twin.

### 3.4 Integration

The constructor integrates `canonicalise(as_built)`, the survey's frame, and says so (the draft's open question 3, (a)).
That needs a **state-in entry** beside `computeIC(chart, uv, simKey)`, since an off-slice IC has no `uv` and decoding
`z` is not bitwise:

```
computeICState(state, simKey) → SimState     // one IC given as its full state, full f64
```

the same integration `computeIC` runs after its decode, from the state given, on the CPU in f64 (RK45 under its
tolerances, R-33), and the only other way into the engine the Inspector has. A picked IC still enters through
`computeIC`.

**Which frame `canonicalise` gives, under R-544, is RQ-563.** R-544 makes the shape sphere chart's decode signed, the
mirror "never identified", and `C(x)` taken there with its step 3, the mirror, not taken. Meanwhile the constructor
integrates `C(as_built)` with step 3 not taken, so the state integrated stays in the hemisphere it was built in, as the
shape sphere chart's own decode does; `canonical.z`, `mirrored` and `hemisphere` are recorded as above. Nothing waits.

**What `C` does here** (applied per R-369, fix round 4 of PR #287, physics review 5638148080, P2): steps 1a, 0, 1b and 2 are taken and step 3 is not, so the
integrated initial state is bitwise the shared `C`'s step-3-not-taken result on `as_built` (REQ-CHART-081,
TASK-M2-43): the centre of mass at the origin and at rest, `I = 1` (the rescale `λ` recorded, `lookup_rescaled`),
`ρ̃` on +x, `λ̃_y`'s sign kept. Skipping the rescale or the boost would integrate another system in the survey's
units, whose horizon is in those units (`principia_integrator_contract.md` Part 5).
`C` runs on the `as_built` normalised at entry (§3.1), so a reopened IC integrates bitwise the state its first run did
(fix round 5 of PR #287, physics review 5638172877, P5).

### 3.5 In views: marks

*Keep* stays scratch: the kept-orbit list and its tabs are `ViewUI`. **Mark** becomes the durable act: `Trace.ic` widens
to

```
IcRef = OnChart { chart, uv } | Built(IcRecord)
```

inside `RenderState.Lines` (the draft's open question 1, (a): marks are drawn, so they are render state), and a mark is a
`Trace` of the new kind `Marked(id)` (applied per R-369: the kind that holds a mark), drawn as REQ-GUI-120's crosshair at
its IC's projection on the current slice and, under the trace settings, its path. So a marked IC travels in snapshots,
saved views, share links, the animation spec and the pxpack (tier 2), round-tripping under R-484. *Mark all kept*
promotes the kept list to marks. One record is roughly 0.6 KB (an estimate, to be measured), so marks crowd 64²'s tier
2 (1,516 B, embedding §7): tier 2 drops whole, and `tEXt` keeps everything.

**Mark** sends one `SetField` on `RenderField::Lines` and makes one undo entry, and *Mark all kept* with n kept orbits
is one `SetField` and one undo entry, never n (R-69, R-52; applied per R-369, fix round 1 of PR #287, gui review
5638052211, G1).

**A kept orbit's `Trace` is RQ-564.** R-553 (RQ-428 (a)) made a kept orbit's `Trace`, its IC and style, `Lines` state
that round-trips with `RenderState`; the draft's "Keep stays scratch" and its test "kept-only ICs are absent" were
written before it. Meanwhile R-553 stands: a kept orbit's `Trace` stays in `Lines`, its `ic` an `IcRef` too, so a kept
built IC round-trips; the kept-orbit tabs stay `ViewUI`; and the test's "kept-only ICs are absent" arm waits on RQ-564.

### 3.6 Trajectory export (R-587)

*Export trajectory…* in the Inspector writes `<label>.traj.csv`:

- **A `#` preamble** holds the IC record (§3.1); the integrator (RK45 at f64 and its tolerances, R-33); the outcome and
  its `detail`, 0-based as the payload stores it, with its 1-based gloss (R-449); "incomplete: step budget" where that
  applies (`principia_scratchpad_pointer_channels.md` §3); and the line **"bodies are labelled 1-based (body1, body2,
  body3), as everywhere they are shown; the record's own fields above are 0-based (R-449)"**.
- **The columns** are `t`, then each body's position and velocity, **labelled 1-based** (the human's change: "CSV body
  labels are 1-based (`body1_x` …), as bodies are always displayed 1-based"): `body1_x, body1_y, body2_x, body2_y,
  body3_x, body3_y, body1_vx, body1_vy, body2_vx, body2_vy, body3_vx, body3_vy`, then `E, Lz`, then the draft's
  `n0, n1, n2` and `dE_E`. What `n0, n1, n2` are, and their names, is a definition (R-72, REQ-TOOL-213): the draft does
  not say, and a pair- or body-indexed column is named as R-449 and this section name the others.
- **One row per step as taken**, with no resampling, which would invent values. An optional stride is labelled "every
  k-th step; close encounters not resolved" (`principia_00_philosophy.md` §7.3).
- **Frame and units** (R-72, REQ-TOOL-213; applied per R-369, fix round 4 of PR #287, physics review 5638148080, P3): every row is in the frame §3.4
  integrates, `C(as_built)` with step 3 not taken (the centre of mass at the origin and at rest, `I = 1`, `ρ̃` on +x at
  `t = 0`), in the canonical units the integrator inherits, `G = M = I = 1` (`principia_integrator_contract.md`
  Part 5): `t` in canonical time, the positions and velocities in canonical length and velocity, `E` and `Lz` in
  canonical energy and angular momentum. The preamble states that frame and the rescale `C` applied, `λ` as
  `lookup_rescaled` reports it (`principia_inverse_encode_contract.md` Part 6, step 0), so a row can be related to the
  `as_built` the record holds; the columns after `Lz` carry their frame and unit in their definition.
- **Back to `as_built`** (applied per R-369, fix round 5 of PR #287, physics review 5638172877): a row's frame is `as_built`'s translated by its centre of
  mass, boosted by its centre-of-mass velocity, rescaled by `λ` and rotated by `θ`, step 2's angle; the preamble states
  the centre-of-mass offset and velocity and `θ` beside `λ`, each recomputable from the `as_built` it holds.

## 4. Fit with rulings

- **Amends** state contract §2 and lines §3 (`Trace.ic` becomes `IcRef`, a `Marked` kind), and R-484's tier table
  (marks are tier 2, in `Lines`).
- **Amends** `computeIC`'s contract (parity contract §1; §G8's "the engine is reached only through it";
  REQ-GUI-117): a state-in entry beside it. The draft said "This needs a ruling"; the human accepted the proposal as it
  stands, and R-586 records it.
- **Amends** R-449 for the trajectory CSV's column labels only: they are 1-based, and the preamble says so. The record's
  own fields, the payload's `detail` and every other machine-readable export stay 0-based and say so (R-587).
- R-544: the twin is shown (§3.3); the frame integrated is RQ-563. R-553: a kept orbit's `Trace` is RQ-564.
- §G8 gains Copy, Save, Open and Export trajectory, and File gains *Save IC…* and *Open IC…*.

## 5. Decided

The human, 21 Oct 2026 (`ic_save.md`, "Decided", verbatim):

> Accepted as proposed, with one change: **CSV body labels are 1-based** (`body1_x` …), as bodies are always displayed
> 1-based. The preamble says so.

`DECISIONS_2026-10-21.md`, verbatim: "Saving a starting condition: yes, as drafted (CSV bodies labelled 1-based,
body1/body2/body3)."

## 6. Tests

| Test | Fails when |
|---|---|
| Save, open and re-save give a byte-identical file; `as_built` is bitwise over 10⁴ random ICs, −0 included | a lossy number format, or −0 survives |
| Built in the lower hemisphere, reopened: dot lower, twin upper; on the seam, no twin | `as_built` is replaced by the canonical form |
| A changed link entry hash loads the physics and refuses to locate it, naming the link | a default link stands in |
| A marked IC survives a snapshot, a saved view, `tEXt` and a share link bitwise; kept-only ICs are absent (this arm waits on RQ-564) | marks ride `ViewUI`, or `ViewUI` leaks |
| The CSV's rows equal the Inspector's steps; a truncated run is marked incomplete | resampling, or a silent cut |
| The CSV's `detail` is 0-based and the GUI label 1-based for one collision; the body columns are `body1_…` to `body3_…` | the conversion is skipped or doubled; a 0-based body label |
