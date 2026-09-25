# Principia — symbolic-dynamics contract (free-group word interpretation)

*Moved out of the storage-layout spec (payload §3 stores the word; this contract *interprets* it). The word itself — mixed-radix packing, 76-symbol capacity, the frozen continuation table, append/decode, truncation — is fully specified in `principia_dd_simstate_payload.md §3`. What lives **here** is the topological-dynamics layer: what the symbols mean geometrically, and how to derive body-pair quantities from a word. This is a topological-dynamics problem, not a payload-bits problem, and it was blocking the payload unnecessarily.*

---

## Status: OPEN — specification required before per-pair quantities are trusted

The following are **not yet an authoritative derivation** and must be specified here before use. **How each is settled
(R-125):** §1 is transcribed at M3 by the task that writes the word; §2 and §3 are **v2** — per-pair views are out of v1
(PL-3, R-38).

### 1. Generator ↔ branch-cut convention
The word is over the free group `F₂ = ⟨a, b⟩` on the shape sphere with the three binary-collision points removed. Fix, normatively:
- which two branch cuts correspond to generators `a` and `b` (arc from which collision point to the Lagrange reference pole);
- the crossing-direction sign convention (which crossing direction is the generator vs its inverse) — must match the half-open sign convention in payload §3 / the integrator contract.

**Settled how (R-125):** the a/b assignment and the crossing sign are **transcribed from the literature** (Montgomery;
Šuvakov–Dmitrašinović), with citations, by the task that writes the word (M3); verified against the published braid
classes, physics-reviewed, and confirmed at the gate.

### 2. The punctured-sphere relation (third pair)
**v2 (R-125):** per-pair views are out of v1 (PL-3, R-38).

Two generators suffice for three punctures because a loop around the third collision point is expressible via `a` and `b` through the fundamental-group relation of the thrice-punctured sphere. **Specify that relation explicitly** — it is what lets the third body-pair's encounters be attributed from a two-generator word.

### 3. Third-pair attribution algorithm
**v2 (R-125):** per-pair views are out of v1 (PL-3, R-38).

Given the reduced word (a sequence of `a/A/b/B`), specify the **deterministic algorithm** that maps branch-cut crossings to the three **body-pairs** (01/02/12) and produces:
- `enc_01`, `enc_02`, `enc_12` — per-pair crossing tallies;
- `dominant_pair = argmax` of those.

This is **not** a symbol histogram — a two-generator word does not yield three pair-tallies by counting symbol types. The mapping requires the generator↔cut convention (1), the punctured-sphere relation (2), and the attribution rule (3) together. **Until all three are specified, `dominant_pair` is not deterministic or reproducible.**

### 4. Validity under truncation
Per payload §3, a **truncated** word (`fgw_truncated`) has an unknown reduced form (later cancellations untracked after the length cap). So all per-pair quantities derived here are **invalid for truncated samples** — the derivation must check `fgw_reduced_length_valid` and surface invalidity, not silently produce numbers.

---

## What is already settled (in the payload spec, not here)

- The word storage, packing, and the frozen continuation table (payload §3 — binary format).
- `fgw_length_raw` / `fgw_truncated` / `fgw_reduced_length(_valid)` accessors (payload §6).
- That `reduced crossing count = fgw_reduced_length` (net branch-cut crossings, free-reduced) — this is a *count*, valid when not truncated. The **per-pair split** is what remains open here.
- `dmin_pair` is a **stored latched fact** in the descriptor (payload §2), NOT derived from the word — unaffected by anything in this contract.

---

## Downstream consumers waiting on this contract

- **Symbolic-spread `S_word`** (scheduler / sampling-SSAA note) — the resolve-stage reduction over a footprint's E+1 words. It compares *whole words* and does not need pair-attribution, so it is **not** blocked by this contract (it works on the raw reduced words). Only the *per-pair* interpretation is blocked.
- **Paper 2 / quantitative work** — any per-pair encounter statistics.
- **`dominant_pair` colour mode** — blocked until §3 is specified; v2 (R-125).

*This contract exists so the payload/storage layer can be marked authoritative without waiting on the topological attribution algorithm. The word is stored correctly; interpreting it into pair statistics is a separable, deferred piece of work.*
