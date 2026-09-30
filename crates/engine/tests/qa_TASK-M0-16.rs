//! qa's test for TASK-M0-16's surfaces (gui_state_contract §1, §2; R-133; R-71): the five surfaces are declared in
//! the engine crate, `pub` from its `contract` module, and carry no default: a default would be a value the corpus
//! does not give (R-71), and the task's Deliverables say "no defaults". The engine crate has no `controls` feature
//! and no dev-dependency on `validation`, so the control is written here by hand: a type with a default must trip
//! the same check.

use std::marker::PhantomData;

use engine::contract::render_state::RenderState;
use engine::contract::set_field::SetField;
use engine::contract::sim_config::SimConfig;
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::ViewUI;

struct Probe<T>(PhantomData<T>);

/// Chosen first, by `&Probe<T>`, when `T: Default`.
trait HasDefault {
    fn has_default(&self) -> bool {
        true
    }
}
impl<T: Default> HasDefault for Probe<T> {}

/// Chosen by auto-ref when `T` has no default.
trait NoDefault {
    fn has_default(&self) -> bool {
        false
    }
}
impl<T> NoDefault for &Probe<T> {}

macro_rules! has_default {
    ($t:ty) => {
        (&Probe::<$t>(PhantomData)).has_default()
    };
}

fn check_no_default(surfaces: &[(&str, bool)]) {
    for (name, has) in surfaces {
        assert!(
            !has,
            "the surface `{name}` has a default (R-71; no defaults at M0)"
        );
    }
}

fn surfaces() -> [(&'static str, bool); 5] {
    [
        ("SimConfig", has_default!(SimConfig)),
        ("RenderState", has_default!(RenderState)),
        ("ViewUI", has_default!(ViewUI)),
        ("SetField", has_default!(SetField)),
        ("Snapshot", has_default!(Snapshot)),
    ]
}

#[test]
fn qa_the_five_surfaces_have_no_default() {
    check_no_default(&surfaces());
}

#[derive(Default)]
struct Defaulted;

/// The hand-written negative control: a type with a default trips the check.
#[test]
#[should_panic(expected = "has a default")]
fn qa_the_five_surfaces_have_no_default_control() {
    check_no_default(&[("Defaulted", has_default!(Defaulted))]);
}
