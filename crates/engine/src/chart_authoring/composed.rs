//! A composed chart: the CPU decode and the inverses (encode, exact and partial lookup, ingest) of a validated chart
//! spec, reached through `resolve_spec`'s lowering and the registry's rows (design `principia_chart_composition.md`
//! § 3, § 7).
//!
//! **One dispatch for these callers.** [`Composed::new`] resolves the spec, then builds what the lowering names: a
//! uniform lowering is the latent chart on the lowering's own `z₀, q₁, q₂`; a baked one is the kernel chart its rows
//! close on, taken from the caller's [`Charts`], which holds each chart's parameters (the window, the frozen controls),
//! since a spec carries none. Decode and the inverses then run through the kernel's generic source at the defaults
//! links and the physical norm. The lock (`path::lock`, `path::leaf`, `Segment::lock_on_chart`), the gate's composed
//! rows and the validation ingest all go through it.
//!
//! **What does not go through it, and why.** A caller that chooses the link selection `S` itself cannot: the table
//! holds one boxed chart at the defaults links, and a selection is a type, so the generic `decode_only`, the inspector
//! and the precision ring, `contract::decoded::state` (an energy-normalised decode of a latent `z`, with no pixel) and
//! `nav::directions` (which reads the decoder's blocks, not a state) call the kernel's functions directly. Ingest alone
//! takes `S`, as a method generic, since it reads no chart.
//!
//! **Not built here:** a lowering to the CPU batch (the general and numerical solves) is refused as
//! [`Unresolved::Cpu`]; the pair maps and held constraints keep their own routes.
//!
//! The plan is unclear here; chose to count each call per kind ([`Routed`]) because a bit-for-bit comparison cannot show
//! which route produced the bits, and the count can.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use kernel::chart::{Chart, ChartDecode, Latent, ValidationResult};
use kernel::decode::{Decoded, Ic};
use kernel::encode::by_kind::{encode_into_within, ChartEncoded};
use kernel::encode::ingest::{ingest, Ingested};
use kernel::encode::SimplexClamp;
use kernel::generated::links::{Defaults, Selection};
use ledger::constants::EPS_PHYS;

use super::lowering::{resolve_spec, Lowering};
use super::registry::{AxisSpec, ChartSpec, Primitive, Registry};
use super::validate::{validate_spec, ValidatedSpec};
use crate::contract::sim_config::LATENT_DIM;
use crate::lock::FrozenControls;
use crate::lookup::{
    lookup, lookup_partial, phys, Exact, Fill, Locked, Partial, Rejected, Tolerances,
};

/// A kernel chart, type-erased at f64 for the CPU's decode and inverses.
trait Erased64 {
    fn name(&self) -> String;
    fn validate(&self, u: f64, v: f64) -> ValidationResult;
    fn decode(&self, u: f64, v: f64) -> Decoded<f64>;
    fn encode(&self, ic: &Ic<f64>, tol: &Tolerances) -> ChartEncoded<f64>;
    fn lookup(&self, entry: &Exact, tol: &Tolerances) -> Result<Locked, Rejected>;
    fn partial(
        &self,
        partial: &Partial,
        locked_nu: Option<f64>,
        tol: &Tolerances,
    ) -> Result<Locked, Rejected>;
}

impl<C: Fill> Erased64 for C
where
    f64: From<C::Param>,
{
    fn name(&self) -> String {
        Chart::name(self).to_owned()
    }
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        Chart::validate(self, u, v)
    }
    fn decode(&self, u: f64, v: f64) -> Decoded<f64> {
        self.decode_at::<Defaults, f64>(u, v)
    }
    fn encode(&self, ic: &Ic<f64>, tol: &Tolerances) -> ChartEncoded<f64> {
        let eps = EPS_PHYS.number() * f64::EPSILON;
        encode_into_within::<Defaults, f64, _, _>(self, ic, phys, eps, tol.curve_tol)
    }
    fn lookup(&self, entry: &Exact, tol: &Tolerances) -> Result<Locked, Rejected> {
        lookup::<Defaults, C, _>(self, entry, phys, tol)
    }
    fn partial(
        &self,
        partial: &Partial,
        locked_nu: Option<f64>,
        tol: &Tolerances,
    ) -> Result<Locked, Rejected> {
        lookup_partial::<Defaults, C, _>(self, partial, locked_nu, phys, tol)
    }
}

/// A kernel chart held at f32, type-erased for the f32 decode.
trait Erased32 {
    fn decode(&self, u: f32, v: f32) -> Decoded<f32>;
}

impl<C: ChartDecode<Param = f32>> Erased32 for C {
    fn decode(&self, u: f32, v: f32) -> Decoded<f32> {
        self.decode_at::<Defaults, f32>(u, v)
    }
}

struct Entry {
    f64: Box<dyn Erased64>,
    f32: Option<Box<dyn Erased32>>,
    hidden: Option<Hidden>,
}

/// A nonlinear chart's hidden coordinates, which the lock reads (`FrozenControls`): the controls it holds frozen, and
/// which of the eight its axes set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hidden {
    /// The frozen controls, in the decoder's block layout.
    pub frozen: [f64; LATENT_DIM],
    /// `true` for each control the axes set.
    pub set_by_axes: [bool; LATENT_DIM],
}

/// The kernel charts a baked lowering can name, each with its parameters, by `kernel::chart::REGISTRY` name.
#[derive(Default)]
pub struct Charts {
    entries: BTreeMap<&'static str, Entry>,
}

impl Charts {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// The chart `name`, held at f64.
    pub fn with<C>(mut self, name: &'static str, chart: C) -> Self
    where
        C: Fill + 'static,
        f64: From<C::Param>,
    {
        self.entries.insert(
            name,
            Entry {
                f64: Box::new(chart),
                f32: None,
                hidden: None,
            },
        );
        self
    }

    /// The chart `name` held at f64, with the hidden coordinates the lock reads ([`Composed::hidden`]).
    pub fn with_hidden<C>(mut self, name: &'static str, chart: C) -> Self
    where
        C: Fill + FrozenControls + 'static,
        f64: From<C::Param>,
    {
        let hidden = Hidden {
            frozen: chart.frozen(),
            set_by_axes: chart.set_by_axes(),
        };
        self.entries.insert(
            name,
            Entry {
                f64: Box::new(chart),
                f32: None,
                hidden: Some(hidden),
            },
        );
        self
    }

    /// The same chart `name` held at f32, for [`Composed::decode32`]; the f64 chart must be there first.
    pub fn with_f32<C: ChartDecode<Param = f32> + 'static>(
        mut self,
        name: &'static str,
        chart: C,
    ) -> Self {
        self.entries
            .get_mut(name)
            .expect("the f64 chart is registered first")
            .f32 = Some(Box::new(chart));
        self
    }
}

/// Why a spec does not compose.
#[derive(Clone, Debug, PartialEq)]
pub enum Unresolved {
    /// The spec lowers to the CPU batch, whose solves are not built here.
    Cpu,
    /// The lowering names a chart the table does not hold.
    NoChart(&'static str),
}

/// How many calls of each kind went through a composed chart.
#[derive(Debug, Default)]
pub struct Routed {
    decode: AtomicUsize,
    encode: AtomicUsize,
    lookup: AtomicUsize,
    partial: AtomicUsize,
    ingest: AtomicUsize,
}

/// A snapshot of [`Routed`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    /// Decodes, f64 and f32.
    pub decode: usize,
    /// Encodes.
    pub encode: usize,
    /// Exact lookups.
    pub lookup: usize,
    /// Partial lookups.
    pub partial: usize,
    /// Ingests.
    pub ingest: usize,
}

/// A validated spec's chart: its decode and inverses, through its lowering.
pub struct Composed {
    lowering: Lowering,
    entry: Entry,
    routed: Routed,
}

impl Composed {
    /// The chart of `spec`: its lowering resolved, the kernel chart that lowering names built or taken from
    /// `charts`.
    pub fn new(spec: &ValidatedSpec, charts: Charts) -> Result<Composed, Unresolved> {
        let lowering = resolve_spec(spec).lowering;
        let mut charts = charts;
        let entry = match &lowering {
            Lowering::Uniform { z0, q1, q2 } => {
                let narrow = |v: &[f64; 8]| v.map(|x| x as f32);
                Entry {
                    f64: Box::new(Latent {
                        z0: *z0,
                        q1: *q1,
                        q2: *q2,
                    }),
                    f32: Some(Box::new(Latent {
                        z0: narrow(z0),
                        q1: narrow(q1),
                        q2: narrow(q2),
                    })),
                    hidden: None,
                }
            }
            Lowering::Baked { chart, .. } => charts
                .entries
                .remove(chart)
                .ok_or(Unresolved::NoChart(chart))?,
            Lowering::Cpu { .. } => return Err(Unresolved::Cpu),
        };
        Ok(Composed {
            lowering,
            entry,
            routed: Routed::default(),
        })
    }

    /// The latent chart at the origin, for the calls no chart enters: ingest takes a state and no pixel.
    pub fn free() -> Composed {
        let dir = |k: usize| {
            let mut q = [0.0; LATENT_DIM];
            q[k] = 1.0;
            AxisSpec::of(Primitive::Direction(q))
        };
        let spec = ChartSpec::new(dir(0), dir(1), [0.0; LATENT_DIM]);
        let valid =
            validate_spec(&Registry::standard(), &spec).expect("the latent plane validates");
        Composed::new(&valid, Charts::new()).expect("a latent chart composes")
    }

    /// The chart's name, as `kernel::chart::REGISTRY` lists it.
    pub fn name(&self) -> String {
        self.entry.f64.name()
    }

    /// The chart's domain function at `(u, v)`: `Chart::validate`.
    pub fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.entry.f64.validate(u, v)
    }

    /// The chart's hidden coordinates, where the table registered them ([`Charts::with_hidden`]).
    pub fn hidden(&self) -> Option<&Hidden> {
        self.entry.hidden.as_ref()
    }

    /// How the spec lowered.
    pub fn lowering(&self) -> &Lowering {
        &self.lowering
    }

    /// The calls routed so far.
    pub fn counts(&self) -> Counts {
        let n = |a: &AtomicUsize| a.load(Ordering::Relaxed);
        Counts {
            decode: n(&self.routed.decode),
            encode: n(&self.routed.encode),
            lookup: n(&self.routed.lookup),
            partial: n(&self.routed.partial),
            ingest: n(&self.routed.ingest),
        }
    }

    /// `D ∘ Φ` at `(u, v)` at f64.
    pub fn decode(&self, u: f64, v: f64) -> Decoded<f64> {
        self.routed.decode.fetch_add(1, Ordering::Relaxed);
        self.entry.f64.decode(u, v)
    }

    /// `D ∘ Φ` at `(u, v)` at f32, where the table held the chart at f32.
    pub fn decode32(&self, u: f32, v: f32) -> Option<Decoded<f32>> {
        let chart = self.entry.f32.as_ref()?;
        self.routed.decode.fetch_add(1, Ordering::Relaxed);
        Some(chart.decode(u, v))
    }

    /// `ic` encoded into the chart at `tol`.
    pub fn encode(&self, ic: &Ic<f64>, tol: &Tolerances) -> ChartEncoded<f64> {
        self.routed.encode.fetch_add(1, Ordering::Relaxed);
        self.entry.f64.encode(ic, tol)
    }

    /// An exact entry, looked up.
    pub fn lookup(&self, entry: &Exact, tol: &Tolerances) -> Result<Locked, Rejected> {
        self.routed.lookup.fetch_add(1, Ordering::Relaxed);
        self.entry.f64.lookup(entry, tol)
    }

    /// A partial entry, looked up.
    pub fn lookup_partial(
        &self,
        partial: &Partial,
        locked_nu: Option<f64>,
        tol: &Tolerances,
    ) -> Result<Locked, Rejected> {
        self.routed.partial.fetch_add(1, Ordering::Relaxed);
        self.entry.f64.partial(partial, locked_nu, tol)
    }

    /// `ic` ingested: encode, the free decode of its `z`, the residual and `κ`. The ingest is the free latent encode,
    /// whose inverse is each latent row's, so it is the same for every lowering.
    pub fn ingest<S>(&self, ic: &Ic<f64>) -> Ingested<f64>
    where
        S: Selection,
        S::MassLink: SimplexClamp,
    {
        self.routed.ingest.fetch_add(1, Ordering::Relaxed);
        ingest::<S, f64, _>(ic, phys)
    }
}
