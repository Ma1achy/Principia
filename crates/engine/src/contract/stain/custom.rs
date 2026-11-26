//! A node's own code as the editor tells its author of it (`render::custom`): a compile error at the author's own line
//! of the node's source, the status beside the node, and the values a changed declaration carries.

use render::assemble;

use crate::contract::stain::graph::{GraphError, NodeKind, Occupant, Uniform};

pub use render::custom::{
    line_in, problem_of, reconcile, Note, Origin, Problem, Reconciled, Remembered, Status,
};

/// The uniforms `occupant` declares in a node of `kind`, or why its declaration does not read.
pub fn uniforms(kind: NodeKind, occupant: &Occupant) -> Result<Vec<Uniform>, GraphError> {
    Ok(assemble::declaration(kind.into(), &occupant.into())?.uniforms)
}
