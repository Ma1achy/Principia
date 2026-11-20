# Plan

Every unfinished product task from the old repository is here: none dropped, each in plain English with its old
id in brackets. A task's full spec is its file in `plan/tasks/<milestone>/<id>.md`. Those files were written
under the old process, so read them for what to build and why, and ignore their process (reviewers, rulings,
requirement ids). Where a spec is wrong or has a gap, do the sensible thing and leave one comment in the code.

The order is your 10 November chart plan's (charts, then integrator, then visuals), with two changes: the first
frame comes first, so every later piece lands in a running app, and each feature's GUI is built with the feature,
not saved for the end. Non-visual work (validation, replication harnesses) runs in parallel with visual work.

**Working with the code that exists.** The 234 finished tasks are in the code: the kernel's integrators,
decoder, charts and events; the engine's state store and CPU slice path; the colouring; the ledger; the
validation suite; and the whole GUI, built on a mock engine. Under the new chart system today's charts are its
maps, so they are kept and wrapped, not rewritten (the 10 November plan: nothing built is thrown away), and the
chart-path task converts today's pan, zoom, tilt and lock onto the new object, measuring each against today's
behaviour first. Some tasks below are partly or wholly done already, mostly the GUI's, because the mock GUI was
built under other tasks' names. So every task starts by reading the code and builds only what's missing.

At each checkpoint: show the running app, the key numbers and the calls worth a second look; the owner reacts; a
fix pass; then the next checkpoint. The progress grid in the README draws `plan/tasks.yaml`.

## C1 — First frame

Open the app and see a real three-body field. The smallest piece, and nothing else starts until its first task
(TASK-C1-01) merges.

- **TASK-C1-01 `RealSide` draws a real field.** First, one entry point in the engine, `engine::field`:
  `(SimConfig, window, size) -> Field` (an outcome per sample), behind a `FieldBackend` trait with a CPU
  implementation now and the GPU's at C4. The GUI calls only this, never `first_slice` or the kernel. Then in
  `crates/gui/src/side.rs`, `RealSide::canvas` and `figure` return a canvas showing a 64×64 field: the CPU
  backend decodes the latent chart at each sample (as `engine::first_slice` does) and integrates it on the
  CPU (Heggie, the kernel's default, at the default horizon), coloured with
  `render::colour::outcome`'s CPU palette, uploaded as a texture and drawn into the figure rect the way
  `mock::canvas` draws its stand-in. Blocking and slow is fine. The app starts on the real side; the mock
  stays behind `--features mock`. Test: a fixed 16×16 frame's outcome-class histogram matches a stored one,
  and the control (the mock's field swapped in) fails. Screenshot in the PR.
- **TASK-C1-02 A golden frame.** A fixed 64² frame of a fixed chart window, stored as an image and its class
  histogram, checked in CI, so a regression is caught by a test and not by eye.

## C2 — The chart system

The new chart system as you re-planned it on 10 November, in that plan's build order: the chart as a map seen
through a flat window, the chart-path object, the lock, the blend (morph) chart, the registry and every map,
slopes, the forward and backwards tables, edges (saturate, unfold, wrap), placement, time as a coordinate and
computed axes. Today's charts are kept as maps. Each piece lands with its controls in the running app.

- The decoder's mirror step removed, latent β over the full circle, under the owner's bitwise check (TASK-M2-57)
- The first slice's integrated arms on the default occupant, Heggie × RK4, once its joint march is built
  (TASK-M2-108)
- The chart-path object, and today's pan, zoom, tilt, slice, lock, chart switch and Save view as chart re-
  expressed through it, bitwise (TASK-M2-61)
- Locking doesn't move the view, in the engine: the pin at the clicked pixel, tilt and zoom about it, snap-back,
  exact on nonlinear charts or saying why (TASK-M2-62)
- Save view as chart on curved views, the pivot record at a centred pin, and tilts into the held ν₀, L_z and K, in
  the engine (TASK-M2-63)
- Consolidating the chart path: one leaf per operation through REQ-CHART-060's stop, the merged GUI routes made
  thin adapters (R-894, R-916, R-919 F1 (a)) (TASK-M2-64)
- The lock's solver core, from TASK-M2-73 (TASK-M2-94)
- The blend chart: the state blend, nested blends, both ends navigated, the ends exact, a signed fold indicator
  defined (TASK-M2-90)
- Paths under a lock: the stored anchor (TASK-M2-65)
- The primitive registry, the refusals, the labels and the bitwise digest, on the CPU (TASK-M2-66)
- CPU decode and encode through the composition: every chart through resolve, every inverse through the registry
  (TASK-M2-67)
- Slopes in three tiers: exact (dual numbers), implicit or none, a flag at each kink site (TASK-M2-88)
- The backwards table on the CPU: bake, key, fold flag, search (TASK-M2-92)
- The invariant construction's second sheet, and a state's sheet from one source with it (TASK-M2-96)
- The backwards table on an extended chart: sheet 1 only, a state looked up as itself and as its time reverse
  (TASK-M2-97)
- The one placement function, `place(state, chart) → {point, sheet, residual, route, status}`, and the two
  definitions it owes (TASK-M2-99)
- Unfold in the CPU evaluator: past a fold the chart shows its second sheet (TASK-M2-101)
- Clamp in the CPU evaluator, the clamp below the floor included (TASK-M2-102)
- Saturate in the CPU evaluator (TASK-M2-103)
- Wrap in the CPU evaluator: periodic and pole-crossing edges (TASK-M2-104)
- The edge kinds' count, navigation stop and sim key: `energy::count`, the view state's override, blank's controls
  (TASK-M2-105)
- Region D on the CPU: the third-body pair, its preset, its velocities and the closed round trips (TASK-M2-68)
- Hold L_z on any chart, on the CPU (TASK-M2-69)
- The virial pair: its axis kind, descriptor and entry (TASK-M2-70)
- Warps as options, polar and log-polar pair maps, and a statistic's measure (TASK-M2-71)
- Curved slices off the shape sphere: the momentum and mass spheres (TASK-M2-72)
- Numerical inverses and the general numerical solve, on the CPU (TASK-M2-73)
- The primitives of 6 Nov: axes and overlays (TASK-M2-74)
- Region D's exact map and every target's native coordinates (TASK-M2-75)
- The primitives of the full order (TASK-M2-76)
- Paper frames as presets (TASK-M2-77)
- The fixed-k chart: rigid rotation at a chosen virial coefficient over D₁₁ ∪ D₁₂, a chart and builder axes
  (TASK-M2-78)
- The binary-plus-single scattering chart, with f_col and Heggie & Sweatman 1991 alongside (TASK-M2-79)
- Points of interest's maths: the signed point, the nearest chart point, the off-slice distance and the near-match
  (TASK-M2-80)
- Path re-timing, alignment and flags (TASK-M2-81)
- Time-axis chart mode's maths, through the trajectory interface (TASK-M2-83)
- Time as the last coordinate: the map's time coordinate, the playhead the window's centre, a target time to an
  integer index (TASK-M2-91)
- Computed axes, the maths: terms, name, ticks and marks from the window; the dual ruler in a blend (TASK-M2-98)
- The symmetry filter's maths: the mask, the partner check and the statistics, on a synthetic payload (TASK-M2-84)
- The acute-angle Burrau charts over the full acute range, both labellings, the leg swap through the symmetry
  filter (TASK-M2-85)
- Saved curved charts, the rest: compositions onto three or four hidden coordinates, a saved chart saved again,
  and the composed chart's area weight and repeat counting (TASK-M2-55)
- The measurements P2 and P4, reported to the owner, and P3's timing, reported with no pass line (TASK-M2-100)
- REQ-DEC-060's seam clause: the seam measurement across `z_α = 0` under the criterion R-1022 rules, the seed
  conversion's other clauses built by TASK-M2-93 (TASK-M2-109)
- Manifold view panel: Chart, Navigate, depth readout, Slice centre, Tilt, and the figure's axis labels (TASK-M8-06)
- Lock, compass and time controls: the reticle, the pin, the nav cube and the scrubber (08_lock.png, 01_main.png)
  (TASK-M8-07)
- Chart builder window: axis kinds, presets, domain preview, quick render, Apply (03_chartbuilder.png) (TASK-M8-18)
- The shape sphere past [0,1]² as the whole sphere: no hemisphere mode, and the area statistics over the whole
  sphere (TASK-M8-44)
- A Chart-builder chart that fails validity inside [0,1]²: its drawing, and its console warning only for warped
  axes (TASK-M8-47)
- On the real engine: tilt and slice on every chart, the energy axes' mapping control, the navigation stop in the
  edit path, and Save view as chart on curved views (TASK-M8-48)
- On the real engine: the Decoder window's Apply re-integrating, the Chart builder's read-only links, and the warp
  previews (TASK-M8-52)
- Past the square with the controls shown: pan and zoom past [0,1]², drawn with no gaps, every interaction working
  there, a repeat resolving to its canonical system, the ✕ and the round trip (TASK-M8-55)
- The state map in Explore: the legend and its ring, never a gap past the square, copies as the canonical sample
  and the scheduler's firewall (TASK-M8-66)
- The initial-condition constructor's shape sphere: every point of both halves reachable, the drag across the
  equator flipping the triangle, and the mirror twin shown as a faint marker with its note (TASK-M8-67)
- Past `[0,1]²` on the real engine: the axis extension types, the area statistics and the F3 fill (TASK-M8-87)
- The Manifold view panel on the real engine (TASK-M8-89)
- Lock, compass and time controls on the real engine, and the scrub that re-integrates (TASK-M8-90)
- CPU batch: numerical inverses, their branch rule and seam (TASK-M8-102)
- The Chart builder's composition panels (TASK-M8-103)
- 1D charts in the GUI (TASK-M8-104)
- Live paths: the blend entry and the coarse cover (TASK-M8-107)
- Time-axis chart mode on the real integrator: the column march (TASK-M8-113)
- Time-axis storage and the column cost (TASK-M8-114)
- Time-axis mode in the GUI (TASK-M8-115)
- The symmetry filter: a display option after rendering, GUI and export (TASK-M8-120)
- Locking doesn't move the view: the pin at the clicked pixel, tilt and zoom about it, centre on pin and the auto-
  snap setting, exact on nonlinear charts or saying why (TASK-M8-125)
- Points of interest: saved signed states, their stars on every chart and path frame, and similar-systems overlays
  (TASK-M8-126)
- Final-condition charts: a CPU batch chart of state functions at time T, the round-trip miss measured, large
  misses masked (TASK-M8-128)
- The same-frame slips D19 and D23, fixed in the GUI's adapters onto the chart-path leaves (TASK-M8-129)
- The figure draws the computed axes: the dual ruler, the edge marks, mixed names (TASK-M8-130)
- Marking clamped and saturated areas: a toggle, off by default (TASK-M8-131)

## C3 — Physics you can trust

Escapes and terminal states labelled, failures stored honestly, the inspector's precision paths, the trajectory
panel, and the validation harnesses (anchors, convergence reference, cross-chart and cross-implementation checks).
The non-visual validation runs alongside C2, never instead of it.

- The escape detector: closure and E_rel, the window, the escaper and ionisation (TASK-M3-12)
- Terminal states, precedence and the terminal latch (TASK-M3-17)
- The failure path: sim_failed, NaN-free storage and the failed-state values (TASK-M3-18)
- The Precision ring: computeIC, computeQuad, stepOnce, decodeOnly and the RK45 inspector reference (TASK-M3-19)
- The CPU worker pool and the hover-trajectory benchmark (TASK-M3-20)
- After M2: the trajectory interface on the real integrator, and the outcome-class arms of the conversions
  (TASK-M3-67)
- The hover integration: computeIC in the inspector worker, step budget, and the projection onto (q₁, q₂)
  (TASK-M8-09)
- The hover trace and label on the figure (TASK-M8-10)
- The Trajectory panel: summary, real space, the turning shape sphere, kept orbits (TASK-M8-11)
- Payload purity and the parity-tier tags (TASK-M3-21)
- Seam 5: the producer writes every ledger field, and the scalar and shape views (TASK-M3-22)
- The occupant-comparison harness (TASK-M3-23)
- The ground-truth harness: Path A, Path B, the fixture set and the golden anchors (TASK-M3-24)
- The independent convergence reference (Brutus-style) (TASK-M3-25)
- Order scaling and the symplectic-vs-not gates (TASK-M3-26)
- The Burrau anchor smoke tests: SP-1 run to its horizon, SP-2, and the f64 run against R-33's reference
  (TASK-M3-27)
- Figure-eight closure and Lagrange rigidity (TASK-M3-28)
- Free-fall brake orbits and the reversibility diagnostic (TASK-M3-29)
- Word calculus against the braid classes, and the truncation rate (TASK-M3-30)
- The predictive step limit and the two-artefact ablation (TASK-M3-31)
- Re-registration: the Heggie default gate, the control, the doubling protocol and the logH falsification
  (TASK-M3-32)
- The validation harness's post-escape march and pitfalls §2.4's checks (TASK-M3-33)
- Escape re-validation: precision, recall and the tau gap (TASK-M3-34)
- The patchwork regression and the stop_on_escape images (TASK-M3-35)
- Cross-implementation checks: BodyPlane bit-for-bit and the change-10 re-run (TASK-M3-36)
- Cross-chart consistency: the same state through the shape sphere and the latent chart gives the same z, payload
  and outcome (TASK-M3-37)
- The shape sphere's mirror test, its runs: the run from Π·x the run mirrored, bitwise, fates equal and L_z and
  the frame's winding opposite across pixel pairs on a named non-chaotic fixture, and the frame's winding defined
  (TASK-M3-38)
- Every stepper × regularisation pair: its exact composition, the measured pair table and its warnings (TASK-M3-39)
- The fixed sync quantum: one grid of physical time, the march stopping only on it, live, catch-up and re-boot
  bitwise (TASK-M3-40)
- The regularised integration floor: the step budget as a rate, undetermined on exhaustion, the counted signals
  and the two energies' reading (TASK-M3-41)
- The FTLE field's chart independence (TASK-M3-69)
- LogH, Heggie's and logH's seam, and the visible re-registration count (TASK-M3-75)
- Heggie's landing-order check: the figure-eight convergence order under R-1021's criterion (TASK-M3-79)
- Inspector window and pane 1: the editable IC on the physical canvas (05_inspectors.png) (TASK-M8-14)
- Inspector pane 2: the canonical representative, the gauge buttons, the round-trip ghost, the JS oracle
  (TASK-M8-15)
- Inspector pane 3, the one timeline, and the readouts (TASK-M8-16)
- Inspector: Create & locate, degeneracy routing, and the honest CPU / GPU comparison (TASK-M8-17)
- The Precision ring's state-in entry, `computeICState(state, simKey)`, beside `computeIC`, and the frame the
  constructor integrates (TASK-M8-83)
- Saving an IC: the `principia.ic/1` record, copy, paste, Save IC… and Open IC…, the twin on reopening, marks in
  `Lines`, and the trajectory CSV with its bodies labelled 1-based (TASK-M8-84)

## C4 — GPU

The same field from the GPU kernel, matching the CPU within the parity contract, at interactive speed. The GPU
only integrates the starting systems the CPU hands it.

- The shared kernel compiled to f32 SPIR-V and WGSL, under the compile-discipline lint (TASK-M4-01)
- The native in-process wgpu parity harness: GPU stepOnce and decodeOnly on shared states (TASK-M4-02)
- Tier L: branch decisions bit-identical on identical inputs, at the enumerated branch inputs (TASK-M4-03)
- Tier B and Tier N: integer fields exact, continuous values within measured native-wgpu tolerances (TASK-M4-04)
- Flat-grid compute: in-kernel decode from chart uniforms, one workgroup per quad, an in-place march (TASK-M4-05)
- The baked compute variants: chart × link × occupant × FTLE, with copy_index a uniform (TASK-M4-06)
- The precompile rule and the bounded monomorphisation set (TASK-M4-07)
- The sim key, the render key and resolve() (TASK-M4-08)
- Bounded dispatch: chunked marches that return before a watchdog (TASK-M4-09)
- The fixed-timestep frame loop over the flat grid (TASK-M4-10)
- Checkerboard march: a per-pixel phase mask over whole sample bundles (TASK-M4-11)
- Checkerboard self-erasing: catch-up at rest, the three-state control and export force-off (TASK-M4-12)
- FTLE on the GPU: the f32 shadow form and the horizon map (TASK-M4-13)
- Seam cross-check views on GPU payloads, and the live substep heatmap (TASK-M4-14)
- The closure field on the Config α×β z_q = 0 plane: free-fall orbits light up (TASK-M4-15)
- The pixel inspector's engine side: f64 trace, f32 GPU trace and the divergence overlay (TASK-M4-16)
- The uniform-grid survey: CPU↔GPU aggregate agreement, and the prebake path (TASK-M4-17)
- The permanent parity suite in CI: trajectories, Tier S and the label-flip report (TASK-M4-18)
- First kernel benchmarks: the march's bottleneck, N = 8 vs 16, and one quad thread-count constant (TASK-M4-19)
- Compute fast-math on against off: the march's speed, and the differences measured and reported (TASK-M4-20)
- Link parameters on the GPU: the uniform, bitwise today's at the defaults, and a parameter change that moves only
  the masses it should (TASK-M4-21)
- The kernel cross-backend check: the same IC set marched on NVIDIA Vulkan, Metal, lavapipe and CPU f32, labels
  and branch-sensitive results compared per backend pair (TASK-M4-28)
- Written forms for `E_rel` and `|Δn̂|` that use no driver division or `√` in the decision (TASK-M4-29)
- The header records "Off" only for drivers measured, otherwise wgpu's path with "driver unverified" (TASK-M4-30)
- Near-tie golden cases held only on CI's backends, elsewhere reported (TASK-M4-31)
- The baked entries on the GPU, the axis cap and V1's decode time (TASK-M4-33)
- Region D on the GPU: the pair's baked entry, its lowering row and its GPU reference (TASK-M4-34)
- Per-backend accuracy of cos near π/2 against f64 (TASK-M4-38)

## C5 — Tiles and caching

Quads, the scheduler and the cache: pan and zoom reuse computed squares, each square travelling with its forward
table.

- Quality tiers and the one settings struct (TASK-M5-02)
- Quad addresses in the slice plane's frame (the per-slice quadtree) (TASK-M5-03)
- QuadRequest: flags, per-quad uniforms and quad-local sample positions (TASK-M5-04)
- Chart-domain quad skip and the label of skipped pixels (TASK-M5-05)
- Sim key, render key, cache identity and the payload compatibility signature (TASK-M5-06)
- Payload store: the per-copy index map, chunk sharding, SimState and word in lockstep (TASK-M5-07)
- The current-state cache: lookup, resume points, preview vs refined, no history (TASK-M5-08)
- Eviction: cost-weighted LRU with t_cached, and the pinned classes (TASK-M5-09)
- Memory estimator: payload, word buffer and render targets (TASK-M5-10)
- Memory budgets, the hard cap and continuous pressure (TASK-M5-11)
- Blast radius: what each knob invalidates, and r_coll on the sim key (TASK-M5-12)
- Navigation as chart-uniform edits: in-plane re-addressing and new identities (TASK-M5-13)
- Dispatch queue: epochs, in-flight limit, positional readback and the measurement path (TASK-M5-14)
- Baseline-first tier and display-only ancestor fallback (TASK-M5-15)
- Ensemble copies: Halton offsets and copies as scheduler leaves (TASK-M5-16)
- Resolve stage: outcome and validity reductions, and Undetermined quads (TASK-M5-17)
- Resolve stage: ensemble spread, error_ratio and the co-computations (TASK-M5-18)
- Roundtrip_error and the per-footprint temporal accumulators (TASK-M5-19)
- SSAA colour resolve: colour per sample, average last, no interpolation (TASK-M5-20)
- Frame loop: the live set, off-loop catch-up and atomic promotion (TASK-M5-21)
- Transport: pause, restart, loop and the playhead clock (TASK-M5-22)
- Checkerboard in the frame loop: live set only, render resolution, three-state control, dt ceiling (TASK-M5-23)
- Loop hygiene: never wait, per-frame scheduler budget, background priority, backgrounding, queue depth (TASK-M5-24)
- The two-regime scheduler: in-motion coarse cover, debounce and the backdrop reference (TASK-M5-25)
- Compositor: the stale backdrop, blur as the one signal, never blank (TASK-M5-26)
- Precompiled pipelines: no compile on the gesture path (TASK-M5-27)
- The frame record, percentiles and the bounded telemetry file (TASK-M5-28)
- Tier 3 tile debug shaders and the quad-state transition table (TASK-M5-29)
- Membrane audit: QuadReduction is the sole automatic return (TASK-M5-30)
- The uniform echo view (TASK-M5-31)
- The quadtree past the square: roots tiling the plane, canonical addresses, neighbours across seams, poles and
  folds, scheduling by the union of images, the statistics unchanged (TASK-M8-56)
- Deep zoom in repeats and far from the root, and seams between images with a shared edge (TASK-M8-57)

## C6 — Adaptive refinement and deep zoom

Refinement where the picture needs it, and zoom past the f32 floor through the one-cell table.

- The refinement decision and the quad lifecycle: terminal vs refinable (TASK-M6-01)
- The split gate: the screen floor, the sliding MAX_REL_DEPTH and the motion refinement floor (TASK-M6-02)
- Policy::Tolerance's split rule and the per-footprint latch (TASK-M6-03)
- The stop rule: alpha_area over two levels, neighbour agreement, and the R-42 fixes (TASK-M6-04)
- Error_ratio as the trust flag, and the refinement gate / integrator tolerance pair (TASK-M6-05)
- Merging under a live playhead: coarsening, resident count and merge memory (TASK-M6-06)
- The linearised decoder: x₀ and J_D in f64 on the CPU, x₀ + J_D·δ expanded on the CPU in f64 and narrowed once
  (TASK-M6-07)
- The decoder switchover and AT_F32_FLOOR: DECODE_MODE, collapse detection, the linear-path uniform (TASK-M6-08)
- The per-quad measure weight |det J_D| and the quantitative-claims path (TASK-M6-09)
- The integration floor: saturated quads stop refining, flagged floor-limited (TASK-M6-10)
- Tile priority: P_tile, the pointer-centred P_focus, quad-relative relevance and baseline-first (TASK-M6-11)
- The frontier margin derived from the refill rate, benchmarked against a widened margin (TASK-M6-12)
- Symbolic spread S_word: the word-reduction diagnostic and its measurement (TASK-M6-14)
- QualitySettings on three axes, the tiers keyed off eps, and the preset selector (TASK-M6-15)
- Auto: device characterisation — limits, probe, info, solve and the tier inversion (TASK-M6-16)
- The arbiter: one writer, the throughput model, the ladder and the lever order (TASK-M6-17)
- The arbiter's timescales: motion offset, change under cover, hysteresis, sensors and persistence (TASK-M6-18)
- The sea_fraction(eps) estimator (TASK-M6-19)
- Refinement reporting: per-chart reports, weighted stop breakdowns, the binding axis, dp/u (TASK-M6-20)
- The event-driven precision warning: DECODE_SWITCHOVER and AT_F32_FLOOR in the snapshot (TASK-M6-21)
- Quality in the GUI: the Run window selector, the Custom fields and the arbiter overlay (TASK-M6-22)
- The first refine milestone's measurements: depth past level 6, tau, merging under motion, zoom regression
  (TASK-M6-23)
- The GUI track: RQ-340 ruled — Tip as sheet the default, Lean each axis an option, over the pole, the compass
  drawing the active map, the readout's lines (TASK-M6-48)
- The GUI track: Save view as chart on the mock — the button and View entry, the Saved group, provenance, one undo
  entry, the oblique latent chart reopened bitwise (TASK-M6-49)
- The GUI track: the Decoder window on the mock, its rows, staged Apply, live previews through the engine's link
  evaluator, the Chart section's Links row, and the decoder badge (TASK-M6-50)
- The GUI track: a latent view saved at a tilt, tilted again in the orthonormal frame of its projected targets,
  the stored pair kept (TASK-M6-51)
- Where precision runs out: four runs of the clicked point, the chord's two marks, the trust line, and nothing
  changed in the atlas (TASK-M8-76)

## C7 — Colour

The stain graph, colour composition, legends, traces and contours on the real field, and the print stage.

- Site-blend: kernels, blend space, swatches, static site sets and the vMF engine (TASK-M7-06)
- The seamless LUT sphere and analytic Cubehelix (TASK-M7-07)
- ScalarField sources, derived operators and the physics site generators (TASK-M7-09)
- Combinators and the physics overlay (blob blend) (TASK-M7-10)
- Custom per-node WGSL: async compile, last-valid fallback, the Problems surface and eject (TASK-M7-14)
- The render key and render freedom (TASK-M7-15)
- The bake tier: the equirect texture for pure-f(n̂) occupants, and the sphere widget (TASK-M7-16)
- The preset library: presets as whole graphs, debug views as locked presets (TASK-M7-17)
- The §7.1 map presets and their golden-image suite (TASK-M7-18)
- The seam-free guarantee and the two rotations (TASK-M7-19)
- The display chain: Print graph → display scale → gamut clamp → colour-vision simulation → screen, the line
  passes either side (TASK-M7-21)
- The stain editor canvas (egui): library, graph canvas, preview and ghosted None (TASK-M7-22)
- The node inspector: morphing editors, source availability, the L-conflict flag and the map controls (TASK-M7-23)
- Code views: Pipeline WGSL, Node WGSL and the Problems pane (TASK-M7-24)
- The Display window: display settings outside the pipeline, CVD on every surface (TASK-M7-25)
- The print stage: the Print graph's executor in style's place, Plain the bitwise identity, measurement blind to
  it, debug views bypassing it, goldens under Plain (TASK-M7-26)
- Image embedding: what travels — the contract-named payload and the build-hash refusal (TASK-M7-31)
- Image embedding in PNG export: defaults, the tEXt chunk, capacity and the resolution round trip (TASK-M7-32)
- Pipeline precompile at startup and the render-target counts (TASK-M7-33)
- Image embedding: the hybrid variant re-measured on real rendered figures, and §5's ~8 % reconciled (TASK-M7-34)
- The Chazy classifier: the rule, the Fewbody reference, the ten fixtures and the I_max latch (TASK-M7-35)
- The Chazy fields: chazy_final, chazy_status, chazy_structure, chazy_q, chazy_escaper and chazy_qk, the escape
  freeze, and the energy certificates (TASK-M7-36)
- The Chazy flat stains: the seven-class default, four families, Okabe–Ito, the status map, the filter, the ranks
  and palette separation (TASK-M7-37)
- The Chazy energy-channel stains: seven classes × energy, structure × energy, and B's hue (TASK-M7-38)
- Chazy parity: the f32 against f64 label disagreement, measured and reported (TASK-M7-39)
- The Chazy time views: when it was decided, and what just changed (TASK-M7-40)
- The Chazy one-body stains: the body lens, its energy form, the escaper map, the mass-rank views and their tie
  fallback (TASK-M7-41)
- The line engine: its overlay pass, data model and styles, polylines with dashes by arc length, smart ink and
  labels (TASK-M7-42)
- Contours: the expression's variables and totality, evaluation on the decode, levels, marching squares, and
  energy contours that follow the active mapping (TASK-M7-43)
- The embedding's tiers: the tier table, the drop from the bottom and never below physics, the colour flag and
  layout version 4, the report, old records, and capacity re-measured (TASK-M7-44)
- The decoder's identity per link: each recorded link's own entry hash, refusing exactly the changed link
  (TASK-M7-45)
- Chart position: the source, its projection onto each target's quotient, the four results, the edge clamp, the
  colour frame and the t = 0 identity (TASK-M7-47)
- Chart position's colour slot: the bases (Gradient, Site blend, Palette), the layers, the brightness occupants
  and the calibrations (TASK-M7-48)
- Chart position's Picture base, the fade off the slice and the ensemble's nominal copy (TASK-M7-49)
- Replace-L's keep in gamut option: chroma scaled by min(1, L/0.75, (1 − L)/0.35), off by default (TASK-M7-50)
- The stain editor's Chart position: the node, the point⟨q⟩ subtype, the base-and-layers inspector, switching and
  restoring, Problems, the presets and saved stains unchanged (TASK-M7-51)
- The one projection: each target chart's projection, the leaving rule on main's clamped encode and a curve axis's
  nearest point, one Rust source for Chart position and the trace (TASK-M7-52)
- The schema version hashes only the layout and kernel columns; a ledger scale edit re-styles with zero recompute
  (TASK-M7-53)
- Print graph determinism, units and stability: seeded streams, `ref_px` / `device_px` / `fraction`, the screen-
  anchored sheet and `boil`, with the §11 harnesses 3, 4 and 5 (TASK-M7-55)
- Print graph port, Analyse and Generate: 5 node types, each with its lab golden (TASK-M7-56)
- Print graph port, Adjust & combine: 13 node types and one preset, each with its lab golden (TASK-M7-57)
- Print graph port, Paint: 15 node types and 26 presets, each with its lab golden (TASK-M7-58)
- Print graph port, Print: 9 node types and 11 presets, each with its lab golden (TASK-M7-59)
- Print graph port, Screen: 10 node types and 31 presets, each with its lab golden (TASK-M7-60)
- Print graph port, Craft: 11 node types and 16 presets, each with its lab golden (TASK-M7-61)
- Print graph port, Photographic: 5 node types and 9 presets, each with its lab golden (TASK-M7-62)
- Print graph port, Machines: 2 node types and 5 presets, each with its lab golden (TASK-M7-63)
- Print graph port, Slots: the six slot nodes and the slot chain template; Palette on the finished colour; screen
  media and Viewing; the sheet, never the marks, on a pixel that is not a system (TASK-M7-64)
- R-130's eight styles as the Print graph's Poster family: their definitions in the design doc, any node they
  need, and their goldens (TASK-M7-65)
- Print graph field inputs: the `field` node, the stain's window, absent fields, and the four "Driven by the data"
  presets on their real fields (TASK-M7-66)
- `through_medium` in the line renderer: two passes with complementary filters, the sets drawn through before the
  Print graph, smart ink reading each pass's figure (TASK-M7-67)
- Print graph cost classes measured: every node at 1024² on the reference machine, classed from the numbers, and
  each reduced node's factor (TASK-M7-68)
- `device_px` and the screen-anchored sheet in a second pass at screen resolution, at every render_scale and at
  export (TASK-M7-69)
- Base px: every mark length of the line system and the Print graph in one unit, scaled by `output_width /
  base_width`, labels placed at base size, `RenderState.base_width` (TASK-M7-70)
- Contours of any registered primitive (TASK-M7-72)
- The five table debug views (TASK-M7-73)
- The fragment table route: the GPU side of the backwards table, for Chart position and the presets (TASK-M7-74)
- Stain mode layout: library drawer, canvas with Graph | Code, preview, Problems pane (02_stain.png) (TASK-M8-19)
- Stain canvas: selection, wiring gestures, the node palette, node visuals (TASK-M8-20)
- Stain node inspector: morphing editors, Advanced ▸ edit shader, Graph | Code, eject (TASK-M8-21)
- Legend generated by evaluating the stain (06_legend.png) (TASK-M8-22)
- Display window: the last stages and the display overlays (04_windows.png) (TASK-M8-23)
- The Chazy legends: the playhead in the title, B's label, the status legend's full names, the one-body legends
  and the three-body panels (TASK-M8-45)
- On the real engine: View › Debug view and its counts, the decoder-bug warning for the gallery charts, the mass
  simplex's background and hatch at the same pixels, and F3's Fit mode (TASK-M8-46)
- Traces on the line system: the hover, pinned and kept traces drawn by §8, shared with the orbit viewer
  (TASK-M8-49)
- Lines ▾, its editors, the axes and presets, Grid, t_end and class edges out of Overlays, and lines in the
  exported view state (TASK-M8-50)
- The line system's open questions as R-509 rules them: the stain's expression syntax and the ledger's fields, the
  trace fade's proxy, trace labels winning, Screen contour the default (TASK-M8-51)
- The trace fade's general off-plane distance in the pulled-back mass metric (TASK-M8-54)
- The trace's markers and icon sets, its defaults, syzygies, off-screen pins, repeats, not-a-system handling and
  computeIC's guards; where a trace can leave, mixed axes, playback and the time ramp (TASK-M8-58)
- Display › Print, the Print panel: Plain / Print, the slots with live thumbnails and their settings, Boil, the
  111-preset gallery in 12 families, Edit graph…, Reset; and each line set's "Through the medium" (TASK-M8-68)
- The Print graph editor: the Stain's node editor reused on the slots' graph, typed wire refusal, positions in
  `ViewUI`, unstable nodes marked, and custom WGSL nodes (TASK-M8-69)
- The instrument stays honest in the GUI: the "Print: …" badge in the header strip, Styled / Plain export
  defaulting to Plain, and every readout blind to the print (TASK-M8-70)
- Print graph provenance and round trip: `RenderState.print` by value in every destination, tiers 2 and 3, the
  custom node dropped and badged, and keyframes (TASK-M8-71)
- Degrade while moving: per-node motion policies, settle, the playhead not motion, and the print stage in the
  frame record (`stage_ms.post`, `post_level`, schema v2) (TASK-M8-72)
- The print stage's budget: excluded from the Auto rung solve, the post level the arbiter's motion-regime lever
  under Auto only, the budget's definition and the print share (TASK-M8-73)
- Stain mode's layout on the real engine: the registry, the Code view and the Problems pane (TASK-M8-92)
- The Stain canvas on the real engine (TASK-M8-93)
- The Display window on the real engine: the last stages and the display overlays (TASK-M8-94)

## C8 — The whole app

Everything else the instrument does: the state contract, inspector, Listen, run and display windows, export and
sequences, measurement and research tools, the browser build and the release checks.

- The state contract: SimConfig / RenderState / ViewUI, provenance serialisation and the GUI-sized snapshot
  (TASK-M8-01)
- Headless drive: the engine from a JSON state file, and the native headless survey (TASK-M8-02)
- Undo in the contract: the shared SetField history, coalescing, and per-field blast-radius metadata (TASK-M8-03)
- The gui crate and its firewall: F3 layer, crate-graph wall, and the GUI string lints (TASK-M8-04)
- Explore shell: top bar, footer, theme, and the figure left uncovered (01_main.png) (TASK-M8-05)
- Overlays ▾: the grouped toggles, their tiers, the Tier-3 buffer and quad drill-down (TASK-M8-08)
- Listen, 1: the signal, spectrum, pitch, windows, channels and loudness from the one computeIC integration, with
  the audio out (TASK-M8-12)
- Keyboard scopes, global and in-scope keys, focus ring and breadcrumb (07_keyboard.png) (TASK-M8-13)
- Run window: integration, escape and refinement settings under their contract names (04_windows.png) (TASK-M8-24)
- Quality warnings (Green / Yellow / Red), per-choice suppression, and the target-utilisation ceiling (TASK-M8-25)
- Telemetry: the full calibration record, binding resource, throttling inference, passive mode (TASK-M8-26)
- Console: the footer opened, on the telemetry stream (12_console.png) (TASK-M8-27)
- Profiler window, leak detector, precomputed summaries and `prin profile query --live` (04_windows.png)
  (TASK-M8-28)
- Profiler › Run benchmark…: the dialog, the run, the results sheet and Apply suggested tier, and the first-launch
  Quick prompt (TASK-C8-02)
- Failing gracefully: no-GPU fallback, device loss, and every degradation shown and logged (TASK-M8-29)
- Export: the frame loop in blocking mode, as a background job (TASK-M8-30)
- Keyframes, the export planner and the spec object: serialise, share, re-render (TASK-M8-31)
- Export & share window: image with pxpack, state snapshot and share link, present mode (04_windows.png)
  (TASK-M8-32)
- Import picture, saved views, and record a time sweep (09_importrecord.png) (TASK-M8-33)
- Measure: boundary dimension on a dragged region (10_measure.png) (TASK-M8-34)
- Research (v2): periodic-orbit seeding, Newton refinement, catalogue comparison (11_research.png) (TASK-M8-35)
- Research (v2): continuation, Poincaré return map, side by side with linked views (11_research.png) (TASK-M8-36)
- Browser build (v1 web): the wasm engine in a Web Worker on an OffscreenCanvas, the main-thread shell, coi-
  serviceworker (TASK-M8-37)
- V1 web's membranes: set_field and SAB input in, the ~10 Hz snapshot out, the inspector worker (TASK-M8-38)
- The fixed scripted suite and the calibration campaign (TASK-M8-39)
- Benchmark scenarios as data, one runner, and `prin bench run/show/report/compare` (TASK-C8-01)
- Browser gates: Tier-N on real browsers, and the Playwright fragment goldens (TASK-M8-40)
- Release gates: the non-Metal parity run, the seam test index, the atlas horizon label, v1 web on GitHub Pages
  (TASK-M8-41)
- Screen sweep: every artboard screen present, every schema field exposed (TASK-M8-42)
- The compute fast-math setting in the GUI: the Run window control and the Profiler's fast-math line (TASK-M8-43)
- The round-trip rule: every export the identity on `SimConfig` and `RenderState`, no empty struct, every
  destination carrying the links, and the tiers reported in the GUI (TASK-M8-53)
- Listen, 2: the voices, the 39 instruments and makeVoice, strikes on the trajectory's beats, the events and the
  effects (TASK-M8-59)
- Listen, 3: horizon and dwell, the spectrum, spectrogram and readout, sweep a row and the known orbits (TASK-M8-60)
- The trace in Explore, 1: the Trajectory panel's tabs, previews, Trace and Numbers sections, Overlays ▾ › Trace,
  T, ⇧T and Y, and the trace's look (TASK-M8-61)
- The trace in Explore, 2: the Trace… window (TASK-M8-62)
- The trace and Listen in Explore, 3: Listen in the bottom band, the shared time axis and the legend's trace block
  (TASK-M8-63)
- Listen in Explore, 1: the Trajectory panel's Listen section, the Figure's L and the window highlight (TASK-M8-64)
- Listen in Explore, 2: the Listen… window (TASK-M8-65)
- Result, the job: one job.json run in the app or by `prin batch`, f32 or f64, the result folder and its payload,
  cancel and resume (TASK-M8-74)
- Result, the GUI: the Result window, the result view and Compare (TASK-M8-75)
- The Sequence window: whole keyframes, the timeline and its lanes, easing and transitions per segment, the
  `.principia-seq.json` file (TASK-M8-77)
- Sequence: Play preview, the proof strip, the planner's estimate and its basis, and export with the spec beside
  it (TASK-M8-78)
- The spotlight reel's machinery, natively: the reel builder and its manifest, the film played without compute,
  the takeover that keeps playing, the Spotlights window (TASK-M8-79)
- Export details: every measured colour window frozen at export and at Record, the clipped shares and the export
  block, the colour-vision simulation never in an export unless asked, `render_scale` 1, and the physical size
  (TASK-M8-80)
- The video carrier: a recording's record once per file in its container, always (a GIF's comment and application
  extension, an MP4's comment, a WebM's tag), read back with the embedding's three outcomes (TASK-M8-81)
- The uploaded picture: ingest in the app's own code, the SHA-256 `PictureRef` in tier 2, the content-addressed
  store, the `prPc` chunk, and a visible "Picture missing", never a substitute (TASK-M8-82)
- The WebM carrier in the browser build: the `PRINCIPIA_SPEC` tag on the file, tested by the Playwright suite
  (TASK-M8-85)
- The Explore shell on the real engine: egui-wgpu on the engine's device, the status line, the footer and `budget-
  bound` (TASK-M8-88)
- Keyboard scopes and keys on the real engine (TASK-M8-91)
- The Run window on the real engine, and the stepper pairs' warnings (TASK-M8-95)
- The Sequence's morph segment (TASK-M8-108)
- Time-axis mode in Sequence and export sidecars (TASK-M8-116)
- Hold L_z on the GPU: the outcome tags across backends and the calibration of τ (TASK-M8-118)

## C9 — Replication (last)

The published results reproduced as measurements, never gates: each target declares its tier and reports pass,
explained divergence or unexplained.

- The orbit atlas through encode and decode: E and L per row (OC-2), and every atlas IC in the 8D latent chart
  (AT-D6) (TASK-M9-01)
- Atlas closure at order (OC-1, the first batch's DRAFT-VAL-01) (TASK-M9-02)
- Published against refined: each atlas source's published initial conditions measured (OC-3) (TASK-M9-03)
- The syzygy count against the atlas's `syz` on every row (OC-5), and its counting convention (TASK-M9-04)
- The atlas's per-family coverage report (OC-6) (TASK-M9-05)
- Li & Liao's free-fall orbits: closure, rest at T/2 and the alias pairs (LL-1, LL-2, the first batch's DRAFT-
  VAL-02), and the website-only rows (LL-5) (TASK-M9-06)
- Li & Liao's known orbits against Standish 1970 and Yasko & Orlov 2015 (LL-3) (TASK-M9-07)
- Heggie 1974's regularisation targets: the step ratio against Aarseth–Zare (H-1, H-2, DRAFT-VAL-04) and C/3
  toward the Lagrange triple collision (H-3, DRAFT-VAL-03) (TASK-M9-08)
- Tanikawa & Mikkola 2021's triple-collision points on the free-fall plane (TM-1) (TASK-M9-09)
- Agekyan & Anosova II: decay times from exact rest (AA-6, the first batch's DRAFT-VAL-05) (TASK-M9-10)
- The no-integration controls: Monaghan's planar eccentricity row and the random-triangle Z (Q-2's Theory row,
  Q-4; DRAFT-VAL-06) (TASK-M9-11)
- The butterflies' two published periods, decided by closure (BG-4, the first batch's DRAFT-VAL-07) (TASK-M9-12)
- The Belgrade orbits close: the PRL's 15, the gallery's 206 sequence orbits and 57 BHH satellites, Hudomal's 38
  and his E = −0.5 periods (SD-1, BG-1, BG-2, HU-3, HU-5) (TASK-M9-13)
- Period against word length: Li & Liao's period per letter (LL-4) and Hudomal's seven sequence fits (HU-4)
  (TASK-M9-14)
- The prototype's integrator and classifier numbers, reproduced or explained (tier D: D-1, D-2, D-4, D-5)
  (TASK-M9-15)
- Burrau and the Pythagorean family: Chitan et al.'s sixteen triangles, Boekholt et al.'s lifetime and τ_L and
  Mikkola's bounce orbit (CH-1, B-1, B-2, V-6) (TASK-M9-16)
- Aarseth et al. 1994a: the Burrau neighbourhood on six rings per body (P-1 to P-6) (TASK-M9-17)
- Aarseth et al. 1994b: triple approaches and parameter distributions over region D, circle R and the Pythagorean
  escapers (Q-1 to Q-3) (TASK-M9-18)
- Agekyan & Anosova's near-rest runs and Anosova 1969's unequal masses (AA-2 to AA-5, AA-7, AA-8, AN-1) (TASK-M9-19)
- Lehto et al. 2008 at a small grid: local homogeneity, reversibility and the step-halving controls (L-3, L-4,
  L-5) (TASK-M9-20)
- Region-D statistics: the lifetime tail and lifetime against Lyapunov time (V-9, MT-1) (TASK-M9-21)
- Converged-code references: Boekholt et al. 2020's lifetime grid and Breen et al. 2020's Brutus runs (GG-1, BR-1)
  (TASK-M9-22)
- Binary-plus-single targets: Trani et al.'s planar row, Manwadkar et al.'s escape probabilities, Stone & Leigh's
  closed forms (TR-1, MK-1, SL-1) (TASK-M9-23)
- Parischewsky et al.'s equilateral-triangle experiment around Lagrange (PA-1) (TASK-M9-24)
- Hierarchical stability limits, prograde and retrograde (V-11) (TASK-M9-25)
- Lehto's lifetime map at full resolution, and the shield's mirror symmetry (L-1, V-8 (a)) (TASK-M9-26)
- Hudomal's return-proximity map over the Šuvakov plane and the butterfly III zoom (HU-1, HU-2) (TASK-M9-27)
- Symbol-sequence maps: Tanikawa & Mikkola's free-fall and non-zero-L maps, and Valtonen et al.'s L/M/A/H
  partition (TS-1, TS-2, V-1/V-3/V-7) (TASK-M9-28)
- Li, Li & Liao's mass-plane contours for one periodic family (LLL-1) (TASK-M9-29)
- The raised-cap run: the targets recorded as "not evaluated within cap", evaluated at a calibrated raised horizon
  (TASK-M9-30)
- Deep zooms of the shield: Lehto's 3.125×10⁻⁶ zoom and Valtonen et al.'s three windows (L-2, V-8 (b)) (TASK-M9-31)
- The prototype's Chazy-classifier numbers against Fewbody, reproduced or explained (D-3) (TASK-M9-32)
- Stability as structure: atlas orbits off the Šuvakov plane against their published stability (OC-4, AT-D5)
  (TASK-M9-33)
- The atlas's free-fall orbits marked on the closure field, one plane per mass set (AT-D2) (TASK-M9-34)
- The Šuvakov plane rendered, at equal masses and at the eight values of m₃, and OC-4 there (AT-D1, AT-D3, OC-4 on
  the Šuvakov plane) (TASK-M9-35)
- The ruled external targets scored: FF-H, the figure-eight's published shape-sphere curve and SP-1 against
  Szebehely & Peters (TASK-M9-36)
- M9's report: every external target with its tier, outcome and like-for-like checks (TASK-M9-37)
- The GPU-path tightening defined: what agreeing better means, while staying interactive (TASK-M9-38)
- The softened force law: Plummer softening, built after the core build, and the refusal of ε ≠ 0 lifted
  (TASK-M9-39)
- The Šuvakov-plane chart, at equal masses and at fixed m₃, defined and built (TASK-M9-40)
- Candidate original results: per-point FTLE maps over the batch-152 planes and a time-weighted shape-sphere
  occupancy accumulator (TASK-M9-41)
- GG-1: the eight neighbours of (3, 43), a peek (TASK-M9-42)
- LL-6: all 316 free-fall orbits against the authors' table, a peek target (TASK-M9-44)
- The side-ratio remap onto the shape sphere at rest, its preset on the equilateral point, and the two side ratios
  in the Inspector (TASK-M9-45)
- The closure field as a stability reading: well width and `alpha`, and Szebehely–Peters located on it, a
  measurement (TASK-C9-01)

## Excluded

- Chazy *subtitles* (captions over the figure) are excluded from the dev GUI by an old decision; the Chazy
  classifier and its stains are in C7.

## The PC

Work for the Windows PC's own session (`plan/MACHINES.md`). Not a checkpoint.

- The Windows runner's DX12 run, Phase 2: the GPU suites on DX12 as a named measurement (TASK-M0-58)
- Mutation testing off the merge path: the PC's weekly sweep of the kernel (TASK-M0-73)

