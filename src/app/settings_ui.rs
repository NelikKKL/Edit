//! The Settings editor tab (VS Code style): setting definitions, controls and layout.

use super::*;

/// Left-hand table of contents in the Settings editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsCat {
    Common,
    Editor,
    Workbench,
}

impl SettingsCat {
    const ALL: [SettingsCat; 3] = [SettingsCat::Common, SettingsCat::Editor, SettingsCat::Workbench];

    fn label(self, lang: crate::i18n::Lang) -> &'static str {
        use crate::i18n::t;
        match self {
            SettingsCat::Common => t(lang, "set.toc.common"),
            SettingsCat::Editor => t(lang, "set.toc.editor"),
            SettingsCat::Workbench => t(lang, "set.toc.workbench"),
        }
    }

    /// The "Editor: " / "Workbench: " prefix VS Code puts before a setting's name.
    fn prefix_key(self) -> &'static str {
        match self {
            SettingsCat::Editor => "set.pre.editor",
            _ => "set.pre.workbench",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingId {
    ColorTheme,
    CustomTheme,
    SideBar,
    Language,
    FontFamily,
    FontSize,
    LineNumbers,
    WordWrap,
    TabSize,
    AutoClose,
    Syntax,
}

#[derive(Clone, Copy)]
pub(super) struct SettingDef {
    id: SettingId,
    /// Which real section this belongs to (never `SettingsCat::Common`).
    section: SettingsCat,
    /// Also listed under "Commonly Used".
    common: bool,
    name: &'static str,
    desc: &'static str,
}

const fn def(id: SettingId, section: SettingsCat, common: bool, name: &'static str, desc: &'static str) -> SettingDef {
    SettingDef { id, section, common, name, desc }
}

const SETTING_DEFS: [SettingDef; 11] = [
    def(SettingId::FontFamily, SettingsCat::Editor, true, "set.fontFamily.name", "set.fontFamily.desc"),
    def(SettingId::FontSize, SettingsCat::Editor, true, "set.fontSize.name", "set.fontSize.desc"),
    def(SettingId::LineNumbers, SettingsCat::Editor, false, "set.lineNumbers.name", "set.lineNumbers.desc"),
    def(SettingId::WordWrap, SettingsCat::Editor, true, "set.wordWrap.name", "set.wordWrap.desc"),
    def(SettingId::TabSize, SettingsCat::Editor, true, "set.tabSize.name", "set.tabSize.desc"),
    def(SettingId::AutoClose, SettingsCat::Editor, false, "set.autoClose.name", "set.autoClose.desc"),
    def(SettingId::Syntax, SettingsCat::Editor, false, "set.syntax.name", "set.syntax.desc"),
    def(SettingId::ColorTheme, SettingsCat::Workbench, true, "set.colorTheme.name", "set.colorTheme.desc"),
    def(SettingId::CustomTheme, SettingsCat::Workbench, false, "set.customTheme.name", "set.customTheme.desc"),
    def(SettingId::SideBar, SettingsCat::Workbench, false, "set.sideBar.name", "set.sideBar.desc"),
    def(SettingId::Language, SettingsCat::Workbench, false, "set.language.name", "set.language.desc"),
];

impl EditApp {
    /// Opens (or switches to) the Settings editor tab — VS Code shows its
    /// settings as a regular editor tab rather than a dialog.
    pub(super) fn open_settings_tab(&mut self) {
        if let Some(i) = self.tabs.iter().position(|t| t.is_settings) {
            self.active = i;
            return;
        }
        let title = self.t("toolbar.settings").to_string();
        self.tabs.push(EditorTab::settings(title));
        self.active = self.tabs.len() - 1;
    }

    pub(super) fn setting_modified(&self, id: SettingId) -> bool {
        let d = Settings::default();
        let s = &self.settings;
        match id {
            SettingId::ColorTheme => s.theme != d.theme,
            SettingId::CustomTheme => s.custom_css_path.is_some(),
            SettingId::SideBar => s.show_sidebar != d.show_sidebar,
            SettingId::Language => s.lang.is_some(),
            SettingId::FontFamily => s.font_family.is_some(),
            SettingId::FontSize => s.font_size != d.font_size,
            SettingId::LineNumbers => s.show_line_numbers != d.show_line_numbers,
            SettingId::WordWrap => s.word_wrap != d.word_wrap,
            SettingId::TabSize => s.tab_width != d.tab_width,
            SettingId::AutoClose => s.auto_close_brackets != d.auto_close_brackets,
            SettingId::Syntax => s.syntax_highlighting != d.syntax_highlighting,
        }
    }

    /// Restores one setting to its default. Returns whether the theme/fonts
    /// need re-applying afterwards.
    pub(super) fn reset_setting(&mut self, id: SettingId) -> bool {
        let d = Settings::default();
        match id {
            SettingId::ColorTheme => {
                self.settings.theme = d.theme;
                true
            }
            SettingId::CustomTheme => {
                self.settings.custom_css_path = None;
                true
            }
            SettingId::SideBar => {
                self.settings.show_sidebar = d.show_sidebar;
                false
            }
            SettingId::Language => {
                self.settings.lang = None;
                self.lang = crate::i18n::Lang::detect_system();
                false
            }
            SettingId::FontFamily => {
                self.settings.font_family = None;
                true
            }
            SettingId::FontSize => {
                self.settings.font_size = d.font_size;
                false
            }
            SettingId::LineNumbers => {
                self.settings.show_line_numbers = d.show_line_numbers;
                false
            }
            SettingId::WordWrap => {
                self.settings.word_wrap = d.word_wrap;
                false
            }
            SettingId::TabSize => {
                self.settings.tab_width = d.tab_width;
                false
            }
            SettingId::AutoClose => {
                self.settings.auto_close_brackets = d.auto_close_brackets;
                false
            }
            SettingId::Syntax => {
                self.settings.syntax_highlighting = d.syntax_highlighting;
                false
            }
        }
    }

    /// The control for one setting, laid out the way VS Code's Settings
    /// editor does it: checkboxes carry the description as their label,
    /// everything else shows the description above the input.
    pub(super) fn setting_control(&mut self, ui: &mut egui::Ui, id: SettingId, desc: &'static str, theme_changed: &mut bool, changed: &mut bool) {
        match id {
            SettingId::LineNumbers => {
                if ui.checkbox(&mut self.settings.show_line_numbers, desc).changed() {
                    *changed = true;
                }
                return;
            }
            SettingId::WordWrap => {
                if ui.checkbox(&mut self.settings.word_wrap, desc).changed() {
                    *changed = true;
                }
                return;
            }
            SettingId::AutoClose => {
                if ui.checkbox(&mut self.settings.auto_close_brackets, desc).changed() {
                    *changed = true;
                }
                return;
            }
            SettingId::Syntax => {
                if ui.checkbox(&mut self.settings.syntax_highlighting, desc).changed() {
                    *changed = true;
                }
                return;
            }
            SettingId::SideBar => {
                if ui.checkbox(&mut self.settings.show_sidebar, desc).changed() {
                    *changed = true;
                }
                return;
            }
            _ => {}
        }

        ui.label(RichText::new(desc).color(self.theme.fg_dim));
        ui.add_space(6.0);

        match id {
            SettingId::ColorTheme => {
                let current = self.settings.theme.label(self.lang);
                egui::ComboBox::from_id_source("set_color_theme")
                    .selected_text(current)
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for kind in ThemeKind::ALL {
                            if ui.selectable_value(&mut self.settings.theme, kind, kind.label(self.lang)).clicked() {
                                *theme_changed = true;
                            }
                        }
                    });
            }
            SettingId::FontFamily => {
                let builtin = self.t("settings.builtin_mono");
                let current = self.settings.font_family.clone().unwrap_or_else(|| builtin.to_string());
                let (fonts_ready, font_names) = self
                    .system_fonts
                    .lock()
                    .map(|s| (s.is_ready(), s.names()))
                    .unwrap_or((false, Vec::new()));
                egui::ComboBox::from_id_source("set_font_family")
                    .selected_text(current)
                    .width(280.0)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(self.settings.font_family.is_none(), builtin).clicked() {
                            self.settings.font_family = None;
                            *theme_changed = true;
                        }
                        for name in font_names {
                            let selected = self.settings.font_family.as_deref() == Some(name.as_str());
                            if ui.selectable_label(selected, &name).clicked() {
                                self.settings.font_family = Some(name);
                                *theme_changed = true;
                            }
                        }
                    });
                if !fonts_ready {
                    ui.add_space(4.0);
                    ui.weak(self.t("settings.scanning_fonts"));
                }
            }
            SettingId::FontSize => {
                if ui
                    .add(egui::DragValue::new(&mut self.settings.font_size).clamp_range(8.0..=36.0).speed(0.1))
                    .changed()
                {
                    *changed = true;
                }
            }
            SettingId::TabSize => {
                if ui
                    .add(egui::DragValue::new(&mut self.settings.tab_width).clamp_range(1..=8))
                    .changed()
                {
                    *changed = true;
                }
            }
            SettingId::Language => {
                let system_label = self.t("settings.language_system");
                let system_lang = crate::i18n::Lang::detect_system();
                let current = match self.settings.lang {
                    None => system_label,
                    Some(l) => l.label(),
                };
                egui::ComboBox::from_id_source("set_language")
                    .selected_text(current)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(self.settings.lang.is_none(), system_label).clicked() {
                            self.settings.lang = None;
                            self.lang = system_lang;
                            *changed = true;
                        }
                        for l in [crate::i18n::Lang::Ru, crate::i18n::Lang::En] {
                            if ui.selectable_label(self.settings.lang == Some(l), l.label()).clicked() {
                                self.settings.lang = Some(l);
                                self.lang = l;
                                *changed = true;
                            }
                        }
                    });
            }
            SettingId::CustomTheme => {
                let browse = self.t("settings.browse");
                let create_example = self.t("settings.create_example_css");
                let apply = self.t("settings.apply_as_theme");
                let reread = self.t("settings.reread_css");
                let mut path_str = self.settings.custom_css_path.clone().unwrap_or_default();
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::TextEdit::singleline(&mut path_str).desired_width(320.0))
                        .changed()
                    {
                        self.settings.custom_css_path = if path_str.is_empty() { None } else { Some(path_str.clone()) };
                        *changed = true;
                    }
                    if ui.button(browse).clicked() {
                        if let Some(p) = rfd::FileDialog::new().add_filter("CSS", &["css"]).pick_file() {
                            self.settings.custom_css_path = Some(p.to_string_lossy().to_string());
                            *changed = true;
                        }
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button(create_example).clicked() {
                        if let Some(default_path) = Settings::default_css_path() {
                            if let Some(dir) = default_path.parent() {
                                let _ = std::fs::create_dir_all(dir);
                            }
                            if std::fs::write(&default_path, custom_css::EXAMPLE_CSS).is_ok() {
                                self.settings.custom_css_path = Some(default_path.to_string_lossy().to_string());
                                *changed = true;
                            }
                        }
                    }
                    if ui.button(apply).clicked() {
                        self.settings.theme = ThemeKind::Custom;
                        *theme_changed = true;
                    }
                    if ui.button(reread).clicked() {
                        *theme_changed = true;
                    }
                });
                if let Some(err) = &self.css_status {
                    ui.colored_label(self.theme.error, err);
                }
            }
            _ => {}
        }
    }

    /// The Settings editor tab, modeled on VS Code's: a search box on top,
    /// a "User" scope tab, a table of contents on the left and the matching
    /// settings on the right (each with a blue bar and a reset gear when
    /// changed from its default).
    pub(super) fn settings_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, idx: usize) {
        let theme = self.theme.clone();
        self.tabs[idx].title = self.t("toolbar.settings").to_string();
        let mut theme_changed = false;
        let mut changed = false;

        egui::Frame::none()
            .inner_margin(Margin { left: 28.0, right: 28.0, top: 14.0, bottom: 0.0 })
            .show(ui, |ui| {
                // ---- search box ----
                let hint = self.t("set.search_hint");
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_search)
                        .hint_text(hint)
                        .desired_width(f32::INFINITY),
                );

                let query = self.settings_search.trim().to_lowercase();
                let searching = !query.is_empty();
                let visible: Vec<SettingDef> = SETTING_DEFS
                    .iter()
                    .copied()
                    .filter(|d| {
                        if searching {
                            let hay = format!(
                                "{} {} {}",
                                self.t(d.section.prefix_key()),
                                self.t(d.name),
                                self.t(d.desc)
                            )
                            .to_lowercase();
                            hay.contains(&query)
                        } else if self.settings_cat == SettingsCat::Common {
                            d.common
                        } else {
                            d.section == self.settings_cat
                        }
                    })
                    .collect();

                // ---- "User" scope tab (+ result count while searching) ----
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let user = ui.label(RichText::new(self.t("set.user")).color(theme.fg));
                    let underline = egui::Rect::from_min_max(
                        egui::pos2(user.rect.left(), user.rect.bottom() + 2.0),
                        egui::pos2(user.rect.right(), user.rect.bottom() + 3.5),
                    );
                    ui.painter().rect_filled(underline, 0.0, theme.accent);
                    if searching {
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(crate::i18n::settings_found(self.lang, visible.len()))
                                    .color(theme.fg_dim)
                                    .small(),
                            );
                        });
                    }
                });
                ui.add_space(6.0);
                ui.separator();

                // ---- body: table of contents | settings list ----
                let avail = ui.available_size();
                let toc_w = 200.0;
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    ui.allocate_ui_with_layout(egui::vec2(toc_w, avail.y), egui::Layout::top_down(Align::Min), |ui| {
                        ui.add_space(8.0);
                        for cat in SettingsCat::ALL {
                            let selected = !searching && self.settings_cat == cat;
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(toc_w - 12.0, 26.0), Sense::click());
                            if selected {
                                ui.painter().rect_filled(rect, 3.0, theme.accent.gamma_multiply(0.35));
                            } else if resp.hovered() {
                                ui.painter().rect_filled(rect, 3.0, theme.button_hover);
                            }
                            ui.painter().text(
                                egui::pos2(rect.left() + 12.0, rect.center().y),
                                Align2::LEFT_CENTER,
                                cat.label(self.lang),
                                egui::FontId::proportional(13.0),
                                if selected { theme.fg } else { theme.fg_dim },
                            );
                            if resp.clicked() {
                                self.settings_cat = cat;
                                self.settings_search.clear();
                            }
                        }
                    });

                    ui.allocate_ui_with_layout(
                        egui::vec2((avail.x - toc_w).max(120.0), avail.y),
                        egui::Layout::top_down(Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                                ui.add_space(10.0);
                                ui.set_max_width(780.0);
                                if !searching {
                                    ui.label(
                                        RichText::new(self.settings_cat.label(self.lang))
                                            .size(20.0)
                                            .strong()
                                            .color(theme.fg),
                                    );
                                    ui.add_space(8.0);
                                }
                                for d in visible.iter().copied() {
                                    let name = self.t(d.name);
                                    let desc = self.t(d.desc);
                                    let prefix = self.t(d.section.prefix_key());
                                    let reset_tip = self.t("set.reset");
                                    let modified = self.setting_modified(d.id);
                                    let mut do_reset = false;

                                    let row = egui::Frame::none()
                                        .inner_margin(Margin { left: 14.0, right: 0.0, top: 8.0, bottom: 16.0 })
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 0.0;
                                                ui.label(RichText::new(format!("{prefix}: ")).color(theme.fg_dim));
                                                ui.label(RichText::new(name).strong().color(theme.fg));
                                                if modified {
                                                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                                        let gear = ui.add(
                                                            egui::Button::new(crate::codicons::rich(crate::codicons::SETTINGS_GEAR, 14.0))
                                                                .frame(false),
                                                        );
                                                        if gear.on_hover_text(reset_tip).clicked() {
                                                            do_reset = true;
                                                        }
                                                    });
                                                }
                                            });
                                            ui.add_space(2.0);
                                            self.setting_control(ui, d.id, desc, &mut theme_changed, &mut changed);
                                        });

                                    // VS Code's "modified" marker: a blue bar down the left edge.
                                    if modified {
                                        let r = row.response.rect;
                                        let bar = egui::Rect::from_min_max(
                                            egui::pos2(r.left() + 2.0, r.top() + 8.0),
                                            egui::pos2(r.left() + 4.0, r.bottom() - 14.0),
                                        );
                                        ui.painter().rect_filled(bar, 0.0, theme.accent);
                                    }
                                    if do_reset {
                                        if self.reset_setting(d.id) {
                                            theme_changed = true;
                                        }
                                        changed = true;
                                    }
                                }
                            });
                        },
                    );
                });
            });

        if theme_changed {
            self.apply_theme(ctx);
            self.settings.save();
        } else if changed {
            self.settings.save();
        }
    }
}
