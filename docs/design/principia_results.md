# Principia: Result — from a view to a full-precision batch run, and back

**Adopted by R-572 and R-573 (21 Oct 2026, the human's gap sweep, PR 2).** This document is part of the corpus from
those rulings on: where it says "proposal", the proposal is what is built, except where R-572, R-573 or a later ruling
says otherwise. Its source is the bundle's `designs/results.md` (drafted against main @45caf8be), recorded byte for byte
in `docs/incoming/2026-10-21_gap_sweep_results_honesty/`; its four open questions are closed by the human's "Decided"
section (§5). Line numbers cited below are the draft's, against main @45caf8be.

---

## 1. What exists

The README promises "the batch job, run at full precision, buys results" (`README.md`). Philosophy §2's prebake is the
same kernel on uniform tiles with no scheduler, at "arbitrary precision, or GPU at emulated double/double-double"
(`principia_00_philosophy.md` § "The prebake — a measuring instrument" and the three-rungs table; *was*, before R-572).
R-265 made the shared kernel f32 and f64 only; arbitrary precision lives only in R-33's independent reference, which
checks rather than produces. The GPU is f32-locked, so f64 means the CPU (`principia_canonical_spec.md` §1). Philosophy
§7.3 wants a per-pixel dump as the default output, and REQ-SYS-007 parked it. TASK-M8-02's `prin survey` wrote "payload
summary, image, frame records": "payload summary" appeared nowhere else, and "frame records" are the adaptive loop's.
The GUI's only progress-and-cancel is the Run window's, for the live view (`principia_render_gui_spec.md` § "Run —
from the top bar"). prin-rs's `prin` writes a self-describing `.raw` (f64 fields at either precision) and PNGs with no
settings (`docs/reference/prin-rs`, reference only).

## 2. The gap

There was no in-app path to a result, no result format, and no mark that tells a result from exploration. "Payload
summary" was undefined, and philosophy §2 contradicted R-265.

## 3. The design

A **Result** is the prebake (philosophy §2) made reachable from the app: one view's region, run at full precision on a
uniform grid with refinement off, written to a folder that reopens as a view drawn from what was stored. "Result" is
its name in the GUI; the job is a **batch** (`prin batch`); `prin survey` stays the headless live path (TASK-M8-02).

### 3.1 Launch: the Result window

*Make a result…* opens the **Result** window from the view (File › Make a result…), from Measure's kept region
(`principia_render_gui_spec.md` § "G10. Measure — a tool on the figure", Keep region) or from the Inspector (§G8),
prefilled from the view it was opened over. The window lists every field that differs from that view:

- **Region:** the plane, frame, tilt and links by value (the view's `SimConfig`); a uniform `nx × ny` grid;
  refinement off.
- **Precision:** **f64** (the default; the shared kernel on CPU threads) or **f32** (the survey kernel on the GPU).
  Nothing else: the shared kernel is f32 and f64 only (R-265). Both use the view's integrator occupant (stepper ×
  regularisation, R-560, R-561), not the Inspector's adaptive RK45.
- **Horizon and thresholds:** horizon `T`, the ensemble `E` (so `E + 1` copies) and `r_coll`, all `SimConfig` fields.
- **Stored times:** the horizon by default, plus any playheads the user adds (§5, open question 2).
- **Copies:** one copy per pixel, the nominal copy with the per-pixel reductions, by default; or all `E + 1` copies, an
  option (§5, open question 3).
- **Estimate:** a cost and size estimate from a **timed 64-pixel pilot** run of the job itself, never a constant; and a
  warning when `T` passes the precision's predictability horizon (≈ 23 at f32, ≈ 52 at f64; philosophy §7.1, an
  estimate, `t_max = ln(1/eps)/λ` with `λ ≈ 0.7`; the measured f32 value once REQ-VAL-071 has run).

*Applied per R-369 (the port):* the pilot's 64 pixels are an 8 × 8 lattice spread evenly over the job's grid, so the
estimate sees the region and not one corner; the size estimate is computed from the payload's layout (§3.3) and needs
no run.

### 3.2 Where it runs: the job

One `job.json`: `SimConfig` and `RenderState` in their canonical serialisation (R-309), a `run` block (precision,
`nx × ny`, the stored times, the copies option) and the build hash. The app runs it in the background, at lower
priority than the frame loop (REQ-SCHED-043, `principia_dd_telemetry_and_tiers.md`, "Drop priority on background work"); *Copy command* gives `prin batch --job job.json`, which runs
**the same code**. Progress shows rows done, failures and time left.

**Cancel keeps what finished**, labelled "cancelled at 37 %"; *Resume* continues. Rows are pure, since a pixel's
`SimState` is a pure function of `(IC, sim key, t)` (philosophy §7.2), and **every row takes its cell width from the
global grid**, never its own extent, so a resumed run is bitwise the uninterrupted one. This borrows §7.2's argument
for resumability without its VRAM tiling, which stays parked.

**Determinism.** At f64 on CPU threads, the payload is bitwise the same however the rows are scheduled: each pixel's
arithmetic is its own, and no reduction across pixels depends on thread order. The app's job and `prin batch` write
byte-identical payloads.

### 3.3 Output: the result folder

A folder, `<name>__<build8>.principia-result/`:

- `result.json`: the job record (`job.json`'s contents), the schema version, each link's entry hash (R-521), the host
  and timings, and a summary computed from the payload, with **failed and undetermined pixels counted, never dropped**
  (philosophy §4.3; R-563: a regularised copy out of step budget is undetermined).
- `payload.prin`: the per-pixel record at each stored time, **f64 wide at both precisions** (an f32 value widened
  exactly), with the same fields at f32 and at f64. Its fields are philosophy §7.3's reduced tier, "a CONTRACT, not a
  convenience": the nominal copy's `SimState` and the per-pixel reductions across the `E + 1` copies, including fields
  no current shader reads; with the copies option, every copy's `SimState` (§7.3's per-copy tier). A pixel that failed
  or ran out of budget is present, with its reason. **The field list and the file's layout are a definition (R-72,
  REQ-SYS-090)**, written here by the task that builds the payload, from `principia_dd_simstate_payload.md` and the
  reduction fields (R-135), and approved by the physics reviewer.
- `image.png`: the view rendered from the payload, its pxpack and `tEXt` carrying the record (R-484) with a tier-1
  `run` block, read as "result, f64, 1024²" (§4).
- `log.txt`.

### 3.4 Reopening: the result view

*File › Open result…* gives a **result view**: a badge "Result · f64 · 1024² · T 50" in the figure's header strip
(R-523) and a distinct frame. It draws from the payload and **integrates nothing**. Stain edits stay live and undoable
(R-69); sim-key edits are disabled, with the reason; *Explore from here* leaves for a live view with the same
`SimConfig`. Zoom shows square pixels, one per grid cell, and the scrubber snaps to the stored times. A live view has
no such badge.

**Zoom and pan in a result view** (gui review 5627122883 of PR #283, G1; applied per R-369) are display magnification
of the stored result pixels, each cell a square: they are not chart construction (render_gui_spec §G1), edit no
`SimConfig` field and no sim key, and enter no undo history. They live in the view source, `Result { id, display_zoom,
display_pan }`, changed by a result-display request; the engine draws from it and the snapshot carries it, so the GUI
keeps no copy of the payload (gui review 5627280803 of PR #283, N2; applied per R-369). *Explore from here* builds a live view from the result's own
`SimConfig`, bitwise.

**The view source** (G2; applied per R-369): which source the figure draws from is engine state, a view-source field
`Live | Result { id, display_zoom, display_pan }` (`principia_gui_state_contract.md` §2). File › Open result… sets it to
`Result { id, .. }` and *Explore from
here* or closing the result sets it to `Live`; opening a result is not an edit and enters no undo history. The snapshot
carries the view source, and the badge and the disabled sim-key controls read it.

*Applied per R-369 (the port):* with one copy per pixel, the result view colours the nominal copy, one sample per
pixel; with all copies stored, it colours each copy and averages, as the live view does
(`principia_sampling_msaa_note.md`). Which source the engine draws from (the live march or an opened result) is engine
state, set through the contract by an open-result request, since the engine never reads `ViewUI`
(`principia_gui_state_contract.md` §2); `ViewUI` holds the Result window, and the badge reads the snapshot. *The draft
said* "`ViewUI` gains the Result window and a result mode" (§4).

### 3.5 Compare

**Where it lives** (gui review G5; applied per R-369): the header strip holds badges only (R-523) and nothing sits on
the figure (§G1), so Compare is a section of the **Result window**, which, while a result view is open, shows that
result's record. The section holds **Compare with live** (a toggle that flips the figure between the result and the
live view, its label on the toggle) and **Make the f32 twin**; once the twin has run, the per-pixel difference map
shows in the same section as an image at the grid's size, with its counts and the distribution of the differences
beside it.

A **flip** against the live view, labelled "visual only: different sampling", since the live view is adaptive. The
measured comparison is **Make the f32 twin**: the same job at f32, then a per-pixel difference map with its counts and
distribution.

### 3.6 Philosophy §2

Philosophy §2's prebake paragraph becomes the draft's sentence: "… the shared kernel at f64 on the CPU, or f32 on the
GPU for a fast uniform pass. Arbitrary precision lives only in R-33's independent reference, which checks a result
rather than making one (R-265)." The three-rungs table reads "f64 (f32, labelled)" for the prebake and "f64" for the
Inspector. Applied there by R-572.

## 4. Fit with rulings

- **R-265 holds.** The §2 edit is R-572's, since R-265 amended §7.1 and §7.7 only.
- **REQ-SYS-007 is amended** (R-572): §7.3's reduced tier is unparked as the Result's payload, and its per-copy tier
  as the copies option. Resuming by rows borrows §7.2's argument without its VRAM tiling; tiled prebake, extended
  precision, headless datasets, the regularisation matrix and the GPU-port lever stay parked.
- **R-484:** `result.json` and the image import as the identity on `SimConfig` and `RenderState`. The `run` block is
  in tier 1 of the embedding (`principia_dd_image_embedding.md` §6), and `crates/engine/src/contract/tiers.rs`
  assigns it a tier like every other field (REQ-TOOL-181).
- **TASK-M8-02:** `prin survey` stays the headless live path; `prin batch` is new; "payload summary" is deleted (R-573).
- **State contract §2:** `ViewUI`'s open windows gain Result (§3.4 for the result mode).

## 5. Decided (Malachy, 21 Oct 2026)

The human's words, verbatim from the design's "Decided" section:

> - Called **Result** ("Yeah call it result").
> - **Which times are saved: the user chooses** ("let the user choose ig"): the horizon by default, plus any playheads
>   the user adds. Open question 2 is closed.
> - **One copy per pixel or all `E+1` copies: an option** ("make that an option"), one copy by default. Open question 3
>   is closed.
> - **"Payload summary" is deleted from TASK-M8-02**: it is defined nowhere, used nowhere, and no test asks for it.

The draft's four open questions, as closed: (1) the name, **(a)** "Result" in the GUI, "batch" for the job; (2) the
stored times, **(a)** the horizon plus chosen playheads; (3) the contents, **(c)** the per-pixel reductions plus the
nominal copy, with all `E + 1` copies as an option; (4) "payload summary", **(a)** struck from TASK-M8-02.

## 6. Tests

Each test with what makes it fail (the draft's table, kept; the port adds the last three rows).

| Test | Fails when |
|---|---|
| The same f64 job run twice gives a bitwise-equal payload | thread order reaches the arithmetic |
| In-app and `prin batch` give byte-identical payloads | two code paths exist |
| Cancel, resume and an uninterrupted run are bitwise equal | a row takes its cell width from its own extent |
| The payload holds `nx·ny` pixels; an injected budget-exhausted pixel is present with its reason | failures are filtered |
| f32 and f64 payloads have the same fields and widths | the width follows the precision |
| A stain edit in a result view runs 0 integrations; a sim-key edit is refused; a live view has no badge | the result view integrates, or the badge is always drawn |
| The payload holds exactly the horizon and each chosen playhead, and with all copies `(E + 1)·nx·ny` records per time | a stored time is dropped or added, or the option is ignored |
| The pilot's estimate changes when the pilot's measured time does (a slowed pilot doubles it) | the estimate is a constant |
| A result's image imports as the identity, its `run` block in tier 1, never dropped | the `run` block is outside the tier table or below tier 1 |
