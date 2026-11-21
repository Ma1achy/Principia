//! The NaN scanner (payload §1, "Storage never holds NaN"): every stored float of a payload buffer read for a NaN bit
//! pattern, for the tests that hold storage to that rule (`tests/failure.rs`).
//!
//! **Every stored float** is each f32 (or f64) member of a `SimState`, the phase state, the shadow, the accumulators,
//! the drift references, `closure_min` and the lagged register, and each f16 half a `u32` packs: `packed_a`'s `d_min`
//! and `packed_b`'s `dE_max` and `dLz_max`. A NaN is the exponent field all ones with a non-zero fraction, read on the
//! bits, so every NaN payload and sign is one; `±∞` is not a NaN (`d_min`'s and `closure_min`'s unset value is `+∞`).
//!
//! **Complete by construction.** Each struct is read by a destructuring that names every member and has no `..`, so a
//! member added to the generated layout fails to compile here until it is read as a float or named as not one.

#![allow(dead_code)]

use kernel::payload::{PayloadReal, SimStateBaseOf, SimStateFTLEOf};

/// A stored float the scanner found NaN: the sample's index in the buffer and the member's name, with its component
/// where it has several (`r[1][0]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nan {
    /// The sample's index in the buffer.
    pub index: usize,
    /// The member, as `r[1][0]`, `S` or `packed_b.dE_max`.
    pub field: String,
}

/// A payload struct whose stored floats the scanner reads.
pub trait Scan {
    /// Calls `f` with each stored float's name and whether it is NaN, every one, in layout order.
    fn floats(&self, f: &mut dyn FnMut(String, bool));
}

/// Whether the binary16 `h` is NaN: exponent field all ones, fraction non-zero.
#[inline]
pub fn f16_nan(h: u32) -> bool {
    (h & 0x7C00) == 0x7C00 && (h & 0x03FF) != 0
}

/// The NaNs of `buffer`, every one, in order of sample and then layout.
pub fn nans<S: Scan>(buffer: &[S]) -> Vec<Nan> {
    let mut found = Vec::new();
    for (index, sample) in buffer.iter().enumerate() {
        sample.floats(&mut |field, nan| {
            if nan {
                found.push(Nan { index, field });
            }
        });
    }
    found
}

/// How many stored floats each sample of `S` has, read on `sample`.
pub fn count<S: Scan>(sample: &S) -> usize {
    let mut n = 0;
    sample.floats(&mut |_, _| n += 1);
    n
}

/// The six vectors' twelve components of a state, named `name[i][j]`.
fn vectors<R: PayloadReal>(name: &str, v: &[[R; 2]; 3], f: &mut dyn FnMut(String, bool)) {
    for (i, x) in v.iter().enumerate() {
        for (j, c) in x.iter().enumerate() {
            f(format!("{name}[{i}][{j}]"), c.nan_bits());
        }
    }
}

/// The 16-bit halves the packed words hold: `d_min` binary16, the drifts `log16` (payload §2; R-605, R-632), each read
/// back to f32 and tested for NaN on its bits.
fn halves(packed_a: u32, packed_b: u32, de_reg: u32, f: &mut dyn FnMut(String, bool)) {
    let log16 = |h: u32| kernel::payload::log16_to_f32(h as u16).is_nan();
    f("packed_a.d_min".into(), f16_nan(packed_a >> 16));
    f("packed_b.dE_max".into(), log16(packed_b & 0xFFFF));
    f("packed_b.dLz_max".into(), log16(packed_b >> 16));
    f("dE_reg.dE_reg_final".into(), log16(de_reg & 0xFFFF));
    f("dE_reg.dE_reg_max".into(), log16(de_reg >> 16));
}

impl<R: PayloadReal> Scan for SimStateBaseOf<R> {
    fn floats(&self, f: &mut dyn FnMut(String, bool)) {
        let SimStateBaseOf {
            r,
            p,
            S,
            theta,
            mean_y,
            C_ty,
            E_0,
            Lz_0,
            packed_a,
            packed_b,
            times: _,
            total_substeps: _,
            budget_bits: _,
            dE_reg,
            closure_min,
            closure_step: _,
            _reserved: _,
            coll_floor_bits: _,
            _nhat_lag_0,
            _nhat_lag_1,
            _nhat_lag_2,
            // u32s, no float: the scan reads floats and the f16 halves only
            _dmin_d2: _,
            _march_bits: _,
            _tail: _,
        } = *self;
        vectors("r", &r, f);
        vectors("p", &p, f);
        for (name, x) in [
            ("S", S),
            ("theta", theta),
            ("mean_y", mean_y),
            ("C_ty", C_ty),
            ("E_0", E_0),
            ("Lz_0", Lz_0),
            ("closure_min", closure_min),
            ("_nhat_lag_0", _nhat_lag_0),
            ("_nhat_lag_1", _nhat_lag_1),
            ("_nhat_lag_2", _nhat_lag_2),
        ] {
            f(name.into(), x.nan_bits());
        }
        halves(packed_a, packed_b, dE_reg, f);
    }
}

impl<R: PayloadReal> Scan for SimStateFTLEOf<R> {
    fn floats(&self, f: &mut dyn FnMut(String, bool)) {
        let SimStateFTLEOf {
            r,
            p,
            r_sh,
            p_sh,
            S,
            theta,
            mean_y,
            C_ty,
            E_0,
            Lz_0,
            packed_a,
            packed_b,
            times: _,
            total_substeps: _,
            budget_bits: _,
            dE_reg,
            closure_min,
            closure_step: _,
            _reserved: _,
            coll_floor_bits: _,
            _nhat_lag_0,
            _nhat_lag_1,
            _nhat_lag_2,
            // u32s, no float: the scan reads floats and the f16 halves only
            _dmin_d2: _,
            _march_bits: _,
            _tail: _,
        } = *self;
        vectors("r", &r, f);
        vectors("p", &p, f);
        vectors("r_sh", &r_sh, f);
        vectors("p_sh", &p_sh, f);
        for (name, x) in [
            ("S", S),
            ("theta", theta),
            ("mean_y", mean_y),
            ("C_ty", C_ty),
            ("E_0", E_0),
            ("Lz_0", Lz_0),
            ("closure_min", closure_min),
            ("_nhat_lag_0", _nhat_lag_0),
            ("_nhat_lag_1", _nhat_lag_1),
            ("_nhat_lag_2", _nhat_lag_2),
        ] {
            f(name.into(), x.nan_bits());
        }
        halves(packed_a, packed_b, dE_reg, f);
    }
}
