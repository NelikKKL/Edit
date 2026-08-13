use std::path::PathBuf;

pub struct EditorTab {
    pub path: Option<PathBuf>,
    pub title: String,
    pub content: String,
    pub dirty: bool,
    /// Cursor/scroll id so egui keeps per-tab TextEdit state separate.
    pub id: egui::Id,
}

impl EditorTab {
    pub fn untitled(n: usize) -> Self {
        Self {
            path: None,
            title: format!("Без имени {n}"),
            content: String::new(),
            dirty: false,
            id: egui::Id::new(format!("tab-untitled-{n}")),
        }
    }

    pub fn from_path(path: PathBuf, content: String) -> Self {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Без имени".to_string());
        let id = egui::Id::new(path.to_string_lossy().to_string());
        Self {
            path: Some(path),
            title,
            content,
            dirty: false,
            id,
        }
    }

    pub fn extension(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("txt")
            .to_lowercase()
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(path) = &self.path {
            std::fs::write(path, &self.content)?;
            self.dirty = false;
        }
        Ok(())
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        std::fs::write(&path, &self.content)?;
        self.title = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Без имени".to_string());
        self.id = egui::Id::new(path.to_string_lossy().to_string());
        self.path = Some(path);
        self.dirty = false;
        Ok(())
    }
}
