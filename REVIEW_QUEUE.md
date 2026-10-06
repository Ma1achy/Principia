# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

*Found applying R-389 (TASK-M1-11). Nothing is chosen.*

## RQ-225: an IC that starts inside θ̃'s pole radius has no stored longitude for its first exit *(physics, R-389, TASK-M1-11, REQ-INT-001)*

- **File, section:**
  - `decisions.md` § "R-389", the human's words: "Hold below a radius, with a frozen reference: on entering
    √(n_u²+n_v²) < r_pole, store the last longitude; add no delta while inside; on exit, add wrap(exit longitude −
    stored longitude) into (−π, π]."
  - `decisions.md` § "R-389", Q1's answer: "θ̃ counts net turning from the IC: orbit_count is completed revolutions,
    retrograde is net clockwise motion, independent of starting longitude."
  - `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep,
    ratified)", the paragraph R-389 adds: "An IC that starts inside the radius has no stored longitude; that case is
    open (RQ-225)."
- **Silence:** the reference is stored "on entering" the radius. An IC with `√(n_u² + n_v²) < r_pole` at `t = 0`
  never enters it, so on its first exit there is no stored longitude to subtract. Whether that exit adds nothing, or a
  difference from `n(0)`'s own longitude, changes θ̃ by up to π, and with it `retrograde` and, near a whole turn,
  `orbit_count`.
- **Options seen:**
  1. **The first exit adds nothing** (recommended): θ̃ counts turning from the first longitude the hold trusts, the
     exit longitude. It keeps the hold's premise that a longitude inside the radius is not trusted.
  2. **The stored longitude is `n(0)`'s**, `atan2(n_v, n_u)` at the IC (taken as 0 when `n_u = n_v = 0` exactly):
     the IC counts as the entry point, so the first exit adds `wrap(exit − longitude(n(0)))`, reading Q1's "net
     turning from the IC" literally.
- **Needed:** the human's choice: it changes `retrograde` and `orbit_count` for such ICs (physics, R-369). REQ-INT-001
  carries `rq: RQ-225`. Only this case waits: TASK-M1-11 builds and tests every other part of R-389, and its
  `theta_unwrap` adds the start-inside case once this is ruled.
