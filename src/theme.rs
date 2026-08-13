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

    pub fn label(&self) -> &'static str {
        match self {
            ThemeKind::Light => "Светлая",
            ThemeKind::Dark => "Тёмная",
            ThemeKind::Char => "Char",
            ThemeKind::Custom => "Своя (CSS)",
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
    pub fn light() -> Self {
        Self {
            bg: Color32::from_rgb(0xf7, 0xf7, 0xf8),
            panel_bg: Color32::from_rgb(0xef, 0xef, 0xf1),
            titlebar_bg: Color32::from_rgb(0xe7, 0xe7, 0xea),
            titlebar_fg: Color32::from_rgb(0x20, 0x20, 0x22),
            sidebar_bg: Color32::from_rgb(0xf0, 0xf0, 0xf2),
            editor_bg: Color32::from_rgb(0xff, 0xff, 0xff),
            fg: Color32::from_rgb(0x1c, 0x1c, 0x1e),
            fg_dim: Color32::from_rgb(0x6e, 0x6e, 0x73),
            accent: Color32::from_rgb(0x2f, 0x6f, 0xed),
            line_number: Color32::from_rgb(0xb0, 0xb0, 0xb5),
            line_number_active: Color32::from_rgb(0x50, 0x50, 0x55),
            selection: Color32::from_rgba_premultiplied(0x2f, 0x6f, 0xed, 60),
            cursor: Color32::from_rgb(0x1c, 0x1c, 0x1e),
            button_hover: Color32::from_rgb(0xe0, 0xe0, 0xe4),
            button_active: Color32::from_rgb(0xd4, 0xd4, 0xda),
            border: Color32::from_rgb(0xd8, 0xd8, 0xdc),
            error: Color32::from_rgb(0xd9, 0x3a, 0x3a),
            close_hover: Color32::from_rgb(0xe8, 0x4a, 0x4a),
        }
    }

    pub fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(0x1e, 0x1e, 0x1e),
            panel_bg: Color32::from_rgb(0x25, 0x25, 0x26),
            titlebar_bg: Color32::from_rgb(0x23, 0x21, 0x21),
            titlebar_fg: Color32::from_rgb(0xe4, 0xe4, 0xe6),
            sidebar_bg: Color32::from_rgb(0x21, 0x21, 0x22),
            editor_bg: Color32::from_rgb(0x1e, 0x1e, 0x1e),
            fg: Color32::from_rgb(0xd4, 0xd4, 0xd6),
            fg_dim: Color32::from_rgb(0x8a, 0x8a, 0x8f),
            accent: Color32::from_rgb(0x56, 0x9c, 0xd6),
            line_number: Color32::from_rgb(0x5a, 0x5a, 0x5e),
            line_number_active: Color32::from_rgb(0xc0, 0xc0, 0xc4),
            selection: Color32::from_rgba_premultiplied(0x26, 0x4f, 0x78, 180),
            cursor: Color32::from_rgb(0xe4, 0xe4, 0xe6),
            button_hover: Color32::from_rgb(0x33, 0x33, 0x35),
            button_active: Color32::from_rgb(0x3d, 0x3d, 0x40),
            border: Color32::from_rgb(0x33, 0x33, 0x35),
            error: Color32::from_rgb(0xe0, 0x6c, 0x6c),
            close_hover: Color32::from_rgb(0xc4, 0x3b, 0x3b),
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
