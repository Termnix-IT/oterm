use std::path::PathBuf;

use anyhow::{anyhow, Result};

use super::{home_dir, read_lossy};

pub(super) fn load() -> Result<Vec<String>> {
    let path = std::env::var_os("HISTFILE")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".bash_history")))
        .ok_or_else(|| anyhow!("cannot locate the bash history file"))?;
    Ok(parse(&read_lossy(&path)?))
}

/// With HISTTIMEFORMAT set, bash writes a `#<epoch>` line before each entry.
fn parse(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !is_timestamp(line))
        .map(str::to_string)
        .collect()
}

fn is_timestamp(line: &str) -> bool {
    line.strip_prefix('#')
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn timestamp_lines_are_skipped() {
        let text = "#1700000000\nls -la\n\n#1700000001\ngit status\n# note\n";
        assert_eq!(parse(text), vec!["ls -la", "git status", "# note"]);
    }
}
