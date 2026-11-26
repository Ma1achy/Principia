//! The preset library's files (`presets/{production,debug}/`): each holds its catalogue metadata and its graph in one
//! file, every file is embedded, and the debug section's presets each sit in the group colour_composition §9.6 names.
//!
//! Pass criteria, written before the runs:
//! - the embedded table is exactly the files in the two directories (no file left out, none missing);
//! - a debug file names a group and a production file none; the section is the directory;
//! - `group_of` files the structural views under quad-structure and the cache age under profiling, and the DECODE,
//!   agreement and ROUNDTRIP presets are decoded-IC;
//! - each addressing preset's channels: TL = RB leaves G at 0 in its WGSL and centre = BG leaves R at 0.

use std::collections::BTreeSet;
use std::path::Path;

use render::presets::file::{self, group_of, DebugGroup, Section};
use validation::negative_control;

fn on_disk(section: &str) -> BTreeSet<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("presets")
        .join(section);
    std::fs::read_dir(dir)
        .expect("the preset directory")
        .map(|e| {
            let name = e
                .expect("an entry")
                .file_name()
                .into_string()
                .expect("utf-8");
            name.strip_suffix(".json")
                .expect("only .json files")
                .to_owned()
        })
        .collect()
}

fn embedded(section: Section) -> BTreeSet<String> {
    file::FILES
        .iter()
        .filter(|(s, _, _)| *s == section)
        .map(|(_, id, _)| (*id).to_owned())
        .collect()
}

fn check_directories(extra: Option<&str>) {
    let mut debug = on_disk("debug");
    if let Some(e) = extra {
        debug.insert(e.to_owned());
    }
    assert_eq!(
        embedded(Section::Debug),
        debug,
        "the debug files and the embedded table differ"
    );
    assert_eq!(
        embedded(Section::Production),
        on_disk("production"),
        "the production files and the embedded table differ"
    );
}

#[test]
fn preset_load_embeds_every_file() {
    check_directories(None);
}

negative_control!(
    preset_load_embeds_every_file,
    "a file in the directory and not in the table is refused",
    expected = "the debug files and the embedded table differ",
    check_directories(Some("not_embedded"))
);

fn check_metadata(files: &[file::File]) {
    for f in files {
        assert!(!f.name.is_empty(), "{} has no name", f.id);
        assert_eq!(
            f.group.is_some(),
            f.locked(),
            "{}: a debug preset names a group, a production one none",
            f.id
        );
        assert!(
            f.graph["nodes"].is_array() && f.graph["wires"].is_array(),
            "{}: no graph",
            f.id
        );
    }
}

#[test]
fn preset_load_metadata_and_graph_share_a_file() {
    check_metadata(&file::files().expect("the library reads"));
}

negative_control!(
    preset_load_metadata_and_graph_share_a_file,
    "a debug preset stripped of its group fails",
    expected = "a debug preset names a group, a production one none",
    {
        let mut files = file::files().expect("the library reads");
        for f in &mut files {
            f.group = None;
        }
        check_metadata(&files);
    }
);

fn check_parse(section: Section, text: &str) -> Result<file::File, String> {
    file::parse(section, "x", text)
}

#[test]
fn preset_load_refuses_a_group_in_the_wrong_section() {
    let graph = r#""graph": {"nodes": [], "wires": []}"#;
    let debug_without = format!(r#"{{"name": "a", {graph}}}"#);
    let production_with = format!(r#"{{"name": "a", "group": "validity", {graph}}}"#);
    assert!(check_parse(Section::Debug, &debug_without).is_err());
    assert!(check_parse(Section::Production, &production_with).is_err());
    assert!(check_parse(Section::Debug, &production_with).is_ok());
    assert!(check_parse(Section::Production, &debug_without).is_ok());
}

negative_control!(
    preset_load_refuses_a_group_in_the_wrong_section,
    "a debug preset with no group is accepted by a loader that does not check, so the test's refusal fails",
    expected = "debug preset `x` names no group",
    {
        let text = r#"{"name": "a", "graph": {"nodes": [], "wires": []}}"#;
        check_parse(Section::Debug, text).map(|_| ()).unwrap_or_else(|e| panic!("{e}"));
    }
);

/// The group of every debug preset of the library: files, then the Rust presets.
fn groups() -> Vec<(String, DebugGroup)> {
    let mut out: Vec<(String, DebugGroup)> = file::files()
        .expect("the library reads")
        .into_iter()
        .filter_map(|f| f.group.map(|g| (f.id.to_owned(), g)))
        .collect();
    out.extend(
        render::presets::library()
            .iter()
            .map(|p| (p.name().to_owned(), p.group())),
    );
    out
}

fn check_groups(groups: &[(String, DebugGroup)]) {
    for (id, g) in groups {
        let want = match id.as_str() {
            "uv_screen" | "uv_quad" | "coordinate_uv" | "coordinate_delta" | "tl_rb"
            | "centre_bg" | "index_grey" => DebugGroup::Addressing,
            other
                if ["DECODE", "Agreement", "ROUNDTRIP"]
                    .iter()
                    .any(|p| other.starts_with(p)) =>
            {
                DebugGroup::DecodedIc
            }
            other => panic!("preset `{other}` has no expected group in this test"),
        };
        assert_eq!(*g, want, "`{id}` sits in the wrong debug group");
    }
}

#[test]
fn debug_groups_hold() {
    let all = groups();
    assert!(all.iter().any(|(id, _)| id == "tl_rb"));
    check_groups(&all);
    for id in ["s_depth", "s_impurity", "s_state"] {
        assert_eq!(
            group_of(&format!("debug/{id}")),
            Some(DebugGroup::QuadStructure),
            "{id}"
        );
    }
    assert_eq!(group_of("debug/s_cache_age"), Some(DebugGroup::Profiling));
    assert_eq!(group_of("colour/outcome_state"), None);
}

negative_control!(
    debug_groups_hold,
    "a debug preset filed in another group fails",
    expected = "sits in the wrong debug group",
    {
        let mut all = groups();
        for (id, g) in &mut all {
            if id == "tl_rb" {
                *g = DebugGroup::Validity;
            }
        }
        check_groups(&all);
    }
);

/// The text of the WGSL function a preset file holds in its colour node.
fn colour_text(id: &str) -> String {
    let f = file::file(Section::Debug, id).expect("the preset");
    f.graph["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["kind"] == "colour")
        .and_then(|n| n["occupant"]["custom"].as_str())
        .expect("a custom colour node")
        .to_owned()
}

/// The channels a colour's `vec3` return leaves at the literal `0.0`, by position.
fn zero_channels(wgsl: &str) -> Vec<usize> {
    let ret = wgsl
        .split("return vec3<f32>(")
        .nth(1)
        .expect("a vec3 return");
    let args = ret.split(");").next().expect("closed");
    args.split(',')
        .enumerate()
        .filter(|(_, a)| a.trim() == "0.0")
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn addressing_channels_hold() {
    assert_eq!(
        zero_channels(&colour_text("tl_rb")),
        vec![1],
        "TL = RB leaves G at 0"
    );
    assert_eq!(
        zero_channels(&colour_text("centre_bg")),
        vec![0],
        "centre = BG leaves R at 0"
    );
}

negative_control!(
    addressing_channels_hold,
    "an addressing preset whose third channel carries data fails",
    expected = "TL = RB leaves G at 0",
    {
        let carrying = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x, ctx.inputs[0].y, ctx.inputs[0].y); }";
        assert_eq!(zero_channels(carrying), vec![1], "TL = RB leaves G at 0");
    }
);
