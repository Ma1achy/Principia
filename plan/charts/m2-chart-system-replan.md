# M2 chart system: the re-plan around the owner's decisions of 10 Nov 2026

**Status: proposed, 10 Nov 2026.** This document plans; it builds nothing and dispatches nothing. It answers the owner's
re-plan task of 10 Nov (R-926), on the owner's decisions R-925 and R-926. `plan/tasks.yaml` and the task files are
**not** edited here: the task list in § B is proposed, and `tasks.yaml` follows once the one-way items of § E are
answered (applied per R-369: task scoping). The owner's instruction of 10 Nov (R-925 quotes it) holds new chart
tasks, TASK-M2-57 to TASK-M2-85; PR #469, not merged when this was written, records the pause and its exceptions, so
every task below, the first slice of § C included, is proposed and waits for the pause to lift. Integrator work carries
on as agreed.

**Sources.** The owner's words: R-925 and R-926, verbatim in `decisions.md`. The prototype, a proposal that stays on the
orchestrator's drive and is cited by path, never copied: `orch/charts/prototype-2026-11-10/` (`README.md`,
`chart-system-prototype-design.md`, here "the prototype design", `lut-exploration-2026-11-10.md`,
`generic-chart-system-2026-11-10.md` with its § 10 corrections, `chart-review-2026-11-10.md` with its § 5 point 4
withdrawn). Its § 9 (the quadtree) is a stand-in and is not copied: this plan works against
`docs/contracts/principia_scheduler_contract.md`, the caching contract and `docs/design/principia_deep_zoom.md`. The
orchestrator's surveys against `origin/main` 054e7f72 (`orch/charts/replan-2026-11-10/`) and its earlier reading
(`orch/charts/generic-2026-11-10/step1-code-claims.md`, which the owner accepted, and the step 2 and step 3 notes); file
and line evidence below was spot-checked on 054e7f72. Every number from the prototype is a prototype measurement on toy
charts (R-649): nothing here rests on one until § C and § B re-measure it on the kernel's charts.

**Where this lives.** `plan/charts/` is a new folder; `python3 plan/check_plan.py` accepts it (it reads plan documents
for references only).

## 0. Ids used here, each with a plain-English gloss (R-693)

**Rulings.** R-82, one seed rule for momentum on energy charts; R-90, when the deep-zoom decoder switches to its
linear form; R-92, which navigation changes the cached simulation; R-113, where the linearised decoder's CPU part sits;
R-116, shader decode generated from the one Rust source; R-186, timings run on the owner's Mac; R-369, the orchestrator's two-way decisions; R-407, how an
axis extends past its range; R-408, statistics count each system once; R-437, `atan2` defined as 0 at the poles; R-469, navigation stops at the last valid
system; R-474, the two tilt maps, "Tip as sheet" the default; R-507, tilt units; R-542, the state map's "Chart
position" colouring; R-565, the integrator's fixed time grid; R-593, the lock offers a re-centre, never forces it;
R-599, collapse judged per axis; R-602, the unregularised grid; R-632, the deep-zoom f32 floor is measured; R-633,
central-difference slopes take a fixed step; R-649, targets are measurements, never gates; R-650, replication tiers;
R-693, every id glossed; R-837, at most 4 extra GPU decode entries; R-839, live views decode on the GPU (superseded
by R-926); R-842, time on an axis as a chart mode (amended by R-926); R-849, the locked anchor slides continuously; R-853, the chart-space model corrected, the mirror half never folded;
R-850, one path abstraction for every view edit; R-863, the five build stages of the chart work; R-864, the chart path
is one object, bitwise first; R-866, locking doesn't move the view; R-867, the anchor stored as a state; R-872, the
2 s start-up limit is the Mac's; R-894, one leaf per operation after stage 1; R-898, latent `β` periodic; R-902, the
lock belongs to the path; R-908, product tests never retired; R-916, M2 is the whole non-visual chart system; R-917,
build order charts, then integrator, then visuals; R-918, M2's integrator-needing maths tested on exact trajectories;
R-919, the owner's answers on the chart plan; R-920, the slider spans; R-925, the owner's six chart decisions of 10 Nov;
R-926, the owner's eleven chart-system decisions of 10 Nov.

**Review queue.** RQ-758, slopes against central differences; RQ-759, what the lock's re-solve guarantees; RQ-760,
the other path arms, the mass block, statistics and folds on a blend; RQ-761, time on the integrator's integer grid;
RQ-762, the window's per-coordinate scale; RQ-763, how much of stage 1 stays bitwise; RQ-764, the momentum seed;
RQ-765, what the GPU decode entries become; RQ-766, a large quad's forward table; RQ-767, the table route's accuracy
outside a morph; RQ-768, what `ctx.chart.z` means in R-926; RQ-769, stage 3 beside stages 1 and 2; RQ-770, whether pan and zoom keep the cache.

**Tasks** are glossed by their row in § A.1 the first time they appear; requirements by their row in § A.2. Two
appear outside those tables: TASK-M2-36, the two tilt maps on the latent-control charts (built), and REQ-INT-038, the
horizon counted in whole integer steps.

## A. Reconciliation

Every task, requirement and ruling of the current M2 chart plan, and the tasks of other milestones it bears on, marked
**KEEP** (stands as written), **CHANGE** (stands, its text or scope rewritten), **DROP** (retired, its job gone or
taken by another) or **NEW** (proposed). Where a mark waits on an owner's answer, the RQ is named; the mark is the
plan's recommendation.

### A.1 Tasks

| Task | What it builds (gloss) | Mark | Why |
|---|---|---|---|
| TASK-M2-57 | the decoder's mirror step removed, latent `β` periodic | KEEP | Independent of R-926; proposed to land before TASK-M2-61's baseline, its edge to TASK-M2-65 dropped (applied per R-369: task scoping), so the baseline is frozen once |
| TASK-M2-58 | the gate: the owner's rulings applied to the composition design | CHANGE | Also carries R-925, R-926 and the answers to RQ-758 to RQ-770 into the design's body |
| TASK-M2-59 | the chart composition in the contracts | CHANGE | Its "three lowerings" become one CPU evaluation and two fragment routes (closed form, table); adds the CPU-to-GPU record and table contract (N2) |
| TASK-M2-60 | chart paths in the contracts | CHANGE | A path is window edits plus the system blend (R-926 items 1, 3); § 17.2's other arms wait on RQ-760 |
| TASK-M2-61 | the chart-path object, today's operations bitwise through it | CHANGE | Core types become map, window `(c, Q)`, blend and lock (R-926); the type sketch is revised from v10 for the owner (R-864 item 1, R-919); its bitwise scope waits on RQ-763; its window scale on RQ-762 |
| TASK-M2-62 | locking doesn't move the view: pin at the clicked pixel | CHANGE | The pin is R-926's one constraint and a window edit re-solves `c`; snap-back, refusals and R-866 stand; RQ-759 |
| TASK-M2-63 | Save view as chart on curved views, the pivot record, tilts into held `ν₀`, `L_z`, `K` | KEEP | Tilts are edits of `Q` already; R-920's spans stand |
| TASK-M2-64 | consolidation: one leaf per operation, GUI routes thin adapters | CHANGE | The end state is window edits (move `c`, scale `Q`, rotate `Q` per R-474 map, cut, rebase, lock, re-solve, set `s`); `engine::nav::directions`' "morph" renamed a tilt (R-925 item 4; applied per R-369: naming) |
| TASK-M2-65 | paths under a lock: stored anchor, based morph, slide | CHANGE | The morph is the state blend (R-926 item 3), the lock one constraint re-solved (item 4); continuity and the select wait on RQ-759 and RQ-760 |
| TASK-M2-66 | the primitive registry, refusals, labels, digest | CHANGE | The lowering label becomes the fragment route (RQ-765); each map gains a slope tier (R-926 item 9) and a scale (RQ-762) |
| TASK-M2-67 | CPU decode and encode through the composition | KEEP | It is the f64 evaluator R-926 item 5 calls for |
| TASK-M2-68 | region D on the CPU | KEEP | A map; its slide arm calls the lock's solver (RQ-759) |
| TASK-M2-69 | hold `L_z` on any chart, on the CPU | KEEP | A map option |
| TASK-M2-70 | the virial pair | KEEP | A map; "a baked pair inside the cap" read under RQ-765 |
| TASK-M2-71 | warps, polar pair maps, a statistic's measure | KEEP | Map composition |
| TASK-M2-72 | curved slices: momentum and mass spheres | KEEP | Maps, the window flat in exponential-map coordinates |
| TASK-M2-73 | numerical inverses and the general solve, on the CPU | CHANGE | Split: its solver core serves the lock's re-solve before TASK-M2-65 (N11); "CPU batch" and "CPU-only" lose their meaning (every chart is CPU-evaluated; the fragment reaches it by table, R-925) |
| TASK-M2-74 | the primitives of 6 Nov | KEEP | Map coordinates, each with a slope tier |
| TASK-M2-75 | region D's exact map, every target's native coordinates | KEEP | A map |
| TASK-M2-76 | the primitives of the full order | KEEP | As TASK-M2-74 |
| TASK-M2-77 | paper frames as presets | KEEP | Presets over maps and windows |
| TASK-M2-78 | the fixed-`k` chart | KEEP | A map |
| TASK-M2-79 | the binary-plus-single scattering chart | KEEP | A map |
| TASK-M2-80 | points of interest's maths | KEEP | The nearest chart point is the lock solver's output and residual |
| TASK-M2-81 | path re-timing, alignment, flags | CHANGE | Its arms shrink to the system blend and window edits; "CPU-batch path lowering" goes |
| TASK-M2-82 | the trajectory interface and its exact fixtures | KEEP | Needed more: time as a coordinate reads through it |
| TASK-M2-83 | time-axis chart mode's maths | CHANGE | Time is a coordinate (R-926 item 2), tilt toward time allowed, the cut to an IC-only chart gone; the integer grid waits on RQ-761 |
| TASK-M2-84 | the symmetry filter's maths | KEEP | Display after rendering, unaffected |
| TASK-M2-85 | acute-angle Burrau charts | KEEP | Maps (the seed, RQ-764, may touch its seam) |
| TASK-M6-07 | the linearised decoder, `x₀ + J_D·δ` on the GPU | CHANGE | It is the one-cell forward table (R-926 item 11); its slope tier waits on RQ-758 |
| TASK-M5-04 | the quad request: flags, per-quad uniforms | CHANGE | The quad goes with its forward table; `DECODE_MODE` means grid or one-cell patch (RQ-766) |
| TASK-M4-05 | flat-grid compute: in-kernel decode, the march | CHANGE | The integrator entry reads records, never decodes (R-926 item 5); a per-sample stop waits on RQ-761 |
| TASK-M8-102 | CPU batch for numerical inverses | CHANGE | CPU evaluation is every view's route (R-839 superseded); its branch rule and seam masking stay, as part of the evaluator |
| TASK-M2-06 | the kernel decode stage on the GPU (built) | KEEP | Built; it stays as the test and parity entry under RQ-765 (a) |
| N1 | **NEW:** the first slice, § C | NEW | One real chart through CPU f64, per-quad table, records, integrator, against today's decode |
| N2 | **NEW:** the CPU-to-GPU record and table contract | NEW | R-926 items 5, 6: the record is REQ-DEC-005's `(m, r, p)`, its descriptor and tag words, a branch word and a stop index (RQ-761); `SimState` stays a separate buffer |
| N3 | **NEW:** slopes in three tiers | NEW | R-926 item 9: a dual-number type over the four bit-level traits, a flag set at each kink site; RQ-758 |
| N4 | **NEW:** the per-quad forward-table builder on the CPU | NEW | R-926 item 6: threaded f64 evaluation and narrowing, measured against the frame budget (M-A, § B) |
| N5 | **NEW:** the blend chart | NEW | R-926 item 3: state blend, nested blends, both ends navigated, ends exact |
| N6 | **NEW:** time as the last coordinate | NEW | R-926 item 2: the map's time coordinate, the playhead as the window's centre, target time to integer index (RQ-761) |
| N7 | **NEW:** the backwards table on the CPU | NEW | R-926 item 7: bake, key, fold flag, search, built and measured on the CPU before any shader |
| N8 | **NEW:** the table debug views | NEW | R-925 item 6, R-926 item 10: five views; visual, so after M2's gate (R-917) |
| N9 | **NEW:** the fragment table route | NEW | R-925 items 1, 2: the GPU side of N7 for Chart position and the presets; visual, after M2's gate |
| N10 | **NEW:** the seed conversion | NEW | Only if RQ-764 (a) or (b): before TASK-M2-61's baseline |
| N11 | **NEW:** the lock's solver core | NEW | TASK-M2-73's core, moved before TASK-M2-65 for R-926 item 4 |

### A.2 Requirements

Of the tasks above. Each line: id, gloss, mark, why.

| Requirement | Gloss | Mark | Why |
|---|---|---|---|
| REQ-DEC-055, REQ-DEC-056 | mirror step removed; periodic `β` link | KEEP | TASK-M2-57 as written |
| REQ-CHART-125 | the composition's rulings applied before its tasks | CHANGE | Adds R-925, R-926 and RQ-758 to RQ-770 |
| REQ-CHART-094 | composition in the contracts, three lowerings | CHANGE | One CPU evaluation; fragment closed form or table |
| REQ-CHART-095 | region D defined in chart_reference | KEEP | A map |
| REQ-CHART-108, REQ-CHART-109, REQ-CHART-110 | both-held's computed form, outcomes; encode under a hold | KEEP | Map maths |
| REQ-CHART-112 | the path type and its arms, blend at the last differing layer | CHANGE | State blend and window edits (R-926); other arms per RQ-760 |
| REQ-CHART-147 | one chart-path object, core types, sketch to the owner | CHANGE | Types from R-926; bitwise scope per RQ-763 |
| REQ-CHART-113 | path ends bitwise, `s = 0` and `1` by selection | KEEP | Gives R-926's "both ends are exactly the end charts" |
| REQ-CHART-160 | easing per path segment | KEEP | Unaffected |
| REQ-CHART-166 | locking doesn't move the view | KEEP | R-866 stands |
| REQ-CHART-163 | pivot at an off-centre pin on curved charts | KEEP | The pin is the constraint; its wording of the `d0` mechanism is TASK-M2-62's to check |
| REQ-GUI-429 | the inexact pin's bound, a calibration | KEEP | Only if the pin can't be exact |
| REQ-CHART-167 | Save view on curved charts, in the engine | KEEP | Save is a rebase of the window |
| REQ-CHART-157 | the pivot record as the pinned state | KEEP | R-867 |
| REQ-CHART-141 | held-`K` warp constants, a calibration | KEEP | A map's constant |
| REQ-CHART-188 | the slider spans | KEEP | They are the window's scale where defined (RQ-762) |
| REQ-CHART-156 | one leaf per operation after stage 1 | CHANGE | The leaves are window edits; two tilt maps stay (R-474) |
| REQ-CHART-168 | every edit through REQ-CHART-060's stop | KEEP | R-469 stands; how it meets the re-solve is RQ-759 |
| REQ-CHART-115 | the based homotopy under a lock, stored anchor | CHANGE | State blend plus re-solve; the stored-words select per RQ-759 |
| REQ-CHART-146 | "nearest point in B" defined | KEEP | The re-solve's metric (RQ-759) |
| REQ-CHART-158 | the lock's tolerance, a calibration | KEEP | Unaffected |
| REQ-CHART-159 | the saved-file rewrite of stage 2 | KEEP | Unaffected |
| REQ-CHART-161 | the lock across blends; the owner's gate test | CHANGE | Pan, zoom, rotate on a blend apply to both ends (R-926 refines R-902) |
| REQ-CHART-164 | the pivot record across a switch | KEEP | Unaffected |
| REQ-CHART-096 | one registry row per primitive, its inverse class | CHANGE | The inverse class routes the fragment, not the integrator; a slope tier added |
| REQ-CHART-097 | any axis pairs with any other | KEEP | Unaffected |
| REQ-CHART-098 | a chart's lowering label | CHANGE | RQ-765 |
| REQ-CHART-099 | every chart reopens bitwise; the digest | KEEP | The digest is the CPU evaluator's |
| REQ-CHART-127, REQ-CHART-128 | round-trip test per primitive; each row's class | KEEP | Unaffected |
| REQ-CHART-169 | the f32 band's label | CHANGE | There is no product f32 decode on the compute side; the label concerns the fragment's generated code and the test entries |
| REQ-CHART-170 | CPU decode and encode through the composition | KEEP | The f64 evaluator |
| REQ-CHART-101, REQ-CHART-142, REQ-CHART-171, REQ-CHART-180 | region D's round trips, presets, CPU decode, tolerance | KEEP | Maps |
| REQ-CHART-102, REQ-CHART-111 | hold `L_z`; ray-degenerate momentum planes | KEEP | Maps |
| REQ-CHART-103, REQ-CHART-139, REQ-CHART-187 | the virial axis, its floor, its tolerance | KEEP | Maps |
| REQ-CHART-104, REQ-CHART-131, REQ-CHART-181 | warps and pair maps; a statistic's measure; tolerance | KEEP | Maps |
| REQ-CHART-105, REQ-CHART-106, REQ-CHART-182 | curved slices; `ExpMap<Target>`; tolerance | KEEP | Maps |
| REQ-CHART-107 | a numerical-inverse pair as CPU batch | CHANGE | Every chart is CPU-evaluated; the branch rule and seam stay |
| REQ-CHART-145 | the general numerical solve, labelled CPU-only | CHANGE | "CPU-only" moot; the fragment reaches it by table (R-925) |
| REQ-CHART-183 | the numerical inverse's tolerance | KEEP | Unaffected |
| REQ-CHART-117 to REQ-CHART-122, REQ-CHART-184 to REQ-CHART-186 | the primitives, region D's map, paper frames, their tolerances | KEEP | Maps and presets |
| REQ-CHART-143, REQ-CHART-144, REQ-VAL-354 | fixed-`k` chart; scattering chart; `f_col` | KEEP | Maps |
| REQ-CHART-172, REQ-GUI-427, REQ-GUI-428 | points of interest's maths, definitions, near-match | KEEP | The solver's output |
| REQ-CHART-114 | a frame's flags by the Gram area factor | KEEP | Applies to a blend chart |
| REQ-CHART-116 | control charts, alignment, re-timing | KEEP | Unaffected |
| REQ-CHART-124 | the metric on the blended state | KEEP | The blend's own metric |
| REQ-CHART-133 | the mirror half never dropped by a path | KEEP | Unaffected |
| REQ-CHART-173 | M2's trajectory reads through the interface | KEEP | Unaffected |
| REQ-CHART-123 | time-axis chart mode, columns once, cut to IC-only | CHANGE | Time a coordinate; cut gone (R-926); grid per RQ-761 |
| REQ-CHART-174 | tilt and slice on a time-axis chart, time never a tilt target | CHANGE | Time may be tilted toward (R-926 item 1); its scale RQ-762 |
| REQ-CHART-135, REQ-CHART-137, REQ-CHART-150, REQ-CHART-175, REQ-CHART-176 | the symmetry filter | KEEP | Display only |
| REQ-CHART-151, REQ-CHART-162 | acute Burrau charts and their range | KEEP | Maps |
| REQ-DEC-034 | `J_D` by central differences | CHANGE | RQ-758 |
| REQ-DEC-035 | global precision in f64 on the CPU, local f32 on the GPU | KEEP | Is R-926 item 11 |
| REQ-DEC-042 | the linearised decode's slope tolerance, a calibration | KEEP | Unaffected |
| REQ-GEN-018 | `J_D`'s determinant, in magnitude, matches the analytic link derivatives | KEEP | A check of any tier |
| REQ-DEC-031 | the quad's centre and half-width from the CPU | KEEP | The patch and the fragment still need them |
| REQ-PAY-064 | `QuadRequest.flags` bits | CHANGE | `DECODE_MODE`'s meaning (RQ-766) |
| REQ-SCHED-024 | deep zoom's global f64, local f32 | KEEP | Is R-926 item 11 |
| REQ-DEC-036 | `x₀` and `J_D` by central differences on the CPU | CHANGE | RQ-758 |
| REQ-TOOL-158 | per-quad local sample coordinates past `ℓ_switch` | KEEP | Unaffected |
| REQ-DEC-051, REQ-DEC-052 | the f32 decode floor, its gate | KEEP | The patch's floor (R-632) |
| REQ-INT-070 | decode in the kernel, never an IC buffer | DROP | Reversed by R-926 item 5; replaced by N2's record requirement |
| REQ-PERF-012, REQ-PERF-013, REQ-SCHED-012, REQ-SCHED-010, REQ-SCHED-011, REQ-PERF-008 | dispatch shape, march in place, horizon assert, stop gate, word buffer | KEEP | The integrator's |
| REQ-SYS-022 | no GPU interpreter; chart map and decode lower into compute | CHANGE | "No interpreter" stands; chart map and decode leave the compute pipeline |
| REQ-CHART-129, REQ-CHART-130 | jitter in chart coordinates; seams masked | KEEP | Now on the CPU |
| REQ-CHART-155 | the browser's CPU batch in a worker | CHANGE | Every chart is CPU-evaluated in the browser too |
| REQ-DEC-005 | the integrator's one input type, canonical `(m, r, p)` | KEEP | It fixes the record's content (N2) |
| REQ-DEC-007 | chart and links baked, `DECODE_MODE` a per-quad uniform | CHANGE | Baked for the CPU and the fragment, not the integrator |
| REQ-CHART-001, REQ-TOOL-030, REQ-DEC-043, REQ-DEC-047 | monomorphised charts; descriptor views; f32-eps factor; `tanh_sat` | KEEP | Unaffected |
| REQ-COL-057 | what `ctx.chart.z` holds on each chart | KEEP | RQ-768 reads it forwards |
| REQ-COL-006 | fragment decode and encode generated | CHANGE | Where closed form; tables otherwise (R-925 amends R-116) |
| REQ-RENDER-027 | the DECODE preset | KEEP | Unaffected; on a table-route chart it reads the table |
| REQ-PERF-116 | baked pairs under R-837's cap | CHANGE | RQ-765 |
| REQ-DEC-033 | switch to the linear decoder on collapse | KEEP | R-90 stands (RQ-766) |
| REQ-CHART-060, REQ-CHART-010 | the navigation stop; lock re-centre on an invariant chart | KEEP | RQ-759 |
| NEW (N2) | the record contract | NEW | The integrator reads records the CPU wrote |
| NEW (N3) | each map's slope tier, flags at kink sites | NEW | R-926 item 9 |
| NEW (N5) | a blend is a chart; edits on both ends; ends exact | NEW | R-925 item 4, R-926 item 3 |
| NEW (N5) | a signed fold indicator on a blend chart, defined (R-72) | NEW | The Gram factor is unsigned (§ 17.11); RQ-760 (iii-b) and O4, O6 need a signed one |
| NEW (N6) | time the last coordinate; the playhead the window's centre | NEW | R-926 item 2 |
| NEW (N7, N9) | the backwards table and its search; never a refusal | NEW | R-925 items 1, 2; R-926 items 7, 8 |
| NEW (N8) | the five table debug views | NEW | R-925 item 6; R-926 item 10 |

### A.3 Rulings

| Ruling | Mark | Why |
|---|---|---|
| R-82 | KEEP | Stands until RQ-764 is answered |
| R-90 | KEEP | The switch to the one-cell patch (RQ-766 recommends keeping it) |
| R-92 | KEEP | Pan and zoom re-address cached quads; reading R-926 item 6's "recompute and re-integrate" as "no inverse on the path", not "no cache", is the owner's: RQ-770 recommends it |
| R-116 | CHANGE | Amended in part by R-925 |
| R-407, R-408 | KEEP | Axis extension types and the count stand; blend folds per RQ-760 |
| R-469 | KEEP | The stop stands; with the re-solve, RQ-759 |
| R-474 | KEEP | Both tilt maps are edits of `Q` (sheet: one rotation of the frame; lean: each axis); Tip as sheet stays the default and stays built (TASK-M2-36). The prototype's lean-only tilt is a gap of the prototype, not a decision |
| R-507, R-920 | KEEP | Their units are the window's scale where defined (RQ-762) |
| R-542 | CHANGE | Refined by R-925 |
| R-565, R-602 | KEEP | The grid stands; per-sample targets on it, RQ-761 |
| R-593 | KEEP | Pending RQ-759 |
| R-632 | KEEP | The patch's f32 floor |
| R-633 | KEEP | Governs central differences wherever they remain (RQ-758) |
| R-837 | CHANGE | RQ-765 recommends retiring it as moot after a measurement |
| R-839 | DROP | Superseded by R-926 |
| R-842 | CHANGE | Amended in part by R-926 |
| R-849 | KEEP | Its slide holds; how the re-solve meets it, RQ-759 |
| R-850 | CHANGE | Amended in part by R-925 |
| R-863 | KEEP | Stage order stands unless RQ-769 (a) |
| R-864 | KEEP | Item 3's scope, RQ-763; item 5 (chart maps and decode in the kernel, shared) holds in letter: the CPU and the fragment share the source |
| R-866, R-867 | KEEP | Agree with R-926 item 4 |
| R-894 | KEEP | Its end state is the window edits |
| R-898 | KEEP | TASK-M2-57 |
| R-902 | CHANGE | Refined by R-926 |
| R-916, R-917, R-918 | KEEP | M2 is the non-visual system; the visual parts (N8, N9) wait for M2's gate |
| R-919 | KEEP | Its sketch approval now takes a revised sketch (R-864 item 1) |

### A.4 Code and contract items the owner named

| Item | Today (file:line on 054e7f72) | Mark | Under R-926 |
|---|---|---|---|
| The `Chart` trait, `map(u, v)` generic over `Float` | `crates/kernel/src/chart/mod.rs:196`, `:202`; `ChartDecode::decode_at` `:273` | CHANGE | Stays the map's source, shared by the CPU (f64) and the fragment (f32 WGSL). The window moves out of the chart: only the latent chart bakes its window in today (`chart/latent.rs:70`); curved charts take the engine's `Window` (`crates/engine/src/contract/sim_config.rs:84`). A dual number needs the four bit-level traits, not only `Float` (RQ-758) |
| `stage/decode.rs`'s `write_ic` and the integrator's input buffer | `crates/kernel/src/stage/decode.rs:605` (`ic_words`), `:653` (`write_ic`) | KEEP | The record layout is `Ic<f32>`, 15 words, masses, positions, momenta (`decode/mod.rs:59`); the host fills it (N2, N4). No integrator entry reads it yet |
| GPU decode | `stage/decode.rs:771` (15 compute entries), `crates/engine/src/decode_dispatch.rs:3` (tests and gates only), validation's decode gates, the DECODE, ROUNDTRIP and agreement presets (`crates/render/src/presets/mod.rs:30`) | KEEP (tests, presets) | Not the product compute path (R-926 item 5); stays as test and parity entries (RQ-765) and as the fragment's generated code (R-925) |
| `decode_only_with` | `crates/engine/src/decode_dispatch.rs:781` | KEEP | The f64 evaluator the slice and N4 use |
| The registry's inverse classes | `docs/design/principia_chart_composition.md` § "3. The primitive registry" (line 200 on this PR's head): "`closed` (live on the GPU where baked), `numerical` (CPU batch only, never in the kernel) or `none`" | CHANGE | `closed`: generated fragment code; `numerical` and `none`: the table route, never refused (R-925) |
| R-407 and R-408 against periodic seams and folds | `crates/kernel/src/chart/energy.rs:124` (`ExtensionType`) | KEEP | A declared extension folds an answer back into the primary range (the prototype's 99.7 to 100% on its toy sphere); an undeclared fold has no rule (RQ-760 (iv)) |
| `ctx.chart.z` | `docs/design/principia_colour_composition.md:453` (forward, generated `D ∘ Φ`) | KEEP | Forward reading, RQ-768 |
| The state map, R-542 | `docs/design/principia_state_map.md` § SM2 | CHANGE | A backwards table where no formula (R-925) |
| Decode and round-trip presets, R-116 | `crates/render/src/presets/` | KEEP | Generated where closed form, table otherwise |
| The linearised decoder | `DecodeMode::Linearised` refused: `decode_dispatch.rs:320`, `crates/kernel/src/stage/fragment.rs:160` | CHANGE | It is the one-cell forward table (R-926 item 11), built on the CPU, its slopes per RQ-758 |
| Deep-zoom precision against the one-cell patch | `docs/contracts/principia_scheduler_contract.md` § `QuadRequest.flags` | KEEP | Global f64 on the CPU, local f32 on the GPU (REQ-DEC-035). The local arithmetic keeps `δ` exact, but `x₀ + J_D·δ` is still an absolute f32 IC of order `\|x₀\|` that the f32 integrator starts from, so the product's f32 path has a floor too, the measured value quoted with `\|x₀\|` (deep zoom § 2, R-632; REQ-DEC-051) |

### A.5 Counts

Tasks (A.1): KEEP 19, CHANGE 15, DROP 0, NEW 11. Requirements (A.2, rows; a row may group several ids): KEEP 52,
CHANGE 22, DROP 1, NEW 7. Rulings (A.3, rows): KEEP 19, CHANGE 6, DROP 1 (R-925 and R-926 are this PR's). Code items
(A.4): KEEP 7, CHANGE 4. Total: KEEP 97, CHANGE 47, DROP 2, NEW 18.

## B. The updated M2 chart task list, in dependency order

Proposed. Each task gives its acceptance test, the measurement that decides any open choice, that measurement's
criterion written now, before any run (R-649), and its door: **two-way** (decided by the measurement, recorded "applied
per R-369") or **one-way** (the owner's, the RQ named). No tolerance is tuned to pass; a replication or prototype
number is a measurement, never a gate (R-649, R-650). Visual work (N8, N9, the GUI) waits for M2's gate (R-917);
integrator work (TASK-M4-05) carries on as agreed.

### B.1 Before the baseline

1. **TASK-M2-58, the gate (CHANGE).** Acceptance: every answer to RQ-758 to RQ-770 written into the composition design's
   body, each with its *Was*. Measurement: none. Door: one-way, every RQ.
2. **N1, the first slice (NEW).** § C. Door: two-way; its outcome decides nothing the owner has ruled.
3. **TASK-M2-57, `β` periodic (KEEP).** Acceptance: as its task file. Its edge to TASK-M2-65 dropped so it lands before
   TASK-M2-61's baseline: two-way, task scoping.
4. **N10, the seed conversion (NEW, only under RQ-764 (a) or (b)).** Acceptance: the conversion test of every invariant
   and mixed-energy chart image where `K* > K_min`; the legacy rule a named option. Measurement: the seam jump in the
   decoded velocity across `z_α = 0` on the kernel's `(L_z, E)` chart; criterion: after the change no neighbouring pair
   of pixels across the line differs in decoded velocity by more than the largest neighbouring difference within 8
   pixels of it on either side. Control (pitfalls § 3, the measurement shown able to fire): the same measurement under
   R-82's legacy rule, the named option, must fail the same criterion; if it passes, the criterion cannot see the seam
   and the result is void. Under the legacy rule the decoded velocity is predicted to jump by `2√(K*−K_min)` at each
   pixel on the line, 1.9545 at the chart review's one point (`orch/charts/prototype-2026-11-10/chart-review-2026-11-10.md`
   § 2, a prototype figure, R-649); no threshold depends on that number. Door: one-way, RQ-764.

### B.2 Stage 1: the object, today's operations through it

5. **TASK-M2-59 and TASK-M2-60, the contracts (CHANGE), with N2 (NEW), the record contract.** Acceptance: the
   lowering and scheduler contracts rewritten from R-926's dated notes; the record contract (REQ-DEC-005's `(m, r, p)`,
   descriptor and tag words, a branch word, the stop index of RQ-761), `SimState` a separate buffer (REQ-SCHED-012).
   Measurement: none. Door: one-way where RQ-760, RQ-761 and RQ-765 decide; the rest two-way.
6. **TASK-M2-61, the object (CHANGE).** Map, window `(c, Q)` with its scale (RQ-762), blend, lock; the type sketch
   revised from v10 and taken to the owner (R-864 item 1, R-919). Acceptance: G1 (`plan/MILESTONES.md`), in the scope
   RQ-763 answers. Measurement: the D2 pair measurement, per operation at f64 and f32, the share of a fixed input grid
   (written before the run: 4,096 inputs per operation, a seeded uniform grid over each operation's parameter range) on
   which today's route and the generic edit agree bitwise; criterion: an operation at 100% becomes one leaf, any other
   is a recorded conversion. Door: one-way, RQ-763 and RQ-762.
7. **TASK-M2-62, the pin (CHANGE).** Acceptance: G7; a window edit re-solves `c` so the pinned pixel decodes the pinned
   state bitwise on affine charts, and within REQ-GUI-429's bound where it can't be exact. Door: one-way where RQ-759
   decides (R-593, R-469); otherwise two-way.
8. **TASK-M2-63, save and held tilts (KEEP).** As its task file; tilts are edits of `Q` in R-920's units.
9. **TASK-M2-64, consolidation (CHANGE).** Acceptance: G2; the end state's window edits, two tilt maps (R-474); the
   `engine::nav::directions` "morph" renamed (applied per R-369: naming). Door: two-way.

### B.3 Stage 2: the lock and the blend

10. **N11, the solver core (NEW, from TASK-M2-73).** Acceptance: a least-squares solve over a map with a recorded branch
    rule, convergence reported per call, failures tagged. Measurement: the share of re-solves along a 64-frame morph
    that change basin under a four-seed restart against a continued solve; criterion: reported, and RQ-759 (a)'s
    continuity holds if the continued solve's largest step falls to at most 0.6 of the previous each time the frame step
    halves, twice (G8's criterion). Door: one-way, RQ-759.
11. **N5, the blend chart (NEW), and TASK-M2-65, locked paths (CHANGE).** Acceptance: G4's ends bitwise (REQ-CHART-113),
    G8 and G9, the owner's lock-with-morphs test; a blend nests (a blend's end may be a blend); pan, zoom and rotate on a
    blend reach both ends; a signed fold indicator on a blend chart is defined (R-72, a definition requirement:
    the Gram area factor is unsigned, § 17.11), for example the sign of the 2 × 2 Jacobian determinant of a fixed 2D key
    of the swept state composed with the chart, under physics review. Measurement: the held-`E*` miss mid-morph (§ 17.6's 0.054 median) reported, labelled, not a
    gate (R-850: mid-morph frames never claim held values). Door: one-way, RQ-759 and RQ-760.

### B.4 Stage 3: the registry and the maps (beside stages 1 and 2 under RQ-769 (a))

12. **TASK-M2-66 (CHANGE), TASK-M2-67 (KEEP).** Acceptance: G3, G5, G6; each row's slope tier and fragment route.
    Door: one-way, RQ-765; the rest two-way.
13. **N3, slopes (NEW).** Acceptance: each map declares its tier, exact (dual numbers), implicit or none (R-926 item 9:
    a map with no exact slopes is still a valid chart); a tier-1 or tier-2 map sets a flag at each kink site the
    orchestrator's reading lists (the feasibility boundary, `k = 0`, `warped_k` at `d = 0`, `normalise`'s edges, the
    exponential map's `r = 0`, the poles, the seed switches); a tier-3 map is accepted with no slope, each sample
    evaluated on the CPU. Measurement M-B, for each tier-1 or tier-2 map: the dual slope against a central difference
    at R-633's step, `h_k = ε^{1/3}·max(1, |x_k|)` in f64 (the step REQ-DEC-034 names), at points away from the
    flagged sites by more than that step; criterion: per column, agreement within the central difference's error at
    that step, its truncation `h_k²·|f'''|/6` plus its rounding `ε·|f|/h_k`, each of order `ε^{2/3}`, with `|f'''|`
    bounded from the map's own closed form and stated before the run; the control flips the sign of a column whose
    norm is at least 100 times that bound, and must fail. Door: one-way, RQ-758.
14. **N4, the forward-table builder (NEW).** Acceptance: a quad's table built from `decode_only_with` on threads,
    narrowed once, bitwise equal to the per-sample evaluation. Measurement M-A, criterion written now: on the Mac,
    1920 × 1080 samples per chart, median of 5 warm runs; PASS if one thread reaches 8.1·10⁶ samples a second
    (composition § 17.12's in-motion coarse cover, 32,400 samples in 4 ms) on every chart of § C, the all-core rate
    reported against 4.97·10⁸ a second (the whole screen at High, N = 16, E = 3, 16.7 ms) as information. Door: two-way.
15. **TASK-M2-68 to TASK-M2-72, TASK-M2-74 to TASK-M2-79 (KEEP), TASK-M2-73 (CHANGE).** As their task files, each map
    with its slope tier.
16. **N7, the backwards table on the CPU (NEW).** Acceptance: for each kernel chart of § C, a baked backwards table and
    search on the CPU return every forward node to its own cell. Measurement: at `t = 0` and after a run, 256 × 256
    samples, table 64, scan 16 (the prototype's sensible point, re-measured, not adopted): the wrong share (a worse
    match than an exhaustive search of the same table) and the round-trip error in cells; criterion written now: PASS if
    the wrong share at `t = 0` is at most the prototype's figure for the same kind of chart (0.00% on the plain sphere
    and latent charts, 0.12% on `(L_z, E)`); EXPLAINED DIVERGENCE if worse with a named cause; else UNEXPLAINED. The
    accuracy outside a morph is RQ-767's. Door: two-way for the construction; one-way for the accuracy, RQ-767.
17. **TASK-M2-80 (KEEP).** As its task file.

### B.5 Stages 4 and 5

18. **TASK-M2-81 (CHANGE).** Acceptance: G10 on the system blend and window edits. Door: two-way.
19. **TASK-M2-82 (KEEP)**, then **N6, time as a coordinate (NEW), with TASK-M2-83 (CHANGE).** Acceptance: G11; a
    tilt toward time at 90° equals the time axis at its home extent (RQ-762's check); a target time maps to the latest
    boundary at or before it in integers. Measurement: § 18.5's C1 with per-sample stops, criterion as C1 wrote it (every
    row bitwise against a run with that row as horizon, `bounded` excepted). Door: one-way, RQ-761 and RQ-762.
20. **TASK-M2-84, TASK-M2-85 (KEEP).**

### B.6 After M2's gate (R-917)

21. **TASK-M4-05 (CHANGE):** the integrator entry reads records, a per-sample stop (RQ-761). **TASK-M5-04, TASK-M6-07
    (CHANGE):** the quad's forward table, the one-cell patch (RQ-758, RQ-766). **TASK-M8-102 (CHANGE).**
22. **N9, the fragment table route, and N8, the five debug views (NEW):** the forward table by block, the forward-table
    check, the backwards table over its key, the route, the distance off the chart. Acceptance for each view: a test
    that renders a known table and reads back its colours, with a control (a table off by one node) that fails.
    Measurement: per-sample cost of the backwards pass on Metal at 512 × 512, criterion written now: reported against
    the 16.7 ms frame budget; PASS if the pass takes at most a quarter of it.

### B.7 The prototype's open points, each a named open item

Each with its owner (a role) and the measurement that closes it. None is solved here.

| # | Open item | Owner | Measurement |
|---|---|---|---|
| O1 | Nothing has run on the kernel's real charts; every number is the JS/WGSL prototype's (§ 13.1; R-926) | implementer of N1 | § C, then N4, N7 and M-B on the kernel's charts |
| O2 | Exact slopes not built; every prototype Jacobian is a finite difference (§ 13.2) | implementer of N3; owner, RQ-758 | M-B (B.4 item 13) |
| O3 | Tilt lean-each-axis only in the prototype (§ 13.3) | implementer of TASK-M2-61 | none needed: R-474's both maps, built (TASK-M2-36), each re-expressed as an edit of `Q` and tested bitwise per G1 |
| O4 | A blend 0.4 to 1.0% wrong backwards, cause unknown (`α × β` to the sphere; § 13.4) | implementer of N7; physics reviewer for the cause | the same blend on the kernel's latent chart (periodic `β`, TASK-M2-57) to the exponential-map sphere: wrong share by route, against the folded share by the signed fold indicator N5 defines (R-72; the Gram factor is unsigned, § 17.11), at table 64 and 128 |
| O5 | No lift for periodic angles; an antipodal vortex; blends can fold (§ 13.4) | implementer of N5; owner, RQ-760 | the review's lifted-blend check: jump between neighbouring pixels at mid-morph, lifted against per-pixel shortest arc (prototype: 3.14 rad against 0.002) |
| O6 | A fold no extension declares has no rule for the true copy (§ 13.5) | owner, RQ-760 (iv) | folded share per blend pair, by the signed fold indicator N5 defines (R-72), with § 17.11's limit stated: a fold narrower than a sample can be stepped over |
| O7 | Feasibility by a plain comparison in the prototype (§ 13.6) | implementer of N1 | the kernel's exact predicate (`decode/invariant.rs:273`) is used, and the review's f32/f64 triple is the measurement: the triple run through the kernel's predicate and through the prototype's plain comparison, each verdict reported; criterion: the kernel's predicate gives the f64 verdict on the triple and the plain comparison, the control, does not |
| O8 | Records absolute f32, so "ragged grid lines at deep zoom" (§ 13.7). The product's f32 path has the same floor: `x₀ + J_D·δ` is an absolute f32 number of order `\|x₀\|` that the f32 integrator starts from (deep zoom § 2, R-632) | implementer of TASK-M6-07; owner, RQ-766 | the f32 path's distinguishable depth as integrated, quoted with `\|x₀\|`, at an O(1) coordinate and at the chart origin (REQ-DEC-052's gate; its value an R-71 calibration, REQ-DEC-051); criterion: reported with the level reached, never asserted against a fixed depth; deep zoom § 2's "a floor near ~50" applies only to an f64 sum and integration, measured separately on the CPU |
| O9 | `q` not rescaled by size in the prototype's `live` (§ 13.8) | implementer of N7 | the kernel's encode (step 0, `canon.rs:9`) is reused, never a new map; measured by a backwards round trip on states on the chart's own image (or through the full-latent encode), each also taken at a size `I ≠ 1`; the encode's step 0 rescales to `I = 1` (REQ-ENC-006, the size gauge), so a decode returns the canonical, size-gauged state, never the original `I`; criterion, two checks: the chart point found for the state at `I ≠ 1` equals the one found for the same state rescaled to `I = 1`, within the stated tolerance, and the decode equals the state's canonical form; the prototype's unrescaled `live`, the control, lands on a different chart point and fails the first check |
| O10 | The seed not ruled (§ 13.9) | owner, RQ-764 | B.1 item 4 |
| O11 | No timing of the CPU side in Rust with threads (§ 13.10) | implementer of N4 | M-A (B.4 item 14) |
| O12 | Tracking across morph frames does not work, 69 to 88% (§ 12) | physics reviewer | none planned: each morph frame is a different set of systems; recorded as a negative result |

## C. The first slice (N1)

**What.** The smallest piece that puts one real kernel chart through the system R-926 rules, and checks it against
today's decode, so that § 12's forward numbers are re-measured on a kernel chart. **Proposed, not dispatched:** the
chart tasks are paused (PR #469), and N1 waits for the owner to lift the pause or let it through as a measurement.

**Charts.** The latent chart first (`crates/kernel/src/chart/latent.rs`, the one chart that bakes its window in), then
the shape sphere's exponential map (`chart/shape_sphere.rs`, with its `r = 0` centre and its poles).

**The path, per sample.** `decode_only_with` (`crates/engine/src/decode_dispatch.rs:781`) evaluates the chart in f64 at
each sample's `(u, v)`; the per-quad forward table is the `Ic<f64>` of each sample of the quad (one node per sample,
RQ-766 (a)); each is narrowed once to `Ic<f32>` and written with `write_ic` (`stage/decode.rs:653`) as the record. No
new type: the slice uses today's charts, today's `Window` and today's record layout.

**Resolution.** One quad grid of 256 × 256 samples over each chart's home window, and the same at zoom 2¹⁰ and 2²⁰
about an O(1) point (each latent coordinate 0.7) and about the chart's origin; throughput on 1920 × 1080 samples.

**Backends.** The Mac only: CPU, and the GPU decode stage on Metal (`DecodeStage::run`). Timings run on the Mac
(R-186). No Windows runner job.

**What it integrates with.** There is no GPU integrator entry on `main` (no `#[spirv(compute)]` in
`crates/kernel/src/driver/`); the slice integrates with the CPU reference, `kernel::driver::wrapper::integrate`
(`wrapper.rs:84`), at f32 on the CPU, as validation's drift check does, for a fixed horizon `T` and the default occupant.

**Compared bit for bit** (predicates, PASS required):
1. The host-written records of a quad equal `write_ic` of the same narrowed `Ic<f32>`, word for word.
2. Two runs of the slice give the same records, bitwise.
3. Each narrowed component is the correctly rounded value of its f64 evaluation (within half an f32 ulp of it).
4. Equal records integrate to equal `SimState`s, bitwise.

**Compared by bound** (measured and reported, not asserted): the narrowed f64 records against today's CPU f32 decode
(`sample_with::<…, f32>`) and against the GPU decode stage on Metal, per component in f32 ulps at `max(1, |x|)`; the
branch selects (pole and upper/lower selects) that differ between the two; for differing records, the integrated
outcome class and `t_end_step` after `T`; and, at each zoom (home, 2¹⁰, 2²⁰, where 2²⁰ is R-90's `ℓ_switch = 20`), the
share of neighbouring sample pairs whose narrowed records are bitwise identical.

**The control arm** (measured beside the comparison, R-84, labels on chaotic trajectories may differ across precisions): the same narrowed records each moved by one f32 ulp in one
component, integrated for the same `T`; the share of outcome-class and `t_end_step` flips it gives is the chaos
baseline. A flip rate between today's f32 decode and the narrowed f64 records is read against this baseline, never
alone: a rate at or below it says the conversion moves outcomes no more than rounding already does.

**The criterion, written before any run:**
- **PASS:** predicates 1 to 4 hold; no branch select differs between today's f32 decode and the narrowed f64 records;
  every record where the two differ is listed with its size (that none of the narrowed records is farther from the f64
  value than the f32 decode follows from predicate 3 and is not a separate criterion); one thread reaches
  8.1·10⁶ samples a second (M-A's in-motion figure) on both charts.
- **EXPLAINED DIVERGENCE:** a predicate fails, or a branch select differs, with a named cause (for example the f32
  `atan2`'s integer path, `arctan.rs`, at a pole select, R-437); or the rate falls short with a named cost.
- **UNEXPLAINED:** anything else.
The outcome is reported whichever way; nothing is tuned to pass. The record difference between today's f32 decode
and the f64-then-narrow path is the size of the conversion R-926 makes, which TASK-M2-61's baseline must record.

**What it does not do.** No backwards table (N7), no slopes (N3), no blend (N5), no time coordinate (N6), no shader.

## D. Contradictions

Everything in the prototype's documents that contradicts the repository, the contracts or the accepted code reading
(`step1-code-claims.md`), plainly. "Design" is `orch/charts/prototype-2026-11-10/chart-system-prototype-design.md`.

1. **The record mixes inputs and outputs** (design:141, 20 floats: time reached, closest approach, flag, collision
   time). The integrator's input is `Ic` alone (`crates/kernel/src/stage/decode.rs:35`); those four are `SimState`
   outputs (`crates/kernel/src/driver/wrapper.rs:27`, `t_end_step`, `d_min`, `state`), a separate buffer marched in
   place (REQ-SCHED-012). The flag −1 is the `DEGENERATE` tag, not a slot.
2. **Velocities, not momenta.** The design's record holds velocities; the kernel's `Ic` holds 15 floats, masses,
   positions and momenta (`crates/kernel/src/decode/mod.rs:59`), REQ-DEC-005's one input type.
3. **An invented refusal at `10⁻¹⁰`** (design:44). The kernel never refuses at `n₀ = ±1`: R-437 sets `atan2 = 0` at the
   poles (`chart/shape_sphere.rs:262`); invalid decodes carry one of five tags (`decode/degenerate.rs`); feasibility is
   exact (`decode/invariant.rs:273`, `:280`). Body 2 at the inner pair's centre of mass is a valid system.
4. **A float target time against integer boundaries** (design:130-133, `t ≥ max(0, T + τ)`). R-565 stops the march only
   on its grid; REQ-INT-038 counts integer steps; § 18.2 keeps rows as integer indices (731 of 65,535 `k` fail float
   division); `bounded` belongs to the horizon (§ 18.5 C1). The `max(0, ·)` hides that there is no backward march.
5. **A toy integrator** (design:20, :339): adaptive leapfrog with a collision cut-off; the product's is fixed macro-steps
   with substeps under `N_max` (`wrapper.rs:192`). Every integration-dependent prototype number (the 1.7 ms, the morph
   hybrid's 97.6 to 100%) depends on it.
6. **`live` without the size rescale** (design:214). The kernel's canonicalisation step 0 sets `r ← r/√I`,
   `p ← I^{1/4}·p` (`crates/kernel/src/canon.rs:9`); the encode must be reused, and later states stay signed and unfolded
   (R-853).
7. **Tip as sheet "not in the prototype"** (design:78) is already built: `TiltMap::Sheet`, the default
   (`crates/kernel/src/chart/tilt.rs:88`), by TASK-M2-36, under R-474.
8. **The LM argmin jump** (design:114-118): "the pin becomes `φ'(P')`", from four seeds, may switch basin; R-849 asks a
   continuous slide, REQ-CHART-146 a defined nearest point (RQ-759).
9. **"Ends exact, measured"** (design:95). REQ-CHART-113 and § 17.4 select `s = 0` and `s = 1` by an exact comparison,
   because `a + (b − a) ≠ b` on 23% of pairs; the design relies on a measurement.
10. **"GPU has no chart code"** (design:154) is false today on the fragment side: the decode, encode and shape WGSL is
    generated (`crates/kernel/src/stage/fragment.rs`), and the presets use it (`crates/render/src/presets/mod.rs:30`).
    R-925 keeps it where a chart has closed-form code.
11. **"Centre-plus-offsets not built"** (design:160) is designed: the scheduler contract's per-quad centre and half-width
    and `x₀`, `J_D` (Part 5), deep zoom § 2, REQ-SCHED-024. It does not remove the floor: on the f32 path the IC
    `x₀ + J_D·δ` is an absolute f32 number of order `|x₀|`, so adjacent ICs closer than its ulp are one IC to the
    integrator; the floor is the measured value quoted with `|x₀|` (R-632, REQ-DEC-051), and "a floor near ~50" needs
    the sum and the integration in f64 (deep zoom § 2). The prototype's absolute records reach the same ulp at `|x₀|`;
    any earlier or ragged collapse in the prototype is its own evaluation error, which O8's measurement can show.
12. **Not every chart is "assemble ∘ map"** (design § 1). The invariant and construction charts build momenta from
    targets with a seed (`decode/invariant.rs:385`); masses come through several links. Each chart's own decode gives
    REQ-DEC-005's `(m, r, p)`; the blend encodes each end to the signed state, as § 17.2's line does.
13. **A uniform `Σ` on every coordinate** (design § 2) against R-507: the latent tilt is the decoder's basis edit with no
    scale; hidden coordinates are of mixed kinds (`tilt.rs:31`). RQ-762.
14. **Time as a coordinate in `Q`** (design § 2) against § 18.1 and REQ-CHART-174: settled for the coordinate by R-926;
    the integer grid is RQ-761.
15. **A state-level blend** (design § 3) against § 17.2's "blend at the last layer where the ends differ" and § 17.6's
    held-`E*` miss: settled by R-926 item 3; the other arms are RQ-760.
16. **Time blended linearly** (design § 3, `ta + s(tb − ta)`) against § 18.4's cut from a time-axis chart to an IC-only
    chart and "time to time only with the same window": settled by R-926 item 3, R-842 amended.
17. **"A morph" naming** collides with `engine::nav::directions::MorphInto` (`crates/engine/src/nav/directions.rs:163`),
    which is a window turn: a tilt by R-925 item 4.
18. **`numerical` run live in the fragment** (design § 7) against composition § 3's "`numerical` (CPU batch only, never
    in the kernel)": settled for the fragment by R-925; the integrator still never runs it.
19. **Tables as a new render binding** against SM2's "reads only what the fragment already has": settled by R-925 (a
    render-side binding, not a sim struct).
20. **The fold rule** (design § 7.6, a chosen copy) is neither R-407's extension types nor § 7.3's branch rule, "a pure
    function of the pixel" (RQ-760 (iv)).
21. **Slopes exact** (design § 8) against REQ-DEC-034's central differences: RQ-758.
22. **The patch's codomain.** The design's patch maps `u` to the 9-number signed state; `J_D` maps to the physical
    `(m, r, p)` (TASK-M6-07). Under R-926 item 5 the integrator decodes nothing, so the patch must be in `(m, r, p)`,
    REQ-DEC-005's: two-way, decided so here (applied per R-369: an implementation choice the rulings force).
23. **The patch's switch** (lut-exploration § 5, "within a sample from ×64 to ×256") is not R-90's trigger (bitwise-equal
    neighbours, or `ℓ_switch = 20`): RQ-766.
24. **The seed `w = r/√I`** (chart review § 2, "owner's decision") is in no ruling; R-82 stands: RQ-764.
25. **The window "flat in coordinate space" for curved charts:** today a curved chart's window is axis-aligned with no
    rotation (`sim_config.rs:84`); in-plane rotation exists only on latent views (`tilt.rs:480`). R-926 item 1 makes
    rotation an edit on every chart: new work in TASK-M2-61.
26. **The prototype's quadtree** (design § 9) refines by zoom only, not by the dynamics, against the scheduler
    contract's refinement criterion: not copied (R-926).
27. **Rotation re-integrates** (R-92), while the design's tables survive a rotation (design § 7.1): compatible (the
    tables are the chart's, the samples the view's), noted because the design's "nothing is recomputed" reads otherwise.

## E. The one-way decisions waiting on the owner

| RQ | In plain English |
|---|---|
| RQ-758 | Exact slopes: do the old "nudge each coordinate" slopes stay as the deep-zoom decoder's slopes, become a fourth kind, or only check the exact ones? |
| RQ-759 | The lock: when the map changes, must the pinned system move smoothly, measured how, and do the "offer, don't force a re-centre" and "stop at the last valid system" rules stay? |
| RQ-760 | Morphs: do the old axis-blend and bend paths survive as maps, are masses blended as their logarithms, may area statistics be taken on a blend, and which copy counts where a blend folds? |
| RQ-761 | Time: how a sample with its own time stops on the integrator's fixed step grid, and what playing backwards does |
| RQ-762 | Turning the window: what one unit of time (and of any coordinate with no slider) is, so an angle toward it means something |
| RQ-763 | Bitwise first: rebuild all 35 of today's view operations exactly before converting, or measure first and convert the ones that differ at once? |
| RQ-764 | The momentum seed: adopt the seamless expansion seed everywhere, on some charts, or keep today's rule? |
| RQ-765 | The GPU's own chart decoding: keep it for tests and pictures only, and retire the cap on how many are compiled? |
| RQ-766 | A big quad's starting systems: worked out exactly for every sample, or filled in between grid points on the GPU? |
| RQ-767 | The backwards table outside a morph: how accurate, and may it sharpen its answer with the chart's own forward code? |
| RQ-768 | `ctx.chart.z`: the forward value the corpus defines, or something new read backwards? |
| RQ-769 | May the registry and map tasks run in parallel with the path object and the lock, roughly halving the chain? |
| RQ-770 | Does sliding or zooming the picture still reuse the squares already worked out, or recompute everything? |

RQ-770 was filed in review; the five further queue numbers reserved for this re-plan are unused and released.
