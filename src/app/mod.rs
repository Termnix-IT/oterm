mod history_modal;
mod modal;
mod run;

use anyhow::{anyhow, Result};
use ratatui::layout::Rect;

use crate::ai::AiClient;
use crate::config::{Config, Profile};
use crate::keymap::Action;
use crate::pane::{SplitOrientation, Tab};

pub use history_modal::{HistoryModal, HistoryModalState};
pub use modal::{AiModal, AiModalState};
pub use run::run;

pub struct App {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub config: Config,
    pub should_quit: bool,
    pub ai: Option<AiClient>,
    pub ai_modal: Option<AiModal>,
    pub history_modal: Option<HistoryModal>,
}

impl App {
    pub fn new(config: Config, pane_area: Rect) -> Result<Self> {
        let profile = pick_profile(&config)?;
        let (rows, cols) = initial_pane_size(pane_area);
        let tab = Tab::new(&profile, profile.name.clone(), rows, cols)?;
        let ai = AiClient::from_config(&config.ai, config.anthropic_api_key());
        Ok(Self {
            tabs: vec![tab],
            active_tab: 0,
            config,
            should_quit: false,
            ai,
            ai_modal: None,
            history_modal: None,
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
            Action::OpenHistoryModal => self.open_history_modal(),
        }
        Ok(())
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
