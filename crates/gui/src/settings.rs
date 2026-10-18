//! The GUI's own settings, kept across sessions (R-461 I4): each window's place, by its name (TASK-M6-43 adds the panel
//! layout). Never the contract (gui_state_contract §2): not undoable, never read by the engine.
//!
//! **How they are stored** (TASK-M6-42's choice): one JSON file, `gui_settings.json`, in a `principia` directory under
//! the platform's config directory ([`platform_path`]), written by the `serde_json` the crate already depends on and
//! the standard library alone, so no dependency is added; eframe's own persistence would add its `persistence`
//! feature's crates and keep egui's memory as well. The file is
//! `{"windows": {"<name>": {"x": …, "y": …, "w": …, "h": …}, …}}`, each place in points. It is written each time a
//! window's move or resize ends, through a file beside it renamed over it, so a write cut short leaves the last one.
//!
//! A file that cannot be read is ignored, logged once to the console, and replaced on the next save; with no file,
//! every window opens at the artboards' place. A failed save is logged once to the console; nothing else depends on it.

use std::path::{Path, PathBuf};

use eframe::egui::{pos2, vec2, Rect};
use serde_json::{json, Map, Value};

use crate::windows::{self, place::Places};

/// The settings file's name.
pub const FILE: &str = "gui_settings.json";

/// The directory under the platform's config directory that holds it.
pub const DIR: &str = "principia";

/// The GUI's settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    /// Each window's place.
    pub places: Places,
}

impl Settings {
    /// The settings as the file's text.
    pub fn to_json(&self) -> String {
        let mut windows = Map::new();
        for (window, rect) in self.places.rects() {
            windows.insert(
                windows::entry(*window).name.to_owned(),
                json!({ "x": rect.min.x, "y": rect.min.y, "w": rect.width(), "h": rect.height() }),
            );
        }
        let mut text = serde_json::to_string_pretty(&json!({ "windows": windows }))
            .expect("a JSON value of finite numbers serialises");
        text.push('\n');
        text
    }

    /// The settings the file's `text` holds, or why it cannot be read: not JSON, not an object, or a place that is
    /// not four finite numbers with a positive size. A window name the GUI does not have is skipped.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let root = value.as_object().ok_or("not a JSON object")?;
        let mut rects = Vec::new();
        if let Some(places) = root.get("windows") {
            let places = places.as_object().ok_or("`windows` is not an object")?;
            for (name, place) in places {
                let rect = rect(place)
                    .ok_or_else(|| format!("the place of `{name}` is not x, y, w, h"))?;
                if let Some(e) = windows::REGISTRY.iter().find(|e| e.name == name) {
                    rects.push((e.window, rect));
                }
            }
        }
        Ok(Self {
            places: Places::remembered(rects),
        })
    }
}

/// A place's rect: `{x, y, w, h}`, each finite, `w` and `h` positive.
fn rect(place: &Value) -> Option<Rect> {
    let get = |k: &str| {
        place
            .get(k)
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .filter(|v| v.is_finite())
    };
    let (x, y, w, h) = (get("x")?, get("y")?, get("w")?, get("h")?);
    (w > 0.0 && h > 0.0).then(|| Rect::from_min_size(pos2(x, y), vec2(w, h)))
}

/// Where the settings are kept: a file, or nowhere (the tests' GUIs, and a platform with no config directory).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Store {
    path: Option<PathBuf>,
}

/// What loading the settings found: the settings, and the console's line when the file could not be read.
#[derive(Debug, Default, PartialEq)]
pub struct Loaded {
    /// The settings: the file's, or the defaults.
    pub settings: Settings,
    /// Why the file was ignored, as the console's line.
    pub ignored: Option<String>,
}

impl Store {
    /// Settings kept nowhere: each GUI starts with the defaults and saves nothing.
    pub fn none() -> Self {
        Self::default()
    }

    /// Settings kept in the file at `path`.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Some(path.into()),
        }
    }

    /// Settings kept in the platform's config directory ([`platform_path`]), or nowhere without one.
    pub fn platform() -> Self {
        Self {
            path: platform_path(),
        }
    }

    /// The file, if any.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Loads the settings: the defaults with no store or no file; the defaults and the console's line when the file
    /// cannot be read.
    pub fn load(&self) -> Loaded {
        let Some(path) = &self.path else {
            return Loaded::default();
        };
        let read = match std::fs::read_to_string(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Loaded::default(),
            Err(e) => Err(e.to_string()),
            Ok(text) => Settings::from_json(&text),
        };
        match read {
            Ok(settings) => Loaded {
                settings,
                ignored: None,
            },
            Err(why) => Loaded {
                settings: Settings::default(),
                ignored: Some(format!(
                    "the GUI's settings at {} could not be read ({why}); the defaults are used, and the file is \
                     replaced on the next save",
                    path.display()
                )),
            },
        }
    }

    /// Saves `settings`, through a file beside the store's renamed over it; nothing with no store.
    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let write = || -> std::io::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let part = path.with_extension("json.part");
            std::fs::write(&part, settings.to_json())?;
            std::fs::rename(&part, path)
        };
        write().map_err(|e| {
            format!(
                "the GUI's settings could not be saved at {} ({e})",
                path.display()
            )
        })
    }
}

/// The settings file under the platform's config directory: `~/Library/Application Support` on macOS, `%APPDATA%` on
/// Windows, else `$XDG_CONFIG_HOME` or `~/.config`; none when the variable it reads is unset.
pub fn platform_path() -> Option<PathBuf> {
    let var = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let base = if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library").join("Application Support"))
    } else if cfg!(windows) {
        var("APPDATA")
    } else {
        var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|h| h.join(".config")))
    };
    base.map(|b| b.join(DIR).join(FILE))
}
