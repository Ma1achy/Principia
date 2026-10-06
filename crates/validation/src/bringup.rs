//! The bring-up pattern's check (colour_composition Appendix A; REQ-TOOL-013, REQ-TOOL-015, REQ-TOOL-123), shared by
//! the kernel's, the engine's and validation's tests. It is written from Appendix A's definition, not from
//! `kernel::bringup`'s writer, so a fault in the writer's arithmetic fails it. A stored `SimState` is read through the
//! generated unpack, `sim_state_from_ftle` (lowering Part 3a), each field compared with the pattern; and `packed_a`'s
//! raw word too, by `roundtrip_ctl`, so a contaminated reserved bit that the unpack masks off fails (pitfalls §9).
//! `packed_b`'s and `times`' fields cover all their bits, and `_reserved`, which the unpack does not read, is compared
//! as stored.

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
        // The masses enter only `energy_drift`, which the pattern does not define and the check does not read.
        [1.0; 3],
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
    // Appendix A's packed_a, its fields compared one by one and its packed word with the stored one.
    let expected = PackedA {
        state: i % 6,
        detail: (i / 2) % 4,
        saturated: (i / 8) % 2 == 1,
        dmin_pair: (i / 16) % 4,
        last_symbol: (i / 64) % 4,
        d_min: f32::INFINITY,
    };
    same(i, "state", read.state, expected.state)?;
    same(i, "detail", read.detail, expected.detail)?;
    same(i, "saturated", read.saturated, expected.saturated)?;
    same(i, "dmin_pair", read.dmin_pair, expected.dmin_pair)?;
    same(i, "last_symbol", read.last_symbol, expected.last_symbol)?;
    same(i, "d_min unset", pa_d_min_is_unset(s.packed_a), true)?;
    same(i, "dE_max bits", read.dE_max.to_bits(), 0)?;
    same(i, "dLz_max bits", read.dLz_max.to_bits(), 0)?;
    same(i, "t_end_step", read.t_end_step, j)?;
    same(i, "t_dmin_step", read.t_dmin_step, 0xffff - j)?;
    same(i, "total_substeps", read.total_substeps, i)?;
    same(i, "closure_step", read.closure_step, j)?;
    same(
        i,
        "packed_a's raw word matches",
        roundtrip_ctl(&expected, s.packed_a),
        true,
    )?;
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

#[cfg(test)]
mod tests {
    use kernel::bringup::pattern;
    use kernel::payload::{set_detail, set_dmin_pair, set_last_symbol, set_saturated, set_state};

    use super::*;

    /// A check of sample `i`: [`check`], or a control's lenient one.
    type Check = fn(u32, &SimStateFTLE) -> Result<(), String>;

    /// A corruption of an f64 `SimState`.
    type Corrupt64 = fn(&mut SimStateFTLEOf<f64>);

    /// Sample `i`'s f32 pattern with one field corrupted, for each field, with the name the check must report.
    fn corruptions(i: u32) -> Vec<(&'static str, SimStateFTLE)> {
        let p = pattern::<f32>(i);
        let one = |f: &dyn Fn(&mut SimStateFTLE)| {
            let mut s = p;
            f(&mut s);
            s
        };
        vec![
            ("`r[2][1]`", one(&|s| s.r[2][1] += 1.0)),
            ("`p[1][0]`", one(&|s| s.p[1][0] += 1.0)),
            ("`r_sh[0][1]`", one(&|s| s.r_sh[0][1] += 1.0)),
            ("`p_sh[2][0]`", one(&|s| s.p_sh[2][0] += 1.0)),
            ("`S`", one(&|s| s.S += 1.0)),
            ("`theta`", one(&|s| s.theta += 1.0)),
            ("`mean_y`", one(&|s| s.mean_y += 1.0)),
            ("`C_ty`", one(&|s| s.C_ty += 1.0)),
            ("`E_0`", one(&|s| s.E_0 += 1.0)),
            ("`Lz_0`", one(&|s| s.Lz_0 += 1.0)),
            ("`closure_min`", one(&|s| s.closure_min += 1.0)),
            (
                "`state`",
                one(&|s| s.packed_a = set_state(s.packed_a, (i + 1) % 6)),
            ),
            (
                "`detail`",
                one(&|s| s.packed_a = set_detail(s.packed_a, (i / 2 + 1) % 4)),
            ),
            (
                "`saturated`",
                one(&|s| s.packed_a = set_saturated(s.packed_a, (i / 8).is_multiple_of(2))),
            ),
            (
                "`dmin_pair`",
                one(&|s| s.packed_a = set_dmin_pair(s.packed_a, (i / 16 + 1) % 4)),
            ),
            (
                "`last_symbol`",
                one(&|s| s.packed_a = set_last_symbol(s.packed_a, (i / 64 + 1) % 4)),
            ),
            (
                "`d_min unset`",
                one(&|s| s.packed_a = (s.packed_a & 0xffff) | (0x3c00 << 16)),
            ),
            ("`dE_max bits`", one(&|s| s.packed_b ^= 1)),
            ("`dLz_max bits`", one(&|s| s.packed_b ^= 1 << 16)),
            ("`t_end_step`", one(&|s| s.times ^= 1)),
            ("`t_dmin_step`", one(&|s| s.times ^= 1 << 16)),
            ("`total_substeps`", one(&|s| s.total_substeps ^= 1)),
            ("`closure_step`", one(&|s| s.closure_step ^= 1)),
            (
                "`packed_a's raw word matches`",
                one(&|s| s.packed_a |= 1 << 12),
            ),
            ("`_reserved`", one(&|s| s._reserved = 1)),
        ]
    }

    /// `check` passes each sample's pattern, at f32 and at f64, and fails each corruption naming its field; and
    /// [`check_f64`] fails each f64 real slot moved by less than f32 can hold, which narrowing would hide.
    fn check_names_each_field(check: Check) {
        for i in [0, 1, 77, 65_537, (1 << 19) - 1] {
            check(i, &pattern::<f32>(i)).unwrap_or_else(|e| panic!("{e}"));
            check_f64(i, &pattern::<f64>(i)).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                narrow(&pattern::<f64>(i)),
                pattern::<f32>(i),
                "narrow is not exact on the pattern"
            );
            for (name, s) in corruptions(i) {
                match check(i, &s) {
                    Err(e) if e.contains(name) => {}
                    other => panic!("sample {i}: a corrupted {name} reads {other:?}"),
                }
            }
            let f64_cases: [(&str, Corrupt64); 11] = [
                ("`r[0][0]`", |s| s.r[0][0] += 1e-6),
                ("`p[2][1]`", |s| s.p[2][1] += 1e-6),
                ("`r_sh[1][1]`", |s| s.r_sh[1][1] += 1e-6),
                ("`p_sh[0][0]`", |s| s.p_sh[0][0] += 1e-6),
                ("`S`", |s| s.S += 1e-6),
                ("`theta`", |s| s.theta += 1e-6),
                ("`mean_y`", |s| s.mean_y += 1e-6),
                ("`C_ty`", |s| s.C_ty += 1e-6),
                ("`E_0`", |s| s.E_0 += 1e-6),
                ("`Lz_0`", |s| s.Lz_0 += 1e-6),
                ("`closure_min`", |s| s.closure_min += 1e-6),
            ];
            for (name, f) in f64_cases {
                let mut s = pattern::<f64>(i);
                f(&mut s);
                match check_f64(i, &s) {
                    Err(e) if e.contains(name) => {}
                    other => panic!("sample {i}: a corrupted f64 {name} reads {other:?}"),
                }
            }
        }
    }

    #[test]
    fn bringup_check_names_each_field() {
        check_names_each_field(check);
    }

    crate::negative_control!(
        bringup_check_names_each_field,
        "a check that accepts every SimState must fail",
        expected = "a corrupted",
        check_names_each_field(|_, _| Ok(()))
    );
}
