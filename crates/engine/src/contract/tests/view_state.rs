//! The ViewState record and `SimConfig`'s links (REQ-SYS-011, REQ-SYS-083, REQ-SYS-084; chart_decoder_contract Part
//! 2.5, "Integrity: the link is part of the experiment"; R-482 C6; R-483 X1, X2, H1, H2). Each test registers its
//! negative control (R-176).
//!
//! - `viewstate_roundtrip`: a tilted view, with a non-default link, a non-default link parameter and a nonlinear
//!   chart's window off the square, is exported and restored, and reproduces bitwise: the record's text, `z` at every
//!   pixel of the plane, the chart point at every pixel of the window, each block's link id and parameter values, and
//!   the state each pixel decodes to under the restored links. Control (C6): the same record with the parameter stripped
//!   does not reproduce.
//! - `links_filled`: `Links` has the four blocks, each an id and its parameters, and the canonical text of a
//!   `SimConfig` names each; a non-default link serialises differently from the default.
//! - `links_written_in_full`: at the defaults the text names every block's id and every parameter, no field of the
//!   links is skipped or defaulted by serde, and a text with one parameter removed is refused.
//!
//! The test inputs (the plane, the window, `μ_max = 4`, the pixel grid) are test data, not register constants
//! (REQ-SYS-082).

use std::collections::BTreeMap;

use kernel::decode::{decode_at, Ic, Latent as KernelLatent};
use kernel::generated::links::{
    Defaults, Link, Select, Selection, SigmoidBeta, SigmoidQ, SoftmaxTanh, TanhAlpha,
};

use crate::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, LinkChoice, Links, Lock, Plane,
    Quality, SimConfig, Slice, SliceFrame, Window, Z_Q3,
};
use crate::contract::view_state::{export, restore};
use crate::lock::point;
use crate::nav::tilt_vector;

/// The non-default selection the view uses: `tanh_alpha` for `α`, the rest the defaults.
type TanhAlphaLinks =
    Select<SoftmaxTanh, TanhAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

/// The pixels sampled per side.
const SIDE: usize = 17;

/// The unit vector along the latent coordinate `k`.
fn axis(k: usize) -> Latent {
    std::array::from_fn(|i| if i == k { 1.0 } else { 0.0 })
}

/// A `SimConfig` with `plane`, `window` and `links`, every other group at the skeleton's values.
fn config(plane: Plane, window: Window, links: Links) -> SimConfig {
    SimConfig {
        chart: Chart { window },
        plane,
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links,
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 100.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

/// The view: a plane off the origin, its `q₁` tilted 30° from `z_α` into `z_q2` and its `q₂` tilted 20° from `z_β` into
/// `z_μ1`, so the plane lies along no coordinate axis; the window off the square; `α` under `tanh_alpha` and the masses
/// under `softmax_tanh` at `μ_max = 4`.
fn tilted_view() -> SimConfig {
    let q1 = tilt_vector(&axis(0).map(|x| 0.75 * x), 30.0, &axis(4)).expect("a tilt");
    let q2 = tilt_vector(&axis(1).map(|x| 0.5 * x), 20.0, &axis(6)).expect("a tilt");
    let plane = Plane {
        z0: [0.25, -0.5, 0.125, -0.25, 0.375, 0.0625, -0.75, 0.5],
        q1,
        q2,
    };
    let window = Window {
        centre: [0.3, 0.6],
        half: [0.125, 0.25],
    };
    let mut links = Links::of::<TanhAlphaLinks>().expect("one momentum link");
    links.mass.params.insert("mu_max".to_owned(), 4.0);
    config(plane, window, links)
}

/// The pixel `(i, j)` of the grid, `(s, t)` in `[0, 1]²`.
fn pixel(i: usize, j: usize) -> (f64, f64) {
    let at = |k: usize| k as f64 / (SIDE - 1) as f64;
    (at(i), at(j))
}

fn bits<const N: usize>(v: &[f64; N]) -> [u64; N] {
    v.map(f64::to_bits)
}

fn ic_bits(ic: &Ic<f64>) -> Vec<u64> {
    ic.m.iter()
        .chain(ic.r.iter().flatten())
        .chain(ic.p.iter().flatten())
        .map(|x| x.to_bits())
        .collect()
}

/// Each block's id and parameter values, the parameters as bits.
fn link_bits(links: &Links) -> Vec<(String, BTreeMap<String, u64>)> {
    links
        .blocks()
        .iter()
        .map(|(_, c)| {
            let params = c.params.iter().map(|(k, v)| (k.clone(), v.to_bits()));
            (c.id.clone(), params.collect())
        })
        .collect()
}

/// `record`, `sim`'s export or a text standing in for it, restores to `sim` bitwise: its text, `z` at every pixel of the
/// plane, the chart point at every pixel of the window, the link ids and parameter values, and each pixel's decoded
/// state under the restored links (`TanhAlphaLinks`, at the restored parameters).
fn check_reproduces(sim: &SimConfig, record: &str) {
    let back = match restore(record) {
        Ok(back) => back,
        Err(e) => panic!("the record does not reproduce the view: {e}"),
    };
    let text = |s: &SimConfig| export(s).expect("a canonical text");
    assert_eq!(
        text(&back),
        text(sim),
        "the record does not reproduce the view: its text differs"
    );
    assert_eq!(
        link_bits(&back.links),
        link_bits(&sim.links),
        "the record does not reproduce the view: a link id or parameter differs"
    );
    let (want, got) = (sim.links.link_params(), back.links.link_params());
    for i in 0..SIDE {
        for j in 0..SIDE {
            let (s, t) = pixel(i, j);
            let (z, z_back) = (point(&sim.plane, s, t), point(&back.plane, s, t));
            assert_eq!(
                bits(&z_back),
                bits(&z),
                "the record does not reproduce the view: z at ({s}, {t}) differs"
            );
            assert_eq!(
                bits(&back.chart.window.at(s, t)),
                bits(&sim.chart.window.at(s, t)),
                "the record does not reproduce the view: the window's chart point at ({s}, {t}) differs"
            );
            let ic = decode_at::<TanhAlphaLinks, f64>(&KernelLatent::new(z), &want).ic;
            let ic_back = decode_at::<TanhAlphaLinks, f64>(&KernelLatent::new(z_back), &got).ic;
            assert_eq!(
                ic_bits(&ic_back),
                ic_bits(&ic),
                "the record does not reproduce the view: the state at ({s}, {t}) differs"
            );
        }
    }
}

#[test]
fn viewstate_roundtrip() {
    let sim = tilted_view();
    // The view is what it says: tilted off every axis, a non-default link and a non-default parameter.
    let (q1, q2) = (&sim.plane.q1, &sim.plane.q2);
    assert!(
        q1[0] != 0.0 && q1[4] != 0.0 && q2[1] != 0.0 && q2[6] != 0.0,
        "the view is not tilted"
    );
    assert_ne!(
        sim.links.alpha,
        Links::defaults().alpha,
        "α's link is the default"
    );
    assert_ne!(
        sim.links.mass,
        Links::defaults().mass,
        "μ_max is the default"
    );
    let record = export(&sim).expect("a canonical text");
    check_reproduces(&sim, &record);
}

validation::negative_control!(
    viewstate_roundtrip,
    "the same record with the parameter stripped must fail to reproduce (R-482 C6)",
    expected = "the record does not reproduce the view",
    {
        let sim = tilted_view();
        let record = export(&sim).expect("a canonical text");
        let stripped = record.replace(r#""params":{"mu_max":4}"#, r#""params":{}"#);
        assert_ne!(stripped, record, "the control stripped nothing");
        check_reproduces(&sim, &stripped)
    }
);

// ----- links_filled (REQ-SYS-083) -----

/// The canonical text of `links`, as `SimConfig`'s text carries it.
fn links_text(links: &Links) -> String {
    let blocks: Vec<String> = links
        .blocks()
        .iter()
        .map(|(name, c)| {
            let params: Vec<String> = c
                .params
                .iter()
                .map(|(k, v)| format!(r#""{k}":{v}"#))
                .collect();
            format!(
                r#""{name}":{{"id":"{}","params":{{{}}}}}"#,
                c.id,
                params.join(",")
            )
        })
        .collect();
    // JCS sorts the members: alpha, beta, mass, momenta.
    let mut sorted = blocks;
    sorted.sort();
    format!(r#""links":{{{}}}"#, sorted.join(","))
}

/// `sim`'s canonical text carries `links`, each block by name with its id and its parameters.
fn check_filled(sim: &SimConfig, links: &Links) {
    let text = export(sim).expect("a canonical text");
    let want = links_text(links);
    assert!(
        text.contains(&want),
        "the SimConfig text does not carry the links {want}: {text}"
    );
}

/// The four blocks, by field: a compile-time list, so a block removed or renamed does not compile.
fn four_blocks(links: &Links) -> [&LinkChoice; 4] {
    let Links {
        mass,
        alpha,
        beta,
        momenta,
    } = links;
    [mass, alpha, beta, momenta]
}

#[test]
fn links_filled() {
    let defaults = Links::defaults();
    let ids: Vec<&str> = four_blocks(&defaults)
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["softmax_tanh", "sigmoid_alpha", "sigmoid_beta", "sigmoid_q"],
        "the default links are not dd_decoder §3's"
    );
    assert_eq!(
        Links::of::<Defaults>(),
        Ok(defaults.clone()),
        "the defaults are not the default selection's links"
    );
    let default_view = config(tilted_view().plane, Window::SQUARE, defaults.clone());
    check_filled(&default_view, &defaults);
    let view = tilted_view();
    check_filled(&view, &view.links);
    assert_ne!(
        export(&view).expect("a text"),
        export(&config(view.plane.clone(), view.chart.window, defaults)).expect("a text"),
        "a non-default link serialises as the default"
    );
}

validation::negative_control!(
    links_filled,
    "a SimConfig with a non-default link must not serialise as the default",
    expected = "the SimConfig text does not carry the links",
    check_filled(&tilted_view(), &Links::defaults())
);

// ----- links_written_in_full (REQ-SYS-084) -----

/// The default links' text, every block's id and every parameter.
const FULL: &str = r#""links":{"alpha":{"id":"sigmoid_alpha","params":{}},"beta":{"id":"sigmoid_beta","params":{}},"mass":{"id":"softmax_tanh","params":{"mu_max":5}},"momenta":{"id":"sigmoid_q","params":{"q_max":2}}}"#;

/// `text` writes the default links in full: every block's id and every parameter, as [`FULL`] has them.
fn check_full(text: &str) {
    assert!(
        text.contains(FULL),
        "the links are not written in full at the defaults: {text}"
    );
}

/// `text` is refused, the refusal naming `param`.
fn check_refused(text: &str, param: &str) {
    match restore(text) {
        Ok(_) => panic!("a record without `{param}` was read back"),
        Err(e) => assert!(
            e.contains(param),
            "the refusal does not name `{param}`: {e}"
        ),
    }
}

#[test]
fn links_written_in_full() {
    let sim = config(tilted_view().plane, Window::SQUARE, Links::defaults());
    let text = export(&sim).expect("a canonical text");
    check_full(&text);
    // No serde attribute in sim_config.rs skips or defaults a field.
    let attributes = include_str!("../sim_config.rs")
        .lines()
        .filter(|l| l.trim_start().starts_with("#[serde"));
    for line in attributes {
        assert!(
            !line.contains("skip") && !line.contains("default"),
            "sim_config.rs has `{line}`: a link field may be skipped or defaulted"
        );
    }
    let edit = |from: &str, to: &str| {
        let edited = text.replace(from, to);
        assert_ne!(edited, text, "the test's edit `{from}` changed nothing");
        edited
    };
    check_refused(&edit(r#"{"mu_max":5}"#, "{}"), "mu_max");
    check_refused(&edit(r#"{"q_max":2}"#, "{}"), "q_max");
    let alpha = r#""id":"sigmoid_alpha","params":{}"#;
    let extra = edit(alpha, r#""id":"sigmoid_alpha","params":{"mu_max":5}"#);
    check_refused(&extra, "mu_max");
    check_refused(
        &edit(alpha, r#""id":"no_such_link","params":{}"#),
        "no_such_link",
    );
    check_refused(
        &edit(alpha, r#""id":"sigmoid_beta","params":{}"#),
        "sigmoid_beta",
    );
    check_refused(
        &edit(r#""beta":{"id":"sigmoid_beta","params":{}},"#, ""),
        "beta",
    );
}

validation::negative_control!(
    links_written_in_full,
    "a text with one parameter removed must not pass as written in full",
    expected = "the links are not written in full at the defaults",
    check_full(
        &export(&config(
            tilted_view().plane,
            Window::SQUARE,
            Links::defaults()
        ))
        .expect("a canonical text")
        .replace(r#"{"mu_max":5}"#, "{}")
    )
);

validation::negative_control!(
    links_written_in_full_refuses,
    "a complete record must not count as refused",
    expected = "was read back",
    check_refused(
        &export(&config(
            tilted_view().plane,
            Window::SQUARE,
            Links::defaults()
        ))
        .expect("a canonical text"),
        "mu_max"
    )
);

/// `Link::NAME` is the id `Links` records: the selection's α link is `tanh_alpha`.
#[test]
fn links_of_records_the_selection() {
    check_alpha::<TanhAlphaLinks>("tanh_alpha");
}

fn check_alpha<S: Selection>(want: &str) {
    let links = Links::of::<S>().expect("one momentum link");
    assert_eq!(
        links.alpha.id,
        <S::AlphaLink as Link<1, 1>>::NAME,
        "Links::of does not record the selection's α link"
    );
    assert_eq!(links.alpha.id, want, "the α link is not {want}");
}

validation::negative_control!(
    links_of_records_the_selection,
    "the default selection's α link is not tanh_alpha",
    expected = "the α link is not tanh_alpha",
    check_alpha::<Defaults>("tanh_alpha")
);

// ----- The selection's links and the parameters the kernel reads -----

/// The selection with `tanh_q` on the first momentum control alone.
type SplitMomenta = Select<
    SoftmaxTanh,
    TanhAlpha,
    SigmoidBeta,
    kernel::generated::links::TanhQ,
    SigmoidQ,
    SigmoidQ,
    SigmoidQ,
>;

/// The selection with `stick_breaking` for the masses and `tanh_q` for every momentum control.
type StickTanhQ = Select<
    kernel::generated::links::StickBreaking,
    TanhAlpha,
    SigmoidBeta,
    kernel::generated::links::TanhQ,
    kernel::generated::links::TanhQ,
    kernel::generated::links::TanhQ,
    kernel::generated::links::TanhQ,
>;

/// `Links::of` refuses a selection whose momentum controls differ, naming each, and `link_params` reads `μ_max` from
/// the mass block and `q_max` from the momenta, leaving a parameter the chosen entry does not take at its default.
fn check_params(links: &Links, want: (f64, f64)) {
    let p = links.link_params();
    assert_eq!(
        (p.mu_max.to_bits(), p.q_max.to_bits()),
        (want.0.to_bits(), want.1.to_bits()),
        "link_params read the wrong values"
    );
}

#[test]
fn links_params_and_selection() {
    let split = Links::of::<SplitMomenta>().expect_err("four momentum links in one row");
    assert!(
        split.contains("tanh_q") && split.contains("sigmoid_q"),
        "the refusal does not name the momentum links: {split}"
    );
    let mut links = Links::of::<StickTanhQ>().expect("one momentum link");
    assert_eq!(
        links.mass.params,
        BTreeMap::new(),
        "stick_breaking takes no parameter"
    );
    assert_eq!(links.momenta.id, "tanh_q", "the momenta's link");
    check_params(&links, (5.0, 2.0));
    links.momenta.params.insert("q_max".to_owned(), 3.0);
    check_params(&links, (5.0, 3.0));
    let mut view = tilted_view().links;
    check_params(&view, (4.0, 2.0));
    view.momenta.params.insert("q_max".to_owned(), 1.5);
    check_params(&view, (4.0, 1.5));
}

validation::negative_control!(
    links_params_and_selection,
    "the defaults' parameters are not μ_max = 4",
    expected = "link_params read the wrong values",
    check_params(&Links::defaults(), (4.0, 2.0))
);
