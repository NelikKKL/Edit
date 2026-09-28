use crate::autoclose;
use crate::custom_css;
use crate::editor_tab::{is_image_extension, EditorTab, HighlightCache};
use crate::file_tree::{FileTree, TreeAction};
use crate::fonts::{FontsState, SystemFonts};
use crate::search::SearchState;
use crate::settings::Settings;
use crate::syntax_highlight::Highlighter;
use crate::theme::{Theme, ThemeKind};
use egui::{Align, Align2, Color32, Frame, Margin, RichText, Sense, Stroke};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};

/// Files at/above this size are read on a background thread instead of
/// blocking the UI thread inside `open_path`, so opening a big file never
/// freezes the window (a placeholder tab is shown immediately instead).
const ASYNC_LOAD_BYTES: u64 = 2 * 1024 * 1024; // 2 MB

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsPage {
    Appearance,
    Font,
    Editor,
    Advanced,
}

impl SettingsPage {
    const ALL: [SettingsPage; 4] = [
        SettingsPage::Appearance,
        SettingsPage::Font,
        SettingsPage::Editor,
        SettingsPage::Advanced,
    ];
    fn label(&self, lang: crate::i18n::Lang) -> &'static str {
        use crate::i18n::t;
        match self {
            SettingsPage::Appearance => t(lang, "settings.appearance"),
            SettingsPage::Font => t(lang, "settings.font"),
            SettingsPage::Editor => t(lang, "settings.editor"),
            SettingsPage::Advanced => t(lang, "settings.advanced"),
        }
    }
}

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

    show_settings: bool,
    settings_page: SettingsPage,
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
            pending_loads: Vec::new(),
            search_cache_key: None,
            search_cache_ranges: Vec::new(),
            cursor_cache: None,
            last_selection: None,
            show_settings: false,
            settings_page: SettingsPage::Appearance,
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

    // ---------------------------------------------------------------- data

    fn new_tab(&mut self) {
        let tab = EditorTab::untitled(self.untitled_counter, self.lang);
        self.untitled_counter += 1;
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    fn open_path(&mut self, path: PathBuf) {
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
        if self.tabs.len() == 1 && self.tabs[0].path.is_none() && !self.tabs[0].dirty && self.tabs[0].content.is_empty() {
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
    fn insert_loaded_tab(&mut self, path: PathBuf, content: String) {
        if self.tabs.len() == 1 && self.tabs[0].path.is_none() && !self.tabs[0].dirty && self.tabs[0].content.is_empty() {
            self.tabs[0] = EditorTab::from_path(path, content, self.lang);
            self.active = 0;
        } else {
            self.tabs.push(EditorTab::from_path(path, content, self.lang));
            self.active = self.tabs.len() - 1;
        }
    }

    /// Same idea as `insert_loaded_tab`, for an image-preview tab.
    fn insert_image_tab(&mut self, path: PathBuf, bytes: Vec<u8>) {
        let tab = EditorTab::from_image(path, bytes);
        if self.tabs.len() == 1 && self.tabs[0].path.is_none() && !self.tabs[0].dirty && self.tabs[0].content.is_empty() {
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
    fn poll_pending_loads(&mut self) {
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

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().set_title(self.t("dialog.open_file")).pick_file() {
            self.open_path(path);
        }
    }

    fn open_folder_dialog(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().set_title(self.t("dialog.open_folder")).pick_folder() {
            self.file_tree.set_root(dir.clone());
            self.settings.show_sidebar = true;
            self.settings.last_folder = Some(dir.to_string_lossy().to_string());
            self.settings.save();
        }
    }

    fn save_tab(&mut self, idx: usize) {
        let Some(tab) = self.tabs.get_mut(idx) else { return };
        if tab.is_image() {
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

    fn save_tab_as(&mut self, idx: usize) {
        if self.tabs.get(idx).is_some_and(|t| t.is_image()) {
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

    fn request_close_tab(&mut self, idx: usize) {
        if self.tabs.get(idx).map(|t| t.dirty).unwrap_or(false) {
            self.close_confirm = Some(idx);
        } else {
            self.close_tab_now(idx);
        }
    }

    fn close_tab_now(&mut self, idx: usize) {
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

    // --------------------------------------------------------------- theme

    fn apply_theme(&mut self, ctx: &egui::Context) {
        self.theme = match self.settings.theme {
            ThemeKind::Light => Theme::light(),
            ThemeKind::Dark => Theme::dark(),
            ThemeKind::Char => Theme::char_theme(),
            ThemeKind::Custom => self.load_custom_theme(),
        };
        self.theme.apply_to_ctx(ctx);
        self.install_font(ctx);
    }

    fn load_custom_theme(&mut self) -> Theme {
        let Some(path) = self.settings.custom_css_path.clone() else {
            self.css_status = Some(crate::i18n::theme_file_not_selected(self.lang).to_string());
            return Theme::dark();
        };
        match custom_css::parse_css_file(Path::new(&path)) {
            Ok(parsed) => {
                let (font_family, font_size) = custom_css::font_overrides_from_css(&parsed);
                if let Some(ff) = font_family {
                    self.settings.font_family = Some(ff);
                }
                if let Some(fs) = font_size {
                    self.settings.font_size = fs;
                }
                self.css_status = None;
                custom_css::theme_from_css(&parsed)
            }
            Err(e) => {
                self.css_status = Some(crate::i18n::css_read_failed(self.lang, &path, &e.to_string()));
                Theme::dark()
            }
        }
    }

    fn install_font(&mut self, ctx: &egui::Context) {
        let Some(family) = self.settings.font_family.clone() else {
            ctx.set_fonts(with_icon_fonts(egui::FontDefinitions::default()));
            self.font_install_pending = false;
            return;
        };

        // The system font list is still being scanned in the background;
        // keep the built-in font for now and try again next frame (see
        // `font_install_pending` polling in `update`) instead of blocking
        // here until the scan finishes.
        let Ok(state) = self.system_fonts.lock() else { return };
        if !state.is_ready() {
            self.font_install_pending = true;
            return;
        }

        let mut defs = egui::FontDefinitions::default();
        if let Some(bytes) = state.load_bytes(&family) {
            defs.font_data
                .insert("user_font".to_owned(), egui::FontData::from_owned(bytes));
            defs.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "user_font".to_owned());
            defs.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "user_font".to_owned());
        } else {
            self.status_message = Some(crate::i18n::font_not_found(self.lang, &family));
        }
        drop(state);
        ctx.set_fonts(with_icon_fonts(defs));
        self.font_install_pending = false;
    }

    // ----------------------------------------------------------- shortcuts

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};

        ctx.input_mut(|i| {
            if i.consume_key(Modifiers::CTRL, Key::O) {
                self.open_file_dialog();
            }
            if i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::O) {
                self.open_folder_dialog();
            }
            if i.consume_key(Modifiers::CTRL, Key::S) {
                self.save_tab(self.active);
            }
            if i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::S) {
                self.save_tab_as(self.active);
            }
            if i.consume_key(Modifiers::CTRL, Key::N) {
                self.new_tab();
            }
            if i.consume_key(Modifiers::CTRL, Key::W) {
                self.request_close_tab(self.active);
            }
            if i.consume_key(Modifiers::CTRL, Key::F) {
                self.search.open();
            }
            if i.consume_key(Modifiers::CTRL, Key::Comma) {
                self.show_settings = true;
            }
            if i.consume_key(Modifiers::NONE, Key::F11) {
                self.fullscreen = !self.fullscreen;
            }
            if i.consume_key(Modifiers::NONE, Key::Escape) && self.search.open {
                self.search.close();
            }
        });
    }

    /// Lets the user resize the (undecorated) main window by dragging its
    /// edges/corners, the way a normal OS window border would work — with
    /// the cursor changing shape near the edge, and a drag actually
    /// resizing the window.
    fn handle_window_resize(&mut self, ctx: &egui::Context) {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        if maximized || self.fullscreen {
            return;
        }

        let screen = ctx.screen_rect();
        let (hover_pos, primary_pressed) =
            ctx.input(|i| (i.pointer.hover_pos(), i.pointer.primary_pressed()));

        let Some(pos) = hover_pos else { return };

        let west = pos.x <= screen.left() + RESIZE_BORDER;
        let east = pos.x >= screen.right() - RESIZE_BORDER;
        let north = pos.y <= screen.top() + RESIZE_BORDER;
        let south = pos.y >= screen.bottom() - RESIZE_BORDER;

        let hit = if west && north {
            Some((egui::CursorIcon::ResizeNwSe, egui::ResizeDirection::NorthWest))
        } else if east && south {
            Some((egui::CursorIcon::ResizeNwSe, egui::ResizeDirection::SouthEast))
        } else if east && north {
            Some((egui::CursorIcon::ResizeNeSw, egui::ResizeDirection::NorthEast))
        } else if west && south {
            Some((egui::CursorIcon::ResizeNeSw, egui::ResizeDirection::SouthWest))
        } else if west {
            Some((egui::CursorIcon::ResizeHorizontal, egui::ResizeDirection::West))
        } else if east {
            Some((egui::CursorIcon::ResizeHorizontal, egui::ResizeDirection::East))
        } else if north {
            Some((egui::CursorIcon::ResizeVertical, egui::ResizeDirection::North))
        } else if south {
            Some((egui::CursorIcon::ResizeVertical, egui::ResizeDirection::South))
        } else {
            None
        };

        if let Some((cursor, direction)) = hit {
            ctx.set_cursor_icon(cursor);
            if primary_pressed {
                ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
            }
        }
    }

    // -------------------------------------------------------------- panels

    /// VS Code puts file actions (New/Open/Save/...) in the "File" menu
    /// rather than as toolbar buttons; this app has a custom, undecorated
    /// title bar instead of a native menu bar, so the menu lives there.
    fn file_menu(&mut self, ui: &mut egui::Ui) {
        let label = RichText::new(self.t("menu.file")).color(self.theme.titlebar_fg);
        ui.menu_button(label, |ui| {
            if ui.button(self.t("menu.new_tab")).clicked() {
                self.new_tab();
                ui.close_menu();
            }
            if ui.button(self.t("toolbar.open")).clicked() {
                self.open_file_dialog();
                ui.close_menu();
            }
            if ui.button(self.t("toolbar.folder")).clicked() {
                self.open_folder_dialog();
                ui.close_menu();
            }
            ui.separator();
            if ui.button(self.t("toolbar.save")).clicked() {
                self.save_tab(self.active);
                ui.close_menu();
            }
            if ui.button(self.t("dialog.save_as")).clicked() {
                self.save_tab_as(self.active);
                ui.close_menu();
            }
            ui.separator();
            if ui.button(self.t("menu.exit")).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                ui.close_menu();
            }
        });
    }

    fn title_bar(&mut self, ctx: &egui::Context) {
        let height = 34.0;
        egui::TopBottomPanel::top("title_bar")
            .exact_height(height)
            .frame(Frame::none().fill(self.theme.titlebar_bg))
            .show(ctx, |ui| {
                let full_rect = ui.max_rect();
                let drag_rect = egui::Rect::from_min_max(
                    full_rect.min + egui::vec2(RESIZE_BORDER, RESIZE_BORDER),
                    full_rect.max - egui::vec2(RESIZE_BORDER, 0.0),
                );
                let response = ui.interact(drag_rect, egui::Id::new("title_bar_drag"), Sense::click_and_drag());
                if response.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if response.double_clicked() {
                    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }

                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new("edit").color(self.theme.titlebar_fg).strong());
                    ui.add_space(6.0);
                    self.file_menu(ui);
                    if let Some(tab) = self.tabs.get(self.active) {
                        let dirty = if tab.dirty { " *" } else { "" };
                        ui.label(RichText::new(format!("— {}{}", tab.title, dirty)).color(self.theme.fg_dim));
                    }

                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if window_button(ui, "X", self.theme.close_hover, self.theme.titlebar_fg).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                        let sym = if maximized { "[ ]" } else { "[]" };
                        if window_button(ui, sym, self.theme.button_hover, self.theme.titlebar_fg).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if window_button(ui, "_", self.theme.button_hover, self.theme.titlebar_fg).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });
    }

    fn tab_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("tabs")
            .exact_height(30.0)
            .frame(Frame::none().fill(self.theme.bg))
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut to_activate = None;
                        let mut to_close = None;
                        for (i, tab) in self.tabs.iter().enumerate() {
                            let selected = i == self.active;
                            let bg = if selected { self.theme.panel_bg } else { self.theme.bg };
                            egui::Frame::none()
                                .fill(bg)
                                .inner_margin(Margin::symmetric(8.0, 4.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let dirty = if tab.dirty { " *" } else { "" };
                                        let text = RichText::new(format!("{}{}", tab.title, dirty))
                                            .color(if selected { self.theme.fg } else { self.theme.fg_dim });
                                        if ui.selectable_label(false, text).clicked() {
                                            to_activate = Some(i);
                                        }
                                        if ui.small_button("X").clicked() {
                                            to_close = Some(i);
                                        }
                                    });
                                });
                        }
                        let plus = ui.add(egui::Button::new(crate::codicons::rich(crate::codicons::ADD, 14.0)).frame(false));
                        if plus.on_hover_text(self.t("toolbar.new_tab_tooltip")).clicked() {
                            self.new_tab();
                        }
                        if let Some(i) = to_activate {
                            self.active = i;
                        }
                        if let Some(i) = to_close {
                            self.request_close_tab(i);
                        }
                    });
                });
            });
    }

    /// VS Code's Activity Bar: the narrow icon strip to the left of the
    /// Sidebar. Only icons for things this app actually does — Explorer
    /// (toggles the file tree sidebar) and Search (opens the in-editor
    /// find widget, same as Ctrl+F) up top, Settings pinned to the bottom.
    fn activity_bar(&mut self, ctx: &egui::Context) {
        let theme = self.theme.clone();
        egui::SidePanel::left("activity_bar")
            .exact_width(46.0)
            .resizable(false)
            .frame(Frame::none().fill(theme.sidebar_bg))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    let explorer_tip = self.t("activity.explorer");
                    if activity_icon(ui, &theme, crate::codicons::FILES, self.settings.show_sidebar, explorer_tip).clicked() {
                        self.settings.show_sidebar = !self.settings.show_sidebar;
                        self.settings.save();
                    }
                    let search_tip = self.t("toolbar.search");
                    if activity_icon(ui, &theme, crate::codicons::SEARCH, false, search_tip).clicked() {
                        self.search.open();
                    }

                    // Pin the settings icon to the bottom of the strip.
                    let reserved_for_bottom_icon = 46.0;
                    let gap = ui.available_height() - reserved_for_bottom_icon;
                    if gap > 0.0 {
                        ui.add_space(gap);
                    }
                    let settings_tip = self.t("toolbar.settings");
                    if activity_icon(ui, &theme, crate::codicons::SETTINGS_GEAR, self.show_settings, settings_tip).clicked() {
                        self.show_settings = true;
                    }
                });
            });
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        if !self.settings.show_sidebar {
            return;
        }
        let theme = self.theme.clone();
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(230.0)
            .width_range(150.0..=500.0)
            .frame(
                Frame::none()
                    .fill(theme.sidebar_bg)
                    .inner_margin(Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.t("activity.explorer").to_uppercase()).color(theme.fg_dim).small());
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let folder_btn = ui.add(egui::Button::new(crate::codicons::rich(crate::codicons::FOLDER_OPENED, 15.0)).frame(false));
                        if folder_btn.on_hover_text(self.t("toolbar.folder")).clicked() {
                            self.open_folder_dialog();
                        }
                        let file_btn = ui.add(egui::Button::new(crate::codicons::rich(crate::codicons::FILE, 15.0)).frame(false));
                        if file_btn.on_hover_text(self.t("toolbar.open")).clicked() {
                            self.open_file_dialog();
                        }
                    });
                });
                ui.add_space(4.0);

                if self.file_tree.root.is_none() {
                    ui.add_space(16.0);
                    ui.label(RichText::new(self.t("file_tree.empty_hint")).color(theme.fg_dim).small());
                    ui.add_space(8.0);
                    if ui.button(self.t("toolbar.folder")).clicked() {
                        self.open_folder_dialog();
                    }
                    return;
                }

                match self.file_tree.ui(ui, &theme, self.lang) {
                    TreeAction::OpenFile(path) => self.open_path(path),
                    TreeAction::None => {}
                }
            });
    }

    fn editor(&mut self, ctx: &egui::Context) {
        let theme = self.theme.clone();
        egui::CentralPanel::default()
            .frame(Frame::none().fill(theme.editor_bg))
            .show(ctx, |ui| {
                if self.tabs.is_empty() {
                    ui.centered_and_justified(|ui| ui.weak(self.t("editor.no_open_files")));
                    return;
                }
                let idx = self.active.min(self.tabs.len() - 1);
                self.active = idx;

                if self.tabs[idx].loading {
                    ui.centered_and_justified(|ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.weak(self.t("editor.loading_file"));
                        });
                    });
                    return;
                }

                if self.tabs[idx].is_image() {
                    self.image_view(ui, ctx, idx);
                    return;
                }

                // ---- gather everything the layouter closure needs, as owned
                // values, BEFORE taking a mutable borrow of the tab's content.
                let font_size = self.settings.font_size;
                // Syntax highlighting is auto-disabled for very large files:
                // syntect re-tokenizes the whole buffer from scratch on
                // every edit (it isn't incremental), which turns into
                // per-keystroke lag once a file gets big enough.
                let syntax_enabled = self.settings.syntax_highlighting && !self.tabs[idx].large;
                let word_wrap = self.settings.word_wrap;
                let dark = theme.is_dark();
                let default_color = theme.fg;
                let match_bg = theme.accent.gamma_multiply(0.30);
                let current_match_bg = theme.accent.gamma_multiply(0.65);
                let extension = self.tabs[idx].extension();
                let tab_id = self.tabs[idx].id;
                // Drop any stale cached selection from a different tab so
                // the context menu never acts on another file's text.
                if self.last_selection.is_some_and(|(id, _, _)| id != tab_id) {
                    self.last_selection = None;
                }
                let tab_large = self.tabs[idx].large;

                let search_ranges = self.search_matches();
                let current_match = if search_ranges.is_empty() {
                    None
                } else {
                    Some(self.search.current_match.min(search_ranges.len() - 1))
                };

                // The bracket/quote auto-close feature needs a full
                // before/after diff of the buffer, which means a full
                // clone up front; skip that (and the feature) for very
                // large files where it'd mean copying megabytes of text on
                // every single frame just in case a keystroke happened.
                let before_text = if tab_large { None } else { Some(self.tabs[idx].content.clone()) };

                let line_count = self.tabs[idx].line_count();
                let font_id = egui::FontId::monospace(font_size);
                let line_h = ui.text_style_height(&egui::TextStyle::Monospace).max(font_size * 1.3);

                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            if self.settings.show_line_numbers {
                                line_number_gutter(ui, &theme, line_count, font_id.clone());
                            }

                            let highlighter = &self.highlighter;
                            let font_id_c = font_id.clone();
                            let ext_c = extension.clone();
                            let ranges_c = search_ranges.clone();

                            // Reused across frames where nothing that
                            // affects the highlighted layout has changed —
                            // see `HighlightCache` docs. It's parked in a
                            // `RefCell` (rather than mutated directly on
                            // `self.tabs[idx]`) purely because the closure
                            // below runs *while* `self.tabs[idx].content`
                            // is already mutably borrowed by `TextEdit`;
                            // the cache is moved back onto the tab right
                            // after `.show()` returns.
                            let cache_cell = RefCell::new(self.tabs[idx].highlight_cache.take());
                            let cache_ref = &cache_cell;

                            let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                                let effective_wrap = if word_wrap { wrap_width } else { f32::INFINITY };
                                let mut cache = cache_ref.borrow_mut();

                                let syntax_ready = highlighter.is_ready();
                                let hit = cache.as_ref().is_some_and(|c| {
                                    c.matches(
                                        text,
                                        &ext_c,
                                        dark,
                                        default_color,
                                        syntax_enabled,
                                        syntax_ready,
                                        font_id_c.size,
                                        effective_wrap,
                                        &ranges_c,
                                        current_match,
                                        match_bg,
                                        current_match_bg,
                                    )
                                });

                                let job = if hit {
                                    cache.as_ref().unwrap().job.clone()
                                } else {
                                    let mut job = highlighter.build_job(
                                        text,
                                        &ext_c,
                                        font_id_c.clone(),
                                        dark,
                                        default_color,
                                        syntax_enabled,
                                        &ranges_c,
                                        current_match,
                                        match_bg,
                                        current_match_bg,
                                    );
                                    job.wrap.max_width = effective_wrap;
                                    *cache = Some(HighlightCache::new(
                                        text,
                                        ext_c.clone(),
                                        dark,
                                        default_color,
                                        syntax_enabled,
                                        syntax_ready,
                                        font_id_c.size,
                                        effective_wrap,
                                        ranges_c.clone(),
                                        current_match,
                                        match_bg,
                                        current_match_bg,
                                        job.clone(),
                                    ));
                                    job
                                };
                                drop(cache);
                                ui.fonts(|f| f.layout_job(job))
                            };

                            let output = egui::TextEdit::multiline(&mut self.tabs[idx].content)
                                .id(tab_id)
                                .font(font_id.clone())
                                .desired_width(f32::INFINITY)
                                .frame(false)
                                .lock_focus(true)
                                .layouter(&mut layouter)
                                .show(ui);

                            drop(layouter);
                            self.tabs[idx].highlight_cache = cache_cell.into_inner();

                            // Remember the selection while it's non-empty (see
                            // `last_selection`'s doc comment for why: the click that
                            // opens the context menu below already collapses
                            // `output.cursor_range` for *this* frame).
                            if let Some(cursor_range) = output.cursor_range {
                                let (lo, hi) = selection_byte_range(&self.tabs[idx].content, &cursor_range);
                                if lo != hi {
                                    self.last_selection = Some((tab_id, lo, hi));
                                }
                            }

                            // Right-click (or long-press, on touch) context menu with the
                            // usual Cut/Copy/Paste/Select all actions — `TextEdit` doesn't
                            // provide one on its own, only keyboard shortcuts.
                            let lang = self.lang;
                            output.response.context_menu(|ui| {
                                let selection = self
                                    .last_selection
                                    .filter(|(id, lo, hi)| *id == tab_id && lo != hi);
                                let has_selection = selection.is_some();

                                if ui
                                    .add_enabled(has_selection, egui::Button::new(crate::i18n::t(lang, "context_menu.cut")))
                                    .clicked()
                                {
                                    if let Some((_, lo, hi)) = selection {
                                        let cut = self.tabs[idx].content[lo..hi].to_string();
                                        ui.ctx().output_mut(|o| o.copied_text = cut);
                                        self.tabs[idx].content.replace_range(lo..hi, "");
                                        self.tabs[idx].touch();
                                        self.tabs[idx].dirty = true;
                                        self.last_selection = None;
                                    }
                                    ui.close_menu();
                                }

                                if ui
                                    .add_enabled(has_selection, egui::Button::new(crate::i18n::t(lang, "context_menu.copy")))
                                    .clicked()
                                {
                                    if let Some((_, lo, hi)) = selection {
                                        let copied = self.tabs[idx].content[lo..hi].to_string();
                                        ui.ctx().output_mut(|o| o.copied_text = copied);
                                    }
                                    ui.close_menu();
                                }

                                if ui.button(crate::i18n::t(lang, "context_menu.paste")).clicked() {
                                    // egui only sees pasted text through OS paste *events*
                                    // (Ctrl+V), it has no on-demand "read the clipboard now"
                                    // API — so a clipboard-reading crate is needed to back a
                                    // clickable Paste menu item.
                                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                        if let Ok(text) = clipboard.get_text() {
                                            let (lo, hi) = selection
                                                .map(|(_, lo, hi)| (lo, hi))
                                                .or_else(|| {
                                                    output
                                                        .cursor_range
                                                        .map(|r| selection_byte_range(&self.tabs[idx].content, &r))
                                                })
                                                .unwrap_or_else(|| {
                                                    let end = self.tabs[idx].content.len();
                                                    (end, end)
                                                });
                                            self.tabs[idx].content.replace_range(lo..hi, &text);
                                            self.tabs[idx].touch();
                                            self.tabs[idx].dirty = true;
                                            self.last_selection = None;
                                        }
                                    }
                                    ui.close_menu();
                                }

                                ui.separator();

                                if ui.button(crate::i18n::t(lang, "context_menu.select_all")).clicked() {
                                    let char_count = self.tabs[idx].content.chars().count();
                                    let mut state = egui::text_edit::TextEditState::load(ui.ctx(), tab_id).unwrap_or_default();
                                    state.set_ccursor_range(Some(egui::text::CCursorRange::two(
                                        egui::text::CCursor::new(0),
                                        egui::text::CCursor::new(char_count),
                                    )));
                                    egui::text_edit::TextEditState::store(state, ui.ctx(), tab_id);
                                    self.last_selection = Some((tab_id, 0, self.tabs[idx].content.len()));
                                    ui.close_menu();
                                }
                            });

                            if output.response.changed() {
                                self.tabs[idx].touch();
                                match &before_text {
                                    Some(before) => {
                                        let after_text = self.tabs[idx].content.clone();
                                        if after_text != *before {
                                            self.tabs[idx].dirty = true;
                                            if self.settings.auto_close_brackets {
                                                if let Some(fixed) = autoclose::process_edit(before, &after_text) {
                                                    self.tabs[idx].content = fixed;
                                                    self.tabs[idx].touch();
                                                }
                                            }
                                        }
                                    }
                                    None => {
                                        self.tabs[idx].dirty = true;
                                    }
                                }
                            }

                            // Track roughly where the cursor is for the status bar.
                            // `line_col_of` scans from the start of the buffer,
                            // so only rerun it when the cursor position (not
                            // just the frame) actually changed.
                            if let Some(cursor_range) = output.cursor_range {
                                let ccursor = cursor_range.primary.ccursor.index;
                                let cache_hit = self
                                    .cursor_cache
                                    .as_ref()
                                    .is_some_and(|(id, c, _)| *id == tab_id && *c == ccursor);
                                let (line, col) = if cache_hit {
                                    self.cursor_cache.unwrap().2
                                } else {
                                    let lc = line_col_of(&self.tabs[idx].content, ccursor);
                                    self.cursor_cache = Some((tab_id, ccursor, lc));
                                    lc
                                };
                                self.status_line_col = (line, col);
                            }
                        });

                        if let Some(target_line) = self.pending_scroll_line.take() {
                            let y = target_line as f32 * line_h;
                            ui.scroll_to_rect(
                                egui::Rect::from_min_size(egui::pos2(0.0, y), egui::vec2(1.0, line_h)),
                                Some(Align::Center),
                            );
                        }
                    });
            });
    }

    /// Renders an image-preview tab: decode + upload the texture once
    /// (cached on the tab itself in `image_texture`, same idea as
    /// `highlight_cache`), then show it centered and scaled down to fit
    /// the pane if it's bigger than the available space — mirrors what
    /// VS Code's own built-in image preview does for a raster image tab.
    fn image_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, idx: usize) {
        let theme = self.theme.clone();

        if self.tabs[idx].image_texture.is_none() {
            if let Some(bytes) = self.tabs[idx].image_bytes.clone() {
                match image::load_from_memory(&bytes) {
                    Ok(decoded) => {
                        let decoded = decoded.into_rgba8();
                        let (w, h) = decoded.dimensions();
                        let pixels = decoded.into_raw();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
                        let texture = ctx.load_texture(format!("image-tab-{idx}"), color_image, egui::TextureOptions::LINEAR);
                        self.tabs[idx].image_texture = Some(texture);
                    }
                    Err(e) => {
                        self.status_message = Some(crate::i18n::open_file_error(self.lang, &e.to_string()));
                        // Don't retry the decode every single frame on a
                        // corrupt/unsupported file.
                        self.tabs[idx].image_bytes = None;
                    }
                }
            }
        }

        let Some(texture) = self.tabs[idx].image_texture.clone() else {
            return;
        };

        let available = ui.available_size();
        let tex_size = texture.size_vec2();
        let scale = if tex_size.x > 0.0 && tex_size.y > 0.0 {
            (available.x / tex_size.x).min(available.y / tex_size.y).min(1.0)
        } else {
            1.0
        };
        let shown = tex_size * scale;

        ui.allocate_ui_with_layout(available, egui::Layout::top_down(Align::Center), |ui| {
            let top_pad = ((available.y - shown.y - 24.0) / 2.0).max(0.0);
            ui.add_space(top_pad);
            ui.add(egui::Image::new(&texture).max_size(shown));
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("{} × {}", tex_size.x as i32, tex_size.y as i32))
                    .color(theme.fg_dim)
                    .small(),
            );
        });
    }

    /// Byte ranges of every match of the current search query in the active
    /// tab, recomputed only when the tab, its content, the query, or the
    /// case-sensitivity toggle actually changed since the last call —
    /// shared between the editor's highlight overlay and the search popup
    /// so the buffer isn't rescanned (and, for case-insensitive search,
    /// copied) twice per frame.
    fn search_matches(&mut self) -> Vec<(usize, usize)> {
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

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .frame(
                Frame::none()
                    .fill(self.theme.sidebar_bg)
                    .inner_margin(Margin::symmetric(10.0, 3.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(crate::i18n::line_col(self.lang, self.status_line_col.0, self.status_line_col.1))
                            .color(self.theme.fg_dim)
                            .small(),
                    );
                    ui.separator();
                    if let Some(tab) = self.tabs.get(self.active) {
                        let ext = tab.extension();
                        ui.label(RichText::new(ext.to_uppercase()).color(self.theme.fg_dim).small());
                        ui.separator();
                        ui.label(RichText::new("UTF-8").color(self.theme.fg_dim).small());
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if let Some(msg) = &self.status_message {
                            ui.label(RichText::new(msg).color(self.theme.accent).small());
                        }
                        ui.label(RichText::new(self.settings.theme.label(self.lang)).color(self.theme.fg_dim).small());
                    });
                });
            });
    }

    fn search_window(&mut self, ctx: &egui::Context) {
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

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let theme = self.theme.clone();
        let mut open = true;
        let mut theme_changed = false;

        egui::Window::new(self.t("toolbar.settings"))
            .open(&mut open)
            .resizable(true)
            .default_size([600.0, 440.0])
            .frame(
                Frame::window(&ctx.style())
                    .fill(theme.panel_bg)
                    .stroke(Stroke::new(1.0_f32, theme.border)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(170.0);
                        for page in SettingsPage::ALL {
                            if ui.selectable_label(self.settings_page == page, page.label(self.lang)).clicked() {
                                self.settings_page = page;
                            }
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| match self.settings_page {
                        SettingsPage::Appearance => {
                            ui.heading(self.t("settings.theme"));
                            ui.add_space(6.0);
                            for kind in ThemeKind::ALL {
                                if ui
                                    .radio_value(&mut self.settings.theme, kind, kind.label(self.lang))
                                    .clicked()
                                {
                                    theme_changed = true;
                                }
                            }
                        }
                        SettingsPage::Font => {
                            ui.heading(self.t("settings.font"));
                            ui.add_space(6.0);
                            let current = self.settings.font_family.clone().unwrap_or_else(|| self.t("settings.builtin_mono").to_string());
                            let (fonts_ready, font_names) = self
                                .system_fonts
                                .lock()
                                .map(|s| (s.is_ready(), s.names()))
                                .unwrap_or((false, Vec::new()));
                            egui::ComboBox::from_label(self.t("settings.font_family"))
                                .selected_text(current)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(self.settings.font_family.is_none(), self.t("settings.builtin_mono")).clicked() {
                                        self.settings.font_family = None;
                                        theme_changed = true;
                                    }
                                    for name in font_names {
                                        let selected = self.settings.font_family.as_deref() == Some(name.as_str());
                                        if ui.selectable_label(selected, &name).clicked() {
                                            self.settings.font_family = Some(name);
                                            theme_changed = true;
                                        }
                                    }
                                });
                            if !fonts_ready {
                                ui.add_space(4.0);
                                ui.weak(self.t("settings.scanning_fonts"));
                            }
                            ui.add_space(8.0);
                            let font_size_label = self.t("settings.font_size");
                            if ui.add(egui::Slider::new(&mut self.settings.font_size, 8.0..=36.0).text(font_size_label)).changed() {
                                self.settings.save();
                            }
                        }
                        SettingsPage::Editor => {
                            ui.heading(self.t("settings.editor"));
                            ui.add_space(6.0);
                            let line_numbers_label = self.t("settings.line_numbers");
                            ui.checkbox(&mut self.settings.show_line_numbers, line_numbers_label);
                            let syntax_label = self.t("settings.syntax_highlight");
                            ui.checkbox(&mut self.settings.syntax_highlighting, syntax_label);
                            let autoclose_label = self.t("settings.autoclose");
                            ui.checkbox(&mut self.settings.auto_close_brackets, autoclose_label);
                            let word_wrap_label = self.t("settings.word_wrap");
                            ui.checkbox(&mut self.settings.word_wrap, word_wrap_label);
                            let show_tree_label = self.t("settings.show_tree");
                            ui.checkbox(&mut self.settings.show_sidebar, show_tree_label);
                            ui.add_space(8.0);
                            let tab_width_label = self.t("settings.tab_width");
                            ui.add(egui::Slider::new(&mut self.settings.tab_width, 1..=8).text(tab_width_label));
                        }
                        SettingsPage::Advanced => {
                            ui.heading(self.t("settings.language"));
                            ui.add_space(6.0);
                            let system_lang = crate::i18n::Lang::detect_system();
                            egui::ComboBox::from_id_source("lang_combo")
                                .selected_text(match self.settings.lang {
                                    None => self.t("settings.language_system"),
                                    Some(l) => l.label(),
                                })
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(self.settings.lang.is_none(), self.t("settings.language_system")).clicked() {
                                        self.settings.lang = None;
                                        self.lang = system_lang;
                                        self.settings.save();
                                    }
                                    for l in [crate::i18n::Lang::Ru, crate::i18n::Lang::En] {
                                        if ui.selectable_label(self.settings.lang == Some(l), l.label()).clicked() {
                                            self.settings.lang = Some(l);
                                            self.lang = l;
                                            self.settings.save();
                                        }
                                    }
                                });
                            ui.add_space(16.0);

                            ui.heading(self.t("settings.css_theme"));
                            ui.label(self.t("settings.css_theme_desc"));
                            ui.add_space(6.0);
                            let mut path_str = self.settings.custom_css_path.clone().unwrap_or_default();
                            ui.horizontal(|ui| {
                                if ui.text_edit_singleline(&mut path_str).changed() {
                                    self.settings.custom_css_path = if path_str.is_empty() { None } else { Some(path_str.clone()) };
                                }
                                if ui.button(self.t("settings.browse")).clicked() {
                                    if let Some(p) = rfd::FileDialog::new().add_filter("CSS", &["css"]).pick_file() {
                                        self.settings.custom_css_path = Some(p.to_string_lossy().to_string());
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button(self.t("settings.create_example_css")).clicked() {
                                    if let Some(default_path) = Settings::default_css_path() {
                                        if let Some(dir) = default_path.parent() {
                                            let _ = std::fs::create_dir_all(dir);
                                        }
                                        if std::fs::write(&default_path, custom_css::EXAMPLE_CSS).is_ok() {
                                            self.settings.custom_css_path = Some(default_path.to_string_lossy().to_string());
                                        }
                                    }
                                }
                                if ui.button(self.t("settings.apply_as_theme")).clicked() {
                                    self.settings.theme = ThemeKind::Custom;
                                    theme_changed = true;
                                }
                                if ui.button(self.t("settings.reread_css")).clicked() {
                                    theme_changed = true;
                                }
                            });
                            if let Some(err) = &self.css_status {
                                ui.colored_label(theme.error, err);
                            }
                        }
                    });
                });
            });

        self.show_settings = open;
        if theme_changed {
            self.apply_theme(ctx);
            self.settings.save();
        }
    }

    fn close_confirm_modal(&mut self, ctx: &egui::Context) {
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

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
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

        self.title_bar(ctx);
        self.status_bar(ctx);
        self.activity_bar(ctx);
        self.sidebar(ctx);
        self.tab_bar(ctx);
        self.editor(ctx);
        self.search_window(ctx);
        self.settings_window(ctx);
        self.close_confirm_modal(ctx);
    }
}

// -------------------------------------------------------------- free fns

/// Registers the embedded icon fonts — Seti (file-type icons, see
/// `file_icons.rs`) and Codicons (UI icons, see `codicons.rs`) — each under
/// its own dedicated font family, without touching the regular text
/// fallback chains: they're only ever requested explicitly. Wraps whichever
/// `FontDefinitions` `install_font` is about to hand to `ctx.set_fonts`, so
/// the icon fonts survive switching the editor's own font.
fn with_icon_fonts(mut defs: egui::FontDefinitions) -> egui::FontDefinitions {
    defs.font_data.insert(
        crate::file_icons::FONT_NAME.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/seti-icons.ttf")),
    );
    defs.families
        .insert(crate::file_icons::font_family(), vec![crate::file_icons::FONT_NAME.to_owned()]);
    defs.font_data.insert(
        crate::codicons::FONT_NAME.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/codicon.ttf")),
    );
    defs.families
        .insert(crate::codicons::font_family(), vec![crate::codicons::FONT_NAME.to_owned()]);
    defs
}

fn window_button(ui: &mut egui::Ui, symbol: &str, hover_bg: Color32, fg: Color32) -> egui::Response {
    let size = egui::vec2(44.0, 34.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, hover_bg);
    }
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, symbol, egui::FontId::proportional(13.0), fg);
    response
}

/// One icon in the Activity Bar: a plain glyph, brighter + a left accent
/// bar when its panel is the active one (VS Code's own convention for
/// showing which sidebar view is open).
fn activity_icon(ui: &mut egui::Ui, theme: &Theme, symbol: &str, active: bool, tooltip: &str) -> egui::Response {
    let size = egui::vec2(46.0, 40.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if active {
        let bar = egui::Rect::from_min_size(rect.min, egui::vec2(2.0, rect.height()));
        ui.painter().rect_filled(bar, 0.0, theme.accent);
    }
    let color = if active || response.hovered() { theme.fg } else { theme.fg_dim };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, symbol, crate::codicons::font_id(22.0), color);
    response.on_hover_text(tooltip)
}

fn line_number_gutter(ui: &mut egui::Ui, theme: &Theme, line_count: usize, font_id: egui::FontId) {
    let digits = line_count.to_string().len().max(2);
    let char_w = ui.fonts(|f| f.glyph_width(&font_id, '0'));
    let width = char_w * digits as f32 + 16.0;

    ui.vertical(|ui| {
        ui.set_width(width);
        ui.add_space(2.0);
        let mut text = String::new();
        for n in 1..=line_count {
            text.push_str(&format!("{n:>width$}\n", width = digits));
        }
        ui.add(
            egui::Label::new(RichText::new(text).font(font_id).color(theme.line_number))
                .selectable(false),
        );
    });
}

fn line_of(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset.min(text.len())].matches('\n').count()
}

fn line_col_of(text: &str, char_offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, c) in text.chars().enumerate() {
        if i >= char_offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Converts a `CCursor`-style char index (as used by egui's `TextEdit`
/// cursor/selection API) into a byte offset usable with `String`'s own
/// (byte-indexed) slicing and `replace_range`.
fn char_index_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}

/// Sorted `(start, end)` byte range of the current selection, from a
/// `TextEdit`'s `cursor_range` output. `start == end` when there's no
/// selection, just a caret position — still useful as the insertion point
/// for Paste.
fn selection_byte_range(text: &str, cursor_range: &egui::text::CursorRange) -> (usize, usize) {
    let a = cursor_range.primary.ccursor.index;
    let b = cursor_range.secondary.ccursor.index;
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    (char_index_to_byte(text, lo), char_index_to_byte(text, hi))
}
