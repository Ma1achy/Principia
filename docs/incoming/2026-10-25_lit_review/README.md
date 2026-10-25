# Related work (literature review), March 2026, as delivered 25 Oct 2026

The human's literature review, as a record (R-1, R-159). It is not authority. The human says it is old and needs more
work. Its reference list was already in `docs/archive/spec_sources/links.txt`; this is the text itself. Byte-for-byte
as delivered; only the extension was changed, from `.txt` to `.tex`.

| Path | sha256 | What it is |
|---|---|---|
| `lit_review.tex` | `8a6e9970de30422bb4b7c7fdb397f0882b14320bd3025c2076302165e6d4675c` | "Related Work — Principia: A Real-Time WebGPU Explorer of the Three-Body Initial-Condition Manifold", M. Doherty, March 2026 |

Known to be stale against the corpus as of 25 Oct 2026. These are notes, not rulings:

- Integration (§ "Symplectic Numerical Integration"): it describes a fixed-step Yoshida leapfrog with no
  regularisation, which terminates near collisions. The corpus now regularises: Heggie (1974) is the default, with
  Aarseth & Zare (1974) alongside. Neither is in its bibliography.
- The Agekian & Anosova entry cites *Astrophysics* 3, 1–17 (1967). The paper is *Astron. Zh.* 44, 1261 (1967), and
  *Soviet Astronomy* 11, 1006 (1968); see `../2026-10-24_anosova/`. Its starts were near rest, not at rest (eq. 3).
- Its account of the 8-dimensional count says total energy and angular momentum are fixed. The corpus's 8 dimensions
  (2 shape, 4 momentum, 2 mass ratio) come from quotienting translation, rotation and scale; E and L_z remain free.
- Final-motion classification sources (Chazy 1922; Fregeau et al. 2004) are absent.
- The platform is framed as browser-first WebGPU; the corpus is native-first.
