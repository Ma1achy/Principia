Two new instructions from me, Malachy, 20 Oct 2026, in `principia_extension_trace_bundle.zip` (unzip it to
`docs/incoming/2026-10-20_extension_trace/` and commit the bundle as-is):

1. `principia_extension_interaction.md`: the chart's extension past `[0,1]²` is reached by pan and zoom with the GUI
   shown, not only under F3. It must render with no gaps, every interaction must work there, and the quadtree,
   refinement and deep zoom must work in extended and wrapped regions.
2. `principia_trace_markers.md`: the trace's markers and icon sets, an update to `principia_lines.md` §8.

Read both in full, with `README.md` and the demo `demo/trace_markers.html` (open it in a browser), before changing
anything. Docs first, then plan, then code.

Port:
- Each file's §0 is my words: quote them verbatim in the rulings, never paraphrased. Take the next free R-numbers,
  one ruling per file, with their EX and TM items as the applied text per R-369 (mechanical consequences and routine
  choices; I may veto any).
- Apply every row of each file's "What this changes" table to the docs it names, recording each replaced line as
  "stale value, replaced by R-xxx". This includes the decoder contract Parts 3 and 4, the coordinate note, deep_zoom's
  precision split, caching contract Part 4, `principia_lines.md` §8 and the render GUI spec.
- File each "Queue, don't rule" item as an RQ with its recommendation as option (a). Do not rule them yourself.
- Requirements and tasks:
  - EX7 (canonical addressing, roots tiling the plane, neighbours across seams, scheduling by the union of images)
    and EX8 (deep zoom in repeats and far from the root) belong to the kernel and the scheduler, with the physics
    reviewer.
  - EX1–EX6 and EX9 belong to the GUI.
  - The trace markers belong to the line system's tasks (TASK-M8-50/51/54 or new ones).
  - Every test in both files gets its failing arm, as written.
- Put the demo under `docs/gui/reference/trace/` as a reference mock, like the lines canvas. The prin-rs examples
  are reference only and are not ported.

Check against what is open: `rulings/R-469` (R-469 to R-477, R-506 to R-511) is not merged. These files assume
R-469's centre clamp, R-470's blank-plus-overlay, R-472's F3 modes and R-509's trace fade. If anything in these files
contradicts main or that branch, or my words contradict a ruling, tell me and queue it rather than resolving it.

Open a PR for the docs port with the rulings, and paste the doc diffs' summary and any conflicts found in the
description. Do not touch anything outside the repo and build or scratch directories without asking.
