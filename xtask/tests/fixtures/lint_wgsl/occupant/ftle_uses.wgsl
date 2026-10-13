// REQ-RENDER-015: every kind of use of an unguarded `ftle` read: a store, a branch, a magnitude test on its bits, a
// switch, a loop's exit and a return.
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
