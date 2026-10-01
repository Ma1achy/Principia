# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---


## RQ-188: The generated WGSL needs three things the corpus does not give: the word buffer's and `SimState`'s binding (group, number, and which variant), `closure_step`'s WGSL form (WGSL has no u16), and the schema version's WGSL form (WGSL has no u64) *(TASK-M0-13, REQ-RENDER-001, REQ-PAY-091)*

- **File, section:**
  - `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)", its
    WGSL traps: "The word buffer is separately bound and indexed identically to samples (per-copy)."
  - `plan/tasks/M0/TASK-M0-13.md` § "Acceptance tests": "the word buffer is its own binding indexed per copy"; and its
    Notes: "The fragment read side's tier-uniform interface (`has_<feature>` consts, NaN for absent features —
    lowering Part 3a) is M1's; this task emits the stored layouts only."
  - `docs/contracts/principia_lowering_contract.md` § "Part 3a — The uniform read-side interface": "`sample.word` |
    reads the bound word buffer | buffer unbound → returns the **empty/sentinel word**", and "the Benettin **shadow
    trajectory** (48 B, marched every step) and the **word buffer binding** are absent in the feature-off variant".
  - `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct": "the two sizes are handled by
    the baked-per-tier read shader", and, in its "WGSL read-view syntax", "`closure_step : u16,`" then
    "`_reserved    : u16,`".
  - `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract": "The schema version is a content hash
    of the canonicalised §3 ledger"; the generated Rust carries it as `pub const PAYLOAD_SCHEMA_VERSION: u64` (R-36,
    R-63), and the task's Goal puts "the schema version" in the WGSL.
- **What:**
  1. **Bindings.** No section gives a group or binding number for the `SimState` buffer or the word buffer, nor
     which stored variant (`SimStateFTLE` or `SimStateBase`) the generated layer binds. Part 3a makes both the shadow
     and the word binding per-tier (absent in the feature-off variant), and the task leaves that interface to M1, yet
     REQ-RENDER-001 asks this task's WGSL to bind the word buffer separately.
  2. **`closure_step`.** WGSL has no u16 (and `enable f16` is banned, R-86), so the u16 `closure_step` and u16
     `_reserved` at byte 140 can only be one u32 member. Its name, and the accessor that reads the step, are not given.
  3. **Schema version.** WGSL has no u64, so the 64-bit hash needs another form; none is given.
- **Options seen:**
  1. **Bindings.**
     a. **The generated layer declares the full tier's bindings (recommended):**
        `@group(0) @binding(0) var<storage, read> simstate_buffer: array<SimStateFTLE>;` and
        `@group(0) @binding(1) var<storage, read> word_buffer: array<vec4<u32>>;`, each read through a generated
        function of the sample index `i` (`sample_state(i)`, `sample_word(i)`), the same `i` for both. M1's per-tier
        assembly (Part 3a) owns the feature-off variants.
     b. As (a), in group 1, leaving group 0 to the assembler's uniforms.
     c. The generated layer declares no binding; the M1 assembler does, and REQ-RENDER-001's binding clause moves to
        M1.
  2. **`closure_step`.**
     a. **One member `closure_step_reserved: u32` (recommended)**, `closure_step` in bits 0–15 and `_reserved` in bits
        16–31, read through a generated `fn closure_step(w: u32) -> u32 { return extractBits(w, 0u, 16u); }`.
     b. One member named `closure_step: u32`, read whole (the reserved half is zero).
  3. **Schema version.**
     a. **`const PAYLOAD_SCHEMA_VERSION: vec2<u32>`, `.x` the low 32 bits, `.y` the high 32 (recommended)**, the
        order `unpack2x16float` uses.
     b. Two constants, `PAYLOAD_SCHEMA_VERSION_LO` and `PAYLOAD_SCHEMA_VERSION_HI`.
- **Needed:** one option for each. TASK-M0-13 waits; its branch carries the recommended options, marked as waiting
  on this entry.
