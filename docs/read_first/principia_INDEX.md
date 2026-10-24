# Principia — document index and reading order

**183 files in `docs/`, several supersession relationships, and a vertical slice that changed a number of
defaults.** This is the map. Anything prefixed `principia_ARCHIVE_` is kept for the record and
**should not be implemented from**.

---

## Read first

| | |
|---|---|
| **`docs/read_first/principia_00_philosophy.md`** | What the instrument is *for*: atlas → inspector → prebake, the six commitments, what is parked, what was considered and rejected. Return here when a decision feels arbitrary. |
| **`docs/read_first/principia_01_pitfalls.md`** | The failures that earned those commitments, in enough detail to be recognised again. Ten named patterns (§1–2, §4–11; §3 is standing rules), each with the measurement that produced it. |
| **`docs/read_first/principia_INDEX.md`** | this file |

---

## The evidence base — where settled defaults were measured

**`prin-rs` (the vertical slice repository):** `FINDINGS.md`, `README.md`, `results/`.

The minimal text and code set the contracts transcribe from is imported at `docs/reference/prin-rs/`, pinned to commit
`8600d45` (R-159). It is reference, not authority (R-1): transcribe into the contracts and cite; never cite those files as
normative. `results/` and the images are cited by commit (`8600d45`; `70cfbc4` for the original 256² data).

> Every settled default should be traceable to where it was measured. **A default without a citation
> becomes folklore** — this project lost four days to exactly that when a re-registration finding
> written down months earlier had to be rediscovered.

Decisions that live there: **Heggie as the default regularisation**, the **predictive step limit**
(which is what removed the wedges — not the `dtau` fix), the **closure + energy escape criterion**,
and **`Policy::Tolerance`**.

---

## Contracts — the interfaces

| file | subject |
|---|---|
| `docs/contracts/principia_canonical_spec.md` | the laws, and the index to which doc owns each |
| `docs/contracts/principia_integrator_contract.md` | the occupant seam: **Part 2a** the `ADVANCE` signature (change 8, R-19); **Part 2b** regularisation as a second axis; **Part 2c** the GPU kernel — what ports, what is rewritten, and the five rules |
| `docs/contracts/principia_scheduler_contract.md` | descent, veto, decision variants |
| `docs/contracts/principia_render_contract.md` | colour is a pure function of payload |
| `docs/contracts/principia_caching_contract.md` | payload purity; tilt re-addresses, never invalidates |
| `docs/contracts/principia_parity_contract.md` | what must be bitwise across backends |
| `docs/contracts/principia_chart_decoder_contract.md` | `z → (m, r, p)`, the one seam |
| `docs/contracts/principia_gui_state_contract.md` | typed state, snapshot/emit, one-way flow |
| `docs/contracts/principia_lowering_contract.md`, `docs/contracts/principia_checkerboard_contract.md`, `docs/contracts/principia_inverse_encode_contract.md`, `docs/contracts/principia_symbolic_dynamics_contract.md` | |

---

## Design documents — the reasoning

**Physics and payload**

- `docs/design/principia_dd_decoder.md`, `docs/design/principia_dd_encode.md`, `docs/design/principia_chart_reference.md` — the charts
- `docs/design/principia_dd_simstate_payload.md` — the byte layout, closure, `footprint_undetermined`
- `docs/design/principia_dd_integrator.md`, `docs/design/principia_dd_predictability_horizon.md` — *both carry a Heggie banner*
- `docs/design/principia_dd_validation_orbits.md` — closure tests, periodic-orbit detection
- `docs/design/principia_deep_zoom.md` — quad-local coords, the linearised decoder, **switchover is not a stop**
- `docs/design/principia_coordinate_conventions_note.md` — three spaces, one flip

**Allocation**

- **`docs/design/principia_dd_refinement_policy.md`** — `Policy::Tolerance`. **The current design.**

**Meaning**

- `docs/design/principia_colour_composition.md` — the colour algebra; §1.4 the outcome palette
- `docs/design/principia_dd_colouring.md`
- `docs/design/principia_dd_chazy_final_motion.md` — Chazy's final-motion classes as a live, per-playhead field, its derived fields and its stains; a separate system from the event classes (R-442). Transcribed from the Chazy stain spec; evidence prin-rs PR #32
- `docs/design/principia_state_map.md` — the state map: each sample coloured by where its system is now on a chart, the Chart position node, its base and layers (a separate node from Shape n̂), its four projection results and its stain-editor GUI; transcribed from the prototype `docs/gui/reference/state_map/state_flow.html` (R-542)

**Rings**

- `docs/design/principia_systems_architecture.md` — the ladder and six rings; §8 indexes the drill-downs
- `docs/design/principia_dd_telemetry_and_tiers.md` — Observation. Frame record, deriving tiers, graceful failure
- `docs/design/principia_scratchpad_pointer_channels.md` — Observation. Trace / sound / inspector
- `docs/design/principia_listen.md` — Observation. Listen, the trajectory heard as sound: the signal, spectrum, pitch, windows, voices, instruments, events and effects, transcribed from the demo's code (adopted by R-540; its GUI is R-541's)
- `docs/design/principia_dd_image_embedding.md` — Provenance. LSB payload
- `docs/design/principia_memory_tiers.md`, `docs/design/principia_quality_device_note.md` — **ruled (R-40): they key off `eps`** — the eps / frame-budget / hard-cap axes fold into `QualitySettings` and the ladder, and the widths are recomputed (D6)
- `docs/design/principia_temporal_architecture_note.md`, `docs/contracts/principia_export_animation_contract.md`

**Also in `docs/design/`**

- `docs/design/principia_core_design.md` — the seams and the CPU/GPU contract; "the thing to hand a code agent first"
- `docs/design/principia_dd_generation_root.md` — the Generation Root: layout table + link registry; the roots of the build DAG. "This document **is** the ledger"
- `docs/design/principia_debug_tooling_plan.md` — the four-surface matrix: for each observable, what the shader shows and what the test asserts
- `docs/design/principia_trajectory_viewing.md` — hover trace + click inspector; supersedes the render-contract Part 7 hover mechanics
- `docs/design/principia_honesty_precision.md` — where precision runs out, per point: four runs of the clicked IC (f32, f64, each nudged by one ulp), the shape-sphere chord's two marks, "faithful until" 10⁻³ and "parted at" 0.1, and the trust line in the click inspector; nothing changes in the atlas (adopted by R-574 and R-575; RQ-555 to RQ-558 queued)
- `docs/design/principia_results.md` — Result: the full-precision batch run (f32 or f64) from a view, `prin batch`, the result folder and its payload, the result view and Compare (adopted by R-572 and R-573)
- `docs/design/principia_sequence.md` — Sequence: keyframed view animations made in the native GUI, the timeline and its lanes, easing per segment, preview, the proof strip, the planner's estimate and export, the `.principia-seq.json` file (adopted by R-576 and R-577; RQ-559 and RQ-560 queued)
- `docs/design/principia_spotlight_reel.md` — the spotlight reel: a film rendered once per release that plays inside Explore, its manifest, and the takeover at the exact frame that keeps playing live; machinery native, presentation v2 web's (adopted by R-578; RQ-561 queued)
- `docs/design/principia_lines.md` — the line system: contours, traces, axes, grids and styles over the finished figure, outside the node graph (adopted by R-476; its §11 queued as RQ-377 and ruled by R-509)
- `docs/design/principia_print_graph.md` — the Print graph: post-processing as a node graph after the stain, the figure printed in a medium, Plain the identity, never read by measurement; it replaces Display › Style (adopted by R-485 to R-492; its §12 queued as RQ-388 to RQ-391, ruled by R-524 to R-526, RQ-391 deferred by R-527; PR #268's review's RQ-478, RQ-479 and RQ-495 ruled by R-557)
- `docs/design/principia_benchmark.md` — the benchmark: telemetry §1.1's fixed suite as data-driven scenarios, one runner, Run benchmark and `prin bench`, the result file and `prin bench report` (adopted by R-493 to R-499; its §10 queued as RQ-392 to RQ-394 and ruled by R-528 to R-530)

---

## Notes — working notes

- `docs/notes/principia_sampling_msaa_note.md` — sampling, ensemble & SSAA model: colour per sample, average last
- `docs/notes/principia_gpu_determinism_note.md` — GPU determinism & backend-compilation discipline; every rule earned by a measured failure in the toolchain spike
- `docs/notes/principia_validation_ground_truth_note.md` — ground-truth physics validation: self-consistency ≠ correctness
- `docs/notes/ic_inspector_scratchpad.md` — IC Inspector design scratchpad; "Not spec yet". Its "as built" status block supersedes the stale details below it

---

## GUI

- `docs/gui/principia_render_gui_spec.md` — Dev GUI (egui / F3): the whole developer GUI (Part I) and the stain editor (Part II). **Rewritten from the dev-GUI artboards and design notes in step 6**
- `docs/gui/design/` — the twelve dev-GUI artboards and `GUI_DESIGN_NOTES.md` (the notes win over the pictures; rulings win over both)
- `docs/gui/reference/principia_dev_gui.html` — Render / Colour Dev GUI (interactive reference)
- `docs/gui/reference/principia_gui_mock.html` — GUI mock
- `docs/gui/reference/principia_render_modes.html` — Render-Mode Catalogue
- `docs/gui/reference/principia_colour_presets.html` — Colour Composition · Preset Gallery
- `docs/gui/reference/principia_colour_explorer.html` — Colour Composition Explorer
- `docs/gui/reference/ic_inspector.html` — IC Inspector: static single-sample tool; canonical copy (R-8). Prior art: the tool is absorbed into the dev GUI's Inspector window (R-65)
- `docs/gui/reference/principia_nav_cube.html` — Principia — navigation cube: reference for look and behaviour (R-451, R-452); its cream palette is not copied
- `docs/gui/reference/principia_plucker_compass.html` — Principia — orientation compass: reference for look and behaviour (R-451, R-452); its cream palette is not copied
- `docs/gui/reference/lines/` — the line system's mock-ups, `images/` (PNGs) and `canvas/` (artboards, `.dc.html`), the reference for `principia_lines.md` (R-476)
- `docs/gui/reference/trace/` — the trace and extension demo, `trace_markers.html` (one HTML file, version 28, restyled to the dev GUI, its behaviour the follow-up's version 27; it loads the dev GUI's Ubuntu and Ubuntu Mono from Google Fonts), the Listen demo, `trace_sound.html` (version 9, one HTML file loading the same fonts, the transcription source of `principia_listen.md`, R-540), their screenshots, `images/`, and the trace and Listen art boards, `artboards/` (`.dc.html`, `canvas.json`, R-541): the reference for `principia_lines.md` §8's markers and icon sets, where a trace can leave, mixed axes and the time ramp (R-538), for the view past the square with the controls shown (R-537), and for Listen and the trace's place in the dev GUI (R-540, R-541). Both demos' 3× zoom-out cap on the sphere and their `clampView` exist only because their maps are precomputed images: they are never ported, and a latent or flat chart has no limit on pan or zoom-out (R-570). *Was:* "the trace and extension demo, `trace_markers.html` (one HTML file, version 27, the follow-up's; it loads IBM Plex from Google Fonts, and its fonts and look are not copied, the dev GUI's being Ubuntu and Ubuntu Mono), and its screenshots, `images/`"
- `docs/gui/reference/principia_compass_charts.html` — Principia — compass, position & chart transform: reference for look and behaviour (R-451, R-452); its cream palette is not copied
- `docs/gui/reference/print_graph/` — **reference only, non-normative** (as `docs/reference/` is, R-159): the Print graph lab (`reference/print_graph_lab/`), the Python prototype (`reference/fx_prototype/`), the node and preset catalogue, the capture tools, the reference images (`images/`, the port's visual references, R-492) and the design boards; port the lab's nodes and take the prototype's data format, cite `principia_print_graph.md` and the rulings, never a file here (R-485; its README)
- `docs/gui/reference/state_map/` — the state map's prototype, `state_flow.html` (one HTML file, the transcription source of `principia_state_map.md`), and its art boards, `boards/` (`.dc.html`, `canvas.json`, `png/`): editor, base and layers, inspectors, ports, presets, Explore; reference mocks for the Chart position node and its place in the stain editor (R-542); the prototype is version 9 since R-543, the tiling follow-up's; its `clampView`, its navigation limit, exists only because its maps are precomputed images, and it is never ported (R-570)

---

## Experiments — briefs and results

**Briefs** (`docs/experiments/briefs/`; the `ARCHIVE_brief*` files are listed under Archived)

- `docs/experiments/briefs/principia_spike_brief.md` — toolchain spike brief (shared-source Rust engine). The spike ran and chose rust-gpu; the brief is kept as the plan of record

**Results** (`docs/experiments/results/`)

- `docs/experiments/results/principia_dd_refinement_external_report.md` — "Results: three refinement experiments" (Brief 1)
- `docs/experiments/results/principia_dd_refinement_external_report2.md` — "Brief 2: is the `max` criterion well-posed?"
- `docs/experiments/results/principia_dd_refinement_external_report3.md` — "Brief 3: validating the no-discard architecture"
- `docs/experiments/results/principia_dd_refinement_external_report4.md` — "Brief 4 — horizons"
- `docs/experiments/results/findings.md` — toolchain spike findings ("Verdict: rust-gpu"): the evidence for the substrate decision (R-9)
- `docs/experiments/results/principia_ftle_shadow_precision_experiment.md` — can the Benettin FTLE shadow be stored at f16/bf16?
- `docs/experiments/results/xp_results/` — Brief 1 raw measurements and derived tables: `exp1_raw.json`, `exp1_table.json`, `exp1_tables.md`, `exp2_raw.json`, `exp2_tables.md`, `exp2b_raw.json`, `exp3_raw.json`, `exp3_table.json`, `exp3_tables.md`, `xp1_cross.py`, `xp1_report.py`, `xp1b_supp.py`, `xp2_horizon.py`, `xp2c_t120.py`, `xp3_gate.py`, `xp_common.py`, `xp_driver.py`, `xp_probe.py`, `xp_reduce.py`
- `docs/experiments/results/xp_results2/` — Brief 2 raw JSON: `expA_domke.json`, `expB_estimator.json`, `expC_tend.json`
- `docs/experiments/results/xp_results3/` — Brief 3 raw JSON: `expA_nogate.json`, `expB_meters.json`, `expB_roundtrip.json`, `expC_scale.json`
- `docs/experiments/results/xp_results4/` — Brief 4 raw tables and JSON: `exp1_jitter.json`, `exp2_f32.json`, `exp3_escape.json`, `exp4_spacing.json`, `tables.md`
- `docs/experiments/results/refinement_probe/` — refinement criterion probe, Burrau/Pythagorean slice: can a local uncertainty exponent serve as the refinement criterion? `RESULTS.md`, `ana.py`, `run.py`, `tb.py`

---

## Archived — record only, do not implement

Archives are never cited. Where an archived brief once had standing parts, they are superseded by the consolidated
docs (`principia_deep_zoom.md` §3, the scheduler contract, the parity contract); TASK-M0-02 checks that each
standing obligation exists there and ports any that doesn't (R-112).

| file | why |
|---|---|
| `docs/archive/principia_ARCHIVE_dd_refinement_criterion_v0.md` | the α-exponent era; its metric is void |
| `docs/archive/principia_ARCHIVE_dd_tree_dump_analysis_v0.md` | numbers computed on the contaminated field |
| `docs/experiments/briefs/principia_ARCHIVE_brief_structure_criterion.md` | superseded: §4–4.6 (the slippy map) by the consolidated docs (deep_zoom §3, scheduler, parity); the rest does not stand (R-112) |
| `docs/experiments/briefs/principia_ARCHIVE_brief_signal_audit.md` | premise void: scored in render space |
| `docs/experiments/briefs/principia_ARCHIVE_brief_criterion_improvement.md` | superseded |
| `docs/experiments/briefs/principia_ARCHIVE_brief_scheduler_build.md` | built; split/stop rules replaced |
| `docs/experiments/briefs/principia_ARCHIVE_brief_refinement_experiments.md` | run; most conclusions overturned |
| `docs/experiments/briefs/principia_ARCHIVE_brief_kernel_build.md` | built; **defaults changed** — §5 gates superseded by the consolidated docs (deep_zoom §3, scheduler, parity) (R-112) |
| `docs/experiments/briefs/principia_ARCHIVE_brief_vertical_slice.md` | delivered |
| `docs/experiments/briefs/principia_ARCHIVE_brief2_dom_ke.md` | `dom_KE` rejected |
| `docs/experiments/briefs/principia_ARCHIVE_brief3_no_discard.md` | validated, and since extended |
| `docs/archive/principia_parity_testing_note.md` | **SUPERSEDED**: "became `principia_parity_contract.md`; read that instead" |
| `docs/archive/principia_validation_scratchpad.md` | **SUPERSEDED / ABSORBED**: the current authorities are `validation_ground_truth_note` and `parity_contract` |
| `docs/archive/principia_dd_generation_root.md.bak` | an older backup of the Generation Root drill-down (it still targets "TypeScript constants"). The live file is in `docs/design/` |
| `docs/archive/HANDOFF_claude_code.md` | an earlier copy of the untangling handoff; the final one is `docs/archive/untangling/HANDOFF_claude_code.md` |
| `docs/archive/untangling/` | the finished untangling (steps 1–8): `HANDOFF_claude_code.md` (the handoff), `principia_AUDIT_2026-09-24.md` (the audit), `DECISIONS_TO_MAKE.md` (the step-5 decision sheet, ruled by R-21 to R-59), `MOVES.md` (the file-move ledger), `REVIEW_cold_read.md` (the cold-read review, ruled by R-168 to R-186). Record only |
| `docs/archive/principia_spec_pending_changes.md` | the pending-changes register, closed: twelve changes; 1, 7, 8, 9, 10, 11, 12 are folded into their owning files, 3 was folded earlier, 4 is moot; 2, 5 and 6 are ruled — 2 (the Burrau leg-swap quotient) by R-27, 5 (body-index naming) by R-22, 6 (`α_min`) by R-21 |
| `docs/archive/spec_sources/` | the retired implementation spec (`principia_spec_revised`, see canonical_spec §11), the retired colour-map and COM-projection PDFs, the Burrau and render-quadtree PDFs, older IC Inspector copies, and `SECTION_MAP.md` (the ledger of the port into markdown). Record only; the markdown is the authority |
| `docs/archive/principia_archive_20260828.zip` | an earlier snapshot of the corpus, 28 Aug ("archive index: everything produced in this working session") |
| `docs/incoming/2026-10-20_extension_trace/` | the human's bundle of 20 Oct 2026 as delivered (`principia_extension_interaction.md`, `principia_trace_markers.md`, the demo, its images and the prin-rs examples): a record, ported by R-537 and R-538, whose rulings and amended docs are cited instead; its prin-rs examples and `encode_check.py` are reference only, not ported |
| `docs/incoming/2026-10-20_trace_followup/` | the human's follow-up of 20 Oct 2026 to that bundle as delivered (`principia_trace_followup.md`, the demo version 27, its images and a prin-rs example): a record, ported into R-538 (TF1 to TF7), whose ruling and amended docs are cited instead; its demo replaces the bundle's copy in `docs/gui/reference/trace/`; its prin-rs example is reference only, not ported |
| `docs/incoming/2026-10-21_trace_listen/` | the human's bundle of 21 Oct 2026 as delivered (`principia_listen.md`, `principia_trace_listen_gui.md`, the two demos, the art boards and the images): a record, ported by R-540 and R-541, whose rulings and amended docs are cited instead; its demos and art boards are copied to `docs/gui/reference/trace/` as reference mocks, not ported as code |
| `docs/incoming/2026-10-21_state_map/` | the human's bundle of 21 Oct 2026 as delivered (`principia_state_map.md`, `requirements_patch.yaml`, the prototype `demo/state_flow.html`, the art boards, the images and the measurement scripts `tests/*.js`): a record, ported by R-542, whose ruling and amended docs are cited instead; its prototype and art boards are copied to `docs/gui/reference/state_map/` as reference mocks, not ported as code; its measurement scripts are reference only, not ported |
| `docs/incoming/2026-10-21_state_map_tiling/` | the human's follow-up of 21 Oct 2026 to the state map bundle as delivered (`INSTRUCTION.md`, the human's ruling, the prototype `demo/state_flow.html` version 9, `checks/torus_gam.js` and an image), with R-543's own check `checks/r543_tiling_check.py` beside it: a record, ported by R-543, whose ruling and amended docs are cited instead; its prototype replaces version 8 in `docs/gui/reference/state_map/`, a reference mock, not ported as code; `torus_gam.js` and the port's check are reference only |
| `docs/incoming/2026-10-21_trace_playback/` | the human's instruction of 21 Oct 2026 on trace playback and icons as delivered (`INSTRUCTION_trace_playback_2026-10-21.md`, the demos `demo/trace_markers.html` version 29 and `demo/trace_sound.html` version 10, and `evidence/trace_demo_checks.js` with its output): a record, ported by R-569, whose ruling and amended docs are cited instead; the demos are R-569's transcription source (`restartOnNewRun`, `preview`, `iconBox`, `withIcon`), not ported as code, and the copies in `docs/gui/reference/trace/` are unchanged; the check is reference only |
| `docs/incoming/2026-10-21_gap_sweep_results_honesty/` | the gap sweep's PR 2a as delivered (`designs/results.md`, `designs/honesty_precision.md`, the prin-rs `parting_probe` and its output) and its README; the bundle's `INSTRUCTION.md` and decisions are in `docs/incoming/2026-10-21_gap_sweep_integrator/`: a record, ported by R-572 to R-575, whose rulings and the two design docs are cited instead; the probe is reference only, not built |
| `docs/incoming/2026-10-21_gap_sweep_keyframes_reel/` | the gap sweep's PR 2b as delivered (`designs/keyframes_reel.md`, `designs/reel_click.md`) and its README; the bundle's `INSTRUCTION.md` and decisions are in `docs/incoming/2026-10-21_gap_sweep_integrator/`: a record, ported by R-576 to R-578, whose rulings and the two design docs are cited instead |
| `docs/incoming/2026-10-21_browser/` | the human's browser ruling of 21 Oct 2026 as delivered (`INSTRUCTION_browser.md`) and its README: a record, ported by R-571, whose ruling and amended docs are cited instead |
| `docs/archive/principia_design_corpus.zip` | an earlier copy of the corpus in a numbered-folder layout (`arch/00_read_first` … `08_scratchpads`) |

---

## Known open items

> **Resolved since the vertical slice:** the camera now reaches the scheduler. Display adequacy
> landed (`principia_dd_refinement_policy.md` §0.1) — a quad may stop only if its texel is at or below one screen pixel,
> which subsumed the missing `target_depth` driver. Mean texel size is flat within ±5% across the last
> two octaves in the converged arm (was a 4.63× climb, with `max_depth` going *backwards* as zoom
> increased). `docs/design/principia_dd_refinement_policy.md` §0.1.


- **Two `alpha_area` defects** — cannot distinguish an empty mask from a full one (both `0.0000`); negative exponents give `d > 2`. Ruled (R-42): tell empty from full by `n_unresolved`, and refuse the floor on a negative exponent; `alpha_lo` stays 0.005. The fix lands with refine. `docs/design/principia_dd_refinement_policy.md` §2.2
- **A cheap `sea_fraction` estimator** — the named next step for regime detection
- **Tier tables** — shape settled at v0.5 (three axes: `eps` / frame budget / hard cap, binding one reported; `docs/design/principia_dd_telemetry_and_tiers.md` §3.5). **The numbers are guesses and are meant to be** — calibrate from device telemetry. `memory_tiers.md` now carries a banner pointing at the three-axis model
- **`Decision::Undetermined` is a second budget line** the architecture has no place for. Ruled (R-47): it is reported, terminal and flagged, until a chart shows it non-zero
- **`N = 16` may be forced down to 8 on WebGPU** — a quad would want 256 threads, exactly the invocation ceiling. Ruled (R-43): measured, not assumed, and the thread-count inconsistencies are fixed now (`docs/design/principia_systems_architecture.md` §5.5)
- **Frontier scoping** — the frontier is unbounded (29 → 585 quads while in-view work stays ~130), so the camera is a tie-break rather than a filter. Ruled (R-45): the margin is *derived* from the refill rate, and a widened margin is the baseline to beat; a naive cull evicts faster than the descent refills and stalls
- **Relevance arithmetic in global UV** — latent defect past ~depth 40 (coordinate magnitude, not depth: `@origin` reads exactly `0.000e0`). Ruled (R-46): relevance is computed relative to the camera or quad centre, with deep_zoom §1's centre-plus-half-width pattern, before any deep-zoom demo
- **FMA contraction control per backend** — unchecked, and it can move a branch input (`docs/contracts/principia_integrator_contract.md` Part 2c)
