use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme as SynTheme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    fn find_syntax(&self, extension: &str) -> &SyntaxReference {
        self.syntax_set
            .find_syntax_by_extension(extension)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    /// Pick a syntect theme that roughly matches whether the app is in a
    /// light or dark UI theme, so highlighted colors stay readable.
    fn pick_theme(&self, dark: bool) -> &SynTheme {
        let key = if dark {
            "base16-ocean.dark"
        } else {
            "InspiredGitHub"
        };
        self.theme_set
            .themes
            .get(key)
            .unwrap_or_else(|| self.theme_set.themes.values().next().unwrap())
    }

    /// Build a colored `LayoutJob` for one string of source text, optionally
    /// with syntax highlighting and/or search-match background highlights
    /// overlaid on top (used for both the code coloring and the "find"
    /// popup's highlighted matches, since `TextEdit` only accepts a single
    /// layouter callback).
    #[allow(clippy::too_many_arguments)]
    pub fn build_job(
        &self,
        text: &str,
        extension: &str,
        font_id: FontId,
        dark: bool,
        default_color: Color32,
        syntax_enabled: bool,
        search_ranges: &[(usize, usize)],
        current_match: Option<usize>,
        match_bg: Color32,
        current_match_bg: Color32,
    ) -> LayoutJob {
        let mut job = LayoutJob::default();
        job.wrap.max_width = f32::INFINITY;

        let search = SearchOverlay {
            ranges: search_ranges,
            current: current_match,
            match_bg,
            current_match_bg,
        };

        if syntax_enabled {
            let syntax = self.find_syntax(extension);
            let theme = self.pick_theme(dark);
            let mut h = HighlightLines::new(syntax, theme);
            let mut offset = 0usize;
            for line in split_keep_newline(text) {
                let ranges = h.highlight_line(line, &self.syntax_set).unwrap_or_default();
                for (style, piece) in ranges {
                    let mut color = Color32::from_rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    );
                    if color == Color32::BLACK && !dark {
                        color = default_color;
                    }
                    append_with_search(&mut job, piece, offset, &font_id, color, &search);
                    offset += piece.len();
                }
            }
        } else {
            append_with_search(&mut job, text, 0, &font_id, default_color, &search);
        }

        job
    }
}

struct SearchOverlay<'a> {
    ranges: &'a [(usize, usize)],
    current: Option<usize>,
    match_bg: Color32,
    current_match_bg: Color32,
}

/// Appends `piece` (which starts at byte `offset` within the full text) to
/// `job`, splitting it further wherever a search-match range begins or ends
/// so those sub-spans can get a highlighted background.
fn append_with_search(
    job: &mut LayoutJob,
    piece: &str,
    offset: usize,
    font_id: &FontId,
    color: Color32,
    search: &SearchOverlay,
) {
    let piece_start = offset;
    let piece_end = offset + piece.len();

    if search.ranges.is_empty() {
        job.append(
            piece,
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color,
                ..Default::default()
            },
        );
        return;
    }

    // Build cut points (relative to piece) at every match boundary that
    // falls inside this piece.
    let mut cuts: Vec<usize> = vec![0, piece.len()];
    for &(s, e) in search.ranges {
        if s < piece_end && e > piece_start {
            let rel_s = s.saturating_sub(piece_start).min(piece.len());
            let rel_e = e.saturating_sub(piece_start).min(piece.len());
            cuts.push(rel_s);
            cuts.push(rel_e);
        }
    }
    cuts.sort_unstable();
    cuts.dedup();

    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a >= b || b > piece.len() {
            continue;
        }
        let sub = &piece[a..b];
        let abs_mid = piece_start + a;
        let bg = search
            .ranges
            .iter()
            .enumerate()
            .find(|(_, &(s, e))| abs_mid >= s && abs_mid < e)
            .map(|(idx, _)| {
                if Some(idx) == search.current {
                    search.current_match_bg
                } else {
                    search.match_bg
                }
            });
        job.append(
            sub,
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color,
                background: bg.unwrap_or(Color32::TRANSPARENT),
                ..Default::default()
            },
        );
    }
}

/// syntect wants each "line" fed to it to include its trailing `\n` (that's
/// what `load_defaults_newlines` expects), so split manually instead of
/// using `str::lines()` which strips it.
fn split_keep_newline(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if c == '\n' {
            out.push(&text[start..=i]);
            start = i + 1;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}
