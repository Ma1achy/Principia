//! The debug field views, baked one source per field on demand (lowering contract Part 3, fragment side: "one tiny
//! source per field from the catalogue generator; compiling a mega-switch over heterogeneous field types would be the
//! interpreter anti-pattern"; Part 4: "all debug views … compile WGSL on selection, async, last-valid fallback").
//!
//! A field's view is a stain of its own, which the catalogue generator writes ([`ViewGenerator`]; the registry's
//! [`crate::registry::Catalogue`] gives each generated view as one, TASK-M1-08). Selecting a field ([`DebugViews::select`]) bakes its view the first time, keeps it, and requests it
//! from the pipeline cache like any stain: its own fragment key, its own pipeline, compiled off the frame's path.
//! Nothing at runtime chooses between fields inside a shader: each field id is a different pipeline.

use std::collections::HashMap;

use crate::assemble::{AssembleError, Node, Stain, Tier};
use crate::pipeline_cache::{NodeKey, PipelineCache, RequestError, Requested};

/// The catalogue generator's side of the entry point: a field id's view as a stain's nodes.
pub trait ViewGenerator {
    /// The debug view of `field`, or `None` where the catalogue has none.
    fn view(&self, field: &str) -> Option<Vec<Node>>;
}

impl<F: Fn(&str) -> Option<Vec<Node>>> ViewGenerator for F {
    fn view(&self, field: &str) -> Option<Vec<Node>> {
        self(field)
    }
}

/// The node keys of `field`'s view, one per node: the FNV-1a hash of `debug-view/<field>/<i>` with the top bit set,
/// so a view's nodes keep their own last-valid state, apart from each other view's and from a user stain's, whose
/// keys are 32-bit node ids.
pub fn view_keys(field: &str, nodes: usize) -> Vec<NodeKey> {
    (0..nodes)
        .map(|i| ledger::version::fnv1a64(format!("debug-view/{field}/{i}").as_bytes()) | 1 << 63)
        .collect()
}

/// Why a view could not be baked or selected.
#[derive(Clone, Debug, PartialEq)]
pub enum BakeError {
    /// The catalogue has no view of the field.
    NoView(String),
    /// The generated view is not a stain.
    Stain(AssembleError),
    /// The cache refused the request.
    Request(RequestError),
}

impl std::fmt::Display for BakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BakeError::NoView(field) => write!(f, "the catalogue has no view of `{field}`"),
            BakeError::Stain(e) => write!(f, "the generated view is not a stain: {e}"),
            BakeError::Request(e) => write!(f, "{e}"),
        }
    }
}

/// The baked views, by field id, and the generator that bakes the rest.
pub struct DebugViews<G> {
    generator: G,
    baked: HashMap<String, (Stain, Vec<NodeKey>)>,
}

impl<G: ViewGenerator> DebugViews<G> {
    /// No view baked yet.
    pub fn new(generator: G) -> DebugViews<G> {
        DebugViews {
            generator,
            baked: HashMap::new(),
        }
    }

    /// `field`'s view and its node keys: generated and checked the first time, then kept.
    pub fn bake(&mut self, field: &str) -> Result<(&Stain, &[NodeKey]), BakeError> {
        if !self.baked.contains_key(field) {
            let nodes = self
                .generator
                .view(field)
                .ok_or_else(|| BakeError::NoView(field.to_owned()))?;
            let keys = view_keys(field, nodes.len());
            let stain = Stain::new(nodes).map_err(BakeError::Stain)?;
            self.baked.insert(field.to_owned(), (stain, keys));
        }
        let (stain, keys) = &self.baked[field];
        Ok((stain, keys))
    }

    /// Selects `field`'s view at `tier`: baked on demand, then requested from `cache` as the current pipeline.
    pub fn select(
        &mut self,
        field: &str,
        cache: &mut PipelineCache,
        tier: Tier,
    ) -> Result<Requested, BakeError> {
        let (stain, keys) = self.bake(field)?;
        cache.request(stain, keys, tier).map_err(BakeError::Request)
    }

    /// How many views are baked.
    pub fn baked(&self) -> usize {
        self.baked.len()
    }
}
