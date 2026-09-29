# edit

**Русская документация:** [README на русском](RU.md)

**edit** is a fast text editor written in Rust that can also be used as a code editor.

It supports light and dark themes, custom CSS themes, a file tree, tabs, syntax highlighting, search, and automatic bracket closing.

## Features

- Light and dark themes
- Custom CSS themes
- File tree
- Multiple tabs
- Syntax highlighting
- Search result highlighting
- File search
- Line numbers
- Automatic bracket closing
- System and custom fonts
- Keyboard shortcuts
- File and folder opening
- Windows and Linux builds

## Project Structure

```text
edit/
├── Cargo.toml                         # Cargo manifest, dependencies, and build settings
├── build.rs                           # embeds Windows resources and the icon into edit.exe
├── LICENSE                            # project license
├── README.md                          # Russian documentation
├── RU.md                        # English documentation
│
├── assets/
│   ├── icon.ico                       # Windows icon, multiple resolutions
│   ├── icon.png                       # main application icon
│   └── hicolor/                       # Linux icons for different sizes
│       ├── 16x16/apps/edit.png
│       ├── 24x24/apps/edit.png
│       ├── 32x32/apps/edit.png
│       ├── 48x48/apps/edit.png
│       ├── 64x64/apps/edit.png
│       ├── 128x128/apps/edit.png
│       ├── 256x256/apps/edit.png
│       └── 512x512/apps/edit.png
│
├── img/                                # images used in the documentation
│   ├── image.png
│   ├── image(1).png
│   ├── image(2).png
│   └── image(3).png
│
├── src/
│   ├── main.rs                         # application entry point
│   ├── app.rs                          # main UI: title bar, panels, tabs, and editor
│   ├── autoclose.rs                    # automatic bracket closing
│   ├── custom_css.rs                   # loading and parsing custom CSS theme variables
│   ├── editor_tab.rs                   # state of an individual tab and opened file
│   ├── file_tree.rs                    # file tree in the sidebar
│   ├── fonts.rs                        # system font discovery and handling
│   ├── i18n.rs                          # interface localization
│   ├── search.rs                       # search state and logic
│   ├── settings.rs                     # application settings and persistence
│   ├── syntax_highlight.rs              # syntax and search-result highlighting
│   └── theme.rs                         # built-in themes and theme settings
│
├── installer/
│   └── setup.iss                       # Inno Setup: Windows installer and file associations
│
├── linux/
│   └── edit.desktop                    # desktop entry for Linux/GNOME and other environments
│
└── .github/
    └── workflows/
        └── build.yml                   # CI: automatic release artifact builds
```

### Icons

All platform-specific icon files use the same new application icon:

- `assets/icon.png` — main PNG
- `assets/icon.ico` — Windows icon
- `assets/hicolor/*/apps/edit.png` — Linux icon sizes

When changing the application icon, update the complete set so Windows, Linux, and packaging tools use the same artwork.

## Preview

![Search](img/image.png)

![Settings](img/image(1).png)

![Interface in a different theme](img/image(2).png)

![Different font](img/image(3).png)

## Custom Theme (CSS)

Open **Settings → Advanced → "Create sample theme.css"**, or create the file manually:

```css
:root {
  --bg: #1e1e1e;
  --panel-bg: #252526;
  --titlebar-bg: #232121;
  --titlebar-fg: #e4e4e6;
  --sidebar-bg: #212122;
  --editor-bg: #1e1e1e;
  --fg: #d4d4d6;
  --fg-dim: #8a8a8f;
  --accent: #569cd6;
  --line-number: #5a5a5e;
  --selection: #264f78;
  --border: #333335;

  --font-family: Consolas;
  --font-size: 14;
}
```

Specify the path to the file under **Settings → Advanced**, then select the **Custom (CSS)** theme.

## Hotkeys

| Action | Keys |
|---|---|
| Open File | `Ctrl+O` |
| Open Folder | `Ctrl+Shift+O` |
| Save | `Ctrl+S` |
| Save As | `Ctrl+Shift+S` |
| New Tab | `Ctrl+N` |
| Close Tab | `Ctrl+W` |
| Search | `Ctrl+F` |
| Settings | `Ctrl+,` |
| Fullscreen | `F11` |

## Building

Build the project with Cargo:

```bash
cargo build --release
```

On Windows, `build.rs` automatically embeds `assets/icon.ico` into the executable.

Linux builds use `linux/edit.desktop` and `assets/icon.png`; automated packaging is handled by the CI workflow in `.github/workflows/build.yml`.

## License

See the [LICENSE](LICENSE) file.