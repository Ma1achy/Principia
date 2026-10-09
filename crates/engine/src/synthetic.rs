//! The synthetic payload harness (debug tooling plan, "Principle" and Phase 0's step 0b; render contract Part 6, "Why
//! first"): a payload set filled on the CPU with hand-chosen values, before any physics exists, for the fragment to
//! render and the tests to assert on. [`Synthetic`](crate::synthetic::Synthetic) holds the four buffers' contents:
//! the `SimState` buffer and the word buffer, one element per sample, `ICDescriptor`, one per sample, and
//! `RenderQuad`, one per quad, over a flat grid of quads
//! ([`Synthetic::flat`](crate::synthetic::Synthetic::flat)).
//!
//! **The quads' frames** (deep_zoom §1; RQ-214; TASK-M1-07). The harness knows each quad's centre `c` and half-width
//! `h` from its own grid: its quads are the depth-`ℓ` cells of the slice's quadtree whose `(column, row)`, Y-up from the
//! slice's bottom-left, run from the grid's `origin` (the slice plane's own frame, R-97), so the quad in grid column
//! `i` and row `j` has `c = (origin + (i, j) + ½) · 2^−ℓ` and `h = 2^−(ℓ+1)` on each axis, exact in f64 ([`QuadFrame`],
//! [`Synthetic::quad_frame`](crate::synthetic::Synthetic::quad_frame)). The CPU computes them in f64 and the GPU reads
//! them as f32, as deep_zoom §1's per-quad uniforms are passed; the scheduler's `QuadRequest` carries them from M5
//! (TASK-M5-04).
//!
//! **The structural sets** (debug_tooling_plan §F; TASK-M1-13). [`Synthetic::structural`] fills each quad's
//! `RenderQuad` metadata from [`structural_record`]'s table, every quad state among them, and places each quad at one of
//! the depths it is given, at its grid column and row, with that depth's frame ([`Synthetic::place`]): what the
//! structural views and overlays read before any scheduler fills a quad.
//!
//! Every write goes through the generated layout: a sample's packed words through the generated pack routines
//! (`kernel::payload`, payload §6), its word through `fgw_pack`, and a quad's members by their names in the ledger's
//! `RenderQuad` table (`ledger::quad`, dd_generation_root §3.7a), each at its generated offset. The bytes place each
//! member of `SimStateFTLE` and `ICDescriptor` by its name at the ledger's offset for it (`ledger::gen::rust::offsets`),
//! little-endian, as the GPU reads them: no offset is written here.
//!
//! [`QuadFrame`]: crate::synthetic::QuadFrame

use kernel::payload::{
    fgw_pack, pack_packed_b, pack_times, set_d_min, set_d_min_unset, set_detail, set_dmin_pair,
    set_last_symbol, set_saturated, set_state, DminCounters, ICDescriptor, SimStateFTLE,
    SD_DMIN_PAIR_SENTINEL, STATE_RUNNING,
};
use ledger::gen::rust::offsets;
use ledger::quad::RENDER_QUAD;
use ledger::schema::Storage;
use render::bind::Payload;
use render::raster::Grid;

/// `RenderQuad`'s size in words (dd_generation_root §3.7a: each member one 4-byte scalar).
pub const QUAD_WORDS: usize = RENDER_QUAD.len();

/// A quad's frame (deep_zoom §1): its centre `c` and half-width `h`, `(u, v)` each, in f64, in the slice's UV frame,
/// Y-up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadFrame {
    pub c: [f64; 2],
    pub h: [f64; 2],
}

/// A CPU-filled payload set over a grid of quads.
#[derive(Debug)]
pub struct Synthetic {
    grid: Grid,
    /// Each quad's place in the slice's quadtree, `(depth, cell)`: the depth-`ℓ` cell `(column, row)`, Y-up from the
    /// slice's bottom-left, which gives its frame ([`Synthetic::quad_frame`]).
    places: Vec<(u32, [u64; 2])>,
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
    /// Each quad's frame, `(c_u, c_v, h_u, h_v)` as f32 ([`QuadFrame`]; `render::bind::FRAME_BINDING`).
    pub quad_frame: Vec<u8>,
}

impl Bytes {
    /// The buffers as the binding takes them.
    pub fn payload(&self) -> Payload<'_> {
        Payload {
            simstate: &self.simstate,
            word: &self.word,
            ic: &self.ic,
            quad: &self.quad,
            quad_frame: &self.quad_frame,
        }
    }
}

impl Synthetic {
    /// A flat layout: `grid`'s quads all at quadtree depth `depth`, tiling the slice from its bottom-left corner
    /// ([`Synthetic::flat_at`] with the origin `(0, 0)`).
    pub fn flat(grid: Grid, depth: u32) -> Synthetic {
        Synthetic::flat_at(grid, depth, [0, 0])
    }

    /// A flat layout: `grid`'s quads all at quadtree depth `depth`, the quad in grid column `i` and row `j` the slice's
    /// depth-`depth` cell `(origin[0] + i, origin[1] + j)` (its frame, [`Synthetic::quad_frame`]), each `quad_state`
    /// loaded (code 0) and its other members zero. Every sample starts fresh: `running`, `d_min` unset, `dmin_pair` at
    /// its sentinel, the empty word, and equal masses summing to 1; every other member zero.
    pub fn flat_at(grid: Grid, depth: u32, origin: [u64; 2]) -> Synthetic {
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
        let columns = grid.quads[0];
        let places = (0..grid.quad_count())
            .map(|q| {
                let cell = [q % columns, q / columns];
                (depth, [0, 1].map(|k| origin[k] + u64::from(cell[k])))
            })
            .collect();
        Synthetic {
            grid,
            places,
            simstate: vec![fresh; samples],
            word: vec![fgw_pack([0; 4], 0); samples],
            ic: vec![ic; samples],
            quad: vec![quad; grid.quad_count() as usize],
            counters: DminCounters::new(),
        }
    }

    /// A structural set (debug_tooling_plan §F; TASK-M1-13): `grid`'s quads, each fresh as [`Synthetic::flat`] makes
    /// it, quad `q` holding [`structural_record`]`(q)` as its `RenderQuad` metadata and placed at depth `depths[q mod
    /// depths.len()]`, at the cell of its grid column and row `(i, j)` ([`Synthetic::place`]), with its deep_zoom §1
    /// frame. Its quads cover every quad state, twice in ten. Refused with no depth, or a quad whose cell lies outside
    /// the slice at its depth (`i` or `j` at least `2^ℓ`).
    pub fn structural(grid: Grid, depths: &[u32]) -> Result<Synthetic, String> {
        if depths.is_empty() {
            return Err("a structural set needs a depth".to_owned());
        }
        let mut set = Synthetic::flat(grid, depths[0]);
        let columns = grid.quads[0];
        for q in 0..grid.quad_count() {
            let depth = depths[q as usize % depths.len()];
            let cell = [q % columns, q / columns].map(u64::from);
            if cell.iter().any(|&c| depth < 64 && c >> depth != 0) {
                return Err(format!(
                    "quad {q}'s cell {cell:?} lies outside the slice at depth {depth}"
                ));
            }
            set.place(q, depth, cell);
            let r = structural_record(q);
            set.quad(q)
                .u32("quad_state", r.state)?
                .f32("coherence_score", r.coherence)?
                .f32("outcome_impurity", r.impurity)?
                .f32("ensemble_spread", r.spread)?
                .f32("suspect_fraction", r.suspect)?
                .f32("priority_score", r.priority)?
                .u32("ancestor_gap", r.ancestor_gap)?
                .u32("cache_age", r.cache_age)?
                .u32("dominant_outcome", r.dominant_outcome)?;
        }
        Ok(set)
    }

    /// The grid the set covers.
    pub fn grid(&self) -> Grid {
        self.grid
    }

    /// Quad `q`'s frame (deep_zoom §1), from its place: the slice's depth-`ℓ` cell `cell`, so `c = (cell + ½) · 2^−ℓ`
    /// and `h = 2^−(ℓ+1)` on each axis, in f64, Y-up. On a flat layout ([`Synthetic::flat_at`]) its grid column and row
    /// `(i, j)`, `q = j · columns + i`, place it at the cell `origin + (i, j)`, so `c = (origin + (i, j) + ½) · 2^−ℓ`.
    pub fn quad_frame(&self, q: u32) -> QuadFrame {
        let (depth, cell) = self.places[q as usize];
        let h = (-f64::from(depth) - 1.0).exp2();
        let c = cell.map(|x| (x as f64 * 2.0 + 1.0) * h);
        QuadFrame { c, h: [h, h] }
    }

    /// Places quad `q` at the slice's depth-`depth` cell `cell`, `(column, row)` Y-up from the slice's bottom-left,
    /// which gives its frame ([`Synthetic::quad_frame`]), and writes `depth` as its `quad_depth`: a set whose quads
    /// sit at different depths (TASK-M1-13). The raster still draws every quad at the grid's one pixel size (M1).
    pub fn place(&mut self, q: u32, depth: u32, cell: [u64; 2]) -> &mut Self {
        self.places[q as usize] = (depth, cell);
        let (k, _) = quad_slot("quad_depth").expect("RenderQuad has quad_depth");
        self.quad[q as usize][k] = depth;
        self
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

    /// The set's one `d_min` counter pair, the frame's pair its `d_min` packs count into (R-288, R-294); its owner
    /// reads it back with [`DminCounters::read`], `(dmin_nan_unset, dmin_negative_floored)`.
    pub fn counters(&self) -> &DminCounters {
        &self.counters
    }

    /// The four buffers' bytes.
    pub fn bytes(&self) -> Bytes {
        let mut out = Bytes {
            simstate: Vec::new(),
            word: Vec::new(),
            ic: Vec::new(),
            quad: Vec::new(),
            quad_frame: Vec::new(),
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
        for q in 0..self.grid.quad_count() {
            let f = self.quad_frame(q);
            let frame = [f.c[0], f.c[1], f.h[0], f.h[1]].map(|x| x as f32);
            out.quad_frame.extend(f32s(frame));
        }
        out
    }
}

/// The ledger struct `name`'s members, each with its byte offset and size, and the struct's size: the generated
/// layout (`ledger::payload::structs`, `ledger::gen::rust::offsets`), the one source of every offset here.
fn layout(name: &str) -> (Vec<(&'static str, usize, usize)>, usize) {
    let s = ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("the ledger has no struct `{name}`"));
    let (offsets, size) = offsets(&s);
    let members = s
        .members
        .iter()
        .zip(offsets)
        .map(|(m, at)| (m.name, at as usize, m.storage.size() as usize))
        .collect();
    (members, size as usize)
}

/// The words of struct `name`, each member's little-endian bytes, as `value` gives them by the member's name, at its
/// offset in the generated layout ([`layout`]).
fn place(name: &str, value: impl Fn(&str) -> Option<Vec<u8>>) -> Vec<u32> {
    let (members, size) = layout(name);
    let mut bytes = vec![0u8; size];
    for (member, at, len) in members {
        let v = value(member)
            .unwrap_or_else(|| panic!("`{name}` has no value for the ledger's `{member}`"));
        assert_eq!(
            v.len(),
            len,
            "`{name}.{member}` is not the ledger's {len} B"
        );
        bytes[at..at + len].copy_from_slice(&v);
    }
    bytes
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
        .collect()
}

fn le(words: impl IntoIterator<Item = u32>) -> Vec<u8> {
    words.into_iter().flat_map(u32::to_le_bytes).collect()
}

fn f32s(v: impl IntoIterator<Item = f32>) -> Vec<u8> {
    le(v.into_iter().map(f32::to_bits))
}

/// `SimStateFTLE` as the words it uploads as: each member at its ledger offset, by name (`place`); the ledger's size.
pub fn simstate_words(s: &SimStateFTLE) -> Vec<u32> {
    place("SimStateFTLE", |member| {
        Some(match member {
            "r" => f32s(s.r.into_iter().flatten()),
            "p" => f32s(s.p.into_iter().flatten()),
            "r_sh" => f32s(s.r_sh.into_iter().flatten()),
            "p_sh" => f32s(s.p_sh.into_iter().flatten()),
            "S" => f32s([s.S]),
            "theta" => f32s([s.theta]),
            "mean_y" => f32s([s.mean_y]),
            "C_ty" => f32s([s.C_ty]),
            "E_0" => f32s([s.E_0]),
            "Lz_0" => f32s([s.Lz_0]),
            "packed_a" => le([s.packed_a]),
            "packed_b" => le([s.packed_b]),
            "times" => le([s.times]),
            "total_substeps" => le([s.total_substeps]),
            "closure_min" => f32s([s.closure_min]),
            "closure_step" => s.closure_step.to_le_bytes().to_vec(),
            "_reserved" => s._reserved.to_le_bytes().to_vec(),
            _ => return None,
        })
    })
}

/// The `SimStateFTLE` whose words, as it uploads as, are `words` ([`simstate_words`]'s inverse, for a readback): each
/// member read by its name at its ledger offset, little-endian; `Err` if `words` is not the ledger's size.
pub fn simstate_from_words(words: &[u32]) -> Result<SimStateFTLE, String> {
    let (members, size) = layout("SimStateFTLE");
    if words.len() * 4 != size {
        return Err(format!(
            "{} words are not SimStateFTLE's {size} B",
            words.len()
        ));
    }
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let word_at =
        |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let f32_at = |at: usize| f32::from_bits(word_at(at));
    let vectors = |at: usize| {
        let c = |k: usize| f32_at(at + 4 * k);
        [[c(0), c(1)], [c(2), c(3)], [c(4), c(5)]]
    };
    let mut s = SimStateFTLE::default();
    for (member, at, _) in members {
        match member {
            "r" => s.r = vectors(at),
            "p" => s.p = vectors(at),
            "r_sh" => s.r_sh = vectors(at),
            "p_sh" => s.p_sh = vectors(at),
            "S" => s.S = f32_at(at),
            "theta" => s.theta = f32_at(at),
            "mean_y" => s.mean_y = f32_at(at),
            "C_ty" => s.C_ty = f32_at(at),
            "E_0" => s.E_0 = f32_at(at),
            "Lz_0" => s.Lz_0 = f32_at(at),
            "packed_a" => s.packed_a = word_at(at),
            "packed_b" => s.packed_b = word_at(at),
            "times" => s.times = word_at(at),
            "total_substeps" => s.total_substeps = word_at(at),
            "closure_min" => s.closure_min = f32_at(at),
            "closure_step" => s.closure_step = u16::from_le_bytes([bytes[at], bytes[at + 1]]),
            "_reserved" => s._reserved = u16::from_le_bytes([bytes[at], bytes[at + 1]]),
            other => {
                return Err(format!(
                    "`SimStateFTLE` has no reader for the ledger's `{other}`"
                ))
            }
        }
    }
    Ok(s)
}

/// `ICDescriptor` as the words it uploads as: each member at its ledger offset, by name (`place`), its declared
/// padding included; the ledger's size.
pub fn ic_words(d: &ICDescriptor) -> Vec<u32> {
    place("ICDescriptor", |member| {
        Some(match member {
            "m0" => f32s([d.m0]),
            "m1" => f32s([d.m1]),
            "m2" => f32s([d.m2]),
            "q_mass" => f32s([d.q_mass]),
            "rho_mag" => f32s([d.rho_mag]),
            "lambda_mag" => f32s([d.lambda_mag]),
            "rho_ratio" => f32s([d.rho_ratio]),
            "rho_angle" => f32s([d.rho_angle]),
            "K_0" => f32s([d.K_0]),
            "V_0" => f32s([d.V_0]),
            "virial_ratio" => f32s([d.virial_ratio]),
            "r_min_pair_0" => f32s([d.r_min_pair_0]),
            "_pad" => le(d._pad),
            _ => return None,
        })
    })
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

/// One quad's `RenderQuad` metadata in a structural set ([`Synthetic::structural`]), each member §3.7a's of the same
/// name: `quad_state`, `coherence_score`, `outcome_impurity`, `ensemble_spread`, `suspect_fraction`, `priority_score`,
/// `ancestor_gap`, `cache_age` and `dominant_outcome`; its depth is the set's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadRecord {
    pub state: u32,
    pub coherence: f32,
    pub impurity: f32,
    pub spread: f32,
    pub suspect: f32,
    pub priority: f32,
    pub ancestor_gap: u32,
    pub cache_age: u32,
    pub dominant_outcome: u32,
}

const fn record(
    state: u32,
    [coherence, impurity, spread, suspect, priority]: [f32; 5],
    [ancestor_gap, cache_age, dominant_outcome]: [u32; 3],
) -> QuadRecord {
    QuadRecord {
        state,
        coherence,
        impurity,
        spread,
        suspect,
        priority,
        ancestor_gap,
        cache_age,
        dominant_outcome,
    }
}

/// Quad `q`'s record in the structural sets (debug_tooling_plan §F), which repeat ten records: the quad states in their
/// order, twice (0 loaded · 1 pending · 2 refinable · 3 terminal · 4 stale); the fractions within [0, 1]; a priority of
/// each sign; an ancestor gap of 0 and above 0 in a pending quad and in others, so the fallback tint and the pending
/// hatch meet; distinct cache ages and outcomes. Hand-chosen harness values, not constants of the physics, so a table in
/// a function, not a `const` (the constants lint, REQ-SYS-001).
pub fn structural_record(q: u32) -> QuadRecord {
    let records = [
        record(0, [0.92, 0.04, 0.08, 0.0, 0.35], [0, 0, 0]),
        record(1, [0.55, 0.38, 0.42, 0.12, 2.75], [0, 4, 1]),
        record(2, [0.18, 0.71, 0.66, 0.31, 1.1], [1, 17, 2]),
        record(3, [0.73, 0.22, 0.27, 0.86, -0.6], [0, 2, 3]),
        record(4, [0.31, 0.57, 0.91, 0.47, 0.05], [2, 40, 4]),
        record(0, [0.66, 0.29, 0.15, 0.05, 1.9], [3, 9, 5]),
        record(1, [0.08, 0.83, 0.58, 0.63, 3.4], [2, 25, 6]),
        record(2, [0.44, 0.49, 0.36, 0.22, -1.25], [0, 1, 7]),
        record(3, [0.97, 0.01, 0.03, 0.97, 0.8], [0, 60, 8]),
        record(4, [0.26, 0.64, 0.77, 0.39, 2.2], [5, 33, 9]),
    ];
    records[q as usize % records.len()]
}
