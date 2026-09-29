//! Applying the color theme and installing fonts (text font + the embedded icon fonts).

use super::*;

impl EditApp {
    pub(super) fn apply_theme(&mut self, ctx: &egui::Context) {
        self.theme = match self.settings.theme {
            ThemeKind::Light => Theme::light(),
            ThemeKind::Dark => Theme::dark(),
            ThemeKind::Char => Theme::char_theme(),
            ThemeKind::Custom => self.load_custom_theme(),
        };
        self.theme.apply_to_ctx(ctx);
        self.install_font(ctx);
    }

    pub(super) fn load_custom_theme(&mut self) -> Theme {
        let Some(path) = self.settings.custom_css_path.clone() else {
            self.css_status = Some(crate::i18n::theme_file_not_selected(self.lang).to_string());
            return Theme::dark();
        };
        match custom_css::parse_css_file(Path::new(&path)) {
            Ok(parsed) => {
                let (font_family, font_size) = custom_css::font_overrides_from_css(&parsed);
                if let Some(ff) = font_family {
                    self.settings.font_family = Some(ff);
                }
                if let Some(fs) = font_size {
                    self.settings.font_size = fs;
                }
                self.css_status = None;
                custom_css::theme_from_css(&parsed)
            }
            Err(e) => {
                self.css_status = Some(crate::i18n::css_read_failed(self.lang, &path, &e.to_string()));
                Theme::dark()
            }
        }
    }

    pub(super) fn install_font(&mut self, ctx: &egui::Context) {
        let Some(family) = self.settings.font_family.clone() else {
            ctx.set_fonts(with_icon_fonts(egui::FontDefinitions::default()));
            self.base_fonts_installed = true;
            self.font_install_pending = false;
            return;
        };

        // The system font list is still being scanned in the background;
        // keep the built-in font for now and try again next frame (see
        // `font_install_pending` polling in `update`) instead of blocking
        // here until the scan finishes.
        let Ok(state) = self.system_fonts.lock() else { return };
        if !state.is_ready() {
            // Don't leave egui without the icon fonts while waiting: install
            // the built-in text font + icons now, the user's font later.
            if !self.base_fonts_installed {
                ctx.set_fonts(with_icon_fonts(egui::FontDefinitions::default()));
                self.base_fonts_installed = true;
            }
            self.font_install_pending = true;
            return;
        }

        let mut defs = egui::FontDefinitions::default();
        if let Some(bytes) = state.load_bytes(&family) {
            defs.font_data
                .insert("user_font".to_owned(), egui::FontData::from_owned(bytes));
            defs.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "user_font".to_owned());
            defs.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "user_font".to_owned());
        } else {
            self.status_message = Some(crate::i18n::font_not_found(self.lang, &family));
        }
        drop(state);
        ctx.set_fonts(with_icon_fonts(defs));
        self.base_fonts_installed = true;
        self.font_install_pending = false;
    }
}

/// Registers the embedded icon fonts — Seti (file-type icons, see
/// `file_icons.rs`) and Codicons (UI icons, see `codicons.rs`) — each under
/// its own dedicated font family, without touching the regular text
/// fallback chains: they're only ever requested explicitly. Wraps whichever
/// `FontDefinitions` `install_font` is about to hand to `ctx.set_fonts`, so
/// the icon fonts survive switching the editor's own font.
pub(super) fn with_icon_fonts(mut defs: egui::FontDefinitions) -> egui::FontDefinitions {
    defs.font_data.insert(
        crate::file_icons::FONT_NAME.to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/seti-icons.ttf")),
    );
    defs.families
        .insert(crate::file_icons::font_family(), vec![crate::file_icons::FONT_NAME.to_owned()]);
    defs.font_data.insert(
        crate::codicons::FONT_NAME.to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/codicon.ttf")),
    );
    defs.families
        .insert(crate::codicons::font_family(), vec![crate::codicons::FONT_NAME.to_owned()]);
    defs
}
