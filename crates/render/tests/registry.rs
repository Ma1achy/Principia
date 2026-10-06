//! The occupant registry, the scanned filesystem (gui_state_contract §3; TASK-M1-08): ledger codegen's generated
//! field views surface through the same scan as the hand-written occupants, each a debug entry of the `colour` slot
//! (REQ-GEN-009; RQ-219); the scan's slots, debug tag and declarations; and each generated view baked into a stain
//! through the debug-view entry point (TASK-M1-05), loading only its own field (R-378). Each test has a registered
//! negative control (R-176).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use ledger::gen::catalogue::{self, Accessor};
use render::assemble::{self, Kind, Stain, Tier, MAX_INPUTS};
use render::debug_bake::{DebugViews, ViewGenerator};
use render::registry::{self, Catalogue, Category, RegistryError};
use validation::negative_control;

/// A fresh scratch directory named after `case`.
fn scratch(case: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "registry_{case}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// Writes `files` (each a path relative to `root` and its text) under `root`.
fn write(root: &Path, files: &[(&str, &str)]) {
    for (rel, text) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        std::fs::write(&path, text).expect("the file");
    }
}

/// The catalogue's fields, from the ledger.
fn catalogue_fields() -> Vec<&'static str> {
    let l = ledger::layout();
    let entries = ledger::gen::validate(&l).expect("the ledger validates");
    catalogue::views(&l.words, &entries)
        .into_iter()
        .map(|v| v.field)
        .collect()
}

// ── REQ-GEN-009: the generated views surface through the scan ────────────────────────────────────────────────────

/// Runs ledger codegen's catalogue emitter into a scratch workspace, removes the view of each of `removed`, scans the
/// render crate there, and asserts each of the catalogue's views is a debug registry entry of the `colour` slot; then
/// that the crate's own registry holds them the same way.
fn check_generated_views_in_registry(removed: &[&str]) {
    let root = scratch("codegen");
    ledger::gen::run(&ledger::layout(), &[catalogue::emit], &root).expect("ledger codegen runs");
    for field in removed {
        std::fs::remove_file(root.join(format!("{}/{field}.wgsl", catalogue::DIR)))
            .expect("the view was generated");
    }
    let scanned = registry::scan(&root.join("crates/render"));
    let _ = std::fs::remove_dir_all(&root);
    let fields = catalogue_fields();
    assert!(
        fields.len() >= 20,
        "the catalogue has {} views",
        fields.len()
    );
    for (what, entries) in [
        ("the codegen output", scanned.expect("the scan")),
        (
            "the crate",
            registry::registry().expect("the crate's registry"),
        ),
    ] {
        for field in &fields {
            let id = format!("debug/generated/{field}");
            let entry = entries.iter().find(|e| e.id == id);
            let Some(entry) = entry else {
                panic!("`{field}`'s view has no registry entry in {what}");
            };
            assert_eq!(
                entry.category,
                Category::Debug,
                "`{id}` is not tagged debug"
            );
            assert_eq!(entry.slot, Kind::Colour, "`{id}`'s slot is not colour");
            assert!(
                entry.uniform_schema.is_empty() && entry.input_domains.len() == 1,
                "`{id}` declares {:?} and {:?}",
                entry.uniform_schema,
                entry.input_domains
            );
        }
    }
}

#[test]
fn generated_views_in_registry() {
    check_generated_views_in_registry(&[]);
}

negative_control!(
    generated_views_in_registry,
    "a view codegen wrote and that is then removed has no entry",
    expected = "`state`'s view has no registry entry",
    check_generated_views_in_registry(&["state"])
);

// ── The scan: slots, the debug tag, declarations ─────────────────────────────────────────────────────────────────

const COLOUR: &str = "// @uniform gain: f32 = 1.0 [0.0, 4.0]\n// @input t [0.0, 1.0]\n\
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x * uniforms.gain); }\n";
const BRIGHTNESS: &str = "fn brightness(ctx: Ctx) -> f32 { return 0.5; }\n";
const COMBINER: &str = "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb; }\n";
const POST: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb; }\n";
/// A debug occupant defining `brightness` and naming `colour` only in a comment.
const DEBUG_BRIGHTNESS: &str =
    "// not fn colour(ctx: Ctx)\nfn helper() -> f32 { return 0.25; }\nfn brightness(ctx: Ctx) -> f32 { return helper(); }\n";

/// The scan of a render crate holding one occupant of each slot directory and two debug occupants, one hand-written
/// and one generated, gives each its id, slot, category and declarations.
fn check_scan_slots(files: &[(&str, &str)]) {
    let root = scratch("slots");
    write(&root, files);
    let scanned = registry::scan(&root);
    let _ = std::fs::remove_dir_all(&root);
    let entries = scanned.unwrap_or_else(|e| panic!("the scan failed: {e}"));
    let got: Vec<(&str, Kind, Category)> = entries
        .iter()
        .map(|e| (e.id.as_str(), e.slot, e.category))
        .collect();
    assert_eq!(
        got,
        [
            ("brightness/b", Kind::Brightness, Category::Slot),
            ("colour/c", Kind::Colour, Category::Slot),
            ("combiner/m", Kind::Combiner, Category::Slot),
            ("debug/generated/v", Kind::Colour, Category::Debug),
            ("debug/heat", Kind::Brightness, Category::Debug),
            ("post/p", Kind::Post, Category::Slot),
        ],
        "the scanned entries"
    );
    let colour = &entries[1];
    assert_eq!(colour.uniform_schema.len(), 1, "colour/c's uniformSchema");
    assert_eq!(colour.uniform_schema[0].name, "gain");
    assert_eq!(colour.input_domains.len(), 1, "colour/c's inputDomains");
    assert_eq!(colour.input_domains[0].domain, Some((0.0, 1.0)));
    assert_eq!(colour.source, COLOUR, "colour/c's source");
}

fn slot_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("shaders/wgsl/frag/colour/c.wgsl", COLOUR),
        ("shaders/wgsl/frag/brightness/b.wgsl", BRIGHTNESS),
        ("shaders/wgsl/frag/combiner/m.wgsl", COMBINER),
        ("shaders/wgsl/frag/post/p.wgsl", POST),
        ("shaders/wgsl/frag/debug/heat.wgsl", DEBUG_BRIGHTNESS),
        (
            "frag/debug/generated/v.wgsl",
            "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.0); }\n",
        ),
        ("shaders/wgsl/frag/colour/notes.txt", "not a shader"),
    ]
}

#[test]
fn registry_scan_slots_and_debug_tag() {
    check_scan_slots(&slot_files());
}

negative_control!(
    registry_scan_slots_and_debug_tag,
    "a hand-written debug occupant moved into the post directory is scanned as a post, not debug",
    expected = "the scanned entries",
    check_scan_slots(
        &slot_files()
            .into_iter()
            .map(|(p, t)| if p.ends_with("heat.wgsl") {
                ("shaders/wgsl/frag/post/heat.wgsl", POST)
            } else {
                (p, t)
            })
            .collect::<Vec<_>>()
    )
);

/// The scan of a render crate holding `file` alone is refused, the error naming the file and saying `why`.
fn check_refused(file: (&str, &str), why: &str) {
    let root = scratch("refused");
    write(&root, &[file]);
    let scanned = registry::scan(&root);
    let _ = std::fs::remove_dir_all(&root);
    let error = scanned.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        error.contains(file.0) && error.contains(why),
        "`{}` was not refused for {why:?}: {error:?}",
        file.0
    );
}

#[test]
fn registry_scan_refuses_a_file_under_no_slot() {
    check_refused(
        ("shaders/wgsl/frag/stray.wgsl", COLOUR),
        "not under a slot directory",
    );
    check_refused(
        ("shaders/wgsl/frag/source/s.wgsl", COLOUR),
        "not under a slot directory",
    );
}

negative_control!(
    registry_scan_refuses_a_file_under_no_slot,
    "a file in a slot directory is scanned",
    expected = "was not refused",
    check_refused(
        ("shaders/wgsl/frag/colour/s.wgsl", COLOUR),
        "not under a slot directory"
    )
);

#[test]
fn registry_scan_refuses_a_debug_file_without_one_slot() {
    let two = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.0); }\n\
               fn brightness(ctx: Ctx) -> f32 { return 0.0; }\n";
    check_refused(("shaders/wgsl/frag/debug/two.wgsl", two), "defines 2");
    check_refused(
        (
            "frag/debug/generated/none.wgsl",
            "fn helper() -> f32 { return 0.0; }\n",
        ),
        "defines 0",
    );
}

negative_control!(
    registry_scan_refuses_a_debug_file_without_one_slot,
    "a debug file defining exactly one slot function is scanned",
    expected = "was not refused",
    check_refused(
        ("shaders/wgsl/frag/debug/one.wgsl", BRIGHTNESS),
        "defines 1"
    )
);

#[test]
fn registry_scan_refuses_a_malformed_declaration() {
    check_refused(
        ("shaders/wgsl/frag/colour/bad.wgsl", "// @unifrom gain: f32 = 1.0\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.0); }\n"),
        "declaration, line 1",
    );
}

negative_control!(
    registry_scan_refuses_a_malformed_declaration,
    "a well-formed declaration is scanned",
    expected = "was not refused",
    check_refused(
        ("shaders/wgsl/frag/colour/good.wgsl", COLOUR),
        "declaration, line 1"
    )
);

#[test]
fn registry_scan_errors_name_the_file() {
    let path = PathBuf::from("x.wgsl");
    let io = RegistryError::Io(path.clone(), "gone".into()).to_string();
    assert_eq!(io, "x.wgsl: gone");
    let slots = RegistryError::DebugSlot(path, vec![Kind::Colour, Kind::Post]).to_string();
    assert!(
        slots.starts_with("x.wgsl: a debug occupant defines exactly one"),
        "{slots}"
    );
    assert!(slots.ends_with("this one defines 2"), "{slots}");
    assert!(
        registry::scan(Path::new("/nonexistent/render")).is_ok_and(|e| e.is_empty()),
        "a crate with neither directory has no entries"
    );
}

negative_control!(
    registry_scan_errors_name_the_file,
    "an I/O error's text is the path and the reason",
    expected = "assertion",
    assert_eq!(
        RegistryError::Io(PathBuf::from("y.wgsl"), "gone".into()).to_string(),
        "x.wgsl: gone"
    )
);

// ── The generated views baked through the debug-view entry point ─────────────────────────────────────────────────

/// The field set each of `catalogue`'s views must load: the read-side members its accessors name, the word as
/// `word.w`, the one component its `.w` accessor reads (R-378).
fn expected_fields(field: &str) -> Vec<String> {
    let l = ledger::layout();
    let entries = ledger::gen::validate(&l).expect("the ledger validates");
    let view = catalogue::views(&l.words, &entries)
        .into_iter()
        .find(|v| v.field == field)
        .expect("the catalogue has the view");
    view.read
        .accessors(field)
        .into_iter()
        .filter_map(|a| match a {
            Accessor::Member(m) if m == "word" => Some("word.w".to_owned()),
            Accessor::Member(m) => Some(m),
            Accessor::Function(_) => None,
        })
        .collect()
}

/// Each view of `catalogue` bakes through the debug-view entry point into a stain that assembles at the full tier and
/// loads exactly its own field.
fn check_views_bake(catalogue: Catalogue) {
    let fields = catalogue_fields();
    assert_eq!(
        catalogue.fields().len(),
        fields.len(),
        "the catalogue's views"
    );
    let mut views = DebugViews::new(catalogue);
    for field in fields {
        let (stain, keys) = views
            .bake(field)
            .unwrap_or_else(|e| panic!("`{field}`'s view does not bake: {e}"));
        assert_eq!(keys.len(), 4, "`{field}`'s view has 4 nodes");
        let fragment = assemble::assemble(stain, Tier::FULL)
            .unwrap_or_else(|e| panic!("`{field}`'s view does not assemble: {e}"));
        assert_eq!(
            fragment.fields,
            expected_fields(field),
            "`{field}`'s view loads other fields than its own"
        );
    }
    assert_eq!(views.baked(), catalogue_fields().len());
}

#[test]
fn generated_views_bake_through_the_entry_point() {
    let registry = registry::registry().expect("the crate's registry");
    check_views_bake(Catalogue::new(&registry));
}

negative_control!(
    generated_views_bake_through_the_entry_point,
    "a view that also reads `theta` loads another field",
    expected = "loads other fields than its own",
    check_views_bake(Catalogue::new(
        &registry::registry()
            .expect("the crate's registry")
            .into_iter()
            .map(|mut e| {
                e.source = e.source.replace(
                    "return ",
                    "let other = ctx.sample.theta;\n    return other * 0.0 + ",
                );
                e
            })
            .collect::<Vec<_>>()
    ))
);

/// A field with no view has none at the entry point, and a view's stain is the zero source, the view's colour, the
/// pass-through combiner and `OUT`.
fn check_view_shape(catalogue: &Catalogue) {
    assert_eq!(catalogue.view("no_such_field"), None, "a view of no field");
    let nodes = catalogue.view("state").expect("`state` has a view");
    let kinds: Vec<Kind> = nodes.iter().map(|n| n.kind).collect();
    assert_eq!(
        kinds,
        [Kind::Source, Kind::Colour, Kind::Combiner, Kind::Out],
        "the view's stain"
    );
    Stain::new(nodes).expect("the view is a stain");
}

#[test]
fn generated_views_stain_shape() {
    check_view_shape(&Catalogue::new(
        &registry::registry().expect("the registry"),
    ));
}

negative_control!(
    generated_views_stain_shape,
    "a catalogue with no views has no `state` view",
    expected = "`state` has a view",
    check_view_shape(&Catalogue::new(&[]))
);

/// The stain's context holds `inputs` of `n` fields.
fn check_context_inputs(n: usize) {
    assert!(
        assemble::CONTEXT.contains(&format!("inputs: array<Field, {n}>,")),
        "the stain's context does not hold {n} inputs"
    );
}

#[test]
fn stain_context_holds_max_inputs() {
    check_context_inputs(MAX_INPUTS);
}

negative_control!(
    stain_context_holds_max_inputs,
    "one more input than MAX_INPUTS",
    expected = "does not hold",
    check_context_inputs(MAX_INPUTS + 1)
);
