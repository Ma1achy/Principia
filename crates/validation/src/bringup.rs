//! The bring-up pattern's check (colour_composition Appendix A; REQ-TOOL-013, REQ-TOOL-015, REQ-TOOL-123), shared by
//! the kernel's, the engine's and validation's tests. It is written from Appendix A's definition, not from
//! `kernel::bringup`'s writer, so a fault in the writer's arithmetic fails it. A stored `SimState` is read through the
//! generated unpack, `sim_state_from_ftle` (lowering Part 3a), each field compared with the pattern; and its raw words
//! too, `packed_a` by `roundtrip_ctl`, so a contaminated bit that the unpack masks off fails (pitfalls §9).

use kernel::payload::roundtrip::{roundtrip_ctl, PackedA};
use kernel::payload::{
    canonical_nan, pa_d_min_is_unset, sim_state_from_ftle, ReadParams, SimStateFTLE,
    SimStateFTLEOf, FGW_UNBOUND,
};

/// Appendix A's value of real slot `k` of sample `i`, `32·i + k`, as f64 (exact).
fn slot(i: u32, k: u32) -> f64 {
    32.0 * f64::from(i) + f64::from(k)
}

/// `Err` naming sample `i`'s field `name` unless `got == want`.
fn same<T: PartialEq + std::fmt::Debug>(i: u32, name: &str, got: T, want: T) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!(
            "sample {i}: `{name}` is {got:?}, not the bring-up pattern's {want:?}"
        ))
    }
}

/// The three 2-vectors `v` against slots `k0 .. k0 + 6`, `[b][c]` at `k0 + 2b + c`.
fn vectors<R: Copy + Into<f64>>(i: u32, name: &str, v: [[R; 2]; 3], k0: u32) -> Result<(), String> {
    for b in 0..3u32 {
        for c in 0..2u32 {
            let got: f64 = v[b as usize][c as usize].into();
            same(
                i,
                &format!("{name}[{b}][{c}]"),
                got,
                slot(i, k0 + 2 * b + c),
            )?;
        }
    }
    Ok(())
}

/// `Ok` if `s` is sample `i`'s f32 `SimState` under the bring-up mode, read through the generated unpack and by its
/// raw words; otherwise the first field that differs.
pub fn check(i: u32, s: &SimStateFTLE) -> Result<(), String> {
    let j = i & 0xffff;
    let third = 1.0 / 3.0;
    let params = ReadParams {
        dt_macro: 1.0,
        delta_0: 1.0,
        n_renorm: 0,
        horizon_steps: 0xffff,
    };
    let read = sim_state_from_ftle(
        s,
        FGW_UNBOUND,
        false,
        canonical_nan(),
        false,
        [third; 3],
        &params,
    );
    vectors(i, "r", read.r, 0)?;
    vectors(i, "p", read.p, 6)?;
    vectors(i, "r_sh", s.r_sh, 12)?;
    vectors(i, "p_sh", s.p_sh, 18)?;
    let scalars = [
        ("S", read.S, 24),
        ("theta", read.theta, 25),
        ("mean_y", read.mean_y, 26),
        ("C_ty", read.C_ty, 27),
        ("E_0", read.E_0, 28),
        ("Lz_0", read.Lz_0, 29),
        ("closure_min", read.closure_min, 30),
    ];
    for (name, got, k) in scalars {
        same(i, name, f64::from(got), slot(i, k))?;
    }
    same(i, "state", read.state, i % 6)?;
    same(i, "detail", read.detail, (i / 2) % 4)?;
    same(i, "saturated", read.saturated, (i / 8) % 2 == 1)?;
    same(i, "dmin_pair", read.dmin_pair, (i / 16) % 4)?;
    same(i, "last_symbol", read.last_symbol, (i / 64) % 4)?;
    same(i, "d_min unset", pa_d_min_is_unset(s.packed_a), true)?;
    same(i, "dE_max bits", read.dE_max.to_bits(), 0)?;
    same(i, "dLz_max bits", read.dLz_max.to_bits(), 0)?;
    same(i, "t_end_step", read.t_end_step, j)?;
    same(i, "t_dmin_step", read.t_dmin_step, 0xffff - j)?;
    same(i, "total_substeps", read.total_substeps, i)?;
    same(i, "closure_step", read.closure_step, j)?;
    let expected = PackedA {
        state: i % 6,
        detail: (i / 2) % 4,
        saturated: (i / 8) % 2 == 1,
        dmin_pair: (i / 16) % 4,
        last_symbol: (i / 64) % 4,
        d_min: f32::INFINITY,
    };
    same(
        i,
        "packed_a's raw word matches",
        roundtrip_ctl(&expected, s.packed_a),
        true,
    )?;
    same(i, "packed_b's raw word", s.packed_b, 0)?;
    same(i, "times' raw word", s.times, j | ((0xffff - j) << 16))?;
    same(i, "_reserved", s._reserved, 0)
}

/// `s` at f64 narrowed to the f32 payload, member by member: exact for the pattern, whose every real is an integer
/// below 2²⁴.
pub fn narrow(s: &SimStateFTLEOf<f64>) -> SimStateFTLE {
    let v = |a: [[f64; 2]; 3]| a.map(|p| p.map(|x| x as f32));
    SimStateFTLE {
        r: v(s.r),
        p: v(s.p),
        r_sh: v(s.r_sh),
        p_sh: v(s.p_sh),
        S: s.S as f32,
        theta: s.theta as f32,
        mean_y: s.mean_y as f32,
        C_ty: s.C_ty as f32,
        E_0: s.E_0 as f32,
        Lz_0: s.Lz_0 as f32,
        packed_a: s.packed_a,
        packed_b: s.packed_b,
        times: s.times,
        total_substeps: s.total_substeps,
        closure_min: s.closure_min as f32,
        closure_step: s.closure_step,
        _reserved: s._reserved,
        _tail: [],
    }
}

/// `Ok` if `s` is sample `i`'s f64 `SimState` under the bring-up mode: each real slot exactly its value at f64, and
/// the whole, narrowed to the f32 payload, through [`check`].
pub fn check_f64(i: u32, s: &SimStateFTLEOf<f64>) -> Result<(), String> {
    vectors(i, "r", s.r, 0)?;
    vectors(i, "p", s.p, 6)?;
    vectors(i, "r_sh", s.r_sh, 12)?;
    vectors(i, "p_sh", s.p_sh, 18)?;
    let scalars = [
        ("S", s.S, 24),
        ("theta", s.theta, 25),
        ("mean_y", s.mean_y, 26),
        ("C_ty", s.C_ty, 27),
        ("E_0", s.E_0, 28),
        ("Lz_0", s.Lz_0, 29),
        ("closure_min", s.closure_min, 30),
    ];
    for (name, got, k) in scalars {
        same(i, name, got, slot(i, k))?;
    }
    check(i, &narrow(s))
}
