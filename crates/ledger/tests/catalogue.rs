//! The debug catalogue and the host export decoder, generated from the layout table beside Rust pack/unpack and the
//! WGSL unpack, one source for all four artefacts (dd_generation_root §1, §4 seam 13; render contract Part 5, Part 6;
//! TASK-M1-08):
//! - REQ-GEN-010: mutating one ledger entry changes all four artefacts together, and the generated-file guard finds a
//!   checked-in catalogue or decoder that is not the emitters' output, or a stale view (`one_source_four_artefacts`).
//! - REQ-GEN-011: a field added with metadata appears in the catalogue and the decoder; without metadata, or with no
//!   read the fragment has, generation fails naming it (`new_field_in_catalogue`).
//! - REQ-TOOL-017: each view's WGSL and its test reference the same generated accessor symbols, the WGSL by naga's IR
//!   (`shader_and_test_share_accessor`).
//! - REQ-TOOL-020: the catalogue's field list is the ledger's, `ledger::layout().entries`, on disk too
//!   (`catalogue_equals_ledger`; RQ-218).
//!
//! Each test has a registered negative control (R-176).

use std::collections::BTreeSet;
use std::path::Path;

use ledger::gen::catalogue::{self, Accessor, View};
use ledger::gen::{self, export, read, rust, wgsl, GenError, Generated};
use ledger::schema::{Bound, EntryBuilder, FieldType, Ledger, Location, Range, Scale, Span};
use naga::{Expression, Module, Statement, TypeInner};
use validation::negative_control;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn checked_in(path: &str) -> String {
    let path = root().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn generate(ledger: &Ledger) -> Vec<Generated> {
    gen::generate(ledger, gen::EMITTERS).unwrap_or_else(|e| panic!("{e}"))
}

/// The catalogue's views of `ledger`.
fn views_of(ledger: &Ledger) -> Vec<View> {
    let entries = gen::validate(ledger).unwrap_or_else(|e| panic!("{e}"));
    catalogue::views(&ledger.words, &entries)
}

// ── REQ-GEN-010: one source, four artefacts ──────────────────────────────────────────────────────────────────────

/// The four artefact families, each by the paths of its generated files: Rust pack/unpack (and the Rust read side),
/// the host export decoder, the WGSL unpack (and the WGSL read side), the catalogue (its views and their tests).
fn family(path: &Path) -> Option<&'static str> {
    let p = path.to_string_lossy();
    if p == rust::PATH || p == read::RUST_PATH {
        Some("the Rust pack/unpack")
    } else if p == export::PATH {
        Some("the export decoder")
    } else if p == wgsl::PATH || p == read::WGSL_PATH {
        Some("the WGSL unpack")
    } else if p.starts_with(catalogue::DIR) || p == catalogue::TESTS_PATH {
        Some("the catalogue")
    } else {
        None
    }
}

/// Each family's files, concatenated in emission order.
fn families(files: &[Generated]) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::new();
    for f in files {
        let Some(name) = family(&f.path) else {
            continue;
        };
        let text = format!("{}\n{}", f.path.display(), f.contents);
        match out.iter_mut().find(|(n, _)| *n == name) {
            Some((_, all)) => all.push_str(&text),
            None => out.push((name, text)),
        }
    }
    out
}

/// The payload ledger with its packed field `saturated` renamed `sticky`, or unchanged.
fn renamed(rename: bool) -> Ledger {
    let mut l = ledger::layout();
    if rename {
        let e = l
            .entries
            .iter_mut()
            .find(|e| e.name == Some("saturated"))
            .expect("the ledger has `saturated`");
        e.name = Some("sticky");
    }
    l
}

/// Generating from `mutated` changes each of the four artefact families the payload ledger generates.
fn check_four_change(mutated: &Ledger) {
    let before = families(&generate(&ledger::layout()));
    let after = families(&generate(mutated));
    assert_eq!(
        before.len(),
        4,
        "the emitters write {} families",
        before.len()
    );
    for (name, text) in &before {
        let other = after.iter().find(|(n, _)| n == name).map(|(_, t)| t);
        assert!(
            other.is_some_and(|t| t != text),
            "{name} did not change with the ledger entry"
        );
    }
}

#[test]
fn one_source_four_artefacts_change_together() {
    check_four_change(&renamed(true));
}

negative_control!(
    one_source_four_artefacts_change_together,
    "an unchanged ledger changes no artefact",
    expected = "did not change with the ledger entry",
    check_four_change(&renamed(false))
);

/// The generated-file guard over the catalogue and the export decoder: each file the emitters write is `on_disk`'s,
/// and the views on disk are the emitted ones, no stale view left (`listing`, the `.wgsl` files in the views'
/// directory).
fn check_guard(on_disk: &dyn Fn(&str) -> String, listing: &[String]) {
    let files = generate(&ledger::layout());
    let mut views = Vec::new();
    for f in &files {
        let path = f.path.to_string_lossy().into_owned();
        if family(&f.path).is_none_or(|n| n != "the catalogue" && n != "the export decoder") {
            continue;
        }
        if path.starts_with(catalogue::DIR) {
            views.push(path.clone());
        }
        assert!(
            on_disk(&path) == f.contents,
            "the checked-in {path} is not the emitter's output: run `cargo xtask codegen`"
        );
    }
    views.sort();
    assert_eq!(
        listing, views,
        "the views on disk are not the emitted ones: a stale view is left, or one is missing"
    );
}

/// The `.wgsl` files in the views' directory, as paths relative to the root, sorted.
fn listing() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(root().join(catalogue::DIR))
        .expect("the views' directory")
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| n.ends_with(".wgsl"))
        .map(|n| format!("{}/{n}", catalogue::DIR))
        .collect();
    out.sort();
    out
}

#[test]
fn one_source_four_artefacts_guard_detects_an_edit() {
    check_guard(&|p| checked_in(p), &listing());
}

negative_control!(
    one_source_four_artefacts_guard_detects_an_edit,
    "a checked-in view with its ramp edited is not the emitter's output",
    expected = "is not the emitter's output",
    check_guard(
        &|p| checked_in(p).replace("dbg_cat(", "dbg_lin("),
        &listing()
    )
);

/// The guard refuses the checked-in files with a view of `stale` left beside them.
fn check_guard_refuses_stale(stale: Option<&str>) {
    let mut views = listing();
    views.extend(stale.map(|f| format!("{}/{f}.wgsl", catalogue::DIR)));
    views.sort();
    let guard = std::panic::AssertUnwindSafe(|| check_guard(&|p| checked_in(p), &views));
    assert!(
        std::panic::catch_unwind(guard).is_err(),
        "the guard passed the views on disk with {stale:?} left"
    );
}

#[test]
fn one_source_four_artefacts_guard_detects_a_stale_view() {
    check_guard_refuses_stale(Some("removed_field"));
}

negative_control!(
    one_source_four_artefacts_guard_detects_a_stale_view,
    "with no stale view the guard passes",
    expected = "the guard passed the views on disk",
    check_guard_refuses_stale(None)
);

// ── REQ-GEN-011: a new field appears in the catalogue, or generation fails ──────────────────────────────────────

/// The payload ledger with `probe_bit`, a flag in `packed_a`'s bit 10, its reserved span narrowed to bits 11–15; its
/// entry built by `entry`.
fn with_probe_bit(entry: impl FnOnce(EntryBuilder) -> EntryBuilder) -> Ledger {
    let mut l = ledger::layout();
    let a = l
        .words
        .iter_mut()
        .find(|w| w.name == "packed_a")
        .expect("packed_a");
    a.reserved = vec![Span {
        offset: 11,
        width: 5,
    }];
    let builder = EntryBuilder::new("probe_bit")
        .location(Location::Packed {
            word: "packed_a",
            offset: 10,
            width: 1,
        })
        .ty(FieldType::UBits)
        .scale(Scale::Flag)
        .range(Range::int(0, 1))
        .provenance(ledger::schema::Provenance::Kernel)
        .consumers(&[
            ledger::schema::Consumer::Render,
            ledger::schema::Consumer::Export,
            ledger::schema::Consumer::Debug,
        ]);
    l.entries.push(entry(builder));
    l
}

/// Generating from `ledger` writes a view of `probe_bit`, its test and its export decoding.
fn check_new_field_viewed(ledger: &Ledger) {
    let files = generate(ledger);
    let view = format!("{}/probe_bit.wgsl", catalogue::DIR);
    let text = |path: &str| {
        files
            .iter()
            .find(|f| f.path.to_string_lossy() == path)
            .map(|f| f.contents.clone())
            .unwrap_or_default()
    };
    assert!(
        text(&view).contains("dbg_flag(ctx.sample.probe_bit)"),
        "the catalogue has no view of the new field `probe_bit`"
    );
    assert!(
        text(catalogue::TESTS_PATH).contains("fn catalogue_view_probe_bit()"),
        "the new field's view has no test"
    );
    assert!(
        text(export::PATH).contains("pub probe_bit: bool,"),
        "the export decoder does not decode the new field"
    );
}

#[test]
fn new_field_in_catalogue_with_metadata_appears() {
    check_new_field_viewed(&with_probe_bit(|e| e));
}

negative_control!(
    new_field_in_catalogue_with_metadata_appears,
    "the payload ledger without the new field has no view of it",
    expected = "has no view of the new field",
    check_new_field_viewed(&ledger::layout())
);

/// Generation from `ledger` is refused, naming `field` and `why`.
fn check_refused(ledger: &Ledger, field: &str, why: &str) {
    let message = match gen::generate(ledger, gen::EMITTERS) {
        Ok(_) => panic!("generation was not refused"),
        Err(e) => e.to_string(),
    };
    assert!(
        message.contains(&format!("`{field}`")) && message.contains(why),
        "generation was refused without naming `{field}` and {why:?}: {message}"
    );
}

#[test]
fn new_field_in_catalogue_without_metadata_fails_naming_it() {
    check_refused(
        &with_probe_bit(|e| e.without("scale")),
        "probe_bit",
        "missing `scale`",
    );
}

negative_control!(
    new_field_in_catalogue_without_metadata_fails_naming_it,
    "a new field with its metadata complete generates",
    expected = "generation was not refused",
    check_refused(&with_probe_bit(|e| e), "probe_bit", "missing `scale`")
);

/// The payload ledger with `t_end`, dd_generation_root §3.8's derived field, which the read side does not compute, so
/// the fragment has no read of it; or unchanged.
fn with_t_end(add: bool) -> Ledger {
    let mut l = ledger::layout();
    if add {
        l.entries.push(
            EntryBuilder::new("t_end")
                .location(Location::Derived {
                    from: vec!["t_end_step"],
                })
                .ty(FieldType::F32)
                .scale(Scale::Lin)
                .range(Range::int(0, 1))
                .provenance(ledger::schema::Provenance::Kernel)
                .consumers(&[ledger::schema::Consumer::Debug]),
        );
    }
    l
}

#[test]
fn new_field_in_catalogue_unread_fails_naming_it() {
    check_refused(&with_t_end(true), "t_end", "has no debug view");
}

negative_control!(
    new_field_in_catalogue_unread_fails_naming_it,
    "the payload ledger, every field read, generates",
    expected = "generation was not refused",
    check_refused(&with_t_end(false), "t_end", "has no debug view")
);

#[test]
fn new_field_in_catalogue_refusal_is_its_own_error() {
    let l = with_t_end(true);
    let entries = gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
    let refused = catalogue::refused(&l.words, &entries);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert_eq!(
        export::refused(&l.words, &entries).len(),
        1,
        "the export decoder has no read of `t_end` either"
    );
    assert!(
        matches!(gen::generate(&l, gen::EMITTERS), Err(GenError::Unread(lines)) if lines.len() == 2),
        "both refusals are reported as unread fields"
    );
    assert!(
        gen::generate(&l, &[rust::emit, wgsl::emit]).is_ok(),
        "the struct emitters alone need no read of a derived field"
    );
}

negative_control!(
    new_field_in_catalogue_refusal_is_its_own_error,
    "the payload ledger refuses nothing",
    expected = "assertion",
    {
        let l = ledger::layout();
        let entries = gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(catalogue::refused(&l.words, &entries).len(), 1);
    }
);

// ── The placeholder ramp, the probe and the header at their boundaries ───────────────────────────────────────────

/// The payload ledger with `name`, a field of `width` bits at `packed_a`'s bit 10, its reserved span narrowed to the
/// bits above it to 15, of `scale` over `range`.
fn with_probe(name: &'static str, width: u32, scale: Scale, range: Range) -> Ledger {
    let mut l = with_probe_bit(|_| {
        EntryBuilder::new(name)
            .location(Location::Packed {
                word: "packed_a",
                offset: 10,
                width,
            })
            .ty(FieldType::UBits)
            .scale(scale)
            .range(range)
            .provenance(ledger::schema::Provenance::Kernel)
            .consumers(&[
                ledger::schema::Consumer::Render,
                ledger::schema::Consumer::Export,
                ledger::schema::Consumer::Debug,
            ])
    });
    let a = l
        .words
        .iter_mut()
        .find(|w| w.name == "packed_a")
        .expect("packed_a");
    a.reserved = vec![Span {
        offset: 10 + width,
        width: 6 - width,
    }];
    l
}

/// The file at `path` generated from `ledger`.
fn emitted(ledger: &Ledger, path: &str) -> String {
    generate(ledger)
        .into_iter()
        .find(|f| f.path.to_string_lossy() == path)
        .map(|f| f.contents)
        .unwrap_or_else(|| panic!("nothing was generated at {path}"))
}

/// The view of `probe_bit` generated from `ledger` colours it with `ramp`.
fn check_ramp(ledger: &Ledger, ramp: &str) {
    let view = emitted(ledger, &format!("{}/probe_bit.wgsl", catalogue::DIR));
    assert!(
        view.contains(&format!("return {ramp};")),
        "the view of `probe_bit` is not coloured with {ramp}:\n{view}"
    );
}

#[test]
fn view_ramp_of_a_wider_flag_compares_with_zero() {
    let two_bits = with_probe("probe_bit", 2, Scale::Flag, Range::int(0, 1));
    check_ramp(&two_bits, "dbg_flag(ctx.sample.probe_bit != 0u)");
}

negative_control!(
    view_ramp_of_a_wider_flag_compares_with_zero,
    "a one-bit flag is read as a `bool`, and is not compared with zero",
    expected = "is not coloured with",
    check_ramp(
        &with_probe("probe_bit", 1, Scale::Flag, Range::int(0, 1)),
        "dbg_flag(ctx.sample.probe_bit != 0u)"
    )
);

/// A two-bit categorical probe of four classes with the stored sentinel `sentinel`.
fn categorical_with(sentinel: f64) -> Ledger {
    let mut l = with_probe("probe_bit", 2, Scale::Categorical(4), Range::int(0, 3));
    l.entries
        .iter_mut()
        .find(|e| e.name == Some("probe_bit"))
        .expect("the probe")
        .sentinel = Some(sentinel);
    l
}

/// A categorical view shows a stored sentinel as its literal value (R-136) only when a u-bits field can store it: a
/// finite, non-negative value; an infinite or negative one leaves the classes alone (TASK-M1-09).
#[test]
fn view_ramp_of_a_categorical_sentinel_is_literal_when_storable() {
    check_ramp(
        &categorical_with(2.0),
        "select(dbg_cat(ctx.sample.probe_bit, 4u), dbg_sentinel(f32(ctx.sample.probe_bit), ctx.frag_xy), \
         ctx.sample.probe_bit == 2u)",
    );
    check_ramp(&categorical_with(0.0), "select(dbg_cat(ctx.sample.probe_bit, 4u), dbg_sentinel(f32(ctx.sample.probe_bit), ctx.frag_xy), ctx.sample.probe_bit == 0u)");
    for s in [f64::INFINITY, -1.0] {
        check_ramp(&categorical_with(s), "dbg_cat(ctx.sample.probe_bit, 4u)");
    }
}

negative_control!(
    view_ramp_of_a_categorical_sentinel_is_literal_when_storable,
    "an infinite sentinel, which no u-bits field stores, is not drawn literally",
    expected = "is not coloured with",
    check_ramp(
        &categorical_with(f64::INFINITY),
        "select(dbg_cat(ctx.sample.probe_bit, 4u), dbg_sentinel(f32(ctx.sample.probe_bit), ctx.frag_xy), \
         ctx.sample.probe_bit == 0u)"
    )
);

/// The payload ledger with `field`'s range replaced by `range`.
fn with_range(field: &str, range: Range) -> Ledger {
    let mut l = ledger::layout();
    let entry = l
        .entries
        .iter_mut()
        .find(|e| e.name == Some(field))
        .unwrap_or_else(|| panic!("no entry `{field}`"));
    entry.range = Some(range);
    l
}

/// `S`, an `f32` on a linear scale, its range closed at zero and at `hi`.
fn closed_at(hi: f64) -> Ledger {
    let range = Range {
        lo: Bound::Closed(0.0),
        hi: Bound::Closed(hi),
    };
    with_range("S", range)
}

/// The view of `field` generated from `ledger` colours it with `ramp`.
fn check_ramp_of(ledger: &Ledger, field: &str, ramp: &str) {
    let view = emitted(ledger, &format!("{}/{field}.wgsl", catalogue::DIR));
    assert!(
        view.contains(&format!("return {ramp};")),
        "the view of `{field}` is not coloured with {ramp}:\n{view}"
    );
}

/// `S`'s numeric view (RQ-231): its fixed range from 0 to `hi`, which an end with no finite bound leaves to the
/// measured end, `u_range`'s.
fn s_ramp(hi: &str) -> String {
    format!("ramp_viridis(range_norm(raw, 0.0, {hi}, uniforms.RANGE_AUTO != 0u, uniforms.u_range))")
}

#[test]
fn view_ramp_of_an_infinite_closed_bound_is_the_measured_end() {
    check_ramp_of(
        &closed_at(f64::INFINITY),
        "S",
        &s_ramp("uniforms.u_range.y"),
    );
}

negative_control!(
    view_ramp_of_an_infinite_closed_bound_is_the_measured_end,
    "a range closed at two finite ends is coloured over that range",
    expected = "is not coloured with",
    check_ramp_of(&closed_at(1.0), "S", &s_ramp("uniforms.u_range.y"))
);

/// The test of `field` generated from `ledger` stores `value`, and its control `altered`.
fn check_probe(ledger: &Ledger, field: &str, value: u64, altered: u64) {
    let tests = emitted(ledger, catalogue::TESTS_PATH);
    let want = format!("pick(alter, \"{field}\", {value}, {altered})");
    assert!(
        tests.contains(&want),
        "the probe does not store {value} in `{field}`, altered to {altered}"
    );
}

#[test]
fn view_probe_of_a_negative_bound_takes_the_width() {
    let negative = with_range("closure_step", Range::int(-5, -1));
    check_probe(&negative, "closure_step", 32768, 32769);
}

negative_control!(
    view_probe_of_a_negative_bound_takes_the_width,
    "a range ending at 3 is probed at its middle, not the width's",
    expected = "the probe does not store",
    check_probe(
        &with_range("closure_step", Range::int(0, 3)),
        "closure_step",
        32768,
        32769
    )
);

/// The header comments of `views`, each view's lines after its first, `//` and all, up to its declarations
/// (`// @uniform`, which are not prose): each is at most 120 columns
/// unless it holds one word alone, none is empty, and none could have taken the next line's first word (the wrap is
/// greedy).
fn check_headers(views: &[String]) {
    for view in views {
        let lines: Vec<&str> = view
            .lines()
            .skip(1)
            .take_while(|l| l.starts_with("//") && !l.starts_with("// @"))
            .collect();
        for pair in lines.windows(2) {
            let next = pair[1]["// ".len()..].split(' ').next().unwrap_or("");
            assert!(
                pair[0].chars().count() + 1 + next.chars().count() > 120,
                "a header line could have taken the next line's first word: {:?} then {next:?}",
                pair[0]
            );
        }
        for line in &lines {
            let words = line.strip_prefix("// ").unwrap_or("").split(' ').count();
            assert!(line.len() > "// ".len(), "a header line is empty: {line:?}");
            assert!(
                line.chars().count() <= 120 || words == 1,
                "a header line of more than one word is wider than 120 columns: {line:?}"
            );
        }
    }
}

#[test]
fn view_header_wraps_greedily_at_120_columns() {
    let mut views: Vec<String> = views_of(&ledger::layout())
        .into_iter()
        .map(|v| v.wgsl)
        .collect();
    // Every length to a line's width, then a name too long for any line, which takes a line of its own.
    for k in (0..48).chain([130]) {
        let name: &'static str = Box::leak(format!("probe_{}", "x".repeat(k)).into_boxed_str());
        let ledger = with_probe(name, 1, Scale::Flag, Range::int(0, 1));
        views.push(emitted(&ledger, &format!("{}/{name}.wgsl", catalogue::DIR)));
    }
    check_headers(&views);
}

negative_control!(
    view_header_wraps_greedily_at_120_columns,
    "a header wrapped before its line is full is not greedy",
    expected = "could have taken the next line's first word",
    check_headers(&["// generated\n// a\n// b\nfn colour() {}\n".to_owned()])
);

// ── REQ-TOOL-017: the shader and its test share the accessor ─────────────────────────────────────────────────────

/// What precedes a view at assembly, from the checked-in files: the prelude, the library, the unpack layer, the read
/// side and the stain's context.
fn view_context() -> String {
    let lib = "crates/render/shaders/wgsl/lib";
    [
        format!("{lib}/prelude.wgsl"),
        format!("{lib}/colour_space.wgsl"),
        format!("{lib}/present.wgsl"),
        wgsl::PATH.to_owned(),
        read::WGSL_PATH.to_owned(),
        "crates/render/shaders/wgsl/stain/context.wgsl".to_owned(),
    ]
    .iter()
    .map(|p| checked_in(p))
    .collect::<Vec<_>>()
    .join("\n")
}

/// The handle and members of the struct named `name` in `module`.
fn named_struct(
    module: &Module,
    name: &str,
) -> Option<(naga::Handle<naga::Type>, Vec<naga::StructMember>)> {
    let (h, t) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some(name))?;
    match &t.inner {
        TypeInner::Struct { members, .. } => Some((h, members.clone())),
        _ => None,
    }
}

/// The functions the shared library defines, the prelude, the colour-space maps and the presentation layer
/// (render_gui_spec §10.1; render contract Part 5): what a view calls to colour, not to read its field.
fn library_functions() -> BTreeSet<String> {
    let lib = "crates/render/shaders/wgsl/lib";
    let source = ["prelude", "colour_space", "present"]
        .iter()
        .map(|f| checked_in(&format!("{lib}/{f}.wgsl")))
        .collect::<Vec<_>>()
        .join("\n");
    let module =
        naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| panic!("the library: {e}"));
    module
        .functions
        .iter()
        .filter_map(|(_, f)| f.name.clone())
        .collect()
}

/// A view's `// @uniform` block as the assembler declares it (gui_state_contract §3): a struct of its uniforms bound
/// as `uniforms`, which the view reads as `uniforms.<name>`.
fn uniform_block(view: &str) -> String {
    let members: String = view
        .lines()
        .filter_map(|l| l.strip_prefix("// @uniform "))
        .filter_map(|l| {
            let (name, rest) = l.split_once(':')?;
            let (ty, _) = rest.split_once('=')?;
            Some(format!("    {}: {},\n", name.trim(), ty.trim()))
        })
        .collect();
    if members.is_empty() {
        return String::new();
    }
    format!("struct ViewUniforms {{\n{members}}}\n@group(0) @binding(1) var<uniform> uniforms: ViewUniforms;\n")
}

/// The accessor symbols `colour` references in `module`: each member of the read-side `SimState` it takes from
/// `ctx.sample`, each member of `ICDescriptor` it takes from `ctx.ic` (RQ-227), and each function it calls but the
/// shared library's ([`library_functions`]).
fn wgsl_accessors(module: &Module) -> BTreeSet<Accessor> {
    let library = library_functions();
    let mut out = BTreeSet::new();
    let (Some((simstate, members)), Some((ic, ic_members))) = (
        named_struct(module, "SimState"),
        named_struct(module, "ICDescriptor"),
    ) else {
        return out;
    };
    let Some((_, colour)) = module
        .functions
        .iter()
        .find(|(_, f)| f.name.as_deref() == Some("colour"))
    else {
        return out;
    };
    for (_, e) in colour.expressions.iter() {
        if let Expression::AccessIndex { base, index } = *e {
            if is_ctx_member(module, base, colour, simstate, "sample") {
                let name = members[index as usize].name.clone().unwrap_or_default();
                out.insert(Accessor::Member(name));
            }
            if is_ctx_member(module, base, colour, ic, "ic") {
                let name = ic_members[index as usize].name.clone().unwrap_or_default();
                out.insert(Accessor::IcMember(name));
            }
        }
    }
    let mut calls = Vec::new();
    collect_calls(&colour.body, &mut calls);
    for f in calls {
        let name = module.functions[f].name.clone().unwrap_or_default();
        if !library.contains(&name) {
            out.insert(Accessor::Function(name));
        }
    }
    out
}

/// Whether `e` in `f` is `ctx.<member>`: the member `member`, of type `ty`, of `f`'s first argument, a `Ctx`.
fn is_ctx_member(
    module: &Module,
    e: naga::Handle<Expression>,
    f: &naga::Function,
    ty: naga::Handle<naga::Type>,
    member: &str,
) -> bool {
    let Expression::AccessIndex { base, index } = f.expressions[e] else {
        return false;
    };
    let (Expression::FunctionArgument(0), Some(arg)) = (&f.expressions[base], f.arguments.first())
    else {
        return false;
    };
    match &module.types[arg.ty].inner {
        TypeInner::Struct { members, .. } => members
            .get(index as usize)
            .is_some_and(|m| m.ty == ty && m.name.as_deref() == Some(member)),
        _ => false,
    }
}

fn collect_calls(block: &naga::Block, out: &mut Vec<naga::Handle<naga::Function>>) {
    for s in block.iter() {
        match s {
            Statement::Call { function, .. } => out.push(*function),
            Statement::Block(b) => collect_calls(b, out),
            Statement::If { accept, reject, .. } => {
                collect_calls(accept, out);
                collect_calls(reject, out);
            }
            _ => {}
        }
    }
}

/// The body of the generated Rust check `check_<field>` in `tests`.
fn rust_check(tests: &str, field: &str) -> String {
    let start = format!("fn check_{}(p: &Probe) {{", field.to_lowercase());
    let at = tests
        .find(&start)
        .unwrap_or_else(|| panic!("the tests have no `{start}`"));
    let body = &tests[at..];
    body[..body.find("\n}\n").unwrap_or(body.len())].to_owned()
}

/// Each view's WGSL, compiled after its context, references exactly its accessors, and its Rust check in `tests`
/// references each: a member as `read.<member>`, an `ICDescriptor` member as `ic.<member>`, a function as
/// `<function>(`.
fn check_shared(views: &[View], wgsl_of: &dyn Fn(&View) -> String, tests: &str) {
    let context = view_context();
    for v in views {
        let view = wgsl_of(v);
        let source = format!("{context}\n{}{view}", uniform_block(&view));
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("`{}`'s view: {}", v.field, e.emit_to_string(&source)));
        let want: BTreeSet<Accessor> = v.read.accessors(v.field).into_iter().collect();
        assert!(!want.is_empty(), "`{}`'s view names no accessor", v.field);
        assert_eq!(
            wgsl_accessors(&module),
            want,
            "`{}`'s view does not reference its accessors",
            v.field
        );
        let check = rust_check(tests, v.field);
        for a in &want {
            let needle = match a {
                Accessor::Member(m) => format!("read.{m}"),
                Accessor::IcMember(m) => format!("ic.{m}"),
                Accessor::Function(f) => format!("{f}("),
            };
            assert!(
                check.contains(&needle),
                "`{}`'s test does not reference `{needle}`:\n{check}",
                v.field
            );
        }
    }
}

#[test]
fn shader_and_test_share_accessor() {
    let views = views_of(&ledger::layout());
    check_shared(
        &views,
        &|v| checked_in(&v.path.to_string_lossy()),
        &checked_in(catalogue::TESTS_PATH),
    );
}

negative_control!(
    shader_and_test_share_accessor,
    "a view of `state` that reads `detail` does not reference its accessor",
    expected = "`state`'s view does not reference its accessors",
    check_shared(
        &views_of(&ledger::layout()),
        &|v| checked_in(&v.path.to_string_lossy()).replace("ctx.sample.state", "ctx.sample.detail"),
        &checked_in(catalogue::TESTS_PATH)
    )
);

// ── REQ-TOOL-020: the catalogue's field list is the ledger's ─────────────────────────────────────────────────────

/// The catalogue's fields, and the views on disk, are `ledger::layout().entries`, in the ledger's order.
fn check_catalogue_equals_ledger(views: &[View], on_disk: &[String]) {
    let l = ledger::layout();
    let ledger_fields: Vec<&str> = l.entries.iter().filter_map(|e| e.name).collect();
    let catalogue_fields: Vec<&str> = views.iter().map(|v| v.field).collect();
    let missing: Vec<&&str> = ledger_fields
        .iter()
        .filter(|f| !catalogue_fields.contains(f))
        .collect();
    assert!(
        catalogue_fields == ledger_fields,
        "the catalogue's fields are not the ledger's: no view of {missing:?}"
    );
    let mut want: Vec<String> = ledger_fields
        .iter()
        .map(|f| format!("{}/{f}.wgsl", catalogue::DIR))
        .collect();
    want.sort();
    assert_eq!(
        on_disk, want,
        "the views on disk are not one per ledger field"
    );
}

#[test]
fn catalogue_equals_ledger() {
    check_catalogue_equals_ledger(&views_of(&ledger::layout()), &listing());
}

negative_control!(
    catalogue_equals_ledger,
    "a catalogue with one field deliberately unregistered is not the ledger's (PIT-3)",
    expected = "the catalogue's fields are not the ledger's",
    check_catalogue_equals_ledger(
        &views_of(&ledger::layout())
            .into_iter()
            .filter(|v| v.field != "state")
            .collect::<Vec<_>>(),
        &listing()
    )
);
