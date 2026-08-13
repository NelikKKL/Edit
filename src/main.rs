// On Windows release builds, run as a GUI app (no console window pops up).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autoclose;
mod custom_css;
mod editor_tab;
mod file_tree;
mod fonts;
mod search;
mod settings;
mod syntax_highlight;
mod theme;

fn main() -> eframe::Result<()> {
    // If launched with a file path (e.g. via "Open with" / file association),
    // open it on startup.
    let initial_file = std::env::args().nth(1);

    let icon = load_icon();

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([640.0, 420.0])
        .with_decorations(false) // we draw our own themeable title bar
        .with_icon(icon)
        .with_title("edit");

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "edit",
        native_options,
        Box::new(|cc| Ok(Box::new(app::EditApp::new(cc, initial_file)))),
    )
}

fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(bytes)
        .expect("built-in icon.png is invalid")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}
