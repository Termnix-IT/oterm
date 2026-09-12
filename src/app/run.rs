use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::layout::Rect;
use ratatui::DefaultTerminal;
use tracing::info;

use crate::config::Config;
use crate::keymap::{self, Action};
use crate::ui;

use super::{AiModalState, App};

const POLL_INTERVAL: Duration = Duration::from_millis(16);

pub fn run(terminal: &mut DefaultTerminal, config: Config) -> Result<()> {
    let initial = terminal.size()?;
    let pane_area = ui::pane_area(Rect::new(0, 0, initial.width, initial.height));
    let mut app = App::new(config, pane_area)?;
    info!(
        tabs = app.tabs.len(),
        ai_enabled = app.ai.is_some(),
        "app initialized"
    );

    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &mut app))?;
        app.poll_ai();

        if !event::poll(POLL_INTERVAL)? {
            continue;
        }
        match event::read()? {
            Event::Key(key) => {
                if app.ai_modal.is_some() {
                    app.handle_modal_key(&key)?;
                } else if let Some(action) = keymap::map_key(&key) {
                    app.handle_action(action)?;
                }
            }
            Event::Paste(text) => {
                if app.ai_modal.is_some() {
                    if let Some(modal) = app.ai_modal.as_mut() {
                        if matches!(modal.state, AiModalState::Editing | AiModalState::Error(_)) {
                            modal.input.push_str(&text);
                        }
                    }
                } else {
                    app.handle_action(Action::Paste(text))?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
