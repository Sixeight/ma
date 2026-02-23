use unicode_width::UnicodeWidthStr;

pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Split text on `<br/>`, `<br>`, `<br />` (case-insensitive).
pub fn split_br(s: &str) -> Vec<&str> {
    let lower = s.to_ascii_lowercase();
    let lower_bytes = lower.as_bytes();
    let mut result = Vec::new();
    let mut start = 0;
    let mut i = 0;

    while i + 3 < lower_bytes.len() {
        if lower_bytes[i] == b'<' && lower_bytes[i + 1] == b'b' && lower_bytes[i + 2] == b'r' {
            let tag_len = if i + 5 <= lower_bytes.len()
                && lower_bytes[i + 3] == b'/'
                && lower_bytes[i + 4] == b'>'
            {
                5 // <br/>
            } else if i + 6 <= lower_bytes.len()
                && lower_bytes[i + 3] == b' '
                && lower_bytes[i + 4] == b'/'
                && lower_bytes[i + 5] == b'>'
            {
                6 // <br />
            } else if lower_bytes[i + 3] == b'>' {
                4 // <br>
            } else {
                0
            };

            if tag_len > 0 {
                result.push(&s[start..i]);
                start = i + tag_len;
                i = start;
                continue;
            }
        }
        i += 1;
    }
    result.push(&s[start..]);
    result
}

/// Maximum display width among lines split by `<br/>`.
pub fn multiline_width(s: &str) -> usize {
    split_br(s)
        .iter()
        .map(|line| display_width(line))
        .max()
        .unwrap_or(0)
}

/// Number of lines after splitting by `<br/>`.
pub fn line_count(s: &str) -> usize {
    split_br(s).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_br_no_break() {
        assert_eq!(split_br("hello"), vec!["hello"]);
    }

    #[test]
    fn split_br_single() {
        assert_eq!(split_br("Hello<br/>World"), vec!["Hello", "World"]);
    }

    #[test]
    fn split_br_variant_no_slash() {
        assert_eq!(split_br("A<br>B"), vec!["A", "B"]);
    }

    #[test]
    fn split_br_variant_space() {
        assert_eq!(split_br("A<br />B"), vec!["A", "B"]);
    }

    #[test]
    fn split_br_case_insensitive() {
        assert_eq!(split_br("A<BR/>B"), vec!["A", "B"]);
        assert_eq!(split_br("A<Br>B"), vec!["A", "B"]);
    }

    #[test]
    fn split_br_multiple() {
        assert_eq!(split_br("A<br/>B<br/>C"), vec!["A", "B", "C"]);
    }

    #[test]
    fn multiline_width_single_line() {
        assert_eq!(multiline_width("hello"), 5);
    }

    #[test]
    fn multiline_width_multi_line() {
        assert_eq!(multiline_width("Hi<br/>World"), 5);
    }

    #[test]
    fn line_count_single() {
        assert_eq!(line_count("hello"), 1);
    }

    #[test]
    fn line_count_multi() {
        assert_eq!(line_count("A<br/>B<br/>C"), 3);
    }
}
