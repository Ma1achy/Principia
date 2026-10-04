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
