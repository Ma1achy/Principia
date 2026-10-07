//! QA tests for TASK-M1-07, written from REQ-TOOL-027 (`cargo xtask golden m1-coords`: green increases upward in the
//! coordinate view's golden image, its fragment taking the flip from `lib/coords.wgsl`, which the runner prepends by
//! path; RQ-210), not from the implementation:
//! - the case renders, through the runner, an image whose green rises up every column and red along every row, its
//!   top-left pixel near UV (0, 1) and its bottom-left near (0, 0);
//! - the case's fragment has no flip of its own: without the prepended `crates/render/shaders/wgsl/lib/coords.wgsl`
//!   it does not render.
//!
//! Each test registers its negative control (R-176).

use std::path::Path;

use validation::negative_control;
use xtask::golden::{Case, Config, Renderer};

const LIB: &str = "crates/render/shaders/wgsl/lib/coords.wgsl";

fn case() -> Case {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/golden/m1-coords/coords");
    Case::load(&dir, "m1-coords/coords").expect("the m1-coords case")
}

fn without_prepend(config: &Config) -> Config {
    let mut c = config.clone();
    c.0.remove("prepend");
    c
}

/// The case rendered with `config`: green rises up each column, red along each row, and the corners are the UV
/// frame's (to half a pixel plus half an 8-bit level).
fn check_rendered_view(dir: &Path, config: &Config) {
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let image = renderer
        .render(dir, config)
        .unwrap_or_else(|e| panic!("the m1-coords case did not render: {e}"));
    let (w, h) = (image.width, image.height);
    for x in 0..w {
        for y in 1..h {
            assert!(
                image.pixel(x, y - 1)[1] > image.pixel(x, y)[1],
                "green does not rise upward at ({x}, {y})"
            );
        }
    }
    for y in 0..h {
        for x in 1..w {
            assert!(
                image.pixel(x, y)[0] > image.pixel(x - 1, y)[0],
                "red does not rise rightward at ({x}, {y})"
            );
        }
    }
    let near = 0.5 / f64::from(h) + 0.5 / 255.0;
    for (name, y, v) in [("top-left", 0, 1.0), ("bottom-left", h - 1, 0.0)] {
        let p = image.pixel(0, y);
        let uv = [f64::from(p[0]) / 255.0, f64::from(p[1]) / 255.0];
        assert!(
            uv[0] <= 0.5 / f64::from(w) + 0.5 / 255.0 && (uv[1] - v).abs() <= near,
            "the {name} pixel is UV {uv:?}"
        );
    }
}

#[test]
fn qa_golden_m1_coords_green_rises_upward() {
    let c = case();
    assert_eq!(
        c.config.prepend().expect("prepend"),
        vec![LIB.to_owned()],
        "the case does not prepend the convention's flip"
    );
    check_rendered_view(&c.dir, &c.config);
}

negative_control!(
    qa_golden_m1_coords_green_rises_upward,
    "the case's fragment without the prepended flip does not render",
    expected = "did not render",
    {
        let c = case();
        check_rendered_view(&c.dir, &without_prepend(&c.config));
    }
);

/// The case's fragment, with `config`, does not render: it takes the flip only from the prepended library.
fn check_needs_the_library(config: &Config) {
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let c = case();
    assert!(
        renderer.render(&c.dir, config).is_err(),
        "the case's fragment renders without lib/coords.wgsl: it has a flip of its own"
    );
}

#[test]
fn qa_golden_m1_coords_takes_the_flip_from_the_library() {
    check_needs_the_library(&without_prepend(&case().config));
}

negative_control!(
    qa_golden_m1_coords_takes_the_flip_from_the_library,
    "with the library prepended it renders",
    expected = "renders without lib/coords.wgsl",
    check_needs_the_library(&case().config)
);
