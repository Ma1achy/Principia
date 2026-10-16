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

/// The shown figure's span of the field, `[s₀, t₀, s₁, t₁]`: `[0, 1]²`.
pub const UNIT: [f64; 4] = [0.0, 0.0, 1.0, 1.0];

/// The span `[s₀, t₀, s₁, t₁]` of the figure in `(u, v)`: the shown figure, [`UNIT`], is the visible square, centre
/// `z₀`'s in-plane point and half-width `|q₁|`; a wider span, the window with the egui layer hidden (R-406), is more of
/// the plane at the same scale.
fn view_of(plane: &Plane, span: [f64; 4]) -> Option<Quad> {
    let (n1, n2) = (norm(&plane.q1), norm(&plane.q2));
    // `in_plane` gives `None` for a degenerate basis.
    let (u, v) = crate::explore::manifold_view::in_plane(plane, &plane.z0)?;
    let at = |c: f64, n: f64, s: f64| c + n * (2.0 * s - 1.0);
    Some([
        at(u, n1, span[0]),
        at(v, n2, span[1]),
        at(u, n1, span[2]),
        at(v, n2, span[3]),
    ])
}

/// Whether two quads overlap.
pub fn overlaps(a: &Quad, b: &Quad) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// The leaf quads in view: the root split at least [`MIN_DEPTH`] deep, then while a quad in view is wider than
/// [`SPLIT_WIDE`] of the figure, or near the frontier and wider than [`SPLIT_NEAR`], down to [`MAX_DEPTH`].
pub fn quads(plane: &Plane) -> Vec<Quad> {
    quads_over(plane, UNIT)
}

/// How many copies of the root are laid past it each way: so a window past the root is covered by quads, and a far
/// zoom out stays a bounded tree (placeholder content).
pub const MAX_ROOT_COPIES: i32 = 4;

/// Whether two quads overlap or touch.
pub fn touches(a: &Quad, b: &Quad) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

/// [`quads`] over the figure's span `span` (`[s₀, t₀, s₁, t₁]`): the same tree, its quads sized against the shown
/// figure, so a span past `[0, 1]²` adds quads at the shown quads' size (R-406). Past the root `[−ROOT, ROOT]²` the
/// plane is covered by copies of the root, out to [`MAX_ROOT_COPIES`] each way, split by the same rules; a copy's quads are
/// kept only clear of the shown view, so the shown figure's quads are [`quads`]'s, the layer shown or hidden.
pub fn quads_over(plane: &Plane, span: [f64; 4]) -> Vec<Quad> {
    let Some(shown) = view_of(plane, UNIT) else {
        return Vec::new();
    };
    leaves_over(plane, span)
        .into_iter()
        .filter(|(q, copy)| !(*copy && touches(q, &shown)))
        .map(|(q, _)| q)
        .collect()
}

/// The leaf quads over `span`, each with whether it is a copy of the root's: the root's first, in [`quads`]'s order.
fn leaves_over(plane: &Plane, span: [f64; 4]) -> Vec<(Quad, bool)> {
    let (Some(shown), Some(view)) = (view_of(plane, UNIT), view_of(plane, span)) else {
        return Vec::new();
    };
    let width = shown[2] - shown[0];
    // The copies first, the root last, so the root's quads come first, in the order [`quads`] gives them.
    let mut stack = Vec::new();
    for i in -MAX_ROOT_COPIES..=MAX_ROOT_COPIES {
        for j in -MAX_ROOT_COPIES..=MAX_ROOT_COPIES {
            if (i, j) != (0, 0) {
                let (du, dv) = (f64::from(i) * 2.0 * ROOT, f64::from(j) * 2.0 * ROOT);
                stack.push(([du - ROOT, dv - ROOT, du + ROOT, dv + ROOT], 0u32, true));
            }
        }
    }
    stack.push(([-ROOT, -ROOT, ROOT, ROOT], 0u32, false));
    let mut leaves = Vec::new();
    while let Some((q, depth, copy)) = stack.pop() {
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
                stack.push((c, depth + 1, copy));
            }
        } else {
            leaves.push((q, copy));
        }
    }
    leaves
}

/// The bounds of the quads in view, as segments of the figure, `[(s, t), (s, t)]`: each quad's four edges.
pub fn bounds(plane: &Plane) -> Vec<[[f64; 2]; 2]> {
    bounds_over(plane, UNIT)
        .into_iter()
        .map(|(edge, _)| edge)
        .collect()
}

/// How far past `[0, 1]²`, in `(s, t)`, a point still counts as on the shown figure: room for `f32`'s rounding of an
/// edge on the shown rect's edge, and far under half a pixel of any shown figure up to fifty thousand pixels wide, so
/// no pixel centre is misjudged.
pub const ON_SHOWN_SLACK: f64 = 1e-5;

/// Whether the figure point `p` is on the shown figure, `[0, 1]²` (with [`ON_SHOWN_SLACK`]).
pub fn on_shown(p: [f64; 2]) -> bool {
    p.iter()
        .all(|v| (-ON_SHOWN_SLACK..=1.0 + ON_SHOWN_SLACK).contains(v))
}

/// [`bounds`] over `span`, each edge with whether it is drawn only outside the shown figure: past `[0, 1]²`, `(s, t)`
/// continues at the same scale. The shown figure's quads, [`quads`]'s, are drawn whole, as with the layer shown; every
/// other quad's edges are drawn only outside the shown figure (the marks' shader drops each of their pixels whose
/// centre is on it, [`on_shown`]), so a quad straddling its edge, a copy of the root or the root's own, is bounded
/// outside it right up to its edge, and the shown figure is drawn as with the layer shown (R-406). An edge of those
/// lying wholly on the shown figure draws nothing and is left out.
pub fn bounds_over(plane: &Plane, span: [f64; 4]) -> Vec<([[f64; 2]; 2], bool)> {
    let Some(shown) = view_of(plane, UNIT) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for (q, copy) in leaves_over(plane, span) {
        let corners = [(q[0], q[1]), (q[2], q[1]), (q[2], q[3]), (q[0], q[3])];
        let points: Option<Vec<[f64; 2]>> = corners.iter().map(|c| on_figure(plane, *c)).collect();
        let Some(points) = points else {
            continue;
        };
        let whole = !copy && overlaps(&q, &shown);
        for i in 0..4 {
            let edge = [points[i], points[(i + 1) % 4]];
            if whole {
                lines.push((edge, false));
            } else if !edge.iter().all(|p| on_shown(*p)) {
                lines.push((edge, true));
            }
        }
    }
    lines
}
