//! Substep control (TASK-M3-03; dd_integrator §3.3, integrator_contract Part 3 and Part 4, R-502, R-86):
//! - `nsub_table_host` (REQ-INT-091): the host generator's bits for every configuration of
//!   `fixtures/gates/nsub-table/tables.json` equal the reference script's correctly rounded bits, or, at an entry the
//!   fixture lists, one of its two neighbours and the generator's own rounding; `N_max = L_max + 1` and `N_max = 0`
//!   are refused with the typed error, as are an `r_sub` or `γ_sub` not finite and positive, a table that rises, and a
//!   threshold below `N_max` that is not a normal f32; the packing into `SimUniforms`' rows.
//! - `nsub_default_table_bitwise` (REQ-INT-092): the host generator at the default sim key reproduces the checked-in
//!   build-time table bit for bit.
//! - `nsub_bucket_tree` (REQ-INT-028, at the unit level): at every threshold, its value and its two f32 neighbours, the
//!   generated tree's `N_sub` is the table's own count `min(1 + #{m : d² < THR[m]}, N_max)`; `d² = THR[n]` is bucket
//!   `n`, the f32 below it `n + 1`; `0`, `+∞` and NaN.
//! - `nsub_d2_quantised`: the square's floor at `2⁻⁴⁰`, the closest pair, and the f64 and f32 kernels' agreement on
//!   positions whose f32 casts agree.
//! - `nsub_live_readout` (REQ-INT-027), `saturated_sticky` (REQ-TOOL-033), `total_substeps_proxy` (REQ-TOOL-034).

use kernel::driver::nsub_table::{
    check_table, pack, threshold_bits, threshold_f64, thresholds, NsubConfig, NsubTableError,
};
use kernel::driver::substep::{
    d2_quantised, n_sub, square, NsubThresholds, Quantise, SubstepState,
};
use kernel::generated::nsub_thresholds::{
    DEFAULT_THRESHOLD_BITS, GAMMA_SUB_DEFAULT, NSUB_TABLE_MAX, NSUB_TABLE_ROWS, N_MAX_DEFAULT,
    R_SUB_DEFAULT,
};
use kernel::payload::{sd_saturated, total_substeps_log2};
use validation::negative_control;

const L: usize = NSUB_TABLE_MAX;

fn config(r_sub: f64, gamma_sub: f64, n_max: u32) -> NsubConfig {
    NsubConfig {
        r_sub,
        gamma_sub,
        n_max,
    }
}

fn default_table() -> NsubThresholds {
    thresholds(&NsubConfig::DEFAULT).expect("the default sim key generates")
}

/// The f32 next below / above a positive finite `x`.
fn below(x: f32) -> f32 {
    f32::from_bits(x.to_bits() - 1)
}
fn above(x: f32) -> f32 {
    f32::from_bits(x.to_bits() + 1)
}

// ── REQ-INT-091: the host generator against the independent reference ───────────────────────────────────────────

/// One configuration of the fixture: the sim key, its correctly rounded bits, and its listed exceptions
/// `(n, correctly rounded, other)`.
struct Reference {
    config: NsubConfig,
    bits: Vec<u32>,
    exceptions: Vec<(usize, u32, u32)>,
}

fn hex(v: &serde_json::Value) -> u32 {
    let s = v.as_str().expect("a hex string");
    u32::from_str_radix(s.trim_start_matches("0x"), 16).expect("hex bits")
}

fn references() -> Vec<Reference> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/gates/nsub-table/tables.json"
    );
    let text = std::fs::read_to_string(path).expect("read the nsub-table fixture");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("the fixture is JSON");
    assert_eq!(
        doc["l_max"].as_u64(),
        Some(L as u64),
        "the fixture's l_max is not the register's"
    );
    doc["configs"]
        .as_array()
        .expect("configs")
        .iter()
        .map(|c| Reference {
            config: config(
                c["r_sub"].as_f64().expect("r_sub"),
                c["gamma_sub"].as_f64().expect("gamma_sub"),
                c["n_max"].as_u64().expect("n_max") as u32,
            ),
            bits: c["bits"]
                .as_array()
                .expect("bits")
                .iter()
                .map(hex)
                .collect(),
            exceptions: c["exceptions"]
                .as_array()
                .expect("exceptions")
                .iter()
                .map(|e| {
                    (
                        e["n"].as_u64().expect("n") as usize,
                        hex(&e["correctly_rounded"]),
                        hex(&e["other"]),
                    )
                })
                .collect(),
        })
        .collect()
}

/// The generator's table for `r.config` against `r`: each entry the correctly rounded bits, or, where listed, one of
/// the two neighbours and the generator's own rounding of its f64 value.
fn check_host(r: &Reference) {
    let c = r.config;
    let got = threshold_bits(&c).unwrap_or_else(|e| panic!("{c:?} is refused: {e}"));
    assert_eq!(r.bits.len(), L, "the fixture's table length for {c:?}");
    for n in 1..=L {
        let g = got[n - 1];
        match r.exceptions.iter().find(|e| e.0 == n) {
            Some(&(_, cr, other)) => {
                assert!(
                    g == cr || g == other,
                    "THR[{n}] of {c:?}: {g:#010X} is neither listed neighbour"
                );
                let own = (threshold_f64(c.r_sub, c.gamma_sub, n as u32) as f32).to_bits();
                assert_eq!(g, own, "THR[{n}] of {c:?} is not the generator's own rounding");
            }
            None => assert_eq!(
                g,
                r.bits[n - 1],
                "THR[{n}] of {c:?}: the generator's {g:#010X} is not the fixture's correctly rounded {:#010X}",
                r.bits[n - 1]
            ),
        }
    }
}

#[test]
fn nsub_table_host() {
    let refs = references();
    assert!(refs.len() >= 4, "the fixture holds its configurations");
    let d = NsubConfig::DEFAULT;
    let changed = |f: fn(&NsubConfig) -> bool| refs.iter().any(|r| f(&r.config));
    assert!(
        changed(|c| *c == NsubConfig::DEFAULT),
        "the default is in the fixture"
    );
    assert!(
        changed(|c| c.r_sub != R_SUB_DEFAULT) && changed(|c| c.gamma_sub != GAMMA_SUB_DEFAULT),
        "r_sub and gamma_sub are each changed"
    );
    assert!(
        refs.iter()
            .any(|r| r.config.n_max != d.n_max && !r.config.n_max.is_power_of_two()),
        "a non-power-of-two N_max"
    );
    for r in &refs {
        check_host(r);
    }
}

negative_control!(
    nsub_table_host,
    "a fixture entry one ulp off",
    expected = "is not the fixture's correctly rounded",
    {
        let mut r = references().remove(0);
        r.bits[10] += 1;
        check_host(&r);
    }
);

/// The listed-exception arm: an entry listed with the generator's bits as either neighbour passes; one listed with
/// neither fails.
#[test]
fn nsub_table_host_listed_exception() {
    let mut r = references().remove(0);
    let g = r.bits[4];
    r.bits[4] = g + 1;
    r.exceptions = vec![(5, g + 1, g)];
    check_host(&r);
    r.exceptions = vec![(5, g, g - 1)];
    check_host(&r);
}

negative_control!(
    nsub_table_host_listed_exception,
    "an exception listing neither of the generator's neighbours",
    expected = "is neither listed neighbour",
    {
        let mut r = references().remove(0);
        let g = r.bits[4];
        r.exceptions = vec![(5, g + 1, g + 2)];
        check_host(&r);
    }
);

/// `c` is refused with `want`, whose message names it. Compared by their `Debug` text, so a NaN field compares.
fn check_refused(c: NsubConfig, want: NsubTableError, says: &str) {
    let want_text = format!("{:?}", Err::<(), _>(want));
    assert_eq!(
        format!("{:?}", threshold_bits(&c).map(|_| ())),
        want_text,
        "{c:?} is not refused as {want:?}"
    );
    assert_eq!(
        format!("{:?}", thresholds(&c).map(|_| ())),
        want_text,
        "{c:?} packs though refused"
    );
    let message = want.to_string();
    assert!(
        message.contains(says),
        "{want:?} says `{message}`, not `{says}`"
    );
}

#[test]
fn nsub_table_host_refusals() {
    let l = L as u32;
    let d = NsubConfig::DEFAULT;
    check_refused(
        NsubConfig { n_max: l + 1, ..d },
        NsubTableError::NMaxAboveTable { n_max: l + 1 },
        "N_max 257 is above the threshold table's maximum length, 256",
    );
    check_refused(
        NsubConfig { n_max: 0, ..d },
        NsubTableError::NMaxBelowOne,
        "N_max is 0",
    );
    for n_max in [1, l] {
        assert!(
            threshold_bits(&NsubConfig { n_max, ..d }).is_ok(),
            "N_max {n_max} is refused"
        );
    }
    for r_sub in [0.0, -0.05, f64::NAN, f64::INFINITY] {
        check_refused(
            NsubConfig { r_sub, ..d },
            NsubTableError::RSub { r_sub },
            "is not finite and positive",
        );
    }
    for gamma_sub in [0.0, -1.5, f64::NAN, f64::INFINITY] {
        check_refused(
            NsubConfig { gamma_sub, ..d },
            NsubTableError::GammaSub { gamma_sub },
            "gamma_sub",
        );
    }
    // The least positive r_sub is accepted where no threshold decides (N_max = 1); at N_max = 2, THR[1] = 0 decides.
    assert!(
        threshold_bits(&config(f64::MIN_POSITIVE, 1.5, 1)).is_ok(),
        "a tiny r_sub at N_max 1"
    );
    check_refused(
        config(1e-25, 1.5, 2),
        NsubTableError::NotNormal { n: 1, bits: 0 },
        "THR[1] (bits 0x00000000) decides N_sub and is not a normal f32",
    );
    // A large gamma_sub gives equal neighbours, which do not rise.
    assert!(
        threshold_bits(&config(0.05, 1e12, l)).is_ok(),
        "equal thresholds are refused"
    );
}

negative_control!(
    nsub_table_host_refusals,
    "a valid sim key checked as refused",
    expected = "is not refused as",
    check_refused(
        NsubConfig::DEFAULT,
        NsubTableError::NMaxBelowOne,
        "N_max is 0"
    )
);

/// A table of `head` then zeros.
fn table_of(head: &[u32]) -> [u32; L] {
    let mut t = [0; L];
    t[..head.len()].copy_from_slice(head);
    t
}

#[test]
fn nsub_table_host_check_table() {
    let least = 0x0080_0000; // 2⁻¹²⁶, the least normal
    let max = 0x7F7F_FFFF;
    let inf = 0x7F80_0000;
    assert_eq!(check_table(&table_of(&[least + 1, least]), 3), Ok(()));
    assert_eq!(
        check_table(&table_of(&[least + 1, least]), 4),
        Err(NsubTableError::NotNormal { n: 3, bits: 0 })
    );
    assert_eq!(
        check_table(&table_of(&[least, least - 1]), 3),
        Err(NsubTableError::NotNormal {
            n: 2,
            bits: least - 1
        })
    );
    assert_eq!(check_table(&table_of(&[max, max]), 3), Ok(()));
    assert_eq!(
        check_table(&table_of(&[inf, max]), 2),
        Err(NsubTableError::NotNormal { n: 1, bits: inf })
    );
    assert_eq!(
        check_table(&table_of(&[least]), L as u32),
        Err(NsubTableError::NotNormal { n: 2, bits: 0 })
    );
    assert_eq!(
        check_table(&table_of(&[least, least + 1]), 1),
        Err(NsubTableError::Rises { n: 2 })
    );
    let mut rise_last = [least; L];
    rise_last[L - 1] = least + 1;
    assert_eq!(
        check_table(&rise_last, L as u32),
        Err(NsubTableError::Rises { n: L as u32 })
    );
    assert_eq!(check_table(&[least; L], L as u32), Ok(()));
    assert!(NsubTableError::Rises { n: 7 }
        .to_string()
        .contains("rise at THR[7]"));
}

negative_control!(
    nsub_table_host_check_table,
    "a rising table checked as accepted",
    expected = "assertion",
    assert_eq!(check_table(&table_of(&[1, 2]), 1), Ok(()))
);

/// `t` holds `bits` at `SimUniforms`' layout: `THR[n]` at row `(n − 1)/4`, lane `(n − 1) mod 4`.
fn check_pack(t: &NsubThresholds, bits: &[u32; L]) {
    assert_eq!(t.rows.len(), NSUB_TABLE_ROWS, "the rows");
    for n in 1..=L {
        assert_eq!(
            t.rows[(n - 1) / 4][(n - 1) % 4],
            bits[n - 1],
            "THR[{n}] is not at its row and lane"
        );
    }
}

#[test]
fn nsub_table_host_pack() {
    let bits = threshold_bits(&NsubConfig::DEFAULT).expect("generates");
    check_pack(&pack(&bits), &bits);
    check_pack(&default_table(), &bits);
}

negative_control!(
    nsub_table_host_pack,
    "two lanes swapped",
    expected = "is not at its row and lane",
    {
        let bits = threshold_bits(&NsubConfig::DEFAULT).expect("generates");
        let mut t = pack(&bits);
        let r = t.rows[3];
        t.rows[3] = spirv_std::glam::UVec4::new(r.y, r.x, r.z, r.w);
        check_pack(&t, &bits);
    }
);

// ── REQ-INT-092: the default sim key reproduces the build-time table ────────────────────────────────────────────

fn check_default(build_time: &[u32; L]) {
    let got = threshold_bits(&NsubConfig::DEFAULT).expect("the default sim key generates");
    for n in 1..=L {
        assert_eq!(
            got[n - 1],
            build_time[n - 1],
            "THR[{n}]: the host generator's bits differ from the build-time table's"
        );
    }
}

#[test]
fn nsub_default_table_bitwise() {
    assert_eq!(
        NsubConfig::DEFAULT,
        config(0.05, 1.5, 64),
        "the default sim key"
    );
    assert_eq!(N_MAX_DEFAULT, 64);
    check_default(&DEFAULT_THRESHOLD_BITS);
}

negative_control!(
    nsub_default_table_bitwise,
    "the build-time table with one bit flipped",
    expected = "differ from the build-time table's",
    {
        let mut t = DEFAULT_THRESHOLD_BITS;
        t[17] ^= 1;
        check_default(&t);
    }
);

// ── The tree (REQ-INT-028 at the unit level) ─────────────────────────────────────────────────────────────────────

/// The table's own count: `min(1 + #{m : d2 < THR[m]}, n_max)`.
fn count(bits: &[u32; L], d2: f32, n_max: u32) -> u32 {
    let k = bits.iter().filter(|&&b| d2 < f32::from_bits(b)).count() as u32;
    (k + 1).min(n_max)
}

/// At every threshold, its value and its two neighbours, and at `0`, `+∞`, NaN and the least positive f32, the tree's
/// `N_sub` through `table` is `bits`' count.
fn check_tree(table: &NsubThresholds, bits: &[u32; L], n_max: u32) {
    let mut d2s = vec![0.0, f32::INFINITY, f32::NAN, f32::from_bits(1), f32::MAX];
    for &b in bits {
        let t = f32::from_bits(b);
        d2s.extend([below(t), t, above(t)]);
    }
    for d2 in d2s {
        assert_eq!(
            n_sub(d2, table, n_max),
            count(bits, d2, n_max),
            "the tree at d² {d2:e} (N_max {n_max}) is not the table's count"
        );
    }
}

#[test]
fn nsub_bucket_tree() {
    for n_max in [1, 50, 64, L as u32] {
        let c = NsubConfig {
            n_max,
            ..NsubConfig::DEFAULT
        };
        let bits = threshold_bits(&c).expect("generates");
        check_tree(&pack(&bits), &bits, n_max);
    }
    let bits = threshold_bits(&config(0.02, 1.0, 100)).expect("generates");
    check_tree(&pack(&bits), &bits, 100);
    // Exactly at an edge `d² = THR[n]` is bucket `n`; the f32 below is `n + 1` (the default table strictly decreases).
    let t = default_table();
    let l = L as u32;
    for n in 1..=l {
        let thr = f32::from_bits(DEFAULT_THRESHOLD_BITS[n as usize - 1]);
        assert_eq!(t.bucket(thr), n, "d² = THR[{n}]");
        assert_eq!(t.bucket(below(thr)), n + 1, "d² just below THR[{n}]");
    }
    assert_eq!(t.bucket(0.0), l + 1, "d² = 0");
    assert_eq!(t.bucket(f32::INFINITY), 1, "d² = +∞");
    assert_eq!(t.bucket(f32::NAN), 1, "NaN");
    assert_eq!(n_sub(0.0, &t, 64), 64, "the cap");
}

negative_control!(
    nsub_bucket_tree,
    "a table whose lanes the tree reads in the wrong order",
    expected = "is not the table's count",
    {
        let bits = threshold_bits(&NsubConfig::DEFAULT).expect("generates");
        let mut t = pack(&bits);
        let r = t.rows[5];
        t.rows[5] = spirv_std::glam::UVec4::new(r.w, r.z, r.y, r.x);
        check_tree(&t, &bits, L as u32);
    }
);

// ── The quantised d² ────────────────────────────────────────────────────────────────────────────────────────────

/// `square`'s floor and form, and the closest pair of three.
fn check_d2(sq: fn(f32) -> f32) {
    let floor = f32::from_bits(0x2B80_0000); // 2⁻⁴⁰
    assert_eq!(
        sq(floor),
        f32::from_bits(0x1780_0000),
        "2⁻⁴⁰ squared is 2⁻⁸⁰"
    );
    assert_eq!(sq(-floor), sq(floor), "the square is even");
    assert_eq!(sq(below(floor)), 0.0, "below the floor the square is 0");
    assert_eq!(sq(f32::from_bits(1)), 0.0, "a subnormal's square is 0");
    assert_eq!(sq(3.0), 9.0);
    let x = 1.0 + f32::EPSILON;
    assert!(
        (f64::from(sq(x)) - f64::from(x) * f64::from(x)).abs() <= 2.0 * f64::from(f32::EPSILON),
        "the split product is within an ulp or two of x²"
    );
    assert!(sq(f32::NAN).is_nan());
}

#[test]
fn nsub_d2_quantised() {
    check_d2(square);
    // Each pair closest in turn: pair k is the side opposite body k.
    let far = [[0.0_f32, 0.0], [3.0, 0.0], [0.0, 4.0]];
    assert_eq!(
        d2_quantised(&far),
        9.0,
        "pair 2, bodies 0 and 1, the closest"
    );
    assert_eq!(
        d2_quantised(&[[0.0_f32, 0.0], [3.0, 0.0], [3.0, 2.0]]),
        4.0,
        "pair 0"
    );
    assert_eq!(
        d2_quantised(&[[0.0_f32, 0.0], [5.0, 0.0], [1.0, 1.0]]),
        2.0,
        "pair 1"
    );
    // Off the axes, so each subtraction's operands matter: pair 0 is (0, 2) apart, pair 1 (−3, −2), pair 2 (3, 0).
    assert_eq!(
        d2_quantised(&[[0.0_f32, 1.0], [3.0, 1.0], [3.0, 3.0]]),
        4.0,
        "pair 0 off the axes"
    );
    // The f64 kernel's d² is the f32 kernel's on the f32 casts: positions with detail below f32's resolution.
    let r: [[f64; 2]; 3] = [
        [0.1, -0.2 + 1e-12],
        [0.100_123_456_789, -0.2],
        [5.0 + 1e-13, 5.0],
    ];
    let r32 = r.map(|p| p.map(|x| x as f32));
    assert_eq!(
        d2_quantised(&r).to_bits(),
        d2_quantised(&r32).to_bits(),
        "f64 against f32"
    );
    assert_eq!(0.1_f64.quantise(), 0.1_f32);
    assert_eq!(0.1_f32.quantise(), 0.1_f32);
}

negative_control!(
    nsub_d2_quantised,
    "a plain square, with no floor",
    expected = "below the floor the square is 0",
    check_d2(|x| x * x)
);

// ── REQ-INT-027: the live N_sub readout ─────────────────────────────────────────────────────────────────────────

/// Each macro-step's readout equals the bucket value used for it: `(used, read)` per step.
fn check_readout(steps: &[(u32, u32)]) {
    for (i, &(used, read)) in steps.iter().enumerate() {
        assert_eq!(
            read, used,
            "macro-step {i}: the live N_sub {read} is not the bucket value {used} used"
        );
    }
}

/// A `d²` sequence that walks through buckets, up to the cap and back.
fn walk() -> Vec<f32> {
    let thr = |n: usize| f32::from_bits(DEFAULT_THRESHOLD_BITS[n - 1]);
    vec![
        1.0,
        thr(1),
        below(thr(1)),
        below(thr(7)),
        thr(63),
        below(thr(63)),
        0.0,
        below(thr(20)),
        1.0,
    ]
}

/// Runs `walk` through a record, giving each step's bucket value and readout.
fn readouts(n_max: u32) -> Vec<(u32, u32)> {
    let t = default_table();
    let mut s = SubstepState::default();
    walk()
        .into_iter()
        .map(|d2| {
            let used = s.macro_step(d2, &t, n_max);
            assert_eq!(
                used,
                n_sub(d2, &t, n_max),
                "the macro-step's N_sub is not the bucket's"
            );
            (used, s.live_n_sub)
        })
        .collect()
}

#[test]
fn nsub_live_readout() {
    let steps = readouts(64);
    let used: Vec<u32> = steps.iter().map(|s| s.0).collect();
    assert_eq!(used, [1, 1, 2, 8, 63, 64, 64, 21, 1], "the walk's buckets");
    check_readout(&steps);
}

negative_control!(
    nsub_live_readout,
    "a readout one macro-step late",
    expected = "is not the bucket value",
    {
        let steps = readouts(64);
        let late: Vec<(u32, u32)> = steps
            .iter()
            .enumerate()
            .map(|(i, s)| (s.0, if i == 0 { s.1 } else { steps[i - 1].1 }))
            .collect();
        check_readout(&late);
    }
);

// ── REQ-TOOL-033: saturated is sticky and set iff N_sub == N_max occurred ───────────────────────────────────────

/// Per step `(N_sub, saturated)`: the flag is set exactly from the first step at `n_max` on.
fn check_sticky(steps: &[(u32, bool)], n_max: u32) {
    let mut seen = false;
    for (i, &(n, flag)) in steps.iter().enumerate() {
        seen |= n == n_max;
        assert_eq!(
            flag, seen,
            "macro-step {i} (N_sub {n}, N_max {n_max}): saturated is {flag}"
        );
    }
}

/// `d2s` through a record at `n_max`: each step's `N_sub` and `saturated`.
fn flags(d2s: &[f32], n_max: u32) -> Vec<(u32, bool)> {
    let t = default_table();
    let mut s = SubstepState::default();
    d2s.iter()
        .map(|&d2| (s.macro_step(d2, &t, n_max), s.saturated))
        .collect()
}

#[test]
fn saturated_sticky() {
    let thr = |n: usize| f32::from_bits(DEFAULT_THRESHOLD_BITS[n - 1]);
    // Hits the cap of 64 once, then relaxes.
    let once = flags(&[1.0, thr(63), below(thr(63)), thr(10), 1.0], 64);
    assert_eq!(
        once,
        [(1, false), (63, false), (64, true), (10, true), (1, true)]
    );
    check_sticky(&once, 64);
    // N_max = 50, not a power of two: set exactly when N_sub reaches 50, not at 49 nor at a power of two.
    let fifty = flags(&[below(thr(31)), below(thr(48)), below(thr(49)), 1.0], 50);
    assert_eq!(fifty, [(32, false), (49, false), (50, true), (1, true)]);
    check_sticky(&fifty, 50);
    // The descriptor bit: bit 5 of packed_a, every other bit kept.
    let mut s = SubstepState::default();
    let t = default_table();
    assert!(!sd_saturated(s.packed_a(0)));
    assert_eq!(
        s.packed_a(0xFFFF_FFFF),
        0xFFFF_FFDF,
        "an unsaturated run clears bit 5 only"
    );
    s.macro_step(0.0, &t, 50);
    s.macro_step(1.0, &t, 50);
    assert!(sd_saturated(s.packed_a(0)), "a saturated run sets bit 5");
    assert_eq!(s.packed_a(0), 1 << 5, "bit 5 alone");
}

negative_control!(
    saturated_sticky,
    "a flag that clears when the cap relaxes",
    expected = "saturated is false",
    check_sticky(&[(64, true), (1, false)], 64)
);

// ── REQ-TOOL-034: total_substeps and its log proxy ──────────────────────────────────────────────────────────────

/// `⌊log₂ t⌋` by halving, 0 for `t ≤ 1`.
fn floor_log2(mut t: u32) -> u32 {
    let mut k = 0;
    while t > 1 {
        t /= 2;
        k += 1;
    }
    k
}

/// `proxy` is `⌊log₂⌋` at every power of two and its neighbours, and 0 at 0 and 1.
fn check_proxy(proxy: fn(u32) -> u32) {
    assert_eq!(proxy(0), 0, "the proxy of 0");
    assert_eq!(proxy(1), 0, "the proxy of 1");
    for k in 1..32 {
        let p = 1u32 << k;
        for t in [p - 1, p, p + 1] {
            assert_eq!(proxy(t), floor_log2(t), "the proxy of {t}");
        }
    }
    assert_eq!(proxy(u32::MAX), 31, "the proxy of u32::MAX");
}

/// `n_subs` through a record at `N_max = L_max`, giving the exact total and the checks on it.
fn total_of(d2s: &[f32]) -> (u32, u64) {
    let t = default_table();
    let mut s = SubstepState::default();
    let mut sum = 0u64;
    for &d2 in d2s {
        sum += u64::from(s.macro_step(d2, &t, L as u32));
    }
    (s.total_substeps, sum)
}

#[test]
fn total_substeps_proxy() {
    check_proxy(total_substeps_log2);
    let thr = |n: usize| f32::from_bits(DEFAULT_THRESHOLD_BITS[n - 1]);
    // 4 × 256 = 1024, a power of two, and its neighbours 1023 = 3 × 256 + 255 and 1025 = 1024 + 1.
    for (d2s, want, log) in [
        (vec![0.0; 4], 1024, 10),
        (vec![0.0, 0.0, 0.0, thr(255)], 1023, 9),
        (vec![0.0, 0.0, 0.0, 0.0, 1.0], 1025, 10),
        (vec![], 0, 0),
        (vec![1.0], 1, 0),
    ] {
        let (total, sum) = total_of(&d2s);
        assert_eq!(u64::from(total), sum, "total_substeps is Σ N_sub");
        assert_eq!(total, want);
        assert_eq!(total_substeps_log2(total), log, "the proxy of {total}");
    }
    // The largest run the table allows, 65535 macro-steps at N_max = L_max, fits the u32 exactly.
    let mut s = SubstepState::default();
    let t = default_table();
    for _ in 0..65535 {
        s.macro_step(0.0, &t, L as u32);
    }
    assert_eq!(u64::from(s.total_substeps), 65535 * L as u64);
}

negative_control!(
    total_substeps_proxy,
    "a proxy one above ⌊log₂⌋",
    expected = "the proxy of",
    check_proxy(|t| 32 - t.leading_zeros())
);
