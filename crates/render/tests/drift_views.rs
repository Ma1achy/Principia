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
//! - `drift_view_neutral_*`: `symlog`'s neutral band (render contract Part 5; R-652): a fresh sample renders exactly
//!   neutral on the backend the test runs on, a drift past the band still shows, and the band follows `|E_0|`, `|Lz_0|`.
//!   On Vulkan these run under CI's `ci` job, on lavapipe; on Metal under the `gpu-metal` suites' job and locally.

use ledger::constants::{DRIFT_LIN_R_E, DRIFT_LIN_R_LZ, EPS_E_DEFAULT, EPS_L_DEFAULT};
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

// ── `symlog`'s neutral band (render contract Part 5; R-652) ─────────────────────────────────────────────────────────

/// A view at its defaults, each by its own read of `s`.
fn view_of(view: View) -> Colouring {
    debug(view.id, &[])
}

/// `c`'s 8-bit values, as the golden harness quantises a render.
fn bytes(c: Rgb) -> [u8; 3] {
    c.map(|v| (v * 255.0).round() as u8)
}

/// The neutral `c0` exactly, by its 8-bit values and within [`TOL`].
fn check_neutral(s: &Scene, image: &[Rgb], i: u32, what: &str) {
    let c0 = present::drift_ramp(0.5, false);
    let width = s.size().0;
    for (x, y) in s.pixels(i) {
        let got = image[(y * width + x) as usize];
        assert!(
            bytes(got) == bytes(c0) && near(got, c0),
            "{what}: pixel ({x}, {y}) of sample {i} of `{}` is {got:?}, not neutral {c0:?}",
            s.name
        );
    }
}

/// A scene of `view` over the showcase's eight states, each sample fresh: `E_0` and `Lz_0` its own `H(r, p)` and
/// `L_z(r, p)` by the CPU's read side, its latches 0 so that no styling draws. `offset` moves each reference by that
/// fraction of itself, for the control. Every state's rounding bound is within the band's `k`, asserted here
/// (the band's domain: `(9K + 15|V|)/|E_0| ≤ 128` and `5·Σ|xᵢp_{y,i}| + |yᵢp_{x,i}| / |Lz_0| ≤ 128`).
fn fresh_scene(colouring: Colouring, offset: f32) -> Scene {
    let mut s = row(8, colouring);
    validation::golden_scene::showcase(&mut s.set, 6);
    for i in 0..8 {
        s.set.sample(i).E_0(0.0).Lz_0(0.0).drift_max(0.0, 0.0);
        let read = s.read(i);
        let (e, l) = (read.energy_drift, read.Lz_drift);
        s.set
            .sample(i)
            .E_0(e * (1.0 + offset))
            .Lz_0(l * (1.0 + offset));
    }
    s
}

/// The samples of [`fresh_scene`] whose `H` and `L_z` are not near a cancellation, by their own rounding bounds.
fn in_domain(s: &Scene, i: u32, energy: bool) -> bool {
    let read = s.read(i);
    let m = s.masses(i);
    let f = |x: f32| f64::from(x);
    let terms = |k: usize| {
        (
            f(read.r[k][0]) * f(read.p[k][1]),
            f(read.r[k][1]) * f(read.p[k][0]),
        )
    };
    if energy {
        let kinetic: f64 = (0..3)
            .map(|k| (f(read.p[k][0]).powi(2) + f(read.p[k][1]).powi(2)) / (2.0 * f(m[k])))
            .sum();
        let v: f64 = [(0, 1), (0, 2), (1, 2)]
            .iter()
            .map(|&(a, b)| {
                f(m[a]) * f(m[b])
                    / (f(read.r[a][0]) - f(read.r[b][0])).hypot(f(read.r[a][1]) - f(read.r[b][1]))
            })
            .sum();
        9.0 * kinetic + 15.0 * v <= 128.0 * f(read.E_0).abs()
    } else {
        let sum: f64 = (0..3)
            .map(|k| {
                let (a, b) = terms(k);
                a.abs() + b.abs()
            })
            .sum();
        5.0 * sum <= 128.0 * f(read.Lz_0).abs()
    }
}

/// Every in-domain fresh sample of `s` renders exactly neutral; at least `least` of them are in the domain.
fn check_fresh(h: &GpuHarness, view: View, s: &Scene, least: u32) {
    let image = render(h, s);
    let mut judged = 0;
    for i in 0..8 {
        if in_domain(s, i, view.energy) {
            judged += 1;
            check_neutral(s, &image, i, view.id);
        }
    }
    assert!(judged >= least, "only {judged} fresh samples in the domain");
}

#[test]
fn drift_view_neutral_fresh_sample_renders_exactly_neutral() {
    let h = gpu();
    for view in VIEWS {
        check_fresh(&h, view, &fresh_scene(view_of(view), 0.0), 4);
    }
}

negative_control!(
    drift_view_neutral_fresh_sample_renders_exactly_neutral,
    "each reference off by 2⁻¹⁵ of itself, twice the band: a real drift, not a fresh sample",
    expected = "not neutral",
    {
        let h = gpu();
        check_fresh(
            &h,
            VIEWS[1],
            &fresh_scene(view_of(VIEWS[1]), 1.0 / 32_768.0),
            4,
        );
    }
);

/// A three-tile scene of `view` on the dyadic state: tile 0's drift is `inside` times the band at the reference, tile
/// 1's `past` times it and tile 2's `−past` times it.
fn band_scene(view: View, colouring: Colouring, inside: f64, past: f64) -> Scene {
    let mut s = row(3, colouring);
    let (h, l) = invariants(&mut s, 0);
    let reference = if view.energy { h } else { l };
    let band = present::DRIFT_NEUTRAL_BAND * f64::from(reference.abs());
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

/// Tile 0 of [`band_scene`] is neutral, and tiles 1 and 2 show their drifts: off `c0` on the `c+` side and the `c−` side.
fn check_band(view: View, s: &Scene, image: &[Rgb]) {
    check_neutral(s, image, 0, "a drift inside the band");
    let c0 = present::drift_ramp(0.5, false);
    let (above, below) = (tile(s, image, 1), tile(s, image, 2));
    assert!(
        distance(above, c0) > 0.02 && distance(below, c0) > 0.02,
        "`{}`: the band swallowed a drift past it: {above:?}, {below:?}, c0 {c0:?}",
        view.id
    );
    assert!(
        above[0] > above[2] && below[2] > below[0],
        "`{}`: a drift past the band is on the wrong side of c0: {above:?}, {below:?}",
        view.id
    );
}

#[test]
fn drift_view_neutral_band_keeps_a_drift_past_it() {
    let h = gpu();
    for view in VIEWS {
        let s = band_scene(view, view_of(view), 0.5, 2.0);
        check_band(view, &s, &render(&h, &s));
    }
}

negative_control!(
    drift_view_neutral_band_keeps_a_drift_past_it,
    "a band 8 times wider swallows the drift past it",
    expected = "the band swallowed a drift past it",
    {
        let view = VIEWS[1];
        let colouring = broken(view.id, "ctx.sample.Lz_0)", "8.0 * ctx.sample.Lz_0)");
        let s = band_scene(view, colouring, 0.5, 2.0);
        check_band(view, &s, &render(&gpu(), &s));
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
        check_band(view, &s, &render(&gpu(), &s));
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
    let band = present::DRIFT_NEUTRAL_BAND as f32;
    let reference = 4.0_f32;
    let at = band * reference;
    let above = f32::from_bits(at.to_bits() + 1);
    let symlog = present::DRIFT_SYMLOG;
    assert_eq!(shown(at, symlog, reference), 0.0, "at the band");
    assert_eq!(shown(-at, symlog, -reference), 0.0, "at the band, negative");
    assert_eq!(shown(above, symlog, reference), above, "just past the band");
    assert_eq!(
        shown(-above, symlog, reference),
        -above,
        "just past it, negative"
    );
    assert_eq!(
        shown(at, present::DRIFT_LIN, reference),
        at,
        "`lin` has no band"
    );
    assert_eq!(
        shown(at, present::DRIFT_LOG, reference),
        at,
        "`log` has no band"
    );
    // A zero reference has an empty band: exactly 0 stays 0, and the least drift shows.
    assert_eq!(shown(0.0, symlog, 0.0), 0.0, "0 at a zero reference");
    assert_eq!(
        shown(1e-30, symlog, 0.0),
        1e-30,
        "a drift at a zero reference"
    );
    let nan = f32::from_bits(present::ABSENT_NAN_BITS);
    assert_eq!(
        shown(nan, symlog, reference).to_bits(),
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
    check_shown(
        |x, _, r| if x.abs() < (present::DRIFT_NEUTRAL_BAND as f32) * r.abs() {
            0.0
        } else {
            x
        }
    )
);

/// The band never makes a non-finite operand neutral: `|x| ≤ band·|∞|` holds for every `x` (R-436). A NaN of either sign
/// counts, and the largest finite reference still has its band.
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
            "a finite drift against the reference {bad:?}"
        );
        assert_eq!(
            shown(bad, symlog, 1.0).to_bits(),
            bad.to_bits(),
            "the drift {bad:?}"
        );
        assert_eq!(
            shown(bad, symlog, bad).to_bits(),
            bad.to_bits(),
            "the drift and the reference {bad:?}"
        );
    }
    assert_eq!(shown(1.0, symlog, f32::MAX), 0.0, "the largest reference");
}

#[test]
fn drift_view_neutral_band_passes_a_non_finite_operand() {
    check_shown_non_finite(present::dbg_drift_shown);
}

negative_control!(
    drift_view_neutral_band_passes_a_non_finite_operand,
    "a band testing only the absence NaN, as before R-436's guard, so an infinite reference makes a drift neutral",
    expected = "a finite drift against the reference inf",
    check_shown_non_finite(|x, _, r| {
        if x.to_bits() == present::ABSENT_NAN_BITS {
            x
        } else if x.abs() <= (present::DRIFT_NEUTRAL_BAND as f32) * r.abs() {
            0.0
        } else {
            x
        }
    })
);

/// The mirror agrees with the assembled source on the band's literal.
#[test]
fn drift_view_neutral_band_literal_is_the_mirrors() {
    let source = include_str!("../shaders/wgsl/lib/drift.wgsl");
    let literal = format!(
        "const DBG_DRIFT_NEUTRAL_BAND: f32 = {:?};",
        present::DRIFT_NEUTRAL_BAND
    );
    assert!(source.contains(&literal), "no `{literal}` in drift.wgsl");
}

negative_control!(
    drift_view_neutral_band_literal_is_the_mirrors,
    "another literal",
    expected = "in drift.wgsl",
    {
        let source = include_str!("../shaders/wgsl/lib/drift.wgsl");
        assert!(
            source.contains("DBG_DRIFT_NEUTRAL_BAND: f32 = 1.0;"),
            "not in drift.wgsl"
        );
    }
);
