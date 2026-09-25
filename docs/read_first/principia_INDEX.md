# Principia — document index and reading order

**173 files in `docs/`, several supersession relationships, and a vertical slice that changed a number of
defaults.** This is the map. Anything prefixed `principia_ARCHIVE_` is kept for the record and
**should not be implemented from**.

---

## Read first

| | |
|---|---|
| **`docs/read_first/principia_00_philosophy.md`** | What the instrument is *for*: atlas → inspector → prebake, the six commitments, what is parked, what was considered and rejected. Return here when a decision feels arbitrary. |
| **`docs/read_first/principia_01_pitfalls.md`** | The failures that earned those commitments, in enough detail to be recognised again. Nine named patterns (§1–2, §4–10; §3 is standing rules), each with the measurement that produced it. |
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

**Rings**

- `docs/design/principia_systems_architecture.md` — the ladder and six rings; §8 indexes the drill-downs
- `docs/design/principia_dd_telemetry_and_tiers.md` — Observation. Frame record, deriving tiers, graceful failure
- `docs/design/principia_scratchpad_pointer_channels.md` — Observation. Trace / sound / inspector
- `docs/design/principia_dd_image_embedding.md` — Provenance. LSB payload
- `docs/design/principia_memory_tiers.md`, `docs/design/principia_quality_device_note.md` — **ruled (R-40): they key off `eps`** — the eps / frame-budget / hard-cap axes fold into `QualitySettings` and the ladder, and the widths are recomputed (D6)
- `docs/design/principia_temporal_architecture_note.md`, `docs/contracts/principia_export_animation_contract.md`

**Also in `docs/design/`**

- `docs/design/principia_core_design.md` — the seams and the CPU/GPU contract; "the thing to hand a code agent first"
- `docs/design/principia_dd_generation_root.md` — the Generation Root: layout table + link registry; the roots of the build DAG. "This document **is** the ledger"
- `docs/design/principia_debug_tooling_plan.md` — the four-surface matrix: for each observable, what the shader shows and what the test asserts
- `docs/design/principia_trajectory_viewing.md` — hover trace + click inspector; supersedes the render-contract Part 7 hover mechanics

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
| `docs/archive/HANDOFF_claude_code.md` | a duplicate of the root `HANDOFF_claude_code.md`. The root copy is the live one |
| `docs/archive/principia_spec_pending_changes.md` | the pending-changes register, closed: twelve changes; 1, 7, 8, 9, 10, 11, 12 are folded into their owning files, 3 was folded earlier, 4 is moot; 2, 5 and 6 are ruled — 2 (the Burrau leg-swap quotient) by R-27, 5 (body-index naming) by R-22, 6 (`α_min`) by R-21 |
| `docs/archive/spec_sources/` | the retired implementation spec (`principia_spec_revised`, see canonical_spec §11), the retired colour-map and COM-projection PDFs, the Burrau and render-quadtree PDFs, older IC Inspector copies, and `SECTION_MAP.md` (the ledger of the port into markdown). Record only; the markdown is the authority |
| `docs/archive/principia_archive_20260828.zip` | an earlier snapshot of the corpus, 28 Aug ("archive index: everything produced in this working session") |
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
