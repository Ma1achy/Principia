# Drill-down — the SimState payload (consolidated authoritative spec)

*The per-sample payload, in one place. This supersedes the scattered field definitions across the ledger, render contract, and design sessions — it is the single authoritative statement of what a sample stores, in what precision, at what bit offsets, and what is derived rather than stored. When implementation starts, this transcribes directly into the **Rust layout definition** (the codegen source, from which are generated — to two targets, the substrate split — **Rust** pack/unpack for the kernel + the Rust host export decoder, and **WGSL** unpack accessors + debug catalogue for the fragment side; generation-root §1). Bit layouts here are the truth; the accessors in the render contract are illustrative of generated output.*

---

## 0. The two buffers (split by access pattern)

The payload is **two physical buffers, one logical entity**, split by *when the march kernel touches each field*:

| Buffer | Contents | Access pattern | Size |
|---|---|---|---|
| **`SimState`** (hot) | phase state, shadow, accumulators, drift refs, packed status/times/scalars | read/updated **every substep** by the march | **144 B** (FTLE-on); **96 B** (FTLE-off) — *was 136 / 88 before the 8 B closure field, §1* |
| **word buffer** (cold) | the free-group word (`vec4<u32>`) | **append on branch-cut crossing; read at resolve/inspect** — never per-step | 16 B/entry, ×(E+1) (no shadow, appended in place), sharded per-quad |

**Why split.** `SimState` fields are **hot** in the sense of being *co-resident march state updated during marching* — the physics and shadow (every substep), the accumulators, the drift refs and latches, `times`/`total_substeps`/descriptor (updated as the march progresses). Not every field is literally loaded+written at every substep (`packed_a/b`, `times`, `total_substeps` are march bookkeeping updated during marching, not per-substep operands) — they belong in the hot struct because they are co-resident march state, not because each is touched every substep. The word alone is **cold**: appended only on branch-cut crossings and read only at resolve/inspect. Co-locating it inline (a) forced the struct to 16-byte alignment (the `vec4` is the sole 16-align driver), padding the tail with **12 dead bytes**; (b) dragged 16 B of unused word through every hot cache line. Moving the word to a parallel buffer drops `SimState` to **8-byte alignment**, kills the padding (→ 4 B residual), and cuts the persistent hot-state footprint 160→136 (FTLE-on) / 112→88 (FTLE-off) — a **~15% reduction in the persistent hot-state footprint**, expected to reduce storage traffic and working-set pressure (the *measured* bottleneck is unconfirmed — see §3). Cost: one extra buffer binding + an indexed word fetch on the *cold* paths only. **Only the word moves** — splitting the hot fields further would fragment co-accessed data and hurt the prefetcher.

**Indexing & lifecycle.** `word_buffer[i]` is the word for the sample whose state is `simstate_buffer[i]` — same `(quad, sample, copy)`→index map, per-copy. The two buffers are **one logical payload** `f(IC, sim key, t)`: they allocate, evict, recompute, and resume together, and share the cache compatibility signature (caching contract). A `SimState` at index `i` and a word at index `i` must always describe the same trajectory at the same playhead — never desynchronise.

---

## 1. `SimState` — the hot struct

```wgsl
// Shown in WGSL read-view syntax for readability; the AUTHORITATIVE/stored definition is the Rust
// kernel layout (the kernel is Rust → SPIR-V — lowering Part 2). Bit offsets/sizes are language-agnostic.
// TWO stored sizes — the Benettin shadow is 48 B, so FTLE-on/off are monomorphised as two Rust variants
// to avoid paying for the shadow when FTLE is off (a MEMORY split, NOT a language limitation):
//   SimStateBase (FTLE off): no shadow, 96 B actual / 96 B effective.
//   SimStateFTLE (FTLE on):  with shadow, 144 B actual / 144 B effective.
// The fragment READ side is UNIFIED (one type; absent shadow → NaN via the has_ftle const — §5, lowering
// Part 3a), so it needs no runtime member-omission; the two sizes are handled by the baked-per-tier read shader.

struct SimStateFTLE {
    // ── PHASE STATE (f32) — CoM-frame PARTICLE coordinates ──  48 B
    r  : array<vec2<f32>, 3>,     // body 0,1,2 positions (CoM frame), re-projected to CoM each step
    p  : array<vec2<f32>, 3>,     // body 0,1,2 momenta   (CoM frame)
    //   NOT Jacobi. Jacobi (2 vectors, 8D) is the CHART/IC representation; the integrator
    //   works in particle coordinates (clean force loop — no per-step Jacobi↔particle conversion).
    //   Conversion Jacobi→particle happens ONCE at IC decode. The CoM constraint makes one of the
    //   three vectors redundant, but all three are stored symmetrically so the force loop is uniform
    //   and branch-free, and the per-step CoM re-projection operates on all three.

    // ── BENETTIN SHADOW (f32) — CoM-frame particle coords ───  48 B
    r_sh : array<vec2<f32>, 3>,   // shadow trajectory — this sample's own ftle ingredient;
    p_sh : array<vec2<f32>, 3>,   //   marches in lockstep, also re-projected to CoM each step

    // ── ACCUMULATORS (f32 — summed, MUST stay f32) ──────────  16 B
    S      : f32,                 // Benettin sum → ftle = S_final/(step_count·dt); finalise partial interval at read (§5)
    theta  : f32,                 // unwrapped phase     → orbit_count, retrograde
    mean_y : f32,                 // Welford diffusion (per-sample)
    C_ty   : f32,                 // Welford diffusion (per-sample)

    // ── DRIFT REFERENCES (f32) ──────────────────────────────   8 B
    E_0    : f32,                 // initial energy — per-step ref: ΔE = H(r,p) − E_0
    Lz_0   : f32,                 // initial L_z    — per-step ref: ΔLz = L_z(r,p) − Lz_0

    // ── PACKED WORDS (u32 ×3) ───────────────────────────────  12 B
    packed_a : u32,               // sample_descriptor[bits 0–9 used, 10–15 reserved] | d_min:f16[high 16]
    packed_b : u32,               // dE_max:f16[low] | dLz_max:f16[high]
    times    : u32,               // t_end_step:u16[low] | t_dmin_step:u16[high]  (EXACT step indices)

    // ── EXACT SUBSTEP COUNTER (u32) ─────────────────────────  4 B
    total_substeps : u32,         // exact Σ N_sub over the trajectory (resumable — see note).
                                  //   The log₂ complexity proxy is DERIVED at read (countLeadingZeros),
                                  //   NOT stored in the descriptor (descriptor uses bits 0–9; 10–15 reserved).

    // ── CLOSURE (return-map) ────────────────────────────────  8 B
    closure_min  : f32,           // running min over t > t_min of |n̂(t) − n̂(0)| — the SHAPE-sphere
                                  //   closure, so rotation is quotiented out and RELATIVE periodic
                                  //   orbits count, not only inertially-periodic ones. Latched in a
                                  //   local f32 during the march (the `d_min` rule); this is a
                                  //   running minimum, NOT a history — two registers, no array.
    closure_step : u16,           // exact step index of the minimum — binary-parity surface, same
                                  //   convention as t_dmin_step. This is the period label.
    _reserved    : u16,           // free under alignment (4 B and 8 B cost the same); do not spend
                                  //   it without re-checking the alignment argument below.
}
// SimStateFTLE: 144 B actual / 144 B effective (8-byte aligned)
// SimStateBase: drop r_sh,p_sh → 96 B actual / 96 B effective
```

### Why closure is here, at this width, unconditionally

**What it is.** A **running minimum** — `if d < closure_min { closure_min = d; closure_step = k }`.
Two registers updated once per step, the same shape as `d_min`. **No array, no history**, so it obeys
*reduce before evaporate* by construction.

**Why f32 and not f16.** The precision that binds is the **trajectory**, not the storage. `r, p` are
f32, so a difference of two O(1) f32 states has a floor of **~1.2e-7**. f16 cannot hold that
linearly — its smallest subnormal is 6.0e-8 with ~1 digit, and a closure of 1e-9 **flushes to zero**,
making every periodic orbit indistinguishable from a merely quiet region. That is the whole signal
destroyed by the storage choice. Log-encoded f16 would work; f32 is chosen because **it is free**
(below) and needs no encode/decode, and because it does not truncate the **f64 CPU path**, where
closure genuinely reaches ~1e-16.

**Why 8 B costs the same as 4.** `SimStateFTLE` was 136 B = 17×8, exactly packed. Adding 4 B gives
140, which pads to **144**; adding 8 B gives **144** as well. So the `u16 _reserved` is genuinely
free — but that also means **spending it later is not free**, since the next field pushes to 152.

**Why unconditional rather than tier-gated.** The FTLE split exists because the shadow is **48 B —
55% of the 88 B base**, which justifies monomorphising. Closure is **8 B, 5.9%**. Gating it would
turn 2 variants into 4, each needing its own pack/unpack, its own WGSL reader and its own tier-table
row — a large combinatorial surface for 8 bytes. Cost: **136 → 144 (+5.9%)** FTLE-on,
**88 → 96 (+9.1%)** FTLE-off; ~+4 KB per quad at N=8, E+1=8; ~+8 MB against the quoted ~140 MB.

**Why inline and NOT a parallel buffer.** The word lives in its own buffer because it is **cold** —
appended only on branch-cut crossings, read only at resolve. **Closure is hot**: updated every step,
co-resident march state. The word's precedent does not apply, and moving closure out by analogy
would drag an extra binding onto the hot path for no benefit.

**The floor is precision-dependent, and this must be reported.** Closure bottoms out at **~1e-7 on
the f32 GPU path** and **~1e-16 on the f64 CPU path** — so an orbit closing to 1e-9 in the reference
reads as 1e-7 on the GPU. **That is saturation, not a lost orbit**, and it is the same class of
reportable limit as `t_max = ln(1/eps)/lambda`. Note the contrast is still ~7 decades against a
random IC's O(1) closure, so orbits remain unmissable at f32.

**One semantic that must be pinned:** closure is trivially 0 at `t = 0`, so the minimum is taken over
`t > t_min`. Define `t_min` **gauge-covariantly** — after the state has moved by more than a stated
fraction of the system size — not as an absolute time, or the field inherits a scale.

**Coordinate pipeline (resolves the "Jacobi" naming):** Jacobi coordinates (2 position + 2 momentum vectors, the minimal 8D symmetry-reduced set) are the **chart / IC** representation — where configurations are *defined*. The **integrator** works in **CoM-frame particle coordinates** (3 bodies) — where the force loop is direct (no per-substep Jacobi↔particle conversion; the potential depends on pairwise particle separations, which are ugly in Jacobi). Conversion happens **once at IC decode** (Jacobi → 3 particles). **After each step the state is re-projected to the CoM** so numerical error doesn't let the centre of mass drift. This re-projection is **part of the deterministic fixed-`dt` step** (integrate → project). Re-projection manages CoM position / total linear momentum; it **does not explicitly restore energy or `L_z`** — but note that subtracting the spurious CoM *velocity* removes bulk kinetic energy, so the projection **does numerically affect the evaluated energy** (and, in principle, `L_z`). The correct framing: the projection does not explicitly restore E or `L_z`; any change from removing accumulated CoM position/momentum drift **remains visible in the post-projection invariant diagnostics** (which is what those diagnostics are for).

> **Parity: the float trajectory is NOT bit-identical CPU↔GPU** (see the parity contract's Tier B/N/S model — this summarises).
> WGSL permits operation-specific floating-point error, FMA/fused ops, and binary16 subnormal flushing; JS uses f64; GPU backends differ. So promising bit-identical *particle state* across CPU and GPU is impossible. Split it:
> - **Binary parity (exact):** packed descriptors, the integer step counts (`t_end_step`, `t_dmin_step`), `total_substeps`, enum/`state` values, and the **word arithmetic** (integer mixed-radix) must match bit-for-bit CPU↔GPU. These are the fields the cache/export/parity machinery compares exactly.
> - **Numerical parity (within declared tolerance):** particle state `r,p`, energy, `L_z`, and any trajectory-derived quantity must match within stated tolerances, **not** bit-for-bit.
> The high-precision locked inspector (f64 CPU) is a **higher-precision validation path** — its purpose is to *check* the GPU trajectory to tolerance, **not** to reproduce it bit-for-bit. (It is the *shared kernel* at f64: its value is **precision**, not independence — it shares source with the survey; the shared-bug-immune reference is the *separate* convergence integrator, `principia_validation_ground_truth_note.md` / Precision ring.) The re-projection being "on the parity surface" means it must be applied **identically in structure** (same operation, same order) on both sides so the *numerical* comparison is apples-to-apples — it does **not** mean the float results are bit-identical.

**Shared derived-time constants (NOT a mutable global):** the Welford time-only moments and step count are *derived*, not stored or barrier-shared — see §4.

**Precision rule (why each field is the width it is):**
- **f32, mandatory:** phase state and shadow (integration — f16 storage of a chaotic state injects rounding that chaos amplifies; empirically confirmed, §5); all summed accumulators `S, theta, mean_y, C_ty` (summing many terms in f16 loses the tail); `E_0, Lz_0` (subtraction operands for drift — f16 resolves O(1) energy to ~1e-3 = the drift magnitude, tested catastrophic).
- **f16, safe — but they are MONOTONE LATCHES, not "written once":** `d_min = min(d_min, d)`, `dE_max = max(dE_max, |ΔE|)`, `dLz_max = max(dLz_max, |ΔLz|)` — updated repeatedly during the march (absolute maxima / running min over the whole trajectory). Two precise semantics, pick one: **(a)** hold the latch in a **local f32** during the march and pack to f16 only when writing persistent state (one requantisation per persist boundary — preferred); or **(b)** accept repeated f16 requantisation at every resume/write. Either way, **threshold/suspect decisions must use the live f32 measurement, never the unpacked f16 latch** (the f16 latch is for display, not for control flow).
- **f16 packing does NOT require the `shader-f16` feature.** `pack2x16float`/`unpack2x16float` take/return `vec2<f32>` and are core WGSL — available everywhere. The `shader-f16` extension is only needed for *native f16 types* (`f16` scalars/vectors), which this design does not use. So the f16-packed scalars are **not tier-gated** — no f32 fallback needed.
- **Pack requires clamp/canonicalise first:** `pack2x16float` gives an **indeterminate** result when an input is outside binary16's finite range (±65504). The packer must clamp or canonicalise before packing. **Failed-state contents are defined:** for `sim_failed`/`decode_failed` samples, `d_min`/`dE_max`/`dLz_max` halves hold **0.0** (a canonical sentinel — the `state` field already marks the sample untrusted, so the scalar values are not read for those states; 0.0 is chosen over clamped-65504 or NaN so a stray read is visibly-null rather than plausibly-real or `isNan`-unreliable). Valid-state values are clamped to ±65504 before packing (a drift exceeding 65504 in normalised units is already pathological and the sample would be flagged).
- **bf16:** nowhere — wrong precision/range trade for bounded normalised quantities (bf16 buys exponent range you don't need at the cost of mantissa you do), and not in web WGSL anyway.

---

## 2. Bit layouts (the packed u32s)

### `packed_a` (u32)
```
 bit  31 ····························· 16 | 15 ························· 0
      └──────── d_min : f16 ───────────┘ | └──── sample_descriptor ─────┘
                                         |      (10 bits used)
```
`d_min = unpack2x16float(packed_a).y`. The descriptor occupies the low 16 bits (bits 0–9 used, 10–15 reserved — §2); `sd_*` accessors read bits 0–9 of `packed_a` directly.

### `sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved
| Field | Bits | Width | Values / scale |
|---|---|---|---|
| `state` | 0–2 | 3 | **enum(6), mutually-exclusive**: 0 escape · 1 bounded · 2 collision · 3 running · 4 sim_failed · 5 decode_failed. **Codes 6–7 reserved** — a binary decoder treats **unknown non-running states as conservatively finished and untrusted** (forward-compat). `detail` meaningful when state ∈ {escape, collision, sim_failed, decode_failed} (enum below) |
| `detail` | 3–4 | 2 | **union keyed by state** — 4 codes per state. escape → body id (0–2), `3` = **triple ejection**; collision → pair id (0–2), `3` = **triple collision** — one rule, *3 means all three* (pending change 7); sim_failed / decode_failed → failure category |
| `saturated` | 5 | 1 | sticky — **`N_sub == N_max` occurred** at some macro-step (substep cap hit; advance-and-flag, never terminates) |
| `dmin_pair` | 6–7 | 2 | categorical(3) — which pair achieved `d_min` (pair id 0–2 per the map below, `3`=unset/invalid; latched, NOT from the word) |
| `last_symbol` | 8–9 | 2 | **final symbol of the free-group word**, cached from the append loop for O(1) fragment read (frozen codes `a=0, A=1, b=2, B=3`). Redundant with the sidecar word `W` (recoverable there only in O(length)); kept coherent by the march (§3). **No in-band "none" code** — validity gates on the sidecar `length`: meaningful iff `length ≥ 1 && length ≠ 127` (empty word → no last symbol; truncated → invalid, like every word-derived read). Written by `set_last_symbol` wherever the loop mutates `prev` (§3, §6) |
| *reserved* | 10–15 | 6 | `total_substeps_log2` derived at read from the exact `total_substeps` u32 (§1), not stored (a log accumulator is not resumable). Reserved, decode as zero, never opportunistically reused |

> **`last_symbol` (bits 8–9) is a deliberate, versioned assignment — a binary-format version bump, not an opportunistic reuse of reserved space.** Bits 8–15 were held "decode as zero, never opportunistically reused"; spending 8–9 is a format change on the same footing as changing the frozen continuation table (§3) — every reader must agree on the new layout, so it carries a format-version increment. The remaining 10–15 stay reserved under the same rule. The field is a *redundant cache* of the sidecar word's final symbol (§3): it adds no new physical information (the word `W` already determines it), only O(1) fragment access; a reader that ignores it loses only the cheap last-symbol read, never correctness.

> **`bounded` (state 1) is a FINITE-HORIZON classification, not a proof of permanent boundedness.** It means: neither escape nor collision occurred **within horizon `T`** (which lives in the sim key). Permanent boundedness is (in general) **not decidable** for the three-body problem — no finite integration can certify it — so `bounded` is the standard field term for "survived to `T`" and carries no claim about `t > T`. This is not a limitation being papered over: finite-horizon outcome *is* the only well-posed classification the system admits. Consequences: basin fractions, ML labels, and quad impurity are all "at horizon `T`"; cross-horizon comparison must account for `T` (the sim key carries it, so a single dataset is automatically horizon-consistent). The UI may render `bounded` as "bounded over the selected horizon." The term stays `bounded` (literature-standard — Lehto et al. etc.); it is not renamed to `survived_horizon`, which would fix nothing (the same might-escape-later caveat applies) while breaking legibility with the field.

**`detail` failure enum — `decode_failed` is DECODER-ONLY (a valid t=0 terminal is NOT a decode failure):**
- **`sim_failed`** (state 4, numerical breakdown *during integration*): `0` = NaN in state; `1` = Inf/overflow in state; `2` = non-finite derived quantity (energy/force blew up); `3` = reserved.
- **`decode_failed`** (state 5, **the chart/decoder could not produce a valid physical IC** — nothing to do with dynamics): `0` = non-finite decode output; `1` = degenerate configuration (e.g. exact zero-separation collinear); `2` = invalid mass construction (a mass → 0, mass-simplex boundary); `3` = other/reserved.
- **Body and pair ids (R-22).** Bodies are 0-based (`0, 1, 2`), as the decode is. **Pair id `k` names the side opposite body `k`:**
  pair 0 = bodies (1, 2), pair 1 = (2, 0), pair 2 = (0, 1). Every consumer of `detail` (collision) and `dmin_pair` uses this map.
- **escape / collision** use `detail` as body id / pair id, and `3` means all three (triple ejection / triple collision, pending change 7). The old `3` = invalid sentinel is dropped. `detail` is written in the same operation as `state` and is meaningful only for `state ∈ {escape, collision, sim_failed, decode_failed}`, so an unwritten `detail` cannot occur without a wrong `state`, which the state field's own gating already catches. (`dmin_pair` keeps its own `3` = unset.)

> **A valid t=0 terminal is a real outcome, NOT a decode failure.** If a validly-decoded IC *begins* inside `r_coll`, that is a **collision outcome at step 0** (`state=collision`, `detail=pair`, `t_end_step=0`) — the decoder *succeeded*; the state is simply already-collided. Escape has no step-0 case: its settling test needs a window of history (R-29), so an IC that is already escaping is classified when its window completes (RQ-19). These must NOT be folded into `decode_failed` — doing so would undercount the collision/escape basins and inflate the failure diagnostics. `decode_failed` is reserved strictly for the decoder failing to produce a valid physical IC.

**Drift latches are ABSOLUTE maxima:** `dE_max = max_t |ΔE(t)|`, `dLz_max = max_t |ΔL_z(t)|` (in `packed_b`, f16 — §1, held in f32 during the march). `d_min = min_t |separation|` (in `packed_a` high half). All monotone latches over the whole trajectory.

> **Why `total_substeps` is an EXACT u32, not the log proxy: resumability.** A log-compressed accumulator **cannot be incrementally updated**: `⌊log₂ Σ⌋ = 10` means `Σ ∈ [1024, 2047]` — the exact value is lost, so on a **cached-state resume** (a quad continuing its march from cached `SimState` at `t_cached`, which lockstep does) the next `⌊log₂(Σ + ΔΣ)⌋` cannot be computed from the stored exponent. Storing the **exact** running sum (in the otherwise-dead 4 B tail padding — free) makes it resumable. Proxy derived at read via `countLeadingZeros` (§6). **Dispatch guarantee:** `horizon_steps × N_max ≤ 2³²−1` (joint constraint, asserted at dispatch) so the counter cannot overflow.

### `packed_b` (u32)
```
 bit  31 ··················· 16 | 15 ··················· 0
      └──── dLz_max : f16 ─────┘ | └──── dE_max : f16 ────┘
```
`dE_max = unpack2x16float(packed_b).x`, `dLz_max = unpack2x16float(packed_b).y`.

### `times` (u32)
```
 bit  31 ··················· 16 | 15 ··················· 0
      └── t_dmin_step : u16 ───┘ | └── t_end_step : u16 ──┘
```
**Exact unsigned 16-bit macro-step indices** — the *only* format (no Q0.16 fallback; single meaning per bit pattern so cache/export decode never depends on external mode metadata).

**`t_end_step` normative meaning = `step_count`: the number of COMPLETED macro-steps, at all times.** This resolves the running-sample ambiguity — the field is not "only meaningful at termination":
- **running:** current completed-step count (advances each step);
- **terminal:** the count latched at terminalisation (stops advancing);
- **initial:** 0.
So Welford `n` (§4), current elapsed time (`step_count · dt_macro`), resume, and termination all read this one self-contained field — consistent with the `f(IC, sim key, t)` lifecycle (the payload carries its own clock, not depending on an external playhead). The bit name stays `t_end_step` for the binary format; the meaning is "completed-step count, latched at termination."

`t_dmin_step` = absolute macro-step index of closest approach (the old `t_dmin_frac`-needing-`t_end` form is gone — undefined mid-march). Display fraction derived with the `horizon_steps` uniform: `f32(t_end_step)/f32(horizon_steps)`. Exact indices → exact CPU/GPU **binary** parity (no rounding contract). `total_steps` redundant (`step_count` at termination *is* it).

> **Enforced dispatch invariant: `horizon_steps = ⌈T/dt_macro⌉ ≤ 65535`.** Hard-asserted at dispatch — **not** a soft fallback. Long integrations that would exceed it must use a **coarser `dt_macro`**, **multiple march epochs**, or a future widened layout — the binary format stays single-meaning. (Schedules can reach ~2×10⁵ macro-steps, so this genuinely constrains `(T, dt_macro)` and the assertion must fire on violation.)

---

## 3. The word buffer — `free_group_word`

Lives in the separate cold buffer (§0), one `vec4<u32>` per sample. Records **branch-cut crossings** (topological), not physical encounters — see terminology note below.

**Mixed-radix constraint packing — 76 symbols in 121 bits.** A symbol is one of the four F₂ generators `a/A/b/B`. A freely-reduced word can never have a symbol followed by its inverse (`aA/Aa/bB/Bb` cancel on append), so given the previous symbol only **3** of 4 continuations are legal → `log₂3 ≈ 1.585` bits/symbol. Pack as a mixed-radix integer built by the **Horner recurrence** (this is the recurrence-compatible definition — the earlier `d₀ + 4(e₁ + 3(e₂+…))` form was NOT compatible with the `3W+e` append and is corrected here):
```
W₁ = d₀                          // first symbol, base-4
W_{k+1} = 3·W_k + e_k            // each subsequent symbol, base-3 digit e_k ∈ {0,1,2}
```
closed form `W = d₀·3^(ℓ−1) + Σ_{k=1}^{ℓ−1} e_k·3^(ℓ−1−k)`. Capacity: 1 + ⌊(121−2)/log₂3⌋ = **76 symbols**.

**Bit budget — 121 bits reclaimed.** Payload spans all of `x, y, z` (96 bits) plus `.w` bits 0–24 (**25 bits**) = 121 bits. Truncation is **NOT** a separate flag bit — it is encoded as an **impossible length sentinel**: `length` (`.w` bits 25–31, 7 bits) holds 0…76 for valid words; **`length = 127` means truncated**. Valid lengths use only 0…76, leaving 77…127 as sentinel space. This frees the bit that a separate `word_truncated` flag would cost, giving the full 121-bit payload and 76 symbols.

**`.w` bit map:**
| Field | Bits (of `.w`) | Width |
|---|---|---|
| payload (high 25 bits of `W`) | 0–24 | 25 |
| `length` | 25–31 | 7 (0…76 valid; **127 = truncated** sentinel) |

(Payload low bits fill `x`, then `y`, then `z`, then `.w[0:24]`.)

**Continuation encoding MUST be a reversible permutation.** For cancellation-pop to recover the *new* previous symbol in O(1), the digit must select a permutation of the symbol set such that both directions exist:
```
next        = continuation_symbol(prev, e)     // forward: prev + digit → next symbol
prev_before = predecessor_symbol(next, e)      // reverse: next + digit → the prev it came from
```
This is arrangeable while still excluding inverse transitions (assign, per `prev`, the 3 legal next-symbols to digits 0/1/2 by a fixed bijection that is invertible given `next` and `e`). **Without this property, a cancellation-pop cannot recover the new final symbol without replaying the entire remaining word** — so the reversible-permutation continuation table is a hard requirement, not a convenience. Carrying the live 2-bit `prev` gives the *current* prev; the reverse table is what recovers prev *after a pop*.

**Append — the normative rule is the LENGTH CAP, not numeric overflow** (a 77-symbol word can have small numeric `W` when early digits are zero, so `push_would_overflow` cannot define capacity — it may remain a defensive assertion). Handles the empty and one-symbol edge cases explicitly:
```
if truncated:                       // length_raw == 127
    ignore later crossings          //   (word-derived quantities are already invalid — see below)
else if length == 0:                // first symbol
    W = symbol_code(s); prev = s; length = 1
else if s == inverse(prev):         // free reduction: pop
    if length == 1:
        W = 0; prev = INVALID; length = 0
    else:
        e_last = W mod 3
        W      = W div 3
        prev   = predecessor_symbol(prev, e_last)   // O(1) via reverse table (REQUIRES reversible perm)
        length -= 1
else if length == 76:               // AT CAPACITY — the normative truncation trigger
    length_raw = 127                //   sentinel; stop tracking
else:                               // push
    e = continuation_index(prev, s)
    W = 3·W + e; prev = s; length += 1
```
Multiply-add (`3W+e`) push / div-mod pop. Detection of a crossing runs per accepted substep (see the branch-cut adequacy note); the multiword arithmetic and word-buffer mutation occur **only when a crossing is detected**.

> **`last_symbol` cache (SimState `packed_a` bits 8–9, §2).** The march's live `prev` is a kernel-local — it evaporates when the kernel returns, so the fragment side (which only reads stored buffers) cannot see it, and decoding the final symbol out of the stored `W` is O(length). To give the frag O(1) access (for the animated last-symbol debug view — a per-pixel debug-catalogue field), the loop mirrors `prev` into the descriptor **every time `prev` changes** — after the push (`prev = s`) and after the cancellation-pop (`prev = predecessor_symbol(...)`): `packed_a = set_last_symbol(packed_a, prev)` (§6). This is one op on an already-resident word, fires only on a crossing, and leaves the append O(1) per crossing. On the empty-word case (`length = 0`, pop-to-empty) there is no last symbol; the field is not read (validity gates on `length`, §2), so it may hold any value. **`last_symbol` is a redundant cache, never a source of truth** — the sidecar `W` is authoritative; the two must be written coherently (same `(quad,sample,copy)` slot, same crossing) exactly as SimState[i] and word[i] must never desynchronise (§0). After truncation the loop stops appending, so `last_symbol` freezes at the cap value and is invalid alongside every other word-derived read.

> **Once truncated, word-derived quantities are INVALID.** After the cap, later crossings are ignored — but a later symbol *might* have cancelled a previously-stored one, and that is no longer tracked. So the retained value is a *prefix*, not the true reduced word, and reduced-crossing-count / any word derivation must be treated as invalid for truncated samples (the `state`/validity plumbing must surface this).

**Decode (at resolve/inspect — cold path):** O(length) sequential — pop the base-3 tail (`while depth: e = W mod 3; W = W div 3`), the residue is `d₀`, replay forward via `continuation_symbol`. Not random-access, but consumers read the whole word anyway.

**Small fixed shader tables:** `inverse(s)`; `continuation_index(prev,s)→{0,1,2}`; `continuation_symbol(prev,e)→s`; **`predecessor_symbol(next,e)→prev`** (the reverse table — mandatory for O(1) cancellation-pop).

**NORMATIVE continuation table (FROZEN — part of the binary format; changing it changes the meaning of every stored word).** Symbol codes `a=0, A=1, b=2, B=3`; `inverse = [1,0,3,2]`. The three digit-maps are permutations of the symbol set that each exclude the inverse of `prev` (verified: no continuation equals `inverse(prev)`; each digit is a permutation ⇒ `predecessor` is well-defined; the 3 digits cover exactly the 3 legal continuations):

| prev | digit 0 | digit 1 | digit 2 | (excluded = inverse) |
|---|---|---|---|---|
| `a` | `a` | `b` | `B` | (`A`) |
| `A` | `A` | `B` | `b` | (`a`) |
| `b` | `b` | `a` | `A` | (`B`) |
| `B` | `B` | `A` | `a` | (`b`) |

As arrays (`next = cont_symbol[digit][prev]`):
```
inverse       = [1,0,3,2]
cont_symbol[0]= [0,1,2,3]   // digit 0 = identity
cont_symbol[1]= [2,3,0,1]   // digit 1
cont_symbol[2]= [3,2,1,0]   // digit 2
```
**Each permutation is self-inverse** (an involution), so `predecessor_symbol[e] == cont_symbol[e]` — the *same* table serves both forward (`prev,digit→next`) and reverse (`next,digit→prev`), and `continuation_index` is derived by inverting `cont_symbol` (`continuation_index[prev][next]` = the digit `e` with `cont_symbol[e][prev]==next`). **The Rust kernel/host and the WGSL fragment side MUST use this identical generated table** — it is frozen in the Rust layout definition (emitted to both targets) and any change is a binary-format version bump.

**Length / truncation accessors (do NOT expose 127 as a crossing count):**
```
fn fgw_length_raw(w:vec4u)->u32     { return extractBits(w.w, 25u, 7u); }        // 0…76 valid; 127 = truncated
fn fgw_truncated(w:vec4u)->bool     { return fgw_length_raw(w) == 127u; }
fn fgw_prefix_length(w:vec4u)->u32  { return select(fgw_length_raw(w), 76u, fgw_truncated(w)); } // clamps sentinel
```

> **Terminology (physical precision).** The word is generated by **branch-cut crossing** events (the shape trajectory crossing a cut on the shape sphere), detected by sign-checking during the march — *not* by physical close-encounter events (though the two often coincide). So: "append on branch-cut crossing," and **`fgw_length` = reduced topological crossing count / reduced word length**, NOT "encounter count." Free reduction means it counts *net* crossings (a cross-and-immediately-recross cancels). The loose term "encounter count" is convenient but physically imprecise — use the topological name.

> **Crossing detection must be topologically adequate, and ordering must be chronological.** Detecting "once per macro-step" can **miss** two crossings within one macro-step, a cross-and-recross, or crossings during a rapidly-resolved close approach. **Crossing detection must run at every accepted adaptive substep** (the word-buffer *mutation* stays event-only/cold, but *detection* is substep-granular), OR the topology contract must prove each tested segment holds ≤1 crossing. **Simultaneous crossings must be ordered chronologically, not by fixed cut-ID:**
> 1. For each cut whose signed-distance endpoints change sign over the segment, estimate the crossing fraction `τ = d₀ / (d₀ − d₁)`.
> 2. **Sort crossings by increasing `τ`** (geometrically-meaningful chronological order within the segment).
> 3. Use a **fixed cut-ID priority only as a tie-break** when `τ` values are equal within a defined tolerance.
>
> A fixed priority alone is deterministic but does *not* preserve the true crossing order near the cuts' shared endpoint, which would corrupt the word. The `τ`-sort gives the right order *and* a deterministic fallback.
> **Endpoint-exactly-on-a-cut: half-open sign convention** (fixed, to avoid duplicate/missed crossings): count a transition as a crossing iff **`negative → non-negative`** (i.e. `d < 0` then `d ≥ 0`); the reverse (`non-negative → negative`) is the other direction — the convention must be fixed and identical CPU/GPU. Word *content* is only as deterministic as these detection branches (Tier-L/Tier-B, parity contract): binary word parity is tested for trajectories away from the numerical-ambiguity band, or under a fully-specified quantised comparison. Specify the `τ`-sort, tolerance, and sign convention in the integrator contract.

**Regime justification (why the extra compute is plausibly cheap — a hypothesis, not a settled fact):** the split is motivated by reducing persistent memory and avoiding loading unused symbolic data on the hot path — true regardless of the bottleneck. The stronger claim that the extra mixed-radix arithmetic is *free* rests on the march being bandwidth-bound, which **depends on dispatch granularity and is unconfirmed**: if one invocation runs many substeps with state held in registers and writes once, storage traffic is per-*dispatch* not per-substep, and the integration may be arithmetic-, transcendental-, occupancy-, or register-pressure-bound instead. Either way the mixed-radix cost is off the hot path (append per-crossing, decode at resolve/inspect). **Phrase as: expected to reduce storage traffic and working-set pressure; confirm the actual bottleneck and any register spilling by measurement.**

---

## 4. Welford diffusion (streaming regression)

Slope of spread `y` on time `t` (the diffusion coefficient), computed as **centered co-moments** — stable, no catastrophic cancellation (unlike raw moments `Σt,Σt²,Σy,Σty,Σy²` whose slope `(nΣty−ΣtΣy)/(nΣt²−(Σt)²)` subtracts large near-equal products).

**Indexing convention (fixed, normative):** `step_count = 0` at the initial state; sample `y` **after each completed macro-step**; sample times are `h, 2h, …, nh` where `h = dt_macro`; **`n = step_count`** (the count *after* completing the current step).

**The time-only moments are DERIVED, not a mutable shared global.** An earlier draft had one invocation update shared `n, mean_t, C_tt` and broadcast to all samples per macro-step — **impossible in a GPU dispatch** (synchronisation is workgroup-scoped; no dispatch-wide barrier to publish a global mid-dispatch — the same on Rust → SPIR-V as it was on WGSL). No mutable global is needed: for uniform sampling the time-only quantities are **closed-form**:
```
n       = step_count                          // from the playhead (t_end_step for a latched sample)
mean_t  = (n+1)·h / 2                          // mean of t_1..t_n = h,2h,…,nh
C_tt(n) = h² · n(n²−1) / 12                    // Σ(t_i − mean_t)²  — independent of start offset
```

**Per-sample update — the covariance term uses the deviation from the OLD (pre-insertion) time mean**, `δ_t = t_n − mean_{n−1}`, which for uniform sampling is **`δ_t = 0.5·n·h`** (NOT `t_n − mean_n`; using the post-insertion mean gives a systematically wrong `C_ty`):
```
// PER-SAMPLE, after completing the step that makes the count = step_count (= n):
let n_f     = f32(step_count);        // count AFTER this step
let delta_t = 0.5 * n_f * dt_macro;   // t_n − mean_{n−1}  (OLD-mean deviation — the Welford cross-term)
let delta_y = y - state.mean_y;
state.mean_y += delta_y / n_f;
state.C_ty   += delta_t * (y - state.mean_y);   // OLD time-dev × NEW y-dev

// READ-TIME (any playhead, or terminal):
// C_tt(n) = h²·n(n²−1)/12 ; slope = C_ty / C_tt(n)
```
Per-sample state: **`mean_y, C_ty`** (2 × f32, in `SimState`). No shared mutable state. Must stay f32.

> **`n < 2` guard.** `C_tt(0) = C_tt(1) = 0` (zero or one time-point has no time-variance). The read accessor **must return an invalid/sentinel slope, not divide by zero** — e.g. `slope_valid = (n ≥ 2)`, and a NaN/sentinel or a `diffusion = −1` marker (payload sentinel convention) when `n < 2`.

> **Terminal-sample subtlety.** A latched (terminated) sample stops at *its* terminal step, so its denominator uses **its own `n = t_end_step`** (exact, §2), not the current playhead — handled by construction since `n` is derived per-sample from `t_end_step`, and `C_tt` is evaluated at *that* `n`.

> **`(mean_t, C_tt)` for a non-uniform schedule** → a small read-only **precomputed prefix table** indexed by `n`, replacing the closed form. Still no barrier, still no mutable global.

---

## 5. Derived — computed at read, NOT stored

Everything that is a function of stored state is a shader helper, zero storage:

> **This derive-at-read design also gives the shader interface tier-uniformity for free (lowering Part 3a).** Because `ftle`, `ensemble_spread`, and the like are *computed* from stored state rather than stored, their "presence" is just whether the accessor computes a value or returns a **NaN sentinel** — which costs zero bytes. So the read-side `SimState` type is **fixed across all tiers** (built-in and custom shaders read `sample.ftle` by plain field access at every tier); a tier that bakes the *expensive state* out (the shadow trajectory, the ensemble copies, the word buffer) makes the corresponding read-side field return NaN, with a baked `has_<feature>` const for authors who want to branch. No struct-shape change, no memory inflation, no getter pattern. The memory-motivated stored-vs-derived split is what makes the interface uniform.

| Derived quantity | From | Note |
|---|---|---|
| `ftle` | `S_final / (step_count · dt_macro)` — see FTLE note | **NOT simply `S/t`**: `S` only contains growth through the last *completed* renormalisation, so at read the **partial interval must be finalised** first. Validity conditional (§6 `ftle_valid`) |
| `diffusion_slope` | `C_ty / C_tt(n)` | `C_tt(n)=h²n(n²−1)/12`. **Invalid for `n < 2`** (`C_tt=0`) → sentinel slope, never divide-by-zero (§4, §6) |
| `orbit_count` | `⌊|theta| / 2π⌋` | winding count |
| `retrograde` | `theta < 0` | winding sense |
| current drift `ΔE` | `H(r,p) − E_0` | cancellation-tolerant (feeds thresholds/display); threshold uses live f32 |
| current drift `ΔLz` | `L_z(r,p) − Lz_0` | ditto |
| reduced crossing count | `fgw_reduced_length` — **valid only if `fgw_reduced_length_valid`** | **net branch-cut crossings** (free reduction); **INVALID for truncated samples** (later cancellations untracked after the cap — §3). `fgw_retained_prefix_length` (=76 when truncated) is a debug/export quantity, NOT the reduced count; never expose `127` |
| `total_substeps_log2` | `total_substeps` (exact u32) via `countLeadingZeros` | complexity proxy derived at read, not stored (§2) |
| `n` (shape) | `shape(r)` or `shape(r, masses)` | current shape-sphere point (`r` mass-weighted, or pass masses); NOT `shape(r,p)` |
| current substep count | live read off the march | animated integrator-effort heatmap (Part-5 live view) |

> **FTLE denominator — finalise the partial renormalisation interval.** The Benettin sum `S` accumulates `log(δ/δ₀)` **only at renormalisation boundaries**. If renorm occurs every `M_renorm` steps and a sample reads/terminates at a step that is *not* a boundary (renorm every 16, terminate at step 30), `S` contains growth only through step 16 — dividing by the full elapsed `30·h` gives a systematically **too-small** FTLE. The contract **finalises the partial interval at read/terminalisation**:
> ```
> S_final = S + log(δ_current / δ₀)              // fold in the unfinished interval's growth
> ftle    = S_final / (step_count · dt_macro)    // divide by FULL elapsed time
> ```
> (Alternative — ignore the partial interval and divide by `N_renorm · M_renorm · h` — is rejected: it discards real growth and makes the denominator not simply `step_count·dt`.) `completed_renorms` need not be stored under a uniform schedule: `completed_renorms = step_count / renorm_interval_steps` (derived). `ftle_valid` requires `completed_renorms > 0` (§6).

> **`enc_01/02/12` and `dominant_pair` — MOVED to a separate symbolic-dynamics contract (`principia_symbolic_dynamics_contract.md`), no longer a payload blocker.** These are **not** an authoritative derivation and do not belong in the storage-layout spec: a two-generator (`a/b`) word does not yield three pair-tallies by a symbol histogram — attribution requires the generator↔cut convention, the punctured-sphere relation, and a deterministic third-pair algorithm. That is a topological-dynamics problem, not a payload-bits problem. The payload stores the word (§3); *interpreting* it into pair tallies is specified in the symbolic-dynamics contract. (`dmin_pair` is a *stored* latched fact and is unaffected.)

**Removed from storage entirely** (were eager-era fields): `arc_length_n` (weak/redundant); peak substep (`saturated` + live read replace it); `ftle_valid` (removed as a stored bit; validity is a DERIVED helper — conditional on tier/state/n/renorms, NOT unconditionally true); `energy_drift`/`Lz_drift` current (derived); `orbit_count`/`retrograde`/`encounter_count`/per-pair tallies/`dominant_pair` (derived); ensemble spread (a resolve-stage footprint quantity, not per-sample); the entire `trajectory_stats` u32 (fields derived or moved); `timeout` state (merged into `bounded` — reaching horizon without escape/collision IS bounded).

---

## 6. Accessors (illustrative of generated output; source is the Rust layout definition — emitted as Rust for the kernel/host and WGSL for the fragment side)

```wgsl
// sample_descriptor (read from packed_a low half — bits 0–9 used; 10–15 reserved)
fn sd_state(w:u32)->u32               { return extractBits(w, 0u, 3u); }
fn sd_detail(w:u32)->u32              { return extractBits(w, 3u, 2u); }
fn sd_saturated(w:u32)->bool          { return extractBits(w, 5u, 1u)==1u; }
fn sd_dmin_pair(w:u32)->u32           { return extractBits(w, 6u, 2u); }

// last_symbol — packed_a bits 8–9; cached final word symbol for O(1) frag read (codes a=0,A=1,b=2,B=3).
// No in-band "none": meaningful iff length>=1 && length!=127 (empty → no symbol; truncated → invalid).
// `len` is fgw_length_raw(word_buffer[i]) from the SIDECAR (§3) — validity is gated on the word, not packed_a.
fn sd_last_symbol(w:u32)->u32          { return extractBits(w, 8u, 2u); }
fn sd_last_symbol_valid(len:u32)->bool { return len >= 1u && len != 127u; }
// SETTER — kernel-side, mirrors `prev` into the descriptor on every append (§3). Preserves bits 0–7 and d_min (16–31).
fn sd_set_last_symbol(pa:u32, sym:u32)->u32 { return insertBits(pa, sym, 8u, 2u); }

// State predicates — "finished" (stop marching) is DISTINCT from "resolved outcome" (a real result).
// sim_failed/decode_failed are FINISHED but NOT resolved outcomes. NEVER gate re-dispatch on
// is_resolved_outcome — a failed sample would be re-marched forever. Use is_finished for "stop marching".
fn sd_is_resolved_outcome(w:u32)->bool { return sd_state(w) <= 2u; }   // escape/bounded/collision
fn sd_is_running(w:u32)->bool          { return sd_state(w) == 3u; }
fn sd_is_failed(w:u32)->bool           { return sd_state(w) >= 4u; }   // sim_failed/decode_failed (+reserved 6/7)
fn sd_is_finished(w:u32)->bool         { return !sd_is_running(w); }   // scheduler: stop marching iff finished

// complexity proxy — DERIVED from the exact total_substeps u32 (NOT a descriptor field)
fn total_substeps_log2(total:u32)->u32 { return select(0u, 31u - countLeadingZeros(max(total,1u)), total > 1u); } // max() avoids unsigned underflow in the unused branch

// f16-packed scalars (unpack2x16float → vec2: .x low 16, .y high 16)
fn pa_d_min(pa:u32)->f32              { return unpack2x16float(pa).y; }
fn pb_dE_max(pb:u32)->f32             { return unpack2x16float(pb).x; }
fn pb_dLz_max(pb:u32)->f32            { return unpack2x16float(pb).y; }

// times — EXACT u16 step indices; fraction derived with a horizon_steps uniform (single format, no Q0.16)
fn tm_t_end_step(w:u32)->u32          { return extractBits(w,  0u, 16u); }
fn tm_t_dmin_step(w:u32)->u32         { return extractBits(w, 16u, 16u); }
fn tm_t_end_fraction(w:u32, horizon_steps:u32)->f32  { return select(0.0, f32(tm_t_end_step(w))/f32(horizon_steps), horizon_steps > 0u); }  // guard /0 in generic tooling
fn tm_t_dmin_fraction(w:u32, horizon_steps:u32)->f32 { return select(0.0, f32(tm_t_dmin_step(w))/f32(horizon_steps), horizon_steps > 0u); }

// validity — DERIVED helpers, no stored bit (ftle_valid is NOT unconditionally true)
fn ftle_valid(state:u32, ftle_tier_on:bool, n:u32, benettin_renorms:u32)->bool {
    return ftle_tier_on && !sd_is_failed(state) && n > 0u && benettin_renorms > 0u;
}
fn diffusion_slope_valid(n:u32)->bool { return n >= 2u; }   // C_tt(n)=0 for n<2 → sentinel slope, no divide-by-zero

// word (from word_buffer[i], NOT SimState)
fn fgw_length_raw(w:vec4u)->u32       { return extractBits(w.w, 25u, 7u); }        // 0…76 valid; 127 = truncated
fn fgw_truncated(w:vec4u)->bool       { return fgw_length_raw(w) == 127u; }
// REDUCED crossing count — valid ONLY when not truncated (later cancellations untracked after cap)
fn fgw_reduced_length_valid(w:vec4u)->bool { return !fgw_truncated(w); }
fn fgw_reduced_length(w:vec4u)->u32   { return fgw_length_raw(w); }   // use only when fgw_reduced_length_valid
// RETAINED prefix length — for debug/export tooling only; NOT the reduced crossing count
fn fgw_retained_prefix_length(w:vec4u)->u32 { return select(fgw_length_raw(w), 76u, fgw_truncated(w)); } // clamps 127
// fgw_decode(w) → symbols: sequential base-3 unpack (Horner) + continuation_symbol replay
```

**Kernel-side write (authoritative — Rust; the WGSL `sd_set_last_symbol` above is illustrative parity).** The `last_symbol` cache is written by the march, not the fragment side; the setter lives in the Rust layout definition alongside the pack routines:
```rust
#[inline]
fn set_last_symbol(packed_a: u32, sym: u32) -> u32 {
    (packed_a & !(0b11 << 8)) | ((sym & 0b11) << 8)   // clears bits 8–9, writes sym; preserves descriptor 0–7 and d_min 16–31
}
// in the append loop, wherever `prev` changes (push tail + cancellation-pop):
//   packed_a = set_last_symbol(packed_a, prev);
```

**WGSL traps:** use the **u32** overload of `extractBits` (i32 sign-extends); no f64; f16 pairs via `pack2x16float`/`unpack2x16float` (**core WGSL — no `shader-f16` needed**; clamp/canonicalise before packing, §1); `SimState` is 8-byte aligned (word moved out) — pack the live block in vec2 groupings. **Two stored sizes** `SimStateBase`/`SimStateFTLE` (the 48 B shadow — monomorphised Rust variants, §1); the fragment READ side is **unified** (`has_ftle` const + NaN sentinel, §5), so it needs no runtime member-omission.

---

## 7. Memory

**Storage multiplier is `(E+1)`, NOT `2(E+1)`.** Critical distinction:
- **The *trajectory / compute* count is `(E+1)(1 + 𝟙_FTLE)`** — per nominal sample there are `(E+1)` samples (base + E ensemble copies), each running its own trajectory *plus* a Benettin shadow **only when FTLE is enabled**. So integrations/pixel = `2(E+1)` with FTLE on, `(E+1)` with FTLE off. This is the number for the compute/probe estimate (quality-device note).
- **`(E+1)` is the *storage* multiplier** (both tiers) — there are `(E+1)` `SimState`s per nominal sample, and **each `SimState` already contains its shadow** (the `r_sh, p_sh` fields, part of the 144 B). Multiplying storage by `2(E+1)` would **double-count the shadow** (once in the struct size, once in the multiplier). Memory = `bytes × (E+1) × live_pixels`.

**No double-buffering.** The march updates state **in place** each step (temporal note) — samples are independent (no stencil/neighbour hazard: each sample's force loop couples only its own 3 bodies), so read-modify-write of a sample's own slot is safe, no ping-pong copy.

> **Recomputed at 144 / 96 B (pending change 9).** The closure field (§1) moved `SimStateFTLE` 136 → 144
> (**+5.9%**) and `SimStateBase` 88 → 96 (**+9.1%**). Both stay 8-byte aligned, so nothing repacks. The
> figures below are at the new widths; at the old widths they were 1.261 / 0.863 / 0.431 / 5.043 / 1.725 GB.

Per-sample: hot `SimState` **144 B (FTLE-on) / 96 B (FTLE-off)** effective + word buffer **16 B** (when symbolic features active; separable — also `×(E+1)`, no shadow, appended in place). Scales as `bytes × (E+1) × live_pixels`. Exact figures (decimal GB, 1920×1080 / 3840×2160):

| Config (at the stated *render* resolution) | Hot | +Word | Total |
|---|---|---|---|
| 1080p E=3 FTLE-on (High-like) | 1.194 GB | 0.133 | **1.327 GB** |
| 1080p E=3 FTLE-off | 0.796 GB | 0.133 | **0.929 GB** |
| 1080p E=1 FTLE-off | 0.398 GB | 0.066 | **0.464 GB** |
| 4K E=3 FTLE-on (High-like) | 4.778 GB | 0.531 | **5.308 GB** |
| 4K E=1 FTLE-off | 1.593 GB | 0.265 | **1.858 GB** |

Span ~88 MB (phone: FTLE-off E=0 720p, hot only) to ~5.3 GB (4K FTLE-on E=3), managed by the quality/device controller. **These rows are payload-only at the stated render resolution** — tier totals including render targets and each tier's `render_scale` are in `principia_memory_tiers.md` §4 (whose High@4K payload split, 5.04 GB, was computed at the old 136 B width and now matches the old E=3 figure, not the 5.31 GB here — open-questions). Unified-memory devices (Apple Silicon) get a different budget heuristic than discrete-VRAM (build-time note).

> **The payload budget is NOT the process budget.** These figures are the *logical payload only*. They exclude render targets, quad metadata, staging/readback buffers, transient allocations during export or resize, shader/driver overhead, and the browser + OS. So "1080p E=4 FTLE-on fits in 1.7 GB" (1.6 GB at the old width) means the *payload* fits — the full process budget on, e.g., a 16 GB unified-memory machine is viable but **needs measurement**, not assumed-comfortable. Do not claim large headroom from the payload figure alone.

**WebGPU allocation — logical buffers are SHARDED.** WebGPU guaranteed defaults: **`maxStorageBufferBindingSize` = 128 MiB, `maxBufferSize` = 256 MiB** (adapters may expose larger, but the baseline must be assumed for reach). So "the `SimState` buffer" and "the word buffer" are **logical** entities implemented as *many* physical quad/chunk buffers — a multi-GB payload cannot be one `GPUBuffer` and cannot be bound in one binding, regardless of available memory. Sharding is per-quad (natural — quads are the compute/eviction unit already); the allocator hands out chunk buffers and the scheduler tracks which quad lives in which chunk. Hard constraint, not an optimisation.

---

## 8. Build-time settles (measure / specify once running)

- **`enc_XY`/`dominant_pair` attribution algorithm (BLOCKING for those quantities)** — specify the generator↔cut convention, punctured-sphere relation, and third-pair attribution before per-pair tallies or `dominant_pair` are deterministic (§5).
- **`horizon_steps ≤ 65535` — ENFORCED** (single time format, no fallback; §2). Assert at dispatch; long integrations use coarser `dt_macro`/epochs. Also assert the joint `horizon_steps × N_max ≤ 2³²−1` for the `total_substeps` counter.
- **Bottleneck confirmation** — the march may be bandwidth-bound *or* arithmetic/occupancy/register-pressure-bound depending on dispatch granularity (per-substep vs per-dispatch storage traffic). Confirm by measurement, and check for register spilling; the mixed-radix "free compute" argument is contingent on this (§3).
- Word truncation rate (drives whether 76 symbols suffices; mixed-radix is the alternative to widening).
- Crossing distribution (whether symbolic-spread `S_word` is additive to outcome-impurity).
- Displacement-vs-absolute f32 shadow: test against known-Lyapunov periodic orbits; switch to displacement only if FTLE accuracy meaningfully improves (accuracy question, not memory — f32 retained either way; shadow-precision experiment).
- Quality controller thresholds/ladder; device-characterisation budget heuristics (unified vs discrete memory).
- f16 latch semantics (a) vs (b) — confirm the local-f32-pack-at-persist path (a) is what's implemented (§1).
- **Branch-cut crossing detection at every accepted substep** (not per-macro-step) OR a proof that each tested segment holds ≤1 crossing (§3) — required for topological adequacy.
- **Deterministic tie-break for simultaneous both-cut crossings** (§3) — specify in the integrator contract; without it CPU/GPU word parity depends on FP test order.
- **Failure `detail` enum** — the sim_failed/decode_failed category codes (§2) are drawn from the integrator contract's actual failure modes; confirm they match the contract's enumeration when it's finalised.

---

> ### ⚠ ESCAPE CRITERION — LANDED (was: superseded). See `principia_01_pitfalls.md` §2
> The old test (`E_rel > 0 ∧ receding`, optionally `∧ d > r_esc`) **fires on transients**: measured
> 0 of 895 escapes still unbound 8 boundaries later. It is replaced by
>
> ```
> ESCAPE  ⟺  |Δn̂| over a window < tau    AND    E_rel > 0
> ```
>
> **100% precision, 96.3% recall**, against 97.9% for the old test. `receding` and `d > r_esc` are
> **redundant** once both hold (identical to the digit), so three tuned constants are eliminated.
> `tau` sits in a **383× gap** and is not tuned *(to re-measure, R-29)*. Fires at `t≈10` rather than `t≈1.5` — **late rather
> than wrong**, which is correct for a *stored* `t_end`.
>
> `E_rel`, the window and the escaper are defined by R-29 (integrator contract Part 7). The precision, recall and gap above
> predate R-29's `E_rel` and are to re-validate.
>
> **And escape must not terminate integration until §2.4's three checks pass.** Freezing a
> trajectory whose displayed quantity is still moving is what produced the patchwork artefact
> (§1). Collision stays terminal — it is a singularity, not a heuristic.


---

## VERTICAL-SLICE ADDITIONS

**`footprint_undetermined` and `Decision::Undetermined` — because a truncated state is a good number.**

A quad with **every footprint budget-exhausted and all 512 copies flagged unusable** reads
`ensemble_spread` **finite on every one**. Measured against a healthy quad in the same region:

| quad | budget | non-finite | `spread_median` | `error_ratio_max` |
|---|---|---|---|---|
| near-field, production | 0/64 | 0/512 | 2.584e-3 | 1.0020 |
| near-field, **starved** | 64/64 | 512/512 | **4.580e-4** | **1.0000** |

**The starved quad reads 5.6× SMALLER — i.e. better resolved — and `error_ratio_max` reads exactly its
converged 1.0000**, because every copy stopped at the same early point and so agrees perfectly. **A
trust metric reporting maximum confidence when there is nothing to be confident about.**

**And it is not a correctable bias**: near-field reads better, `deep interior` worse. It has to be
*detected*, not calibrated out. Hence keying on the real conditions rather than on the statistic.

**`f64::max` ignores NaN**, so `ensemble_spread` silently returns the **event** spread when the shape
spread is undetermined — 11 such footprints in `deep interior` under the old kernel. Same shape as the
`d_min` discriminator poisoned by its own subject: **the measurement was correct and the column chosen
could not see it.**

**Closure** (`closure_min: f32`, `closure_step: u16`) is specified above. It is related to the escape criterion's settling
test but is not the same quantity: `closure_min` is the running minimum of `|n̂(t) − n̂(0)|` against the *initial* shape,
while the settling test is `|Δn̂|` over a 0.4-time-unit window (R-29), which needs `n̂` from one window earlier. Closure → 0
and spectral entropy → 0 are one statement. It is
also the input to the **sonification** channel (`principia_scratchpad_pointer_channels.md`).
