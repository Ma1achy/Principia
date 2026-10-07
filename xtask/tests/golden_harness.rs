//! `cargo xtask golden`'s harness case kind (RQ-229, decided per R-369; TASK-M1-09): a case's `render` names a
//! synthetic scene, `{ "harness": "<scene>", "width": <w>, "height": <h> }`; the runner spawns validation's
//! `golden_harness` binary, which writes the scene's `Rgba32Float` image, and quantises it in its own pass:
//! - the config's fields: a harness case has `harness`, `width` and `height` alone (`golden_harness_config_*`);
//! - the float image's format, refused unless whole and of the case's size (`golden_harness_decode_*`);
//! - a harness case renders through the binary, its stored levels the floats rounded half to even, and has no
//!   automatic output (`golden_harness_renders_*`).
//!
//! Each test registers its negative control (R-176).

use std::path::Path;

use serde_json::json;
use validation::negative_control;
use xtask::golden::{decode_harness, Config, Output, Renderer, HARNESS_MAGIC};

fn config(render: serde_json::Value) -> Result<Config, String> {
    Config::from_render(&render)
}

/// Checks that `render` is refused with a message containing `why`.
fn check_refused(render: serde_json::Value, why: &str) {
    match config(render.clone()) {
        Ok(_) => panic!("{render} is accepted"),
        Err(e) => assert!(e.contains(why), "{render} is refused as `{e}`, not `{why}`"),
    }
}

#[test]
fn golden_harness_config_names_a_scene() {
    let c = config(json!({ "harness": "ftle", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c.harness(), Some("ftle"));
    assert_eq!(c.size(), Ok((64, 8)));
    let shader = json!({ "shader": "a.wgsl", "fragment": "f", "width": 1, "height": 1 });
    assert_eq!(config(shader).map(|c| c.harness().is_none()), Ok(true));
    check_refused(
        json!({ "harness": "ftle", "width": 64 }),
        "`render.height` is missing",
    );
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "shader": "a.wgsl" }),
        "`render.shader` is not a render field",
    );
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "constants": { "k": 1 } }),
        "`render.constants` is not a render field",
    );
    check_refused(
        json!({ "harness": 3, "width": 64, "height": 8 }),
        "`render.harness` is not a string",
    );
    check_refused(
        json!({ "shader": "a.wgsl", "width": 1, "height": 1 }),
        "`render.fragment` is missing",
    );
}

negative_control!(
    golden_harness_config_names_a_scene,
    "a harness case with a shader is refused, so the check of its acceptance fails",
    expected = "is refused as",
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "shader": "a.wgsl" }),
        "accepted"
    )
);

/// Checks that setting `field` on `c` is refused, naming the fields.
fn check_set_refused(c: &mut Config, field: &str) {
    match c.set(field, json!(1)) {
        Ok(()) => panic!("`{field}` is set"),
        Err(e) => assert!(e.contains("is not a field"), "`{field}`: {e}"),
    }
}

#[test]
fn golden_harness_config_sets_its_own_fields() {
    let mut c = config(json!({ "harness": "ftle", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    c.set("harness", json!("d_min"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c.harness(), Some("d_min"));
    c.set("width", json!(32)).unwrap_or_else(|e| panic!("{e}"));
    check_set_refused(&mut c, "constants.k");
    check_set_refused(&mut c, "shader");
    let mut s = config(json!({ "shader": "a.wgsl", "fragment": "f", "width": 1, "height": 1 }))
        .unwrap_or_else(|e| panic!("{e}"));
    s.set("constants.k", json!(2))
        .unwrap_or_else(|e| panic!("{e}"));
    check_set_refused(&mut s, "harness");
}

negative_control!(
    golden_harness_config_sets_its_own_fields,
    "a harness case's own field is settable, so expecting it refused fails",
    expected = "is set",
    {
        let mut c = config(json!({ "harness": "ftle", "width": 64, "height": 8 }))
            .unwrap_or_else(|e| panic!("{e}"));
        check_set_refused(&mut c, "harness");
    }
);

/// The float image of `width` × `height` pixels, each `(k, 0, 0, 1)` for its index `k`, with `extra` bytes more or
/// fewer.
fn image(width: u32, height: u32, extra: isize) -> Vec<u8> {
    let mut out = HARNESS_MAGIC.to_vec();
    out.extend(width.to_le_bytes());
    out.extend(height.to_le_bytes());
    for k in 0..width * height {
        for c in [k as f32, 0.0, 0.0, 1.0] {
            out.extend(c.to_le_bytes());
        }
    }
    match extra {
        0 => {}
        e if e > 0 => out.extend(std::iter::repeat_n(0u8, e as usize)),
        e => out.truncate(out.len() - e.unsigned_abs()),
    }
    out
}

/// Checks that `bytes` is refused as a `width` × `height` image with a message containing `why`.
fn check_decode_refused(bytes: &[u8], width: u32, height: u32, why: &str) {
    match decode_harness(bytes, width, height) {
        Ok(_) => panic!("the image is accepted"),
        Err(e) => assert!(e.contains(why), "refused as `{e}`, not `{why}`"),
    }
}

#[test]
fn golden_harness_decode_is_whole_and_sized() {
    let floats = decode_harness(&image(3, 2, 0), 3, 2).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(floats.len(), 3 * 2 * 16);
    assert_eq!(&floats[16..20], &1.0f32.to_le_bytes(), "pixel 1's red");
    assert_eq!(decode_harness(&image(0, 0, 0), 0, 0), Ok(Vec::new()));
    check_decode_refused(&image(3, 2, 1), 3, 2, "not 96");
    check_decode_refused(&image(3, 2, -1), 3, 2, "not 96");
    check_decode_refused(
        &image(3, 2, 0),
        2,
        3,
        "the scene is 3 × 2, but the case says 2 × 3",
    );
    check_decode_refused(&image(3, 2, 0), 3, 3, "the scene is 3 × 2");
    check_decode_refused(&image(3, 2, 0)[1..], 3, 2, "magic");
    check_decode_refused(&HARNESS_MAGIC[..], 3, 2, "no size");
    check_decode_refused(&image(3, 2, 0)[..15], 3, 2, "no size");
}

negative_control!(
    golden_harness_decode_is_whole_and_sized,
    "a whole image of the right size is accepted",
    expected = "the image is accepted",
    check_decode_refused(&image(3, 2, 0), 3, 2, "not 96")
);

/// Checks that `levels` (RGB bytes) are `floats` rounded half to even, per channel.
fn check_levels(floats: &[[f32; 4]], levels: &[u8]) {
    for (k, p) in floats.iter().enumerate() {
        for c in 0..3 {
            let want = (p[c].clamp(0.0, 1.0) * 255.0).round_ties_even() as u8;
            assert_eq!(
                levels[3 * k + c],
                want,
                "pixel {k} channel {c}: {} is not stored as {want}",
                p[c]
            );
        }
    }
}

#[test]
fn golden_harness_renders_through_the_binary() {
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let c = config(json!({ "harness": "length_ramp", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    let floats = renderer
        .render_float(dir, &c)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(floats.len(), 64 * 8);
    let image = renderer.render(dir, &c).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((image.width, image.height), (64, 8));
    check_levels(&floats, &image.rgb);
    let distinct: std::collections::BTreeSet<[u8; 3]> =
        (0..64).map(|x| image.pixel(x, 0)).collect();
    assert!(
        distinct.len() > 4,
        "the scene's samples differ: {distinct:?}"
    );
    match renderer.render_output(dir, &c, Output::Automatic) {
        Ok(_) => panic!("a harness case rendered with the automatic output"),
        Err(e) => assert!(e.contains("only the quantised output"), "{e}"),
    }
    let wrong = config(json!({ "harness": "length_ramp", "width": 32, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    match renderer.render(dir, &wrong) {
        Ok(_) => panic!("a case of the wrong size rendered"),
        Err(e) => assert!(
            e.contains("the scene is 64 × 8, but the case says 32 × 8"),
            "{e}"
        ),
    }
    let unknown = config(json!({ "harness": "no_such_scene", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    match renderer.render(dir, &unknown) {
        Ok(_) => panic!("an unknown scene rendered"),
        Err(e) => assert!(
            e.contains("golden_harness --scene no_such_scene failed"),
            "{e}"
        ),
    }
}

negative_control!(
    golden_harness_renders_through_the_binary,
    "levels rounded half away from zero, not to even, at a tie, are not the quantise pass's",
    expected = "is not stored as",
    check_levels(&[[0.5 / 255.0, 0.0, 0.0, 1.0]], &[1, 0, 0])
);
