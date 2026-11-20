//! qa's tests for TASK-M7-13, the fragment registry, written from the requirements it closes and their source
//! (gui_state_contract §3; R-53; R-815), not from the implementation:
//!
//! - REQ-GEN-020: the registry is the scan of `shaders/wgsl/frag/{colour, brightness, combiner, post, debug}`, so a
//!   `.wgsl` file added to any of the five registers an occupant; a custom authored at run time joins the same way;
//!   the fixed compositor passes are not scanned.
//! - REQ-GEN-021: an entry is `{id, slot, source, category, uniformSchema, inputDomains}` keyed by directory, and an
//!   occupant is valid only in its slot.
//! - REQ-GEN-019: every entry carries `inputDomains`, an untransformed input recording "inherits"; on a wired node
//!   its domain is the manifest `[lo, hi]` of the wired field, and a field whose range is unbounded takes `(0, 1)`
//!   (R-815); a node that transforms its field declares its own.
//!
//! What each entry should hold is read here from the files themselves, by this file's own walk of the directories
//! and its own reading of the `// @uniform` and `// @input` lines (gui_state_contract §3, "The declaration format").
//! A field with exactly one finite end has no ruled domain (RQ-786, open; REQ-GEN-019 carries `rq: RQ-786`): it is
//! tested only for what the never-guess rule asks, that no domain is chosen for it (`plan/WORKFLOW.md` §
//! "Escalation"), neither `(0, 1)` nor its finite end standing in.
//!
//! Every failure panics "qa M7-13:", and each test registers its negative control (R-176).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use ledger::schema::{Bound, Range};
use render::assemble::Kind;
use render::registry::{self, Category, Entry};
use validation::negative_control;

// ── Fixtures ─────────────────────────────────────────────────────────────────────────────────────────────────────

const COLOUR: &str =
    "// @uniform gain: f32 = 1.0 [0.0, 4.0]\n// @input a [2.0, 5.0]\n// @input b\n\
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x * uniforms.gain); }\n";
const PLAIN_COLOUR: &str =
    "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }\n";
const BRIGHTNESS: &str = "fn brightness(ctx: Ctx) -> f32 { return 0.5; }\n";
const COMBINER: &str = "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb; }\n";
const POST: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb; }\n";

/// The scanned root and the compositor's, relative to a render crate (gui_state_contract §3's tree).
const FRAG: &str = "shaders/wgsl/frag";
const COMPOSITOR: &str = "shaders/wgsl/compositor";

/// A tree before anything is added: one combiner, and a fixed pass and a library file beside `frag/`.
const BASE: [(&str, &str); 3] = [
    ("shaders/wgsl/frag/combiner/base.wgsl", COMBINER),
    ("shaders/wgsl/compositor/blur.wgsl", POST),
    ("shaders/wgsl/lib/oklab.wgsl", POST),
];

/// One occupant in each of the five scanned directories: its file under `frag/`, its text, and the slot and the
/// category its directory gives it (the debug one a colour occupant, tagged debug).
const FIVE: [(&str, &str, Kind, Category); 5] = [
    ("colour/qa_c", COLOUR, Kind::Colour, Category::Slot),
    (
        "brightness/qa_b",
        BRIGHTNESS,
        Kind::Brightness,
        Category::Slot,
    ),
    ("combiner/qa_m", COMBINER, Kind::Combiner, Category::Slot),
    ("post/qa_p", POST, Kind::Post, Category::Slot),
    ("debug/qa_d", PLAIN_COLOUR, Kind::Colour, Category::Debug),
];

const KINDS: [Kind; 6] = [
    Kind::Source,
    Kind::Colour,
    Kind::Brightness,
    Kind::Combiner,
    Kind::Post,
    Kind::Out,
];

fn scratch(case: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_m7_13_{case}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
    std::fs::write(&path, text).expect("the file");
}

fn base(case: &str) -> PathBuf {
    let root = scratch(case);
    for (rel, text) in BASE {
        write(&root, rel, text);
    }
    root
}

/// The scan of a tree holding [`BASE`] and [`FIVE`].
fn five() -> Vec<Entry> {
    let root = base("five");
    for (id, text, _, _) in FIVE {
        write(&root, &format!("{FRAG}/{id}.wgsl"), text);
    }
    let scanned = registry::scan(&root);
    let _ = std::fs::remove_dir_all(&root);
    scanned.expect("qa M7-13: the scan of one occupant per directory")
}

fn ids(entries: &[Entry]) -> Vec<String> {
    entries.iter().map(|e| e.id.clone()).collect()
}

/// Every `.wgsl` file under `dir`, by this file's own walk.
fn wgsl_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(items) = std::fs::read_dir(dir) else {
        return;
    };
    for item in items {
        let path = item.expect("a directory item").path();
        if path.is_dir() {
            wgsl_under(&path, out);
        } else if path.extension().is_some_and(|x| x == "wgsl") {
            out.push(path);
        }
    }
}

fn render_crate() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The declarations a file's text makes, read here: the names of its `// @uniform` lines, and each `// @input` line's
/// name with its bracketed domain, if it has one.
#[allow(clippy::type_complexity)] // the two lists a declaration is, read once
fn declared(source: &str) -> (Vec<String>, Vec<(String, Option<(f64, f64)>)>) {
    let (mut uniforms, mut inputs) = (Vec::new(), Vec::new());
    for line in source.lines() {
        let Some(rest) = line.trim_start().strip_prefix("// @") else {
            continue;
        };
        if let Some(u) = rest.strip_prefix("uniform ") {
            let name = u.split(':').next().expect("a name").trim();
            uniforms.push(name.to_owned());
        } else if let Some(i) = rest.strip_prefix("input ") {
            let (name, domain) = match i.split_once('[') {
                None => (i.trim(), None),
                Some((name, bracket)) => {
                    let inner = bracket.trim().trim_end_matches(']');
                    let (lo, hi) = inner.split_once(',').expect("two ends");
                    let end = |s: &str| s.trim().parse::<f64>().expect("a number");
                    (name.trim(), Some((end(lo), end(hi))))
                }
            };
            inputs.push((name.to_owned(), domain));
        }
    }
    (uniforms, inputs)
}

/// The inputs an entry of `slot` whose text is `source` has (gui_state_contract §3): the ones it declares, and for a
/// colour or a brightness that declares none, the one input `field`, its domain inherited.
fn expected_inputs(slot: Kind, source: &str) -> Vec<(String, Option<(f64, f64)>)> {
    let (_, mut inputs) = declared(source);
    if inputs.is_empty() && matches!(slot, Kind::Colour | Kind::Brightness) {
        inputs.push(("field".to_owned(), None));
    }
    inputs
}

fn inputs_of(entry: &Entry) -> Vec<(String, Option<(f64, f64)>)> {
    entry
        .input_domains
        .iter()
        .map(|i| (i.name.clone(), i.domain))
        .collect()
}

// ── REQ-GEN-020: adding a file registers it ──────────────────────────────────────────────────────────────────────

/// Adds the file `rel` to [`BASE`] and asserts the scan lists it as `id`, an occupant of `slot` tagged `category`,
/// that it was not listed before, and that nothing else joined or left.
fn check_added(rel: &str, text: &str, id: &str, slot: Kind, category: Category) {
    let root = base("added");
    let before = registry::scan(&root);
    write(&root, rel, text);
    let after = registry::scan(&root);
    let _ = std::fs::remove_dir_all(&root);
    let before = before.expect("qa M7-13: the scan before the file is added");
    let after = after.unwrap_or_else(|e| panic!("qa M7-13: the scan after `{rel}` is added: {e}"));

    assert!(
        !ids(&before).iter().any(|i| i == id),
        "qa M7-13: `{id}` is listed before its file is added"
    );
    let entry = after.iter().find(|e| e.id == id).unwrap_or_else(|| {
        panic!(
            "qa M7-13: `{id}` is not listed after `{rel}` is added: {:?}",
            ids(&after)
        )
    });
    assert_eq!(entry.slot, slot, "qa M7-13: `{id}`'s slot");
    assert_eq!(entry.category, category, "qa M7-13: `{id}`'s category");
    assert_eq!(entry.source, text, "qa M7-13: `{id}`'s source");
    let mut expected = ids(&before);
    expected.push(id.to_owned());
    expected.sort_unstable();
    let mut got = ids(&after);
    got.sort_unstable();
    assert_eq!(
        got, expected,
        "qa M7-13: the registry after `{rel}` is added"
    );
}

#[test]
fn qa_registry_scan_each_directory_registers_an_added_file() {
    for (id, text, slot, category) in FIVE {
        check_added(&format!("{FRAG}/{id}.wgsl"), text, id, slot, category);
    }
}

negative_control!(
    qa_registry_scan_each_directory_registers_an_added_file,
    "a file added to compositor/, which is not scanned, must fail the check that an added file is listed",
    expected = "is not listed after",
    check_added(
        "shaders/wgsl/compositor/qa_p.wgsl",
        POST,
        "compositor/qa_p",
        Kind::Post,
        Category::Slot
    )
);

/// The six fields of an entry that do not say where its source came from.
fn content(e: &Entry) -> impl PartialEq + std::fmt::Debug + '_ {
    (
        &e.id,
        e.slot,
        &e.source,
        e.category,
        &e.uniform_schema,
        &e.input_domains,
    )
}

/// A custom registered at run time as `id` with the text `custom` gives the registry the entries a file of the text
/// `file` dropped at `id` gives the scan: the same ids in the same order, each entry's six fields equal.
fn check_custom(id: &str, file: &str, custom: &str) {
    let root = base("custom");
    let scanned = registry::scan(&root);
    write(&root, &format!("{FRAG}/{id}.wgsl"), file);
    let dropped = registry::scan(&root);
    let _ = std::fs::remove_dir_all(&root);
    let dropped = dropped.expect("qa M7-13: the scan with the file dropped");
    let mut registered = scanned.expect("qa M7-13: the scan before");

    let replaced = registry::register(&mut registered, id, custom)
        .unwrap_or_else(|e| panic!("qa M7-13: registering the custom `{id}`: {e}"));
    assert!(
        replaced.is_none(),
        "qa M7-13: `{id}` was already registered"
    );
    assert_eq!(
        ids(&registered),
        ids(&dropped),
        "qa M7-13: the custom `{id}` does not join as a dropped file does"
    );
    for (r, d) in registered.iter().zip(&dropped) {
        assert_eq!(
            format!("{:?}", content(r)),
            format!("{:?}", content(d)),
            "qa M7-13: the custom `{}` is not the entry its dropped file is",
            d.id
        );
        assert!(
            content(r) == content(d),
            "qa M7-13: the custom `{}` is not the entry its dropped file is",
            d.id
        );
    }
}

#[test]
fn qa_registry_scan_a_runtime_custom_joins_as_a_file_does() {
    for (id, text, _, _) in FIVE {
        check_custom(id, text, text);
    }
}

negative_control!(
    qa_registry_scan_a_runtime_custom_joins_as_a_file_does,
    "a custom whose text declares another domain than the dropped file's must fail the comparison",
    expected = "is not the entry its dropped file is",
    check_custom(
        "colour/qa_c",
        COLOUR,
        &COLOUR.replace("[2.0, 5.0]", "[2.0, 6.0]")
    )
);

/// No fixed pass under `render`'s `compositor/` is in `entries` (by id, by path or by text), and every `.wgsl` file
/// under its `frag/` is, with its text.
fn check_compositor_absent(render: &Path, entries: &[Entry], what: &str) {
    let mut passes = Vec::new();
    wgsl_under(&render.join(COMPOSITOR), &mut passes);
    assert!(
        !passes.is_empty(),
        "qa M7-13: no compositor pass under {} to look for",
        render.display()
    );
    for pass in &passes {
        let text = std::fs::read_to_string(pass).expect("the pass");
        let stem = pass.file_stem().expect("a stem").to_string_lossy();
        for e in entries {
            assert!(
                e.path != *pass && e.source != text && !e.id.starts_with("compositor"),
                "qa M7-13: the compositor pass `{stem}` is in {what} as `{}`",
                e.id
            );
        }
    }
    let frag = render.join(FRAG);
    let mut occupants = Vec::new();
    wgsl_under(&frag, &mut occupants);
    assert!(
        !occupants.is_empty(),
        "qa M7-13: no occupant under {}",
        frag.display()
    );
    for file in &occupants {
        let under = file.strip_prefix(&frag).expect("under frag/");
        let id = under
            .with_extension("")
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let entry = entries
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("qa M7-13: the file `{id}` under frag/ is not in {what}"));
        assert_eq!(
            entry.source,
            std::fs::read_to_string(file).expect("the occupant"),
            "qa M7-13: `{id}`'s source in {what} is not its file's text"
        );
    }
}

#[test]
fn qa_registry_scan_the_compositor_is_absent() {
    let render = render_crate();
    let shipped = registry::registry().expect("qa M7-13: the crate's registry");
    check_compositor_absent(&render, &shipped, "the crate's registry");
    let scanned = registry::scan(&render).expect("qa M7-13: the scan of the crate");
    check_compositor_absent(&render, &scanned, "the scan of the crate");
    // The registry the build embedded is the scan of the tree it was built from.
    assert_eq!(
        ids(&shipped),
        ids(&scanned),
        "qa M7-13: the crate's registry is not the scan of its tree"
    );
    for (s, d) in shipped.iter().zip(&scanned) {
        assert!(
            content(s) == content(d),
            "qa M7-13: the crate's registry entry `{}` is not the scanned one",
            d.id
        );
    }
}

negative_control!(
    qa_registry_scan_the_compositor_is_absent,
    "a registry holding a compositor pass's text must fail the check",
    expected = "qa M7-13: the compositor pass",
    {
        let render = render_crate();
        let mut entries = registry::registry().expect("the crate's registry");
        let mut passes = Vec::new();
        wgsl_under(&render.join(COMPOSITOR), &mut passes);
        let mut stray = entries[0].clone();
        stray.id = "post/qa_stray".to_owned();
        stray.source = std::fs::read_to_string(&passes[0]).expect("the pass");
        entries.push(stray);
        check_compositor_absent(&render, &entries, "the tampered registry")
    }
);

// ── REQ-GEN-021: keyed by directory, valid only in its slot ──────────────────────────────────────────────────────

/// The slot a slot directory holds (gui_state_contract §3's tree); `None` for `debug/`, whose occupants are colour
/// or brightness ones.
fn directory_slot(id: &str) -> Option<Kind> {
    match id.split('/').next() {
        Some("colour") => Some(Kind::Colour),
        Some("brightness") => Some(Kind::Brightness),
        Some("combiner") => Some(Kind::Combiner),
        Some("post") => Some(Kind::Post),
        _ => None,
    }
}

/// Each entry gives its occupant in exactly one slot and is refused in every other; an entry under a slot directory
/// in that directory's slot, a colour occupant so refused in the brightness slot.
fn check_slots(entries: &[Entry]) {
    assert!(!entries.is_empty(), "qa M7-13: no entry to check");
    for e in entries {
        let taken: Vec<Kind> = KINDS
            .into_iter()
            .filter(|&k| e.occupant(k).is_ok())
            .collect();
        match directory_slot(&e.id) {
            Some(own) => assert_eq!(
                taken,
                vec![own],
                "qa M7-13: `{}` is valid only in the {} slot, its directory's",
                e.id,
                own.name()
            ),
            None => assert_eq!(
                taken.len(),
                1,
                "qa M7-13: `{}` is valid in exactly one slot, not {taken:?}",
                e.id
            ),
        }
        if directory_slot(&e.id) == Some(Kind::Colour) {
            let refused = e.occupant(Kind::Brightness);
            assert!(
                refused.is_err(),
                "qa M7-13: the colour occupant `{}` is taken by the brightness slot",
                e.id
            );
        }
    }
}

#[test]
fn qa_registry_slots_an_occupant_is_valid_only_in_its_slot() {
    let ours = five();
    assert_eq!(ours.len(), FIVE.len() + 1, "qa M7-13: {:?}", ids(&ours));
    check_slots(&ours);
    check_slots(&registry::registry().expect("qa M7-13: the crate's registry"));
    // A custom registered at run time is held to its slot as a file is.
    let mut with_custom = Vec::new();
    registry::register(&mut with_custom, "colour/qa_custom", PLAIN_COLOUR)
        .expect("qa M7-13: registering a colour custom");
    check_slots(&with_custom);
}

negative_control!(
    qa_registry_slots_an_occupant_is_valid_only_in_its_slot,
    "a colour occupant that the brightness slot takes must fail the check",
    expected = "is valid only in the colour slot",
    {
        let mut entries = five();
        let colour = entries
            .iter_mut()
            .find(|e| e.id == "colour/qa_c")
            .expect("the colour");
        colour.slot = Kind::Brightness;
        check_slots(&entries)
    }
);

/// The file an entry of the crate's own registry was scanned from, found from its id: under `shaders/wgsl/frag/`, or
/// for a generated debug view under `frag/` (REQ-GEN-009).
fn file_of(render: &Path, id: &str) -> Option<PathBuf> {
    [FRAG, "frag"]
        .iter()
        .map(|root| render.join(root).join(format!("{id}.wgsl")))
        .find(|p| p.is_file())
}

/// Each entry carries the six fields its file gives it: its id unique; its slot its directory's; its source the
/// file's text (`text_of`); its category debug exactly under a `debug/` directory; its uniformSchema the file's
/// `// @uniform` lines, in order; its inputDomains the file's `// @input` lines.
fn check_six_fields(entries: &[Entry], text_of: impl Fn(&str) -> Option<String>) {
    assert!(!entries.is_empty(), "qa M7-13: no entry to check");
    let mut seen = ids(entries);
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), entries.len(), "qa M7-13: an id is listed twice");
    for e in entries {
        let id = &e.id;
        assert!(
            !id.is_empty() && !id.ends_with(".wgsl"),
            "qa M7-13: the id `{id}`"
        );
        let text = text_of(id).unwrap_or_else(|| panic!("qa M7-13: `{id}` has no file"));
        assert_eq!(e.source, text, "qa M7-13: `{id}`'s source");
        if let Some(own) = directory_slot(id) {
            assert_eq!(e.slot, own, "qa M7-13: `{id}`'s slot is its directory's");
        }
        let debug = id.split('/').next() == Some("debug");
        let category = if debug {
            Category::Debug
        } else {
            Category::Slot
        };
        assert_eq!(e.category, category, "qa M7-13: `{id}`'s category");
        let (uniforms, _) = declared(&text);
        let schema: Vec<String> = e.uniform_schema.iter().map(|u| u.name.clone()).collect();
        assert_eq!(schema, uniforms, "qa M7-13: `{id}`'s uniformSchema");
        assert_eq!(
            inputs_of(e),
            expected_inputs(e.slot, &text),
            "qa M7-13: `{id}`'s inputDomains"
        );
    }
}

fn five_text(id: &str) -> Option<String> {
    FIVE.iter()
        .find(|(i, ..)| *i == id)
        .map(|(_, text, ..)| (*text).to_owned())
        .or_else(|| (id == "combiner/base").then(|| COMBINER.to_owned()))
}

#[test]
fn qa_registry_slots_every_entry_carries_its_six_fields() {
    let ours = five();
    check_six_fields(&ours, five_text);
    // The values, for the one that declares each kind of line.
    let colour = ours
        .iter()
        .find(|e| e.id == "colour/qa_c")
        .expect("the colour");
    let gain = &colour.uniform_schema[0];
    assert_eq!(
        (gain.default.as_slice(), gain.range),
        (&[1.0][..], Some((0.0, 4.0))),
        "qa M7-13: the declared uniform's default and range"
    );

    let render = render_crate();
    let shipped = registry::registry().expect("qa M7-13: the crate's registry");
    check_six_fields(&shipped, |id| {
        file_of(&render, id).map(|p| std::fs::read_to_string(p).expect("the file"))
    });
    assert!(
        shipped.iter().any(|e| !e.uniform_schema.is_empty())
            && shipped.iter().any(|e| e.category == Category::Debug)
            && shipped.iter().any(|e| e.category == Category::Slot),
        "qa M7-13: the crate's registry has a uniform schema, a debug entry and a slot entry to check"
    );
}

negative_control!(
    qa_registry_slots_every_entry_carries_its_six_fields,
    "an entry that lost its uniformSchema must fail the check",
    expected = "uniformSchema",
    {
        let mut entries = five();
        let colour = entries
            .iter_mut()
            .find(|e| e.id == "colour/qa_c")
            .expect("the colour");
        colour.uniform_schema.clear();
        check_six_fields(&entries, five_text)
    }
);

// ── REQ-GEN-019: inputDomains ────────────────────────────────────────────────────────────────────────────────────

/// Every entry has its inputDomains: one per input its file declares, a declared domain recorded as declared and an
/// untransformed input recording "inherits"; a colour or a brightness has at least one input.
fn check_input_domains(entries: &[Entry]) {
    assert!(!entries.is_empty(), "qa M7-13: no entry to check");
    for e in entries {
        let expected = expected_inputs(e.slot, &e.source);
        if matches!(e.slot, Kind::Colour | Kind::Brightness) {
            assert!(
                !e.input_domains.is_empty(),
                "qa M7-13: `{}` has no inputDomains",
                e.id
            );
        }
        for (got, want) in inputs_of(e).iter().zip(&expected) {
            assert_eq!(
                got.1, want.1,
                "qa M7-13: `{}`'s input `{}` records {:?}, and its file declares {:?} (None is inherits)",
                e.id, want.0, got.1, want.1
            );
        }
        assert_eq!(
            inputs_of(e),
            expected,
            "qa M7-13: `{}`'s inputDomains",
            e.id
        );
    }
}

#[test]
fn qa_registry_input_domains_every_entry_has_them() {
    let ours = five();
    check_input_domains(&ours);
    let colour = ours
        .iter()
        .find(|e| e.id == "colour/qa_c")
        .expect("the colour");
    assert_eq!(
        inputs_of(colour),
        vec![("a".to_owned(), Some((2.0, 5.0))), ("b".to_owned(), None)],
        "qa M7-13: a declared domain, and an untransformed input recording inherits"
    );
    let brightness = ours
        .iter()
        .find(|e| e.id == "brightness/qa_b")
        .expect("the brightness");
    assert_eq!(
        inputs_of(brightness),
        vec![("field".to_owned(), None)],
        "qa M7-13: an occupant declaring no input has `field`, inherited"
    );
    let shipped = registry::registry().expect("qa M7-13: the crate's registry");
    check_input_domains(&shipped);
    assert!(
        shipped
            .iter()
            .any(|e| matches!(e.slot, Kind::Colour | Kind::Brightness)),
        "qa M7-13: the crate's registry has a colour or a brightness to check"
    );
}

negative_control!(
    qa_registry_input_domains_every_entry_has_them,
    "an untransformed input that records a domain, where it should record inherits, must fail the check",
    expected = "(None is inherits)",
    {
        let mut entries = five();
        let colour = entries
            .iter_mut()
            .find(|e| e.id == "colour/qa_c")
            .expect("the colour");
        colour.input_domains[1].domain = Some((0.0, 1.0));
        check_input_domains(&entries)
    }
);

/// A range's `[lo, hi]` where both its ends are finite numbers.
fn both_ends(range: Range) -> Option<(f64, f64)> {
    let end = |b: Bound| match b {
        Bound::Closed(v) | Bound::Open(v) if v.is_finite() => Some(v),
        _ => None,
    };
    end(range.lo).zip(end(range.hi))
}

/// `resolved(input, range)` is the domain of the colour [`COLOUR`]'s input `input` (0 declares `[2, 5]`, 1 is
/// untransformed) when a field of manifest range `range` is wired to it. Over every field of the manifest: the
/// untransformed input has the field's `[lo, hi]` where the range has both, and `(0, 1)` where it is unbounded
/// (R-815); the declared one keeps its own. A field with one finite end is passed over.
fn check_wired(resolved: impl Fn(usize, Range) -> Option<(f64, f64)>) {
    let fields = ledger::gen::validate(&ledger::layout()).expect("qa M7-13: the ledger validates");
    let mut wires: Vec<(String, Range)> = fields
        .iter()
        .map(|f| (f.name.to_string(), f.range))
        .collect();
    wires.push(("a hand-made [0, 65535]".to_owned(), Range::int(0, 65535)));
    wires.push((
        "a hand-made [-2.5, 7.25]".to_owned(),
        Range {
            lo: Bound::Closed(-2.5),
            hi: Bound::Closed(7.25),
        },
    ));
    wires.push(("a hand-made unbounded".to_owned(), Range::UNBOUNDED));
    let (mut bounded, mut unbounded) = (0, 0);
    for (name, range) in &wires {
        assert_eq!(
            resolved(0, *range),
            Some((2.0, 5.0)),
            "qa M7-13: a declared domain is the node's own, wired to `{name}`"
        );
        let expected = match both_ends(*range) {
            Some(ends) => {
                bounded += 1;
                ends
            }
            None if *range == Range::UNBOUNDED => {
                unbounded += 1;
                (0.0, 1.0)
            }
            None => continue,
        };
        assert_eq!(
            resolved(1, *range),
            Some(expected),
            "qa M7-13: an untransformed input wired to `{name}`, range {range:?}"
        );
    }
    assert!(
        bounded > 3 && unbounded > 1,
        "qa M7-13: the manifest gave {bounded} bounded and {unbounded} unbounded fields beyond the hand-made ones"
    );
}

#[test]
fn qa_registry_input_domains_a_wired_input_takes_its_fields_range() {
    let ours = five();
    let colour = ours
        .iter()
        .find(|e| e.id == "colour/qa_c")
        .expect("the colour");
    check_wired(|input, range| colour.domain(input, range));
    // The same of a custom registered at run time.
    let mut customs = Vec::new();
    registry::register(&mut customs, "colour/qa_custom", COLOUR)
        .expect("qa M7-13: registering the custom");
    check_wired(|input, range| customs[0].domain(input, range));
}

negative_control!(
    qa_registry_input_domains_a_wired_input_takes_its_fields_range,
    "an untransformed input given (0, 1) whatever is wired, the field's range ignored, must fail the check",
    expected = "qa M7-13: an untransformed input wired to",
    check_wired(|input, _| Some(if input == 0 { (2.0, 5.0) } else { (0.0, 1.0) }))
);

// ── REQ-GEN-019, RQ-786: a field with one finite end is given no domain ─────────────────────────────────────────

/// Whether a range has exactly one finite end, an end at `±∞` (closed or open) or `Unbounded` counted as none.
fn one_finite_end(range: Range) -> bool {
    let finite = |b: Bound| matches!(b, Bound::Closed(v) | Bound::Open(v) if v.is_finite());
    finite(range.lo) != finite(range.hi)
}

/// The wires to a field with one finite end: every such field of the manifest (render_gui_spec §10.1 names `K_0`
/// `[0, ∞)` and `V_0` `(−∞, 0]`), and hand-made ones with the missing end written each way, on each side.
fn one_finite_end_wires() -> Vec<(String, Range)> {
    let fields = ledger::gen::validate(&ledger::layout()).expect("qa M7-13: the ledger validates");
    let mut wires: Vec<(String, Range)> = fields
        .iter()
        .filter(|f| one_finite_end(f.range))
        .map(|f| (f.name.to_string(), f.range))
        .collect();
    assert!(
        !wires.is_empty(),
        "qa M7-13: the manifest has no field with one finite end to wire"
    );
    let r = |lo, hi| Range { lo, hi };
    for (what, lo, hi) in [
        ("[0, unbounded)", Bound::Closed(0.0), Bound::Unbounded),
        ("(unbounded, 0]", Bound::Unbounded, Bound::Closed(0.0)),
        (
            "(1.5, +inf]",
            Bound::Open(1.5),
            Bound::Closed(f64::INFINITY),
        ),
        (
            "[-4, +inf)",
            Bound::Closed(-4.0),
            Bound::Open(f64::INFINITY),
        ),
        (
            "[-inf, 2)",
            Bound::Closed(f64::NEG_INFINITY),
            Bound::Open(2.0),
        ),
        (
            "(-inf, -2]",
            Bound::Open(f64::NEG_INFINITY),
            Bound::Closed(-2.0),
        ),
    ] {
        wires.push((format!("a hand-made {what}"), r(lo, hi)));
    }
    wires
}

/// `entries`' inputs, each wired to every field with one finite end: `domain(entry, input, range)` is the domain the
/// registry gives it. An untransformed input gets none; a declared one keeps its own (R-53: a node that transforms
/// its field declares its domain, whatever is wired).
fn check_unruled(entries: &[Entry], domain: impl Fn(&Entry, usize, Range) -> Option<(f64, f64)>) {
    let wires = one_finite_end_wires();
    let mut inherited = 0;
    for e in entries {
        for (i, input) in e.input_domains.iter().enumerate() {
            for (name, range) in &wires {
                let got = domain(e, i, *range);
                match input.domain {
                    None => {
                        inherited += 1;
                        assert_eq!(
                            got, None,
                            "qa M7-13: `{}`'s untransformed input `{}` wired to `{name}`, range {range:?}, was given \
                             a domain, and RQ-786 rules none",
                            e.id, input.name
                        );
                    }
                    Some(own) => assert_eq!(
                        got,
                        Some(own),
                        "qa M7-13: `{}`'s declared input `{}` wired to `{name}` lost its own domain",
                        e.id,
                        input.name
                    ),
                }
            }
        }
    }
    assert!(inherited > 0, "qa M7-13: no untransformed input was wired");
}

/// The entries the unruled arm is checked on: one per scanned slot directory, a custom registered at run time, and
/// the crate's own registry.
fn unruled_entries() -> Vec<Entry> {
    let mut entries = five();
    registry::register(&mut entries, "colour/qa_custom", COLOUR)
        .expect("qa M7-13: registering the custom");
    entries.extend(registry::registry().expect("qa M7-13: the crate's registry"));
    entries
}

#[test]
fn qa_registry_input_domains_one_finite_end_is_given_none() {
    let entries = unruled_entries();
    check_unruled(&entries, |e, i, range| e.domain(i, range));
    // The same through `resolve`, which says why: no domain is defined for that range.
    check_unruled(&entries, |e, i, range| {
        match registry::resolve(e.input_domains[i].domain, range) {
            Ok(d) => Some(d),
            Err(registry::Unruled(named)) => {
                assert_eq!(
                    named, range,
                    "qa M7-13: the refusal names another range than the wired one"
                );
                None
            }
        }
    });
}

negative_control!(
    qa_registry_input_domains_one_finite_end_is_given_none,
    "an untransformed input given (0, 1) on a field with one finite end, a default chosen for RQ-786, must fail",
    expected = "and RQ-786 rules none",
    check_unruled(&unruled_entries(), |e, i, range| {
        e.domain(i, range).or(Some(registry::UNIT_DOMAIN))
    })
);
