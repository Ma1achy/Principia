//! Export & share (render_gui_spec §G5 "Export & share"; GUI_DESIGN_NOTES § "04 Windows"; 04_windows.png; R-309,
//! R-406, R-407): the image, its size in multiples of the view and its format, with the view embedded (pxpack) and
//! optionally the stain's WGSL; the state, as snapshot JSON (copy, save, load) and a share link; and present mode,
//! which hides all chrome until Esc.
//!
//! On the mock (applied per R-369): copy puts `SimConfig` and `RenderState` on the clipboard in their one canonical
//! serialisation, JCS (gui_state_contract §2, R-309; `engine::contract::canonical`), and the share link carries the
//! same JSON percent-encoded, `principia://view?v=1&s=…` (§G5); Save PNG…, Save snapshot… and Load… write and read no
//! file, which TASK-M8-32 builds. Present hides the chrome and Esc returns, the focus on Present (applied per R-369);
//! making the figure fill the window is TASK-M6-30's (R-407, A3).
//!
//! (The file is `export.rs`, as TASK-M6-28's deliverables name it; TASK-M8-32 expects `export_share.rs`.)

use eframe::egui::{self, Button, Checkbox, RichText, Ui};
use engine::contract::canonical;
use engine::contract::render_state::RenderState;
use engine::contract::sim_config::SimConfig;

use crate::keyboard::scopes::{Scope, ScopeId, ScopeTree};
use crate::windows::{form_label, note, section, Keys};

/// The window's scope.
pub const WINDOW: ScopeId = "win_export";
/// Its title, as 04_windows.png's.
pub const TITLE: &str = "Export & share";

/// The Image section.
pub const IMAGE: ScopeId = "ex_image";
/// The State section.
pub const STATE: ScopeId = "ex_state";
/// The Present section.
pub const PRESENT_SECTION: ScopeId = "ex_present_section";

/// The image's size, in multiples of the view.
pub const SIZE: ScopeId = "ex_size";
/// The image's format.
pub const FORMAT: ScopeId = "ex_format";
/// Embed the view (pxpack).
pub const PXPACK: ScopeId = "ex_pxpack";
/// Embed the stain's WGSL.
pub const WGSL: ScopeId = "ex_wgsl";
/// Save PNG….
pub const SAVE_PNG: ScopeId = "ex_save_png";
/// Copy snapshot (JSON).
pub const COPY_SNAPSHOT: ScopeId = "ex_copy_snapshot";
/// Save snapshot….
pub const SAVE_SNAPSHOT: ScopeId = "ex_save_snapshot";
/// Load….
pub const LOAD: ScopeId = "ex_load";
/// The share link's Copy.
pub const COPY_LINK: ScopeId = "ex_copy_link";
/// Present (hide all chrome).
pub const PRESENT: ScopeId = "ex_present";

/// The image sizes offered, in multiples of the view (a look choice, R-390: 04_windows.png shows "(2×)").
pub const MULTIPLES: [u32; 3] = [1, 2, 4];
/// The image format: PNG, the one that carries pxpack (§G5).
pub const PNG: &str = "PNG";
/// Present's label (04_windows.png).
pub const PRESENT_LABEL: &str = "Present (hide all chrome)";
/// The line beside it.
pub const PRESENT_NOTE: &str = "Esc returns";
/// The line under the image's controls (§G5).
pub const IMAGE_NOTE: &str =
    "the picture carries its own settings: open it to recreate the view exactly";
/// What a file control says on the mock.
pub const NO_FILE: &str = "the mock writes and reads no file";

/// Export & share's own choices: the GUI's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    /// The size, by its index in [`MULTIPLES`]: 2× by default (04_windows.png).
    pub multiple: usize,
    /// Embed the view (pxpack): on by default (§G5).
    pub pxpack: bool,
    /// Embed the stain's WGSL: optional, off by default.
    pub wgsl: bool,
    /// The last file control's note, if any.
    pub note: Option<&'static str>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            multiple: 1,
            pxpack: true,
            wgsl: false,
            note: None,
        }
    }
}

/// What an Export & share control asks of the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// Copy this text to the clipboard.
    Copy(String),
    /// Enter present mode.
    Present,
}

/// The snapshot JSON: `{"render": RenderState, "sim": SimConfig}` in the canonical serialisation, JCS, its keys in
/// JCS's order (gui_state_contract §2, R-309, R-318).
pub fn snapshot_json(sim: &SimConfig, render: &RenderState) -> Result<String, canonical::Error> {
    Ok(format!(
        "{{\"render\":{},\"sim\":{}}}",
        canonical::to_string(render)?,
        canonical::to_string(sim)?
    ))
}

/// The share link's form up to its state: version 1, then `s=`, the snapshot JSON percent-encoded.
pub const LINK_PREFIX: &str = "principia://view?v=1&s=";
/// How much of the share link the window shows; Copy takes the whole link (a look choice, R-390).
pub const LINK_SHOWN: usize = 72;

/// The share link: `SimConfig` and `RenderState` in their one canonical serialisation, the snapshot JSON, carried in
/// the query percent-encoded (render_gui_spec §G5 "Export & share", gui_state_contract §2, R-309).
pub fn share_link(sim: &SimConfig, render: &RenderState) -> Result<String, canonical::Error> {
    Ok(format!(
        "{LINK_PREFIX}{}",
        percent_encode(&snapshot_json(sim, render)?)
    ))
}

/// The snapshot JSON a share link carries, or `None` if `link` is not one (what TASK-M8-32 reads back).
pub fn link_snapshot(link: &str) -> Option<String> {
    percent_decode(link.strip_prefix(LINK_PREFIX)?)
}

/// What the window shows of `link`: its first [`LINK_SHOWN`] characters, then its length.
pub fn link_shown(link: &str) -> String {
    let n = link.chars().count();
    if n <= LINK_SHOWN {
        link.to_owned()
    } else {
        let head: String = link.chars().take(LINK_SHOWN).collect();
        format!("{head}… ({n} characters)")
    }
}

/// `text` percent-encoded (RFC 3986 §2.1): every UTF-8 byte but the unreserved characters (§2.3) as `%XX`.
fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The inverse of [`percent_encode`]: `None` for a malformed escape or bytes that are not UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The image's size in pixels at `multiple` times a view of `view` pixels.
pub fn image_size(view: [u32; 2], multiple: u32) -> [u32; 2] {
    view.map(|v| v * multiple)
}

/// Joins the window's sections and elements to `tree`, in order.
pub fn register(tree: &mut ScopeTree) {
    tree.register(Some(WINDOW), Scope::group(IMAGE, "Image"));
    for (id, label) in [
        (SIZE, "size"),
        (FORMAT, "format"),
        (PXPACK, "embed the view (pxpack)"),
        (WGSL, "embed the stain's WGSL"),
        (SAVE_PNG, "Save PNG…"),
    ] {
        tree.register(Some(IMAGE), Scope::control(id, label));
    }
    tree.register(Some(WINDOW), Scope::group(STATE, "State"));
    for (id, label) in [
        (COPY_SNAPSHOT, "Copy snapshot (JSON)"),
        (SAVE_SNAPSHOT, "Save snapshot…"),
        (LOAD, "Load…"),
        (COPY_LINK, "Copy share link"),
    ] {
        tree.register(Some(STATE), Scope::control(id, label));
    }
    tree.register(Some(WINDOW), Scope::group(PRESENT_SECTION, "Present"));
    tree.register(Some(PRESENT_SECTION), Scope::control(PRESENT, "Present"));
}

/// Enter on control `id`, as its click, with the snapshot's state.
pub fn activate(id: &str, state: &mut State, sim: &SimConfig, render: &RenderState) -> Option<Act> {
    match id {
        SIZE => state.multiple = (state.multiple + 1) % MULTIPLES.len(),
        PXPACK => state.pxpack = !state.pxpack,
        WGSL => state.wgsl = !state.wgsl,
        SAVE_PNG | SAVE_SNAPSHOT | LOAD => state.note = Some(NO_FILE),
        COPY_SNAPSHOT => return snapshot_json(sim, render).ok().map(Act::Copy),
        COPY_LINK => return share_link(sim, render).ok().map(Act::Copy),
        PRESENT => return Some(Act::Present),
        _ => {}
    }
    None
}

/// Draws the window's content, `view` the figure's size in pixels; its clicks act as Enter does.
pub fn show(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    state: &mut State,
    (sim, render): (&SimConfig, &RenderState),
    view: [u32; 2],
) -> Option<Act> {
    let mut clicked: Vec<ScopeId> = Vec::new();
    section(ui, keys, IMAGE, "Image", |ui, keys| {
        ui.horizontal(|ui| {
            form_label(ui, "size");
            let multiple = MULTIPLES[state.multiple];
            let [w, h] = image_size(view, multiple);
            let size = ui.button(format!("{w} × {h} ({multiple}×)"));
            let format = ui.add(Button::selectable(true, PNG));
            for (id, r) in [(SIZE, &size), (FORMAT, &format)] {
                if r.clicked() {
                    clicked.push(id);
                }
                keys.place(id, r);
            }
        });
        let mut pxpack = state.pxpack;
        let embed = ui.add(Checkbox::new(&mut pxpack, "embed the view (pxpack)"));
        let mut wgsl = state.wgsl;
        let shader = ui.add(Checkbox::new(&mut wgsl, "embed the stain's WGSL"));
        note(ui, IMAGE_NOTE);
        let save = ui.button("Save PNG…");
        for (id, r) in [(PXPACK, &embed), (WGSL, &shader), (SAVE_PNG, &save)] {
            if r.clicked() {
                clicked.push(id);
            }
            keys.place(id, r);
        }
    });
    section(ui, keys, STATE, "State", |ui, keys| {
        ui.horizontal_wrapped(|ui| {
            for (id, label) in [
                (COPY_SNAPSHOT, "Copy snapshot (JSON)"),
                (SAVE_SNAPSHOT, "Save snapshot…"),
                (LOAD, "Load…"),
            ] {
                let button = ui.button(label);
                if button.clicked() {
                    clicked.push(id);
                }
                keys.place(id, &button);
            }
        });
        ui.horizontal(|ui| {
            form_label(ui, "share link");
            // The link's head and its length: the whole state is longer than the window; Copy takes all of it.
            let link = share_link(sim, render).unwrap_or_default();
            ui.add(egui::Label::new(RichText::new(link_shown(&link)).monospace()).wrap());
        });
        let copy = ui.button("Copy");
        if copy.clicked() {
            clicked.push(COPY_LINK);
        }
        keys.place(COPY_LINK, &copy);
        if let Some(text) = state.note {
            note(ui, text);
        }
    });
    section(ui, keys, PRESENT_SECTION, "Present", |ui, keys| {
        ui.horizontal(|ui| {
            let present = ui.button(PRESENT_LABEL);
            if present.clicked() {
                clicked.push(PRESENT);
            }
            keys.place(PRESENT, &present);
            note(ui, PRESENT_NOTE);
        });
    });
    let mut act = None;
    for id in clicked {
        act = activate(id, state, sim, render).or(act);
    }
    act
}
