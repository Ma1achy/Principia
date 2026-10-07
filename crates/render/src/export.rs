//! The image export (principia_coordinate_conventions_note.md, "Code paths that must honour the single convention",
//! path 5; REQ-SYS-080; TASK-M1-07): a rendered target, read back by the headless helper ([`crate::headless`]), as a
//! PNG.
//!
//! **The export seam** ([`png_rows`]). The readback is already display-oriented: its rows run from the top, as the
//! framebuffer's do and as PNG stores them ([`Image`]: "rows from the top"). So the export writes the PNG's rows in
//! the readback's order and reverses none. Path 5's flip, the encode step flipping back once, fires only for a Y-up
//! internal image; the readback is not one, so it does not fire on this path (RQ-212), and the one-line rule's
//! "mirrored once at image export", path 5's flip stated in short, does not fire here either. A mirrored export is a
//! wrong count of the convention's one flip (`coords.wgsl`), fixed there, never by reversing rows here.

use std::io::Write;

use crate::headless::Image;

/// The formats the export writes: 8-bit RGBA, linear-coded or sRGB-coded, each texel's bytes as the PNG's.
fn rgba8(format: wgpu::TextureFormat) -> bool {
    matches!(
        format,
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb
    )
}

/// **The export seam**: the PNG's rows, top first, in the readback's order, which already runs from the top; no row
/// is reversed (path 5's flip does not fire for a readback; REQ-SYS-080). Each row is `width` RGBA texels.
pub fn png_rows(image: &Image) -> impl Iterator<Item = &[u8]> {
    let row = image.width as usize * image.texel_size();
    image.bytes.chunks_exact(row.max(1))
}

/// `image` as an 8-bit RGBA PNG, its rows from [`png_rows`], written to `out`. A target that is not 8-bit RGBA, or
/// whose bytes are not `width × height` texels, is refused.
pub fn write_png(image: &Image, out: impl Write) -> Result<(), String> {
    if !rgba8(image.format) {
        return Err(format!(
            "{:?} is not an 8-bit RGBA target; the export writes Rgba8Unorm or Rgba8UnormSrgb",
            image.format
        ));
    }
    let expected = image.width as usize * image.height as usize * image.texel_size();
    if image.bytes.len() != expected || expected == 0 {
        return Err(format!(
            "{} bytes for a {} × {} RGBA8 image ({expected} expected)",
            image.bytes.len(),
            image.width,
            image.height
        ));
    }
    let mut encoder = png::Encoder::new(out, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    let data: Vec<u8> = png_rows(image).flatten().copied().collect();
    writer.write_image_data(&data).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())
}
