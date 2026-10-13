// Seam 5 (REQ-COL-005): naming a payload buffer reads the payload past its accessors.
fn word(i: u32) -> vec4<u32> {
    return word_buffer[i]; // fires payload-buffer
}

fn colour(ctx: Ctx) -> vec3<f32> {
    let m = ic_buffer[0u].m0; // fires payload-buffer
    return vec3<f32>(m, f32(fgw_length_raw(word(0u))), 0.0);
}
