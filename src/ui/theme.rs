use ratatui::style::{Color, Modifier, Style};

use crate::config::{color_or, Theme as ConfigTheme};

pub struct Theme {
    pub status: Style,
    pub tab_active: Style,
    pub tab_inactive: Style,
    pub focus_border: Style,
    pub inactive_border: Style,
}

impl Theme {
    pub fn from_config(c: &ConfigTheme) -> Self {
        Self {
            status: Style::default()
                .fg(color_or(&c.status_fg, Color::Black))
                .bg(color_or(&c.status_bg, Color::Cyan))
                .add_modifier(Modifier::BOLD),
            tab_active: Style::default()
                .fg(color_or(&c.tab_active_fg, Color::Black))
                .bg(color_or(&c.tab_active_bg, Color::Cyan))
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default()
                .fg(color_or(&c.tab_inactive_fg, Color::Gray))
                .bg(color_or(&c.tab_inactive_bg, Color::Black)),
            focus_border: Style::default()
                .fg(color_or(&c.focus_border, Color::Yellow))
                .add_modifier(Modifier::BOLD),
            inactive_border: Style::default()
                .fg(color_or(&c.inactive_border, Color::DarkGray)),
        }
    }
}
