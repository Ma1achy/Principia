//! The structural presets and sets (`engine::structural`, `engine::synthetic`; TASK-M1-13):
//! - REQ-RENDER-024: the graph holding the boundary overlay serialises, loads back to an equal graph with an equal
//!   render key, and the overlay's width, opacity, colour and level are its params (`boundary_overlay_roundtrip`);
//! - REQ-TOOL-026: every structural view, and the overlays after it, is a graph that lowers and assembles
//!   (`structural_presets_*`), whose colour node is found under the overlays; the structural sets cover every quad
//!   state, and a priority of each sign, at depths 3 and 20 with their deep_zoom §1 frames (`structural_sets_*`).
//!
//! Each test registers its negative control (R-176).

use engine::stain::{NodeId, NodeKind, Occupant, StainGraph};
use engine::structural::{boundaries, colour_node, overlay, view, Level, VIEWS};
use engine::synthetic::{structural_record, Synthetic};
use render::assemble::{assemble, Tier};
use render::raster::Grid;
use render::structural::QuadMeta;
use validation::negative_control;

/// The boundary overlay over the quad-state view: both levels, every style param set off its default; and the
/// overlay's node.
fn overlay_graph() -> (StainGraph, NodeId) {
    let mut g = view("s_state").unwrap_or_else(|e| panic!("{e}"));
    let p =
        overlay(&mut g, Occupant::Builtin("edge_line".into())).unwrap_or_else(|e| panic!("{e}"));
    for (name, value) in [
        ("width", vec![3.5]),
        ("opacity", vec![0.75]),
        ("tile_width", vec![1.25]),
        ("tile_opacity", vec![0.4]),
        ("colour", vec![1.0, 0.5, 0.25]),
        ("level", vec![2.0]),
    ] {
        g.set_param(p, name, value)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    (g, p)
}

/// Checks that `saved`, loaded, is `g`, with `g`'s render key.
fn check_roundtrip(g: &StainGraph, saved: &str) {
    let loaded: StainGraph = serde_json::from_str(saved).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(&loaded, g, "the loaded graph is not the saved one");
    let key = |g: &StainGraph| g.canonical().unwrap_or_else(|e| panic!("{e}")).render_key();
    assert_eq!(key(&loaded), key(g), "the loaded graph's render key");
}

#[test]
fn boundary_overlay_roundtrip() {
    let (g, p) = overlay_graph();
    let saved = serde_json::to_string(&g).unwrap_or_else(|e| panic!("{e}"));
    assert!(saved.contains(r#""builtin":"edge_line""#), "{saved}");
    for param in [
        r#""width":[3.5]"#,
        r#""opacity":[0.75]"#,
        r#""colour":[1.0,0.5,0.25]"#,
        r#""level":[2.0]"#,
    ] {
        assert!(
            saved.contains(param),
            "the saved graph lacks {param}: {saved}"
        );
    }
    check_roundtrip(&g, &saved);
    // The style is the render key's: another width is another key.
    let mut wider = g.clone();
    wider
        .set_param(p, "width", vec![4.0])
        .unwrap_or_else(|e| panic!("{e}"));
    let key = |g: &StainGraph| g.canonical().unwrap_or_else(|e| panic!("{e}")).render_key();
    assert_ne!(key(&wider), key(&g), "the width is not in the render key");
}

negative_control!(
    boundary_overlay_roundtrip,
    "a saved graph whose width was edited loads to another graph",
    expected = "the loaded graph is not the saved one",
    {
        let (g, _) = overlay_graph();
        let saved = serde_json::to_string(&g).expect("saved");
        check_roundtrip(&g, &saved.replace(r#""width":[3.5]"#, r#""width":[3.0]"#));
    }
);

/// Checks that `g` lowers and assembles.
fn check_assembles(g: &StainGraph, what: &str) {
    let stain = g.lower().unwrap_or_else(|e| panic!("{what}: {e}"));
    assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("{what}: {e}"));
}

#[test]
fn structural_presets_assemble() {
    for id in VIEWS {
        let mut g = view(id).unwrap_or_else(|e| panic!("{id}: {e}"));
        check_assembles(&g, id);
        for post in ["fallback_tint", "pending_hatch", "edge_line"] {
            overlay(&mut g, Occupant::Builtin(post.into())).unwrap_or_else(|e| panic!("{e}"));
        }
        check_assembles(&g, &format!("{id} under the overlays"));
    }
    for level in [Level::Quad, Level::Tile, Level::Both] {
        let g = boundaries(level).unwrap_or_else(|e| panic!("{e}"));
        check_assembles(&g, &format!("{level:?}"));
        let text = serde_json::to_string(&g).expect("saved");
        assert!(
            text.contains(&format!(r#""level":[{}.0]"#, level as u32)),
            "{level:?}: {text}"
        );
    }
    match view("s_nothing") {
        Ok(_) => panic!("a view that is none loaded"),
        Err(e) => assert!(
            e.to_string().contains("`s_nothing` is no structural view"),
            "{e}"
        ),
    }
}

negative_control!(
    structural_presets_assemble,
    "a view that is none is refused",
    expected = "is no structural view",
    {
        view("s_nothing").unwrap_or_else(|e| panic!("{e}"));
    }
);

/// Checks that `set`, over `grid`, places quad `q` at `depths[q mod len]` with its record and frame.
fn check_set(set: &Synthetic, grid: Grid, depths: &[u32]) {
    let mut states = [false; 5];
    for q in 0..grid.quad_count() {
        let depth = depths[q as usize % depths.len()];
        let r = structural_record(q);
        let m = QuadMeta::from_words(&set.quad_words(q));
        assert_eq!(
            (m.depth, m.state, m.impurity, m.ancestor_gap),
            (depth, r.state, r.impurity, r.ancestor_gap),
            "quad {q}'s depth, state, impurity and ancestor gap"
        );
        assert_eq!(
            (
                m.coherence,
                m.spread,
                m.suspect_frac,
                m.priority,
                m.cache_age,
                m.dominant_outcome
            ),
            (
                r.coherence,
                r.spread,
                r.suspect,
                r.priority,
                r.cache_age,
                r.dominant_outcome
            ),
            "quad {q}'s other members"
        );
        states[r.state as usize] = true;
        let cell = [q % grid.quads[0], q / grid.quads[0]];
        let h = (-f64::from(depth) - 1.0).exp2();
        let f = set.quad_frame(q);
        assert_eq!(f.h, [h, h], "quad {q}'s half-width at depth {depth}");
        assert_eq!(
            f.c,
            cell.map(|c| (2.0 * f64::from(c) + 1.0) * h),
            "quad {q}'s centre"
        );
    }
    assert_eq!(states, [true; 5], "every quad state");
}

#[test]
fn structural_sets_cover_every_state_at_their_depths() {
    let grid = Grid::new([5, 2], 2, 0, 8).expect("a grid");
    for depths in [&[3][..], &[20], &[3, 20]] {
        let set = Synthetic::structural(grid, depths).unwrap_or_else(|e| panic!("{e}"));
        check_set(&set, grid, depths);
    }
    let one = Grid::new([1, 1], 1, 0, 1).expect("a grid");
    assert!(
        Synthetic::structural(one, &[]).is_err_and(|e| e.contains("needs a depth")),
        "no depth"
    );
    let wide = Grid::new([9, 1], 1, 0, 1).expect("a grid");
    assert!(
        Synthetic::structural(wide, &[3])
            .is_err_and(|e| e.contains("quad 8's cell [8, 0] lies outside the slice at depth 3")),
        "a cell outside the slice"
    );
    assert!(
        Synthetic::structural(Grid::new([8, 8], 1, 0, 1).expect("a grid"), &[3]).is_ok(),
        "the last cell, 7"
    );
    assert!(Synthetic::structural(wide, &[64]).is_ok(), "a depth of 64");
}

negative_control!(
    structural_sets_cover_every_state_at_their_depths,
    "the depth-3 set read as the depth-20 set fails",
    expected = "quad 0's depth, state, impurity and ancestor gap",
    {
        let grid = Grid::new([5, 2], 2, 0, 8).expect("a grid");
        let set = Synthetic::structural(grid, &[3]).expect("a set");
        check_set(&set, grid, &[20]);
    }
);

/// Checks that `place` moves quad `q`'s frame and depth and leaves the others.
fn check_place(depth: u32, cell: [u64; 2]) {
    let grid = Grid::new([2, 1], 1, 0, 1).expect("a grid");
    let mut set = Synthetic::flat_at(grid, 3, [5, 2]);
    let before = set.quad_frame(0);
    set.place(1, depth, cell);
    assert_eq!(set.quad_frame(0), before, "quad 0's frame");
    assert_eq!(set.quad_words(1)[0], depth, "quad 1's depth");
    let h = (-f64::from(depth) - 1.0).exp2();
    let f = set.quad_frame(1);
    assert_eq!(f.h, [h, h], "quad 1's half-width");
    assert_eq!(
        f.c,
        cell.map(|c| (2.0 * c as f64 + 1.0) * h),
        "quad 1's centre"
    );
}

#[test]
fn structural_sets_place_a_quad() {
    check_place(20, [629_145, 3]);
    check_place(0, [0, 0]);
}

negative_control!(
    structural_sets_place_a_quad,
    "a centre read off the cell's corner fails",
    expected = "quad 1's centre",
    {
        let grid = Grid::new([2, 1], 1, 0, 1).expect("a grid");
        let mut set = Synthetic::flat(grid, 3);
        set.place(1, 2, [1, 1]);
        let f = set.quad_frame(1);
        assert_eq!(f.c, [0.75, 0.25], "quad 1's centre");
    }
);

/// Checks that `found`, in `g`, is a colour node: the view's, not its source and not a post.
fn check_colour_node(g: &StainGraph, found: Option<NodeId>) {
    let id = found.unwrap_or_else(|| panic!("no node feeds the combiner's colour"));
    let kind = g.node(id).map(|n| n.kind);
    assert_eq!(
        kind,
        Some(NodeKind::Colour),
        "{id:?} is not the view's colour node"
    );
}

#[test]
fn structural_presets_colour_node_is_the_views() {
    for id in VIEWS {
        let mut g = view(id).unwrap_or_else(|e| panic!("{e}"));
        check_colour_node(&g, colour_node(&g));
        for post in ["fallback_tint", "edge_line"] {
            overlay(&mut g, Occupant::Builtin(post.into())).unwrap_or_else(|e| panic!("{e}"));
        }
        check_colour_node(&g, colour_node(&g));
    }
    // The backbone alone, and the boundaries over it, feed the combiner nothing: no colour node, though OUT's port 0
    // is wired.
    let g = boundaries(Level::Both).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        colour_node(&g),
        None,
        "the boundaries over the backbone have a colour node"
    );
    assert_eq!(
        colour_node(&StainGraph::new()),
        None,
        "the backbone has a colour node"
    );
}

negative_control!(
    structural_presets_colour_node_is_the_views,
    "the view's source taken for its colour node fails",
    expected = "is not the view's colour node",
    {
        let g = view("s_depth").expect("a view");
        let source = g
            .wires()
            .into_iter()
            .find(|w| w.to != NodeId(0))
            .map(|w| w.from);
        check_colour_node(&g, source);
    }
);

/// Checks that `priorities`, the structural records' in order, hold the two negative priorities the sets carry.
fn check_priority_signs(priorities: &[f32]) {
    let negative: Vec<f32> = priorities.iter().copied().filter(|&p| p < 0.0).collect();
    assert_eq!(
        negative,
        [-0.6, -1.25],
        "the structural records' negative priorities"
    );
    assert!(priorities.iter().any(|&p| p > 0.0), "no positive priority");
}

#[test]
fn structural_sets_carry_a_priority_of_each_sign() {
    let priorities: Vec<f32> = (0..10).map(|q| structural_record(q).priority).collect();
    check_priority_signs(&priorities);
    assert_eq!(
        structural_record(13),
        structural_record(3),
        "the records repeat every ten quads"
    );
}

negative_control!(
    structural_sets_carry_a_priority_of_each_sign,
    "priorities all of one sign fail",
    expected = "the structural records' negative priorities",
    check_priority_signs(&[0.35, 2.75, 1.1, 0.6])
);
