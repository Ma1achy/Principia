# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-207: the L_z-suspect threshold has no pass criterion: which occupant, on which orbit, must light it and which must not *(TASK-M3-05, REQ-INT-085)*

- **File, section:**
  - `docs/design/principia_dd_integrator.md` § "3.5 Invariant monitoring (per `STEP`, post-projection)": "suspect
    predicates (READ-TIME, over the stored drift latches — no stored suspect bits, payload §5): energy-suspect on
    relative δE ; L_z-suspect on **absolute** ΔL_z". It names the predicate and its quantity, not a criterion.
  - `docs/design/principia_dd_integrator.md` § "5. Unit tests", test 2: "KDK/Yoshida energy error *oscillates* about E₀
    with no secular trend; RK4 and Euler drift secularly; **Euler's violent drift lighting up `SUSPECT_ENERGY` is the
    pass condition**". Test 7: "a close-encounter IC shows `max|ΔE| ≫ |ΔE_final|` (spike that recovered)". Both are
    about energy only; no test names L_z-suspect.
  - `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile": "On
    KDK/Yoshida, energy oscillates around `E_0` and drift measures the *physics* (close-encounter stress). On Euler/RK4
    it measures the *integrator* — Euler drifts secularly and will light up `SUSPECT_ENERGY` everywhere." Energy only.
  - `docs/contracts/principia_integrator_contract.md` § "COM projection and invariant monitoring": "accumulate the worst
    deviation so far, … and likewise $\Delta L_{z,\max}$" and "The exact accumulators and suspect predicates are in
    `principia_dd_integrator.md` §3.4–3.5". No criterion.
  - `docs/design/principia_debug_tooling_plan.md` § "D. Payload field views — `SimState` scalars (ledger §3.4)", the
    `Lz_drift` row: "**diverging** | absolute-gated suspect". It names the gate, not what it must separate.
  - `plan/requirements.yaml` REQ-INT-085 (R-71) and `plan/tasks/M3/TASK-M3-05.md` § "Acceptance tests", the
    "Proposal:" line: the thresholds are proposed "with evidence", but only the energy threshold has a pass/fail rule
    ("Euler lights SUSPECT_ENERGY and a symplectic occupant does not").
- **What:** the corpus gives the energy-suspect threshold a criterion (dd test 2: Euler lights it, the symplectic
  occupants oscillate), and says a symplectic spike on a close encounter is physics, not integrator error (integrator
  contract Part 2; dd test 7). For the L_z-suspect threshold it gives none: no text says which occupant, on which
  orbit, must light the L_z-suspect predicate, or must not. So the proposal can measure absolute ΔL_z, final and max,
  but any value it picks passes (PR #137's qa review on 55ce919, finding 1). Background, not a ruling: for pairwise
  central forces each KDK kick and drift conserves Σ rᵢ × pᵢ exactly, so KDK and the Yoshida compositions drift L_z by
  round-off only, close encounters included; an Euler step changes it by dt²·Σ vᵢ × Fᵢ, and RK4 by its truncation
  error.
- **Options seen:**
  1. **Mirror the energy rule:** on dd test 2's long bounded orbit, Euler lights the L_z-suspect predicate and no
     symplectic occupant does; RK4 and test 7 are recorded only.
  2. **Round-off bound for the symplectic occupants:** no symplectic occupant lights it on either test 2's orbit or
     test 7's close-encounter IC (their L_z drift is round-off by construction); Euler and RK4 are recorded, with no
     requirement that they light it.
  3. **Both:** 1 and 2 together, so the threshold sits above the symplectic occupants' round-off on both fixtures and
     below Euler's drift on test 2's orbit.
- **Needed:** a ruling on the L_z-suspect threshold's pass criterion, the occupants and fixtures on which it must and
  must not light. REQ-INT-085's verify detail and TASK-M3-05's "Proposal:" line give the L_z part only as far as the
  corpus goes (measured and recorded on both fixtures) until then; nothing in TASK-M1-03 waits on it.

---

## RQ-208: the drift views' ramp: the preset's `diverging` through a neutral, or `dbg_sentinel`'s viridis *(TASK-M3-05, REQ-TOOL-149)*

- **File, section:**
  - `docs/design/principia_colour_composition.md` § "6. Debug views as presets": "`f_edrift` =
    `energy_drift · diverging · symlog`", and § "1.2 Family B — field-ramp → `vec3` or `f32`": "**Ramp** (scalar →
    colour): … `diverging(c−,c0,c+)` (through a neutral)", and "**Default ramps by field role.** Signed fields
    (energy, L_z, drifts) default to diverging-through-neutral so the zero-crossing is a legible contour".
  - `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug
    view)": "Any other value … shows as its literal value on the viridis ramp at `t = 0.5 + 0.5·x/(1 + |x|)`", and
    "the view that reads the drift suspect predicates (Part 4; the energy-drift and L_z-drift views) applies the
    styling on `dbg_sentinel`'s output".
  - `decisions.md` § "R-381": "**`symlog` is the default**: the field-view preset `f_edrift` = `energy_drift ·
    diverging · symlog`", and "**The value fed to `dbg_sentinel` is the compacted value**".
  - `plan/requirements.yaml` REQ-TOOL-149 and `plan/tasks/M3/TASK-M3-05.md` § "Acceptance tests",
    `drift_suspect_styling`: a non-suspect pixel equals `dbg_sentinel`'s output on the compacted value.
- **What:** R-381 settles the compaction, not the ramp. The preset colours the compacted drift with a diverging ramp
  through a neutral; `dbg_sentinel`, whose output the drift views style (R-379), draws viridis, a sequential ramp with
  no neutral at its middle. `dbg_sentinel`'s `0.5 + 0.5·x/(1 + |x|)` does put 0 at the middle and the two signs on
  either side, so the sign stays legible, but the colours differ from the preset's, and the corpus gives the drift
  views both.
- **Options seen:**
  1. **`dbg_sentinel`'s viridis:** the drift views are `dbg_sentinel(compacted value)`, as R-379 and REQ-TOOL-149
     read; colour_composition §6's `diverging` names the scale's signed midpoint, not the colour ramp.
  2. **The preset's diverging ramp:** the drift views colour the compacted value with `diverging(c−,c0,c+)`, and the
     absence hatch and the suspect styling are applied as `dbg_sentinel` applies them; REQ-TOOL-149's "equals
     `dbg_sentinel`'s output" is restated for that ramp, and the ramp's three colours are a definition (R-72).
  3. **Both, as the palette swap:** viridis by default, the diverging ramp through the presets' palette swap
     (colour_composition §6), or the reverse.
- **Needed:** a ruling on which ramp the drift views' compacted value is drawn on. TASK-M3-05's styling definition
  (REQ-TOOL-149) waits for it; nothing in TASK-M1-03 does.
