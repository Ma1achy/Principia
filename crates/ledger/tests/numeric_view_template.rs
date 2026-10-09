//! The numeric field view's template (`ledger::gen::numeric`; render_gui_spec §10.1; R-114, R-136; RQ-231;
//! TASK-M1-09), REQ-RENDER-022:
//! - every generated numeric view is the two-line template, the bitcast NaN guard then the `range_norm` ramp, with no
//!   self-comparison (`numeric_view_template_is_the_two_lines`);
//! - each ledger scale generates RQ-231's `raw`, and the CPU twin places it as the WGSL does
//!   (`numeric_view_template_scales_*`);
//! - a field with an unbounded ledger range defaults to `RANGE_AUTO = 1` (`numeric_view_template_unbounded_*`);
//! - setting the `RANGE_AUTO` param writes the header default `@uniform RANGE_AUTO: u32 = <0|1> [0, 1]`, which the
//!   assembler's `Declaration::parse` gives back (`numeric_view_template_range_auto_round_trips`).
//!
//! Each test registers its negative control (R-176).

use ledger::gen::catalogue::numeric_view;
use ledger::gen::numeric::{self, End, NumericView, Placement, Sentinel, Shown};
use ledger::schema::{Entry, Ledger, Scale};
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
            || last.starts_with("    return ramp_twilight(range_norm("))
            && last.contains(", uniforms.RANGE_AUTO != 0u, "),
        "the ramp last",
    );
    for line in &lines[2..lines.len() - 1] {
        let sentinel =
            line.starts_with("    if (raw == ") && line.contains("{ return dbg_sentinel(");
        let unset = *line == "    if (bitcast<u32>(raw) == 0x7f800000u) { return DBG_NOT_YET; }";
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

/// Checks that each view's ramp places RQ-231's `raw` for its ledger scale, and that every scale is seen.
fn check_scales(views: &[(Entry, NumericView, String)]) {
    let mut seen = [false; 4];
    for (e, n, wgsl) in views {
        let ramp = ramp_line(wgsl);
        let (k, want) = match e.scale {
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
                        format!("ramp_viridis(range_norm(raw, -{r}, {r}, ")
                    }
                    _ => "ramp_viridis(range_norm(raw, -m, m, ".to_owned(),
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
            Shown::Ramp { twilight: true, .. }
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
            twilight: false,
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

/// Checks that `n` draws +∞ in the grey and the largest finite f32 and −∞ on the ramp.
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

/// Checks that the diverging view `n`'s ramp line places raw on `[-r, r]`, `r` written as `want`.
fn check_symmetric(n: &NumericView, want: &str) {
    let wgsl = n.colour("ctx.sample.probe");
    let ramp = ramp_line(&wgsl);
    assert!(
        ramp.contains(&format!("range_norm(raw, -{want}, {want}, ")),
        "the diverging view is not on [-{want}, {want}]: {ramp}"
    );
    assert!(ramp.ends_with(", vec2<f32>(-m, m)));"), "{ramp}");
}

/// A diverging view with both ends fixed places raw on the symmetric range of the larger magnitude, `[-2, 2]` for
/// `[-2, 1]` and for `[-0.5, 2]`; with an end unbounded, on the measured symmetric `[-m, m]`.
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
