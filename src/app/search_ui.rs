//! In-editor find: match computation and the floating find widget.

use super::editor::line_of;
use super::*;

impl EditApp {
    /// Ctrl+F / the Search activity-bar icon: a no-op on a tab the find
    /// widget can't actually do anything on — the `TextEdit` it looks for
    /// (by the active tab's id) doesn't exist for image/settings/large-file
    /// tabs, so opening it there would just show empty, do-nothing UI.
    pub(super) fn try_open_search(&mut self) {
        let Some(tab) = self.tabs.get(self.active) else { return };
        if tab.is_special() || tab.large {
            return;
        }
        self.search.open();
    }

    /// Byte ranges of every match of the current search query in the active
    /// tab, recomputed only when the tab, its content, the query, or the
    /// case-sensitivity toggle actually changed since the last call —
    /// shared between the editor's highlight overlay and the search popup
    /// so the buffer isn't rescanned (and, for case-insensitive search,
    /// copied) twice per frame.
    pub(super) fn search_matches(&mut self) -> Vec<(usize, usize)> {
        if !self.search.open || self.search.query.is_empty() {
            return Vec::new();
        }
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return Vec::new();
        };
        let key = (self.active, tab.version, self.search.query.clone(), self.search.match_case);
        if self.search_cache_key.as_ref() != Some(&key) {
            self.search_cache_ranges = self.search.find_all(&tab.content);
            self.search_cache_key = Some(key);
        }
        self.search_cache_ranges.clone()
    }

    pub(super) fn search_window(&mut self, ctx: &egui::Context) {
        if !self.search.open {
            return;
        }
        let theme = self.theme.clone();
        let mut still_open = true;

        let matches = self.search_matches();

        egui::Window::new(self.t("toolbar.search"))
            .open(&mut still_open)
            .collapsible(false)
            .resizable(true)
            .default_size([300.0, 110.0])
            // Anchored just below the title bar (34px) + tab bar (30px), the
            // way VS Code's own Find widget sits right under the tab strip
            // rather than the very top of the window. Was 78px when there
            // was still a 42px toolbar between them; recalibrated now that
            // it's gone.
            .anchor(Align2::RIGHT_TOP, egui::vec2(-16.0, 72.0))
            .frame(
                Frame::window(&ctx.style())
                    .fill(theme.panel_bg)
                    .stroke(Stroke::new(1.0_f32, theme.border)),
            )
            .show(ctx, |ui| {
                ui.set_min_width(280.0);
                let resp = ui.text_edit_singleline(&mut self.search.query);
                if self.search.focus_requested {
                    resp.request_focus();
                    self.search.focus_requested = false;
                }
                if resp.changed() {
                    self.search.current_match = 0;
                }
                let case_sensitive_label = self.t("search.case_sensitive");
                ui.checkbox(&mut self.search.match_case, case_sensitive_label);

                ui.horizontal(|ui| {
                    if matches.is_empty() {
                        ui.weak(if self.search.query.is_empty() { "" } else { self.t("search.no_matches") });
                    } else {
                        ui.label(format!("{} / {}", self.search.current_match + 1, matches.len()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(self.t("common.close")).clicked() {
                            self.search.close();
                        }
                        if ui.button(">").clicked() && !matches.is_empty() {
                            self.search.current_match = (self.search.current_match + 1) % matches.len();
                            let (s, _) = matches[self.search.current_match];
                            self.pending_scroll_line = Some(line_of(&self.tabs[self.active].content, s));
                        }
                        if ui.button("<").clicked() && !matches.is_empty() {
                            self.search.current_match = (self.search.current_match + matches.len() - 1) % matches.len();
                            let (s, _) = matches[self.search.current_match];
                            self.pending_scroll_line = Some(line_of(&self.tabs[self.active].content, s));
                        }
                    });
                });
            });

        if !still_open {
            self.search.close();
        }
    }
}
