#[derive(Default)]
pub struct SearchState {
    pub open: bool,
    pub query: String,
    pub match_case: bool,
    pub current_match: usize,
    pub focus_requested: bool,
}

impl SearchState {
    pub fn open(&mut self) {
        self.open = true;
        self.focus_requested = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
    }

    /// Byte-offset ranges of every match of `query` in `text`.
    pub fn find_all(&self, text: &str) -> Vec<(usize, usize)> {
        if self.query.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let (haystack, needle) = if self.match_case {
            (text.to_string(), self.query.clone())
        } else {
            (text.to_lowercase(), self.query.to_lowercase())
        };
        if needle.is_empty() {
            return out;
        }
        let mut start = 0;
        while let Some(pos) = haystack[start..].find(&needle) {
            let s = start + pos;
            let e = s + needle.len();
            out.push((s, e));
            start = e.max(s + 1);
            if start > haystack.len() {
                break;
            }
        }
        out
    }
}
