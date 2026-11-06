use kernel::chart::{Latent, ShapeMap, ShapeSphere};
use kernel::generated::links::Defaults;
use kernel::stage::decode::sample;
use std::hint::black_box;
use std::time::Instant;

const N: usize = 200_000;

fn run<F: FnMut(f64, f64) -> f64>(mut f: F) -> f64 {
    let mut t = vec![];
    for _ in 0..5 {
        let a = Instant::now();
        let mut x = 0.0;
        for i in 0..N {
            let u = (i as f64 + 0.5) / N as f64;
            let v = ((i * 7919) % N) as f64 / N as f64;
            x += f(black_box(u), black_box(v));
        }
        black_box(x);
        t.push(a.elapsed().as_nanos() as f64 / N as f64);
    }
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    t[2]
}

#[test]
fn morph_cost() {
    let s = ShapeSphere::<f64> {
        map: ShapeMap::Spherical,
        n0: [1.0, 0.0, 0.0],
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.3, -0.4],
        z_mom: [0.2, -0.1, 0.15, 0.05],
    };
    let l = Latent::<f64> {
        z0: [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4],
        q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
    let a = run(|u, v| sample::<_, Defaults, f64>(&s, u, v).ic.m[0]);
    let b = run(|u, v| sample::<_, Defaults, f64>(&l, u, v).ic.m[0]);
    println!("RESULT ns_per_sample shape_sphere {a:.1} latent {b:.1}");
}
