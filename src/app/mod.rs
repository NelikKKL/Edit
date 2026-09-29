use crate::autoclose;
use crate::custom_css;
use crate::editor_tab::{is_image_extension, EditorTab, HighlightCache};
use crate::file_tree::{FileTree, TreeAction};
use crate::fonts::{FontsState, SystemFonts};
use crate::search::SearchState;
use crate::settings::Settings;
use crate::syntax_highlight::Highlighter;
use crate::terminal::Terminal;
use crate::theme::{Theme, ThemeKind};
use egui::{Align, Align2, Color32, Frame, Margin, RichText, Sense, Stroke};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};

mod chrome;
mod editor;
mod files;
mod search_ui;
mod settings_ui;
mod shortcuts;
mod terminal_panel;
mod theme_fonts;

use self::settings_ui::SettingsCat;

/// Files at/above this size are read on a background thread instead of
/// blocking the UI thread inside `open_path`, so opening a big file never
/// freezes the window (a placeholder tab is shown immediately instead).
const ASYNC_LOAD_BYTES: u64 = 2 * 1024 * 1024; // 2 MB

/// Extra rows rendered above/below the actually-visible range in
/// `EditApp::large_file_view`, so a fast scroll doesn't flash blank space
/// while the next batch of rows lays out.
const LARGE_FILE_OVERSCAN_ROWS: usize = 10;
/// A single line longer than this (in chars) gets truncated in
/// `large_file_view` rather than laid out in full — guards against the
/// pathological case of a huge file that's one gigantic line (minified
/// JS/JSON, a log line, ...), which `LARGE_FILE_BYTES` alone doesn't catch
/// since that's a whole-file size check, not a per-line one.
const MAX_LARGE_LINE_CHARS: usize = 4000;

/// One in-flight background file read, tracked so `EditApp` can poll it
/// each frame without blocking.
struct PendingLoad {
    tab_id: egui::Id,
    path: PathBuf,
    rx: mpsc::Receiver<std::io::Result<String>>,
}

/// Width, in points, of the invisible strip along each edge of the
/// (undecorated) main window that lets the user grab it to resize — since
/// disabling native decorations for our custom title bar also removes the
/// OS's own edge-drag-to-resize behavior, we reimplement it ourselves.
const RESIZE_BORDER: f32 = 6.0;

pub struct EditApp {
    settings: Settings,
    /// Resolved interface language: `settings.lang` if the user picked one
    /// explicitly, otherwise whatever `Lang::detect_system()` returned at
    /// startup (or when the user last switched back to "follow system").
    /// Cached here rather than recomputed on every string lookup since
    /// reading the OS locale isn't free and doesn't change while running.
    lang: crate::i18n::Lang,
    theme: Theme,
    tabs: Vec<EditorTab>,
    active: usize,
    untitled_counter: usize,

    file_tree: FileTree,
    search: SearchState,
    highlighter: Highlighter,
    system_fonts: Arc<Mutex<FontsState>>,
    /// Set at startup when `settings.font_family` names a custom font that
    /// hasn't been installed yet because the background font scan (see
    /// `SystemFonts::scan_async`) is still running; checked once per frame
    /// (cheap `try_lock`) until it can be applied.
    font_install_pending: bool,
    /// Whether egui already has the default fonts *plus the icon fonts*
    /// (Seti/Codicons) installed. The UI draws icons from the very first
    /// frame, and egui panics on a `FontFamily::Name` that isn't bound to
    /// any font — so this has to happen even while the user's chosen text
    /// font is still waiting on the background system-font scan.
    base_fonts_installed: bool,
    pending_loads: Vec<PendingLoad>,
    /// Cache for the search-match byte ranges, so both the editor's
    /// highlight overlay and the search popup can share one computation per
    /// frame instead of each re-scanning (and, for case-insensitive
    /// search, re-allocating a whole lowercased copy of) the buffer.
    search_cache_key: Option<(usize, u64, String, bool)>,
    search_cache_ranges: Vec<(usize, usize)>,
    /// `line_col_of` walks the buffer from byte 0 up to the cursor, so for
    /// a big file it's not something we want to redo on every idle frame
    /// just to keep repainting the status bar; only recompute when the
    /// cursor actually moved.
    cursor_cache: Option<(egui::Id, usize, (usize, usize))>,
    /// The most recent non-empty text selection: `(tab id, start byte,
    /// end byte)`. `TextEdit` collapses the selection to a caret as part
    /// of processing the very click that opens our right-click context
    /// menu (it doesn't special-case the secondary mouse button for
    /// cursor placement), so by the time the menu's `Cut`/`Copy` handlers
    /// run, `output.cursor_range` for *that* frame is already empty. This
    /// field is only ever updated while the selection is non-empty, so it
    /// keeps the last real selection around for the menu to act on
    /// instead of "nothing selected".
    last_selection: Option<(egui::Id, usize, usize)>,
    /// Set by the context-menu closure every frame the menu is showing, and
    /// read (and reset) at the start of the next frame's editor pass. While
    /// it's true the editor re-applies `last_selection` and re-requests
    /// keyboard focus before drawing: clicking a menu item counts as
    /// "clicking elsewhere" for the `TextEdit`, which drops focus — and
    /// egui only paints a selection for the focused text field — so without
    /// this the highlight vanishes as soon as the menu opens.
    context_menu_open: bool,

    /// Integrated terminal panel (VS Code's Ctrl+`): one or more command
    /// sessions, shown at the bottom of the editor area when `show_terminal`
    /// is set.
    terminals: Vec<Terminal>,
    active_terminal: usize,
    show_terminal: bool,
    terminal_focus_pending: bool,
    terminal_serial: usize,

    /// Settings editor state: the search box text and the selected TOC entry.
    settings_search: String,
    settings_cat: SettingsCat,
    css_status: Option<String>,

    pending_scroll_line: Option<usize>,
    status_line_col: (usize, usize),
    status_message: Option<String>,

    close_confirm: Option<usize>,
    fullscreen: bool,
}

impl EditApp {
    /// Shorthand for `i18n::t(self.lang, key)`, used throughout the UI code
    /// below instead of hardcoded Russian strings.
    fn t(&self, key: &'static str) -> &'static str {
        crate::i18n::t(self.lang, key)
    }

    pub fn new(cc: &eframe::CreationContext<'_>, initial_file: Option<String>) -> Self {
        let settings = Settings::load();
        // Scanning every system font file (to build the family picker in
        // Settings > Font) can take a while on machines with large font
        // collections. Do it on a background thread so it never delays the
        // very first frame; `font_install_pending` below makes sure a
        // custom font from settings still gets applied automatically as
        // soon as the scan finishes.
        let system_fonts = SystemFonts::scan_async();
        let font_install_pending = settings.font_family.is_some();
        let lang = settings.lang.unwrap_or_else(crate::i18n::Lang::detect_system);

        let mut app = Self {
            lang,
            theme: Theme::dark(),
            tabs: Vec::new(),
            active: 0,
            untitled_counter: 1,
            file_tree: FileTree::new(),
            search: SearchState::default(),
            highlighter: Highlighter::new(),
            system_fonts,
            font_install_pending,
            base_fonts_installed: false,
            pending_loads: Vec::new(),
            search_cache_key: None,
            search_cache_ranges: Vec::new(),
            cursor_cache: None,
            last_selection: None,
            context_menu_open: false,
            terminals: Vec::new(),
            active_terminal: 0,
            show_terminal: false,
            terminal_focus_pending: false,
            terminal_serial: 0,
            settings_search: String::new(),
            settings_cat: SettingsCat::Common,
            css_status: None,
            pending_scroll_line: None,
            status_line_col: (1, 1),
            status_message: None,
            close_confirm: None,
            fullscreen: false,
            settings,
        };

        app.apply_theme(&cc.egui_ctx);

        if let Some(path) = initial_file {
            app.open_path(PathBuf::from(path));
        }
        if app.tabs.is_empty() {
            app.new_tab();
        }

        app
    }
}

impl eframe::App for EditApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_pending_loads();
        if self.font_install_pending {
            self.install_font(ctx);
        }
        // egui only calls `update` again on input/animation by default; while
        // a background file read or font scan is still in flight, ask for a
        // steady trickle of repaints so we notice it finishing even if the
        // user isn't touching the mouse/keyboard.
        if !self.pending_loads.is_empty() || self.font_install_pending {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        self.handle_shortcuts(ctx);
        self.handle_window_resize(ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        self.handle_dropped_files(ctx);

        let lang = self.lang;
        for term in &mut self.terminals {
            term.poll(lang);
        }
        self.title_bar(ctx);
        self.status_bar(ctx);
        self.activity_bar(ctx);
        self.sidebar(ctx);
        self.tab_bar(ctx);
        self.terminal_panel(ctx);
        self.editor(ctx);
        self.search_window(ctx);
        self.close_confirm_modal(ctx);
    }
}
