//! The libm-pin measurement's reading and counting (`xtask::libm_pin`; R-991, REQ-CHART-212):
//! - `libm_pin_parse`: a run's file reads back to its header and digests; a wrong header or a short body is refused.
//! - `libm_pin_compare`: two runs' records are counted apart at f64 (the digest's high half) and after narrowing (its
//!   low half), per chart and zoom, and the table prints them; runs over different grids are refused.
//! - `libm_pin_outcome`: R-991's criterion: void where the control changes no record at f64, PASS where no other
//!   machine's pinned run differs at either level, UNEXPLAINED naming each machine where one does.

use xtask::libm_pin::{
    compare, outcome, parse, table, totals, Count, Header, Outcome, Run, CHARTS, ZOOMS,
};

use validation::negative_control;

/// A run's file: its header line and `digests`.
fn file(header: &str, digests: &[u64]) -> Vec<u8> {
    let mut bytes = format!("{header}\n").into_bytes();
    bytes.extend(digests.iter().flat_map(|d| d.to_le_bytes()));
    bytes
}

/// A run on a `2 × 1` grid, every digest `0x0000_0001_0000_0002` but those `set` gives.
fn run(host: &str, set: &[(usize, u64)]) -> Run {
    let mut digests = vec![0x0000_0001_0000_0002; CHARTS.len() * ZOOMS.len() * 2];
    for &(i, d) in set {
        digests[i] = d;
    }
    Run {
        header: Header {
            arm: "pinned".to_owned(),
            host: host.to_owned(),
            width: 2,
            height: 1,
        },
        digests,
    }
}

/// Checks `read`, a run's parser.
fn check_parse(read: fn(&[u8]) -> Result<Run, String>) {
    let digests: Vec<u64> = (0..18).map(|k| k * 0x1_0000_0001).collect();
    let back = read(&file("libm-pin 1 std linux-x86_64 2 1", &digests)).expect("a run");
    assert_eq!(
        back.header,
        Header {
            arm: "std".to_owned(),
            host: "linux-x86_64".to_owned(),
            width: 2,
            height: 1
        },
        "the header"
    );
    assert_eq!(back.digests, digests, "the digests, in order");
    for (bad, what) in [
        (
            file("libm-pin 1 std linux-x86_64 2 1", &digests[..17]),
            "a short body",
        ),
        (
            file(
                "libm-pin 1 std linux-x86_64 2 1",
                &[digests.clone(), vec![0]].concat(),
            ),
            "a long body",
        ),
        (
            file("libm-pin 2 std linux-x86_64 2 1", &digests),
            "version 2",
        ),
        (
            file("libm-pinx 1 std linux-x86_64 2 1", &digests),
            "another magic",
        ),
        (
            file("libm-pin 1 std linux-x86_64 2", &digests),
            "five words",
        ),
        (
            file("libm-pin 1 std linux-x86_64 two 1", &digests),
            "a width not a number",
        ),
        (b"libm-pin 1 std linux-x86_64 2 1".to_vec(), "no newline"),
    ] {
        assert!(read(&bad).is_err(), "parse refuses {what}");
    }
}

#[test]
fn libm_pin_parse() {
    check_parse(parse);
}

negative_control!(
    libm_pin_parse,
    "a parser that keeps the body's whole bytes whatever their length",
    expected = "parse refuses a short body",
    check_parse(|bytes| {
        let mut padded = bytes.to_vec();
        let full = bytes.iter().position(|&b| b == b'\n').map_or(0, |n| n + 1) + 18 * 8;
        padded.resize(full.max(padded.len()), 0);
        parse(&padded)
    })
);

/// Checks `count`, two runs' comparison.
fn check_compare(count: fn(&Run, &Run) -> Result<Vec<Count>, String>) {
    let a = run("macos-aarch64", &[]);
    // Record 0 (latent at 2^0, x = 0) differs at f64 only, record 3 (latent at 2^10, x = 1) after narrowing only, and
    // record 17 (the last chart's 2^20, x = 1) at both.
    let b = run(
        "linux-x86_64",
        &[
            (0, 0x0000_0009_0000_0002),
            (3, 0x0000_0001_0000_0009),
            (17, 0x7),
        ],
    );
    let counts = count(&a, &b).expect("one grid");
    assert_eq!(
        counts.len(),
        CHARTS.len() * ZOOMS.len(),
        "one count per chart and zoom"
    );
    let at = |chart, zoom| {
        counts
            .iter()
            .find(|c| (c.chart, c.zoom) == (chart, zoom))
            .copied()
            .expect("a count")
    };
    assert_eq!(
        (at(0, 0).wide, at(0, 0).narrow, at(0, 0).of),
        (1, 0, 2),
        "latent at 2^0"
    );
    assert_eq!((at(0, 1).wide, at(0, 1).narrow), (0, 1), "latent at 2^10");
    assert_eq!(
        (at(2, 2).wide, at(2, 2).narrow),
        (1, 1),
        "the last chart at 2^20"
    );
    assert_eq!(totals(&counts), (2, 2), "the totals");
    assert_eq!(
        totals(&count(&a, &a).expect("one grid")),
        (0, 0),
        "a run against itself"
    );
    let text = table("pinned linux-x86_64 against macos-aarch64", &counts);
    assert!(
        text.starts_with("pinned linux-x86_64 against macos-aarch64\n"),
        "the title: {text}"
    );
    assert!(
        text.contains("latent                     2^0   f64         1  narrowed         0  of 2"),
        "a row: {text}"
    );
    assert!(
        text.contains("invariant-lz-e             2^20  f64         1  narrowed         1  of 2"),
        "a row: {text}"
    );
    assert!(
        text.contains("total                            f64         2  narrowed         2"),
        "the total: {text}"
    );
    let mut wider = run("windows-x86_64", &[]);
    wider.header.width = 1;
    wider.header.height = 2;
    assert!(
        count(&a, &wider).is_err(),
        "runs over different grids are refused"
    );
    let mut shorter = run("windows-x86_64", &[]);
    shorter.digests.pop();
    assert!(
        count(&a, &shorter).is_err(),
        "runs of different lengths are refused"
    );
}

#[test]
fn libm_pin_compare() {
    check_compare(compare);
}

negative_control!(
    libm_pin_compare,
    "a comparison that reads the narrowed level from the f64 half",
    expected = "latent at 2^0",
    check_compare(|a, b| {
        compare(a, b).map(|counts| {
            counts
                .into_iter()
                .map(|c| Count {
                    narrow: c.wide,
                    ..c
                })
                .collect()
        })
    })
);

/// `n` counts, every one zero but the first's `(wide, narrow)`.
fn counts(first: (usize, usize)) -> Vec<Count> {
    (0..CHARTS.len() * ZOOMS.len())
        .map(|k| Count {
            chart: k / ZOOMS.len(),
            zoom: k % ZOOMS.len(),
            wide: if k == 0 { first.0 } else { 0 },
            narrow: if k == 0 { first.1 } else { 0 },
            of: 2,
        })
        .collect()
}

/// An outcome function: the control's counts and each other machine's, named by its host.
type Judge = fn(&[Count], &[(String, Vec<Count>)]) -> Outcome;

/// Checks `judge`, the outcome.
fn check_outcome(judge: Judge) {
    let clean = |host: &str| (host.to_owned(), counts((0, 0)));
    let all_clean = [
        clean("linux-x86_64"),
        clean("windows-x86_64"),
        clean("wasm32-node-22"),
    ];
    assert_eq!(
        judge(&counts((1, 0)), &all_clean),
        Outcome::Pass,
        "control 1 at f64, every machine 0"
    );
    assert_eq!(
        judge(&counts((0, 1)), &all_clean),
        Outcome::Void,
        "control 0 at f64, 1 narrowed"
    );
    assert_eq!(
        judge(&counts((0, 0)), &all_clean),
        Outcome::Void,
        "control 0"
    );
    let narrowed = [
        clean("linux-x86_64"),
        ("windows-x86_64".to_owned(), counts((0, 1))),
    ];
    assert_eq!(
        judge(&counts((5, 0)), &narrowed),
        Outcome::Unexplained(vec!["windows-x86_64".to_owned()]),
        "one narrowed record on Windows"
    );
    let wide = [
        ("linux-x86_64".to_owned(), counts((1, 0))),
        ("wasm32-node-22".to_owned(), counts((2, 2))),
    ];
    assert_eq!(
        judge(&counts((5, 0)), &wide),
        Outcome::Unexplained(vec!["linux-x86_64".to_owned(), "wasm32-node-22".to_owned()]),
        "f64 records on Linux and wasm"
    );
    assert_eq!(
        judge(&counts((1, 0)), &[]),
        Outcome::Pass,
        "no other machine: PASS for the arms that ran"
    );
}

#[test]
fn libm_pin_outcome() {
    check_outcome(outcome);
}

negative_control!(
    libm_pin_outcome,
    "an outcome that reads the control after narrowing, where a one-ulp sin flips almost nothing",
    expected = "control 1 at f64, every machine 0",
    check_outcome(|control, pinned| {
        let narrowed: Vec<Count> = control
            .iter()
            .map(|c| Count {
                wide: c.narrow,
                ..*c
            })
            .collect();
        outcome(&narrowed, pinned)
    })
);
