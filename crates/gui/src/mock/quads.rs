//! The mock's fake quads and their bounds (R-390, PLACEHOLDERS: "the tile-bounds overlay working on fake quads";
//! applied per R-369, review 5434766412 on PR #157): a quadtree fixed in the plane's own coordinates, refined toward a
//! fake frontier and where its quads are large on the screen, so a pan or a zoom moves and scales its bounds with the
//! stand-in. With the overlay set's tile bounds on, the mock draws them into the figure, as the real post node does
//! (Part II §12.1, Tier 1); the GUI draws nothing over the figure. Placeholder content throughout.

use engine::contract::sim_config::{Latent, Plane};

use crate::explore::manifold_view::{add, chart_coords, norm};

/// The quadtree's root, in the plane's unit in-plane coordinates `(u, v)`: `[−ROOT, ROOT]²`.
pub const ROOT: f64 = 2.0;
/// Every quad is split at least this deep.
pub const MIN_DEPTH: u32 = 2;
/// No quad is split deeper.
pub const MAX_DEPTH: u32 = 9;
/// A quad wider than this fraction of the figure is split, wherever it is.
pub const SPLIT_WIDE: f64 = 0.25;
/// A quad near the frontier is split while wider than this fraction of the figure.
pub const SPLIT_NEAR: f64 = 1.0 / 40.0;
/// The fake frontier: a circle in `(u, v)`, its centre and radius (placeholder content).
pub const FRONTIER: ([f64; 2], f64) = ([0.3, 0.45], 0.35);

/// One quad, `[u0, v0, u1, v1]` in the plane's unit in-plane coordinates.
pub type Quad = [f64; 4];

/// The figure point `(s, t)` of the in-plane point `(u, v)` of `plane`, or `None` for a degenerate basis.
pub fn on_figure(plane: &Plane, (u, v): (f64, f64)) -> Option<[f64; 2]> {
    let (n1, n2) = (norm(&plane.q1), norm(&plane.q2));
    // A degenerate basis divides by zero here, and `chart_coords` gives `None` for it before reading `z`.
    let z: Latent = add(&plane.q1.map(|x| u * x / n1), v / n2, &plane.q2);
    chart_coords(plane, &z).map(|(s, t)| [s, t])
}

/// Whether `quad` comes within its own width of the frontier circle.
pub fn near_frontier(quad: &Quad) -> bool {
    let ([cu, cv], r) = FRONTIER;
    let w = quad[2] - quad[0];
    let du = (cu - cu.clamp(quad[0], quad[2])).abs();
    let dv = (cv - cv.clamp(quad[1], quad[3])).abs();
    let nearest = du.hypot(dv);
    let farthest = (cu - quad[0])
        .abs()
        .max((cu - quad[2]).abs())
        .hypot((cv - quad[1]).abs().max((cv - quad[3]).abs()));
    nearest <= r + w && farthest >= r - w
}

/// The figure's span in `(u, v)`: the visible square, centre `z₀`'s in-plane point and half-width `|q₁|`.
fn view_of(plane: &Plane) -> Option<Quad> {
    let (n1, n2) = (norm(&plane.q1), norm(&plane.q2));
    // `in_plane` gives `None` for a degenerate basis.
    let (u, v) = crate::explore::manifold_view::in_plane(plane, &plane.z0)?;
    Some([u - n1, v - n2, u + n1, v + n2])
}

/// Whether two quads overlap.
pub fn overlaps(a: &Quad, b: &Quad) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// The leaf quads in view: the root split at least [`MIN_DEPTH`] deep, then while a quad in view is wider than
/// [`SPLIT_WIDE`] of the figure, or near the frontier and wider than [`SPLIT_NEAR`], down to [`MAX_DEPTH`].
pub fn quads(plane: &Plane) -> Vec<Quad> {
    let Some(view) = view_of(plane) else {
        return Vec::new();
    };
    let width = view[2] - view[0];
    let mut stack = vec![([-ROOT, -ROOT, ROOT, ROOT], 0u32)];
    let mut leaves = Vec::new();
    while let Some((q, depth)) = stack.pop() {
        if !overlaps(&q, &view) {
            continue;
        }
        let frac = (q[2] - q[0]) / width;
        let split = depth < MIN_DEPTH
            || (depth < MAX_DEPTH
                && (frac > SPLIT_WIDE || (frac > SPLIT_NEAR && near_frontier(&q))));
        if split {
            let (mu, mv) = ((q[0] + q[2]) / 2.0, (q[1] + q[3]) / 2.0);
            for c in [
                [q[0], q[1], mu, mv],
                [mu, q[1], q[2], mv],
                [q[0], mv, mu, q[3]],
                [mu, mv, q[2], q[3]],
            ] {
                stack.push((c, depth + 1));
            }
        } else {
            leaves.push(q);
        }
    }
    leaves
}

/// The bounds of the quads in view, as segments of the figure, `[(s, t), (s, t)]`: each quad's four edges.
pub fn bounds(plane: &Plane) -> Vec<[[f64; 2]; 2]> {
    let mut lines = Vec::new();
    for q in quads(plane) {
        let corners = [(q[0], q[1]), (q[2], q[1]), (q[2], q[3]), (q[0], q[3])];
        let points: Option<Vec<[f64; 2]>> = corners.iter().map(|c| on_figure(plane, *c)).collect();
        let Some(points) = points else {
            continue;
        };
        for i in 0..4 {
            lines.push([points[i], points[(i + 1) % 4]]);
        }
    }
    lines
}
