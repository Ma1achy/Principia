//! The CPU field's stored records (`engine::field::record`): each sample's `SimState`, word and `ICDescriptor`, read
//! back through the ledger's CPU read side as the shader reads them.
//!
//! The view is the app's zero view (`gui::real::capture_view`): the latent chart `z_α × z_β` over its default window,
//! `z₀` at the origin, horizon 50, here at 16 × 16 samples. The pass criteria:
//! - **the label**: every record, unpacked by `kernel::payload::sim_state_from_ftle` from the bytes the buffers hold,
//!   reads the field's own outcome, `state` and `detail` equal, on every sample;
//! - **the march's own values**: on every sample the march ran to an end it did not fail on, the stored `r` and `p`,
//!   the read side's shape point `n`, its energy drift against the march's own `E_0` and `t_end_step` equal the
//!   march's (Heggie's `HgAdvanced`, the cadence's monitor, `kernel::shape::shape`) bit for bit, and `t_end_fraction`
//!   is `boundary / k_target`; each record's masses are the march's IC's, bit for bit;
//! - **workers**: the records of one worker and of three are the same, every byte, and the plain field (marched
//!   without the word) has their outcomes;
//! - **no plausible zero**: every record reads `ftle` as the canonical NaN, `ftle_valid` false even where the read side
//!   counts renormalisations, and over the view the values the march computes (`t_end_step`, `theta`, `t_dmin_step`,
//!   `total_substeps`, the word's length) are not all zero;
//! - **settled at dispatch**: a start already inside `r_coll` reads `collision` with its pair, `t_end_step` 0, and a
//!   degenerate decode reads `decode_failed` with its detail;
//! - **a failed decode through the field** (the plane's mass coordinate NaN): each record reads `decode_failed`, every
//!   count 0, and holds the decode stage's own descriptor, narrowed once;
//! - **the shape readout beside the wrapper's**: on an 8 × 8 zero view, every record whose outcome the kernel's wrapper
//!   (`first_slice::march::<HeggieRk4>`) shares has the wrapper's `theta`, bit for bit.
//!
//! Each has a control arm: a word swapped, the rows flipped, one record changed under the other worker count, the shadow's
//! failed mark replaced by a finite sum, the computed values zeroed, the descriptor zeroed, `theta` moved by one ulp.

use std::num::NonZeroUsize;
use std::sync::OnceLock;

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::field::record::{read_params, sample, Record, TIER};
use engine::field::{boundaries, heggie_params, Cpu, Stored};
use engine::first_slice::{narrow, uniforms, Quad};
use engine::pool::Pool;
use kernel::chart::Latent;
use kernel::decode::{Degenerate, Ic};
use kernel::driver::cadence::WrapperCadence;
use kernel::payload::{
    energy_drift, fgw_length_raw, ReadParams, SimState, CANONICAL_QNAN_BITS, STATE_COLLISION,
    STATE_DECODE_FAILED, STATE_SIM_FAILED,
};
use kernel::regularisation::az::End;
use ledger::gen::rust::offsets;
use validation::negative_control;

const SIZE: [usize; 2] = [16, 16];
const HORIZON: f64 = 50.0;

/// The app's zero view at its horizon of 50.
fn zero_view() -> SimConfig {
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0f32 } else { 0.0 });
    let quad = Quad {
        chart: Latent {
            z0: [0.0; 8],
            q1: e(0),
            q2: e(1),
        },
        window: Window::SQUARE,
        size: [1, 1],
    };
    SimConfig {
        horizon: Horizon { t: HORIZON },
        ..quad.sim_key()
    }
}

fn pool(workers: usize) -> Pool {
    Pool::with_threads(NonZeroUsize::new(workers).expect("workers")).expect("the pool")
}

/// The zero view's field and records on `workers` workers.
fn stored_on(workers: usize) -> Stored {
    Cpu.stored_on(&pool(workers), &zero_view(), Window::SQUARE, SIZE)
        .expect("the zero view's records")
}

/// The zero view's records on three workers, computed once for every test and control in this binary.
fn stored() -> &'static Stored {
    static STORED: OnceLock<Stored> = OnceLock::new();
    STORED.get_or_init(|| stored_on(3))
}

/// The zero view's records on one worker, computed once.
fn stored_one() -> &'static Stored {
    static STORED: OnceLock<Stored> = OnceLock::new();
    STORED.get_or_init(|| stored_on(1))
}

/// The ledger struct `name`'s members with their byte offsets, and its size.
fn layout(name: &str) -> (Vec<(&'static str, usize)>, usize) {
    let s = ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == name)
        .expect("the ledger's struct");
    let (at, size) = offsets(&s);
    let members = s
        .members
        .iter()
        .zip(at)
        .map(|(m, at)| (m.name, at as usize))
        .collect();
    (members, size as usize)
}

/// The little-endian u32 at byte `at` of `b`.
fn word_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Record `i` rebuilt from the three buffers' bytes, each member read at the ledger's offset for it, as the shader
/// loads it: every 4-byte member of `SimStateFTLE` and `ICDescriptor` (the one u16 pair, `closure_step` and
/// `_reserved`, read as their shared word), and the word's four.
fn from_bytes(stored: &Stored, i: usize) -> Record {
    let bytes = stored.bytes();
    let (sim_members, sim_size) = layout("SimStateFTLE");
    let (ic_members, ic_size) = layout("ICDescriptor");
    let sim = &bytes.simstate[i * sim_size..(i + 1) * sim_size];
    let ic = &bytes.ic[i * ic_size..(i + 1) * ic_size];
    let at = |members: &[(&str, usize)], name: &str| {
        members
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("the ledger has no `{name}`"))
            .1
    };
    let f = |b: &[u8], members: &[(&str, usize)], name: &str, k: usize| {
        f32::from_bits(word_at(b, at(members, name) + 4 * k))
    };
    let v2 = |name: &str| -> [[f32; 2]; 3] {
        core::array::from_fn(|j| core::array::from_fn(|c| f(sim, &sim_members, name, 2 * j + c)))
    };
    let u = |name: &str| word_at(sim, at(&sim_members, name));
    let s = |name: &str| f(sim, &sim_members, name, 0);
    let d = |name: &str| f(ic, &ic_members, name, 0);
    let pair = u("closure_step");
    let mut out = Record::default();
    out.sim.r = v2("r");
    out.sim.p = v2("p");
    out.sim.r_sh = v2("r_sh");
    out.sim.p_sh = v2("p_sh");
    out.sim.S = s("S");
    out.sim.theta = s("theta");
    out.sim.mean_y = s("mean_y");
    out.sim.C_ty = s("C_ty");
    out.sim.E_0 = s("E_0");
    out.sim.Lz_0 = s("Lz_0");
    out.sim.packed_a = u("packed_a");
    out.sim.packed_b = u("packed_b");
    out.sim.times = u("times");
    out.sim.total_substeps = u("total_substeps");
    out.sim.budget_bits = u("budget_bits");
    out.sim.dE_reg = u("dE_reg");
    out.sim.closure_min = s("closure_min");
    out.sim.closure_step = pair as u16;
    out.sim._reserved = (pair >> 16) as u16;
    out.sim.coll_floor_bits = u("coll_floor_bits");
    out.sim._nhat_lag_0 = s("_nhat_lag_0");
    out.sim._nhat_lag_1 = s("_nhat_lag_1");
    out.sim._nhat_lag_2 = s("_nhat_lag_2");
    out.sim._dmin_d2 = u("_dmin_d2");
    out.sim._march_bits = u("_march_bits");
    out.word = core::array::from_fn(|k| word_at(&bytes.word, 16 * i + 4 * k));
    out.ic.m0 = d("m0");
    out.ic.m1 = d("m1");
    out.ic.m2 = d("m2");
    out.ic.q_mass = d("q_mass");
    out.ic.rho_mag = d("rho_mag");
    out.ic.lambda_mag = d("lambda_mag");
    out.ic.rho_ratio = d("rho_ratio");
    out.ic.rho_angle = d("rho_angle");
    out.ic.K_0 = d("K_0");
    out.ic.V_0 = d("V_0");
    out.ic.virial_ratio = d("virial_ratio");
    out.ic.r_min_pair_0 = d("r_min_pair_0");
    out
}

// ── The label ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// Every record of `stored`, rebuilt from the buffers' bytes and unpacked by the read side, reads its sample's outcome,
/// and the bytes rebuild the record exactly.
fn check_label(stored: &Stored) {
    assert_eq!(stored.records.len(), SIZE[0] * SIZE[1]);
    const {
        assert!(
            TIER.has_ftle && TIER.has_word,
            "the records' tier is the full one"
        )
    };
    for (i, (record, outcome)) in stored
        .records
        .iter()
        .zip(&stored.field.outcomes)
        .enumerate()
    {
        let rebuilt = from_bytes(stored, i);
        assert_eq!(
            bits(&rebuilt),
            bits(record),
            "sample {i}'s bytes do not rebuild its record"
        );
        let s = rebuilt.read(&stored.read);
        assert_eq!(
            (s.state, s.detail),
            (outcome.state, outcome.detail),
            "sample {i}'s record does not read its outcome"
        );
    }
}

/// Every field of `r` as bits, so a NaN or a signed zero compares as itself.
fn bits(r: &Record) -> Vec<u32> {
    let s = &r.sim;
    let f = |v: &[[f32; 2]; 3]| v.iter().flatten().map(|x| x.to_bits()).collect::<Vec<_>>();
    let mut out = [f(&s.r), f(&s.p), f(&s.r_sh), f(&s.p_sh)].concat();
    out.extend(
        [
            s.S,
            s.theta,
            s.mean_y,
            s.C_ty,
            s.E_0,
            s.Lz_0,
            s.closure_min,
            s._nhat_lag_0,
            s._nhat_lag_1,
            s._nhat_lag_2,
        ]
        .map(f32::to_bits),
    );
    out.extend([
        s.packed_a,
        s.packed_b,
        s.times,
        s.total_substeps,
        s.budget_bits,
        s.dE_reg,
        u32::from(s.closure_step),
        u32::from(s._reserved),
        s.coll_floor_bits,
        s._dmin_d2,
        s._march_bits,
    ]);
    out.extend(r.word);
    let d = &r.ic;
    out.extend(
        [
            d.m0,
            d.m1,
            d.m2,
            d.q_mass,
            d.rho_mag,
            d.lambda_mag,
            d.rho_ratio,
            d.rho_angle,
            d.K_0,
            d.V_0,
            d.virial_ratio,
            d.r_min_pair_0,
        ]
        .map(f32::to_bits),
    );
    out
}

#[test]
fn every_record_reads_its_outcome() {
    check_label(stored());
}

negative_control!(
    every_record_reads_its_outcome,
    "packed_a and packed_b swapped in every record, the label is read from the drift latches",
    expected = "does not read its outcome",
    check_label(&{
        let mut s = stored().clone();
        for r in &mut s.records {
            core::mem::swap(&mut r.sim.packed_a, &mut r.sim.packed_b);
        }
        s
    })
);

// ── The march's own values ──────────────────────────────────────────────────────────────────────────────────────────

/// Each sample's decoded IC, as the field decodes and narrows it.
fn ics() -> &'static Vec<(Ic<f32>, Option<Degenerate>)> {
    static ICS: OnceLock<Vec<(Ic<f32>, Option<Degenerate>)>> = OnceLock::new();
    ICS.get_or_init(|| {
        let sim = zero_view();
        let quad = Quad {
            chart: engine::field::latent(&sim),
            window: Window::SQUARE,
            size: SIZE,
        };
        let key = quad.sim_key();
        (0..quad.len())
            .map(|i| {
                let out = engine::decode_dispatch::decode_only_with::<
                    Latent<f32>,
                    kernel::generated::links::Defaults,
                >(&quad.chart, quad.uv(i), &key, None)
                .expect("the decode");
                (narrow(&out.ic), out.degenerate)
            })
            .collect()
    })
}

/// On every sample the march ran to an end it did not fail on, the read side of `stored`'s record gives the march's own
/// state, shape point, energy drift and end, bit for bit; every record's masses are its IC's. Returns how many samples
/// were compared.
fn check_march_values(stored: &Stored) -> usize {
    let key = uniforms(HORIZON).expect("the horizon");
    let params = heggie_params();
    let k_target = boundaries(HORIZON, &params);
    assert_eq!(stored.read, read_params(k_target, &params));
    let mut compared = 0;
    for (i, ((ic, degenerate), record)) in ics().iter().zip(&stored.records).enumerate() {
        let m = [record.ic.m0, record.ic.m1, record.ic.m2].map(f32::to_bits);
        assert_eq!(
            m,
            ic.m.map(f32::to_bits),
            "sample {i}'s masses are not its IC's"
        );
        let marched = sample(
            ic,
            *degenerate,
            key.radius,
            k_target,
            &params,
            key.stop_on_escape,
            true,
        );
        let Some(end) = marched.end else { continue };
        let s: SimState = record.read(&stored.read);
        if s.state == STATE_SIM_FAILED || end.count.end == End::NonFinite {
            continue;
        }
        compared += 1;
        let v = |x: [[f32; 2]; 3]| x.map(|c| c.map(f32::to_bits));
        assert_eq!(
            (v(s.r), v(s.p)),
            (v(end.state.r), v(end.state.p)),
            "sample {i}'s stored state is not where its march ended"
        );
        assert_eq!(
            s.n.map(f32::to_bits),
            kernel::shape::shape(end.state.r, ic.m).map(f32::to_bits),
            "sample {i}'s shape point is not its march's"
        );
        let e_0 = WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, key.radius)
            .monitor_state()
            .e_0();
        assert_eq!(
            s.E_0.to_bits(),
            e_0.to_bits(),
            "sample {i}'s E_0 is not its march's"
        );
        assert_eq!(
            s.energy_drift.to_bits(),
            energy_drift(end.state.r, end.state.p, ic.m, e_0).to_bits(),
            "sample {i}'s energy drift is not its march's"
        );
        assert_eq!(
            s.t_end_step, end.boundary,
            "sample {i}'s t_end_step is not its march's end"
        );
        assert_eq!(
            s.t_end_fraction.to_bits(),
            (end.boundary as f32 / k_target as f32).to_bits(),
            "sample {i}'s t_end_fraction is not its end over the horizon"
        );
    }
    compared
}

#[test]
fn every_record_holds_its_marchs_own_values() {
    let compared = check_march_values(stored());
    println!("compared {compared} of {} samples", SIZE[0] * SIZE[1]);
    assert!(compared > 0, "no sample's march was compared");
}

negative_control!(
    every_record_holds_its_marchs_own_values,
    "the rows flipped: each record is another sample's",
    expected = "is not",
    {
        let mut s = stored().clone();
        let [w, h] = SIZE;
        s.records = (0..h)
            .rev()
            .flat_map(|y| stored().records[y * w..(y + 1) * w].to_vec())
            .collect();
        check_march_values(&s);
    }
);

// ── Workers ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// `a`'s and `b`'s records and outcomes are the same, every byte.
fn check_same(a: &Stored, b: &Stored) {
    assert_eq!(
        a.field, b.field,
        "the outcomes differ between worker counts"
    );
    assert_eq!(a.read, b.read);
    assert!(
        a.bytes() == b.bytes(),
        "the records differ between worker counts"
    );
}

#[test]
fn one_worker_and_three_store_the_same_records() {
    check_same(stored_one(), stored());
    // The plain field, which marches without the word, has the records' outcomes.
    let field = Cpu
        .field_on(&pool(3), &zero_view(), Window::SQUARE, SIZE)
        .expect("the field");
    assert_eq!(
        field,
        stored().field,
        "the plain field's outcomes are not the records'"
    );
}

negative_control!(
    one_worker_and_three_store_the_same_records,
    "one record's last word changed under one worker",
    expected = "the records differ between worker counts",
    check_same(
        &{
            let mut s = stored_one().clone();
            s.records[SIZE[0] * SIZE[1] - 1].word[3] ^= 1;
            s
        },
        stored()
    )
);

// ── No plausible zero ───────────────────────────────────────────────────────────────────────────────────────────────

/// No record reads an FTLE, even where the read side counts renormalisations.
fn check_no_ftle(stored: &Stored) {
    let counted = ReadParams {
        n_renorm: 1,
        delta_0: 1e-6,
        ..stored.read
    };
    for (i, r) in stored.records.iter().enumerate() {
        for params in [stored.read, counted] {
            let s = r.read(&params);
            assert!(
                !s.ftle_valid && s.ftle.to_bits() == CANONICAL_QNAN_BITS,
                "sample {i} reads an FTLE ({} valid {})",
                s.ftle,
                s.ftle_valid
            );
        }
    }
}

#[test]
fn no_record_reads_an_ftle() {
    check_no_ftle(stored());
}

negative_control!(
    no_record_reads_an_ftle,
    "the shadow's failed mark replaced by a finite sum",
    expected = "reads an FTLE",
    check_no_ftle(&{
        let mut s = stored().clone();
        for r in &mut s.records {
            r.sim.S = 0.5;
        }
        s
    })
);

/// Over the view, the values the march computes are not all zero.
fn check_computed(stored: &Stored) {
    let reads: Vec<SimState> = stored
        .records
        .iter()
        .map(|r| r.read(&stored.read))
        .collect();
    let any = |what: &str, f: &dyn Fn(&SimState) -> bool| {
        assert!(reads.iter().any(f), "no sample stores a {what}: all zero");
    };
    any("t_end_step", &|s| s.t_end_step > 0);
    any("theta", &|s| s.theta != 0.0);
    any("t_dmin_step", &|s| s.t_dmin_step > 0);
    any("total_substeps", &|s| s.total_substeps > 0);
    any("mean_y", &|s| s.mean_y != 0.0);
    any("word", &|s| fgw_length_raw(s.word) > 0);
}

#[test]
fn the_computed_values_are_stored() {
    check_computed(stored());
}

negative_control!(
    the_computed_values_are_stored,
    "the march's computed values zeroed, as a record built from the state and label alone would hold them",
    expected = "all zero",
    check_computed(&{
        let mut s = stored().clone();
        for r in &mut s.records {
            r.sim.theta = 0.0;
            r.sim.times = 0;
            r.sim.total_substeps = 0;
            r.sim.mean_y = 0.0;
            r.word = [0; 4];
        }
        s
    })
);

// ── Settled at dispatch ─────────────────────────────────────────────────────────────────────────────────────────────

/// A start whose bodies 0 and 1 are `gap` apart, body 2 far: inside `r_coll` for a small `gap`.
fn close_start(gap: f32) -> Ic<f32> {
    Ic {
        m: [1.0, 1.0, 1.0],
        r: [[-gap / 2.0, 0.0], [gap / 2.0, 0.0], [0.0, 1.0]],
        p: [[0.0, 0.1], [0.0, -0.1], [0.0, 0.0]],
    }
}

/// The record of `ic`, tagged `degenerate`, unpacked: what a sample settled at dispatch stores.
fn settled(ic: &Ic<f32>, degenerate: Option<Degenerate>) -> (engine::field::Outcome, SimState) {
    let key = uniforms(HORIZON).expect("the horizon");
    let params = heggie_params();
    let k = boundaries(HORIZON, &params);
    let s = sample(
        ic,
        degenerate,
        key.radius,
        k,
        &params,
        key.stop_on_escape,
        true,
    );
    let record = Record {
        sim: s.sim,
        word: s.word,
        ic: Default::default(),
    };
    (s.outcome, record.read(&read_params(k, &params)))
}

/// A start `gap` apart reads `collision` of pair 2 at `t_end_step` 0, with no march; a degenerate decode reads
/// `decode_failed` with the degenerate's detail.
fn check_settled(gap: f32) {
    let (outcome, s) = settled(&close_start(gap), None);
    assert_eq!(
        (
            outcome.state,
            outcome.detail,
            s.state,
            s.detail,
            s.t_end_step
        ),
        (STATE_COLLISION, 2, STATE_COLLISION, 2, 0),
        "a start inside r_coll does not store its collision at t_end_step 0"
    );
    let (outcome, s) = settled(&close_start(0.5), Some(Degenerate::M01Tiny));
    assert_eq!(
        (outcome.state, s.state, s.detail),
        (
            STATE_DECODE_FAILED,
            STATE_DECODE_FAILED,
            Degenerate::M01Tiny.detail()
        ),
        "a degenerate decode does not store decode_failed"
    );
}

#[test]
fn a_sample_settled_at_dispatch_stores_its_label() {
    check_settled(1e-4);
}

negative_control!(
    a_sample_settled_at_dispatch_stores_its_label,
    "the bodies started outside r_coll: the start is marched",
    expected = "does not store its collision at t_end_step 0",
    check_settled(0.5)
);

// ── The shape readout beside the wrapper's ──────────────────────────────────────────────────────────────────────────

/// The samples of an 8 × 8 zero view the field's records and the kernel's wrapper (`first_slice::march::<HeggieRk4>`,
/// the default occupant at the field's values) both mark with the same outcome, and each one's `theta` from both.
fn thetas() -> &'static Vec<(f32, f32)> {
    static THETAS: OnceLock<Vec<(f32, f32)>> = OnceLock::new();
    THETAS.get_or_init(|| {
        let s = Cpu
            .stored_on(&pool(3), &zero_view(), Window::SQUARE, [8, 8])
            .expect("the 8 x 8 records");
        let quad = Quad {
            chart: engine::field::latent(&zero_view()),
            window: Window::SQUARE,
            size: [8, 8],
        };
        let key = quad.sim_key();
        let march_key = uniforms(HORIZON).expect("the horizon");
        (0..quad.len())
            .filter_map(|i| {
                let out = engine::decode_dispatch::decode_only_with::<
                    Latent<f32>,
                    kernel::generated::links::Defaults,
                >(&quad.chart, quad.uv(i), &key, None)
                .expect("the decode");
                let w = engine::first_slice::march::<engine::first_slice::HeggieRk4>(
                    &narrow(&out.ic),
                    &march_key,
                );
                let o = s.field.outcomes[i];
                let same = (
                    kernel::payload::sd_state(w.packed_a),
                    kernel::payload::sd_detail(w.packed_a),
                ) == (o.state, o.detail);
                same.then_some((s.records[i].sim.theta, w.theta))
            })
            .collect()
    })
}

/// Each record's `theta` is the wrapper's, bit for bit.
fn check_theta(pairs: &[(f32, f32)]) {
    assert!(
        !pairs.is_empty(),
        "no sample's outcome agrees with the wrapper's"
    );
    for (i, (field, wrapper)) in pairs.iter().enumerate() {
        assert_eq!(
            field.to_bits(),
            wrapper.to_bits(),
            "sample {i}'s theta {field} is not the wrapper's {wrapper}"
        );
    }
}

#[test]
fn every_records_theta_is_the_wrappers() {
    println!("theta compared on {} of 64 samples", thetas().len());
    check_theta(thetas());
}

negative_control!(
    every_records_theta_is_the_wrappers,
    "one record's theta moved by one unit in the last place",
    expected = "is not the wrapper's",
    check_theta(&{
        let mut t = thetas().clone();
        t[0].0 = f32::from_bits(t[0].0.to_bits() + 1);
        t
    })
);

// ── A failed decode, through the field ──────────────────────────────────────────────────────────────────────────────

/// The zero view with `z₀`'s mass coordinate `z[6]` NaN: every sample's decode fails (`decode_failed`, non-finite
/// decode output), 2 × 2 samples.
fn failed_view() -> Stored {
    let mut sim = zero_view();
    sim.plane.z0[6] = f64::NAN;
    Cpu.stored_on(&pool(1), &sim, Window::SQUARE, [2, 2])
        .expect("the failed view's records")
}

/// Every sample of `stored` a failed decode: its record reads `decode_failed` with its outcome's detail, every count
/// 0, no FTLE, the word unbound to its empty value, and its `ICDescriptor` the decode stage's own for that IC, narrowed
/// once, the masses NaN as the decode left them.
fn check_failed(stored: &Stored) {
    let mut sim = zero_view();
    sim.plane.z0[6] = f64::NAN;
    let quad = Quad {
        chart: engine::field::latent(&sim),
        window: Window::SQUARE,
        size: [2, 2],
    };
    let key = quad.sim_key();
    for (i, (record, outcome)) in stored
        .records
        .iter()
        .zip(&stored.field.outcomes)
        .enumerate()
    {
        let out = engine::decode_dispatch::decode_only_with::<
            Latent<f32>,
            kernel::generated::links::Defaults,
        >(&quad.chart, quad.uv(i), &key, None)
        .expect("the decode");
        let s = record.read(&stored.read);
        assert_eq!(
            (
                outcome.state,
                s.state,
                s.detail,
                s.t_end_step,
                s.total_substeps
            ),
            (
                STATE_DECODE_FAILED,
                STATE_DECODE_FAILED,
                outcome.detail,
                0,
                0
            ),
            "sample {i} does not store its failed decode"
        );
        assert!(!s.ftle_valid && s.ftle.is_nan(), "sample {i} reads an FTLE");
        let expected = engine::field::record::narrow_descriptor(&out.descriptor);
        let b = |d: &kernel::payload::ICDescriptor| {
            [d.m0, d.m1, d.m2, d.K_0, d.V_0, d.r_min_pair_0, d.rho_angle].map(f32::to_bits)
        };
        assert_eq!(
            b(&record.ic),
            b(&expected),
            "sample {i}'s descriptor is not the decode's"
        );
    }
}

#[test]
fn a_failed_decode_stores_the_decodes_descriptor() {
    check_failed(&failed_view());
}

negative_control!(
    a_failed_decode_stores_the_decodes_descriptor,
    "the descriptor left at its zero, a plausible IC",
    expected = "is not the decode's",
    check_failed(&{
        let mut s = failed_view();
        s.records[0].ic = Default::default();
        s
    })
);
