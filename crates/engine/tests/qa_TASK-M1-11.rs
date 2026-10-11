//! QA tests for TASK-M1-11's engine side, written from REQ-TOOL-015 (colour_composition Appendix A; R-75; RQ-221):
//! enabling the bring-up variant writes the known pattern into the payload, read back through the generated unpack, and
//! it changes the sim key; and from REQ-TOOL-013: UV, DECODE and ROUNDTRIP are not kernel variants. The pattern is
//! REQ-TOOL-123's, transcribed here from Appendix A's table, not from the writer. Each test has its negative control
//! (R-176).
// The file name `qa_TASK-M1-11` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::bringup::{native, samples};
use engine::contract::canonical;
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice,
};
use engine::synthetic::{simstate_words, Synthetic};
use kernel::bringup::{words_per_sample, write_words};
use kernel::payload::{
    canonical_nan, sim_state_from_ftle, ReadParams, SimStateFTLE, SimStateFTLEOf, FGW_UNBOUND,
};
use render::raster::Grid;
use validation::negative_control;

// ----- Appendix A's pattern, from its table -----

/// Sample `i`'s pattern at f32, built member by member from Appendix A's table: real slot `k` (ledger member order,
/// `r[b][c]` at `2b + c`) holds `32·i + k`; `packed_a` by the payload's §2 bits (`state` 0–2 = `i mod 6`, `detail`
/// 3–4 = `⌊i/2⌋ mod 4`, `saturated` 5 = `⌊i/8⌋ mod 2`, `dmin_pair` 6–7 = `⌊i/16⌋ mod 4`, `last_symbol` 8–9 =
/// `⌊i/64⌋ mod 4`, reserved 10–15 zero, `d_min` 16–31 unset, f16 +∞ = `0x7c00`, R-271); `packed_b = 0`; `times`
/// `t_end_step = j` low, `t_dmin_step = 65535 − j` high, `j = i mod 2¹⁶`; `total_substeps = i`; `closure_step = j`;
/// `_reserved = 0`.
fn appendix_a(i: u32) -> SimStateFTLE {
    let slot = |k: u32| (32 * i + k) as f32;
    let vectors = |k0: u32| {
        [
            [slot(k0), slot(k0 + 1)],
            [slot(k0 + 2), slot(k0 + 3)],
            [slot(k0 + 4), slot(k0 + 5)],
        ]
    };
    let j = i % 65536;
    SimStateFTLE {
        r: vectors(0),
        p: vectors(6),
        r_sh: vectors(12),
        p_sh: vectors(18),
        S: slot(24),
        theta: slot(25),
        mean_y: slot(26),
        C_ty: slot(27),
        E_0: slot(28),
        Lz_0: slot(29),
        packed_a: (i % 6)
            | ((i / 2 % 4) << 3)
            | ((i / 8 % 2) << 5)
            | ((i / 16 % 4) << 6)
            | ((i / 64 % 4) << 8)
            | (0x7c00 << 16),
        packed_b: 0,
        times: j | ((65535 - j) << 16),
        total_substeps: i,
        closure_min: slot(30),
        closure_step: j as u16,
        _reserved: 0,
        ..SimStateFTLE::default()
    }
}

/// `s` at f64 narrowed to the f32 payload (exact: every pattern value is an integer below 2²⁴), and `Err` unless each
/// real slot was exactly that integer at f64 too.
fn narrow(s: &SimStateFTLEOf<f64>) -> Result<SimStateFTLE, String> {
    let exact = |x: f64| {
        let y = x as f32;
        if f64::from(y) == x {
            Ok(y)
        } else {
            Err(format!("{x} is not an integer exact in f32"))
        }
    };
    let v = |a: [[f64; 2]; 3]| -> Result<[[f32; 2]; 3], String> {
        Ok([
            [exact(a[0][0])?, exact(a[0][1])?],
            [exact(a[1][0])?, exact(a[1][1])?],
            [exact(a[2][0])?, exact(a[2][1])?],
        ])
    };
    Ok(SimStateFTLE {
        r: v(s.r)?,
        p: v(s.p)?,
        r_sh: v(s.r_sh)?,
        p_sh: v(s.p_sh)?,
        S: exact(s.S)?,
        theta: exact(s.theta)?,
        mean_y: exact(s.mean_y)?,
        C_ty: exact(s.C_ty)?,
        E_0: exact(s.E_0)?,
        Lz_0: exact(s.Lz_0)?,
        packed_a: s.packed_a,
        packed_b: s.packed_b,
        times: s.times,
        total_substeps: s.total_substeps,
        closure_min: exact(s.closure_min)?,
        closure_step: s.closure_step,
        _reserved: s._reserved,
        ..SimStateFTLE::default()
    })
}

/// `got`, sample `i` as stored, equals Appendix A's, member by member, and read through the generated unpack
/// (lowering Part 3a) decodes to the same fields as Appendix A's state does.
fn check_sample(i: u32, got: &SimStateFTLE) {
    let want = appendix_a(i);
    assert!(
        got == &want,
        "sample {i}: stored {got:?}, not Appendix A's pattern {want:?}"
    );
    let params = ReadParams {
        dt_macro: 1.0,
        delta_0: 1.0,
        n_renorm: 0,
        horizon_steps: 65535,
    };
    let read = |s: &SimStateFTLE| {
        sim_state_from_ftle(
            s,
            FGW_UNBOUND,
            false,
            canonical_nan(),
            false,
            [1.0 / 3.0; 3],
            &params,
        )
    };
    let (g, w) = (read(got), read(&want));
    let j = i % 65536;
    let fields = [
        ("state", g.state, i % 6),
        ("detail", g.detail, i / 2 % 4),
        ("saturated", u32::from(g.saturated), i / 8 % 2),
        ("dmin_pair", g.dmin_pair, i / 16 % 4),
        ("last_symbol", g.last_symbol, i / 64 % 4),
        ("t_end_step", g.t_end_step, j),
        ("t_dmin_step", g.t_dmin_step, 65535 - j),
        ("total_substeps", g.total_substeps, i),
        ("closure_step", g.closure_step, j),
    ];
    for (name, got, want) in fields {
        assert!(
            got == want,
            "sample {i}: decoded `{name}` is {got}, not Appendix A's {want}"
        );
    }
    assert!(
        g.d_min == f32::INFINITY && g.dE_max.to_bits() == 0 && g.dLz_max.to_bits() == 0,
        "sample {i}: decoded latches d_min {}, dE_max {}, dLz_max {}, not unset, +0, +0",
        g.d_min,
        g.dE_max,
        g.dLz_max
    );
    assert!(
        g.r == w.r
            && g.p == w.p
            && [g.S, g.theta, g.mean_y, g.C_ty, g.E_0, g.Lz_0, g.closure_min]
                == [w.S, w.theta, w.mean_y, w.C_ty, w.E_0, w.Lz_0, w.closure_min],
        "sample {i}: the decoded reals are not Appendix A's"
    );
}

/// The flat layout with `quads` quads of 8 × 8 tiles, no ensemble copies.
fn flat(quads: [u32; 2]) -> Grid {
    Synthetic::flat(Grid::new(quads, 8, 0, 2).expect("a grid"), 0).grid()
}

/// 33 × 32 quads of 64 samples: 67 584 samples, past `j`'s wrap at 2¹⁶ and through every packed field's cycle.
fn wide() -> Grid {
    flat([33, 32])
}

/// Every sample of `out` is Appendix A's, and there is one per sample of `grid`.
fn check_native(out: &[SimStateFTLEOf<f64>], grid: Grid) {
    assert_eq!(
        out.len(),
        grid.sample_count() as usize,
        "the native variant wrote {} samples, not the grid's {}",
        out.len(),
        grid.sample_count()
    );
    for (i, s) in out.iter().enumerate() {
        let n = narrow(s).unwrap_or_else(|e| panic!("sample {i}: {e}"));
        check_sample(i as u32, &n);
    }
}

#[test]
fn qa_bringup_pattern_native_is_appendix_a() {
    let grid = wide();
    assert!(
        grid.sample_count() > 65536,
        "the grid does not pass j's wrap"
    );
    let out = native(KernelVariant::BringUp, grid).unwrap_or_else(|e| panic!("{e:?}"));
    check_native(&out, grid);
}

negative_control!(
    qa_bringup_pattern_native_is_appendix_a,
    "a pattern written one sample on must fail (pitfalls §9)",
    expected = "not Appendix A's pattern",
    check_native(
        &native(KernelVariant::BringUp, flat([2, 1])).unwrap()[1..65],
        flat([1, 1])
    )
);

#[cfg(feature = "controls")]
mod native_controls {
    use super::*;

    negative_control!(
        qa_bringup_pattern_native_is_appendix_a,
        "a pattern with two real slots swapped must fail (pitfalls §9)",
        expected = "not Appendix A's pattern",
        check_native(
            &native(KernelVariant::BringUp, flat([1, 1]))
                .unwrap()
                .into_iter()
                .map(|mut s| {
                    std::mem::swap(&mut s.E_0, &mut s.Lz_0);
                    s
                })
                .collect::<Vec<_>>(),
            flat([1, 1])
        )
    );
}

// ----- The f32 writer, as words in the SimState buffer -----

/// `write_words` for sample `i` into a buffer of `i + 2` samples filled with `0xA5A5A5A5`: its words are Appendix A's
/// state as the ledger lays it out (`simstate_words`), and no word of another sample is touched.
fn check_words(write: fn(u32, &mut [u32]), i: u32) {
    let n = words_per_sample();
    let mut buf = vec![0xA5A5_A5A5u32; (i as usize + 2) * n];
    write(i, &mut buf);
    let base = i as usize * n;
    let want = simstate_words(&appendix_a(i));
    assert_eq!(want.len(), n, "the ledger's SimStateFTLE is not {n} words");
    for (k, w) in want.iter().enumerate() {
        assert!(
            buf[base + k] == *w,
            "sample {i}: word {k} is {:#x}, not Appendix A's {w:#x}",
            buf[base + k]
        );
    }
    for (k, w) in buf.iter().enumerate() {
        if !(base..base + n).contains(&k) {
            assert!(
                *w == 0xA5A5_A5A5,
                "sample {i}: word {k}, outside its sample, was written"
            );
        }
    }
}

#[test]
fn qa_bringup_pattern_f32_words() {
    for i in [0, 1, 7, 63, 64, 255, 65535, 65536, 65537] {
        check_words(write_words, i);
    }
}

negative_control!(
    qa_bringup_pattern_f32_words,
    "the f32 writer one sample on must fail",
    expected = "word",
    check_words(|i, out| write_words(i + 1, out), 5)
);

// ----- The sim key (RQ-221; caching contract Parts 1–2) -----

/// The sim key with the kernel variant `v`, every other group as at M0.
fn config(v: KernelVariant) -> SimConfig {
    SimConfig {
        chart: Chart {},
        plane: Plane {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
        slice: Slice {},
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links {},
        integrator: Integrator {},
        kernel_variant: v,
        horizon: Horizon {},
        collision: Collision {},
        quality: Quality {},
    }
}

/// The sim key's canonical text with the bring-up mode differs from the physics one's, each reads back to its own
/// variant, and a key naming any other variant, or none, is refused: UV, DECODE and ROUNDTRIP are not kernel
/// variants (R-75).
fn check_sim_key(physics: &str, bring_up: &str) {
    assert!(
        physics != bring_up,
        "enabling the bring-up mode leaves the sim key unchanged: {physics}"
    );
    for (text, v) in [
        (physics, KernelVariant::Physics),
        (bring_up, KernelVariant::BringUp),
    ] {
        let back: SimConfig =
            serde_json::from_str(text).unwrap_or_else(|e| panic!("{text} does not read back: {e}"));
        assert_eq!(back, config(v), "{text} reads back another sim key");
    }
    for bad in ["uv", "decode", "roundtrip", "round_trip", "debug_mode", ""] {
        let text = physics.replace("\"physics\"", &format!("\"{bad}\""));
        assert!(
            serde_json::from_str::<SimConfig>(&text).is_err(),
            "the sim key accepts the kernel variant `{bad}`"
        );
    }
    let none = physics.replace(",\"kernel_variant\":\"physics\"", "");
    assert!(
        none != physics && serde_json::from_str::<SimConfig>(&none).is_err(),
        "the sim key reads with no kernel variant: {none}"
    );
}

#[test]
fn qa_bringup_pattern_changes_the_sim_key() {
    let physics = canonical::to_string(&config(KernelVariant::Physics)).unwrap();
    let bring_up = canonical::to_string(&config(KernelVariant::BringUp)).unwrap();
    check_sim_key(&physics, &bring_up);
}

negative_control!(
    qa_bringup_pattern_changes_the_sim_key,
    "a bring-up mode left off the sim key must fail",
    expected = "leaves the sim key unchanged",
    check_sim_key(
        &canonical::to_string(&config(KernelVariant::Physics)).unwrap(),
        &canonical::to_string(&config(KernelVariant::Physics)).unwrap()
    )
);

// ----- What the M1 dispatch refuses -----

/// The physics kernel is not built at M1, so its dispatch is refused; the bring-up mode covers exactly 2¹⁹ samples, the
/// most whose every pattern value is exact in f32 (`32·(2¹⁹ − 1) + 30 < 2²⁴`), and refuses one quad more.
fn check_refusals(at_limit: [u32; 2], past: [u32; 2]) {
    assert!(
        native(KernelVariant::Physics, flat([1, 1])).is_err(),
        "the physics variant dispatched at M1"
    );
    assert_eq!(
        samples(KernelVariant::BringUp, flat(at_limit)).ok(),
        Some(1 << 19),
        "the bring-up mode does not cover 2¹⁹ samples"
    );
    assert!(
        samples(KernelVariant::BringUp, flat(past)).is_err(),
        "the bring-up mode covers {} samples, past 2¹⁹",
        flat(past).sample_count()
    );
}

#[test]
fn qa_bringup_pattern_refusals() {
    check_refusals([128, 64], [8193, 1]);
}

negative_control!(
    qa_bringup_pattern_refusals,
    "a grid one quad past 2¹⁹ samples must be refused",
    expected = "past 2¹⁹",
    check_refusals([128, 64], [128, 64])
);
