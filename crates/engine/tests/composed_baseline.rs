//! The inverses' baseline for the composition: each chart's encode, exact and partial lookup and ingest, produced by
//! calling the kernel's generic functions on the chart directly (`fixtures/chart_composition/inverses.json`), so
//! `composed_decode_encode` compares the composition with words the composition did not produce. `digest.json` and
//! `registry_charts.json` hold the decode; this holds the rest.
//!
//! **A word** is FNV-1a 64 of the `Debug` text of every result of an operation at a fixed set of inputs: Rust prints an
//! f64's `Debug` as the shortest text that reads back to the same bits, so equal text is equal bits.
//! **The inputs**: the chart's own decode at `path_support`'s 3 × 3 probe through its window (each state also with one
//! momentum moved, which leaves the chart's reach), the Pythagorean pair `(2, 1)`, the triple `(3, 4, 5)`, a raw `z`,
//! and three partial entries (the masses alone, the positions alone, the momenta alone, at `ν = 0.3`).
//!
//! **The criterion**: each chart's four words equal the recorded ones exactly, and every recorded chart is produced.
//! The file is written only by `PRINCIPIA_BLESS_INVERSES_BASELINE=1`; a different word is a finding to report, never
//! a re-record.
//!
//! **Control**: one mantissa bit of one decoded momentum flipped before the operations. It must fail.

mod path_support;

use std::collections::BTreeMap;

use engine::contract::sim_config::Window;
use engine::lookup::{
    lookup, lookup_partial, phys, Exact, Fill, Locked, Partial, Rejected, Tolerances,
};
use kernel::decode::Ic;
use kernel::encode::by_kind::encode_into_within;
use kernel::encode::by_kind::ChartEncoded;
use kernel::encode::ingest::ingest;
use kernel::encode::ingest::Ingested;
use kernel::generated::links::Defaults;
use ledger::constants::EPS_PHYS;
use path_support::{each_chart, inverse_words, root, Ops, Visit};
use validation::negative_control;

/// The baseline, relative to the workspace root.
const BASELINE: &str = "fixtures/chart_composition/inverses.json";

/// A chart called as the kernel's generics are, directly.
struct Direct<'a, C>(&'a C, Tolerances);

impl<C: Fill> Ops for Direct<'_, C>
where
    f64: From<C::Param>,
{
    fn decode(&self, u: f64, v: f64) -> Ic<f64> {
        self.0.decode_at::<Defaults, f64>(u, v).ic
    }
    fn encode(&self, ic: &Ic<f64>) -> ChartEncoded<f64> {
        let eps = EPS_PHYS.number() * f64::EPSILON;
        encode_into_within::<Defaults, f64, _, _>(self.0, ic, phys, eps, self.1.curve_tol)
    }
    fn lookup(&self, e: &Exact) -> Result<Locked, Rejected> {
        lookup::<Defaults, C, _>(self.0, e, phys, &self.1)
    }
    fn partial(&self, p: &Partial) -> Result<Locked, Rejected> {
        lookup_partial::<Defaults, C, _>(self.0, p, Some(0.3), phys, &self.1)
    }
    fn ingest(&self, ic: &Ic<f64>) -> Ingested<f64> {
        ingest::<Defaults, f64, _>(ic, phys)
    }
}

/// The four words of each chart, `flip` applied to each decoded state first.
struct Words {
    flip: fn(&mut Ic<f64>),
    out: BTreeMap<String, String>,
}

impl Visit for Words {
    fn chart<C>(&mut self, name: &'static str, chart: C, window: Window)
    where
        C: Fill + 'static,
        f64: From<C::Param>,
    {
        let tol = Tolerances::register(128.0 * f64::EPSILON);
        inverse_words(
            name,
            &Direct(&chart, tol),
            &window,
            self.flip,
            &mut self.out,
        );
    }
}

/// Every chart's words.
pub fn words(flip: fn(&mut Ic<f64>)) -> BTreeMap<String, String> {
    let mut w = Words {
        flip,
        out: BTreeMap::new(),
    };
    each_chart(&mut w);
    w.out
}

/// No change.
fn keep(_: &mut Ic<f64>) {}

/// The low mantissa bit of body 0's `p_x` flipped (the control).
#[cfg(feature = "controls")]
fn flip_bit(ic: &mut Ic<f64>) {
    ic.p[0][0] = f64::from_bits(ic.p[0][0].to_bits() ^ 1);
}

/// The recorded baseline.
pub fn recorded() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root().join(BASELINE)).expect("the inverses baseline");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    serde_json::from_value(value["words"].clone()).expect("the words")
}

fn check(flip: fn(&mut Ic<f64>), plain: bool) {
    let actual = words(flip);
    // Only the plain arm records: a control arm, run under the same variable, never writes its flipped words.
    if std::env::var_os("PRINCIPIA_BLESS_INVERSES_BASELINE").is_some() && plain {
        let value = serde_json::json!({
            "what": "Each chart's encode, exact and partial lookup and ingest at fixed inputs, FNV-1a 64 of the results' Debug text (crates/engine/tests/composed_baseline.rs), recorded from the per-chart calls before the composition.",
            "command": "PRINCIPIA_BLESS_INVERSES_BASELINE=1 cargo test -p engine --test composed_baseline",
            "words": actual,
        });
        let text = serde_json::to_string_pretty(&value).expect("JSON") + "\n";
        std::fs::write(root().join(BASELINE), text).expect("the inverses baseline written");
        return;
    }
    let expected = recorded();
    assert_eq!(
        expected.keys().collect::<Vec<_>>(),
        actual.keys().collect::<Vec<_>>(),
        "the inverses baseline's words"
    );
    for (name, want) in &expected {
        assert_eq!(
            &actual[name], want,
            "{name}: not bit for bit with the inverses baseline"
        );
    }
}

#[test]
fn inverses_baseline_bitwise() {
    check(keep, true);
}

negative_control!(
    inverses_baseline_bitwise,
    "one mantissa bit of a decoded momentum flipped before the operations",
    expected = "not bit for bit with the inverses baseline",
    check(flip_bit, false)
);
