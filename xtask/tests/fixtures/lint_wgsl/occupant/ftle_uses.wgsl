// REQ-RENDER-015: every kind of use of an unguarded `ftle` read: a store, a branch, a magnitude test on its bits, a
// switch, a loop's exit and a return; and the expressions a read passes through.
fn colour(ctx: Ctx) -> vec3<f32> {
    var x = 0.0;
    x = ctx.sample.ftle; // fires guard
    if (ctx.sample.ftle > 1.0) { // fires guard
        x = 2.0;
    }
    if (bitcast<u32>(ctx.sample.ftle) > 0x7f800000u) { // fires guard
        x = 3.0;
    }
    switch (u32(ctx.sample.ftle)) { // fires guard
        default: {
            x = 4.0;
        }
    }
    loop {
        continuing {
            break if ctx.sample.ftle < x; // fires guard
        }
    }
    return vec3<f32>(x, ctx.sample.ftle, 0.0); // fires guard
}

// A read spread over a vector.
fn spread(ctx: Ctx) -> vec3<f32> {
    return vec3<f32>(ctx.sample.ftle); // fires guard
}

// A read swizzled, loaded through a local's member, selected, and tested by a relational builtin.
fn swizzled(ctx: Ctx) -> vec3<f32> {
    return vec3<f32>(vec2<f32>(ctx.sample.ftle, 1.0).yx, 0.0); // fires guard
}

fn through_local(ctx: Ctx) -> f32 {
    var c = ctx;
    return c.sample.ftle; // fires guard
}

fn selected(ctx: Ctx) -> f32 {
    return select(0.0, ctx.sample.ftle, ctx.sample.ftle_valid); // fires guard
}

fn any_big(ctx: Ctx) -> bool {
    return any(vec2<bool>(ctx.sample.ftle > 1.0, false)); // fires guard
}

// A use inside a switch's case.
fn in_case(ctx: Ctx, k: u32) -> f32 {
    switch (k) {
        default: {
            return ctx.sample.ftle; // fires guard
        }
    }
}

// A read compared by `==` or `!=` that is not the absence test, on either side.
fn compared(ctx: Ctx) -> bool {
    return ctx.sample.ftle == 1.0; // fires guard
}

fn compared_right(ctx: Ctx) -> bool {
    return 1.0 != ctx.sample.ftle; // fires guard
}
