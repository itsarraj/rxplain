# rxplain

Explains what a regex actually matches against real input, in the
terminal. The alternative today is pasting a pattern and a sample string
into regex101.com — a website — every time you need to sanity-check a
capture group or figure out why a pattern isn't matching what you expect.
This is the same highlighted-match, labeled-groups view as a local
binary: no browser, no pasting a real log line or a real customer email
into a third-party site just to debug a regex.

## Usage

```bash
rxplain '(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})' --text "shipped 2024-01-15"
rxplain '\berror\b' --file app.log -i          # case-insensitive, from a file
rxplain '^\s*#' --file config.toml -m           # multi-line ^/$ mode
some-command | rxplain 'timeout|refused'        # reads stdin if neither --text nor --file given
```

```
$ rxplain '(?P<exchange>\d{3})-(?P<line>\d{4})' --text "call 555-1234 or 555-5678" --no-color
--- input (matches highlighted) ---
call «555-1234» or «555-5678»
--- end input ---

2 match(es) found:

match 1 — line 1, col 6 (bytes 5..13): "555-1234"
  group 1 "exchange" = "555" (bytes 5..8)
  group 2 "line" = "1234" (bytes 9..13)
match 2 — line 1, col 18 (bytes 17..25): "555-5678"
  group 1 "exchange" = "555" (bytes 17..20)
  group 2 "line" = "5678" (bytes 21..25)
```

On a real terminal (and without `--no-color`/`NO_COLOR`), the highlighted
input pane uses actual ANSI color instead of `«...»` guillemets, and each
group's label in the breakdown below is colored from a 6-color palette
cycling by group index, so group 1 and group 7 read the same but group 1
and group 2 are visually distinct.

## What gets highlighted where

Two separate views, deliberately not merged into one nested rendering:

1. **The input pane** — the original text printed back out with every
   **full match** span highlighted in place. This answers "where in my
   real data does this pattern actually fire."
2. **The per-match breakdown** — for every match, every capture group
   (named via `(?P<name>...)` or plain numbered) listed with its index,
   name if any, matched value, and byte range. A group that's part of the
   pattern but didn't participate in a given match (the untaken side of
   an alternation, an optional group that wasn't taken) is reported as
   "did not participate in this match" — not a blank string, which would
   be indistinguishable from an actual empty match.

Capture groups are **not** inline-highlighted nested inside the input
pane itself — see Scope below.

Flags map directly to `regex::RegexBuilder`: `-i`/`--ignore-case`,
`-m`/`--multi-line` (`^`/`$` at line boundaries), `-s`/`--dot-matches-newline`
(`.` matches `\n`), `-x`/`--extended` (whitespace/`#`-comments in the
pattern itself are ignored unless escaped, for writing a readable
multi-line pattern).

Exit codes: `0` if at least one match was found, `1` if the pattern
compiled fine but found zero matches in the input, `1` also for I/O
errors and invalid regex syntax (the error message on stderr
distinguishes these — nothing scripting on the exit code alone needs a
third code here, since "compiled but didn't match" and "didn't compile"
are both just "nothing to act on").

## Scope: full-match highlighting is nested, capture-group highlighting isn't

Capture groups are always properly nested or sequential within their
enclosing match (that's a structural property of how parentheses work in
a regex), so a fully faithful regex101-style view — each group
individually colored *inside* the highlighted match, including groups
nested inside other groups — is possible in principle. It's not done
here: the input pane highlights full matches only, and group detail lives
in the separate per-match breakdown underneath. This is a real scope cut,
not an oversight — the two-pane approach covers the actual debugging
question ("what matched, and what did each group capture") without the
extra rendering complexity of tracking overlapping ANSI regions.

## Status: built, verified against real fixtures with named groups, multiple matches, and a no-match case

- **18 unit tests** (`cargo test --lib`): line/column computation across
  multi-line text (first line, a line after one newline, a line after
  two, mid-word); a single match with no groups; zero matches; three
  matches in one string with 1-based sequential indices; named groups
  (`year`/`month`/`day`) each correctly labeled and valued; mixed
  named+numbered groups in the same pattern; a non-participating group
  from an untaken alternation reported as `None`, not an empty string;
  an untaken optional group likewise; multi-line input reporting the
  correct line number for matches on line 2 and line 4; both rendering
  modes (`--no-color` guillemets vs. real ANSI codes) on the highlighter;
  the per-match summary correctly labeling a named group, marking a
  non-participating group, and saying "no capture groups" when the
  pattern has none.
- **`cargo clippy --all-targets -- -D warnings`**: clean. (One real
  warning surfaced and was fixed during development —
  `clippy::needless_range_loop` on the group-building loop in
  `matcher.rs`, which originally indexed `names[gi]` inside a
  `for gi in 1..names.len()` loop; rewritten as
  `names.iter().enumerate().skip(1)`.)
- **Live-run against a realistic multi-line access-log fixture** (3 lines,
  IP/date/time/method/path/status) with a 6-named-group pattern
  (`ip`, `date`, `time`, `method`, `path`, `status`) and `-m`: all 3
  matches found, every group's value and byte range checked by hand
  against the actual file content, correct line numbers (1, 2, 3) for
  each match.
- **Live-verified the no-color/color split on a real pty**: piped output
  (this shell) renders `«...»` guillemets; run under `script` (a real
  pseudo-terminal) with `-i`, the actual bytes are
  `\x1b[1;33mBAR\x1b[0m` around a case-insensitively matched `BAR` —
  confirmed byte-for-byte with `cat -A`, not just assumed from the code.
- **Live-verified error handling**: an unclosed group (`'(unclosed'`)
  produces a clean `regex parse error` on stderr and exit code `1`, not
  a panic; a pattern with zero matches against real input prints the
  unhighlighted input back, says `no matches found`, and exits `1`.

**Not done / deliberately deferred**: no nested inline group highlighting
in the input pane (see Scope); no PCRE-only features `regex` itself
doesn't support (backreferences, lookaround) — whatever `regex` rejects,
`rxplain` reports as a parse error rather than pretending to support it;
column numbers are byte offsets within the line, not grapheme-cluster
positions, so a match after multi-byte UTF-8 content on the same line
will have a byte column that doesn't equal what a text editor would call
the character column (documented here rather than silently wrong).
