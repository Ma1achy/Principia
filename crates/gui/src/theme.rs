//! The theme (render_gui_spec §G1, "Dark default egui theme, Ubuntu / Ubuntu Mono … no styling beyond egui's own";
//! RQ-251): egui's dark visuals; text keeps egui's own Ubuntu Light; Ubuntu Mono Regular, from the Ubuntu font
//! family's upstream release (`assets/fonts/README.md`, under the Ubuntu Font Licence 1.0, `assets/fonts/UFL.txt`),
//! is egui's `Monospace` family, which numeric fields, readouts and the status line use.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

/// Ubuntu Mono Regular's name among egui's fonts.
pub const UBUNTU_MONO: &str = "Ubuntu-Mono-Regular";

/// egui's own monospace font's name among its fonts.
const HACK: &str = "Hack";

/// The font file, as committed.
const UBUNTU_MONO_TTF: &[u8] = include_bytes!("../assets/fonts/UbuntuMono-R.ttf");

/// egui's fonts with Ubuntu Mono first in the `Monospace` family; `Proportional` is egui's own, Ubuntu Light first,
/// with egui's own Hack last, as a fallback for the marks Ubuntu Light and egui's emoji fonts lack, such as
/// Overlays ▾'s arrow.
pub fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        UBUNTU_MONO.to_owned(),
        FontData::from_static(UBUNTU_MONO_TTF).into(),
    );
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, UBUNTU_MONO.to_owned());
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(HACK.to_owned());
    fonts
}

/// Installs the theme on `ctx`: the dark theme and the fonts. Labels are not selectable, so a click on the footer's
/// text reaches the footer (RQ-249).
pub fn install(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_fonts(fonts());
    ctx.all_styles_mut(|style| style.interaction.selectable_labels = false);
}
