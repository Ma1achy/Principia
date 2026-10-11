//! REQ-VAL-011: `t_dmin_step`, the 16-bit step index of `times`' high half (payload §2, §6), round-trips exactly for
//! every u16 value: packed by the kernel's generated Rust, then unpacked by the same Rust and, on the GPU, by the
//! generated fragment unpack layer, `crates/render/frag/generated/payload_unpack.wgsl`, the layer the `t_dmin_step`
//! view reads through. `t_end_step`, the low half, comes back unchanged beside it.

use std::path::Path;

use validation::gpu::GpuHarness;
use validation::{negative_control, pack_times, set_t_dmin_step, tm_t_dmin_step, tm_t_end_step};

/// The `t_end_step` packed beside `v`: a value varying with `v`, so that every bit of the low half is set somewhere.
fn t_end(v: u32) -> u32 {
    (v ^ 0xa5a5).rotate_left(3) & 0xffff
}

/// The checked-in fragment unpack layer with a compute entry that writes `tm_t_dmin_step` of each input word; the
/// layer's own group-1 buffers go unused, so unbound.
fn module() -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../render/frag/generated/payload_unpack.wgsl");
    let unpack =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    format!(
        "{unpack}
@group(0) @binding(0) var<storage, read> t_in: array<u32>;
@group(0) @binding(1) var<storage, read_write> t_out: array<u32>;

@compute @workgroup_size(64)
fn t_dmin(@builtin(global_invocation_id) id: vec3<u32>) {{
    if (id.x < arrayLength(&t_in)) {{
        t_out[id.x] = tm_t_dmin_step(t_in[id.x]);
    }}
}}
"
    )
}

/// Checks that every u16 `v`, packed by `pack` beside [`t_end`], unpacks to `v` in Rust and on the GPU, its
/// `t_end_step` unchanged, and that `set_t_dmin_step` over a word already holding the opposite bits writes it exactly.
fn check_roundtrip(pack: fn(u32, u32) -> u32) {
    let words: Vec<u32> = (0..=0xffff).map(|v| pack(t_end(v), v)).collect();
    for (v, &w) in (0u32..).zip(&words) {
        assert_eq!(
            tm_t_dmin_step(w),
            v,
            "t_dmin_step {v}: Rust unpacks {}",
            tm_t_dmin_step(w)
        );
        assert_eq!(
            tm_t_end_step(w),
            t_end(v),
            "t_dmin_step {v}: t_end_step changed"
        );
        let over = set_t_dmin_step(pack(t_end(v), !v & 0xffff), v);
        assert_eq!(
            over, w,
            "t_dmin_step {v}: set over another value is not the packed word"
        );
    }
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let got = gpu.run_wgsl(&module(), "t_dmin", &[&words]);
    for (v, &g) in (0u32..).zip(&got) {
        assert_eq!(g, v, "t_dmin_step {v}: WGSL unpacks {g}");
    }
}

#[test]
fn t_dmin_roundtrip_every_u16() {
    check_roundtrip(pack_times);
}

negative_control!(
    t_dmin_roundtrip_every_u16,
    "a packing that drops the step index's top bit",
    expected = "t_dmin_step 32768: Rust unpacks 0",
    check_roundtrip(|t_end, v| pack_times(t_end, v & 0x7fff))
);
