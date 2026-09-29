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

    /// Byte-offset ranges of every match of `query` in `text` — always
    /// offsets into `text` itself, on char boundaries.
    ///
    /// Case-insensitive matching lowercases both sides, but lowercasing can
    /// change byte lengths (`İ` becomes `i` + a combining dot, ...), so
    /// offsets found in the lowercased haystack can't be used on the
    /// original directly. Each original char records where its lowercase
    /// form starts, and matches are mapped back through that table.
    pub fn find_all(&self, text: &str) -> Vec<(usize, usize)> {
        if self.query.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();

        if self.match_case {
            let needle = self.query.as_str();
            let mut start = 0;
            while let Some(pos) = text[start..].find(needle) {
                let s = start + pos;
                let e = s + needle.len();
                out.push((s, e));
                start = e;
            }
            return out;
        }

        let needle = self.query.to_lowercase();
        if needle.is_empty() {
            return out;
        }
        // (start in lowercased haystack, start in text, end in text) per char
        let mut haystack = String::with_capacity(text.len());
        let mut table: Vec<(usize, usize, usize)> = Vec::with_capacity(text.len());
        for (i, c) in text.char_indices() {
            table.push((haystack.len(), i, i + c.len_utf8()));
            haystack.extend(c.to_lowercase());
        }

        let mut start = 0;
        let mut last_end = 0;
        while let Some(pos) = haystack[start..].find(&needle) {
            let ls = start + pos;
            let le = ls + needle.len();
            start = le; // needle is non-empty, so this always advances

            // char whose lowercase output contains byte `ls` / byte `le - 1`
            let first = table.partition_point(|&(lo, _, _)| lo <= ls).saturating_sub(1);
            let last = table.partition_point(|&(lo, _, _)| lo <= le - 1).saturating_sub(1);
            let (Some(a), Some(b)) = (table.get(first), table.get(last)) else { continue };
            let (s, e) = (a.1, b.2);
            if s >= last_end {
                out.push((s, e));
                last_end = e;
            }
        }
        out
    }
}
