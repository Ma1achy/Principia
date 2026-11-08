//! Saved charts (R-475 S1 to S3, L4 to L6; R-513 Q1 to Q3; R-473 J9; chart_reference §1.1, "Twelve is the full count;
//! Save view as chart commits"): "Save view as chart" commits the current view as a chart of its own, which can be
//! tilted and sliced like any other, so tilts compose.
//!
//! **A latent view** ([`Saved::Oblique`]) is saved as the oblique chart `(z₀, q₁, q₂)`, the stored vectors exactly as
//! the view held them, never re-orthonormalised (under Lean each axis the pair may be skewed, and is kept so), its axes
//! affine: reopened, it is the [`Latent`] chart on those vectors, so it decodes the saved view bit for bit (S3, L6).
//!
//! **A curved view** ([`Saved::Curved`], [`Curved`]) is saved as its parent chart, its id and params (R-475 L5,
//! REQ-CHART-073): its registry id, and the chart value by value, the hidden coordinates' values at the slice centre
//! among its params ([`ParentParams`]), so the definition alone decodes the view ([`Curved::open`]) and a parent with
//! other params is refused ([`Curved::reopen`]); its centre `s` in the parent's own parameters and its tilt, the
//! stored angles, targets and map with the targets' slider spans; `R` is recomputed from them ([`TiltSpec::columns`],
//! [`TiltSpec::inputs`]; R-474 R3). Its own parameters are the saved
//! view's window offsets from `s`, so a new tilt `R′` acts on them first and the saved `R` after ([`Composed`]; L5,
//! R-513):
//! - `(u′, v′, e′) = R′·(ũ, ṽ, 0)`, `(ũ, ṽ)` the pixel's offset from the view's centre `c`, which is also the new
//!   tilt's pivot (R-473 J7), and `e′` the new offsets;
//! - `(Δa, Δb, d̃) = R·(u′ + c − s, v′, w)` (`v′` likewise offset), `w` the new offset on a saved target, entering
//!   `R` at that target's input (Q1), so into the saved target the rotations compose, `R·R′·(u, v, 0)` (option (b)),
//!   the offsets never added;
//! - `(a, b) = s + (Δa, Δb)` through the parent's map exactly; `d̃` scaled by the saved target's slider span and added
//!   to it; a new offset on any other target scaled by its own span and added to that target, which is the block
//!   composition `(R ⊕ 1)·(1 ⊕ R′)` in `(a, b, d_saved, d_new)` (REQ-CHART-073).
//!
//! Both rotations act in R-507 U1's scaled coordinates (Q2): the parent's parameters in units of their full range at
//! zoom 1, as on any tilted chart, and each hidden offset in units of its target's slider span, the saved target's
//! own in `R`'s input, so `R·R′` composes in one scaled space and nothing reads the zoom.
//!
//! **Reopened untilted, bitwise** (S3, Q3, R-473 J6). At `τ′ = 0` the new tilt takes no path and the saved chart is
//! [`Tilted`] on the parent under the saved tilt, the arithmetic of the view it was saved from; with the saved tilt
//! itself zero, the parent's own.
//!
//! **What is refused** (`None` from [`Composed::new`]): a new tilt into a target of a saved Lean each axis view with one
//! target and both angles set, whose skewed `R` has no third input (RQ-746; REQ-CHART-149, built by TASK-M2-56 once
//! ruled); and a composition whose offsets land on more than two hidden coordinates, which one [`Shift`] cannot carry
//! (REQ-CHART-148, built by TASK-M2-55). Saving a saved chart again, the chain `R·R′·R″`, and the composed chart's
//! area weight and the shape sphere's repeat counting are TASK-M2-55's too (REQ-CHART-148); meanwhile
//! [`Saved::curved`] refuses a saved or tilted chart as a parent by type ([`SaveParent`]), never saving it without its
//! tilt.
//!
//! **The serialisation** ([`Saved::to_words`], [`Saved::from_words`]): the definition as [`WORDS`] doubles, by value,
//! so it reads back bit for bit (R-483 H3); a saved chart carries no links (R-478). The words: the kind (0 oblique,
//! 1 curved), then `z₀`, `q₁`, `q₂` for an oblique chart, the rest `+0`; or, for a curved one, the parent's registry
//! index, `s`, `τ₁`, `τ₂`, the map (0 Tip as sheet, 1 Lean each axis), `X`, `Y` and the two spans, then the parent's
//! [`PARAMS`] words ([`ParentParams`]): its form, which of its registry entry's charts it is, then its params, the
//! rest `+0`. A word list that is not one of these, or holds a non-finite value, reads as `None`; params that are not
//! the named parent's are refused on reopening. The layout is the kernel's own (applied per R-369); the export format
//! is REQ-TOOL-065's and REQ-SYS-011's.
//!
//! **The parent's params** ([`ParentParams`]): each value field in declaration order, an enum as its index in
//! declaration order ([`Mapping`], [`ShapeMap`]), a body or other index as itself; each type-level parameter as a
//! code, ahead of the fields: a mixed axis's [`axis_code`] (its [`mixed::Coordinate`] and whether it is derived), an
//! energy axis's [`Quantity`] index; a wrapped chart's params where the wrapper's field is. The link selection a chart
//! is built for is never written: a saved chart carries no links (R-478); the links are `SimConfig`'s own group.

use spirv_std::num_traits::Float;

use super::burrau::{Burrau, EuclidPlane};
use super::burrau_acute::{BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz};
use super::construction::{HeldK, HeldLz, RestStart};
use super::energy::{Mapped, Mapping, Quantity};
use super::invariant::{Invariant, InvariantLE, InvariantLK};
use super::latent::Latent;
use super::mass_simplex::MassSimplex;
use super::mixed;
use super::shape_sphere::{ShapeMap, ShapeSphere};
use super::tilt::{narrow, Placement, Shift, Tilt, TiltMap, TiltSpec, Tiltable, Tilted};
use super::{Chart, REGISTRY};

/// The words of a curved chart's header: the kind, the parent's index, `s`, `τ₁`, `τ₂`, the map, `X`, `Y`, the spans.
const HEADER: usize = 11;

/// The words a parent's params take ([`ParentParams`]): its form, then the longest params, the shape sphere's 19.
pub const PARAMS: usize = 20;

/// The length of a saved chart's serialisation, in doubles: a curved chart's header and its parent's params, longer
/// than the kind and the oblique chart's 24.
pub const WORDS: usize = HEADER + PARAMS;

/// An oblique chart's words: the kind and `z₀`, `q₁`, `q₂`.
const OBLIQUE_WORDS: usize = 25;

const _: () = assert!(OBLIQUE_WORDS <= WORDS);

/// An oblique chart's kind word.
const OBLIQUE: f64 = 0.0;
/// A curved chart's kind word.
const CURVED: f64 = 1.0;

/// A saved curved view: its parent, slice centre and tilt (R-475 S2, L5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curved {
    /// The parent chart's index in [`REGISTRY`].
    pub parent: u32,
    /// The view's centre `s` in the parent's own parameters.
    pub centre: [f64; 2],
    /// The saved tilt: the stored angles, the map and the targets.
    pub tilt: TiltSpec,
    /// The targets' slider spans (R-507 U1).
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

/// A saved curved chart's parent, by value (R-475 L5; R-483 H3; the module's note): its params as words, in the
/// layout the module's note gives, and back. Every [`SaveParent`] has it, and each chart a parent wraps.
pub trait ParentParams: Sized {
    /// Which of its registry entry's charts the parent is, `0` for the entry's own chart: the charts that share a
    /// name ([`Chart::name`]) have distinct forms.
    const FORM: u32 = 0;
    /// The words the params take, the form not counted.
    const LEN: usize;
    /// The params, [`ParentParams::LEN`] words.
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

/// `parent`'s form and params, as [`Curved::params`] holds them.
fn params_of<C: ParentParams>(parent: &C) -> [f64; PARAMS] {
    const {
        assert!(
            C::LEN < PARAMS,
            "a parent's params past the saved chart's words"
        )
    };
    let mut words = [0.0; PARAMS];
    let mut writer = Writer {
        words: &mut words,
        at: 0,
    };
    writer.code(C::FORM);
    parent.write(&mut writer);
    words
}

/// The latent chart is never a curved parent, saving as an oblique chart ([`Saved::curved`] refuses it): it writes
/// nothing and reads as `None`.
impl ParentParams for Latent<f64> {
    const LEN: usize = 0;
    fn write(&self, _: &mut Writer<'_>) {}
    fn read(_: &mut Reader<'_>) -> Option<Self> {
        None
    }
}

/// `map`, `n₀`, `e₁`, `e₂`, `scale`, `inertia`, `phase`, `z_mass`, `z_mom`.
impl ParentParams for ShapeSphere<f64> {
    const LEN: usize = 19;
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
    const LEN: usize = 6;
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
            const LEN: usize = <Invariant<f64> as ParentParams>::LEN;
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

/// The chart's, then each axis's mapping. Form 1: the mapped chart shares its chart's name. The link selection `S`
/// is not written: a saved chart carries no links (R-478).
impl<C: ParentParams, S> ParentParams for Mapped<C, S> {
    const FORM: u32 = 1;
    const LEN: usize = C::LEN + 2;
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
    const LEN: usize = 4;
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

/// The chart's, `K_max`, `γ_K`, the mapping. Form 1: it shares `BurrauThetaLz`'s name.
impl ParentParams for HeldK<f64> {
    const FORM: u32 = 1;
    const LEN: usize = <BurrauThetaLz<f64> as ParentParams>::LEN + 3;
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
    const LEN: usize = 4;
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
    const LEN: usize = 4;
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
    const LEN: usize = C::LEN + 2;
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
    const LEN: usize = 2;
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
    const LEN: usize = 4;
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
    const LEN: usize = 5;
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
    const LEN: usize = C::LEN + 4;
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
/// names its parent ([`Chart::name`]), so saving it as a parent would drop its tilt, the saved `R` with it. Saving a
/// saved chart again, the chain `R·R′·R″`, is TASK-M2-55's (R-475 S1, REQ-CHART-148).
pub trait SaveParent: Chart {}

impl<C: Tiltable> SaveParent for C {}

impl<P: Copy> SaveParent for Latent<P> {}

/// A saved chart's definition (R-475 S2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Saved {
    /// A latent view, as the oblique chart `(z₀, q₁, q₂)`, the stored vectors exactly.
    Oblique(Latent<f64>),
    /// A curved view.
    Curved(Curved),
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
            Saved::Curved(_) => None,
        }
    }

    /// The definition as [`WORDS`] doubles (the module's note).
    pub fn to_words(&self) -> [f64; WORDS] {
        let mut words = [0.0; WORDS];
        match self {
            Saved::Oblique(chart) => {
                words[0] = OBLIQUE;
                let vectors = chart.z0.iter().chain(&chart.q1).chain(&chart.q2);
                for (word, value) in words[1..].iter_mut().zip(vectors) {
                    *word = *value;
                }
            }
            Saved::Curved(c) => {
                let map = match c.tilt.map {
                    TiltMap::Sheet => 0.0,
                    TiltMap::Lean => 1.0,
                };
                let fields = [
                    CURVED,
                    f64::from(c.parent),
                    c.centre[0],
                    c.centre[1],
                    c.tilt.tau1,
                    c.tilt.tau2,
                    map,
                    f64::from(c.tilt.x),
                    f64::from(c.tilt.y),
                    c.span[0],
                    c.span[1],
                ];
                words[..HEADER].copy_from_slice(&fields);
                words[HEADER..].copy_from_slice(&c.params);
            }
        }
        words
    }

    /// The definition [`Saved::to_words`] wrote, bit for bit; `None` where `words` is no definition: an unknown kind,
    /// a parent outside the registry or the latent chart, a map, target or form that is no index, a non-finite value,
    /// or an oblique chart's padding word other than `+0`. A curved chart's params are read on reopening
    /// ([`Curved::open`]).
    pub fn from_words(words: &[f64; WORDS]) -> Option<Self> {
        if !words.iter().all(|w| w.is_finite()) {
            return None;
        }
        let kind = words[0].to_bits();
        if kind == OBLIQUE.to_bits() && words[OBLIQUE_WORDS..].iter().all(|w| w.to_bits() == 0) {
            let mut v = [[0.0; 8]; 3];
            for (k, vector) in v.iter_mut().enumerate() {
                vector.copy_from_slice(&words[1 + 8 * k..9 + 8 * k]);
            }
            Some(Saved::Oblique(Latent {
                z0: v[0],
                q1: v[1],
                q2: v[2],
            }))
        } else if kind == CURVED.to_bits() {
            let any = u64::from(u32::MAX) + 1;
            index(words[HEADER], any)?;
            let mut params = [0.0; PARAMS];
            params.copy_from_slice(&words[HEADER..]);
            let parent = index(words[1], REGISTRY.len() as u64).filter(|&p| p != 0)?;
            let map = match index(words[6], 2)? {
                0 => TiltMap::Sheet,
                _ => TiltMap::Lean,
            };
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

/// The index `x` writes, below `below`: `None` unless `x` is exactly the double of a `u32` below it.
fn index(x: f64, below: u64) -> Option<u32> {
    let k = x as u32;
    (f64::from(k).to_bits() == x.to_bits() && u64::from(k) < below).then_some(k)
}

impl Curved {
    /// The parent, from the definition alone ([`Curved::params`]); `None` where `C` is not the saved parent: another
    /// registry entry or another of its charts, or words that are not `C`'s params.
    pub fn parent<C: Chart + ParentParams>(&self) -> Option<C> {
        let mut words = Reader {
            words: &self.params,
            at: 0,
        };
        words.expect(C::FORM)?;
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

/// Where one of a composition's hidden offsets comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    /// The saved target `X`'s, out of `R`.
    SavedX,
    /// The saved second target `Y`'s, out of `R`.
    SavedY,
    /// The new tilt's `X′`, on a target of its own.
    NewX,
    /// The new tilt's second target `Y′`, on a target of its own.
    NewY,
}

/// A saved curved chart's placement (the module's note): the new tilt `R′` about the view's centre, then the saved `R`
/// about the saved centre, each new offset on a saved target entering `R` at that target's input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Composed<P> {
    /// The saved tilt `R`, about the saved centre `s`.
    saved: Tilt<P>,
    /// The new tilt `R′`, about the view's centre `c`.
    new: Tilt<P>,
    /// `R·e_X` and `R·e_Y` ([`TiltSpec::inputs`]).
    inputs: [[P; 4]; 2],
    /// `into[i][j]`: the new offset `i` (`X′`, `Y′`) enters `R` at the saved target `j`'s input (`X`, `Y`).
    into: [[bool; 2]; 2],
    /// Each offset of the shift, by source.
    sources: [Source; 2],
    /// The hidden coordinates the shift names.
    targets: [u32; 2],
    /// Whether the shift has a second target.
    two: bool,
}

impl Composed<f64> {
    /// The saved tilt `saved` about `saved_centre`, spans `saved_span`, composed after the new tilt `new` about the view's
    /// centre `centre`, spans `span`. `None` where a new offset enters a saved target whose `R` has no input there
    /// ([`TiltSpec::inputs`]; RQ-746, TASK-M2-56), or where the offsets land on more than two hidden coordinates
    /// (TASK-M2-55).
    pub fn new(
        saved: &TiltSpec,
        saved_centre: [f64; 2],
        saved_span: [f64; 2],
        new: &TiltSpec,
        centre: [f64; 2],
        span: [f64; 2],
    ) -> Option<Self> {
        let r = Tilt::new(saved, saved_centre, saved_span);
        let r_new = Tilt::new(new, centre, span);
        let mut composed = Composed {
            saved: r,
            new: r_new,
            inputs: [[0.0; 4]; 2],
            into: [[false, false], [false, false]],
            sources: [Source::SavedX, Source::SavedY],
            targets: [saved.x, saved.y],
            two: r.two,
        };
        if !r_new.on {
            return Some(composed);
        }
        if !r.on {
            composed.sources = [Source::NewX, Source::NewY];
            composed.targets = [new.x, new.y];
            composed.two = r_new.two;
            return Some(composed);
        }
        let saved_targets = [Some(saved.x), r.two.then_some(saved.y)];
        let mut sources = [
            (Source::SavedX, saved.x),
            (Source::SavedY, saved.y),
            (Source::NewX, 0),
            (Source::NewY, 0),
        ];
        let mut count = if r.two { 2 } else { 1 };
        let new_targets = [Some(new.x), r_new.two.then_some(new.y)];
        for (i, target) in new_targets.into_iter().enumerate() {
            let Some(t) = target else { continue };
            match saved_targets.iter().position(|&s| s == Some(t)) {
                Some(j) => composed.into[i][j] = true,
                None => {
                    sources[count] = ([Source::NewX, Source::NewY][i], t);
                    count += 1;
                }
            }
        }
        if count > 2 {
            return None;
        }
        if composed.into.iter().flatten().any(|&fed| fed) {
            composed.inputs = saved.inputs()?;
        }
        composed.sources = [sources[0].0, sources[1].0];
        composed.targets = [sources[0].1, sources[1].1];
        composed.two = count == 2;
        Some(composed)
    }
}

impl<P: Float> Composed<P> {
    /// The same composition with its params at the float `Q`, each rounded, as the kernel's uniform holds them (R-473
    /// J5).
    pub fn at_precision<Q: Float>(&self) -> Composed<Q> {
        Composed {
            saved: tilt_at(&self.saved),
            new: tilt_at(&self.new),
            inputs: self.inputs.map(|c| c.map(narrow)),
            into: self.into,
            sources: self.sources,
            targets: self.targets,
            two: self.two,
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
    /// The saved tilt `R`.
    #[inline]
    pub fn saved(&self) -> &Tilt<P> {
        &self.saved
    }

    /// The new tilt `R′`.
    #[inline]
    pub fn new_tilt(&self) -> &Tilt<P> {
        &self.new
    }
}

impl<P: Copy> Placement<P> for Composed<P> {
    #[inline]
    fn on(&self) -> bool {
        self.saved.on || self.new.on
    }

    /// The module's note: `R′` first, then `R`; with either off, the other alone, with no arithmetic of the one off.
    fn place<R: Float + From<P>>(&self, u: R, v: R) -> ([R; 2], Shift<R>) {
        if !self.new.on {
            return self.saved.place(u, v);
        }
        if !self.saved.on {
            return self.new.place(u, v);
        }
        let w = <R as From<P>>::from;
        let ([cu, cv], [su, sv]) = (self.new.centre, self.saved.centre);
        let [n1, n2] = self.new.columns;
        let (du, dv) = (u - w(cu), v - w(cv));
        let pa = (w(cu) - w(su)) + (w(n1[0]) * du + w(n2[0]) * dv);
        let pb = (w(cv) - w(sv)) + (w(n1[1]) * du + w(n2[1]) * dv);
        let e0 = w(n1[2]) * du + w(n2[2]) * dv;
        let e1 = w(n1[3]) * du + w(n2[3]) * dv;
        let [[x0, y0], [x1, y1]] = self.into;
        let fed = |by_first: bool, by_second: bool| {
            if by_first {
                Some(e0)
            } else if by_second {
                Some(e1)
            } else {
                None
            }
        };
        let (wx, wy) = (fed(x0, x1), fed(y0, y1));
        let [r1, r2] = self.saved.columns;
        let [ix, iy] = self.inputs;
        let at = |kx: P, ky: P| (wx.map(|f| (w(kx), f)), wy.map(|f| (w(ky), f)));
        let (fx, fy) = at(ix[0], iy[0]);
        let da = turned(w(r1[0]), w(r2[0]), pa, pb, fx, fy);
        let (fx, fy) = at(ix[1], iy[1]);
        let db = turned(w(r1[1]), w(r2[1]), pa, pb, fx, fy);
        let (fx, fy) = at(ix[2], iy[2]);
        let dx = turned(w(r1[2]), w(r2[2]), pa, pb, fx, fy);
        let (fx, fy) = at(ix[3], iy[3]);
        let dy = turned(w(r1[3]), w(r2[3]), pa, pb, fx, fy);
        let value = |source: Source| match source {
            Source::SavedX => w(self.saved.span[0]) * dx,
            Source::SavedY => w(self.saved.span[1]) * dy,
            Source::NewX => w(self.new.span[0]) * e0,
            Source::NewY => w(self.new.span[1]) * e1,
        };
        let [first, second] = self.sources;
        let shift = Shift {
            x: self.targets[0],
            dx: value(first),
            y: self.targets[1],
            dy: value(second),
            two: self.two,
        };
        ([w(su) + da, w(sv) + db], shift)
    }

    #[inline]
    fn targets(&self) -> ([u32; 2], bool) {
        (self.targets, self.two)
    }
}
