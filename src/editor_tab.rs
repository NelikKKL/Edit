use egui::text::LayoutJob;
use egui::Color32;
use std::path::PathBuf;

/// Files at or above this size get a few extra performance safeguards:
/// syntax highlighting starts out disabled for them (re-tokenizing a huge
/// buffer with syntect on every keystroke is the main source of typing lag
/// in a big file) and the bracket/quote auto-close feature, which needs to
/// diff the whole buffer on every edit, is skipped. The file itself opens
/// completely normally otherwise.
pub const LARGE_FILE_BYTES: u64 = 2 * 1024 * 1024; // 2 MB

/// Everything the syntax highlighter needs in order to know whether a
/// previously built `LayoutJob` is still valid for the current frame, plus
/// the job itself. Rebuilding this (running syntect over the whole buffer
/// and re-shaping the text) is by far the most expensive thing the editor
/// does per frame, so it's only done when something in here actually
/// changed — not on every redraw (cursor blink, scrolling, window resize,
/// hovering a button, ...) like before.
pub struct HighlightCache {
    text_len: usize,
    text_hash: u64,
    extension: String,
    dark: bool,
    default_color: Color32,
    syntax_enabled: bool,
    font_size_bits: u32,
    wrap_bits: u32,
    search_ranges: Vec<(usize, usize)>,
    current_match: Option<usize>,
    match_bg: Color32,
    current_match_bg: Color32,
    pub job: LayoutJob,
}

impl HighlightCache {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        text: &str,
        extension: String,
        dark: bool,
        default_color: Color32,
        syntax_enabled: bool,
        font_size: f32,
        wrap: f32,
        search_ranges: Vec<(usize, usize)>,
        current_match: Option<usize>,
        match_bg: Color32,
        current_match_bg: Color32,
        job: LayoutJob,
    ) -> Self {
        Self {
            text_len: text.len(),
            text_hash: hash_text(text),
            extension,
            dark,
            default_color,
            syntax_enabled,
            font_size_bits: font_size.to_bits(),
            wrap_bits: wrap.to_bits(),
            search_ranges,
            current_match,
            match_bg,
            current_match_bg,
            job,
        }
    }

    /// Cheap-to-expensive ordered checks: bail out on the first mismatch so
    /// the O(n) text hash at the end is only computed when everything else
    /// already matches.
    #[allow(clippy::too_many_arguments)]
    pub fn matches(
        &self,
        text: &str,
        extension: &str,
        dark: bool,
        default_color: Color32,
        syntax_enabled: bool,
        font_size: f32,
        wrap: f32,
        search_ranges: &[(usize, usize)],
        current_match: Option<usize>,
        match_bg: Color32,
        current_match_bg: Color32,
    ) -> bool {
        self.text_len == text.len()
            && self.extension == extension
            && self.dark == dark
            && self.default_color == default_color
            && self.syntax_enabled == syntax_enabled
            && self.font_size_bits == font_size.to_bits()
            && self.wrap_bits == wrap.to_bits()
            && self.current_match == current_match
            && self.match_bg == match_bg
            && self.current_match_bg == current_match_bg
            && self.search_ranges == search_ranges
            && self.text_hash == hash_text(text)
    }
}

fn hash_text(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

pub struct EditorTab {
    pub path: Option<PathBuf>,
    pub title: String,
    pub content: String,
    pub dirty: bool,
    /// Cursor/scroll id so egui keeps per-tab TextEdit state separate.
    pub id: egui::Id,

    /// True while a file's contents are still being read on a background
    /// thread (see `EditApp::open_path`). The editor shows a lightweight
    /// placeholder instead of a `TextEdit` for these so opening a huge file
    /// never freezes the UI.
    pub loading: bool,
    /// Set for files at/above `LARGE_FILE_BYTES`; see that constant's docs.
    pub large: bool,

    /// Bumped every time `content` is mutated (by typing, autoclose,
    /// finishing a background load, ...). Cheap to compare, so it's used to
    /// invalidate the line-count cache below without rescanning the whole
    /// buffer on every frame just to redraw the gutter.
    pub version: u64,
    line_count_cache: Option<(u64, usize)>,

    /// Reused across frames when nothing that affects it has changed; see
    /// `HighlightCache` docs.
    pub highlight_cache: Option<HighlightCache>,
}

impl EditorTab {
    pub fn untitled(n: usize) -> Self {
        Self {
            path: None,
            title: format!("Без имени {n}"),
            content: String::new(),
            dirty: false,
            id: egui::Id::new(format!("tab-untitled-{n}")),
            loading: false,
            large: false,
            version: 0,
            line_count_cache: None,
            highlight_cache: None,
        }
    }

    pub fn from_path(path: PathBuf, content: String) -> Self {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Без имени".to_string());
        let id = egui::Id::new(path.to_string_lossy().to_string());
        let large = content.len() as u64 >= LARGE_FILE_BYTES;
        Self {
            path: Some(path),
            title,
            content,
            dirty: false,
            id,
            loading: false,
            large,
            version: 0,
            line_count_cache: None,
            highlight_cache: None,
        }
    }

    /// Placeholder tab shown immediately while a big file is being read on
    /// a background thread; `EditApp` swaps its contents in once the read
    /// finishes (matched up by `id`, which only depends on `path`).
    pub fn loading(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Без имени".to_string());
        let id = egui::Id::new(path.to_string_lossy().to_string());
        Self {
            path: Some(path),
            title: format!("{name} (загрузка…)"),
            content: String::new(),
            dirty: false,
            id,
            loading: true,
            large: false,
            version: 0,
            line_count_cache: None,
            highlight_cache: None,
        }
    }

    /// Call after directly mutating `content` outside of the normal
    /// TextEdit-diff path (e.g. finishing a background load) so caches keyed
    /// on `version` get invalidated.
    pub fn touch(&mut self) {
        self.version = self.version.wrapping_add(1);
        self.highlight_cache = None;
    }

    pub fn extension(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("txt")
            .to_lowercase()
    }

    /// Number of lines in `content`, recomputed only when `content` has
    /// actually changed since the last call (tracked via `version`) instead
    /// of rescanning the whole buffer on every frame.
    pub fn line_count(&mut self) -> usize {
        if let Some((v, n)) = self.line_count_cache {
            if v == self.version {
                return n;
            }
        }
        let n = self.content.lines().count().max(1);
        self.line_count_cache = Some((self.version, n));
        n
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
