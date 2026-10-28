//! The link-variation diagnostic (REQ-VAL-024; validation ground-truth note § "Link variation as a conditioning
//! diagnostic (not a default change)"; inverse-encode contract Part 7): for each registry link `L`, a foreign IC is
//! canonicalised and inverted under `L` (`kernel::encode::ingest`), and the diagnostic reports Part 7's two outputs,
//! the physical residual `residual_L = ‖D_L(z_L) − C(x)‖_phys` and the conditioning number `κ_L`.
//!
//! **"Each registry link"** is each selection of the entries the blocks can select (REQ-GEN-013): a block's slot
//! codomain names its entries, and a row's selection takes one per block, the four momentum controls sharing one
//! (R-479 E1). The rows are the whole product, 2 mass × 3 `α` × 3 `β` × 3 momenta = 54 selections, not the default
//! with one block replaced: `residual_L` is the whole state's, so an IC whose difficulty sits in two blocks at once
//! clamps in every selection off the default in one block alone, and only a selection off it in both reaches it. Over
//! the product, an IC no row reaches is hard under every selection the registry offers; one some row reaches is a
//! conditioning difficulty that selection fixes. Entries no block can select (`softplus`, `exp`, `identity`) have no
//! row.
//!
//! **Provenance.** Each row carries the `SimConfig` `Links` of its whole selection ([`Row::links`]): a per-validation
//! link choice, encoding an IC under the selection that reaches it best and forward-checking under that selection,
//! records that row's `links` as the IC's provenance (R-483: the links live in `Links`, the one place).
//!
//! **The firewall.** The diagnostic is pure: it reads the registry and writes neither it nor the default selection
//! (`kernel::generated::links::Defaults`), so it never changes the default link, which is chosen for the sampling
//! measure, never for validation reachability. Nothing here defines a default.
//!
//! **What it does not do.** It classifies nothing: the note's "clean" and "bad" would need a threshold on `κ` that the
//! corpus does not give; the rows report `residual_L` and `κ_L` and the reader judges.

use engine::contract::sim_config::Links;
use kernel::decode::Ic;
use kernel::encode::ingest::ingest;
use kernel::encode::SimplexClamp;
use kernel::generated::links::{
    Alpha, Beta, Link, Mass, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ, SoftmaxTanh,
    SoftsignAlpha, SoftsignBeta, SoftsignQ, StickBreaking, TanhAlpha, TanhBeta, TanhQ,
};

use crate::roundtrip::phys;

/// One selection's row: the IC inverted under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The selection's links, at their parameters' defaults: what a per-validation choice of this row records as the
    /// IC's provenance.
    pub links: Links,
    /// `z_L`, the IC encoded under the selection.
    pub z: [f64; 8],
    /// `residual_L = ‖D_L(z_L) − C(x)‖_phys` (Part 7's output 2).
    pub residual: f64,
    /// `κ_L`, the weighted encode Jacobian's largest singular value at the IC (Part 7's output 3; Part 4).
    pub kappa: f64,
    /// `lookup_clamped`: a control's inverse clamped, or the inner pair coincides.
    pub clamped: bool,
}

/// `ic` inverted under the selection `S`.
fn row<S>(ic: &Ic<f64>) -> Row
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    let got = ingest::<S, f64, _>(ic, phys);
    Row {
        links: Links::of::<S>().expect("each row's momentum controls share one link"),
        z: got.encoded.z.z,
        residual: got.residual,
        kappa: got.kappa,
        clamped: got.encoded.report.lookup_clamped(),
    }
}

/// The rows of the mass link `M`, `α`'s `A` and `β`'s `B`: one per momentum link, in registry order.
fn momenta<M, A, B>(ic: &Ic<f64>, rows: &mut Vec<Row>)
where
    M: Link<2, 3, Codomain = Mass> + SimplexClamp,
    A: Link<1, 1, Codomain = Alpha>,
    B: Link<1, 1, Codomain = Beta>,
{
    rows.push(row::<Select<M, A, B, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>>(ic));
    rows.push(row::<Select<M, A, B, TanhQ, TanhQ, TanhQ, TanhQ>>(ic));
    rows.push(row::<
        Select<M, A, B, SoftsignQ, SoftsignQ, SoftsignQ, SoftsignQ>,
    >(ic));
}

/// The rows of the mass link `M` and `α`'s `A`: one run of [`momenta`] per `β` link, in registry order.
fn beta<M, A>(ic: &Ic<f64>, rows: &mut Vec<Row>)
where
    M: Link<2, 3, Codomain = Mass> + SimplexClamp,
    A: Link<1, 1, Codomain = Alpha>,
{
    momenta::<M, A, SigmoidBeta>(ic, rows);
    momenta::<M, A, TanhBeta>(ic, rows);
    momenta::<M, A, SoftsignBeta>(ic, rows);
}

/// The rows of the mass link `M`: one run of [`beta`] per `α` link, in registry order.
fn alpha<M>(ic: &Ic<f64>, rows: &mut Vec<Row>)
where
    M: Link<2, 3, Codomain = Mass> + SimplexClamp,
{
    beta::<M, SigmoidAlpha>(ic, rows);
    beta::<M, TanhAlpha>(ic, rows);
    beta::<M, SoftsignAlpha>(ic, rows);
}

/// The diagnostic of `ic`, a physical IC in any frame (positive masses summing to 1, as `C` requires): one [`Row`] per
/// selection of the registry's entries, mass outermost, then `α`, `β` and the momenta, each block's entries in registry
/// order, so the first row is the default selection's.
pub fn diagnose(ic: &Ic<f64>) -> Vec<Row> {
    let mut rows = Vec::new();
    alpha::<SoftmaxTanh>(ic, &mut rows);
    alpha::<StickBreaking>(ic, &mut rows);
    rows
}
