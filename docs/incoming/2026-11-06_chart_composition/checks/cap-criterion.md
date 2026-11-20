# Cap measurement: the criterion, written before any measurement (6 Nov 2026)

Question: how many closed-form axis pairs may be baked into the GPU kernel? (order item 5; REQ-PERF-007's bound)

Quantity P = baked pairs added to the kernel of main 9188686f, each with its own compute entry point in the one module.

Measured, per P in {0, 1, 4, 8, 16}:
 M1 `cargo xtask build-kernel` wall time, an edit to crates/kernel forcing the kernel to recompile, backend built.
 M2 kernel.spv and kernel.wgsl bytes.
 M3 One pipeline creation through engine::compute::pipeline on this Mac's Metal GPU, median of 5: wgpu validates and
    naga parses the WHOLE module at each creation, so it grows with P even for an unrelated entry.
 M4 naga parse + validate alone on the CPU, median of 5.
 M5 What a pair needs of the uniform: floats used of DecodeUniforms.chart's 24.

Pass, at the proposed cap P*, all of:
 C1 build: T(P*) - T(0) <= 30 s and T(P*) / T(0) <= 1.25. build-kernel runs in every code CI and every kernel edit;
    a quarter more is the most a design option should cost without its own ruling.
 C2 module: wgsl(P*) / wgsl(0) <= 1.25, for the same reason, and it is parsed once per pipeline.
 C3 pipeline: the eager set (REQ-PERF-006, one pipeline per chart at startup: 12 decode entries today, plus P*)
    created one after another in <= 2 s on this Mac, and one creation <= 1.25 x its baseline. Chart switching must be
    instant (lowering Part 4) and the reel covers the start-up window; 2 s is my number, for veto.
 C4 limits: naga validates the module; a pair's parameters fit DecodeUniforms.chart's 24 floats.
 C5 the cross-product: stated separately. Each cost above is per (pair, link set); if M4-06 bakes decode inside the
    integrate entry the count multiplies by occupants x FTLE x link sets. The cap must pass C1 to C3 under the multiplier
    the plan actually implies, not only the decode-only one. If that cannot be measured here, say so and state what was.

The cap is the largest P passing C1 to C4 under C5's multiplier, not above what the known customers need. If the
customers need fewer than the measured limit, propose the customers' number and say the measurement allows more.
If the owner's "about 4" fails, say so; don't tune the criterion to pass it.

Stand-in for a pair: a closed-form pair at least as heavy as region D's (mass-weighted Jacobi from (xi, eta), sqrt, atan,
atan2, and the near-rest momentum), each entry a distinct monomorphisation. One arm with the cheapest real pair
(Mixed<H, V>) as the lower bound, at the largest P.

Honesty: a run is one Mac under the queue's other load; each arm's time is a median of 3; the load average is recorded.
Metal pipeline cost is this Mac's only: nothing here says what Vulkan, DX12 or WebGPU pay.
