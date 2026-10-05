//! The sample → tile rasterisation (canonical spec §8: QUAD → N×N SAMPLES → TILE → PIXELS; one sample, one tile, no
//! interpolation). A [`Grid`] of quads, each N × N tiles of `tile_px` × `tile_px` pixels, covers the target exactly,
//! so every tile edge is a pixel edge and every pixel lies in one tile: the pixel reads its tile's one sample, never a
//! blend of two. [`WGSL`] is the fragment side's `raster`; [`Grid::cell`] is its CPU mirror, which the tests hold the
//! GPU to pixel by pixel.
//!
//! **Orientation** (colour_composition §3, R-72): rows and the within-cell coordinates count from the bottom-left, `v`
//! upward, the post-flip Y-up convention; the target's pixel rows count from the top, as wgpu's framebuffer does. The
//! quad and tile in column `i` and row `j` are `j · columns + i`, and `ctx.quad.uv = (vec2(i, j) + ctx.tile.uv) / N`
//! for the tile at `(i, j)` in its quad.
//!
//! **The index map.** A sample's index in the payload buffers is `(quad · N² + tile) · (E + 1) + copy` (applied per
//! R-369: payload §0 names a `(quad, sample, copy)` → index map and does not give it). A pixel reads its tile's base
//! sample, copy 0; the E copies are the resolve stage's.

/// A flat grid of quads covering the target: `quads[0]` columns by `quads[1]` rows of quads, each `n` × `n` tiles
/// (`N = SAMPLES_PER_QUAD_AXIS`), each tile `tile_px` pixels square, each sample with `e` ensemble copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub quads: [u32; 2],
    pub n: u32,
    pub e: u32,
    pub tile_px: u32,
}

/// Where a pixel rasterises to: its quad and tile, each by index and by `(column, row)` from the bottom-left, and the
/// base sample it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub quad: u32,
    pub quad_xy: [u32; 2],
    pub tile: u32,
    pub tile_xy: [u32; 2],
    pub sample: u32,
}

impl Grid {
    /// The grid, or why not: a zero count or size, or a target or buffer past u32.
    pub fn new(quads: [u32; 2], n: u32, e: u32, tile_px: u32) -> Result<Grid, String> {
        if quads.contains(&0) || n == 0 || tile_px == 0 {
            return Err(format!(
                "a grid of {quads:?} quads of {n} × {n} tiles of {tile_px} px covers no pixel"
            ));
        }
        let grid = Grid {
            quads,
            n,
            e,
            tile_px,
        };
        let side = |q: u32| q.checked_mul(n)?.checked_mul(tile_px);
        let samples = (quads[0] as u64) * (quads[1] as u64) * (n as u64).pow(2) * (e as u64 + 1);
        if side(quads[0]).is_none() || side(quads[1]).is_none() || samples > u32::MAX as u64 {
            return Err(format!("{grid:?} overflows u32"));
        }
        Ok(grid)
    }

    /// The target it covers, `(width, height)` in pixels.
    pub fn target(&self) -> (u32, u32) {
        let side = |q: u32| q * self.n * self.tile_px;
        (side(self.quads[0]), side(self.quads[1]))
    }

    /// The number of quads.
    pub fn quad_count(&self) -> u32 {
        self.quads[0] * self.quads[1]
    }

    /// The samples in one quad, copies included: `N² · (E + 1)`.
    pub fn samples_per_quad(&self) -> u32 {
        self.n * self.n * (self.e + 1)
    }

    /// The payload buffers' length in samples.
    pub fn sample_count(&self) -> u32 {
        self.quad_count() * self.samples_per_quad()
    }

    /// The index of copy `copy` of tile `tile`'s sample in quad `quad`: `(quad · N² + tile) · (E + 1) + copy`.
    pub fn sample_index(&self, quad: u32, tile: u32, copy: u32) -> u32 {
        (quad * self.n * self.n + tile) * (self.e + 1) + copy
    }

    /// Where pixel `(x, y)` rasterises to, `y` counted from the top: [`WGSL`]'s `raster` on the CPU.
    pub fn cell(&self, x: u32, y: u32) -> Cell {
        let (_, height) = self.target();
        let column = x / self.tile_px;
        let row = (height - 1 - y) / self.tile_px;
        let quad_xy = [column / self.n, row / self.n];
        let tile_xy = [column % self.n, row % self.n];
        let quad = quad_xy[1] * self.quads[0] + quad_xy[0];
        let tile = tile_xy[1] * self.n + tile_xy[0];
        Cell {
            quad,
            quad_xy,
            tile,
            tile_xy,
            sample: self.sample_index(quad, tile, 0),
        }
    }

    /// The pixels of the tile at `tile_xy` in the quad at `quad_xy`, as `(x0, y0, x1, y1)`, half-open, `y` from the top.
    pub fn tile_pixels(&self, quad_xy: [u32; 2], tile_xy: [u32; 2]) -> (u32, u32, u32, u32) {
        let (_, height) = self.target();
        let t = self.tile_px;
        let column = quad_xy[0] * self.n + tile_xy[0];
        let row = quad_xy[1] * self.n + tile_xy[1];
        let y1 = height - row * t;
        (column * t, y1 - t, column * t + t, y1)
    }
}

/// The fragment side's rasterisation: `raster(pos, quads, n, e, tile_px)`, `pos` the fragment's
/// `@builtin(position).xy` (a pixel's centre), gives the pixel, the target, the screen-space `uv`, and the quad, tile
/// and base sample it reads, with its within-quad and within-tile `uv`, each as [`Grid::cell`] gives them.
pub const WGSL: &str = r"
// ── The sample → tile rasterisation (canonical spec §8; TASK-M1-06): one sample per tile, no interpolation ──
struct Raster {
    pixel: vec2<u32>,
    target_dims: vec2<u32>,
    screen_uv: vec2<f32>,
    quad_xy: vec2<u32>,
    quad: u32,
    tile_xy: vec2<u32>,
    tile: u32,
    sample: u32,
    quad_uv: vec2<f32>,
    tile_uv: vec2<f32>,
}

// Where the pixel at `pos` rasterises to (render::raster): rows and uv from the bottom-left, Y-up; pixel rows from the
// top. Every quantity but the uv is integer, so a tile's pixels all read its one sample.
fn raster(pos: vec2<f32>, quads: vec2<u32>, n: u32, e: u32, tile_px: u32) -> Raster {
    var r: Raster;
    r.target_dims = quads * (n * tile_px);
    r.pixel = vec2<u32>(pos);
    let cell = vec2<u32>(r.pixel.x, r.target_dims.y - 1u - r.pixel.y) / tile_px;
    r.quad_xy = cell / n;
    r.tile_xy = cell % n;
    r.quad = r.quad_xy.y * quads.x + r.quad_xy.x;
    r.tile = r.tile_xy.y * n + r.tile_xy.x;
    r.sample = (r.quad * n * n + r.tile) * (e + 1u);
    let up = vec2<f32>(pos.x, f32(r.target_dims.y) - pos.y);
    r.screen_uv = up / vec2<f32>(r.target_dims);
    r.tile_uv = (up - vec2<f32>(cell * tile_px)) / f32(tile_px);
    r.quad_uv = (vec2<f32>(r.tile_xy) + r.tile_uv) / f32(n);
    return r;
}
";
