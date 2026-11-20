# Extension interaction and trace markers: bundle

Two instructions from the human, 20 Oct 2026, and the demo they were designed in.

| File | What it is |
|---|---|
| `INSTRUCTION.md` | What to do with this bundle (the same text as the inline instruction) |
| `principia_extension_interaction.md` | Past the square with the controls shown: navigation, interaction, rendering, the quadtree and deep zoom in extended and wrapped regions. EX1–EX9, tests T1–T13, queue Q1–Q6 |
| `principia_trace_markers.md` | The trace's markers and icon sets, syzygies, off-screen pinning, repeats, not-a-system handling. An update to `principia_lines.md` §8. Tests T1–T12, queue Q1–Q4 |
| `demo/trace_markers.html` | The working demo, self-contained (open it in a browser). Hover traces, tap pins; drag pans, wheel or pinch zooms, Home resets |
| `images/` | Screenshots: `nav_B*` the sphere's repeats, panned and zoomed with the controls shown; `nav_A`/`A_wide` the latent chart's extension; `nav_S`/`S_fit` the mass simplex, blank past its edge, with the ✕; `B_new`/`C_new` the sphere and its hemisphere toggle; `D_new_ext` the momenta chart |
| `reference/prin_rs_examples/` | The prin-rs examples that made the demo's backgrounds (scratch, uncommitted in prin-rs; reference only, not for porting) |
| `reference/encode_check.py` | The Python encode and integrator behind §9's measured findings |

The demo is not the app's code. `principia_trace_markers.md` §10 and `principia_extension_interaction.md` §5 list
where it differs.
