# TASK-M1-21 — Undeclared ranges are ledger gaps (R-426)

- **Milestone:** M1
- **Closes:** REQ-GEN-035
- **Depends on:** TASK-M1-20
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
**R-426** amends R-400's reading: an undeclared range is a ledger gap, not a range that spans zero; a field is
diverging only when its declared range spans zero (RQ-260). Today the ledger's `entry()` writes (−∞, ∞) for every field
it is given no range for, and the numeric template places a `lin` field on `ramp_coolwarm` whenever its range spans
zero, so the default puts fields there. This task makes the ledger tell a declared range from an undeclared one, so
the default never reads as spanning zero, and declares, each with its citation, the ranges the corpus settles:
`S`, `ftle` and `diffusion` signed, (−∞, ∞), by R-423's physics check; `E_0` and `Lz_0` signed, (−∞, ∞), by their
`diverging` scale (dd_generation_root §3.4). `q_mass` and `mean_y` are marked provisional (R-426): they keep
`ramp_coolwarm` until TASK-M2-03 and TASK-M3-13 declare their ranges (REQ-PAY-097, REQ-PAY-098). A `lin` or
`diverging` scalar field whose range is undeclared and not so marked fails generation, naming it, and every remaining
gap is listed with the requirement that fills it, for the M1 gate report. No view changes ramp.

**Fix round of PR #205 (applied per R-369):** `C_ty` is declared (−∞, ∞), signed, citing R-423's check of `diffusion`
and `C_ty = diffusion · C_tt(n)`, `C_tt(n) > 0` for `n ≥ 2` (RQ-277, decided by the orchestrator, option (a)), and
stays on `ramp_coolwarm`. `energy_drift` and `Lz_drift` are declared (−∞, ∞), signed, citing §3.8's worked entries
and §3.4: the ledger gives them `diverging` and a `floor` but no range. The range states are §3.8's
`declared(lo, hi, citation) | gap | provisional(requirement)`.

## References
- `decisions.md` § "R-426 — `q_mass` and `mean_y` stay on cool-warm provisionally, recorded as provisional at the M1 gate, and are re-checked when TASK-M2-03 and TASK-M3-13 declare their ranges; an undeclared range is a ledger gap, not a range that spans zero *(closes RQ-274; amends R-400)*"
- `decisions.md` § "R-400 — A numeric field view whose declared ledger range spans zero is on a diverging ramp centred at zero; fields that can't be negative keep viridis *(closes RQ-260)*"
- `decisions.md` § "R-403 — `K_0` and `V_0` are `lin` on viridis over `[0, ∞)` and `(−∞, 0]`; a field carries the `diverging` scale only when its declared range spans zero *(closes RQ-261)*"
- `decisions.md` § "R-423 — At the M1 gate physics checks the six fields R-400 moved to cool-warm; `theta`, an angle, takes the cyclic ramp, not the diverging one *(amends R-400)*"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library"
- `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"

## Deliverables
- `crates/ledger/src/schema.rs` and `crates/ledger/src/payload.rs`: a range is declared, with its citation, or
  undeclared (a gap), or marked provisional (R-426); `entry()`'s (−∞, ∞) default becomes undeclared, and an
  undeclared range never spans zero. `S`, `ftle` and `diffusion` are declared (−∞, ∞) citing R-423's physics check;
  `E_0` and `Lz_0` (−∞, ∞) citing §3.4's `diverging` scale; `q_mass` and `mean_y` marked provisional (R-426), naming
  REQ-PAY-097 and REQ-PAY-098. Every other field the template draws keeps the range it declares.
- In the same place: `C_ty` declared (−∞, ∞), signed, citing R-423 and `C_ty = diffusion · C_tt(n)` (RQ-277, decided
  per R-369); `energy_drift` and `Lz_drift` declared (−∞, ∞), signed, citing §3.8's worked entries and §3.4. Each
  range is one of §3.8's three states: `declared(lo, hi, citation)`, `gap` or `provisional(requirement)`.
- `crates/ledger/src/gen/numeric.rs` (with the generation gate): a `lin` or `diverging` scalar field whose range is
  undeclared and not marked provisional fails generation, naming the field and R-426; a provisional range is drawn as
  one that spans zero, as R-426 keeps those two fields.
- The gap list: every entry whose range is still undeclared (for example `theta`, `rho_angle` and the vectors `r`,
  `p`, `r_sh`, `p_sh`, which no ramp reads the range of), each with the requirement that fills it or "none", printed
  by a test or `cargo xtask` command and attached to the PR for the M1 gate report.
- The payload.rs module doc and §3.8's worked entries, where they read the default as a range, conformed.

## Acceptance tests
- `cargo test -p ledger declared_ranges` — every `lin` or `diverging` scalar entry the numeric template draws has a declared range with its citation, or is `q_mass` or `mean_y` marked provisional (R-426); `S`, `ftle` and `diffusion` are declared (−∞, ∞) citing R-423's check, `E_0` and `Lz_0` citing §3.4's `diverging` scale, `C_ty` citing R-423 and `C_ty = diffusion · C_tt(n)`; `energy_drift` and `Lz_drift`, which the numeric template skips as floored fields, are named explicitly and declared (−∞, ∞) citing §3.8's worked entries and §3.4, so generation passes on them; controls: a fixture `lin` entry with an undeclared range and no provisional mark fails generation naming it, and a fixture `lin` entry with an undeclared range is not placed on `ramp_coolwarm` by it; the gap list names each remaining gap with the requirement that fills it (REQ-GEN-035).
- `cargo test -p ledger numeric_view_template` and `cargo xtask golden --all` — every view that drew on `ramp_coolwarm` before still does, and no golden changes (REQ-GEN-035).
- Review checklist (physics): each declared range's citation, and `q_mass`'s and `mean_y`'s provisional marks, name what R-426 and R-423 settle (REQ-GEN-035).

## Notes
- New M1 work from R-426, applied per R-369: the human's "an undeclared range is a ledger gap, not 'spans zero'"
  needs the ledger to tell the two apart. An undeclared range fails generation where it would choose the ramp, as
  §3.8's "A field without a complete entry fails generation loudly" has it, `range` being a key every entry carries;
  q_mass and mean_y are the ruling's own exception. The M1 gate waits on this task.
- Waits on RQ-277 (`C_ty`, on cool-warm only by the same default): REQ-GEN-035 carries `rq: RQ-277`, so the PR does
  not merge until it is ruled. The task may be built meanwhile, `C_ty` left as TASK-M1-17 left it, and finished when
  the ruling lands.
  *Superseded in PR #205's fix round:* RQ-277 was decided per R-369 (option (a)); REQ-GEN-035 no longer carries
  `rq: RQ-277`, and this task waits on nothing but TASK-M1-20.
- After TASK-M1-20, which changes `theta`'s entry in `payload.rs` (R-423); TASK-M2-03 depends on this task, since it
  removes `q_mass`'s provisional mark (REQ-PAY-097).
- qa tests of TASK-M1-09, TASK-M1-17 or TASK-M1-20 that build an entry with the unbounded default and expect it on
  `ramp_coolwarm` (for example `crates/ledger/tests/qa_TASK-M1-17.rs`'s `retagged("q_mass", range(Unbounded,
  Unbounded))`) are ruling-forced changes (R-426): qa makes them, and the PR lists each with the assertion it changes
  (R-290, R-369).
