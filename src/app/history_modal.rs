use std::sync::mpsc;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::ai::AiOutcome;
use crate::history::{self, ShellKind};

use super::App;

const UNSUPPORTED_SHELL: &str =
    "History search works in PowerShell and bash panes only (not SSH or cmd).";

pub struct HistoryModal {
    pub input: String,
    pub shell: Option<ShellKind>,
    pub state: HistoryModalState,
}

pub enum HistoryModalState {
    Editing,
    InFlight(mpsc::Receiver<AiOutcome>),
    Results {
        items: Vec<String>,
        selected: usize,
        terms: Vec<String>,
        ai_used: bool,
    },
    Error(String),
}

impl App {
    pub(super) fn open_history_modal(&mut self) {
        let shell = self
            .tabs
            .get(self.active_tab)
            .and_then(|tab| tab.panes.get(&tab.active))
            .and_then(|pty| ShellKind::from_profile(pty.profile()));
        let state = match shell {
            Some(_) => HistoryModalState::Editing,
            None => HistoryModalState::Error(UNSUPPORTED_SHELL.into()),
        };
        self.history_modal = Some(HistoryModal {
            input: String::new(),
            shell,
            state,
        });
    }

    pub(super) fn handle_history_key(&mut self, key: &KeyEvent) -> Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        let Some(modal) = self.history_modal.as_mut() else {
            return Ok(());
        };
        let busy = matches!(modal.state, HistoryModalState::InFlight(_));
        match key.code {
            KeyCode::Esc => self.history_modal = None,
            KeyCode::Enter => match &modal.state {
                HistoryModalState::InFlight(_) => {}
                HistoryModalState::Results {
                    items, selected, ..
                } => {
                    if let Some(command) = items.get(*selected).cloned() {
                        self.history_modal = None;
                        self.send_input(command.as_bytes())?;
                    }
                }
                HistoryModalState::Editing | HistoryModalState::Error(_) => {
                    self.start_history_search();
                }
            },
            KeyCode::Up | KeyCode::Down => {
                if let HistoryModalState::Results {
                    items, selected, ..
                } = &mut modal.state
                {
                    let last = items.len().saturating_sub(1);
                    *selected = if key.code == KeyCode::Up {
                        selected.saturating_sub(1)
                    } else {
                        (*selected + 1).min(last)
                    };
                }
            }
            KeyCode::Tab if !busy => {
                if let Some(client) = self.ai.as_mut() {
                    client.cycle_provider();
                }
            }
            KeyCode::BackTab if !busy => {
                if let Some(client) = self.ai.as_mut() {
                    client.cycle_model();
                }
            }
            KeyCode::Backspace if !busy => {
                modal.input.pop();
                modal.state = HistoryModalState::Editing;
            }
            KeyCode::Char(c) if !busy && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                modal.input.push(c);
                modal.state = HistoryModalState::Editing;
            }
            _ => {}
        }
        Ok(())
    }

    fn start_history_search(&mut self) {
        let Some(modal) = self.history_modal.as_mut() else {
            return;
        };
        let query = modal.input.trim().to_string();
        if query.is_empty() {
            return;
        }
        let Some(shell) = modal.shell else {
            modal.state = HistoryModalState::Error(UNSUPPORTED_SHELL.into());
            return;
        };
        modal.state = match &self.ai {
            Some(client) => {
                HistoryModalState::InFlight(client.search_terms_async(query, shell.label()))
            }
            None => search_history(shell, history::literal_terms(&query), false),
        };
    }

    pub(super) fn poll_history(&mut self) {
        let Some(modal) = self.history_modal.as_mut() else {
            return;
        };
        let HistoryModalState::InFlight(rx) = &modal.state else {
            return;
        };
        let next = match rx.try_recv() {
            Ok(AiOutcome::Ok(text)) => match modal.shell {
                Some(shell) => {
                    let mut terms = history::parse_terms(&text);
                    terms.extend(history::literal_terms(&modal.input));
                    search_history(shell, terms, true)
                }
                None => HistoryModalState::Error(UNSUPPORTED_SHELL.into()),
            },
            Ok(AiOutcome::Err(err)) => HistoryModalState::Error(err),
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                HistoryModalState::Error("AI worker thread disconnected".into())
            }
        };
        modal.state = next;
    }
}

fn search_history(shell: ShellKind, terms: Vec<String>, ai_used: bool) -> HistoryModalState {
    let terms = history::normalize_terms(terms);
    match shell.load() {
        Ok(entries) => HistoryModalState::Results {
            items: history::search(&entries, &terms, history::MAX_RESULTS),
            selected: 0,
            terms,
            ai_used,
        },
        Err(err) => HistoryModalState::Error(format!("{err:#}")),
    }
}
