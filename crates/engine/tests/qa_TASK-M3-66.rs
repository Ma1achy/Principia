//! QA's tests for TASK-M3-66, written from REQ-SYS-096 and integrator_contract Part 5 (R-668), each with its negative
//! control (R-176). The kernel runs only at `G = 1` and `M = 1`: dispatch refuses a `SimUniforms` whose `g` or `m_total`
//! is not 1, NaN and ±∞ included, with a typed reason naming the field and its value, as it refuses `ε ≠ 0`; a source
//! in another `G` or total mass is converted at the import boundary, never carried to the kernel.
//!
//! Every name starts `units_refusal` so that the acceptance command runs it. What these add to the task's own tests:
//! - the two values next to 1 are refused at f64, where a check at the kernel's f32 reads both as 1;
//! - any NaN and either ∞, its bits carried unchanged, where a check on finite values passes them;
//! - the reason parses: the field's name as a word and the value at full precision, `m_total`'s reason not naming `g`;
//! - `G·M = 1` with neither 1 is refused, where the one number import divides by, `√(GM)`, would read it as a unit;
//! - the refusal holds at the ends of the horizon's range and of the `N_max` range, not at the default sim key alone;
//! - the module note's order against every earlier refusal, not two of them;
//! - the boundary itself: what dispatch refuses, `validation::import` converts or refuses (dd_validation_orbits §2).

use std::cell::Cell;

use engine::contract::sim_uniforms::SimUniforms;
use engine::dispatch::validate::{dispatch, validate, DispatchError};
use kernel::decode::Ic;
use validation::import::{import, unit_state, Start};
use validation::negative_control;

/// A dispatch under test, counting its launches.
type Rule = dyn Fn(&SimUniforms, &Cell<u32>) -> Result<(), DispatchError>;

/// `dispatch`, counting its launches.
fn counted(u: &SimUniforms, launched: &Cell<u32>) -> Result<(), DispatchError> {
    dispatch(u, |_| launched.set(launched.get() + 1))
}

/// A dispatch that reads as 1 every `g` and `m_total` that `reads_as_one` says it does.
#[cfg(feature = "controls")]
fn reading(
    u: &SimUniforms,
    launched: &Cell<u32>,
    reads_as_one: fn(f64) -> bool,
) -> Result<(), DispatchError> {
    let one = |x: f64| if reads_as_one(x) { 1.0 } else { x };
    let read = SimUniforms {
        g: one(u.g),
        m_total: one(u.m_total),
        ..*u
    };
    counted(&read, launched)
}

/// The two carried units, as the requirement names them.
#[derive(Clone, Copy, Debug)]
enum Field {
    G,
    MTotal,
}

/// Both carried units.
const FIELDS: [Field; 2] = [Field::G, Field::MTotal];

impl Field {
    /// The field's name in a reason.
    fn name(self) -> &'static str {
        match self {
            Field::G => "g",
            Field::MTotal => "m_total",
        }
    }

    /// The other field's name.
    fn other_name(self) -> &'static str {
        match self {
            Field::G => "m_total",
            Field::MTotal => "g",
        }
    }

    /// The default sim key with this field at `x`.
    fn at(self, x: f64) -> SimUniforms {
        self.on(SimUniforms::default(), x)
    }

    /// `u` with this field at `x`.
    fn on(self, u: SimUniforms, x: f64) -> SimUniforms {
        match self {
            Field::G => SimUniforms { g: x, ..u },
            Field::MTotal => SimUniforms { m_total: x, ..u },
        }
    }

    /// The value `e` carries, if `e` is this field's own refusal.
    fn own(self, e: &DispatchError) -> Option<f64> {
        match (self, *e) {
            (Field::G, DispatchError::GravitationalConstant { g }) => Some(g),
            (Field::MTotal, DispatchError::TotalMass { m_total }) => Some(m_total),
            _ => None,
        }
    }
}

/// `rule` refuses `field` at each of `values`: nothing launched, and this field's own refusal, its value bit for bit.
fn check_refused(rule: &Rule, field: Field, values: &[f64]) {
    let name = field.name();
    for &x in values {
        let launched = Cell::new(0);
        let got = rule(&field.at(x), &launched);
        assert!(
            got.is_err() && launched.get() == 0,
            "{name} = {x:?} is dispatched {} time(s): {got:?}",
            launched.get()
        );
        assert_eq!(
            got.as_ref()
                .err()
                .and_then(|e| field.own(e))
                .map(f64::to_bits),
            Some(x.to_bits()),
            "{name} = {x:?} is not refused as {name}: {got:?}"
        );
    }
}

/// The list the requirement names: 2, 0, −1, the nearest f64 on either side of 1, NaN and ±∞; and −0, the other 0.
fn listed() -> [f64; 9] {
    let one = 1.0_f64;
    [
        2.0,
        0.0,
        -0.0,
        -1.0,
        one.next_down(),
        one.next_up(),
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ]
}

#[test]
fn units_refusal_listed_g() {
    check_refused(&counted, Field::G, &listed());
}

negative_control!(
    units_refusal_listed_g,
    "a check_carried that skips g",
    expected = "is dispatched",
    check_refused(
        &|u: &SimUniforms, l: &Cell<u32>| counted(&SimUniforms { g: 1.0, ..*u }, l),
        Field::G,
        &listed()
    )
);

#[test]
fn units_refusal_listed_m_total() {
    check_refused(&counted, Field::MTotal, &listed());
}

negative_control!(
    units_refusal_listed_m_total,
    "a check_carried that skips m_total",
    expected = "is dispatched",
    check_refused(
        &|u: &SimUniforms, l: &Cell<u32>| counted(&SimUniforms { m_total: 1.0, ..*u }, l),
        Field::MTotal,
        &listed()
    )
);

/// The values within an f32 rounding of 1, none of them 1 at f64.
fn beside_one() -> [f64; 4] {
    let one = 1.0_f64;
    [
        one.next_down(),
        one.next_up(),
        one.next_down().next_down(),
        one.next_up().next_up(),
    ]
}

/// The f32 kernel reads every one of these as 1, so a refusal has to be made at f64.
#[test]
fn units_refusal_beside_one() {
    for field in FIELDS {
        check_refused(&counted, field, &beside_one());
    }
}

negative_control!(
    units_refusal_beside_one,
    "units compared after rounding to the kernel's f32",
    expected = "is dispatched",
    for field in FIELDS {
        check_refused(
            &|u: &SimUniforms, l: &Cell<u32>| reading(u, l, |x| x as f32 == 1.0),
            field,
            &beside_one(),
        );
    }
);

/// Every NaN and both infinities: a quiet NaN of either sign, a signalling one, ±∞.
fn not_finite() -> [f64; 5] {
    [
        f64::NAN,
        f64::from_bits(0xFFF8_0000_0000_0001),
        f64::from_bits(0x7FF0_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
    ]
}

#[test]
fn units_refusal_not_finite() {
    for field in FIELDS {
        check_refused(&counted, field, &not_finite());
    }
}

negative_control!(
    units_refusal_not_finite,
    "a check that reads only finite values",
    expected = "is dispatched",
    for field in FIELDS {
        check_refused(
            &|u: &SimUniforms, l: &Cell<u32>| reading(u, l, |x| !x.is_finite()),
            field,
            &not_finite(),
        );
    }
);

/// Each field's refusal is its own: `g` is never reported as `m_total`, nor the reverse.
#[test]
fn units_refusal_typed_by_field() {
    for field in FIELDS {
        check_refused(&counted, field, &[2.0]);
    }
}

negative_control!(
    units_refusal_typed_by_field,
    "one refusal for both fields, reporting g as m_total",
    expected = "is not refused as g",
    check_refused(
        &|u: &SimUniforms, l: &Cell<u32>| counted(u, l).map_err(|e| match e {
            DispatchError::GravitationalConstant { g } => DispatchError::TotalMass { m_total: g },
            other => other,
        }),
        Field::G,
        &[2.0]
    )
);

/// The words of a reason.
fn words(reason: &str) -> Vec<&str> {
    reason
        .split(|c: char| c.is_whitespace() || ",:;()=".contains(c))
        .filter(|w| !w.is_empty())
        .collect()
}

/// `read` gives a reason for each listed value that names the field as a word, not the other field, and gives the
/// value in digits that parse back to it exactly (the two beside 1 need all of theirs).
fn check_reason(read: &dyn Fn(&DispatchError) -> String) {
    for field in FIELDS {
        for x in listed() {
            let e = validate(&field.at(x)).unwrap_err();
            let reason = read(&e);
            let said = words(&reason);
            let name = field.name();
            assert!(
                said.contains(&name),
                "{name}'s refusal does not name it: {reason}"
            );
            assert!(
                !said.contains(&field.other_name()),
                "{name}'s refusal names the other field: {reason}"
            );
            let gives_value = said.iter().any(|w| {
                w.parse::<f64>()
                    .is_ok_and(|v| v.to_bits() == x.to_bits() || (v.is_nan() && x.is_nan()))
            });
            assert!(
                gives_value,
                "{name}'s refusal does not give the value {x:?}: {reason}"
            );
        }
    }
}

#[test]
fn units_refusal_reason_names_field_and_value() {
    check_reason(&|e: &DispatchError| e.to_string());
}

negative_control!(
    units_refusal_reason_names_field_and_value,
    "a reason that rounds the value it was given",
    expected = "does not give the value",
    check_reason(&|e: &DispatchError| match *e {
        DispatchError::GravitationalConstant { g } => format!("g {g:.3} is refused"),
        DispatchError::TotalMass { m_total } => format!("m_total {m_total:.3} is refused"),
        other => other.to_string(),
    })
);

/// `G·M = 1` with neither 1 is still another unit system, though `√(GM)`, what import divides velocities by, is 1.
fn check_product_one(rule: &Rule) {
    for (g, m_total) in [(2.0, 0.5), (0.5, 2.0), (4.0, 0.25), (-1.0, -1.0)] {
        let u = SimUniforms {
            g,
            m_total,
            ..SimUniforms::default()
        };
        let launched = Cell::new(0);
        let got = rule(&u, &launched);
        assert!(
            got.is_err() && launched.get() == 0,
            "g = {g}, m_total = {m_total} is dispatched: {got:?}"
        );
    }
}

#[test]
fn units_refusal_product_of_one() {
    check_product_one(&counted);
}

negative_control!(
    units_refusal_product_of_one,
    "a check of the product G·M alone",
    expected = "is dispatched",
    check_product_one(&|u: &SimUniforms, l: &Cell<u32>| {
        let read = if u.g * u.m_total == 1.0 {
            SimUniforms {
                g: 1.0,
                m_total: 1.0,
                ..*u
            }
        } else {
            *u
        };
        counted(&read, l)
    })
);

/// At the ends of `T ∈ [0.4, 200]` and at both sides of `T = 65.535`, where `dt_macro` changes form, at `N_max` 1 and
/// 64: each sim key dispatches at `g = m_total = 1`, so the refusal is the units' and nothing else's.
fn check_every_horizon(rule: &Rule) {
    let d = SimUniforms::default();
    for n_max in [1, 64] {
        for t in [0.4, 50.0, 65.535, 65.538, 200.0] {
            let u = d
                .with_substep_law(d.r_sub, d.gamma_sub, n_max)
                .unwrap()
                .with_horizon(t);
            let launched = Cell::new(0);
            assert_eq!(rule(&u, &launched), Ok(()), "T = {t}, N_max = {n_max}");
            assert_eq!(launched.get(), 1);
            for field in FIELDS {
                let refused = rule(&field.on(u, 2.0), &launched);
                assert!(
                    refused.is_err() && launched.get() == 1,
                    "{} = 2 is dispatched at T = {t}, N_max = {n_max}",
                    field.name()
                );
            }
        }
    }
}

#[test]
fn units_refusal_every_horizon() {
    check_every_horizon(&counted);
}

negative_control!(
    units_refusal_every_horizon,
    "a refusal made at the default horizon alone",
    expected = "is dispatched at T",
    check_every_horizon(&|u: &SimUniforms, l: &Cell<u32>| {
        if u.t_horizon == SimUniforms::default().t_horizon {
            counted(u, l)
        } else {
            counted(
                &SimUniforms {
                    g: 1.0,
                    m_total: 1.0,
                    ..*u
                },
                l,
            )
        }
    })
);

/// A sim key refused for each reason the module's note lists ahead of the units.
fn earlier_refusals() -> Vec<(&'static str, SimUniforms)> {
    let d = SimUniforms::default();
    vec![
        (
            "escape_window",
            SimUniforms {
                escape_window: 0.0,
                ..d
            },
        ),
        ("t_horizon", d.with_horizon(1000.0)),
        ("dt_macro", SimUniforms { dt_macro: 0.0, ..d }),
        (
            "thresholds",
            SimUniforms {
                r_sub: d.r_sub * 2.0,
                ..d
            },
        ),
        ("r_coll", SimUniforms { r_coll: -1.0, ..d }),
        ("tau", SimUniforms { tau: 0.0, ..d }),
        ("r_close", SimUniforms { r_close: -1.0, ..d }),
        ("eps_e", SimUniforms { eps_e: 0.0, ..d }),
        ("eps_l", SimUniforms { eps_l: 0.0, ..d }),
        ("epsilon", SimUniforms { epsilon: 1e-3, ..d }),
    ]
}

/// The module note's order: every earlier refusal stands with both units wrong too, and `g` comes before `m_total`.
fn check_note_order(rule: &Rule) {
    let wrong = |u: SimUniforms| SimUniforms {
        g: 2.0,
        m_total: 3.0,
        ..u
    };
    let launched = Cell::new(0);
    for (name, u) in earlier_refusals() {
        let alone = rule(&u, &launched);
        assert!(alone.is_err(), "the {name} sim key is not refused");
        assert_eq!(
            rule(&wrong(u), &launched),
            alone,
            "the units are refused ahead of {name}"
        );
    }
    assert_eq!(
        rule(&wrong(SimUniforms::default()), &launched),
        Err(DispatchError::GravitationalConstant { g: 2.0 }),
        "m_total is refused ahead of g"
    );
    assert_eq!(launched.get(), 0);
}

#[test]
fn units_refusal_note_order() {
    check_note_order(&counted);
}

negative_control!(
    units_refusal_note_order,
    "units checked first, ahead of the rest of the note",
    expected = "the units are refused ahead of",
    check_note_order(&|u: &SimUniforms, l: &Cell<u32>| {
        if u.g == 1.0 {
            counted(u, l)
        } else {
            Err(DispatchError::GravitationalConstant { g: u.g })
        }
    })
);

/// `g = m_total = 1`, written as the literal the requirement gives, dispatches once and is the sim key accepted.
fn check_one_dispatches(rule: &Rule) {
    let d = SimUniforms::default();
    assert_eq!((d.g, d.m_total), (1.0, 1.0), "the default units are not 1");
    let u = SimUniforms {
        g: 1.0,
        m_total: 1.0,
        ..d
    };
    let launched = Cell::new(0);
    assert_eq!(
        rule(&u, &launched),
        Ok(()),
        "g = m_total = 1 does not dispatch"
    );
    assert_eq!(
        launched.get(),
        1,
        "g = m_total = 1 launched other than once"
    );
}

#[test]
fn units_refusal_one_dispatches_once() {
    check_one_dispatches(&counted);
    let accepted = validate(&SimUniforms::default()).unwrap();
    assert_eq!(accepted.uniforms().g.to_bits(), 1.0_f64.to_bits());
    assert_eq!(accepted.uniforms().m_total.to_bits(), 1.0_f64.to_bits());
}

negative_control!(
    units_refusal_one_dispatches_once,
    "a dispatch that refuses the unit it runs at",
    expected = "g = m_total = 1 does not dispatch",
    check_one_dispatches(&|u: &SimUniforms, _: &Cell<u32>| Err(
        DispatchError::GravitationalConstant { g: u.g }
    ))
);

/// A sim key is a key whether or not it dispatches: `g = 2` and `m_total = 2` still move it, and neither dispatches.
fn check_keyed_not_dispatched(key_of: &dyn Fn(&SimUniforms) -> Vec<u8>) {
    let base = key_of(&SimUniforms::default());
    let keys = FIELDS.map(|field| {
        let u = field.at(2.0);
        let key = key_of(&u);
        assert_ne!(key, base, "{} = 2 does not move the sim key", field.name());
        assert!(validate(&u).is_err(), "{} = 2 dispatches", field.name());
        key
    });
    assert_ne!(keys[0], keys[1], "g and m_total share a sim-key byte");
}

#[test]
fn units_refusal_keyed_not_dispatched() {
    check_keyed_not_dispatched(&SimUniforms::sim_key_bytes);
}

negative_control!(
    units_refusal_keyed_not_dispatched,
    "a sim key dropped along with the refusal",
    expected = "does not move the sim key",
    check_keyed_not_dispatched(&|u: &SimUniforms| SimUniforms {
        g: 1.0,
        m_total: 1.0,
        ..*u
    }
    .sim_key_bytes())
);

// ── The import boundary (R-668): what dispatch refuses, import converts or refuses ───────────────────────────────

/// A generic start with its total mass `m_total`, its constant `g`, and its velocities multiplied by `speed`.
fn start(g: f64, m_total: f64, speed: f64) -> Start {
    Start {
        name: "qa".into(),
        citation: "none: a generic triangle".into(),
        g,
        m: [0.25, 0.25, 0.5].map(|m| m * m_total),
        r: [[1.0, 0.0], [-0.5, 0.75], [-0.25, -0.375]],
        v: [[0.0, 0.25], [0.125, -0.125], [-0.0625, -0.0625]].map(|v| v.map(|c| c * speed)),
    }
}

/// A state's bits.
fn bits(x: &Ic<f64>) -> Vec<u64> {
    x.m.iter()
        .copied()
        .chain(x.r.iter().flatten().copied())
        .chain(x.p.iter().flatten().copied())
        .map(f64::to_bits)
        .collect()
}

/// A conversion to `G = 1`, `M = 1`.
type Convert = dyn Fn(&Start) -> Result<Ic<f64>, String>;

/// The recorded rescale's time rescaling (dd_validation_orbits §2, velocities `vᵢ/√(GM)`): a source at `G·M = 4` whose
/// velocities are twice a unit source's is that unit source, bit for bit; the factors are powers of 2, so nothing
/// rounds and no tolerance enters.
fn check_converted(convert: &Convert) {
    let unit = bits(&convert(&start(1.0, 1.0, 1.0)).unwrap());
    for (what, s) in [
        ("another G", start(4.0, 1.0, 2.0)),
        ("another total mass", start(1.0, 4.0, 2.0)),
    ] {
        assert_eq!(
            bits(&convert(&s).unwrap()),
            unit,
            "a source in {what} is not converted to G = 1, M = 1"
        );
    }
}

#[test]
fn units_refusal_import_converts() {
    check_converted(&unit_state);
}

negative_control!(
    units_refusal_import_converts,
    "an import that carries the source's velocities through unconverted",
    expected = "is not converted",
    check_converted(&|s: &Start| unit_state(&Start {
        g: 1.0,
        m: s.m.map(|m| m / (s.m[0] + s.m[1] + s.m[2])),
        ..s.clone()
    }))
);

/// A `g` next to 1, or 2, is one import converts, and the same number carried to dispatch is refused; the rescale
/// records it, so the source's own units reach `back` and never a `SimUniforms`.
#[test]
fn units_refusal_import_takes_what_dispatch_refuses() {
    let one = 1.0_f64;
    for g in [2.0, one.next_down(), one.next_up()] {
        let imported = import(&start(g, 1.0, 1.0)).unwrap();
        assert_eq!(imported.rescale.g.to_bits(), g.to_bits());
        check_refused(&counted, Field::G, &[imported.rescale.g]);
    }
    let imported = import(&start(1.0, 2.0, 1.0)).unwrap();
    assert_eq!(imported.rescale.mass, 2.0);
    check_refused(&counted, Field::MTotal, &[imported.rescale.mass]);
}

negative_control!(
    units_refusal_import_takes_what_dispatch_refuses,
    "a dispatch that runs the source's g as carried",
    expected = "is dispatched",
    check_refused(
        &|u: &SimUniforms, l: &Cell<u32>| counted(&SimUniforms { g: 1.0, ..*u }, l),
        Field::G,
        &[import(&start(2.0, 1.0, 1.0)).unwrap().rescale.g]
    )
);

/// A source that cannot be converted is refused at import, for `G` and for the mass, so neither reaches dispatch: `G`
/// and `M` not finite and positive have no `√(GM)`.
fn check_unconvertible(convert: &Convert) {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            convert(&start(bad, 1.0, 1.0)).is_err(),
            "G = {bad} is not refused at import"
        );
        assert!(
            convert(&start(1.0, bad, 1.0)).is_err(),
            "M = {bad} is not refused at import"
        );
    }
}

#[test]
fn units_refusal_import_refuses_unconvertible() {
    check_unconvertible(&unit_state);
}

negative_control!(
    units_refusal_import_refuses_unconvertible,
    "an import that reads a bad G as 1",
    expected = "is not refused at import",
    check_unconvertible(&|s: &Start| unit_state(&Start {
        g: 1.0,
        ..s.clone()
    }))
);
