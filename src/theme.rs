use egui::Color32;
use serde::{Deserialize, Serialize};

/// Which theme is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeKind {
    Light,
    Dark,
    Char,
    Custom,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 4] = [
        ThemeKind::Light,
        ThemeKind::Dark,
        ThemeKind::Char,
        ThemeKind::Custom,
    ];

    pub fn label(&self, lang: crate::i18n::Lang) -> &'static str {
        match self {
            ThemeKind::Light => crate::i18n::t(lang, "theme.light"),
            ThemeKind::Dark => crate::i18n::t(lang, "theme.dark"),
            ThemeKind::Char => "Char",
            ThemeKind::Custom => crate::i18n::t(lang, "theme.custom_css"),
        }
    }
}

/// A full color palette for the editor UI. `Theme` is the single source of
/// truth for every color used in the app; built-in presets and the
/// user-supplied CSS theme both produce one of these.
#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color32,
    pub panel_bg: Color32,
    pub titlebar_bg: Color32,
    pub titlebar_fg: Color32,
    pub sidebar_bg: Color32,
    pub editor_bg: Color32,
    pub fg: Color32,
    pub fg_dim: Color32,
    pub accent: Color32,
    pub line_number: Color32,
    pub line_number_active: Color32,
    pub selection: Color32,
    pub cursor: Color32,
    pub button_hover: Color32,
    pub button_active: Color32,
    pub border: Color32,
    pub error: Color32,
    pub close_hover: Color32,
}

impl Theme {
    /// VS Code's current default light theme ("Light Modern"). Values are
    /// the actual color tokens from `extensions/theme-defaults/themes/
    /// light_modern.json` in the VS Code source (editor.background,
    /// sideBar.background, focusBorder, ...), not eyeballed.
    pub fn light() -> Self {
        Self {
            bg: Color32::from_rgb(0xf8, 0xf8, 0xf8),               // tab.inactiveBackground / editorGroupHeader.tabsBackground
            panel_bg: Color32::from_rgb(0xff, 0xff, 0xff),         // editor.background / tab.activeBackground
            titlebar_bg: Color32::from_rgb(0xf8, 0xf8, 0xf8),      // titleBar.activeBackground
            titlebar_fg: Color32::from_rgb(0x1e, 0x1e, 0x1e),      // titleBar.activeForeground
            sidebar_bg: Color32::from_rgb(0xf8, 0xf8, 0xf8),       // sideBar.background / activityBar.background
            editor_bg: Color32::from_rgb(0xff, 0xff, 0xff),        // editor.background
            fg: Color32::from_rgb(0x3b, 0x3b, 0x3b),               // editor.foreground
            fg_dim: Color32::from_rgb(0x61, 0x61, 0x61),           // tab.inactiveForeground / activityBar.inactiveForeground
            accent: Color32::from_rgb(0x00, 0x5f, 0xb8),           // focusBorder / button.background
            line_number: Color32::from_rgb(0x6e, 0x76, 0x81),      // editorLineNumber.foreground
            line_number_active: Color32::from_rgb(0x17, 0x11, 0x84), // editorLineNumber.activeForeground
            selection: Color32::from_rgba_premultiplied(0x00, 0x5f, 0xb8, 55),
            cursor: Color32::from_rgb(0x3b, 0x3b, 0x3b),
            button_hover: Color32::from_rgb(0xe8, 0xe8, 0xe8),
            button_active: Color32::from_rgb(0xdc, 0xdc, 0xdc),
            border: Color32::from_rgb(0xe5, 0xe5, 0xe5),           // tab.border
            error: Color32::from_rgb(0xd9, 0x3a, 0x3a),
            close_hover: Color32::from_rgb(0xc4, 0x2b, 0x1c),      // Windows-style close-hover red, matches VS Code's own title bar
        }
    }

    /// VS Code's current default dark theme ("Dark Modern"). Values are
    /// the actual color tokens from `extensions/theme-defaults/themes/
    /// dark_modern.json` in the VS Code source.
    pub fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(0x18, 0x18, 0x18),               // tab.inactiveBackground / editorGroupHeader.tabsBackground
            panel_bg: Color32::from_rgb(0x1f, 0x1f, 0x1f),         // editor.background / tab.activeBackground
            titlebar_bg: Color32::from_rgb(0x18, 0x18, 0x18),      // titleBar.activeBackground
            titlebar_fg: Color32::from_rgb(0xcc, 0xcc, 0xcc),      // titleBar.activeForeground
            sidebar_bg: Color32::from_rgb(0x18, 0x18, 0x18),       // sideBar.background / activityBar.background
            editor_bg: Color32::from_rgb(0x1f, 0x1f, 0x1f),        // editor.background
            fg: Color32::from_rgb(0xcc, 0xcc, 0xcc),               // editor.foreground
            fg_dim: Color32::from_rgb(0x9d, 0x9d, 0x9d),           // tab.inactiveForeground
            accent: Color32::from_rgb(0x00, 0x78, 0xd4),           // focusBorder / button.background
            line_number: Color32::from_rgb(0x6e, 0x76, 0x81),      // editorLineNumber.foreground
            line_number_active: Color32::from_rgb(0xcc, 0xcc, 0xcc), // editorLineNumber.activeForeground
            selection: Color32::from_rgba_premultiplied(0x26, 0x4f, 0x78, 180),
            cursor: Color32::from_rgb(0xcc, 0xcc, 0xcc),
            button_hover: Color32::from_rgb(0x2a, 0x2a, 0x2a),
            button_active: Color32::from_rgb(0x33, 0x33, 0x33),
            border: Color32::from_rgb(0x2b, 0x2b, 0x2b),           // tab.border / panel.border
            error: Color32::from_rgb(0xe0, 0x6c, 0x6c),
            close_hover: Color32::from_rgb(0xc4, 0x2b, 0x1c),      // Windows-style close-hover red, matches VS Code's own title bar
        }
    }

    /// The "Char" theme — requested base tone #211F24.
    pub fn char_theme() -> Self {
        Self {
            bg: Color32::from_rgb(0x21, 0x1f, 0x24),
            panel_bg: Color32::from_rgb(0x27, 0x25, 0x2b),
            titlebar_bg: Color32::from_rgb(0x1a, 0x18, 0x1d),
            titlebar_fg: Color32::from_rgb(0xe6, 0xe1, 0xec),
            sidebar_bg: Color32::from_rgb(0x24, 0x22, 0x28),
            editor_bg: Color32::from_rgb(0x21, 0x1f, 0x24),
            fg: Color32::from_rgb(0xe0, 0xdc, 0xe6),
            fg_dim: Color32::from_rgb(0x93, 0x8d, 0x9c),
            accent: Color32::from_rgb(0xa6, 0x7c, 0xf2),
            line_number: Color32::from_rgb(0x5c, 0x57, 0x66),
            line_number_active: Color32::from_rgb(0xc7, 0xc0, 0xd4),
            selection: Color32::from_rgba_premultiplied(0xa6, 0x7c, 0xf2, 70),
            cursor: Color32::from_rgb(0xa6, 0x7c, 0xf2),
            button_hover: Color32::from_rgb(0x33, 0x30, 0x3a),
            button_active: Color32::from_rgb(0x3e, 0x3a, 0x46),
            border: Color32::from_rgb(0x35, 0x32, 0x3c),
            error: Color32::from_rgb(0xf2, 0x7c, 0x8d),
            close_hover: Color32::from_rgb(0xc4, 0x3b, 0x3b),
        }
    }

    pub fn is_dark(&self) -> bool {
        luminance(self.bg) < 0.5
    }

    /// Apply this palette to egui's global `Style` (widget rounding/backgrounds/etc).
    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        let v = &mut style.visuals;

        v.dark_mode = luminance(self.bg) < 0.5;
        v.window_fill = self.panel_bg;
        v.panel_fill = self.bg;
        v.faint_bg_color = self.panel_bg;
        v.extreme_bg_color = self.editor_bg;
        v.override_text_color = Some(self.fg);
        v.hyperlink_color = self.accent;
        v.selection.bg_fill = self.selection;
        v.selection.stroke.color = self.accent;

        v.widgets.noninteractive.bg_fill = self.panel_bg;
        v.widgets.noninteractive.fg_stroke.color = self.fg;
        v.widgets.inactive.bg_fill = self.panel_bg;
        v.widgets.inactive.fg_stroke.color = self.fg;
        v.widgets.hovered.bg_fill = self.button_hover;
        v.widgets.hovered.fg_stroke.color = self.fg;
        v.widgets.active.bg_fill = self.button_active;
        v.widgets.active.fg_stroke.color = self.fg;
        v.widgets.open.bg_fill = self.button_active;

        v.window_stroke.color = self.border;
        v.widgets.noninteractive.bg_stroke.color = self.border;

        style.visuals = v.clone();
        ctx.set_style(style);
    }
}

fn luminance(c: Color32) -> f32 {
    0.299 * c.r() as f32 / 255.0 + 0.587 * c.g() as f32 / 255.0 + 0.114 * c.b() as f32 / 255.0
}
