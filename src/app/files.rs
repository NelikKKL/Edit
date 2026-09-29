//! Opening, saving and closing files/tabs, drag & drop, and the unsaved-changes prompt.

use super::*;

impl EditApp {
    pub(super) fn new_tab(&mut self) {
        let tab = EditorTab::untitled(self.untitled_counter, self.lang);
        self.untitled_counter += 1;
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    pub(super) fn open_path(&mut self, path: PathBuf) {
        // Already open? just switch to it.
        if let Some(idx) = self.tabs.iter().position(|t| t.path.as_deref() == Some(path.as_path())) {
            self.active = idx;
            return;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if is_image_extension(&ext) {
            // Images are previewed, not edited — no point reading them on a
            // background thread the way large text files are below; a
            // single image is never going to be big enough to matter.
            match std::fs::read(&path) {
                Ok(bytes) => self.insert_image_tab(path, bytes),
                Err(e) => {
                    self.status_message = Some(crate::i18n::open_file_error(self.lang, &e.to_string()));
                }
            }
            return;
        }

        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size < ASYNC_LOAD_BYTES {
            // Small/typical file: just read it inline, it'll be
            // effectively instant and this keeps the common case simple.
            match std::fs::read_to_string(&path) {
                Ok(content) => self.insert_loaded_tab(path, content),
                Err(e) => {
                    self.status_message = Some(crate::i18n::open_file_error(self.lang, &e.to_string()));
                }
            }
            return;
        }

        // Big file: reading it synchronously here would freeze the whole
        // window until the read (and UTF-8 validation) finishes. Show a
        // placeholder tab immediately and do the read on a background
        // thread instead.
        let tab = EditorTab::loading(path.clone(), self.lang);
        let tab_id = tab.id;
        if self.tabs.len() == 1 && self.tabs[0].path.is_none() && !self.tabs[0].dirty && self.tabs[0].content.is_empty() && !self.tabs[0].is_special() {
            self.tabs[0] = tab;
            self.active = 0;
        } else {
            self.tabs.push(tab);
            self.active = self.tabs.len() - 1;
        }

        let (tx, rx) = mpsc::channel();
        let read_path = path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(std::fs::read_to_string(&read_path));
        });
        self.pending_loads.push(PendingLoad { tab_id, path, rx });
    }

    /// Places freshly-read file content into a tab, reusing the current
    /// single empty "Untitled" tab if that's all there is.
    pub(super) fn insert_loaded_tab(&mut self, path: PathBuf, content: String) {
        if self.tabs.len() == 1 && self.tabs[0].path.is_none() && !self.tabs[0].dirty && self.tabs[0].content.is_empty() && !self.tabs[0].is_special() {
            self.tabs[0] = EditorTab::from_path(path, content, self.lang);
            self.active = 0;
        } else {
            self.tabs.push(EditorTab::from_path(path, content, self.lang));
            self.active = self.tabs.len() - 1;
        }
    }

    /// Same idea as `insert_loaded_tab`, for an image-preview tab.
    pub(super) fn insert_image_tab(&mut self, path: PathBuf, bytes: Vec<u8>) {
        let tab = EditorTab::from_image(path, bytes);
        if self.tabs.len() == 1 && self.tabs[0].path.is_none() && !self.tabs[0].dirty && self.tabs[0].content.is_empty() && !self.tabs[0].is_special() {
            self.tabs[0] = tab;
            self.active = 0;
        } else {
            self.tabs.push(tab);
            self.active = self.tabs.len() - 1;
        }
    }

    /// Checks in on any background file reads kicked off by `open_path`
    /// and, once one finishes, swaps its placeholder tab's content in
    /// place (matched by id, since the tab's index may have moved).
    pub(super) fn poll_pending_loads(&mut self) {
        if self.pending_loads.is_empty() {
            return;
        }
        let mut done = Vec::new();
        for (i, pending) in self.pending_loads.iter().enumerate() {
            match pending.rx.try_recv() {
                Ok(result) => done.push((i, result)),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    done.push((
                        i,
                        Err(std::io::Error::new(std::io::ErrorKind::Other, crate::i18n::could_not_read_file(self.lang))),
                    ));
                }
            }
        }
        // Remove from the back so earlier indices stay valid.
        for (i, result) in done.into_iter().rev() {
            let pending = self.pending_loads.remove(i);
            match result {
                Ok(content) => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == pending.tab_id) {
                        *tab = EditorTab::from_path(pending.path, content, self.lang);
                    }
                }
                Err(e) => {
                    self.status_message = Some(crate::i18n::open_file_error(self.lang, &e.to_string()));
                    if let Some(pos) = self.tabs.iter().position(|t| t.id == pending.tab_id) {
                        self.tabs.remove(pos);
                        if self.tabs.is_empty() {
                            self.new_tab();
                        } else if self.active >= self.tabs.len() {
                            self.active = self.tabs.len() - 1;
                        }
                    }
                }
            }
        }
    }

    pub(super) fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().set_title(self.t("dialog.open_file")).pick_file() {
            self.open_path(path);
        }
    }

    pub(super) fn open_folder_dialog(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().set_title(self.t("dialog.open_folder")).pick_folder() {
            self.file_tree.set_root(dir.clone());
            self.settings.show_sidebar = true;
            self.settings.last_folder = Some(dir.to_string_lossy().to_string());
            self.settings.save();
        }
    }

    pub(super) fn save_tab(&mut self, idx: usize) {
        let Some(tab) = self.tabs.get_mut(idx) else { return };
        if tab.is_special() {
            // Nothing to save — images are previewed, not edited.
            return;
        }
        if tab.path.is_some() {
            if let Err(e) = tab.save() {
                self.status_message = Some(crate::i18n::save_error(self.lang, &e.to_string()));
            } else {
                self.status_message = Some(self.t("status.saved").to_string());
            }
        } else {
            self.save_tab_as(idx);
        }
    }

    pub(super) fn save_tab_as(&mut self, idx: usize) {
        if self.tabs.get(idx).is_some_and(|t| t.is_special()) {
            // Nothing to save — images are previewed, not edited.
            return;
        }
        let lang = self.lang;
        if let Some(path) = rfd::FileDialog::new().set_title(self.t("dialog.save_as")).save_file() {
            if let Some(tab) = self.tabs.get_mut(idx) {
                if let Err(e) = tab.save_as(path, lang) {
                    self.status_message = Some(crate::i18n::save_error(self.lang, &e.to_string()));
                } else {
                    self.status_message = Some(self.t("status.saved").to_string());
                }
            }
        }
    }

    pub(super) fn request_close_tab(&mut self, idx: usize) {
        if self.tabs.get(idx).map(|t| t.dirty).unwrap_or(false) {
            self.close_confirm = Some(idx);
        } else {
            self.close_tab_now(idx);
        }
    }

    pub(super) fn close_tab_now(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            self.new_tab();
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
    }

    pub(super) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped {
            if path.is_dir() {
                self.file_tree.set_root(path);
                self.settings.show_sidebar = true;
            } else {
                self.open_path(path);
            }
        }
    }

    pub(super) fn close_confirm_modal(&mut self, ctx: &egui::Context) {
        let Some(idx) = self.close_confirm else { return };
        let theme = self.theme.clone();
        let title = self.tabs.get(idx).map(|t| t.title.clone()).unwrap_or_default();
        egui::Window::new(self.t("close_confirm.title"))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(Frame::window(&ctx.style()).fill(theme.panel_bg).stroke(Stroke::new(1.0_f32, theme.border)))
            .show(ctx, |ui| {
                ui.label(crate::i18n::save_changes_prompt(self.lang, &title));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(self.t("toolbar.save")).clicked() {
                        self.save_tab(idx);
                        self.close_tab_now(idx);
                        self.close_confirm = None;
                    }
                    if ui.button(self.t("close_confirm.dont_save")).clicked() {
                        self.close_tab_now(idx);
                        self.close_confirm = None;
                    }
                    if ui.button(self.t("common.cancel")).clicked() {
                        self.close_confirm = None;
                    }
                });
            });
    }
}
