# Related work (literature review): the March 2026 original and its October 2026 revision

The human's literature review, as a record (R-1, R-159). It is not authority. Its reference list was already in
`docs/archive/spec_sources/links.txt`; these files are the text itself.

| Path | sha256 | What it is |
|---|---|---|
| `lit_review.tex` | `8a6e9970de30422bb4b7c7fdb397f0882b14320bd3025c2076302165e6d4675c` | "Related Work — Principia: A Real-Time WebGPU Explorer of the Three-Body Initial-Condition Manifold", M. Doherty, March 2026. Byte for byte as delivered on 25 Oct 2026; only the extension was changed, from `.txt` to `.tex` |
| `lit_review_2026-10.tex` | `19e20554fb815806022b6a8c2839ac233677a9e75b4a7d37f978b32044ea1f8d` | The same review revised on 25 Oct 2026 against the corpus and the region D papers (`../2026-10-24_anosova/`). Approved by the human, who kept the two prototype results and the Pythagorean-indexing claim |
| `lit_review_2026-10.pdf` | `7bc6ba4f023cf982093efa7b8d9d4cafa5d1c21a7704a7baf5c6664971f04556` | The revision, typeset |

What the revision changed:

- **Platform:** native-first, with one Rust engine (f32 on the GPU, f64 on the CPU). v1 web is the native app compiled
  for the browser.
- **The 8-dimensional count:** it comes from quotienting translation, rotation and scale, with the masses free on the
  simplex. E and L_z are derived, not fixed.
- **Agekyan & Anosova:** the corrected citation (*Soviet Astronomy* 11, 1006; *Astron. Zh.* 44, 1261) and region D as
  the paper defines it. Its starts are near rest, not at rest. Part II and Li & Liao (2018) are added.
- **New sections:** final motions and termination (Chazy, Fewbody, Mardling & Aarseth, Standish); regularisation
  (Kustaanheimo–Stiefel, Aarseth & Zare, Heggie, Mikkola & Aarseth, Mikkola & Tanikawa, Preto & Tremaine); numerical
  reliability (Boekholt, Hernandez); ensemble spread; validation targets.
- **Removed as unverifiable:** the L/M/A/H subdivision attributed to the 1967 paper, the "28–84% regular" figure for
  Trani et al., and "symmetric about both axes" for the Lehto map.
- **Corrected:** the author list of Trani et al. (2024), and Burrau (1913)'s venue (*Astron. Nachr.* 195, 113).
- **Frequency diffusion:** kept as related work, but no longer described as computed.
- **The per-pixel topological word:** described as the corpus has it, built from branch-cut crossings, with its
  conventions to be transcribed.

Two quoted results come from the prin-rs prototype, not from this build: the classifier against Fewbody's termination
test, and the tenfold drop in sync-cadence sensitivity under Heggie.

Citations corrected against Crossref after a second check: Li & Liao is *New Astron.* 70, 22 (2019); Trani et al. is
*A&A* 689, A24; Heinämäki et al. on Kolmogorov–Sinai entropy is 1999, under its own title; Lehto et al. runs 965–970.
Second revision, 25 Oct 2026, after the replication papers (`../2026-10-25_replication/papers/`) were in hand, with
each new description checked against the paper itself:

- **Mapping section:** two new subsections. "Converged maps and the structure of the free-fall plane" covers Boekholt
  et al. 2020, the Tanikawa–Mikkola collision-curve and symbol work (1995, 2008, 2015, 2021), Mikkola & Tanikawa 2007,
  and Parischewsky et al. 2023. "Beyond the zero-velocity, equal-mass surface" covers Tanikawa & Mikkola 2008 at
  non-zero angular momentum, Šuvakov & Dmitrašinović's velocity slice, Li, Li & Liao's mass plane, Anosova 1969 and
  Valtonen et al. 2005.
- **Burrau section:** Aarseth et al. 1994 is now described from the paper (100 points on each of six circles, time
  reversal, a regular region around each standard position). Boekholt, Moerman & Portegies Zwart 2021 and Chitan,
  Mylläri & Haque 2022 are added.
- **Regularisation:** Aarseth & Zare and Heggie are now described from their own abstracts.
- **Statistics:** Valtonen et al. 2005, Manwadkar et al. 2020 and 2021, Kol 2021 and Orlov et al. 2010 are added.
- **Gap claims softened.** "The momentum and mass dimensions are absent from existing maps" became "few and narrow",
  with the exceptions named. The structured-family gap now acknowledges that Chitan et al. integrated the sixteen
  Pythagorean triangles, one trajectory each. The Pythagorean-indexing claim stands, narrowed to indexing and to
  mapping around each member.
- **Removed:** the Heinämäki et al. 1998 citation (*MNRAS* 298, 790) and its strange-attractor dimension
  D ≈ 2.1. The paper could not be found; the nearest match is an extended abstract by Chernin (Kluwer, 1999) that
  states no dimension.
