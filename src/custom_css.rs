use crate::theme::Theme;
use egui::Color32;
use std::collections::HashMap;
use std::path::Path;

/// `edit` lets users theme the app with a small CSS-like file made of
/// custom properties, e.g.:
///
/// ```css
/// :root {
///   --bg: #1e1e1e;
///   --fg: #d4d4d4;
///   --accent: #569cd6;
///   --sidebar-bg: #252526;
///   --titlebar-bg: #232121;
///   --editor-bg: #1e1e1e;
///   --line-number: #6e7681;
///   --selection: #264f78;
///   --border: #333333;
///   --font-family: Consolas;
///   --font-size: 14;
/// }
/// ```
///
/// This is a deliberately small subset of real CSS (custom properties only,
/// no selectors/cascade) — enough to let people restyle the whole app from
/// one readable file without shipping a full CSS engine.
pub struct ParsedCss {
    pub vars: HashMap<String, String>,
}

pub fn parse_css_file(path: &Path) -> std::io::Result<ParsedCss> {
    let text = std::fs::read_to_string(path)?;
    Ok(parse_css_str(&text))
}

pub fn parse_css_str(text: &str) -> ParsedCss {
    let mut vars = HashMap::new();
    for raw_line in text.lines() {
        // Strip /* comments */ naively (single-line safe usage is fine here).
        let line = strip_comment(raw_line).trim();
        let line = line.trim_end_matches(';').trim();
        if !line.starts_with("--") {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().trim_start_matches("--").to_string();
            let value = value.trim().trim_end_matches('}').trim().to_string();
            if !key.is_empty() && !value.is_empty() {
                vars.insert(key, value);
            }
        }
    }
    ParsedCss { vars }
}

/// Strips a same-line `/* ... */` comment. Multi-line comments aren't
/// supported since the format is meant to be one declaration per line.
fn strip_comment(line: &str) -> &str {
    if let Some(start) = line.find("/*") {
        &line[..start]
    } else {
        line
    }
}

fn parse_color(s: &str) -> Option<Color32> {
    let s = s.trim().trim_start_matches('#');
    let (r, g, b, a) = match s.len() {
        6 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            255,
        ),
        8 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            u8::from_str_radix(&s[6..8], 16).ok()?,
        ),
        3 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            (r, g, b, 255)
        }
        _ => return None,
    };
    Some(Color32::from_rgba_premultiplied(r, g, b, a))
}

/// Start from a base theme (Dark, so unset properties still look coherent)
/// and override every color the user specified in their CSS file.
pub fn theme_from_css(css: &ParsedCss) -> Theme {
    let mut theme = Theme::dark();
    macro_rules! apply {
        ($field:ident, $key:literal) => {
            if let Some(v) = css.vars.get($key).and_then(|s| parse_color(s)) {
                theme.$field = v;
            }
        };
    }
    apply!(bg, "bg");
    apply!(panel_bg, "panel-bg");
    apply!(titlebar_bg, "titlebar-bg");
    apply!(titlebar_fg, "titlebar-fg");
    apply!(sidebar_bg, "sidebar-bg");
    apply!(editor_bg, "editor-bg");
    apply!(fg, "fg");
    apply!(fg_dim, "fg-dim");
    apply!(accent, "accent");
    apply!(line_number, "line-number");
    apply!(line_number_active, "line-number-active");
    apply!(selection, "selection");
    apply!(cursor, "cursor");
    apply!(button_hover, "button-hover");
    apply!(button_active, "button-active");
    apply!(border, "border");
    apply!(error, "error");
    apply!(close_hover, "close-hover");
    theme
}

/// Pull `--font-family` / `--font-size` out of the CSS file, if present.
pub fn font_overrides_from_css(css: &ParsedCss) -> (Option<String>, Option<f32>) {
    let family = css.vars.get("font-family").cloned();
    let size = css.vars.get("font-size").and_then(|s| s.parse::<f32>().ok());
    (family, size)
}

pub const EXAMPLE_CSS: &str = r#":root {
  /* This file is loaded as the "Своя (CSS)" theme in edit's settings. */
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
  --line-number-active: #c0c0c4;
  --selection: #264f78;
  --cursor: #e4e4e6;
  --button-hover: #333335;
  --button-active: #3d3d40;
  --border: #333335;
  --error: #e06c6c;
  --close-hover: #c43b3b;

  --font-family: Consolas;
  --font-size: 14;
}
"#;
