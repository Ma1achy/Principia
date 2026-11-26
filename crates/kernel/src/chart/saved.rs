//! Saved charts (chart_reference §1.1, "Twelve is the full count;
//! Save view as chart commits"): "Save view as chart" commits the current view as a chart of its own, which can be
//! tilted and sliced like any other, so tilts compose.
//!
//! **A latent view** ([`Saved::Oblique`]) is saved as the oblique chart `(z₀, q₁, q₂)`, the stored vectors exactly as
//! the view held them, never re-orthonormalised (under Lean each axis the pair may be skewed, and is kept so), its axes
//! affine: reopened, it is the [`Latent`] chart on those vectors, so it decodes the saved view bit for bit.
//!
//! **A curved view** ([`Saved::Curved`], [`Curved`]) is saved as its parent chart, its id and params: its
//! registry id, and the chart value by value, the hidden coordinates' values at the slice centre
//! among its params ([`ParentParams`]), so the definition alone decodes the view ([`Curved::open`]) and a parent with
//! other params is refused ([`Curved::reopen`]); its centre `s` in the parent's own parameters and its tilt, the
//! stored angles, targets and map with the targets' slider spans; `R` is recomputed from them ([`TiltSpec::columns`],
//! [`TiltSpec::inputs`]). Its own parameters are the saved
//! view's window offsets from `s`, so a new tilt `R′` acts on them first and the saved `R` after ([`Composed`]):
//! - `(u′, v′, e′) = R′·(ũ, ṽ, 0)`, `(ũ, ṽ)` the pixel's offset from the view's centre `c`, which is also the new
//!   tilt's pivot, and `e′` the new offsets;
//! - `(Δa, Δb, d̃) = R·(u′ + c − s, v′, w)` (`v′` likewise offset), `w` the new offset on a saved target, entering
//!   `R` at that target's input, so into the saved target the rotations compose, `R·R′·(u, v, 0)`,
//!   the offsets never added;
//! - `(a, b) = s + (Δa, Δb)` through the parent's map exactly; `d̃` scaled by the saved target's slider span and added
//!   to it; a new offset on any other target scaled by its own span and added to that target, which is the block
//!   composition `(R ⊕ 1)·(1 ⊕ R′)` in `(a, b, d_saved, d_new)`.
//!
//! Both rotations act in the tilt's scaled coordinates: the parent's parameters in units of their full range at
//! zoom 1, as on any tilted chart, and each hidden offset in units of its target's slider span, the saved target's
//! own in `R`'s input, so `R·R′` composes in one scaled space and nothing reads the zoom.
//!
//! **Reopened untilted, bitwise.** At `τ′ = 0` the new tilt takes no path and the saved chart is
//! [`Tilted`] on the parent under the saved tilt, the arithmetic of the view it was saved from; with the saved tilt
//! itself zero, the parent's own.
//!
//! **A saved Lean each axis view with one target and both angles set**: its
//! columns `û₁`, `û₂` are skewed, no rotation's, and a new offset on its target enters `R`'s third input `t̂`, the
//! saved plane's unit normal `û₁ × û₂` normalised, its sign toward `+e_X` ([`TiltSpec::inputs`]): `R·R′`
//! literally, `R = [û₁ û₂ t̂]`, the composition no rotation, as the saved map is none. So the saved target is offered
//! there too.
//!
//! **What is refused** (`None` from [`Composed::of`]): a new tilt into the target of a saved Lean each axis view at
//! the rank-losing corner, `|τ₁| = |τ₂| = 90°` with one target, which has no frame (`ray-degenerate`); offsets that
//! land on more than four hidden coordinates, which a [`Shift`] carries; and more than [`MAX_LEVELS`] tilts.
//! [`Saved::curved`] refuses a saved or tilted chart as a parent by type ([`SaveParent`]), never saving it without its
//! tilt.
//!
//! **A saved chart saved again** ([`Saved::again`], [`Chain`]): the saved chart under the view's tilt `R′` about its
//! centre, saved, its definition the chain of saved tilts; reopened under a new tilt `R″` it is `R·R′·R″`, each tilt
//! acting on the one inside it, and at `τ″ = 0` the view it was saved from, bit for bit, the tilt that is off taking no
//! arithmetic. An inner tilt's offset on a target an outer tilt also names enters that tilt's `R` at the target's
//! input and is replaced by its output; on any other target it passes on, scaled by its own span. A chain is two
//! records ([`Saved::to_words`], [`Saved::link_words`], [`Saved::from_chain_words`]); a chain is not saved again.
//!
//! **The composed chart's weight and folds** ([`Composed::weight`], [`Composed::rank_loss`],
//! [`Composed::normal_zero`]): with one tilt on, that tilt's, bit for bit. With several, the weight is 1 where every
//! tilt is a rotation, and else the area of the window's steps in the scaled coordinates, the innermost tilt at the
//! corner `ray-degenerate`; the lattice of repeats is folded only where every tilt keeps every hidden offset, since
//! only one tilt's angles decide a fold exactly.
//!
//! **The serialisation** ([`Saved::to_words`], [`Saved::from_words`]): the definition as [`WORDS`] doubles, by value,
//! so it reads back bit for bit; a saved chart carries no links. The words: the kind (2 oblique,
//! 3 curved), then `z₀`, `q₁`, `q₂` for an oblique chart, the rest `+0`; or, for a curved one, the parent's registry
//! index, `s`, `τ₁`, `τ₂`, the map (0 Tip as sheet, 1 Lean each axis), `X`, `Y` and the two spans, then the parent's
//! [`PARAMS`] words ([`ParentParams`]): its form, which of its registry entry's charts it is, then its params, the
//! rest `+0`. A word list that is not one of these, or holds a non-finite value, reads as `None`; params that are not
//! the named parent's are refused on reopening. The plan is unclear on this layout; chose the kernel's own, because
//! chart_reference §1.1 and the export format name what a saved chart carries, not how its words are laid out.
//!
//! **The version, and an old file converted on load.** Kinds 0 and 1
//! are the same layouts written before latent `β` became periodic, their `z_β` made under a retired `β` link
//! (`crate::decode::retired`); [`Saved::from_words`] refuses them, and [`Saved::load`] reads them, converting each
//! stored `z_β` once, `z_β′ = L(z_β) − π/2`, from the link the file pins, else from `sigmoid_beta`, the default it
//! was made under, the assumption reported for the notice on load ([`Loaded::assumed`]). The converted definition
//! writes the current kinds. The stored `z_β` words: an oblique chart's `z₀[1]`, and a curved chart's frozen `z_β`
//! where its parent holds one, the invariant charts' `z_cfg[1]` and the mixed charts' `frozen[1]` ([`beta_word`]); no
//! other parent stores a latent `z_β`. A definition whose plane runs along `z_β`, which the retired link made curved
//! in the periodic coordinate, is refused rather than approximated by a plane: an oblique chart
//! with `q₁[1]` or `q₂[1]` nonzero, a mixed chart with a raw `z_β` axis, a curved chart tilted into `z_β`.
//! A plane's components along `z_β` are directions, which the link cannot convert: such a plane is redefined in the
//! new coordinates, not converted (dd_decoder §3.2, "How a stored value converts").
//!
//! **The parent's params** ([`ParentParams`]): each value field in declaration order, an enum as its index in
//! declaration order ([`Mapping`], [`ShapeMap`]), a body or other index as itself; each type-level parameter as a
//! code, ahead of the fields: a mixed axis's [`axis_code`] (its [`mixed::Coordinate`] and whether it is derived), an
//! energy axis's [`Quantity`] index; a wrapped chart's params where the wrapper's field is. The link selection a chart
//! is built for is never written: a saved chart carries no links; the links are `SimConfig`'s own group.

use spirv_std::num_traits::Float;

use super::burrau::{Burrau, EuclidPlane};
use super::burrau_acute::{BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz};
use super::construction::{HeldK, HeldLz, RestStart};
use super::energy::{Mapped, Mapping, Quantity};
use super::invariant::{Invariant, InvariantLE, InvariantLK};
use super::latent::Latent;
use super::mass_simplex::MassSimplex;
use super::mixed;
use super::shape_sphere::PixelCount;
use super::shape_sphere::{ShapeMap, ShapeSphere};
use super::tilt::{
    image, narrow, Folds, Placement, Shift, Strip, Tilt, TiltMap, TiltSpec, Tiltable, Tilted,
};
use super::{Chart, SystemImage, REGISTRY};
#[cfg(not(target_arch = "spirv"))]
use crate::decode::retired::{RetiredLink, ASSUMED};
use crate::generated::constants::{SAVED_HEADER_WORDS, SAVED_OBLIQUE_WORDS, SAVED_PARENT_WORDS};
use spirv_std::num_traits::FloatConst;

/// The words of a curved chart's header: the kind, the parent's index, `s`, `τ₁`, `τ₂`, the map, `X`, `Y`, the spans
/// (the register's `saved_header_words`).
const HEADER: usize = SAVED_HEADER_WORDS;

/// The words a parent's params take ([`ParentParams`]): its form, then the longest params, the shape sphere's (the
/// register's `saved_parent_words`).
pub const PARAMS: usize = SAVED_PARENT_WORDS;

/// The length of a saved chart's serialisation, in doubles: a curved chart's header and its parent's params, longer
/// than the kind and the oblique chart's 24.
pub const WORDS: usize = HEADER + PARAMS;

/// An oblique chart's words: the kind and `z₀`, `q₁`, `q₂` (the register's `saved_oblique_words`).
const OBLIQUE_WORDS: usize = SAVED_OBLIQUE_WORDS;

const _: () = assert!(OBLIQUE_WORDS <= WORDS);

/// A saved chart's kind, its word the index in declaration order (the module's note).
#[derive(Clone, Copy)]
#[repr(u32)]
enum Kind {
    /// An oblique chart written before latent `β` became periodic, its `z_β` under a retired link.
    // Built only by `Saved::load`, which is off the SPIR-V build; the variant stays there so the kinds keep their words
    // (an index in declaration order), so the SPIR-V build's dead-code lint is allowed on it.
    #[cfg_attr(target_arch = "spirv", allow(dead_code))]
    RetiredOblique,
    /// A curved chart written before latent `β` became periodic, its `z_β` under a retired link.
    // As `RetiredOblique`.
    #[cfg_attr(target_arch = "spirv", allow(dead_code))]
    RetiredCurved,
    /// An oblique chart.
    Oblique,
    /// A curved chart.
    Curved,
    /// A curved chart saved again: the head record of a chain, laid out as a curved chart's.
    Chained,
    /// The tilt a chain's second record holds.
    Link,
}

impl Kind {
    /// The kind's word.
    #[inline]
    fn word(self) -> f64 {
        f64::from(self as u32)
    }
}

/// Which of the charts one registry name covers a parent is ([`ParentParams::FORM`]), its word the index in
/// declaration order: the entry's own chart, then the others that share its name ([`Chart::name`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Form {
    /// The registry entry's own chart.
    Own,
    /// The second: an invariant chart's [`Mapped`], `BurrauThetaLz`'s [`HeldK`], `Mixed`'s [`MixedEnergy`].
    ///
    /// [`MixedEnergy`]: super::physical::MixedEnergy
    Second,
    /// The third: `Mixed`'s [`MixedMassRatio`].
    ///
    /// [`MixedMassRatio`]: super::physical::MixedMassRatio
    Third,
}

/// A saved curved view: its parent, slice centre and tilt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curved {
    /// The parent chart's index in [`REGISTRY`].
    pub parent: u32,
    /// The view's centre `s` in the parent's own parameters.
    pub centre: [f64; 2],
    /// The saved tilt: the stored angles, the map and the targets.
    pub tilt: TiltSpec,
    /// The targets' slider spans.
    pub span: [f64; 2],
    /// The parent chart by value ([`ParentParams`]): its form, then its params, the rest `+0`.
    pub params: [f64; PARAMS],
}

/// The words of a parent's params ([`ParentParams::write`]), in order.
pub struct Writer<'a> {
    words: &'a mut [f64],
    at: usize,
}

impl Writer<'_> {
    /// The value `x`, as it is.
    #[inline]
    pub fn put(&mut self, x: f64) {
        self.words[self.at] = x;
        self.at += 1;
    }

    /// Each of `xs`, in order.
    #[inline]
    pub fn all(&mut self, xs: &[f64]) {
        for &x in xs {
            self.put(x);
        }
    }

    /// The index or code `k`, as its double.
    #[inline]
    pub fn code(&mut self, k: u32) {
        self.put(f64::from(k));
    }

    /// The mapping's index in declaration order.
    #[inline]
    pub fn mapping(&mut self, m: Mapping) {
        self.code(m as u32);
    }
}

/// The words of a parent's params ([`ParentParams::read`]), in order.
pub struct Reader<'a> {
    words: &'a [f64],
    at: usize,
}

impl Reader<'_> {
    /// The next word; `None` past the last.
    #[inline]
    pub fn take(&mut self) -> Option<f64> {
        let x = *self.words.get(self.at)?;
        self.at += 1;
        Some(x)
    }

    /// The next `N` words.
    #[inline]
    pub fn array<const N: usize>(&mut self) -> Option<[f64; N]> {
        let mut out = [0.0; N];
        for x in &mut out {
            *x = self.take()?;
        }
        Some(out)
    }

    /// The next word as an index below `below`; `None` where it is not exactly one.
    #[inline]
    pub fn code(&mut self, below: u64) -> Option<u32> {
        index(self.take()?, below)
    }

    /// The next word, which must be the code `k`.
    #[inline]
    pub fn expect(&mut self, k: u32) -> Option<()> {
        (self.take()?.to_bits() == f64::from(k).to_bits()).then_some(())
    }

    /// The next word as a mapping.
    #[inline]
    pub fn mapping(&mut self) -> Option<Mapping> {
        Some(match self.code(2)? {
            0 => Mapping::Warped,
            _ => Mapping::Linear,
        })
    }

    /// Whether every word left is `+0`.
    #[inline]
    fn rest_zero(&self) -> bool {
        self.words[self.at..].iter().all(|w| w.to_bits() == 0)
    }
}

/// A saved curved chart's parent, by value (the module's note): its params as words, in the
/// layout the module's note gives, and back. Every [`SaveParent`] has it, and each chart a parent wraps.
pub trait ParentParams: Sized {
    /// Which of its registry entry's charts the parent is: the charts that share a name ([`Chart::name`]) have
    /// distinct forms.
    const FORM: Form = Form::Own;
    /// The params, in order ([`params_len`] words).
    fn write(&self, words: &mut Writer<'_>);
    /// The chart [`ParentParams::write`] wrote; `None` where the words are not its params.
    fn read(words: &mut Reader<'_>) -> Option<Self>;
}

/// The code of the mixed axis `Q`: its [`mixed::Coordinate`]'s index, plus 8 for a derived quantity.
pub fn axis_code<Q: mixed::Quantity>() -> u32 {
    Q::COORDINATE as u32 + if Q::DERIVED { 8 } else { 0 }
}

/// The code of the physical quantity `q`: its index in declaration order ([`Quantity::ALL`]).
pub fn quantity_code(q: Quantity) -> u32 {
    q as u32
}

/// `parent`'s form and params, as [`Curved::params`] holds them. Panics where they pass [`PARAMS`] words, which no
/// parent's do (`saved_chart_parent_round_trip`).
fn params_of<C: ParentParams>(parent: &C) -> [f64; PARAMS] {
    let mut words = [0.0; PARAMS];
    let mut writer = Writer {
        words: &mut words,
        at: 0,
    };
    writer.code(C::FORM as u32);
    parent.write(&mut writer);
    words
}

/// The words `parent`'s params take, its form not counted ([`ParentParams::write`]). Panics where they pass
/// [`PARAMS`] words, as [`Saved::curved`] would.
pub fn params_len<C: ParentParams>(parent: &C) -> usize {
    let mut words = [0.0; PARAMS];
    let mut writer = Writer {
        words: &mut words,
        at: 0,
    };
    parent.write(&mut writer);
    writer.at
}

/// The latent chart is never a curved parent, saving as an oblique chart ([`Saved::curved`] refuses it): it writes
/// nothing and reads as `None`.
impl ParentParams for Latent<f64> {
    fn write(&self, _: &mut Writer<'_>) {}
    fn read(_: &mut Reader<'_>) -> Option<Self> {
        None
    }
}

/// `map`, `n₀`, `e₁`, `e₂`, `scale`, `inertia`, `phase`, `z_mass`, `z_mom`.
impl ParentParams for ShapeSphere<f64> {
    fn write(&self, w: &mut Writer<'_>) {
        w.code(self.map as u32);
        for v in [&self.n0, &self.e1, &self.e2] {
            w.all(v);
        }
        w.all(&[self.scale, self.inertia, self.phase]);
        w.all(&self.z_mass);
        w.all(&self.z_mom);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        let map = match r.code(2)? {
            0 => ShapeMap::Spherical,
            _ => ShapeMap::Exponential,
        };
        Some(ShapeSphere {
            map,
            n0: r.array()?,
            e1: r.array()?,
            e2: r.array()?,
            scale: r.take()?,
            inertia: r.take()?,
            phase: r.take()?,
            z_mass: r.array()?,
            z_mom: r.array()?,
        })
    }
}

/// `z_cfg`, `z_mass`, `k_max`, `γ_K`.
impl ParentParams for Invariant<f64> {
    fn write(&self, w: &mut Writer<'_>) {
        w.all(&self.z_cfg);
        w.all(&self.z_mass);
        w.all(&[self.k_max, self.gamma_k]);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(Invariant {
            z_cfg: r.array()?,
            z_mass: r.array()?,
            k_max: r.take()?,
            gamma_k: r.take()?,
        })
    }
}

/// The invariant charts: their [`Invariant`]'s.
macro_rules! invariant_params {
    ($($chart:ident),*) => {$(
        impl ParentParams for $chart<f64> {
            fn write(&self, w: &mut Writer<'_>) {
                self.0.write(w);
            }
            fn read(r: &mut Reader<'_>) -> Option<Self> {
                Invariant::read(r).map($chart)
            }
        }
    )*};
}

invariant_params!(InvariantLE, InvariantLK);

/// The chart's, then each axis's mapping. The second form: the mapped chart shares its chart's name. The link
/// selection `S` is not written: a saved chart carries no links.
impl<C: ParentParams, S> ParentParams for Mapped<C, S> {
    const FORM: Form = Form::Second;
    fn write(&self, w: &mut Writer<'_>) {
        self.chart.write(w);
        w.mapping(self.mapping[0]);
        w.mapping(self.mapping[1]);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        let chart = C::read(r)?;
        Some(Mapped::new(chart, [r.mapping()?, r.mapping()?]))
    }
}

/// `θ_range`, `K`, `λ_L`.
impl ParentParams for BurrauThetaLz<f64> {
    fn write(&self, w: &mut Writer<'_>) {
        w.all(&self.theta_range);
        w.all(&[self.k, self.lz_fraction]);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(BurrauThetaLz {
            theta_range: r.array()?,
            k: r.take()?,
            lz_fraction: r.take()?,
        })
    }
}

/// The chart's, `K_max`, `γ_K`, the mapping. The second form: it shares `BurrauThetaLz`'s name.
impl ParentParams for HeldK<f64> {
    const FORM: Form = Form::Second;
    fn write(&self, w: &mut Writer<'_>) {
        self.chart.write(w);
        w.all(&[self.k_max, self.gamma_k]);
        w.mapping(self.mapping);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(HeldK {
            chart: BurrauThetaLz::read(r)?,
            k_max: r.take()?,
            gamma_k: r.take()?,
            mapping: r.mapping()?,
        })
    }
}

/// `ν_range`, `K_max`, `γ_K`.
impl ParentParams for Burrau<f64> {
    fn write(&self, w: &mut Writer<'_>) {
        w.all(&self.nu_range);
        w.all(&[self.k_max, self.gamma_k]);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(Burrau {
            nu_range: r.array()?,
            k_max: r.take()?,
            gamma_k: r.take()?,
        })
    }
}

/// `θ_range`, `K_max`, `γ_K`.
impl ParentParams for BurrauThetaK<f64> {
    fn write(&self, w: &mut Writer<'_>) {
        w.all(&self.theta_range);
        w.all(&[self.k_max, self.gamma_k]);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(BurrauThetaK {
            theta_range: r.array()?,
            k_max: r.take()?,
            gamma_k: r.take()?,
        })
    }
}

/// The chart's, the held `L_z`, the mapping.
impl<C: ParentParams> ParentParams for HeldLz<C, f64> {
    fn write(&self, w: &mut Writer<'_>) {
        self.chart.write(w);
        w.put(self.lz);
        w.mapping(self.mapping);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(HeldLz {
            chart: C::read(r)?,
            lz: r.take()?,
            mapping: r.mapping()?,
        })
    }
}

/// `ν₀`, `ε_m`.
impl ParentParams for MassSimplex<f64> {
    fn write(&self, w: &mut Writer<'_>) {
        w.all(&[self.nu0, self.eps_m]);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(MassSimplex {
            nu0: r.take()?,
            eps_m: r.take()?,
        })
    }
}

/// `m_range`, `ν_range`.
impl ParentParams for EuclidPlane<f64> {
    fn write(&self, w: &mut Writer<'_>) {
        w.all(&self.m_range);
        w.all(&self.nu_range);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(EuclidPlane {
            m_range: r.array()?,
            nu_range: r.array()?,
        })
    }
}

/// `θ_range`, `m_target`.
impl ParentParams for BurrauThetaDeltaM<f64> {
    fn write(&self, w: &mut Writer<'_>) {
        w.all(&self.theta_range);
        w.all(&self.m_target);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(BurrauThetaDeltaM {
            theta_range: r.array()?,
            m_target: r.array()?,
        })
    }
}

/// The chart's, `z_mom`.
impl<C: ParentParams> ParentParams for RestStart<C, f64> {
    fn write(&self, w: &mut Writer<'_>) {
        self.chart.write(w);
        w.all(&self.z_mom);
    }
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(RestStart {
            chart: C::read(r)?,
            z_mom: r.array()?,
        })
    }
}

/// A chart [`Saved::curved`] takes as a parent: a bare chart, every [`Tiltable`] one and the latent chart (which it
/// refuses, a latent view saving as an oblique chart). A tilted chart, a reopened saved one included, is not one: it
/// names its parent ([`Chart::name`]), so saving it as a parent would drop its tilt, the saved `R` with it; a saved chart
/// saved again goes through [`Saved::again`].
pub trait SaveParent: Chart {}

impl<C: Tiltable> SaveParent for C {}

impl<P: Copy> SaveParent for Latent<P> {}

/// A saved chart's definition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Saved {
    /// A latent view, as the oblique chart `(z₀, q₁, q₂)`, the stored vectors exactly.
    Oblique(Latent<f64>),
    /// A curved view.
    Curved(Curved),
    /// A saved curved chart saved again.
    Chained(Chain),
}

impl Saved {
    /// The latent view `view` saved as an oblique chart: its vectors as they are.
    pub fn latent(view: &Latent<f64>) -> Self {
        Saved::Oblique(*view)
    }

    /// The view of `parent` under `tilt` about `centre`, the targets' spans `span`, saved, `parent` by value
    /// ([`ParentParams`]); `None` where `parent` is the latent chart, which saves as an oblique one, or is not in the
    /// registry. `parent` is a bare chart ([`SaveParent`]): a tilted or reopened saved chart is refused by type, its
    /// tilt never dropped.
    pub fn curved<C: SaveParent + ParentParams>(
        parent: &C,
        tilt: TiltSpec,
        centre: [f64; 2],
        span: [f64; 2],
    ) -> Option<Self> {
        let index = REGISTRY.iter().position(|name| *name == parent.name())?;
        (parent.name() != REGISTRY[0]).then_some(Saved::Curved(Curved {
            parent: index as u32,
            centre,
            tilt,
            span,
            params: params_of(parent),
        }))
    }

    /// The oblique chart, reopened: the latent chart on the stored vectors; `None` for a curved one.
    pub fn oblique(&self) -> Option<Latent<f64>> {
        match self {
            Saved::Oblique(chart) => Some(*chart),
            Saved::Curved(_) | Saved::Chained(_) => None,
        }
    }

    /// A saved curved chart `self` under the new tilt `new` about the view's centre `centre`, spans `span`, saved again:
    /// the chain `R·R′`, `R′` the new tilt. `None` for an oblique chart, which a latent view tilts as it is, and for a
    /// chart that is already a chain, whose reopening under one more tilt is the deepest composition. The plan is
    /// unclear on how deep saving goes; chose two saved tilts, because the kernel has no allocator and the chain is a
    /// `Copy` value of fixed size.
    pub fn again(&self, new: &TiltSpec, centre: [f64; 2], span: [f64; 2]) -> Option<Saved> {
        let Saved::Curved(base) = self else {
            return None;
        };
        Some(Saved::Chained(Chain {
            base: *base,
            link: Link {
                tilt: *new,
                centre,
                span,
            },
        }))
    }

    /// The definition as [`WORDS`] doubles (the module's note).
    pub fn to_words(&self) -> [f64; WORDS] {
        let mut words = [0.0; WORDS];
        match self {
            Saved::Oblique(chart) => {
                words[0] = Kind::Oblique.word();
                let vectors = chart.z0.iter().chain(&chart.q1).chain(&chart.q2);
                for (word, value) in words[1..].iter_mut().zip(vectors) {
                    *word = *value;
                }
            }
            Saved::Curved(c) => words = c.words(Kind::Curved),
            Saved::Chained(chain) => words = chain.base.words(Kind::Chained),
        }
        words
    }

    /// A chain's second record: the tilt it adds ([`Link`]); `None` for a definition that is not a chain.
    pub fn link_words(&self) -> Option<[f64; WORDS]> {
        let Saved::Chained(chain) = self else {
            return None;
        };
        let l = &chain.link;
        let mut words = [0.0; WORDS];
        let fields = [
            Kind::Link.word(),
            l.centre[0],
            l.centre[1],
            l.tilt.tau1,
            l.tilt.tau2,
            map_word(l.tilt.map),
            f64::from(l.tilt.x),
            f64::from(l.tilt.y),
            l.span[0],
            l.span[1],
        ];
        words[..fields.len()].copy_from_slice(&fields);
        Some(words)
    }

    /// The chain `head` and `link` write ([`Saved::to_words`], [`Saved::link_words`]), bit for bit; `None` where either
    /// is no such record, the head is not a chain's, or a padding word is not `+0`.
    pub fn from_chain_words(head: &[f64; WORDS], link: &[f64; WORDS]) -> Option<Saved> {
        let Saved::Curved(base) = Saved::parse(head, Kind::Oblique, Kind::Chained)? else {
            return None;
        };
        if !link.iter().all(|w| w.is_finite()) || link[0].to_bits() != Kind::Link.word().to_bits() {
            return None;
        }
        let any = u64::from(u32::MAX) + 1;
        let tilt = TiltSpec {
            tau1: link[3],
            tau2: link[4],
            map: map_of(link[5])?,
            x: index(link[6], any)?,
            y: index(link[7], any)?,
        };
        link[10..]
            .iter()
            .all(|w| w.to_bits() == 0)
            .then_some(Saved::Chained(Chain {
                base,
                link: Link {
                    tilt,
                    centre: [link[1], link[2]],
                    span: [link[8], link[9]],
                },
            }))
    }

    /// The definition [`Saved::to_words`] wrote, bit for bit; `None` where `words` is no definition: an unknown kind,
    /// a parent outside the registry or the latent chart, a map, target or form that is no index, a non-finite value,
    /// or an oblique chart's padding word other than `+0`. A curved chart's params are read on reopening
    /// ([`Curved::open`]).
    pub fn from_words(words: &[f64; WORDS]) -> Option<Self> {
        Saved::parse(words, Kind::Oblique, Kind::Curved)
    }

    /// The definition `words` hold, an oblique chart where the kind is `oblique`'s, a curved one where it is
    /// `curved`'s ([`Saved::from_words`]).
    fn parse(words: &[f64; WORDS], oblique: Kind, curved: Kind) -> Option<Self> {
        if !words.iter().all(|w| w.is_finite()) {
            return None;
        }
        let kind = words[0].to_bits();
        if kind == oblique.word().to_bits()
            && words[OBLIQUE_WORDS..].iter().all(|w| w.to_bits() == 0)
        {
            let mut v = [[0.0; 8]; 3];
            for (k, vector) in v.iter_mut().enumerate() {
                vector.copy_from_slice(&words[1 + 8 * k..9 + 8 * k]);
            }
            Some(Saved::Oblique(Latent {
                z0: v[0],
                q1: v[1],
                q2: v[2],
            }))
        } else if kind == curved.word().to_bits() {
            let any = u64::from(u32::MAX) + 1;
            index(words[HEADER], any)?;
            let mut params = [0.0; PARAMS];
            params.copy_from_slice(&words[HEADER..]);
            let parent = index(words[1], REGISTRY.len() as u64).filter(|&p| p != 0)?;
            let map = map_of(words[6])?;
            Some(Saved::Curved(Curved {
                parent,
                centre: [words[2], words[3]],
                tilt: TiltSpec {
                    tau1: words[4],
                    tau2: words[5],
                    map,
                    x: index(words[7], any)?,
                    y: index(words[8], any)?,
                },
                span: [words[9], words[10]],
                params,
            }))
        } else {
            None
        }
    }
}

#[cfg(not(target_arch = "spirv"))]
impl Saved {
    /// A saved file's words read (the module's note): a definition of the current kinds as it is; one written before
    /// latent `β` became periodic, kinds 0 and 1, converted from the retired `β` link `pinned`, the link the file pins,
    /// else from [`ASSUMED`], `sigmoid_beta`, the assumption reported ([`Loaded::assumed`]). `None` where `words` is no
    /// definition of either version, or an old one whose plane runs along `z_β` ([`Saved::converted`]).
    pub fn load(words: &[f64; WORDS], pinned: Option<RetiredLink>) -> Option<Loaded> {
        if let Some(saved) = Saved::from_words(words) {
            return Some(Loaded {
                saved,
                assumed: None,
            });
        }
        let old = Saved::parse(words, Kind::RetiredOblique, Kind::RetiredCurved)?;
        let link = pinned.unwrap_or(ASSUMED);
        Some(Loaded {
            saved: old.converted(link)?,
            assumed: pinned.is_none().then_some(link),
        })
    }

    /// The definition with each stored `z_β`, made under the retired link `link`, converted to the periodic `β`
    /// (`z_β′ = link(z_β) − π/2`, at f64), everything else as it is; `None` where its plane runs along `z_β` (the
    /// module's note).
    pub fn converted(&self, link: RetiredLink) -> Option<Self> {
        match *self {
            Saved::Oblique(mut chart) => {
                if chart.q1[1] != 0.0 || chart.q2[1] != 0.0 {
                    return None;
                }
                chart.z0[1] = link.convert(chart.z0[1]);
                Some(Saved::Oblique(chart))
            }
            Saved::Curved(mut c) => {
                if let Some(at) = beta_word(&c)? {
                    c.params[at] = link.convert(c.params[at]);
                }
                Some(Saved::Curved(c))
            }
            Saved::Chained(chain) => Some(Saved::Chained(chain)),
        }
    }
}

/// What [`Saved::load`] read.
#[cfg(not(target_arch = "spirv"))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Loaded {
    /// The definition, of the current kinds.
    pub saved: Saved,
    /// The retired `β` link an old file that pins none was converted from, for the notice on load naming the
    /// assumption; `None` where the file is of the current kinds or pins its link.
    pub assumed: Option<RetiredLink>,
}

#[cfg(not(target_arch = "spirv"))]
/// The latent index of `z_β`, the hidden coordinate a tilt into `z_β` names on the invariant and mixed charts:
/// [`mixed::Coordinate::ZBeta`]'s.
const BETA: u32 = mixed::Coordinate::ZBeta as u32;

#[cfg(not(target_arch = "spirv"))]
/// The mixed axis code ([`axis_code`]) of a raw `z_β` axis, as a params word holds it: [`BETA`], not derived.
const RAW_BETA: f64 = BETA as f64;

#[cfg(not(target_arch = "spirv"))]
/// Where in `c`'s params its parent stores a frozen latent `z_β` (the module's note): `Some(None)` where it stores
/// none, `None` where its plane runs along `z_β`, a raw `z_β` axis or a tilt into `z_β`, which no conversion keeps
/// affine. The offsets count the form word: the invariant charts, either form, `[form, z_cfg[0], z_cfg[1], …]`;
/// `Mixed`, `[form, H, V, frozen…]`; its second form, `[form, A, Q, frozen…]`; its third, `[form, A, frozen…]`.
fn beta_word(c: &Curved) -> Option<Option<usize>> {
    let name = REGISTRY.get(c.parent as usize).copied();
    let form = c.params[0];
    let at = match name {
        Some("InvariantLE" | "InvariantLK") => 2,
        Some("Mixed") if form == f64::from(Form::Own as u32) => {
            if c.params[1] == RAW_BETA || c.params[2] == RAW_BETA {
                return None;
            }
            4
        }
        Some("Mixed") if form == f64::from(Form::Second as u32) => {
            if c.params[1] == RAW_BETA {
                return None;
            }
            4
        }
        Some("Mixed") => {
            if c.params[1] == RAW_BETA {
                return None;
            }
            3
        }
        _ => return Some(None),
    };
    let tilted = c.tilt.tau1 != 0.0 || c.tilt.tau2 != 0.0;
    if tilted && (c.tilt.x == BETA || c.tilt.y == BETA) {
        return None;
    }
    Some(Some(at))
}

/// A tilt map's word: 0 Tip as sheet, 1 Lean each axis.
fn map_word(map: TiltMap) -> f64 {
    match map {
        TiltMap::Sheet => 0.0,
        TiltMap::Lean => 1.0,
    }
}

/// The tilt map `x` writes; `None` where it is neither word.
fn map_of(x: f64) -> Option<TiltMap> {
    Some(match index(x, 2)? {
        0 => TiltMap::Sheet,
        _ => TiltMap::Lean,
    })
}

/// The index `x` writes, below `below`: `None` unless `x` is exactly the double of a `u32` below it.
fn index(x: f64, below: u64) -> Option<u32> {
    let k = x as u32;
    (f64::from(k).to_bits() == x.to_bits() && u64::from(k) < below).then_some(k)
}

impl Curved {
    /// The head record of a curved chart of kind `kind`.
    fn words(&self, kind: Kind) -> [f64; WORDS] {
        let c = self;
        let mut words = [0.0; WORDS];
        let fields = [
            kind.word(),
            f64::from(c.parent),
            c.centre[0],
            c.centre[1],
            c.tilt.tau1,
            c.tilt.tau2,
            map_word(c.tilt.map),
            f64::from(c.tilt.x),
            f64::from(c.tilt.y),
            c.span[0],
            c.span[1],
        ];
        words[..HEADER].copy_from_slice(&fields);
        words[HEADER..].copy_from_slice(&c.params);
        words
    }

    /// The parent, from the definition alone ([`Curved::params`]); `None` where `C` is not the saved parent: another
    /// registry entry or another of its charts, or words that are not `C`'s params.
    pub fn parent<C: Chart + ParentParams>(&self) -> Option<C> {
        let mut words = Reader {
            words: &self.params,
            at: 0,
        };
        words.expect(C::FORM as u32)?;
        let chart = C::read(&mut words)?;
        let named = REGISTRY.get(self.parent as usize) == Some(&chart.name());
        (named && words.rest_zero()).then_some(chart)
    }

    /// The saved chart reopened from its definition alone, on its parent ([`Curved::parent`]), under a new tilt `new`
    /// about the view's centre `centre`, its targets' spans `span` ([`Composed`]); `None` where `C` is not the saved
    /// parent, a target is not one of its hidden coordinates, or [`Composed::new`] refuses the composition.
    pub fn open<C: Tiltable<Param = f64> + ParentParams>(
        &self,
        new: &TiltSpec,
        centre: [f64; 2],
        span: [f64; 2],
    ) -> Option<Tilted<C, f64, Composed<f64>>> {
        let parent = self.parent::<C>()?;
        let composed = Composed::new(&self.tilt, self.centre, self.span, new, centre, span)?;
        Tilted::placed(parent, composed)
    }

    /// [`Curved::open`], where `parent` is the saved parent, its params bitwise the saved ones; `None` for any other.
    pub fn reopen<C: Tiltable<Param = f64> + ParentParams>(
        &self,
        parent: C,
        new: &TiltSpec,
        centre: [f64; 2],
        span: [f64; 2],
    ) -> Option<Tilted<C, f64, Composed<f64>>> {
        let same = params_of(&parent).map(f64::to_bits) == self.params.map(f64::to_bits);
        same.then(|| self.open(new, centre, span)).flatten()
    }
}

/// The tilt a saved-again chart adds to its base: the view's tilt `R′` about its centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    /// The stored tilt.
    pub tilt: TiltSpec,
    /// The view's centre, in the parent's own parameters.
    pub centre: [f64; 2],
    /// The targets' slider spans.
    pub span: [f64; 2],
}

/// A saved curved chart saved again (the module's note): its definition the chain of saved tilts, `R` of the base about
/// its slice centre and `R′` of the link about the view's centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chain {
    /// The saved curved chart: its parent, slice centre and `R`.
    pub base: Curved,
    /// The tilt the view added to it.
    pub link: Link,
}

impl Chain {
    /// The parent ([`Curved::parent`]).
    pub fn parent<C: Chart + ParentParams>(&self) -> Option<C> {
        self.base.parent::<C>()
    }

    /// The chain reopened from its definition alone on its parent, under a new tilt `new` about the view's centre
    /// `centre`, its targets' spans `span`: `R·R′·R″`, each tilt acting on the one inside it ([`Composed`]); `None`
    /// where `C` is not the parent, a target is not hidden, or [`Composed::of`] refuses the composition.
    pub fn open<C: Tiltable<Param = f64> + ParentParams>(
        &self,
        new: &TiltSpec,
        centre: [f64; 2],
        span: [f64; 2],
    ) -> Option<Tilted<C, f64, Composed<f64>>> {
        let parent = self.parent::<C>()?;
        let Link {
            tilt,
            centre: link_centre,
            span: link_span,
        } = self.link;
        let composed = Composed::of(&[
            Level {
                tilt: self.base.tilt,
                centre: self.base.centre,
                span: self.base.span,
            },
            Level {
                tilt,
                centre: link_centre,
                span: link_span,
            },
            Level {
                tilt: *new,
                centre,
                span,
            },
        ])?;
        Tilted::placed(parent, composed)
    }
}

/// The most tilts a composition holds: two saved ([`Chain`]) and the new one.
pub const MAX_LEVELS: usize = 3;

/// The most hidden coordinates one composition's offsets name: the four a [`Shift`] carries.
const MAX_TARGETS: usize = 4;

/// One tilt of a composition: its stored tilt about its centre, the targets' slider spans.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    /// The stored tilt.
    pub tilt: TiltSpec,
    /// The tilt's centre, in the parent's own parameters.
    pub centre: [f64; 2],
    /// The targets' slider spans.
    pub span: [f64; 2],
}

/// A saved curved chart's placement (the module's note): the tilts of a composition, each acting on the one inside it,
/// the new tilt `R′` about the view's centre first and the saved `R` last, each offset a tilt takes on a target an inner
/// tilt also offsets entering that tilt's input at the target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Composed<P> {
    /// The tilts, outermost (the saved `R`) first; the ones past `count` are off.
    levels: [Tilt<P>; MAX_LEVELS],
    /// How many tilts the composition was built from.
    count: usize,
    /// `R·e_X` and `R·e_Y` of each tilt ([`TiltSpec::inputs`]), set where an inner tilt feeds it.
    inputs: [[[P; 4]; 2]; MAX_LEVELS],
    /// Whether an inner tilt's offset on a tilt's target `X` or `Y` enters its `R` at that target's input.
    fed: [[bool; 2]; MAX_LEVELS],
    /// The slot of the shift each tilt's targets write.
    slot: [[usize; 2]; MAX_LEVELS],
    /// The hidden coordinates the shift names, by slot; a slot never filled holds the outermost tilt's `Y` (the second)
    /// or none.
    targets: [u32; MAX_TARGETS],
    /// How many of `targets` are filled.
    filled: usize,
    /// Each pixel's area weight ([`Composed::weight`]).
    weight: P,
    /// Whether the view loses rank ([`Composed::rank_loss`]).
    rank_loss: bool,
    /// `[N_a = 0, N_b = 0]` ([`Composed::normal_zero`]).
    normal_zero: [bool; 2],
    /// The lattice of repeats ([`Composed::strip`]).
    strip: Option<Strip>,
}

impl Composed<f64> {
    /// The saved tilt `saved` about `saved_centre`, spans `saved_span`, composed after the new tilt `new` about the view's
    /// centre `centre`, spans `span`. `None` where a new offset enters a saved target whose `R` has no input there, at
    /// the rank-losing corner ([`TiltSpec::inputs`]), or where the offsets land on more than four hidden coordinates.
    pub fn new(
        saved: &TiltSpec,
        saved_centre: [f64; 2],
        saved_span: [f64; 2],
        new: &TiltSpec,
        centre: [f64; 2],
        span: [f64; 2],
    ) -> Option<Self> {
        Composed::of(&[
            Level {
                tilt: *saved,
                centre: saved_centre,
                span: saved_span,
            },
            Level {
                tilt: *new,
                centre,
                span,
            },
        ])
    }

    /// The composition of `levels`, outermost (the saved `R`) first, at most [`MAX_LEVELS`]; `None` past them, where an
    /// offset enters a tilt whose `R` has no input at its target, or where the offsets land on more than four hidden
    /// coordinates. A tilt that is off takes no arithmetic; with one tilt on, the composition is that tilt's.
    pub fn of(levels: &[Level]) -> Option<Self> {
        if levels.is_empty() || levels.len() > MAX_LEVELS {
            return None;
        }
        let off = Tilt::new(&levels[0].tilt, levels[0].centre, levels[0].span);
        let mut composed = Composed {
            levels: [off; MAX_LEVELS],
            count: levels.len(),
            inputs: [[[0.0; 4]; 2]; MAX_LEVELS],
            fed: [[false; 2]; MAX_LEVELS],
            slot: [[0; 2]; MAX_LEVELS],
            targets: [levels[0].tilt.x, levels[0].tilt.y, u32::MAX, u32::MAX],
            filled: 0,
            weight: 1.0,
            rank_loss: false,
            normal_zero: [true, true],
            strip: None,
        };
        let mut skewed = false;
        for (k, level) in levels.iter().enumerate() {
            composed.levels[k] = Tilt::new(&level.tilt, level.centre, level.span);
            let t = &composed.levels[k];
            skewed |= t.on
                && level.tilt.map == TiltMap::Lean
                && !t.two
                && !level.tilt.untilted()
                && level.tilt.tau1 != 0.0
                && level.tilt.tau2 != 0.0;
        }
        let on = |k: usize| composed.levels[k].on;
        let first = (0..levels.len()).find(|&k| on(k));
        let Some(first) = first else {
            composed.filled = if composed.levels[0].two { 2 } else { 1 };
            composed.slot[0] = [0, 1];
            return Some(composed);
        };
        // The slots, outermost tilt first: a target an outer tilt already names keeps its slot.
        for k in first..levels.len() {
            if !on(k) {
                continue;
            }
            let t = &composed.levels[k];
            let named = [(t.x, true), (t.y, t.two)];
            for (j, (target, active)) in named.into_iter().enumerate() {
                if !active {
                    continue;
                }
                let at = composed.targets[..composed.filled]
                    .iter()
                    .position(|&n| n == target);
                composed.slot[k][j] = match at {
                    Some(s) => s,
                    None => {
                        if composed.filled == MAX_TARGETS {
                            return None;
                        }
                        composed.targets[composed.filled] = target;
                        composed.filled += 1;
                        composed.filled - 1
                    }
                };
            }
        }
        // An inner tilt's offset on a target feeds the outer tilts that name it.
        for k in first..levels.len() {
            if !on(k) {
                continue;
            }
            let t = &composed.levels[k];
            for (j, (target, active)) in [(t.x, true), (t.y, t.two)].into_iter().enumerate() {
                let inner = (k + 1..levels.len()).any(|m| {
                    let n = &composed.levels[m];
                    n.on && (n.x == target || (n.two && n.y == target))
                });
                composed.fed[k][j] = active && inner;
            }
            if composed.fed[k].iter().any(|&f| f) {
                composed.inputs[k] = levels[k].tilt.inputs()?;
            }
        }
        let active = (first..levels.len()).filter(|&k| on(k));
        let single = active.clone().count() == 1;
        if single {
            let t = &composed.levels[first];
            composed.weight = t.weight;
            composed.rank_loss = t.rank_loss;
            composed.normal_zero = t.normal_zero;
            composed.strip = t.strip;
        } else {
            let inner = active.clone().next_back().unwrap_or(first);
            composed.rank_loss = composed.levels[inner].rank_loss;
            composed.normal_zero = if active
                .clone()
                .all(|k| composed.levels[k].normal_zero == [true, true])
            {
                [true, true]
            } else {
                [false, false]
            };
            composed.weight = if composed.rank_loss {
                0.0
            } else if skewed {
                composed.gram()
            } else {
                1.0
            };
        }
        Some(composed)
    }

    /// The area of the parallelogram the window's two unit steps map to in `(Δa, Δb, d̃…)`, the offsets in their scaled
    /// units, as the root of the sum of its squared 2 × 2 minors, which no rounding makes negative.
    fn gram(&self) -> f64 {
        let (cu, cv) = (self.run(1.0, 0.0, false), self.run(0.0, 1.0, false));
        let col =
            |r: &([f64; 2], [f64; 4], [f64; 4])| [r.0[0], r.0[1], r.1[0], r.1[1], r.1[2], r.1[3]];
        let (a, b) = (col(&cu), col(&cv));
        let mut sum = 0.0;
        for i in 0..6 {
            for j in i + 1..6 {
                let minor = a[i] * b[j] - a[j] * b[i];
                sum += minor * minor;
            }
        }
        Float::sqrt(sum)
    }
}

impl<P: Float> Composed<P> {
    /// The same composition with its params at the float `Q`, each rounded, as the kernel's uniform holds them.
    pub fn at_precision<Q: Float>(&self) -> Composed<Q> {
        Composed {
            levels: self.levels.map(|t| tilt_at(&t)),
            count: self.count,
            inputs: self.inputs.map(|i| i.map(|c| c.map(narrow))),
            fed: self.fed,
            slot: self.slot,
            targets: self.targets,
            filled: self.filled,
            weight: narrow(self.weight),
            rank_loss: self.rank_loss,
            normal_zero: self.normal_zero,
            strip: self.strip,
        }
    }
}

/// A tilt's params at the float `Q`, each rounded.
fn tilt_at<P: Float, Q: Float>(t: &Tilt<P>) -> Tilt<Q> {
    Tilt {
        on: t.on,
        two: t.two,
        rank_loss: t.rank_loss,
        centre: t.centre.map(narrow),
        columns: t.columns.map(|c| c.map(narrow)),
        x: t.x,
        y: t.y,
        span: t.span.map(narrow),
        weight: narrow(t.weight),
        normal_zero: t.normal_zero,
        strip: t.strip,
    }
}

/// One component of `R·(u′, v′, w)`: `R·e_a`'s and `R·e_b`'s, then each fed input's, `(input, w)`.
#[inline]
fn turned<R: Float>(ra: R, rb: R, u: R, v: R, fx: Option<(R, R)>, fy: Option<(R, R)>) -> R {
    let x = ra * u + rb * v;
    let x = match fx {
        Some((input, w)) => x + input * w,
        None => x,
    };
    match fy {
        Some((input, w)) => x + input * w,
        None => x,
    }
}

impl<P: Copy> Composed<P> {
    /// The saved tilt `R`, the outermost.
    #[inline]
    pub fn saved(&self) -> &Tilt<P> {
        &self.levels[0]
    }

    /// The new tilt `R′`, the innermost.
    #[inline]
    pub fn new_tilt(&self) -> &Tilt<P> {
        &self.levels[self.count - 1]
    }

    /// Each pixel's area weight: the one tilt's where one is on; with several, the area of the parallelogram the
    /// window's unit steps map to in `(Δa, Δb, d̃…)`, exactly 1 where every tilt is a rotation, which it is but for
    /// Lean each axis with one target and both angles set, whose columns are skewed ([`TiltSpec::area_weight`]); 0 at
    /// the rank-losing corner.
    #[inline]
    pub fn weight(&self) -> P {
        self.weight
    }

    /// Whether the view loses rank, `ray-degenerate`: decided on the stored angles, the innermost tilt that is on at its
    /// rank-losing corner ([`TiltSpec::rank_loss`]). The plan is unclear on a composition whose outer tilt is at the
    /// corner and an inner one offsets another target; chose to keep its rank, since the inner offsets still span the
    /// plane, and to weigh it by the area it has.
    #[inline]
    pub fn rank_loss(&self) -> bool {
        self.rank_loss
    }

    /// `[N_a = 0, N_b = 0]`: the one tilt's where one is on; with several `[true, true]` where every tilt's is, its
    /// hidden offsets all nominally zero, and `[false, false]` elsewhere. The plan is unclear on the repeats of a
    /// composition of tilts that move the offsets; chose to fold none, counting each such pixel as a system of its
    /// own, because only the closed forms of one tilt's angles decide a fold exactly, and an over-count is the safer
    /// error.
    #[inline]
    pub fn normal_zero(&self) -> [bool; 2] {
        self.normal_zero
    }

    /// The lattice of repeats of the one tilt that is on; none with several.
    #[inline]
    pub fn strip(&self) -> Option<Strip> {
        self.strip
    }
}

impl<P: Copy> Composed<P> {
    /// The pixel at `(u, v)` through the tilts, innermost first: `(Δa, Δb)` from the outermost centre, each target's
    /// offset before its span, and each slot's span. With `pan` off the centres are taken as one point, which the area
    /// of the window's steps does not read.
    fn run<R: Float + From<P>>(&self, u: R, v: R, pan: bool) -> ([R; 2], [R; 4], [R; 4]) {
        let w = <R as From<P>>::from;
        let zero = R::zero();
        let (mut cur, mut span) = ([zero; 4], [zero; 4]);
        let mut prev: Option<(usize, [R; 2])> = None;
        let mut out = [zero; 2];
        for k in (0..self.count).rev() {
            let t = &self.levels[k];
            if !t.on {
                continue;
            }
            let (pa, pb) = match prev {
                None if pan => (u - w(t.centre[0]), v - w(t.centre[1])),
                None => (u, v),
                Some((m, [da, db])) if pan => {
                    let c = (self.levels[m].centre, t.centre);
                    ((w(c.0[0]) - w(c.1[0])) + da, (w(c.0[1]) - w(c.1[1])) + db)
                }
                Some((_, [da, db])) => (da, db),
            };
            let [r1, r2] = t.columns;
            let [ix, iy] = self.inputs[k];
            let fed = |j: usize, slot: usize| {
                if self.fed[k][j] {
                    Some(cur[slot])
                } else {
                    None
                }
            };
            let (wx, wy) = (fed(0, self.slot[k][0]), fed(1, self.slot[k][1]));
            let at = |kx: P, ky: P| (wx.map(|f| (w(kx), f)), wy.map(|f| (w(ky), f)));
            let (fx, fy) = at(ix[0], iy[0]);
            let da = turned(w(r1[0]), w(r2[0]), pa, pb, fx, fy);
            let (fx, fy) = at(ix[1], iy[1]);
            let db = turned(w(r1[1]), w(r2[1]), pa, pb, fx, fy);
            let (fx, fy) = at(ix[2], iy[2]);
            let dx = turned(w(r1[2]), w(r2[2]), pa, pb, fx, fy);
            let (fx, fy) = at(ix[3], iy[3]);
            let dy = turned(w(r1[3]), w(r2[3]), pa, pb, fx, fy);
            cur[self.slot[k][0]] = dx;
            span[self.slot[k][0]] = w(t.span[0]);
            if t.two {
                cur[self.slot[k][1]] = dy;
                span[self.slot[k][1]] = w(t.span[1]);
            }
            prev = Some((k, [da, db]));
            out = [da, db];
        }
        (out, cur, span)
    }
}

impl<P: Copy> Placement<P> for Composed<P> {
    #[inline]
    fn on(&self) -> bool {
        self.levels.iter().any(|t| t.on)
    }

    /// The module's note: the innermost tilt first, the saved `R` last; a tilt that is off takes no arithmetic, and
    /// with one on, its own placement.
    fn place<R: Float + From<P>>(&self, u: R, v: R) -> ([R; 2], Shift<R>) {
        let mut on = (0..self.count).filter(|&k| self.levels[k].on);
        match (on.next(), on.next()) {
            (None, _) => return self.levels[0].place(u, v),
            (Some(k), None) => return self.levels[k].place(u, v),
            _ => {}
        }
        let ([da, db], cur, span) = self.run(u, v, true);
        let w = <R as From<P>>::from;
        let outer = self
            .levels
            .iter()
            .position(|t| t.on)
            .map_or(self.levels[0].centre, |k| self.levels[k].centre);
        let value = |s: usize| span[s] * cur[s];
        let shift = Shift {
            x: self.targets[0],
            dx: value(0),
            y: self.targets[1],
            dy: if self.filled >= 2 {
                value(1)
            } else {
                R::zero()
            },
            two: self.filled >= 2,
            z: self.targets[2],
            dz: if self.filled >= 3 {
                value(2)
            } else {
                R::zero()
            },
            w: self.targets[3],
            dw: if self.filled >= 4 {
                value(3)
            } else {
                R::zero()
            },
            more: if self.filled >= 2 {
                (self.filled - 2) as u32
            } else {
                0
            },
        };
        ([w(outer[0]) + da, w(outer[1]) + db], shift)
    }

    #[inline]
    fn targets(&self) -> ([u32; 2], bool) {
        ([self.targets[0], self.targets[1]], self.filled >= 2)
    }

    #[inline]
    fn area_weight(&self) -> P {
        self.weight
    }

    #[inline]
    fn more_targets(&self) -> ([u32; 2], u32) {
        let more = self.filled.saturating_sub(2);
        ([self.targets[2], self.targets[3]], more as u32)
    }
}

impl<P: Copy + FloatConst> Tilted<ShapeSphere<P>, P, Composed<P>>
where
    f64: From<P>,
{
    fn folds(&self) -> Folds {
        let c = self.tilt();
        Folds {
            normal_zero: c.normal_zero,
            strip: c.strip,
        }
    }

    /// The composed view's `system_image`: `ray-degenerate` where it loses rank ([`Composed::rank_loss`]), else the
    /// chart's own.
    pub fn system_image(&self) -> SystemImage {
        image(self.tilt().rank_loss, self.chart().system_image())
    }

    /// How the composed view's pixel at `(u, v)` counts ([`Tilted::pixel`]), under the composition's folds.
    pub fn pixel(&self, u: f64, v: f64) -> Option<PixelCount> {
        self.pixel_by(self.folds(), u, v)
    }

    /// The counted total over the pixels at `points` ([`Tilted::counted_total`]), under the composition's folds.
    pub fn counted_total<I>(&self, points: I) -> u64
    where
        I: IntoIterator<Item = [f64; 2]>,
        I::IntoIter: Clone,
    {
        self.counted_by(self.folds(), points)
    }
}
