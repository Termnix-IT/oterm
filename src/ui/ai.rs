use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{AiModal, AiModalState};

use super::theme::Theme;

pub fn render(frame: &mut Frame, modal: &AiModal, theme: &Theme) {
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
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    render_input(frame, chunks[0], modal);
    render_separator(frame, chunks[1]);
    render_body(frame, chunks[2], modal);
    render_hint(frame, chunks[3], modal);
}

fn render_input(frame: &mut Frame, area: Rect, modal: &AiModal) {
    let label = Line::from(vec![
        Span::styled(
            "  ▎ describe what you want: ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
    ]);
    let input = Line::from(vec![
        Span::raw("  > "),
        Span::styled(&modal.input, Style::default().fg(Color::White)),
        Span::styled("▏", Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK)),
    ]);
    let p = Paragraph::new(vec![label, Line::raw(""), input]).wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn render_separator(frame: &mut Frame, area: Rect) {
    let line = "─".repeat(area.width as usize);
    let p = Paragraph::new(line).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(p, area);
}

fn render_body(frame: &mut Frame, area: Rect, modal: &AiModal) {
    let (text, style) = match &modal.state {
        AiModalState::Editing => (
            "  Press Enter to ask Claude.".to_string(),
            Style::default().fg(Color::Gray),
        ),
        AiModalState::InFlight(_) => (
            "  Thinking…".to_string(),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
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
        AiModalState::Editing => " Enter: send  |  Esc: cancel ",
        AiModalState::InFlight(_) => " (Esc: cancel) ",
        AiModalState::Result(_) => " Enter: insert into pane  |  Esc: cancel ",
        AiModalState::Error(_) => " Enter: retry  |  Esc: cancel ",
    };
    let p = Paragraph::new(hint).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(p, area);
}

fn popup_area(parent: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(parent);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}
