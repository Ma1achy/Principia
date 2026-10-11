# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-269: Custom's `N` has an upper bound but no lower bound, and memory_tiers §3's model has no per-quad `QuadReduction` term, which at Custom `N` = 1 or 2 is the size of `SimState` per render pixel *(silence, R-398, R-410, REQ-PERF-025, REQ-PERF-035; context: TASK-M5-01, PR #191)*

*Found in perf review 5481822520 of PR #191 (TASK-M5-01), as an observation "outside this task's deliverables; not a
finding against this diff". u8 bins (80 B) would leave the same gap, so R-410 does not create it. Nothing is chosen.*

- **File, section:**
  - `docs/design/principia_memory_tiers.md` § "3. Memory model — payload + render targets": "`payload_mem    = (E+1) ×
    bytes × render_px` (the SimState samples — one per render pixel at the floor)" and "`total          ≈
    payload_mem + render_targets + fixed            (fixed = bake texture, staging)`", with `bytes` = "144/96".
  - `docs/design/principia_memory_tiers.md` § "4. The six quality tiers": "`N~`/`depth~` are indicative — the tier
    sets the sample grid and refinement budget", the table's `N~` of 8 (Potato, Low) and 16 (Medium to Extreme), and
    "**Ultra and Extreme cap `N` at 16** (`N² ≤ 256`, the one-workgroup-per-quad invocation ceiling)".
  - `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact": "**Custom mode** exposes
    `render_scale` (0.25–2.0; <1 performance, >1 supersampling), `N`, `MAX_REL_DEPTH`, E, and FTLE directly (arbiter
    off)", and "**Custom's `N` is a power of two (R-398)**, each within the one-workgroup-per-quad thread ceiling (`N² ≤`
    `maxComputeInvocationsPerWorkgroup`, REQ-PERF-011); it offers no other value."
  - `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy": "`SAMPLES_PER_QUAD_AXIS` (`N` —
    the sample grid inside a quad; each sample is a full simulation, so this is *the* quality/memory/compute driver —
    too low misclassifies a quad as coherent by undersampling itself) … **Both are tier-gated with a Custom
    override.** Per-tier values in the memory-tiers table (`N` ranges 8–32 across the six tiers; rel-depth budget
    likewise per tier)."
  - `docs/design/principia_dd_generation_root.md` § "3.7 `QuadReduction` — completed ledger": "There is no memory
    pressure: `QuadReduction` is per *quad*, against `SimState` at per *sample* (millions). The quads to cost it at are
    memory_tiers §3–§4's: render pixels / N² at each tier's `N`, at the largest display, plus their ancestors."
  - `plan/requirements.yaml` REQ-PERF-086: "Ultra and Extreme must cap N at 16 (replacing memory_tiers §4's 24 and
    32, which exceed the one-workgroup-per-quad ceiling of 256 invocations)".
  - `plan/requirements.yaml` REQ-PERF-035 (Custom's bounds): "N must be a power of two at every tier and in the Custom
    override, which offers no other value (R-398)", verified as "Custom offers powers of two only, each within
    REQ-PERF-011's thread ceiling (R-398)"; REQ-PERF-011's check "asserts N² ≤ maxComputeInvocationsPerWorkgroup for
    every tier and Custom value".
  - `plan/requirements.yaml` REQ-PERF-025 (the estimator): "The memory estimate must be payload_mem = (E+1) × bytes ×
    render_px plus render_targets = k_d × display_px × 4 + k_r × render_px × 4 (k_d ≈ 3, k_r ≈ 3) plus fixed".
- **Silence:** Custom's `N` is bounded above (`N² ≤ maxComputeInvocationsPerWorkgroup`) but not below: the "8–32" of
  the scheduler contract covers only the named tiers, so Custom offers `N` = 1 and 2 as powers of two. §3's model has
  no per-quad term, and §3.7's "no memory pressure" is costed at "each tier's `N`" only. `QuadReduction` is 176 B with
  u32 bins (R-410; 80 B with u8). Per render pixel, leaves plus ancestors (× 4/3): about 3.7 B at `N` = 8, but about
  59 B at `N` = 2 and 235 B at `N` = 1, the order of `SimState`'s 96–144 B, and the model would not count them. The
  corpus says neither whether `N` = 1 or 2 is a valid Custom setting nor whether the estimate counts the reductions.
- **Options seen:**
  - (a) A lower bound on Custom's `N` (memory_tiers §5, REQ-PERF-035), at which the reductions stay negligible
    against `SimState`.
  - (b) A per-quad reduction term in memory_tiers §3's model (`≈ size_of(QuadReduction) × quads`, ancestors
    included), carried into REQ-PERF-025's estimator, so every `N` is costed.
  - (c) Both.
- **Applied meanwhile:** nothing.
- **Waits:** REQ-PERF-035 (Custom's `N` values; TASK-M6-15) and REQ-PERF-025 (the memory estimate; TASK-M5-10) carry
  `rq: RQ-269`, so those two tasks wait, and through TASK-M6-15 the Custom `N` field (REQ-GUI-014, TASK-M6-22). Both
  tasks are unmerged and also held by the M1 gate. TASK-M5-01 (PR #191) does not wait: perf judged this not a finding
  against it, and none of its requirements carries the mark.

---

## RQ-270: at f32 a saturated configuration control decodes to `α = fl₃₂(π/2)` and `β = fl₃₂(π)`, just past their bounds, and to `α = 0` for `z_α ≲ −88`, where dd_decoder §3.2 says finite `z` never reaches the poles *(conflict, R-21, R-82, REQ-DEC-027, REQ-DEC-028, REQ-CHART-034; context: TASK-M2-02, PR #192)*

*Found by TASK-M2-02's implementer, PR #192, § "Observations for the physics reviewer (not changed here)". Nothing is
chosen.*

- **File, section:**
  - `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi": "`α = (π/2) ·
    σ(z_α)          β = π · σ(z_β)`", and "Finite `z` never reaches the exact poles (`α = (π/2)σ(z_α) ∈ (0, π/2)`
    strictly); a config ingested *at* a pole is caught by those flags, and exact-pole `atan2(0,0)=0` yields a
    deterministic canonical value rather than a NaN."
  - `docs/design/principia_dd_decoder.md` § "5. Unit tests", test 3: "**Range respect:** `α ∈ (0, π/2)`, `β ∈ [0, π]`,
    `|qₖ| ≤ q_max`, for all finite z including ±∞-ish saturation." Test 12 runs the decode as "the CPU-f64 and
    GPU-f32 instantiations".
  - `plan/requirements.yaml` REQ-DEC-027 (TASK-M2-03): "For all finite z, including saturation, the decode must
    respect its ranges α ∈ (0, π/2), β ∈ [0, π], |qₖ| ≤ q_max", and TASK-M2-03's acceptance line "`cargo test -p
    kernel decode_ranges` — decoder test 3 over fuzzed z including ±large values: α ∈ (0, π/2), β ∈ [0, π], |qₖ| ≤
    q_max; masses stay in the open simplex (REQ-DEC-027)."
  - `docs/notes/principia_validation_ground_truth_note.md` § "Degenerate configs: open & unguarded, approach as a
    limit": "the parametrisation stays **open `[0,1]`, endpoints excluded** (no endpoint-clamping — no landing
    machinery), and there are **no artificial guards** against approaching degenerate configs. … (sigmoid/softmax
    approach the boundary as `z → ±∞`, so `[0,1]` gets arbitrarily close but never lands)". REQ-DEC-028 (TASK-M2-02):
    "The chart parametrisation must stay open on `[0,1]` with endpoints excluded, with no endpoint clamping and no
    artificial guards against approaching degenerate configurations."
  - `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part
    2.5)": "| Bounded (a,b) | scaled/shifted `σ` | …", and § "5. Tests (properties any generator must satisfy)", test
    8: "(a) constraint preservation ∀ inputs incl. saturation (simplex outputs sum to 1 and stay positive; bounded
    outputs in range)". REQ-CHART-034 (TASK-M2-01, merged): "constraint preservation for all inputs incl.
    saturation". TASK-M2-01's `link_properties_a_constraint` (`crates/kernel/tests/links.rs`) checks bounded outputs
    in the closed `[lo, hi]` at the f32-rounded bounds, "which a saturated control reaches exactly, as `σ(x)` rounds
    to 1".
  - `docs/design/principia_dd_decoder.md` § "3.3 Canonicalise": "mirror … iff λ̃_y < −δ_λ   (λ̃ = √μ_λ·λ, δ_λ = 10⁻¹²)".
  - `crates/kernel/src/generated/links.rs` (main): `const LIT_6: f32 = core::f32::consts::FRAC_PI_2;` and `const
    LIT_7: f32 = core::f32::consts::PI;`, both rounded up from π/2 and π (by 4.4×10⁻⁸ and 8.7×10⁻⁸).
- **Conflict:** at f32, `σ` rounds to 1 for `z ≳ 17`, so a saturated control gives `α = fl₃₂(π/2) > π/2` and
  `β = fl₃₂(π) > π`. Then `ρ̃_x = cos α ≈ −4.4×10⁻⁸` and `λ̃_y ≈ −8.7×10⁻⁸·sin α`, past `δ_λ = 10⁻¹²`: the pole and
  the mirror seam are reached by rounding, not by the formula. For `z_α ≲ −88`, `α = 0` exactly. The outputs stay
  finite and decode's identities hold there (PR #192). §3.2's "never reaches the exact poles … strictly", decoder test
  3 and REQ-DEC-027's `α ∈ (0, π/2)`, `β ∈ [0, π]`, and the note's "never lands" do not hold at f32 as written.
  TASK-M2-01's merged test reads test 8 (a)'s "in range" as the closed interval at the f32-rounded bounds, which the
  §3.9 row's "(a,b)" does not state.
- **Options seen:**
  - (a) Amend dd_decoder §3.2 and REQ-DEC-027 (and test 3) to state the f32 behaviour: a closed range at f32, the poles
    reachable at saturation, the outputs finite and the identities holding.
  - (b) Generate the f32 constants rounded toward zero (`fl↓(π/2)`, `fl↓(π)`) so the image stays inside the bounds
    (a change to the generated links, TASK-M2-01's output). `α = 0` at `z_α ≲ −88` would remain.
  - (c) Another form the corpus permits. Clamping the output is not one: REQ-DEC-028 and the note exclude "endpoint
    clamping" and "artificial guards".
- **Applied meanwhile:** nothing.
- **Waits:**
  - REQ-DEC-027 carries `rq: RQ-270`, so TASK-M2-03 (`decode_ranges`) waits.
  - REQ-DEC-028 carries `rq: RQ-270` (applied per R-369: at f32 the parametrisation reaches its endpoints by rounding,
    which its "endpoints excluded" directly contradicts), so TASK-M2-02 (PR #192) waits.
  - The REQ-CHART-034 tension: §3.9's "Bounded (a,b)" and test 8 (a)'s "bounded outputs in range" against TASK-M2-01's
    merged `link_properties_a_constraint`, which reads the range as closed at the f32-rounded bounds. TASK-M2-01 has
    merged, so the ruling may need a follow-up to that test or to the generated links. REQ-CHART-034 is not marked.
