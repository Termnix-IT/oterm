use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::ai::AiClient;
use crate::app::{AiModal, AiModalState};

use super::modal::{popup_area, provider_spans, render_input, render_separator};
use super::theme::Theme;

pub fn render(frame: &mut Frame, modal: &AiModal, ai: Option<&AiClient>, theme: &Theme) {
    let area = popup_area(frame.area(), 70, 50);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.focus_border)
        .title(" AI command suggestion (Esc to close) ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new(Line::from(provider_spans(ai))), chunks[0]);
    render_input(frame, chunks[1], "describe what you want: ", &modal.input);
    render_separator(frame, chunks[2]);
    render_body(frame, chunks[3], modal);
    render_hint(frame, chunks[4], modal);
}

fn render_body(frame: &mut Frame, area: Rect, modal: &AiModal) {
    let (text, style) = match &modal.state {
        AiModalState::Editing => (
            "  Press Enter to send.".to_string(),
            Style::default().fg(Color::Gray),
        ),
        AiModalState::InFlight(_) => (
            "  Thinking…".to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        AiModalState::Result(text) => (
            format!("  $ {text}"),
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ),
        AiModalState::Error(err) => (
            format!("  error: {err}"),
            Style::default().fg(Color::LightRed),
        ),
    };
    let p = Paragraph::new(text).style(style).wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn render_hint(frame: &mut Frame, area: Rect, modal: &AiModal) {
    let hint = match &modal.state {
        AiModalState::Editing => {
            " Enter: send  |  Tab: provider  |  Shift+Tab: model  |  Esc: cancel "
        }
        AiModalState::InFlight(_) => " (Esc: cancel) ",
        AiModalState::Result(_) => " Enter: insert into pane  |  Esc: cancel ",
        AiModalState::Error(_) => {
            " Enter: retry  |  Tab: provider  |  Shift+Tab: model  |  Esc: cancel "
        }
    };
    let p = Paragraph::new(hint).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(p, area);
}
