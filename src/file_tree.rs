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

    pub fn ui(&mut self, ui: &mut egui::Ui, theme: &Theme) -> TreeAction {
        let Some(root) = self.root.clone() else {
            ui.weak("Папка не выбрана");
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

        let header = egui::CollapsingHeader::new(name)
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
                    let resp = ui.selectable_label(false, fname);
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
