//! The colour resolve: a supersampled field's copies each drawn through the same stain, their colours averaged into
//! the pixel, last.
//!
//! **Order.** Each copy's records are drawn whole through the stain view (classification, colour, combine and post,
//! per sample), one texel per sample; only the finished colours are averaged, pixel by pixel over the `E + 1` copies.
//! Nothing upstream of the colour is averaged: no class, no index, no stored field. Each pixel is its own copies'
//! colours alone, never a neighbour's, so no sample is interpolated.
//!
//! **Colour space.** The plan is unclear here (the render contract's resolve names no space; dd_colouring §3.7 says
//! "blended sRGB"); chose linear RGB, the colour slot's space, because an area-fraction blend is a mix of light: each
//! texel's 8-bit sRGB code is decoded to linear, the copies' mean taken in f64, and encoded back to 8 bits, rounded.
//! Alpha is averaged as it is stored. One copy (E = 0) round-trips every 8-bit code exactly, so its picture is the
//! stain's, byte for byte.
//!
//! **A sample whose decode failed** is drawn by the stain as the canvas background, like any other sample's colour,
//! and that colour enters the mean. The plan is unclear here; chose to average it in because the contract colours
//! every sample through the pipeline and names no sample the resolve skips, and a weight that depended on the data
//! would make the resolve read data.

use crate::headless::Image;
use crate::present::{linear_to_srgb, srgb_to_linear};
use crate::stain_view::{Records, StainView};

/// An 8-bit sRGB code decoded to linear.
fn linear(code: u8) -> f64 {
    srgb_to_linear(f64::from(code) / 255.0)
}

/// A linear value encoded to its nearest 8-bit sRGB code.
fn code(linear: f64) -> u8 {
    (linear_to_srgb(linear.clamp(0.0, 1.0)) * 255.0).round() as u8
}

/// The resolve of `copies`, each the same picture's RGBA8 bytes, 8-bit sRGB-encoded, a texel per sample: each texel's
/// colour the mean of the copies' colours in linear RGB, its alpha the mean of their alphas (the module's note).
/// Refused for no copy, or copies of different lengths or not whole texels.
pub fn resolve_srgb8(copies: &[&[u8]]) -> Result<Vec<u8>, String> {
    let first = copies.first().ok_or("there is no copy to resolve")?;
    if first.len() % 4 != 0 {
        return Err(format!("{} B is not whole RGBA8 texels", first.len()));
    }
    if let Some(c) = copies.iter().find(|c| c.len() != first.len()) {
        return Err(format!(
            "the copies' pictures are {} B and {} B",
            first.len(),
            c.len()
        ));
    }
    let n = copies.len() as f64;
    Ok((0..first.len())
        .map(|i| {
            if i % 4 == 3 {
                let sum: f64 = copies.iter().map(|c| f64::from(c[i])).sum();
                (sum / n).round() as u8
            } else {
                let sum: f64 = copies.iter().map(|c| linear(c[i])).sum();
                code(sum / n)
            }
        })
        .collect())
}

/// Draws each copy's records (`copies[k]`, copy `k`'s whole field) through `view`'s stain, a pixel per sample at the
/// view's placement, and resolves them ([`resolve_srgb8`]): the supersampled picture, `w × h`, in the view's format,
/// rows from the top.
/// The view is left holding copy 0's records. Refused for no copy or records the view refuses.
pub fn draw(view: &mut StainView, copies: &[Records<'_>]) -> Result<Image, String> {
    let [w, h] = view.size();
    let mut pictures = Vec::with_capacity(copies.len());
    for records in copies {
        view.set_records(*records)?;
        pictures.push(view.render(w, h)?);
    }
    if let Some(first) = copies.first() {
        view.set_records(*first)?;
    }
    if let Some(p) = pictures.iter().find(|p| p.texel_size() != 4) {
        return Err(format!(
            "the resolve reads RGBA8 texels, not {:?}",
            p.format
        ));
    }
    let bytes: Vec<&[u8]> = pictures.iter().map(|p| p.bytes.as_slice()).collect();
    Ok(Image {
        width: w,
        height: h,
        format: view.format(),
        bytes: resolve_srgb8(&bytes)?,
    })
}
