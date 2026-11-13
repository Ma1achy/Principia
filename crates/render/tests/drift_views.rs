//! The drift views, rendered on the GPU through the golden harness's scenes (render contract Part 5; R-379, R-381;
//! RQ-208, decided per R-369, option 3; REQ-TOOL-149; TASK-M3-05). Each expected pixel is computed here from the
//! definition's formulas, over the value the kernel's read side gives:
//! - `drift_suspect_styling_*`: an energy-only suspect (Euler's stored latches on dd test 2, its `dLz_max` set below
//!   the threshold), an L_z-only suspect (set through `dLz_max`) and a non-suspect (KDK's stored latches on test 2).
//!   Rows are the energy predicate's and columns the L_z predicate's, in every drift view; the absence NaN is hatched
//!   and never styled.
//! - `drift_view_compaction_*`: `symlog` on `diverging` by default, viridis under the palette swap, `lin` at
//!   `range_norm` over the clamped `[−R, R]` on `diverging`, and `log` of `|x|` on viridis.
//!
//! **Domain** (pitfalls §10): one state with dyadic positions and momenta, so that `L_z` and its dyadic drifts are exact
//! in f32 on every backend, and drifts of magnitude at least `R/8`, where the f32 rounding of `H` moves a colour by less
//! than the tolerance. Near zero, `symlog` at the default floor resolves that rounding, so the views are not compared
//! there (`golden_scene::CASE_PARAMS`).
//! - `drift_view_neutral_*`: `symlog`'s neutral band (render contract Part 5; R-652, R-928, R-929; REQ-TOOL-227): a
//!   fresh sample renders exactly neutral on the backend the test runs on, in the three views and through a field ramp
//!   on each drift, references near zero included; a drift past the band still shows; and the band scales by the
//!   rounding, `15·2⁻²³·(K + |V|)` and `5·2⁻²³·S`, one definition wherever the drift is read.
//!   On Vulkan these run under CI's `ci` job, on lavapipe; on Metal under the `gpu-metal` suites' job and locally.
//! - `drift_view_non_finite_*`: the non-finite guard (render contract Part 5; R-436): `x = ±∞` on a finite reference
//!   hatches in every view under every compaction, where the backend delivers `∞`'s bits; and the `L_z` view keeps its
//!   finite drift, the energy predicate unlit, on a non-finite `E_0`.

use ledger::constants::{DRIFT_LIN_R_E, DRIFT_LIN_R_LZ, EPS_E_DEFAULT, EPS_L_DEFAULT};
use ledger::gen::catalogue;
use render::colour::field_ramp::{
    Compaction, FieldRamp, Lut, Map, Ramp, Reads, ScalarField, Shown as RampShown,
};
use render::present::{self, Rgb};
use render::registry;
use validation::drift;
use validation::golden_scene::{drift_suspects, row_scene, Colouring, DebugCase, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The largest per-channel difference, linear RGB, between a render and its expected colour, `debug_views.rs`'s.
const TOL: f64 = 1e-5;

/// The tile's state: dyadic, so `L_z` is exact in f32.
const R: [[f32; 2]; 3] = [[0.625, -0.25], [-0.5, 0.5], [-0.125, -0.25]];

/// The tile's momenta (the note on [`R`]).
const P: [[f32; 2]; 3] = [[0.125, 0.25], [-0.25, -0.0625], [0.125, -0.1875]];

/// A drift view: its registry id, whether it shows `energy_drift` rather than `Lz_drift`, its floor and its `lin` range.
#[derive(Clone, Copy)]
struct View {
    id: &'static str,
    energy: bool,
    eps: f64,
    lin_r: f64,
}

const VIEWS: [View; 3] = [
    View {
        id: "debug/generated/energy_drift",
        energy: true,
        eps: EPS_E_DEFAULT.number(),
        lin_r: DRIFT_LIN_R_E.number(),
    },
    View {
        id: "debug/generated/Lz_drift",
        energy: false,
        eps: EPS_L_DEFAULT.number(),
        lin_r: DRIFT_LIN_R_LZ.number(),
    },
    View {
        id: "debug/derived/energy_drift",
        energy: true,
        eps: EPS_E_DEFAULT.number(),
        lin_r: DRIFT_LIN_R_E.number(),
    },
];

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// A row of `samples` tiles coloured by `colouring`.
fn row(samples: u32, colouring: Colouring) -> Scene {
    row_scene("drift view", samples, colouring).unwrap_or_else(|e| panic!("{e}"))
}

/// The view `id` at `params` over its header's defaults.
fn debug(id: &str, params: &[(&str, f64)]) -> Colouring {
    Colouring::Debug(Box::leak(Box::new(DebugCase {
        name: format!("test {id}"),
        id: id.to_owned(),
        params: params
            .iter()
            .map(|(k, v)| ((*k).to_owned(), vec![*v]))
            .collect(),
        nudge: 0,
    })))
}

/// The view `id`'s WGSL as registered, with `from` replaced by `to`: a broken view for a control.
fn broken(id: &str, from: &str, to: &str) -> Colouring {
    let source = registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no registry entry `{id}`"))
        .source;
    assert!(source.contains(from), "`{id}` has no `{from}`");
    Colouring::Probe(source.replace(from, to), Vec::new())
}

/// Sample `i` at the tile's state with its stored `E_0`, `Lz_0` and latches.
fn fill(s: &mut Scene, i: u32, e_0: f32, lz_0: f32, de_max: f32, dlz_max: f32) {
    s.set
        .sample(i)
        .r(R)
        .p(P)
        .E_0(e_0)
        .Lz_0(lz_0)
        .times(1000, 500)
        .drift_max(de_max, dlz_max);
}

/// `(H, L_z)` of the tile's state with sample `i`'s masses, read through the kernel's read side: the drifts from
/// `E_0 = Lz_0 = 0`. The sample is left for the caller to fill.
fn invariants(s: &mut Scene, i: u32) -> (f32, f32) {
    fill(s, i, 0.0, 0.0, 0.0, 0.0);
    let read = s.read(i);
    (read.energy_drift, read.Lz_drift)
}

/// The run of `occupant` at f32 on dd test 2's orbit.
fn on_test_2(occupant: &str) -> drift::Drifts {
    drift::measure(&drift::figure_eight())
        .into_iter()
        .find(|m| m.occupant == occupant)
        .unwrap_or_else(|| panic!("no occupant `{occupant}`"))
        .f32
}

/// The drift `view` shows at sample `i`, as the read side reads it.
fn shown(view: View, s: &Scene, i: u32) -> f32 {
    let read = s.read(i);
    if view.energy {
        read.energy_drift
    } else {
        read.Lz_drift
    }
}

fn render(h: &GpuHarness, s: &Scene) -> Vec<Rgb> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect()
}

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

/// Checks every pixel of sample `i`'s tile against `want`, given the pixel's coordinates.
fn check_tile(s: &Scene, image: &[Rgb], i: u32, want: &dyn Fn(u32, u32) -> Rgb, what: &str) {
    let width = s.size().0;
    for (x, y) in s.pixels(i) {
        let got = image[(y * width + x) as usize];
        let colour = want(x, y);
        assert!(
            near(got, colour),
            "pixel ({x}, {y}) of sample {i} of `{}` is {got:?}, not {what} {colour:?}",
            s.name
        );
    }
}

/// `s = sign(x)·ln(1 + |x|/ε)`, the compacted value.
fn symlog(x: f32, eps: f64) -> f64 {
    let x = f64::from(x);
    (1.0 + x.abs() / eps).ln().copysign(x)
}

/// `symlog`'s place on `diverging`, `t = 0.5 + 0.5·s/(1 + |s|)`.
fn symlog_diverging(x: f32, eps: f64) -> Rgb {
    let s = symlog(x, eps);
    present::drift_ramp(0.5 + 0.5 * s / (1.0 + s.abs()), false)
}

/// The suspect styling over `rgb` at the pixel `(x, y)`: rows for `energy`, columns for `lz`, in ink by luminance.
fn styled(rgb: Rgb, energy: bool, lz: bool, x: u32, y: u32) -> Rgb {
    if (energy && y.is_multiple_of(4)) || (lz && x.is_multiple_of(4)) {
        let luminance = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
        [if luminance >= 0.18 { 0.0 } else { 1.0 }; 3]
    } else {
        rgb
    }
}

/// The styling scene (the module's note): sample 0 energy-only, 1 L_z-only, 2 neither.
fn styling_scene(colouring: Colouring) -> Scene {
    let euler = on_test_2("Euler");
    let kdk = on_test_2("KDK");
    let tau_e = ledger::constants::SUSPECT_ENERGY_THRESHOLD.number() as f32;
    let tau_l = ledger::constants::SUSPECT_LZ_THRESHOLD.number() as f32;
    let mut s = row(3, colouring);
    let (h, lz) = invariants(&mut s, 1);
    let e_0 = h - 0.25;
    fill(
        &mut s,
        0,
        euler.e_0_stored,
        lz - 0.25,
        euler.de_max_stored as f32,
        tau_l / 2.0,
    );
    fill(
        &mut s,
        1,
        e_0,
        lz - 0.25,
        e_0.abs() * tau_e / 2.0,
        tau_l * 2.0,
    );
    fill(
        &mut s,
        2,
        kdk.e_0_stored,
        lz - 0.25,
        kdk.de_max_stored as f32,
        kdk.dlz_max_stored as f32,
    );
    s
}

/// The predicates the scene's samples are to light, `(SUSPECT_ENERGY, SUSPECT_LZ)`.
const LIT: [(bool, bool); 3] = [(true, false), (false, true), (false, false)];

/// Checks each sample's pixels against the drift on `diverging` at `symlog`, under the styling `lit`'s predicates
/// drive: a pixel that shows the other predicate's style, lacks its own, or differs from the ramp fails.
fn check_styles(view: View, s: &Scene, image: &[Rgb], lit: [(bool, bool); 3]) {
    for (i, (energy, lz)) in (0u32..).zip(lit) {
        let rgb = symlog_diverging(shown(view, s, i), view.eps);
        check_tile(
            s,
            image,
            i,
            &|x, y| styled(rgb, energy, lz, x, y),
            "the styled ramp",
        );
    }
}

#[test]
fn drift_suspect_styling_predicates_drive_their_styles() {
    let h = gpu();
    for view in VIEWS {
        let s = styling_scene(debug(view.id, &[]));
        for (i, lit) in (0u32..).zip(LIT) {
            assert_eq!(
                drift_suspects(&s.read(i)),
                lit,
                "sample {i}'s predicates, `{}`",
                view.id
            );
        }
        check_styles(view, &s, &render(&h, &s), LIT);
    }
}

negative_control!(
    drift_suspect_styling_predicates_drive_their_styles,
    "a view that draws each predicate's style for the other",
    expected = "not the styled ramp",
    {
        let view = VIEWS[0];
        let s = styling_scene(broken(
            view.id,
            "dbg_suspect(rgb, x, energy, lz,",
            "dbg_suspect(rgb, x, lz, energy,",
        ));
        check_styles(view, &s, &render(&gpu(), &s), LIT);
    }
);

/// The absence NaN's bits as a value the shader can't fold, `COMPACTION` being 0.
fn absent_wgsl() -> String {
    format!(
        "bitcast<f32>({}u + uniforms.COMPACTION)",
        present::ABSENT_NAN_BITS
    )
}

/// Checks that the row's samples draw the hatch, unstyled.
fn check_hatched(s: &Scene, image: &[Rgb]) {
    for i in 0..3 {
        check_tile(
            s,
            image,
            i,
            &|x, y| present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]),
            "the hatch",
        );
    }
}

/// The styling scene's view with its drift replaced by the absence NaN.
fn absent_scene(id: &str, field: &str) -> Scene {
    styling_scene(broken(
        id,
        &format!("let x = ctx.sample.{field};"),
        &format!("let x = {};", absent_wgsl()),
    ))
}

#[test]
fn drift_suspect_styling_never_styles_the_hatch() {
    let h = gpu();
    for (id, field) in [
        ("debug/generated/energy_drift", "energy_drift"),
        ("debug/generated/Lz_drift", "Lz_drift"),
    ] {
        let s = absent_scene(id, field);
        check_hatched(&s, &render(&h, &s));
    }
}

negative_control!(
    drift_suspect_styling_never_styles_the_hatch,
    "a view that draws the absence NaN's value on the ramp, styled",
    expected = "not the hatch",
    {
        let s = styling_scene(debug("debug/generated/energy_drift", &[]));
        check_hatched(&s, &render(&gpu(), &s));
    }
);

/// A compaction and a palette the test sets.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Default,
    Swap,
    Lin,
    Log,
}

impl Mode {
    fn params(self) -> &'static [(&'static str, f64)] {
        match self {
            Mode::Default => &[],
            Mode::Swap => &[("PALETTE", 1.0)],
            Mode::Lin => &[("COMPACTION", 1.0)],
            Mode::Log => &[("COMPACTION", 2.0)],
        }
    }
}

/// The non-suspect drifts of the compaction scene, in units of the view's `R`: beyond each end, and within it.
const DRIFTS_OF_R: [f64; 6] = [-2.0, -0.5, -0.125, 0.125, 0.5, 2.0];

/// The compaction scene: a tile per drift of [`DRIFTS_OF_R`], neither predicate lit.
fn compaction_scene(view: View, colouring: Colouring) -> Scene {
    let mut s = row(DRIFTS_OF_R.len() as u32, colouring);
    for (i, k) in (0u32..).zip(DRIFTS_OF_R) {
        let x = (k * view.lin_r) as f32;
        let (h, lz) = invariants(&mut s, i);
        if view.energy {
            fill(&mut s, i, h - x, lz, 0.0, 0.0);
        } else {
            fill(&mut s, i, h, lz - x, 0.0, 0.0);
        }
        assert_eq!(drift_suspects(&s.read(i)), (false, false));
    }
    s
}

/// The pixel of a drift `x`: each compaction's placement on its ramp (render contract Part 5; R-381).
fn placed(view: View, mode: Mode, x: f32, frag: [f64; 2]) -> Rgb {
    match mode {
        Mode::Default => symlog_diverging(x, view.eps),
        Mode::Swap => present::dbg_sentinel(symlog(x, view.eps) as f32, frag),
        Mode::Lin => {
            let t = (f64::from(x) + view.lin_r) / (2.0 * view.lin_r);
            present::drift_ramp(t.clamp(0.0, 1.0), false)
        }
        Mode::Log => {
            let s = (1.0 + f64::from(x).abs() / view.eps).ln();
            present::ramp_viridis(1.0 - 1.0 / (1.0 + s))
        }
    }
}

/// Checks each tile against `mode`'s placement.
fn check_placement(view: View, mode: Mode, s: &Scene, image: &[Rgb]) {
    for i in 0..DRIFTS_OF_R.len() as u32 {
        let x = shown(view, s, i);
        check_tile(
            s,
            image,
            i,
            &|px, py| placed(view, mode, x, [f64::from(px) + 0.5, f64::from(py) + 0.5]),
            "its compaction's placement",
        );
    }
}

/// The colour of the tile of sample `i`.
fn tile(s: &Scene, image: &[Rgb], i: u32) -> Rgb {
    let (x, y) = s.pixels(i)[0];
    image[(y * s.size().0 + x) as usize]
}

fn distance(a: Rgb, b: Rgb) -> f64 {
    a.iter().zip(&b).map(|(x, y)| (x - y).powi(2)).sum()
}

/// Checks `lin`'s ends: a drift beyond `±R` saturates to `c−` or `c+`, and a negative one within it is on `c−`'s side.
fn check_lin_ends(s: &Scene, image: &[Rgb]) {
    let [lo, mid, hi] = present::DRIFT_DIVERGING_SRGB8.map(present::srgb8);
    assert!(near(tile(s, image, 0), lo), "a drift below −R is not c−");
    assert!(near(tile(s, image, 5), hi), "a drift above R is not c+");
    for i in [1, 2] {
        let c = tile(s, image, i);
        assert!(
            !near(c, mid) && distance(c, lo) < distance(c, hi),
            "a negative drift, sample {i}, is not on c−'s side of c0"
        );
    }
}

/// Checks `log`: equal magnitudes draw one colour, and not on `diverging`.
fn check_log_unsigned(view: View, s: &Scene, image: &[Rgb]) {
    for (a, b) in [(0, 5), (1, 4), (2, 3)] {
        assert!(
            near(tile(s, image, a), tile(s, image, b)),
            "a loss and a gain of one size differ, samples {a} and {b}"
        );
    }
    let x = f64::from(shown(view, s, 4));
    let place = 1.0 - 1.0 / (1.0 + (1.0 + x.abs() / view.eps).ln());
    assert!(
        !near(tile(s, image, 4), present::drift_ramp(place, false)),
        "`log` draws on the diverging ramp"
    );
}

#[test]
fn drift_view_compaction_each_at_its_place_on_its_ramp() {
    let h = gpu();
    for view in VIEWS {
        let mut images = Vec::new();
        for mode in [Mode::Default, Mode::Swap, Mode::Lin, Mode::Log] {
            let s = compaction_scene(view, debug(view.id, mode.params()));
            let image = render(&h, &s);
            check_placement(view, mode, &s, &image);
            match mode {
                Mode::Lin => check_lin_ends(&s, &image),
                Mode::Log => check_log_unsigned(view, &s, &image),
                Mode::Default | Mode::Swap => {}
            }
            images.push((mode, image));
        }
        for (mode, image) in &images[1..] {
            assert_ne!(
                image, &images[0].1,
                "`{}`: the override {mode:?} is ignored",
                view.id
            );
        }
    }
}

negative_control!(
    drift_view_compaction_each_at_its_place_on_its_ramp,
    "a view that ignores the `lin` override",
    expected = "not its compaction's placement",
    {
        let view = VIEWS[0];
        let mut s = compaction_scene(view, broken(view.id, "uniforms.COMPACTION", "0u"));
        s.colouring = match s.colouring {
            Colouring::Probe(wgsl, _) => {
                Colouring::Probe(wgsl, vec![("COMPACTION".to_owned(), vec![1.0])])
            }
            other => other,
        };
        check_placement(view, Mode::Lin, &s, &render(&gpu(), &s));
    }
);

negative_control!(
    drift_view_compaction_default_is_symlog,
    "a default that isn't `symlog` on `diverging`",
    expected = "not its compaction's placement",
    {
        let view = VIEWS[1];
        let s = compaction_scene(view, debug(view.id, Mode::Lin.params()));
        check_placement(view, Mode::Default, &s, &render(&gpu(), &s));
    }
);

negative_control!(
    drift_view_compaction_lin_ends,
    "a drift beyond ±R not saturated, on a `symlog` render",
    expected = "a drift below −R is not c−",
    {
        let view = VIEWS[0];
        let s = compaction_scene(view, debug(view.id, &[]));
        check_lin_ends(&s, &render(&gpu(), &s));
    }
);

negative_control!(
    drift_view_compaction_log_unsigned,
    "a signed render read as `log`",
    expected = "a loss and a gain of one size differ",
    {
        let view = VIEWS[0];
        let s = compaction_scene(view, debug(view.id, &[]));
        check_log_unsigned(view, &s, &render(&gpu(), &s));
    }
);

/// Checks the CPU mirror, `present::dbg_drift`, against each compaction's placement written here.
fn check_mirror(mirror: fn(f32, u32, bool, f64, f64, [f64; 2]) -> Rgb) {
    let frag = [0.5, 0.5];
    for view in VIEWS {
        for k in DRIFTS_OF_R {
            let x = (k * view.lin_r) as f32;
            for (mode, compaction, swap) in [
                (Mode::Default, present::DRIFT_SYMLOG, false),
                (Mode::Swap, present::DRIFT_SYMLOG, true),
                (Mode::Lin, present::DRIFT_LIN, false),
                (Mode::Log, present::DRIFT_LOG, false),
                (Mode::Log, present::DRIFT_LOG, true),
            ] {
                let got = mirror(x, compaction, swap, view.lin_r, view.eps, frag);
                let want = placed(view, mode, x, frag);
                assert!(
                    near(got, want),
                    "the mirror at {mode:?}, x = {x}: {got:?}, not {want:?}"
                );
            }
        }
    }
}

#[test]
fn drift_view_compaction_mirror_is_the_placement() {
    check_mirror(present::dbg_drift);
}

negative_control!(
    drift_view_compaction_mirror_is_the_placement,
    "a mirror that ignores the compaction",
    expected = "the mirror at Lin",
    check_mirror(|x, _, swap, r, eps, frag| present::dbg_drift(
        x,
        present::DRIFT_SYMLOG,
        swap,
        r,
        eps,
        frag
    ))
);

/// Checks the ink at either side of the luminance split, on each channel alone, so that each weight is seen.
fn check_ink(ink: fn(Rgb) -> Rgb) {
    for (channel, weight) in [(0, 0.2126), (1, 0.7152), (2, 0.0722)] {
        for (factor, want) in [(1.01, 0.0), (0.99, 1.0)] {
            let mut c = [0.0; 3];
            c[channel] = factor * 0.18 / weight;
            assert_eq!(ink(c), [want; 3], "the ink over {c:?}");
        }
    }
    assert_eq!(ink([0.18; 3]), [0.0; 3], "the ink at the split");
}

#[test]
fn drift_suspect_styling_ink_splits_at_the_luminance() {
    check_ink(present::suspect_ink);
}

negative_control!(
    drift_suspect_styling_ink_splits_at_the_luminance,
    "ink always black",
    expected = "the ink over",
    check_ink(|_| [0.0; 3])
);

// ── `symlog`'s neutral band (render contract Part 5; R-652, R-928, R-929) ──────────────────────────────────────────

/// A view at its defaults, each by its own read of `s`.
fn view_of(view: View) -> Colouring {
    debug(view.id, &[])
}

/// `c`'s 8-bit values, as the golden harness quantises a render.
fn bytes(c: Rgb) -> [u8; 3] {
    c.map(|v| (v * 255.0).round() as u8)
}

/// Sample `i`'s pixels are `neutral` exactly, by their 8-bit values and within [`TOL`].
fn check_neutral(s: &Scene, image: &[Rgb], i: u32, neutral: Rgb, what: &str) {
    let width = s.size().0;
    for (x, y) in s.pixels(i) {
        let got = image[(y * width + x) as usize];
        assert!(
            bytes(got) == bytes(neutral) && near(got, neutral),
            "{what}: pixel ({x}, {y}) of sample {i} of `{}` is {got:?}, not neutral {neutral:?}",
            s.name
        );
    }
}

/// The drift views' neutral, `c0`.
fn c0() -> Rgb {
    present::drift_ramp(0.5, false)
}

/// The samples of [`fresh_scene`]: the showcase's eight states, then the two whose reference is near zero.
const FRESH: u32 = 10;

/// The fresh set's states beyond the showcase's, as `(r, p, m)`: dd_integrator test 2's figure-eight at its start, whose
/// `L_z` is 0 but for rounding, its `S` not; and a near-parabolic state, the dyadic tile's momenta scaled so that
/// `K = |V|`, whose `E` is 0 but for rounding, its `K + |V|` not. The band relative to the reference is nearly empty on
/// each, and the band of the rounding is not.
fn near_zero_states() -> [([[f32; 2]; 3], [[f32; 2]; 3], [f32; 3]); 2] {
    let eight = drift::figure_eight().ic;
    let f = |a: [[f64; 2]; 3]| a.map(|v| v.map(|c| c as f32));
    let m = [0.25_f32, 0.375, 0.375];
    let (k, v) = kinetic_potential(R, P, m);
    let scale = (v / k).sqrt();
    let p = P.map(|q| q.map(|c| (f64::from(c) * scale) as f32));
    [
        (f(eight.r), f(eight.p), eight.m.map(|c| c as f32)),
        (R, p, m),
    ]
}

/// `(K, |V|)` of the f32 state in f64.
fn kinetic_potential(r: [[f32; 2]; 3], p: [[f32; 2]; 3], m: [f32; 3]) -> (f64, f64) {
    let f = |x: f32| f64::from(x);
    let k = (0..3)
        .map(|j| (f(p[j][0]).powi(2) + f(p[j][1]).powi(2)) / (2.0 * f(m[j])))
        .sum();
    let v = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(a, b)| f(m[a]) * f(m[b]) / (f(r[a][0]) - f(r[b][0])).hypot(f(r[a][1]) - f(r[b][1])))
        .sum();
    (k, v)
}

/// `(H, L_z, K + |V|, S)` of the f32 state in f64: the invariants a correctly rounded CPU reference rounds once, and the
/// band's scales.
fn exact(r: [[f32; 2]; 3], p: [[f32; 2]; 3], m: [f32; 3]) -> [f64; 4] {
    let f = |x: f32| f64::from(x);
    let (k, v) = kinetic_potential(r, p, m);
    let lz = (0..3)
        .map(|j| f(r[j][0]) * f(p[j][1]) - f(r[j][1]) * f(p[j][0]))
        .sum();
    let s = (0..3)
        .map(|j| (f(r[j][0]) * f(p[j][1])).abs() + (f(r[j][1]) * f(p[j][0])).abs())
        .sum();
    [k - v, lz, k + v, s]
}

/// A scene of [`FRESH`] fresh samples coloured by `colouring`: the showcase's eight states, then
/// [`near_zero_states`]', each with `E_0` and `Lz_0` its own `H(r, p)` and `L_z(r, p)` in f64 rounded once to f32, a
/// correctly rounded CPU reference (render contract Part 5), its latches 0, so that no styling draws, and bounded, so
/// that a field ramp places it. `offset(i, energy)` moves sample `i`'s reference by that much, for the controls.
fn fresh_scene(colouring: Colouring, offset: &dyn Fn(&Scene, u32, bool) -> f64) -> Scene {
    let mut s = row(FRESH, colouring);
    validation::golden_scene::showcase(&mut s.set, 6);
    for (i, (r, p, m)) in (8..).zip(near_zero_states()) {
        s.set.sample(i).r(r).p(p);
        let ic = s.set.ic(i);
        (ic.m0, ic.m1, ic.m2) = (m[0], m[1], m[2]);
    }
    for i in 0..FRESH {
        let read = s.read(i);
        let [h, lz, ..] = exact(read.r, read.p, s.masses(i));
        let (de, dl) = (offset(&s, i, true), offset(&s, i, false));
        s.set
            .sample(i)
            .E_0((h - de) as f32)
            .Lz_0((lz - dl) as f32)
            .drift_max(0.0, 0.0)
            .state(ledger::display::State::Bounded.code());
    }
    s
}

/// No offset: every sample fresh.
fn fresh(_: &Scene, _: u32, _: bool) -> f64 {
    0.0
}

/// The fresh scene's references near zero: on the near-parabolic state `|E_0|`, on the figure-eight `|Lz_0|`, each
/// under 2⁻²⁰ of its band's scale, so that a band relative to the reference is under 2⁻¹³ of the rounding there.
fn check_near_zero(s: &Scene) {
    let (eight, parabolic) = (s.read(8), s.read(9));
    let [.., s_eight] = exact(eight.r, eight.p, s.masses(8));
    let [_, _, kv, _] = exact(parabolic.r, parabolic.p, s.masses(9));
    assert!(
        f64::from(eight.Lz_0.abs()) < s_eight / 1_048_576.0,
        "the figure-eight's Lz_0 {} is not near zero against S {s_eight}",
        eight.Lz_0
    );
    assert!(
        f64::from(parabolic.E_0.abs()) < kv / 1_048_576.0,
        "the near-parabolic state's E_0 {} is not near zero against K + |V| {kv}",
        parabolic.E_0
    );
}

/// Every sample of `s` renders exactly `neutral` in the render `image`.
fn check_fresh(s: &Scene, image: &[Rgb], neutral: Rgb, what: &str) {
    for i in 0..FRESH {
        check_neutral(s, image, i, neutral, what);
    }
}

/// A field ramp of the drift `field`, its default: `diverging`, the symlog, at the floor `x0`, on cool-warm (`defaults`;
/// colour_composition §6's `f_edrift`).
fn drift_ramp_of(field: &str, x0: f64) -> FieldRamp {
    FieldRamp::new(
        ScalarField::payload(field).unwrap_or_else(|e| panic!("{e}")),
        Compaction::Diverging { x0 },
        Map::ramp(Ramp::Lut(Lut::Coolwarm)),
    )
}

/// The field ramps on `energy_drift` and `Lz_drift`, each with its floor.
fn drift_ramps() -> [FieldRamp; 2] {
    [
        drift_ramp_of("energy_drift", EPS_E_DEFAULT.number()),
        drift_ramp_of("Lz_drift", EPS_L_DEFAULT.number()),
    ]
}

/// The cool-warm ramp's neutral, its middle, where the symlog places 0.
fn ramp_neutral() -> Rgb {
    Ramp::Lut(Lut::Coolwarm).colour(0.5)
}

/// `ramp` with its band cut: the drift compacted as it is read, for the control.
fn unbanded(ramp: &FieldRamp) -> Colouring {
    let band = ramp.field.band.unwrap_or_else(|| panic!("no band"));
    let source = ramp.wgsl();
    let cut = format!("dbg_drift_banded(v, {band})");
    assert!(source.contains(&cut), "the ramp has no `{cut}`");
    Colouring::Probe(source.replace(&cut, "v"), Vec::new())
}

#[test]
fn drift_view_neutral_fresh_sample_renders_exactly_neutral() {
    let h = gpu();
    for view in VIEWS {
        let s = fresh_scene(view_of(view), &fresh);
        check_near_zero(&s);
        check_fresh(&s, &render(&h, &s), c0(), view.id);
    }
    for ramp in drift_ramps() {
        let s = fresh_scene(
            Colouring::Ramp {
                ramp: ramp.clone(),
                override_on: false,
            },
            &fresh,
        );
        check_fresh(&s, &render(&h, &s), ramp_neutral(), ramp.field.field);
    }
    // The CPU twins: the golden scenes' look of the `energy_drift` view, and each field ramp's.
    let s = fresh_scene(Colouring::View("energy_drift"), &fresh);
    for i in 0..FRESH {
        let look = s.look(i).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            bytes(look.colour([0, 0])),
            bytes(c0()),
            "the look of fresh sample {i}"
        );
    }
    for ramp in drift_ramps() {
        let s = fresh_scene(view_of(VIEWS[0]), &fresh);
        for i in 0..FRESH {
            let read = s.read(i);
            let (x, band) = if ramp.field.field == "Lz_drift" {
                (read.Lz_drift, present::drift_band_lz(read.r, read.p))
            } else {
                let band = present::drift_band_energy(read.r, read.p, s.masses(i));
                (read.energy_drift, band)
            };
            assert_eq!(
                ramp.shown_banded(x, true, Reads::HEADER, band),
                RampShown::Ramp(0.5),
                "`{}`'s twin at fresh sample {i}",
                ramp.field.field
            );
        }
    }
}

negative_control!(
    drift_view_neutral_fresh_sample_renders_exactly_neutral,
    "each reference off by twice its band: a real drift, not a fresh sample",
    expected = "not neutral",
    {
        let h = gpu();
        let twice = |s: &Scene, i: u32, energy: bool| {
            let read = s.read(i);
            2.0 * f64::from(if energy {
                present::drift_band_energy(read.r, read.p, s.masses(i))
            } else {
                present::drift_band_lz(read.r, read.p)
            })
        };
        let s = fresh_scene(view_of(VIEWS[1]), &twice);
        check_fresh(&s, &render(&h, &s), c0(), VIEWS[1].id);
    }
);

/// The band of R-652 before R-928, `k·2⁻²³·|reference|` with `k = 128`, in place of the view's.
fn relative_band(view: View) -> Colouring {
    let field = if view.energy {
        "energy_drift"
    } else {
        "Lz_drift"
    };
    let band = catalogue::drift_band_wgsl(field).unwrap_or_else(|| panic!("no band"));
    let reference = catalogue::drift_reference(field);
    broken(
        view.id,
        &format!("let band = {band};"),
        &format!("let band = 1.52587890625e-5 * abs(ctx.sample.{reference});"),
    )
}

negative_control!(
    drift_view_neutral_fresh_sample_renders_exactly_neutral_relative_band,
    "the band relative to |Lz_0| (k = 128): the figure-eight's rounding shows",
    expected = "not neutral",
    {
        let s = fresh_scene(relative_band(VIEWS[1]), &fresh);
        check_neutral(&s, &render(&gpu(), &s), 8, c0(), VIEWS[1].id);
    }
);

negative_control!(
    drift_view_neutral_fresh_sample_renders_exactly_neutral_relative_band_energy,
    "the band relative to |E_0| (k = 128): the near-parabolic state's rounding shows",
    expected = "not neutral",
    {
        let s = fresh_scene(relative_band(VIEWS[2]), &fresh);
        check_neutral(&s, &render(&gpu(), &s), 9, c0(), VIEWS[2].id);
    }
);

negative_control!(
    drift_view_neutral_fresh_sample_renders_exactly_neutral_unbanded_ramp,
    "a field ramp on Lz_drift without the band: a fresh sample's rounding shows",
    expected = "not neutral",
    {
        let [_, lz] = drift_ramps();
        let s = fresh_scene(unbanded(&lz), &fresh);
        check_fresh(&s, &render(&gpu(), &s), ramp_neutral(), "Lz_drift");
    }
);

negative_control!(
    drift_view_neutral_fresh_sample_renders_exactly_neutral_unbanded_ramp_energy,
    "a field ramp on energy_drift without the band: a fresh sample's rounding shows",
    expected = "not neutral",
    {
        let [energy, _] = drift_ramps();
        let s = fresh_scene(unbanded(&energy), &fresh);
        check_fresh(&s, &render(&gpu(), &s), ramp_neutral(), "energy_drift");
    }
);

/// The band's half-width at sample `i` of `s` in `view`, the mirror's.
fn band_at(view: View, s: &Scene, i: u32) -> f64 {
    let read = s.read(i);
    f64::from(if view.energy {
        present::drift_band_energy(read.r, read.p, s.masses(i))
    } else {
        present::drift_band_lz(read.r, read.p)
    })
}

/// A three-tile scene of `view` on the dyadic state: tile 0's drift is `inside` times the band, tile 1's `past` times it
/// and tile 2's `−past` times it.
fn band_scene(view: View, colouring: Colouring, inside: f64, past: f64) -> Scene {
    let mut s = row(3, colouring);
    let (h, l) = invariants(&mut s, 0);
    let band = band_at(view, &s, 0);
    let offsets = [inside * band, past * band, -past * band];
    for (i, off) in offsets.iter().enumerate() {
        let (mut e, mut z) = (h, l);
        if view.energy {
            e = (f64::from(h) - off) as f32;
        } else {
            z = (f64::from(l) - off) as f32;
        }
        fill(&mut s, i as u32, e, z, 0.0, 0.0);
    }
    s
}

/// Tile 0 of [`band_scene`] is `neutral`, and tiles 1 and 2 show their drifts: off it on the warm side and the cool side.
fn check_band(view: View, s: &Scene, image: &[Rgb], neutral: Rgb) {
    check_neutral(s, image, 0, neutral, "a drift inside the band");
    let (above, below) = (tile(s, image, 1), tile(s, image, 2));
    assert!(
        distance(above, neutral) > 0.02 && distance(below, neutral) > 0.02,
        "`{}`: the band swallowed a drift past it: {above:?}, {below:?}, neutral {neutral:?}",
        view.id
    );
    assert!(
        above[0] > above[2] && below[2] > below[0],
        "`{}`: a drift past the band is on the wrong side of neutral: {above:?}, {below:?}",
        view.id
    );
}

#[test]
fn drift_view_neutral_band_keeps_a_drift_past_it() {
    let h = gpu();
    for view in VIEWS {
        let s = band_scene(view, view_of(view), 0.5, 2.0);
        check_band(view, &s, &render(&h, &s), c0());
    }
    for (view, ramp) in [VIEWS[0], VIEWS[1]].into_iter().zip(drift_ramps()) {
        let colouring = Colouring::Ramp {
            ramp,
            override_on: false,
        };
        let s = band_scene(view, colouring, 0.5, 2.0);
        check_band(view, &s, &render(&h, &s), ramp_neutral());
    }
}

negative_control!(
    drift_view_neutral_band_keeps_a_drift_past_it,
    "a band 8 times wider swallows the drift past it",
    expected = "the band swallowed a drift past it",
    {
        let view = VIEWS[1];
        let colouring = broken(view.id, "ctx.sample.p);", "ctx.sample.p) * 8.0;");
        let s = band_scene(view, colouring, 0.5, 2.0);
        check_band(view, &s, &render(&gpu(), &s), c0());
    }
);

negative_control!(
    drift_view_neutral_band_is_drawn,
    "the band removed: a drift inside it shows",
    expected = "not neutral",
    {
        let view = VIEWS[1];
        let colouring = broken(view.id, "dbg_drift(shown,", "dbg_drift(x,");
        let s = band_scene(view, colouring, 0.5, 2.0);
        check_band(view, &s, &render(&gpu(), &s), c0());
    }
);

/// Under `log`, which has no band, a drift inside `symlog`'s band keeps its place, off the place of 0.
fn check_log_unbanded(view: View, s: &Scene, image: &[Rgb]) {
    let zero = present::dbg_drift(
        0.0,
        present::DRIFT_LOG,
        false,
        view.lin_r,
        view.eps,
        [0.5, 0.5],
    );
    assert!(
        distance(tile(s, image, 0), zero) > 0.02,
        "`{}`: `log` drew a drift inside `symlog`'s band as 0",
        view.id
    );
}

#[test]
fn drift_view_neutral_band_is_drawn() {
    let h = gpu();
    for view in VIEWS {
        let s = band_scene(view, debug(view.id, &[("COMPACTION", 2.0)]), 0.5, 2.0);
        check_log_unbanded(view, &s, &render(&h, &s));
    }
    // Under the palette swap `symlog` keeps the band: 0 is viridis at its middle.
    let view = VIEWS[1];
    let s = band_scene(view, debug(view.id, &[("PALETTE", 1.0)]), 0.5, 2.0);
    let middle = present::drift_ramp(0.5, true);
    assert!(
        near(tile(&s, &render(&h, &s), 0), middle),
        "the palette swap dropped the band"
    );
}

/// The CPU mirror, at the comparison's boundary and through each passing arm.
fn check_shown(shown: fn(f32, u32, f32) -> f32) {
    let band = 4.0 * (present::DRIFT_BAND_E as f32);
    let above = f32::from_bits(band.to_bits() + 1);
    let symlog = present::DRIFT_SYMLOG;
    assert_eq!(shown(band, symlog, band), 0.0, "at the band");
    assert_eq!(shown(-band, symlog, band), 0.0, "at the band, negative");
    assert_eq!(shown(above, symlog, band), above, "just past the band");
    assert_eq!(
        shown(-above, symlog, band),
        -above,
        "just past it, negative"
    );
    assert_eq!(
        shown(band, present::DRIFT_LIN, band),
        band,
        "`lin` has no band"
    );
    assert_eq!(
        shown(band, present::DRIFT_LOG, band),
        band,
        "`log` has no band"
    );
    // Any other compaction is `symlog`, as `dbg_drift_place` reads it.
    assert_eq!(shown(band, 7, band), 0.0, "another compaction");
    // An empty band: exactly 0 stays 0, and the least drift shows.
    assert_eq!(shown(0.0, symlog, 0.0), 0.0, "0 in an empty band");
    assert_eq!(shown(1e-30, symlog, 0.0), 1e-30, "a drift in an empty band");
    let nan = f32::from_bits(present::ABSENT_NAN_BITS);
    assert_eq!(
        shown(nan, symlog, band).to_bits(),
        present::ABSENT_NAN_BITS,
        "the absence NaN"
    );
}

#[test]
fn drift_view_neutral_mirror_at_the_boundary() {
    check_shown(present::dbg_drift_shown);
}

negative_control!(
    drift_view_neutral_mirror_at_the_boundary,
    "a band read with `<`, which drops its boundary",
    expected = "at the band",
    check_shown(|x, _, band| if x.abs() < band { 0.0 } else { x })
);

/// The band never makes a non-finite operand neutral: `|x| ≤ ∞` holds for every `x` (R-436). A NaN of either sign
/// counts, and the largest finite band still holds.
fn check_shown_non_finite(shown: fn(f32, u32, f32) -> f32) {
    let symlog = present::DRIFT_SYMLOG;
    for bad in [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xFFC0_0000),
    ] {
        assert_eq!(
            shown(1.0, symlog, bad),
            1.0,
            "a finite drift in the band {bad:?}"
        );
        assert_eq!(
            shown(bad, symlog, 1.0).to_bits(),
            bad.to_bits(),
            "the drift {bad:?}"
        );
        assert_eq!(
            shown(bad, symlog, bad).to_bits(),
            bad.to_bits(),
            "the drift and the band {bad:?}"
        );
    }
    assert_eq!(shown(1.0, symlog, f32::MAX), 0.0, "the largest band");
}

#[test]
fn drift_view_neutral_band_passes_a_non_finite_operand() {
    check_shown_non_finite(present::dbg_drift_shown);
}

negative_control!(
    drift_view_neutral_band_passes_a_non_finite_operand,
    "a band testing only the absence NaN, as before R-436's guard, so an infinite band makes a drift neutral",
    expected = "a finite drift in the band inf",
    check_shown_non_finite(|x, _, band| {
        if x.to_bits() == present::ABSENT_NAN_BITS {
            x
        } else if x.abs() <= band {
            0.0
        } else {
            x
        }
    })
);

/// The band's widths, `k'·2⁻²³` of the f64 `K + |V|` and `S` of the state: on the dyadic state, where `S` is exact in
/// f32, and on the figure-eight, whose `L_z` is near 0 and its `S` is not.
fn check_scales(
    energy: fn([[f32; 2]; 3], [[f32; 2]; 3], [f32; 3]) -> f32,
    lz: fn([[f32; 2]; 3], [[f32; 2]; 3]) -> f32,
) {
    let [(r8, p8, m8), _] = near_zero_states();
    for (r, p, m) in [(R, P, [0.25_f32, 0.375, 0.375]), (r8, p8, m8)] {
        let [.., kv, s] = exact(r, p, m);
        let e = f64::from(energy(r, p, m));
        assert!(
            (e - present::DRIFT_BAND_E * kv).abs() <= 1e-6 * e,
            "the energy band {e:e}, not 15·2⁻²³·(K + |V|) = {:e}",
            present::DRIFT_BAND_E * kv
        );
        let l = f64::from(lz(r, p));
        assert!(
            (l - present::DRIFT_BAND_LZ * s).abs() <= 1e-6 * l,
            "the L_z band {l:e}, not 5·2⁻²³·S = {:e}",
            present::DRIFT_BAND_LZ * s
        );
    }
    assert_eq!(
        present::DRIFT_BAND_E,
        15.0 / 8_388_608.0,
        "k' for the energy"
    );
    assert_eq!(present::DRIFT_BAND_LZ, 5.0 / 8_388_608.0, "k' for L_z");
}

#[test]
fn drift_view_neutral_band_scales_by_the_rounding() {
    check_scales(present::drift_band_energy, present::drift_band_lz);
}

negative_control!(
    drift_view_neutral_band_scales_by_the_rounding,
    "the energy band of K − |V|, the signed H, rather than K + |V|",
    expected = "the energy band",
    check_scales(
        |r, p, m| {
            let (k, v) = kinetic_potential(r, p, m);
            ((k - v).abs() * present::DRIFT_BAND_E) as f32
        },
        present::drift_band_lz
    )
);

negative_control!(
    drift_view_neutral_band_scales_by_the_rounding_lz,
    "the L_z band of |L_z|, the signed sum, rather than S",
    expected = "the L_z band",
    check_scales(present::drift_band_energy, |r, p| {
        let [_, lz, ..] = exact(r, p, [1.0; 3]);
        (lz.abs() * present::DRIFT_BAND_LZ) as f32
    })
);

/// The mirror agrees with the assembled source on the band's literals.
#[test]
fn drift_view_neutral_band_literal_is_the_mirrors() {
    let source = include_str!("../shaders/wgsl/lib/drift.wgsl");
    for (name, value) in [
        ("DBG_DRIFT_BAND_E", present::DRIFT_BAND_E),
        ("DBG_DRIFT_BAND_LZ", present::DRIFT_BAND_LZ),
    ] {
        let literal = format!("const {name}: f32 = {value:?};");
        assert!(source.contains(&literal), "no `{literal}` in drift.wgsl");
    }
}

negative_control!(
    drift_view_neutral_band_literal_is_the_mirrors,
    "another literal",
    expected = "in drift.wgsl",
    {
        let source = include_str!("../shaders/wgsl/lib/drift.wgsl");
        assert!(
            source.contains("DBG_DRIFT_BAND_E: f32 = 1.0;"),
            "not in drift.wgsl"
        );
    }
);

/// Every place a current drift is read under `symlog` takes the one band (R-929): each drift view's source holds
/// `let band = <the field's band>;`, and a field ramp on `energy_drift` or `Lz_drift` compacts
/// `dbg_drift_banded(v, <its band>)` under `diverging` and `v` under any other form; a ramp of another field has none.
fn check_one_band(views: &[(&str, &str)], ramp: &dyn Fn(&str, Compaction) -> String) {
    for (id, field) in views {
        let source = registry::registry()
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .find(|e| e.id == *id)
            .unwrap_or_else(|| panic!("no registry entry `{id}`"))
            .source;
        let band = catalogue::drift_band_wgsl(field).unwrap_or_else(|| panic!("no band"));
        assert!(
            source.contains(&format!("let band = {band};")),
            "`{id}` does not read the band of `{field}`"
        );
    }
    for field in ["energy_drift", "Lz_drift"] {
        let band = catalogue::drift_band_wgsl(field).unwrap_or_else(|| panic!("no band"));
        let banded = format!("compact_symlog(dbg_drift_banded(v, {band}), 1e-6)");
        assert!(
            ramp(field, Compaction::Diverging { x0: 1e-6 }).contains(&banded),
            "a field ramp on `{field}` has no `{banded}`"
        );
        assert!(
            ramp(field, Compaction::Log { eps: 1e-6 }).contains("compact_log(v, 1e-6)"),
            "a `log` field ramp on `{field}` has a band"
        );
    }
    assert!(
        ramp("S", Compaction::Diverging { x0: 1e-6 }).contains("compact_symlog(v, 1e-6)"),
        "a field ramp on `S` has a band"
    );
}

/// The drift views by registry id and field.
const VIEW_FIELDS: [(&str, &str); 3] = [
    ("debug/generated/energy_drift", "energy_drift"),
    ("debug/generated/Lz_drift", "Lz_drift"),
    ("debug/derived/energy_drift", "energy_drift"),
];

/// A colour field ramp of `field` under `compaction`, its WGSL.
fn ramp_wgsl(field: &str, compaction: Compaction) -> String {
    FieldRamp::new(
        ScalarField::payload(field).unwrap_or_else(|e| panic!("{e}")),
        compaction,
        Map::ramp(Ramp::Lut(Lut::Coolwarm)),
    )
    .wgsl()
}

#[test]
fn drift_view_neutral_band_is_one_definition() {
    check_one_band(&VIEW_FIELDS, &ramp_wgsl);
}

negative_control!(
    drift_view_neutral_band_is_one_definition,
    "a field ramp that compacts the drift as it is read",
    expected = "has no `compact_symlog(dbg_drift_banded",
    check_one_band(&VIEW_FIELDS, &|field, compaction| {
        ramp_wgsl(field, compaction).replace("dbg_drift_banded(v, ", "(v, ")
    })
);

/// The field ramp's CPU twin bands a drift under `diverging` only, and only a drift field: at a band of 1, 0.5 places
/// at 0 on `energy_drift` and `Lz_drift`, and keeps its place under `log`, and on `S`.
fn check_twin_band(shown: &dyn Fn(&FieldRamp, f32, f32) -> RampShown) {
    for field in ["energy_drift", "Lz_drift"] {
        let ramp = drift_ramp_of(field, 1e-6);
        assert_eq!(
            shown(&ramp, 0.5, 1.0),
            RampShown::Ramp(0.5),
            "`{field}`'s twin at a drift inside the band"
        );
        let past = shown(&ramp, 2.0, 1.0);
        assert_eq!(
            past,
            RampShown::Ramp(Compaction::Diverging { x0: 1e-6 }.place(2.0)),
            "`{field}`'s twin at a drift past the band"
        );
        let log = FieldRamp::new(
            ScalarField::payload(field).unwrap_or_else(|e| panic!("{e}")),
            Compaction::Log { eps: 1e-6 },
            Map::ramp(Ramp::Lut(Lut::Viridis)),
        );
        assert_eq!(
            shown(&log, 0.5, 1.0),
            RampShown::Ramp(Compaction::Log { eps: 1e-6 }.place(0.5)),
            "`{field}`'s `log` twin"
        );
    }
    let s = drift_ramp_of("S", 1e-6);
    assert_eq!(
        shown(&s, 0.5, 1.0),
        RampShown::Ramp(Compaction::Diverging { x0: 1e-6 }.place(0.5)),
        "`S`'s twin"
    );
    // A band of 0, `shown_at`'s, places an exact 0 at the middle and the least drift off it.
    let lz = drift_ramp_of("Lz_drift", 1e-6);
    assert_eq!(
        shown(&lz, 0.0, 0.0),
        RampShown::Ramp(0.5),
        "0 in an empty band"
    );
}

#[test]
fn drift_view_neutral_field_ramp_twin_bands() {
    check_twin_band(&|ramp, x, band| ramp.shown_banded(x, true, Reads::HEADER, band));
}

negative_control!(
    drift_view_neutral_field_ramp_twin_bands,
    "the twin without the band",
    expected = "twin at a drift inside the band",
    check_twin_band(&|ramp, x, _| ramp.shown_at(x, true, Reads::HEADER))
);

// ── The non-finite guard (render contract Part 5; R-436) ────────────────────────────────────────────────────────────

/// The guard on a view's drift.
const X_GUARD: &str = "if (dbg_drift_non_finite(x)) { return debug_invalid(ctx.frag_xy); }";

/// The state of a tile whose drift is `±∞` on finite references: overflowing momenta and coincident bodies for `H`, an
/// overflowing `x₀p_{y,0}` for `L_z`.
fn infinite_state(energy: bool, positive: bool) -> ([[f32; 2]; 3], [[f32; 2]; 3]) {
    let sign = if positive { 1.0 } else { -1.0 };
    match (energy, positive) {
        (true, true) => (R, [[2e20, 0.0], [-2e20, 0.0], [0.0, 0.0]]),
        (true, false) => ([[0.0, 0.0], [0.0, 0.0], [0.0, 0.5]], [[0.0; 2]; 3]),
        (false, _) => (
            [[4e33, 0.0], [0.0, 0.5], [0.0, -0.5]],
            [[1e6, sign * 1e6], [0.0, 0.0], [0.0, 0.0]],
        ),
    }
}

/// Two tiles of `view`, drift `+∞` and `−∞` by the read side (asserted), on the finite references `E_0 = −1` and
/// `Lz_0 = 1`, the latches 0.
fn infinite_scene(view: View, colouring: Colouring) -> Scene {
    let mut s = row(2, colouring);
    for (i, positive) in [(0, true), (1, false)] {
        let (r, p) = infinite_state(view.energy, positive);
        s.set
            .sample(i)
            .r(r)
            .p(p)
            .E_0(-1.0)
            .Lz_0(1.0)
            .times(1000, 500)
            .drift_max(0.0, 0.0);
        let want = if positive {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        };
        assert_eq!(shown(view, &s, i), want, "tile {i}'s drift, `{}`", view.id);
    }
    s
}

/// `view` with its guard cut and its colour the flag "this backend's `x` has an exponent of all ones": white or black.
fn delivered_probe(view: View) -> Colouring {
    let Colouring::Probe(wgsl, _) = broken(view.id, X_GUARD, "") else {
        unreachable!("`broken` is a probe")
    };
    let last = "return dbg_suspect(rgb, x, energy, lz, ctx.frag_xy);";
    assert!(wgsl.contains(last), "`{}` has no `{last}`", view.id);
    let flag = "return vec3<f32>(select(0.0, 1.0, dbg_drift_non_finite(x)));";
    Colouring::Probe(wgsl.replace(last, flag), Vec::new())
}

/// Which of [`infinite_scene`]'s tiles this backend computes `∞`'s bits for. WGSL lets it assume no infinities, so a live
/// `x` is tested as it was computed (render contract Part 5), and a tile it doesn't deliver is recorded and not judged.
fn delivered(h: &GpuHarness, view: View) -> [bool; 2] {
    let s = infinite_scene(view, delivered_probe(view));
    let image = render(h, &s);
    [0, 1].map(|i| {
        let got = tile(&s, &image, i)[0] > 0.5;
        if !got {
            eprintln!(
                "`{}`: tile {i}: this backend does not deliver `∞`'s bits, so the hatch is not judged",
                view.id
            );
        }
        got
    })
}

/// Checks that the delivered tiles of `s` draw the hatch, unstyled.
fn check_infinite_hatched(s: &Scene, image: &[Rgb], delivered: [bool; 2]) {
    for (i, judged) in (0u32..).zip(delivered) {
        if judged {
            check_tile(
                s,
                image,
                i,
                &|x, y| present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]),
                "the hatch",
            );
        }
    }
}

#[test]
fn drift_view_non_finite_infinite_drift_hatches_under_every_compaction() {
    let h = gpu();
    for view in VIEWS {
        let judged = delivered(&h, view);
        for mode in [Mode::Default, Mode::Lin, Mode::Log] {
            let s = infinite_scene(view, debug(view.id, mode.params()));
            check_infinite_hatched(&s, &render(&h, &s), judged);
        }
    }
}

/// The control's body: `view` at `compaction` with its guard on `x` cut, which clamps the `∞` onto the ramp.
#[cfg(feature = "controls")]
fn guard_cut(view: View, compaction: f64) {
    let h = gpu();
    let judged = delivered(&h, view);
    let mut s = infinite_scene(view, broken(view.id, X_GUARD, ""));
    if let Colouring::Probe(_, params) = &mut s.colouring {
        params.push(("COMPACTION".to_owned(), vec![compaction]));
    }
    check_infinite_hatched(&s, &render(&h, &s), judged);
}

negative_control!(
    drift_view_non_finite_infinite_drift_hatches_under_every_compaction,
    "the generated energy view at `symlog` with its guard on `x` cut",
    expected = "not the hatch",
    guard_cut(VIEWS[0], 0.0)
);

negative_control!(
    drift_view_non_finite_lz_guard_is_held,
    "the `L_z` view at `lin` with its guard on `x` cut",
    expected = "not the hatch",
    guard_cut(VIEWS[1], 1.0)
);

negative_control!(
    drift_view_non_finite_derived_guard_is_held,
    "the derived energy view at `log` with its guard on `x` cut",
    expected = "not the hatch",
    guard_cut(VIEWS[2], 2.0)
);

/// The energy predicate's line in the `L_z` view.
#[cfg(feature = "controls")]
const LZ_ENERGY: &str = "!dbg_drift_non_finite(ctx.sample.E_0) && suspect_energy(ctx.sample.dE_max, ctx.sample.E_0, uniforms.EPS_E)";

/// A tile per non-finite `E_0`, the `L_z` drift finite and off the band, a latch that lights the energy predicate on any
/// finite `E_0`.
fn lz_on_bad_e_0(colouring: Colouring) -> Scene {
    let mut s = row(4, colouring);
    let (_, lz) = invariants(&mut s, 0);
    for (i, e_0) in (0u32..).zip([
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xFFC0_0000),
    ]) {
        fill(&mut s, i, e_0, lz - 0.25, 1.0, 0.0);
    }
    s
}

/// Checks that each tile shows its finite drift on the ramp, unstyled.
fn check_lz_on_bad_e_0(s: &Scene, image: &[Rgb]) {
    let view = VIEWS[1];
    for i in 0..4 {
        let rgb = symlog_diverging(shown(view, s, i), view.eps);
        check_tile(s, image, i, &|_, _| rgb, "the unstyled ramp");
    }
}

#[test]
fn drift_view_non_finite_e_0_leaves_the_lz_view_unlit() {
    let s = lz_on_bad_e_0(debug(VIEWS[1].id, &[]));
    check_lz_on_bad_e_0(&s, &render(&gpu(), &s));
}

negative_control!(
    drift_view_non_finite_e_0_leaves_the_lz_view_unlit,
    "the `L_z` view with the energy predicate lit on a non-finite `E_0`",
    expected = "not the unstyled ramp",
    {
        let s = lz_on_bad_e_0(broken(VIEWS[1].id, LZ_ENERGY, "true"));
        check_lz_on_bad_e_0(&s, &render(&gpu(), &s));
    }
);
