# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

*Found applying R-400. Nothing is chosen; it is a look question (R-390) on two fields, for the human, with TASK-M1-17's
other look values at the M1 gate.*

## RQ-261: `K_0` and `V_0` carry the `diverging` scale, but `K_0` can't be negative and `V_0` can't be positive; R-400 derives the ramp from the declared range *(look, R-400, R-390, RQ-231, REQ-RENDER-084, TASK-M1-17, M1 gate)*

- **File, section:**
  - `decisions.md` § "R-400 — A numeric field view whose declared ledger range spans zero is on a diverging ramp
    centred at zero; fields that can't be negative keep viridis *(closes RQ-260)*", the human's words: "a diverging
    ramp, centred at zero, for every field whose declared ledger range spans zero; derived from the range, no new
    ledger mark. Fields that can't be negative keep viridis."
  - `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)": "`K_0`, `V_0` (diverging)", with
    no range. `crates/ledger/src/payload.rs`:174–175 gives both `Scale::Diverging` over the unbounded range, per its
    line 7: "Where §3 gives a field no scale or range, the entry is `lin` over (−∞, ∞)".
  - `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time, pre-integration)": "E₀ = K₀ + V₀ ;   virial_ratio = 2K₀ / |V₀|". `K₀` is the kinetic energy,
    never negative; `V₀` is the gravitational potential energy, never positive.
  - `docs/archive/review_queue/M0.md` § RQ-231, decided per R-369, item 1: "`diverging` uses a symmetric range".
- **The conflict:** `K_0`'s declared range is unbounded, so R-400's range rule puts it on the diverging ramp, as its
  `diverging` scale does, while R-400's last sentence keeps a field that can't be negative on viridis. Declaring
  `K_0`'s range `[0, ∞)` (as R-400's port does for fields the corpus gives a non-negative domain) satisfies the last
  sentence but leaves a `diverging`-scale field whose range doesn't span zero, under RQ-231 item 1's symmetric range,
  half of it unused. `V_0` is the mirror case: it can't be positive, and R-400 doesn't say which ramp a field that can't
  be positive takes; under its range rule, with its range declared `(−∞, 0]`, it would be on viridis.
- **Options seen:**
  1. `K_0` and `V_0` are declared `[0, ∞)` and `(−∞, 0]` and their scale becomes `lin` (§3.6 changes): both on
     viridis.
  2. Both keep `diverging` and their unbounded ranges: both on the diverging ramp centred at zero, each using one half
     of it.
  3. `K_0` as in option 1 (viridis, `lin`); `V_0` keeps `diverging` and takes the diverging ramp's cool half, so a
     potential reads as the negative side of the energy views `E_0` shares.
- **Applied meanwhile:** TASK-M1-17 applies R-400 to every other field; its `K_0` and `V_0` views follow the ruling.
  REQ-RENDER-084 carries `rq: RQ-261`.
- **Waits:** TASK-M1-17's `K_0` and `V_0` views (the task merges once this is ruled). Nothing else is blocked.
