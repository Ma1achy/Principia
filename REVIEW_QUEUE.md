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

*Found by the physics review of PR #160 (TASK-M1-07, review 5438322479). Nothing is chosen; it goes to the human with
REQ-TOOL-152's calibration at the M1 gate.*

## RQ-242: REQ-TOOL-019's "no banding" has no depth range, and an f32 u bands at depth *(physics, REQ-TOOL-019, REQ-TOOL-152, REQ-DEC-031, TASK-M1-07, M5)*

- **File, section:**
  - `plan/requirements.yaml`, REQ-TOOL-019: "The UV preset … must reconstruct each sample's UV coordinate as deep_zoom
    §1 writes it, u = c_u + h_u·(2t − 1) and v likewise, … for sampled quads with no banding (adjacent-sample deltas
    smooth, not step-quantised, by the criterion REQ-TOOL-152 calibrates)."
  - `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision": "Within-quad precision is full
    f32 at any depth."
  - `plan/requirements.yaml`, REQ-DEC-031: "… the GPU must compute sample positions as u = centre + half·(2t − 1) with
    t = (i + 0.5)/N, never from the quad's min/max bounds." REQ-SCHED-067 uses the same centre-plus-half-width pattern.
- **What was measured (PR #160):** on M1's flat grid every form is exact (departure 0). At depth 30, the f32 sum
  c + h·(2t − 1), which is the coordinate view and the form REQ-DEC-031 names, bands like the global form (departure
  1). c (f64) + δ stays exact (departure 0). qa's sweep (N = 6) shows the f32 sum's departure doubling per level
  (N·2^(ℓ−24)), so at the proposed bound of 1/16 it reads "banded" from about ℓ = 19, just inside ℓ_switch = 20 (R-90).
  §1's "full f32 at any depth" holds for t and δ, but not for a u formed in f32. The decoder is correct only because
  §2 consumes δ, never u.
- **The question (physics; it changes what M5 builds):**
  - (a) Does REQ-TOOL-019's "no banding" apply to an f32 u only down to the depth where REQ-TOOL-152's bound is first
    exceeded, or to the c (f64) + δ path at every depth?
  - (b) Is §1's u carried on the GPU as (c, δ), and never as a global f32 u, past ℓ_switch? That would bear on
    REQ-DEC-031's wording, REQ-SCHED-067 and TASK-M5-04's per-quad uniforms.
- **Not blocked:** TASK-M1-07 meets REQ-TOOL-019's verify on M1's flat grid. The answer shapes M5.
