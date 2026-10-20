//! The round trip's shared support (`validation::roundtrip`; TASK-M2-16; inverse-encode contract Parts 3, 4 and 8):
//! the norm `‖·‖_phys` and `ε_phys`, the clamp oracle and its two bounds, the round trip, and the chart registry.

use kernel::decode::{self, Ic, Latent};
use kernel::generated::links::Defaults;
use validation::eps_phys::Mismatched;
use validation::negative_control;
use validation::roundtrip::{
    chart, eps_phys, keys, margins, phys, registry, trip, trip_with, widen, Precision, Sides,
    INSIDE, PAST,
};

/// A state with positions of extent `r` and momenta of extent `p` (body 0's), masses `(0.2, 0.3, 0.5)`.
fn state(r: f64, p: f64) -> Ic<f64> {
    Ic {
        m: [0.2, 0.3, 0.5],
        r: [[r, 0.0], [0.0, -0.5 * r], [0.1 * r, 0.1 * r]],
        p: [[0.0, p], [-0.4 * p, 0.0], [0.2 * p, -0.3 * p]],
    }
}

// ── ‖·‖_phys and ε_phys ──────────────────────────────────────────────────────────────────────────────────────────

/// `got` is `want` to rounding.
fn near(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() <= 1e-12 * want,
        "{what}: {got:e} against {want:e}"
    );
}

/// Part 4's norm: each block's largest body difference over the reference's extent floored at 1, the masses absolute,
/// the largest of the three; measured against the reference, not the other state; NaN through.
fn check_phys(norm: fn(&Ic<f64>, &Ic<f64>) -> f64) {
    for (r, p) in [(0.5, 0.25), (8.0, 4.0)] {
        let x = state(r, p);
        let (big_r, big_p) = (f64::max(1.0, r), f64::max(1.0, p));
        let mut y = x;
        y.r[1][1] += 3e-3;
        near(
            norm(&x, &y),
            3e-3 / big_r,
            &format!("a position difference at extent {r}"),
        );
        let mut y = x;
        y.p[2] = [y.p[2][0] + 3e-3, y.p[2][1] + 4e-3];
        near(
            norm(&x, &y),
            5e-3 / big_p,
            &format!("a momentum difference at extent {p}"),
        );
        let mut y = x;
        y.m[0] += 1e-3;
        near(norm(&x, &y), 1e-3, "a mass difference");
        let mut y = x;
        y.m[2] -= 1e-3;
        y.r[0][0] += 4e-3 * big_r;
        y.p[1][1] -= 2e-3 * big_p;
        near(norm(&x, &y), 4e-3, "the largest of the three");
        assert_eq!(norm(&x, &x), 0.0, "a state from itself");
    }
    let (x, y) = (state(8.0, 0.25), state(0.5, 0.25));
    assert_ne!(
        norm(&x, &y),
        norm(&y, &x),
        "measured against the reference's extent"
    );
    for k in 0..3 {
        let mut y = state(0.5, 0.5);
        match k {
            0 => y.m[1] = f64::NAN,
            1 => y.r[2][0] = f64::NAN,
            _ => y.p[0][1] = f64::NAN,
        }
        assert!(norm(&state(0.5, 0.5), &y).is_nan(), "a NaN in block {k}");
        assert!(
            norm(&y, &state(0.5, 0.5)).is_nan(),
            "a NaN in the reference's block {k}"
        );
    }
}

#[test]
fn roundtrip_phys_norm() {
    check_phys(phys);
}

negative_control!(
    roundtrip_phys_norm,
    "a norm with the positions absolute, not over the extent, must fail",
    expected = "a position difference at extent 8",
    check_phys(|x, y| {
        let mut worst = phys(x, y);
        for i in 0..3 {
            worst = worst.max((y.r[i][0] - x.r[i][0]).hypot(y.r[i][1] - x.r[i][1]));
        }
        worst
    })
);

/// `ε_phys = k·ε_R`, `k` the register's 64.
fn check_eps(f64_eps: f64, f32_eps: f64) {
    assert_eq!(f64_eps, 64.0 * f64::EPSILON, "ε_phys at f64");
    assert_eq!(f32_eps, 64.0 * f64::from(f32::EPSILON), "ε_phys at f32");
    assert_eq!(<f64 as Precision>::EPS, f64::EPSILON);
    assert_eq!(<f32 as Precision>::EPS, f64::from(f32::EPSILON));
    assert_eq!(<f32 as Precision>::of(0.1), 0.1f32);
    assert_eq!(<f64 as Precision>::of(0.1), 0.1f64);
}

#[test]
fn roundtrip_eps_phys() {
    check_eps(eps_phys::<f64>(), eps_phys::<f32>());
}

negative_control!(
    roundtrip_eps_phys,
    "ε_phys read at f32 where f64 is asked must fail",
    expected = "ε_phys at f64",
    check_eps(eps_phys::<f32>(), eps_phys::<f32>())
);

// ── The clamp oracle ─────────────────────────────────────────────────────────────────────────────────────────────

/// The margins of the decode at `z` (Part 3's formulas, the defaults): `α` and `β` by `s = σ(z)`, so the margin is
/// `min(s, 1 − s)/ε`; each `q` likewise; each mass by `(μ_max − |μₖ|)/(ε_μ·μ_max)`, `μₖ = 5·tanh(z_μk)`.
fn check_margins(margin: fn(&Ic<f64>) -> [f64; 8]) {
    let z = [1.5, -2.0, 3.0, -4.0, 0.5, 0.0, 1.0, -0.5];
    let x = decode::decode::<Defaults, f64>(&Latent::new(z)).ic;
    let got = margin(&x);
    let sigma = |t: f64| 1.0 / (1.0 + (-t).exp());
    for k in 0..6 {
        let s = sigma(z[k]);
        let want = s.min(1.0 - s) / 1e-6;
        assert!(
            (got[k] - want).abs() <= 1e-8 * want,
            "control {k}: margin {} against {want}",
            got[k]
        );
    }
    for k in 6..8 {
        let want = (5.0 - (5.0 * z[k].tanh()).abs()) / (1e-6 * 5.0);
        assert!(
            (got[k] - want).abs() <= 1e-8 * want,
            "mass control {k}: margin {} against {want}",
            got[k]
        );
    }
}

#[test]
fn roundtrip_margins() {
    check_margins(margins);
}

negative_control!(
    roundtrip_margins,
    "margins with the momenta read unweighted, p₁ for p_ρ, must fail",
    expected = "control 2",
    check_margins(|x| {
        let mut y = *x;
        y.p[1] = [x.p[1][0] + 0.3, x.p[1][1]];
        margins(&y)
    })
);

/// `Sides`: a margin at [`INSIDE`] is inside, at [`PAST`] past, between neither, NaN neither; interior only with all
/// eight inside.
fn check_sides(of: fn([f64; 8]) -> Sides) {
    assert_eq!((INSIDE, PAST), (2.0, 0.5), "the two bounds");
    let mut m = [10.0; 8];
    assert_eq!(
        of(m),
        Sides {
            inside: 0xff,
            past: 0
        },
        "all inside"
    );
    assert!(of(m).interior(), "all inside is interior");
    m[3] = INSIDE;
    assert_eq!(of(m).inside, 0xff, "a margin at INSIDE is inside");
    m[3] = INSIDE.next_down();
    assert_eq!(
        of(m).inside,
        0xff & !(1 << 3),
        "a margin below INSIDE is not"
    );
    assert_eq!(of(m).past, 0, "nor past");
    assert!(!of(m).interior(), "one control not inside is not interior");
    m[3] = PAST;
    assert_eq!(of(m).past, 1 << 3, "a margin at PAST is past");
    m[3] = PAST.next_up();
    assert_eq!(of(m).past, 0, "a margin above PAST is not");
    m[5] = f64::NAN;
    let s = of(m);
    assert_eq!(
        (s.inside & (1 << 5), s.past & (1 << 5)),
        (0, 0),
        "a NaN margin"
    );
    m = [-1.0; 8];
    assert_eq!(
        of(m),
        Sides {
            inside: 0,
            past: 0xff
        },
        "all past"
    );
}

#[test]
fn roundtrip_sides() {
    check_sides(Sides::of);
}

negative_control!(
    roundtrip_sides,
    "sides read with INSIDE strict must fail",
    expected = "a margin at INSIDE is inside",
    check_sides(|m| {
        let s = Sides::of(m);
        let strict = m
            .iter()
            .enumerate()
            .filter(|(_, &x)| x > INSIDE)
            .fold(0u32, |b, (k, _)| b | 1 << k);
        Sides {
            inside: strict,
            past: s.past,
        }
    })
);

// ── The round trip ───────────────────────────────────────────────────────────────────────────────────────────────

/// The round trip at the golden IC is exact to rounding and reports its residual as `‖x − D(E(x))‖_phys`; with every
/// link swapped on the encode side it is far off.
fn check_trip(swapped: bool) {
    let z = [0.4, -0.3, 0.2, 0.1, -0.2, 0.3, 0.5, -0.6];
    let x = decode::decode::<Defaults, f64>(&Latent::new(z)).ic;
    let t = if swapped {
        trip_with::<Mismatched, Defaults, f64>(&x)
    } else {
        trip::<Defaults, f64>(&x)
    };
    assert_eq!(
        t.residual,
        phys(&x, &t.back.ic),
        "the residual is not ‖x − D(E(x))‖_phys"
    );
    assert!(
        t.residual <= eps_phys::<f64>(),
        "the round trip is {:e} off",
        t.residual
    );
    let x32 = decode::decode::<Defaults, f32>(&Latent::new(z.map(|v| v as f32))).ic;
    let t32 = trip::<Defaults, f32>(&x32);
    assert_eq!(t32.residual, phys(&widen(&x32), &widen(&t32.back.ic)));
    assert!(
        t32.residual <= eps_phys::<f32>(),
        "the f32 round trip is off"
    );
}

#[test]
fn roundtrip_trip() {
    check_trip(false);
}

negative_control!(
    roundtrip_trip,
    "the encode under every link swapped must fail",
    expected = "the round trip is",
    check_trip(true)
);

/// `widen` is exact.
fn check_widen(w: fn(&Ic<f32>) -> Ic<f64>) {
    let x = Ic {
        m: [0.1f32, 0.2, 0.7],
        r: [[1.5, -0.25], [0.125, 3.0], [-2.0, 0.3]],
        p: [[0.7, 0.1], [-0.2, -0.9], [0.4, 0.6]],
    };
    let y = w(&x);
    let pairs =
        x.m.iter()
            .zip(&y.m)
            .chain(x.r.iter().flatten().zip(y.r.iter().flatten()))
            .chain(x.p.iter().flatten().zip(y.p.iter().flatten()));
    for (a, b) in pairs {
        assert_eq!(f64::from(*a), *b, "widen is not exact");
    }
}

#[test]
fn roundtrip_widen() {
    check_widen(widen);
}

negative_control!(
    roundtrip_widen,
    "a widening that swaps the momenta's axes must fail",
    expected = "widen is not exact",
    check_widen(|x| {
        let mut y = widen(x);
        for p in y.p.iter_mut() {
            p.swap(0, 1);
        }
        y
    })
);

// ── The registry ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The registry: its fourteen keys in chart_reference §5.1's order, each a chart of its type that decodes, the key
/// its own; no chart for an unknown key.
fn check_registry(listed: Vec<&'static str>) {
    let want = [
        ("latent", "Latent"),
        ("shape_sphere", "ShapeSphere"),
        ("shape_sphere_exponential", "ShapeSphere"),
        ("burrau", "Burrau"),
        ("burrau_full", "Burrau"),
        ("euclid_plane", "EuclidPlane"),
        ("burrau_theta_k", "BurrauThetaK"),
        ("burrau_theta_lz", "BurrauThetaLz"),
        ("burrau_theta_delta_m", "BurrauThetaDeltaM"),
        ("invariant_le", "InvariantLE"),
        ("invariant_lk", "InvariantLK"),
        ("mass_simplex", "MassSimplex"),
        ("mixed", "Mixed"),
        ("mixed_latent", "Mixed"),
    ];
    assert_eq!(
        listed,
        want.iter().map(|w| w.0).collect::<Vec<_>>(),
        "the registry's keys"
    );
    for (key, ty) in want {
        let c = chart(key).unwrap_or_else(|| panic!("no chart {key}"));
        assert_eq!((c.key(), c.type_name()), (key, ty), "the chart {key}");
        let (a, b) = (c.decode64(0.3, 0.6).ic, c.decode32(0.3, 0.6).ic);
        assert!(
            (a.m.iter().sum::<f64>() - 1.0).abs() < 1e-12,
            "{key} decodes at f64"
        );
        assert!(
            (b.m.iter().sum::<f32>() - 1.0).abs() < 1e-5,
            "{key} decodes at f32"
        );
    }
    assert!(
        chart("no_such_chart").is_none(),
        "a chart for an unknown key"
    );
    let all = registry();
    assert_eq!(
        all.iter().map(|c| c.key()).collect::<Vec<_>>(),
        listed,
        "registry() against keys()"
    );
}

#[test]
fn roundtrip_registry() {
    check_registry(keys());
}

negative_control!(
    roundtrip_registry,
    "a registry missing its last chart must fail",
    expected = "the registry's keys",
    {
        let mut k = keys();
        k.pop();
        check_registry(k)
    }
);
