//! qa's tests for the Print graph's plan (PR #281), written from the rulings and the requirements, not from the plan's
//! own tables:
//! - R-485, the human's step 2: "Phase the Print graph's port by node category, with every one of the 78 node types
//!   and 103 presets in a task."
//! - REQ-COL-143 to REQ-COL-151: one port requirement per category, each naming its node types, and each lab preset
//!   "ported by the task of the last category its nodes need, in the order Analyse and Generate, Adjust & combine,
//!   Paint, Print, Screen, Craft, Photographic, Machines" (their notes, applied per R-369).
//! - REQ-COL-159: the four "Driven by the data" presets (the catalogue's starred ones) belong to the field-input task.
//! - REQ-COL-044 and REQ-GUI-358 (R-532): R-130's eight beside the lab's 103, "111 presets in 12 families".
//!
//! The catalogue is the lab's own, `docs/gui/reference/print_graph/catalogue/` (R-485 point 3, R-492). The Render
//! node (`source`) and `output` are registered by TASK-M7-54, not ported, so no category port names them. Each test
//! registers negative controls (R-176) that trip its assertion by message.
// The file name `qa_R-485_…` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;

/// The category ports in R-485's order, each with the lab catalogue's category keys it covers.
const PORTS: [(&str, &[&str]); 9] = [
    ("REQ-COL-143", &["analyse", "gen"]),
    ("REQ-COL-144", &["op"]),
    ("REQ-COL-145", &["paint"]),
    ("REQ-COL-146", &["print"]),
    ("REQ-COL-147", &["screen"]),
    ("REQ-COL-148", &["craft"]),
    ("REQ-COL-149", &["photo"]),
    ("REQ-COL-150", &["machine"]),
    ("REQ-COL-151", &["slot"]),
];
/// The catalogue categories no port covers: the Render node and `output` (TASK-M7-54 registers them).
const NOT_PORTED: [&str; 2] = ["source", "out"];
/// The data-driven presets' requirement.
const DATA_DRIVEN: &str = "REQ-COL-159";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("qa_print_cov: {}: {e}", p.display()))
}

/// The lab catalogue: node type → category key; preset name → the node types its graph uses; the family count.
#[derive(Clone)]
struct Catalogue {
    nodes: BTreeMap<String, String>,
    presets: BTreeMap<String, BTreeSet<String>>,
    families: usize,
}

fn catalogue() -> Catalogue {
    let dir = "docs/gui/reference/print_graph/catalogue/";
    let nodes: serde_json::Value =
        serde_json::from_str(&read(&format!("{dir}node_catalogue.json"))).unwrap();
    let presets: serde_json::Value =
        serde_json::from_str(&read(&format!("{dir}presets.json"))).unwrap();
    let families: serde_json::Value =
        serde_json::from_str(&read(&format!("{dir}preset_families.json"))).unwrap();
    let nodes = nodes["nodes"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v["category"].as_str().unwrap().to_owned()))
        .collect();
    let presets = presets
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, g)| {
            let types = g["nodes"]
                .as_object()
                .unwrap()
                .values()
                .map(|n| n["type"].as_str().unwrap().to_owned())
                .collect();
            (name.clone(), types)
        })
        .collect();
    Catalogue {
        nodes,
        presets,
        families: families.as_array().unwrap().len(),
    }
}

/// The plan: requirement id → statement (for the ids asked); requirement id → closing task; task → its Depends on.
#[derive(Clone)]
struct Plan {
    statements: BTreeMap<String, String>,
    closed_by: BTreeMap<String, String>,
    depends: BTreeMap<String, BTreeSet<String>>,
}

fn task_ids(s: &str) -> Vec<String> {
    s.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .filter(|w| w.starts_with("TASK-M"))
        .map(str::to_owned)
        .collect()
}

fn plan() -> Plan {
    let mut wanted: BTreeSet<&str> = PORTS.iter().map(|(r, _)| *r).collect();
    wanted.extend([DATA_DRIVEN, "REQ-COL-044", "REQ-GUI-358"]);
    let mut statements = BTreeMap::new();
    let mut id = String::new();
    for l in read("plan/requirements.yaml").lines() {
        if let Some(i) = l.strip_prefix("- id: ") {
            id = i.trim().to_owned();
        } else if let Some(s) = l.strip_prefix("  statement: ") {
            if wanted.contains(id.as_str()) {
                let s: String = serde_json::from_str(s.trim()).unwrap_or_else(|e| {
                    panic!("qa_print_cov: {id}'s statement is not one quoted line: {e}")
                });
                statements.insert(id.clone(), s);
            }
        }
    }
    for w in &wanted {
        assert!(
            statements.contains_key(*w),
            "qa_print_cov: {w} has no statement in plan/requirements.yaml"
        );
    }
    let (mut closed_by, mut depends) = (BTreeMap::new(), BTreeMap::new());
    for m in fs::read_dir(root().join("plan/tasks")).unwrap() {
        for f in fs::read_dir(m.unwrap().path()).unwrap() {
            let p = f.unwrap().path();
            let task = p.file_stem().unwrap().to_string_lossy().into_owned();
            let text = fs::read_to_string(&p).unwrap();
            for l in text.lines() {
                if let Some(c) = l.strip_prefix("- **Closes:** ") {
                    for r in c.split(", ") {
                        closed_by.insert(r.trim().to_owned(), task.clone());
                    }
                } else if let Some(d) = l.strip_prefix("- **Depends on:** ") {
                    depends.insert(task.clone(), task_ids(d).into_iter().collect());
                }
            }
        }
    }
    Plan {
        statements,
        closed_by,
        depends,
    }
}

/// The `sep`-separated list between the first `open` in `s` and the next `close`.
fn list<'a>(s: &'a str, open: &str, close: &str, sep: &str) -> Option<Vec<&'a str>> {
    let start = s.find(open)? + open.len();
    let len = s[start..].find(close)?;
    Some(s[start..start + len].split(sep).map(str::trim).collect())
}

/// Each port requirement's node types and presets, as its statement names them.
fn ports(plan: &Plan) -> Vec<(&'static str, Vec<String>, Vec<String>)> {
    PORTS
        .iter()
        .map(|(r, _)| {
            let s = &plan.statements[*r];
            let nodes = list(s, "node types (", ")", ",")
                .unwrap_or_else(|| panic!("qa_print_cov: {r} names no node types"))
                .into_iter()
                .map(str::to_owned)
                .collect();
            let presets = list(s, "presets (", "), each converted", ";")
                .or_else(|| list(s, "preset (", "), each converted", ";"))
                .unwrap_or_default()
                .into_iter()
                .map(str::to_owned)
                .collect();
            (*r, nodes, presets)
        })
        .collect()
}

fn port_of_category(cat: &str) -> Option<usize> {
    PORTS.iter().position(|(_, cats)| cats.contains(&cat))
}

fn check_nodes(cat: &Catalogue, plan: &Plan) {
    let ports = ports(plan);
    for (r, nodes, _) in &ports {
        for n in nodes {
            assert!(
                cat.nodes.contains_key(n),
                "qa_print_cov: {r} names node type `{n}`, which the lab catalogue lacks"
            );
        }
    }
    for (n, c) in &cat.nodes {
        if NOT_PORTED.contains(&c.as_str()) {
            continue;
        }
        let named: Vec<&str> = ports
            .iter()
            .filter(|(_, ns, _)| ns.contains(n))
            .map(|(r, _, _)| *r)
            .collect();
        assert!(
            named.len() == 1,
            "qa_print_cov: node type `{n}` is named by {named:?}, not by exactly one port"
        );
        let want = port_of_category(c)
            .unwrap_or_else(|| panic!("qa_print_cov: category `{c}` has no port"));
        assert!(
            named[0] == PORTS[want].0,
            "qa_print_cov: node type `{n}` (category `{c}`) is in {}, not its category's port {}",
            named[0],
            PORTS[want].0
        );
    }
}

/// The port index of each preset's owner (None for a data-driven preset), checking each is placed exactly once.
fn owners(cat: &Catalogue, plan: &Plan) -> BTreeMap<String, Option<usize>> {
    let ports = ports(plan);
    let dd = list(&plan.statements[DATA_DRIVEN], "presets (", ")", ",")
        .expect("qa_print_cov: REQ-COL-159 lists none");
    for (r, _, ps) in &ports {
        for p in ps {
            assert!(
                cat.presets.contains_key(p),
                "qa_print_cov: {r} names preset `{p}`, which the lab catalogue lacks"
            );
        }
    }
    let mut out = BTreeMap::new();
    for name in cat.presets.keys() {
        let named: Vec<usize> = (0..ports.len())
            .filter(|&i| ports[i].2.contains(name))
            .collect();
        if let Some(bare) = name.strip_suffix('*') {
            assert!(
                dd.contains(&bare),
                "qa_print_cov: data-driven preset `{bare}` is not in {DATA_DRIVEN}"
            );
            assert!(
                named.is_empty(),
                "qa_print_cov: data-driven preset `{name}` is also in a category port"
            );
            out.insert(name.clone(), None);
        } else {
            assert!(
                named.len() == 1,
                "qa_print_cov: preset `{name}` is in {} category ports, not exactly one",
                named.len()
            );
            out.insert(name.clone(), Some(named[0]));
        }
    }
    assert_eq!(
        dd.len(),
        4,
        "qa_print_cov: {DATA_DRIVEN} lists {} presets, not the four",
        dd.len()
    );
    out
}

/// The ports a preset's nodes need.
fn needs(cat: &Catalogue, preset: &str) -> BTreeSet<usize> {
    cat.presets[preset]
        .iter()
        .filter_map(|t| cat.nodes.get(t).and_then(|c| port_of_category(c)))
        .collect()
}

fn check_presets(cat: &Catalogue, plan: &Plan) {
    for (name, owner) in owners(cat, plan) {
        let Some(owner) = owner else { continue };
        let last = needs(cat, &name).into_iter().max().unwrap_or(0);
        assert!(
            owner == last,
            "qa_print_cov: preset `{name}` is ported by {}, not by {}, the port of the last category its nodes need",
            PORTS[owner].0,
            PORTS[last].0
        );
    }
}

fn check_dependencies(cat: &Catalogue, plan: &Plan) {
    let task = |r: &str| {
        plan.closed_by
            .get(r)
            .unwrap_or_else(|| panic!("qa_print_cov: no task closes {r}"))
            .clone()
    };
    for (name, owner) in owners(cat, plan) {
        let t = task(owner.map_or(DATA_DRIVEN, |i| PORTS[i].0));
        for n in needs(cat, &name) {
            let dep = task(PORTS[n].0);
            if dep == t {
                continue;
            }
            assert!(
                plan.depends.get(&t).is_some_and(|d| d.contains(&dep)),
                "qa_print_cov: {t} ports preset `{name}`, whose nodes need {dep}, but does not depend on {dep}"
            );
        }
    }
}

fn check_totals(cat: &Catalogue, plan: &Plan) {
    assert_eq!(
        cat.nodes.len(),
        78,
        "qa_print_cov: the lab catalogue holds {} node types, not R-485's 78",
        cat.nodes.len()
    );
    assert_eq!(
        cat.presets.len(),
        103,
        "qa_print_cov: the lab catalogue holds {} presets, not R-485's 103",
        cat.presets.len()
    );
    let r130 = [
        "Watercolour & pencil",
        "Default",
        "Drops",
        "Newspaper",
        "Vintage",
        "Poster78",
        "Riso",
        "Duotone",
    ];
    let s044 = &plan.statements["REQ-COL-044"];
    for n in r130 {
        assert!(
            s044.contains(n),
            "qa_print_cov: REQ-COL-044 does not name R-130's `{n}`"
        );
    }
    let (total, families) = (cat.presets.len() + r130.len(), cat.families + 1);
    let s358 = &plan.statements["REQ-GUI-358"];
    assert!(
        s358.contains(&format!("{total} presets")) && s358.contains(&format!("in {families} families")),
        "qa_print_cov: REQ-GUI-358 does not give {total} presets in {families} families (the lab's plus R-130's Poster)"
    );
}

#[test]
fn qa_print_cov_every_node_type_in_its_category_port() {
    check_nodes(&catalogue(), &plan());
}

#[test]
fn qa_print_cov_every_preset_in_the_port_of_its_last_category() {
    check_presets(&catalogue(), &plan());
}

#[test]
fn qa_print_cov_every_preset_task_depends_on_its_categories() {
    check_dependencies(&catalogue(), &plan());
}

#[test]
fn qa_print_cov_totals_111_presets_in_12_families() {
    check_totals(&catalogue(), &plan());
}

/// `plan` with `id`'s statement edited, `from` replaced by `to` once; a control whose target is gone fails here.
#[cfg(feature = "controls")]
fn edited(id: &str, from: &str, to: &str) -> Plan {
    let mut p = plan();
    let s = p.statements.get_mut(id).unwrap();
    assert!(
        s.contains(from),
        "qa_print_cov: {id} has no {from:?} to edit"
    );
    *s = s.replacen(from, to, 1);
    p
}

negative_control!(
    qa_print_cov_every_node_type_in_its_category_port,
    "the Screen port drops `crt`",
    expected = "node type `crt` is named by [], not by exactly one port",
    check_nodes(&catalogue(), &edited("REQ-COL-147", "crt, ", ""))
);

mod node_wrong_port {
    use super::*;
    negative_control!(
        qa_print_cov_every_node_type_in_its_category_port,
        "`linocut` moved from the Print port to the Craft port",
        expected = "node type `linocut` (category `print`) is in REQ-COL-148",
        check_nodes(&catalogue(), &{
            let p = edited("REQ-COL-146", "linocut, ", "");
            let mut p2 = p.clone();
            let s = p2.statements.get_mut("REQ-COL-148").unwrap();
            *s = s.replacen("(mcblocks, ", "(mcblocks, linocut, ", 1);
            p2
        })
    );
}

negative_control!(
    qa_print_cov_every_preset_in_the_port_of_its_last_category,
    "`Polaroid` ported by the Paint port, before its Photographic node exists",
    expected = "preset `Polaroid` is ported by REQ-COL-145, not by REQ-COL-149",
    check_presets(&catalogue(), &{
        let mut p = edited("REQ-COL-149", "(Polaroid; ", "(");
        let s = p.statements.get_mut("REQ-COL-145").unwrap();
        *s = s.replacen(
            "(Acrylic, sculptural; ",
            "(Acrylic, sculptural; Polaroid; ",
            1,
        );
        p
    })
);

mod preset_missing {
    use super::*;
    negative_control!(
        qa_print_cov_every_preset_in_the_port_of_its_last_category,
        "`Linocut` left out of every port",
        expected = "preset `Linocut` is in 0 category ports",
        check_presets(
            &catalogue(),
            &edited("REQ-COL-146", "Mezzotint; Linocut; ", "Mezzotint; ")
        )
    );
}

mod data_driven_missing {
    use super::*;
    negative_control!(
        qa_print_cov_every_preset_in_the_port_of_its_last_category,
        "`Craquelure` left out of REQ-COL-159",
        expected = "data-driven preset `Craquelure` is not in REQ-COL-159",
        check_presets(&catalogue(), &edited(DATA_DRIVEN, "Craquelure, ", ""))
    );
}

negative_control!(
    qa_print_cov_every_preset_task_depends_on_its_categories,
    "the Photographic port's task loses its dependency on the Paint port's task",
    expected = "does not depend on TASK-M7-58",
    check_dependencies(&catalogue(), &{
        let mut p = plan();
        let t = p.closed_by["REQ-COL-149"].clone();
        p.depends.get_mut(&t).unwrap().remove("TASK-M7-58");
        p
    })
);

mod data_driven_dependency {
    use super::*;
    negative_control!(
        qa_print_cov_every_preset_task_depends_on_its_categories,
        "the field-input task loses its dependency on the Adjust & combine port's task",
        expected = "does not depend on TASK-M7-57",
        check_dependencies(&catalogue(), &{
            let mut p = plan();
            let t = p.closed_by[DATA_DRIVEN].clone();
            p.depends.get_mut(&t).unwrap().remove("TASK-M7-57");
            p
        })
    );
}

negative_control!(
    qa_print_cov_totals_111_presets_in_12_families,
    "the panel requirement counting only the lab's 103",
    expected = "REQ-GUI-358 does not give 111 presets in 12 families",
    check_totals(
        &catalogue(),
        &edited("REQ-GUI-358", "111 presets", "103 presets")
    )
);

mod poster_missing {
    use super::*;
    negative_control!(
        qa_print_cov_totals_111_presets_in_12_families,
        "REQ-COL-044 without Poster78",
        expected = "REQ-COL-044 does not name R-130's `Poster78`",
        check_totals(&catalogue(), &edited("REQ-COL-044", "Poster78", "Poster"))
    );
}
