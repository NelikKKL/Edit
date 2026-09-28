use crate::codicons;
use crate::file_icons;
use crate::i18n::{t, Lang};
use crate::theme::Theme;
use std::path::{Path, PathBuf};

/// Action requested by the user while interacting with the tree.
pub enum TreeAction {
    OpenFile(PathBuf),
    None,
}

pub struct FileTree {
    pub root: Option<PathBuf>,
}

impl FileTree {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn set_root(&mut self, path: PathBuf) {
        self.root = Some(path);
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, theme: &Theme, lang: Lang) -> TreeAction {
        let Some(root) = self.root.clone() else {
            ui.weak(t(lang, "file_tree.no_folder"));
            return TreeAction::None;
        };

        let mut action = TreeAction::None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(a) = Self::dir_ui(ui, &root, theme, true) {
                    action = a;
                }
            });
        action
    }

    fn dir_ui(ui: &mut egui::Ui, dir: &Path, theme: &Theme, root: bool) -> Option<TreeAction> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .ok()?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| {
            let is_dir = e.path().is_dir();
            (!is_dir, e.file_name().to_string_lossy().to_lowercase())
        });

        let mut result = None;

        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.to_string_lossy().to_string());

        // Icon and name use different fonts (Codicons vs. the UI font), so the
        // header title is a two-section LayoutJob rather than a plain string.
        let mut title = egui::text::LayoutJob::default();
        title.append(
            codicons::FOLDER,
            0.0,
            egui::text::TextFormat {
                font_id: codicons::font_id(15.0),
                color: theme.fg,
                ..Default::default()
            },
        );
        title.append(
            &name,
            6.0,
            egui::text::TextFormat {
                font_id: egui::TextStyle::Body.resolve(ui.style()),
                color: theme.fg,
                ..Default::default()
            },
        );

        let header = egui::CollapsingHeader::new(title)
            .default_open(root)
            .id_source(dir.to_string_lossy().to_string());

        header.show(ui, |ui| {
            for entry in entries {
                let path = entry.path();
                if is_hidden(&path) {
                    continue;
                }
                if path.is_dir() {
                    if let Some(a) = Self::dir_ui(ui, &path, theme, false) {
                        result = Some(a);
                    }
                } else {
                    let fname = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let icon = file_icons::icon_for_extension(&ext);
                    let (r, g, b) = if theme.is_dark() { icon.dark_color } else { icon.light_color };

                    let resp = ui
                        .horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 18.0), egui::Sense::hover());
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                icon.glyph,
                                egui::FontId::new(15.0, file_icons::font_family()),
                                egui::Color32::from_rgb(r, g, b),
                            );
                            ui.selectable_label(false, fname)
                        })
                        .inner;
                    if resp.clicked() {
                        result = Some(TreeAction::OpenFile(path.clone()));
                    }
                }
            }
        });

        result
    }
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
        || path
            .file_name()
            .map(|n| n == "node_modules" || n == "target")
            .unwrap_or(false)
}
