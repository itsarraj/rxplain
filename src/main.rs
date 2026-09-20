use std::io::{IsTerminal, Read};
use std::path::PathBuf;

use clap::Parser;
use regex::RegexBuilder;

use rxplain::matcher::find_matches;
use rxplain::render::{render_highlighted, render_match_summary};

/// Explains a regex match against real input, in the terminal — the full
/// match and every capture group highlighted and labeled, no regex101
/// round-trip needed.
#[derive(Parser)]
#[command(name = "rxplain", version, about)]
struct Cli {
    /// The regex pattern (Rust `regex` crate syntax — very close to PCRE,
    /// no backreferences/lookaround).
    pattern: String,

    /// Input text to match against. If omitted, reads --file or stdin.
    #[arg(long, conflicts_with = "file")]
    text: Option<String>,

    /// Input file to match against. If omitted (and --text isn't given),
    /// reads stdin.
    #[arg(long)]
    file: Option<PathBuf>,

    /// Case-insensitive matching.
    #[arg(short = 'i', long)]
    ignore_case: bool,

    /// Multi-line mode: `^`/`$` match at line boundaries, not just the
    /// start/end of the whole input.
    #[arg(short = 'm', long = "multi-line")]
    multi_line: bool,

    /// `.` also matches `\n` (off by default, like most regex engines).
    #[arg(short = 's', long = "dot-matches-newline")]
    dot_matches_newline: bool,

    /// Extended/verbose syntax: whitespace and `#`-comments in the pattern
    /// are ignored unless escaped.
    #[arg(short = 'x', long)]
    extended: bool,

    /// Disable ANSI highlighting even on a real terminal.
    #[arg(long)]
    no_color: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let input = match (&cli.text, &cli.file) {
        (Some(t), _) => t.clone(),
        (None, Some(path)) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?,
        (None, None) => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| anyhow::anyhow!("reading stdin: {e}"))?;
            buf
        }
    };

    let re = RegexBuilder::new(&cli.pattern)
        .case_insensitive(cli.ignore_case)
        .multi_line(cli.multi_line)
        .dot_matches_new_line(cli.dot_matches_newline)
        .ignore_whitespace(cli.extended)
        .build()
        .map_err(|e| anyhow::anyhow!("invalid regex '{}': {e}", cli.pattern))?;

    let color =
        !cli.no_color && std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();

    let matches = find_matches(&re, &input);

    println!("--- input (matches highlighted) ---");
    println!("{}", render_highlighted(&input, &matches, color));
    println!("--- end input ---\n");

    if matches.is_empty() {
        println!("no matches found");
        std::process::exit(1);
    }

    println!("{} match(es) found:\n", matches.len());
    for m in &matches {
        print!("{}", render_match_summary(m, color));
    }

    Ok(())
}
