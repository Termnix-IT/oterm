use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{anyhow, Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tracing::warn;

use crate::config::{Profile, ProfileType};

pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    parser: Arc<Mutex<vt100::Parser>>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Pty {
    pub fn spawn(profile: &Profile, rows: u16, cols: u16) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("failed to open pty")?;

        let mut cmd = build_command(profile)?;
        if matches!(profile.kind, ProfileType::Local) {
            if let Some(home) = home_dir() {
                cmd.cwd(home);
            }
        }
        let child = pair
            .slave
            .spawn_command(cmd)
            .with_context(|| format!("failed to spawn profile '{}' into pty", profile.name))?;
        drop(pair.slave);

        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 1000)));
        let mut reader = pair
            .master
            .try_clone_reader()
            .context("failed to clone pty reader")?;
        let parser_for_reader = Arc::clone(&parser);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut p) = parser_for_reader.lock() {
                            p.process(&buf[..n]);
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "pty reader error, exiting reader thread");
                        break;
                    }
                }
            }
        });

        let writer = pair
            .master
            .take_writer()
            .context("failed to take pty writer")?;

        Ok(Self {
            master: pair.master,
            writer,
            parser,
            _child: child,
        })
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer.write_all(bytes).context("pty write failed")?;
        self.writer.flush().ok();
        Ok(())
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("pty resize failed")?;
        if let Ok(mut p) = self.parser.lock() {
            p.screen_mut().set_size(rows, cols);
        }
        Ok(())
    }

    pub fn parser(&self) -> Arc<Mutex<vt100::Parser>> {
        Arc::clone(&self.parser)
    }
}

fn build_command(profile: &Profile) -> Result<CommandBuilder> {
    match profile.kind {
        ProfileType::Local => {
            let cmd = profile.command.as_deref().ok_or_else(|| {
                anyhow!("local profile '{}' is missing 'command'", profile.name)
            })?;
            let mut cb = CommandBuilder::new(cmd);
            for a in &profile.args {
                cb.arg(a);
            }
            Ok(cb)
        }
        ProfileType::Ssh => {
            let host = profile.host.as_deref().ok_or_else(|| {
                anyhow!("ssh profile '{}' is missing 'host'", profile.name)
            })?;
            let bin = profile.command.as_deref().unwrap_or("ssh");
            let mut cb = CommandBuilder::new(bin);
            for a in &profile.extra_args {
                cb.arg(a);
            }
            if let Some(p) = profile.port {
                cb.arg("-p");
                cb.arg(p.to_string());
            }
            let target = match &profile.user {
                Some(user) => format!("{user}@{host}"),
                None => host.to_string(),
            };
            cb.arg(target);
            Ok(cb)
        }
    }
}

fn home_dir() -> Option<std::path::PathBuf> {
    if cfg!(windows) {
        std::env::var_os("USERPROFILE").map(Into::into)
    } else {
        std::env::var_os("HOME").map(Into::into)
    }
}
