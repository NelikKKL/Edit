//! The central editor area: text editor, image preview and the large-file viewer.

use super::*;

impl EditApp {
    pub(super) fn editor(&mut self, ctx: &egui::Context) {
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

                if self.tabs[idx].is_settings {
                    self.settings_view(ui, ctx, idx);
                    return;
                }

                if self.tabs[idx].is_image() {
                    self.image_view(ui, ctx, idx);
                    return;
                }

                if self.tabs[idx].large {
                    self.large_file_view(ui, idx);
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

                            // Keep the selection visible while the context menu is (or is
                            // about to be) open: re-apply it to the widget's state and
                            // take focus back before the text is drawn. See
                            // `context_menu_open` for why focus matters.
                            let menu_was_open = std::mem::take(&mut self.context_menu_open);
                            let secondary_down = ui.input(|i| i.pointer.button_down(egui::PointerButton::Secondary));
                            if menu_was_open || secondary_down {
                                if let Some((id, lo, hi)) = self.last_selection {
                                    let content = &self.tabs[idx].content;
                                    if id == tab_id && lo != hi && content.is_char_boundary(lo) && content.is_char_boundary(hi) {
                                        let a = content[..lo].chars().count();
                                        let b = content[..hi].chars().count();
                                        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), tab_id).unwrap_or_default();
                                        state.set_ccursor_range(Some(egui::text::CCursorRange::two(
                                            egui::text::CCursor::new(a),
                                            egui::text::CCursor::new(b),
                                        )));
                                        egui::text_edit::TextEditState::store(state, ui.ctx(), tab_id);
                                        ui.memory_mut(|m| m.request_focus(tab_id));
                                        ui.ctx().request_repaint();
                                    }
                                }
                            }

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
                            // `output.cursor_range` for *this* frame). Forget it once
                            // the selection is really gone — content edited, or the
                            // caret collapsed by something other than a right-click /
                            // the open menu — so a stale range never resurfaces.
                            if output.response.changed() {
                                self.last_selection = None;
                            }
                            let selection_now = output.cursor_range.and_then(|cursor_range| {
                                let (lo, hi) = selection_byte_range(&self.tabs[idx].content, &cursor_range);
                                (lo != hi).then_some((lo, hi))
                            });
                            if let Some((lo, hi)) = selection_now {
                                self.last_selection = Some((tab_id, lo, hi));
                            } else if !menu_was_open && !secondary_down && !output.response.secondary_clicked() {
                                self.last_selection = None;
                            }

                            // Right-click (or long-press, on touch) context menu with the
                            // usual Cut/Copy/Paste/Select all actions — `TextEdit` doesn't
                            // provide one on its own, only keyboard shortcuts.
                            let lang = self.lang;
                            output.response.context_menu(|ui| {
                                self.context_menu_open = true;
                                let content_ref = &self.tabs[idx].content;
                                let selection = self.last_selection.filter(|(id, lo, hi)| {
                                    *id == tab_id && lo != hi && content_ref.is_char_boundary(*lo) && content_ref.is_char_boundary(*hi)
                                });
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
    pub(super) fn image_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, idx: usize) {
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

    /// Read-only, virtualized viewer for files at/above `LARGE_FILE_BYTES`
    /// (see that constant's docs). The problem this replaces: even with
    /// syntax highlighting switched off, handing the *whole* file to a
    /// normal `TextEdit` still lays it out into one giant `Galley` — glyph
    /// positions for every character in the file — every single frame,
    /// which is what actually turns a 10 MB file into roughly a gigabyte of
    /// RAM. This instead only ever lays out the handful of lines actually
    /// on screen, using `ScrollArea::show_rows` for virtualization plus a
    /// small overscan (`LARGE_FILE_OVERSCAN_ROWS`) above/below so fast
    /// scrolling doesn't flash blank space, and a cached byte-offset index
    /// (`EditorTab::line_offsets`) so jumping to an arbitrary row doesn't
    /// need to rescan the file from the start. The tradeoff for staying in
    /// this fast path: these files are view/select/copy-only here (no
    /// in-place editing, no word wrap, no find) — see the notice line this
    /// draws at the top of the tab, and the chat reply this shipped with
    /// for the reasoning.
    pub(super) fn large_file_view(&mut self, ui: &mut egui::Ui, idx: usize) {
        let theme = self.theme.clone();
        let lang = self.lang;

        self.tabs[idx].ensure_line_offsets();
        let total_rows = self.tabs[idx].line_offsets.as_ref().map_or(1, |o| o.len().max(1));
        let byte_len = self.tabs[idx].content.len();

        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new(crate::i18n::large_file_notice(lang, byte_len)).color(theme.fg_dim).small());
        });
        ui.add_space(4.0);
        ui.separator();

        let font_id = egui::FontId::monospace(self.settings.font_size);
        let row_height = ui.fonts(|f| f.row_height(&font_id));
        let gutter_w = (total_rows.to_string().len() as f32) * (self.settings.font_size * 0.62) + 12.0;

        egui::ScrollArea::both().auto_shrink([false, false]).show_rows(ui, row_height, total_rows, |ui, row_range| {
            let start = row_range.start.saturating_sub(LARGE_FILE_OVERSCAN_ROWS);
            let end = (row_range.end + LARGE_FILE_OVERSCAN_ROWS).min(total_rows);
            for row in start..end {
                let Some((s, e)) = self.tabs[idx].line_range(row) else { continue };
                let raw = &self.tabs[idx].content[s..e];
                let raw = raw.strip_suffix('\n').unwrap_or(raw);
                let raw = raw.strip_suffix('\r').unwrap_or(raw);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.add_sized(
                        [gutter_w, row_height],
                        egui::Label::new(RichText::new((row + 1).to_string()).font(font_id.clone()).color(theme.line_number)),
                    );
                    if raw.chars().count() > MAX_LARGE_LINE_CHARS {
                        let mut shown: String = raw.chars().take(MAX_LARGE_LINE_CHARS).collect();
                        shown.push('…');
                        ui.label(RichText::new(shown).font(font_id.clone()).color(theme.fg));
                    } else {
                        ui.label(RichText::new(raw).font(font_id.clone()).color(theme.fg));
                    }
                });
            }
        });
    }
}

pub(super) fn line_number_gutter(ui: &mut egui::Ui, theme: &Theme, line_count: usize, font_id: egui::FontId) {
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

pub(super) fn line_of(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset.min(text.len())].matches('\n').count()
}

pub(super) fn line_col_of(text: &str, char_offset: usize) -> (usize, usize) {
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
pub(super) fn char_index_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}

/// Sorted `(start, end)` byte range of the current selection, from a
/// `TextEdit`'s `cursor_range` output. `start == end` when there's no
/// selection, just a caret position — still useful as the insertion point
/// for Paste.
pub(super) fn selection_byte_range(text: &str, cursor_range: &egui::text::CursorRange) -> (usize, usize) {
    let a = cursor_range.primary.ccursor.index;
    let b = cursor_range.secondary.ccursor.index;
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    (char_index_to_byte(text, lo), char_index_to_byte(text, hi))
}
