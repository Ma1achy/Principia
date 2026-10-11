//! The numeric field view's template (`ledger::gen::numeric`; render_gui_spec §10.1; R-114, R-136; RQ-231;
//! TASK-M1-09), REQ-RENDER-022:
//! - every generated numeric view is the two-line template, the bitcast NaN guard then the `range_norm` ramp, with no
//!   self-comparison (`numeric_view_template_is_the_two_lines`);
//! - each ledger scale generates RQ-231's `raw`, and the CPU twin places it as the WGSL does
//!   (`numeric_view_template_scales_*`);
//! - a field with an unbounded ledger range defaults to `RANGE_AUTO = 1` (`numeric_view_template_unbounded_*`);
//! - REQ-RENDER-084 (R-400, R-403; TASK-M1-17): a field whose declared range spans zero is on `ramp_coolwarm` over the
//!   symmetric `[−M, M]`, fixed and auto, 0 at `t = ½`; a field with `lo ≥ 0` on `ramp_viridis`; the cyclic field on
//!   `ramp_twilight`; the ledger gains no member (`numeric_view_template_zero_spanning_*`,
//!   `numeric_view_template_ledger_*`); `K_0` and `V_0` on `ramp_viridis`, increasing in the value in both modes,
//!   defaulting to `RANGE_AUTO = 1`, `V_0`'s most negative value at `t = 0` (`numeric_view_template_k0_v0_*`);
//! - setting the `RANGE_AUTO` param writes the header default `@uniform RANGE_AUTO: u32 = <0|1> [0, 1]`, which the
//!   assembler's `Declaration::parse` gives back (`numeric_view_template_range_auto_round_trips`).
//!
//! Each test registers its negative control (R-176).

use ledger::gen::catalogue::numeric_view;
use ledger::gen::numeric::{self, End, NumericView, Placement, Ramp, Sentinel, Shown};
use ledger::schema::{Bound, Entry, EntryBuilder, FieldType, Ledger, Location, Range, Scale};
use validation::{negative_control, Declaration, UniformType};

/// The guard every numeric view opens with: the bitcast test against the canonical quiet NaN (R-114).
const GUARD: &str =
    "    if (bitcast<u32>(raw) == 0x7fc00000u) { return debug_invalid(ctx.frag_xy); }";

fn entries(l: &Ledger) -> Vec<Entry> {
    ledger::gen::validate(l).unwrap_or_else(|e| panic!("{e}"))
}

/// Every field the template colours: its entry, its view and the generated WGSL, `RANGE_AUTO` at its default.
fn numeric_views() -> Vec<(Entry, NumericView, String)> {
    let l = ledger::layout();
    let entries = entries(&l);
    entries
        .iter()
        .filter_map(|e| {
            let n = NumericView::of(e)?;
            let wgsl = numeric_view(&l.words, &entries, e.name, None)
                .unwrap_or_else(|| panic!("`{}` has a numeric view but no WGSL", e.name));
            Some((e.clone(), n, wgsl))
        })
        .collect()
}

/// The lines of `view`'s `colour()` body.
fn body(view: &str) -> Vec<&str> {
    view.lines()
        .skip_while(|l| !l.starts_with("fn colour("))
        .skip(1)
        .take_while(|l| *l != "}")
        .collect()
}

/// Checks that `view` (of `field`) is the two-line template: `raw`'s binding, the guard, at most the sentinel's line
/// (and a diverging view's symmetric bound), then the ramp; and that nothing compares `raw` with itself.
fn check_template(field: &str, view: &str) {
    for bad in ["raw != raw", "raw == raw", "isnan"] {
        assert!(
            !view.contains(bad),
            "`{field}`'s view tests NaN by a self-comparison, `{bad}`"
        );
    }
    let lines = body(view);
    let shape = |ok: bool, why: &str| {
        assert!(
            ok,
            "`{field}`'s view is not the two-line template ({why}):\n{}",
            lines.join("\n")
        )
    };
    shape(lines.len() >= 3, "too short");
    shape(
        lines[0].starts_with("    let raw = "),
        "`raw`'s binding first",
    );
    shape(lines[1] == GUARD, "the bitcast guard second");
    let last = lines[lines.len() - 1];
    shape(
        (last.starts_with("    return ramp_viridis(range_norm(")
            || last.starts_with("    return ramp_twilight(range_norm(")
            || last.starts_with("    return ramp_coolwarm(range_norm("))
            && last.contains(", uniforms.RANGE_AUTO != 0u, "),
        "the ramp last",
    );
    for line in &lines[2..lines.len() - 1] {
        let sentinel =
            line.starts_with("    if (raw == ") && line.contains("{ return dbg_sentinel(");
        let unset = *line
            == "    if (bitcast<u32>(raw) == 0x7f800000u) { return debug_not_yet(ctx.frag_xy); }";
        let symmetric =
            *line == "    let m = max(abs(uniforms.u_range.x), abs(uniforms.u_range.y));";
        shape(sentinel || unset || symmetric, "a line between the two");
    }
}

#[test]
fn numeric_view_template_is_the_two_lines() {
    let views = numeric_views();
    assert!(views.len() >= 10, "{} numeric views", views.len());
    for (e, _, wgsl) in &views {
        check_template(e.name, wgsl);
    }
}

negative_control!(
    numeric_view_template_is_the_two_lines,
    "a view guarding NaN by `raw != raw` fails the check",
    expected = "by a self-comparison",
    {
        let (e, _, wgsl) = numeric_views().remove(0);
        check_template(
            e.name,
            &wgsl.replace("bitcast<u32>(raw) == 0x7fc00000u", "raw != raw"),
        );
    }
);

/// The ramp line of `wgsl`: its last body line.
fn ramp_line(wgsl: &str) -> String {
    body(wgsl).last().map_or(String::new(), |l| (*l).to_owned())
}

/// Checks that each view's ramp places RQ-231's `raw` for its ledger scale, a `lin` field whose range spans zero as a
/// diverging one is (R-400), and that every scale is seen.
fn check_scales(views: &[(Entry, NumericView, String)]) {
    let mut seen = [false; 4];
    for (e, n, wgsl) in views {
        let ramp = ramp_line(wgsl);
        let scale = match e.scale {
            Scale::Lin if e.range.spans_zero() => Scale::Diverging,
            s => s,
        };
        let (k, want) = match scale {
            Scale::Lin => (0, "ramp_viridis(range_norm(raw, ".to_owned()),
            Scale::Log => (
                1,
                "ramp_viridis(range_norm(1.0 - 1.0 / (1.0 + log(1.0 + abs(raw) / 5.9604645e-8)), 0.0, 1.0, "
                    .to_owned(),
            ),
            Scale::Cyclic => (
                2,
                "ramp_twilight(range_norm(fract(raw / 6.2831855), 0.0, 1.0, ".to_owned(),
            ),
            Scale::Diverging => {
                let want = match (n.lo, n.hi) {
                    (End::Fixed(lo), End::Fixed(hi)) => {
                        let r = numeric::float(lo.abs().max(hi.abs()));
                        format!("ramp_coolwarm(range_norm(raw, -{r}, {r}, ")
                    }
                    _ => "ramp_coolwarm(range_norm(raw, -m, m, ".to_owned(),
                };
                assert!(
                    ramp.ends_with(", vec2<f32>(-m, m)));"),
                    "`{}`'s measured range is not symmetric: {ramp}",
                    e.name
                );
                (3, want)
            }
            _ => panic!("`{}` is numeric but {:?}", e.name, e.scale),
        };
        seen[k] = true;
        assert!(
            ramp.contains(&want),
            "`{}` ({:?}) does not place RQ-231's raw, `{want}`: {ramp}",
            e.name,
            e.scale
        );
    }
    assert_eq!(
        seen, [true; 4],
        "the scales seen: lin, log, cyclic, diverging"
    );
}

#[test]
fn numeric_view_template_scales_generate_their_raw() {
    check_scales(&numeric_views());
}

negative_control!(
    numeric_view_template_scales_generate_their_raw,
    "a log view placing raw linearly fails the check",
    expected = "does not place RQ-231's raw",
    {
        let views: Vec<_> = numeric_views()
            .into_iter()
            .map(|(e, n, wgsl)| {
                let wgsl = if e.scale == Scale::Log {
                    let ramp = ramp_line(&wgsl);
                    wgsl.replace(
                        &ramp,
                        "    return ramp_viridis(range_norm(raw, 0.0, 1.0, x));",
                    )
                } else {
                    wgsl
                };
                (e, n, wgsl)
            })
            .collect();
        check_scales(&views);
    }
);

#[test]
fn numeric_view_template_step_indices_use_the_horizon() {
    let views = numeric_views();
    for field in ["t_end_step", "t_dmin_step"] {
        let (_, n, wgsl) = views
            .iter()
            .find(|(e, ..)| e.name == field)
            .unwrap_or_else(|| panic!("no view of `{field}`"));
        assert_eq!((n.lo, n.hi), (End::Fixed(0.0), End::Horizon), "`{field}`");
        check_horizon(field, &ramp_line(wgsl));
    }
}

/// Checks that `ramp` places its value on `[0, horizon_steps]`.
fn check_horizon(field: &str, ramp: &str) {
    assert!(
        ramp.contains("range_norm(raw, 0.0, f32(ctx.params.horizon_steps), "),
        "`{field}` is not on [0, horizon_steps]: {ramp}"
    );
}

negative_control!(
    numeric_view_template_step_indices_use_the_horizon,
    "a step index on the ledger's register bound fails the check",
    expected = "is not on [0, horizon_steps]",
    check_horizon(
        "t_end_step",
        "    return ramp_viridis(range_norm(raw, 0.0, 65535.0, uniforms.RANGE_AUTO != 0u, uniforms.u_range));"
    )
);

/// A view of `placement` on the fixed `[lo, hi]`, with `sentinel`.
fn view(placement: Placement, lo: End, hi: End, sentinel: Option<Sentinel>) -> NumericView {
    NumericView {
        field: "probe",
        placement,
        lo,
        hi,
        sentinel,
        range_auto: false,
    }
}

/// The ramp position `n` shows `x` at, with the fixed range, or `None` off the ramp.
fn t(n: &NumericView, x: f32) -> Option<f64> {
    match n.shown(x, false, [0.0, 1.0], 1000) {
        Shown::Ramp { t, .. } => Some(t),
        _ => None,
    }
}

/// Checks the CPU twin's placement of each scale at its boundaries, `cyclic` the cyclic view under test.
fn check_twin_places(cyclic: &NumericView) {
    let lin = view(Placement::Lin, End::Fixed(0.0), End::Fixed(4.0), None);
    assert_eq!(lin.compact(-2.5), -2.5, "lin is the identity");
    assert_eq!(t(&lin, 1.0), Some(0.25));
    assert_eq!(t(&lin, -1.0), Some(0.0), "below lo clamps to 0");
    assert_eq!(t(&lin, 9.0), Some(1.0), "above hi clamps to 1");
    let floor = numeric::log_floor();
    let log = view(
        Placement::Log { floor },
        End::Fixed(0.0),
        End::Fixed(1.0),
        None,
    );
    assert_eq!(log.compact(0.0), 0.0, "log places 0 at 0");
    assert_eq!(log.compact(-0.3), log.compact(0.3), "log places |x|");
    let at_floor = 1.0 - 1.0 / (1.0 + 2f64.ln());
    assert!((log.compact(floor) - at_floor).abs() < 1e-15, "log at ε");
    let period = numeric::cyclic_period();
    assert_eq!(cyclic.compact(0.0), 0.0, "cyclic places 0 at 0");
    assert!(
        cyclic.compact(period).abs() < 1e-12,
        "a whole turn wraps to 0"
    );
    assert!(
        (cyclic.compact(-period / 4.0) - 0.75).abs() < 1e-12,
        "a quarter turn back is 0.75"
    );
    assert!(
        matches!(
            cyclic.shown(1.0, false, [0.0, 1.0], 1),
            Shown::Ramp {
                ramp: Ramp::Twilight,
                ..
            }
        ),
        "cyclic is on twilight"
    );
    let measured = view(Placement::Diverging, End::Measured, End::Measured, None);
    assert_eq!(measured.range([-1.0, 3.0], 1), ([-3.0, 3.0], [-3.0, 3.0]));
    let fixed = view(
        Placement::Diverging,
        End::Fixed(-2.0),
        End::Fixed(1.0),
        None,
    );
    assert_eq!(fixed.range([-1.0, 3.0], 1), ([-2.0, 2.0], [-3.0, 3.0]));
    let step = view(Placement::Lin, End::Fixed(0.0), End::Horizon, None);
    assert_eq!(t(&step, 500.0), Some(0.5), "a step on [0, horizon_steps]");
    let flat = view(Placement::Lin, End::Fixed(2.0), End::Fixed(2.0), None);
    assert_eq!(t(&flat, 7.0), Some(0.0), "a degenerate range places at 0");
    let open = view(Placement::Lin, End::Fixed(0.0), End::Measured, None);
    assert_eq!(
        open.range([1.0, 8.0], 1).0,
        [0.0, 8.0],
        "the open end is measured"
    );
    assert_eq!(
        open.shown(4.0, true, [2.0, 6.0], 1),
        Shown::Ramp {
            ramp: Ramp::Viridis,
            t: 0.5
        },
        "RANGE_AUTO places on the measured range"
    );
}

#[test]
fn numeric_view_template_scales_place_on_the_cpu_twin() {
    check_twin_places(&view(
        Placement::Cyclic {
            period: numeric::cyclic_period(),
        },
        End::Fixed(0.0),
        End::Fixed(1.0),
        None,
    ));
}

negative_control!(
    numeric_view_template_scales_place_on_the_cpu_twin,
    "a cyclic view of period 1, not a turn, fails the check",
    expected = "a whole turn wraps to 0",
    check_twin_places(&view(
        Placement::Cyclic { period: 1.0 },
        End::Fixed(0.0),
        End::Fixed(1.0),
        None,
    ))
);

#[test]
fn numeric_view_template_guard_and_sentinels_on_the_twin() {
    let literal = view(
        Placement::Lin,
        End::Fixed(0.0),
        End::Fixed(76.0),
        Some(Sentinel::Literal(127.0)),
    );
    let qnan = f32::from_bits(0x7fc0_0000);
    assert_eq!(literal.shown(qnan, false, [0.0, 1.0], 1), Shown::Invalid);
    let other_nan = f32::from_bits(0x7fc0_0001);
    assert_ne!(
        literal.shown(other_nan, false, [0.0, 1.0], 1),
        Shown::Invalid,
        "the guard is the canonical NaN's bits alone"
    );
    assert_eq!(
        literal.shown(127.0, false, [0.0, 1.0], 1),
        Shown::Literal(127.0)
    );
    for x in [126.0, 128.0, f32::from_bits(127.0f32.to_bits() + 1)] {
        assert!(t(&literal, x).is_some(), "{x} is on the ramp");
    }
    let unset = view(
        Placement::Lin,
        End::Fixed(0.0),
        End::Fixed(2.0),
        Some(Sentinel::Unset {
            bits: f32::INFINITY.to_bits(),
        }),
    );
    check_unset(&unset);
}

/// Checks that `n` draws +∞ in the "not yet" style and the largest finite f32 and −∞ on the ramp.
fn check_unset(n: &NumericView) {
    assert_eq!(
        n.shown(f32::INFINITY, false, [0.0, 1.0], 1),
        Shown::NotYet,
        "+inf is not the grey"
    );
    for x in [f32::MAX, f32::NEG_INFINITY] {
        assert!(t(n, x).is_some(), "{x} is on the ramp");
    }
}

negative_control!(
    numeric_view_template_guard_and_sentinels_on_the_twin,
    "a view with no unset value draws +inf on the ramp",
    expected = "+inf is not the grey",
    check_unset(&view(
        Placement::Lin,
        End::Fixed(0.0),
        End::Fixed(2.0),
        None
    ))
);

#[test]
fn numeric_view_template_measures_the_ramp_values_only() {
    let n = view(
        Placement::Lin,
        End::Fixed(0.0),
        End::Measured,
        Some(Sentinel::Literal(127.0)),
    );
    let qnan = f32::from_bits(0x7fc0_0000);
    check_measured(&n, &[3.0, qnan, 127.0, -1.0, 9.0], [-1.0, 9.0]);
    check_measured(&n, &[qnan, 127.0], [0.0, 1.0]);
    check_measured(&n, &[5.0], [5.0, 5.0]);
}

/// Checks that `n` measures `xs` as `want`.
fn check_measured(n: &NumericView, xs: &[f32], want: [f64; 2]) {
    let got = n.measured(xs);
    assert_eq!(got, want, "{xs:?} measures as {got:?}, not {want:?}");
}

negative_control!(
    numeric_view_template_measures_the_ramp_values_only,
    "a measurement taking the sentinel in reaches 127",
    expected = "not [-1.0, 9.0]",
    check_measured(
        &view(Placement::Lin, End::Fixed(0.0), End::Measured, None),
        &[3.0, 127.0, -1.0, 9.0],
        [-1.0, 9.0]
    )
);

/// Checks that each view defaults `RANGE_AUTO` to 1 exactly where its ledger range is unbounded and it is not cyclic,
/// in the view and in its header.
fn check_defaults(views: &[(Entry, NumericView, String)]) {
    let (mut unbounded, mut bounded) = (0, 0);
    for (e, n, wgsl) in views {
        let finite = |b: ledger::schema::Bound| match b {
            ledger::schema::Bound::Closed(v) | ledger::schema::Bound::Open(v) => v.is_finite(),
            _ => false,
        };
        let open = !(finite(e.range.lo) && finite(e.range.hi));
        let want = open && e.scale != Scale::Cyclic;
        if open {
            unbounded += 1;
        } else {
            bounded += 1;
        }
        let header = format!("// @uniform RANGE_AUTO: u32 = {} [0, 1]", u32::from(want));
        assert!(
            n.range_auto == want && wgsl.contains(&header),
            "`{}` does not default to RANGE_AUTO = {}",
            e.name,
            u32::from(want)
        );
    }
    assert!(
        unbounded > 0 && bounded > 0,
        "{unbounded} unbounded, {bounded} bounded"
    );
}

#[test]
fn numeric_view_template_unbounded_defaults_range_auto() {
    check_defaults(&numeric_views());
}

negative_control!(
    numeric_view_template_unbounded_defaults_range_auto,
    "views generated with RANGE_AUTO = 0 throughout miss the unbounded fields' default",
    expected = "does not default to RANGE_AUTO = 1",
    {
        let l = ledger::layout();
        let entries = entries(&l);
        let views: Vec<_> = numeric_views()
            .into_iter()
            .map(|(e, n, _)| {
                let wgsl =
                    numeric_view(&l.words, &entries, e.name, Some(false)).unwrap_or_default();
                (e, n.with_range_auto(false), wgsl)
            })
            .collect();
        check_defaults(&views);
    }
);

#[test]
fn numeric_view_template_has_no_view_for_the_uncoloured() {
    let l = ledger::layout();
    for e in entries(&l) {
        let none = matches!(e.scale, Scale::Categorical(_) | Scale::Flag)
            || e.floor.is_some()
            || matches!(e.ty, ledger::schema::FieldType::Vector { .. });
        assert_eq!(
            NumericView::of(&e).is_none(),
            none,
            "`{}`: a numeric view only for a numeric scalar with no floor",
            e.name
        );
    }
    for field in ["energy_drift", "Lz_drift", "state", "r"] {
        assert!(
            numeric_view(&l.words, &entries(&l), field, None).is_none(),
            "`{field}` keeps its view"
        );
    }
}

negative_control!(
    numeric_view_template_has_no_view_for_the_uncoloured,
    "the drifts, given a numeric view, would lose R-381's symlog default",
    expected = "keeps its view",
    {
        let l = ledger::layout();
        let field = "ftle";
        assert!(
            numeric_view(&l.words, &entries(&l), field, None).is_none(),
            "`{field}` keeps its view"
        );
    }
);

/// Checks that the header of `wgsl` declares `RANGE_AUTO: u32 = <on> [0, 1]` and `u_range: vec2<f32>`, by the
/// assembler's parser.
fn check_round_trip(wgsl: &str, on: bool) {
    let d = Declaration::parse(wgsl).unwrap_or_else(|e| panic!("{e}"));
    let names: Vec<&str> = d.uniforms.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(names, ["RANGE_AUTO", "u_range"], "the view's uniforms");
    let r = &d.uniforms[0];
    assert_eq!(r.ty, UniformType::U32);
    assert_eq!(r.range, Some((0.0, 1.0)));
    assert_eq!(
        r.default,
        vec![f64::from(u32::from(on))],
        "the header does not give the param back"
    );
    assert_eq!(d.uniforms[1].ty, UniformType::Vec2);
}

#[test]
fn numeric_view_template_range_auto_round_trips() {
    let l = ledger::layout();
    let entries = entries(&l);
    for field in ["ftle", "d_min", "rho_angle", "E_0", "length"] {
        for on in [false, true] {
            let wgsl = numeric_view(&l.words, &entries, field, Some(on))
                .unwrap_or_else(|| panic!("no numeric view of `{field}`"));
            check_round_trip(&wgsl, on);
        }
    }
}

negative_control!(
    numeric_view_template_range_auto_round_trips,
    "a generator ignoring the param writes the field's default back",
    expected = "does not give the param back",
    {
        let l = ledger::layout();
        let wgsl = numeric_view(&l.words, &entries(&l), "ftle", Some(true)).unwrap_or_default();
        check_round_trip(&wgsl, false);
    }
);

/// Checks that `float` writes each of `cases` as its literal.
fn check_floats(cases: &[(f64, &str)]) {
    for &(x, want) in cases {
        assert_eq!(numeric::float(x), want, "{x} is not written as {want}");
    }
}

/// The template's numbers are WGSL f32 literals: the f32's shortest round-trip form, with a decimal point unless it
/// has one or an exponent already.
#[test]
fn numeric_view_template_floats_are_wgsl_literals() {
    check_floats(&[
        (1.0, "1.0"),
        (0.0, "0.0"),
        (-2.0, "-2.0"),
        (2.5, "2.5"),
        (76.0, "76.0"),
        (1e-8, "1e-8"),
        (1e20, "1e20"),
        (numeric::log_floor(), "5.9604645e-8"),
        (numeric::cyclic_period(), "6.2831855"),
        (f64::from(f32::MAX), "3.4028235e38"),
    ]);
}

negative_control!(
    numeric_view_template_floats_are_wgsl_literals,
    "an integer-valued number written without its decimal point is no f32 literal",
    expected = "is not written as",
    check_floats(&[(76.0, "76")])
);

/// Checks that `float` refuses each of `cases`, a value with no WGSL f32 literal, by a panic naming it.
fn check_refused(cases: &[f64]) {
    for &x in cases {
        let got = std::panic::catch_unwind(|| numeric::float(x));
        match got {
            Ok(s) => panic!("{x} is written as `{s}`, not refused"),
            Err(e) => {
                let msg = e.downcast_ref::<String>().cloned().unwrap_or_default();
                assert!(msg.contains("has no WGSL literal"), "{x}: {msg}");
            }
        }
    }
}

/// WGSL has no literal for an infinity or a NaN: `float` refuses a value that is not finite as an f32, an f64 past
/// f32's range included (applied per R-369, code review 5469198081 N1).
#[test]
fn numeric_view_template_floats_refuse_non_finite() {
    check_refused(&[f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 1e39, -1e39]);
}

negative_control!(
    numeric_view_template_floats_refuse_non_finite,
    "a finite value, which has its literal",
    expected = "not refused",
    check_refused(&[1.5])
);

/// Checks that the diverging view `n`'s ramp line places raw on `[-r, r]`, `r` written as `want`, on `ramp_coolwarm`.
fn check_symmetric(n: &NumericView, want: &str) {
    let wgsl = n.colour("ctx.sample.probe");
    let ramp = ramp_line(&wgsl);
    assert!(
        ramp.contains(&format!("ramp_coolwarm(range_norm(raw, -{want}, {want}, ")),
        "the diverging view is not on [-{want}, {want}]: {ramp}"
    );
    assert!(ramp.ends_with(", vec2<f32>(-m, m)));"), "{ramp}");
}

/// A diverging view with both ends fixed places raw on the symmetric range of the larger magnitude, `[-2, 2]` for
/// `[-2, 1]` and for `[-0.5, 2]`; with one end unbounded, on `[−M, M]`, `M` the larger magnitude of the fixed end and
/// the measured one (R-400: "an unbounded end takes the measured end"); with both unbounded, on the measured `[-m, m]`.
#[test]
fn numeric_view_template_diverging_fixed_is_symmetric() {
    check_symmetric(
        &view(
            Placement::Diverging,
            End::Fixed(-2.0),
            End::Fixed(1.0),
            None,
        ),
        "2.0",
    );
    check_symmetric(
        &view(
            Placement::Diverging,
            End::Fixed(-0.5),
            End::Fixed(2.0),
            None,
        ),
        "2.0",
    );
    check_symmetric(
        &view(Placement::Diverging, End::Fixed(-1.0), End::Measured, None),
        "max(abs(-1.0), abs(uniforms.u_range.y))",
    );
    check_symmetric(
        &view(Placement::Diverging, End::Measured, End::Fixed(3.0), None),
        "max(abs(uniforms.u_range.x), abs(3.0))",
    );
    check_symmetric(
        &view(Placement::Diverging, End::Measured, End::Measured, None),
        "m",
    );
}

negative_control!(
    numeric_view_template_diverging_fixed_is_symmetric,
    "a fixed diverging view is not on the measured range",
    expected = "is not on [-m, m]",
    check_symmetric(
        &view(
            Placement::Diverging,
            End::Fixed(-2.0),
            End::Fixed(1.0),
            None
        ),
        "m"
    )
);

// ── R-400 and R-403 (TASK-M1-17): the ramp follows the declared range ──────────────────────────────────────────────

/// The ramp a view of `e` must draw on (R-400): twilight for the cyclic field, cool-warm where the declared range spans
/// zero, viridis otherwise.
fn want_ramp(e: &Entry) -> Ramp {
    if e.scale == Scale::Cyclic {
        Ramp::Twilight
    } else if e.range.spans_zero() {
        Ramp::Coolwarm
    } else {
        Ramp::Viridis
    }
}

/// Checks that each view draws on the ramp its declared range gives, in the view and in its WGSL; that each on
/// cool-warm places 0 at `t = ½`, a negative value below and a positive above, fixed and auto, on the symmetric
/// range; and that the three ramps are all seen.
fn check_ramps(views: &[(Entry, NumericView, String)]) {
    let mut seen = [false; 3];
    for (e, n, wgsl) in views {
        let want = want_ramp(e);
        seen[want as usize] = true;
        let line = ramp_line(wgsl);
        assert!(
            n.ramp() == want && line.starts_with(&format!("    return {}(", want.wgsl())),
            "`{}` over {} is not on {}: {line}",
            e.name,
            e.range,
            want.wgsl()
        );
        if want != Ramp::Coolwarm {
            continue;
        }
        for auto in [false, true] {
            let at = |x: f32| match n.shown(x, auto, [-0.4, 1.6], 1) {
                Shown::Ramp { ramp, t } => {
                    assert_eq!(ramp, Ramp::Coolwarm, "`{}` at {x}", e.name);
                    t
                }
                other => panic!("`{}` shows {x} as {other:?}", e.name),
            };
            let (zero, below, above) = (at(0.0), at(-0.3), at(0.3));
            assert!(
                zero == 0.5 && below < 0.5 && above > 0.5,
                "`{}` (auto {auto}) does not place 0 at the neutral: −0.3 at {below}, 0 at {zero}, 0.3 at {above}",
                e.name
            );
            assert!(
                (above - 0.5 - (0.5 - below)).abs() < 1e-12,
                "`{}` (auto {auto}) is not symmetric about 0",
                e.name
            );
        }
    }
    assert_eq!(
        seen, [true; 3],
        "the ramps seen: viridis, twilight, cool-warm"
    );
}

#[test]
fn numeric_view_template_zero_spanning_is_coolwarm_centred_at_zero() {
    let views = numeric_views();
    check_ramps(&views);
    let coolwarm: Vec<&str> = views
        .iter()
        .filter(|(_, n, _)| n.ramp() == Ramp::Coolwarm)
        .map(|(e, ..)| e.name)
        .collect();
    for field in [
        "E_0",
        "Lz_0",
        "C_ty",
        "S",
        "theta",
        "mean_y",
        "ftle",
        "diffusion",
    ] {
        assert!(coolwarm.contains(&field), "`{field}` is not on cool-warm");
    }
}

negative_control!(
    numeric_view_template_zero_spanning_is_coolwarm_centred_at_zero,
    "every view left on viridis, as TASK-M1-09 built it, puts E_0's zero mid-viridis",
    expected = "is not on ramp_coolwarm",
    check_ramps(
        &numeric_views()
            .into_iter()
            .map(|(e, n, wgsl)| (e, n, wgsl.replace("ramp_coolwarm(", "ramp_viridis(")))
            .collect::<Vec<_>>()
    )
);

/// A one-field entry `name`: an f32 at scalar index 0, `scale` over `range`, as the payload ledger writes one.
fn probe(name: &'static str, scale: Scale, range: Range) -> Entry {
    EntryBuilder::new(name)
        .location(Location::Scalar(0))
        .ty(FieldType::F32)
        .scale(scale)
        .range(range)
        .provenance(ledger::schema::Provenance::Decode)
        .consumers(&[ledger::schema::Consumer::Debug])
        .build()
        .unwrap_or_else(|e| panic!("{e}"))
}

/// The ramp the template gives a `lin` field over `[lo, hi]`.
fn lin_ramp(lo: Bound, hi: Bound) -> Ramp {
    let e = probe("probe", Scale::Lin, Range { lo, hi });
    NumericView::of(&e)
        .unwrap_or_else(|| panic!("no view of a lin field"))
        .ramp()
}

/// Checks the ramp at the range's boundary: a `lin` field reaches cool-warm only when `lo < 0 < hi`; an end at 0,
/// open or closed, keeps viridis, either side.
fn check_boundary(ramp: fn(Bound, Bound) -> Ramp) {
    use Bound::{Closed, Open, Unbounded};
    let cases = [
        (Unbounded, Unbounded, Ramp::Coolwarm),
        (Closed(-1.0), Closed(1.0), Ramp::Coolwarm),
        (Open(-1e-30), Open(1e-30), Ramp::Coolwarm),
        (Closed(0.0), Unbounded, Ramp::Viridis),
        (Open(0.0), Unbounded, Ramp::Viridis),
        (Unbounded, Closed(0.0), Ramp::Viridis),
        (Unbounded, Open(0.0), Ramp::Viridis),
        (Closed(0.0), Closed(1.0), Ramp::Viridis),
        (Closed(-2.0), Closed(-1.0), Ramp::Viridis),
        (Closed(1e-30), Unbounded, Ramp::Viridis),
    ];
    for (lo, hi, want) in cases {
        let got = ramp(lo, hi);
        let range = Range { lo, hi };
        assert_eq!(got, want, "a lin field over {range} is on {got:?}");
    }
}

#[test]
fn numeric_view_template_zero_spanning_boundary() {
    check_boundary(lin_ramp);
}

negative_control!(
    numeric_view_template_zero_spanning_boundary,
    "a rule counting a closed end at 0 as spanning zero puts [0, ∞) on cool-warm",
    expected = "a lin field over [0, ∞) is on Coolwarm",
    check_boundary(|lo, hi| {
        let below = !matches!(lo, Bound::Closed(v) | Bound::Open(v) if v > 0.0);
        let above = !matches!(hi, Bound::Closed(v) | Bound::Open(v) if v < 0.0);
        if below && above {
            Ramp::Coolwarm
        } else {
            Ramp::Viridis
        }
    })
);

/// Checks that the ledger's §3.8 keys are the seven required ones, `keys`, with no signed-or-not mark (R-400: "no new
/// ledger mark").
fn check_keys(keys: &[&str]) {
    assert_eq!(
        keys,
        [
            "name",
            "location",
            "type",
            "scale",
            "range",
            "provenance",
            "consumers"
        ],
        "the ledger's required keys changed"
    );
}

/// The ledger gains no member (REQ-RENDER-084): its required keys are §3.8's, and an entry is exactly §3.8's members,
/// which the destructuring below names, each, so a new one fails to compile.
#[test]
fn numeric_view_template_ledger_gains_no_member() {
    check_keys(&ledger::schema::REQUIRED_KEYS);
    for e in entries(&ledger::layout()) {
        let Entry {
            name: _,
            location: _,
            ty: _,
            scale: _,
            range: _,
            sentinel: _,
            tier_gate: _,
            overflow: _,
            floor: _,
            provenance: _,
            consumers: _,
        } = e;
    }
}

negative_control!(
    numeric_view_template_ledger_gains_no_member,
    "a `signed` key added to the ledger's required keys",
    expected = "the ledger's required keys changed",
    {
        let mut keys = ledger::schema::REQUIRED_KEYS.to_vec();
        keys.push("signed");
        check_keys(&keys);
    }
);

/// `K_0`'s and `V_0`'s views and their WGSL.
fn energies() -> [(NumericView, String); 2] {
    let views = numeric_views();
    ["K_0", "V_0"].map(|field| {
        let (_, n, wgsl) = views
            .iter()
            .find(|(e, ..)| e.name == field)
            .unwrap_or_else(|| panic!("no view of `{field}`"));
        (n.clone(), wgsl.clone())
    })
}

/// `n`'s place for `x`, with `RANGE_AUTO` `auto` and `u_range` measured over `xs`.
fn place(n: &NumericView, x: f32, auto: bool, xs: &[f32]) -> f64 {
    match n.shown(x, auto, n.measured(xs), 1) {
        Shown::Ramp { ramp, t } => {
            assert_eq!(ramp, Ramp::Viridis, "`{}` is not on ramp_viridis", n.field);
            t
        }
        other => panic!("`{}` shows {x} as {other:?}", n.field),
    }
}

/// Checks R-403's map: `K_0` and `V_0` on `ramp_viridis`, the template's `lin`, increasing in the value in both
/// modes, their headers defaulting to `RANGE_AUTO = 1`; under auto `t = 0` at the measured minimum; under fixed, `K_0`'s
/// `t = 0` at its declared 0 and `V_0`'s at its measured minimum, a positive `V_0` clamping to `t = 1`. `V_0`'s most
/// negative value is at `t = 0` in both modes.
fn check_energies([(k, kw), (v, vw)]: [(NumericView, String); 2]) {
    for (n, wgsl) in [(&k, &kw), (&v, &vw)] {
        let line = ramp_line(wgsl);
        assert!(
            n.placement == Placement::Lin && line.starts_with("    return ramp_viridis("),
            "`{}` is not on ramp_viridis, lin: {line}",
            n.field
        );
        assert!(
            n.range_auto && wgsl.contains("// @uniform RANGE_AUTO: u32 = 1 [0, 1]"),
            "`{}` does not default to RANGE_AUTO = 1",
            n.field
        );
    }
    let ks = [0.2f32, 0.45, 0.9, 1.3];
    let vs = [-2.1f32, -1.4, -0.9, -0.35];
    for auto in [false, true] {
        for (n, xs) in [(&k, &ks[..]), (&v, &vs[..])] {
            let ts: Vec<f64> = xs.iter().map(|&x| place(n, x, auto, xs)).collect();
            assert!(
                ts.windows(2).all(|w| w[0] < w[1]),
                "`{}` (auto {auto}) does not increase in the value: {ts:?}",
                n.field
            );
        }
        assert_eq!(
            place(&v, -2.1, auto, &vs),
            0.0,
            "`V_0`'s most negative value is not at t = 0 (auto {auto})"
        );
    }
    assert_eq!(
        place(&k, 0.2, true, &ks),
        0.0,
        "auto: `K_0`'s minimum at t = 0"
    );
    assert_eq!(
        place(&k, 0.0, false, &ks),
        0.0,
        "fixed: `K_0`'s declared 0 at t = 0"
    );
    assert!(
        place(&k, 0.2, false, &ks) > 0.0,
        "fixed: `K_0`'s minimum above its 0"
    );
    assert_eq!(
        place(&k, 1.3, false, &ks),
        1.0,
        "fixed: `K_0`'s top is measured"
    );
    let top = place(&v, -0.35, false, &vs);
    assert!(
        (top - (1.0 - 0.35 / 2.1)).abs() < 1e-6,
        "fixed: `V_0`'s top is not its 0: −0.35 at {top}"
    );
    assert_eq!(place(&v, 0.0, false, &vs), 1.0, "fixed: `V_0`'s 0 at t = 1");
    assert_eq!(
        place(&v, 10.0, false, &vs),
        1.0,
        "fixed: a positive `V_0` clamps to t = 1"
    );
}

#[test]
fn numeric_view_template_k0_v0_are_viridis_increasing() {
    check_energies(energies());
}

negative_control!(
    numeric_view_template_k0_v0_are_viridis_increasing,
    "`V_0` on its old `diverging` placement, before R-403, lands on cool-warm",
    expected = "`V_0` is not on ramp_viridis, lin",
    {
        let [k, (mut v, vw)] = energies();
        v.placement = Placement::Diverging;
        check_energies([k, (v, vw.replace("ramp_viridis", "ramp_coolwarm"))]);
    }
);
