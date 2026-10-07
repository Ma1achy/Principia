//! The theme (REQ-GUI-075; RQ-251): egui's dark theme; text keeps egui's own Ubuntu Light first; Ubuntu Mono Regular,
//! the committed font, is the `Monospace` family's first font; egui's Hack is the last fallback for text.

use eframe::egui::{self, FontFamily};

use crate::theme::{fonts, install, UBUNTU_MONO};

#[test]
fn theme_fonts_and_the_dark_theme() {
    let defs = fonts();
    assert_eq!(defs.families[&FontFamily::Monospace][0], UBUNTU_MONO);
    let text = &defs.families[&FontFamily::Proportional];
    assert_eq!(
        text[0], "Ubuntu-Light",
        "text is not egui's own Ubuntu Light"
    );
    assert_eq!(text.last().map(String::as_str), Some("Hack"));
    assert!(defs.font_data.contains_key(UBUNTU_MONO));
    let ctx = egui::Context::default();
    install(&ctx);
    assert_eq!(ctx.theme(), egui::Theme::Dark);
    assert!(!ctx.global_style().interaction.selectable_labels);
    // The fonts are the context's once a pass has run.
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    let families = ctx.fonts(|f| f.definitions().families.clone());
    assert_eq!(families[&FontFamily::Monospace][0], UBUNTU_MONO);
}
