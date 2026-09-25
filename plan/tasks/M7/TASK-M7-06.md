# TASK-M7-06 — Site-blend: kernels, blend space, swatches, static site sets and the vMF engine

- **Milestone:** M7
- **Closes:** REQ-COL-012, REQ-COL-013, REQ-COL-014, REQ-COL-020, REQ-COL-034
- **Depends on:** TASK-M7-02, TASK-M7-03
- **Needs (earlier milestones):** REQ-RENDER-020
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
Family A: `SiteBlend{sites, kernel, colours, space}`. Kernels vmf(κ) with w_i = exp(κ(d_i − d_max)), topk(k, ks) as a softmax over the k largest d_i, and nearest as a discrete argmax code path (never the κ → ∞ limit of exp); BlendSpace an explicit oklab | rgb parameter; a site's colour always a swatch, with the Full-OKLab and Okabe–Ito tables as preset swatch-sets and colours from generators (golden-angle, OI-cycle, gradient A → B) or a LUT sampled at i/N; the static generators axes6, corner8, ico12, fib(N), ring(N, tilt, rot) computed once and bound as uniform arrays; and the vMF engine of dd_colouring §3.2 (C·(cos θ_i, sin θ_i) weighted by exp(κ n̂·p̂_i), no atan2 in the blend).

## References
- `docs/design/principia_colour_composition.md` § "1.1 Family A — site-blend  →  `vec3`"
- `docs/design/principia_colour_composition.md` § "2. Site-set kinds"
- `docs/design/principia_dd_colouring.md` § "3.2 The vMF engine"

## Deliverables
- `crates/render/shaders/wgsl/lib/vmf.wgsl` (`vmf_weight`, `nearest`, `topk`) and `lib/site_blend.wgsl`.
- `crates/render/src/colour/sites.rs` — the static site generators (CPU, once), uploaded as uniform arrays.
- `crates/render/src/colour/swatch.rs` — swatch type, the FullOKLab and OI swatch-sets, the generators.
- The SiteBlend node variant and its codegen in the occupant tree.

## Acceptance tests
- `cargo test -p render siteblend_kernels` — vmf weights shift-invariant under d_max subtraction; topk(1, ·) equals nearest; the generated WGSL for nearest contains no `exp` (REQ-COL-012).
- `cargo test -p render blend_space` — the same sites/weights give different outputs for oklab vs rgb; the oklab midpoint chroma is lower than the endpoints (REQ-COL-013).
- `cargo test -p render swatch_sets` — the FullOKLAB and OI swatch-sets reproduce dd_colouring §3.2's hue angles (REQ-COL-014).
- `cargo test -p render static_sites` — generator outputs are unit vectors matching §7.1's formulas (icosahedron vertices, Fibonacci n_z,i / r_i / φ_i, the N-pole ring); they are bound as uniforms, not recomputed per pixel (REQ-COL-020).
- `cargo test -p render vmf_engine` — dd_colouring unit test 3: opposing-pole midpoints give a = b = 0; large κ gives the nearest-pole colour; blend(Rn̂, R·poles) = blend(n̂, poles) (REQ-COL-034).

## Notes
- The swatch table's L and C for the preset sets come from the §8 ranges and the map's parameters; nothing new is chosen here.
