# Drill-down — the Generation Root (layout table + link registry)

*Second drill-down. The roots of the build DAG: the two single-sources-of-truth from which pack, unpack, export, the debug catalogue, and the compactification link functions are generated. This document **is** the ledger — when implementation starts, §3 transcribes directly into the **Rust layout definition** (the single codegen source under the substrate — `principia_spike_brief.md`, render Part 5). No codegen mechanics here (implementation); only the authoritative content and the properties any generator must satisfy.*

---

## 1. What it is

Not a rung and not quite a ring: the **roots**. Two tables — the **layout table** (every field of every struct, with presentation metadata) and the **link registry** (every compactification function, with its inverse and measure) — from which four artefact families are generated, **to two language targets** (the substrate split, lowering Part 2): **Rust** pack/unpack for the **kernel** (the kernel is Rust → SPIR-V, so it writes and re-reads the payload in Rust — *not* WGSL) and the **Rust** host export decoder; **WGSL** unpack accessors for the **fragment** side (read-only, to colour `SimState`) and the **WGSL** debug catalogue. Seam 13's law is unchanged: **one source generates all four; hand-duplication is forbidden; a field without metadata fails generation loudly** — the only change is that the one source now emits Rust *and* WGSL.

---

## 2. Consolidated contract

From the **render contract**: physical structs are bit-packed; the logical view is the accessor layer (no unpack-copy pass); the catalogue is generated and exhaustive by construction; sentinels, never NaN, **in storage** (the read-side *accessor* NaN for tier-absent features is a different, compatible mechanism — lowering Part 3a; validity is answered by `has_<feature>` consts and descriptor predicates, never `isnan`).

From the **chart contract (Part 2.5)**: every link is constraint-preserving, invertible with a conditioned inverse, C¹, and **carries its log-det Jacobian plus an over/under-sampling note**; link ids live in provenance; findings must survive link swaps.

From the **lowering contract** (Part 2, the substrate split): the generated artefacts split by target — link functions and the kernel's pack/unpack are **generated Rust** (compute side, monomorphised, no assembler); the fragment unpack accessors and debug catalogue are **generated WGSL** flowing through the fragment assembler alongside authored colour code; link selection is baked per block.

From the **caching/integrator contracts**: the **payload compatibility signature** includes the payload schema version — a layout or kernel change must invalidate every cached payload. **The schema version is a content hash of the canonicalised §3 ledger's columns that decide the payload's layout or the kernel's output, not a hand-bumped integer (R-36, R-545):** `scale`, and any render-only column a ruling names, leaves the hash, so a ledger scale change re-styles the view with zero recompute (dd_colouring seam 5; RQ-297). *Was:* "a layout change must invalidate every cached payload. **The schema version is a content hash of the canonicalised §3 ledger, not a hand-bumped integer (R-36).**"

From the **inverse-encode contract**: link inverses are the encode path's block inverses; tolerances asserted in physical units; ε clamps are part of the registry entries.

---

## 3. The ledger (the exact content — two generators must emit identical bits)

> **The full per-sample payload is consolidated in `principia_dd_simstate_payload.md`** — the single authoritative statement of the `SimState` struct, the separate word buffer, every bit layout, the precision rule, and what is derived rather than stored. That document is what transcribes into the **Rust layout definition** (the codegen source). The tables below (§3.1–3.3a) are the field-level detail; where they and the consolidated doc could drift, the consolidated doc is canonical.

### 3.1 `sample_descriptor` (u32)

**`sample_descriptor` (u32) — authoritative bit layout (10 bits used, 0–9; `total_substeps` is a separate exact u32 in the struct, NOT a descriptor field):**

| Field | Bits | Width | Values / scale |
|---|---|---|---|
| `state` | 0–2 | 3 | **enum(6), mutually-exclusive**: 0 escape · 1 bounded · 2 collision · 3 running · 4 sim_failed (NaN/Inf *during integration*) · 5 decode_failed (**decoder could not produce a valid physical IC** — NOT a t=0 dynamical terminal; a valid IC already inside r_coll is `collision` at step 0; there is no step-0 `escape`, since the window rule needs history, so an IC already escaping is classified when its window completes — R-60). **`bounded` is FINITE-HORIZON** (neither escape nor collision within `T`, which is in the sim key) — permanent boundedness is not decidable for the 3-body problem and is not claimed; term stays `bounded` (literature-standard), not renamed. Replaces the old `class`(2) + `running`/`sim_failed`/`decode_failed` flags — all six are exclusive, so one enum. `detail` is a 4-state union (payload §2): escape → body id; collision → pair id; **sim_failed / decode_failed → failure category** (the failure states carry a diagnostic detail, not an outcome); undefined for `running`. **Substep-cap saturation is NOT a state value** — it is the separate `saturated` bit (a non-terminal confidence flag); every `state` is dynamical, annotated by trustworthiness not replaced by a numerical-limit label |
| `detail` | 3–4 | 2 | **union keyed by `state`** (payload §2): escape → body id (0-based); collision → pair id (pair `k` is the side opposite body `k`, R-22); sim_failed/decode_failed → failure category; undefined for running |
| `saturated` | 5 | 1 | sticky flag — set iff `N_sub == N_max` occurred at some macro-step (the integrator hit its per-step subdivision cap; the trajectory continued, advance-and-flag — payload §2, R-86). `N_max` is a raised tunable (integrator contract), not hardcoded, up to the substep table's maximum length and refused above it (R-502) |
| `dmin_pair` | 6–7 | 2 | categorical(3) — which pair achieved `d_min` (a latched fact, NOT derivable from the word); pair ids as `detail` (R-22) |
| `last_symbol` | 8–9 | 2 | final symbol of the free-group word, a redundant cache of the sidecar word for O(1) fragment read (frozen codes `a=0, A=1, b=2, B=3`); meaningful iff the word's `length ≥ 1 && length ≠ 127`. A deliberate, versioned assignment (payload §2, R-86) |
| *reserved* | 10–15 | 6 | headroom — reserved means reserved (flag §6). **`total_substeps_log2` is NOT here** — the log proxy (⌊log₂⌋, 0 for a total ≤ 1 — payload §6, R-86) is *derived at read* (`countLeadingZeros`) from the exact `total_substeps` u32 (a log accumulator is not resumable; payload §2). Most likely future tenant is a widened word-length field if the word ever grows |

(The descriptor uses **10 of its low-16 bits**; the high 16 bits of `packed_a` hold `d_min:f16`. Bits 0–9 are used and 10–15 reserved — payload §2.)

**Derived, NOT packed (removed from the descriptor — functions of stored state):** `orbit_count = ⌊|θ̃|/2π⌋` and `retrograde = θ̃<0` (from `theta`); `enc_01/02/12`, `dominant_pair` (topological read of the word — symbolic-dynamics contract); `ftle = S_final/(step_count·dt)` (finalise the partial renorm interval at read — payload §5, **not** simply `S/t`); `total_substeps_log2` (from the exact `total_substeps` u32 via `countLeadingZeros`); current substep count (live read off the march); current drifts (`H(r,p)−E_0`). The old `encounter_count`/`benettin_count`/`suspect_energy`/`suspect_lz`/`ftle_valid` descriptor fields are gone (derived, dropped, or — for suspect flags — folded into read-time predicates over the drift values).

**Live current-substep count — NOT a stored field.** How many substeps the *current* macro-step took, read live off the marching state each frame (the substepper already computes `N_sub`; this exposes an already-computed intermediate, like the live shape readout `n(t)`). A live-derived quantity (not latched, no time-series), colourable and animating as the playhead marches — a live heatmap of integrator effort. Belongs with the live-state readouts (render contract Part 5), not the descriptor.

**`times` (u32) — authoritative bit layout:**

| Field | Bits | Width | Values / scale |
|---|---|---|---|
| `t_end_step` | 0–15 | 16 | **EXACT** completed-macro-step count `step_count` (u16), at all times — running: advances each step; terminal: latched; initial: 0 (payload §2). Display fraction derived `/horizon_steps`. At termination it is the total macro-steps (`total_steps` redundant). **Dispatch refuses a configuration with `horizon_steps=⌈T/dt⌉ > 65535`** (R-86; single format, no Q0.16 fallback; long integrations use coarser dt/epochs) |
| `t_dmin_step` | 16–31 | 16 | **EXACT** absolute macro-step index of closest approach (u16). The old `t_dmin_frac`-needing-`t_end` form is gone — undefined mid-march under lockstep |

Exact integer indices → exact CPU/GPU parity (no rounding contract) and exact Welford prefix selection.

### 3.3 `free_group_word` (uint4) — mixed-radix packing, **in a separate buffer**

> **Architecture: the word lives in its own storage buffer, NOT inline in `SimState`.** Split by *access pattern*, not data type. Every other `SimState` field is **hot** — read/updated by the march kernel *every substep* (`r,p`; the shadow, which marches in lockstep; `S,θ̃`; Welford `mean_y,C_ty`; `E_0,Lz_0` read for the drift-max latch; `dE_max,dLz_max`; `times`; `d_min`; the descriptor). The word alone is **cold**: appended only on encounter *events* (a few dozen times over a ~200k-step march) and read only at resolve (symbolic-spread) and inspect (display) — *never* in the per-step march. Co-locating it inline wasted bandwidth two ways: (1) it made `SimState` 16-byte-aligned (the `vec4` is the sole 16-align driver), padding the struct tail with **12 dead bytes**; (2) every hot cache line the integrator loaded dragged 16 B of word it didn't need that step. **Pulling the word into a parallel buffer** (indexed identically to samples — per-copy, same sample→index map, 16 B/entry) drops `SimState` to **8-byte alignment** (largest remaining member `array<vec2>`), kills the 12 B padding (→ only 4 B residual), shrinks the **hot** per-sample footprint 160→136 B effective (FTLE-on) / 112→88 B (FTLE-off) — **152 / 104 B** with R-603's and R-605's two u32s from TASK-M3-41 on, 144 / 96 B as generated until then (payload §1, §7; *was* "**144 / 96 B** since the 8 B closure field", perf review 5662126729 of PR #293, P6) — a ~15% cut in the persistent hot-state footprint, expected to reduce storage traffic and working-set pressure (the bottleneck is unconfirmed — payload §3, §8) — and lets the word buffer be sized/tiered independently (drop word features → free the whole buffer without touching integration state). Cost: one extra buffer binding + an indexed word fetch on the *cold* paths (resolve/inspect) only. If the march is bandwidth-bound, that trades rare-path fetches for hot-path bandwidth; whether it is depends on dispatch granularity and is to be measured (payload §3). **Only the word moves** — the physics/accumulators/refs are all hot and co-accessed every step, so splitting them further would fragment co-accessed data and hurt the prefetcher; two buffers (hot `SimState` + cold word) is the sweet spot.

**76 symbols in 121 bits** (vs 58 flat), by exploiting free reduction. Truncation is encoded as a **length sentinel** (`length=127`), not a flag bit, reclaiming the bit for the full 121-bit payload (all of x,y,z + `.w[0:24]`). Per-copy (each of the **(E+1)** samples carries its own word). Lives in the **word buffer** (§3.3a), not `SimState`. **Cancellation requires a reversible-permutation continuation table** (`predecessor_symbol(next,e)`) so a pop recovers the new prev in O(1) — else it replays the whole word.

**Why denser than 2 bits/symbol.** A symbol is one of the four F₂ generators `a/A/b/B` — 2 bits *if all four are always legal*. But a **freely-reduced** word can never have a symbol immediately followed by its inverse (`aA/Aa/bB/Bb` cancel on append). So given the previous symbol, only **3** of the 4 continuations are legal → `log₂3 ≈ 1.585` bits/symbol, not 2. The "can't be the inverse of the previous" is redundant information; a flat 2-bit code wastes ~0.4 bits/symbol.

**The packing — a mixed-radix integer:**
```
W₁ = d₀ ;  W_{k+1} = 3·W_k + e_k    (Horner recurrence — compatible with the 3W+e append; the earlier d₀+4(e₁+3(e₂+…)) form was NOT and is corrected)
```
`d₀ ∈ {0,1,2,3}` = first symbol (base-4, no prior constraint); each `eₖ ∈ {0,1,2}` = *which of the 3 legal continuations* given the previous symbol (base-3). Density = log₂3 ⇒ **~76 symbols** in the 121 usable bits (128 − 7-bit length − 1 truncated). Capacity vs the old flat 2-bit/58-symbol layout: **+31%, same bytes** — directly reduces truncation on the long-lived chaotic trajectories where the word matters most.

**Append (per-encounter — a rare *event* op, NOT per-step):**
```
if s == inverse(prev):  e = W mod 3 ; W = W div 3 ; prev = predecessor_symbol(prev, e) ; length -= 1   // free reduction: pop
else if length == 76:   length_raw = 127                                                          // at capacity: truncate
else:                   W = W*3 + continuation_index(prev, s) ; prev = s ; length += 1            // push
```
The full rule, with the empty-word, pop-to-empty and already-truncated cases, is payload §3's.
A **multiply-add** (`W*3+e`) replaces the flat **shift-or** — 2–3 extra ALU ops, on a path that fires a few dozen times over a ~200k-step march, never per-step. Carry a live 2-bit `prev` symbol during the march (needed for the constraint + cancellation recovery; cheap).

**Decode (at resolve / inspect — a *cold* path):** O(length) sequential — pop the base-3 tail by depth, not by magnitude (`while depth: e = W mod 3; W = W div 3`, `length − 1` times — payload §3), the residue is `d₀`, then replay forward through the continuation table. **Not random-access** — but the consumers (symbolic-spread reduction comparing whole words, inspection display) read the *entire* word anyway, so O(length) sequential *is* their access pattern. No practical loss.

**Small fixed tables (shader constants):** `inverse(s)` (4 entries); `continuation_index(prev,s)→{0,1,2}`, 3 where `s = inverse(prev)` (R-307), and its inverse `continuation_symbol(prev,e)→s` (the 3 legal continuations per prev, fixed order); and the reverse `predecessor_symbol(next,e)→prev`, mandatory for the O(1) pop (payload §3's frozen table).

**Layout in `.w`:** payload high bits in `.w[0:24]` (25 bits — part of the 121-bit budget); `length` **7 bits** at `.w[25:31]`, values 0…76, with **`length = 127` = the truncation sentinel** (NO separate flag bit — the sentinel reclaims it; payload §3). **Truncation is the length cap:** a push onto a word already at 76 symbols sets `length_raw = 127` and stops growing (payload §3 — a 77-symbol word can have a small numeric `W`, so bit occupancy cannot define capacity).

> **Why the extra compute is plausibly cheap (a hypothesis, not settled).** The split reduces persistent memory and avoids loading unused symbolic data on the hot path — true regardless of the bottleneck. The stronger "free compute" claim rests on the march being **bandwidth-bound**, which **depends on dispatch granularity and is unconfirmed**: if one invocation runs many substeps with state in registers and writes once, storage traffic is per-*dispatch* not per-substep, and integration may be arithmetic-/occupancy-/register-pressure-bound instead. Either way the mixed-radix cost is off the hot path — **append per branch-cut crossing (rare), decode at resolve/inspect (cold)** — no per-step cost, no new divergence (the cancellation branch already exists), no hot-kernel register pressure (decode runs in the resolve/inspect kernel). **Phrase as: expected to reduce storage traffic and working-set pressure; confirm the actual bottleneck and register spilling by measurement.** (Storage traffic per sample is `(E+1)`× struct, not `2(E+1)` — the shadow is *inside* the struct, §consolidated payload.)

> **Deriving encounter counts from the word — a subtlety.** `encounter_count` and the per-pair tallies (`enc_01/02/12`, `dominant_pair`) are **derived from the word, not stored** (the word is the single source of encounter truth; storing separate tallies would be redundant — dropped). But the derivation is **not a raw symbol histogram**: a word symbol is one of the four **F₂ generators** `a/A/b/B`, which records *which branch cut was crossed and in which direction* (winding around collision points), **not** directly "which body-pair had an encounter." So:
> - `encounter_count` ≈ word length is a fair first-order proxy (each crossing is a close-approach event), but note **free reduction** removes `aA`/`Aa`/`bB`/`Bb` cancelling pairs live, so the stored length is the *reduced* word — a wind-out-and-back-immediately pass cancels and is *not* counted. That's arguably correct (no net topological encounter) but means length counts *net* topological encounters, not raw close approaches.
> - **per-pair tallies need the generator→pair geometric correspondence** — mapping branch-cut crossings (`a/b` and the implicit third via the punctured-sphere relation) to the three body-pairs, then counting. A topological reading of the word, not `count symbols == value k`. The helper must encode that mapping; it is more involved than a histogram. (`dmin_pair` is unaffected — it is a separate latched fact, not derived from the word.)
> Flagged so the derivation helpers are written correctly; "derive the tallies from the word" is right but is a *topological* derivation, not a trivial count.

### 3.3a The word buffer — a parallel cold buffer

The word lives here, not in `SimState`. Specification:

- **Layout:** one `vec4<u32>` (16 B) per sample, indexed **identically** to the `SimState` sample buffer — same `(quad, sample, copy)` → linear index map. Per-copy (each of the **(E+1)** samples has its own word — note `2(E+1)` is the *trajectory* count including shadows, but there are only `(E+1)` *samples* and thus `(E+1)` words). It is a *parallel array*: `word_buffer[i]` is the word for the sample whose state is `simstate_buffer[i]`.
- **Written:** only on encounter events by the march kernel (mixed-radix append — §3.3), a rare per-event op. Between encounters the march never touches it.
- **Read:** at resolve (symbolic-spread reduction `S_word` over a footprint's E+1 words) and at inspect (decode + display for a clicked trajectory). Never read per-step.
- **Residency & tiering:** sized `16 B × (E+1) × live_pixels` (storage multiplier `(E+1)`, NOT `2(E+1)` — the word has no shadow and is per-*sample*), sharded per-quad (WebGPU 128 MiB/binding, 256 MiB/buffer baseline forces logical buffers into physical quad-chunks), independent of the hot `SimState` allocation. Because it is only consumed at resolve/inspect, it can be tiered separately — e.g. dropped entirely when symbolic features are off (freeing the whole buffer without touching integration state), or given a different residency policy than the hot state. The quality/device controller treats it as a separable line item.
- **Lifecycle parity with `SimState`:** the word is part of the payload `f(IC, sim key, t)` — it must be **allocated, evicted, and recomputed in lockstep with its `SimState`** (caching contract). A sample and its word are one logical payload split across two physical buffers; eviction frees both, recompute rebuilds both, device-loss recovery restores both. Never let the two buffers desynchronise (a `SimState` at index `i` and a word at index `i` must always describe the same trajectory at the same playhead).
- **Alignment payoff:** removing the `vec4` from `SimState` drops the struct from 16-byte to 8-byte alignment (largest remaining member `array<vec2>`), eliminating 12 B of vec4-forced tail padding and shrinking the hot per-sample footprint 160→136 B (FTLE-on) / 112→88 B (FTLE-off); with the 8 B closure field and R-603's and R-605's two u32s it is **152 B effective (FTLE-on) / 104 B (FTLE-off)** from TASK-M3-41 on, 144 / 96 B as generated until then (payload §1, §7; *was* "with the 8 B closure field it is **144 B effective (FTLE-on) / 96 B (FTLE-off)**", perf review 5662126729 of PR #293, P6) — a ~15% cut in the persistent hot-state footprint (the bottleneck is unconfirmed — payload §3).


### 3.4 `SimState` scalars — with presentation metadata

**Lockstep semantics (temporal note, ratified): these are *evolving values at the playhead*, not completed-trajectory summaries.** On termination the sample latches and they freeze at their terminal values.

> **Dropped: `arc_length_n`** (shape-sphere cumulative path length). A weak, redundant quantity: its "trajectory richness" role is dominated by FTLE / the free-group word / winding (all more interpretable), and its only real historical use — the `arc ≥ chord` sanity bound — moved to the f64 parity/validation suite where history exists. Un-interpretable as a colour mode ("total wiggle"), no downstream computation depends on it, and it costs an accumulator ×`2(E+1)`×viewport for near-zero value. **Speculative future use:** the quantitative (fractal-dimension) work *might* want shape-space path length on the manifold — if so, re-add trivially as `Σ|Δn|`. Not now — that's over-engineering an unmeasured need.

> **Welford diffusion — the streaming update (mini-spec).** Regression of spread `y` on time `t`, slope = diffusion rate. Stable (no catastrophic cancellation) and split into shared-vs-per-sample under lockstep:
> ```
> // TIME-ONLY (common to all samples; shown as the streaming recurrence it computes — but DERIVED closed-form at read, NOT an actual mutable global; see note):
> n      += 1
> dt      = t - mean_t              // deviation from OLD mean_t
> mean_t += dt / n
> C_tt   += dt * (t - mean_t)       // old-dev × new-dev (the stable cross-term)
>
> // PER-SAMPLE (in the march; consumes shared dt, n as read-only broadcasts):
> dy      = y - mean_y
> mean_y += dy / n
> C_ty   += dt * (y - mean_y)       // shared time-dev × per-sample new-dev
>
> // READ-TIME (any playhead):
> slope = C_ty / C_tt               // = Cov(t,y)/Var(t); diffusion coefficient
> ```
> Per-sample state: **`mean_y, C_ty`** (2 × f32). The time-only moments `n, mean_t, C_tt` are **DERIVED, not a shared mutable global** — WGSL has no dispatch-wide barrier to publish one safely mid-dispatch. For uniform sampling they are closed-form: `n = step_count`, `mean_t = (n+1)h/2`, `C_tt(n) = h²·n(n²−1)/12`. The per-sample covariance update uses the **old-mean** time deviation `δ_t = 0.5·n·h` (not `t − mean_t` post-insertion, which is wrong). Slope `C_ty/C_tt(n)`, invalid for `n<2` (reads NaN by the predicate `n ≥ 2`, R-245). Must stay f32 (means/co-moments precision-sensitive). (Full detail: payload spec §4.)

| Field | Scale | Range / notes |
|---|---|---|
| `t_end_step` (→ fraction) | lin | [0, T] via `/horizon_steps` — **normative meaning = completed-step count at ALL times** (running: advances each step; terminal: latched; initial: 0 — payload §2; no sentinel needed); EXACT u16 |
| `d_min` | log | > 0 |
| `ftle` | lin | tier-gated (`ftle_valid`); reads NaN whenever `ftle_valid` is false, `step_count = 0` and no completed renorm included, no sentinel (R-253, R-254). **A *point* quantity — every sample (base and each ensemble copy) computes its own** from its Benettin shadow, so it anti-aliases under the SSAA resolve (sampling/SSAA note) |
| `energy_drift` | **diverging** (signed) | log-magnitude styling; floor `eps_E` |
| `diffusion` | lin | invalid fit (`n < 2`) reads NaN, by the predicate `n ≥ 2` (R-245) |
| `delta_E_max_abs` | log | ≥ 0 |
| `Lz_drift` | **diverging** (signed) | floor `eps_L` |
| `delta_Lz_max_abs` | log | ≥ 0 |
| `E_0` | diverging | must equal `K₀+V₀` (cross-check view) |
| `Lz_0` | diverging | invariant-chart gradient view input |
| `closure_min` | log | ≥ 0, f32; the running minimum of `|n̂(t) − n̂(0)|` once the shape has departed by `δ_dep` (R-37, payload §1) |
| `closure_step` | lin | exact u16 step index of the minimum (the period label); binary-parity surface, same convention as `t_dmin_step` |
| `theta` | **cyclic** | the unwrapped phase `θ̃`, the shape sphere's equatorial longitude accumulated in radians (dd_integrator §3.7); an angle, so its view is cyclic, period 2π, on `ramp_twilight`, not a diverging ramp (R-423); `orbit_count` and `retrograde` derive from it |

> Metadata nuance the catalogue generator needs: **the two drift fields are signed** → *diverging* colour scale, not sequential-log. (Finding, §6.)

### 3.5 The live-state block (replaces the checkpoint array — lockstep, ratified)

**There is no stored trajectory.** The shape vector `n` is *derived from the current state each step* (Montgomery map, integrator dd §3.7) — never stored. What persists per sample is fixed-size, O(1) in `t`:

| Group | Contents | Notes |
|---|---|---|
| **Phase state** | `r, p` — 12 × f32 (vec2-grouped, `array<vec2<f32>, 3>` each; 8-byte aligned — payload §1, R-86) | the marching state at the playhead |
| **Shadow states** (tier-gated) | 1 Benettin shadow **per sample** (base and each ensemble copy), 12 × f32 each | **resident under lockstep — they march too.** Every sample is a full, uniform `SimState` computing its own FTLE (the sampling/SSAA decision: uniformity over micro-saving), so its FTLE shadow rides with it. Tier gates existence; `contains-ensemble` / `contains-FTLE` advertise it. The shadow is an *ingredient* of the sample's `ftle` field — never itself a coloured sample (renormalisation corrupts its endpoint) |
| **Ensemble copies** (tier-gated) | E full uniform `SimState`s per nominal sample — a footprint has **E+1 samples, `copy_index` 0..E**: copy 0 is the un-jittered centre, copies 1..E sit at **Halton (2,3) points 1..E**, centred (minus ½) and scaled to the footprint (R-80) | structurally identical to the base (colour through any render graph, no special-casing) but **scheduler-leaves** — a copy never spawns its own ensemble (`ENSEMBLE_ENABLED` applies only to nominal samples; no recursion). They double as the SSAA sample pool *and* the spread-metric pool. Per-pixel ≈ `2(E+1)` trajectories (each sample + its shadow); E is the tier's free-valued SSAA knob (0–15; High = 3, Extreme = 15 — memory-tiers) |
| **Running accumulators** | Benettin `S` (f32); unwrapped phase `θ̃` (f32 — `orbit_count`/`retrograde` derived at read); drift running-max accumulators (the current drift is derived at read, §3.1; R-246); **diffusion via Welford streaming regression** — per-sample `mean_y, C_ty` (2 × f32); the time-only terms `n, mean_t, C_tt` are **the same for all samples** (lockstep synchronises `t`) and are **DERIVED closed-form** (from `step_count`), not stored per-sample or in a mutable global — see the invariant | ⚠ **INVARIANT (footgun 1): diffusion is accumulated as *streaming regression state*, never as `(t, y)` points collected for a later fit** — points regrow the history the reversal deleted. **Welford, not raw moments:** the old raw-moment form (`Σt, Σt², Σy, Σty, Σy²`) computes slope as `(nΣty−ΣtΣy)/(nΣt²−(Σt)²)` — a *catastrophic-cancellation* difference of large nearly-equal products, precision-risky even at f32 as the sums grow over ~10⁵ steps. Welford tracks *centered* co-moments (running means + `C_tt, C_ty`), so **slope = `C_ty / C_tt`** directly — stable, no cancellation, no unbounded sums. **Lockstep — time-moments are DERIVED, not a shared mutable global** (WGSL has no dispatch-wide barrier to publish one safely mid-dispatch). For uniform sampling: `n=step_count`, `mean_t=(n+1)h/2`, **`C_tt(n)=h²·n(n²−1)/12`** (closed form). The per-sample covariance update uses the **OLD-mean** time deviation `δ_t=0.5·n·h` (NOT `t−mean_t` post-insertion, which is wrong): `mean_y += (y−mean_y)/n; C_ty += δ_t·(y−mean_y)`. Slope `C_ty/C_tt(n)`, **invalid for `n<2`** (reads NaN by the predicate `n ≥ 2`, R-245; no divide-by-zero). Only `mean_y, C_ty` are stored. (Non-uniform schedule → small precomputed prefix table, still read-only.) Net: **2 per-sample f32** (was 5), *and* numerically robust. Slope + R² derived from `(C_ty, C_tt, mean_y, mean_t, n)` at any playhead |

Terminal latch: on termination the whole block freezes (state stops advancing, accumulators stop updating).

*Note (R-72; REQ-RENDER-018; TASK-M1-15):* that the shadow is never itself a coloured sample is checked by
`cargo xtask lint wgsl`: no fragment-stage WGSL reads `r_sh` or `p_sh` but the generated read side, which loads them
for `ftle`, and the debug catalogue's field views of the two, which show the stored shadow as a field (render contract
Part 4, "The occupant rules `cargo xtask lint wgsl` checks").

**`θ̃`'s pole reference (R-397).** The hold of `θ̃` near the poles (integrator dd §3.7; R-389, R-392) keeps its frozen
reference longitude in `SimState`'s `_reserved`, a u16 code in steps of `2π/65535` with 0xFFFF for "no reference"
(payload §2). It is march state, not a field: as a `_`-named member it has no §3.8 entry and no view. Whether the
sample is inside the hold is not stored; a resumed march recomputes it from the current state.

**Ensemble spread is NOT a stored field — it is derived at the resolve stage.** Spread is a *footprint* quantity (one value per nominal sample), computed in the shader at the resolve stage from the E+1 samples of a footprint — the data-side twin of the SSAA colour resolve (colours→pixel colour on the render side; outcomes→spread scalar on the data side). It is consumed *live* for display (colour-by-spread) and aggregated into `QuadReduction` for the scheduler's refinement signal. It carries no per-sample `SimState` field: storing a group property on each sample is wrong-shaped and costs an f32 ×`2(E+1)`×viewport for nothing. **No per-sample "is ensemble" tag either** — grouping is structural (E+1 samples per footprint in the dispatch layout), `copy_index` marks nominal (0) vs copies (1..E), and ensemble-on/off is a quad/tier flag (`contains-ensemble`).

### 3.6 `ICDescriptor` (12 × f32)

`m0 m1 m2` (0-based body indices, R-22), `q_mass`, `rho_mag`, `lambda_mag`, `rho_ratio` (log), `rho_angle` (**cyclic**), `K_0` (lin, [0, ∞)), `V_0` (lin, (−∞, 0]), `virial_ratio`, `r_min_pair_0` (log). Provenance: decode stage, pre-integration.

**`K_0` and `V_0` are one-signed (R-403).** `K_0`, the kinetic energy, is declared `[0, ∞)`, and `V_0`, the
gravitational potential energy, `(−∞, 0]` (dd_decoder §3.6: `K₀ = Σᵢ ‖pᵢ‖²/2mᵢ`, `V₀ = −Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖`).
Neither range spans zero, so neither carries the `diverging` scale (§3.8): both are `lin`, on viridis, mapped
monotonically, `V_0`'s most negative value at the dark end. `E₀ = K_0 + V_0` keeps its `diverging` scale (§3.4).

**64 B with explicit padding** (R-86): the 12 × f32 fields are 48 B, and the remaining 16 B are declared padding, never
implicit. `E₀` is **derived** (`K_0 + V_0`), not stored.

**`ICDescriptor` follows `Real` (R-313).** Its twelve float fields have the width of `Real`, as `SimState`'s widening
fields do (dd_simstate_payload §1), so the payload is a function of `Real` throughout (REQ-PAY-017). The 64 B above is
the **f32 instantiation**. The f64 and DoubleF64 sizes and declared padding are part of REQ-PAY-087's definition,
written here by the task that makes the descriptor generic (R-72).

**Layout at a `Real` of `w` bytes (REQ-PAY-087; R-313).** One rule, dd_simstate_payload §1's: every f32 member is a
`Real`, and every other member keeps its width. The twelve fields, in the order above, are `Real`s at `k·w` (`k` = 0 …
11); `_pad`, the declared padding, follows at `12w` as four u32s, 16 B at every width; the struct is aligned to
`Real`'s alignment (4 at f32). The size is `12w + 16`, a multiple of that alignment at every row below, so no tail
padding is needed: the declared `_tail` is empty at each row (R-86).

| Row | `w`, alignment | fields | `_pad` | size |
|---|---|---|---|---|
| f32 (the GPU's) | 4, 4 | 0 … 44 | 48 | **64 B** (R-86) |
| f64 (the CPU kernel's) | 8, 8 | 0 … 88 | 96 | **112 B** |
| DoubleF64 (stub) | 16, 8 | 0 … 176 | 192 | **208 B** |

The DoubleF64 row is a row of the generated layout table only, with no `Real` impl and no instantiation
(dd_simstate_payload §1, R-265). `E₀ = K_0 + V_0` is formed at `Real` width on every path.

### 3.7 `QuadReduction` — completed ledger

**Status: complete.** The earlier instruction (to transcribe the remaining members verbatim from an older source) was
stale — that source held only a prose sentence, not an enumeration. The member list below is instead **derived from consumers** (scheduler
contract Part 6 names every field its predicates read) and **from measurement**: every inclusion and
exclusion traces to `principia_dd_refinement_criterion.md`. Sections cited as `[RC §n]`.

**Size.** **176 B, aligned to 4** (R-410; TASK-M5-01): the members below with u32 histogram bins, laid out as
"Layout" (below) gives, and measured as `size_of::<QuadReduction>()` of the generated Rust struct and as naga's size
of the generated WGSL struct. The bins are 128 B of it; §3.7's former u8 bins would have made it 80 B. The figure is
**descriptive, not a cap**: a member added later changes it. There is no memory pressure: `QuadReduction` is per
*quad*, against `SimState` at per *sample* (millions). At memory_tiers §3–§4's quad count (render pixels / N² at each
tier's `N`, at the largest display, 4K's 3840 × 2160, plus their ancestors, about a third more), at memory_tiers §4's
indicative `N` (8 for Potato and Low, 16 above), a tier holds at most 43 200 quads, 7.6 MB of reductions (Low and
High, and Ultra and Extreme, whose rows are provisional, R-137; Potato 10 800 quads, 1.9 MB; Medium 24 300, 4.3 MB),
against the 0.16–5.77 GB Potato's to High's totals give at 4K and the provisional 11.35 and 22.49 GB of Ultra and
Extreme from TASK-M3-41 on (*was* 0.16–5.51, 10.82 and 21.43 GB at 144 / 96 B, before R-603's and R-605's two u32s; perf review 5661999637 of PR #293, P1). The count follows each tier's `N`, which REQ-PERF-011 settles for WebGPU: at `N = 8` on every tier it is
97 200 quads, 17.1 MB for Medium and 172 800 quads, 30.4 MB for High, Ultra and Extreme, under 1.1 % of each of those
four tiers' 4K totals. Potato and Low are at `N = 8` already, at 1.2 % and 2.1 % of theirs. The real constraint is **categorical, not numerical** —
*reduce before evaporate*: fixed-size scalars only, never a history.

#### Identity

| member | type | note |
|---|---|---|
| `level` | u8 | quadtree depth; feeds the `ℓ < camera_depth + MAX_REL_DEPTH` guard |

`id` is **omitted**: the CPU indexes the readback by the quad it asked for, so identity is positional.

#### Outcome (all at joint `class ⊕ detail` grain)

| member | type | note |
|---|---|---|
| `class_histogram[N]` | u32 × N | derives dominant *and* impurity from **one** source, so they cannot disagree; each bin a u32 (R-410) |
| `dominant_outcome` | packed | `class ⊕ detail`, 5 bits |
| `outcome_impurity` | f16 | `1 − max(class fraction)` |
| `terminated_fraction` | f16 | **required** — without it "pure because settled" is indistinguishable from "pure because nothing has happened yet" [RC §7] |

**Bin width (R-410).** Each bin of `class_histogram` is a u32, so it counts up to 2³² − 1 footprint copies. A quad
holds `N² × (E+1)` copies, with `N` the samples per quad side (memory_tiers §4; not the bin count) and `E+1` the copies
per footprint, so a bin overflows only if `N² × (E+1) > 2³² − 1`: at `N = 32`, the largest a 1024-invocation
workgroup allows (memory_tiers §5), that needs more than about 4.2 M copies per footprint (2³²/1024 ≈ 4.19 × 10⁶),
which can't be allocated. **Dispatch asserts `N² × (E+1) ≤ 2³² − 1`**; no setting is capped for it, and a bin never
wraps or saturates silently.

**Bin count (REQ-PAY-075; definition, R-72).** The `N` of `class_histogram[N]`, the bin count (written `N_bins` to keep
it apart from memory_tiers' `N`), is **32 = 2⁵: one bin per code of the 5-bit `class ⊕ detail` field**, `state`
(bits 0–2 of `sample_descriptor`, §3.1) ⊕ `detail` (bits 3–4). **A copy's bin index is `state | d << 3`, where `d` is
its `detail` in a state payload §2 gives `detail` a meaning (escape, collision, sim_failed, decode_failed), and 0 in
every other state**: bounded and running, whose `detail` payload §2 and §3.1 leave undefined, and the reserved
`state` 6–7. A stray `detail` never splits a class: the joint set has one bounded class and one running class
(colour_composition §1.4), and a quad bounded everywhere reads `outcome_impurity = 0`. `dominant_outcome` holds this
index, and `RenderQuad` copies it in bits 0–4 (§3.7a); the generated `qr_class_histogram_bin(state, detail)` computes
it in Rust and WGSL. Bin `k` counts the quad's footprint copies whose bin index is `k`: every copy of every footprint,
running copies included (the uniformity invariant, below), so the bins sum to `N² × (E+1)`. The set is the field's
whole code space, not only the codes payload §2 gives a meaning: then `dominant_outcome` is the index of the largest
bin, with no table between bin and code, and a reserved `state` (6–7) is counted in a bin of its own, never merged
into another's. A bin no copy reaches stays 0, as the bins of a nonzero `d` under bounded, running or a reserved
state always do.

**Impurity grain (REQ-PAY-076; definition, R-72).** `outcome_impurity = 1 − max_k f_k`, where
`f_k = class_histogram[k] / Σ_j class_histogram[j]` is the fraction of the quad's copies whose bin index (above) is
`k`: the **joint
`class ⊕ detail` grain**, as every event-derived field here is ("One grain everywhere", below), over every copy, running
ones included (`Σ_j = N² × (E+1)`). `dominant_outcome` is the `k` of that maximum, so the two come from one histogram
and cannot disagree. The impurity mask (render contract Part 6, through `dominant_outcome`) and its cross-check
(debug_tooling_plan §G; REQ-VAL-083, TASK-M5-17) use the same bin index and this fraction.

**One grain everywhere.** Pending change 1 asked whether `dominant_outcome` needed a class-only
companion, since a masked accessor is unsound (argmax does not commute with masking). **Defining
every event-derived field at the joint grain dissolves the question** — one grain, no companion field,
no commuting problem [RC §6.5]. This also covers triple collision / triple ejection at
`detail = 11` (pending change 7, landed; payload §2).

#### Ensemble spread — the two bounded contributors

| member | type | note |
|---|---|---|
| `spread_shape` | f16 | `spread(n̂) / 2.0` — chord bound on the unit sphere |
| `spread_event` | f16 | `disagreement(class⊕detail) / (1 − 1/(E+1))` — attainable maximum |
| `error_ratio` | f16 | `sigma_E(t) / sigma_E(0)` — **exactly 1.0 under exact dynamics**, so any excess is integration error. Parameter-free. **Detects spread, not drift** — an ensemble whose copies drift *together* has a low ratio however badly they drift (measured: zero flagged pixels at 11% energy error). Pair with `worst_energy_drift`, which is the field to threshold on for absolute conservation. **Max-deviation internally, not MAD** — the requirement is **NaN-safe *and* sensitive**, and MAD fails the second: with 8 copies it medians 8 deviations, so a single wild copy is arithmetically invisible. Measured: a pixel at 2.9% energy drift reads **204.8** under max-deviation and **0.9018** under MAD — *quieter than at `t=0`*. Max-deviation is NaN-safe too (an infinite deviation is the correct answer, where `std` gives NaN). **`max`-aggregated over footprints** — an independent decision that happens to share the word. **Boolean flag only**; magnitude unstable [RC §7.19c, prin-rs RESULTS.md] |
| `roundtrip_error` | f16 | time-reversal round-trip displacement. **Catches phase error, which conserves `E` exactly and is invisible to `error_ratio`** — four measured cases had every conserved quantity at 1.000000 while the trajectory had moved 0.3–4.7% of a system radius. `corr` with drift +0.186, i.e. independent [RC §7.19f] |
| `spread_winner` | 2 bits | which contributor won the `max`; makes parent↔child switching visible [RC §7.13]. Codes (REQ-PAY-094; R-72): 0 `spread_shape`, 1 `spread_event`, a tie 0, the first operand of the `max`; 2 and 3 unused |

```
ensemble_spread = max(spread_shape, spread_event)        -- how error_ratio enters is OPEN
```

> **Uniformity invariant — every footprint carries `E+1` copies at all times; no copy is ever removed
> from the sample.** A trajectory that fails to integrate is a *measurement outcome* ("this could not
> be determined"), not missing data, and it is the strongest available statement that a pixel is
> undetermined — so it must raise `ensemble_spread`, not vanish from it.
>
> An earlier design **discarded** high-drift copies before reducing. That was wrong on four counts
> [RC §7.16]: it is **chaos-selective** (drift is caused by close encounters, so it excludes exactly
> the most chaotic samples — measured to understate escape by up to 0.19); it turns
> `1 − 1/(E+1)` from a constant into a **per-pixel variable**, which is the same defect that
> disqualified KE; failures are absorbing, so **image quality would decay monotonically with the
> playhead**; and it violates the sampling note's *"a pixel is a pixel"* principle. It also produced
> confident-looking answers from biased subsets — at `t=80` it reported an exponent computed on 44%
> of the data.
>
> Robust statistics (MAD without a gate) were tested as an alternative and fail past a 50% breakdown
> point — no statistic recovers information that was never obtained.
>
> **A conserved quantity is a free error meter.** The property that *disqualifies* conserved
> quantities as spread contributors — they have no dynamics, so their spread reports only that the
> ICs differ — is exactly what *qualifies* them as error measures: any change is error, by
> construction. Each trajectory conserves its own `E` and `L_z`, so the ensemble's *spread* of those
> is fixed at `t=0` and must stay constant. Two independent meters, no tuned constant. (A third is
> available on the AZ path: the regularised Hamiltonian `Gamma` is autonomous in `tau`, so it is
> conserved too.)
>
> **Contamination is tolerated deliberately.** Bad values do enter the reduction and nothing removes
> them. This is defensible because the exponent is used **ordinally** (contamination must be
> systematically *rank*-changing to matter, and it affects parent and child similarly), and because
> the failure direction is **conservative**: contamination inflates spread, which pushes toward
> *refine* — wasting budget rather than losing structure [RC §7.18a].
>
> **There is also no "failed" category.** Calling the drift threshold a *classifier* rather than a
> *filter* does not fix it — an arbitrary threshold still decides the answer, which is the `dom_KE`
> defect relocated. Numerical error here is a continuum: non-finite values are 0–4% while the finite
> drift tail spans **ten orders of magnitude**. And below some scale the error is not contamination at
> all: in a chaotic system a slightly-inaccurate trajectory is the exact trajectory of a
> slightly-different IC, which is precisely what the ensemble is already sampling. Measured, at
> `t=13`, `sigma_E(t)/sigma_E(0) = 1.0000` — the numerical error is invisible against the deliberate
> perturbation. **`error_ratio` replaces any notion of a failure count** [RC §7.17].

**Every contributor is bounded by its achievable maximum, so the normalisations are facts rather than
choices.** This is load-bearing, not tidiness: `max` over normalised contributors makes the
normalisation constants act as a decision threshold while looking like units, so an unbounded
contributor wins by virtue of its scaling rather than its information [RC §7.15c].

**Kinetic energy was measured, ranked first, and then dropped.** It won 69–100% of footprints — but
it is unbounded, spans 396× within a single configuration (against `event`'s 3.5×), and **no display
domain is sensible in more than one configuration**; the Burrau configuration admits none at all.
Clamping does not help: it changes the value but never the winner [RC §7.15a–b]. Also excluded, each
on evidence: total energy and `L_z` (conserved — amplification ≡ 1.00, so their spread reports only
that the ICs differ), `d_min` (latches), FTLE spread (map exponent 0.115 — refinement cannot reduce
it), diffusion (wins the `max` in ≤6% of footprints), `S_word` (pairwise-LCP, a different complexity
class, and it *lags* divergence rather than leading it), `t_end` (conditional — see below).

#### Refinement — the scaling exponent

| member | type | note |
|---|---|---|
| `alpha_area` | f16 | the stop rule's exponent, `log2(unresolved_area(coarse) / unresolved_area(children))`, judged over two levels (policy §2) — a dimension: a line reads 1, a sea 0, and the floor is below `alpha_lo`. An empty mask is told from a full one by `n_unresolved`, and the floor is refused on a negative exponent (R-42) |
| `alpha_energy` | f16 | **sanity field.** Total energy's exponent is known analytically to be 1.0 |
| `worst_energy_drift` | f16 | the field to threshold on for absolute conservation; pairs with `error_ratio` (R-87) |

(`error_ratio` doubles as the estimator trust flag, replacing the retention-based detector of
§7.15e: `error_ratio` departing from 1.0 is the same signal without a tuned threshold.)

**Only energy works as a conserved-quantity meter.** The construction needs the quantity to *vary
across the ensemble at `t=0`*. An `L_z` meter is **structurally undefined for from-rest ICs**: `v = 0`
gives `L_z ≡ 0` for any positions, so `sigma_Lz(0) = 0` and the ratio is `0/0` — undefined in 18/18
Burrau probes. Jitter moves positions, which always changes potential energy (`sigma_E(0) > 0`) but
never changes `L_z` from rest. Where an `L_z` meter does exist it correlates **+0.910** with the energy
meter. `Gamma` is redundant by construction (`corr = +1.0000000000`) [RC §7.19b, d].

**COM and total momentum are not meters here** — AZ integrates `(u1,p1,u2,p2)`, 8 phase dims and
exactly the relative motion, so the COM's 4 are never integrated and the COM is *derived* from `R1,R2`
and the masses. A measured `|COM| = 1.5e-16` is the correct value of a non-dynamical quantity, and the
frame re-projection producing it must be **kept** — it is what holds coordinates well-conditioned.
(Under a Cartesian integrator COM drift *would* be a genuine meter; it does not transfer) [RC §7.19e].

**Energy conservation does not catch phase error** — a trajectory can conserve `E` to machine precision
while having moved several percent of a system radius. Hence `roundtrip_error` as a second,
independent meter rather than a refinement of the first.

**Bounded by the predictability horizon, which is physics rather than a defect.** Round-off amplified
by chaos reaches system scale at `t_max = ln(1/eps)/lambda` — **~16 crossing times in f32 (the GPU
kernel), ~36 in f64 (the CPU kernel)**. Confirmed eta-independent: quadrupling integrator tolerance
does not move it. So *no* criterion can succeed past `t_max`, and the CPU↔GPU cross-check is
meaningless beyond the f32 horizon. Since `lambda` is already stored per sample as FTLE, `t_max` is a
computable **field**, not a constant — see `principia_dd_predictability_horizon.md`.

**Validated at short horizon only.** With nothing discarded, MAD over the full uniform sample recovers
the control in 10/12 quads at `t=13` and preserves the region ranking at Spearman **+0.943**. At `t=40`
no dispersion measure survives (5/12 at best): **removing the gate stops the sample shrinking but does
not remove the horizon problem**, and copy count is not the lever (8 → 16 copies moved 7/12 to 8/12)
[RC §7.19a].

Four properties of this block, all measured:

1. **`alpha` is ordinal only.** It ranks quads reliably; it does **not** forecast the realised gain
   as `2^-alpha` (median log error 0.219, worst 1.971) [RC §7.12].
2. **A heavy drift tail can dominate a variance** — 2% of bad values moved a measured exponent from
   1.0 to 6.7 [RC §7.11]. But there is no principled binary "failed" category to exclude on
   [RC §7.17], so the remedy is **measurement, not exclusion**: `error_ratio` reports how much of a
   footprint's apparent spread is numerical rather than physical, and the sample stays uniform.
   **How it should enter `ensemble_spread` is open** — it is unbounded above, so it cannot naively
   join a `max` of bounded contributors without reintroducing the §7.15c defect.
3. **`alpha_energy` is a free per-quad correctness check** — the field excluded for having *no*
   dynamics turns out to be the ideal control. If `|alpha_energy − 1| > 0.05`, every exponent from
   that quad is untrustworthy.
4. **Estimator failure is read from `error_ratio`** departing from 1.0 (R-87). Note there is **no
   better-estimator fallback**: Theil–Sen *is* two-point OLS at two scales (verified to 8.7e-10),
   so its advantage came from consuming more scales. The fallback is *acquire a third scale*
   [RC §7.15e].

#### Temporal accumulators (scheduler Part 8)

| member | type | note |
|---|---|---|
| `running_mean_divergence` | f32 | diagnostic, not a split input (R-99) |
| `first_divergence_t` | f32 | write-once; sentinel until crossed; diagnostic, not a split input (R-99) |
| `n_unresolved` | u16 | the latch's verdict (R-142, below): the count of the quad's unresolved footprints, latched ones included; at most N², as `valid_sample_count` (R-315). A count, not an accumulator |

**The latch is per footprint, not a `QuadReduction` member (R-99).** `running_max_divergence` (f32, max-updated,
**latching**) is held per footprint with the resident quad, and goes when the cache evicts or merges the quad, so it
never pins memory; its storage layout is defined by the task that builds it (R-72). It feeds "unresolved", not a second
split trigger (R-91): under the one `eps`, a footprint is unresolved if its spread exceeds `eps` now, or its latched
running maximum ever did, and a quad is unresolved if any of its footprints is. `θ_s`, `θ_max`, `θ_trend` and the trend
signal are dropped, so there is no `divergence_trend` member.

**The latch is evaluated on the GPU; only its verdict returns (R-142).** The latch is evaluated in the resolve pass
(R-135), and its state stays in GPU-resident per-quad memory. `QuadReduction` carries only the verdict: the count of
the quad's unresolved footprints, latched ones included (policy §1's `n_unresolved`). `QuadReduction` stays the sole
automatic return.

**`running_mean_divergence`'s formula and range (REQ-PAY-109; definition, R-72; R-545).** A footprint's divergence
at a sim time is the spread the latch reads, the one compared with `eps` (scheduler contract Part 6,
"Split/keep/merge": `spread_shape > eps`): the footprint's `spread_shape`, `spread(n̂)/2.0` over its `E+1` copies
(above), whose latched running maximum is `running_max_divergence` (refinement policy §1: "its latched running
maximum of spread_shape"). It is a time average over the fixed sim-time grid, never over scheduling events: `Δ` is the
sim key's fixed step `dt`, or the sync quantum `Δ_sync` under a regularised occupant (scheduler contract Part 7,
R-565), and `t_j = j·Δ`. At sim time `t = n·Δ`, with `d(f, t_j)` footprint `f`'s divergence at `t_j` and
`D_j = (1/N²)·Σ_f d(f, t_j)` its mean over the quad's N² footprints,

```
running_mean_divergence(t) = (1/n)·Σ_{j=1..n} D_j       -- updated in place: m_j = m_{j−1} + (D_j − m_{j−1})/j
```

the mean over the quad's footprints of each footprint's time-averaged spread, the two means commuting. It is a pure
function of the IC, the sim key and `t`: the live march, catch-up after reveal and a re-boot after eviction reach the
same `D_j` at the same `t_j` (scheduler contract Part 1, Part 7), however many resolve passes (R-135) or frames fell
between. A footprint whose copies have ended their march holds its state, so its `d` is its last value. Where the member
is held and updated is REQ-REF-046's; the formula is indexed by `t_j` alone. *Was (physics review 5679374140
of PR #318, F1):* "At the quad's `k`-th resolve pass (`k = 1, 2, …`), with `d_k(f)` footprint `f`'s divergence then
and `D_k = (1/N²)·Σ_f d_k(f)` its mean over the quad's N² footprints, `running_mean_divergence = (1/k)·Σ_{j=1..k}
D_j`, updated in place: `m_k = m_{k−1} + (D_k − m_{k−1})/k`". The in-place update
starts from `m_0 = 0`, written before the first update, so `m_1 = D_1` exactly and memory left uninitialised or
recycled from another quad is never read; with every `D_j` in `[0, 1]`, each f32 update stays in `[0, 1]`. *Was
(physics review 5679374140 of PR #318, F3):* "The in-place update sets `m_1 = D_1` whatever `m_0` holds, so the member
needs no initial value", false in floating point (a non-finite `m_0` gives NaN, and a finite `m_0 ≠ 0` rounds).
**An undetermined footprint reads 1.** Where `f`'s
`spread_shape` is non-finite (refinement policy §1's "the footprint is undetermined (non-finite spread, or an unusable
copy)"), `d(f, t_j) = 1`, the contributor's attainable maximum, chosen by a select on the operand's bits (an exponent
of all ones), never by a float comparison (R-79, R-297, as for `d_min`); the footprint stays in the N² of `D_j`. A
failure raises the spread and never vanishes from it (the uniformity invariant, "Ensemble spread" above), and one
NaN can't poison the mean for the rest of the quad's life. **Range `[0, 1]`, `lin`**: each `d(f, t_j)`
lies in `[0, 1]`, a finite one by `spread_shape`'s chord bound (its row, below) and an undetermined one as 1, so every
mean of them does; *was (physics review 5679374140 of PR #318, F2):* "each `d_k(f)` lies in `[0, 1]` by
`spread_shape`'s chord bound (its row, below), so every mean of them does"; the range does not
span zero, so the member is one-signed and on viridis, never on the diverging ramp (R-545, RQ-278 (d)).

**f32, not f16** — these accumulate in place over thousands of steps, where f16 would drift. Contrast
the thresholded scalars above, which are compared against `τ` and need no more precision.

**`first_divergence_t`'s sentinel (REQ-PAY-094; definition, R-72) is +∞**, f32 bits `0x7f80_0000`, held until the
first crossing: the minimum of the empty set of crossing times, as `d_min`'s unset value is f16 +∞ (R-271). It is
distinct from every crossing time, each finite and ≥ 0, and it is not NaN, the invalid value (R-79, R-136). A minimum
over footprints passes it by, and a quad that never crossed sorts after every quad that did. Like `d_min`'s, it is
written and tested by its bits, never by a float comparison (R-297).

Latching is empirically justified, not merely prudent: ensemble spread was observed to **fall 6×**
between `t=6` and `t=8` in one region — diverge-then-reconverge — so an instantaneous read genuinely
misses divergence that has already occurred [RC §5.2]. And `first_divergence_t` is the only
**saturation-immune** member here: every instantaneous spread saturates (and then reports `λ ≈ 0` for
the *most* chaotic regions), whereas a crossing time cannot [RC §5.1].

#### Validity and diagnostics

| member | type | note |
|---|---|---|
| `suspect_fraction` | f16 | integration-floor stop condition |
| `saturated_fraction` | f16 | substep-saturation — a **second** condition Part 6 gates on; was missing |
| `valid_sample_count` | u16 | decoded of N² |
| `coherence` | f32 | priority ordering (`complexity = 1 − coherence`) |
| `escape_time_min` / `_max` | f16 × 2 | diagnostic range; with no copy escaped, `_min` is +∞ (`0x7c00`) and `_max` −∞ (`0xfc00`), and "no escape" reads `_min > _max` (R-418) |
| `retrograde_fraction` | f16 | tiebreaker |
| `mean_orbit_count_spread` | f16 | tiebreaker |

**`coherence`'s formula and range (REQ-PAY-109; definition, R-72; R-545).** The priority's complexity term is
`1 − coherence`, the term that "drives adaptive refinement" (scheduler contract Part 6, "Priority"), and refinement is
driven by unresolved footprints: a quad splits iff any footprint is unresolved (Part 6, "Split/keep/merge"). So
complexity is the quad's unresolved share, and

```
coherence = 1 − n_unresolved / N²
```

with `n_unresolved` the quad's unresolved footprints, latched ones included (§3.7's "Temporal accumulators"), and N²
its footprint count. A quad with every footprint resolved reads 1, smooth and cheap to recompute, so evicted first
(Part 6, "Eviction"); a quad with none resolved reads 0. **Range `[0, 1]`, `lin`**: `0 ≤ n_unresolved ≤ N²` (R-315);
the range does not span zero, so the member is one-signed and on viridis, never on the diverging ramp (R-545, RQ-278
(d)). Priority orders work and never changes a payload (Part 1), so the formula decides scheduling order only.

**No escape (R-418).** `escape_time_min` and `escape_time_max` are the min and max reductions of the escape times of
the quad's copies that escaped, each starting from its reduction's identity: `_min` from +∞ (binary16 `0x7c00`) and
`_max` from −∞ (`0xfc00`). There is no no-escape branch: a quad none of whose copies escaped stores exactly those
bits, and "no escape" reads `_min > _max`. `_max`'s −∞ lies below its range, as a sentinel may; an escape time past
f16's range saturates, so every recorded time is finite and neither identity is ever a recorded time. Their ranges and
`mean_orbit_count_spread`'s, each proposed as `[0, ∞)` with `overflow: saturate`, join REQ-PAY-093's five and are
confirmed at the M5 gate (R-418).

#### Conditional — not yet included

| member | condition |
|---|---|
| `spread_t_end` | `alpha ≈ 0.86–0.95` once the escape time is **bisected inside the firing interval** rather than recorded at a sync boundary — its resolution otherwise *is* the sync interval, which was mistaken for absence of signal. Trust only where escape fraction ≈ 1.0 [RC §7.15f] |

#### Not reduction fields

- **Parent–child disagreement** needs no member: the CPU holds both reductions and compares them.
  That comparison **is** the `alpha` estimator and should be specified as such.
- **`priority`, `cacheAge`, `lifecycle`, `computeCostMs`, `gentime`** are CPU-computed quad metadata,
  not GPU reductions. A fragment shader cannot read them from this struct; they reach the display via
  tile debug shaders (render GUI spec §12.1). The engine holds them in `QuadMeta`, one per quad beside its reduction:
  `priority`, `cache_age`, `lifecycle`, `compute_cost_ms` and `gentime` (TASK-M5-01).

#### Layout and §3.8 entries

**Member order and packing (REQ-PAY-077; definition, R-72).** `QuadReduction` is 44 scalar slots of 4 bytes, 176 B,
aligned to 4, with no padding: `class_histogram` first, bin `k` at slot `k` (slots 0–31, bytes 0–127); then nine packed
words `qr_w0` … `qr_w8` (slots 32–40, bytes 128–163), the low half of each first: `qr_w0`'s low half holds the three
members narrower than 16 bits, and the 16-bit members follow two to a word in §3.7's order; then the three f32 members in §3.7's order (slots 41–43, bytes 164–175). The 2-bit `spread_winner` and the
5-bit `dominant_outcome` share `qr_w0`'s low half with the u8 `level`: `dominant_outcome` in bits 0–4, where
`RenderQuad` and `sample_descriptor` hold `class ⊕ detail` (§3.7a, §3.1), `spread_winner` in bits 5–6, bit 7 reserved,
`level` in bits 8–15. An f16 member is one binary16 value at a packed 16-bit location (§3.8's `f16`, R-312): bits 0–15
are `unpack2x16float`'s `.x`, bits 16–31 its `.y`, read widened to f32 (R-317). No member is a history or grows with
time or sample count (REQ-PAY-006); the bin count is fixed, so the layout does not follow the sim key's `N` or `E`
(R-410).

**The §3.8 entries.** Every member is `provenance: reduction` and `consumers: [scheduler, debug]`; an f16 range lies
within ±65504 or the entry states its `overflow` (R-248). A fraction is `[0, 1]`; `spread_shape` and `spread_event`
are `[0, 1]` by their normalisations (above); a count holds its width, `n_unresolved` and `valid_sample_count` being
at most N² (R-315); each member's range is the one its row in the table below declares, citing §3.7's table: a declaration, not a default for a member given no range (R-545, RQ-278). *Was:* "an f32 member §3.7 gives no range is `(−∞, ∞)`, as §3.8's worked entries are." `running_mean_divergence` and `coherence` are one-signed: each is declared `[0, 1]`, citing its formula in §3.7 ("Temporal accumulators" and "Validity and diagnostics"), a range that does not span zero (R-72, REQ-PAY-109; TASK-M5-32), so both stay `lin` on viridis. *Was:* "each gets a formula and a declared range that does not span zero, with its citation (R-72, REQ-PAY-109), so both stay `lin` on viridis; until then their rows keep the `(−∞, ∞)` the ledger was built with (PR #208), marked as waiting on it." `alpha_energy`'s range and its placement, centred on its analytic 1.0, are settled with REQ-PAY-093 at the M5 gate. The ranges and
`overflow` of `error_ratio`, `roundtrip_error`, `alpha_area`, `alpha_energy`, `worst_energy_drift`,
`escape_time_min`, `escape_time_max` and `mean_orbit_count_spread` are proposed, R-71 (REQ-PAY-093; R-418 adds the
last three), with their evidence in TASK-M5-01's PR, until the human confirms them at the M5 gate. The sentinels of
`escape_time_min` and `escape_time_max` are their no-escape values, +∞ (binary16 `0x7c00`) and −∞ (`0xfc00`), each
its reduction's identity (above, "No escape (R-418)"), tested by their bits.

| member | location | type | scale | range | sentinel · overflow |
|---|---|---|---|---|---|
| `class_histogram` | scalar-index 0 | vector(u-bits, 32) | lin | [0, 4294967295] | — |
| `dominant_outcome` | (qr_w0, 0, 5) | u-bits | categorical(32) | [0, 31] | — |
| `spread_winner` | (qr_w0, 5, 2) | u-bits | categorical(2) | [0, 1] | — |
| `level` | (qr_w0, 8, 8) | u-bits | lin | [0, 255] | — |
| `outcome_impurity` | (qr_w0, 16, 16) | f16 | lin | [0, 1] | — |
| `terminated_fraction` | (qr_w1, 0, 16) | f16 | lin | [0, 1] | — |
| `spread_shape` | (qr_w1, 16, 16) | f16 | lin | [0, 1] | — |
| `spread_event` | (qr_w2, 0, 16) | f16 | lin | [0, 1] | — |
| `error_ratio` | (qr_w2, 16, 16) | f16 | log | [0, ∞) | overflow inf |
| `roundtrip_error` | (qr_w3, 0, 16) | f16 | log | [0, ∞) | overflow inf |
| `alpha_area` | (qr_w3, 16, 16) | f16 | diverging | (−∞, ∞) | overflow inf |
| `alpha_energy` | (qr_w4, 0, 16) | f16 | lin | (−∞, ∞) | overflow inf |
| `worst_energy_drift` | (qr_w4, 16, 16) | f16 | log | [0, ∞) | overflow inf |
| `n_unresolved` | (qr_w5, 0, 16) | u-bits | lin | [0, 65535] | — |
| `suspect_fraction` | (qr_w5, 16, 16) | f16 | lin | [0, 1] | — |
| `saturated_fraction` | (qr_w6, 0, 16) | f16 | lin | [0, 1] | — |
| `valid_sample_count` | (qr_w6, 16, 16) | u-bits | lin | [0, 65535] | — |
| `escape_time_min` | (qr_w7, 0, 16) | f16 | lin | [0, ∞) | sentinel +∞ · overflow saturate |
| `escape_time_max` | (qr_w7, 16, 16) | f16 | lin | [0, ∞) | sentinel −∞ · overflow saturate |
| `retrograde_fraction` | (qr_w8, 0, 16) | f16 | lin | [0, 1] | — |
| `mean_orbit_count_spread` | (qr_w8, 16, 16) | f16 | lin | [0, ∞) | overflow saturate |
| `running_mean_divergence` | scalar-index 41 | f32 | lin | [0, 1] | — |
| `first_divergence_t` | scalar-index 42 | f32 | lin | [0, ∞) | sentinel +∞ |
| `coherence` | scalar-index 43 | f32 | lin | [0, 1] | — |

*The three rows as built (R-545, RQ-278).* The `alpha_energy`, `running_mean_divergence` and `coherence` rows above are
the ledger as built, which `crates/ledger/tests/quad_reduction.rs` parses against this table, so a row and its ledger
entry change in one commit. TASK-M5-32 conformed the two one-signed rows with the ledger: `running_mean_divergence`
and `coherence` are declared `[0, 1]`, each citing its formula in §3.7 above (REQ-PAY-109), a range that does not span
zero, so both stay `lin` on viridis. `alpha_energy` keeps `lin`, centred on its analytic 1.0, its range and that
placement proposed with REQ-PAY-093 and confirmed at the M5 gate; until then its `(−∞, ∞)` here is the built value,
not a declaration R-545 makes. *Was (TASK-M5-32):* "they change together with the ledger in TASK-M5-32, the task that
implements RQ-278's ruling. Under R-545: `running_mean_divergence` and `coherence` are one-signed, each declared a range
that does not span zero, with its formula and citation (REQ-PAY-109), so both stay `lin` on viridis; […] Until
TASK-M5-32 lands, the `(−∞, ∞)` here is the built value, not a declaration R-545 makes." *Was (fix round 1 of PR #274):* the three cells read "lin, centred on 1.0 / proposed
(REQ-PAY-093; M5 gate)" and "one-signed, declared by REQ-PAY-109 (built as (−∞, ∞), PR #208)".

**Scales (REQ-PAY-094).** A fraction, a spread, a count, a time and a crossing time are `lin`. `dominant_outcome` is
`categorical(32)`, one category per bin, and `spread_winner` `categorical(2)`, its two codes. `error_ratio`,
`roundtrip_error` and `worst_energy_drift` are `log`: each is a magnitude ≥ 0 whose values span decades (`error_ratio`
reads 1.0 under exact dynamics and 204.8 at 2.9% drift; drift's finite tail spans ten orders of magnitude, above).
`running_mean_divergence` and `coherence` are `lin` over `[0, 1]`, one-signed by their formulas (above; R-545, RQ-278 (d)). `alpha_area` is `diverging`: its sign is load-bearing, the floor being refused on a negative exponent (R-42), and its
range spans zero (R-403). `alpha_energy` is `lin`, centred on its analytic 1.0, not on zero; its range and that placement are confirmed with REQ-PAY-093 at the M5 gate (R-545, RQ-278 (c)).

#### Open

**Length scales are admissible in canonical units.** `r_coll` (collision radius) and an optional
softening `epsilon` are **gauge-covariant when expressed in canonical units** (`sqrt(I) = 1`), because
dynamical similarity rescales them along with the configuration — verified to five decimals across a
16× size range — since confirmed to **10 decimal places**, with `absolute` mode breaking it in the 2nd
decimal and *flipping the sign of `n̂₀`* [RC §7.19g]. Three constraints: they must be **fixed at `t=0`,
never co-moving** — a co-moving length makes the Hamiltonian time-dependent and destroys the energy
conservation `error_ratio` and `alpha_energy` depend on. **Measured: `|dE/E| = 3.062e-02`, identical at
`dt=1e-4` and `dt=2e-5`** — insensitive to step size, which is the wrong-equation signature rather than
an accuracy problem. Note co-moving is **gauge-covariant and still fails: covariance and conservation
are separate requirements.** Second, **gauge-covariant is not correct** — softened gravity is
a different force law at any `epsilon`, and ε>0 runs must be tagged and never pooled with ε=0 runs;
and **`r_coll` is the better default mechanism**, since softening makes the dynamics wrong everywhere
while a collision radius leaves them exact and only decides when to stop — recording a *result*
(pending change 7) rather than applying a fudge. **Adopted: `r_coll` nonzero by default, `epsilon`
default 0 and optional.**

**Long-horizon behaviour is now a reporting property rather than a limit.** With nothing discarded
and `error_ratio` read as the trust flag (R-87), a pixel whose copies cannot be integrated reads
**indeterminate** — which is true — instead of emitting an exponent computed from the tame minority
that survived. Measured, near-field at `t=80`: `ensemble_spread = 0.664`. And the refinement decision follows
automatically: as failures accumulate, parent and child spreads both saturate, `alpha → 0`, and the
quad **floors** — correct, since refining does not make a close encounter easier. Still open: gate threshold and integrator tolerance must be specified **as a pair**
(at `eta=0.005` the gate stops mattering; at `eta=0.02` no threshold reaches the trust bar) [RC §7.14a].

### 3.7a `RenderQuad` — the CPU-written quad record

Render contract Part 1 names `RenderQuad`'s fields but gives them no types. This table gives them; it is the ledger
table the generated `RenderQuad` follows, member for member (REQ-PAY-090; written by TASK-M1-06, R-72). The CPU writes
one per visible quad, and the fragment reads it as `ctx.quad` (render contract Part 1; Part 6's "Quad fields" view).
It is scheduler state, not payload, and the payload compatibility
signature excludes scheduler state (scheduler contract: "No carrying scheduler state into the payload"), so it is not
hashed into the schema version.

Every member is one 4-byte scalar: 40 B, aligned to 4, with no padding. A storage buffer holds no u8, u16 or f16
without a WGSL extension, so the counts and codes are u32 and the fractions f32.

| member | type | note |
|---|---|---|
| `quad_depth` | u32 | the quadtree depth `ℓ` (§3.7's `level`) |
| `quad_state` | u32 | 0 loaded · 1 pending · 2 refinable · 3 terminal · 4 stale (the debug tooling plan's quad-state enum, §F, in its order) |
| `coherence_score` | f32 | §3.7's `coherence` |
| `outcome_impurity` | f32 | §3.7's `outcome_impurity` |
| `ensemble_spread` | f32 | §3.7's `max(spread_shape, spread_event)` |
| `suspect_fraction` | f32 | §3.7's `suspect_fraction` |
| `priority_score` | f32 | the scheduler's priority (debug tooling plan §F: `w_v·P_v + w_z·P_z + w_c·P_c + w_f·P_f`) |
| `ancestor_gap` | u32 | the levels between the quad and the ancestor drawn in its place; 0 when its own payload is drawn |
| `cache_age` | u32 | the quad's age in the cache, in frames since its payload was last used (the cache's LRU age) |
| `dominant_outcome` | u32 | §3.7's `dominant_outcome`, `class ⊕ detail` in bits 0–4, as `sample_descriptor` holds `state` and `detail` (§3.1); render contract Part 6's impurity mask reads it ("`RenderQuad` exposes `dominant_outcome`") |

### 3.8 Metadata schema (what every entry must carry)

```
{ name, location: (word, offset, width) | scalar-index | derived(from: [field, …]),
  type: u-bits | f32 | f16 | f16-pair | fixed16 | vector(type, k),
  scale: lin | log | cyclic | diverging | categorical(n) | flag,
  range: declared(lo, hi, citation) | gap | provisional(requirement),
  sentinel?, tier_gate?, overflow?: saturate | inf, floor?: <sim-key parameter>,
  provenance: kernel | decode | reduction | cpu,
  consumers: [render, export, debug, scheduler] }
```

**A field without a complete entry fails generation loudly.** Coverage is enforced, not hoped for.

**Body and pair indices in exports stay 0-based, and say so (R-449; REQ-GEN-036).** The export decoder (seam 13) and
every machine-readable dump write a body- or pair-indexed field with the internal 0-based ids (R-22): `detail` under
escape and collision, `dmin_pair`, `m0 m1 m2` and any per-body or per-pair field. The export's header or schema states
the base for each: "body index, 0-based", or for a pair id "pair id, 0-based; pair `k` is the side opposite body `k`".
Which key of the entry carries that text is the task's to choose (REQ-GEN-036). What a person reads is 1-based, through
the one display function (payload §2, "What a person reads"); an export's caption is read by a person and is 1-based,
its columns are not.

**The `diverging` scale agrees with the range (R-403; REQ-GEN-033).** An entry carries `scale: diverging` only when
its declared `range` spans zero (`lo < 0 < hi`, either end possibly unbounded), as R-400 derives a field's diverging
ramp from its range. A `diverging` entry whose declared range does not span zero fails generation, naming the field
and its range; a one-signed field is `lin` (or `log`), as `K_0` and `V_0` are (§3.6).

**An undeclared range is a ledger gap, not a range that spans zero (R-426; REQ-GEN-035).** `range` is a key every
entry carries. Where §3 gives a field no range, the (−∞, ∞) the ledger writes in its place declares nothing: it is a
gap in the ledger, and it never makes a field diverging. A field is on the diverging ramp, or carries the `diverging`
scale, only when its declared range spans zero (R-400, R-403). Of the fields §3 gives no range, these are declared,
each citing its source: `S`, `ftle` and `diffusion` signed, (−∞, ∞), by R-423's physics check ("signed at finite time
with zero a meaningful centre"); `E_0` and `Lz_0` signed, (−∞, ∞), by their `diverging` scale (§3.4). Two fields
stay on the diverging ramp provisionally (R-426), recorded as provisional at the M1 gate: `q_mass`, until its formula
(REQ-PAY-088) declares its range (REQ-PAY-097), and `mean_y`, until `y`'s definition (REQ-PAY-073) declares its
(REQ-PAY-098); the physics reviewer re-checks each ramp then. A field whose range is still a gap is listed as one,
with the requirement that fills it.
`q_mass`'s range is declared by TASK-M2-03 (REQ-PAY-097): `[0, 1]`, from its formula, `min(m₀, m₁)/max(m₀, m₁)`
(dd_decoder §3.6; REQ-PAY-088). It does not span zero, so the field leaves `ramp_coolwarm` for `ramp_viridis`, and
`mean_y` is the one provisional range left.
Also declared (R-426's fix round, applied per R-369): `energy_drift` and `Lz_drift` (−∞, ∞), signed, citing the
worked entries below and §3.4, which already give them that range, so the ledger must write it; and `C_ty`
(−∞, ∞), signed, citing R-423's check of `diffusion` and `C_ty = diffusion · C_tt(n)`, `C_tt(n) = h²·n(n²−1)/12 > 0`
for `n ≥ 2` (§3.4), so `C_ty` has `diffusion`'s sign whatever `y` is (RQ-277, decided per R-369).

**The three range states (R-426; REQ-GEN-035).** An entry's `range` is in one of three states:
- **`declared(lo, hi, citation)`**: the range §3, a ruling or a definition gives, with where it is given. Only a
  declared range spans zero or not: a `lin` field whose declared range spans zero is on the diverging ramp (R-400), and
  only such a range admits the `diverging` scale (R-403).
- **`gap`**: no range is declared; the (−∞, ∞) the ledger once wrote by default. A gap never spans zero. A `lin` or
  `diverging` scalar field whose range is a gap fails generation, naming the field; on any other entry (a cyclic field,
  a vector, whose views read no range) a gap generates and is listed, with the requirement that fills it.
- **`provisional(requirement)`**: a range the human has kept provisional until `requirement` declares it, drawn
  meanwhile as (−∞, ∞), so spanning zero, and listed at each gate as provisional: `q_mass` (REQ-PAY-097) and `mean_y`
  (REQ-PAY-098), R-426. It is the range's counterpart of the constants register's `provisional?` key: proposed, used,
  and confirmed or replaced when its requirement lands.

`overflow` (R-248) states what an `f16-pair` value does past f16's finite range (±65504): `saturate` clamps to
±65504, `inf` rounds to ±∞. An `f16-pair` field's declared range must lie within ±65504; an unbounded end is
allowed only when the entry states `overflow`. At a packed location, `f16-pair` and `fixed16` take exactly 16 bits
and `f32` exactly 32, with no range-against-width test; any other non-integer type there fails the static check.

`f16` (R-312) is one binary16 value at a packed location of exactly 16 bits; `f16-pair` stays the type of two halves
sharing a 32-bit word. `f16` follows R-248's rules as `f16-pair` does: exactly 16 bits, a declared range within
±65504, and an unbounded end allowed only when the entry states `overflow`. It is a type for a packed 16-bit
location only: not a `scalar-index` location, and not a `vector` component type.

**`f16` is storage-only (R-317).** f16 arithmetic in WGSL is optional in WebGPU (the `shader-f16` feature), so no
generated code computes in binary16: a read accessor widens an `f16` value to f32 (`unpack2x16float`, core WGSL), and
no generated WGSL declares `enable f16` or an `f16`-typed value.

`floor` (R-263) names the sim-key parameter a log-magnitude or diverging view floors at: `eps_E` on `energy_drift`,
`eps_L` on `Lz_drift` (§3.4). The parameter is the same for every sample, so it is named, not stored, and it is
neither a ledger entry nor a register constant.

**Derived fields and vector fields** *(definition, R-72; REQ-GEN-024)*:
- **`derived(from: [field, …])`** is the location of a field computed at read rather than stored (§3.1's "Derived, NOT
  packed" list: `total_substeps_log2` is `derived(from: [total_substeps])`, `orbit_count` and `retrograde` are
  `derived(from: [theta])`). A derived field occupies no bits, so the static check (§5 test 1) does not see it; it
  carries the rest of the entry (type, scale, range, …) like any field, so the catalogue generates its view.
- **`from`** names the stored fields it is computed from; each must be an entry of the ledger whose location is a
  packed word or a scalar index, never another derived field, and `from` is never empty. It names entries by `name`,
  in any §3 struct: `n` and `energy_drift` name the `ICDescriptor` masses `m0 m1 m2` (§3.6). It names ledger entries
  only. A sim-key parameter is the same for every sample and is not an entry, so it is left out of `from`, and the
  read accessor takes it as an argument: `T` (in the sim key, §3.1), `dt` (§3.4's `h`) and `horizon_steps = ⌈T/dt⌉`
  (§3.1).
- **`provenance`** of a derived field names the stage that computes it, not the stages that wrote its inputs, since
  each entry in `from` carries its own. Any of `kernel | decode | reduction | cpu` is valid. `kernel` covers GPU code
  over the per-sample payload: the march, and the generated read accessor in the shaders that read it. §3.4's derived
  fields and `n` are all `kernel` (`n` is computed each step in the march, §3.5; the rest at read).
- **`vector(type, k)`** is the type of a vector-valued field: `k ≥ 2` components, each of the scalar `type`
  (`u-bits`, `f32`, `f16-pair` or `fixed16`, never a vector). `n` is `vector(f32, 3)`; the stored `r` and `p` are each
  `vector(f32, 6)` (§3.5's `array<vec2<f32>, 3>`, the same memory). Stored at a scalar index, a 32-bit component
  (`u-bits`, `f32`) takes one index, so the vector takes `k` consecutive indices from its `scalar-index`. A 16-bit
  component (`f16-pair`, `fixed16`) takes half of one: two share a 32-bit slot, component `2j` in the low half and
  `2j+1` in the high half of slot `j` (the `.x` and `.y` of `unpack2x16float`, §5 test 3), so the vector takes ⌈k/2⌉
  consecutive indices. `f16-pair` names that packing: each component is one binary16 value in half of a u32, as
  `d_min` is the high half of `packed_a` (§3.1).
- **Per component:** `scale`, `range` and `sentinel` apply to each component, not to the vector. `n` is `lin` over
  [−1, 1] in each component; `‖n‖ = 1` is a check on the vector (the catalogue's `|n|−1` view, render contract Part 6),
  not its range. The type is what tells the generator the field is a vector, so the catalogue offers its reductions
  (`‖·‖` as a scalar beside direction cosines, gui_state_contract §4).

**Worked entries: §3.4's derived fields and `n`.** Each is `f32` unless shown, has no `tier_gate` or `sentinel` unless
shown, has `provenance: kernel`, and has `consumers: [render, export, debug]`: render because §3.4 gives the
presentation metadata of the five scalars, and for `n` because gui_state_contract §4 and render contract Part 5/6
render it (§3.4 gives `n` none); export because seam 13 generates the export decoder for every ledger field
(systems_architecture §5); debug because render contract Part 6 gives each a view. The stored entries named in `from`
(`t_end_step`, `S`, `shadow`, `r`, `p`, `C_ty`, `E_0`, `Lz_0`, `m0 m1 m2`) are §3's own.

| name | location | value | type | scale | range | sentinel · tier_gate |
|---|---|---|---|---|---|---|
| `t_end` | `derived(from: [t_end_step])` | `T · t_end_step / horizon_steps`, the fraction of the horizon (§3.1, §3.4) | f32 | lin | [0, T] (§3.4; render contract Part 6) | — |
| `ftle` | `derived(from: [S, shadow, r, p, t_end_step])` | `S_final/(step_count·dt)`, the partial renorm interval closed from the shadow's separation (§3.1, §3.5) | f32 | lin | (−∞, ∞), signed: §3.4 gives no bound; declared by R-423's physics check (R-426) | tier_gate `ftle_valid` (§3.4); tier-absent it reads NaN (R-79); it reads NaN whenever `ftle_valid` is false, `step_count = 0` included (R-253, R-254) |
| `energy_drift` | `derived(from: [r, p, m0, m1, m2, E_0])` (R-246) | `H(r,p) − E_0` (§3.1) | f32 | diverging | (−∞, ∞), signed (§3.4) | — |
| `Lz_drift` | `derived(from: [r, p, Lz_0])` (R-246) | `L_z(r,p) − Lz_0` (§3.1's current drifts) | f32 | diverging | (−∞, ∞), signed (§3.4) | — |
| `diffusion` | `derived(from: [C_ty, t_end_step])` | `C_ty/C_tt(n)`, `n = step_count`, `C_tt(n) = h²·n(n²−1)/12` (§3.4, §3.5) | f32 | lin | (−∞, ∞), signed: §3.4 gives no bound; declared by R-423's physics check (R-426) | — (an invalid fit, `n < 2`, reads NaN by the predicate `n ≥ 2`, R-245) |
| `n` | `derived(from: [r, m0, m1, m2])` | the Montgomery map, never stored (§3.5; integrator dd §3.7) | vector(f32, 3) | lin, per component | [−1, 1] per component | — |

**The constants register** *(definition, R-72; REQ-SYS-063)*. Every settled constant, default or threshold is
declared once, in the constants register beside the layout table in `crates/ledger`, as an entry:

```
{ name, value: exact(v) | threshold(v) | calibration,
  class: achievable-maximum | conservation-law | canonical-units,
  citation: corpus(file, section) | prin-rs(commit, path) | calibration(requirement id),
  relative_basis?: distribution(source, percentile, populations) | gap(source, below, above),
  provisional?: gate(milestone) }
population = { name, lo, hi, count }
```

- **`value`**: a settled number, a settled threshold, or `calibration`. A constant the corpus leaves open is not
  entered with a value: its value is `calibration` and its citation is its calibration requirement, until the human
  confirms the value (R-71). A value is `calibration` exactly when its citation is a calibration requirement. A
  settled number or threshold is finite.
  **Except a provisional value (R-427):** a constant the corpus leaves open may instead be entered with its proposed
  number or threshold, marked `provisional` until its gate (below); it still cites its calibration requirement, so for
  it a number goes with that citation.
- **`class`**: why the constant is admissible, one of philosophy §4.2's three: bounded by its own achievable maximum,
  fixed by a conservation law, or expressed in canonical units. `achievable-maximum` covers a format's achievable
  extreme: a value fixed by a number format's limits, its greatest finite value or its smallest positive value
  (`f16_finite_max`, `f16_min_subnormal`; R-278).
- **`citation`**: where the value was measured or derived (INDEX, "The evidence base"): a corpus section, named by
  file and heading; a prin-rs `FINDINGS.md`, `README.md` or `results/` path at its prin-rs commit (R-159); or a
  calibration requirement id.
- **`relative_basis`**, required of a threshold. A threshold on a quantity spanning decades is relative: it is set from
  the observed distribution or from a measured gap (pitfalls §3). Each population is named with the closed range
  `[lo, hi]` of its values and its `count`, the observations it holds; its ends are finite, `lo ≤ hi`, and
  `count ≥ 1`. `distribution` names the populations the observed distribution shows and the percentile the threshold
  sits at; `gap` names the population below the gap and the one above it. Either way the threshold is finite and sits
  between populations of its own distribution (R-250): it lies inside the range of none of them, at least one lies
  wholly below it and at least one wholly above; a `gap` threshold lies inside the gap. A `distribution` threshold's
  percentile is derived from the counts: `100 · (count of the populations wholly below) / (total count)`, and the
  recorded percentile must equal it. There is no numeric bound on the percentile: a threshold inside any population
  fails, at whatever percentile, and one between populations passes at whatever percentile (R-250). Closure's absolute
  cutoff of 2e-3, inside the bound population's range, and `tau_display` at the 0.4th percentile of its own
  distribution, inside the population of quads it was to split, both fail (pitfalls §3; philosophy §4.2).
- **`provisional`** *(R-427; REQ-SYS-081)*: the gate, a milestone's, until which the value is a proposal (R-71, R-182):
  proposed with its evidence by the task that needs it, checked by a reviewer and used meanwhile. It is read like any
  other entry, its proposed number at once, and is never wrapped in a function in its place (below). Every gate can hold
  such entries: its report lists each entry marked provisional until it, and an entry still so marked blocks the gate,
  as an unconfirmed calibration does. When the human confirms the value at the gate, the mark comes off and the
  citation becomes the ruling that records the confirmation (`decisions.md`); a value the human changes is entered as
  the ruling gives it; a value confirmed provisionally, to be re-confirmed at a later gate, is marked provisional until
  that gate (R-419's two floors, until M2).
- **The gate:** an entry missing its value, class or citation, a non-finite value, a threshold without a relative
  basis, one whose basis names an ill-formed population, one that does not sit between its populations or whose
  recorded percentile is not the one its counts give, and a value that is `calibration` without citing its
  requirement (or the reverse) fail generation, naming the constant.
  A `provisional` entry is the exception to "the reverse": it carries a number and cites its calibration requirement.
  It fails generation if its value is `calibration`, if it names no gate, or if its citation is not a calibration
  requirement (R-427).
- **Reading a constant:** code in the physics and engine crates (`kernel`, `ledger`, `engine`) reads each number from
  the register; `cargo xtask lint constants` fails on a numeric `const` or `static` there that does not, naming file
  and line. The generated files are exempt: their numbers are emitted from the ledger.
  A provisional entry is read the same way. **No number is wrapped in a function to get past the lint (R-427;
  REQ-SYS-082):** a `fn` or `const fn` that returns a register constant's number in place of a register entry is the
  `const` the lint refuses, under another form. A proposed value is entered `provisional` and read from the register.
  **A counter's starting zero is not a register value (R-440):** a count from none, written `Cell::default()`, is not a
  default, threshold or calibration. This covers counters only, never real values: a default, threshold or
  calibration is read from the register, even at 0, in whatever form it is written. An IEEE-754 format constant written
  from its std name, such as `f32::INFINITY.to_bits()` for the f32 exponent mask, is not one either (applied per R-369,
  recorded in R-440); the scope is format constants, never calibration values. A representation sentinel meaning "no
  value", written from its type's bound (`i16::MIN`, `u32::MAX`), is not a register value either (applied per R-369;
  TASK-M2-07); the scope is such sentinels only, never defaults, thresholds or calibrations.
- **The hash:** the register entries that decide what the payload's stored bits mean are part of the ledger hashed
  into the schema version (R-36): the word's capacity and length sentinel (§3.3), the `horizon_steps` limit (§3.1),
  the f16 pack clamp (payload §1), the f16 subnormal floor (R-271), and the step count and sentinel of `θ̃`'s pole
  reference in `_reserved` (payload §2; R-397), which today are all seven entries below. The
  canonicalised table the hash covers includes each such entry's value, type and class, not its citation text
  (R-251): changing a hashed entry's value, type or class changes the schema version, and a citation-only edit does
  not. Any other register entry, such as a
  render or scheduler constant the lint brings into the register, is not hashed into the schema version: changing it
  has the blast radius caching_contract Part 2 gives its knob (render settings and
  scheduler knobs invalidate nothing; canonical_spec §9, invariant 3). The link registry's entries are hashed too, by
  their semantic content (§3.9, "The hash"; R-340).
  The `provisional` mark and its gate are not hashed, as the citation is not: confirming a value at its gate does not
  change the schema version; changing a hashed entry's value does (R-427).

The register's entries, the constants the payload ledger uses:

| name | value | class | citation |
|---|---|---|---|
| `horizon_steps_max` | 65535, the greatest count the exact u16 `times` holds; dispatch refuses more | achievable-maximum | R-86 |
| `fgw_capacity` | 76 symbols, 1 + ⌊(121 − 2)/log₂3⌋ in the 121-bit mixed-radix payload | achievable-maximum | payload §3 |
| `fgw_length_sentinel` | 127, the greatest value of the 7-bit length field | achievable-maximum | payload §3 |
| `f16_finite_max` | 65504, binary16's greatest finite value; the pack clamp ±65504 | achievable-maximum | payload §1 |
| `f16_min_subnormal` | 2⁻²⁴ ≈ 5.96e-8, binary16's smallest positive subnormal; a valid `d_min` below it is stored as it (bits `0x0001`), so 0.0 never appears | achievable-maximum | R-271, R-278 |
| `theta_ref_steps` | 65535, the codes of `θ̃`'s pole reference in `_reserved`: steps of 2π/65535, codes 0…65534, the u16's greatest value spared for the sentinel | achievable-maximum | R-397 |
| `theta_ref_none` | 65535 (0xFFFF), the u16's greatest value: `_reserved` holds no reference (outside the pole disc, or R-392's IC that starts inside it) | achievable-maximum | R-397 |

`diffusion` has no sentinel in the register: an invalid fit reads NaN by the predicate `n ≥ 2` (R-245).

The register's provisional entries (R-427; REQ-SYS-081), each a proposed value marked `provisional` until its gate,
citing its calibration requirement, and read from the register like any other; none is hashed:

| name | proposed value | class | gate | requirement |
|---|---|---|---|---|
| `log_floor_rho_ratio` | 2⁻²³ ≈ 1.19e-7, `rho_ratio`'s log floor, assuming the mass-weighted ‖ρ̃‖/‖λ̃‖; confirmed provisionally at the M1 gate and re-marked provisional until M2 (R-419, R-428) | canonical-units | M2 | REQ-TOOL-163 |
| `log_floor_r_min_pair_0` | 2⁻¹⁴ ≈ 6.10e-5, `r_min_pair_0`'s log floor, a distance; confirmed provisionally at the M1 gate and re-marked provisional until M2 (R-419, R-428) | canonical-units | M2 | REQ-TOOL-163 |
| `eps_m01` | 10⁻⁵, the `DEGENERATE(M01_TINY)` threshold (dd_decoder §3.1), a threshold with its measured gap | canonical-units | M2 | REQ-DEC-039 |
| `tanh_sat` | 20, the bound §3.9's `tanh` operator clamps its argument to: `tanh` rounds to ±1 from `abs(x)` ≈ 9.01 at f32 and ≈ 19.06 at f64, so the clamp changes no CPU value at either width, while Metal's GPU `tanh` fails from 43.671875, 0 and then NaN (TASK-M2-06) | canonical-units | M2 | REQ-DEC-047 |
| `r_pole` | 0.01, the pole radius of the unwrapped phase `θ̃` on the unit shape sphere (dd_integrator §3.7, R-389); confirmed at the M1 gate and marked provisional until M3, where it is re-confirmed against real orbits' near-pole arcs (R-433) | canonical-units | M3 | REQ-INT-086 |
| `diverging_x0_E_0` | 1, `E_0`'s `x₀`, an energy: the magnitude where its `diverging` symlog's compression sets in (dd_colouring §3.6, REQ-COL-065), one unit of the normalised system's binding energy (TASK-M7-05) | canonical-units | M7 | REQ-COL-066 |
| `diverging_x0_Lz_0` | 1, `Lz_0`'s `x₀`, an angular momentum, as `E_0`'s (dd_colouring §3.6, REQ-COL-065; TASK-M7-05) | canonical-units | M7 | REQ-COL-066 |
| `shape_vec_js_tol` | 2⁻⁴⁸ ≈ 3.55e-15, the agreement of the shape map's `n` with the IC Inspector's on random ICs, the largest component difference (chart_reference §3.1; TASK-M2-08) | canonical-units | M2 | REQ-CHART-049 |
| `shape_roundtrip_bound` | 10⁻¹³, the shape-sphere round trip's bound, `max ‖shape(Φ(u, v)) − n(u, v)‖` with `Φ` alone (chart_reference §3.2; R-178, R-181; TASK-M2-08) | canonical-units | M2 | REQ-CHART-053 |
| `eps_phys` | 64, `k` in `ε_phys = k·ε_R`, the round trip's tolerance in `‖·‖_phys`, `ε_R` the precision's machine epsilon (inverse-encode contract Part 4; TASK-M2-16) | canonical-units | M2 | REQ-ENC-024 |
| `decode_sanity` | 16, `k` in `k·ε_R`, lookup's layer-3 tolerance on the CoM and `Σpᵢ` residuals `‖Σmᵢrᵢ‖/R_x` and `‖Σpᵢ‖/P_x` of the decoded lock (inverse-encode contract § "Chart-aware validation", "Layer 3's tolerance"; TASK-M2-19) | canonical-units | M2 | REQ-ENC-027 |
| `zero_class` | 16, `k` in `k·ε_R`, lookup's layer-1 zero-class band, relative to the first-order bound on each value's rounding, `S_{L_z}` and `S_E` on the entered state's extents, and each clamp's move's own (inverse-encode contract § "Chart-aware validation", "The zero class in floating point"; TASK-M2-19) | canonical-units | M2 | REQ-ENC-037 |
| `literature_import_tol` | 256, `k` in `k·ε` relative at f64, the literature import's tolerance on each start's `E`, `L_z`, `R` and `L_z²·|E|/(G²·M⁵)` after multiplying back by the recorded rescale (inverse-encode contract Part 7; dd_validation_orbits §1.6 and §2; TASK-M2-20) | canonical-units | M2 | REQ-VAL-120 |

The register's entries confirmed at the M1 gate (R-428): each was provisional until M1 (R-427), is confirmed as
proposed, and cites R-428 in place of its calibration requirement (REQ-TOOL-160 for the floors, REQ-COL-055 for the
hatch); none is hashed. `d_min`'s floor at binary16's smallest normal is intended: a floor at 2⁻²⁰ would compress
`d_min` ∈ [6e-5, 1] from half the ramp to an eighth of it (R-428, answering R-427's question). `rho_ratio`'s and
`r_min_pair_0`'s floors, confirmed there provisionally, are in the table above, provisional until M2.

| name | value | class | citation |
|---|---|---|---|
| `log_floor_d_min` | 2⁻¹⁴ ≈ 6.10e-5, `d_min`'s log floor, a distance (binary16's smallest normal) | canonical-units | R-428 |
| `log_floor_dE_max` | 2⁻²⁰ ≈ 9.54e-7, `dE_max`'s log floor, absolute, in the stored field's normalised units | canonical-units | R-428 |
| `log_floor_dLz_max` | 2⁻²⁰ ≈ 9.54e-7, `dLz_max`'s log floor, absolute, in the stored field's normalised units | canonical-units | R-428 |
| `log_floor_closure_min` | 2⁻²³ ≈ 1.19e-7, `closure_min`'s log floor, a dimensionless chord | canonical-units | R-428 |
| `hatch_violet` | `0x9B00FF`, the invalid hatch's first colour, 8-bit sRGB | canonical-units | R-428 |
| `hatch_cyan` | `0x48FFFF`, the invalid hatch's second colour, 8-bit sRGB | canonical-units | R-428 |
| `hatch_half_period` | 4, the hatch's stripe width in pixels | canonical-units | R-428 |

The register's entry R-433 confirms at the M1 gate: a calibration the task that needed it proposed as a test's
definition (REQ-VAL-122, TASK-M1-12), entered confirmed, citing R-433, so that its tests read it from here rather than
each defining it; it is not hashed. `r_pole`, which R-433 confirms too, is in the provisional table above, until M3.

| name | value | class | citation |
|---|---|---|---|
| `n_norm_tol` | 2⁻¹⁸ ≈ 3.81e-6, 64 U with U = 2⁻²⁴ the f32 unit roundoff: the tolerance on `‖n‖ − 1` at f32 of the paired test asserting `‖n‖ = 1` (gui_state_contract §4) | canonical-units | R-433 |

### 3.9 The link registry (consolidated from chart contract Part 2.5)

| Constraint | Link (forward) | Inverse | log-det | Sampling note |
|---|---|---|---|---|
| Simplex Δ² | softmax ∘ `μ_max·tanh` | log-ratio → `artanh`, clamp `(1−ε_μ)μ_max` | `log √det(JᵀJ)`, `J` the forward's 3×2 Jacobian (softmax Jacobian × sech² factors) | under-samples simplex edges/corners |
| Bounded [a,b] (R-417) | scaled/shifted `σ` | `logit`, clamp `[ε_z, 1−ε_z]` | `log((b−a)·σ')` | edge-heavy: crowds the bounds |
| Bounded alt | scaled `tanh` | `artanh` | `log(c·sech²)` | interchangeable with σ up to reparam (`tanh x = 2σ(2x)−1`) — differs in slope profile only |
| Positive (0,∞) | softplus / exp | inverse-softplus / log | `log σ(x)` / `x` | exp is heavy-tailed |
| Symmetric [−c,c] (R-417) | `c·tanh` | `artanh` | `log(c·sech²)` | as bounded |
| Unbounded ℝ | identity | identity | 0 | neutral |

Each entry ships **forward, inverse, log-det, ε clamps, and the sampling note** — the robustness-sweep picker uses the last to choose alternatives that *disagree where it matters*.

**The log-det column (R-368).** An entry's log-det is the log of its link's volume factor: `log |det J|` of the
forward's Jacobian `J` where `J` is square, and, where it is not, the log of the area element `√det(JᵀJ)`. The simplex
link's `J` is 3×2 (two controls, the three masses of Δ²), so its entry is `log √det(JᵀJ)`. The other entries are the
logs of the factors this table held until R-368 (`(b−a)·σ'`, `c·sech²`, `σ(x)` / `eˣ`, and the identity's 1, whose log
is 0). The physics reviewer checks this conversion at TASK-M2-01, which writes the registry's entries.

**The hash (R-340).** The link registry is part of the ledger hashed into the schema version (R-36; §2's
"canonicalised §3 ledger"), since its definitions decide how a chart coordinate decodes, and so what a cached or saved
result means. Each entry is hashed by its semantic content, not its prose, as a register constant is by its value,
type and class and not its citation (§3.8, R-251). An entry's hashed members are:
- its name, the link id provenance records (chart_decoder_contract § "Integrity: the link is part of the experiment");
- its constraint, the block codomain its row names (the table's first column);
- its forward, inverse and log-det, each in a canonical form with no formatting-dependent bytes (the form is a
  definition TASK-M0-46 writes here, R-72, REQ-GEN-032);
- its ε clamps, and each parameter its forward, inverse or log-det reads (`μ_max`, `q_max`, `α_min`), by value.
- *(R-481)* the declarations of its tunable parameters (below, "Tunable parameters"): the entry's name, the parameter's
  name, its default and its allowed range's two ends by value, not its citation, as a block of their own after the
  chart constants, so the bytes above are unchanged. A view's active values are never hashed: they are the view's
  state (`SimConfig`'s `Links`, `principia_gui_state_contract.md` §2), not the ledger's, so a changed default or range
  changes the schema version and a view at `μ_max = 4` does not.

Its sampling note is not hashed: it is prose for the robustness-sweep picker, and no decode or encode reads it.
Changing a hashed member of any entry, or adding or removing an entry, changes the schema version; a sampling-note-only
edit does not. Every entry is hashed, whether or not a block uses it by default (R-344). The registry's chart
constants are hashed too, each by value, whether or not a link reads it: those that no link reads, `δ_λ` (decode's
mirror tie-break), `ε_w` (its seed-selection floor) and `ε_M01` (the mass block's M01_TINY threshold, dd_decoder §3.1;
added by TASK-M2-02), change how a chart coordinate decodes, as the links do (R-344, closing RQ-189). `tanh_sat`, the
`tanh` operator's argument bound (added by TASK-M2-06), is read by the operator, not by a tree, and is hashed likewise.

**Each entry's own hash (R-521).** Beside the schema version, each entry has an entry hash: the 64-bit hash of that
entry's canonical bytes alone, its hashed members above and the declarations of its tunable parameters, computed by
the same hash as the schema version and generated with the registry. It also covers, by value, the registry constants
outside the entries that the entry's decode reaches: `tanh_sat` for an entry whose trees hold a `tanh` node, `ε_M01`
for an entry onto the masses, and `δ_λ` and `ε_w` in every entry (applied per R-369, physics review P2 of PR #265).
Not every decode reads those two: `δ_λ` is read only by dd_decoder §3.3's canonicalise and `ε_w` only by §3.4's
invariant construction; neither depends on the link, so hashing both into every entry is a deliberate, conservative
choice, a needless refusal being safe where a stale recreation is not (applied per R-369, physics review P6 of PR #265;
*was:* "`δ_λ` and `ε_w`, which every decode reads, in every entry", reworded, kept here). A change to any of them
changes the hashes of the links it reaches and is refused naming them, never recreated. The refusal names the
constants as well as the link: the record carries hashes, not the writer's constants, so it names each mismatched link
with the constants its hash covers ("its entry, or `tanh_sat`, `δ_λ` or `ε_w`, changed"), and when every block
mismatches it leads with the constants all four mismatched hashes cover, computed from the record's links: `δ_λ` and `ε_w` always, and `tanh_sat` as well when every recorded link is tanh-bearing (softmax_tanh or stick_breaking on the masses, a tanh_* link on α, β and q), saying that one of them may be what changed, so a user is not told their
links changed when none did (applied per R-369, physics review P6 of PR #265; the lead per physics review P8 of PR
#265, applied per R-369; *was:* "and when every block mismatches it says first that `δ_λ` or `ε_w`, the constants all
four share, may be what changed", stale value, replaced by physics review P8 of PR #265). Every record and export carries, per block, the
entry hash of the link it records, a u64 written as a decimal string (R-322), beside the build hash and the schema
version, outside `SimConfig` since it derives from the build's registry and the link ids, not from the view
(`principia_dd_image_embedding.md` §6; `principia_gui_state_contract.md` §2). A reader compares each block's against
its own registry's entry of that id: a mismatch names exactly that link, and a match recreates across a schema change
that left the record's links and what they reach alone, such as a payload-ledger entry no decode reads. Changing an
entry, or a constant it reaches, changes its entry hash and the schema version; changing another entry, or the payload
ledger, changes the schema version only. The entry hashes are not the whole decoder's identity: three things no entry
hash or constant sees stay with the build-hash component (`principia_dd_image_embedding.md` §6's `build hash` line,
REQ-TOOL-112), which refuses a change to them: dd_decoder §3.2–3.5's own construction (the reconstruction, the seeded
fallback family and its order, the scale gauge); how the closed operators are evaluated (the canonical form hashes an
operator's spelling, not its evaluation); and the emitter's rules, which leave the hashed bytes unchanged (the f32
images of π/2 and π rounded toward zero, each tunable's f32 value rounded to nearest) (applied per R-369, physics
review P5 of PR #265).

**The canonical form of a link's functions** *(definition, R-72; REQ-GEN-032)*. An entry's forward, inverse and
log-det are written as expression trees, never as text:
- **The functions.** The forward is one tree per physical component, over the control's components; the inverse is
  one tree per control component, over the physical value's components; the log-det is one tree, the entry's log-det,
  over the control's components. A vector map is written out per component: softmax's component `i` is
  `div(exp(x_i), add(exp(x_0), …))`.
- **The nodes.** `input(i)` is component `i` of the function's argument, counted from 0. `param(name)` reads one of
  the entry's ε clamps or parameters by name; its value is hashed with the entry's clamps and parameters, never inline,
  so a tree naming one the entry does not declare fails generation. `num(v)` is a literal number. An operator node
  applies one of a closed list to its arguments, in order: `add`, `mul` (two or more, `x_0 + x_1 + …`,
  `x_0 · x_1 · …`); `sub`, `div` (two, `x_0 − x_1`, `x_0 / x_1`); `clamp` (three, `min(max(x_0, x_1), x_2)`); and,
  of one, `neg`, `exp`, `log` (natural), `tanh` (evaluated as `tanh(clamp(x, −tanh_sat, tanh_sat))`, its result then
  selected to ±1 where its magnitude exceeds 1, by comparison, a NaN kept; definition, R-72, TASK-M2-06: the clamp acts
  only where `tanh` already rounds to ±1 at f32 and f64, and gives ±1 where a GPU's `tanh` fails at large arguments, as
  Metal's does from 43.671875, 0 and then NaN; the select keeps the operator in `tanh`'s own range, `[−1, 1]`, where a
  backend's accuracy bound lets its result pass 1, as Metal's `tanh` returns 1 + 2⁻²³ at about 1.5% of arguments in
  `[9, 20]`, and never acts on the CPU, whose `tanh` stays in `[−1, 1]`. Both are an operator-level correction to the
  backend, distinct from the link-level rule below that no link clamps its output: neither lands a link on an endpoint
  nor guards against a degenerate configuration (validation ground-truth note), and `qₖ` reaches `±q_max` by
  saturation, as R-417 has it), `artanh`, `sigmoid` (`σ(x) = 1/(1 + e⁻ˣ)`), `logit` (`log(x/(1 − x))`,
  σ's inverse), `softplus` (`log(1 + eˣ)`), `inv_softplus` (`log(eˣ − 1)`) and `sech2` (`1/cosh² x`). An operator given
  another number of arguments fails generation; one outside the list cannot be written, and adding one is an edit to
  this list.
- **The bytes.** A tree is written in prefix order: the node's spelling above (`input`, `param`, `num` or the
  operator's), length-prefixed UTF-8, then `input`'s index as a big-endian u32, `param`'s name length-prefixed, `num`'s
  value by its f64 bits, big-endian, or an operator's argument count as a big-endian u32 and each argument. A list of
  trees is its count, then each tree. An entry is written as its name; its constraint, spelled `simplex`, `bounded`,
  `positive`, `symmetric` or `unbounded`; its forward and inverse lists; its log-det tree; then its ε clamps and its
  parameters, each list sorted by its names' UTF-8 bytes, lexicographically, each item its name and its value's f64
  bits. Entries are sorted by their names' UTF-8 bytes, lexicographically, then the chart constants, sorted by their
  names' UTF-8 bytes, lexicographically, each its name and value's bits. The sampling note is not written. Two entries,
  two chart constants, or two of one entry's clamps and parameters, of one name fail generation. Every length prefix
  (a string's UTF-8 byte count) and every count is a big-endian u32, and every value's f64 bits are big-endian; an
  entry's ε clamps, its parameters, the entries and the chart constants are each written as their count, then each
  item.
- **No formatting-dependent bytes.** No source text is hashed: whitespace, parentheses, comments and a literal's
  spelling (`1`, `1.0`, `1e0`) are not in the tree, and a literal is its bits. A change of meaning, another operator,
  argument, literal, parameter or argument order, changes the bytes. Two trees equal in algebra but not in structure
  (`add(a, b)` and `add(b, a)`) hash differently: the tree is the order of evaluation, which can change a float result's
  bits, and a needless invalidation is safe where a stale one is not.

**The chart constants** *(REQ-DEC-009)*. The registry's parameters are the chart constants. Each is a
constants-register entry in canonical units, with an exact value. The link trees read them by name (`param(name)`), and
the generator emits them into the kernel as `crates/kernel/src/generated/constants.rs`, so that no decode or encode
formula writes one as a literal. They are `mu_max` = 5 and `q_max` = 2 (R-10; the defaults of two tunable parameters, R-481), `alpha_min` = 0 (R-21), `eps_mu` =
`eps_z` = `eps_q` = 10⁻⁶ (chart_decoder_contract § "Three hard requirements on any registered link"), `delta_lambda` =
10⁻¹² (R-82) and `eps_w` = 10⁻¹⁰ (chart_reference §2.2).

**Tunable parameters** *(R-481 ⚑; the declaration's form applied per R-369, its ranges R-71 calibrations,
REQ-GEN-038)*. A parameter an entry's trees read may be declared tunable, in a table beside the entries, one row per
tunable parameter: the entry's name, the parameter's name (a chart constant the entry declares and reads), its default
(that constant's register value), its allowed range `[lo, hi]`, closed, and its citation. Today the rows are
`softmax_tanh`'s `mu_max`, its range's `hi` at most 10 (R-597, the human's cap, so that no far-field pixel reaches the
`M01_TINY` fence; REQ-GEN-038), and `sigmoid_q`'s, `tanh_q`'s and `softsign_q`'s `q_max`; `alpha_min` is declared by the α
entries and stays fixed (R-21; R-518, which rules that it does not become a parameter; *was:* "(R-21; RQ-382)", a
stale value, replaced by R-518). The declaration adds no member to an entry, so the entries' canonical bytes
above stand.
- **The check.** Generation refuses a range under which the entry breaks any of the chart decoder contract's three hard
  requirements (§ "Three hard requirements on any registered link"), tested at both ends and at a seeded sample of at
  least 16 interior values, uniform over the range, each under a seeded fuzz of the control that includes saturating
  controls: the output in the codomain (the simplex within its rounding); the inverse, with its ε clamps, returning the
  control within the encode's tolerance where the forward is not saturated, at f32 and f64; and C¹, the derivative from
  the log-det finite and agreeing with a central difference of the forward. A refusal names the entry, the parameter,
  the value and the requirement. An entry whose forward leaves the codomain it claims is refused at every value, and a
  slot given an entry of another codomain is refused (REQ-GEN-013).
- **The active value.** Everything that reads a tunable parameter by name reads the active value: the trees'
  `param(name)`, the codomain's bounds, and, outside the trees, the encode's clamps and its layer-1 range check. The
  generated code takes the tunable parameters as an argument, one field per tunable of the registry at the float type,
  in place of the constant; the functions without it stay, evaluated at the defaults. On the GPU the values are a
  uniform, beside the chart params, each f32 value the f64 value rounded to nearest as a literal is (R-481, G7: a
  uniform, not a rebuild per value). At the defaults the results are bitwise those of the constants. The link ids stay
  type parameters, so a link change is another monomorphised variant (REQ-GEN-015) and a parameter change is not.
- **The log-det** is evaluated at the active values, so a quantitative pass reweights with them, never the default.

**The registry's entries** *(TASK-M2-01; the two rows marked R-72 are definitions, REQ-GEN-026)*. Each row of the table
above is registered once for each block codomain it serves. The codomains are:
- the mass simplex Δ², two controls onto three masses;
- `α ∈ [α_min, π/2 − α_min]` and `β ∈ [0, π]` (dd_decoder §3.2), closed (R-417);
- each free momentum `q ∈ [−q_max, q_max]` (dd_decoder §3.4), closed (R-417), whose cap is the table's symmetric row;
- the two lines, which no block uses.

An interval `(a, b)` has width `b − a` and half-width `c = (b − a)/2`; on the symmetric cap, `a = −c`. A link is
selected per block control, only among the entries onto that control's codomain (REQ-GEN-013). The defaults are
`softmax_tanh` (mass), `sigmoid_alpha` and `sigmoid_beta` (config), and `sigmoid_q` (each free momentum).

**The closed range and the f32 images of π/2 and π (R-417).** In exact arithmetic a bounded link's forward never
reaches its bounds; in floating point it does, by its own rounding, so each codomain above is closed. A saturated
control reaches each lower end where the link's output underflows or its `tanh` or softsign rounds to −1 (`α = 0`,
`β = 0`, `q = −q_max`), at f32 and f64, and `q`'s upper end `q_max`, which is exact (2). The upper ends `π/2` and `π` are irrational, so the generator emits their f32
images rounded toward zero, `fl↓(π/2)` (bits `0x3FC9_0FDA`) and `fl↓(π)` (`0x4049_0FDA`), not the nearest values, which
lie above them; the f64 nearest values already lie below. So no bounded link's image passes its upper bound at either
precision. The rounding is the emitter's: a tree's `num` is the f64 value's bits, so the hashed bytes do not change.
No link clamps its output (validation ground-truth note: no endpoint clamping, no artificial guards).

| Entry | Codomain | Forward | Inverse (with its ε clamp) | log-det | Sampling note |
|---|---|---|---|---|---|
| `softmax_tanh` | Δ² | `μₖ = μ_max·tanh zₖ`, `m = softmax(0, μ₁, μ₂)` | `zₖ = artanh(clamp(log(mₖ/m₀), ±(1−ε_μ)μ_max)/μ_max)` | `½ log 3 + Σᵢ log mᵢ + 2 log μ_max + log sech² z₁ + log sech² z₂` | under-samples simplex edges/corners |
| `stick_breaking` (R-72) | Δ² | `s = ½(1 + (1−ε_μ) tanh z₁)`, `u = ½(1 + (1−ε_μ) tanh z₂)`, `m = (1−s, s(1−u), su)` | `s = m₁ + m₂`, `u = m₂/s`, each `v` of them to `artanh(clamp(2v−1, ±(1−ε_μ)²)/(1−ε_μ))` | `log(√3/4 · (1−ε_μ)² · s · sech² z₁ · sech² z₂)` | reaches the edges and corners that `softmax_tanh` stops short of; its area element is `4√3·m₀m₁m₂` (exactly so at `ε_μ = 0`), the plain softmax measure, so it over-samples every edge and corner, symmetrically in the bodies |
| `sigmoid_alpha`, `sigmoid_beta`, `sigmoid_q` | α, β, q | `a + (b−a)·σ(x)` | `logit(clamp((y−a)/(b−a), ε, 1−ε))` | `log((b−a)·σ(x)·σ(−x))` | edge-heavy: crowds the bounds |
| `tanh_alpha`, `tanh_beta`, `tanh_q` | α, β, q | `a + c·(1 + tanh x)` | `artanh(clamp((y−a)/c − 1, ±(1−ε)))` | `log(c·sech² x)` | the bounded-alt row's note (α, β); the symmetric row's (q) |
| `softsign_alpha`, `softsign_beta`, `softsign_q` (R-72) | α, β, q | `a + c·(1 + u)`, `u = x/(2 + abs(x))` | `x = 2u/(1 − abs(u))`, `u = clamp((y−a)/c − 1, ±(1−ε))` | `log(2c) − 2 log(2 + abs(x))` | heavier-tailed than σ: it nears each bound as `1/abs(x)`, not as `e^(−abs(x))`, so relative to σ it under-samples the bounds' immediate neighbourhoods |
| `softplus` | (0, ∞) | `softplus x` | `inv_softplus y` | `log σ(x)` | linear for large `x`, so not heavy-tailed |
| `exp` | (0, ∞) | `eˣ` | `log y` | `x` | exp is heavy-tailed |
| `identity` | ℝ | `x` | `y` | 0 | neutral |

Every sampling note reads "over-" and "under-samples" as the density of samples drawn with uniform controls, not as
the link's Jacobian weight (R-413), as chart_decoder_contract § "Integrity: the link is part of the experiment" has it
("which region of the block it over- and under-samples relative to uniform"). So σ's note is "edge-heavy: crowds the
bounds": with uniform controls, every control past a few units lands in a sliver at each bound.

`ε` is `ε_μ` on the simplex, `ε_z` onto α and β, and `ε_q` onto a momentum. `abs(x)` is written `clamp(x, −x, +∞)`
in the closed operator list. Each clamp keeps the inverse finite at the codomain's boundary, where a float forward
saturates. In σ's units, `s = (1 + u)/2`, the tanh and softsign clamp `±(1 − ε)` is `[ε/2, 1 − ε/2]`, half the σ
clamp `[ε, 1 − ε]`.

The two simplex inverses are defined on the open simplex. At an exact corner, with two masses 0, `softmax_tanh`'s
`log(mₖ/m₀)` and `stick_breaking`'s `u = m₂/(m₁ + m₂)` divide 0 by 0. On the host, Rust's `max` ignores the NaN, so
the clamp returns a finite value, but SPIR-V's `FMax` with a NaN operand is undefined, so a caller passes no exact
corner (TASK-M2-15's encode).

The trees write some terms in a form equal in algebra to the table's but with no step that overflows or cancels short
of the float's own range (canonical_spec §9, walls 7 and 9, measure honesty and totality). At f32 and f64, with `MAX`
the float's largest finite value, each `log sech² x` is about `log 4 − 2·abs(x)`, so an entry with that term has a
log-det that is −∞ exactly when the computed `2·abs(x)` overflows, for one control (`tanh_*`: past `abs(x) = MAX/2`),
or the computed sum `2·abs(z₁) + 2·abs(z₂)` overflows, for two (`softmax_tanh`, `stick_breaking`), and finite
elsewhere. That edge is within half an ulp of where the true value passes `−MAX`, but not always on it: where the
computed sum is an exact tie at `MAX` plus half an ulp, ties-to-even gives −∞ even where the true value, with its
constant terms, rounds to `−MAX` (as for `stick_breaking` with `z₁ > 0`, whose constant terms sum to a positive
number). The other entries are finite for every finite control. The
forms: each `log sech² x` as `2·(log 2 − abs(x) − softplus(−2·abs(x)))`, each `log σ(x)` as `−softplus(−x)`, and
stick-breaking's `s` and `1 − s` as `σ(2z₁) − ½ε_μ·tanh z₁` and `σ(−2z₁) + ½ε_μ·tanh z₁` (and `u`, `1 − u` likewise
in `z₂`).

- **The edge-reaching simplex link** *(definition, R-72; REQ-GEN-026)* is the stick-breaking map above. Where
  `softmax_tanh` stops at mass ratios `e^(±2μ_max)`, it reaches within `ε_μ/2` of each edge and of the corner
  `m₀ = 1`, and within about `ε_μ` of the corners `m₁ = 1` and `m₂ = 1`. Its Jacobian is 3×2, so its log-det is
  `log √det(JᵀJ)` (R-368), where `det(JᵀJ) = 3·(s'u')²·s²`, with `s' = ½(1−ε_μ) sech² z₁` and
  `u' = ½(1−ε_μ) sech² z₂`. At `ε_μ = 0`, `s' = 2s(1−s)` and `u' = 2u(1−u)`, so its area element is `4√3·m₀m₁m₂`:
  symmetric in the bodies, and the measure of plain softmax, from which `softmax_tanh` departs only by its saturation
  factor. With uniform controls it therefore over-samples every edge and corner. Near
  the corner `m₂ → 1`, `m₀ + m₁` falls to about `ε_μ`, where dd_decoder's `DEGENERATE(M01_TINY)` fence (`M₀₁ < ε`) can
  fire.
- **The heavier-tailed bounded link** *(definition, R-72; REQ-GEN-026)* is softsign with σ's slope at the centre,
  `u = x/(2 + abs(x))`, onto α, β and the momenta. Its tails approach the bounds as `1/abs(x)` rather than
  exponentially. Its log-det is `log(c·u')`, with `u' = 2/(2 + abs(x))²`. Its second derivative jumps at `x = 0`, so it
  is C¹ but not C².
- **Temperature-softmax** is not registered. The corpus gives no formula or parameter for it, and the two definitions
  above already give each block a second link with a different sampling note (REQ-GEN-014).
- **No new entry until a study needs one (R-519).** None is added now: not a temperature-softmax, a bounded map
  heavier-tailed than softsign, or a simplex map uniform over the simplex. A study that needs a link the registry
  lacks, to stress its claim (chart_decoder_contract § "Integrity: the link is part of the experiment"), adds it by its
  own task, as a definition (R-72) with its sampling note and its parameters' declarations, under physics review and
  the check above.

---

## 4. Seams (obligations → integration tests)

| Seam | Obligation | Test |
|---|---|---|
| **13** one-source generation | pack, unpack, export decoder, catalogue emitted from §3 only | mutate one ledger entry → all four artefacts change together; a hand-edit to any generated file is detected (generated-file guard) |
| **5 / Memory** | the payload's physical truth is this ledger | the sub-field debug views (visual bit-layout unit test) show live, sane values per field |
| **2 / 8** links | decode consumes registry link functions (generated Rust); encode consumes registry inverses | link swap ⇒ recompile + re-integrate; T2 round-trips through registry inverses only |
| **sim key** | schema version (the ledger's content hash over its layout and kernel columns, R-36, R-545) ∈ payload compatibility signature | a layout or kernel change invalidates every cached payload (cache serves nothing stale-schema'd); a `scale` edit does not (RQ-297). *Was:* "any ledger change invalidates every cached payload" |
| **Observation ring** | catalogue exhaustive by construction | new field appears in the picker automatically or generation fails |

---

## 5. Tests (properties any generator must satisfy)

1. **Disjointness & coverage (static):** within each packed word, fields never overlap; declared bits are covered or explicitly reserved; widths fit ranges (word length ~76 fits 7 bits with mixed-radix packing; note `encounter_count`/`orbit_count`/`total_substeps_log2` are all DERIVED, not packed fields — the last from the exact `total_substeps` u32, §3.1).
2. **Pack∘unpack = id**, per field, property-fuzzed over the full value range — in **Rust** (host + the kernel's own pack/unpack) *and* in a GPU self-test dispatch of the **WGSL fragment** unpack (which is where the `i32-extractBits` sign-extension trap lives — u32 overload only; the Rust side has no `extractBits`, so that trap is fragment-specific).
3. **f16 pairs** round-trip via `pack2x16float`/`unpack2x16float` within f16 eps.
4. **Exact step indices:** `t_end_step`/`t_dmin_step` (in `times`) round-trip exactly as u16 — no fixed-point, no Q0.16 (R-86); dispatch refuses a configuration with `⌈T/dt⌉ > 65535`; bit-identical CPU/GPU on identical inputs (parity).
5. **Sentinels:** a stored sentinel survives pack/unpack bit-exact; catalogue styles it, never scales it. (`diffusion` is no longer one: an invalid fit reads NaN, R-245.)
6. **Metadata gate:** delete any entry's `scale` → generation fails with the field named.
7. **Schema-version discipline:** the version is the hash of the canonicalised §3 table (R-36), §3.9's link registry included (R-340), so flipping one bit-offset changes it and the signature, with no number to forget to bump; the cache test then proves zero stale-schema payloads are ever served.
8. **Registry properties, per link:** (a) constraint preservation ∀ inputs incl. saturation (simplex outputs sum to 1 and stay positive; bounded outputs in the closed range `[a, b]`, compared at the exact bounds, not their rounded images: R-417); (b) inverse round-trip within ε-clamp tolerance, asserted in *physical* units; (c) analytic log-det matches a numeric Jacobian to tolerance across the domain; (d) C¹: central-difference derivative continuous across the range (no kinks).
9. **Union-field semantics:** `detail` renders/decodes per `state` — an escape's detail is a body id, a collision's a pair id; the catalogue's detail view switches legend accordingly.

---

## 6. Deferred / flagged

- **Adopted (R-36): the schema version is a content hash of the ledger, not a hand-bumped integer.** A layout edit without a version bump is the drift catastrophe seam 13 exists to prevent — deriving the version (hash of the canonicalised table) makes the failure impossible rather than merely forbidden. *Was: recommendation to adopt.*
- **`free_group_word` length field is 7 bits (mixed-radix ~76-symbol capacity)** — reserved means reserved; any future use is a ledger edit (⇒ version change) not an opportunistic squat.
- ~~**`QuadReduction` completion** (§3.7)~~ — **done.** Not a transcription task after all: the older source held only prose, so the member list was derived from consumers (scheduler Part 6) and from measurement (`principia_dd_refinement_criterion.md`). **Pending change 1 (the `dominant_outcome` grain) is dissolved rather than decided** — defining every event-derived field at the joint `class ⊕ detail` grain removes the two-grain problem entirely, so no class-only companion field is needed.
- **Drift-sign presentation** — the diverging-scale metadata for `energy_drift`/`Lz_drift` is a generator requirement, recorded here so the catalogue doesn't ship them as broken sequential-log views.
- **Body-index naming** — settled 0-based (R-22); this ledger's `ICDescriptor` names changed with it (⇒ schema version change, correctly — automatic under R-36). What a person reads is 1-based (R-449); the ledger's names and the exports stay 0-based and say so (§3.8).

---

*Two tables, four artefacts, one law: nothing about a bit exists twice. The version is the hash. Reserved means reserved. And a field without metadata does not ship.*
