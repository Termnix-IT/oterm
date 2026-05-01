mod ai;
mod app;
mod config;
mod keymap;
mod pane;
mod pty;
mod ui;

use std::fs::OpenOptions;
use std::path::PathBuf;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    init_tracing()?;
    tracing::info!("oterm starting");

    let cfg = config::Config::load().unwrap_or_else(|err| {
        tracing::warn!(error = %err, "config load failed; using defaults");
        config::Config::defaults().expect("default config must parse")
    });

    let mut terminal = ratatui::init();
    let result = app::run(&mut terminal, cfg);
    ratatui::restore();

    if let Err(err) = &result {
        eprintln!("oterm exited with error: {err:?}");
    }
    tracing::info!("oterm shutdown");
    result
}

fn init_tracing() -> Result<()> {
    let path = log_path();
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file)
        .with_ansi(false)
        .init();
    Ok(())
}

fn log_path() -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("oterm.log");
    dir
}
