//! The mock's outcome legend is the recorded one, `evaluate(&default_stain())`'s entries pasted as written.

use engine::contract::legend::{LegendKey, SwatchStyle};
use gui::mock::legend::{default_stain, evaluate};

#[rustfmt::skip]
const RECORDED: [(&str, [u8; 3], f64); 9] = [
    ("bounded / nothing yet", [20, 20, 24], 31.2),
    ("collision 1–2", [222, 45, 45], 6.4),
    ("collision 1–3", [46, 188, 78], 5.9),
    ("collision 2–3", [52, 98, 224], 6.3),
    ("triple collision", [242, 150, 32], 0.9),
    ("escape body 1", [240, 222, 50], 11.0),
    ("escape body 2", [224, 52, 198], 24.8),
    ("escape body 3", [48, 200, 220], 12.1),
    ("triple ejection", [97, 21, 151], 1.4),
];

/// The outcome key's entries: label, flat colour and share.
fn entries(key: &LegendKey) -> Vec<(String, [u8; 3], f64)> {
    let LegendKey::Categorical { entries, .. } = key else {
        panic!("the first key is the outcome key");
    };
    entries
        .iter()
        .map(|e| match e.swatch {
            SwatchStyle::Flat(c) => (e.label.clone(), c, e.share),
            other => panic!("`{}` is drawn {other:?}, not flat", e.label),
        })
        .collect()
}

fn check(got: &[(String, [u8; 3], f64)]) {
    let want: Vec<(String, [u8; 3], f64)> = RECORDED
        .iter()
        .map(|&(l, c, s)| (l.to_owned(), c, s))
        .collect();
    assert_eq!(got, want, "the outcome legend is not the recorded one");
}

#[test]
fn outcome_legend_is_as_recorded() {
    let legend = evaluate(&default_stain());
    let got = entries(&legend.keys[0]);
    check(&got);
    // The control: one channel of one swatch a step off must be refused.
    let mut off = got.clone();
    off[6].1[2] += 1;
    let refused = std::panic::catch_unwind(|| check(&off)).is_err();
    assert!(refused, "the check passed a legend one step off");
}
