//! QA's tests for TASK-M1-06's REQ-PAY-090, written from the requirement and its sources: the generated `RenderQuad`
//! is compared field by field with dd_generation_root §3.7a's table, read from the doc, on the WGSL side through naga's
//! IR (name, type, offset, size) and on the Rust side through the generated layout; render contract Part 1's nine
//! `RenderQuad` fields, read from the doc, are a subset. Each test registers its negative control (R-176).

use ledger::gen::rust::offsets;
use ledger::quad::{render_quad, wgsl_struct};
use ledger::schema::Storage;
use validation::negative_control;

fn doc(path: &str) -> String {
    std::fs::read_to_string(format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn generation_root() -> String {
    doc("docs/design/principia_dd_generation_root.md")
}

fn render_contract() -> String {
    doc("docs/contracts/principia_render_contract.md")
}

/// §3.7a's table in `text`: each row's member and type, in order.
fn table_3_7a(text: &str) -> Vec<(String, String)> {
    let start = text
        .find("### 3.7a `RenderQuad`")
        .expect("dd_generation_root has §3.7a");
    let section = &text[start..];
    let end = section.find("\n### 3.8").expect("§3.8 follows §3.7a");
    section[..end]
        .lines()
        .filter(|l| l.starts_with("| `"))
        .map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            (cells[1].trim_matches('`').to_owned(), cells[2].to_owned())
        })
        .collect()
}

/// Render contract Part 1's `RenderQuad` field list in `text`.
fn part_1_fields(text: &str) -> Vec<String> {
    let line = text
        .lines()
        .find(|l| l.starts_with("- **`RenderQuad`**"))
        .expect("render contract Part 1 lists RenderQuad");
    let list = line
        .split('`')
        .nth(3)
        .expect("Part 1's RenderQuad field list in backticks");
    list.split(',').map(|f| f.trim().to_owned()).collect()
}

fn wgsl_type(s: naga::Scalar) -> &'static str {
    match s.kind {
        naga::ScalarKind::Uint => "u32",
        naga::ScalarKind::Float => "f32",
        _ => "other",
    }
}

/// The generated `RenderQuad` WGSL, through naga's IR: each member `(name, type, offset)` and the struct's size.
fn generated_wgsl() -> (Vec<(String, String, u32)>, u32) {
    let text = wgsl_struct(&render_quad());
    let m = naga::front::wgsl::parse_str(&text)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&text)));
    let (_, t) = m
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("RenderQuad"))
        .expect("the generated WGSL declares RenderQuad");
    let naga::TypeInner::Struct { members, span } = &t.inner else {
        panic!("RenderQuad is not a struct")
    };
    let members = members
        .iter()
        .map(|s| {
            let ty = match m.types[s.ty].inner {
                naga::TypeInner::Scalar(sc) => wgsl_type(sc),
                _ => "other",
            };
            (s.name.clone().unwrap_or_default(), ty.to_owned(), s.offset)
        })
        .collect();
    (members, *span)
}

/// The generated layout matches `table` field by field: in WGSL (naga's IR) the same names and types in the same
/// order, each a 4-byte scalar at offset 4·k, 4 B per row in all with no padding (§3.7a: "40 B, aligned to 4, with no
/// padding"); and the Rust-side struct the same names, storages and offsets.
fn check_against(table: &[(String, String)]) {
    let (wgsl, span) = generated_wgsl();
    let want: Vec<(String, String, u32)> = table
        .iter()
        .zip(0u32..)
        .map(|((n, t), k)| (n.clone(), t.clone(), 4 * k))
        .collect();
    assert_eq!(
        wgsl, want,
        "the generated WGSL RenderQuad is not §3.7a's table field by field"
    );
    assert_eq!(span, 4 * table.len() as u32, "RenderQuad is not 4 B a row");
    assert_eq!(span, 40, "RenderQuad is not §3.7a's 40 B");
    let s = render_quad();
    let (at, size) = offsets(&s);
    let rust: Vec<(String, String, u32)> = s
        .members
        .iter()
        .zip(at)
        .map(|(m, o)| {
            let ty = match m.storage {
                Storage::U32 => "u32",
                Storage::F32 => "f32",
                _ => "other",
            };
            (m.name.to_owned(), ty.to_owned(), o)
        })
        .collect();
    assert_eq!(
        rust, want,
        "the generated Rust RenderQuad layout is not §3.7a's table field by field"
    );
    assert_eq!(size, span, "the Rust and WGSL RenderQuad sizes differ");
}

#[test]
fn renderquad_fields_qa_generated_layout_is_section_3_7a() {
    let table = table_3_7a(&generation_root());
    assert_eq!(table.len(), 10, "§3.7a's table does not have ten rows");
    check_against(&table);
}

negative_control!(
    renderquad_fields_qa_generated_layout_is_section_3_7a,
    "a §3.7a whose cache_age is f32 fails the field-by-field comparison",
    expected = "is not §3.7a's table field by field",
    check_against(&table_3_7a(
        &generation_root().replace("| `cache_age` | u32 |", "| `cache_age` | f32 |")
    ))
);

/// Render contract Part 1's `RenderQuad` fields, the requirement's nine, are each a member of the generated struct.
fn check_subset(part_1: &[String]) {
    assert_eq!(
        part_1.len(),
        9,
        "render contract Part 1 does not list REQ-PAY-090's nine fields"
    );
    let (wgsl, _) = generated_wgsl();
    for f in part_1 {
        assert!(
            wgsl.iter().any(|(n, _, _)| n == f),
            "Part 1's `{f}` is not in the generated RenderQuad"
        );
    }
}

#[test]
fn renderquad_fields_qa_part_1_is_a_subset() {
    check_subset(&part_1_fields(&render_contract()));
}

negative_control!(
    renderquad_fields_qa_part_1_is_a_subset,
    "a Part 1 naming `majority_class` (deleted by R-20) for `cache_age` is not a subset",
    expected = "Part 1's `majority_class` is not in the generated RenderQuad",
    check_subset(&part_1_fields(&render_contract().replace(
        "ancestor_gap, cache_age`",
        "ancestor_gap, majority_class`"
    )))
);
