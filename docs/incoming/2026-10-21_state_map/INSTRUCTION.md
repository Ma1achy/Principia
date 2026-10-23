A new instruction from me, Malachy, 21 Oct 2026, in `principia_state_map_bundle.zip` (unzip it to
`docs/incoming/2026-10-21_state_map/` and commit the bundle as-is):

`principia_state_map.md`: the state map. A new stain node, **Chart position**, gives each sample its own system's
position on a chosen chart at the playhead (or t = 0, or a fixed t). It works on every chart, the shape sphere included,
and is coloured by a base (Gradient, Site blend, Palette, Picture) with layers on top (grid lines, checkerboard, tile or
cell edges, stripes, rings, site dots) — the shape sphere's colour composition, on the chart's point (SM11). Shape n̂
is a different node and is unchanged.

Read it in full, with `README.md`, the prototype `demo/state_flow.html` (open it in a browser and play it) and the art
boards in `boards/`, before changing anything. Docs first, then plan, then code.

Port:
- §0 is my words: quote them verbatim in the ruling, never paraphrased. Take the next free R-number, one ruling, with
  the SM items as the applied text per R-369 (mechanical consequences and routine choices; I may veto any).
- Port `principia_state_map.md` to `docs/design/principia_state_map.md`, headings unchanged, and add it to
  `docs/read_first/principia_INDEX.md`.
- The prototype's code is the transcription source. Where the md and the code disagree, the code wins and the
  disagreement is queued, not resolved.
- **SM2 is a hard constraint: no change to `SimState`, the payload, the `ICDescriptor` or any layout.** The field is
  derived at read in the fragment stage from data it already has (`r, p` at the playhead, the masses, the `state`
  lane, the WGSL encode). If anything turns out to need a stored field, stop and queue it; do not add one.
- **Shape n̂ is settled in §0:** a separate node, kept exactly as it is, nothing migrated. **The node name is settled in
  §0:** Chart position. **The exact last position is settled in §0:** not stored; the edge clamp (SM5a) is the
  behaviour.
- Apply SM14's "What this changes" rows to the docs they name, recording each replaced line as "stale value,
  replaced by R-xxx".
- File each "Queue, don't rule" item (Q1–Q9) as an RQ with its recommendation as option (a). Do not rule them
  yourself. Q4 concerns the existing vMF shape colouring: record it, change nothing in it.
- Requirements: `requirements_patch.yaml` is a draft in `plan/tools/reqio.py` patch format. Replace `R-NEW` with the
  ruling's number and each `MILESTONE` with the right one from `plan/MILESTONES.md`, check every `source` heading
  against the ported doc (`plan/tools/sections.py`), then apply it with `reqio.py` and run `python3 plan/check_plan.py`.
  Physics reviewer on the Chart position, projection and four-results requirements (SM1, SM4, SM5); the GUI ones go
  with `boards/`. Every test in SM12 keeps its failing arm, as written — the copies test is the bitwise IC identity, not
  a colour comparison at t = 20, which cannot fire (measured).
- Tasks: one for the docs port (this PR), then tasks for the colour pipeline (SM1–SM7, SM11) and the GUI (SM8–SM10,
  SM15), cut from the requirements.
- Put the prototype and `boards/` under `docs/gui/reference/state_map/` as reference mocks, like the lines canvas.
  They are not ported as code; the scripts in `tests/` are reference only.

Check against what is open. This assumes:
- the extension rules (R-407, R-412, R-469, R-470) and EX7's canonical addressing from the extension bundle;
- the trace follow-up's F1 (where a projection has no value);
- L-ownership in the render contract.

If anything here contradicts main, an open branch or a ruling — or my words contradict a ruling — tell me and queue it
rather than resolving it.

Open a PR for the docs port with the ruling, and paste the doc diffs' summary and any conflicts found in the
description. Do not touch anything outside the repo and build or scratch directories without asking.
