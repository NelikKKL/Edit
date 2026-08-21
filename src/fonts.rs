use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Family name -> path to a .ttf/.otf file for it. Built by scanning the OS
/// font directories and reading every font file's `name` table, similar to
/// what Notepad's font picker draws from.
///
/// On a typical Linux install this walks `/usr/share/fonts` and friends,
/// which can easily be a few thousand files — parsing all of them
/// synchronously used to run before the very first frame was shown, adding
/// a very noticeable delay to app startup. Use [`SystemFonts::scan_async`]
/// instead of calling [`SystemFonts::scan`] directly on the UI thread.
pub struct SystemFonts {
    pub families: BTreeMap<String, PathBuf>,
}

/// Shared handle to a font scan that may still be running on a background
/// thread. Cheap to check (`try_lock`) from the UI thread every frame.
pub enum FontsState {
    Loading,
    Ready(SystemFonts),
}

impl FontsState {
    pub fn is_ready(&self) -> bool {
        matches!(self, FontsState::Ready(_))
    }

    pub fn names(&self) -> Vec<String> {
        match self {
            FontsState::Loading => Vec::new(),
            FontsState::Ready(f) => f.names(),
        }
    }

    pub fn load_bytes(&self, family: &str) -> Option<Vec<u8>> {
        match self {
            FontsState::Loading => None,
            FontsState::Ready(f) => f.load_bytes(family),
        }
    }
}

impl SystemFonts {
    pub fn scan() -> Self {
        let mut families = BTreeMap::new();
        for dir in font_dirs() {
            scan_dir(&dir, &mut families);
        }
        Self { families }
    }

    /// Kicks off the (potentially slow) filesystem scan on a background
    /// thread and returns immediately with a handle that starts out as
    /// `FontsState::Loading`. The app can keep drawing its first frames
    /// (using the built-in monospace font) while the scan finishes in the
    /// background; once it's done the handle flips to `FontsState::Ready`
    /// and the app can install the user's chosen font, if any.
    pub fn scan_async() -> Arc<Mutex<FontsState>> {
        let state = Arc::new(Mutex::new(FontsState::Loading));
        let state_bg = state.clone();
        std::thread::spawn(move || {
            let fonts = SystemFonts::scan();
            if let Ok(mut guard) = state_bg.lock() {
                *guard = FontsState::Ready(fonts);
            }
        });
        state
    }

    pub fn names(&self) -> Vec<String> {
        self.families.keys().cloned().collect()
    }

    pub fn load_bytes(&self, family: &str) -> Option<Vec<u8>> {
        let path = self.families.get(family)?;
        std::fs::read(path).ok()
    }
}

fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        } else {
            dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
        }
        if let Some(local) = dirs::data_local_dir() {
            dirs.push(local.join("Microsoft").join("Windows").join("Fonts"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".fonts"));
        }
        if let Some(data) = dirs::data_dir() {
            dirs.push(data.join("fonts"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join("Library").join("Fonts"));
        }
    }

    dirs
}

fn scan_dir(dir: &Path, out: &mut BTreeMap<String, PathBuf>) {
    let Ok(walker) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, out);
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if ext != "ttf" && ext != "otf" {
            continue;
        }
        if let Some(name) = family_name(&path) {
            out.entry(name).or_insert(path);
        }
    }
}

/// Reads the font's `name` table to find a human-readable family name.
fn family_name(path: &Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    let face = ttf_parser::Face::parse(&data, 0).ok()?;
    face.names().into_iter().find_map(|n| {
        // name_id 1 = "Font Family", 16 = "Typographic Family" (preferred).
        if (n.name_id == 16 || n.name_id == 1) && n.is_unicode() {
            n.to_string()
        } else {
            None
        }
    })
}
