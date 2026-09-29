//! Global keyboard shortcuts and edge-drag window resizing.

use super::*;

impl EditApp {
    pub(super) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};

        // Ctrl+W is "delete previous word" in every shell; while the terminal
        // input has focus it must not close the editor tab underneath it.
        let terminal_focused = self.show_terminal
            && self
                .terminals
                .get(self.active_terminal)
                .is_some_and(|t| ctx.memory(|m| m.has_focus(t.input_id())));

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
            if !terminal_focused && i.consume_key(Modifiers::CTRL, Key::W) {
                self.request_close_tab(self.active);
            }
            if i.consume_key(Modifiers::CTRL, Key::F) {
                self.try_open_search();
            }
            if i.consume_key(Modifiers::CTRL, Key::Comma) {
                self.open_settings_tab();
            }
            if i.consume_key(Modifiers::CTRL, Key::Backtick) {
                self.toggle_terminal();
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
    pub(super) fn handle_window_resize(&mut self, ctx: &egui::Context) {
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
}
