mod bash;
mod psreadline;

use std::cmp::Reverse;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::BaseDirs;

use crate::config::{Profile, ProfileType};

pub const MAX_RESULTS: usize = 20;

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "be", "by", "command", "commands", "do", "for", "from", "if",
    "in", "into", "is", "it", "me", "my", "of", "on", "or", "ran", "run", "so", "that", "the",
    "this", "to", "up", "used", "was", "we", "were", "with", "you",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    PowerShell,
    Bash,
}

impl ShellKind {
    /// Judged from the pane's profile; a shell started inside the pane later isn't detected.
    pub fn from_profile(profile: &Profile) -> Option<Self> {
        if profile.kind != ProfileType::Local {
            return None;
        }
        let command = profile.command.as_deref()?;
        let file = command.rsplit(['/', '\\']).next()?.to_ascii_lowercase();
        match file.strip_suffix(".exe").unwrap_or(&file) {
            "powershell" | "pwsh" => Some(Self::PowerShell),
            "bash" => Some(Self::Bash),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::PowerShell => "PowerShell",
            Self::Bash => "bash",
        }
    }

    /// Single-line history entries, oldest first.
    pub fn load(self) -> Result<Vec<String>> {
        match self {
            Self::PowerShell => psreadline::load(),
            Self::Bash => bash::load(),
        }
    }
}

/// ASCII command-like words in the query, so `deploy.ps1` matches even if the model omits it.
pub fn literal_terms(query: &str) -> Vec<String> {
    query
        .split(|c: char| !(c.is_ascii_alphanumeric() || "._-/\\:".contains(c)))
        .filter(|t| t.len() >= 2 && t.chars().any(|c| c.is_ascii_alphanumeric()))
        .filter(|t| !STOPWORDS.contains(&t.to_ascii_lowercase().as_str()))
        .map(str::to_string)
        .collect()
}

/// Small local models add bullets, numbering or quotes despite the prompt; strip them.
pub fn parse_terms(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| {
            strip_list_marker(line)
                .trim_matches(|c| c == '`' || c == '"' || c == '\'')
                .trim()
        })
        .filter(|t| !t.is_empty() && !is_placeholder(t))
        .map(str::to_string)
        .collect()
}

fn is_placeholder(term: &str) -> bool {
    term.starts_with('<') && term.ends_with('>')
}

fn strip_list_marker(line: &str) -> &str {
    let line = line.trim();
    for marker in ["- ", "* ", "• "] {
        if let Some(rest) = line.strip_prefix(marker) {
            return rest.trim_start();
        }
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    let rest = &line[digits..];
    match rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
        Some(stripped) if digits > 0 => stripped.trim_start(),
        _ => line,
    }
}

/// Lowercases, drops empties and removes repeats, keeping first-seen order.
pub fn normalize_terms(terms: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    terms
        .into_iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty() && seen.insert(t.clone()))
        .collect()
}

/// Ranked by distinct terms matched, then recency; a repeated command keeps its latest spot.
pub fn search(entries: &[String], terms: &[String], limit: usize) -> Vec<String> {
    if terms.is_empty() {
        return Vec::new();
    }
    let mut seen = HashSet::new();
    let mut ranked = Vec::new();
    for (position, entry) in entries.iter().enumerate().rev() {
        if !seen.insert(entry.as_str()) {
            continue;
        }
        let lower = entry.to_lowercase();
        let score = terms.iter().filter(|t| lower.contains(t.as_str())).count();
        if score > 0 {
            ranked.push((score, position, entry));
        }
    }
    ranked.sort_by_key(|&(score, position, _)| Reverse((score, position)));
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, _, entry)| entry.clone())
        .collect()
}

fn read_lossy(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("cannot read history file {}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(text.trim_start_matches('\u{feff}').to_string())
}

fn home_dir() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(command: &str) -> Profile {
        Profile {
            name: "test".into(),
            kind: ProfileType::Local,
            command: Some(command.into()),
            args: Vec::new(),
            host: None,
            user: None,
            port: None,
            extra_args: Vec::new(),
        }
    }

    #[test]
    fn shell_kind_follows_the_profile_command() {
        let pwsh = r"C:\Program Files\PowerShell\7\pwsh.exe";
        assert_eq!(
            ShellKind::from_profile(&local("powershell.exe")),
            Some(ShellKind::PowerShell)
        );
        assert_eq!(
            ShellKind::from_profile(&local(pwsh)),
            Some(ShellKind::PowerShell)
        );
        assert_eq!(
            ShellKind::from_profile(&local("/bin/bash")),
            Some(ShellKind::Bash)
        );
        assert_eq!(ShellKind::from_profile(&local("cmd.exe")), None);

        let mut ssh = local("ssh");
        ssh.kind = ProfileType::Ssh;
        assert_eq!(ShellKind::from_profile(&ssh), None);
    }

    #[test]
    fn literal_terms_pick_command_words_out_of_japanese_and_prose() {
        assert_eq!(
            literal_terms("先週dockerのログを見た deploy.ps1"),
            vec!["docker", "deploy.ps1"]
        );
        assert_eq!(
            literal_terms("the command to list files"),
            vec!["list", "files"]
        );
    }

    #[test]
    fn parse_terms_strips_list_markers_quotes_and_placeholders() {
        let text =
            "1. docker logs\n- `docker compose logs`\n* \"journalctl\"\n<container_name>\n\n2) kubectl logs";
        assert_eq!(
            parse_terms(text),
            vec![
                "docker logs",
                "docker compose logs",
                "journalctl",
                "kubectl logs"
            ]
        );
    }

    #[test]
    fn parse_terms_keeps_leading_digits_that_are_not_numbering() {
        assert_eq!(parse_terms("7z x archive.7z"), vec!["7z x archive.7z"]);
    }

    #[test]
    fn search_ranks_by_matched_terms_then_recency_and_dedupes() {
        let entries: Vec<String> = [
            "docker ps",
            "docker logs web",
            "git status",
            "docker logs web",
            "docker compose logs -f",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let terms = normalize_terms(["Docker".into(), "logs".into(), "docker".into()]);
        assert_eq!(terms, vec!["docker", "logs"]);
        assert_eq!(
            search(&entries, &terms, 10),
            vec!["docker compose logs -f", "docker logs web", "docker ps"]
        );
    }

    #[test]
    fn search_with_no_terms_finds_nothing() {
        let entries = vec!["ls".to_string()];
        assert!(search(&entries, &[], 10).is_empty());
    }
}
