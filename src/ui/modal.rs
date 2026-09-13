use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::ai::AiClient;

pub(super) fn popup_area(parent: Rect, percent_x: u16, percent_y: u16) -> Rect {
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

pub(super) fn provider_spans(ai: Option<&AiClient>) -> Vec<Span<'static>> {
    match ai {
        Some(client) => vec![
            Span::styled("  provider: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                client.status_label(),
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ],
        None => vec![Span::styled(
            "  provider: unavailable",
            Style::default().fg(Color::DarkGray),
        )],
    }
}

pub(super) fn render_input(frame: &mut Frame, area: Rect, label: &str, input: &str) {
    let label = Line::from(Span::styled(
        format!("  ▎ {label}"),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    let input = Line::from(vec![
        Span::raw("  > "),
        Span::styled(input, Style::default().fg(Color::White)),
        Span::styled(
            "▏",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::SLOW_BLINK),
        ),
    ]);
    let p = Paragraph::new(vec![label, Line::raw(""), input]).wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

pub(super) fn render_separator(frame: &mut Frame, area: Rect) {
    let line = "─".repeat(area.width as usize);
    let p = Paragraph::new(line).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(p, area);
}
