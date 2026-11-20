// Seam 6 (REQ-COL-005): the quad lane's RenderQuad members are the scheduler's.
fn colour(ctx: Ctx) -> vec3<f32> {
    let depth = ctx.quad.depth; // fires scheduler-state
    let state = ctx.quad.state; // fires scheduler-state
    let impurity = ctx.quad.impurity; // fires scheduler-state
    let spread = ctx.quad.spread; // fires scheduler-state
    let suspect = ctx.quad.suspect_frac; // fires scheduler-state
    let priority = ctx.quad.priority; // fires scheduler-state
    let age = ctx.quad.cache_age; // fires scheduler-state
    let coherence = ctx.quad.coherence; // fires scheduler-state
    let gap = ctx.quad.ancestor_gap; // fires scheduler-state
    let dominant = ctx.quad.dominant_outcome; // fires scheduler-state
    let n = f32(depth + state + age + gap + dominant);
    return vec3<f32>(n, impurity + spread + suspect, priority + coherence);
}
