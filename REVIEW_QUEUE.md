# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-277: `C_ty` is on `ramp_coolwarm` only because its range is undeclared, which R-426 makes a ledger gap, not "spans zero"; the corpus gives it no range and no sign, R-423's physics check did not cover it, and nothing says what its view shows meanwhile *(silence, R-426, R-400, R-423, REQ-GEN-035, REQ-RENDER-084, REQ-PAY-073; context: the M1 gate, TASK-M1-21, TASK-M1-17, TASK-M3-13)*

*Found by the port of R-426's survey (every numeric field on cool-warm only by the ledger's unbounded default). Filed
because R-426's specific clause names `q_mass` and `mean_y` only. Nothing is chosen.*

- **File, section:**
  - `decisions.md` § "R-426 — …", the human's words: "an undeclared range is a ledger gap, not "spans zero"; a field
    is diverging only when its declared range spans zero (RQ-260)", and, for two fields only: "q_mass and mean_y stay
    on cool-warm provisionally, recorded as provisional at the gate".
  - `crates/ledger/src/payload.rs`:218 (at 7063e004), `f32_at("C_ty", 27, Scale::Lin),` with no `.range(…)`, so the
    unbounded default of `entry()` (:26–37), which `NumericView::of` (`crates/ledger/src/gen/numeric.rs`:161,
    `Scale::Lin if e.range.spans_zero() => Placement::Diverging`) puts on `ramp_coolwarm`;
    `crates/ledger/tests/numeric_view_template.rs`:762 expects it there, and PR #195's `m1-numeric/c_ty_signed`
    golden draws it there.
  - `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata": `C_ty` has
    no row (the table gives no scale or range); the mini-spec: "`C_ty   += dt * (y - mean_y)       // shared time-dev ×
    per-sample new-dev`" and "`slope = C_ty / C_tt               // = Cov(t,y)/Var(t); diffusion coefficient`".
    `docs/design/principia_dd_simstate_payload.md` § "4. Welford diffusion (streaming regression)": "`state.C_ty   +=
    delta_t * (y - state.mean_y);   // OLD time-dev × NEW y-dev`". `docs/contracts/principia_render_contract.md` §
    "Field views (one per field, every struct)" lists `C_ty` among the `SimState` scalars that each have a view.
  - `decisions.md` § "R-423 — …" and `plan/MILESTONES.md` § "M1"'s gate item: physics checked "the six fields R-400
    moved to cool-warm", `q_mass`, `ftle`, `mean_y`, `S`, `theta` and `diffusion`, and confirmed `S`, `ftle` and
    `diffusion` signed. `C_ty` is not among them.
- **Silence:** `C_ty` has no declared range in §3 or the ledger and no sign in the corpus, so under R-426 its range is
  a gap and does not make it diverging. R-426 keeps only `q_mass` and `mean_y` on the diverging ramp provisionally. The
  corpus does not say which ramp a field with an undeclared range takes meanwhile, nor whether `C_ty` is signed. Its
  definition reads as a centred co-moment, `Σ δ_t·(y − ȳ)`, whose sign is the diffusion slope's (`C_tt > 0` for
  `n ≥ 2`), and `diffusion` passed R-423's check; but no physics check of `C_ty` has been made, and `y` is undefined
  until REQ-PAY-073 (TASK-M3-13, M3).
- **Options seen:**
  - (a) Declare `C_ty`'s range (−∞, ∞), signed, now: a physics check confirms that the co-moment is signed whatever
    `y` is (its sign that of the diffusion slope), as R-423's check did for `diffusion`; it stays on `ramp_coolwarm`,
    confirmed at the M1 gate.
  - (b) Keep it on `ramp_coolwarm` provisionally, as R-426 does for `mean_y`, recorded as provisional at the M1 gate;
    TASK-M3-13, which defines `y` (REQ-PAY-073), declares its range, and the physics reviewer re-checks the ramp then.
  - (c) Move it off the diverging ramp until its range is declared (on viridis, the template's ramp for a range that
    does not span zero), its golden re-rendered; TASK-M3-13 then declares its range and the ramp follows it.
- **Applied meanwhile:** nothing is built: `C_ty` stays on `ramp_coolwarm` as TASK-M1-17 left it, which settles
  nothing here. R-426's other fields are settled: `S`, `ftle` and `diffusion` are declared signed by R-423's check,
  `E_0` and `Lz_0` by their `diverging` scale (§3.4).
- **Waits:** REQ-GEN-035 carries `rq: RQ-277`, so TASK-M1-21's PR, which closes it and declares every such range,
  does not merge until this entry is ruled; with it waits the M1 gate. TASK-M3-13 (REQ-PAY-073) does not wait: it
  defines `y` under every option, and (b) and (c) need it to run; it is named, not marked.
