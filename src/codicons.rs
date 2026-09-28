//! Real VS Code UI icons (Codicons, © Microsoft, CC BY 4.0 — see
//! `assets/codicon-LICENSE.txt`). `assets/codicon.ttf` is a small subset of
//! the font holding just the glyphs used here, at the official Codicon
//! codepoints (from the vscode-codicons `mapping.json`).

pub const FILES: &str = "\u{EAF0}";
pub const SEARCH: &str = "\u{EA6D}";
pub const SETTINGS_GEAR: &str = "\u{EB51}";
pub const FOLDER: &str = "\u{EA83}";
pub const FOLDER_OPENED: &str = "\u{EAF7}";
pub const FILE: &str = "\u{EA7B}";
pub const ADD: &str = "\u{EA60}";

/// Registered by `with_icon_fonts` in `app.rs`, under its own dedicated
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
