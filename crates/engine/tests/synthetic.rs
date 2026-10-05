//! The synthetic payload harness's CPU side (TASK-M1-06; debug tooling plan step 0b): the bytes it uploads are the
//! generated structs' layouts, member for member at the ledger's offsets, and its setters write through the generated
//! pack routines, so the generated accessors read back what was set (REQ-TOOL-014). Each test registers its negative
//! control (R-176).

use engine::synthetic::{ic_words, simstate_words, Synthetic, QUAD_WORDS};
use kernel::payload::{
    fgw_reduced_length, pa_d_min, pa_d_min_is_unset, pb_dE_max, sd_detail, sd_dmin_pair,
    sd_saturated, sd_state, tm_t_dmin_step, tm_t_end_step, ICDescriptor, SimStateFTLE,
    STATE_RUNNING,
};
use ledger::gen::rust::offsets;
use ledger::quad::RENDER_QUAD;
use ledger::schema::Struct;
use render::raster::Grid;
use validation::negative_control;

fn grid() -> Grid {
    Grid::new([2, 1], 2, 1, 1).expect("the test grid")
}

fn ledger_struct(name: &str) -> Struct {
    ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("the ledger has no `{name}`"))
}

/// Every member of `SimStateFTLE` set to its own marker: the k-th f32 component `k + 1`, the u32s and u16s distinct
/// bit patterns.
fn marked_simstate() -> SimStateFTLE {
    let mut k = 0.0f32;
    let mut next = || {
        k += 1.0;
        k
    };
    let mut group = || [[next(), next()], [next(), next()], [next(), next()]];
    SimStateFTLE {
        r: group(),
        p: group(),
        r_sh: group(),
        p_sh: group(),
        S: 25.0,
        theta: 26.0,
        mean_y: 27.0,
        C_ty: 28.0,
        E_0: 29.0,
        Lz_0: 30.0,
        packed_a: 0xa1a1_a1a1,
        packed_b: 0xb2b2_b2b2,
        times: 0xc3c3_c3c3,
        total_substeps: 0xd4d4_d4d4,
        closure_min: 35.0,
        closure_step: 0x1234,
        _reserved: 0x5678,
        ..SimStateFTLE::default()
    }
}

/// The marker [`marked_simstate`] gives `member`'s first word.
fn simstate_marker(member: &str) -> u32 {
    match member {
        "r" => 1.0f32.to_bits(),
        "p" => 7.0f32.to_bits(),
        "r_sh" => 13.0f32.to_bits(),
        "p_sh" => 19.0f32.to_bits(),
        "S" => 25.0f32.to_bits(),
        "theta" => 26.0f32.to_bits(),
        "mean_y" => 27.0f32.to_bits(),
        "C_ty" => 28.0f32.to_bits(),
        "E_0" => 29.0f32.to_bits(),
        "Lz_0" => 30.0f32.to_bits(),
        "packed_a" => 0xa1a1_a1a1,
        "packed_b" => 0xb2b2_b2b2,
        "times" => 0xc3c3_c3c3,
        "total_substeps" => 0xd4d4_d4d4,
        "closure_min" => 35.0f32.to_bits(),
        "closure_step" => 0x1234,
        "_reserved" => 0x5678,
        other => panic!("no marker for `{other}`"),
    }
}

/// `words`, the bytes a struct uploads as, hold each of the ledger struct `s`'s members at its offset, read by
/// `marker`, and are its size.
fn check_layout(s: &Struct, words: &[u32], marker: impl Fn(&str) -> u32) {
    let (offs, size) = offsets(s);
    assert_eq!(
        words.len() as u32 * 4,
        size,
        "the uploaded `{}` is not the ledger's size",
        s.name
    );
    for (m, &at) in s.members.iter().zip(&offs) {
        if m.name == "_pad" {
            continue;
        }
        let word = words[at as usize / 4];
        let got = if at % 4 == 2 {
            word >> 16
        } else if m.storage == ledger::schema::Storage::U16 {
            word & 0xffff
        } else {
            word
        };
        assert_eq!(
            got,
            marker(m.name),
            "the uploaded `{}.{}` is not at the ledger's offset {at}",
            s.name,
            m.name
        );
    }
}

#[test]
fn synthetic_bytes_match_the_generated_simstate_layout() {
    check_layout(
        &ledger_struct("SimStateFTLE"),
        &simstate_words(&marked_simstate()),
        simstate_marker,
    );
}

negative_control!(
    synthetic_bytes_match_the_generated_simstate_layout,
    "words with S and theta swapped are off the ledger's offsets",
    expected = "is not at the ledger's offset",
    {
        let mut w = simstate_words(&marked_simstate());
        w.swap(24, 25);
        check_layout(&ledger_struct("SimStateFTLE"), &w, simstate_marker)
    }
);

fn marked_ic() -> ICDescriptor {
    ICDescriptor {
        m0: 1.0,
        m1: 2.0,
        m2: 3.0,
        q_mass: 4.0,
        rho_mag: 5.0,
        lambda_mag: 6.0,
        rho_ratio: 7.0,
        rho_angle: 8.0,
        K_0: 9.0,
        V_0: 10.0,
        virial_ratio: 11.0,
        r_min_pair_0: 12.0,
        ..ICDescriptor::default()
    }
}

fn ic_marker(member: &str) -> u32 {
    let order = [
        "m0",
        "m1",
        "m2",
        "q_mass",
        "rho_mag",
        "lambda_mag",
        "rho_ratio",
        "rho_angle",
        "K_0",
        "V_0",
        "virial_ratio",
        "r_min_pair_0",
    ];
    let k = order
        .iter()
        .position(|n| *n == member)
        .unwrap_or_else(|| panic!("no marker for `{member}`"));
    (k as f32 + 1.0).to_bits()
}

#[test]
fn synthetic_bytes_match_the_generated_ic_layout() {
    check_layout(
        &ledger_struct("ICDescriptor"),
        &ic_words(&marked_ic()),
        ic_marker,
    );
}

negative_control!(
    synthetic_bytes_match_the_generated_ic_layout,
    "words with K_0 and V_0 swapped are off the ledger's offsets",
    expected = "is not at the ledger's offset",
    {
        let mut w = ic_words(&marked_ic());
        w.swap(8, 9);
        check_layout(&ledger_struct("ICDescriptor"), &w, ic_marker)
    }
);

/// A quad set member by member through its named setters, each to its slot's index plus one (as bits for an f32).
fn marked_quad(set: &mut Synthetic) {
    for (k, f) in RENDER_QUAD.iter().enumerate() {
        let mut q = set.quad(1);
        let done = match f.storage {
            ledger::schema::Storage::F32 => q.f32(f.name, k as f32 + 1.0).map(|_| ()),
            _ => q.u32(f.name, k as u32 + 1).map(|_| ()),
        };
        done.unwrap_or_else(|e| panic!("{e}"));
    }
}

fn quad_marker(member: &str) -> u32 {
    let k = RENDER_QUAD
        .iter()
        .position(|f| f.name == member)
        .unwrap_or_else(|| panic!("no marker for `{member}`"));
    match RENDER_QUAD[k].storage {
        ledger::schema::Storage::F32 => (k as f32 + 1.0).to_bits(),
        _ => k as u32 + 1,
    }
}

#[test]
fn synthetic_bytes_match_the_generated_quad_layout() {
    let mut set = Synthetic::flat(grid(), 4);
    marked_quad(&mut set);
    let bytes = set.bytes();
    assert_eq!(bytes.quad.len(), 2 * QUAD_WORDS * 4, "two quads' bytes");
    let words: Vec<u32> = bytes.quad[QUAD_WORDS * 4..]
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    check_layout(&ledger::quad::render_quad(), &words, quad_marker);
    assert_eq!(
        set.quad_words(0)[0],
        4,
        "the flat layout's quad 0 is not at depth 4"
    );
    let err = set.quad(0).f32("quad_depth", 1.0).err();
    assert!(
        err.is_some(),
        "an f32 written to the u32 quad_depth was accepted"
    );
}

negative_control!(
    synthetic_bytes_match_the_generated_quad_layout,
    "the quad's words with two members swapped are off the ledger's offsets",
    expected = "is not at the ledger's offset",
    {
        let mut set = Synthetic::flat(grid(), 4);
        marked_quad(&mut set);
        let mut w = set.quad_words(1);
        w.swap(0, 1);
        check_layout(&ledger::quad::render_quad(), &w, quad_marker)
    }
);

/// Sample 2 set through every setter reads back, through the generated accessors, as it was set; `fresh` is a sample
/// left at the flat layout's defaults.
fn check_setters(set: &Synthetic, fresh: u32) {
    let s = set.simstate(2);
    let a = s.packed_a;
    assert_eq!(
        (sd_state(a), sd_detail(a), sd_saturated(a), sd_dmin_pair(a)),
        (2, 1, true, 0),
        "the descriptor does not read back as set"
    );
    assert_eq!(pa_d_min(a), 0.5, "d_min does not read back as set");
    assert_eq!(
        pb_dE_max(s.packed_b),
        0.25,
        "dE_max does not read back as set"
    );
    assert_eq!(
        (tm_t_end_step(s.times), tm_t_dmin_step(s.times)),
        (700, 300),
        "times do not read back as set"
    );
    assert_eq!(
        fgw_reduced_length(set.word(2)),
        5,
        "the word's length does not read back as set"
    );
    let f = set.simstate(fresh).packed_a;
    assert!(
        sd_state(f) == STATE_RUNNING && pa_d_min_is_unset(f) && sd_dmin_pair(f) == 3,
        "a fresh sample is not running with d_min unset and dmin_pair at its sentinel"
    );
    assert_eq!(set.dmin_counts(), (0, 0), "a d_min pack was counted");
}

fn set_sample_2() -> Synthetic {
    let mut set = Synthetic::flat(grid(), 0);
    set.sample(2)
        .state(2)
        .detail(1)
        .saturated(true)
        .dmin_pair(0)
        .d_min(0.5)
        .drift_max(0.25, 0.125)
        .times(700, 300)
        .word([9, 0, 0, 0], 5);
    set
}

#[test]
fn synthetic_setters_write_through_the_pack_routines() {
    check_setters(&set_sample_2(), 0);
}

negative_control!(
    synthetic_setters_write_through_the_pack_routines,
    "sample 2 read back as the fresh sample fails",
    expected = "a fresh sample is not running",
    check_setters(&set_sample_2(), 2)
);
