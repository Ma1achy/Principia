//! QA tests for TASK-M1-08 on the render side, written from the requirements and their sources, not from the
//! implementation:
//! - REQ-GEN-009 (gui_state_contract §3; RQ-219): after codegen every ledger field's generated view is a registry
//!   entry `{id, slot, source, category, uniformSchema, inputDomains}`, tagged debug, its slot `colour`, its source
//!   the file, no uniforms and the one default input; nothing else is under the generated directory. Hand-written debug
//!   occupants and generated views surface through one scan, each debug occupant's slot the one slot function it
//!   defines; a file in no slot directory, or a debug file defining other than one slot function, is refused.
//! - REQ-TOOL-020 / R-378: each catalogue view, baked into a stain, assembles at the full and the base tier reading
//!   only its own field: an `ICDescriptor` field as `ic.<member>` (RQ-227), the shadow as itself (RQ-228), a word field
//!   as the word's `.w` alone (R-378 as applied: "`fgw_length_raw` needs `.w` alone").
//! - RQ-227: the assembler refuses a stain that reads a `ctx.ic` member its field set does not fill, and one that
//!   names the read side's `ICDescriptor` buffer or readers (`ic_buffer`, `ic_read`, `ic_read_<…>`) instead of reading
//!   `ctx.ic`; RQ-228 likewise for an unfilled shadow.
//! - RQ-227 / RQ-228 drawn: a stain reading `ctx.ic` and the shadow, drawn per sample from hand-filled buffers, shows
//!   each sample's own stored descriptor members, and its stored shadow at the FTLE tier, the canonical quiet NaN
//!   (lowering Part 3a) at the base tier.
//!
//! Each test has a registered negative control (R-176).

use std::path::{Path, PathBuf};

use ledger::gen::prelude;
use naga::TypeInner;
use render::assemble::{self, AssembleError, Kind, Node, Occupant, Stain, Tier};
use render::debug_bake::ViewGenerator;
use render::registry::{self, Catalogue, Category, Entry, RegistryError};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

/// Render contract Part 6's `ICDescriptor` row, "all 12", read from `ctx.ic`.
const IC: [&str; 12] = [
    "m0",
    "m1",
    "m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
];

/// Lowering Part 3a's canonical quiet NaN.
const QNAN: u32 = 0x7FC0_0000;

const BASE: Tier = Tier {
    has_ftle: false,
    has_word: true,
};

fn ledger_fields() -> Vec<&'static str> {
    ledger::layout()
        .entries
        .iter()
        .map(|e| e.name.expect("named"))
        .collect()
}

fn render_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

// ── REQ-GEN-009: every generated view is a debug registry entry ───────────────────────────────────────────────────

/// `entries` hold, for each ledger field, exactly one debug entry of the `colour` slot whose source is its generated
/// file, with no uniforms and the one default input; and no generated entry for anything else.
fn check_registry(entries: &[Entry]) {
    let fields = ledger_fields();
    for f in &fields {
        let id = format!("debug/generated/{f}");
        let found: Vec<&Entry> = entries.iter().filter(|e| e.id == id).collect();
        assert!(!found.is_empty(), "no registry entry for `{f}`'s view");
        assert_eq!(found.len(), 1, "two registry entries for `{f}`'s view");
        let e = found[0];
        assert_eq!(e.category, Category::Debug, "`{id}` is not tagged debug");
        assert_eq!(e.slot, Kind::Colour, "`{id}`'s slot is not colour");
        let file = render_dir().join(format!("frag/debug/generated/{f}.wgsl"));
        let text =
            std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        assert_eq!(e.source, text, "`{id}`'s source is not its file");
        assert!(e.uniform_schema.is_empty(), "`{id}` declares uniforms");
        assert_eq!(
            e.input_domains.len(),
            1,
            "`{id}` has not the one default input"
        );
    }
    for e in entries
        .iter()
        .filter(|e| e.id.starts_with("debug/generated/"))
    {
        let f = &e.id["debug/generated/".len()..];
        assert!(
            fields.contains(&f),
            "a generated entry `{}` is no ledger field",
            e.id
        );
    }
    let catalogue = Catalogue::new(entries);
    let mut got: Vec<&str> = catalogue.fields();
    got.sort_unstable();
    let mut want = fields.clone();
    want.sort_unstable();
    assert_eq!(got, want, "the catalogue's fields are not the ledger's");
}

#[test]
fn qa_gen009_every_ledger_field_is_a_debug_colour_entry() {
    let entries = registry::registry().unwrap_or_else(|e| panic!("{e}"));
    for f in IC.iter().chain(&["r_sh", "p_sh"]) {
        assert!(ledger_fields().contains(f), "`{f}` is not a ledger field");
    }
    check_registry(&entries);
}

negative_control!(
    qa_gen009_every_ledger_field_is_a_debug_colour_entry,
    "a registry missing `r_sh`'s view is caught",
    expected = "no registry entry for `r_sh`'s view",
    {
        let entries: Vec<Entry> = registry::registry()
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .filter(|e| e.id != "debug/generated/r_sh")
            .collect();
        check_registry(&entries)
    }
);

/// A generated view mis-tagged as a slot occupant is caught.
#[cfg(feature = "controls")]
mod qa_gen009_every_ledger_field_is_a_debug_colour_entry_tag {
    use super::*;

    negative_control!(
        qa_gen009_every_ledger_field_is_a_debug_colour_entry,
        "a `K_0` view not tagged debug is caught",
        expected = "`debug/generated/K_0` is not tagged debug",
        {
            let entries: Vec<Entry> = registry::registry()
                .unwrap_or_else(|e| panic!("{e}"))
                .into_iter()
                .map(|mut e| {
                    if e.id == "debug/generated/K_0" {
                        e.category = Category::Slot;
                    }
                    e
                })
                .collect();
            check_registry(&entries)
        }
    );
}

/// A fresh scratch directory under the test target's temporary directory.
fn scratch(case: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let p = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_m1_08_{case}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&p);
    p
}

fn write(root: &Path, files: &[(&str, &str)]) {
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().expect("a parent")).expect("a directory");
        std::fs::write(&p, text).expect("a file");
    }
}

/// The scan of `files` written into a fresh render crate.
fn scan_of(files: &[(&str, &str)]) -> Result<Vec<Entry>, RegistryError> {
    let root = scratch("scan");
    write(&root, files);
    let got = registry::scan(&root);
    let _ = std::fs::remove_dir_all(&root);
    got
}

const HAND_DEBUG: &str =
    "// A hand-written debug view (gui_state_contract §3: quad-depth heatmap and the like).\n\
fn brightness(ctx: Ctx) -> f32 { return ctx.sample.d_min; }\n";
const GENERATED: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.ic.m0); }\n";
const PLAIN: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }\n";

/// A hand-written debug brightness (at `debug_dir`), a plain colour and a generated view surface through one scan,
/// each with its slot and category.
fn check_one_scan(debug_dir: &str) {
    let hand = format!("shaders/wgsl/frag/{debug_dir}/qa_depth.wgsl");
    let got = scan_of(&[
        (&hand, HAND_DEBUG),
        ("shaders/wgsl/frag/colour/qa_plain.wgsl", PLAIN),
        ("frag/debug/generated/qa_view.wgsl", GENERATED),
    ])
    .unwrap_or_else(|e| panic!("{e}"));
    let find = |id: &str| {
        got.iter()
            .find(|e| e.id == id)
            .map(|e| (e.slot, e.category))
            .unwrap_or_else(|| {
                panic!(
                    "no entry `{id}` in {:?}",
                    got.iter().map(|e| &e.id).collect::<Vec<_>>()
                )
            })
    };
    assert_eq!(find("debug/qa_depth"), (Kind::Brightness, Category::Debug));
    assert_eq!(find("colour/qa_plain"), (Kind::Colour, Category::Slot));
    assert_eq!(
        find("debug/generated/qa_view"),
        (Kind::Colour, Category::Debug)
    );
    assert_eq!(got.len(), 3, "the scan has other entries");
}

#[test]
fn qa_gen009_hand_written_and_generated_debug_share_one_scan() {
    check_one_scan("debug");
}

negative_control!(
    qa_gen009_hand_written_and_generated_debug_share_one_scan,
    "a hand-written view outside `debug/` is not a debug entry",
    expected = "no entry `debug/qa_depth`",
    check_one_scan("brightness")
);

/// The scan refuses a debug file defining two slot functions or none, and a file under no slot directory.
fn check_scan_refusals(two: &str) {
    match scan_of(&[("shaders/wgsl/frag/debug/qa_two.wgsl", two)]) {
        Err(RegistryError::DebugSlot(p, kinds)) => {
            assert!(
                p.ends_with("qa_two.wgsl"),
                "the refusal names {}",
                p.display()
            );
            assert_eq!(kinds.len(), 2, "the slot functions found: {kinds:?}");
        }
        other => panic!("a debug file defining two slot functions: {other:?}"),
    }
    let none = "fn helper() -> f32 { return 1.0; }\n// fn colour(ctx: Ctx) -> vec3<f32>\n";
    assert!(
        matches!(
            scan_of(&[("frag/debug/generated/qa_none.wgsl", none)]),
            Err(RegistryError::DebugSlot(_, ref k)) if k.is_empty()
        ),
        "a generated view defining no slot function (one only in a comment) is not refused"
    );
    assert!(
        matches!(
            scan_of(&[("shaders/wgsl/frag/qa_loose.wgsl", PLAIN)]),
            Err(RegistryError::NoSlot(_))
        ),
        "a file in no slot directory is not refused"
    );
}

#[test]
fn qa_gen009_scan_refuses_ambiguous_debug_and_unslotted_files() {
    check_scan_refusals(&format!("{PLAIN}{HAND_DEBUG}"));
}

negative_control!(
    qa_gen009_scan_refuses_ambiguous_debug_and_unslotted_files,
    "a debug file with one slot function is not refused",
    expected = "a debug file defining two slot functions: Ok",
    check_scan_refusals(PLAIN)
);

// ── REQ-TOOL-020 / R-378: each view assembles reading only its own field ─────────────────────────────────────────

/// What `field`'s view reads (R-378, RQ-227, RQ-228).
fn expected_fields(field: &str, word_fields: &[&str]) -> Vec<String> {
    if IC.contains(&field) {
        vec![format!("ic.{field}")]
    } else if word_fields.contains(&field) {
        vec!["word.w".to_owned()]
    } else {
        vec![field.to_owned()]
    }
}

fn word_fields() -> Vec<&'static str> {
    ledger::layout()
        .entries
        .iter()
        .filter(|e| {
            matches!(
                e.location,
                Some(ledger::schema::Location::Packed { word: "fgw_w", .. })
            )
        })
        .map(|e| e.name.expect("named"))
        .collect()
}

/// Each view, baked into a stain (`Catalogue`'s `ViewGenerator`), assembles at the full and the base tier with the
/// field set `want(field)`.
fn check_views_assemble(want: &dyn Fn(&str) -> Vec<String>) {
    let entries = registry::registry().unwrap_or_else(|e| panic!("{e}"));
    let cat = Catalogue::new(&entries);
    for f in ledger_fields() {
        let nodes: Vec<Node> = cat.view(f).unwrap_or_else(|| panic!("no view of `{f}`"));
        let stain = Stain::new(nodes).unwrap_or_else(|e| panic!("`{f}`: {e}"));
        for tier in [Tier::FULL, BASE] {
            let got = assemble::assemble(&stain, tier)
                .unwrap_or_else(|e| panic!("`{f}` at {tier:?}: {e}"));
            assert_eq!(
                got.fields,
                want(f),
                "`{f}`'s view at {tier:?} does not read exactly its own field"
            );
        }
    }
}

#[test]
fn qa_tool020_each_view_assembles_reading_only_its_field() {
    let words = word_fields();
    check_views_assemble(&|f| expected_fields(f, &words));
}

negative_control!(
    qa_tool020_each_view_assembles_reading_only_its_field,
    "a `V_0` view held to reading `ic.K_0` too reads less than that",
    expected = "`V_0`'s view at",
    {
        let words = word_fields();
        check_views_assemble(&|f| {
            let mut w = expected_fields(f, &words);
            if f == "V_0" {
                w.insert(0, "ic.K_0".to_owned());
            }
            w
        })
    }
);

// ── RQ-227 / RQ-228: the assembler's refusals ─────────────────────────────────────────────────────────────────────

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// The backbone: the `d_min` source into a colour of `colour`, the pass-through combiner, `OUT`.
fn stain_of(colour: &str) -> Result<Stain, AssembleError> {
    Stain::new(vec![
        node(Kind::Source, Occupant::Field("d_min".into()), &[]),
        node(
            Kind::Colour,
            Occupant::Custom(colour.to_owned()),
            &[Some(0)],
        ),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ])
}

/// Each colour of `named` is refused for naming the read side's `ICDescriptor` buffer or readers, the refusal naming
/// it; a stain reading `ctx.ic.K_0` or the shadow is refused at a field set missing it, and assembles at one holding it.
fn check_ic_refusals(named: &[(&str, &str)]) {
    for (name, body) in named {
        let colour = format!("fn colour(ctx: Ctx) -> vec3<f32> {{ {body} }}");
        let got = stain_of(&colour).and_then(|s| assemble::assemble(&s, Tier::FULL));
        match got {
            Ok(_) => panic!("a colour naming `{name}` was not refused"),
            Err(e) => assert!(
                e.to_string().contains(&format!("`{name}`")),
                "the refusal of `{name}` does not name it: {e}"
            ),
        }
    }
    let k0 = stain_of("fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.ic.K_0); }")
        .unwrap_or_else(|e| panic!("{e}"));
    for fields in [&["d_min"][..], &["d_min", "ic.V_0"]] {
        assert_eq!(
            assemble::assemble_reading(&k0, Tier::FULL, fields).err(),
            Some(AssembleError::UnfilledField("ic.K_0".into())),
            "a stain reading ctx.ic.K_0 at {fields:?}"
        );
    }
    assemble::assemble_reading(&k0, Tier::FULL, &["d_min", "ic.K_0"])
        .unwrap_or_else(|e| panic!("the stain at its own fields: {e}"));
    let sh =
        stain_of("fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.sample.r_sh[1].y); }")
            .unwrap_or_else(|e| panic!("{e}"));
    for tier in [Tier::FULL, BASE] {
        assert_eq!(
            assemble::assemble_reading(&sh, tier, &["d_min", "p_sh"]).err(),
            Some(AssembleError::UnfilledField("r_sh".into())),
            "a stain reading the shadow r_sh at {tier:?}"
        );
    }
}

const IC_NAMED: [(&str, &str); 4] = [
    ("ic_buffer", "return vec3<f32>(ic_buffer[0u].m0);"),
    ("ic_read", "return vec3<f32>(ic_read(0u).m0);"),
    ("ic_read_m0", "return vec3<f32>(ic_read_m0(0u));"),
    (
        "ic_read_rho_angle",
        "let ic_read_rho_angle = ctx.ic.rho_angle; return vec3<f32>(ic_read_rho_angle);",
    ),
];

#[test]
fn qa_rq227_assembler_refuses_unfilled_or_bypassing_ic_reads() {
    check_ic_refusals(&IC_NAMED);
}

negative_control!(
    qa_rq227_assembler_refuses_unfilled_or_bypassing_ic_reads,
    "a colour reading `ctx.ic.m0` as the contract says is not refused",
    expected = "a colour naming `ctx` was not refused",
    check_ic_refusals(&[("ctx", "return vec3<f32>(ctx.ic.m0);")])
);

// ── RQ-227 / RQ-228 drawn per sample ──────────────────────────────────────────────────────────────────────────────

/// Samples drawn, one per pixel of a 4 × 1 target.
const N: u32 = 4;

const DRAW_ENTRY: &str = r"
@fragment
fn qa_draw(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let i = u32(pos.x);
    let rgb = shade_sample(i, pos.xy, 0.25, vec3<f32>(1.0, 1.0, 2.0), ReadParams(0.125, 0.0625, 16u, 1000u));
    return vec4<u32>(bitcast<vec3<u32>>(rgb), i);
}
";

const DRAW_COLOUR: &str =
    "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.ic.rho_angle, ctx.ic.K_0, ctx.sample.r_sh[1].y); }";

/// The named struct's size in words and member word offsets, from naga's layout of `source`.
fn layout(source: &str, name: &str) -> (usize, Vec<(String, usize)>) {
    let m = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    let (_, ty) = m
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no struct `{name}`"));
    let TypeInner::Struct { members, span } = &ty.inner else {
        panic!("`{name}` is not a struct");
    };
    (
        *span as usize / 4,
        members
            .iter()
            .map(|mb| (mb.name.clone().unwrap_or_default(), mb.offset as usize / 4))
            .collect(),
    )
}

fn at(members: &[(String, usize)], n: &str) -> usize {
    members
        .iter()
        .find(|(m, _)| m == n)
        .map(|(_, o)| *o)
        .unwrap_or_else(|| panic!("no member `{n}`"))
}

fn rho_angle(i: u32) -> f32 {
    0.125 * (i + 1) as f32
}
fn k0(i: u32) -> f32 {
    -((i + 1) as f32)
}
fn r_sh_1y(i: u32) -> f32 {
    0.5 * (i + 1) as f32 + 0.03125
}

/// Draws the stain reading `ctx.ic.rho_angle`, `ctx.ic.K_0` and `ctx.sample.r_sh[1].y` per sample at `tier`, the
/// descriptors given in sample order `order`, and checks each pixel shows its own sample's stored values, the shadow
/// NaN at the base tier.
fn check_drawn(order: &[u32]) {
    let h = GpuHarness::new().expect("a GPU device");
    let stain = stain_of(DRAW_COLOUR).unwrap_or_else(|e| panic!("{e}"));
    for tier in [Tier::FULL, BASE] {
        let f = assemble::assemble(&stain, tier).unwrap_or_else(|e| panic!("{e}"));
        let source = f.source + DRAW_ENTRY;
        let variant = if tier.has_ftle {
            "SimStateFTLE"
        } else {
            "SimStateBase"
        };
        let (s_words, s_members) = layout(&source, variant);
        let (i_words, i_members) = layout(&source, "ICDescriptor");
        let mut state = vec![0u32; s_words * N as usize];
        let mut ic = vec![0u32; i_words * N as usize];
        for i in 0..N {
            let base = s_words * i as usize;
            if tier.has_ftle {
                // r_sh[1].y: the shadow's fourth component (payload §1, vec2-grouped).
                state[base + at(&s_members, "r_sh") + 3] = r_sh_1y(i).to_bits();
            }
            let slot = i_words * order[i as usize] as usize;
            ic[slot + at(&i_members, "rho_angle")] = rho_angle(i).to_bits();
            ic[slot + at(&i_members, "K_0")] = k0(i).to_bits();
        }
        let word = vec![0u32; 4 * N as usize];
        let kernel = h
            .fragment(
                &source,
                "qa_draw",
                &[
                    &[BindingKind::Uniform],
                    &[
                        BindingKind::Storage,
                        BindingKind::Storage,
                        BindingKind::Storage,
                    ],
                ],
                N,
                1,
            )
            .unwrap_or_else(|e| panic!("{tier:?}: {e}"));
        let uniforms = prelude::uniform_words(0);
        let px = kernel
            .draw(&[&[&uniforms], &[&state, &word, &ic]])
            .unwrap_or_else(|e| panic!("{tier:?}: {e}"));
        for i in 0..N {
            let p = px[i as usize];
            assert_eq!(p[3], i, "pixel {i} shaded sample {}", p[3]);
            assert_eq!(
                (p[0], p[1]),
                (rho_angle(i).to_bits(), k0(i).to_bits()),
                "{tier:?}: sample {i}'s ctx.ic is not its own stored descriptor (RQ-227)"
            );
            let sh = if tier.has_ftle {
                r_sh_1y(i).to_bits()
            } else {
                QNAN
            };
            assert_eq!(
                p[2], sh,
                "{tier:?}: sample {i}'s r_sh[1].y is {:#010x}, not {sh:#010x} (RQ-228)",
                p[2]
            );
        }
    }
}

#[test]
fn qa_rq227_rq228_ctx_ic_and_shadow_drawn_per_sample() {
    check_drawn(&[0, 1, 2, 3]);
}

negative_control!(
    qa_rq227_rq228_ctx_ic_and_shadow_drawn_per_sample,
    "descriptors stored in another sample order are not each sample's own",
    expected = "sample 0's ctx.ic is not its own stored descriptor",
    check_drawn(&[1, 0, 3, 2])
);
