# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-200: a link registry entry's log-det: the log of the Jacobian factor or the factor, and which density for the simplex link *(REQ-GEN-018, REQ-CHART-032, REQ-CHART-034)*

- **File, section:**
  - `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part
    2.5)", the table's column header: "| Constraint | Link (forward) | Inverse | log-det | Sampling note |".
  - The same table, its log-det cells: simplex "softmax Jacobian × sech² factors"; bounded "`(b−a)·σ'`"; bounded alt
    "`c·sech²`"; positive "`σ(x)` / `eˣ`"; symmetric "`c·sech²`"; unbounded "1". Each is the Jacobian factor, not its
    log (the identity's log-det would be 0, and its cell reads 1).
  - The same document, § "2. Consolidated contract": "carries its log-det Jacobian plus an over/under-sampling note".
  - `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model": "each carrying its log-det
    Jacobian; quantitative claims either correct by the Jacobian or are barred (measure honesty)".
  - `plan/requirements.yaml`, REQ-GEN-018, statement: "|det J_D| must match the analytic link derivatives wherever
    closed forms exist."; its verify detail: "compare numeric |det J_D| against the sum of registry log-det Jacobians
    at sampled points, per link".
  - `plan/requirements.yaml`, REQ-CHART-032: "Every link-registry entry must ship forward, inverse, log-det Jacobian,
    ε clamps and a sampling note"; REQ-CHART-034: "analytic log-det matching a numeric Jacobian".
  - `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)":
    "`Y ≅ Y_mass × Y_cfg × Y_mom`
    (2×2×4=8); block ordering fixed `z[0:2]=config, z[2:6]=momentum, z[6:8]=mass`", so the simplex link maps 2
    controls to the 3 masses of Δ².
- **What:** two things the corpus does not settle, both raised by the physics review of PR #119 (TASK-M0-46):
  1. The column header, §2, canonical_spec, REQ-CHART-032 and REQ-GEN-018's "sum of registry log-det Jacobians" read
     the entry as a log (logs sum where factors multiply). The table's cells hold the factor itself. A registry entry
     written from the cells hashes and evaluates a different function from one written from the header.
  2. For the simplex link, the forward's Jacobian is 3×2 (2 controls, 3 masses), so `det J` is not defined. Which
     density the entry carries is not stated: the 2-D area element of the simplex in ℝ³, `√det(JᵀJ)`, or the 2×2
     Jacobian onto two of the masses (the third being `1 − m_0 − m_1`), which differ by the constant factor `√3`, or
     another.
  TASK-M0-46's definition (§3.9 "The canonical form of a link's functions") was made neutral: "the log-det is one
  tree, the entry's log-det, over the control's components", so it holds whichever is ruled.
- **Options seen:**
  1. **The log of the factor (recommended for question 1).** The name, §2, canonical_spec, REQ-CHART-032,
     REQ-CHART-034 and REQ-GEN-018's sum all read it as a log; only the cells differ. TASK-M2-01 rewrites the cells as
     logs (e.g. bounded `log((b−a)·σ')`, unbounded `0`).
  2. The factor itself: the column is renamed and REQ-GEN-018's detail compares against the product of the factors.
  3. For question 2: the simplex's area element `√det(JᵀJ)`; or the 2×2 Jacobian onto two named masses (which two);
     or another density. The corpus does not lean between them, so there is no recommendation.
- **Needed:** a ruling on each question. TASK-M2-01 (the registry's entries) needs it, and REQ-GEN-018's comparison
  depends on it. TASK-M0-46 does not wait: its canonical form hashes whichever tree the entry carries.
