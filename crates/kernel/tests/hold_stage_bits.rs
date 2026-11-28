//! The decode stage's hold step (`kernel::decode::hold::apply_shared`), which runs the hold only where the `L_z` switch
//! is on, equals bit for bit, case by case over the set below at f64 and f32, a reference step that runs `hold` and
//! `η_E` both and selects on the switch ([`selected`]) (`hold_stage_bits_fixed`).
//!
//! The two are compared in one process, not against digests recorded on one machine: the set's f64 decodes differ
//! between aarch64 and x86_64, so a recorded digest holds on one platform only.
//!
//! **The set**: 24 seeded states, each with four velocities (moving, near rigid, exactly rigid, rest) and one with two
//! bodies coincident (the collision pole), each untagged and tagged; under every `E*` row (off, above `E₀`, below `U`,
//! `U + ½`), every `L*` row (off, 0, near the state's own, far above the floor, 1) and every chart flag (`E` forbidden
//! or not; `L_z` free, an axis, or held by construction). The census must reach every outcome but `AtTarget`, which
//! needs `E* = E₀` exactly.
//!
//! The momenta are compared as bits; the descriptor and the tag by their printed forms, which hide a NaN's sign and
//! payload; both steps form the descriptor from the momenta with the same `describe_decoded`.
//!
//! The control scales `L*` by `1 + 2⁻²⁰` in the step under test alone, which must make the outputs differ.

mod decode_support;
mod hold_support;

use std::collections::BTreeSet;

use decode_support::P;
use hold_support::*;
use kernel::chart::invariant::WarpReal;
use kernel::chart::{ChartDecode, Latent, LzHold};
use kernel::decode::descriptor::describe_decoded;
use kernel::decode::energy_norm::{self, Outcome};
use kernel::decode::hold::{apply_shared, hold};
use kernel::decode::invariant::ExactSign;
use kernel::decode::{Decoded, Degenerate, Ic};
use kernel::generated::links::Defaults;
use kernel::payload::PayloadReal;
use validation::negative_control;

/// The reference step: [`hold`] on a decode and `η_E` alone both computed, the switch selecting the momenta, the tag and
/// the outcome, the descriptor formed from the chosen IC. The branched step must equal it bit for bit; don't update it
/// to follow the kernel.
fn selected<R: ExactSign + PayloadReal>(
    decoded: Decoded<R>,
    e_star: Option<R>,
    forbids_energy_normalisation: bool,
    lz_star: Option<R>,
    lz_hold: LzHold,
) -> (Decoded<R>, Outcome) {
    let (on, lz) = match lz_star {
        Some(l) => (lz_hold == LzHold::Free, l),
        None => (false, R::zero()),
    };
    let base = energy_norm::apply_shared(decoded, e_star, forbids_energy_normalisation);
    let (e_on, target) = match e_star {
        Some(e) => (!forbids_energy_normalisation, e),
        None => (false, R::zero()),
    };
    let ic = decoded.ic;
    let velocity = |i: usize| [ic.p[i][0] / ic.m[i], ic.p[i][1] / ic.m[i]];
    let v = [velocity(0), velocity(1), velocity(2)];
    let d = &decoded.descriptor;
    let h = hold(ic.m, &ic.r, &v, d.V_0, d.r_min_pair_0, lz, e_on, target);
    let moved = (h.outcome == Outcome::Held) | (h.outcome == Outcome::OnFloor);
    let momentum = |i: usize| {
        [
            if moved {
                ic.m[i] * h.v[i][0]
            } else {
                ic.p[i][0]
            },
            if moved {
                ic.m[i] * h.v[i][1]
            } else {
                ic.p[i][1]
            },
        ]
    };
    let held_p = [momentum(0), momentum(1), momentum(2)];
    let held_tag = decoded.degenerate.or(h.outcome.tag());
    let ic = Ic {
        p: if on { held_p } else { base.0.ic.p },
        ..decoded.ic
    };
    (
        Decoded {
            ic,
            descriptor: describe_decoded(&ic, &decoded.config),
            degenerate: if on { held_tag } else { base.0.degenerate },
            ..decoded
        },
        if on { h.outcome } else { base.1 },
    )
}

/// A step's outputs, printed: the momenta's bits, the descriptor, the tag and the outcome.
fn printed<R: P>(out: &Decoded<R>, outcome: Outcome) -> String {
    format!(
        "{:?}|{:?}|{:?}|{outcome:?}",
        out.ic.p.map(|b| b.map(|x| x.bits())),
        out.descriptor,
        out.degenerate
    )
}

/// The decodes of the set at `R`: a latent decode's blocks with the IC replaced by each state and the descriptor
/// rewritten from it, as a decode writes it.
fn decodes<R>() -> Vec<Decoded<R>>
where
    R: P + WarpReal,
    Latent<R>: ChartDecode<Param = R>,
{
    let template = Latent {
        z0: [0.4, -0.2, 0.2, 0.9, -0.6, 0.1, 0.5, 0.3].map(R::of),
        q1: [R::of(0.0); 8],
        q2: [R::of(0.0); 8],
    }
    .decode_at::<Defaults, R>(R::of(0.0), R::of(0.0));
    let mut rng = Rng(0x5EED_4B1D);
    let mut out = Vec::new();
    let mut push = |m: [f64; 3], r: &Bodies, v: &Bodies| {
        let ic = Ic {
            m: m.map(R::of),
            r: to::<R>(r),
            p: to::<R>(&[0, 1, 2].map(|i| [m[i] * v[i][0], m[i] * v[i][1]])),
        };
        let descriptor = describe_decoded(&ic, &template.config);
        for degenerate in [None, Some(Degenerate::M01Tiny)] {
            out.push(Decoded {
                ic,
                descriptor,
                degenerate,
                ..template
            });
        }
    };
    for _ in 0..24 {
        let (m, r) = state(&mut rng);
        let w = jr(&r);
        let c = rng.normal();
        push(m, &r, &velocity(&m, &mut rng));
        push(m, &r, &near_rigid(&m, &r, 1e-3, &mut rng));
        push(m, &r, &[0, 1, 2].map(|i| [c * w[i][0], c * w[i][1]]));
        push(m, &r, &[[0.0; 2]; 3]);
        let mut pole = r;
        pole[1] = pole[0];
        push(m, &pole, &velocity(&m, &mut rng));
    }
    out
}

/// The cases at `R` where the step differs from the reference [`selected`], the step's `L*` scaled by `1 + 2⁻²⁰` where `nudge`,
/// with the first such case, and the outcomes the step reached.
fn differ<R>(nudge: bool) -> (usize, Option<String>, BTreeSet<String>)
where
    R: P + WarpReal + ExactSign,
    Latent<R>: ChartDecode<Param = R>,
{
    let scale = if nudge { 1.0 + 1.0 / 1_048_576.0 } else { 1.0 };
    let mut count = 0;
    let mut first = None;
    let mut seen = BTreeSet::new();
    for d in decodes::<R>() {
        let e0 = d.descriptor.K_0.f() + d.descriptor.V_0.f();
        let l0 = lz(&d.ic.m.map(|x| x.f()), &from(&d.ic.r), &{
            let p = from(&d.ic.p);
            [0, 1, 2].map(|i| [p[i][0] / d.ic.m[i].f(), p[i][1] / d.ic.m[i].f()])
        });
        let u = d.descriptor.V_0.f();
        // `E* = U + ½` with `L* = 1` puts a rest or rigid start on the floor `2K* = L*²`: `U + ½ − U` is exact.
        let floor = d.descriptor.V_0 + R::of(0.5);
        for e in [
            None,
            Some(R::of(e0 + 0.5)),
            Some(R::of(u - 1.0)),
            Some(floor),
        ] {
            for l in [None, Some(0.0), Some(l0 + 0.3), Some(40.0), Some(1.0)] {
                for forbids in [false, true] {
                    for class in [LzHold::Free, LzHold::Axis, LzHold::Construction] {
                        let (want, want_outcome) = selected(d, e, forbids, l.map(R::of), class);
                        let (got, outcome) =
                            apply_shared(d, e, forbids, l.map(|l| R::of(l * scale)), class);
                        seen.insert(format!("{outcome:?}"));
                        let (got, want) = (printed(&got, outcome), printed(&want, want_outcome));
                        if got != want {
                            count += 1;
                            first.get_or_insert(format!(
                                "{e:?} {l:?} {forbids} {class:?}: {got} against {want}"
                            ));
                        }
                    }
                }
            }
        }
    }
    (count, first, seen)
}

/// The step against the reference at both precisions, the step's `L*` nudged where `nudge`.
fn check(nudge: bool) {
    let (n64, first64, seen64) = differ::<f64>(nudge);
    let (n32, first32, seen32) = differ::<f32>(nudge);
    // The nudge takes the floor rows off the floor, so the census is the unnudged set's.
    for seen in [&seen64, &seen32].into_iter().filter(|_| !nudge) {
        for want in [
            "Off",
            "Scaled",
            "Infeasible",
            "Unscalable",
            "Held",
            "BelowFloor",
            "OnFloor",
        ] {
            assert!(
                seen.contains(want),
                "the set never reaches {want}: {seen:?}"
            );
        }
    }
    assert!(
        n64 == 0 && n32 == 0,
        "the held decodes' bits moved{}: {n64} cases at f64, {n32} at f32; first {:?}",
        if nudge { " under the L* nudge" } else { "" },
        first64.or(first32)
    );
}

#[test]
fn hold_stage_bits_fixed() {
    check(false);
}

negative_control!(
    hold_stage_bits_fixed,
    "a 2⁻²⁰ relative change in L* moves the step's bits",
    expected = "the held decodes' bits moved under the L* nudge",
    check(true)
);
