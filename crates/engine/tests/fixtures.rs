//! The named synthetic fixture set's accessor tests (`engine::fixtures`; debug tooling plan, "Principle"; TASK-M1-14),
//! paired with its renders, the golden suite `m1-synthetic`:
//! - REQ-TOOL-018: each fixture's samples read back through the generated accessors as the values written, the
//!   descriptor's fields, `d_min`'s bits, the step count, the word and the current energy drift
//!   (`fixtures_accessors_*`);
//! - PIT-9: the adversarial descriptor's raw words are compared whole, so a contaminated reserved bit, which every
//!   accessor masks, fails the check (`fixtures_adversarial_*`);
//! - REQ-PAY-024: no stored value of the fixtures is NaN, and the only non-finite one is `d_min`'s unset f16 `+inf`,
//!   `0x7C00` in bits 16–31 of `packed_a`, held only by a failed sample or one before its first step (R-79, R-271;
//!   `stored_values_no_nan`).
//!
//! Each test registers its negative control (R-176).

use engine::fixtures::{self, Known, NAMES, UNSET};
use engine::synthetic::Synthetic;
use kernel::payload::roundtrip::{roundtrip_ctl, PackedA};
use kernel::payload::{
    canonical_nan, f16_bits_to_f32, fgw_length_raw, pb_dE_max, pb_dLz_max, sd_detail, sd_dmin_pair,
    sd_last_symbol, sd_saturated, sd_state, sim_state_from_ftle, tm_t_dmin_step, tm_t_end_step,
    ReadParams, PACKED_A_RESERVED, STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE,
    STATE_SIM_FAILED,
};
use ledger::gen::rust::offsets;
use ledger::schema::Storage;
use render::raster::Grid;
use validation::negative_control;

/// A row of eight fresh samples, one quad each.
fn row() -> Synthetic {
    Synthetic::flat(Grid::new([8, 1], 1, 0, 8).expect("the row"), 0)
}

/// The fixture `name`, filled into a fresh row.
fn filled(name: &str) -> Synthetic {
    let mut set = row();
    fixtures::fill(name, &mut set).unwrap_or_else(|e| panic!("{e}"));
    set
}

fn known(name: &str) -> Vec<Known> {
    fixtures::known(name).unwrap_or_else(|e| panic!("{e}"))
}

/// Sample `i` of `set` through the kernel's read side, a FULL-tier read with the sample's own masses.
fn read(set: &mut Synthetic, i: u32) -> kernel::payload::SimState {
    let ic = set.ic(i);
    let masses = [ic.m0, ic.m1, ic.m2];
    let params = ReadParams {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
    };
    sim_state_from_ftle(
        set.simstate(i),
        set.word(i),
        true,
        canonical_nan(),
        false,
        masses,
        &params,
    )
}

/// The fields `k` holds, as the descriptor parity check takes them.
fn fields(k: &Known) -> PackedA {
    PackedA {
        state: k.state,
        detail: k.detail,
        saturated: k.saturated,
        dmin_pair: k.dmin_pair,
        last_symbol: k.last_symbol,
        d_min: f16_bits_to_f32(k.d_min),
    }
}

/// Checks that each sample of `set` reads back as `want`'s row through the generated accessors: each descriptor field,
/// `d_min`'s bits, `t_end_step`, the word's length and the current energy drift, within 10⁻⁶.
fn check_accessors(name: &str, set: &mut Synthetic, want: &[Known]) {
    for (i, k) in (0u32..).zip(want) {
        let s = *set.simstate(i);
        let w = s.packed_a;
        let got = (
            sd_state(w),
            sd_detail(w),
            sd_saturated(w),
            sd_dmin_pair(w),
            sd_last_symbol(w),
        );
        let wanted = (k.state, k.detail, k.saturated, k.dmin_pair, k.last_symbol);
        assert_eq!(got, wanted, "`{name}` sample {i}: the descriptor's fields");
        assert_eq!(
            w >> 16,
            u32::from(k.d_min),
            "`{name}` sample {i}: d_min's bits"
        );
        assert_eq!(
            tm_t_end_step(s.times),
            k.t_end_step,
            "`{name}` sample {i}: t_end_step"
        );
        assert_eq!(
            tm_t_dmin_step(s.times),
            if k.state == STATE_COLLISION {
                k.t_end_step
            } else {
                k.t_end_step / 2
            },
            "`{name}` sample {i}: t_dmin_step, a collision's its last step, any other's half of t_end_step"
        );
        let (de_max, dlz_max) = (pb_dE_max(s.packed_b), pb_dLz_max(s.packed_b));
        if k.failed() {
            assert_eq!(
                (de_max, dlz_max),
                (0.0, 0.0),
                "`{name}` sample {i}: a failed sample's drift latches"
            );
        } else {
            assert!(
                de_max >= k.drift.abs() + 0.04 && dlz_max > 0.0,
                "`{name}` sample {i}: the latches {de_max}, {dlz_max} are not above the current drift {}",
                k.drift
            );
        }
        assert_eq!(
            fgw_length_raw(set.word(i)),
            k.length,
            "`{name}` sample {i}: the word's length"
        );
        let drift = read(set, i).energy_drift;
        assert!(
            (drift - k.drift).abs() <= 1e-6,
            "`{name}` sample {i}: the energy drift reads {drift}, not {}",
            k.drift
        );
    }
}

#[test]
fn fixtures_accessors_return_the_known_values() {
    for name in NAMES {
        check_accessors(name, &mut filled(name), &known(name));
    }
}

negative_control!(
    fixtures_accessors_return_the_known_values,
    "the known escaper read as escaping body 2, not body 1, fails the check",
    expected = "the descriptor's fields",
    {
        let mut want = known("known_escaper");
        want[1].detail = 2;
        check_accessors("known_escaper", &mut filled("known_escaper"), &want);
    }
);

/// Checks that the energy-drift fixture's drifts rise in even steps of 0.4 from its first, as its samples read them.
fn check_ramp(set: &mut Synthetic) {
    let drifts: Vec<f32> = (0..8).map(|i| read(set, i).energy_drift).collect();
    for (k, pair) in drifts.windows(2).enumerate() {
        assert!(
            (pair[1] - pair[0] - 0.4).abs() <= 2e-6,
            "the drift does not rise by 0.4 from sample {k}: {drifts:?}"
        );
    }
}

#[test]
fn fixtures_accessors_energy_drift_is_a_ramp() {
    check_ramp(&mut filled("energy_drift_ramp"));
}

negative_control!(
    fixtures_accessors_energy_drift_is_a_ramp,
    "the set state's samples, which hold no drift, are no ramp",
    expected = "does not rise by 0.4",
    check_ramp(&mut filled("set_state"))
);

/// Checks that the energy-drift fixture's drifts run from negative to positive, so the ramp reads both signs of the
/// drift.
fn check_crosses_zero(set: &mut Synthetic) {
    let drifts: Vec<f32> = (0..8).map(|i| read(set, i).energy_drift).collect();
    assert!(
        drifts[0] < 0.0 && drifts[7] > 0.0,
        "the drift does not cross zero: {drifts:?}"
    );
}

#[test]
fn fixtures_accessors_energy_drift_crosses_zero() {
    check_crosses_zero(&mut filled("energy_drift_ramp"));
}

negative_control!(
    fixtures_accessors_energy_drift_crosses_zero,
    "the known escaper's samples, which hold no drift, do not cross zero",
    expected = "does not cross zero",
    check_crosses_zero(&mut filled("known_escaper"))
);

/// The binary collisions of `rows`, by sample index. The adversarial descriptor, a bit pattern drawn only on the
/// sub-field views, is not a physical sample and is left out.
fn binary_collisions(rows: &[Known]) -> impl Iterator<Item = (u32, &Known)> {
    (0u32..)
        .zip(rows)
        .filter(|(_, k)| k.state == STATE_COLLISION && k.detail != 3 && k.descriptor.is_none())
}

/// Checks that each binary collision of `rows` is one the detector could have fired: its `d_min` below the default
/// `r_coll` = 10⁻³ (integrator contract Part 7), being the separation that crossed it (dd_integrator §3.6).
fn check_collision_d_min(name: &str, rows: &[Known]) {
    for (i, k) in binary_collisions(rows) {
        let d = f16_bits_to_f32(k.d_min);
        assert!(
            d < 1e-3,
            "`{name}` sample {i}: a binary collision's d_min {d} is not below r_coll = 1e-3"
        );
    }
}

#[test]
fn fixtures_collisions_d_min_below_r_coll() {
    let mut seen = 0;
    for name in NAMES {
        let rows = known(name);
        seen += binary_collisions(&rows).count();
        check_collision_d_min(name, &rows);
    }
    assert_eq!(
        seen, 2,
        "the fixtures' binary collisions are set_state's and known_escaper's"
    );
}

negative_control!(
    fixtures_collisions_d_min_below_r_coll,
    "a binary collision whose d_min, 2.5e-3, is above r_coll fails the check",
    expected = "is not below r_coll",
    {
        let mut rows = known("set_state");
        rows[2].d_min = kernel::payload::f32_to_f16_bits(2.5e-3);
        check_collision_d_min("set_state", &rows);
    }
);

/// Checks that each escape of `rows` outside the adversarial descriptor comes once the settling window could complete:
/// escape needs a full window of history (payload §2, R-29), 0.4 time units, so at least 40 steps at the harness's
/// `dt_macro` 0.01.
fn check_escape_window(name: &str, rows: &[Known]) {
    for (i, k) in (0u32..).zip(rows) {
        if k.state == STATE_ESCAPE && k.descriptor.is_none() {
            assert!(
                k.t_end_step >= 40,
                "`{name}` sample {i}: an escape at step {} comes before the settling window completes",
                k.t_end_step
            );
        }
    }
}

#[test]
fn fixtures_escapes_complete_the_settling_window() {
    for name in NAMES {
        check_escape_window(name, &known(name));
    }
}

negative_control!(
    fixtures_escapes_complete_the_settling_window,
    "an escape at step 39, one short of the window, fails the check",
    expected = "before the settling window completes",
    {
        let mut rows = known("known_escaper");
        rows[4].t_end_step = 39;
        check_escape_window("known_escaper", &rows);
    }
);

/// Checks that each binary collision of `set`, filled from `rows`, has its closest approach at the collision step,
/// `t_dmin_step == t_end_step` (payload §2): no earlier step came within `r_coll`, or the collision would have fired
/// then.
fn check_collision_t_dmin(name: &str, set: &mut Synthetic, rows: &[Known]) {
    for (i, _) in binary_collisions(rows) {
        let times = set.simstate(i).times;
        assert_eq!(
            tm_t_dmin_step(times),
            tm_t_end_step(times),
            "`{name}` sample {i}: a binary collision's closest approach is not its collision step"
        );
    }
}

#[test]
fn fixtures_collisions_closest_at_the_collision_step() {
    for name in NAMES {
        check_collision_t_dmin(name, &mut filled(name), &known(name));
    }
}

negative_control!(
    fixtures_collisions_closest_at_the_collision_step,
    "a binary collision whose closest approach is half its collision step fails the check",
    expected = "is not its collision step",
    {
        let rows = known("known_escaper");
        let mut set = filled("known_escaper");
        set.sample(6).times(40, 20);
        check_collision_t_dmin("known_escaper", &mut set, &rows);
    }
);

/// Checks that `Known::failed` holds exactly for the failed codes, `sim_failed` (4) and `decode_failed` (5), of the
/// eight `state` codes (payload §2), and for none of the reserved 6 and 7.
fn check_failed(failed: &[u32]) {
    let base = known("set_state")[0];
    for state in 0..8 {
        let k = Known { state, ..base };
        assert_eq!(
            k.failed(),
            failed.contains(&state),
            "state {state}: failed() is {}",
            k.failed()
        );
    }
}

#[test]
fn fixtures_known_failed_is_states_4_and_5() {
    check_failed(&[STATE_SIM_FAILED, STATE_DECODE_FAILED]);
}

negative_control!(
    fixtures_known_failed_is_states_4_and_5,
    "a reserved code counted as failed is refused",
    expected = "state 6: failed() is false",
    check_failed(&[STATE_SIM_FAILED, STATE_DECODE_FAILED, 6])
);

/// Checks that every sample of every fixture holds the shared configuration in the CoM frame, positions and momenta
/// each summing to zero within 10⁻⁶ with no body at the origin, its shadow's momenta the state's and its shadow's
/// positions the state's but body 0's x, between 0.5 and 2 × 10⁻⁶ ahead.
fn check_frame(name: &str, set: &Synthetic) {
    for i in 0..8 {
        let s = set.simstate(i);
        for (what, v) in [("positions", s.r), ("momenta", s.p)] {
            let sum = [0, 1].map(|c| v.iter().map(|b| b[c]).sum::<f32>());
            assert!(
                sum.iter().all(|x| x.abs() <= 1e-6) && v.iter().all(|b| b != &[0.0, 0.0]),
                "`{name}` sample {i}: the {what} {v:?} are not in the CoM frame"
            );
        }
        let dx = s.r_sh[0][0] - s.r[0][0];
        let mut r_sh = s.r_sh;
        r_sh[0][0] = s.r[0][0];
        assert!(
            (5e-7..=2e-6).contains(&dx) && r_sh == s.r && s.p_sh == s.p,
            "`{name}` sample {i}: the shadow {:?}, {:?} is not the state offset in body 0's x",
            s.r_sh,
            s.p_sh
        );
    }
}

#[test]
fn fixtures_configuration_is_in_the_com_frame() {
    for name in NAMES {
        check_frame(name, &filled(name));
    }
}

negative_control!(
    fixtures_configuration_is_in_the_com_frame,
    "a sample whose positions are shifted off the CoM frame is refused",
    expected = "are not in the CoM frame",
    {
        let mut set = filled("set_state");
        let r = set.simstate(2).r;
        set.sample(2).r(r.map(|b| [b[0] + 0.1, b[1]]));
        check_frame("set_state", &set);
    }
);

/// Checks each adversarial sample's raw `packed_a` whole against the packing of the fields it holds
/// (`roundtrip_ctl`), and its reserved bits 10–15 clear: a raw-word comparison, which a bit the accessors mask
/// still fails (pitfalls §9).
fn check_raw_words(set: &Synthetic, want: &[Known]) {
    for (i, k) in (0u32..).zip(want) {
        let w = set.simstate(i).packed_a;
        for (at, width) in PACKED_A_RESERVED {
            assert_eq!(
                (w >> at) & ((1 << width) - 1),
                0,
                "adversarial sample {i}: a reserved bit is set in {w:#010x}"
            );
        }
        assert!(
            roundtrip_ctl(&fields(k), w),
            "adversarial sample {i}: the raw word {w:#010x} is not the packing of its fields"
        );
    }
}

#[test]
fn fixtures_adversarial_raw_words_are_their_fields_packed() {
    check_raw_words(
        &filled("adversarial_descriptor"),
        &known("adversarial_descriptor"),
    );
}

negative_control!(
    fixtures_adversarial_raw_words_are_their_fields_packed,
    "the adversarial descriptor with reserved bit 12 contaminated fails the raw-word check",
    expected = "a reserved bit is set",
    {
        let mut set = filled("adversarial_descriptor");
        let w = set.simstate(5).packed_a;
        set.sample(5).packed_a(w | (1 << 12));
        check_raw_words(&set, &known("adversarial_descriptor"));
    }
);

/// Checks that setting bit `bit` of each adversarial sample's `packed_a` changes none of the fields its accessors
/// read: the masking that makes a check of the unpacked fields blind to it (pitfalls §9).
fn check_masked(bit: u32) {
    let set = filled("adversarial_descriptor");
    for i in 0..8 {
        let w = set.simstate(i).packed_a;
        let (a, b) = (PackedA::unpack(w), PackedA::unpack(w | (1 << bit)));
        assert!(
            (a.state, a.detail, a.saturated, a.dmin_pair, a.last_symbol)
                == (b.state, b.detail, b.saturated, b.dmin_pair, b.last_symbol)
                && a.d_min.to_bits() == b.d_min.to_bits(),
            "bit {bit} of adversarial sample {i} changes an unpacked field"
        );
    }
}

#[test]
fn fixtures_adversarial_reserved_bits_are_masked_by_the_unpack() {
    for (at, width) in PACKED_A_RESERVED {
        for bit in at..at + width {
            check_masked(bit);
        }
    }
}

negative_control!(
    fixtures_adversarial_reserved_bits_are_masked_by_the_unpack,
    "bit 9, the last symbol's top bit, is read by its accessor",
    expected = "changes an unpacked field",
    check_masked(9)
);

/// The f32 at byte `at` of `bytes`, little-endian.
fn f32_at(bytes: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

/// Checks the f16 half `h` of the word `member`, at `bits` of sample `i`: never NaN, and finite but where it is
/// `d_min`'s half holding the unset `+inf`, `0x7C00` exactly, on a failed sample (`failed`: `sim_failed` or
/// `decode_failed`) or one before its first step (`t_end_step == 0`, no step completed; R-271).
fn check_half(
    i: u32,
    member: &str,
    bits: &str,
    h: u16,
    d_min: bool,
    failed: bool,
    t_end_step: u32,
) {
    let v = f16_bits_to_f32(h);
    assert!(
        !v.is_nan(),
        "sample {i}: {member} {bits} holds the f16 NaN {h:#06x}"
    );
    if v.is_finite() {
        return;
    }
    assert!(
        d_min && h == UNSET,
        "sample {i}: {member} {bits} holds the non-finite {h:#06x}, which only d_min's unset +inf may"
    );
    assert!(
        failed || t_end_step == 0,
        "sample {i}: d_min's unset +inf is held by a sample neither failed nor before its first step"
    );
}

/// Checks every stored value of `set`'s first `n` samples, member by member by the ledger's layout of `SimStateFTLE`
/// and `ICDescriptor`: every f32 is not NaN and finite; `packed_a`'s top half, `d_min`, and both halves of `packed_b`,
/// `dE_max` and `dLz_max`, are f16s held to [`check_half`]; the other words are integers. A member the check has no
/// rule for fails it, so a new member is not passed unread.
fn check_stored(set: &Synthetic, n: u32) {
    let bytes = set.bytes();
    let structs = ledger::payload::structs();
    for (name, data) in [
        ("SimStateFTLE", &bytes.simstate),
        ("ICDescriptor", &bytes.ic),
    ] {
        let s = structs
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("the ledger has no `{name}`"));
        let (at, size) = offsets(s);
        for i in 0..n {
            let base = (i * size) as usize;
            let sample = set.simstate(i);
            // Failed: `sim_failed` or `decode_failed`, the states whose `d_min` payload §1 defines as unset; a
            // reserved code is untrusted (payload §2) but not one of them.
            let failed = matches!(
                sd_state(sample.packed_a),
                STATE_SIM_FAILED | STATE_DECODE_FAILED
            );
            let t_end_step = tm_t_end_step(sample.times);
            for (m, &off) in s.members.iter().zip(&at) {
                let o = base + off as usize;
                let floats = match m.storage {
                    Storage::F32 => 1,
                    Storage::Vec2x3 => 6,
                    Storage::U32 => {
                        let w = u32::from_le_bytes(data[o..o + 4].try_into().expect("four bytes"));
                        let (lo, hi) = ((w & 0xffff) as u16, (w >> 16) as u16);
                        match m.name {
                            "packed_a" => {
                                check_half(i, m.name, "bits 16–31", hi, true, failed, t_end_step)
                            }
                            "packed_b" => {
                                check_half(i, m.name, "bits 0–15", lo, false, failed, t_end_step);
                                check_half(i, m.name, "bits 16–31", hi, false, failed, t_end_step);
                            }
                            "times" | "total_substeps" => {}
                            other => panic!("`{name}.{other}`: no rule for this u32 member"),
                        }
                        0
                    }
                    Storage::U16 | Storage::Pad(_) => 0,
                    Storage::U32x4 | Storage::U32Array(_) => {
                        panic!("`{name}.{}`: no rule for this member", m.name)
                    }
                };
                for k in 0..floats {
                    let v = f32_at(data, o + 4 * k);
                    assert!(
                        v.is_finite(),
                        "sample {i}: `{name}.{}` component {k} stores {v}, not a finite value",
                        m.name
                    );
                }
            }
        }
    }
}

#[test]
fn stored_values_no_nan() {
    for name in NAMES {
        check_stored(&filled(name), 8);
    }
}

negative_control!(
    stored_values_no_nan,
    "a forced failure storing the NaN that failed it, in its phase state, fails the check",
    expected = "not a finite value",
    {
        let mut set = filled("forced_failure");
        set.sample(2).r([[f32::NAN, 0.0], [0.0, 0.0], [0.0, 0.0]]);
        check_stored(&set, 8);
    }
);

#[test]
fn stored_values_no_nan_dmin_nan() {
    // The unset bits are allowed on the adversarial descriptor's decode_failed sample 3, which is unstepped.
    check_stored(&filled("adversarial_descriptor"), 8);
}

negative_control!(
    stored_values_no_nan_dmin_nan,
    "an f16 NaN in d_min's half fails the check",
    expected = "holds the f16 NaN",
    {
        let mut set = filled("adversarial_descriptor");
        let w = set.simstate(3).packed_a;
        set.sample(3).packed_a((w & 0xffff) | (0x7e00 << 16));
        check_stored(&set, 8);
    }
);

#[test]
fn stored_values_no_nan_misplaced_inf() {
    // A failed sample's latches: `dE_max` and `dLz_max` 0.0, `d_min` unset.
    check_stored(&filled("forced_failure"), 8);
}

negative_control!(
    stored_values_no_nan_misplaced_inf,
    "f16 +inf in dE_max's half, not d_min's, fails the check",
    expected = "which only d_min's unset +inf may",
    {
        let mut set = filled("forced_failure");
        set.sample(0).packed_b(0x7c00);
        check_stored(&set, 8);
    }
);

#[test]
fn stored_values_no_nan_unset_only_failed_or_unstepped() {
    // Every fixture's unset `d_min` sits on a failed sample or an unstepped one.
    for name in NAMES {
        let want = known(name);
        for (i, k) in want.iter().enumerate() {
            assert!(
                k.d_min != UNSET || k.failed() || k.t_end_step == 0,
                "`{name}` sample {i} is unset but neither failed nor unstepped"
            );
        }
        check_stored(&filled(name), 8);
    }
}

negative_control!(
    stored_values_no_nan_unset_only_failed_or_unstepped,
    "a stepped, running sample holding the unset +inf fails the check",
    expected = "neither failed nor before its first step",
    {
        let mut set = filled("set_state");
        set.sample(3).d_min(f32::INFINITY);
        check_stored(&set, 8);
    }
);

#[test]
fn stored_values_no_nan_negative_inf() {
    check_stored(&filled("set_state"), 8);
}

negative_control!(
    stored_values_no_nan_negative_inf,
    "f16 −inf in d_min's half, a non-finite value other than the unset +inf, fails the check",
    expected = "which only d_min's unset +inf may",
    {
        let mut set = filled("set_state");
        let w = set.simstate(4).packed_a;
        set.sample(4).packed_a((w & 0xffff) | (0xfc00 << 16));
        check_stored(&set, 8);
    }
);

/// Checks that a failed sample's latches are the defined failed-state values: `dE_max = dLz_max = 0.0` and `d_min`
/// unset (payload §1; R-271).
fn check_failed_state(set: &Synthetic, want: &[Known]) {
    for (i, k) in (0u32..).zip(want) {
        if !k.failed() {
            continue;
        }
        let s = set.simstate(i);
        assert_eq!(
            s.packed_b, 0,
            "failed sample {i}: dE_max and dLz_max are not 0.0"
        );
        assert_eq!(
            s.packed_a >> 16,
            u32::from(UNSET),
            "failed sample {i}: d_min is not unset"
        );
    }
}

#[test]
fn fixtures_forced_failure_holds_the_failed_state() {
    check_failed_state(&filled("forced_failure"), &known("forced_failure"));
}

negative_control!(
    fixtures_forced_failure_holds_the_failed_state,
    "a failed sample keeping a nonzero drift latch fails the check",
    expected = "are not 0.0",
    {
        let mut set = filled("forced_failure");
        set.sample(6).drift_max(0.5, 0.0);
        check_failed_state(&set, &known("forced_failure"));
    }
);

/// Checks that the fixtures refuse an unknown name, and that filling `set`, a row of `n` samples, is refused as short.
fn check_refused(set: &mut Synthetic, n: u32) {
    match fixtures::known("no_such_fixture") {
        Ok(_) => panic!("an unknown fixture loaded"),
        Err(e) => assert!(e.contains("no synthetic fixture `no_such_fixture`"), "{e}"),
    }
    match fixtures::fill("set_state", set) {
        Ok(()) => panic!("a fixture filled a row of {n} samples"),
        Err(e) => assert!(
            e.contains(&format!("needs 8 samples, the set has {n}")),
            "{e}"
        ),
    }
}

#[test]
fn fixtures_unknown_or_short_set_is_refused() {
    check_refused(
        &mut Synthetic::flat(Grid::new([7, 1], 1, 0, 8).expect("the row"), 0),
        7,
    );
}

negative_control!(
    fixtures_unknown_or_short_set_is_refused,
    "a row of eight samples is not short",
    expected = "a fixture filled a row of 8 samples",
    check_refused(&mut row(), 8)
);
