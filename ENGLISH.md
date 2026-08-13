# edit

A text editor written in Rust with the ability to set a dark or light theme, supports custom CSS, and can serve as a code editor.

# Features

Search.
Line numbers
Colorizes syntax (works poorly at the moment)
Supports tree view
Helps you write code (closes brackets, etc.)
You can set your own font or choose from the system fonts. There are keyboard shortcuts

## Project Structure

```
edit-editor/
├── Cargo.toml # dependencies + metadata for cargo-deb
├── build.rs # embeds .ico in edit.exe on Windows
├── assets/
│ ├── icon.ico # Windows icon (multi-resolution)
│ └── icon.png
├── img
| ├── image.png
| ├── image(1).png
| ├── image(2).png
| ├── image(3).png
├── src/
│ ├── main.rs # entry point, no console on Windows
│ ├── app.rs # entire layout: title, toolbar, tabs, editor
│ ├── theme.rs # palettes: Light / Dark / Char
│ ├── custom_css.rs # CSS variable parser for your theme
│ ├── settings.rs # settings, saved in JSON
│ ├── syntax_highlight.rs # syntax highlighting (syntect) + search highlighting
│ ├── fonts.rs # system font scanning
│ ├── file_tree.rs # file tree in the sidebar
│ ├── search.rs # search window state
│ ├── editor_tab.rs # one open tab/file
│ └── autoclose.rs # auto-close brackets (without tying to the egui cursor)
├── installer/setup.iss # Inno Setup: Windows installer + file type registration
├── linux/edit.desktop # application shortcut for Ubuntu/GNOME
└── .github/workflows/build.yml # CI: builds .exe, installer, and .deb
```
# Preview

![Search](img/image.png)
![Settings](img/image(1).png)
![Interface in a different theme](img/image(2).png)
![Different font](img/image(3).png)

## Custom Theme (CSS)

Settings → Advanced → "Create sample theme.css", or create a file
yourself:

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

Specify the path to the file in Settings → Advanced, select the "Custom (CSS)" theme.

## Hotkeys

| Action | Keys |
|---|---|
| Open File | Ctrl+O |
| Open Folder | Ctrl+Shift+O |
| Save | Ctrl+S |
| Save As | Ctrl+Shift+S |
| New Tab | Ctrl+N |
| Close Tab | Ctrl+W |
| Search | Ctrl+F |
| Settings | Ctrl+, |
| Fullscreen | F11 |
