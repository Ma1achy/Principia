# Results — from a view to a batch run, and back

*Draft against main @45caf8be. Everything not cited is new.*

## 1. What exists today

The README promises "the batch job, run at full precision, buys results" (`README.md`:36). Philosophy §2's prebake is
the same kernel on uniform tiles with no scheduler, at "arbitrary precision, or GPU at emulated double/double-double"
(`principia_00_philosophy.md`:42–48, table :76–81). R-265 made the shared kernel f32 and f64 only; arbitrary
precision lives only in R-33's independent reference, which checks rather than produces (`decisions.md`:2576–2597).
The GPU is f32-locked, so f64 means the CPU (canonical_spec §1 :24–25). §7.3 wants a per-pixel dump as the default
output (:325–350), and REQ-SYS-007 parks it (`requirements.yaml`:28643). TASK-M8-02's `prin survey` writes "payload
summary, image, frame records" (`TASK-M8-02.md`:20): "payload summary" appears nowhere else, and "frame records" are
the adaptive loop's. The GUI's only progress-and-cancel is the Run window's, for the live view (§G5 :960). prin-rs's
`prin` writes a self-describing `.raw` (f64 fields at either precision) and PNGs with no settings
(`src/bin/prin.rs`:22–33; `src/output/raw.rs`:1–60).

## 2. The gap

There is no in-app path to a result, no result format, and no mark that tells a result from exploration. "Payload
summary" is undefined, and philosophy §2 contradicts R-265.

## 3. Proposal (new)

**Launch.** *Make a result…* from the view, from Measure's kept region (§G10 :1242), or from the Inspector. A
**Result** window, prefilled from the view, lists every field that differs from it:
- region: plane, frame, tilt and links by value; a uniform `nx × ny`; refinement off;
- precision: **f64** (default; the shared kernel on CPU threads) or **f32** (the survey kernel on the GPU); both use
  the view's integrator occupant, not the Inspector's RK45;
- horizon `T`, `E+1` and `r_coll`; the stored times: the horizon plus any chosen playheads;
- a cost and size estimate from a **timed 64-pixel pilot**, never a constant, and a warning when `T` passes the
  precision's predictability horizon (≈23 at f32, ≈52 at f64; §7.1 :280, an estimate).

**Where it runs.** One `job.json` (`SimConfig` and `RenderState` in the canonical serialisation, a `run` block, build
hash). The app runs it in the background; *Copy command* gives `prin batch --job job.json`, the same code. Progress
shows rows done, failures and time left. **Cancel keeps what finished**, labelled "cancelled at 37 %"; *Resume*
continues, since rows are pure (§7.2 :308–314) and take their cell width from the global grid.

**Output.** A folder, `<name>__<build8>.principia-result/`:
- `result.json`: the job record, schema version, link entry hashes (R-521), host and timings, and a summary computed
  from the payload, with **failed and undetermined pixels counted, never dropped**;
- `payload.prin`: the per-pixel record at each stored time, f64 wide at both precisions, the fields §7.3's contract;
- `image.png`, whose pxpack and `tEXt` gain a tier-1 `run` block ("result, f64, 1024²");
- `log.txt`.

**Reopening.** *File › Open result…* gives a **result view**: a badge "Result · f64 · 1024² · T 50" in the header
strip (R-523) and a distinct frame. It draws from the payload and integrates nothing. Stain edits stay live and
undoable; sim-key edits are disabled with the reason; *Explore from here* leaves with the same `SimConfig`. Zoom shows
square pixels, and the scrubber snaps to the stored times.

**Compare.** A flip against the live view, labelled "visual only: different sampling", since the live view is
adaptive. The measured comparison is *Make the f32 twin*: the same job at f32, then a per-pixel difference map with its
counts and distribution.

**Philosophy §2 becomes:** "… the shared kernel at f64 on the CPU, or f32 on the GPU for a fast uniform pass.
Arbitrary precision lives only in R-33's independent reference, which checks a result rather than making one
(R-265)." The table reads "f64 (f32, labelled)" for the prebake and "f64" for the Inspector.

## 4. Fit with rulings

- R-265 holds. The §2 edit needs a ruling, since R-265 amended §7.1 and §7.7 only.
- **Amends REQ-SYS-007**: unparks §7.3's reduced dump as the payload. Resuming by rows borrows §7.2's argument without
  its VRAM tiling.
- R-484: `result.json` and the image import as the identity. The `run` block amends embedding §6's tier 1 and
  `tiers.rs`.
- TASK-M8-02: `prin survey` stays the headless live path, `prin batch` is new, and "payload summary" is struck or
  defined (Q4).
- State contract §2 :87: `ViewUI` gains the Result window and a result mode.

## 5. Open questions

1. **Name.** (a) "Result" in the GUI, "batch" for the job; (b) keep "prebake"; (c) "survey". *Recommend (a)*;
   "survey" is already taken by the live path.
2. **Stored times.** (a) horizon plus chosen playheads; (b) horizon only; (c) every sync boundary. *Recommend (a)*;
   (c) is to be sized first.
3. **Contents.** (a) per-pixel reductions plus the nominal copy; (b) all `E+1` copies; (c) (a), with (b) as an option.
   *Recommend (c)*.
4. **"Payload summary".** (a) strike it from M8-02; (b) define it as this payload; (c) define it as per-quad
   reductions. *Recommend (a)*.

## 6. Tests

| Test | Fails when |
|---|---|
| The same f64 job run twice gives a bitwise-equal payload | thread order reaches the arithmetic |
| In-app and `prin batch` give byte-identical payloads | two code paths exist |
| Cancel, resume and an uninterrupted run are bitwise equal | a row takes its cell width from its own extent |
| The payload holds `nx·ny` pixels; an injected budget-exhausted pixel is present with its reason | failures are filtered |
| f32 and f64 payloads have the same fields and widths | the width follows the precision |
| A stain edit in a result view runs 0 integrations; a sim-key edit is refused; a live view has no badge | the result view integrates, or the badge is always drawn |


## Decided (Malachy, 21 Oct 2026)

- Called **Result** ("Yeah call it result").
- **Which times are saved: the user chooses** ("let the user choose ig"): the horizon by default, plus any playheads
  the user adds. Open question 2 is closed.
- **One copy per pixel or all `E+1` copies: an option** ("make that an option"), one copy by default. Open question 3
  is closed.
- **"Payload summary" is deleted from TASK-M8-02**: it is defined nowhere, used nowhere, and no test asks for it.
