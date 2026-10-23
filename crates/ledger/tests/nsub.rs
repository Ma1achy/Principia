//! The `N_sub` emitter (`ledger::gen::nsub`; dd_integrator §3.3, R-502; REQ-INT-028, REQ-INT-092):
//! - `nsub_generated_is_emitted`: the checked-in `crates/kernel/src/generated/nsub_thresholds.rs` is the emitter's
//!   output, byte for byte, and the driver's emitters write it.
//! - `nsub_emitted_table`: the default table is `THR[n] = round_f32((r_sub·n^{−1/γ_sub})²)` at the register's defaults,
//!   the f64 value rounded once, `L_max` of them, the first `0.05²` exactly rounded, non-increasing; the sizes are the
//!   register's `L_max` and `⌈L_max/4⌉` rows.
//! - `nsub_emitted_tree`: the tree reads each threshold `n = 1 … L_max` once, at row `(n − 1)/4` and lane
//!   `(n − 1) mod 4`, has the `L_max + 1` leaves `1 … L_max + 1`, each once, and no loop.

use ledger::constants::{GAMMA_SUB, NSUB_TABLE_MAX, N_MAX, R_SUB};
use ledger::gen::nsub::{
    default_bits, emit, nsub_file, rows, table_max, threshold_f64, tree, NSUB_PATH,
};
use validation::negative_control;

/// The checked-in file.
fn on_disk() -> String {
    std::fs::read_to_string(format!("{}/../../{NSUB_PATH}", env!("CARGO_MANIFEST_DIR")))
        .expect("read the generated N_sub file")
}

/// Checks that `file` is the emitter's output, and that the driver's emitters write the one path.
fn check_emitted(file: &str) {
    let emitted = emit(&[], &[]);
    assert_eq!(emitted.len(), 1, "the emitter writes one file");
    assert_eq!(
        emitted[0].path.to_str(),
        Some(NSUB_PATH),
        "the emitted path"
    );
    assert!(
        emitted[0].contents == file,
        "the checked-in nsub_thresholds.rs is not the emitter's output"
    );
    assert_eq!(nsub_file(), emitted[0].contents, "the file is nsub_file()");
    let all = ledger::gen::generate(&ledger::layout(), ledger::gen::EMITTERS).expect("generates");
    assert!(
        all.iter().any(|g| g.path.to_str() == Some(NSUB_PATH)),
        "the driver's emitters do not write {NSUB_PATH}"
    );
}

#[test]
fn nsub_generated_is_emitted() {
    check_emitted(&on_disk());
}

negative_control!(
    nsub_generated_is_emitted,
    "a checked-in file with one threshold's last digit changed",
    expected = "the checked-in nsub_thresholds.rs is not the emitter's output",
    check_emitted(&on_disk().replacen("0x3B23_D70A", "0x3B23_D70B", 1))
);

/// `bits` is the default table: `L_max` entries, each the register defaults' f64 value rounded once to f32, by `thr`.
fn check_table(bits: &[u32], thr: fn(f64, f64, u32) -> f64) {
    assert_eq!(
        table_max() as f64,
        NSUB_TABLE_MAX.number(),
        "L_max is the register's"
    );
    assert_eq!(table_max(), 256);
    assert_eq!(rows(), 64, "⌈L_max/4⌉ rows");
    assert_eq!(bits.len(), table_max() as usize, "the table's length");
    let (r, g) = (R_SUB.number(), GAMMA_SUB.number());
    assert_eq!((r, g, N_MAX.number()), (0.05, 1.5, 64.0), "the defaults");
    for (i, &b) in bits.iter().enumerate() {
        let n = i as u32 + 1;
        let s = r * (f64::from(n)).powf(-1.0 / g);
        let want = (thr(r, g, n) as f32).to_bits();
        assert_eq!(b, want, "THR[{n}] is not its f64 value rounded once");
        // Against std's powf, an independent f64 pow, to within an f32 ulp.
        assert!(
            (f64::from(f32::from_bits(b)) / (s * s) - 1.0).abs() < f64::from(f32::EPSILON),
            "THR[{n}] is not (r_sub·n^(−1/γ_sub))²"
        );
        if i > 0 {
            assert!(
                f32::from_bits(b) <= f32::from_bits(bits[i - 1]),
                "THR[{n}] rises"
            );
        }
    }
    assert_eq!(
        bits[0],
        ((0.05f64 * 0.05) as f32).to_bits(),
        "THR[1] = r_sub²"
    );
}

#[test]
fn nsub_emitted_table() {
    check_table(&default_bits(), threshold_f64);
}

negative_control!(
    nsub_emitted_table,
    "a law that drops the square, r_sub·n^(−1/γ_sub)",
    expected = "is not its f64 value rounded once",
    check_table(&default_bits(), |r, g, n| r * f64::from(n).powf(-1.0 / g))
);

/// `t`, the tree's body, reads each threshold once at its row and lane and returns each leaf `1 … L_max + 1` once.
fn check_tree(t: &str) {
    let l = table_max();
    for n in 1..=l {
        let read = format!(
            "d2 < f32::from_bits(thresholds[{}].{}) {{",
            (n - 1) / 4,
            ["x", "y", "z", "w"][((n - 1) % 4) as usize]
        );
        assert_eq!(
            t.matches(&read).count(),
            1,
            "THR[{n}] is not read once: `{read}`"
        );
    }
    let leaves: Vec<u32> = t
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect();
    let mut sorted = leaves.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        (1..=l + 1).collect::<Vec<_>>(),
        "the leaves are not 1 … L_max + 1, each once"
    );
    assert!(
        !t.contains("for ") && !t.contains("while ") && !t.contains("loop"),
        "the tree loops"
    );
}

#[test]
fn nsub_emitted_tree() {
    check_tree(&tree());
    assert!(nsub_file().contains(&tree()), "the file holds the tree");
}

negative_control!(
    nsub_emitted_tree,
    "a tree with one leaf repeated",
    expected = "the leaves are not 1 … L_max + 1",
    check_tree(&tree().replacen("        257\n", "        256\n", 1))
);
