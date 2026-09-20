//! Turns `MatchResult`s into terminal output: the original input with every
//! full match highlighted in place, and a per-match breakdown of every
//! capture group underneath.

use crate::matcher::MatchResult;

const RESET: &str = "\x1b[0m";
const MATCH_COLOR: &str = "\x1b[1;33m"; // bold yellow
const GROUP_PALETTE: [&str; 6] = [
    "\x1b[36m", // cyan
    "\x1b[35m", // magenta
    "\x1b[32m", // green
    "\x1b[34m", // blue
    "\x1b[33m", // yellow
    "\x1b[31m", // red
];

fn group_color(index: usize) -> &'static str {
    GROUP_PALETTE[(index - 1) % GROUP_PALETTE.len()]
}

/// Returns `text` with every match's full-match span wrapped: ANSI bold
/// yellow when `color` is on, plain `«...»` guillemets otherwise (so the
/// highlighting still reads clearly with `--no-color`, `NO_COLOR` set, or
/// output piped to a file).
pub fn render_highlighted(text: &str, matches: &[MatchResult], color: bool) -> String {
    let mut out = String::with_capacity(text.len() + matches.len() * 16);
    let mut pos = 0usize;
    for m in matches {
        out.push_str(&text[pos..m.start]);
        if color {
            out.push_str(MATCH_COLOR);
            out.push_str(&text[m.start..m.end]);
            out.push_str(RESET);
        } else {
            out.push('\u{ab}'); // «
            out.push_str(&text[m.start..m.end]);
            out.push('\u{bb}'); // »
        }
        pos = m.end;
    }
    out.push_str(&text[pos..]);
    out
}

/// Renders the full breakdown for one match: header line (index, position,
/// the matched text) plus one line per capture group.
pub fn render_match_summary(m: &MatchResult, color: bool) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "match {} — line {}, col {} (bytes {}..{}): {:?}\n",
        m.match_index, m.line, m.col, m.start, m.end, m.text
    ));

    if m.groups.is_empty() {
        s.push_str("  (pattern has no capture groups)\n");
        return s;
    }

    for g in &m.groups {
        let label = match &g.name {
            Some(name) => format!("group {} \"{}\"", g.index, name),
            None => format!("group {}", g.index),
        };
        let label = if color {
            format!("{}{label}{RESET}", group_color(g.index))
        } else {
            label
        };
        match (&g.value, g.start, g.end) {
            (Some(v), Some(start), Some(end)) => {
                s.push_str(&format!("  {label} = {v:?} (bytes {start}..{end})\n"));
            }
            _ => {
                s.push_str(&format!(
                    "  {label} = (did not participate in this match)\n"
                ));
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::find_matches;
    use regex::Regex;

    #[test]
    fn highlight_no_color_wraps_with_guillemets() {
        let re = Regex::new(r"\d+").unwrap();
        let matches = find_matches(&re, "a1 b22");
        let rendered = render_highlighted("a1 b22", &matches, false);
        assert_eq!(rendered, "a\u{ab}1\u{bb} b\u{ab}22\u{bb}");
    }

    #[test]
    fn highlight_color_wraps_with_ansi() {
        let re = Regex::new(r"\d+").unwrap();
        let matches = find_matches(&re, "a1");
        let rendered = render_highlighted("a1", &matches, true);
        assert!(rendered.contains(MATCH_COLOR));
        assert!(rendered.contains(RESET));
        assert!(rendered.contains('1'));
    }

    #[test]
    fn highlight_no_matches_returns_text_unchanged() {
        let matches: Vec<MatchResult> = vec![];
        let rendered = render_highlighted("nothing to see here", &matches, true);
        assert_eq!(rendered, "nothing to see here");
    }

    #[test]
    fn summary_lists_named_group_and_value() {
        let re = Regex::new(r"(?P<year>\d{4})").unwrap();
        let matches = find_matches(&re, "2024");
        let s = render_match_summary(&matches[0], false);
        assert!(s.contains("group 1 \"year\""));
        assert!(s.contains("\"2024\""));
    }

    #[test]
    fn summary_marks_non_participating_group() {
        let re = Regex::new(r"(foo)|(bar)").unwrap();
        let matches = find_matches(&re, "bar");
        let s = render_match_summary(&matches[0], false);
        assert!(s.contains("did not participate"));
    }

    #[test]
    fn summary_no_groups_says_so() {
        let re = Regex::new(r"hello").unwrap();
        let matches = find_matches(&re, "hello");
        let s = render_match_summary(&matches[0], false);
        assert!(s.contains("no capture groups"));
    }
}
