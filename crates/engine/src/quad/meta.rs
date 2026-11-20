//! `QuadMeta`, the CPU-side metadata of one quad (dd_generation_root §3.7, "Not reduction fields"; REQ-REF-008):
//! `priority`, `cacheAge`, `lifecycle`, `computeCostMs` and `gentime` are computed on the CPU, never GPU reductions,
//! so they live here, one per quad beside its `QuadReduction`, and never in the GPU struct. A fragment shader cannot
//! read them from the reduction; they reach the display through `RenderQuad` and the tile debug shaders (§3.7a; render
//! GUI spec §12.1). The scheduler fills them (TASK-M5-17 to TASK-M5-19); this is their shape.

/// One quad's CPU-side metadata (dd_generation_root §3.7, "Not reduction fields").
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuadMeta {
    /// The scheduler's priority, which `RenderQuad`'s `priority_score` shows (§3.7a).
    pub priority: f32,
    /// The quad's age in the cache, in frames since its payload was last used, as `RenderQuad`'s `cache_age` (§3.7a).
    pub cache_age: u32,
    /// The quad's lifecycle state, as `RenderQuad`'s `quad_state` code: 0 loaded, 1 pending, 2 refinable, 3 terminal,
    /// 4 stale (§3.7a).
    pub lifecycle: u32,
    /// What computing the quad's payload cost, in milliseconds.
    pub compute_cost_ms: f32,
    /// The frame the quad's payload was generated in.
    pub gentime: u64,
}
