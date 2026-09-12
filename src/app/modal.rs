use std::sync::mpsc;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::ai::AiOutcome;

use super::App;

pub struct AiModal {
    pub input: String,
    pub state: AiModalState,
}

pub enum AiModalState {
    Editing,
    InFlight(mpsc::Receiver<AiOutcome>),
    Result(String),
    Error(String),
}

impl App {
    pub(super) fn open_ai_modal(&mut self) {
        if self.ai.is_none() {
            self.ai_modal = Some(AiModal {
                input: String::new(),
                state: AiModalState::Error(
                    "AI is disabled. Set [ai].enabled = true in config.toml, then pick a provider \
                     (anthropic needs a key in secrets.toml; ollama needs a local `ollama serve`)."
                        .into(),
                ),
            });
            return;
        }
        self.ai_modal = Some(AiModal {
            input: String::new(),
            state: AiModalState::Editing,
        });
    }

    pub(super) fn handle_modal_key(&mut self, key: &KeyEvent) -> Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        let Some(modal) = self.ai_modal.as_mut() else {
            return Ok(());
        };
        match key.code {
            KeyCode::Esc => {
                self.ai_modal = None;
                return Ok(());
            }
            KeyCode::Enter => match &modal.state {
                AiModalState::Editing | AiModalState::Error(_) => {
                    if !modal.input.trim().is_empty() {
                        if let Some(client) = &self.ai {
                            let rx = client.suggest_async(modal.input.trim().to_string());
                            modal.state = AiModalState::InFlight(rx);
                        }
                    }
                }
                AiModalState::Result(text) => {
                    let bytes = text.clone();
                    self.ai_modal = None;
                    self.send_input(bytes.as_bytes())?;
                }
                AiModalState::InFlight(_) => {}
            },
            KeyCode::Tab => {
                if matches!(modal.state, AiModalState::Editing | AiModalState::Error(_)) {
                    if let Some(client) = self.ai.as_mut() {
                        client.cycle_provider();
                    }
                }
            }
            KeyCode::BackTab => {
                if matches!(modal.state, AiModalState::Editing | AiModalState::Error(_)) {
                    if let Some(client) = self.ai.as_mut() {
                        client.cycle_model();
                    }
                }
            }
            KeyCode::Backspace => {
                if matches!(modal.state, AiModalState::Editing | AiModalState::Error(_)) {
                    modal.input.pop();
                }
            }
            KeyCode::Char(c)
                if matches!(modal.state, AiModalState::Editing | AiModalState::Error(_))
                    && !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                modal.input.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn poll_ai(&mut self) {
        let Some(modal) = self.ai_modal.as_mut() else {
            return;
        };
        let next = match &modal.state {
            AiModalState::InFlight(rx) => match rx.try_recv() {
                Ok(AiOutcome::Ok(text)) => Some(AiModalState::Result(text)),
                Ok(AiOutcome::Err(err)) => Some(AiModalState::Error(err)),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(AiModalState::Error("AI worker thread disconnected".into()))
                }
            },
            _ => None,
        };
        if let Some(state) = next {
            modal.state = state;
        }
    }
}
