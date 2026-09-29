//! Real VS Code UI icons (Codicons, © Microsoft, CC BY 4.0 — see
//! `assets/codicon-LICENSE.txt`). `assets/codicon.ttf` is a small subset of
//! the font holding just the glyphs used here, at the official Codicon
//! codepoints (from the vscode-codicons `mapping.json`).
#![allow(dead_code)]

pub const FILES: &str = "\u{EAF0}";
pub const SEARCH: &str = "\u{EA6D}";
pub const SETTINGS_GEAR: &str = "\u{EB51}";
pub const FOLDER: &str = "\u{EA83}";
pub const FOLDER_OPENED: &str = "\u{EAF7}";
pub const FILE: &str = "\u{EA7B}";
pub const ADD: &str = "\u{EA60}";
pub const TERMINAL: &str = "\u{EA85}";
pub const TRASH: &str = "\u{EA81}";
pub const CLOSE: &str = "\u{EA76}";
pub const CHEVRON_UP: &str = "\u{EAB7}";
pub const CHEVRON_DOWN: &str = "\u{EAB4}";
pub const CHEVRON_RIGHT: &str = "\u{EAB6}";
pub const CLEAR_ALL: &str = "\u{EABF}";
pub const CIRCLE_FILLED: &str = "\u{EA71}";
pub const DEBUG_STOP: &str = "\u{EAD7}";
pub const CHROME_CLOSE: &str = "\u{EAB8}";
pub const CHROME_MAXIMIZE: &str = "\u{EAB9}";
pub const CHROME_MINIMIZE: &str = "\u{EABA}";
pub const CHROME_RESTORE: &str = "\u{EABB}";
pub const SPLIT_HORIZONTAL: &str = "\u{EB56}";
pub const LAYOUT_PANEL: &str = "\u{EBF2}";
pub const SAVE: &str = "\u{EB4B}";
pub const SAVE_AS: &str = "\u{EB4A}";
pub const NEW_FILE: &str = "\u{EA7F}";
pub const NEW_FOLDER: &str = "\u{EA80}";
pub const REFRESH: &str = "\u{EB37}";
pub const COLLAPSE_ALL: &str = "\u{EAC5}";
pub const ARROW_UP: &str = "\u{EAA1}";
pub const ARROW_DOWN: &str = "\u{EA9A}";
pub const CASE_SENSITIVE: &str = "\u{EAB1}";
pub const WHOLE_WORD: &str = "\u{EB7E}";
pub const REGEX: &str = "\u{EB38}";
pub const PLAY: &str = "\u{EB2C}";

/// Registered by `with_icon_fonts` in `app/theme_fonts.rs`, under its own dedicated
/// family so it never mixes into the regular text fallback chains.
pub const FONT_NAME: &str = "codicon";

pub fn font_family() -> egui::FontFamily {
    egui::FontFamily::Name(std::sync::Arc::from(FONT_NAME))
}

pub fn font_id(size: f32) -> egui::FontId {
    egui::FontId::new(size, font_family())
}

/// A codicon glyph as `RichText`, for buttons/labels.
pub fn rich(icon: &str, size: f32) -> egui::RichText {
    egui::RichText::new(icon).font(font_id(size))
}
