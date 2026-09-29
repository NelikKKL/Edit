//! The integrated terminal panel (terminal sessions live in `crate::terminal`).

use super::*;

impl EditApp {
    pub(super) fn new_terminal(&mut self) {
        let cwd = self
            .file_tree
            .root
            .clone()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        self.terminal_serial += 1;
        let name = format!("{} {}", Terminal::shell_name(), self.terminal_serial);
        self.terminals.push(Terminal::new(name, cwd, self.terminal_serial));
        self.active_terminal = self.terminals.len() - 1;
        self.show_terminal = true;
        self.terminal_focus_pending = true;
    }

    pub(super) fn toggle_terminal(&mut self) {
        if self.terminals.is_empty() {
            self.new_terminal();
            return;
        }
        self.show_terminal = !self.show_terminal;
        if self.show_terminal {
            self.terminal_focus_pending = true;
        }
    }

    /// VS Code's integrated terminal panel: a row of terminal tabs (each a
    /// separate shell session, see `terminal.rs`) plus New/Kill/Close, and
    /// the active session's scrollback + input below it.
    pub(super) fn terminal_panel(&mut self, ctx: &egui::Context) {
        if !self.show_terminal || self.terminals.is_empty() {
            return;
        }
        if self.active_terminal >= self.terminals.len() {
            self.active_terminal = self.terminals.len() - 1;
        }
        let theme = self.theme.clone();
        let focus = std::mem::take(&mut self.terminal_focus_pending);

        egui::TopBottomPanel::bottom("terminal_panel")
            .resizable(true)
            .default_height(220.0)
            .height_range(120.0..=600.0)
            .frame(Frame::none().fill(theme.panel_bg).stroke(Stroke::new(1.0_f32, theme.border)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    ui.label(RichText::new(self.t("terminal.title")).color(theme.fg_dim).small());
                    ui.add_space(10.0);

                    let mut to_activate = None;
                    let mut to_close = None;
                    for (i, term) in self.terminals.iter().enumerate() {
                        let selected = i == self.active_terminal;
                        let fg = if selected { theme.fg } else { theme.fg_dim };
                        if ui.selectable_label(selected, RichText::new(term.name.as_str()).color(fg).small()).clicked() {
                            to_activate = Some(i);
                        }
                        let close = ui.add(egui::Button::new(crate::codicons::rich(crate::codicons::CLOSE, 11.0)).frame(false));
                        if close.clicked() {
                            to_close = Some(i);
                        }
                        ui.add_space(4.0);
                    }
                    if let Some(i) = to_activate {
                        self.active_terminal = i;
                        self.terminal_focus_pending = true;
                    }
                    if let Some(i) = to_close {
                        self.terminals.remove(i);
                        if self.terminals.is_empty() {
                            self.show_terminal = false;
                        } else if self.active_terminal >= i {
                            self.active_terminal = self.active_terminal.saturating_sub(1);
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let close_panel_tip = self.t("terminal.close_panel");
                        if ui
                            .add(egui::Button::new(crate::codicons::rich(crate::codicons::CLOSE, 14.0)).frame(false))
                            .on_hover_text(close_panel_tip)
                            .clicked()
                        {
                            self.show_terminal = false;
                        }
                        ui.add_space(4.0);
                        let kill_tip = self.t("terminal.kill");
                        if ui
                            .add(egui::Button::new(crate::codicons::rich(crate::codicons::TRASH, 14.0)).frame(false))
                            .on_hover_text(kill_tip)
                            .clicked()
                        {
                            if let Some(t) = self.terminals.get_mut(self.active_terminal) {
                                t.kill();
                            }
                        }
                        ui.add_space(4.0);
                        let new_tip = self.t("terminal.new");
                        if ui
                            .add(egui::Button::new(crate::codicons::rich(crate::codicons::ADD, 14.0)).frame(false))
                            .on_hover_text(new_tip)
                            .clicked()
                        {
                            self.new_terminal();
                        }
                    });
                });
                ui.add_space(2.0);
                ui.separator();

                let font_size = self.settings.font_size.min(16.0);
                if let Some(term) = self.terminals.get_mut(self.active_terminal) {
                    term.ui(ui, &theme, font_size, focus);
                }
            });
    }
}
