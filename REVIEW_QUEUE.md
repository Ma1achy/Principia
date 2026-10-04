# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-204: `dbg_sentinel`'s "suspect-flag styling hook" has no input and no definition *(REQ-TOOL-009)*

- **File, section:**
  - `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug
    view)": "fn dbg_sentinel(x: f32, frag_xy: vec2f) -> vec3f // absence-NaN (exact bitcast test) →
    debug_invalid(frag_xy), the hatch (R-136); a stored sentinel such as −1.0 shows as its literal value on the ramp
    (R-79); suspect-flag styling hook".
  - `plan/requirements.yaml`, REQ-TOOL-009: "… dbg_sentinel(x, frag_xy) (absence-NaN via exact bitcast →
    debug_invalid(frag_xy), the hatch; a stored sentinel such as −1.0 shown as its literal value on the ramp; suspect
    styling hook; R-136)"; its verify detail names no suspect case.
  - `plan/requirements.yaml`, REQ-TOOL-122 (the definitions TASK-M1-03 writes, R-72): "dbg_sentinel's hatch pattern
    for absence-NaN, dbg_cat's golden-angle lightness and chroma …, dbg_log's compression formula and its eps,
    dbg_hash_u32's hash, and dbg_flag's green and red". It doesn't list the suspect styling.
  - `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules": "the drift suspect gates are read-time
    predicates over the stored latches (payload §5)".
- **What:** the signature `dbg_sentinel(x, frag_xy)` carries no suspect flag, and no doc says what the styling is
  (a tint, an outline, a second hatch), which suspect predicate drives it, or whether "hook" means an extension point
  for a later task, not behaviour now. REQ-TOOL-122 assigns TASK-M1-03 the other `dbg_*` definitions, not this one.
  TASK-M1-03 builds `dbg_sentinel` as the contract states it, NaN bits to the hatch and every other value on the ramp,
  and adds no suspect styling.
- **Options seen:**
  1. **"Hook" is an extension point, not behaviour (recommended):** `dbg_sentinel` stays as built. The presentation
     layer's line says the suspect styling is applied by the view that reads the suspect predicate, in a later task
     (the drift field views), on `dbg_sentinel`'s output. REQ-TOOL-009 closes with TASK-M1-03.
  2. **A styling now:** `dbg_sentinel` gains a `suspect: bool` argument, which changes the contract's signature, and a
     defined style (for example, every other stripe of the hatch's pattern, or a darkened ramp colour). TASK-M1-03
     defines and builds it.
- **Needed:** the human's choice. Option 1 reads the hook as later work, and only the human defers. Option 2 changes a
  contract signature. Until then, REQ-TOOL-009's suspect part waits; the rest of TASK-M1-03 proceeds.
