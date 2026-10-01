# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-188: Whether the link registry's chart constants that no link reads, `δ_λ` and `ε_w`, are hashed into the schema version with the links *(physics, R-340, TASK-M0-46, TASK-M2-01, REQ-GEN-031)*

- **File, section:**
  - `decisions.md` § "R-340 — The schema version hashes each link registry entry's semantic content, not its prose": the
    human's words, "the link registry's definitions determine how chart coordinates decode, so changing them changes
    what cached and saved results mean. … hash each link's semantic content, not its prose."
  - `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part 2.5)":
    "Each entry ships **forward, inverse, log-det, ε clamps, and the sampling note**". The table has one row per link
    and no chart-constant rows.
  - `plan/tasks/M2/TASK-M2-01.md`, Goal: "The chart constants μ_max = 5, q_max = 2, α_min = 0, ε_μ = ε_z = ε_q = 10⁻⁶,
    δ_λ = 10⁻¹² and ε_w = 10⁻¹⁰ are registry data".
  - `docs/design/principia_dd_decoder.md` § "3. The maths": `δ_λ` is the one mirror test's tie-break ("|λ̃_y| ≤ δ_λ → no
    mirror"), and `ε_w` the seed-selection floor ("among seeds with ‖w⁽²⁾‖²_m > ε_w").
  - `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)", "The hash": a
    register entry is hashed when it decides "what the payload's stored bits mean"; others are not.
- **What:** R-340 hashes each link entry's semantic content, and with it the parameters and clamps its forward, inverse
  and log-det read (`μ_max`, `q_max`, `α_min`, the ε clamps). `δ_λ` and `ε_w` are registry data by TASK-M2-01 but are
  read by no link: they are decode's mirror tie-break and seed floor. They change how a chart coordinate decodes, which
  is the human's reason for hashing the registry, but they are not "each link's" content, and they are not register
  entries that decide stored bits under §3.8.
- **Options seen:**
  1. **Hash them too (recommended).** Every chart constant in the registry is hashed by value, as the links' own
     parameters are: each decides how a chart coordinate decodes, the reason R-340 gives. TASK-M0-46 hashes the
     registry's constants table with its entries.
  2. **Hash only what a link reads.** `δ_λ` and `ε_w` stay out; a change to either is carried by the compatibility
     signature's "chart/decode version" (caching contract), not by the schema version.
- **Needed:** which one. TASK-M0-46 waits for it.
