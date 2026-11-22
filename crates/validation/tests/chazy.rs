//! The Fewbody reference's arms (chazy drill-down §3.7, §12.1; REQ-VAL-184): `validation::chazy::fewbody_resolved` at
//! `δ = 10⁻⁵` reads H for three bodies flying apart, B-stable for the Mardling–Aarseth hierarchy, HE at `d = 50` and
//! unresolved at `d = 20`, where the tidal ratios are `8·10⁻⁶` and `1.25·10⁻⁴`. Each with its control. The reference is
//! the classifier's yardstick, never a candidate: it reads none of `kernel::chazy`.

use validation::chazy::{
    binary_plus_escaper, equilateral_state, fewbody_resolved, fewbody_tidal, ma_hierarchy,
    ma_stable, Reference, State,
};
use validation::negative_control;

/// δ, the drill-down's (§3.3).
const DELTA: f64 = 1e-5;

fn flying_apart(s: &State) {
    assert!(s.energy() > 0.0, "E > 0: E = {}", s.energy());
    let got = fewbody_resolved(s, DELTA);
    assert_eq!(got, Reference::H, "three bodies flying apart read {got:?}");
}

#[test]
fn chazy_fewbody_three_bodies_flying_apart_is_h() {
    flying_apart(&equilateral_state(50.0, 2.0));
}

negative_control!(
    chazy_fewbody_three_bodies_flying_apart_is_h,
    "falling together (velocities reversed) is unresolved: no pair recedes",
    expected = "three bodies flying apart read",
    flying_apart(&equilateral_state(50.0, -2.0))
);

fn hierarchy(s: &State) {
    assert!(ma_stable(s, 0, 1, 2), "ma_stable(0, 1, 2) is true");
    let got = fewbody_resolved(s, DELTA);
    assert_eq!(got, Reference::BStable, "the hierarchy reads {got:?}");
}

#[test]
fn chazy_fewbody_a_mardling_aarseth_stable_hierarchy_is_b_stable() {
    hierarchy(&ma_hierarchy(0.0));
}

negative_control!(
    chazy_fewbody_a_mardling_aarseth_stable_hierarchy_is_b_stable,
    "the outer body brought in to 0.25 (ρ = 2.5), where Mardling–Aarseth fails",
    expected = "ma_stable(0, 1, 2) is true",
    {
        let s = ma_hierarchy(0.0);
        let mut r = s.r;
        r[2] = [0.25, 0.0];
        let mut v = s.v;
        v[2] = [0.0, (1.0f64 / 0.25).sqrt()];
        hierarchy(&State::com(s.m, r, v))
    }
);

fn tidal(delta: f64) {
    for (d, ratio, want) in [
        (50.0, 8e-6, Reference::He),
        (20.0, 1.25e-4, Reference::Unresolved),
    ] {
        let s = binary_plus_escaper(1.0, 0.0, d, 2.0);
        let t = fewbody_tidal(&s, 0, 1, 2);
        assert!(
            (t / ratio - 1.0).abs() < 1e-9,
            "d = {d}: tidal ratio {t:e}, want {ratio:e}"
        );
        let got = fewbody_resolved(&s, delta);
        assert_eq!(got, want, "the reference at d = {d} reads {got:?}");
    }
}

#[test]
fn chazy_fewbody_resolves_he_only_once_tidally_isolated() {
    tidal(DELTA);
}

negative_control!(
    chazy_fewbody_resolves_he_only_once_tidally_isolated,
    "at δ = 10⁻³ the reference resolves d = 20 too",
    expected = "the reference at d = 20",
    tidal(1e-3)
);

/// The reference is independent of the classifier: it agrees with the classifier where §12.1 says it does (H, B-stable,
/// HE at d = 50) and where it reads unresolved the classifier still commits on the hierarchy gate (HE at d = 20), as
/// §12.1's tidal row says.
#[test]
fn chazy_fewbody_agrees_with_the_classifier_where_the_fixtures_say() {
    check_agreement(DELTA);
}

negative_control!(
    chazy_fewbody_agrees_with_the_classifier_where_the_fixtures_say,
    "a reference at δ = 10⁻³ resolves d = 20, which §12.1 says it does not",
    expected = "the reference reads",
    check_agreement(1e-3)
);

/// [`fewbody_resolved`] at `delta` and the classifier read each fixture as §12.1 says.
fn check_agreement(delta: f64) {
    use kernel::chazy::{classify, inertia, ChazyCfg, Motion, Playhead, Previous};
    let at = |s: &State| Playhead {
        m: s.m,
        r: s.r,
        v: s.v,
        e: s.energy(),
        r_0: 1.0,
        i_max: inertia(&s.m, &s.r),
    };
    let label = |s: &State| classify(&at(s), &ChazyCfg::defaults(), &Previous::none());
    let cases = [
        (equilateral_state(50.0, 2.0), Reference::H, Motion::H),
        (ma_hierarchy(0.0), Reference::BStable, Motion::BStable),
        (
            binary_plus_escaper(1.0, 0.0, 50.0, 2.0),
            Reference::He,
            Motion::He,
        ),
        (
            binary_plus_escaper(1.0, 0.0, 20.0, 2.0),
            Reference::Unresolved,
            Motion::He,
        ),
    ];
    for (s, reference, classifier) in cases {
        let got = fewbody_resolved(&s, delta);
        assert_eq!(got, reference, "the reference reads {got:?}");
        assert_eq!(label(&s), classifier);
    }
}
