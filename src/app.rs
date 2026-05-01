use std::sync::mpsc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::DefaultTerminal;
use tracing::info;

use crate::ai::{AiClient, AiOutcome};
use crate::config::{Config, Profile};
use crate::keymap::{self, Action};
use crate::pane::{SplitOrientation, Tab};
use crate::ui;

const POLL_INTERVAL: Duration = Duration::from_millis(16);

pub struct App {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub config: Config,
    pub should_quit: bool,
    pub ai: Option<AiClient>,
    pub ai_modal: Option<AiModal>,
}

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
    pub fn new(config: Config, pane_area: Rect) -> Result<Self> {
        let profile = pick_profile(&config)?;
        let (rows, cols) = initial_pane_size(pane_area);
        let tab = Tab::new(&profile, profile.name.clone(), rows, cols)?;
        let ai = AiClient::from_config(&config.ai);
        Ok(Self {
            tabs: vec![tab],
            active_tab: 0,
            config,
            should_quit: false,
            ai,
            ai_modal: None,
        })
    }

    pub fn handle_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Quit => self.should_quit = true,
            Action::NewTab => self.new_tab()?,
            Action::ClosePane => self.close_active(),
            Action::NextTab => self.cycle_tab(1),
            Action::PrevTab => self.cycle_tab(-1),
            Action::Split(o) => self.split(o)?,
            Action::Focus(d) => {
                if let Some(t) = self.tabs.get_mut(self.active_tab) {
                    t.focus_pane(d);
                }
            }
            Action::SendInput(bytes) => self.send_input(&bytes)?,
            Action::Paste(text) => self.send_input(text.as_bytes())?,
            Action::OpenAiModal => self.open_ai_modal(),
        }
        Ok(())
    }

    fn open_ai_modal(&mut self) {
        if self.ai.is_none() {
            self.ai_modal = Some(AiModal {
                input: String::new(),
                state: AiModalState::Error(
                    "AI is disabled. Set [ai].enabled = true and provide an API key (config or ANTHROPIC_API_KEY env var)."
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

    pub fn handle_modal_key(&mut self, key: &KeyEvent) -> Result<()> {
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
            KeyCode::Backspace => {
                if matches!(modal.state, AiModalState::Editing | AiModalState::Error(_)) {
                    modal.input.pop();
                }
            }
            KeyCode::Char(c) => {
                if matches!(modal.state, AiModalState::Editing | AiModalState::Error(_)) {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        modal.input.push(c);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn poll_ai(&mut self) {
        let Some(modal) = self.ai_modal.as_mut() else {
            return;
        };
        let next = match &modal.state {
            AiModalState::InFlight(rx) => match rx.try_recv() {
                Ok(AiOutcome::Ok(text)) => Some(AiModalState::Result(text)),
                Ok(AiOutcome::Err(err)) => Some(AiModalState::Error(err)),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(AiModalState::Error(
                    "AI worker thread disconnected".into(),
                )),
            },
            _ => None,
        };
        if let Some(state) = next {
            modal.state = state;
        }
    }

    fn send_input(&mut self, bytes: &[u8]) -> Result<()> {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            if let Some(pty) = tab.active_pane_mut() {
                pty.write(bytes)?;
            }
        }
        Ok(())
    }

    fn new_tab(&mut self) -> Result<()> {
        let profile = pick_profile(&self.config)?;
        let (rows, cols) = self
            .tabs
            .get(self.active_tab)
            .and_then(|t| t.rects.get(&t.active).copied())
            .map(|r| (r.height.max(1), r.width.max(1)))
            .unwrap_or((24, 80));
        let tab = Tab::new(&profile, profile.name.clone(), rows, cols)?;
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        Ok(())
    }

    fn close_active(&mut self) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        let empty = tab.close_active();
        if empty {
            self.tabs.remove(self.active_tab);
            if self.tabs.is_empty() {
                self.should_quit = true;
                return;
            }
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    fn cycle_tab(&mut self, delta: i32) {
        if self.tabs.is_empty() {
            return;
        }
        let n = self.tabs.len() as i32;
        let new = (self.active_tab as i32 + delta).rem_euclid(n);
        self.active_tab = new as usize;
    }

    fn split(&mut self, orientation: SplitOrientation) -> Result<()> {
        let profile = pick_profile(&self.config)?;
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.split_active(&profile, orientation)?;
        }
        Ok(())
    }
}

fn pick_profile(config: &Config) -> Result<Profile> {
    config
        .pick_default_profile()
        .cloned()
        .ok_or_else(|| anyhow!("no profile defined in config"))
}

fn initial_pane_size(pane_area: Rect) -> (u16, u16) {
    let inner_h = pane_area.height.saturating_sub(2).max(1);
    let inner_w = pane_area.width.saturating_sub(2).max(1);
    (inner_h, inner_w)
}

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
