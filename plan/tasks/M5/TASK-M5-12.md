# TASK-M5-12 — Blast radius: what each knob invalidates, and r_coll on the sim key

- **Milestone:** M5
- **Closes:** REQ-SCHED-017, REQ-SCHED-026, REQ-SCHED-046, REQ-PERF-016, REQ-RENDER-042, REQ-EVT-022, REQ-INT-075
- **Depends on:** TASK-M5-02, TASK-M5-06, TASK-M5-08
- **Needs (earlier milestones):** REQ-SCHED-006, REQ-SCHED-007, REQ-RENDER-030, REQ-SYS-018, REQ-EVT-003, REQ-INT-029
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-9
- **Size:** ~400 lines

## Goal
Every knob invalidates exactly its caching Part 2 blast radius: render/palette nothing, colour params the bake
only, T / dt_macro / thresholds / eps floors / occupant / links / chart-decode version all sim buffers, quality tier only
its sim-key components (N, FTLE, word), E, MAX_REL_DEPTH, render_scale, lock-to-native, transport and scheduler knobs
nothing. A link or axis-warp change re-integrates; a colour compactification change only recolours. `r_coll` is a
user-exposed sim-key parameter (default 1e-3 of R) passed identically to both pipelines, and the treatment of the
`r_coll` / `N_max` coupling is written into integrator_contract Part 7 (REQ-INT-075).

## References
- `docs/contracts/principia_caching_contract.md` § "Part 2 — What invalidates what (the dependency graph)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 2.5 — Link functions & compactification (customisable)"
- `docs/design/principia_quality_device_note.md` § "Consequence: quality changes are (mostly) sim-key changes"
- `docs/contracts/principia_gui_state_contract.md` § "6. Quality settings — preset selector over one struct (see `principia_quality_device_note.md`)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/contracts/principia_caching_contract.md` § "Part 1 — Two-level keying: identity vs validity"
- `docs/contracts/principia_canonical_spec.md` § "5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*"
- `docs/contracts/principia_canonical_spec.md` § "8. Locked vocabulary"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "The two keys as ladder geometry"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `decisions.md` § "R-89 — Depth and E are not on the sim key *(closes RQ-40)*"
- `decisions.md` § "R-92 — What the sim key holds of navigation *(closes RQ-43)*"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-97 — Quad addresses live in the slice plane *(closes RQ-57)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `docs/contracts/principia_integrator_contract.md` Part 7: which of the two `r_coll`/`N_max` options holds and how it
  is exposed (REQ-INT-075), with the "Removed lines" note.
- `crates/engine/src/cache/invalidate.rs`: knob → blast-radius table driven by the key diff.
- Property test `crates/engine/tests/blast_radius.rs`: per knob, which entries survive; E lowered 3 → 1 → 3 drops and
  respawns (or reuses) copies 2..3 without re-boot.

## Acceptance tests
- `cargo test -p engine blast_radius_per_knob` — per knob, assert which cache entries survive; a tilt adds new-identity entries and leaves the old ones valid; an E or transport change invalidates nothing (REQ-SCHED-017).
- `cargo test -p engine link_vs_compactification` — change colour compactification: no integration dispatch occurs; change a link or axis warp: affected quads are invalidated (REQ-SCHED-026).
- `cargo test -p engine quality_knob_invalidation` — changing N changes the sim-key hash and evicts; changing E, MAX_REL_DEPTH or render_scale leaves the hash and cache intact (REQ-SCHED-046).
- `cargo test -p engine quality_generation_unchanged` — changing render_scale, E or MAX_REL_DEPTH leaves the cache generation unchanged; changing N invalidates it; lowering E from 3 to 1 and raising it back never invalidates the nominal entries, drops copies 2..3 then respawns them (or reuses them from cache per copy_index), with no re-boot to t = 0 (REQ-PERF-016).
- `cargo test -p engine render_scale_invalidates_nothing` — changing render_scale across 0.25–2.0 leaves the sim key, cache and sim buffers unchanged (REQ-RENDER-042).
- `cargo test -p engine r_coll_sim_key` — change r_coll -> sim-key change -> invalidate + re-boot; parity harness passes the same r_coll to both sides (REQ-EVT-022).
- Review checklist (physics) — Part 7 states which of the two options holds and how it is exposed; physics reviewer approved; the doc change is in this PR and the physics reviewer approves it before merge (REQ-INT-075).

## Notes
- Definitions (R-72) this task writes: REQ-INT-075.
- The blur-stale backdrop that masks a quality re-boot (REQ-SCHED-046) is built in TASK-M5-26; this task asserts the
  key and cache behaviour.
