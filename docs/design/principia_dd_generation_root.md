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

From the **caching/integrator contracts**: the **payload compatibility signature** includes the payload schema version — a layout change must invalidate every cached payload. **The schema version is a content hash of the canonicalised §3 ledger, not a hand-bumped integer (R-36).**

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
| `saturated` | 5 | 1 | sticky flag — set iff `N_sub == N_max` occurred at some macro-step (the integrator hit its per-step subdivision cap; the trajectory continued, advance-and-flag — payload §2, R-86). `N_max` is a raised tunable (integrator contract), not hardcoded |
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

> **Architecture: the word lives in its own storage buffer, NOT inline in `SimState`.** Split by *access pattern*, not data type. Every other `SimState` field is **hot** — read/updated by the march kernel *every substep* (`r,p`; the shadow, which marches in lockstep; `S,θ̃`; Welford `mean_y,C_ty`; `E_0,Lz_0` read for the drift-max latch; `dE_max,dLz_max`; `times`; `d_min`; the descriptor). The word alone is **cold**: appended only on encounter *events* (a few dozen times over a ~200k-step march) and read only at resolve (symbolic-spread) and inspect (display) — *never* in the per-step march. Co-locating it inline wasted bandwidth two ways: (1) it made `SimState` 16-byte-aligned (the `vec4` is the sole 16-align driver), padding the struct tail with **12 dead bytes**; (2) every hot cache line the integrator loaded dragged 16 B of word it didn't need that step. **Pulling the word into a parallel buffer** (indexed identically to samples — per-copy, same sample→index map, 16 B/entry) drops `SimState` to **8-byte alignment** (largest remaining member `array<vec2>`), kills the 12 B padding (→ only 4 B residual), shrinks the **hot** per-sample footprint 160→136 B effective (FTLE-on) / 112→88 B (FTLE-off) — **144 / 96 B** since the 8 B closure field (payload §1, §7) — a ~15% cut in the persistent hot-state footprint, expected to reduce storage traffic and working-set pressure (the bottleneck is unconfirmed — payload §3, §8) — and lets the word buffer be sized/tiered independently (drop word features → free the whole buffer without touching integration state). Cost: one extra buffer binding + an indexed word fetch on the *cold* paths (resolve/inspect) only. If the march is bandwidth-bound, that trades rare-path fetches for hot-path bandwidth; whether it is depends on dispatch granularity and is to be measured (payload §3). **Only the word moves** — the physics/accumulators/refs are all hot and co-accessed every step, so splitting them further would fragment co-accessed data and hurt the prefetcher; two buffers (hot `SimState` + cold word) is the sweet spot.

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

**Small fixed tables (shader constants):** `inverse(s)` (4 entries); `continuation_index(prev,s)→{0,1,2}` and its inverse `continuation_symbol(prev,e)→s` (the 3 legal continuations per prev, fixed order); and the reverse `predecessor_symbol(next,e)→prev`, mandatory for the O(1) pop (payload §3's frozen table).

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
- **Alignment payoff:** removing the `vec4` from `SimState` drops the struct from 16-byte to 8-byte alignment (largest remaining member `array<vec2>`), eliminating 12 B of vec4-forced tail padding and shrinking the hot per-sample footprint 160→136 B (FTLE-on) / 112→88 B (FTLE-off); with the 8 B closure field it is **144 B effective (FTLE-on) / 96 B (FTLE-off)** (payload §1, §7) — a ~15% cut in the persistent hot-state footprint (the bottleneck is unconfirmed — payload §3).


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
> Per-sample state: **`mean_y, C_ty`** (2 × f32). The time-only moments `n, mean_t, C_tt` are **DERIVED, not a shared mutable global** — WGSL has no dispatch-wide barrier to publish one safely mid-dispatch. For uniform sampling they are closed-form: `n = step_count`, `mean_t = (n+1)h/2`, `C_tt(n) = h²·n(n²−1)/12`. The per-sample covariance update uses the **old-mean** time deviation `δ_t = 0.5·n·h` (not `t − mean_t` post-insertion, which is wrong). Slope `C_ty/C_tt(n)`, invalid (sentinel) for `n<2`. Must stay f32 (means/co-moments precision-sensitive). (Full detail: payload spec §4.)

| Field | Scale | Range / notes |
|---|---|---|
| `t_end_step` (→ fraction) | lin | [0, T] via `/horizon_steps` — **normative meaning = completed-step count at ALL times** (running: advances each step; terminal: latched; initial: 0 — payload §2; no sentinel needed); EXACT u16 |
| `d_min` | log | > 0 |
| `ftle` | lin | tier-gated (`ftle_valid`). **A *point* quantity — every sample (base and each ensemble copy) computes its own** from its Benettin shadow, so it anti-aliases under the SSAA resolve (sampling/SSAA note) |
| `energy_drift` | **diverging** (signed) | log-magnitude styling; floor `eps_E` |
| `diffusion` | lin | **sentinel −1.0** = fit invalid |
| `delta_E_max_abs` | log | ≥ 0 |
| `Lz_drift` | **diverging** (signed) | floor `eps_L` |
| `delta_Lz_max_abs` | log | ≥ 0 |
| `E_0` | diverging | must equal `K₀+V₀` (cross-check view) |
| `Lz_0` | diverging | invariant-chart gradient view input |
| `closure_min` | log | ≥ 0, f32; the running minimum of `|n̂(t) − n̂(0)|` once the shape has departed by `δ_dep` (R-37, payload §1) |
| `closure_step` | lin | exact u16 step index of the minimum (the period label); binary-parity surface, same convention as `t_dmin_step` |

> Metadata nuance the catalogue generator needs: **the two drift fields are signed** → *diverging* colour scale, not sequential-log. (Finding, §6.)

### 3.5 The live-state block (replaces the checkpoint array — lockstep, ratified)

**There is no stored trajectory.** The shape vector `n` is *derived from the current state each step* (Montgomery map, integrator dd §3.7) — never stored. What persists per sample is fixed-size, O(1) in `t`:

| Group | Contents | Notes |
|---|---|---|
| **Phase state** | `r, p` — 12 × f32 (vec2-grouped, `array<vec2<f32>, 3>` each; 8-byte aligned — payload §1, R-86) | the marching state at the playhead |
| **Shadow states** (tier-gated) | 1 Benettin shadow **per sample** (base and each ensemble copy), 12 × f32 each | **resident under lockstep — they march too.** Every sample is a full, uniform `SimState` computing its own FTLE (the sampling/SSAA decision: uniformity over micro-saving), so its FTLE shadow rides with it. Tier gates existence; `contains-ensemble` / `contains-FTLE` advertise it. The shadow is an *ingredient* of the sample's `ftle` field — never itself a coloured sample (renormalisation corrupts its endpoint) |
| **Ensemble copies** (tier-gated) | E full uniform `SimState`s per nominal sample — a footprint has **E+1 samples, `copy_index` 0..E**: copy 0 is the un-jittered centre, copies 1..E sit at **Halton (2,3) points 1..E**, centred (minus ½) and scaled to the footprint (R-80) | structurally identical to the base (colour through any render graph, no special-casing) but **scheduler-leaves** — a copy never spawns its own ensemble (`ENSEMBLE_ENABLED` applies only to nominal samples; no recursion). They double as the SSAA sample pool *and* the spread-metric pool. Per-pixel ≈ `2(E+1)` trajectories (each sample + its shadow); E is the tier's free-valued SSAA knob (0–15; High = 3, Extreme = 15 — memory-tiers) |
| **Running accumulators** | Benettin `S` (f32); unwrapped phase `θ̃` (f32 — `orbit_count`/`retrograde` derived at read); drift running-final + running-max pairs; **diffusion via Welford streaming regression** — per-sample `mean_y, C_ty` (2 × f32); the time-only terms `n, mean_t, C_tt` are **the same for all samples** (lockstep synchronises `t`) and are **DERIVED closed-form** (from `step_count`), not stored per-sample or in a mutable global — see the invariant | ⚠ **INVARIANT (footgun 1): diffusion is accumulated as *streaming regression state*, never as `(t, y)` points collected for a later fit** — points regrow the history the reversal deleted. **Welford, not raw moments:** the old raw-moment form (`Σt, Σt², Σy, Σty, Σy²`) computes slope as `(nΣty−ΣtΣy)/(nΣt²−(Σt)²)` — a *catastrophic-cancellation* difference of large nearly-equal products, precision-risky even at f32 as the sums grow over ~10⁵ steps. Welford tracks *centered* co-moments (running means + `C_tt, C_ty`), so **slope = `C_ty / C_tt`** directly — stable, no cancellation, no unbounded sums. **Lockstep — time-moments are DERIVED, not a shared mutable global** (WGSL has no dispatch-wide barrier to publish one safely mid-dispatch). For uniform sampling: `n=step_count`, `mean_t=(n+1)h/2`, **`C_tt(n)=h²·n(n²−1)/12`** (closed form). The per-sample covariance update uses the **OLD-mean** time deviation `δ_t=0.5·n·h` (NOT `t−mean_t` post-insertion, which is wrong): `mean_y += (y−mean_y)/n; C_ty += δ_t·(y−mean_y)`. Slope `C_ty/C_tt(n)`, **invalid for `n<2`** (sentinel, no divide-by-zero). Only `mean_y, C_ty` are stored. (Non-uniform schedule → small precomputed prefix table, still read-only.) Net: **2 per-sample f32** (was 5), *and* numerically robust. Slope + R² derived from `(C_ty, C_tt, mean_y, mean_t, n)` at any playhead |

Terminal latch: on termination the whole block freezes (state stops advancing, accumulators stop updating).

**Ensemble spread is NOT a stored field — it is derived at the resolve stage.** Spread is a *footprint* quantity (one value per nominal sample), computed in the shader at the resolve stage from the E+1 samples of a footprint — the data-side twin of the SSAA colour resolve (colours→pixel colour on the render side; outcomes→spread scalar on the data side). It is consumed *live* for display (colour-by-spread) and aggregated into `QuadReduction` for the scheduler's refinement signal. It carries no per-sample `SimState` field: storing a group property on each sample is wrong-shaped and costs an f32 ×`2(E+1)`×viewport for nothing. **No per-sample "is ensemble" tag either** — grouping is structural (E+1 samples per footprint in the dispatch layout), `copy_index` marks nominal (0) vs copies (1..E), and ensemble-on/off is a quad/tier flag (`contains-ensemble`).

### 3.6 `ICDescriptor` (12 × f32)

`m0 m1 m2` (0-based body indices, R-22), `q_mass`, `rho_mag`, `lambda_mag`, `rho_ratio` (log), `rho_angle` (**cyclic**), `K_0`, `V_0` (diverging), `virial_ratio`, `r_min_pair_0` (log). Provenance: decode stage, pre-integration.

**64 B with explicit padding** (R-86): the 12 × f32 fields are 48 B, and the remaining 16 B are declared padding, never
implicit. `E₀` is **derived** (`K_0 + V_0`), not stored.

### 3.7 `QuadReduction` — completed ledger

**Status: complete.** The earlier instruction (to transcribe the remaining members verbatim from an older source) was
stale — that source held only a prose sentence, not an enumeration. The member list below is instead **derived from consumers** (scheduler
contract Part 6 names every field its predicates read) and **from measurement**: every inclusion and
exclusion traces to `principia_dd_refinement_criterion.md`. Sections cited as `[RC §n]`.

**Size.** The `~80 B` figure quoted in earlier docs is **descriptive, not a cap** — grep confirms it
is always written with a tilde, and the temporal accumulators were added without changing it. There
is no memory pressure: `QuadReduction` is per *visible quad* (thousands), against `SimState` at
per *sample* (millions). At ~4k quads the whole array is ~0.3 MB against ~140 MB of `SimState`. The
real constraint is **categorical, not numerical** — *reduce before evaporate*: fixed-size scalars
only, never a history. Size the struct from the member list, then align, then update the figure.

#### Identity

| member | type | note |
|---|---|---|
| `level` | u8 | quadtree depth; feeds the `ℓ < camera_depth + MAX_REL_DEPTH` guard |

`id` is **omitted**: the CPU indexes the readback by the quad it asked for, so identity is positional.

#### Outcome (all at joint `class ⊕ detail` grain)

| member | type | note |
|---|---|---|
| `class_histogram[N]` | u8 × N | derives dominant *and* impurity from **one** source, so they cannot disagree |
| `dominant_outcome` | packed | `class ⊕ detail`, 5 bits |
| `outcome_impurity` | f16 | `1 − max(class fraction)` |
| `terminated_fraction` | f16 | **required** — without it "pure because settled" is indistinguishable from "pure because nothing has happened yet" [RC §7] |

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
| `spread_winner` | 2 bits | which contributor won the `max`; makes parent↔child switching visible [RC §7.13] |

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
| `worst_energy_drift` | f16 | input to the per-copy classifier that sets `failed_fraction` |

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
4. **`failed_fraction > 0.10` detects estimator failure** with one false alarm in six. Note there is
   **no better-estimator fallback**: Theil–Sen *is* two-point OLS at two scales (verified to 8.7e-10),
   so its advantage came from consuming more scales. The fallback is *acquire a third scale*
   [RC §7.15e].

#### Temporal accumulators (scheduler Part 8)

| member | type | note |
|---|---|---|
| `running_max_divergence` | f32 | max-updated, **latching** |
| `running_mean_divergence` | f32 | |
| `divergence_trend` | f32 | EWMA |
| `first_divergence_t` | f32 | write-once; sentinel until crossed |

**f32, not f16** — these accumulate in place over thousands of steps, where f16 would drift. Contrast
the thresholded scalars above, which are compared against `τ` and need no more precision.

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
| `escape_time_min` / `_max` | f16 × 2 | diagnostic range |
| `retrograde_fraction` | f16 | tiebreaker |
| `mean_orbit_count_spread` | f16 | tiebreaker |

#### Conditional — not yet included

| member | condition |
|---|---|
| `spread_t_end` | `alpha ≈ 0.86–0.95` once the escape time is **bisected inside the firing interval** rather than recorded at a sync boundary — its resolution otherwise *is* the sync interval, which was mistaken for absence of signal. Trust only where escape fraction ≈ 1.0 [RC §7.15f] |

#### Not reduction fields

- **Parent–child disagreement** needs no member: the CPU holds both reductions and compares them.
  That comparison **is** the `alpha` estimator and should be specified as such.
- **`priority`, `cacheAge`, `lifecycle`, `computeCostMs`, `gentime`** are CPU-computed quad metadata,
  not GPU reductions. A fragment shader cannot read them from this struct; they reach the display via
  tile debug shaders (render GUI spec §12.1).

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

**Long-horizon behaviour is now a reporting property rather than a limit.** With `failed_fraction`
as a contributor, a pixel whose copies cannot be integrated reads **indeterminate** — which is true —
instead of emitting an exponent computed from the tame minority that survived. Measured, near-field
at `t=80`: `failed_fraction = 0.56`, `ensemble_spread = 0.664`. And the refinement decision follows
automatically: as failures accumulate, parent and child spreads both saturate, `alpha → 0`, and the
quad **floors** — correct, since refining does not make a close encounter easier. Still open: gate threshold and integrator tolerance must be specified **as a pair**
(at `eta=0.005` the gate stops mattering; at `eta=0.02` no threshold reaches the trust bar) [RC §7.14a].

### 3.8 Metadata schema (what every entry must carry)

```
{ name, location: (word, offset, width) | scalar-index,
  type: u-bits | f32 | f16-pair | fixed16,
  scale: lin | log | cyclic | diverging | categorical(n) | flag,
  range, sentinel?, tier_gate?,
  provenance: kernel | decode | reduction | cpu,
  consumers: [render, export, debug, scheduler] }
```

**A field without a complete entry fails generation loudly.** Coverage is enforced, not hoped for.

### 3.9 The link registry (consolidated from chart contract Part 2.5)

| Constraint | Link (forward) | Inverse | log-det | Sampling note |
|---|---|---|---|---|
| Simplex Δ² | softmax ∘ `μ_max·tanh` | log-ratio → `artanh`, clamp `(1−ε_μ)μ_max` | softmax Jacobian × sech² factors | under-samples simplex edges/corners |
| Bounded (a,b) | scaled/shifted `σ` | `logit`, clamp `[ε_z, 1−ε_z]` | `(b−a)·σ'` | centre-heavy vs uniform |
| Bounded alt | scaled `tanh` | `artanh` | `c·sech²` | interchangeable with σ up to reparam (`tanh x = 2σ(2x)−1`) — differs in slope profile only |
| Positive (0,∞) | softplus / exp | inverse-softplus / log | `σ(x)` / `eˣ` | exp is heavy-tailed |
| Symmetric (−c,c) | `c·tanh` | `artanh` | `c·sech²` | as bounded |
| Unbounded ℝ | identity | identity | 1 | neutral |

Each entry ships **forward, inverse, log-det, ε clamps, and the sampling note** — the robustness-sweep picker uses the last to choose alternatives that *disagree where it matters*.

---

## 4. Seams (obligations → integration tests)

| Seam | Obligation | Test |
|---|---|---|
| **13** one-source generation | pack, unpack, export decoder, catalogue emitted from §3 only | mutate one ledger entry → all four artefacts change together; a hand-edit to any generated file is detected (generated-file guard) |
| **5 / Memory** | the payload's physical truth is this ledger | the sub-field debug views (visual bit-layout unit test) show live, sane values per field |
| **2 / 8** links | decode consumes registry link functions (generated Rust); encode consumes registry inverses | link swap ⇒ recompile + re-integrate; T2 round-trips through registry inverses only |
| **sim key** | schema version (the ledger's content hash, R-36) ∈ payload compatibility signature | any ledger change invalidates every cached payload (cache serves nothing stale-schema'd) |
| **Observation ring** | catalogue exhaustive by construction | new field appears in the picker automatically or generation fails |

---

## 5. Tests (properties any generator must satisfy)

1. **Disjointness & coverage (static):** within each packed word, fields never overlap; declared bits are covered or explicitly reserved; widths fit ranges (word length ~76 fits 7 bits with mixed-radix packing; note `encounter_count`/`orbit_count`/`total_substeps_log2` are all DERIVED, not packed fields — the last from the exact `total_substeps` u32, §3.1).
2. **Pack∘unpack = id**, per field, property-fuzzed over the full value range — in **Rust** (host + the kernel's own pack/unpack) *and* in a GPU self-test dispatch of the **WGSL fragment** unpack (which is where the `i32-extractBits` sign-extension trap lives — u32 overload only; the Rust side has no `extractBits`, so that trap is fragment-specific).
3. **f16 pairs** round-trip via `pack2x16float`/`unpack2x16float` within f16 eps.
4. **Exact step indices:** `t_end_step`/`t_dmin_step` (in `times`) round-trip exactly as u16 — no fixed-point, no Q0.16 (R-86); dispatch refuses a configuration with `⌈T/dt⌉ > 65535`; bit-identical CPU/GPU on identical inputs (parity).
5. **Sentinels:** `diffusion = −1.0` survives pack/unpack bit-exact; catalogue styles it, never scales it.
6. **Metadata gate:** delete any entry's `scale` → generation fails with the field named.
7. **Schema-version discipline:** the version is the hash of the canonicalised §3 table (R-36), so flipping one bit-offset changes it and the signature, with no number to forget to bump; the cache test then proves zero stale-schema payloads are ever served.
8. **Registry properties, per link:** (a) constraint preservation ∀ inputs incl. saturation (simplex outputs sum to 1 and stay positive; bounded outputs in range); (b) inverse round-trip within ε-clamp tolerance, asserted in *physical* units; (c) analytic log-det matches a numeric Jacobian to tolerance across the domain; (d) C¹: central-difference derivative continuous across the range (no kinks).
9. **Union-field semantics:** `detail` renders/decodes per `state` — an escape's detail is a body id, a collision's a pair id; the catalogue's detail view switches legend accordingly.

---

## 6. Deferred / flagged

- **Adopted (R-36): the schema version is a content hash of the ledger, not a hand-bumped integer.** A layout edit without a version bump is the drift catastrophe seam 13 exists to prevent — deriving the version (hash of the canonicalised table) makes the failure impossible rather than merely forbidden. *Was: recommendation to adopt.*
- **`free_group_word` length field is 7 bits (mixed-radix ~76-symbol capacity)** — reserved means reserved; any future use is a ledger edit (⇒ version change) not an opportunistic squat.
- ~~**`QuadReduction` completion** (§3.7)~~ — **done.** Not a transcription task after all: the older source held only prose, so the member list was derived from consumers (scheduler Part 6) and from measurement (`principia_dd_refinement_criterion.md`). **Pending change 1 (the `dominant_outcome` grain) is dissolved rather than decided** — defining every event-derived field at the joint `class ⊕ detail` grain removes the two-grain problem entirely, so no class-only companion field is needed.
- **Drift-sign presentation** — the diverging-scale metadata for `energy_drift`/`Lz_drift` is a generator requirement, recorded here so the catalogue doesn't ship them as broken sequential-log views.
- **Body-index naming** — settled 0-based (R-22); this ledger's `ICDescriptor` names changed with it (⇒ schema version change, correctly — automatic under R-36).

---

*Two tables, four artefacts, one law: nothing about a bit exists twice. The version is the hash. Reserved means reserved. And a field without metadata does not ship.*
