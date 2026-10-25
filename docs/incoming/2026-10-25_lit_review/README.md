# Related work (literature review): the March 2026 original and its October 2026 revision

The human's literature review, as a record (R-1, R-159). It is not authority. Its reference list was already in
`docs/archive/spec_sources/links.txt`; these files are the text itself.

| Path | sha256 | What it is |
|---|---|---|
| `lit_review.tex` | `8a6e9970de30422bb4b7c7fdb397f0882b14320bd3025c2076302165e6d4675c` | "Related Work — Principia: A Real-Time WebGPU Explorer of the Three-Body Initial-Condition Manifold", M. Doherty, March 2026. Byte for byte as delivered on 25 Oct 2026; only the extension was changed, from `.txt` to `.tex` |
| `lit_review_2026-10.tex` | `a4c20e22a1ee16bc041de53cf476596caf05c4e7b3275d3034badf6fc9998cd7` | The same review revised on 25 Oct 2026 against the corpus and the region D papers (`../2026-10-24_anosova/`). Approved by the human, who kept the two prototype results and the Pythagorean-indexing claim |
| `lit_review_2026-10.pdf` | `35634a424535e1a936c738d06e373b80651ae0460ef144c34c944af8f4a315ca` | The revision, typeset |

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
One citation from the original is still unverified: Heinämäki et al. 1998 (*MNRAS* 298, 790), which a search could
not locate.
