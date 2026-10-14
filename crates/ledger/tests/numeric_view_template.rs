//! The numeric field view's template (`ledger::gen::numeric`; render_gui_spec §10.1; R-114, R-136; RQ-231;
//! TASK-M1-09), REQ-RENDER-022:
//! - every generated numeric view is the two-line template, the bitcast NaN guard then the `range_norm` ramp, with no
//!   self-comparison (`numeric_view_template_is_the_two_lines`);
//! - each ledger scale generates RQ-231's `raw`, and the CPU twin places it as the WGSL does
//!   (`numeric_view_template_scales_*`);
//! - a field with an unbounded ledger range defaults to `RANGE_AUTO = 1` (`numeric_view_template_unbounded_*`);
//! - REQ-RENDER-084 (R-400, R-403; TASK-M1-17): a field whose declared range spans zero is on `ramp_coolwarm` over the
//!   symmetric `[−M, M]`, fixed and auto, 0 at `t = ½`; a field with `lo ≥ 0` on `ramp_viridis`; the cyclic fields on
//!   `ramp_twilight`; the ledger gains no member (`numeric_view_template_zero_spanning_*`,
//!   `numeric_view_template_ledger_*`); `K_0` and `V_0` on `ramp_viridis`, increasing in the value in both modes,
//!   defaulting to `RANGE_AUTO = 1`, `V_0`'s most negative value at `t = 0` (`numeric_view_template_k0_v0_*`);
//! - REQ-GEN-035 (R-426; TASK-M1-21): the views on `ramp_coolwarm` are the eight that were before, `q_mass` and
//!   `mean_y` provisionally; a gap never spans zero (`numeric_view_template_coolwarm_views_kept_r426`);
//! - REQ-RENDER-085 (R-423; TASK-M1-20): `theta`, an angle, is cyclic: raw = fract(θ̃ / 2π), `fract(x) = x − ⌊x⌋`, on
//!   the fixed `[0, 1]` with `ramp_twilight`, defaulting to `RANGE_AUTO = 0`, one turn apart giving the same raw for
//!   either sign of `θ̃` (`numeric_view_template_theta_*`);
//! - REQ-TOOL-160 (R-401, R-419; TASK-M1-18): every log view places raw at `dbg_log`'s form with its own field's floor,
//!   in the WGSL and the CPU twin, each a power of two an f32 holds exactly; two log fields with different floors
//!   place the same `|x|` differently; a log field with no floor of its own is refused (`numeric_view_template_log_*`);
//! - setting the `RANGE_AUTO` param writes the header default `@uniform RANGE_AUTO: u32 = <0|1> [0, 1]`, which the
//!   assembler's `Declaration::parse` gives back (`numeric_view_template_range_auto_round_trips`).
//!
//! Each test registers its negative control (R-176).

use ledger::constants::{LOG_FLOORS, LOG_FLOOR_CLOSURE_MIN, LOG_FLOOR_DE_MAX, LOG_FLOOR_D_MIN};
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
                format!(
                    "ramp_viridis(range_norm(1.0 - 1.0 / (1.0 + log(1.0 + abs(raw) / {})), 0.0, 1.0, ",
                    numeric::float(want_floor(e.name))
                ),
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
    let floor = LOG_FLOOR_CLOSURE_MIN.number();
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
        (LOG_FLOOR_D_MIN.number(), "6.1035156e-5"),
        (LOG_FLOOR_DE_MAX.number(), "9.536743e-7"),
        (LOG_FLOOR_CLOSURE_MIN.number(), "1.1920929e-7"),
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

/// The ramp a view of `e` must draw on (R-400, R-423, R-426): twilight for a cyclic field, cool-warm where the
/// declared (or provisional) range spans zero, viridis otherwise; a gap never spans zero.
fn want_ramp(e: &Entry) -> Ramp {
    if e.scale == Scale::Cyclic {
        Ramp::Twilight
    } else if ledger::payload::spans_zero(e) {
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
        "q_mass",
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

/// The fields on `ramp_coolwarm` before R-426 (TASK-M1-17, TASK-M1-20): R-426 moves none (REQ-GEN-035).
const COOLWARM: [&str; 8] = [
    "S",
    "mean_y",
    "C_ty",
    "E_0",
    "Lz_0",
    "ftle",
    "diffusion",
    "q_mass",
];

/// Checks that the views on `ramp_coolwarm`, in `views`, are exactly [`COOLWARM`].
fn check_coolwarm_kept(views: &[(Entry, NumericView, String)]) {
    let mut coolwarm: Vec<&str> = views
        .iter()
        .filter(|(_, n, wgsl)| {
            n.ramp() == Ramp::Coolwarm || ramp_line(wgsl).contains("ramp_coolwarm(")
        })
        .map(|(e, ..)| e.name)
        .collect();
    let mut want = COOLWARM.to_vec();
    coolwarm.sort_unstable();
    want.sort_unstable();
    assert_eq!(coolwarm, want, "the views on cool-warm changed (R-426)");
}

#[test]
fn numeric_view_template_coolwarm_views_kept_r426() {
    check_coolwarm_kept(&numeric_views());
}

negative_control!(
    numeric_view_template_coolwarm_views_kept_r426,
    "`q_mass` read as a gap leaves cool-warm, which R-426 keeps it on provisionally",
    expected = "the views on cool-warm changed",
    check_coolwarm_kept(
        &numeric_views()
            .into_iter()
            .filter(|(e, ..)| e.name != "q_mass")
            .collect::<Vec<_>>()
    )
);

/// A one-field entry `name`: an f32 at scalar index 0, `scale` over `range`, declared, as the payload ledger writes
/// one (R-426: an uncited (−∞, ∞) would be a gap).
fn probe(name: &'static str, scale: Scale, range: Range) -> Entry {
    EntryBuilder::new(name)
        .location(Location::Scalar(0))
        .ty(FieldType::F32)
        .scale(scale)
        .range(range)
        .cited("a probe's declared range, R-400's boundary cases (dd_generation_root §3.8, R-426)")
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
/// `declaration` being the state half of §3.8's `range` key (R-426), which the destructuring below names, each, so a
/// new one fails to compile.
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
            declaration: _,
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

/// Checks R-403's map: `K_0` and `V_0` on `ramp_viridis`, the template's `lin`, increasing in the value in both modes,
/// their headers defaulting to `RANGE_AUTO = 1`; under auto `t = 0` at the measured minimum; under fixed, `K_0`'s
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

// ── R-423 (TASK-M1-20): `theta`, an angle, on the cyclic ramp ────────────────────────────────────────────────────────

/// Checks that `wgsl`, the generated view of `theta` (`n`), is cyclic (REQ-RENDER-085): one turn, 2π, its period;
/// `raw = fract(θ̃ / 2π)` on the fixed `[0, 1]` with `ramp_twilight`; its header defaulting to `RANGE_AUTO = 0`.
fn check_theta_view(n: &NumericView, wgsl: &str) {
    let line = ramp_line(wgsl);
    assert!(
        n.ramp() == Ramp::Twilight
            && line.starts_with(
                "    return ramp_twilight(range_norm(fract(raw / 6.2831855), 0.0, 1.0, "
            ),
        "`theta` is not on ramp_twilight at fract(θ̃ / 2π): {line}"
    );
    assert_eq!(
        n.placement,
        Placement::Cyclic {
            period: std::f64::consts::TAU
        },
        "`theta`'s period"
    );
    assert_eq!(
        (n.lo, n.hi),
        (End::Fixed(0.0), End::Fixed(1.0)),
        "`theta`'s range"
    );
    assert!(!n.range_auto, "`theta` defaults to RANGE_AUTO = 1");
    check_round_trip(wgsl, false);
}

/// `theta`'s view and generated WGSL, `RANGE_AUTO` at its default, from the ledger `entries`.
fn theta_view(entries: &[Entry]) -> (NumericView, String) {
    let l = ledger::layout();
    let e = entries
        .iter()
        .find(|e| e.name == "theta")
        .unwrap_or_else(|| panic!("no `theta` in the ledger"));
    let n = NumericView::of(e).unwrap_or_else(|| panic!("`theta` has no numeric view"));
    let wgsl = numeric_view(&l.words, entries, "theta", None)
        .unwrap_or_else(|| panic!("`theta` has a numeric view but no WGSL"));
    (n, wgsl)
}

#[test]
fn numeric_view_template_theta_is_cyclic_on_twilight() {
    let (n, wgsl) = theta_view(&entries(&ledger::layout()));
    check_theta_view(&n, &wgsl);
}

negative_control!(
    numeric_view_template_theta_is_cyclic_on_twilight,
    "`theta` as R-400 left it, lin over (−∞, ∞), is on ramp_coolwarm",
    expected = "is not on ramp_twilight",
    {
        let mut entries = entries(&ledger::layout());
        for e in entries.iter_mut().filter(|e| e.name == "theta") {
            e.scale = Scale::Lin;
        }
        let (n, wgsl) = theta_view(&entries);
        check_theta_view(&n, &wgsl);
    }
);

/// The distance between `a` and `b` modulo one turn of raw, 1: the shorter way round the cyclic ramp, so a raw of 1.0
/// (`fract` of a negative value rounding up to 1, which WGSL's `fract` allows) reads as 0.
fn turn_distance(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(1.0);
    d.min(1.0 - d)
}

/// Checks that `raw`, the raw a cyclic view gives `θ̃`, is `fract(θ̃ / 2π)` with `fract(x) = x − ⌊x⌋`
/// (REQ-RENDER-085): `θ̃ = −π/2` gives 0.75; each `θ̃` gives its raw on `[0, 1]`, `x − ⌊x⌋` in f64; and `θ̃`, `θ̃ + 2π`
/// and `θ̃ − 2π` give the same raw modulo 1, for negative `θ̃` as for positive.
fn check_fract(raw: impl Fn(f32) -> f64) {
    use std::f64::consts::{FRAC_PI_2, PI, TAU};
    let quarter = raw(-FRAC_PI_2 as f32);
    assert!(
        (quarter - 0.75).abs() < 1e-6,
        "θ̃ = −π/2 gives raw {quarter}, not 0.75"
    );
    for x in [
        0.0, 1e-30, -1e-30, 0.5, -0.5, 1.0, -1.0, FRAC_PI_2, PI, -PI, 3.0, -3.0, 6.0, -6.0, 7.5,
        -7.5, 20.0, -20.0, 100.25, -100.25,
    ] {
        let theta = x as f32;
        let r = raw(theta);
        let y = f64::from(theta) / TAU;
        assert!(
            (0.0..=1.0).contains(&r),
            "θ̃ = {theta} gives raw {r}, off [0, 1]"
        );
        assert!(
            r == y - y.floor(),
            "θ̃ = {theta} gives raw {r}, not x − ⌊x⌋ = {}",
            y - y.floor()
        );
        for turn in [TAU, -TAU] {
            let other = (f64::from(theta) + turn) as f32;
            let d = turn_distance(r, raw(other));
            assert!(
                d < 1e-6,
                "θ̃ = {theta} and {other}, one turn apart, give raws {d} apart"
            );
        }
    }
}

/// The raw `theta`'s view gives `x`: its `t` on the fixed `[0, 1]`, on `ramp_twilight`.
fn theta_raw(n: &NumericView, x: f32) -> f64 {
    match n.shown(x, false, [0.0, 1.0], 1) {
        Shown::Ramp {
            ramp: Ramp::Twilight,
            t,
        } => t,
        other => panic!("`theta` shows {x} as {other:?}"),
    }
}

#[test]
fn numeric_view_template_theta_raw_is_fract_of_a_turn() {
    let (n, _) = theta_view(&entries(&ledger::layout()));
    check_fract(|x| theta_raw(&n, x));
}

negative_control!(
    numeric_view_template_theta_raw_is_fract_of_a_turn,
    "a trunc-based fract, Rust's f64::fract (x − trunc x), gives −0.25 at θ̃ = −π/2",
    expected = "not 0.75",
    check_fract(|x| (f64::from(x) / std::f64::consts::TAU).fract())
);

// ── R-401 and R-419 (TASK-M1-18): each log field's own floor (REQ-TOOL-160) ─────────────────────────────────────────

/// The proposed floors (R-71, REQ-TOOL-160), each a power of two, `(field, log2 ε_f)`: the distances 2⁻¹⁴, binary16's
/// smallest normal; the drift latches 2⁻²⁰, nearest `eps_E` and `eps_L`'s default 10⁻⁶; `closure_min` and `rho_ratio`
/// 2⁻²³, f32's spacing on `[1, 2)`.
const FLOORS: [(&str, i32); 6] = [
    ("d_min", -14),
    ("dE_max", -20),
    ("dLz_max", -20),
    ("closure_min", -23),
    ("rho_ratio", -23),
    ("r_min_pair_0", -14),
];

/// `field`'s proposed floor, from [`FLOORS`].
fn want_floor(field: &str) -> f64 {
    let (_, k) = FLOORS
        .iter()
        .find(|(f, _)| *f == field)
        .unwrap_or_else(|| panic!("`{field}` is no log field of REQ-TOOL-160's"));
    2f64.powi(*k)
}

/// `dbg_log`'s place of `x` for the floor `eps` (render contract, presentation layer).
fn dbg_log_place(x: f64, eps: f64) -> f64 {
    1.0 - 1.0 / (1.0 + (1.0 + x.abs() / eps).ln())
}

/// `field`'s floor as the constants register gives it ([`LOG_FLOORS`]; R-427), or `None` for a field with none.
fn register_floor(field: &str) -> Option<f64> {
    LOG_FLOORS
        .iter()
        .find(|(f, _)| *f == field)
        .map(|(_, k)| k.number())
}

/// Checks that the log views are exactly `floors`' fields and that each places raw at `dbg_log`'s form with its own
/// floor: its register entry gives it, provisional until the M1 gate (R-427), the view carries it, an f32 holds it
/// exactly, the WGSL writes it, and the CPU twin places `0`, `ε_f`, `ε_f·(e − 1)` and values about it as `dbg_log`
/// does. A field with no log view has no floor.
fn check_floors(views: &[(Entry, NumericView, String)], floors: &[(&str, i32)]) {
    let logs: Vec<&str> = views
        .iter()
        .filter(|(e, ..)| e.scale == Scale::Log)
        .map(|(e, ..)| e.name)
        .collect();
    let mut want: Vec<&str> = floors.iter().map(|(f, _)| *f).collect();
    let mut got = logs.clone();
    want.sort_unstable();
    got.sort_unstable();
    assert_eq!(got, want, "the log views are not REQ-TOOL-160's six");
    for &(field, k) in floors {
        let eps = 2f64.powi(k);
        let (_, n, wgsl) = views
            .iter()
            .find(|(e, ..)| e.name == field)
            .unwrap_or_else(|| panic!("no view of `{field}`"));
        assert_eq!(
            register_floor(field),
            Some(eps),
            "`{field}`'s floor is not its own, 2^{k}"
        );
        let (_, entry) = LOG_FLOORS
            .iter()
            .find(|(f, _)| *f == field)
            .unwrap_or_else(|| panic!("no register entry for `{field}`'s floor"));
        assert_eq!(
            entry.provisional,
            Some("M1"),
            "`{field}`'s floor is not provisional until M1"
        );
        assert_eq!(
            n.placement,
            Placement::Log { floor: eps },
            "`{field}`'s view is not placed on its own floor, 2^{k}"
        );
        assert_eq!(
            f64::from(eps as f32),
            eps,
            "`{field}`'s floor is not exact in f32"
        );
        let form = format!(
            "1.0 - 1.0 / (1.0 + log(1.0 + abs(raw) / {})), ",
            numeric::float(eps)
        );
        assert!(
            ramp_line(wgsl).contains(&form),
            "`{field}`'s WGSL does not place raw at dbg_log's form with its floor 2^{k}: {}",
            ramp_line(wgsl)
        );
        assert_eq!(n.compact(0.0), 0.0, "`{field}` places 0 at 0");
        assert!(
            (n.compact(eps * (std::f64::consts::E - 1.0)) - 0.5).abs() < 1e-12,
            "`{field}` does not place ε_f·(e − 1) at ½"
        );
        for x in [eps / 8.0, eps, -eps, 3.0 * eps, 1e3 * eps, 1.0] {
            assert_eq!(
                n.compact(x),
                dbg_log_place(x, eps),
                "`{field}`'s twin does not place {x} at dbg_log's form with its floor 2^{k}"
            );
        }
    }
    for field in [
        "ftle",
        "energy_drift",
        "Lz_drift",
        "K_0",
        "theta",
        "t_end_step",
    ] {
        assert_eq!(register_floor(field), None, "`{field}` has a display floor");
    }
}

#[test]
fn numeric_view_template_log_floors_are_each_fields_own() {
    check_floors(&numeric_views(), &FLOORS);
}

negative_control!(
    numeric_view_template_log_floors_are_each_fields_own,
    "`d_min` read against `dE_max`'s floor, the two swapped",
    expected = "`d_min`'s floor is not its own, 2^-20",
    {
        let mut floors = FLOORS;
        floors[0].1 = -20;
        floors[1].1 = -14;
        check_floors(&numeric_views(), &floors);
    }
);

#[cfg(feature = "controls")]
mod log_floors_single_eps {
    use super::*;
    negative_control!(
        numeric_view_template_log_floors_are_each_fields_own,
        "TASK-M1-09's single ε = 2⁻²⁴, which R-401 replaces, read for every field",
        expected = "is not its own, 2^-24",
        check_floors(&numeric_views(), &FLOORS.map(|(f, _)| (f, -24)))
    );
}

/// Checks that, for each pair of log fields whose floors differ, the two views place the same `|x|` differently, the
/// field with the smaller floor higher, in the CPU twin and on the ramp; and that some pair differs.
fn check_differ(views: &[(Entry, NumericView, String)]) {
    let logs: Vec<&NumericView> = views
        .iter()
        .filter(|(e, ..)| e.scale == Scale::Log)
        .map(|(_, n, _)| n)
        .collect();
    let floor = |n: &NumericView| match n.placement {
        Placement::Log { floor } => floor,
        other => panic!("`{}` is log-scaled but placed {other:?}", n.field),
    };
    let mut pairs = 0;
    for a in &logs {
        for b in &logs {
            if floor(a) >= floor(b) {
                continue;
            }
            pairs += 1;
            for x in [1e-6f32, 1e-3, 0.25, 4.0] {
                let (ta, tb) = (t(a, x), t(b, x));
                assert!(
                    a.compact(f64::from(x)) > b.compact(f64::from(x)) && ta > tb,
                    "`{}` and `{}`, floors {} and {}, do not place {x} apart: {ta:?}, {tb:?}",
                    a.field,
                    b.field,
                    floor(a),
                    floor(b)
                );
            }
        }
    }
    assert!(pairs > 0, "no two log fields have different floors");
}

#[test]
fn numeric_view_template_log_floors_place_the_same_x_differently() {
    check_differ(&numeric_views());
}

negative_control!(
    numeric_view_template_log_floors_place_the_same_x_differently,
    "every log view on one floor, TASK-M1-09's single ε",
    expected = "no two log fields have different floors",
    check_differ(
        &numeric_views()
            .into_iter()
            .map(|(e, mut n, wgsl)| {
                if let Placement::Log { .. } = n.placement {
                    n.placement = Placement::Log {
                        floor: 2f64.powi(-24),
                    };
                }
                (e, n, wgsl)
            })
            .collect::<Vec<_>>()
    )
);

/// Checks that the view of a log field named `name` with no ledger `floor` is refused when `name` has no display floor
/// of its own (R-401: no default), by a panic naming it, and given one otherwise.
fn check_refused_floor(name: &'static str, refused: bool) {
    let e = probe(
        name,
        Scale::Log,
        Range {
            lo: Bound::Open(0.0),
            hi: Bound::Unbounded,
        },
    );
    let got = std::panic::catch_unwind(|| NumericView::of(&e));
    match got {
        Ok(n) => assert!(
            !refused,
            "`{name}`, log-scaled with no floor, is not refused: {n:?}"
        ),
        Err(p) => {
            let msg = p.downcast_ref::<String>().cloned().unwrap_or_default();
            assert!(
                refused
                    && msg.contains(&format!("`{name}` is log-scaled"))
                    && msg.contains("R-401"),
                "`{name}` is refused: {msg}"
            );
        }
    }
}

#[test]
fn numeric_view_template_log_field_without_a_floor_is_refused() {
    check_refused_floor("probe", true);
    check_refused_floor("closure_min", false);
}

negative_control!(
    numeric_view_template_log_field_without_a_floor_is_refused,
    "a field with its own floor, read as one that has none",
    expected = "is not refused",
    check_refused_floor("rho_ratio", true)
);
