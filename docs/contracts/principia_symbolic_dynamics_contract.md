# Principia — symbolic-dynamics contract (free-group word interpretation)

*Moved out of the storage-layout spec (payload §3 stores the word; this contract *interprets* it). The word itself — mixed-radix packing, 76-symbol capacity, the frozen continuation table, append/decode, truncation — is fully specified in `principia_dd_simstate_payload.md §3`. What lives **here** is the topological-dynamics layer: what the symbols mean geometrically, and how to derive body-pair quantities from a word. This is a topological-dynamics problem, not a payload-bits problem, and it was blocking the payload unnecessarily.*

---

## Status: OPEN — specification required before per-pair quantities are trusted

The following are **not yet an authoritative derivation** and must be specified here before use. **How each is settled
(R-125):** §1 is transcribed at M3 by the task that writes the word; §2 and §3 are **v2** — per-pair views are out of v1
(PL-3, R-38).

### 1. Generator ↔ branch-cut convention
The word is over the free group `F₂ = ⟨a, b⟩` on the shape sphere with the three binary-collision points removed. Fix, normatively:
- which two branch cuts correspond to generators `a` and `b` (two segments of the collision equator, below; RQ-632). *Was:* "(arc from which collision point to the Lagrange reference pole)";
- the crossing-direction sign convention (which crossing direction is the generator vs its inverse) — must match the half-open sign convention in payload §3 / the integrator contract.

**Settled how (R-125):** the a/b assignment and the crossing sign are **transcribed from the literature** (Montgomery;
Šuvakov–Dmitrašinović), with citations, by the task that writes the word (M3); verified against the published braid
classes, physics-reviewed, and confirmed at the gate.

**The convention (RQ-632: decided 30 Oct 2026 by the orchestrator per R-369 as the consequence of R-125, option (a);
confirmed by the owner at the M3 gate, REQ-PAY-070).** Transcribed from the literature:
- **The generators.** Šuvakov & Dmitrašinović, PRL 110, 114301 (2013), p. 1: "A stereographic projection of this sphere
  onto a plane, using one of the punctures as the "north pole" effectively removes that puncture to infinity … That
  leads to the aforementioned free group on two letters (a, b), where (for definiteness) a denotes a clockwise full turn
  around the right-hand-side puncture, and b denotes the counterclockwise full turn around the other puncture".
  Dmitrašinović, Hudomal, Shibayama & Sugita, J. Phys. A 51, 315101 (2018), Appendix B (Montgomery's method, as its
  authors give it; Montgomery, Nonlinearity 11, 363 (1998), is not in the corpus), Fig. 7: "Three two-body collision
  points (solid red) lie on a meridian (dashed circle), with one of them being at the north pole"; and "a simple loop
  around the third ("infinite") puncture on the shape sphere corresponds to aB". The north pole is a collision point,
  not a Lagrange pole.
- **The cuts are the syzygy segments.** The same paper, Appendix C: "make the substitution a = 12, A = 21, b = 32,
  B = 23", with "11 = 22 = 33 = empty sequence", the digits being "crossings of the equator on the shape sphere, in one
  of three corresponding segments on the said equator, where the index of the body passing between the other two is
  used as a symbol". The three collision points `P₀₁`, `P₀₂`, `P₁₂` cut the equator (the collinear shapes) into three
  segments, each named by the body that lies between the other two along it; the segment with body `k` between runs
  from the collision of `k` with one of the others to its collision with the other.
- **The body map.** The literature leaves it open: Appendix C's note 3 has "irrelevance of which syzygy is denoted by
  which digit", and Appendix B lists the choice of the north-pole puncture as "Another ambiguity". Here it is settled by
  measurement and R-630's labelling. Over the six relabellings of `fixtures/crossings/prl_orbits.json`, two read the
  published words on all ten orbits: Table I's bodies 1, 3, 2 as ours 0, 1, 2, and its outer swap. The other four read
  words of other lengths or classes on all ten, and Table I's literal labelling, 1, 2, 3 as ours 0, 1, 2, is one of the
  four: its body 3 starts in the middle (`crossing_body_relabelling`). So the middle body of the orbit, Table I's 3, is
  ours 1 (R-630, item 3, which labels the plane's chart), and is Appendix C's digit 2; Appendix C's 1 and 3 are ours 0
  and 2. **The word is not invariant under a body relabelling:** a comparison with a published word first puts that
  orbit's middle body at ours 1. The puncture at infinity is then `P₀₂` (Appendix C's 1–3), and:
  - cut `a` is the segment with body 0 between, from `P₀₁` to `P₀₂`;
  - cut `b` is the segment with body 2 between, from `P₁₂` to `P₀₂`;
  - the segment with body 1 between, from `P₀₁` to `P₁₂`, is not a cut and appends nothing (Appendix C's
    "22 = empty sequence").

  The two cuts meet at `P₀₂`, their shared endpoint, and with the puncture there leave the sphere a disc. A loop
  around `P₀₂` crosses cut `a` and then cut `b` the other way: `aB`, Appendix B's class for the third puncture (the BHH
  family's, Appendix C's `aB = 13`).
- **The crossing sign.** Both cuts share one signed function, `d = −((r₁ − r₀) × (r₂ − r₀))`, minus the signed area
  (doubled) of the triangle 0 → 1 → 2: `d < 0` on `L⁺`'s hemisphere (0 → 1 → 2 anticlockwise, `n_w > 0`;
  dd_integrator §3.7), `d > 0` on `L⁻`'s, `d = 0` on the equator. So `d` has the sign of `−n_w`. Under the half-open
  rule (integrator contract Part 5, the τ tie rule), a crossing of cut `a` from `L⁺`'s hemisphere into `L⁻`'s (`d < 0`,
  then `d ≥ 0`) appends `a`, the other way `A`; cut `b` likewise appends `b` and `B`. The literature fixes the
  hemisphere `a` starts from, and which outer body is ours 0, only through its figures. The other hemisphere turns
  every letter into its inverse in place, the `(a, b) ↔ (A, B)` exchange, which REQ-VAL-043's comparison allows (R-630,
  item 5); the other outer-body order reads each of the ten orbits below within those symmetries too, conjugacy where the
  fixture records the inverse and the inverse where it records conjugacy (`qa_m316_prl_fixture_against_appendix_c`,
  `crossing_body_relabelling`; physics review 5720379930 of PR #358 holds RQ-632's option (a), with the owner's
  confirmation at the M3 gate, R-125, REQ-PAY-070). Cross-check: ten orbits of the PRL's Table I, integrated from its initial conditions, read their
  published words under this convention, eight up to cyclic permutation and two up to the inverse
  (`fixtures/crossings/prl_orbits.json`). Their grid is 200000 uniform points per period, DOP853 at
  `rtol = atol = 1e-12`, and `crossing_published_words` replays only the states bracketing each syzygy, so the
  cross-check fixes the convention and **says nothing about the march's substeps**. On that integrator, at 5000 points
  per period 6 of the 10 read a wrong word, and at 20000 two still do; on steps of `η·r_min^{3/2}/√2` all 10 read at
  `η ≤ 0.35`, 4 fail at `η = 0.5` and all 10 at `η ≥ 0.7` (physics review 5720379930 of PR #358; other grids than the
  kernel's). Reading the word from the kernel's own accepted substeps is REQ-VAL-043's (integrator contract Part 5,
  substep granularity).
- **The segment classifier (R-72, REQ-PAY-113; a definition, written by TASK-M3-16, physics-reviewed before merge).**
  At an endpoint of an accepted substep, the body *between* is the body opposite the longest side: the `k` whose pair's
  squared separation `r²ₖ` (pair `k` the side opposite body `k`, R-22) is the greatest of the three, the lowest `k` on a
  tie. On the equator it is the body between the other two, the segment's name; it reads positions only, so it does
  not depend on the masses. A side change of `d` over a substep is a crossing of the segment the classifier names at
  the substep's *non-negative* endpoint, the one with `d ≥ 0`: the end of a crossing in the positive sense, the start of
  one in the negative, and the endpoint on the equator when one is. Body 0 between: cut `a`; body 2: cut `b`; body 1:
  no crossing. `d`, the `r²ₖ` and their comparisons are branch inputs, formed under R-547 (integrator contract Part 4):
  each coordinate taken in as a mantissa in `[1, 2)` and an integer exponent, the differences, products and sums
  formed on those, so no intermediate is subnormal and every IEEE backend rounds them alike. `d` is decided on that
  exponent before it is converted: below the float's normal range it is zero, and in the range the conversion is
  exact, so no subnormal is formed (R-547; gpu_determinism_note § "The discipline", rule 6), and a value a conversion
  would round up to the least normal cannot read differently on a backend that flushes. Its side is then read by Part
  5's sign convention.

The crossing rule's sort, tie tolerance and sign convention, and the cuts' shared `d` with its segment gate, are the
integrator contract's (Part 5, the τ tie rule).

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
