# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-203: current drifts on the read side: TASK-M1-01's Goal names them, its Deliverables and tests don't, and REQ-PAY-031, which it closes, lists them *(REQ-PAY-031, REQ-TOOL-024)*

- **File, section:**
  - `plan/tasks/M1/TASK-M1-01.md` § Goal: "one unified read-side `SimState` type, fixed across tiers, whose derived
    members are computed at read and never stored — `ftle` (…), `diffusion_slope` (…), `total_substeps_log2`, the
    time fractions, `orbit_count`/`retrograde`, current drifts — plus the derived validity predicates".
  - The same file, § Deliverables: "derived-accessor emission for both targets — `ftle`, `ftle_valid`,
    `diffusion_slope`, `diffusion_slope_valid`, `total_substeps_log2`, `tm_t_end_fraction`, `tm_t_dmin_fraction`,
    `orbit_count`, `retrograde`, the `sd_is_resolved_outcome/_running/_failed/_finished` predicates". No drift, and no
    acceptance test names one.
  - The same file, § Closes: "REQ-PAY-031", whose statement in `plan/requirements.yaml` reads "Derived quantities
    (`ftle`, `diffusion_slope`, `orbit_count`, `retrograde`, current drifts, reduced crossing count,
    `total_substeps_log2`, the shape point `n`, the live substep count, suspect predicates) must be computed at read,
    and the removed fields (…) must not be stored."; verify: "review checklist", detail: "the generated ledger
    contains none of the removed fields".
  - `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored": "| current drift `ΔE`
    | `H(r,p) − E_0` | cancellation-tolerant (feeds thresholds/display); threshold uses live f32 |" and "| current drift
    `ΔLz` | `L_z(r,p) − Lz_0` | ditto |".
  - `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)", the derived
    entries: "| `energy_drift` | `derived(from: [r, p, m0, m1, m2, E_0])` (R-246) | `H(r,p) − E_0` (§3.1) | f32 |
    diverging |" and "| `Lz_drift` | `derived(from: [r, p, Lz_0])` (R-246) | `L_z(r,p) − Lz_0` (§3.1's current
    drifts) |".
  - `plan/requirements.yaml`, REQ-TOOL-024 (closed by TASK-M1-12): "Derived views (orbit_count / retrograde from θ̃, the
    reduced crossing count, ftle = S_final/(step_count·dt) with the partial renorm finalised, current drift
    H(r,p) − E_0) must each be computed in the fragment and match a reference computed the same way."
  - `docs/contracts/principia_render_contract.md` § "Part 1 — The payload (render input)": "**`RenderContext`**:
    `{sample: SimState, ic: ICDescriptor, quad: RenderQuad, uv, screen_uv, time}`." The masses `m0 m1 m2` are
    `ICDescriptor`'s; R-343 binds only `SimStateFTLE` and the word buffer to the fragment.
- **What:** the Goal lists current drifts among the read-side `SimState`'s derived members, and REQ-PAY-031, which
  this task closes, lists them among the quantities computed at read; but the Deliverables and the acceptance tests
  name none, and PR #133 builds none. `energy_drift` needs the masses, which are the `ICDescriptor`'s (`ctx.ic`), not
  the stored `SimState`'s, and `H(r,p)` itself (K + V, `G = 1`) is defined in `principia_dd_decoder.md`, which this
  task's References don't cite. TASK-M1-12 computes "current drift H(r,p) − E_0" in the fragment (REQ-TOOL-024). No
  task or requirement says which of the two builds the read-side drift members; PR #133 does check that neither drift
  is stored (`derived_not_stored`), the "must not be stored" half of REQ-PAY-031.
- **Options seen:**
  1. **TASK-M1-12 builds them (recommended):** the read-side drifts are a view over `ctx.sample` and `ctx.ic`
     (REQ-TOOL-024), and TASK-M1-01 closes REQ-PAY-031 by never storing them; the Goal's "current drifts" is read as
     the not-stored list. TASK-M1-01's Goal line gains a note pointing at TASK-M1-12.
  2. TASK-M1-01 builds `energy_drift` and `Lz_drift` as read-side members now: `sample_read` and the Rust unpacks take
     the three masses as arguments (from `ctx.ic`), `H` per the decoder's `K₀ + V₀` with `G = 1`, `L_z = Σ (x p_y −
     y p_x)`, each tested against an f64 reference; the task's References gain the decoder section. `Lz_drift` alone
     needs no mass.
- **Needed:** the human's choice, since option 1 moves part of the task's Goal elsewhere (only the human defers). PR
  #133 waits on it for REQ-PAY-031; everything else in it proceeds.
