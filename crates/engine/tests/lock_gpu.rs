//! The lock on the GPU's decode stage (chart_decoder_contract Part 4, "What the GPU knows about all of this: nothing";
//! TASK-M2-23): the kernel's f32 build, `target/spirv/kernel.wgsl`, dispatched natively. CI runs this file in the
//! `gpu-kernel` job, after `cargo xtask build-kernel`.
//!
//! - `lock_kernel_uniform_identical_gpu` (REQ-SYS-012): a locked and a free view of the same `(z₀, q₁, q₂)`, through
//!   `engine::lock::kernel_chart`, render bitwise identically through the decode stage; with an excursion the same
//!   holds of the plane the excursion gives.

use std::path::Path;

use engine::contract::fast_math::FastMath;
use engine::contract::set_field::{Edit, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice,
};
use engine::decode_dispatch::{DecodeStage, Words};
use engine::lock::{kernel_chart, set};
use kernel::stage::decode::{flags, DecodeMode, LatentDefaults};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The WGSL `cargo xtask build-kernel` wrote.
fn wgsl() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/spirv/kernel.wgsl"),
    )
    .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

fn sim(plane: Plane, lock: Lock) -> SimConfig {
    SimConfig {
        chart: Chart {},
        plane,
        slice: Slice {},
        lock,
        links: Links {},
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 100.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

const FREE: Lock = Lock {
    locked: false,
    z_locked: [0.0; 8],
};

/// A tilted, zoomed plane through an off-origin centre.
fn plane() -> Plane {
    Plane {
        z0: [0.4, -0.2, 0.1, -0.3, 0.2, 0.05, 0.6, -0.5],
        q1: [0.3, -0.1, 0.05, 0.0, 0.2, 0.0, -0.1, 0.0],
        q2: [0.0, 0.25, 0.0, -0.15, 0.0, 0.1, 0.0, 0.2],
    }
}

/// A 9 × 9 grid over `[0,1]²` and past it.
fn grid() -> Vec<[f32; 2]> {
    let at = |i: usize| -0.25 + 1.5 * i as f32 / 8.0;
    (0..9)
        .flat_map(|j| (0..9).map(move |i| [at(i), at(j)]))
        .collect()
}

/// The decode stage's readback of `sim`'s kernel chart over the grid.
fn render(h: &GpuHarness, stage: &DecodeStage<LatentDefaults>, sim: &SimConfig) -> Words {
    stage
        .dispatch(
            h.device(),
            h.queue(),
            &kernel_chart(sim),
            flags(DecodeMode::Full),
            &grid(),
        )
        .expect("the decode dispatch")
}

/// The locked view `locked` and the free view `free` render bitwise identically.
fn check_identical(free: &SimConfig, locked: &SimConfig) {
    let h = GpuHarness::new().expect("a GPU device");
    let stage = DecodeStage::<LatentDefaults>::new(h.device(), &wgsl(), FastMath::default())
        .expect("the decode stage's pipeline");
    let (a, b) = (render(&h, &stage, free), render(&h, &stage, locked));
    assert!(
        a.ic == b.ic,
        "the locked and free renders' (m, r, p) are not bitwise identical"
    );
    assert!(
        a.tags == b.tags,
        "the locked and free renders' tags are not bitwise identical"
    );
    assert!(
        a.descriptors == b.descriptors,
        "the locked and free renders' descriptors are not bitwise identical"
    );
}

/// The plane after the lock at `(s, t)` of `plane()`, and its lock group.
fn locked_at(s: f64, t: f64) -> (Plane, Lock) {
    let [z0, lock] = set(&plane(), s, t).expect("a lock");
    match (z0.edit, lock.edit) {
        (Edit::Sim(SimField::Z0(z0)), Edit::Sim(SimField::Lock(lock))) => {
            (Plane { z0, ..plane() }, lock)
        }
        other => panic!("the lock's edits are {other:?}"),
    }
}

/// `z` moved along `ê_k` by `by`.
fn moved(z: &Latent, k: usize, by: f64) -> Latent {
    let mut z = *z;
    z[k] += by;
    z
}

#[test]
fn lock_kernel_uniform_identical_gpu() {
    let (p, lock) = locked_at(0.7, 0.2);
    check_identical(&sim(p.clone(), FREE), &sim(p.clone(), lock.clone()));
    // An excursion: the same plane, `z₀` off the anchor, renders as the free view of that plane.
    let away = Plane {
        z0: moved(&p.z0, 6, 0.3),
        ..p
    };
    check_identical(&sim(away.clone(), FREE), &sim(away, lock));
}

negative_control!(
    lock_kernel_uniform_identical_gpu,
    "a free view of another plane renders differently",
    expected = "not bitwise identical",
    {
        let (p, lock) = locked_at(0.7, 0.2);
        let other = Plane {
            z0: moved(&p.z0, 6, 0.3),
            ..p.clone()
        };
        check_identical(&sim(other, FREE), &sim(p, lock));
    }
);
