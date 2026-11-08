//! The Print graph (principia_print_graph.md; R-485, R-486, R-492): post-processing as a node graph after the stain
//! and separate from it. It reads the stain's finished colours and makes a picture in a medium, which no measurement
//! reads.
//!
//! - [`graph`]: the graph as data, `principia.print/1` (§4.1): settings and `$` references, presets as sub-graphs, the
//!   one canonical serialisation;
//! - [`ports`]: the five port types and wire validation (§4.2);
//! - [`registry`]: the node types as a registry, each entry declared as data in `nodes/`, and the check render's build
//!   script runs on it and on the presets in `presets/` (§4.3, §11 test 8);
//! - [`codegen`]: the Print graph's own WGSL, generated from the registry's passes; the stain's codegen never sees it.
//!
//! The registry holds the Render node (`render`, category "Print source") and `output`; the lab's other node types
//! are ported by later tasks. The shipped preset is Plain, the identity.

pub mod codegen;
pub mod graph;
pub mod ports;
pub mod registry;

use std::sync::OnceLock;

use registry::{check, Checked};

/// The shipped node entries, their passes and the presets, as render's build script found and checked them: each a
/// file name and its text.
mod shipped {
    include!(concat!(env!("OUT_DIR"), "/print_shipped.rs"));
}

pub use shipped::{NODES, PRESETS, WGSL};

/// The shipped registry and presets, checked ([`registry::check`]). The build script ran the same check on the same
/// files and failed the build had it refused them, so it holds here.
pub fn shipped() -> &'static Checked {
    static SHIPPED: OnceLock<Checked> = OnceLock::new();
    SHIPPED.get_or_init(|| match check(NODES, WGSL, PRESETS) {
        Ok(checked) => checked,
        Err(e) => panic!("render's build script checked the Print graph's registry, yet: {e}"),
    })
}
