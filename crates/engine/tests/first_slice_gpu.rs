//! The first slice's GPU arm: the narrowed f64 records of `engine::first_slice` against the
//! decode stage on this machine's GPU (`DecodeStage::run`, `target/spirv/kernel.wgsl`, which `cargo xtask build-kernel`
//! writes), on each chart's grids, per component in f32 ulps at `max(1, |x|)`; the samples whose `DEGENERATE` tag
//! differs; and the share of neighbouring sample pairs whose GPU records are bitwise identical. Measured and reported,
//! never asserted: the arm that counts is the Mac's Metal, and the report names the adapter it ran on. Run with
//! `--no-capture` to read it. A sample outside the chart's domain (the chart's `validate` does not find its point
//! `Valid`) is counted and left out, as `first_slice` leaves it out.
//!
//! What it asserts is that the comparison is like for like: the GPU decodes the very chart the CPU's f64
//! evaluation reads widened, its parameters through the decode stage's uniform unchanged, at the same `(u, v)` rounded
//! once to f32 as today's CPU f32 decode takes them.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed, on this machine's GPU backend.

#[path = "first_slice/measure.rs"]
mod measure;

use std::path::{Path, PathBuf};

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::DecodeStage;
use engine::first_slice::{forward_table, records, Quad, SliceChart};
use kernel::chart::{InvariantLE, Latent, ShapeSphere};
use kernel::stage::decode::{ic_words, tag, write_ic, DecodeMode};
use measure::{compare, identical_pairs, in_domain};
use validation::first_slice::{Preset, GRIDS, SIDE};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The WGSL `cargo xtask build-kernel` wrote.
fn wgsl() -> String {
    std::fs::read_to_string(root().join("target/spirv/kernel.wgsl"))
        .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

/// `chart` as the decode stage's uniform carries it to the GPU equals the chart the CPU evaluates: the parameters
/// the GPU reads are the CPU's, unchanged.
fn check_uniform<C: SliceChart + PartialEq>(chart: &C, sent: fn(&C) -> C) {
    let read = C::from_uniform(&sent(chart).uniform());
    assert!(
        read == *chart,
        "the GPU's uniform does not carry the chart the CPU evaluates"
    );
}

/// Chart `C`'s grids through the decode stage on `h`'s GPU against the narrowed f64 records: the report.
fn measure_chart<C: Preset + PartialEq>(h: &GpuHarness, wgsl: &str) -> String {
    let stage = DecodeStage::<C::Variant>::new(h.device(), wgsl, FastMath::default())
        .expect("the decode stage's pipeline");
    let mut r = format!(
        "first slice, {}, the decode stage on {:?} ({}): the narrowed f64 records against the GPU's\n",
        C::NAME,
        h.adapter_info().backend,
        h.adapter_info().name
    );
    for grid in GRIDS {
        let q: Quad<C> = C::quad(grid, [SIDE, SIDE]);
        check_uniform(&q.chart, |c| *c);
        let table = forward_table(&q).expect("the physics kernel decodes");
        let narrowed = records(&table);
        let uv: Vec<[f32; 2]> = (0..q.len()).map(|i| q.uv(i).map(|x| x as f32)).collect();
        let gpu = stage
            .run(h.device(), h.queue(), &q.chart, DecodeMode::Full, &uv)
            .expect("the decode dispatch");
        let mut words = vec![0u32; q.len() * ic_words()];
        let mut tags = 0;
        let key = q.sim_key();
        let inside: Vec<bool> = (0..q.len()).map(|i| in_domain(&q.chart, q.uv(i))).collect();
        for (i, d) in gpu.iter().enumerate() {
            write_ic(&mut words, i, &d.ic);
            if !inside[i] {
                continue;
            }
            let wide = engine::decode_dispatch::decode_only_with::<
                C,
                kernel::generated::links::Defaults,
            >(&q.chart, q.uv(i), &key, None)
            .expect("the physics kernel decodes");
            tags += usize::from(tag(wide.degenerate) != tag(d.degenerate));
        }
        let diff = compare(&narrowed, &words, |i| inside[i]);
        let (same, pairs) = identical_pairs(&words, q.size, |i| inside[i]);
        r.push_str(&format!("  {}:\n", grid.name()));
        let outside = inside.iter().filter(|&&a| !a).count();
        if outside > 0 {
            r.push_str(&format!(
                "    outside the chart's domain (its validate does not find the point Valid), left out: {outside} of {} \
                 samples\n",
                q.len()
            ));
        }
        r.push_str(&diff.to_string());
        r.push_str(&format!(
            "    DEGENERATE tags differing: {tags}; neighbouring pairs bitwise identical on the GPU: {same} of {pairs} \
             ({:.4})\n",
            same as f64 / pairs.max(1) as f64
        ));
    }
    r
}

#[test]
fn first_slice_gpu_decode() {
    let h = GpuHarness::new().expect("a GPU device");
    let wgsl = wgsl();
    println!("{}", measure_chart::<Latent<f32>>(&h, &wgsl));
    println!("{}", measure_chart::<ShapeSphere<f32>>(&h, &wgsl));
    println!("{}", measure_chart::<InvariantLE<f32>>(&h, &wgsl));
}

negative_control!(
    first_slice_gpu_decode,
    "a uniform carrying another chart must fail",
    expected = "does not carry the chart the CPU evaluates",
    check_uniform(&<Latent<f32> as Preset>::preset(), |c| Latent {
        z0: c.z0.map(f32::next_up),
        ..*c
    })
);
