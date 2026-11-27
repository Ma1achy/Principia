//! The decode stage's hold step (`kernel::decode::hold::apply_shared`) on a fixed set of decodes, at f64 and f32: its
//! outputs' bits, digested, equal those the step gave when it computed both arms and selected
//! (`hold_stage_bits_fixed`), so running the hold only where the switch is on changed no result.
//!
//! **The set**: 24 seeded states, each with four velocities (moving, near rigid, exactly rigid, rest) and one with two
//! bodies coincident (the collision pole), each untagged and tagged; under every `E*` row (off, above `E₀`, below `U`, `U + ½`),
//! every `L*` row (off, 0, near the state's own, far above the floor, 1) and every chart flag (`E` forbidden or not; `L_z`
//! free, an axis, or held by construction). The census must reach every outcome but `AtTarget`, which needs `E* = E₀`
//! exactly.
//!
//! The momenta enter the digest as bits; the descriptor and the tag by their printed forms, which hide a NaN's sign and
//! payload, but the descriptor is formed from those momenta by code the change left alone.
//!
//! **Pass criterion, written before the run**: each precision's digest equals the one recorded from the step before
//! the change, exactly. The control scales `L*` by `1 + 2⁻²⁰` and must move the digest.

mod decode_support;
mod hold_support;

use std::collections::BTreeSet;

use decode_support::P;
use hold_support::*;
use kernel::chart::invariant::WarpReal;
use kernel::chart::{ChartDecode, Latent, LzHold};
use kernel::decode::descriptor::describe_decoded;
use kernel::decode::hold::apply_shared;
use kernel::decode::invariant::ExactSign;
use kernel::decode::{Decoded, Degenerate, Ic};
use kernel::generated::links::Defaults;
use validation::negative_control;

/// The digests of the step's outputs over the set, recorded from the step that computed both arms and selected.
const F64_DIGEST: u64 = 0xd392_efcf_4095_7c54;
const F32_DIGEST: u64 = 0xc19a_e0a6_67cd_9728;

/// FNV-1a over `bytes`, continuing from `h`.
fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
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

/// The digest of the step over the set at `R`, every `L*` scaled by `1 + 2⁻²⁰` where `nudge`, and the outcomes it
/// reached.
fn digest<R>(nudge: bool) -> (u64, BTreeSet<String>)
where
    R: P + WarpReal + ExactSign,
    Latent<R>: ChartDecode<Param = R>,
{
    let scale = if nudge { 1.0 + 1.0 / 1_048_576.0 } else { 1.0 };
    let mut h = 0xCBF2_9CE4_8422_2325;
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
                        let l = l.map(|l| R::of(l * scale));
                        let (out, outcome) = apply_shared(d, e, forbids, l, class);
                        seen.insert(format!("{outcome:?}"));
                        let text = format!(
                            "{:?}|{:?}|{:?}|{outcome:?}",
                            out.ic.p.map(|b| b.map(|x| x.bits())),
                            out.descriptor,
                            out.degenerate
                        );
                        h = fnv(h, text.as_bytes());
                    }
                }
            }
        }
    }
    (h, seen)
}

/// The step's digest at both precisions, against the recorded ones, `L*` nudged where `nudge`.
fn check(nudge: bool) {
    let (d64, seen64) = digest::<f64>(nudge);
    let (d32, seen32) = digest::<f32>(nudge);
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
    assert_eq!(
        (d64, d32),
        (F64_DIGEST, F32_DIGEST),
        "the held decodes' bits moved: digests {d64:#x} (f64), {d32:#x} (f32)"
    );
}

#[test]
fn hold_stage_bits_fixed() {
    check(false);
}

negative_control!(
    hold_stage_bits_fixed,
    "a 2⁻²⁰ relative change in L* moves the digest",
    expected = "the held decodes' bits moved",
    check(true)
);
