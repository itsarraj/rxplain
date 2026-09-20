//! Runs a compiled regex against input text and turns the raw `regex`
//! captures into a plain-data shape (`MatchResult`/`GroupMatch`) that the
//! renderer and the CLI can both work with without touching `regex` types
//! directly.

use regex::Regex;

/// One capture group inside one match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMatch {
    /// 1-based capture group index (0 is reserved for the whole match and
    /// never appears here).
    pub index: usize,
    /// `Some(name)` for `(?P<name>...)`, `None` for a plain numbered group.
    pub name: Option<String>,
    /// `None` when the group is part of the pattern but didn't participate
    /// in this particular match (e.g. the untaken side of an alternation).
    pub value: Option<String>,
    pub start: Option<usize>,
    pub end: Option<usize>,
}

/// One match of the pattern against the input, 1-based `match_index` in
/// the order matches were found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    pub match_index: usize,
    pub start: usize,
    pub end: usize,
    pub text: String,
    /// 1-based line number the match starts on.
    pub line: usize,
    /// 1-based column (byte offset within the line + 1) the match starts on.
    pub col: usize,
    pub groups: Vec<GroupMatch>,
}

/// Converts a 0-based byte offset into `text` into a 1-based (line, col).
/// `col` is a byte offset within the line, not a character/grapheme count —
/// fine for ASCII/most UTF-8 text, and never panics on multi-byte input
/// since it only ever looks for the single-byte `\n`.
pub fn line_col(text: &str, byte_pos: usize) -> (usize, usize) {
    let bytes = &text.as_bytes()[..byte_pos.min(text.len())];
    let mut line = 1usize;
    let mut last_newline_end = 0usize;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'\n' {
            line += 1;
            last_newline_end = i + 1;
        }
    }
    (line, byte_pos - last_newline_end + 1)
}

/// Finds every non-overlapping match of `re` in `text` and returns the
/// full match plus every capture group (named or numbered), in order.
pub fn find_matches(re: &Regex, text: &str) -> Vec<MatchResult> {
    let names: Vec<Option<String>> = re
        .capture_names()
        .map(|n| n.map(|s| s.to_string()))
        .collect();

    let mut out = Vec::new();
    for (i, caps) in re.captures_iter(text).enumerate() {
        let whole = caps.get(0).expect("capture group 0 always matches");
        let (line, col) = line_col(text, whole.start());

        let mut groups = Vec::with_capacity(names.len().saturating_sub(1));
        for (gi, name) in names.iter().enumerate().skip(1) {
            let name = name.clone();
            let g = caps.get(gi);
            groups.push(GroupMatch {
                index: gi,
                name,
                value: g.map(|m| m.as_str().to_string()),
                start: g.map(|m| m.start()),
                end: g.map(|m| m.end()),
            });
        }

        out.push(MatchResult {
            match_index: i + 1,
            start: whole.start(),
            end: whole.end(),
            text: whole.as_str().to_string(),
            line,
            col,
            groups,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_first_line() {
        assert_eq!(line_col("hello world", 6), (1, 7));
    }

    #[test]
    fn line_col_second_line() {
        let text = "first line\nsecond line\n";
        // "second" starts right after the first \n, at byte 11.
        assert_eq!(line_col(text, 11), (2, 1));
    }

    #[test]
    fn line_col_third_line_mid_word() {
        let text = "aa\nbb\ncc";
        // second 'c' is at byte 7.
        assert_eq!(line_col(text, 7), (3, 2));
    }

    #[test]
    fn single_match_no_groups() {
        let re = Regex::new(r"world").unwrap();
        let matches = find_matches(&re, "hello world");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "world");
        assert_eq!(matches[0].start, 6);
        assert_eq!(matches[0].end, 11);
        assert!(matches[0].groups.is_empty());
    }

    #[test]
    fn no_match_returns_empty() {
        let re = Regex::new(r"xyz").unwrap();
        let matches = find_matches(&re, "hello world");
        assert!(matches.is_empty());
    }

    #[test]
    fn multiple_matches_in_one_string() {
        let re = Regex::new(r"\d+").unwrap();
        let matches = find_matches(&re, "a1 b22 c333");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].text, "1");
        assert_eq!(matches[1].text, "22");
        assert_eq!(matches[2].text, "333");
        assert_eq!(matches[0].match_index, 1);
        assert_eq!(matches[2].match_index, 3);
    }

    #[test]
    fn named_groups_are_labelled_and_valued() {
        let re = Regex::new(r"(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})").unwrap();
        let matches = find_matches(&re, "2024-01-15");
        assert_eq!(matches.len(), 1);
        let g = &matches[0].groups;
        assert_eq!(g.len(), 3);
        assert_eq!(g[0].name.as_deref(), Some("year"));
        assert_eq!(g[0].value.as_deref(), Some("2024"));
        assert_eq!(g[1].name.as_deref(), Some("month"));
        assert_eq!(g[1].value.as_deref(), Some("01"));
        assert_eq!(g[2].name.as_deref(), Some("day"));
        assert_eq!(g[2].value.as_deref(), Some("15"));
    }

    #[test]
    fn mixed_named_and_numbered_groups() {
        let re = Regex::new(r"(\w+)@(?P<domain>[\w.]+)").unwrap();
        let matches = find_matches(&re, "user@example.com");
        let g = &matches[0].groups;
        assert_eq!(g[0].index, 1);
        assert_eq!(g[0].name, None);
        assert_eq!(g[0].value.as_deref(), Some("user"));
        assert_eq!(g[1].index, 2);
        assert_eq!(g[1].name.as_deref(), Some("domain"));
        assert_eq!(g[1].value.as_deref(), Some("example.com"));
    }

    #[test]
    fn non_participating_group_is_none_not_empty_string() {
        // Only one side of the alternation can ever match, so group 2 never
        // participates when group 1 does, and vice versa.
        let re = Regex::new(r"(foo)|(bar)").unwrap();
        let matches = find_matches(&re, "bar");
        let g = &matches[0].groups;
        assert_eq!(g[0].value, None);
        assert_eq!(g[0].start, None);
        assert_eq!(g[1].value.as_deref(), Some("bar"));
    }

    #[test]
    fn optional_group_not_taken_is_none() {
        let re = Regex::new(r"(\d+)(\.\d+)?").unwrap();
        let matches = find_matches(&re, "42");
        let g = &matches[0].groups;
        assert_eq!(g[0].value.as_deref(), Some("42"));
        assert_eq!(g[1].value, None);
    }

    #[test]
    fn match_index_is_one_based_and_sequential() {
        let re = Regex::new(r"[a-z]+").unwrap();
        let matches = find_matches(&re, "ab cd ef gh");
        let indices: Vec<usize> = matches.iter().map(|m| m.match_index).collect();
        assert_eq!(indices, vec![1, 2, 3, 4]);
    }

    #[test]
    fn multiline_input_reports_correct_line_numbers() {
        let re = Regex::new(r"ERROR").unwrap();
        let text = "line one\nline two ERROR here\nline three\nERROR again\n";
        let matches = find_matches(&re, text);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 2);
        assert_eq!(matches[1].line, 4);
    }
}
