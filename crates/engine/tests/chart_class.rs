//! The samples' chart classification, CPU-side (REQ-COL-102; colour_composition §3; TASK-M2-32): `chart_class`
//! classifies a chart point inside `[0,1]²`, its edges included, as forbidden where `validate` returns anything but
//! `Valid` or `Φ` (`Chart::map`) fails to evaluate, and refuses a point outside the square, the extension types' (TASK-M8-05);
//! `fill` writes each sample's classification into a synthetic set, whose bytes carry it as one bit per sample
//! (`render::bind::chart_words`). Each test registers its negative control (R-176).

use engine::chart_class::{fill, forbidden};
use engine::synthetic::Synthetic;
use kernel::chart::{Chart, ChartOut, Constraint, Float, Latent, ValidationResult};
use render::raster::Grid;
use validation::negative_control;

/// A test chart that returns `result` past `u = from`, `Valid` before it, and is the latent chart pinned at `z0`.
struct Walled {
    latent: Latent<f64>,
    from: f64,
    result: ValidationResult,
}

impl Chart for Walled {
    type Param = f64;
    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> ChartOut<F> {
        self.latent.map(u, v)
    }
    fn name(&self) -> &str {
        "walled"
    }
    fn validate(&self, u: f64, _v: f64) -> ValidationResult {
        if u > self.from {
            self.result
        } else {
            ValidationResult::Valid
        }
    }
}

const REJECT: ValidationResult = ValidationResult::Reject {
    failed: Constraint::MassPositive,
};

fn walled(z0: [f64; 8], result: ValidationResult) -> Walled {
    Walled {
        latent: Latent {
            z0,
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
        from: 0.5,
        result,
    }
}

/// Checks `classify` against REQ-COL-102 on the walled chart: inside the square, its edges included, forbidden exactly
/// past the wall, for each non-`Valid` result, and wherever `Φ` gives a non-finite component; `None` just outside.
fn check(classify: &dyn Fn(&Walled, f64, f64) -> Option<bool>) {
    let open = walled([0.0; 8], REJECT);
    for (u, v) in [(0.0, 0.0), (0.0, 1.0), (0.5, 0.5), (0.25, 0.0), (0.25, 1.0)] {
        assert_eq!(
            classify(&open, u, v),
            Some(false),
            "({u}, {v}) before the wall"
        );
    }
    for (u, v) in [(1.0, 1.0), (1.0, 0.0), (0.5f64.next_up(), 0.5)] {
        assert_eq!(
            classify(&open, u, v),
            Some(true),
            "({u}, {v}) past the wall"
        );
    }
    let (below, above) = (0.0f64.next_down(), 1.0f64.next_up());
    for (u, v) in [(below, 0.5), (above, 0.5), (0.25, below), (0.25, above)] {
        assert_eq!(classify(&open, u, v), None, "({u}, {v}) outside [0,1]²");
    }
    for result in [
        ValidationResult::Project {
            u: 0.5,
            v: 0.5,
            failed: Constraint::LzEParabola,
        },
        ValidationResult::Clamp {
            u: 0.5,
            v: 0.5,
            moved: [true, false],
            failed: Constraint::BurrauNu,
        },
    ] {
        assert_eq!(
            classify(&walled([0.0; 8], result), 0.75, 0.5),
            Some(true),
            "{result:?}"
        );
    }
    for k in 0..8 {
        for bad in [f64::NAN, f64::INFINITY] {
            let mut z0 = [0.0; 8];
            z0[k] = bad;
            assert_eq!(
                classify(&walled(z0, REJECT), 0.25, 0.25),
                Some(true),
                "Φ gives z{k} = {bad}"
            );
        }
    }
}

#[test]
fn chart_class_forbidden_where_part_3_hatches() {
    check(&|c, u, v| forbidden(c, u, v));
}

negative_control!(
    chart_class_forbidden_where_part_3_hatches,
    "a classifier reading validate alone misses where Φ fails to evaluate",
    expected = "Φ gives z0 = NaN",
    check(&|c, u, v| forbidden(c, u, v).map(|_| c.validate(u, v) != ValidationResult::Valid))
);

fn grid(n: u32) -> Grid {
    Grid::new([n, 1], 1, 0, 8).expect("a grid")
}

/// Checks `fill` over three samples at `points` on the walled chart: the field each point's classification, the
/// bytes one bit per sample; a fresh set none; and a point outside the square refused by name.
fn check_fill(points: [[f64; 2]; 3], want: [bool; 3]) {
    let mut set = Synthetic::flat(grid(3), 3);
    assert_eq!(
        (0..3).map(|i| set.forbidden(i)).collect::<Vec<_>>(),
        [false; 3],
        "a fresh set"
    );
    fill(&mut set, &walled([0.0; 8], REJECT), |i| points[i as usize])
        .unwrap_or_else(|e| panic!("{e}"));
    let got: Vec<bool> = (0..3).map(|i| set.forbidden(i)).collect();
    assert_eq!(got, want, "the filled field");
    let bits = want
        .iter()
        .enumerate()
        .fold(0u32, |w, (i, f)| w | (u32::from(*f) << i));
    assert_eq!(
        set.bytes().chart_forbidden,
        bits.to_le_bytes(),
        "the bound bits"
    );
    let refused = fill(&mut set, &walled([0.0; 8], REJECT), |i| {
        [points[i as usize][0], 1.5]
    });
    assert!(
        refused.is_err_and(|e| e.contains("sample 0") && e.contains("outside [0,1]²")),
        "a point outside the square"
    );
}

#[test]
fn chart_class_fill_writes_the_field() {
    check_fill([[0.25, 0.5], [0.75, 0.5], [1.0, 0.5]], [false, true, true]);
}

negative_control!(
    chart_class_fill_writes_the_field,
    "the field wanted in another order fails",
    expected = "the filled field",
    check_fill([[0.25, 0.5], [0.75, 0.5], [1.0, 0.5]], [true, false, true])
);

/// Checks `set_forbidden` on sample `i` of 40 (two words): its bit alone set, `forbidden` reading it back, and clearing
/// it again.
fn check_set(i: u32, word: usize, bit: u32) {
    let mut set = Synthetic::flat(grid(40), 3);
    set.set_forbidden(i, true);
    assert!(set.forbidden(i) && (0..40).filter(|&j| set.forbidden(j)).count() == 1);
    let bytes = set.bytes().chart_forbidden;
    let mut want = vec![0u8; 8];
    want[word * 4..word * 4 + 4].copy_from_slice(&(1u32 << bit).to_le_bytes());
    assert_eq!(bytes, want, "sample {i}'s bit");
    set.set_forbidden(i, false);
    assert!(!set.forbidden(i));
}

#[test]
fn chart_class_set_forbidden_one_bit() {
    check_set(0, 0, 0);
    check_set(31, 0, 31);
    check_set(32, 1, 0);
    check_set(37, 1, 5);
}

negative_control!(
    chart_class_set_forbidden_one_bit,
    "sample 32's bit taken in the first word",
    expected = "sample 32's bit",
    check_set(32, 0, 0)
);
