//! The link-variation diagnostic (REQ-VAL-024; validation ground-truth note § "Link variation as a conditioning
//! diagnostic (not a default change)"; inverse-encode contract Part 7): for each registry link `L`, a foreign IC is
//! canonicalised and inverted under `L` (`kernel::encode::ingest`), and the diagnostic reports Part 7's two outputs,
//! the physical residual `residual_L = ‖D_L(z_L) − C(x)‖_phys` and the conditioning number `κ_L`, so a reader can tell
//! an IC that is hard only under the default link (clean under some `L`, a conditioning difficulty) from one that is
//! hard under every link (fundamental).
//!
//! **"Each registry link"** is each entry a block can select (REQ-GEN-013): the block's slot codomain names which. One
//! row per block and entry: the selection is the default one with that block's link replaced by `L`, the four momentum
//! controls sharing one (R-479 E1), so a row varies one block and holds the rest at the defaults. A block's own default
//! entry gives the default selection's row. Entries no block can select (`softplus`, `exp`, `identity`) have no row.
//!
//! **Provenance.** Each row carries the `SimConfig` `Links` of its selection ([`Row::links`]): a per-validation link
//! choice, encoding an IC under the link that reaches it best and forward-checking under that link, records that row's
//! `links` as the IC's provenance (R-483: the links live in `Links`, the one place).
//!
//! **The firewall.** The diagnostic is pure: it reads the registry and the default selection and writes neither, so it
//! never changes the default link, which is chosen for the sampling measure, never for validation reachability. The
//! rows' selections are the default one (`kernel::generated::links::Defaults`) with one block replaced; nothing here
//! defines a default.
//!
//! **What it does not do.** It classifies nothing: the note's "clean" and "bad" would need a threshold on `κ` that the
//! corpus does not give; the rows report `residual_L` and `κ_L` and the reader judges.

use engine::contract::sim_config::Links;
use kernel::decode::Ic;
use kernel::encode::ingest::ingest;
use kernel::encode::SimplexClamp;
use kernel::generated::links::{
    Defaults, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ, SoftmaxTanh, SoftsignAlpha,
    SoftsignBeta, SoftsignQ, StickBreaking, TanhAlpha, TanhBeta, TanhQ,
};

use crate::roundtrip::phys;

/// The default selection with the mass block's link `M`.
type WithMass<M> = Select<
    M,
    <Defaults as Selection>::AlphaLink,
    <Defaults as Selection>::BetaLink,
    <Defaults as Selection>::Q0Link,
    <Defaults as Selection>::Q1Link,
    <Defaults as Selection>::Q2Link,
    <Defaults as Selection>::Q3Link,
>;

/// The default selection with `α`'s link `A`.
type WithAlpha<A> = Select<
    <Defaults as Selection>::MassLink,
    A,
    <Defaults as Selection>::BetaLink,
    <Defaults as Selection>::Q0Link,
    <Defaults as Selection>::Q1Link,
    <Defaults as Selection>::Q2Link,
    <Defaults as Selection>::Q3Link,
>;

/// The default selection with `β`'s link `B`.
type WithBeta<B> = Select<
    <Defaults as Selection>::MassLink,
    <Defaults as Selection>::AlphaLink,
    B,
    <Defaults as Selection>::Q0Link,
    <Defaults as Selection>::Q1Link,
    <Defaults as Selection>::Q2Link,
    <Defaults as Selection>::Q3Link,
>;

/// The default selection with every momentum control's link `Q`.
type WithMomenta<Q> = Select<
    <Defaults as Selection>::MassLink,
    <Defaults as Selection>::AlphaLink,
    <Defaults as Selection>::BetaLink,
    Q,
    Q,
    Q,
    Q,
>;

/// One registry link's row: the IC inverted under the default selection with `block`'s link replaced by `link`.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The block whose link the row varies, by its `Links` field: `mass`, `alpha`, `beta` or `momenta`.
    pub block: &'static str,
    /// The selection's links, at their parameters' defaults, the block's the registry entry `L` ([`Row::link`]): what a
    /// per-validation choice of this row records as the IC's provenance.
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

impl Row {
    /// The registry entry `L` the row's block takes, read from its links, the one place it lives.
    pub fn link(&self) -> &str {
        let (_, choice) = self
            .links
            .blocks()
            .into_iter()
            .find(|(name, _)| *name == self.block)
            .expect("each row's block is a block of Links");
        &choice.id
    }
}

/// `ic` inverted under the selection `S`, the row of `block`'s link.
fn row<S>(block: &'static str, ic: &Ic<f64>) -> Row
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    let got = ingest::<S, f64, _>(ic, phys);
    Row {
        block,
        links: Links::of::<S>().expect("each row's momentum controls share one link"),
        z: got.encoded.z.z,
        residual: got.residual,
        kappa: got.kappa,
        clamped: got.encoded.report.lookup_clamped(),
    }
}

/// The diagnostic of `ic`, a physical IC in any frame (positive masses summing to 1, as `C` requires): one [`Row`] per
/// block and registry link that block can select, in the order mass, `α`, `β`, momenta, each block's links in registry
/// order.
pub fn diagnose(ic: &Ic<f64>) -> Vec<Row> {
    vec![
        row::<WithMass<SoftmaxTanh>>("mass", ic),
        row::<WithMass<StickBreaking>>("mass", ic),
        row::<WithAlpha<SigmoidAlpha>>("alpha", ic),
        row::<WithAlpha<TanhAlpha>>("alpha", ic),
        row::<WithAlpha<SoftsignAlpha>>("alpha", ic),
        row::<WithBeta<SigmoidBeta>>("beta", ic),
        row::<WithBeta<TanhBeta>>("beta", ic),
        row::<WithBeta<SoftsignBeta>>("beta", ic),
        row::<WithMomenta<SigmoidQ>>("momenta", ic),
        row::<WithMomenta<TanhQ>>("momenta", ic),
        row::<WithMomenta<SoftsignQ>>("momenta", ic),
    ]
}
