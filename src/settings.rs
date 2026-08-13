use crate::theme::ThemeKind;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: ThemeKind,
    pub custom_css_path: Option<String>,

    pub font_family: Option<String>, // None = built-in monospace
    pub font_size: f32,

    pub show_line_numbers: bool,
    pub syntax_highlighting: bool,
    pub auto_close_brackets: bool,
    pub word_wrap: bool,
    pub tab_width: u8,
    pub show_sidebar: bool,

    #[serde(default)]
    pub last_folder: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeKind::Dark,
            custom_css_path: None,
            font_family: None,
            font_size: 15.0,
            show_line_numbers: true,
            syntax_highlighting: true,
            auto_close_brackets: true,
            word_wrap: false,
            tab_width: 4,
            show_sidebar: false,
            last_folder: None,
        }
    }
}

impl Settings {
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("edit"))
    }

    fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("settings.json"))
    }

    pub fn load() -> Self {
        Self::config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(dir) = Self::config_dir() {
            let _ = std::fs::create_dir_all(&dir);
            if let Some(path) = Self::config_path() {
                if let Ok(json) = serde_json::to_string_pretty(self) {
                    let _ = std::fs::write(path, json);
                }
            }
        }
    }

    /// Where the default example `theme.css` should live, so the settings
    /// UI can offer to create it on first use.
    pub fn default_css_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("theme.css"))
    }
}
