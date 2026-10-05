//! The synthetic payload harness (debug tooling plan, "Principle" and Phase 0's step 0b; render contract Part 6, "Why
//! first"): a payload set filled on the CPU with hand-chosen values, before any physics exists, for the fragment to
//! render and the tests to assert on. [`Synthetic`] holds the four buffers' contents: the `SimState` buffer and the
//! word buffer, one element per sample, `ICDescriptor`, one per sample, and `RenderQuad`, one per quad, over a flat
//! grid of quads ([`Synthetic::flat`]).
//!
//! Every write goes through the generated layout: a sample's packed words through the generated pack routines
//! (`kernel::payload`, payload §6), its word through `fgw_pack`, and a quad's members by their names in the ledger's
//! `RenderQuad` table (`ledger::quad`, dd_generation_root §3.7a), each at its generated offset. The bytes are the
//! generated structs' members in order, little-endian, as the GPU reads them.

use kernel::payload::{
    fgw_pack, pack_packed_b, pack_times, set_d_min, set_d_min_unset, set_detail, set_dmin_pair,
    set_last_symbol, set_saturated, set_state, DminCounters, ICDescriptor, SimStateFTLE,
    SD_DMIN_PAIR_SENTINEL, STATE_RUNNING,
};
use ledger::quad::RENDER_QUAD;
use ledger::schema::Storage;
use render::bind::Payload;
use render::raster::Grid;

/// `RenderQuad`'s size in words (dd_generation_root §3.7a: each member one 4-byte scalar).
pub const QUAD_WORDS: usize = RENDER_QUAD.len();

/// A CPU-filled payload set over a grid of quads.
#[derive(Debug)]
pub struct Synthetic {
    grid: Grid,
    simstate: Vec<SimStateFTLE>,
    word: Vec<[u32; 4]>,
    ic: Vec<ICDescriptor>,
    quad: Vec<[u32; QUAD_WORDS]>,
    counters: DminCounters,
}

/// The four buffers' contents as bytes, for [`render::bind::upload`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bytes {
    pub simstate: Vec<u8>,
    pub word: Vec<u8>,
    pub ic: Vec<u8>,
    pub quad: Vec<u8>,
}

impl Bytes {
    /// The buffers as the binding takes them.
    pub fn payload(&self) -> Payload<'_> {
        Payload {
            simstate: &self.simstate,
            word: &self.word,
            ic: &self.ic,
            quad: &self.quad,
        }
    }
}

impl Synthetic {
    /// A flat layout: `grid`'s quads all at quadtree depth `depth`, tiling the slice, each `quad_state` loaded (code
    /// 0) and its other members zero. Every sample starts fresh: `running`, `d_min` unset, `dmin_pair` at its sentinel,
    /// the empty word, and equal masses summing to 1; every other member zero.
    pub fn flat(grid: Grid, depth: u32) -> Synthetic {
        let fresh = {
            let w = set_state(0, STATE_RUNNING);
            let w = set_dmin_pair(w, SD_DMIN_PAIR_SENTINEL);
            SimStateFTLE {
                packed_a: set_d_min_unset(w),
                ..SimStateFTLE::default()
            }
        };
        let third = 1.0 / 3.0;
        let ic = ICDescriptor {
            m0: third,
            m1: third,
            m2: third,
            ..ICDescriptor::default()
        };
        let samples = grid.sample_count() as usize;
        let mut quad = [0u32; QUAD_WORDS];
        quad[quad_slot("quad_depth")
            .expect("RenderQuad has quad_depth")
            .0] = depth;
        Synthetic {
            grid,
            simstate: vec![fresh; samples],
            word: vec![fgw_pack([0; 4], 0); samples],
            ic: vec![ic; samples],
            quad: vec![quad; grid.quad_count() as usize],
            counters: DminCounters::new(),
        }
    }

    /// The grid the set covers.
    pub fn grid(&self) -> Grid {
        self.grid
    }

    /// Sample `i`'s setters (its index as `Grid::sample_index` gives it).
    pub fn sample(&mut self, i: u32) -> Sample<'_> {
        Sample {
            s: &mut self.simstate[i as usize],
            word: &mut self.word[i as usize],
            counters: &self.counters,
        }
    }

    /// Sample `i`'s stored `SimState`, as written.
    pub fn simstate(&self, i: u32) -> &SimStateFTLE {
        &self.simstate[i as usize]
    }

    /// Sample `i`'s word, as written.
    pub fn word(&self, i: u32) -> [u32; 4] {
        self.word[i as usize]
    }

    /// Sample `i`'s `ICDescriptor`, written in place.
    pub fn ic(&mut self, i: u32) -> &mut ICDescriptor {
        &mut self.ic[i as usize]
    }

    /// Quad `q`'s setters (its index as `Grid::cell` gives it).
    pub fn quad(&mut self, q: u32) -> Quad<'_> {
        Quad {
            words: &mut self.quad[q as usize],
        }
    }

    /// Quad `q`'s words, as written.
    pub fn quad_words(&self, q: u32) -> [u32; QUAD_WORDS] {
        self.quad[q as usize]
    }

    /// The `d_min` packs' counts so far, `(dmin_nan_unset, dmin_negative_floored)` (R-288).
    pub fn dmin_counts(&self) -> (u32, u32) {
        self.counters.read()
    }

    /// The four buffers' bytes.
    pub fn bytes(&self) -> Bytes {
        let mut out = Bytes {
            simstate: Vec::new(),
            word: Vec::new(),
            ic: Vec::new(),
            quad: Vec::new(),
        };
        for s in &self.simstate {
            out.simstate
                .extend(simstate_words(s).iter().flat_map(|w| w.to_le_bytes()));
        }
        for w in &self.word {
            out.word.extend(w.iter().flat_map(|w| w.to_le_bytes()));
        }
        for d in &self.ic {
            out.ic
                .extend(ic_words(d).iter().flat_map(|w| w.to_le_bytes()));
        }
        for q in &self.quad {
            out.quad.extend(q.iter().flat_map(|w| w.to_le_bytes()));
        }
        out
    }
}

/// `SimStateFTLE`'s 144 B as words, its members in order: the u16 `closure_step` and `_reserved` share one word, the
/// step in its low half, as little-endian memory holds them.
pub fn simstate_words(s: &SimStateFTLE) -> [u32; 36] {
    let mut w = [0u32; 36];
    let groups = [s.r, s.p, s.r_sh, s.p_sh];
    for (g, group) in groups.iter().enumerate() {
        for (k, x) in group.iter().flatten().enumerate() {
            w[6 * g + k] = x.to_bits();
        }
    }
    let rest = [
        s.S.to_bits(),
        s.theta.to_bits(),
        s.mean_y.to_bits(),
        s.C_ty.to_bits(),
        s.E_0.to_bits(),
        s.Lz_0.to_bits(),
        s.packed_a,
        s.packed_b,
        s.times,
        s.total_substeps,
        s.closure_min.to_bits(),
        u32::from(s.closure_step) | u32::from(s._reserved) << 16,
    ];
    w[24..].copy_from_slice(&rest);
    w
}

/// `ICDescriptor`'s 64 B as words: its twelve f32s, then its declared padding.
pub fn ic_words(d: &ICDescriptor) -> [u32; 16] {
    let f = [
        d.m0,
        d.m1,
        d.m2,
        d.q_mass,
        d.rho_mag,
        d.lambda_mag,
        d.rho_ratio,
        d.rho_angle,
        d.K_0,
        d.V_0,
        d.virial_ratio,
        d.r_min_pair_0,
    ];
    let mut w = [0u32; 16];
    for (k, x) in f.iter().enumerate() {
        w[k] = x.to_bits();
    }
    w[12..].copy_from_slice(&d._pad);
    w
}

/// One sample's setters, each a field of its `SimState` or its word, written through the generated pack routines.
pub struct Sample<'a> {
    s: &'a mut SimStateFTLE,
    word: &'a mut [u32; 4],
    counters: &'a DminCounters,
}

#[allow(non_snake_case)] // The setters keep the ledger's field names, `S`, `C_ty`, `E_0` and `Lz_0` (payload §1).
impl Sample<'_> {
    pub fn r(&mut self, v: [[f32; 2]; 3]) -> &mut Self {
        self.s.r = v;
        self
    }

    pub fn p(&mut self, v: [[f32; 2]; 3]) -> &mut Self {
        self.s.p = v;
        self
    }

    pub fn r_sh(&mut self, v: [[f32; 2]; 3]) -> &mut Self {
        self.s.r_sh = v;
        self
    }

    pub fn p_sh(&mut self, v: [[f32; 2]; 3]) -> &mut Self {
        self.s.p_sh = v;
        self
    }

    pub fn S(&mut self, v: f32) -> &mut Self {
        self.s.S = v;
        self
    }

    pub fn theta(&mut self, v: f32) -> &mut Self {
        self.s.theta = v;
        self
    }

    pub fn mean_y(&mut self, v: f32) -> &mut Self {
        self.s.mean_y = v;
        self
    }

    pub fn C_ty(&mut self, v: f32) -> &mut Self {
        self.s.C_ty = v;
        self
    }

    pub fn E_0(&mut self, v: f32) -> &mut Self {
        self.s.E_0 = v;
        self
    }

    pub fn Lz_0(&mut self, v: f32) -> &mut Self {
        self.s.Lz_0 = v;
        self
    }

    pub fn total_substeps(&mut self, v: u32) -> &mut Self {
        self.s.total_substeps = v;
        self
    }

    pub fn closure_min(&mut self, v: f32) -> &mut Self {
        self.s.closure_min = v;
        self
    }

    pub fn closure_step(&mut self, v: u16) -> &mut Self {
        self.s.closure_step = v;
        self
    }

    /// `state`, through `set_state` (payload §2).
    pub fn state(&mut self, v: u32) -> &mut Self {
        self.s.packed_a = set_state(self.s.packed_a, v);
        self
    }

    pub fn detail(&mut self, v: u32) -> &mut Self {
        self.s.packed_a = set_detail(self.s.packed_a, v);
        self
    }

    pub fn saturated(&mut self, v: bool) -> &mut Self {
        self.s.packed_a = set_saturated(self.s.packed_a, v);
        self
    }

    pub fn dmin_pair(&mut self, v: u32) -> &mut Self {
        self.s.packed_a = set_dmin_pair(self.s.packed_a, v);
        self
    }

    pub fn last_symbol(&mut self, v: u32) -> &mut Self {
        self.s.packed_a = set_last_symbol(self.s.packed_a, v);
        self
    }

    /// `d_min`, through `set_d_min`: +∞ writes it unset (R-271).
    pub fn d_min(&mut self, v: f32) -> &mut Self {
        self.s.packed_a = set_d_min(self.s.packed_a, v, self.counters);
        self
    }

    /// `packed_a` written whole: a bitwise-adversarial descriptor (debug tooling plan, "Synthetic-first").
    pub fn packed_a(&mut self, v: u32) -> &mut Self {
        self.s.packed_a = v;
        self
    }

    /// `dE_max` and `dLz_max`, through `pack_packed_b`.
    pub fn drift_max(&mut self, de_max: f32, dlz_max: f32) -> &mut Self {
        self.s.packed_b = pack_packed_b(de_max, dlz_max);
        self
    }

    /// `t_end_step` and `t_dmin_step`, through `pack_times`.
    pub fn times(&mut self, t_end_step: u32, t_dmin_step: u32) -> &mut Self {
        self.s.times = pack_times(t_end_step, t_dmin_step);
        self
    }

    /// The word holding the mixed-radix `W` (four limbs, low first) and `length`, through `fgw_pack` (payload §3).
    pub fn word(&mut self, w: [u32; 4], length: u32) -> &mut Self {
        *self.word = fgw_pack(w, length);
        self
    }

    /// The word written whole, its `.w` included.
    pub fn word_raw(&mut self, w: [u32; 4]) -> &mut Self {
        *self.word = w;
        self
    }
}

/// `name`'s slot in `RenderQuad` and its storage, by the ledger's table.
fn quad_slot(name: &str) -> Option<(usize, Storage)> {
    RENDER_QUAD
        .iter()
        .position(|f| f.name == name)
        .map(|k| (k, RENDER_QUAD[k].storage))
}

/// One quad's setters: each member by its name in the ledger's `RenderQuad` table, at its slot, of its type.
pub struct Quad<'a> {
    words: &'a mut [u32; QUAD_WORDS],
}

impl Quad<'_> {
    fn set(&mut self, name: &str, storage: Storage, bits: u32) -> Result<&mut Self, String> {
        match quad_slot(name) {
            Some((k, s)) if s == storage => {
                self.words[k] = bits;
                Ok(self)
            }
            Some((_, s)) => Err(format!("RenderQuad's `{name}` is {s:?}, not {storage:?}")),
            None => Err(format!("RenderQuad has no member `{name}`")),
        }
    }

    /// The u32 member `name`.
    pub fn u32(&mut self, name: &str, v: u32) -> Result<&mut Self, String> {
        self.set(name, Storage::U32, v)
    }

    /// The f32 member `name`.
    pub fn f32(&mut self, name: &str, v: f32) -> Result<&mut Self, String> {
        self.set(name, Storage::F32, v.to_bits())
    }
}
