//! Which platform call first moves a bit of the builders' inputs: a temporary measurement, printed and failed on purpose
//! so nextest shows it.

use super::*;
use kernel::decode::{config, momentum};
use kernel::generated::links::{tanh_sat, SigmoidAlpha};
use kernel::real::Real;

fn fnv(words: impl IntoIterator<Item = u32>) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for w in words {
        for b in w.to_le_bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

fn words(x: &Ic<f32>) -> [u32; 15] {
    let mut w = [0; 15];
    stage::write_ic(&mut w, 0, x);
    w
}

/// The low 16 bits folded from a hash.
fn short(words: &[u32]) -> u16 {
    let h = fnv(words.iter().copied());
    (h ^ (h >> 16) ^ (h >> 32) ^ (h >> 48)) as u16
}

fn digests(tag: &str, all: &[u16]) {
    for (n, row) in all.chunks(64).enumerate() {
        let text: String = row.iter().map(|d| format!("{d:04x}")).collect();
        println!("XB dig {tag} {} {text}", n * 64);
    }
}

type Call = (&'static str, f32, f32);

/// The decode's platform calls in order, and whether they give the kernel's own masses, angles and controls back.
fn calls(z: [f32; 8]) -> (Vec<Call>, bool) {
    let mut out = Vec::new();
    let mut tanh = |x: f32| {
        out.push(("tanh", x, x.tanh()));
        tanh_sat(x)
    };
    let (t0, t1) = (tanh(z[6]), tanh(z[7]));
    let mut exp = |x: f32| {
        out.push(("exp", x, x.exp()));
        x.exp()
    };
    let (e0, e1) = (exp(f32::MU_MAX * t0), exp(f32::MU_MAX * t1));
    let s = 1.0 + e0 + e1;
    let m = [1.0 / s, e0 / s, e1 / s];
    exp(-z[0]);
    let q = [2, 3, 4, 5].map(|k| f32::Q_MAX * (2.0 * (1.0 / (1.0 + exp(-z[k]))) - 1.0));
    let [alpha] = <SigmoidAlpha as Link<1, 1>>::forward([z[0]]);
    let (_, beta) = config::angles::<Defaults, f32>([z[0], z[1]]);
    let beta = beta.on_circle();
    out.push(("sin", alpha, alpha.sin()));
    out.push(("cos", beta, beta.cos()));
    out.push(("sin", beta, beta.sin()));
    out.push(("cos", alpha, alpha.cos()));
    let sa = alpha.sin();
    let d = decode::<Defaults, f32>(&kernel::decode::Latent::new(z));
    let same = d.ic.m.map(f32::to_bits) == m.map(f32::to_bits)
        && d.config.rho_t[0].to_bits() == alpha.cos().to_bits()
        && d.config.lambda_t[0].to_bits() == (sa * beta.cos()).to_bits()
        && d.config.lambda_t[1].to_bits() == (sa * beta.sin()).to_bits()
        && momentum::controls::<Defaults, f32>([z[2], z[3], z[4], z[5]]).map(f32::to_bits)
            == q.map(f32::to_bits);
    (out, same)
}

/// The f32 nearest the exact value, through the pinned f64 maths.
fn nearest(name: &str, x: f32) -> f32 {
    let x = f64::from(x);
    (match name {
        "tanh" => x.hyperbolic_tangent(),
        "exp" => x.exponential(),
        "sin" => x.sine(),
        "cos" => x.cosine(),
        _ => unreachable!(),
    }) as f32
}

fn f64_call(tag: &str, at: usize, name: &str, x: f64, got: f64) -> bool {
    let want = match name {
        "sin" => engine::exact::sin(x),
        "cos" => engine::exact::cos(x),
        _ => engine::exact::pow(x, -0.5),
    };
    let off = got.to_bits() != want.to_bits();
    if off {
        println!(
            "XB f64 {tag} {at} {name} {:016x} got {:016x} exact {:016x}",
            x.to_bits(),
            got.to_bits(),
            want.to_bits()
        );
    }
    off
}

fn latents(batches: &[crate::decode_f32_eps::Batch]) -> Vec<[f32; 8]> {
    use kernel::chart::Chart;
    batches
        .iter()
        .flat_map(|(c, uv)| {
            uv.iter().map(|p| {
                let z = c.map::<f32>(p[0], p[1]).z;
                z.z
            })
        })
        .collect()
}

/// One f32 function's values in a replay: by name.
type Maths<'a> = &'a dyn Fn(&str, f32) -> f32;

fn platform(name: &str, x: f32) -> f32 {
    match name {
        "tanh" => x.tanh(),
        "exp" => x.exp(),
        "sin" => x.sin(),
        "cos" => x.cos(),
        _ => unreachable!(),
    }
}

/// The latent decode at f32 with its maths calls `f`'s, and under the identity `β` link, unreduced, where `old_beta`.
fn replay(z: [f32; 8], f: Maths, old_beta: bool) -> Ic<f32> {
    let tanh = |x: f32| f("tanh", x.clamp(-20.0, 20.0)).clamp(-1.0, 1.0);
    let sigmoid = |x: f32| 1.0 / (1.0 + f("exp", -x));
    let (e0, e1) = (
        f("exp", f32::MU_MAX * tanh(z[6])),
        f("exp", f32::MU_MAX * tanh(z[7])),
    );
    let s = 1.0 + e0 + e1;
    let masses = kernel::decode::mass::from_masses([1.0 / s, e0 / s, e1 / s]);
    let [half_pi] = <kernel::generated::links::PeriodicBeta as Link<1, 1>>::forward([0.0f32]);
    let alpha = f32::ALPHA_MIN + (half_pi - 2.0 * f32::ALPHA_MIN) * sigmoid(z[0]);
    let beta = if old_beta {
        z[1]
    } else {
        (z[1] + half_pi).on_circle()
    };
    let sa = f("sin", alpha);
    let (_, _, r) = config::reconstruct(
        &masses,
        [f("cos", alpha), 0.0],
        [sa * f("cos", beta), sa * f("sin", beta)],
    );
    let q = [2, 3, 4, 5].map(|k| f32::Q_MAX * (2.0 * sigmoid(z[k]) - 1.0));
    Ic {
        m: masses.m,
        r,
        p: momentum::to_particles(&masses, [q[0], q[1]], [q[2], q[3]]),
    }
}

/// `decode_parity_m01_crossings`' lines with the link's maths `f`'s.
fn crossings_with(f: Maths) -> Vec<[f32; 2]> {
    let eps = kernel::decode::mass::m01_epsilon::<f32>();
    let tanh = |x: f32| f("tanh", x.clamp(-20.0, 20.0)).clamp(-1.0, 1.0);
    let sigmoid = |x: f32| 1.0 / (1.0 + f("exp", -x));
    let forward = |v: [f32; 2]| {
        let t4 = 0.5 * f32::EPS_MU * tanh(v[0]);
        let t7 = sigmoid(2.0 * v[0]) - t4;
        let t12 = 0.5 * f32::EPS_MU * tanh(v[1]);
        [
            sigmoid(-(2.0 * v[0])) + t4,
            t7 * (sigmoid(-(2.0 * v[1])) + t12),
        ]
    };
    let mut out = Vec::new();
    for k in 0..LINES {
        let z1 = (6.3 + 14.0 * f64::from(k) / f64::from(LINES)) as f32;
        let (mut lo, mut hi) = (0.0f32, 30.0f32);
        loop {
            let mid = (lo + hi) * 0.5;
            if mid == lo || mid == hi {
                break;
            }
            let m = forward([mid, z1]);
            if m[0] + m[1] < eps {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        out.push([lo * 0.5, z1 * 0.5]);
        out.push([hi * 0.5, z1 * 0.5]);
    }
    out
}

fn arm(tag: &str, z: &[Vec<[f32; 8]>; 2], f: Maths, held: &[Ic<f32>]) {
    let hash = |z: &[[f32; 8]]| fnv(z.iter().flat_map(|z| words(&replay(*z, f, false))));
    let x = crossings_with(f);
    println!(
        "XB arm {tag} noop {:016x} wide {:016x} crossings {:016x}",
        hash(&z[0]),
        hash(&z[1]),
        fnv(x.iter().flatten().map(|q| q.to_bits()))
    );
    let mut off = [0usize; 3];
    let old = z.iter().flatten().map(|z| replay(*z, f, true));
    for (x, y) in old.zip(held) {
        let (x, y) = (words(&x), words(y));
        for (k, r) in [0..3, 3..9, 9..15].into_iter().enumerate() {
            off[k] += r.filter(|n| x[*n] != y[*n]).count();
        }
    }
    println!("XB arm {tag} old beta link: m,r,p words off the stored fixture's {off:?}");
}

/// The whole input with the latent link's `β` the identity, unreduced, as `049e4ecc1`'s kernel had it.
fn before_the_offset(built: &Inputs, z: &[Vec<[f32; 8]>; 2]) -> Inputs {
    let mut out = built.clone();
    let mut at = 0;
    for p in &built.parts {
        if p.name == "canonicaliser_canonicalise_order" {
            let representatives: Vec<Ic<f64>> = [
                [0.4f64, -0.3, 0.5, -0.2, 0.3, 0.1, 0.6, -0.4],
                [-0.8, 1.1, -0.4, 0.7, -0.6, 0.2, -0.3, 0.9],
                [1.2, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ]
            .iter()
            .map(|z| {
                let d = decode::<Defaults, f64>(&kernel::decode::Latent::new(*z));
                let c = config::from_angles(d.config.alpha, z[1], &d.masses);
                Ic { r: c.r, ..d.ic }
            })
            .collect();
            let states = representatives
                .iter()
                .flat_map(|x| gauges().into_iter().map(move |g| narrow(&apply(&g, x))));
            out.canon.splice(at..at + p.count, states);
        }
        at += p.count;
    }
    let n = out.canon.len() - z[0].len() - z[1].len();
    for (k, z) in z.iter().flatten().enumerate() {
        out.canon[n + k] = replay(*z, &platform, true);
    }
    // The defaults' own `V₀`, every eighth point's, where finite.
    use kernel::chart::Chart;
    let mut own = latent_defaults().into_iter().flat_map(|(c, uv)| {
        uv.into_iter()
            .step_by(8)
            .filter_map(move |p| {
                let x = replay(c.map::<f32>(p[0], p[1]).z.z, &platform, true);
                let v = kernel::decode::descriptor::potential(&x);
                v.is_finite().then(|| v.to_bits())
            })
            .collect::<Vec<_>>()
    });
    let targets = TARGETS.map(f32::to_bits);
    for b in out.batches.iter_mut().filter(|b| b.label == "defaults") {
        if b.points == 1 && b.e_star.is_some_and(|e| !targets.contains(&e)) {
            b.e_star = own.next();
        }
    }
    out
}

#[test]
fn xbackend_probe() {
    let built = build_inputs();
    let held = Inputs::load(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))).unwrap();
    println!("XB total built {} held {}", built.hash(), held.hash());

    // The parts.
    let mut at = 0;
    for p in &built.parts {
        let (a, b) = (
            &built.canon[at..at + p.count],
            &held.canon[at..at + p.count],
        );
        let tag = p.name.replace(' ', "_");
        let mut off = [0usize; 3];
        let mut states = 0;
        for (x, y) in a.iter().zip(b) {
            let (x, y) = (words(x), words(y));
            let n = |r: std::ops::Range<usize>| r.filter(|k| x[*k] != y[*k]).count();
            off[0] += n(0..3);
            off[1] += n(3..9);
            off[2] += n(9..15);
            states += usize::from(x != y);
        }
        println!(
            "XB part {tag} n {} built {:016x} held {:016x} states_off {states} m,r,p words_off {off:?} m {:016x}/{:016x} r {:016x}/{:016x} p {:016x}/{:016x}",
            p.count,
            fnv(a.iter().flat_map(words)),
            fnv(b.iter().flat_map(words)),
            fnv(a.iter().flat_map(|x| words(x)[..3].to_vec())),
            fnv(b.iter().flat_map(|x| words(x)[..3].to_vec())),
            fnv(a.iter().flat_map(|x| words(x)[3..9].to_vec())),
            fnv(b.iter().flat_map(|x| words(x)[3..9].to_vec())),
            fnv(a.iter().flat_map(|x| words(x)[9..].to_vec())),
            fnv(b.iter().flat_map(|x| words(x)[9..].to_vec())),
        );
        let all: Vec<u16> = a.iter().map(|x| short(&words(x))).collect();
        digests(&tag, &all);
        for (n, x) in a.iter().take(6).enumerate() {
            let t: Vec<_> = words(x).iter().map(|w| format!("{w:08x}")).collect();
            println!("XB word {tag} {n} {}", t.join(" "));
        }
        at += p.count;
    }

    // The batches.
    let mut groups: Vec<String> = Vec::new();
    for b in &built.batches {
        let g = format!("{} {}", b.entry, b.label);
        if !groups.contains(&g) {
            groups.push(g);
        }
    }
    let points = with_points(&built);
    let held_points = with_points(&held);
    for g in &groups {
        let pick = |all: &[(usize, &Batch, Vec<[f32; 2]>)]| -> (Vec<u32>, Vec<u32>, Vec<u32>) {
            let mut out = (Vec::new(), Vec::new(), Vec::new());
            for (_, b, uv) in all {
                if format!("{} {}", b.entry, b.label) == *g {
                    out.0.extend(&b.uniform);
                    out.1.push(b.e_star.unwrap_or(0));
                    out.2.extend(uv.iter().flatten().map(|q| q.to_bits()));
                }
            }
            out
        };
        let (a, b) = (pick(&points), pick(&held_points));
        let off = |x: &[u32], y: &[u32]| x.iter().zip(y).filter(|(p, q)| p != q).count();
        println!(
            "XB batch {g:?} uniform {:016x} off {} e {:016x} off {}/{} points {:016x} off {}/{}",
            fnv(a.0.iter().copied()),
            off(&a.0, &b.0),
            fnv(a.1.iter().copied()),
            off(&a.1, &b.1),
            a.1.len(),
            fnv(a.2.iter().copied()),
            off(&a.2, &b.2),
            a.2.len()
        );
        if g.contains("crossings") {
            digests("points", &a.2.chunks(2).map(short).collect::<Vec<_>>());
            for (n, p) in a.2.chunks(4).take(4).enumerate() {
                println!("XB word crossing {n} {p:08x?}");
            }
        } else {
            for (n, row) in a.1.chunks(24).enumerate() {
                let t: Vec<_> = row.iter().map(|w| format!("{w:08x}")).collect();
                println!("XB e {g:?} {} {}", n * 24, t.join(" "));
            }
        }
    }

    // The builders' own f64 calls, against the exact value.
    let mut tally: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    let mut count = |tag: &str, n: usize, name: &str, x: f64, got: f64| {
        let t = tally.entry(format!("{tag} {name}")).or_default();
        t[0] += 1;
        t[1] += usize::from(f64_call(tag, n, name, x, got));
    };
    for (n, t) in [0.3f64, 1.9, -2.6].into_iter().enumerate() {
        let (s, c) = t.sin_cos();
        count("turned", n, "sin", t, s);
        count("turned", n, "cos", t, c);
    }
    let mut rng = SplitMix::new(41);
    for n in 0..SWEEP {
        for _ in 0..3 {
            rng.uniform(0.2, 1.2);
        }
        let t = f64::from(rng.uniform(0.0, 1.0)) * std::f64::consts::TAU;
        for _ in 0..5 {
            rng.uniform(0.0, 1.0);
        }
        for _ in 0..6 {
            rng.uniform(0.0, 1.0);
        }
        let (s, c) = t.sin_cos();
        count("seam", n, "sin", t, s);
        count("seam", n, "cos", t, c);
    }
    for (n, _) in RATIOS.into_iter().enumerate() {
        let mut rng = SplitMix::new(61 + n as u64);
        for k in 0..2000 {
            for _ in 0..3 {
                rng.uniform(0.0, 1.0);
            }
            let t = f64::from(rng.uniform(0.0, 1.0)) * std::f64::consts::TAU;
            for _ in 0..2 {
                rng.uniform(0.0, 1.0);
            }
            count("hierarchy", n * 2000 + k, "cos", t, t.cos());
            count("hierarchy", n * 2000 + k, "sin", t, t.sin());
        }
    }
    count("edge", 0, "cos", 0.7, 0.7f64.cos());
    count("edge", 0, "sin", 0.7, 0.7f64.sin());
    for (n, t) in [0.7f64, 2.9, -1.3].into_iter().enumerate() {
        let (s, c) = t.sin_cos();
        count("order", n, "sin", t, s);
        count("order", n, "cos", t, c);
    }
    for (n, s) in [0.5f64, 3.0, 40.0].into_iter().enumerate() {
        count("order", n, "pow", s, s.powf(-0.5));
    }
    println!("XB f64 tally [calls, off exact] {tally:?}");

    // The decode's f32 calls.
    let all_z: Vec<[f32; 8]> = latents(&noop_batches())
        .into_iter()
        .chain(latents(&wide_batches()))
        .collect();
    let mut sites = [[0usize; 2]; 14];
    let mut untraced = 0;
    for (n, z) in all_z.iter().enumerate() {
        let (c, same) = calls(*z);
        untraced += usize::from(!same);
        for (k, (name, x, y)) in c.iter().enumerate() {
            let off = x.is_finite() && y.to_bits() != nearest(name, *x).to_bits();
            sites[k][0] += 1;
            sites[k][1] += usize::from(off);
        }
        if n < 192 {
            let t: Vec<_> = c
                .iter()
                .map(|(f, x, y)| {
                    format!(
                        "{f}:{:08x}:{:08x}:{:08x}",
                        x.to_bits(),
                        y.to_bits(),
                        nearest(f, *x).to_bits()
                    )
                })
                .collect();
            println!("XB call {n} {}", t.join(" "));
        }
    }
    println!(
        "XB f32 sites tanh z6, tanh z7, exp, exp, exp -z0, exp -z2..5, sin a, cos b, sin b, cos a: [calls, off nearest] {sites:?} untraced {untraced}"
    );

    // The same decodes with the f32 maths exchanged, one function at a time.
    let z = [latents(&noop_batches()), latents(&wide_batches())];
    let stored = &held.canon[held.canon.len() - z[0].len() - z[1].len()..];
    arm("platform", &z, &platform, stored);
    arm("nearest", &z, &nearest, stored);
    for own in ["exp", "tanh", "sin", "cos"] {
        let f = |n: &str, x: f32| {
            if n == own {
                platform(n, x)
            } else {
                nearest(n, x)
            }
        };
        arm(&format!("nearest but {own}"), &z, &f, stored);
    }
    println!(
        "XB before the offset: {}",
        before_the_offset(&built, &z).hash()
    );

    // Each f32 function over one fixed grid.
    for (name, f, half) in [
        ("exp", f32::exp as fn(f32) -> f32, 5.0f32),
        ("tanh", f32::tanh, 5.0),
        ("sin", f32::sin, std::f32::consts::PI),
        ("cos", f32::cos, std::f32::consts::PI),
    ] {
        let n = 1u32 << 18;
        let grid = (0..n).map(|k| -half + 2.0 * half * (k as f32 / n as f32));
        let off = grid
            .clone()
            .filter(|x| f(*x).to_bits() != nearest(name, *x).to_bits())
            .count();
        let hash = fnv(grid.map(|x| f(x).to_bits()));
        println!("XB grid {name} n {n} hash {hash:016x} off nearest {off}");
    }
    panic!("XB the measurement's print");
}
