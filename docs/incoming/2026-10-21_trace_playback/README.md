# Trace playback and icons: the instruction of 21 Oct 2026, as delivered

The human's ruling of 21 Oct 2026, recorded as R-569 in `decisions.md`. This folder is a record, not authority
(R-1, R-159): the ruling and the docs it amends are cited instead. Every file but this README is the human's, byte for
byte as delivered.

| Path | sha256 | What it is |
|---|---|---|
| `INSTRUCTION_trace_playback_2026-10-21.md` | `23e478217ebc4bf09d5d4aed8434b095a9a966771e776579491c7262489492ae` | The human's ruling: his words, the three rulings, the three tests and the reference, quoted verbatim in R-569 |
| `demo/trace_markers.html` | `e07af0c15d2a9101f565d1b69ef9bca4b6f54be662d0e8a0db7d9785106e1ffd` | Trace Markers, version 29: the trace demo carrying the three rulings. Its `restartOnNewRun` (called from `compute`), `preview` in `drawTrace`, and `iconBox` and `withIcon` are the transcription source for R-569 |
| `demo/trace_sound.html` | `7d568f5982f36362f4d221c1c067c8be556db0d1646d4d49d291ef2377e6a47c` | Trace Sound, version 10: the Listen demo, with the same three changes |
| `evidence/trace_demo_checks.js` | `0675a071928353ee8f986fcf14841004e1ce99ce66387ac3baa1b9aa5430c3e6` | The human's headless Playwright check of both demos, before and after (it names `.v28pre` and `.v12pre` copies, which are not in this folder) |
| `evidence/trace_demo_checks.txt` | `d4709bb853fe3772e0062585555ec3d7c33a3710ec4e47d0e044ef3c7bcd1c4d` | That check's output: the playhead restarts at 0 while playing and stays at 6 while paused, and 1728 of 1728 icon cases touched the canvas border before and 0 of 1728 after, in each demo |

The demos and the check were read as text by the port; neither was run. The copies in `docs/gui/reference/trace/`
(Trace Markers version 28, Trace Sound version 9) are unchanged; R-569 cites these.
