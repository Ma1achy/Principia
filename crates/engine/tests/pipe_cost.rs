//! Throwaway (measure/chart-pairs): pipeline cost and one coarse cover's decode, on the backend PRIN_GPU_BACKEND picks.
use engine::compute::{pipeline, ComputeShader};
use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::DecodeStage;
use kernel::chart::{Latent, ShapeMap, ShapeSphere};
use kernel::stage::decode::{DecodeMode, LatentDefaults, ShapeSphereDefaults};
use std::time::Instant;
fn med(mut v: Vec<f64>) -> f64 { v.sort_by(|a, b| a.partial_cmp(b).unwrap()); v[v.len() / 2] }
fn wgsl() -> String { std::fs::read_to_string("../../target/spirv/kernel.wgsl").unwrap() }
fn set(h: &validation::gpu::GpuHarness, wgsl: &str, names: &[String], tag: &str) {
    for pass in 0..3 {
        let t = Instant::now();
        for e in names { pipeline(h.device(), &ComputeShader { label: "m", wgsl, entry: e }, FastMath::Off).unwrap(); }
        println!("RESULT {tag} {} pass {} ms {:.1}", names.len(), pass, t.elapsed().as_secs_f64() * 1e3);
    }
}
#[test]
fn cost() {
    let wgsl = wgsl();
    let h = validation::gpu::GpuHarness::new().unwrap();
    println!("RESULT adapter {:?} {:?}", h.adapter_info().backend, h.adapter_info().name);
    let parse: Vec<f64> = (0..5).map(|_| { let t = Instant::now();
        let m = naga::front::wgsl::parse_str(&wgsl).unwrap();
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all()).validate(&m).unwrap();
        t.elapsed().as_secs_f64() * 1e3 }).collect();
    println!("RESULT naga_ms {:.1}", med(parse));
    let m = naga::front::wgsl::parse_str(&wgsl).unwrap();
    let names: Vec<String> = m.entry_points.iter().map(|e| e.name.clone()).collect();
    let gallery: Vec<String> = names.iter().filter(|e| e.starts_with("stage_decode_") && !e.contains("describe") && !e.contains("link_") && !e.contains("pair_")).cloned().collect();
    let pairs: Vec<String> = names.iter().filter(|e| e.contains("pair_")).cloned().collect();
    println!("RESULT entries {} gallery {} pairs {}", names.len(), gallery.len(), pairs.len());
    if let Some(p) = pairs.first() { let t = Instant::now();
        pipeline(h.device(), &ComputeShader { label: "m", wgsl: &wgsl, entry: p }, FastMath::Off).unwrap();
        println!("RESULT first_pair_ms {:.1}", t.elapsed().as_secs_f64() * 1e3); }
    set(&h, &wgsl, &gallery, "gallery");
    let mut eager = gallery.clone(); eager.extend(pairs.iter().cloned());
    set(&h, &wgsl, &eager, "eager");
}
#[test]
fn cover() {
    let wgsl = wgsl();
    let h = validation::gpu::GpuHarness::new().unwrap();
    let (w, hh) = (240usize, 135usize);
    let uv: Vec<[f32; 2]> = (0..w * hh).map(|i| [((i % w) as f32 + 0.5) / w as f32, ((i / w) as f32 + 0.5) / hh as f32]).collect();
    let l = Latent::<f32> { z0: [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4], q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] };
    let s = ShapeSphere::<f32> { map: ShapeMap::Spherical, n0: [1.0, 0.0, 0.0], e1: [0.0, 1.0, 0.0], e2: [0.0, 0.0, 1.0], scale: 1.0, inertia: 1.0, phase: 0.0, z_mass: [0.3, -0.4], z_mom: [0.2, -0.1, 0.15, 0.05] };
    let a = DecodeStage::<LatentDefaults>::new(h.device(), &wgsl, FastMath::Off).unwrap();
    let b = DecodeStage::<ShapeSphereDefaults>::new(h.device(), &wgsl, FastMath::Off).unwrap();
    let la: Vec<f64> = (0..10).map(|_| { let t = Instant::now(); a.run(h.device(), h.queue(), &l, DecodeMode::Full, &uv).unwrap(); t.elapsed().as_secs_f64() * 1e3 }).skip(1).collect();
    let lb: Vec<f64> = (0..10).map(|_| { let t = Instant::now(); b.run(h.device(), h.queue(), &s, DecodeMode::Full, &uv).unwrap(); t.elapsed().as_secs_f64() * 1e3 }).skip(1).collect();
    println!("RESULT cover samples {} latent_p50_ms {:.2} sphere_p50_ms {:.2}", uv.len(), med(la), med(lb));
    let one = &uv[..1];
    let l1: Vec<f64> = (0..10).map(|_| { let t = Instant::now(); a.run(h.device(), h.queue(), &l, DecodeMode::Full, one).unwrap(); t.elapsed().as_secs_f64() * 1e3 }).skip(1).collect();
    println!("RESULT cover_fixed_overhead_one_sample_p50_ms {:.2}", med(l1));
}
