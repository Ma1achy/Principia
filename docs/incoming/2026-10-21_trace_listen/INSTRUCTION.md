Two new instructions from me, Malachy, 21 Oct 2026, in `principia_trace_listen_bundle.zip` (unzip it to
`docs/incoming/2026-10-21_trace_listen/` and commit the bundle as-is):

1. `principia_listen.md`: Listen, the trajectory heard as sound. It is new, and it is a feature of the trajectory
   viewer.
2. `principia_trace_listen_gui.md`: where the trace and Listen sit in the dev GUI's Explore mode. It amends the dev-GUI
   sketch: the menu bar, the left panel, the Trajectory panel and the bottom band.

Read both in full, with `README.md`, the demos `demo/trace_sound.html` and `demo/trace_markers.html` (open them in a
browser; turn Sound on), and the art boards in `artboards/`, before changing anything. Docs first, then plan, then
code.

Port:
- Each file's §0 is my words: quote them verbatim in the rulings, never paraphrased. Take the next free R-numbers, one
  ruling per file, with their S and G items as the applied text per R-369 (mechanical consequences and routine
  choices; I may veto any).
- `principia_listen.md` treats the demo's code as the transcription source. Where the md and the code disagree, the
  code wins and the disagreement is queued, not resolved. Transcribe the signal, the spectrum, the pitch mapping, the
  windows, the partial tracking, `INSTRUMENTS`, `makeVoice`, the events and the effects; do not re-derive them.
- The GUI file supersedes the earlier art-board pass's `Lines ▾` and `Listen ▾` menus, and the sketch's
  "listen · separations → pitch" row (queued as Q4). Record each replaced line as "stale value, replaced by R-xxx".
- File each "Queue, don't rule" item (Q1–Q8) as an RQ with its recommendation as option (a). Do not rule them
  yourself. Q1 (the audio backend) and Q3 (`RMS_FLOOR` against our integrator) need answers before Listen is built.
- Requirements and tasks:
  - Listen (S1–S12) belongs to the trajectory viewer's tasks, with the physics reviewer on S1–S3 and S7 (the signal,
    the pitch's scale invariance, the floor).
  - G1–G9 belong to the GUI. They depend on the trace markers tasks from the earlier bundles.
  - Every test in both files (S13, G10) gets its failing arm, as written.
- Put both demos under `docs/gui/reference/trace/` and the art boards under `docs/gui/reference/trace/artboards/`, as
  reference mocks like the lines canvas. They are not ported as code.

Check against what is open. These files assume:
- the earlier bundles' trace markers and follow-up (F1–F7);
- R-469's centre clamp, R-470's blank-plus-overlay and R-509's fade.

If anything here contradicts main, an open branch or a ruling — or my words contradict a ruling — tell me and queue it
rather than resolving it.

Open a PR for the docs port with the rulings, and paste the doc diffs' summary and any conflicts found in the
description. Do not touch anything outside the repo and build or scratch directories without asking.
