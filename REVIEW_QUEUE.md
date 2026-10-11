# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-269: Custom's `N` has an upper bound but no lower bound, and the memory models (memory_tiers §3, telemetry §6.2a) have no per-quad `QuadReduction` term, which at Custom `N` = 1 or 2 is the size of `SimState` per render pixel *(silence, R-398, R-410, REQ-PERF-023, REQ-PERF-025, REQ-PERF-035; context: TASK-M5-01, PR #191)*

*Found in perf review 5481822520 of PR #191 (TASK-M5-01), as an observation "outside this task's deliverables; not a
finding against this diff". u8 bins (80 B) would leave the same gap, so R-410 does not create it. Widened in PR #193's
perf review 5481953808 (F1, F2) and physics review 5481954814 (finding 4). Nothing is chosen.*

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
  - `docs/design/principia_dd_telemetry_and_tiers.md` § "6.2a Budgeting before allocating": "The payload
    size is known in advance: `quads x N^2 x (E+1) x sizeof(SimState)`. So:" and "**Budget before allocating.** Query
    the adapter's reported limits, compute the requirement, and if it does not fit, **reduce the tier and report it**
    rather than attempting the allocation."
  - `docs/design/principia_dd_generation_root.md` § "3.7 `QuadReduction` — completed ledger": "There is no memory
    pressure: `QuadReduction` is per *quad*, against `SimState` at per *sample* (millions). The quads to cost it at are
    memory_tiers §3–§4's: render pixels / N² at each tier's `N`, at the largest display, plus their ancestors." It
    gives no size on main: "TASK-M5-01 defines it from the member list, aligns it and measures it, and writes the
    measured figure here".
  - `plan/requirements.yaml` REQ-PERF-086: "Ultra and Extreme must cap N at 16 (replacing memory_tiers §4's 24 and
    32, which exceed the one-workgroup-per-quad ceiling of 256 invocations)".
  - `plan/requirements.yaml` REQ-PERF-035 (Custom's bounds): "N must be a power of two at every tier and in the Custom
    override, which offers no other value (R-398)", verified as "Custom offers powers of two only, each within
    REQ-PERF-011's thread ceiling (R-398)"; REQ-PERF-011's check "asserts N² ≤ maxComputeInvocationsPerWorkgroup for
    every tier and Custom value".
  - `plan/requirements.yaml` REQ-PERF-025 (the estimator): "The memory estimate must be payload_mem = (E+1) × bytes ×
    render_px plus render_targets = k_d × display_px × 4 + k_r × render_px × 4 (k_d ≈ 3, k_r ≈ 3) plus fixed".
  - `plan/requirements.yaml` REQ-PERF-023 (the pre-allocation fit; TASK-M5-11): "Before allocating, the requirement
    quads × N² × (E+1) × sizeof(SimState) must be checked against the adapter's reported limits, and if it does not
    fit the tier must be reduced and reported instead of attempting the allocation".
- **Silence:** Custom's `N` is bounded above (`N² ≤ maxComputeInvocationsPerWorkgroup`) but not below: the "8–32" of
  the scheduler contract covers only the named tiers, so Custom offers `N` = 1 and 2 as powers of two. Neither memory
  model has a per-quad term: memory_tiers §3's estimate and telemetry §6.2a's pre-allocation fit (the check that
  decides whether a Custom setting allocates) both count only `SimState`. §3.7's "no memory pressure" is costed at
  "each tier's `N`" only. `QuadReduction` is 176 B as TASK-M5-01 sums and measures it (PR #191, unmerged; perf review
  5481822520; u32 bins per R-410), 80 B with u8 bins; main does not state a size yet. Per render pixel, leaves plus
  ancestors (× 4/3): about 3.7 B at `N` = 8, but about 59 B at `N` = 2 and 235 B at `N` = 1, the order of
  `SimState`'s 96–144 B, and neither model would count them. The corpus says neither whether `N` = 1 or 2 is a valid
  Custom setting nor whether the memory models count the reductions.
- **A further consequence (correctness, not memory):** the scheduler contract's Part 6 says too low an `N`
  "misclassifies a quad as coherent by undersampling itself". At `N` = 1 a quad has one sample, so a coherence test
  that compares samples has nothing to compare: every quad reads as coherent and refinement never triggers. At `N` = 2
  the test rests on 4 samples. Option (b) alone costs `N` = 1 or 2 correctly but still permits such a setting.
- **Options seen:**
  - (a) A lower bound on Custom's `N` (memory_tiers §5, REQ-PERF-035), at which the reductions stay negligible
    against `SimState`, and at which a quad has enough samples for the coherence test (the further consequence above).
  - (b) A per-quad reduction term (`≈ size_of(QuadReduction) × quads`, ancestors included) in both memory models:
    memory_tiers §3 and REQ-PERF-025's estimator, and telemetry §6.2a and REQ-PERF-023's pre-allocation fit, so every
    `N` is costed and the two models agree.
  - (c) Both.
- **Applied meanwhile:** nothing.
- **Waits:** REQ-PERF-035 (Custom's `N` values; TASK-M6-15), REQ-PERF-025 (the memory estimate; TASK-M5-10) and
  REQ-PERF-023 (the pre-allocation fit; TASK-M5-11, also held through its dependency on TASK-M5-10) carry
  `rq: RQ-269`, so those three tasks wait, and through TASK-M6-15 the Custom `N` field (REQ-GUI-014, TASK-M6-22). All
  three tasks are unmerged and also held by the M1 gate. TASK-M5-01 (PR #191) does not wait: perf judged this not a
  finding against it, and none of its requirements carries the mark.

---

## RQ-270: a saturated configuration control decodes onto or past the poles: at f32 to `α = fl₃₂(π/2)` and `β = fl₃₂(π)`, just past their bounds, and at f32 and f64 to `α = 0` or `β = 0`, for the σ, tanh and softsign links alike, where dd_decoder §3.2 says finite `z` never reaches the poles *(conflict, R-21, R-82, REQ-DEC-018, REQ-DEC-027, REQ-DEC-028, REQ-CHART-034, REQ-VAL-020; context: TASK-M2-02, PR #192)*

*Found by TASK-M2-02's implementer, PR #192, § "Observations for the physics reviewer (not changed here)". Widened in
PR #193's physics review 5481954814 (findings 1–3): the canonical no-op, f64 and the tanh links; and in physics review
5481968627 (F1, the saturated-`z_α` geometry) and code review 5481971521 (F1, the softsign links). Nothing is chosen.*

- **File, section:**
  - `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi": "`α = (π/2) ·
    σ(z_α)          β = π · σ(z_β)`", and "Finite `z` never reaches the exact poles (`α = (π/2)σ(z_α) ∈ (0, π/2)`
    strictly); a config ingested *at* a pole is caught by those flags, and exact-pole `atan2(0,0)=0` yields a
    deterministic canonical value rather than a NaN."
  - `docs/design/principia_dd_decoder.md` § "3.3 Canonicalise": "rotate all rᵢ, pᵢ by R(−φ),  φ = atan2(ρ_y, ρ_x)",
    "mirror (full state: every rᵢ AND every pᵢ, y → −y)  iff λ̃_y < −δ_λ   (λ̃ = √μ_λ·λ, δ_λ = 10⁻¹²)", "|λ̃_y| ≤ δ_λ
    → no mirror   (the one mirror test, in encode's frame — R-82)", and "With the canonical-frame decode both are
    no-ops away from the seam — and **that no-op property is a test** (5.5)."
  - `docs/design/principia_dd_decoder.md` § "5. Unit tests", test 3: "**Range respect:** `α ∈ (0, π/2)`, `β ∈ [0, π]`,
    `|qₖ| ≤ q_max`, for all finite z including ±∞-ish saturation."; test 5: "**Canonical no-op:** for canonical-frame
    decodes, applying §3.3 changes nothing (rotation angle ≈ 0, mirror not taken) away from the `λ̃_y = 0` seam; at
    the seam, the deadband makes the choice deterministic (same result twice)."; test 12 runs the decode as "the
    CPU-f64 and GPU-f32 instantiations".
  - `plan/requirements.yaml` REQ-DEC-027 (TASK-M2-03): "For all finite z, including saturation, the decode must
    respect its ranges α ∈ (0, π/2), β ∈ [0, π], |qₖ| ≤ q_max", and TASK-M2-03's acceptance line "`cargo test -p
    kernel decode_ranges` — decoder test 3 over fuzzed z including ±large values: α ∈ (0, π/2), β ∈ [0, π], |qₖ| ≤
    q_max; masses stay in the open simplex (REQ-DEC-027)."
  - `plan/requirements.yaml` REQ-DEC-018 (TASK-M2-07, `canonical_noop`): "for canonical-frame decodes it must be a
    no-op (rotation angle ≈ 0, mirror not taken) away from the λ̃_y = 0 seam", verified by "decoder test 5: apply C
    to canonical-frame decodes of fuzzed z, assert unchanged". REQ-DEC-002 (TASK-M2-07), named alongside it:
    "Canonicalise must be the single function that quotients a physical state onto the section (CoM at origin, ρ̃ on
    +x, λ̃_y ≥ −δ_λ under the R-82 mirror tie, I = 1), used by both the decode path and encode."
  - `docs/notes/principia_validation_ground_truth_note.md` § "Degenerate configs: open & unguarded, approach as a
    limit": "the parametrisation stays **open `[0,1]`, endpoints excluded** (no endpoint-clamping — no landing
    machinery), and there are **no artificial guards** against approaching degenerate configs. … (sigmoid/softmax
    approach the boundary as `z → ±∞`, so `[0,1]` gets arbitrarily close but never lands)". REQ-DEC-028 (TASK-M2-02):
    "The chart parametrisation must stay open on `[0,1]` with endpoints excluded, with no endpoint clamping and no
    artificial guards against approaching degenerate configurations."
  - `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part
    2.5)": "| Bounded (a,b) | scaled/shifted `σ` | …"; the entries table's rows "| `sigmoid_alpha`, `sigmoid_beta`,
    `sigmoid_q` | α, β, q | `a + (b−a)·σ(x)` | …" and "| `tanh_alpha`, `tanh_beta`, `tanh_q` | α, β, q | `a + c·(1 +
    tanh x)` | …"; and § "5. Tests (properties any generator must satisfy)", test 8: "(a) constraint preservation ∀
    inputs incl. saturation (simplex outputs sum to 1 and stay positive; bounded outputs in range)". REQ-CHART-034
    (TASK-M2-01, merged): "constraint preservation for all inputs incl. saturation". TASK-M2-01's
    `link_properties_a_constraint` (`crates/kernel/tests/links.rs`) checks bounded outputs in the closed `[lo, hi]` at
    the f32-rounded bounds, "which a saturated control reaches exactly, as `σ(x)` rounds to 1".
  - `crates/kernel/src/generated/links.rs` (main): `const LIT_6: f32 = core::f32::consts::FRAC_PI_2;` and `const
    LIT_7: f32 = core::f32::consts::PI;`, both rounded up from π/2 and π (by 4.4×10⁻⁸ and 8.7×10⁻⁸; the f64 constants
    round down); `fn sigmoid` as `R::one() / (R::one() + Float::exp(-x))`; and `TanhAlpha::forward` and
    `TanhBeta::forward`, which read the same `LIT_6` (`(π/2)·½(1 + tanh x)` and `(π/2)(1 + tanh x)`); and
    `SoftsignAlpha::forward` and `SoftsignBeta::forward`, which read the same `LIT_6` and cap `|x|` at `LIT_4`, which is
    `f32::INFINITY` / `f64::INFINITY`, so the cap does nothing for finite `x` and `x/(2 + |x|)` rounds to ±1.
  - The same generated file's `sigmoid_q`, `tanh_q` and `softsign_q`, onto dd_generation_root §3.9's "each free
    momentum `q ∈ (−q_max, q_max)` (dd_decoder §3.4)", reach `±q_max` exactly at saturation, by the same rounding;
    `q_max` = 2 is exact, so they never overshoot. REQ-DEC-027's "|qₖ| ≤ q_max" is closed and admits that; §3.9's open
    codomain and test 8 (a)'s "in range" read as open do not. The mass links (`softmax_tanh`, `stick_breaking`) keep
    every mass positive (`μ_max·tanh` and the `(1−ε_μ)` factor), and the half-line links (`softplus`, `exp`), which
    reach 0 on underflow, serve none of the blocks ("the two lines, which no block uses").
  - *Added from TASK-M2-03's read-only pre-flight:* `docs/design/principia_dd_validation_orbits.md` § "1.3 Lagrange
    and Euler central configurations — the only analytic ones": "**Euler (collinear) is at `beta → 0` and `beta →
    pi`, the top and bottom edges.**" `plan/requirements.yaml` REQ-VAL-020 (TASK-M2-03,
    `chart_landmarks_lagrange_euler`): "A chart-landmark test must assert the decoder places the Lagrange equilateral
    configuration at `z = 0` and the Euler collinear configurations at `beta → 0` and `beta → pi`", verified by
    "decode z = 0 -> equilateral; decode near beta edges -> collinear (approached as a limit)".
- **Conflict:**
  - *Upper poles, f32 only.* At f32, `σ` rounds to 1 for `z ≳ 17` and `tanh` for `z ≳ 9`. For the softsign links,
    `1 + x/(2 + |x|)` rounds up to 2: over most of the band `x/(2 + |x|)` rounds to `1 − 2⁻²⁴`, not to 1, and
    `1 + (1 − 2⁻²⁴)` is a tie that rounds to even, 2. Scanning every f32 (physics review 5481985015, re-checked),
    this first happens at the single point `x = 2²⁴ − 1 = 16777215`, then at every `x` from about 2.237×10⁷ up to 2²⁵,
    at only some `x` up to about 4.474×10⁷, and at every `x` from there on. So a saturated control gives
    `α = fl₃₂(π/2) > π/2` and `β = fl₃₂(π) > π` (for `tanh_beta` and `softsign_beta`, `2·fl₃₂(π/2) = fl₃₂(π)`
    exactly). Then `ρ̃_x = cos α ≈ −4.4×10⁻⁸` and `λ̃_y ≈ −8.7×10⁻⁸·sin α`, past `δ_λ = 10⁻¹²`: the pole and the
    mirror seam are reached by rounding, not by the formula.
  - *Lower poles, f32 and f64.* `α = 0` or `β = 0` exactly where the link's output underflows: for σ at `z ≲ −88.7`
    (f32) and `z ≲ −709.8` (f64), where `exp(−z)` overflows; for the tanh links at `z ≲ −9` (f32) and `z ≲ −19.1`
    (f64), where `tanh` rounds to −1; for the softsign links where `x/(2 + |x|)` rounds to −1, at some `x` from
    `x ≈ −3.4×10⁷` and at every `x` from about `−6.7×10⁷` (f32; 2²⁵ and 2²⁶), and at some `x` from `x ≈ −1.8×10¹⁶`
    and at every `x` from about `−3.6×10¹⁶` (f64; 2⁵⁴ and 2⁵⁵). Code review 5481971521 measured `α = fl₃₂(π/2)` at
    `x = 6.8×10⁷` and `α = 0` at `x = −6.8×10⁷` (f32) and a lower pole at `x ≈ 10¹⁷` (f64); `softsign_beta` has the
    same thresholds. The f64 upper images land on `fl₆₄(π/2)` and `fl₆₄(π)`, which are below π/2 and π.
  - *Canonical no-op.* §3.3 rotates first and then runs the mirror test in the rotated frame. At f32 with `z_α`
    saturated, `ρ̃ = (−4.4×10⁻⁸, 0)`, so `φ = atan2(0, −4.4×10⁻⁸) = π` and §3.3 turns the whole state by a half turn,
    followed by the mirror wherever `sin β` is not ≈ 0: after `R(−π)`, `λ̃_y = −sin α·sin β < −δ_λ` (with
    `λ̃ = sin α·(cos β, sin β)`, §3.2). With `z_β` saturated, `λ̃_y < −δ_λ`, so §3.3 takes the mirror. Decoder test 5
    and REQ-DEC-018 do not hold there. REQ-DEC-002's own text (canonicalise's output lies on the section) still
    holds, since the half turn and the mirror put the state back on it; it is named, not marked.
  - The outputs stay finite and decode's identities hold there (PR #192). §3.2's "never reaches the exact poles …
    strictly", decoder tests 3 and 5, REQ-DEC-027, REQ-DEC-018, and the note's and REQ-DEC-028's "never lands" /
    "endpoints excluded" do not hold as written. TASK-M2-01's merged test reads test 8 (a)'s "in range" as the closed
    interval at the f32-rounded bounds, which the §3.9 row's "(a,b)" does not state.
  - *Chart landmarks (REQ-VAL-020; added from TASK-M2-03's pre-flight).* The β links reach the lower edge exactly,
    `β = 0`, at the lower-pole thresholds above (f32 and f64), and at f32 pass the upper edge, `β = fl₃₂(π) > π` (by
    8.7×10⁻⁸). So a decode "near beta edges" at a large enough `|z_β|` lands on the edge, or at f32 past it, where
    REQ-VAL-020's verify has the Euler configurations "approached as a limit" and validation_orbits §1.3 writes
    `beta → 0` and `beta → pi`. The options below are unchanged by this addition; none names REQ-VAL-020's text, and
    under each `β = 0` stays reachable at saturation ((b) keeps the lower poles).
- **Context for weighing the options (physics review 5481954814, finding 3; not a recommendation):** the states past
  the upper poles are valid three-body configurations. With `z_α` saturated the state is the in-range configuration
  (`α' = π − fl₃₂(π/2) < π/2`, `β' = π − β`), mirrored and turned by π about the CoM; `β = fl₃₂(π) > π` is the
  in-range `β' = 2π − fl₃₂(π)`, mirrored. Both sit at a near-collision of the inner pair (`‖ρ̃‖ ≈ 4×10⁻⁸`), which
  the collision and conditioning flags already fence (§3.2), and the overshoot is under one f32 ulp of α near π/2
  (1.19×10⁻⁷). So what (a) gives up
  is the literal range statements and the canonical no-op, not physical correctness.
- **Options seen:**
  - (a) Amend dd_decoder §3.2, tests 3 and 5, REQ-DEC-027 and REQ-DEC-018 (and the note / REQ-DEC-028) to state the
    behaviour at saturation: the range closed and the poles reachable, at f32 for the upper poles and at f32 and f64
    for the lower poles, for the σ, tanh and softsign links (and, for `q`'s open codomain in §3.9, `sigmoid_q`,
    `tanh_q` and `softsign_q` reaching `±q_max`); the canonical no-op excepting the saturated poles; the outputs
    finite and the identities holding.
  - (b) Generate the f32 constants rounded toward zero (`fl↓(π/2)`, `fl↓(π)`) so the upper image stays inside the
    bounds (a change to the generated links, TASK-M2-01's output). It removes the upper overshoot and so the half turn
    and the mirror; it does not remove the lower poles: `α = 0` or `β = 0` would remain for σ at `z ≲ −88.7` (f32)
    and `z ≲ −709.8` (f64), for the tanh links at `z ≲ −9` (f32) and `z ≲ −19.1` (f64), and for the softsign links
    from `x ≈ −3.4×10⁷` (every `x` from about `−6.7×10⁷`, f32) and from `x ≈ −1.8×10¹⁶` (every `x` from about
    `−3.6×10¹⁶`, f64). The `q` links' `±q_max` would remain as well.
  - (c) Another form the corpus permits. Clamping the output is not one: REQ-DEC-028 and the note exclude "endpoint
    clamping" and "artificial guards".
- **Applied meanwhile:** nothing.
- **Waits:**
  - REQ-DEC-027 carries `rq: RQ-270`, so TASK-M2-03 (`decode_ranges`) waits.
  - REQ-DEC-028 carries `rq: RQ-270` (applied per R-369: at f32 the parametrisation reaches its endpoints by rounding,
    which its "endpoints excluded" directly contradicts), so TASK-M2-02 (PR #192) waits.
  - REQ-DEC-018 carries `rq: RQ-270` (the canonical no-op fails at f32 saturation), so TASK-M2-07 (`canonical_noop`)
    waits. REQ-DEC-002, in the same task, is named but not marked: its text is not contradicted.
  - The REQ-CHART-034 tension: §3.9's "Bounded (a,b)" and test 8 (a)'s "bounded outputs in range" against TASK-M2-01's
    merged `link_properties_a_constraint`, which reads the range as closed at the f32-rounded bounds. TASK-M2-01 has
    merged, so the ruling may need a follow-up to that test or to the generated links. REQ-CHART-034 is not marked.
  - REQ-VAL-020 carries `rq: RQ-270` (added from TASK-M2-03's pre-flight: at saturation `β` reaches 0 at f32 and f64
    and passes π at f32, against its "approached as a limit"), so TASK-M2-03 (`chart_landmarks_lagrange_euler`)
    waits, as it already does through REQ-DEC-027. TASK-M3-28, which needs REQ-VAL-020, waits with it.

---

## RQ-271: REQ-TOOL-160's log floors for `rho_ratio` and `r_min_pair_0` are an M1 calibration, in "that field's units", but the two fields' formulas are an M2 deliverable (REQ-PAY-088, TASK-M2-03): for `rho_ratio` its units with them, for `r_min_pair_0` only the formula's details and the evidence *(silence, R-71, R-72, R-401, REQ-TOOL-160, REQ-PAY-088; context: TASK-M1-18, TASK-M2-03)*

*Found in TASK-M2-03's read-only pre-flight. Narrowed in PR #194's physics review 5482528309 (F1, the split; F3,
`r_min_pair_0`'s units). Nothing is chosen.*

- **File, section:**
  - `plan/tasks/M1/TASK-M1-18.md` § "Deliverables" (lines 33–34; moved from `TASK-M1-17.md` lines 81–82 by this
    entry's PR): "`crates/ledger`: one log floor per log field without a ledger floor (`d_min`, `dE_max`, `dLz_max`,
    `closure_min`, `rho_ratio`, `r_min_pair_0`), each a named presentation constant in that field's units, read by
    the template"; and (lines 35–36) "The REQ-TOOL-160 proposal: each floor, its units, and its evidence (where the
    field's stored values fall under it on M1's fixtures, and why it sits there), attached to the PR."
  - `plan/requirements.yaml` REQ-TOOL-160 (`kind: calibration`, `milestone: M1`, TASK-M1-18): "The log floor ε_f of
    each log-scaled numeric field view whose field has no ledger `floor` (d_min, dE_max, dLz_max, closure_min,
    rho_ratio, r_min_pair_0) — one floor per field, in that field's own units (a distance for the distances, … a
    ratio for rho_ratio; R-401) … must be calibrated: proposed with its evidence per field by the task that needs it,
    checked by a reviewer, confirmed by the human at the M1 gate and recorded in decisions.md."
  - `decisions.md` § "R-401 — Each log-scaled field view has its own floor, in that field's units, proposed with
    evidence for the M1 gate; the single ε = 2⁻²⁴ is replaced": "its own `ε_f`, the magnitude where its compression
    sets in, in that field's own units: a distance for the distances, … and a ratio for `rho_ratio`", and "**The
    floors are calibrations (R-71):** proposed with their evidence for the M1 gate".
  - `plan/requirements.yaml` REQ-PAY-088 (`kind: definition`, `milestone: M2`, TASK-M2-03): "The formulas of the
    ICDescriptor fields `q_mass`, `rho_mag`, `lambda_mag`, `rho_ratio`, `rho_angle` and `r_min_pair_0` (including
    whether `rho_mag` and `lambda_mag` are ‖ρ‖, ‖λ‖ or the mass-weighted ‖ρ̃‖, ‖λ̃‖) must be defined in dd_decoder §3.6
    by the task that needs it". `plan/tasks/M2/TASK-M2-03.md` § "Notes", Gap G7: "the ICDescriptor formulas for …
    `rho_ratio`, `rho_angle` and `r_min_pair_0` are not in the corpus".
  - `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time, pre-integration)":
    "`E₀ = K₀ + V₀ ;   virial_ratio = 2K₀ / |V₀| ;   ρ-magnitudes, ρ_ratio, ρ_angle, r_min_pair₀`", names with no
    formula. `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)": "`rho_ratio` (log), …
    `r_min_pair_0` (log). Provenance: decode stage, pre-integration."
  - `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)":
    "Output is `(m, r, p)` in the dimensionless system `G = M = I = 1`"; § "4. Seams (its side of each — the
    integration-test list)", seam 3: "emit exactly `(m, r, p)`, canonical, `G=M=I=1`".
  - `fixtures/golden/BASELINES.md` (lines 84, 88): M1's `generated-r_min_pair_0` and `generated-rho_ratio` cases render
    each "generated view over the showcase set"; the decode stage that writes the two fields is TASK-M2-03's
    (REQ-PAY-034).
- **Silence:** the corpus says the floor is "in that field's own units" and calibrated at the M1 gate, and that the
  field's formula is defined by TASK-M2-03 at M2; it does not say which comes first. The two fields differ in what is
  open:
  - *`rho_ratio`: units and evidence.* R-401 gives the kind of unit, "a ratio", but not which: ‖ρ‖/‖λ‖ and the
    mass-weighted ‖ρ̃‖/‖λ̃‖ differ by a mass-dependent factor, √(μ_ρ/μ_λ) (REQ-PAY-088's own parenthesis), which moves
    the magnitude where compression should set in.
  - *`r_min_pair_0`: evidence and the formula's details, not its units.* R-401 calls it "a distance", and the decoder's
    output gauge, `G = M = I = 1` (§2, seam 3), fixes the scale a distance is measured in. What stays open is the
    formula's details (REQ-PAY-088) and the evidence.
  - For both, M1's fixtures hold showcase values, not decoded ones, so the evidence REQ-TOOL-160 asks for ("where the
    field's stored values fall under it") has no decoded values to fall under before M2.
  - The other four fields are defined in `docs/design/principia_dd_simstate_payload.md` (`d_min` and `closure_min` as
    running minima, `dE_max` and `dLz_max` as absolute maxima, REQ-TOOL-160's note), so the order question is for
    `rho_ratio` and `r_min_pair_0` only.
- **Options seen** (the field distinction above changes only (c) and (d), as marked):
  - (a) Calibrate those two floors at the M2 gate instead: REQ-TOOL-160 is split (or a new calibration requirement
    raised) so TASK-M1-18 proposes four floors for the M1 gate and the two others are proposed with TASK-M2-03's
    definitions, on decoded values. TASK-M1-18 still needs some constant for the two views meanwhile, which the corpus
    does not give, and the M1 gate confirms four floors, not six.
  - (b) Write TASK-M2-03's two definitions (part of REQ-PAY-088) first, ahead of the M1 calibration: a docs change to
    dd_decoder §3.6 for `rho_ratio` and `r_min_pair_0`, reviewed by physics, before TASK-M1-18 proposes. It splits a
    definition out of an M2 task into the M1 path; the evidence is then the formula, not decoded values, since the
    decode stage still lands at M2.
  - (c) Give the two floors provisionally, confirmed at the M1 gate as provisional and re-confirmed at M2 once
    TASK-M2-03 defines the formulas. For `rho_ratio` the proposal states assumed units (which ratio), and the
    assumption may not match the definition, so the floor may change at M2. For `r_min_pair_0` the units are not
    assumed (a distance in `G = M = I = 1`); only the evidence is provisional, so the re-confirmation at M2 is of the
    evidence. Two confirmations of each value.
  - (d) Read R-401's unit kinds ("a ratio", "a distance") as enough to propose the floors at M1 without the formulas,
    with TASK-M2-03's definitions then bound to be consistent with them. For `rho_ratio` it settles which ratio before
    its definition is written and reviewed; for `r_min_pair_0` it settles no unit the gauge leaves open, only the
    evidence, which is then the showcase values and the gauge's scale.
  - (c) and (d) may also be taken per field: one for `rho_ratio`, another for `r_min_pair_0`.
- **Applied meanwhile:** the plan split, applied per R-369 (physics review 5482528309 of PR #194, F1): REQ-TOOL-160's
  closing moves from TASK-M1-17 to a new task, TASK-M1-18 ("The per-field log floors (R-401)"), which depends on
  TASK-M1-17. TASK-M1-17 keeps its four other closures and is not held by this entry. The split settles nothing here.
- **Waits:** REQ-TOOL-160 carries `rq: RQ-271`, so:
  - TASK-M1-18 waits.
  - The M1 gate's confirmation of REQ-TOOL-160 waits, and so the M1 exit, and with it every task the M1 gate holds
    (all later-milestone tasks, save an early merge under R-415).
  - TASK-M7-05 depends on TASK-M1-18 (its `log` compaction reads each field's R-401 floor), and through it TASK-M7-08,
    TASK-M7-09 and TASK-M7-23 and their dependents, a chain that is behind the M1 exit anyway.
  - Checked and not dependent on TASK-M1-18, as none reads a floor value: TASK-M1-14 (its goldens are re-rendered by
    TASK-M1-18 if they draw a log view), TASK-M1-15, TASK-M2-25, TASK-M3-18 and TASK-M6-27.
  - REQ-PAY-088 is named, not marked: TASK-M2-03 can write its definitions under any option, and under (b) only
    sooner.

---

## RQ-272: REQ-PAY-034 asserts every ICDescriptor field in f32 (and f64) against hand-computed values, but the corpus gives no tolerance for that comparison *(silence, R-71, R-85, REQ-PAY-034, REQ-DEC-043, REQ-DEC-044, REQ-VAL-061; context: TASK-M2-03)*

*Found in TASK-M2-03's read-only pre-flight. Widened in PR #194's physics review 5482528309 (F2, the parity
contract's starting tolerance). Nothing is chosen.*

- **File, section:**
  - `plan/requirements.yaml` REQ-PAY-034 (TASK-M2-03, `ic_descriptor`), verify: "golden IC (z = 0): assert every
    descriptor field in f64 and f32 against hand-computed values; field names match the ledger; the derived E₀ equals
    K_0 + V_0 and the struct has no E₀ field". TASK-M2-03's acceptance line repeats it: "`cargo test -p kernel
    ic_descriptor` — golden IC (z = 0): every descriptor field in f64 and f32 against hand-computed values".
  - `docs/design/principia_dd_decoder.md` § "5. Unit tests", the golden anchor: "**`z = 0` decodes to the canonical
    golden IC** — equal masses `(⅓,⅓,⅓)`, `α = π/4` (‖ρ̃‖ = ‖λ̃‖), `β = π/2`, rest (`q = 0`), `I = 1`. Assert its every
    derived value in f64 and f32." No tolerance is given there.
  - The same section, test 12: "the CPU-f64 and GPU-f32 instantiations (and browser-GPU-via-WGSL) agree on the golden
    IC and fuzzed interiors to f32 eps-scaled tolerance on continuous values". `plan/requirements.yaml` REQ-DEC-043
    (calibration, TASK-M2-06): "dd_decoder test 12's f32-eps scale factor for cross-backend agreement on continuous
    decode values must be calibrated", its evidence "f64-vs-f32 decode differences on the golden IC and fuzzed
    interiors".
  - `plan/requirements.yaml` REQ-DEC-044 (calibration, TASK-M2-02): the tolerances of "dd_decoder unit tests 1 (the
    identities …), 6 (the reconstruction inverse) and 8 (the invariant construction's L_z and K)". On PR #192's branch
    only (not on main), REQ-DEC-045 (test 1's Σpᵢ = 0) and REQ-DEC-046 (test 8's L_z and K) are split from it. None of
    the three names a descriptor field.
  - `docs/contracts/principia_parity_contract.md` § "Tier N — tight numerical (checkable because nothing amplifies)":
    "**decode → `(m, r, p)`, `ICDescriptor`, `E₀`**: the whole Matter rung (decoder dd §5)"; and § "4. Tolerance — and
    the cross-backend reality": "Tier-N tolerance absorbs **two** sources of difference, not one: 1. **f32 vs f64** —
    the intended comparison. 2. **cross-backend f32 variance**" (lines 141–144), and (line 148) "Relative tolerance,
    per quantity class: IC/decode ~1e-5; one-step state ~1e-5 scaled by force magnitude; monitored `E₀`/`L_z` at t=0
    ~1e-6. These are starting points to be replaced by measured values."
  - `plan/requirements.yaml` REQ-VAL-061 (`milestone: M4`, TASK-M4-04): "Parity Tier N must compare elementwise within
    tight tolerance: decode → `(m, r, p)`, `ICDescriptor` and `E₀`; …", verified with "starting tolerances: IC/decode
    ~1e-5 relative; … — to be replaced by measured values". REQ-TOOL-029's note (R-133): "before REQ-VAL-064 sets Tier
    N (M4), 'Tier-N tolerance' here is read as REQ-DEC-043's calibrated f32 decode factor."
- **Silence:** no requirement or doc gives the tolerance at which an f32 (or f64) descriptor field matches its
  hand-computed value. The nearest value is the parity contract's "IC/decode ~1e-5" relative, which covers the
  `ICDescriptor`, but it does not settle this comparison as written: it is Tier N's, a cross-backend parity check
  closed at M4 (REQ-VAL-061, TASK-M4-04) and set there by measurement (R-85, REQ-VAL-064); it is explicitly a
  "starting point" to be replaced; and it compares f32 with f64, not with hand-computed values (though "f32 vs f64 —
  the intended comparison" is one reading of REQ-PAY-034's f32 leg). It gives the f64 leg nothing, and as a relative
  tolerance it says nothing for a field whose golden value is 0 (`K_0` and `virial_ratio` at rest). Test 12's
  tolerance is a cross-backend agreement (CPU-f64 against GPU-f32), not a comparison with hand-computed values, and its factor (REQ-DEC-043) is proposed by TASK-M2-06, which runs after TASK-M2-03
  (TASK-M2-06 depends on TASK-M2-05, which depends on TASK-M2-03). REQ-DEC-044's tests do not include the
  descriptor. "Hand-computed" at f32 is itself open: the f32 rounding of the exact value, or the value the formula
  gives evaluated in f32.
- **Options seen:**
  - (a) A new R-71 calibration requirement for the descriptor fields' per-precision tolerances against the
    hand-computed golden values, proposed with evidence by TASK-M2-03 and confirmed by the human at the M2 gate, as
    REQ-DEC-044 is for tests 1, 6 and 8. One more M2-gate calibration; TASK-M2-03 can run, with its value provisional
    until the gate.
  - (b) Reuse test 12's "f32 eps-scaled tolerance" (REQ-DEC-043's factor) for the f32 comparison. No new value, but
    the factor is calibrated by TASK-M2-06, after TASK-M2-03, for a different comparison (f64 against GPU-f32); either
    TASK-M2-03 waits for it, or REQ-DEC-043's proposal moves earlier. It leaves the f64 comparison without a
    tolerance.
  - (c) Bit-exact f32 (and f64) where the formula allows it, a tolerance only where it does not. At rest
    `K_0 = 0` and `virial_ratio = 0` are exact; fields through a quotient, square root or trigonometric function (`V_0`,
    and whatever REQ-PAY-088 defines for the magnitudes and `rho_angle`) are exact only if "hand-computed" means the
    same operations in the same order at that precision, so the fields left over still need (a) or (b).
  - (d) Take the parity contract's "IC/decode ~1e-5" relative (§4, REQ-VAL-061) as the f32 leg's provisional
    tolerance, against the f64 value or the hand-computed one, until Tier N is measured at M4. A corpus value, no new
    one; but it is a placeholder set for a different, cross-backend check, it leaves the golden's zero-valued fields
    and the f64 leg to (a) or (c), and the M4 measurement may then change what TASK-M2-03's test asserts.
- **Applied meanwhile:** nothing.
- **Waits:** REQ-PAY-034 carries `rq: RQ-272`, so TASK-M2-03 (`ic_descriptor`) waits. TASK-M3-19 and TASK-M3-22, which
  need REQ-PAY-034, wait with it. TASK-M2-03 also waits on RQ-270 (REQ-DEC-027, REQ-VAL-020) and on TASK-M2-02.
