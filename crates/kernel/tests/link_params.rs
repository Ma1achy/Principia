//! The generated links' tunable parameters (R-481; dd_generation_root §3.9, "Tunable parameters"): the parametrised
//! functions at `LinkParams::DEFAULT` bitwise today's (REQ-GEN-040, R-482 C2), the log-det at non-default values
//! against the forward's finite-differenced volume factor (REQ-GEN-039, C5), and decode and encode at the active
//! values (REQ-DEC-048).

use kernel::decode::{decode, decode_at, Latent};
use kernel::encode::{encode, encode_at, SimplexClamp};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::*;
use validation::{eval, negative_control, range_values, registered, tunables, Fl, Values};

/// splitmix64, uniform on `[0, 1)`.
struct Seeded(u64);

impl Seeded {
    fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// `n` controls: 48 seeded draws on `[−8, 8]ⁿ`, then each saturating control in every component.
fn sample(n: usize) -> Vec<Vec<f64>> {
    let mut g = Seeded(41);
    let mut out: Vec<Vec<f64>> = (0..48)
        .map(|_| (0..n).map(|_| 16.0 * g.unit() - 8.0).collect())
        .collect();
    for s in [
        0.0, 0.5, -0.5, 20.0, -20.0, 40.0, -40.0, 1e3, -1e3, 1e8, -1e8,
    ] {
        out.push(vec![s; n]);
    }
    out
}

/// A float the links evaluate at that the ledger's evaluator evaluates at too.
trait Both: LinkReal + Fl + std::fmt::Debug {
    /// The next float above `self`.
    fn up(self) -> Self;
}

impl Both for f64 {
    fn up(self) -> Self {
        f64::from_bits(self.to_bits() + 1)
    }
}

impl Both for f32 {
    fn up(self) -> Self {
        f32::from_bits(self.to_bits() + 1)
    }
}

fn bits<R: Fl>(v: &[R]) -> Vec<u64> {
    v.iter().map(|x| Fl::f64(*x).to_bits()).collect()
}

fn of<R: Fl>(v: &[f64]) -> Vec<R> {
    v.iter().map(|&x| <R as Fl>::of(x)).collect()
}

/// `L`'s three functions at `p`: `(forward, inverse of the forward, log-det)` over `xs`, each flattened.
fn run<const C: usize, const P: usize, L: Link<C, P>, R: Both>(
    xs: &[Vec<f64>],
    p: Option<&LinkParams<R>>,
) -> Vec<u64> {
    let mut out = Vec::new();
    for z in xs {
        let x: [R; C] = of::<R>(z).try_into().unwrap();
        let (y, back, ld) = match p {
            Some(p) => {
                let y = L::forward_at(x, p);
                (y, L::inverse_at(y, p), L::log_det_at(x, p))
            }
            None => {
                let y = L::forward(x);
                (y, L::inverse(y), L::log_det(x))
            }
        };
        out.extend(bits(&y));
        out.extend(bits(&back));
        out.extend(bits(&[ld]));
    }
    out
}

/// The same over the ledger's evaluator of `L`'s registry trees, at the declared constants.
fn run_trees<const C: usize, R: Both>(name: &str, xs: &[Vec<f64>]) -> Vec<u64> {
    let (link, _) = registered().iter().find(|(l, _)| l.name == name).unwrap();
    let declared = [link.clamps, link.params].concat();
    let values = Values {
        declared: &declared,
        active: &[],
    };
    let mut out = Vec::new();
    for z in xs {
        let x = of::<R>(z);
        let y: Vec<R> = link.forward.iter().map(|t| eval(t, &x, &values)).collect();
        let back: Vec<R> = link.inverse.iter().map(|t| eval(t, &y, &values)).collect();
        out.extend(bits(&y));
        out.extend(bits(&back));
        out.extend(bits(&[eval::<R>(&link.log_det, &x, &values)]));
    }
    out
}

/// `L` at `p` is bitwise its at-default functions and its registry trees evaluated at the chart constants (f32 and
/// f64 alike).
fn check_default<const C: usize, const P: usize, L: Link<C, P>, R: Both>(p: &LinkParams<R>) {
    let xs = sample(C);
    let today = run::<C, P, L, R>(&xs, None);
    assert!(
        run::<C, P, L, R>(&xs, Some(p)) == today,
        "`{}` at the parameters is not bitwise its at-default functions",
        L::NAME
    );
    assert!(
        run_trees::<C, R>(L::NAME, &xs) == today,
        "`{}`'s at-default functions are not its trees at the chart constants",
        L::NAME
    );
}

fn check_all_default<R: Both>(p: &LinkParams<R>) {
    check_default::<2, 3, SoftmaxTanh, R>(p);
    check_default::<2, 3, StickBreaking, R>(p);
    check_default::<1, 1, SigmoidAlpha, R>(p);
    check_default::<1, 1, SigmoidBeta, R>(p);
    check_default::<1, 1, TanhAlpha, R>(p);
    check_default::<1, 1, TanhBeta, R>(p);
    check_default::<1, 1, SoftsignAlpha, R>(p);
    check_default::<1, 1, SoftsignBeta, R>(p);
    check_default::<1, 1, SigmoidQ, R>(p);
    check_default::<1, 1, TanhQ, R>(p);
    check_default::<1, 1, SoftsignQ, R>(p);
    check_default::<1, 1, Softplus, R>(p);
    check_default::<1, 1, Exp, R>(p);
    check_default::<1, 1, Identity, R>(p);
}

/// The generated link functions read no chart constant that is tunable by name; they read `p`.
fn check_generated_text(text: &str) {
    for name in ["R::MU_MAX", "R::Q_MAX"] {
        assert!(!text.contains(name), "the generated links read `{name}`");
    }
    for name in ["p.mu_max", "p.q_max"] {
        assert!(
            text.contains(name),
            "the generated links do not read `{name}`"
        );
    }
}

fn links_rs() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/generated/links.rs"
    ))
    .expect("the generated links are read")
}

/// One ulp off the default in each tunable moves some output of each entry that reads it, at `R`.
fn check_ulp_moves<R: Both>() {
    let d = LinkParams::<R>::DEFAULT;
    let mu = LinkParams {
        mu_max: d.mu_max.up(),
        ..d
    };
    let q = LinkParams {
        q_max: d.q_max.up(),
        ..d
    };
    let moved =
        |a: Vec<u64>, b: Vec<u64>, name: &str| assert!(a != b, "one ulp off left `{name}` unmoved");
    moved(
        run::<2, 3, SoftmaxTanh, R>(&sample(2), Some(&mu)),
        run::<2, 3, SoftmaxTanh, R>(&sample(2), None),
        "softmax_tanh",
    );
    moved(
        run::<1, 1, SigmoidQ, R>(&sample(1), Some(&q)),
        run::<1, 1, SigmoidQ, R>(&sample(1), None),
        "sigmoid_q",
    );
    moved(
        run::<1, 1, TanhQ, R>(&sample(1), Some(&q)),
        run::<1, 1, TanhQ, R>(&sample(1), None),
        "tanh_q",
    );
    moved(
        run::<1, 1, SoftsignQ, R>(&sample(1), Some(&q)),
        run::<1, 1, SoftsignQ, R>(&sample(1), None),
        "softsign_q",
    );
}

#[test]
fn link_params_default_bitwise() {
    check_all_default::<f64>(&LinkParams::DEFAULT);
    check_all_default::<f32>(&LinkParams::DEFAULT);
    check_generated_text(&links_rs());
    check_ulp_moves::<f64>();
    check_ulp_moves::<f32>();
    assert_eq!(
        (
            LinkParams::<f64>::DEFAULT.mu_max,
            LinkParams::<f64>::DEFAULT.q_max
        ),
        (
            <f64 as ChartConstants>::MU_MAX,
            <f64 as ChartConstants>::Q_MAX
        )
    );
}

negative_control!(
    link_params_default_bitwise,
    "μ_max one ulp above its default is not today's softmax_tanh",
    expected = "is not bitwise its at-default functions",
    {
        let d = LinkParams::<f64>::DEFAULT;
        check_default::<2, 3, SoftmaxTanh, f64>(&LinkParams {
            mu_max: d.mu_max.up(),
            ..d
        })
    }
);

negative_control!(
    link_params_default_bitwise_text,
    "a generated file reading R::MU_MAX is refused",
    expected = "the generated links read `R::MU_MAX`",
    check_generated_text(&links_rs().replace("p.mu_max", "R::MU_MAX"))
);

// ── REQ-GEN-039: the log-det at the active values ───────────────────────────────────────────────────────────────

/// The Richardson-extrapolated central difference of `f` along control `k` at `x`, `(4D(h) − D(2h))/3` at `h` = 10⁻⁴:
/// its error is `O(h⁴)` where `f` is smooth, which softsign is not at 0 (C¹ but not C², dd_generation_root §3.9).
/// For the simplex, the largest mass's component is taken from the others, the column summing to 0.
fn column<const C: usize, const P: usize>(
    f: &dyn Fn([f64; C]) -> [f64; P],
    x: [f64; C],
    k: usize,
) -> [f64; P] {
    let d = |h: f64| {
        let (mut a, mut b) = (x, x);
        a[k] += h;
        b[k] -= h;
        let (fa, fb) = (f(a), f(b));
        let mut out = [0.0; P];
        for i in 0..P {
            out[i] = (fa[i] - fb[i]) / (a[k] - b[k]);
        }
        out
    };
    let (d1, d2) = (d(1e-4), d(2e-4));
    let mut out = [0.0; P];
    for i in 0..P {
        out[i] = (4.0 * d1[i] - d2[i]) / 3.0;
    }
    if P == 3 {
        // On the simplex `Σmᵢ = 1`, so a column sums to 0: the largest mass's component, whose difference loses the
        // most to rounding, is minus the others'.
        let y = f(x);
        let big = (0..P).max_by(|&i, &j| y[i].total_cmp(&y[j])).unwrap();
        out[big] = -(0..P).filter(|&i| i != big).map(|i| out[i]).sum::<f64>();
    }
    out
}

/// `log √det(JᵀJ)` of `f` at `x`, `J` by [`column`].
fn log_volume<const C: usize, const P: usize>(
    f: &dyn Fn([f64; C]) -> [f64; P],
    x: [f64; C],
) -> f64 {
    let cols: Vec<[f64; P]> = (0..C).map(|k| column(f, x, k)).collect();
    let dot = |a: &[f64; P], b: &[f64; P]| a.iter().zip(b).map(|(p, q)| p * q).sum::<f64>();
    let gram = if C == 1 {
        dot(&cols[0], &cols[0])
    } else {
        dot(&cols[0], &cols[0]) * dot(&cols[1], &cols[1]) - dot(&cols[0], &cols[1]).powi(2)
    };
    0.5 * gram.ln()
}

/// `L`'s log-det at `ld_at` within 10⁻⁶ of the forward's at `fwd_at`, over 32 seeded `z` on `[−3, 3]ⁿ`, and 0 where
/// `smooth_at_0` (every entry but softsign, whose second derivative jumps there).
fn check_log_det<const C: usize, const P: usize, L: Link<C, P>>(
    fwd_at: &LinkParams<f64>,
    ld_at: &LinkParams<f64>,
    smooth_at_0: bool,
) {
    let mut g = Seeded(39);
    let mut xs: Vec<[f64; C]> = (0..32)
        .map(|_| [0.0; C].map(|_| 6.0 * g.unit() - 3.0))
        .collect();
    if smooth_at_0 {
        xs.push([0.0; C]);
    }
    for x in xs {
        let want = log_volume(&|z| L::forward_at(z, fwd_at), x);
        let got = L::log_det_at(x, ld_at);
        assert!(
            (got - want).abs() <= 1e-6,
            "`{}`'s log-det {got} at z = {x:?}, {fwd_at:?} is not the forward's {want}",
            L::NAME
        );
    }
}

/// Each tunable at its range's ends and three interior values.
fn tunable_values() -> Vec<LinkParams<f64>> {
    let d = LinkParams::<f64>::DEFAULT;
    let mut out = Vec::new();
    for t in tunables() {
        for &v in &range_values(t.lo, t.hi)[..5] {
            out.push(match t.param {
                "mu_max" => LinkParams { mu_max: v, ..d },
                _ => LinkParams { q_max: v, ..d },
            });
        }
    }
    out
}

#[test]
fn link_logdet_params() {
    let values = tunable_values();
    assert_eq!(values.len(), 20, "five values per row");
    for p in &values {
        check_log_det::<2, 3, SoftmaxTanh>(p, p, true);
        check_log_det::<1, 1, SigmoidQ>(p, p, true);
        check_log_det::<1, 1, TanhQ>(p, p, true);
        check_log_det::<1, 1, SoftsignQ>(p, p, false);
    }
}

negative_control!(
    link_logdet_params,
    "C5's mutation arm: a log-det left at the default while the forward runs at μ_max = 4",
    expected = "is not the forward's",
    {
        let d = LinkParams::<f64>::DEFAULT;
        check_log_det::<2, 3, SoftmaxTanh>(&LinkParams { mu_max: 4.0, ..d }, &d, true)
    }
);

negative_control!(
    link_logdet_params_q,
    "a momentum log-det left at the default while the forward runs at q_max = 1",
    expected = "is not the forward's",
    {
        let d = LinkParams::<f64>::DEFAULT;
        check_log_det::<1, 1, SigmoidQ>(&LinkParams { q_max: 1.0, ..d }, &d, true)
    }
);

// ── REQ-DEC-048: decode and encode at the active values ─────────────────────────────────────────────────────────

/// A latent `z` whose `q₀` control decodes to 1.8 under the defaults, the rest interior.
fn z_at_1_8() -> [f64; 8] {
    let [zq] = SigmoidQ::inverse([1.8_f64]);
    [0.3, 0.7, zq, 0.1, -0.2, 0.4, 0.2, -0.1]
}

/// At `p`, decode keeps `|q| ≤ 1.5` and reaches it at saturation; encode clamps a momentum of 1.8 to 1.5, raising its
/// clamp bit (layer 1); at `μ_max = 4` the mass cap is `(1 − ε_μ)·4`; at the defaults, decode and encode are today's
/// bit for bit and 1.8 is not clamped.
fn check_active(q15: &LinkParams<f64>) {
    let mut g = Seeded(48);
    for _ in 0..64 {
        let z = [0.0; 8].map(|_| 100.0 * g.unit() - 50.0);
        let q = decode_at::<Defaults, f64>(&Latent::new(z), q15).momentum.q;
        assert!(
            q.iter().all(|v| v.abs() <= 1.5),
            "decode at q_max = 1.5 gives {q:?}"
        );
    }
    let sat = decode_at::<Defaults, f64>(&Latent::new([50.0; 8]), q15)
        .momentum
        .q;
    assert_eq!(sat, [1.5; 4], "saturated momenta at q_max = 1.5");

    let ic = decode::<Defaults, f64>(&Latent::new(z_at_1_8())).ic;
    let enc = encode_at::<Defaults, f64>(&ic, q15);
    assert_ne!(
        enc.report.clamped_controls & 1 << 2,
        0,
        "1.8 is not reported above q_max = 1.5"
    );
    let q0 = decode_at::<Defaults, f64>(&enc.z, q15).momentum.q[0];
    let image = 1.5 * (1.0 - 2.0 * <f64 as ChartConstants>::EPS_Q);
    assert!(
        q0 <= 1.5 && (q0 - image).abs() <= 1e-12,
        "1.8 is clamped to {q0}, not 1.5"
    );
    assert_eq!(
        encode::<Defaults, f64>(&ic).report.clamped_controls & 1 << 2,
        0,
        "1.8 is clamped at q_max = 2"
    );

    let d = LinkParams::<f64>::DEFAULT;
    let mu4 = LinkParams { mu_max: 4.0, ..d };
    let m0 = 1.0 / (1.0 + 4.5_f64.exp() + 1.0_f64.exp());
    let m = [m0, m0 * 4.5_f64.exp(), m0 * 1.0_f64.exp()];
    assert_eq!(
        SoftmaxTanh::clamps_at(m, &mu4),
        1,
        "a ratio of e^4.5 past μ_max = 4's cap"
    );
    assert_eq!(
        SoftmaxTanh::clamps(m),
        0,
        "a ratio of e^4.5 inside μ_max = 5's cap"
    );
    let cap = (1.0 - <f64 as ChartConstants>::EPS_MU) * 4.0;
    let [z1, _] = SoftmaxTanh::inverse_at(m, &mu4);
    assert_eq!(
        z1.to_bits(),
        (cap / 4.0).atanh().to_bits(),
        "the mass cap at μ_max = 4"
    );

    for z in sample(8) {
        let z = Latent::new(<[f64; 8]>::try_from(z).unwrap());
        assert_eq!(
            format!("{:?}", decode_at::<Defaults, f64>(&z, &d)),
            format!("{:?}", decode::<Defaults, f64>(&z)),
            "decode at the defaults is not today's"
        );
    }
    assert_eq!(
        format!("{:?}", encode_at::<Defaults, f64>(&ic, &d)),
        format!("{:?}", encode::<Defaults, f64>(&ic)),
        "encode at the defaults is not today's"
    );
}

#[test]
fn link_params_active() {
    let d = LinkParams::<f64>::DEFAULT;
    check_active(&LinkParams { q_max: 1.5, ..d });
}

negative_control!(
    link_params_active,
    "at the default q_max = 2 decode reaches past 1.5",
    expected = "decode at q_max = 1.5 gives",
    check_active(&LinkParams::<f64>::DEFAULT)
);
