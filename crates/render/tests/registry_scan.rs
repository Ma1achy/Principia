//! The fragment registry (TASK-M7-13; gui_state_contract §3), on a temporary `frag/` tree and on the crate's own:
//! - REQ-GEN-020: a `.wgsl` file dropped into `frag/colour/` is listed by the rebuilt registry, in the build's table
//!   and in the scan, with no code edited; a custom registered at run time appears the same way; the compositor's
//!   fixed passes are not scanned; the crate's registry is the build's embedded scan and reads no file (R-815, RQ-677
//!   (e));
//! - REQ-GEN-021: an occupant is valid only in its slot, a colour occupant refused in the brightness slot, and every
//!   entry carries all six fields;
//! - REQ-GEN-019: every entry has `inputDomains`, an untransformed input recording "inherits"; resolved on a wire, its
//!   domain is the manifest `[lo, hi]` of the wired field, and `(0, 1)` for a field with no finite `[lo, hi]` (R-815).
//!
//! Each test has a registered negative control (R-176).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use ledger::schema::{Bound, Range};
use render::assemble::{Declaration, Kind, Occupant, MAX_INPUTS};
use render::registry::{self, Category, Entry, RegistryError};
use validation::negative_control;

/// A fresh scratch directory named after `case`.
fn scratch(case: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "registry_scan_{case}_{}_{}",
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

const COLOUR: &str =
    "// @uniform gain: f32 = 1.0 [0.0, 4.0]\n// @input t [2.0, 5.0]\n// @input u\n\
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x * uniforms.gain); }\n";
const PLAIN_COLOUR: &str =
    "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }\n";
const BRIGHTNESS: &str = "fn brightness(ctx: Ctx) -> f32 { return 0.5; }\n";
const COMBINER: &str = "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb; }\n";
const POST: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb; }\n";

/// A render crate's tree before anything is dropped into it: one combiner, one generated view, one generated
/// reduction, and the fixed passes and the library beside `frag/`.
const TREE: [(&str, &str); 6] = [
    ("shaders/wgsl/frag/combiner/multiply.wgsl", COMBINER),
    ("frag/debug/generated/state.wgsl", PLAIN_COLOUR),
    ("frag/debug/reductions/rho_dircos.wgsl", PLAIN_COLOUR),
    ("shaders/wgsl/compositor/blur.wgsl", POST),
    ("shaders/wgsl/lib/oklab.wgsl", POST),
    ("shaders/wgsl/frag/colour/notes.txt", PLAIN_COLOUR),
];

/// The ids the scan gives [`TREE`].
const TREE_IDS: [&str; 3] = [
    "combiner/multiply",
    "debug/generated/state",
    "debug/reductions/rho_dircos",
];

fn ids(entries: &[Entry]) -> Vec<&str> {
    entries.iter().map(|e| e.id.as_str()).collect()
}

// ── REQ-GEN-020: adding a file registers it ──────────────────────────────────────────────────────────────────────

/// Drops the file `dropped` into [`TREE`] and asserts the rebuilt registry lists it as `id`, an occupant of `slot`:
/// in the files the build's scan finds, in the table the build embeds, and in the scan's entries; and that none of
/// them listed it before.
fn check_dropped(dropped: (&str, &str), id: &str, slot: Kind) {
    let root = scratch("dropped");
    write(&root, &TREE);
    let before = registry::scan(&root).expect("the scan before");
    assert_eq!(ids(&before), TREE_IDS, "the tree before the drop");
    assert!(
        !registry::shipped_table(&registry::occupant_files(&root).expect("the files before"))
            .contains(id),
        "`{id}` is in the table before it is dropped"
    );

    write(&root, &[dropped]);
    let found = registry::occupant_files(&root).expect("the files after");
    let table = registry::shipped_table(&found);
    let after = registry::scan(&root);
    let _ = std::fs::remove_dir_all(&root);
    let after = after.expect("the scan after");

    let file = found.iter().find(|f| f.id == id);
    let Some(file) = file else {
        panic!(
            "`{id}` is not listed by the build's scan: {:?}",
            found.iter().map(|f| &f.id).collect::<Vec<_>>()
        );
    };
    assert_eq!(file.rel, dropped.0, "`{id}`'s path in the crate");
    assert_eq!(file.path, root.join(dropped.0), "`{id}`'s file");
    let line = format!(
        "    ({id:?}, {:?}, include_str!({:?})),\n",
        dropped.0,
        file.path.to_string_lossy()
    );
    assert!(
        table.contains(&line),
        "the table has no line {line:?}:\n{table}"
    );
    assert!(
        table.starts_with("/// ")
            && table.contains("\npub const SHIPPED: &[(&str, &str, &str)] = &[\n"),
        "the table's declaration:\n{table}"
    );
    assert!(table.ends_with("),\n];\n"), "the table's end:\n{table}");
    assert_eq!(
        table.matches("include_str!").count(),
        TREE_IDS.len() + 1,
        "one line per file:\n{table}"
    );

    let mut expected: Vec<&str> = TREE_IDS.to_vec();
    expected.push(id);
    expected.sort_unstable();
    assert_eq!(ids(&after), expected, "the registry after the drop");
    assert_eq!(
        found.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
        expected,
        "the build's scan, sorted by id"
    );
    let entry = after.iter().find(|e| e.id == id).expect("listed");
    assert_eq!(entry.slot, slot, "`{id}`'s slot");
    assert_eq!(entry.source, dropped.1, "`{id}`'s source");
    assert_eq!(entry.path, root.join(dropped.0), "`{id}`'s path");
}

#[test]
fn registry_scan_lists_a_dropped_file() {
    check_dropped(
        ("shaders/wgsl/frag/colour/new.wgsl", COLOUR),
        "colour/new",
        Kind::Colour,
    );
    check_dropped(
        ("shaders/wgsl/frag/brightness/deep/lit.wgsl", BRIGHTNESS),
        "brightness/deep/lit",
        Kind::Brightness,
    );
    check_dropped(
        ("frag/debug/generated/d_min.wgsl", PLAIN_COLOUR),
        "debug/generated/d_min",
        Kind::Colour,
    );
    check_dropped(
        ("frag/debug/reductions/lambda_dircos.wgsl", PLAIN_COLOUR),
        "debug/reductions/lambda_dircos",
        Kind::Colour,
    );
}

negative_control!(
    registry_scan_lists_a_dropped_file,
    "a file dropped beside frag/, among the fixed passes, is not listed",
    expected = "is not listed by the build's scan",
    check_dropped(
        ("shaders/wgsl/compositor/new.wgsl", COLOUR),
        "colour/new",
        Kind::Colour
    )
);

/// The crate's registry is the build's embedded scan: `entries`, the registry, are the entries of a scan of the
/// crate's tree now, so the table is not stale; and they are made from the table alone, the same from a crate
/// directory that does not exist, but for the path each names.
fn check_embedded(entries: &[Entry]) {
    let render = Path::new(env!("CARGO_MANIFEST_DIR"));
    let scanned = registry::scan(render).expect("the scan of the crate");
    assert!(
        scanned.len() >= 20,
        "the crate has {} entries",
        scanned.len()
    );
    assert_eq!(entries.len(), scanned.len(), "the embedded entries' count");
    for (e, s) in entries.iter().zip(&scanned) {
        assert_eq!(e, s, "`{}` is embedded as scanned", s.id);
    }
    assert_eq!(registry::SHIPPED.len(), scanned.len(), "the table's length");

    let absent = Path::new("/nonexistent/render");
    assert!(!absent.exists());
    let moved = registry::embedded(absent, registry::SHIPPED).expect("the table alone");
    assert_eq!(moved.len(), scanned.len(), "the moved entries' count");
    for ((m, s), &(_, rel, _)) in moved.iter().zip(&scanned).zip(registry::SHIPPED) {
        assert_eq!(
            m.path,
            absent.join(rel),
            "`{}`'s path names its crate",
            s.id
        );
        let mut at_home = m.clone();
        at_home.path = s.path.clone();
        assert_eq!(&at_home, s, "`{}` from the table alone", s.id);
    }
}

#[test]
fn registry_scan_is_embedded_at_build_time() {
    check_embedded(&registry::registry().expect("the crate's registry"));
}

negative_control!(
    registry_scan_is_embedded_at_build_time,
    "a registry lacking one of the crate's files is not the scan",
    expected = "the embedded entries' count",
    check_embedded(&registry::registry().expect("the crate's registry")[1..])
);

/// Registers the custom `source` as `id` at run time in the registry of [`TREE`], and asserts it appears as the same
/// file dropped into the tree does: the same entry but for its path, at its place in id order.
fn check_custom(id: &str, source: &str) {
    let root = scratch("custom");
    write(&root, &TREE);
    let mut registered = registry::scan(&root).expect("the scan");
    let got = registry::register(&mut registered, id, source);
    let file = format!("shaders/wgsl/frag/{id}.wgsl");
    write(&root, &[(file.as_str(), source)]);
    let dropped = registry::scan(&root);
    let _ = std::fs::remove_dir_all(&root);
    let dropped = dropped.expect("the scan with the file dropped");

    assert_eq!(got, Ok(None), "`{id}` was not registered as new");
    assert_eq!(ids(&registered), ids(&dropped), "the ids, in order");
    for (r, d) in registered.iter().zip(&dropped) {
        let mut at_file = r.clone();
        at_file.path = d.path.clone();
        assert_eq!(&at_file, d, "`{}` registered as dropped", d.id);
    }
    let entry = registered.iter().find(|e| e.id == id).expect("registered");
    assert_eq!(entry.path, PathBuf::from(format!("{id}.wgsl")));
}

#[test]
fn registry_scan_registers_a_runtime_custom() {
    // Before, between and after the tree's ids.
    check_custom("colour/custom", COLOUR);
    check_custom("brightness/custom", BRIGHTNESS);
    check_custom("post/custom", POST);

    // Registering at an id again replaces the entry and returns the one it replaced.
    let mut entries = Vec::new();
    assert_eq!(
        registry::register(&mut entries, "colour/custom", PLAIN_COLOUR),
        Ok(None)
    );
    let first = entries[0].clone();
    assert_eq!(
        registry::register(&mut entries, "colour/custom", COLOUR),
        Ok(Some(first))
    );
    assert_eq!(ids(&entries), ["colour/custom"]);
    assert_eq!(entries[0].source, COLOUR);

    // A custom is refused as a scanned file is, and a refusal registers nothing.
    let before = entries.clone();
    assert_eq!(
        registry::register(&mut entries, "custom", COLOUR),
        Err(RegistryError::NoSlot(PathBuf::from("custom.wgsl")))
    );
    assert!(matches!(
        registry::register(&mut entries, "colour/bad", "// @unifrom x: f32 = 1.0\n"),
        Err(RegistryError::Occupant(path, _)) if path == Path::new("colour/bad.wgsl")
    ));
    assert_eq!(entries, before);
}

negative_control!(
    registry_scan_registers_a_runtime_custom,
    "a custom registered under another directory is not the file dropped",
    expected = "the ids, in order",
    {
        let root = scratch("custom_control");
        write(&root, &TREE);
        let mut registered = registry::scan(&root).expect("the scan");
        registry::register(&mut registered, "post/custom", COLOUR).expect("registers");
        write(&root, &[("shaders/wgsl/frag/colour/custom.wgsl", COLOUR)]);
        let dropped = registry::scan(&root).expect("the scan");
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(ids(&registered), ids(&dropped), "the ids, in order");
    }
);

/// The `.wgsl` files under `dir`, recursively, as paths.
fn wgsl_under(dir: &Path, out: &mut Vec<PathBuf>) {
    for item in std::fs::read_dir(dir).expect("the directory") {
        let path = item.expect("an item").path();
        if path.is_dir() {
            wgsl_under(&path, out);
        } else if path.extension().is_some_and(|x| x == "wgsl") {
            out.push(path);
        }
    }
}

/// None of `entries` is a file of the crate's `outside` directory, which holds `.wgsl` files.
fn check_not_scanned(entries: &[Entry], outside: &str) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(outside);
    let mut files = Vec::new();
    wgsl_under(&dir, &mut files);
    assert!(!files.is_empty(), "`{outside}` holds no .wgsl file");
    for e in entries {
        assert!(
            !e.path.starts_with(&dir),
            "`{}`, a file of `{outside}`, is scanned",
            e.id
        );
    }
    for file in files {
        let text = std::fs::read_to_string(&file).expect("the file");
        assert!(
            registry::SHIPPED
                .iter()
                .all(|&(_, _, source)| source != text),
            "{} is embedded",
            file.display()
        );
    }
}

#[test]
fn registry_scan_leaves_the_compositor_out() {
    // The temporary tree: the fixed passes, the library and a file that is not WGSL are no entries.
    let root = scratch("compositor");
    write(&root, &TREE);
    let scanned = registry::scan(&root);
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(ids(&scanned.expect("the scan")), TREE_IDS);
    // The crate: its fixed passes are in no entry and in no line of the embedded table.
    let entries = registry::registry().expect("the crate's registry");
    check_not_scanned(&entries, "shaders/wgsl/compositor");
    check_not_scanned(&entries, "shaders/wgsl/lib");
    assert!(entries.iter().all(|e| !e.id.starts_with("compositor")));
}

negative_control!(
    registry_scan_leaves_the_compositor_out,
    "the occupants' own directory is scanned",
    expected = "is scanned",
    check_not_scanned(
        &registry::registry().expect("the crate's registry"),
        "shaders/wgsl/frag"
    )
);

// ── REQ-GEN-021: six fields, and an occupant valid only in its slot ──────────────────────────────────────────────

const KINDS: [Kind; 6] = [
    Kind::Source,
    Kind::Colour,
    Kind::Brightness,
    Kind::Combiner,
    Kind::Post,
    Kind::Out,
];

/// Each of `entries` gives its occupant, its WGSL, in each slot of `own(entry)` and is refused in every other, the
/// refusal naming the entry, its slot and the slot asked.
fn check_slots(entries: &[Entry], own: impl Fn(&Entry) -> Kind) {
    for e in entries {
        for kind in KINDS {
            let got = e.occupant(kind);
            if kind == own(e) {
                assert_eq!(
                    got,
                    Ok(Occupant::Custom(e.source.clone())),
                    "`{}` in its own slot, {}",
                    e.id,
                    kind.name()
                );
            } else {
                assert_eq!(
                    got,
                    Err(RegistryError::WrongSlot(e.id.clone(), e.slot, kind)),
                    "`{}` was not refused in the {} slot",
                    e.id,
                    kind.name()
                );
            }
        }
    }
}

/// One occupant of each slot directory.
fn one_of_each() -> Vec<Entry> {
    let mut entries = Vec::new();
    for (id, source) in [
        ("colour/c", COLOUR),
        ("brightness/b", BRIGHTNESS),
        ("combiner/k", COMBINER),
        ("post/p", POST),
        ("debug/b", BRIGHTNESS),
    ] {
        registry::register(&mut entries, id, source).expect("registers");
    }
    entries
}

#[test]
fn registry_slots_refuse_another_slot() {
    let entries = one_of_each();
    let slots: Vec<Kind> = entries.iter().map(|e| e.slot).collect();
    assert_eq!(
        slots,
        [
            Kind::Brightness,
            Kind::Colour,
            Kind::Combiner,
            Kind::Brightness,
            Kind::Post
        ],
        "each directory's slot"
    );
    check_slots(&entries, |e| e.slot);
    check_slots(&registry::registry().expect("the crate's registry"), |e| {
        e.slot
    });

    // A colour occupant in the brightness slot.
    let colour = entries
        .iter()
        .find(|e| e.id == "colour/c")
        .expect("the colour");
    let refused = colour.occupant(Kind::Brightness).expect_err("refused");
    assert_eq!(
        refused,
        RegistryError::WrongSlot("colour/c".into(), Kind::Colour, Kind::Brightness)
    );
    assert_eq!(
        refused.to_string(),
        "`colour/c` is a colour occupant, valid only in its slot: the brightness slot does not take it"
    );
}

negative_control!(
    registry_slots_refuse_another_slot,
    "a colour occupant is not taken by the brightness slot",
    expected = "was not refused in the colour slot",
    check_slots(&one_of_each(), |e| match e.slot {
        Kind::Colour => Kind::Brightness,
        slot => slot,
    })
);

/// Every one of `entries` carries all six fields: an id, unique, that names its directory; one of the four slots; its
/// source, defining its slot's function; its category, debug under a `debug/` directory and slot elsewhere; and the
/// `uniformSchema` and `inputDomains` its source declares.
fn check_six_fields(entries: &[Entry]) {
    assert!(!entries.is_empty(), "no entries");
    let mut seen = std::collections::BTreeSet::new();
    for e in entries {
        assert!(seen.insert(&e.id), "`{}` is registered twice", e.id);
        let (dirs, name) = e.id.rsplit_once('/').expect("an id names its directory");
        assert!(!name.is_empty(), "`{}` has no name", e.id);
        let debug = dirs.split('/').any(|d| d == "debug");
        assert_eq!(
            e.category,
            if debug {
                Category::Debug
            } else {
                Category::Slot
            },
            "`{}`'s category",
            e.id
        );
        let by_directory = match dirs.split('/').next() {
            Some("colour") => Some(Kind::Colour),
            Some("brightness") => Some(Kind::Brightness),
            Some("combiner") => Some(Kind::Combiner),
            Some("post") => Some(Kind::Post),
            _ => None,
        };
        assert!(
            debug || by_directory == Some(e.slot),
            "`{}`'s slot is not its directory's",
            e.id
        );
        assert!(
            matches!(
                e.slot,
                Kind::Colour | Kind::Brightness | Kind::Combiner | Kind::Post
            ),
            "`{}`'s slot is no slot",
            e.id
        );
        assert!(
            registry::slot_functions(&e.source).is_ok_and(|f| f.contains(&e.slot)),
            "`{}`'s source does not define its slot's function",
            e.id
        );
        let declared = Declaration::parse(&e.source).expect("the declarations");
        assert_eq!(
            e.uniform_schema, declared.uniforms,
            "`{}`'s uniformSchema",
            e.id
        );
        if !declared.inputs.is_empty() {
            assert_eq!(
                e.input_domains, declared.inputs,
                "`{}`'s inputDomains",
                e.id
            );
        }
        assert!(!e.path.as_os_str().is_empty(), "`{}` has no path", e.id);
    }
}

#[test]
fn registry_slots_every_entry_has_six_fields() {
    let entries = registry::registry().expect("the crate's registry");
    check_six_fields(&entries);
    for slot in [Kind::Colour, Kind::Combiner, Kind::Post] {
        assert!(
            entries.iter().any(|e| e.slot == slot),
            "the crate has no {} occupant",
            slot.name()
        );
    }
    assert!(entries.iter().any(|e| e.category == Category::Debug));
    assert!(entries.iter().any(|e| e.category == Category::Slot));
    assert!(entries.iter().any(|e| !e.uniform_schema.is_empty()));
    check_six_fields(&one_of_each());
}

negative_control!(
    registry_slots_every_entry_has_six_fields,
    "an entry whose source is gone does not carry its fields",
    expected = "source does not define its slot's function",
    {
        let mut entries = one_of_each();
        entries[0].source.clear();
        check_six_fields(&entries)
    }
);

// ── REQ-GEN-019: inputDomains ────────────────────────────────────────────────────────────────────────────────────

/// Every one of `entries` has its `inputDomains`: a colour one to [`MAX_INPUTS`] inputs, a brightness one, a combiner
/// none and a post up to [`MAX_INPUTS`]; each input named, its domain the bracket its `// @input` line declares and,
/// with no bracket, "inherits".
fn check_input_domains(entries: &[Entry]) {
    for e in entries {
        let n = e.input_domains.len();
        let counted = match e.slot {
            Kind::Colour => (1..=MAX_INPUTS).contains(&n),
            Kind::Brightness => n == 1,
            Kind::Combiner => n == 0,
            Kind::Post => n <= MAX_INPUTS,
            Kind::Source | Kind::Out => false,
        };
        assert!(counted, "`{}`, a {}, has {n} inputs", e.id, e.slot.name());
        let lines: Vec<&str> = e
            .source
            .lines()
            .filter_map(|l| l.trim_start().strip_prefix("// @input"))
            .collect();
        if lines.is_empty() {
            assert!(
                e.input_domains
                    .iter()
                    .all(|i| i.name == "field" && i.domain.is_none()),
                "`{}` declares no input, and its default one does not inherit: {:?}",
                e.id,
                e.input_domains
            );
            continue;
        }
        assert_eq!(lines.len(), n, "`{}`'s inputs", e.id);
        for (line, input) in lines.iter().zip(&e.input_domains) {
            assert!(!input.name.is_empty(), "`{}` has an unnamed input", e.id);
            assert_eq!(
                input.domain.is_some(),
                line.contains('['),
                "`{}`'s input `{}` records {:?}, declared `{line}`",
                e.id,
                input.name,
                input.domain
            );
        }
    }
}

#[test]
fn registry_input_domains_inherit_by_default() {
    let entries = registry::registry().expect("the crate's registry");
    check_input_domains(&entries);
    assert!(
        entries
            .iter()
            .any(|e| e.input_domains.iter().any(|i| i.domain.is_none())),
        "no entry of the crate inherits"
    );
    let ours = one_of_each();
    check_input_domains(&ours);
    let colour = ours
        .iter()
        .find(|e| e.id == "colour/c")
        .expect("the colour");
    let domains: Vec<(&str, Option<(f64, f64)>)> = colour
        .input_domains
        .iter()
        .map(|i| (i.name.as_str(), i.domain))
        .collect();
    assert_eq!(domains, [("t", Some((2.0, 5.0))), ("u", None)]);
}

negative_control!(
    registry_input_domains_inherit_by_default,
    "an entry whose inputDomains are gone does not carry them",
    expected = "has 0 inputs",
    {
        let mut entries = one_of_each();
        entries[0].input_domains.clear();
        check_input_domains(&entries)
    }
);

/// The manifest `[lo, hi]` of `range`, read off its bounds: both ends where each is a finite number, else none.
fn manifest(range: Range) -> Option<(f64, f64)> {
    let end = |b: Bound| match b {
        Bound::Closed(v) | Bound::Open(v) => Some(v).filter(|v| v.is_finite()),
        Bound::Unbounded => None,
    };
    end(range.lo).zip(end(range.hi))
}

/// The entry `node`, wired to each of the manifest's fields in turn, `resolved(node, input, range)` its domain there:
/// its untransformed input `inherits` has the field's manifest `[lo, hi]`, `(0, 1)` where the field has none, and its
/// input `declares` keeps the domain `own` whatever is wired. The manifest has fields of each kind.
fn check_wired(
    node: &Entry,
    (inherits, declares, own): (usize, usize, (f64, f64)),
    resolved: impl Fn(&Entry, usize, Range) -> Option<(f64, f64)>,
) {
    assert_eq!(
        node.input_domains[inherits].domain, None,
        "records inherits"
    );
    assert_eq!(node.input_domains[declares].domain, Some(own));
    let fields = ledger::gen::validate(&ledger::layout()).expect("the ledger validates");
    let (mut bounded, mut unbounded, mut half) = (0, 0, 0);
    for field in &fields {
        let expected = match manifest(field.range) {
            Some(range) => {
                bounded += 1;
                range
            }
            None => {
                if field.range == Range::UNBOUNDED {
                    unbounded += 1;
                } else {
                    half += 1;
                }
                (0.0, 1.0)
            }
        };
        assert_eq!(
            resolved(node, inherits, field.range),
            Some(expected),
            "an untransformed input wired to `{}`, whose manifest range is {}",
            field.name,
            field.range
        );
        assert_eq!(
            resolved(node, declares, field.range),
            Some(own),
            "a declared domain, wired to `{}`",
            field.name
        );
    }
    assert!(
        bounded > 0 && unbounded > 0 && half > 0,
        "the manifest's fields: {bounded} bounded, {unbounded} unbounded, {half} with one end"
    );
}

#[test]
fn registry_input_domains_resolve_per_wire() {
    let ours = one_of_each();
    let colour = ours
        .iter()
        .find(|e| e.id == "colour/c")
        .expect("the colour");
    check_wired(colour, (1, 0, (2.0, 5.0)), Entry::domain);
    assert_eq!(colour.domain(2, Range::int(0, 9)), None, "no third input");

    // The ends of the range, each kind of bound.
    let range = |lo, hi| Range { lo, hi };
    let (open, closed, none) = (Bound::Open, Bound::Closed, Bound::Unbounded);
    for (wired, expected) in [
        (Range::int(0, 65535), (0.0, 65535.0)),
        (range(closed(-1.5), open(4.0)), (-1.5, 4.0)),
        (range(open(0.0), closed(2.0)), (0.0, 2.0)),
        (Range::UNBOUNDED, (0.0, 1.0)),
        (range(closed(3.0), none), (0.0, 1.0)),
        (range(none, closed(-3.0)), (0.0, 1.0)),
        (range(closed(3.0), closed(f64::INFINITY)), (0.0, 1.0)),
        (range(open(f64::NEG_INFINITY), closed(7.0)), (0.0, 1.0)),
    ] {
        assert_eq!(
            registry::resolve(None, wired),
            expected,
            "inherited from {wired}"
        );
        assert_eq!(
            colour.domain(1, wired),
            Some(expected),
            "input `u` on {wired}"
        );
        assert_eq!(
            registry::resolve(Some((8.0, 9.0)), wired),
            (8.0, 9.0),
            "declared, on {wired}"
        );
    }
    assert_eq!(registry::UNIT_DOMAIN, (0.0, 1.0));

    // A generated view of the crate, untransformed: its one input inherits whatever field is wired.
    let entries = registry::registry().expect("the crate's registry");
    let view = entries
        .iter()
        .find(|e| e.id == "debug/generated/state")
        .expect("the state view");
    assert_eq!(view.domain(0, Range::int(0, 7)), Some((0.0, 7.0)));
    assert_eq!(view.domain(0, Range::UNBOUNDED), Some((0.0, 1.0)));
}

negative_control!(
    registry_input_domains_resolve_per_wire,
    "an input that resolves to the unit interval on every wire does not inherit its field's range",
    expected = "an untransformed input wired to",
    check_wired(
        one_of_each()
            .iter()
            .find(|e| e.id == "colour/c")
            .expect("the colour"),
        (1, 0, (2.0, 5.0)),
        |node, input, _| node.domain(input, Range::UNBOUNDED)
    )
);
