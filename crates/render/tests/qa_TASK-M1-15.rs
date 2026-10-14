//! QA test for TASK-M1-15, written from REQ-TOOL-028 ("observation taps … must read payloads and uniforms only and
//! never perturb results"; systems_architecture §1), not from the implementation: every debug view of the registry,
//! assembled at the full tier, the one binding every buffer, with the colour pass's entry as the render loop compiles
//! it, binds no global it could write: each `var<storage>` global is read-only and no storage texture is writable. A
//! view that cannot write a buffer cannot change the payload's bytes, whatever it draws (the structural half of the
//! payload-byte property the implementer's GPU test checks by readback).
//!
//! The test registers its negative control (R-176): the same modules with their payload buffers declared
//! `read_write`.

use std::sync::OnceLock;

use render::assemble::{assemble, Tier};
use render::pipeline_cache::STAIN_ENTRY;
use render::registry::{self, Category};
use validation::golden_scene::{row_scene, Colouring};
use validation::negative_control;

/// The compiled sources of every debug view of the registry at the full tier, the tier that binds every payload
/// buffer: `(view, source)`, assembled once per test binary.
fn view_sources() -> &'static [(String, String)] {
    static SOURCES: OnceLock<Vec<(String, String)>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let entries = registry::registry().unwrap_or_else(|e| panic!("{e}"));
        let debug: Vec<_> = entries
            .iter()
            .filter(|e| e.category == Category::Debug)
            .collect();
        assert!(!debug.is_empty(), "the registry lists no debug view");
        debug
            .into_iter()
            .map(|entry| {
                let scene = row_scene(
                    "qa observation taps",
                    8,
                    Colouring::Probe(entry.source.clone(), Vec::new()),
                )
                .unwrap_or_else(|e| panic!("`{}`: {e}", entry.id));
                let stain = scene
                    .stain()
                    .unwrap_or_else(|e| panic!("`{}`: {e}", entry.id));
                let fragment = assemble(&stain, Tier::FULL)
                    .unwrap_or_else(|e| panic!("`{}` does not assemble: {e}", entry.id));
                (
                    entry.id.clone(),
                    format!("{}{STAIN_ENTRY}", fragment.source),
                )
            })
            .collect()
    })
}

/// Every global of `source` is unwritable: a storage buffer has load access only, and no storage texture is
/// writable.
fn check_read_only(view: &str, source: &str) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("`{view}`: {}", e.emit_to_string(source)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("`{view}`: {}", e.emit_to_string(source)));
    let mut storage = 0;
    for (_, g) in module.global_variables.iter() {
        let name = g.name.as_deref().unwrap_or("(unnamed)");
        if let naga::AddressSpace::Storage { access } = g.space {
            storage += 1;
            assert!(
                !access.contains(naga::StorageAccess::STORE),
                "`{view}` binds `{name}` writable: an observation tap could write the payload"
            );
        }
        if let naga::TypeInner::Image {
            class: naga::ImageClass::Storage { access, .. },
            ..
        } = module.types[g.ty].inner
        {
            assert!(
                !access.contains(naga::StorageAccess::STORE),
                "`{view}` binds the storage texture `{name}` writable"
            );
        }
    }
    assert!(
        storage > 0,
        "`{view}` binds no payload buffer: the check would hold vacuously"
    );
}

fn check_all(transform: fn(&str) -> String) {
    for (view, source) in view_sources() {
        check_read_only(view, &transform(source));
    }
}

#[test]
fn qa_m1_15_debug_views_bind_the_payload_read_only() {
    check_all(str::to_owned);
}

negative_control!(
    qa_m1_15_debug_views_bind_the_payload_read_only,
    "every view's payload buffers declared read_write, as a tap that could write the payload would bind them",
    expected = "writable: an observation tap could write the payload",
    check_all(|s| s.replace("var<storage, read>", "var<storage, read_write>"))
);
