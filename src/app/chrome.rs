//! Window chrome: title bar + menus, tab strip, activity bar, sidebar and status bar.

use super::*;

impl EditApp {
    /// VS Code puts file actions (New/Open/Save/...) in the "File" menu
    /// rather than as toolbar buttons; this app has a custom, undecorated
    /// title bar instead of a native menu bar, so the menu lives there.
    pub(super) fn file_menu(&mut self, ui: &mut egui::Ui) {
        let label = RichText::new(self.t("menu.file")).color(self.theme.titlebar_fg);
        ui.menu_button(label, |ui| {
            if ui.button(self.t("menu.new_tab")).clicked() {
                self.new_tab();
                ui.close_menu();
            }
            if ui.button(self.t("toolbar.open")).clicked() {
                self.open_file_dialog();
                ui.close_menu();
            }
            if ui.button(self.t("toolbar.folder")).clicked() {
                self.open_folder_dialog();
                ui.close_menu();
            }
            ui.separator();
            if ui.button(self.t("toolbar.save")).clicked() {
                self.save_tab(self.active);
                ui.close_menu();
            }
            if ui.button(self.t("dialog.save_as")).clicked() {
                self.save_tab_as(self.active);
                ui.close_menu();
            }
            ui.separator();
            if ui.button(self.t("menu.exit")).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                ui.close_menu();
            }
        });
    }

    pub(super) fn terminal_menu(&mut self, ui: &mut egui::Ui) {
        let label = RichText::new(self.t("menu.terminal")).color(self.theme.titlebar_fg);
        ui.menu_button(label, |ui| {
            if ui.button(self.t("terminal.new")).clicked() {
                self.new_terminal();
                ui.close_menu();
            }
            if ui.button(self.t("terminal.toggle")).clicked() {
                self.toggle_terminal();
                ui.close_menu();
            }
        });
    }

    pub(super) fn title_bar(&mut self, ctx: &egui::Context) {
        let height = 34.0;
        egui::TopBottomPanel::top("title_bar")
            .exact_height(height)
            .frame(Frame::none().fill(self.theme.titlebar_bg))
            .show(ctx, |ui| {
                let full_rect = ui.max_rect();
                let drag_rect = egui::Rect::from_min_max(
                    full_rect.min + egui::vec2(RESIZE_BORDER, RESIZE_BORDER),
                    full_rect.max - egui::vec2(RESIZE_BORDER, 0.0),
                );
                let response = ui.interact(drag_rect, egui::Id::new("title_bar_drag"), Sense::click_and_drag());
                if response.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if response.double_clicked() {
                    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }

                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new("edit").color(self.theme.titlebar_fg).strong());
                    ui.add_space(6.0);
                    self.file_menu(ui);
                    self.terminal_menu(ui);
                    if let Some(tab) = self.tabs.get(self.active) {
                        let dirty = if tab.dirty { " *" } else { "" };
                        ui.label(RichText::new(format!("— {}{}", tab.title, dirty)).color(self.theme.fg_dim));
                    }

                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if window_button(ui, crate::codicons::CHROME_CLOSE, self.theme.close_hover, self.theme.titlebar_fg).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                        let sym = if maximized { crate::codicons::CHROME_RESTORE } else { crate::codicons::CHROME_MAXIMIZE };
                        if window_button(ui, sym, self.theme.button_hover, self.theme.titlebar_fg).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if window_button(ui, crate::codicons::CHROME_MINIMIZE, self.theme.button_hover, self.theme.titlebar_fg).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });
    }

    pub(super) fn tab_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("tabs")
            .exact_height(30.0)
            .frame(Frame::none().fill(self.theme.bg))
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut to_activate = None;
                        let mut to_close = None;
                        for (i, tab) in self.tabs.iter().enumerate() {
                            let selected = i == self.active;
                            let bg = if selected { self.theme.panel_bg } else { self.theme.bg };
                            egui::Frame::none()
                                .fill(bg)
                                .inner_margin(Margin::symmetric(8.0, 4.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let text = RichText::new(tab.title.as_str())
                                            .color(if selected { self.theme.fg } else { self.theme.fg_dim });
                                        if ui.selectable_label(false, text).clicked() {
                                            to_activate = Some(i);
                                        }
                                        // Like VS Code: a filled dot marks unsaved changes,
                                        // a close cross otherwise; both close the tab.
                                        let glyph = if tab.dirty { crate::codicons::CIRCLE_FILLED } else { crate::codicons::CLOSE };
                                        let close_btn = ui.add(egui::Button::new(crate::codicons::rich(glyph, 12.0)).frame(false));
                                        if close_btn.clicked() {
                                            to_close = Some(i);
                                        }
                                    });
                                });
                        }
                        let plus = ui.add(egui::Button::new(crate::codicons::rich(crate::codicons::ADD, 14.0)).frame(false));
                        if plus.on_hover_text(self.t("toolbar.new_tab_tooltip")).clicked() {
                            self.new_tab();
                        }
                        if let Some(i) = to_activate {
                            self.active = i;
                        }
                        if let Some(i) = to_close {
                            self.request_close_tab(i);
                        }
                    });
                });
            });
    }

    /// VS Code's Activity Bar: the narrow icon strip to the left of the
    /// Sidebar. Only icons for things this app actually does — Explorer
    /// (toggles the file tree sidebar) and Search (opens the in-editor
    /// find widget, same as Ctrl+F) up top, Settings pinned to the bottom.
    pub(super) fn activity_bar(&mut self, ctx: &egui::Context) {
        let theme = self.theme.clone();
        egui::SidePanel::left("activity_bar")
            .exact_width(46.0)
            .resizable(false)
            .frame(Frame::none().fill(theme.sidebar_bg))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    let explorer_tip = self.t("activity.explorer");
                    if activity_icon(ui, &theme, crate::codicons::FILES, self.settings.show_sidebar, explorer_tip).clicked() {
                        self.settings.show_sidebar = !self.settings.show_sidebar;
                        self.settings.save();
                    }
                    let search_tip = self.t("toolbar.search");
                    if activity_icon(ui, &theme, crate::codicons::SEARCH, false, search_tip).clicked() {
                        self.try_open_search();
                    }

                    // Pin the settings icon to the bottom of the strip.
                    let reserved_for_bottom_icon = 46.0;
                    let gap = ui.available_height() - reserved_for_bottom_icon;
                    if gap > 0.0 {
                        ui.add_space(gap);
                    }
                    let settings_tip = self.t("toolbar.settings");
                    let settings_active = self.tabs.get(self.active).is_some_and(|t| t.is_settings);
                    if activity_icon(ui, &theme, crate::codicons::SETTINGS_GEAR, settings_active, settings_tip).clicked() {
                        self.open_settings_tab();
                    }
                });
            });
    }

    pub(super) fn sidebar(&mut self, ctx: &egui::Context) {
        if !self.settings.show_sidebar {
            return;
        }
        let theme = self.theme.clone();
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(230.0)
            .width_range(150.0..=500.0)
            .frame(
                Frame::none()
                    .fill(theme.sidebar_bg)
                    .inner_margin(Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.t("activity.explorer").to_uppercase()).color(theme.fg_dim).small());
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let folder_btn = ui.add(egui::Button::new(crate::codicons::rich(crate::codicons::FOLDER_OPENED, 15.0)).frame(false));
                        if folder_btn.on_hover_text(self.t("toolbar.folder")).clicked() {
                            self.open_folder_dialog();
                        }
                        let file_btn = ui.add(egui::Button::new(crate::codicons::rich(crate::codicons::FILE, 15.0)).frame(false));
                        if file_btn.on_hover_text(self.t("toolbar.open")).clicked() {
                            self.open_file_dialog();
                        }
                    });
                });
                ui.add_space(4.0);

                if self.file_tree.root.is_none() {
                    ui.add_space(16.0);
                    ui.label(RichText::new(self.t("file_tree.empty_hint")).color(theme.fg_dim).small());
                    ui.add_space(8.0);
                    if ui.button(self.t("toolbar.folder")).clicked() {
                        self.open_folder_dialog();
                    }
                    return;
                }

                match self.file_tree.ui(ui, &theme, self.lang) {
                    TreeAction::OpenFile(path) => self.open_path(path),
                    TreeAction::None => {}
                }
            });
    }

    pub(super) fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .frame(
                Frame::none()
                    .fill(self.theme.sidebar_bg)
                    .inner_margin(Margin::symmetric(10.0, 3.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(crate::i18n::line_col(self.lang, self.status_line_col.0, self.status_line_col.1))
                            .color(self.theme.fg_dim)
                            .small(),
                    );
                    ui.separator();
                    if let Some(tab) = self.tabs.get(self.active) {
                        let ext = tab.extension();
                        ui.label(RichText::new(ext.to_uppercase()).color(self.theme.fg_dim).small());
                        ui.separator();
                        ui.label(RichText::new("UTF-8").color(self.theme.fg_dim).small());
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if let Some(msg) = &self.status_message {
                            ui.label(RichText::new(msg).color(self.theme.accent).small());
                        }
                        ui.label(RichText::new(self.settings.theme.label(self.lang)).color(self.theme.fg_dim).small());
                    });
                });
            });
    }
}

pub(super) fn window_button(ui: &mut egui::Ui, symbol: &str, hover_bg: Color32, fg: Color32) -> egui::Response {
    let size = egui::vec2(44.0, 34.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, hover_bg);
    }
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, symbol, crate::codicons::font_id(15.0), fg);
    response
}

/// One icon in the Activity Bar: a plain glyph, brighter + a left accent
/// bar when its panel is the active one (VS Code's own convention for
/// showing which sidebar view is open).
pub(super) fn activity_icon(ui: &mut egui::Ui, theme: &Theme, symbol: &str, active: bool, tooltip: &str) -> egui::Response {
    let size = egui::vec2(46.0, 40.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if active {
        let bar = egui::Rect::from_min_size(rect.min, egui::vec2(2.0, rect.height()));
        ui.painter().rect_filled(bar, 0.0, theme.accent);
    }
    let color = if active || response.hovered() { theme.fg } else { theme.fg_dim };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, symbol, crate::codicons::font_id(22.0), color);
    response.on_hover_text(tooltip)
}
