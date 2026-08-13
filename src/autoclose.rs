/// Finds the byte offset of the single character that was inserted between
/// `before` and `after`, assuming `after` is exactly one byte longer and is
/// otherwise identical (true for a normal keystroke, false for paste/IME —
/// callers should skip those). Returns `None` if that assumption doesn't
/// hold.
fn single_insert_pos(before: &str, after: &str) -> Option<usize> {
    if after.len() != before.len() + 1 {
        return None;
    }
    let mut i = 0;
    let bb = before.as_bytes();
    let ab = after.as_bytes();
    while i < bb.len() && bb[i] == ab[i] {
        i += 1;
    }
    // Everything after position i in `after` (skipping the inserted byte)
    // must line up with the remainder of `before` for this to be a clean,
    // single-character insertion.
    if ab[i + 1..] == bb[i..] {
        Some(i)
    } else {
        None
    }
}

/// Given the editor text just before and just after one keystroke, decide
/// whether `edit`'s bracket/quote assistant should adjust what gets typed:
///
/// - Typing an opener `( [ { " ' \`` inserts its matching closer right after
///   the cursor.
/// - Typing a closer that's immediately followed by that same character
///   (because we auto-inserted it earlier) just moves past it instead of
///   duplicating it.
///
/// Returns the replacement text for the buffer, or `None` to leave `after`
/// untouched. This is purely a string transform — it deliberately never
/// touches egui's cursor/selection state, which keeps it robust across egui
/// versions: since we only ever insert/remove characters *after* the
/// cursor's current position, the cursor egui already placed stays correct.
pub fn process_edit(before: &str, after: &str) -> Option<String> {
    let pos = single_insert_pos(before, after)?;
    let inserted = after[pos..].chars().next()?;

    // Case 1: typing a closer right in front of the same closer we already
    // auto-inserted -> "type through" it instead of duplicating.
    if matches!(inserted, ')' | ']' | '}' | '"' | '\'' | '`') {
        if let Some(next) = before[pos..].chars().next() {
            if next == inserted {
                let mut out = String::with_capacity(after.len() - 1);
                out.push_str(&after[..pos]);
                out.push_str(&after[pos + inserted.len_utf8()..]);
                return Some(out);
            }
        }
    }

    // Case 2: typing an opener -> insert the matching closer right after it.
    let closer = match inserted {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '"' => '"',
        '\'' => '\'',
        '`' => '`',
        _ => return None,
    };

    if matches!(inserted, '"' | '\'' | '`') {
        // Avoid auto-closing quotes/backticks used as an apostrophe inside a
        // word (e.g. "don't") or right before an identical character.
        let prev_is_word = before[..pos]
            .chars()
            .next_back()
            .map(|c| c.is_alphanumeric())
            .unwrap_or(false);
        if prev_is_word {
            return None;
        }
        if before[pos..].chars().next() == Some(inserted) {
            return None;
        }
    }

    let insert_at = pos + inserted.len_utf8();
    let mut out = String::with_capacity(after.len() + closer.len_utf8());
    out.push_str(&after[..insert_at]);
    out.push(closer);
    out.push_str(&after[insert_at..]);
    Some(out)
}
