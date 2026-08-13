use crate::autoclose;
use crate::custom_css;
use crate::editor_tab::EditorTab;
use crate::file_tree::{FileTree, TreeAction};
use crate::fonts::SystemFonts;
use crate::search::SearchState;
use crate::settings::Settings;
use crate::syntax_highlight::Highlighter;
use crate::theme::{Theme, ThemeKind};
use egui::{Align, Align2, Color32, Frame, Margin, RichText, Sense, Stroke};
use std::path::{Path, PathBuf};

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
    fn label(&self) -> &'static str {
        match self {
            SettingsPage::Appearance => "Внешний вид",
            SettingsPage::Font => "Шрифт",
            SettingsPage::Editor => "Редактор",
            SettingsPage::Advanced => "Дополнительно",
        }
    }
}

pub struct EditApp {
    settings: Settings,
    theme: Theme,
    tabs: Vec<EditorTab>,
    active: usize,
    untitled_counter: usize,

    file_tree: FileTree,
    search: SearchState,
    highlighter: Highlighter,
    system_fonts: SystemFonts,

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
    pub fn new(cc: &eframe::CreationContext<'_>, initial_file: Option<String>) -> Self {
        let settings = Settings::load();
        let system_fonts = SystemFonts::scan();

        let mut app = Self {
            theme: Theme::dark(),
            tabs: Vec::new(),
            active: 0,
            untitled_counter: 1,
            file_tree: FileTree::new(),
            search: SearchState::default(),
            highlighter: Highlighter::new(),
            system_fonts,
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
        let tab = EditorTab::untitled(self.untitled_counter);
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
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                // Replace a single, untouched "Untitled" tab if that's all we have.
                if self.tabs.len() == 1 && self.tabs[0].path.is_none() && !self.tabs[0].dirty && self.tabs[0].content.is_empty() {
                    self.tabs[0] = EditorTab::from_path(path, content);
                    self.active = 0;
                } else {
                    self.tabs.push(EditorTab::from_path(path, content));
                    self.active = self.tabs.len() - 1;
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Не удалось открыть файл: {e}"));
            }
        }
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().set_title("Открыть файл").pick_file() {
            self.open_path(path);
        }
    }

    fn open_folder_dialog(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().set_title("Открыть папку").pick_folder() {
            self.file_tree.set_root(dir.clone());
            self.settings.show_sidebar = true;
            self.settings.last_folder = Some(dir.to_string_lossy().to_string());
            self.settings.save();
        }
    }

    fn save_tab(&mut self, idx: usize) {
        let Some(tab) = self.tabs.get_mut(idx) else { return };
        if tab.path.is_some() {
            if let Err(e) = tab.save() {
                self.status_message = Some(format!("Ошибка сохранения: {e}"));
            } else {
                self.status_message = Some("Сохранено".to_string());
            }
        } else {
            self.save_tab_as(idx);
        }
    }

    fn save_tab_as(&mut self, idx: usize) {
        if let Some(path) = rfd::FileDialog::new().set_title("Сохранить как").save_file() {
            if let Some(tab) = self.tabs.get_mut(idx) {
                if let Err(e) = tab.save_as(path) {
                    self.status_message = Some(format!("Ошибка сохранения: {e}"));
                } else {
                    self.status_message = Some("Сохранено".to_string());
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
            self.css_status = Some("Файл темы не выбран — используется тёмная тема по умолчанию.".into());
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
                self.css_status = Some(format!("Не удалось прочитать {path}: {e}"));
                Theme::dark()
            }
        }
    }

    fn install_font(&mut self, ctx: &egui::Context) {
        let mut defs = egui::FontDefinitions::default();
        if let Some(family) = self.settings.font_family.clone() {
            if let Some(bytes) = self.system_fonts.load_bytes(&family) {
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
                self.status_message = Some(format!("Шрифт «{family}» не найден"));
            }
        }
        ctx.set_fonts(defs);
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

    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar")
            .exact_height(42.0)
            .frame(
                Frame::none()
                    .fill(self.theme.panel_bg)
                    .inner_margin(Margin::symmetric(10.0, 5.0))
                    .stroke(Stroke::new(1.0_f32, self.theme.border)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.button("Открыть").clicked() {
                        self.open_file_dialog();
                    }
                    if ui.button("Папка").clicked() {
                        self.open_folder_dialog();
                    }
                    if ui.button("Сохранить").clicked() {
                        self.save_tab(self.active);
                    }
                    if ui.button("Сохранить как").clicked() {
                        self.save_tab_as(self.active);
                    }
                    ui.separator();
                    if ui.button("Поиск").clicked() {
                        self.search.open();
                    }
                    if ui
                        .selectable_label(self.settings.show_sidebar, "Дерево")
                        .clicked()
                    {
                        self.settings.show_sidebar = !self.settings.show_sidebar;
                        self.settings.save();
                    }
                    ui.separator();
                    if ui.button("+").on_hover_text("Новая вкладка (Ctrl+N)").clicked() {
                        self.new_tab();
                    }

                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Настройки").clicked() {
                            self.show_settings = true;
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
                ui.label(RichText::new("ФАЙЛЫ").color(theme.fg_dim).small());
                ui.add_space(4.0);
                match self.file_tree.ui(ui, &theme) {
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
                    ui.centered_and_justified(|ui| ui.weak("Нет открытых файлов"));
                    return;
                }
                let idx = self.active.min(self.tabs.len() - 1);
                self.active = idx;

                // ---- gather everything the layouter closure needs, as owned
                // values, BEFORE taking a mutable borrow of the tab's content.
                let font_size = self.settings.font_size;
                let syntax_enabled = self.settings.syntax_highlighting;
                let word_wrap = self.settings.word_wrap;
                let dark = theme.is_dark();
                let default_color = theme.fg;
                let match_bg = theme.accent.gamma_multiply(0.30);
                let current_match_bg = theme.accent.gamma_multiply(0.65);
                let extension = self.tabs[idx].extension();
                let tab_id = self.tabs[idx].id;

                let search_active = self.search.open && !self.search.query.is_empty();
                let search_ranges = if search_active {
                    self.search.find_all(&self.tabs[idx].content)
                } else {
                    Vec::new()
                };
                let current_match = if search_ranges.is_empty() {
                    None
                } else {
                    Some(self.search.current_match.min(search_ranges.len() - 1))
                };

                let before_text = self.tabs[idx].content.clone();
                let line_count = before_text.lines().count().max(1);
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

                            let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
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
                                job.wrap.max_width = if word_wrap { wrap_width } else { f32::INFINITY };
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

                            if output.response.changed() {
                                let after_text = self.tabs[idx].content.clone();
                                if after_text != before_text {
                                    self.tabs[idx].dirty = true;
                                    if self.settings.auto_close_brackets {
                                        if let Some(fixed) = autoclose::process_edit(&before_text, &after_text) {
                                            self.tabs[idx].content = fixed;
                                        }
                                    }
                                }
                            }

                            // Track roughly where the cursor is for the status bar.
                            if let Some(cursor_range) = output.cursor_range {
                                let ccursor = cursor_range.primary.ccursor.index;
                                let (line, col) = line_col_of(&self.tabs[idx].content, ccursor);
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

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .frame(
                Frame::none()
                    .fill(self.theme.panel_bg)
                    .inner_margin(Margin::symmetric(10.0, 3.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Строка {}, Столбец {}", self.status_line_col.0, self.status_line_col.1))
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
                        ui.label(RichText::new(self.settings.theme.label()).color(self.theme.fg_dim).small());
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

        let matches = self
            .tabs
            .get(self.active)
            .map(|t| self.search.find_all(&t.content))
            .unwrap_or_default();

        egui::Window::new("Поиск")
            .open(&mut still_open)
            .collapsible(false)
            .resizable(true)
            .default_size([300.0, 110.0])
            .anchor(Align2::RIGHT_TOP, egui::vec2(-16.0, 78.0))
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
                ui.checkbox(&mut self.search.match_case, "Учитывать регистр");

                ui.horizontal(|ui| {
                    if matches.is_empty() {
                        ui.weak(if self.search.query.is_empty() { "" } else { "Нет совпадений" });
                    } else {
                        ui.label(format!("{} / {}", self.search.current_match + 1, matches.len()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Закрыть").clicked() {
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

        egui::Window::new("Настройки")
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
                            if ui.selectable_label(self.settings_page == page, page.label()).clicked() {
                                self.settings_page = page;
                            }
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| match self.settings_page {
                        SettingsPage::Appearance => {
                            ui.heading("Тема");
                            ui.add_space(6.0);
                            for kind in ThemeKind::ALL {
                                if ui
                                    .radio_value(&mut self.settings.theme, kind, kind.label())
                                    .clicked()
                                {
                                    theme_changed = true;
                                }
                            }
                        }
                        SettingsPage::Font => {
                            ui.heading("Шрифт");
                            ui.add_space(6.0);
                            let current = self.settings.font_family.clone().unwrap_or_else(|| "Встроенный (моно)".to_string());
                            egui::ComboBox::from_label("Семейство шрифта")
                                .selected_text(current)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(self.settings.font_family.is_none(), "Встроенный (моно)").clicked() {
                                        self.settings.font_family = None;
                                        theme_changed = true;
                                    }
                                    for name in self.system_fonts.names() {
                                        let selected = self.settings.font_family.as_deref() == Some(name.as_str());
                                        if ui.selectable_label(selected, &name).clicked() {
                                            self.settings.font_family = Some(name);
                                            theme_changed = true;
                                        }
                                    }
                                });
                            ui.add_space(8.0);
                            if ui.add(egui::Slider::new(&mut self.settings.font_size, 8.0..=36.0).text("Размер шрифта")).changed() {
                                self.settings.save();
                            }
                        }
                        SettingsPage::Editor => {
                            ui.heading("Редактор");
                            ui.add_space(6.0);
                            ui.checkbox(&mut self.settings.show_line_numbers, "Нумерация строк");
                            ui.checkbox(&mut self.settings.syntax_highlighting, "Подсветка синтаксиса");
                            ui.checkbox(&mut self.settings.auto_close_brackets, "Автозакрытие скобок и кавычек");
                            ui.checkbox(&mut self.settings.word_wrap, "Перенос строк");
                            ui.checkbox(&mut self.settings.show_sidebar, "Показывать дерево файлов");
                            ui.add_space(8.0);
                            ui.add(egui::Slider::new(&mut self.settings.tab_width, 1..=8).text("Ширина табуляции"));
                        }
                        SettingsPage::Advanced => {
                            ui.heading("CSS-тема");
                            ui.label("Свой файл темы (переменные --bg, --fg, --accent, --font-family, ...).");
                            ui.add_space(6.0);
                            let mut path_str = self.settings.custom_css_path.clone().unwrap_or_default();
                            ui.horizontal(|ui| {
                                if ui.text_edit_singleline(&mut path_str).changed() {
                                    self.settings.custom_css_path = if path_str.is_empty() { None } else { Some(path_str.clone()) };
                                }
                                if ui.button("Обзор…").clicked() {
                                    if let Some(p) = rfd::FileDialog::new().add_filter("CSS", &["css"]).pick_file() {
                                        self.settings.custom_css_path = Some(p.to_string_lossy().to_string());
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Создать пример theme.css").clicked() {
                                    if let Some(default_path) = Settings::default_css_path() {
                                        if let Some(dir) = default_path.parent() {
                                            let _ = std::fs::create_dir_all(dir);
                                        }
                                        if std::fs::write(&default_path, custom_css::EXAMPLE_CSS).is_ok() {
                                            self.settings.custom_css_path = Some(default_path.to_string_lossy().to_string());
                                        }
                                    }
                                }
                                if ui.button("Применить как тему").clicked() {
                                    self.settings.theme = ThemeKind::Custom;
                                    theme_changed = true;
                                }
                                if ui.button("Перечитать CSS").clicked() {
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
        egui::Window::new("Несохранённые изменения")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(Frame::window(&ctx.style()).fill(theme.panel_bg).stroke(Stroke::new(1.0_f32, theme.border)))
            .show(ctx, |ui| {
                ui.label(format!("Сохранить изменения в «{title}»?"));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Сохранить").clicked() {
                        self.save_tab(idx);
                        self.close_tab_now(idx);
                        self.close_confirm = None;
                    }
                    if ui.button("Не сохранять").clicked() {
                        self.close_tab_now(idx);
                        self.close_confirm = None;
                    }
                    if ui.button("Отмена").clicked() {
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
        self.handle_shortcuts(ctx);
        self.handle_window_resize(ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        self.handle_dropped_files(ctx);

        self.title_bar(ctx);
        self.toolbar(ctx);
        self.tab_bar(ctx);
        self.status_bar(ctx);
        self.sidebar(ctx);
        self.editor(ctx);
        self.search_window(ctx);
        self.settings_window(ctx);
        self.close_confirm_modal(ctx);
    }
}

// -------------------------------------------------------------- free fns

fn window_button(ui: &mut egui::Ui, symbol: &str, hover_bg: Color32, fg: Color32) -> egui::Response {
    let size = egui::vec2(44.0, 34.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, hover_bg);
    }
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, symbol, egui::FontId::proportional(13.0), fg);
    response
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
