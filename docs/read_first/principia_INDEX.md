# Principia — document index and reading order

**62 files, several supersession relationships, and a vertical slice that changed a number of
defaults.** This is the map. Anything prefixed `principia_ARCHIVE_` is kept for the record and
**should not be implemented from**.

---

## Read first

| | |
|---|---|
| **`docs/read_first/principia_00_philosophy.md`** | What the instrument is *for*: atlas → inspector → prebake, the six commitments, what is parked, what was considered and rejected. Return here when a decision feels arbitrary. |
| **`docs/read_first/principia_01_pitfalls.md`** | The failures that earned those commitments, in enough detail to be recognised again. Eight named patterns, each with the measurement that produced it. |
| **`docs/read_first/principia_INDEX.md`** | this file |

---

## The evidence base — where settled defaults were measured

**`prin-rs` (the vertical slice repository):** `FINDINGS.md`, `README.md`, `results/`.

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
| `docs/contracts/principia_integrator_contract.md` | the `STEP` occupant slot; **Part 2a** the `advance` signature; **Part 2b** regularisation as a second axis; **Part 2c** the GPU kernel — what ports, what is rewritten, and the five rules |
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
- `docs/design/principia_memory_tiers.md`, `docs/design/principia_quality_device_note.md` — **pending redefinition to key off `eps`**
- `docs/design/principia_temporal_architecture_note.md`, `docs/contracts/principia_export_animation_contract.md`

**Register**

- `docs/principia_spec_pending_changes.md` — twelve changes; 7, 8, 10, 11, 12 have **landed**

---

## Archived — record only, do not implement

| file | why |
|---|---|
| `docs/archive/principia_ARCHIVE_dd_refinement_criterion_v0.md` | the α-exponent era; its metric is void |
| `docs/archive/principia_ARCHIVE_dd_tree_dump_analysis_v0.md` | numbers computed on the contaminated field |
| `docs/experiments/briefs/principia_ARCHIVE_brief_structure_criterion.md` | **§4–4.6, the slippy map, still stands** — the rest does not |
| `docs/experiments/briefs/principia_ARCHIVE_brief_signal_audit.md` | premise void: scored in render space |
| `docs/experiments/briefs/principia_ARCHIVE_brief_criterion_improvement.md` | superseded |
| `docs/experiments/briefs/principia_ARCHIVE_brief_scheduler_build.md` | built; split/stop rules replaced |
| `docs/experiments/briefs/principia_ARCHIVE_brief_refinement_experiments.md` | run; most conclusions overturned |
| `docs/experiments/briefs/principia_ARCHIVE_brief_kernel_build.md` | built; **defaults changed** — §5 gates still stand |
| `docs/experiments/briefs/principia_ARCHIVE_brief_vertical_slice.md` | delivered |
| `docs/experiments/briefs/principia_ARCHIVE_brief2_dom_ke.md` | `dom_KE` rejected |
| `docs/experiments/briefs/principia_ARCHIVE_brief3_no_discard.md` | validated, and since extended |

---

## Known open items

> **Resolved since the vertical slice:** the camera now reaches the scheduler. Display adequacy
> landed at prin-rs `52caf14` — a quad may stop only if its texel is at or below one screen pixel,
> which subsumed the missing `target_depth` driver. Mean texel size is flat within ±5% across the last
> two octaves in the converged arm (was a 4.63× climb, with `max_depth` going *backwards* as zoom
> increased). `docs/design/principia_dd_refinement_policy.md` §0.1.


- **Two `alpha_area` defects** — cannot distinguish an empty mask from a full one (both `0.0000`); negative exponents give `d > 2`. `docs/design/principia_dd_refinement_policy.md` §2.2
- **A cheap `sea_fraction` estimator** — the named next step for regime detection
- **Tier tables** — shape settled at v0.5 (three axes: `eps` / frame budget / hard cap, binding one reported; `docs/design/principia_dd_telemetry_and_tiers.md` §3.5). **The numbers are guesses and are meant to be** — calibrate from device telemetry. `memory_tiers.md` now carries a banner pointing at the three-axis model
- **`Decision::Undetermined` is a second budget line** the architecture has no place for
- **`N = 16` may be forced down to 8 on WebGPU** — a quad would want 256 threads, exactly the invocation ceiling. Measure, do not assume (`docs/design/principia_systems_architecture.md` §5.5)
- **Frontier scoping** — the frontier is unbounded (29 → 585 quads while in-view work stays ~130), so the camera is a tie-break rather than a filter. Held pending a margin *derived* from the refill rate; a naive cull evicts faster than the descent refills and stalls
- **Relevance arithmetic in global UV** — latent defect past ~depth 40 (coordinate magnitude, not depth: `@origin` reads exactly `0.000e0`). Take before any deep-zoom demo
- **FMA contraction control per backend** — unchecked, and it can move a branch input (`docs/contracts/principia_integrator_contract.md` Part 2c)
