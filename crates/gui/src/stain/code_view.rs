//! The assembled code under the canvas (render_gui_spec Part II §1, §10; 02_stain.png), and the code the canvas's
//! Pipeline WGSL and Node WGSL views show: the engine side's text, in egui's monospace, comments, keywords and types
//! in their colours. Read-only here: editing a node's code is TASK-M7-24's.

use eframe::egui::text::LayoutJob;
use eframe::egui::{self, vec2, Color32, FontId, Rect, TextFormat, Ui, UiBuilder};

use super::Content;

/// WGSL's keywords the code colours.
pub const KEYWORDS: [&str; 8] = [
    "fn", "return", "let", "var", "if", "else", "struct", "#import",
];
/// The types the code colours.
pub const TYPES: [&str; 8] = [
    "f32",
    "u32",
    "i32",
    "vec3<f32>",
    "vec4<f32>",
    "Ctx",
    "SampleCtx",
    "Field",
];

/// A token's colour class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Plain text.
    Plain,
    /// A `//` comment, to the line's end.
    Comment,
    /// A keyword.
    Keyword,
    /// A type.
    Type,
}

/// `line` split into runs, each with its class: a comment runs to the line's end; otherwise each word is a keyword,
/// a type or plain, and the rest is plain.
pub fn classify(line: &str) -> Vec<(String, Class)> {
    let (code, comment) = match line.find("//") {
        Some(i) => (&line[..i], Some(&line[i..])),
        None => (line, None),
    };
    let mut runs: Vec<(String, Class)> = Vec::new();
    let mut push = |text: &str, class: Class| match runs.last_mut() {
        Some((t, c)) if *c == class => t.push_str(text),
        _ => runs.push((text.to_owned(), class)),
    };
    let mut word = String::new();
    let flush = |word: &mut String, push: &mut dyn FnMut(&str, Class)| {
        if word.is_empty() {
            return;
        }
        let class = if KEYWORDS.contains(&word.as_str()) {
            Class::Keyword
        } else if TYPES.contains(&word.as_str()) {
            Class::Type
        } else {
            Class::Plain
        };
        push(word, class);
        word.clear();
    };
    for ch in code.chars() {
        if ch.is_alphanumeric() || matches!(ch, '_' | '<' | '>' | '#') {
            word.push(ch);
        } else {
            flush(&mut word, &mut push);
            push(&ch.to_string(), Class::Plain);
        }
    }
    flush(&mut word, &mut push);
    if let Some(comment) = comment {
        push(comment, Class::Comment);
    }
    runs
}

/// The colour of `class` (02_stain.png's: comments green, keywords pink, types teal; a look choice, R-390).
pub fn colour(class: Class, visuals: &egui::Visuals) -> Color32 {
    match class {
        Class::Plain => visuals.text_color(),
        Class::Comment => Color32::from_rgb(110, 170, 100),
        Class::Keyword => Color32::from_rgb(220, 120, 170),
        Class::Type => Color32::from_rgb(100, 190, 190),
    }
}

/// Draws `text` as code: monospace, coloured by [`classify`].
pub fn code(ui: &mut Ui, text: &str) {
    let visuals = ui.visuals().clone();
    let font = FontId::monospace(13.0);
    let mut job = LayoutJob::default();
    for line in text.lines() {
        for (run, class) in classify(line) {
            job.append(
                &run,
                0.0,
                TextFormat::simple(font.clone(), colour(class, &visuals)),
            );
        }
        job.append(
            "\n",
            0.0,
            TextFormat::simple(font.clone(), visuals.text_color()),
        );
    }
    ui.add(egui::Label::new(job).selectable(false).extend());
}

/// Draws the assembled code in `rect`; with no content, an empty pane.
pub fn show(ui: &mut Ui, rect: Rect, content: Option<&Content>) {
    let visuals = ui.visuals().clone();
    ui.painter().rect_filled(rect, 0.0, visuals.panel_fill);
    ui.painter().line_segment(
        [rect.left_top(), rect.right_top()],
        visuals.widgets.noninteractive.bg_stroke,
    );
    let mut pane = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(12.0, 10.0))));
    pane.set_clip_rect(rect.intersect(ui.clip_rect()));
    if let Some(c) = content {
        egui::ScrollArea::both()
            .id_salt("stain_code_text")
            .auto_shrink(false)
            .show(&mut pane, |ui| code(ui, &c.code));
    }
}
