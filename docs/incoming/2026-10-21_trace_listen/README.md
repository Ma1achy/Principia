# Trace and Listen bundle — 21 Oct 2026

Follows `principia_extension_trace_bundle.zip` and `principia_trace_followup.zip`, which are already with you.

| Path | What it is |
|---|---|
| `principia_listen.md` | **New.** The sound system: signal, spectrum, pitch, windows, voices, loudness, instruments, events, effects, display, tests, queue (S1–S14). |
| `principia_trace_listen_gui.md` | **New.** Where the trace and Listen sit in the dev GUI's Explore mode: menus, Trajectory panel, `Trace…` and `Listen…` windows, bottom band, legend, the trace's look (G1–G11). |
| `demo/trace_sound.html` | Reference implementation of Listen (version 9). Open it in a browser; hover the map, press Play, turn Sound on. The code is the transcription source for `principia_listen.md`. |
| `demo/trace_markers.html` | The trace demo (version 28), restyled to the dev GUI. Behaviour unchanged from the follow-up's version 27. |
| `artboards/*.dc.html`, `artboards/canvas.json` | The art-board canvas "Principia dev GUI — trace & listen": seven boards, one file each. Plain HTML with inline styles; read them as markup. They need the canvas runtime to render, so the live canvas is the way to see them (link from me). |
| `images/` | Screenshots of both demos, and the spectrum and spectrogram at full size. |

Art boards:

| File | Board |
|---|---|
| `Main.dc.html` | Explore, playing, sound on: the trace in the view, Overlays ▾ open, Trajectory panel, bottom band |
| `TrajectoryPanel.dc.html` | The panel in three states: at rest, Trace expanded, Listen expanded |
| `TimeDock.dc.html` | The bottom band (Compass · Time · Legend): sound off, sound on, strip expanded |
| `IconSets.dc.html` | Trace icon key and the four icon sets |
| `TraceWindow.dc.html` | `Trace…`: every trace setting |
| `ListenWindow.dc.html` | `Listen…`: every sound setting, instrument menu open |
| `Edges.dc.html` | A trace off-screen, in wrapped copies, leaving the chart, at a forbidden region |

Numbers in the art boards (legend shares, readouts) are illustrative; the measured ones are in the md files, with what
they were measured on.
