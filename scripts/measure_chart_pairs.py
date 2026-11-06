#!/usr/bin/env python3
"""Throwaway (measure/chart-pairs). Criterion: the design's section 11. Run from the repo root, any platform:
    python scripts/measure_chart_pairs.py 0 1 4 8 0 1 4 8 0 1 4 8    # interleaved arms, so load drift falls on all of them
Set PRIN_GPU_BACKEND=vulkan on Windows; on macOS it defaults to metal.
Prints RESULT lines and writes them to chart-pairs-results.txt. Needs `cargo` and a GPU; rewrites kernel sources: never commit its output."""
import os, platform, subprocess, sys, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
INC = os.path.join(ROOT, "crates", "kernel", "src", "stage", "pair_measure.inc")
OUT = open(os.path.join(ROOT, "chart-pairs-results.txt"), "a", encoding="utf-8")

def say(*a):
    line = "RESULT " + " ".join(str(x) for x in a)
    print(line, flush=True)
    OUT.write(line + "\n")
    OUT.flush()

BODY = r'''
use crate::canon;
use crate::decode::config::{self, Config};
use crate::decode::{mass, momentum};
use crate::decode::momentum::Momentum;
use crate::generated::links::LinkParams;
use spirv_std::num_traits::Float;

/// Region D's pair at any masses: `lambda = (xi - d, eta)`, `d` the inner pair's centre of mass; explicit configuration,
/// no link, eq. 8's mask, eq. 3's momentum `q = sqrt(l) q_bar`.
#[inline]
fn pair_ic<const K: u32>(uniforms: &DecodeUniforms, u: f32, v: f32) -> (DecodeOut<f32>, bool) {
    let r = &uniforms.chart;
    let xi = r[0].x + (r[0].y - r[0].x) * u + r[1].z * (K as f32);
    let eta = r[0].z + (r[0].w - r[0].z) * v;
    let masses = mass::decode_at::<Defaults, f32>([r[1].x, r[1].y], &LinkParams::DEFAULT);
    let m = masses.m;
    let m01 = m[0] + m[1];
    let mu_rho = m[0] * m[1] / m01;
    let mu_lambda = m[2] * m01;
    let d = (m[1] - m[0]) / (2.0 * m01);
    let (x, y) = (xi - d, eta);
    let l2 = mu_rho + mu_lambda * (x * x + y * y);
    let l = l2.sqrt();
    let rho_t = [mu_rho.sqrt() / l, 0.0];
    let lambda_t = [mu_lambda.sqrt() * x / l, mu_lambda.sqrt() * y / l];
    let (rho, lambda, rs) = config::reconstruct(&masses, rho_t, lambda_t);
    let norm = (lambda_t[0] * lambda_t[0] + lambda_t[1] * lambda_t[1]).sqrt();
    let alpha = norm.atan2(rho_t[0]);
    let beta = if norm == 0.0 { 0.0 } else { y.atan2(x) };
    let cfg = Config { alpha, beta, rho_t, lambda_t, rho, lambda, r: rs };
    let k = l.sqrt();
    let q = [k * r[2].x, k * r[2].y, k * r[2].z, k * r[2].w];
    let (p_rho, p_lambda) = ([q[0], q[1]], [q[2], q[3]]);
    let mom = Momentum { q, p_rho, p_lambda, p: momentum::to_particles(&masses, p_rho, p_lambda) };
    let _ = &mom;
    let ic = canon::canonicalise_signed(&Ic { m, r: cfg.r, p: mom.p }).ic;
    let inside = xi >= 0.0 && eta >= 0.0 && (xi + 0.5) * (xi + 0.5) + eta * eta <= 1.0;
    (DecodeOut { ic, descriptor: descriptor::describe_decoded(&ic, &cfg), degenerate: masses.degenerate }, inside)
}

#[inline]
fn pair_entry<const K: u32>(i: u32, uniforms: &DecodeUniforms, uv: &[f32], ic: &mut [u32], tags: &mut [u32], descriptors: &mut [u32]) {
    let i = i as usize;
    if i < uv.len() / 2 {
        let (out, inside) = pair_ic::<K>(uniforms, uv[2 * i], uv[2 * i + 1]);
        write_ic(ic, i, &out.ic);
        tags[i] = if inside { tag(out.degenerate) } else { 9 };
        write_descriptor(descriptors, i, &out.descriptor);
    }
}
'''
IF = """    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] uv: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] ic: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] tags: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] descriptors: &mut [u32],
"""

def gen(p):
    salt = int(time.time()) % 100000 * 40
    s = f"// generated {time.time()} P={p}\n"
    if p:
        s += BODY
    for k in range(p):
        s += f"#[spirv(compute(threads(64)))]\npub fn pair_{k}(\n{IF}) {{\n    pair_entry::<{k + 1 + salt}>(id.x, uniforms, uv, ic, tags, descriptors);\n}}\n"
    with open(INC, "w", encoding="utf-8") as f:
        f.write(s)

def run(*cmd, log):
    t = time.time()
    with open(log, "w", encoding="utf-8") as f:
        rc = subprocess.call(list(cmd), cwd=ROOT, stdout=f, stderr=subprocess.STDOUT)
    return rc, time.time() - t

def load():
    try:
        return " ".join(f"{x:.1f}" for x in os.getloadavg())
    except (OSError, AttributeError):
        return "n/a"

def arm(p, runs):
    say("arm P", p, "load", load(), platform.platform())
    for i in range(runs):
        gen(p)
        rc, dt = run("cargo", "xtask", "build-kernel", "--bless", log=f"chart-pairs-build-{p}-{i}.log")
        say("build_s", p, f"{dt:.1f}", "rc", rc)
        if rc:
            sys.exit(rc)
    for n in ("kernel.spv", "kernel.wgsl"):
        say("bytes", p, n, os.path.getsize(os.path.join(ROOT, "target", "spirv", n)))
    log = f"chart-pairs-test-{p}.log"
    rc, _ = run("cargo", "test", "--release", "-p", "engine", "--test", "pipe_cost", "--", "--nocapture", "--test-threads=1", log=log)
    say("test_rc", p, rc)
    for line in open(log, encoding="utf-8", errors="replace"):
        if line.startswith("RESULT") or "panicked" in line or line.startswith("error"):
            say(p, line.replace("RESULT ", "").strip())

if __name__ == "__main__":
    arms = [int(x) for x in sys.argv[1:]] or [0, 1, 4, 8] * 3
    for p in arms:
        arm(p, 1)
    gen(0)
    log = "chart-pairs-cpu.log"
    rc, _ = run("cargo", "test", "--release", "-p", "kernel", "--test", "morph_cost", "--", "--nocapture", log=log)
    say("cpu_rc", rc)
    for line in open(log, encoding="utf-8", errors="replace"):
        if line.startswith("RESULT"):
            say(line.replace("RESULT ", "").strip())
