//! QA tests for TASK-M2-21, written from the requirements (REQ-SYS-011, REQ-SYS-083, REQ-SYS-084; R-482 C6; R-483 X1,
//! X2, H1, H2; gui_state_contract §2, R-69, R-96, R-466), not from the implementation. Each test registers its
//! negative control (R-176).
//!
//! - `qa_viewstate_tilted_nonlinear_roundtrip` (REQ-SYS-011): a tilted view (its basis on no coordinate axis), every
//!   block's link off its default, a non-default `q_max` and a nonlinear chart's window off the square is exported and
//!   reloaded; the reloaded `SimConfig` is the exported one bitwise, field by field, `z` at every pixel of a 33 × 33 grid
//!   is bitwise the same, as is the window's chart point, the link ids and parameter values are identical, and the
//!   state each pixel decodes to under the reloaded links at the reloaded parameter is bitwise the same and differs
//!   from the state at the default parameter (so the parameter carries meaning). Control (C6): the record with the
//!   parameter stripped does not reproduce the view.
//! - `qa_viewstate_roundtrip_property` (REQ-SYS-011, "any chart … restores exactly"): over random planes, windows and
//!   parameter values across the whole finite `f64` range (normal and subnormal, both signs), the record reloads
//!   bitwise. Signed zero is excluded: RFC 8785 writes `-0` as `0` (R-318), noted in the review.
//! - `qa_viewstate_refuses_unread_links` (R-483: an unknown or changed link is refused, never substituted; R-482 C6):
//!   a record whose link id is unknown, on the wrong block's codomain, missing, or whose parameters are missing, extra
//!   or renamed, or whose block is missing, is refused; none is read back under a default.
//! - `qa_links_filled` (REQ-SYS-083): the canonical text's `links` object, parsed, has exactly the four blocks, each
//!   exactly `{id, params}`, the ids the registry slots' defaults and the parameters each entry's tunables at their
//!   register defaults (`ledger::links`), and a non-default link or parameter serialises differently. The grep half: no
//!   struct field in the crates' sources outside `Links` holds a link id or a link parameter value.
//! - `qa_links_written_in_full` (REQ-SYS-084): at the defaults every block's id and every tunable parameter is written;
//!   no serde attribute in `sim_config.rs` skips or defaults a field; removing any one parameter or id is refused.
//! - `qa_lock_window_undo` (REQ-SYS-011's `lock_window_undo` clause; R-69, R-466): a lock on a nonlinear chart sent as
//!   one command under one history key is one history entry: one undo restores `z₀`, the lock group and the window
//!   bitwise, one redo sets all three again, and a second lock under a fresh key is a second entry.
//! - `qa_history_key_drag` (gui_state_contract §2, "A drag coalesces into one history entry"): a drag's entry has its
//!   before at the drag's start and its after at the last value sent; an edit under a new key closes it.
//!
//! Every view, window, pixel grid and parameter value here is test data, not a register constant (REQ-SYS-082).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use engine::contract::interface::EngineInterface;
use engine::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, HistoryKey, SetField, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, LinkChoice, Links, Lock, Plane,
    Quality, SimConfig, Slice, Window,
};
use engine::contract::store::StateStore;
use engine::contract::view_state::{export, restore};
use engine::lock::set_on_chart;
use engine::lookup::Tolerances;
use kernel::chart::invariant::Invariant;
use kernel::chart::InvariantLK;
use kernel::decode::{decode_at, Ic, Latent as KernelLatent};
use kernel::generated::links::{
    Defaults, LinkParams, Select, SigmoidBeta, SoftsignAlpha, StickBreaking, TanhQ,
};
use ledger::links::{self as registry};
use proptest::prelude::*;
use serde_json::Value;
use validation::negative_control;
use validation::prop::run;
use validation::roundtrip::phys;

// ---- shared ----

/// The four blocks of `Links` and the registry slot whose codomain each takes (R-479 E1: the momenta row is the four
/// momentum controls', the first's slot naming the codomain).
const BLOCKS: [(&str, &str); 4] = [
    ("mass", "mass"),
    ("alpha", "alpha"),
    ("beta", "beta"),
    ("momenta", "q0"),
];

/// The view's selection: every block off its default.
type Off = Select<StickBreaking, SoftsignAlpha, SigmoidBeta, TanhQ, TanhQ, TanhQ, TanhQ>;

fn sim(plane: Plane, window: Window, links: Links) -> SimConfig {
    SimConfig {
        chart: Chart { window },
        plane,
        slice: Slice {
            // The slice frame (R-457 E3), at its defaults: the normal and both targets z_q3.
            frame: engine::contract::sim_config::SliceFrame::along(
                engine::contract::sim_config::Z_Q3,
            ),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links,
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 75.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

fn store(s: SimConfig) -> StateStore {
    StateStore::new(
        s,
        RenderState {
            stain_graph: StainGraph::new(),
            overlays: Overlays::default(),
            hover: Hover::default(),
            palette: Palette {},
            playhead: Playhead { t: 0.0 },
            debug_view: false,
        },
    )
}

fn slot(name: &str) -> registry::Slot {
    registry::slots()
        .into_iter()
        .find(|s| s.name == name)
        .expect("a registry slot")
}

/// `entry`'s tunable parameters at their register defaults, from the registry (dd_generation_root §3.9; R-481).
fn tunable_defaults(entry: &str) -> BTreeMap<String, f64> {
    registry::tunables()
        .iter()
        .filter(|t| t.entry == entry)
        .map(|t| (t.param.to_owned(), t.default))
        .collect()
}

fn choice(id: &str) -> LinkChoice {
    LinkChoice {
        id: id.to_owned(),
        params: tunable_defaults(id),
    }
}

/// The non-default value used for `q_max`: halfway from its default to the top of its allowed range (R-481).
fn q_max_off() -> f64 {
    let t = registry::tunables()
        .iter()
        .find(|t| t.entry == "tanh_q" && t.param == "q_max")
        .expect("tanh_q's q_max is tunable");
    let v = t.default + 0.5 * (t.hi - t.default);
    assert!(v != t.default && v >= t.lo && v <= t.hi);
    v
}

/// The view's links: `stick_breaking`, `softsign_alpha`, `sigmoid_beta`, `tanh_q` at the non-default `q_max`.
fn off_links() -> Links {
    let mut momenta = choice("tanh_q");
    momenta.params.insert("q_max".to_owned(), q_max_off());
    Links {
        mass: choice("stick_breaking"),
        alpha: choice("softsign_alpha"),
        beta: choice("sigmoid_beta"),
        momenta,
    }
}

/// A tilted view: an off-origin `z₀`, a basis spread over every coordinate (no axis), a window off the square.
fn tilted() -> SimConfig {
    let plane = Plane {
        z0: [0.1, -0.3, 0.2, -0.15, 0.05, 0.3, -0.4, 0.25],
        q1: [0.31, 0.12, -0.21, 0.07, 0.19, -0.05, 0.11, -0.09],
        q2: [-0.08, 0.27, 0.13, -0.17, 0.06, 0.22, -0.12, 0.15],
    };
    let window = Window {
        centre: [0.41, 0.37],
        half: [0.0625, 0.1875],
    };
    sim(plane, window, off_links())
}

const N: usize = 33;

fn grid() -> impl Iterator<Item = (f64, f64)> {
    (0..N).flat_map(|i| (0..N).map(move |j| (i as f64 / (N - 1) as f64, j as f64 / (N - 1) as f64)))
}

/// `z(s, t) = z₀ + (2s−1)q₁ + (2t−1)q₂`, the affine chart (chart_decoder_contract Part 3).
fn z_at(p: &Plane, s: f64, t: f64) -> Latent {
    std::array::from_fn(|i| p.z0[i] + (2.0 * s - 1.0) * p.q1[i] + (2.0 * t - 1.0) * p.q2[i])
}

fn lbits(z: &Latent) -> [u64; 8] {
    z.map(f64::to_bits)
}

fn ic_bits(ic: &Ic<f64>) -> Vec<u64> {
    ic.m.iter()
        .chain(ic.r.iter().flatten())
        .chain(ic.p.iter().flatten())
        .map(|x| x.to_bits())
        .collect()
}

/// The floats as bits, each block's link id with its parameters as bits, the lock flag and the horizon's bits.
type ConfigBits = (Vec<u64>, Vec<(String, Vec<(String, u64)>)>, bool, u64);

/// Every float `SimConfig` holds today, as bits, with the link ids: equal exactly when the two configs are bitwise one.
fn config_bits(s: &SimConfig) -> ConfigBits {
    let mut f = Vec::new();
    for v in [&s.plane.z0, &s.plane.q1, &s.plane.q2, &s.lock.z_locked] {
        f.extend(lbits(v));
    }
    f.extend(s.chart.window.centre.map(f64::to_bits));
    f.extend(s.chart.window.half.map(f64::to_bits));
    let links = [
        &s.links.mass,
        &s.links.alpha,
        &s.links.beta,
        &s.links.momenta,
    ]
    .iter()
    .map(|c| {
        let p = c.params.iter().map(|(k, v)| (k.clone(), v.to_bits()));
        (c.id.clone(), p.collect())
    })
    .collect();
    (f, links, s.lock.locked, s.horizon.t.to_bits())
}

// ---- qa_viewstate_tilted_nonlinear_roundtrip (REQ-SYS-011; R-482 C6) ----

fn check_reproduces(view: &SimConfig, record: &str) {
    let back = restore(record)
        .unwrap_or_else(|e| panic!("the record does not reproduce the view: refused: {e}"));
    assert_eq!(
        config_bits(&back),
        config_bits(view),
        "the record does not reproduce the view: a field differs bitwise"
    );
    assert_eq!(&back, view, "the record does not reproduce the view");
    let p_view = view.links.link_params();
    let p_back = back.links.link_params();
    let p_default = LinkParams::<f64>::DEFAULT;
    let mut param_matters = false;
    for (s, t) in grid() {
        let (z, zb) = (z_at(&view.plane, s, t), z_at(&back.plane, s, t));
        assert_eq!(
            lbits(&zb),
            lbits(&z),
            "the record does not reproduce the view: z at ({s}, {t})"
        );
        assert_eq!(
            back.chart.window.at(s, t).map(f64::to_bits),
            view.chart.window.at(s, t).map(f64::to_bits),
            "the record does not reproduce the view: the window at ({s}, {t})"
        );
        let x = decode_at::<Off, f64>(&KernelLatent::new(z), &p_view).ic;
        let xb = decode_at::<Off, f64>(&KernelLatent::new(zb), &p_back).ic;
        assert_eq!(
            ic_bits(&xb),
            ic_bits(&x),
            "the record does not reproduce the view: the state at ({s}, {t})"
        );
        let x0 = decode_at::<Off, f64>(&KernelLatent::new(z), &p_default).ic;
        param_matters |= ic_bits(&x0) != ic_bits(&x);
    }
    assert!(
        param_matters,
        "the non-default q_max changes no pixel's state, so the test cannot see it"
    );
}

#[test]
fn qa_viewstate_tilted_nonlinear_roundtrip() {
    let view = tilted();
    for (name, c) in [
        ("mass", &view.links.mass),
        ("alpha", &view.links.alpha),
        ("momenta", &view.links.momenta),
    ] {
        let d = registry::slots()
            .into_iter()
            .find(|s| s.name == BLOCKS.iter().find(|b| b.0 == name).unwrap().1)
            .unwrap()
            .default;
        assert_ne!(c.id, d, "{name}'s link is the default");
    }
    assert_ne!(
        view.chart.window,
        Window::SQUARE,
        "the window is the square"
    );
    let record = export(&view).expect("a record");
    check_reproduces(&view, &record);
}

negative_control!(
    qa_viewstate_tilted_nonlinear_roundtrip,
    "C6: the record with the non-default parameter stripped does not reproduce the view",
    expected = "the record does not reproduce the view",
    {
        let view = tilted();
        let record = export(&view).expect("a record");
        let v: Value = serde_json::from_str(&record).unwrap();
        let mut v = v;
        v["links"]["momenta"]["params"] = serde_json::json!({});
        check_reproduces(&view, &v.to_string())
    }
);

negative_control!(
    qa_viewstate_tilted_nonlinear_roundtrip_window,
    "a record whose window is the square does not reproduce a view off it",
    expected = "the record does not reproduce the view",
    {
        let view = tilted();
        let mut moved = view.clone();
        moved.chart.window = Window::SQUARE;
        check_reproduces(&view, &export(&moved).expect("a record"))
    }
);

// ---- qa_viewstate_roundtrip_property (REQ-SYS-011) ----

/// Any finite, nonzero `f64`, normal or subnormal, of either sign.
fn any_f64() -> impl Strategy<Value = f64> {
    prop_oneof![
        prop::num::f64::NORMAL | prop::num::f64::NEGATIVE | prop::num::f64::POSITIVE,
        prop::num::f64::SUBNORMAL | prop::num::f64::NEGATIVE | prop::num::f64::POSITIVE,
        -4.0f64..4.0,
    ]
    .prop_filter("nonzero", |x| *x != 0.0)
}

fn any_latent() -> impl Strategy<Value = Latent> {
    prop::array::uniform8(any_f64())
}

fn check_property(z0: Latent, q1: Latent, q2: Latent, w: [f64; 4], mu: f64, q: f64) {
    let mut links = Links::defaults();
    links.mass.params.insert("mu_max".to_owned(), mu);
    links.momenta.params.insert("q_max".to_owned(), q);
    let view = sim(
        Plane { z0, q1, q2 },
        Window {
            centre: [w[0], w[1]],
            half: [w[2], w[3]],
        },
        links,
    );
    let back = restore(&export(&view).expect("a record")).expect("a record restores");
    assert_eq!(
        config_bits(&back),
        config_bits(&view),
        "the record does not reload bitwise"
    );
}

#[test]
fn qa_viewstate_roundtrip_property() {
    let mu = registry::tunables()
        .iter()
        .find(|t| t.param == "mu_max")
        .map(|t| t.lo..=t.hi)
        .unwrap();
    let q = registry::tunables()
        .iter()
        .find(|t| t.param == "q_max")
        .map(|t| t.lo..=t.hi)
        .unwrap();
    let strategy = (
        any_latent(),
        any_latent(),
        any_latent(),
        prop::array::uniform4(any_f64()),
        mu,
        q,
    );
    run(&strategy, |(z0, q1, q2, w, mu, q)| {
        check_property(z0, q1, q2, w, mu, q);
        Ok(())
    });
}

negative_control!(
    qa_viewstate_roundtrip_property,
    "a reload compared against a view one ulp away must fail",
    expected = "the record does not reload bitwise",
    {
        let view = tilted();
        let back = restore(&export(&view).unwrap()).unwrap();
        let mut off = view.clone();
        off.plane.q1[3] = f64::from_bits(off.plane.q1[3].to_bits() + 1);
        assert_eq!(
            config_bits(&back),
            config_bits(&off),
            "the record does not reload bitwise"
        );
    }
);

// ---- qa_viewstate_refuses_unread_links (R-483; R-482 C6) ----

/// `edit` applied to the default view's record as JSON gives a record that is refused.
fn check_refused(what: &str, edit: impl Fn(&mut Value)) {
    let mut v: Value = serde_json::from_str(&export(&tilted()).unwrap()).unwrap();
    let before = v.clone();
    edit(&mut v);
    assert_ne!(v, before, "{what}: the edit changed nothing");
    if let Ok(back) = restore(&v.to_string()) {
        panic!("a record with {what} was read back: {:?}", back.links);
    }
}

#[test]
fn qa_viewstate_refuses_unread_links() {
    check_refused("an unknown mass link", |v| {
        v["links"]["mass"]["id"] = "softmax_tanh_v2".into()
    });
    check_refused("a momentum link on the α block", |v| {
        v["links"]["alpha"]["id"] = "tanh_q".into();
        v["links"]["alpha"]["params"] = serde_json::json!({"q_max": 2.0});
    });
    check_refused("a mass link on the β block", |v| {
        v["links"]["beta"]["id"] = "softmax_tanh".into();
        v["links"]["beta"]["params"] = serde_json::json!({"mu_max": 5.0});
    });
    check_refused("no id", |v| {
        v["links"]["alpha"].as_object_mut().unwrap().remove("id");
    });
    check_refused("no params member", |v| {
        v["links"]["beta"].as_object_mut().unwrap().remove("params");
    });
    check_refused("q_max stripped", |v| {
        v["links"]["momenta"]["params"] = serde_json::json!({})
    });
    check_refused("a parameter the entry does not take", |v| {
        v["links"]["mass"]["params"] = serde_json::json!({"mu_max": 5.0})
    });
    check_refused("q_max renamed", |v| {
        v["links"]["momenta"]["params"] = serde_json::json!({"qmax": 3.0})
    });
    check_refused("a block missing", |v| {
        v["links"].as_object_mut().unwrap().remove("momenta");
    });
    check_refused("a fifth block", |v| {
        v["links"]["extra"] = serde_json::json!({"id": "sigmoid_q", "params": {"q_max": 2.0}})
    });
    check_refused("no links group", |v| {
        v.as_object_mut().unwrap().remove("links");
    });
    check_refused("no window", |v| {
        v["chart"].as_object_mut().unwrap().remove("window");
    });
}

negative_control!(
    qa_viewstate_refuses_unread_links,
    "a record whose edit is a valid link swap is read back",
    expected = "was read back",
    check_refused("a valid swap", |v| {
        v["links"]["alpha"]["id"] = "tanh_alpha".into()
    })
);

// ---- qa_links_filled (REQ-SYS-083) ----

/// The `links` object of `sim`'s canonical text, checked block by block against `links`: exactly four blocks, each
/// exactly `{id, params}`, with `links`' ids and parameter values bitwise.
fn check_filled(view: &SimConfig, links: &Links) {
    let v: Value = serde_json::from_str(&export(view).unwrap()).unwrap();
    let obj = v["links"]
        .as_object()
        .unwrap_or_else(|| panic!("the text has no links object: {v}"));
    let mut names: Vec<&str> = obj.keys().map(String::as_str).collect();
    names.sort();
    assert_eq!(
        names,
        ["alpha", "beta", "mass", "momenta"],
        "the links object is not the four blocks"
    );
    for (name, want) in [
        ("mass", &links.mass),
        ("alpha", &links.alpha),
        ("beta", &links.beta),
        ("momenta", &links.momenta),
    ] {
        let b = obj[name].as_object().unwrap();
        let mut keys: Vec<&str> = b.keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(keys, ["id", "params"], "links.{name} is not {{id, params}}");
        assert_eq!(
            b["id"].as_str(),
            Some(want.id.as_str()),
            "the text does not carry links.{name}'s id"
        );
        let got: BTreeMap<String, u64> = b["params"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_f64().unwrap().to_bits()))
            .collect();
        let want_p: BTreeMap<String, u64> = want
            .params
            .iter()
            .map(|(k, v)| (k.clone(), v.to_bits()))
            .collect();
        assert_eq!(
            got, want_p,
            "the text does not carry links.{name}'s parameters"
        );
    }
}

#[test]
fn qa_links_filled() {
    let d = Links::defaults();
    for ((name, slot_name), c) in BLOCKS.iter().zip([&d.mass, &d.alpha, &d.beta, &d.momenta]) {
        let s = slot(slot_name);
        assert_eq!(
            c.id, s.default,
            "links.{name} at the defaults is not its slot's default"
        );
        assert_eq!(
            c.params,
            tunable_defaults(s.default),
            "links.{name}'s parameters are not its entry's tunables at their defaults"
        );
    }
    let base = tilted();
    let mut at_defaults = base.clone();
    at_defaults.links = d.clone();
    check_filled(&at_defaults, &d);
    check_filled(&base, &base.links);
    // A non-default link, and a non-default parameter alone, each serialise differently.
    let mut one_link = at_defaults.clone();
    one_link.links.alpha = choice("tanh_alpha");
    let mut one_param = at_defaults.clone();
    one_param.links.mass.params.insert("mu_max".to_owned(), 4.0);
    let text = export(&at_defaults).unwrap();
    assert_ne!(
        export(&one_link).unwrap(),
        text,
        "a non-default link serialises as the default"
    );
    assert_ne!(
        export(&one_param).unwrap(),
        text,
        "a non-default parameter serialises as the default"
    );
}

negative_control!(
    qa_links_filled,
    "a view off the defaults must not carry the default links",
    expected = "the text does not carry links.",
    check_filled(&tilted(), &Links::defaults())
);

/// The workspace's crates directory.
fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .to_path_buf()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).expect("a directory") {
        let p = e.expect("an entry").path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Each struct-field declaration in `text` (`name: Type,` on its own line, inside a `struct … {}`) whose name says it
/// holds a link (`link`, `links`, `…_link`, `link_id…`) or is a registry tunable's name, and whose type is not `Links`,
/// as `(line, text)`.
fn link_fields(text: &str, tunables: &[&str]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut depth_in_struct: Option<usize> = None;
    let mut depth = 0usize;
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with("//") {
            continue;
        }
        let opens = line.matches('{').count();
        let closes = line.matches('}').count();
        if depth_in_struct.is_none() && (line.contains("struct ") && line.ends_with('{')) {
            depth_in_struct = Some(depth);
        } else if depth_in_struct.is_some_and(|d| depth == d + 1) {
            let decl = line
                .trim_start_matches("pub(crate) ")
                .trim_start_matches("pub ");
            if let Some((name, ty)) = decl.split_once(':') {
                let (name, ty) = (name.trim(), ty.trim().trim_end_matches(','));
                let named_link = name == "link"
                    || name == "links"
                    || name.ends_with("_link")
                    || name.starts_with("link_")
                    || name.ends_with("_links");
                let named_param = tunables.contains(&name);
                let ok_type = ty == "Links" || ty.starts_with("PhantomData") || ty == "LinkedViews";
                if (named_link || named_param) && !ok_type && !name.contains(' ') {
                    out.push((n + 1, raw.to_owned()));
                }
            }
        }
        depth = depth + opens - closes.min(depth + opens);
        if depth_in_struct.is_some_and(|d| depth <= d) {
            depth_in_struct = None;
        }
    }
    out
}

#[test]
fn qa_links_one_place() {
    let tunables: Vec<&str> = registry::tunables().iter().map(|t| t.param).collect();
    // The registry itself (`ledger`) defines the links and does not hold a choice of one; `kernel::generated` is the
    // kernel's uniform the parameters are read into per dispatch (R-481 G7), not a record.
    let mut files = Vec::new();
    for c in std::fs::read_dir(crates_dir()).unwrap() {
        let src = c.unwrap().path().join("src");
        if src.is_dir() && !src.starts_with(crates_dir().join("ledger")) {
            rust_files(&src, &mut files);
        }
    }
    let generated = crates_dir().join("kernel/src/generated");
    let mut found = Vec::new();
    for f in files.iter().filter(|f| !f.starts_with(&generated)) {
        let text = std::fs::read_to_string(f).unwrap();
        for (n, l) in link_fields(&text, &tunables) {
            found.push(format!("{}:{n}: {}", f.display(), l.trim()));
        }
    }
    assert!(
        files.len() > 50,
        "the scan read {} files: it is not reading the sources",
        files.len()
    );
    assert!(
        found.is_empty(),
        "a struct outside Links holds a link id or parameter: {found:#?}"
    );
}

negative_control!(
    qa_links_one_place,
    "a second struct holding a link id must be found",
    expected = "holds a link id or parameter",
    {
        let text = "pub struct Figure {\n    pub alpha_link: String,\n    pub q_max: f64,\n}\n";
        let found = link_fields(text, &["q_max"]);
        assert!(
            found.is_empty(),
            "a struct outside Links holds a link id or parameter: {found:?}"
        );
    }
);

// ---- qa_links_written_in_full (REQ-SYS-084) ----

fn check_full(text: &str) {
    let v: Value = serde_json::from_str(text).unwrap();
    for (name, slot_name) in BLOCKS {
        let id = slot(slot_name).default;
        let b = &v["links"][name];
        assert_eq!(
            b["id"].as_str(),
            Some(id),
            "the links are not written in full: links.{name}.id"
        );
        for (p, d) in tunable_defaults(id) {
            assert_eq!(
                b["params"][&p].as_f64().map(f64::to_bits),
                Some(d.to_bits()),
                "the links are not written in full: links.{name}.params.{p}"
            );
        }
    }
}

#[test]
fn qa_links_written_in_full() {
    let mut view = tilted();
    view.links = Links::defaults();
    let text = export(&view).unwrap();
    check_full(&text);
    // Every tunable of every default entry is written: at least one per parameterised entry the defaults use.
    assert!(
        text.contains("\"mu_max\"") && text.contains("\"q_max\""),
        "the links are not written in full: {text}"
    );
    // No serde attribute in the file defining Links and LinkChoice skips or defaults a field.
    let src =
        std::fs::read_to_string(crates_dir().join("engine/src/contract/sim_config.rs")).unwrap();
    for l in src
        .lines()
        .filter(|l| l.trim_start().starts_with("#[serde"))
    {
        assert!(
            !l.contains("skip_serializing") && !l.contains("default"),
            "sim_config.rs has `{}`",
            l.trim()
        );
    }
    // Removing any one parameter, or any one block's id, is refused.
    let v: Value = serde_json::from_str(&text).unwrap();
    for (name, _) in BLOCKS {
        let params: Vec<String> = v["links"][name]["params"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for p in params {
            let mut w = v.clone();
            w["links"][name]["params"]
                .as_object_mut()
                .unwrap()
                .remove(&p);
            assert!(
                restore(&w.to_string()).is_err(),
                "a record without links.{name}.params.{p} was read back"
            );
        }
        let mut w = v.clone();
        w["links"][name].as_object_mut().unwrap().remove("id");
        assert!(
            restore(&w.to_string()).is_err(),
            "a record without links.{name}.id was read back"
        );
    }
}

negative_control!(
    qa_links_written_in_full,
    "a text with mu_max removed is not written in full",
    expected = "the links are not written in full",
    {
        let mut view = tilted();
        view.links = Links::defaults();
        let mut v: Value = serde_json::from_str(&export(&view).unwrap()).unwrap();
        v["links"]["mass"]["params"]
            .as_object_mut()
            .unwrap()
            .remove("mu_max");
        check_full(&v.to_string())
    }
);

// ---- qa_lock_window_undo (REQ-SYS-011; R-69, R-466) ----

fn lk() -> InvariantLK<f64> {
    InvariantLK(Invariant {
        z_cfg: [0.15, -0.35],
        z_mass: [-0.2, 0.4],
        k_max: 2.0,
        gamma_k: 1.5,
    })
}

fn tol() -> Tolerances {
    Tolerances::register(f64::EPSILON)
}

type Send = fn(&mut StateStore, Vec<SetField>, u64);

fn under_key(st: &mut StateStore, edits: Vec<SetField>, key: u64) {
    for e in edits {
        st.set_field_under(e, HistoryKey(key));
    }
}

fn check_lock_undo(send: Send) {
    let window = Window {
        centre: [0.45, 0.55],
        half: [0.2, 0.15],
    };
    let mut start = tilted();
    start.links = Links::defaults();
    start.chart.window = window;
    let mut st = store(start.clone());
    let first =
        set_on_chart::<Defaults, _, _>(&lk(), &window, 0.7, 0.35, phys, &tol()).expect("a lock");
    assert!(
        first
            .edits
            .iter()
            .any(|e| matches!(e.edit, Edit::Sim(SimField::Window(w)) if w == first.window)),
        "the lock's command carries no window edit"
    );
    send(&mut st, first.edits.to_vec(), 7);
    let locked = st.snapshot().sim;
    assert_eq!(
        locked.chart.window, first.window,
        "the store's window is not the lock's"
    );
    assert!(locked.lock.locked, "the store is not locked");
    assert_eq!(
        st.snapshot().history.undo_depth,
        1,
        "the lock is not one history entry"
    );
    // A second lock, re-centred again, under a fresh key: a second entry.
    let second = set_on_chart::<Defaults, _, _>(&lk(), &first.window, 0.25, 0.6, phys, &tol())
        .expect("a lock");
    send(&mut st, second.edits.to_vec(), 8);
    assert_eq!(
        st.snapshot().history.undo_depth,
        2,
        "the second lock is not its own entry"
    );
    st.undo();
    assert_eq!(
        config_bits(&st.snapshot().sim),
        config_bits(&locked),
        "one undo did not restore the first lock"
    );
    st.undo();
    let undone = st.snapshot();
    assert_eq!(
        config_bits(&undone.sim),
        config_bits(&start),
        "one undo did not restore z₀, the lock group and the window"
    );
    assert_eq!(
        undone.history.undo_depth, 0,
        "the lock left an entry behind"
    );
    st.redo();
    assert_eq!(
        config_bits(&st.snapshot().sim),
        config_bits(&locked),
        "one redo did not set the lock again"
    );
}

#[test]
fn qa_lock_window_undo() {
    check_lock_undo(under_key);
}

negative_control!(
    qa_lock_window_undo,
    "a lock sent edit by edit with no key is three entries, so one undo leaves z₀ and the lock group set",
    expected = "is not one history entry",
    check_lock_undo(|st, edits, _| {
        for e in edits {
            st.set_field(e);
        }
    })
);

negative_control!(
    qa_lock_window_undo_dropped,
    "a lock whose command drops the window edit leaves the window unmoved",
    expected = "the store's window is not the lock's",
    check_lock_undo(|st, edits, key| {
        let kept = edits
            .into_iter()
            .filter(|e| !matches!(e.edit, Edit::Sim(SimField::Window(_))))
            .collect();
        under_key(st, kept, key)
    })
);

// ---- qa_history_key_drag (gui_state_contract §2, R-96) ----

fn horizon(t: f64) -> SetField {
    SetField {
        edit: Edit::Sim(SimField::Horizon(Horizon { t })),
        no_history: false,
    }
}

fn check_drag(send: fn(&mut StateStore, SetField, u64)) {
    let mut st = store(tilted());
    for t in [80.0, 90.0, 120.0, 150.0] {
        send(&mut st, horizon(t), 3);
    }
    assert_eq!(
        st.snapshot().history.undo_depth,
        1,
        "a drag is not one history entry"
    );
    send(&mut st, horizon(60.0), 4);
    assert_eq!(
        st.snapshot().history.undo_depth,
        2,
        "a new key did not close the drag's entry"
    );
    st.undo();
    assert_eq!(
        st.snapshot().sim.horizon.t,
        150.0,
        "the drag's after is not its last value"
    );
    st.undo();
    assert_eq!(
        st.snapshot().sim.horizon.t,
        75.0,
        "the drag's before is not its start"
    );
    st.redo();
    assert_eq!(
        st.snapshot().sim.horizon.t,
        150.0,
        "the drag's redo is not its last value"
    );
}

#[test]
fn qa_history_key_drag() {
    check_drag(|st, e, k| st.set_field_under(e, HistoryKey(k)));
}

negative_control!(
    qa_history_key_drag,
    "a drag whose frames each carry a fresh key is an entry per frame",
    expected = "a drag is not one history entry",
    check_drag({
        fn fresh(st: &mut StateStore, e: SetField, _: u64) {
            let n = st.snapshot().history.undo_depth as u64 + 100;
            st.set_field_under(e, HistoryKey(n));
        }
        fresh
    })
);
