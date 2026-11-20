//! The first slice's charts and grids (REQ-DEC-057, REQ-PERF-120; TASK-M2-86), which `engine::first_slice` puts
//! through R-926's route: the measurement's design, its numbers here, outside the engine's constants register, as a
//! test's inputs are (`xtask::lint_constants`'s note).
//!
//! **The charts** ([`Preset`]): the latent chart, the shape sphere and the `(L_z, E)` chart at the parameters the
//! DECODE preset holds ([`crate::decode_preset::uniforms`], one source), at f32, the shape sphere under its exponential
//! map, as the libm-pin measurement holds them (R-991; `xtask/libm_pin`).
//!
//! **The grids** ([`Grid`], [`GRIDS`]; REQ-DEC-057): one quad of [`SIDE`] × [`SIDE`] samples over each chart's home
//! window, and the same zoomed 2¹⁰ and 2²⁰ (R-90's `ℓ_switch = 20`) about an O(1) point and about the chart's origin.
//! Applied per R-369 (the measurement's grid, a reading of REQ-DEC-057's words on each chart's own window):
//! - the latent chart's window is its plane: at home the preset's; zoomed `2ᵏ` its `q₁`, `q₂` scaled by `2⁻ᵏ`
//!   (`Latent::zoomed`, R-83), exact at f32, about `z₀` = [`POINT`] in every latent coordinate (0.7 rounded once to
//!   f32) and about `z₀ = 0`, the origin;
//! - a curved chart's window is today's `Window` on its own coordinates `(u, v)`: at home `Window::SQUARE`; zoomed
//!   `2ᵏ` a half-width `½·2⁻ᵏ` about `(0.7, 0.7)`, every coordinate [`POINT`], and about the chart's origin, where its
//!   own coordinates are zero: on the exponential map `d = 0`, `(u, v) = (½, ½)`, the map's centre `n₀`; on the
//!   `(L_z, E)` chart `L_z = 0`, `K = 0`, `(u, v) = (½, 0)`, the parabola's apex.

use engine::contract::sim_config::Window;
use engine::first_slice::{Quad, SliceChart};
use kernel::chart::{InvariantLE, Latent, ShapeMap, ShapeSphere};
use kernel::stage::decode::UniformChart;

use crate::decode_preset::{uniforms, DecodeChart};

/// A quad's side in samples (REQ-DEC-057: 256 × 256).
pub const SIDE: usize = 256;

/// The O(1) point's coordinate (REQ-DEC-057: "each latent coordinate 0.7").
pub const POINT: f64 = 0.7;

/// Where a grid is centred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Centre {
    /// The chart's home window, unzoomed.
    Home,
    /// The O(1) point, every coordinate [`POINT`].
    Point,
    /// The chart's origin, where its own coordinates are zero.
    Origin,
}

/// One quad's grid: its zoom `2^zoom` about its centre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    /// `k` in `2ᵏ`.
    pub zoom: u32,
    /// The centre.
    pub centre: Centre,
}

impl Grid {
    /// The grid's name in a report.
    pub fn name(&self) -> String {
        match self.centre {
            Centre::Home => "home".to_owned(),
            Centre::Point => format!("2^{} about the O(1) point", self.zoom),
            Centre::Origin => format!("2^{} about the origin", self.zoom),
        }
    }
}

/// REQ-DEC-057's grids: home, then `2¹⁰` and `2²⁰` about the O(1) point, then the same about the origin.
pub const GRIDS: [Grid; 5] = [
    Grid {
        zoom: 0,
        centre: Centre::Home,
    },
    Grid {
        zoom: 10,
        centre: Centre::Point,
    },
    Grid {
        zoom: 20,
        centre: Centre::Point,
    },
    Grid {
        zoom: 10,
        centre: Centre::Origin,
    },
    Grid {
        zoom: 20,
        centre: Centre::Origin,
    },
];

/// `2⁻ᵏ`, exact.
pub fn shrink(k: u32) -> f64 {
    f64::powi(2.0, -(k as i32))
}

/// A curved chart's window for `grid`: `Window::SQUARE` at home, else the half-width `½·2⁻ᵏ` about `point` or
/// `origin`.
pub fn window(grid: Grid, point: [f64; 2], origin: [f64; 2]) -> Window {
    let centre = match grid.centre {
        Centre::Home => return Window::SQUARE,
        Centre::Point => point,
        Centre::Origin => origin,
    };
    let half = 0.5 * shrink(grid.zoom);
    Window {
        centre,
        half: [half, half],
    }
}

/// A chart of the slice at its preset, and its quad on each grid (the module's notes).
pub trait Preset: SliceChart {
    /// The chart at the DECODE preset's parameters.
    fn preset() -> Self;

    /// The quad of `grid`, `size` samples.
    fn quad(grid: Grid, size: [usize; 2]) -> Quad<Self>;
}

impl Preset for Latent<f32> {
    fn preset() -> Self {
        Latent::from_uniform(&uniforms(DecodeChart::Latent).chart)
    }

    fn quad(grid: Grid, size: [usize; 2]) -> Quad<Self> {
        let preset = Self::preset();
        let z0 = match grid.centre {
            Centre::Home => preset.z0,
            Centre::Point => [POINT as f32; 8],
            Centre::Origin => [0.0; 8],
        };
        Quad {
            chart: Latent { z0, ..preset }.zoomed(shrink(grid.zoom) as f32),
            window: Window::SQUARE,
            size,
        }
    }
}

impl Preset for ShapeSphere<f32> {
    fn preset() -> Self {
        ShapeSphere {
            map: ShapeMap::Exponential,
            ..ShapeSphere::from_uniform(&uniforms(DecodeChart::ShapeSphere).chart)
        }
    }

    fn quad(grid: Grid, size: [usize; 2]) -> Quad<Self> {
        Quad {
            chart: Self::preset(),
            window: window(grid, [POINT, POINT], [0.5, 0.5]),
            size,
        }
    }
}

impl Preset for InvariantLE<f32> {
    fn preset() -> Self {
        InvariantLE::from_uniform(&uniforms(DecodeChart::InvariantLE).chart)
    }

    fn quad(grid: Grid, size: [usize; 2]) -> Quad<Self> {
        Quad {
            chart: Self::preset(),
            window: window(grid, [POINT, POINT], [0.5, 0.0]),
            size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One grid's quad on each chart.
    type Quads = (
        Quad<Latent<f32>>,
        Quad<ShapeSphere<f32>>,
        Quad<InvariantLE<f32>>,
    );

    /// Each chart's quad on each grid: the latent chart's plane zoomed about its centre over the square, a curved
    /// chart's window about its centre with the zoom's half-width.
    fn check_quads(quad: fn(Grid) -> Quads) {
        let preset = Latent::<f32>::preset();
        for g in GRIDS {
            let (l, s, e) = quad(g);
            let k = shrink(g.zoom) as f32;
            let (z0, sphere, apex) = match g.centre {
                Centre::Home => (preset.z0, [0.5, 0.5], [0.5, 0.5]),
                Centre::Point => ([0.7f32; 8], [0.7, 0.7], [0.7, 0.7]),
                Centre::Origin => ([0.0; 8], [0.5, 0.5], [0.5, 0.0]),
            };
            assert_eq!(l.chart.z0, z0, "{}: the latent plane's centre", g.name());
            assert_eq!(l.chart.q1, preset.q1.map(|q| q * k), "{}: q1", g.name());
            assert_eq!(l.chart.q2, preset.q2.map(|q| q * k), "{}: q2", g.name());
            assert_eq!(l.window, Window::SQUARE, "{}: the latent window", g.name());
            let half = 0.5 * shrink(g.zoom);
            for (w, c) in [(s.window, sphere), (e.window, apex)] {
                assert_eq!(w.centre, c, "{}: a curved chart's centre", g.name());
                assert_eq!(
                    w.half,
                    [half, half],
                    "{}: a curved chart's half-width",
                    g.name()
                );
            }
            assert_eq!(s.chart.map, ShapeMap::Exponential, "the sphere's map");
            assert_eq!([l.size, s.size, e.size], [[3, 2]; 3], "the sizes");
        }
    }

    fn quads(g: Grid) -> Quads {
        (
            Latent::quad(g, [3, 2]),
            ShapeSphere::quad(g, [3, 2]),
            InvariantLE::quad(g, [3, 2]),
        )
    }

    #[test]
    fn first_slice_grids() {
        check_quads(quads);
        assert_eq!(
            [0, 10, 20].map(shrink),
            [1.0, 1.0 / 1024.0, 1.0 / 1_048_576.0],
            "2⁻ᵏ"
        );
        assert_eq!(
            GRIDS.map(|g| g.zoom),
            [0, 10, 20, 10, 20],
            "REQ-DEC-057's zooms"
        );
        assert_eq!(GRIDS[1].name(), "2^10 about the O(1) point");
        assert_eq!(GRIDS[4].name(), "2^20 about the origin");
        assert_eq!(GRIDS[0].name(), "home");
    }

    crate::negative_control!(
        first_slice_grids,
        "a zoom about the wrong centre must fail",
        expected = "a curved chart's centre",
        check_quads(|g| {
            let (l, s, mut e) = quads(g);
            e.window.centre[1] += 0.25;
            (l, s, e)
        })
    );

    /// The presets are the DECODE preset's charts, the sphere under its exponential map.
    #[test]
    fn first_slice_presets() {
        assert_eq!(
            Latent::<f32>::preset().uniform(),
            uniforms(DecodeChart::Latent).chart
        );
        assert_eq!(
            InvariantLE::<f32>::preset().uniform(),
            uniforms(DecodeChart::InvariantLE).chart
        );
        let sphere = ShapeSphere::<f32>::preset();
        let decode = ShapeSphere::<f32>::from_uniform(&uniforms(DecodeChart::ShapeSphere).chart);
        assert_eq!(sphere.map, ShapeMap::Exponential, "the exponential map");
        assert_eq!(
            ShapeSphere {
                map: decode.map,
                ..sphere
            },
            decode,
            "the sphere's preset is not the DECODE preset's"
        );
    }

    crate::negative_control!(
        first_slice_presets,
        "a sphere under the spherical map must fail",
        expected = "the exponential map",
        assert_eq!(
            ShapeSphere::<f32>::from_uniform(&uniforms(DecodeChart::ShapeSphere).chart).map,
            ShapeMap::Exponential,
            "the exponential map"
        )
    );
}
