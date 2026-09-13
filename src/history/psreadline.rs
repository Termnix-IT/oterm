use std::path::PathBuf;

use anyhow::{anyhow, Result};

use super::{home_dir, read_lossy};

pub(super) fn load() -> Result<Vec<String>> {
    let path = path().ok_or_else(|| anyhow!("cannot locate the PSReadLine history directory"))?;
    Ok(parse(&read_lossy(&path)?))
}

fn path() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?)
            .join("Microsoft")
            .join("Windows")
            .join("PowerShell")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".local").join("share")))?
            .join("powershell")
    };
    Some(base.join("PSReadLine").join("ConsoleHost_history.txt"))
}

/// Skips multi-line (backtick-continued) entries: inserting them would run early lines unreviewed.
fn parse(text: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut in_multiline = false;
    for line in text.lines() {
        if line.ends_with('`') {
            in_multiline = true;
            continue;
        }
        if std::mem::take(&mut in_multiline) {
            continue;
        }
        let entry = line.trim();
        if !entry.is_empty() {
            entries.push(entry.to_string());
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn multi_line_entries_are_dropped() {
        let text = "ls\nGet-ChildItem `\n  -Recurse `\n  -Force\ncd src\n\n";
        assert_eq!(parse(text), vec!["ls", "cd src"]);
    }
}
