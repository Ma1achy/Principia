//! The conversion of a stored `z_β` to the periodic `β` (dd_decoder §3.2, "How a stored value converts";
//! `kernel::decode::retired`, `kernel::chart::saved`):
//! - `latent_conversion_same_system`: for each retired link `L`, every converted `z`, `z_β′ = L(z_β) − π/2`, decodes under
//!   the periodic link to the same system as its old `z` decoded through `L`, within the round trip's `ε_phys` on the
//!   decoded state in `‖·‖_phys`, at f64 and f32, over `z_β` across the link's range, the saturated ends included.
//!   Control: a value converted by the wrong link fails.
//! - `latent_conversion_saved_file`: a saved file of the old version, kinds 0 and 1, is converted on load
//!   (`Saved::load`), its converted words marked with the new kinds, 2 and 3; it converts from the `β` link it pins, an
//!   unpinned one from `sigmoid_beta` with the assumption reported for the notice on load; an unknown kind is refused by
//!   `Saved::from_words` and by `Saved::load`, an old kind by `Saved::from_words`; a definition whose plane runs along
//!   `z_β` is refused. Control: an unpinned chart converted with no assumption reported fails.
//! - `latent_conversion_saved_link`: the stored `z_β` words each converted by the pinned link, or the assumed one,
//!   and nothing else changed. Control: a chart converted by the wrong link fails.
//!
//! **Domain** (pitfalls §10): decode alone, on `z` held fixed; no trajectory.

use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Beta, Mixed, ZAlpha, ZBeta};
use kernel::chart::saved::{Loaded, Saved, WORDS};
use kernel::chart::{InvariantLE, Latent, ShapeMap, ShapeSphere, TiltMap, TiltSpec};
use kernel::decode::decode;
use kernel::decode::retired::{
    convert, convert_stored_f32, converted, decode_retired, RetiredLink, ASSUMED,
};
use kernel::generated::links::{
    Defaults, Link, LinkReal, RetiredBeta, SigmoidBeta, SoftsignBeta, TanhBeta,
};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys, widen, Precision};

// ── latent_conversion_same_system ───────────────────────────────────────────────────────────────────────────────

/// The other seven controls: interior values, each block's.
const OTHER: [f64; 8] = [0.35, 0.0, -0.25, 0.4, -0.15, 0.3, 0.2, -0.45];

/// The stored `z_β` values: across each link's range, `±40` saturating the sigmoid and tanh links, `±1e6` the softsign
/// link, and `±0`.
fn stored() -> Vec<f64> {
    let mut out: Vec<f64> = (-80..=80).map(|k| f64::from(k) / 8.0).collect();
    out.extend([40.0, -40.0, 1e6, -1e6, 0.0, -0.0, 1e-300]);
    out
}

/// The old `z` with `z_β = b`, at `R`.
fn old_z<R: LinkReal>(b: f64) -> kernel::decode::Latent<R> {
    let mut z = OTHER;
    z[1] = b;
    kernel::decode::Latent::new(z.map(|x| R::from(x).expect("a float")))
}

/// For the retired link `L`, each stored `z_β`: the old decode through `L` at `R` (`decode_retired`), against the
/// periodic decode of the value `conv` converts it to at f64, the stored precision, rounded to `R`; the worst
/// `‖·‖_phys` over `ε_phys`.
fn check_same<L, R>(conv: fn(f64) -> f64) -> f64
where
    L: Link<1, 1, Codomain = RetiredBeta>,
    R: Precision,
{
    let mut worst = 0.0f64;
    for b in stored() {
        let old = decode_retired::<Defaults, L, R>(&old_z::<R>(b)).ic;
        let new = decode::<Defaults, R>(&old_z::<R>(conv(b))).ic;
        let d = phys(&widen(&old), &widen(&new)) / eps_phys::<R>();
        assert!(
            d <= 1.0,
            "{} at z_β = {b:e}, f{}: the converted z decodes {d:.2} ε_phys from the old system",
            L::NAME,
            8 * core::mem::size_of::<R>()
        );
        worst = worst.max(d);
    }
    worst
}

/// [`check_same`] for `L` at f64 and f32, converted by `L` itself.
fn check_link<L: Link<1, 1, Codomain = RetiredBeta>>() -> [f64; 2] {
    [
        check_same::<L, f64>(convert::<L, f64>),
        check_same::<L, f32>(convert::<L, f64>),
    ]
}

#[test]
fn latent_conversion_same_system() {
    let worst = [
        (SigmoidBeta::NAME, check_link::<SigmoidBeta>()),
        (TanhBeta::NAME, check_link::<TanhBeta>()),
        (SoftsignBeta::NAME, check_link::<SoftsignBeta>()),
    ];
    // `converted` is `convert` on `z_β` alone.
    let z = old_z::<f64>(0.7);
    let c = converted::<TanhBeta, f64>(&z);
    let mut want = z;
    want.z[1] = convert::<TanhBeta, f64>(0.7);
    assert_eq!(c, want, "converted is not convert on z_β alone");
    eprintln!(
        "latent_conversion_same_system: {} stored z_β per link; worst ‖·‖_phys in ε_phys (f64, f32): {worst:?}",
        stored().len()
    );
}

negative_control!(
    latent_conversion_same_system,
    "a sigmoid_beta value converted by tanh_beta decodes to another system and must fail",
    expected = "sigmoid_beta at z_β",
    {
        check_same::<SigmoidBeta, f64>(convert::<TanhBeta, f64>);
    }
);

// ── latent_conversion_saved_file ────────────────────────────────────────────────────────────────────────────────

/// An oblique chart's centre, its `z_β` stored under a retired link, its plane off `z_β`.
const Z0: [f64; 8] = [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4];

fn oblique() -> Saved {
    let unit = |k: usize| core::array::from_fn(|n| if n == k { 1.0 } else { 0.0 });
    Saved::latent(&Latent {
        z0: Z0,
        q1: unit(2),
        q2: unit(3),
    })
}

fn spec(tau1: f64, x: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2: 0.0,
        map: TiltMap::Sheet,
        x,
        y: x,
    }
}

/// A curved chart on the `(L_z, E)` chart, whose frozen `z_cfg` holds a `z_β`, tilted 25° into `x`.
fn invariant(x: u32) -> Saved {
    let chart = InvariantLE(Invariant {
        z_cfg: [0.4, -0.3],
        z_mass: [0.2, -0.5],
        k_max: 4.0,
        gamma_k: 1.0,
    });
    Saved::curved(&chart, spec(25.0, x), [0.37, 0.61], [2.0, 2.0]).expect("a curved parent")
}

/// A curved chart on the shape sphere, which stores no `z_β`.
fn sphere() -> Saved {
    let chart = ShapeSphere {
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
    Saved::curved(&chart, spec(25.0, 2), [0.37, 0.61], [2.0, 2.0]).expect("a curved parent")
}

/// `def`'s words as the old version wrote them: kind 0 for an oblique chart, 1 for a curved one, every other word
/// the same.
fn old_words(def: &Saved) -> [f64; WORDS] {
    let mut w = def.to_words();
    w[0] = match def {
        Saved::Oblique(_) => 0.0,
        Saved::Curved(_) => 1.0,
    };
    w
}

/// A loader under test: `Saved::load`, or a control's.
type Load = fn(&[f64; WORDS], Option<RetiredLink>) -> Option<Loaded>;

/// The file checks on `load` (the module's note).
fn check_file(load: Load) {
    for def in [oblique(), invariant(2), sphere()] {
        let new_kind = def.to_words()[0];
        assert!(
            new_kind == 2.0 || new_kind == 3.0,
            "a definition writes kind {new_kind}, not the new kinds"
        );
        let old = old_words(&def);
        assert!(
            Saved::from_words(&old).is_none(),
            "Saved::from_words reads an old kind, {}, without converting",
            old[0]
        );
        let unpinned = load(&old, None).expect("an old file converts on load");
        assert_eq!(
            unpinned.assumed,
            Some(ASSUMED),
            "an unpinned old file converted with no assumption reported"
        );
        assert_eq!(
            unpinned.saved.to_words()[0],
            new_kind,
            "the converted words are not marked with the new kind"
        );
        for link in RetiredLink::ALL {
            let pinned = load(&old, Some(link)).expect("a pinned old file converts on load");
            assert_eq!(pinned.assumed, None, "a pinned file reports an assumption");
            assert_eq!(
                pinned.saved,
                def.converted(link).expect("converts"),
                "a file pinning {} is not converted from it",
                link.name()
            );
        }
        let current = load(&def.to_words(), None).expect("a current file loads");
        assert!(
            current.saved == def && current.assumed.is_none(),
            "a file of the current kind is converted again"
        );
        for kind in [4.0, -0.0, 0.5] {
            let mut unknown = def.to_words();
            unknown[0] = kind;
            assert!(
                Saved::from_words(&unknown).is_none() && load(&unknown, None).is_none(),
                "kind {kind} reads as a definition"
            );
        }
    }
    // A plane along z_β: refused, not approximated.
    let mut along = oblique();
    if let Saved::Oblique(c) = &mut along {
        c.q2 = [0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    }
    let mixed = Saved::curved(
        &Mixed::<ZAlpha, ZBeta, f64>::new(Z0, [-1.0, 1.0], [-1.0, 1.0]),
        spec(0.0, 2),
        [0.5, 0.5],
        [2.0, 2.0],
    )
    .expect("a curved parent");
    for (def, what) in [
        (along, "an oblique plane along z_β"),
        (mixed, "a mixed chart with a raw z_β axis"),
        (invariant(1), "a curved chart tilted into z_β"),
    ] {
        assert!(
            load(&old_words(&def), None).is_none(),
            "{what} is converted as a plane"
        );
    }
    // The derived β axis reads β, not z_β: its chart converts.
    let derived = Saved::curved(
        &Mixed::<ZAlpha, Beta, f64>::new(Z0, [-1.0, 1.0], [0.5, 2.5]),
        spec(0.0, 2),
        [0.5, 0.5],
        [2.0, 2.0],
    )
    .expect("a curved parent");
    assert!(
        load(&old_words(&derived), None).is_some(),
        "a mixed chart with a derived β axis is refused"
    );
}

#[test]
fn latent_conversion_saved_file() {
    check_file(Saved::load);
}

negative_control!(
    latent_conversion_saved_file,
    "a loader that converts an unpinned chart from sigmoid_beta with no assumption reported must fail",
    expected = "an unpinned old file converted with no assumption reported",
    {
        check_file(|w, p| {
            Saved::load(w, p).map(|l| Loaded {
                assumed: None,
                ..l
            })
        });
    }
);

// ── latent_conversion_saved_link ────────────────────────────────────────────────────────────────────────────────

/// The words `convert` gives `def` under `link`: the stored `z_β` words each `link`'s forward of the old value, the
/// rest unchanged, against `Saved::converted`.
fn check_words(conv: fn(RetiredLink, f64) -> f64) {
    for link in RetiredLink::ALL {
        for (def, at) in [(oblique(), 2), (invariant(2), 11 + 2)] {
            let old = def.to_words();
            let got = def.converted(link).expect("converts").to_words();
            for k in 1..WORDS {
                let want = if k == at { conv(link, old[k]) } else { old[k] };
                assert_eq!(
                    got[k].to_bits(),
                    want.to_bits(),
                    "{}: word {k} is {}, not {want}",
                    link.name(),
                    got[k]
                );
            }
        }
        let s = sphere();
        assert_eq!(
            s.converted(link),
            Some(s),
            "a shape sphere, which stores no z_β, is changed"
        );
    }
}

#[test]
fn latent_conversion_saved_link() {
    check_words(|link, x| link.convert(x));
    assert_eq!(
        RetiredLink::ALL.map(RetiredLink::name),
        [SigmoidBeta::NAME, TanhBeta::NAME, SoftsignBeta::NAME]
    );
    for link in RetiredLink::ALL {
        assert_eq!(RetiredLink::named(link.name()), Some(link));
    }
    assert_eq!(RetiredLink::named("periodic_beta"), None);
    assert_eq!(ASSUMED, RetiredLink::Sigmoid);
}

negative_control!(
    latent_conversion_saved_link,
    "a chart converted by the wrong link must fail",
    expected = "word 2 is",
    {
        check_words(|link, x| match link {
            RetiredLink::Sigmoid => RetiredLink::Tanh.convert(x),
            _ => link.convert(x),
        });
    }
);

// ── latent_conversion_stored_f32: a control stored as an f32 ────────────────────────────────────────────────────

/// Stored f32 `z_β` values that are not exact in binary, as a frame's hand-typed controls are (`−0.3`, `−0.2`), with
/// some that are.
const STORED_F32: [f32; 8] = [-0.3, -0.2, 0.1, 0.7, -1.3, 2.9, 0.41, 0.5];

/// For each stored f32 `z_β` under `L`: `conv`'s f32 must be the f32 nearest the f64 conversion of the stored f32
/// itself, widened. The criterion, written before the run: bit equality with that nearest f32, no
/// tolerance.
fn check_stored_f32<L: Link<1, 1, Codomain = RetiredBeta>>(conv: fn(f32) -> f32) {
    for old in STORED_F32 {
        let exact = convert::<L, f64>(f64::from(old));
        let nearest = exact as f32;
        let got = conv(old);
        assert!(
            got.to_bits() == nearest.to_bits(),
            "{} at the stored f32 {old}: converted to {got:e}, {:e} from the conversion of the stored value; the \
             nearest f32 is {nearest:e}, {:e} from it",
            L::NAME,
            f64::from(got) - exact,
            f64::from(nearest) - exact
        );
    }
}

#[test]
fn latent_conversion_stored_f32() {
    check_stored_f32::<SigmoidBeta>(convert_stored_f32::<SigmoidBeta>);
    check_stored_f32::<TanhBeta>(convert_stored_f32::<TanhBeta>);
    check_stored_f32::<SoftsignBeta>(convert_stored_f32::<SoftsignBeta>);
}

negative_control!(
    latent_conversion_stored_f32,
    "converting the decimal the control was typed as (−0.3 at f64), not the stored f32, lands one f32 step off",
    expected = "sigmoid_beta at the stored f32 -0.3: converted to -2.3386806e-1",
    {
        // The stored values' decimals, as typed: what a hand conversion starts from.
        const TYPED: [f64; 8] = [-0.3, -0.2, 0.1, 0.7, -1.3, 2.9, 0.41, 0.5];
        check_stored_f32::<SigmoidBeta>(|old| {
            let k = STORED_F32
                .iter()
                .position(|s| s.to_bits() == old.to_bits())
                .expect("a stored value");
            convert::<SigmoidBeta, f64>(TYPED[k]) as f32
        });
    }
);
