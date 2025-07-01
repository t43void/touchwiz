//! Custom text import: turn an arbitrary text/code file into clean practice
//! material (specification Section 10.8).
//!
//! Whitespace is normalized to single spaces, blank lines are dropped, and—when
//! requested—line (`//`, `#`) and block (`/* */`) comments are stripped. The
//! result is capped to a sensible length so a huge file still yields a usable
//! session.

/// Maximum number of words taken from an imported file.
pub const MAX_WORDS: usize = 600;

/// Removes `/* ... */` block comments, including multi-line ones.
fn strip_block_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Skip until the closing */ (or end of input).
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            out.push(' ');
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Prepares raw file contents into a single normalized practice string.
///
/// With `strip_comments`, `//`/`#` line comments and `/* */` block comments are
/// removed first. Returns an empty string if nothing usable remains.
pub fn prepare_text(raw: &str, strip_comments: bool) -> String {
    let source = if strip_comments {
        strip_block_comments(raw)
    } else {
        raw.to_string()
    };

    let mut words: Vec<&str> = Vec::new();
    for line in source.lines() {
        let mut line = line;
        if strip_comments {
            if let Some(idx) = line.find("//") {
                line = &line[..idx];
            }
            if let Some(idx) = line.find('#') {
                line = &line[..idx];
            }
        }
        words.extend(line.split_whitespace());
        if words.len() >= MAX_WORDS {
            break;
        }
    }
    words.truncate(MAX_WORDS);
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_whitespace_and_drops_blank_lines() {
        let raw = "hello   world\n\n  foo\tbar  \n";
        assert_eq!(prepare_text(raw, false), "hello world foo bar");
    }

    #[test]
    fn strips_line_comments() {
        let raw = "let x = 1; // set x\ndo_thing();  # shell-style\n";
        assert_eq!(prepare_text(raw, true), "let x = 1; do_thing();");
    }

    #[test]
    fn strips_block_comments_across_lines() {
        let raw = "a /* multi\nline comment */ b";
        assert_eq!(prepare_text(raw, true), "a b");
    }

    #[test]
    fn keeps_comments_when_not_requested() {
        let raw = "code // note";
        assert_eq!(prepare_text(raw, false), "code // note");
    }

    #[test]
    fn caps_at_max_words() {
        let raw = "w ".repeat(MAX_WORDS + 50);
        let out = prepare_text(&raw, false);
        assert_eq!(out.split(' ').count(), MAX_WORDS);
    }

    #[test]
    fn empty_when_nothing_usable() {
        assert_eq!(prepare_text("   \n\n", false), "");
    }
}
