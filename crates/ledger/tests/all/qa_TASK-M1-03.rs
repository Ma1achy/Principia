//! qa's tests for TASK-M1-03's REQ-RENDER-014 (render contract Part 2; lowering Part 3a; R-145), written from the
//! requirement:
//! - the prelude bakes `has_ftle` and `has_word` as compile-time constants with the variant's tier bits: WGSL's
//!   `const_assert` holds of the tier's values and fails on the others, which only a const-expression allows (so a
//!   branch on them is dead-code-eliminated, never a runtime read);
//! - `has_ensemble` is not baked: it is read from a uniform, so one compiled fragment variant serves E = 0 and E = 3,
//!   toggled back and forth on the same pipeline; at E = 0 `ensemble_spread` reads the canonical quiet NaN
//!   (`0x7FC00000`, lowering Part 3a) even when the resolve stage's value is a computed 0.0, and at E = 3 it reads that
//!   value, a computed zero included ("a computed zero spread is a real value only when E ≥ 1").
//!
//! Each check's control (R-176) runs it with one thing wrong and must fail.

use ledger::gen::prelude;
use ledger::gen::read::{self, Tier};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

/// Lowering Part 3a's canonical quiet NaN.
const ABSENT: u32 = 0x7fc0_0000;

fn validates(source: &str) -> Result<(), String> {
    let m = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&m)
    .map_err(|e| e.emit_to_string(source))?;
    Ok(())
}

/// At every tier, `bits(tier)` are the values `const_assert` accepts for `has_ftle` and `has_word` in the prelude, and
/// their negations are refused; `has_ensemble` is no constant (a `const` initialised from it is refused).
fn check_baked(bits: fn(Tier) -> (bool, bool)) {
    for tier in Tier::ALL {
        let p = prelude::wgsl(tier);
        let (ftle, word) = bits(tier);
        let holds =
            format!("{p}\nconst_assert has_ftle == {ftle};\nconst_assert has_word == {word};\n");
        validates(&holds).unwrap_or_else(|e| {
            panic!("at {tier:?}, has_ftle == {ftle} and has_word == {word} do not hold at compile time: {e}")
        });
        for wrong in [
            format!("{p}\nconst_assert has_ftle == {};\n", !ftle),
            format!("{p}\nconst_assert has_word == {};\n", !word),
        ] {
            assert!(
                validates(&wrong).is_err(),
                "at {tier:?}, a compile-time assertion of the wrong tier bit passes"
            );
        }
        assert!(
            validates(&format!("{p}\nconst qa_e: bool = has_ensemble();\n")).is_err(),
            "at {tier:?}, has_ensemble is a compile-time constant (R-145: a uniform)"
        );
    }
}

#[test]
fn qa_prelude_has_consts_baked_at_compile_time() {
    check_baked(|t| (t.has_ftle, t.has_word));
}

negative_control!(
    qa_prelude_has_consts_baked_at_compile_time,
    "tier bits read the other way round must fail",
    expected = "do not hold at compile time",
    check_baked(|t| (t.has_word, t.has_ftle))
);

const ENTRY: &str = r"
@group(2) @binding(0) var<storage, read> qa_spread: array<f32>;

@fragment
fn qa_e(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let s = sample_read(0u, qa_spread[0], has_ensemble(), vec3<f32>(1.0, 1.0, 1.0), ReadParams(0.01, 1e-6, 16u, 1000u));
    return vec4<u32>(bitcast<u32>(s.ensemble_spread), select(0u, 1u, has_ensemble()), 7u, 0u);
}
";

/// The uniform block the host writes at E (render_gui_spec §10.1, as built): `has_ensemble` 1 when E ≥ 1, padded to
/// 16 bytes.
fn uniform_at(e: u32) -> [u32; 4] {
    [u32::from(e >= 1), 0, 0, 0]
}

/// At every tier, the fragment variant (unpack layer, read side filling `ensemble_spread`, prelude) is compiled once
/// and drawn at E = 0, 3, 0, 3 with `words(E)` as the prelude's uniform, for spreads 0.0 and 0.5: E = 0 reads the
/// canonical quiet NaN's bits whatever the spread, E = 3 the spread itself.
fn check_toggle(gpu: &GpuHarness, words: fn(u32) -> [u32; 4]) {
    assert_eq!(
        words(0),
        uniform_at(0),
        "the host's uniform at E = 0 is not has_ensemble = 0"
    );
    assert_eq!(
        words(3),
        uniform_at(3),
        "the host's uniform at E = 3 is not has_ensemble = 1"
    );
    let layout = ledger::layout();
    let entries = ledger::gen::validate(&layout).expect("the layout validates");
    for tier in Tier::ALL {
        let source = format!(
            "{}\n{}\n{ENTRY}",
            read::assemble(&layout.words, &entries, tier, &["ensemble_spread"])
                .expect("the variant assembles"),
            prelude::wgsl(tier)
        );
        validates(&source).unwrap_or_else(|e| panic!("{e}"));
        let kernel = gpu
            .fragment(
                &source,
                "qa_e",
                &[&[BindingKind::Uniform], &[], &[BindingKind::Storage]],
                1,
                1,
            )
            .unwrap_or_else(|e| panic!("{e}"));
        for e in [0u32, 3, 0, 3] {
            for spread in [0.0f32, 0.5] {
                let u = words(e);
                let input = [spread.to_bits()];
                let p = kernel
                    .draw(&[&[&u], &[], &[&input]])
                    .unwrap_or_else(|e| panic!("{e}"))[0];
                assert_eq!(p[2], 7, "the variant did not draw");
                let want = if e == 0 { ABSENT } else { spread.to_bits() };
                assert_eq!(
                    p[0], want,
                    "at {tier:?}, E = {e}, spread {spread}: ensemble_spread reads {:#010x}, not {want:#010x}",
                    p[0]
                );
                assert_eq!(
                    p[1],
                    u32::from(e >= 1),
                    "at {tier:?}, E = {e}: has_ensemble() is {}",
                    p[1]
                );
            }
        }
    }
}

#[test]
fn qa_prelude_has_consts_e_toggles_without_rebake() {
    check_toggle(
        &GpuHarness::new().expect("a GPU device"),
        prelude::uniform_words,
    );
}

negative_control!(
    qa_prelude_has_consts_e_toggles_without_rebake,
    "a uniform that reads the ensemble on at E = 0 must fail",
    expected = "E = 0",
    check_toggle(&GpuHarness::new().expect("a GPU device"), |_| [1, 0, 0, 0])
);

/// The draw-level control: the host's words right, but checked against E = 0 reading the given spread, must fail on
/// the NaN.
#[cfg(feature = "controls")]
fn check_nan_is_drawn(gpu: &GpuHarness) {
    let layout = ledger::layout();
    let entries = ledger::gen::validate(&layout).expect("the layout validates");
    let source = format!(
        "{}\n{}\n{ENTRY}",
        read::assemble(&layout.words, &entries, Tier::FULL, &["ensemble_spread"]).unwrap(),
        prelude::wgsl(Tier::FULL)
    );
    let kernel = gpu
        .fragment(
            &source,
            "qa_e",
            &[&[BindingKind::Uniform], &[], &[BindingKind::Storage]],
            1,
            1,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let p = kernel
        .draw(&[&[&uniform_at(0)], &[], &[&[0.0f32.to_bits()]]])
        .unwrap()[0];
    assert_eq!(
        p[0],
        0.0f32.to_bits(),
        "E = 0 reads the NaN, not the spread"
    );
}

negative_control!(
    qa_prelude_has_consts_e0_draws_the_nan,
    "E = 0 must not read a computed zero spread",
    expected = "E = 0 reads the NaN",
    check_nan_is_drawn(&GpuHarness::new().expect("a GPU device"))
);
