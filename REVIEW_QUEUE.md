# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

*Found by the physics review of PR #158 (TASK-M1-11, review 5436669140), raised first by qa there. Nothing is chosen.*

## RQ-226: θ̃'s frozen reference and its has-reference flag have no storage across a resumed march *(physics, ledger, R-389, R-392, TASK-M3-11, REQ-INT-082)*

- **File, section:**
  - `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep,
    ratified)", R-389's paragraph: "on entering `√(n_u² + n_v²) < r_pole`, the last longitude (the last one outside
    the radius) is stored; no delta is added while inside; on exit, `wrap(exit longitude − stored longitude)` is
    added", and R-392's: "An IC that starts inside the radius has no stored longitude, and **its first exit adds no
    delta**".
  - `docs/design/principia_dd_simstate_payload.md` § 2, on resumption: "on a **cached-state resume** (a quad
    continuing its march from cached `SimState` at `t_cached`, which lockstep does)".
  - The same file, § 1 and § 2, on the free space: `_reserved : u16, // free under alignment (4 B and 8 B cost the
    same); do not spend`, and descriptor bits 10–15: "Reserved, decode as zero, never opportunistically reused".
    `SimStateFTLE` is "144 B actual / 144 B effective".
- **The silence:** the corpus says the longitude is "stored" but gives it no slot. The hold needs two values that
  persist between macro-steps, the frozen longitude and a has-reference flag (set outside the radius, clear for an IC
  that starts inside). The previous shape point can be recomputed from the stored positions; these two cannot. Kept
  only in kernel locals, they are lost at a cached-state resume, so θ̃ would depend on how the march was split into
  dispatches. Every place to keep them is protected by the payload document, so any choice is a ledger change.
- **Options** (from the physics review, which prefers (a)):
  - **(a)** A new f32 `theta_ref`, plus the flag as a versioned assignment of descriptor bit 10. `SimStateFTLE` goes
    from 144 B to 152 B (+8 B per sample with padding). Implements R-389 exactly.
  - **(b)** The reference as a u16 longitude in `_reserved` (steps of about 9.6e-5 rad), plus the bit-10 flag. No
    size change, but the exit delta and the exact-π case become quantised.
  - **(c)** Rebuild the reference from θ̃ and the longitude θ̃ counts from. It still needs storage for R-392's
    IC-inside case, and it changes R-389's arithmetic.
  - **(d)** Drop the hold at dispatch boundaries. Physics rejects it: θ̃ would depend on the dispatch split.
- **Needed:** a ruling on where the two values live, before TASK-M3-11 (REQ-INT-082) runs θ̃ in the kernel's march.
  TASK-M1-11 computes θ̃ in a single call and is not blocked.

*Found applying R-395, from PR #160's evidence (physics review 5438875182; qa's
`qa_uv_preset_reconstruction_global_form_depth_sweep`). Nothing is chosen; it goes to the human with REQ-TOOL-152's
calibration at the M1 gate.*

## RQ-258: R-395's "no banding with absolute coordinates up to ℓ_switch" against REQ-TOOL-152's proposed bound at a non-dyadic N *(calibration, physics, R-395, R-90, REQ-TOOL-019, REQ-TOOL-152, TASK-M1-16, M1 gate)*

- **File, section:**
  - `decisions.md` § "R-395 — REQ-TOOL-019's "no banding" holds with absolute coordinates up to ℓ_switch, checked at
    the M1 gate, and through the per-quad local coordinates beyond it, at M5 *(closes RQ-242)*", the human's words:
    "REQ-TOOL-019 means no banding with absolute coordinates up to the deep-zoom switchover (ℓ_switch, R-90) … The M1
    gate checks the first half."
  - `decisions.md` § "R-90 — The decoder switchover trigger *(closes RQ-41)*": "Switch to the linearised decoder when
    the full decoder's adjacent samples give bitwise-identical ICs, with ℓ_switch = 20 as an upper bound (whichever
    comes first)."
  - `docs/design/principia_memory_tiers.md` § "4. The six quality tiers", the tier table: `N~` is 8 (Potato, Low) or
    16 (Medium to Extreme); and § "5. Controller levers, ranked by impact": "**Custom mode** exposes `render_scale`
    (0.25–2.0; …), `N`, `MAX_REL_DEPTH`, E, and FTLE directly".
  - `plan/requirements.yaml`, REQ-TOOL-152: the bound on "how far the adjacent-sample deltas … may depart from the exact
    step 2h/N", "confirmed by the human at the M1 gate". PR #160 proposes `BANDING_BOUND = 1/16`
    (`crates/render/src/coords.rs`, marked proposed, R-71).
- **What was measured** (PR #160, and the same f32 arithmetic recomputed for this entry), the absolute coordinate
  `c + h·(2t − 1)` in f32 near `u ≈ 0.6`, as the departure `max |Δ − 2h/N| / (2h/N)` at ℓ = 16 to 22:
  - N = 6: 1/64, 1/32, 1/16, 1/8, 1/4, 1/2, 1.
  - N = 12: 1/32, 1/16, 1/8, 1/4, 1/2, 1, 2.
  - N = 8: 0 to ℓ = 20, then 1. N = 16: 0 to ℓ = 19, then 1.
- **The conflict:** at a power-of-two N the sum is exact until adjacent samples collapse to one coordinate, and a
  collapse makes the full decoder's adjacent ICs bitwise-identical, so R-90's switchover fires first: R-395's first
  half holds. At a non-dyadic N the departure grows by graded steps, so at 1/16 it reads "banded" from ℓ = 19 at N = 6
  and from ℓ = 18 at N = 12, before ℓ_switch = 20 and before any collapse that would fire the switchover. There, R-395's
  first half and the proposed bound cannot both hold.
- **Options seen:**
  1. N is a power of two. The named tiers' N already are (8 and 16); Custom mode offers only powers of two.
  2. The bound is set at the gate so that no N the product offers exceeds it before ℓ_switch; at N = 12 that is at
     least 1/4 below ℓ = 20, and 1/2 if ℓ_switch itself is included.
  3. The switchover also fires where the absolute coordinate's departure first exceeds the bound, which adds a trigger
     to R-90's two.
  4. R-395's first half is checked at the named tiers' N only, and a Custom non-dyadic N is outside it.
- **Applied meanwhile:** TASK-M1-16's sweep covers N = 8 and N = 16, where the ruling holds with no choice; a
  non-dyadic N joins it once this is ruled. REQ-TOOL-152 carries `rq: RQ-258`.
- **Waits:** the M1 gate's confirmation of REQ-TOOL-152. Nothing is blocked before it.
