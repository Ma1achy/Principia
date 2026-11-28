//! Unfold in the CPU evaluator, on the warped `(L_z, E)` chart.
//!
//! **Criteria**, written before the first run:
//! - `s = 1 + δ` resolves to `s' = 1 − δ` on sheet 2 and decodes, bitwise, to sheet 2 at `1 − δ`; likewise
//!   `s = −δ` to `δ`; and that is not sheet 1's decode there (the control: sheet 2 swapped for blank, and sheet 2 built as
//!   sheet 1 mirrored, a repeat, both fail);
//! - the time-reversal identity: sheet 2 at `s` has positions bitwise equal to, and momenta the exact negation of,
//!   sheet 1 at `1 − s` (`L_z → −L_z`), `+0 == −0`;
//! - sheet 1 is bitwise the chart's own decode;
//! - the period: dyadic `s + 2` resolves to the bits of `s`, counted only for `s̃ ∈ [−½, 3/2)`, a repeat past it where
//!   `N_⊥ = 0`; where `N_⊥ ≠ 0` (a tilt moving the unfolded axis) both pixels are counted, each its own system;
//! - inside `[0,1]²` a pixel comes back with its input bits.

use engine::edge::unfold::{decode, fold, resolve};
use engine::edge::{Count, Kind, Resolved, Sheet};
use kernel::chart::invariant::{decode_on_sheet, Invariant};
use kernel::chart::{Chart, InvariantLE};
use kernel::decode::invariant::Sheet as KSheet;
use kernel::generated::links::Defaults;

fn chart() -> InvariantLE<f64> {
    InvariantLE(Invariant {
        z_cfg: [0.3, 0.7],
        z_mass: [0.2, -0.4],
        k_max: 2.0,
        gamma_k: 1.5,
    })
}

fn ic_bits(d: &kernel::decode::Decoded<f64>) -> Vec<u64> {
    let mut v: Vec<u64> = d.ic.r.iter().flatten().map(|x| x.to_bits()).collect();
    v.extend(d.ic.p.iter().flatten().map(|x| x.to_bits()));
    v
}

fn go(p: [f64; 2], kind: Kind) -> Resolved {
    resolve(p, 0, kind, [true, true])
}

fn check_sheet2(resolve: impl Fn([f64; 2]) -> Resolved, mirrored: bool) {
    let c = chart();
    for &d in &[0.0625, 0.25, 0.375] {
        for &t in &[0.25, 0.5, 1.0] {
            for (s, back) in [(1.0 + d, 1.0 - d), (-d, d)] {
                let r = resolve([s, t]);
                assert_eq!(r.sheet, Sheet::Two, "sheet 2: ({s}, {t}) is on sheet 2");
                assert_eq!(
                    r.at[0].to_bits(),
                    back.to_bits(),
                    "sheet 2: ({s}, {t}) at {:?}",
                    r.at
                );
                let shown = if mirrored {
                    c.decode::<Defaults, f64>(r.at[0], r.at[1])
                } else {
                    decode(&c, &r)
                };
                let want = decode_on_sheet::<Defaults, f64, _>(&c, back, t, KSheet::Two).0;
                assert_eq!(
                    ic_bits(&shown),
                    ic_bits(&want),
                    "sheet 2: decode at ({s}, {t})"
                );
                let one = c.decode::<Defaults, f64>(back, t);
                assert_ne!(
                    ic_bits(&shown),
                    ic_bits(&one),
                    "sheet 2: shows sheet 1's system at ({s}, {t})"
                );
            }
        }
    }
}

#[test]
fn edge_unfold_second_sheet() {
    check_sheet2(|p| go(p, Kind::Unfold), false);
}

validation::negative_control!(
    edge_unfold_second_sheet,
    "the unfold arm swapped for blank",
    expected = "sheet 2:",
    check_sheet2(|p| go(p, Kind::Blank), false)
);

#[test]
fn edge_unfold_not_a_mirror() {
    check_sheet2(|p| go(p, Kind::Unfold), false);
}

validation::negative_control!(
    edge_unfold_not_a_mirror,
    "sheet 2 built as sheet 1 mirrored in s",
    expected = "sheet 2: decode",
    check_sheet2(|p| go(p, Kind::Unfold), true)
);

fn check_reversal(second: KSheet) {
    let c = chart();
    for &s in &[0.0625, 0.25, 0.375, 0.5, 0.75] {
        for &t in &[0.25, 0.5, 1.0] {
            let two = decode_on_sheet::<Defaults, f64, _>(&c, s, t, second).0;
            let one = c.decode::<Defaults, f64>(1.0 - s, t);
            for i in 0..3 {
                for k in 0..2 {
                    assert_eq!(
                        two.ic.r[i][k], one.ic.r[i][k],
                        "reversal: position at s = {s}"
                    );
                    assert_eq!(
                        two.ic.p[i][k], -one.ic.p[i][k],
                        "reversal: momentum at s = {s}"
                    );
                }
            }
        }
    }
}

#[test]
fn edge_unfold_time_reversal() {
    check_reversal(KSheet::Two);
}

validation::negative_control!(
    edge_unfold_time_reversal,
    "sheet 1 in place of sheet 2",
    expected = "reversal: momentum",
    check_reversal(KSheet::One)
);

fn check_baseline(sheet: KSheet) {
    let c = chart();
    for &s in &[0.0, 0.125, 0.5, 1.0] {
        for &t in &[0.25, 0.5, 1.0] {
            let a = decode_on_sheet::<Defaults, f64, _>(&c, s, t, sheet).0;
            assert_eq!(
                ic_bits(&a),
                ic_bits(&c.decode::<Defaults, f64>(s, t)),
                "baseline at ({s}, {t})"
            );
        }
    }
}

#[test]
fn edge_unfold_sheet_one_is_baseline() {
    check_baseline(KSheet::One);
}

validation::negative_control!(
    edge_unfold_sheet_one_is_baseline,
    "sheet 2 as the default",
    expected = "baseline at",
    check_baseline(KSheet::Two)
);

fn check_period(nz: [bool; 2], alone: bool) {
    for &s in &[-0.375, -0.25, 0.25, 0.75, 1.25, 1.375] {
        let canon = resolve([s, 0.5], 0, Kind::Unfold, nz);
        let far = resolve([s + 2.0, 0.5], 0, Kind::Unfold, nz);
        assert_eq!(
            far.at[0].to_bits(),
            canon.at[0].to_bits(),
            "period: s + 2 at {s}"
        );
        assert_eq!(far.sheet, canon.sheet, "period: sheet at {s}");
        assert_eq!(
            canon.count,
            Count::System,
            "period: ({s}) inside the period counts"
        );
        let want = if alone { Count::Repeat } else { Count::System };
        assert_eq!(far.count, want, "period: s + 2 at {s} counts as {want:?}");
    }
}

#[test]
fn edge_unfold_period_untilted() {
    check_period([true, true], true);
}

validation::negative_control!(
    edge_unfold_period_untilted,
    "the repeat kept where N_perp is not zero",
    expected = "period: s + 2",
    check_period([false, true], true)
);

#[test]
fn edge_unfold_period_tilted() {
    check_period([false, true], false);
}

validation::negative_control!(
    edge_unfold_period_tilted,
    "the repeat dropped where N_perp is zero",
    expected = "period: s + 2",
    check_period([true, true], false)
);

fn check_inside(resolve: impl Fn([f64; 2]) -> Resolved) {
    for &s in &[0.0, 0.25, 1.0] {
        for &t in &[0.0, 0.5, 1.0] {
            let r = resolve([s, t]);
            assert_eq!(
                [r.at[0].to_bits(), r.at[1].to_bits()],
                [s.to_bits(), t.to_bits()],
                "inside: moved"
            );
            assert_eq!(
                (r.sheet, r.count, r.applied),
                (Sheet::One, Count::System, None),
                "inside: record"
            );
        }
    }
}

#[test]
fn edge_unfold_inside_unchanged() {
    check_inside(|p| go(p, Kind::Unfold));
    assert_eq!(fold(1.5).1, Sheet::Two);
}

validation::negative_control!(
    edge_unfold_inside_unchanged,
    "a resolver that unfolds inside the square",
    expected = "inside:",
    check_inside(|p| Resolved {
        sheet: Sheet::Two,
        ..Resolved::own(p)
    })
);
