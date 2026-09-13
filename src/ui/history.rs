use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::ai::AiClient;
use crate::app::{HistoryModal, HistoryModalState};

use super::modal::{popup_area, provider_spans, render_input, render_separator};
use super::theme::Theme;

pub fn render(frame: &mut Frame, modal: &HistoryModal, ai: Option<&AiClient>, theme: &Theme) {
    let area = popup_area(frame.area(), 80, 60);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.focus_border)
        .title(" Command history search (Esc to close) ");
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

    let mut header = provider_spans(ai);
    if let Some(shell) = modal.shell {
        header.push(Span::styled(
            format!("  ·  shell: {}", shell.label()),
            Style::default().fg(Color::DarkGray),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(header)), chunks[0]);
    render_input(
        frame,
        chunks[1],
        "describe the command you ran before: ",
        &modal.input,
    );
    render_separator(frame, chunks[2]);
    render_body(frame, chunks[3], modal, ai.is_some());
    render_hint(frame, chunks[4], &modal.state);
}

fn render_body(frame: &mut Frame, area: Rect, modal: &HistoryModal, ai_enabled: bool) {
    match &modal.state {
        HistoryModalState::Editing => {
            let text = if ai_enabled {
                "  Press Enter to search."
            } else {
                "  AI is off: only words like docker or deploy.ps1 in your query are matched."
            };
            let p = Paragraph::new(text)
                .style(Style::default().fg(Color::Gray))
                .wrap(Wrap { trim: false });
            frame.render_widget(p, area);
        }
        HistoryModalState::InFlight(_) => {
            let p = Paragraph::new("  Asking for search terms…").style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(p, area);
        }
        HistoryModalState::Error(err) => {
            let p = Paragraph::new(format!("  error: {err}"))
                .style(Style::default().fg(Color::LightRed))
                .wrap(Wrap { trim: false });
            frame.render_widget(p, area);
        }
        HistoryModalState::Results {
            items,
            selected,
            terms,
            ai_used,
        } => render_results(frame, area, items, *selected, terms, *ai_used),
    }
}

fn render_results(
    frame: &mut Frame,
    area: Rect,
    items: &[String],
    selected: usize,
    terms: &[String],
    ai_used: bool,
) {
    let source = if ai_used {
        ""
    } else {
        "  (AI off: query words only)"
    };
    let mut lines = vec![Line::from(Span::styled(
        format!("  terms: {}{source}", terms.join(", ")),
        Style::default().fg(Color::DarkGray),
    ))];
    if items.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No matching commands.",
            Style::default().fg(Color::Gray),
        )));
    } else {
        let rows = (area.height as usize).saturating_sub(1).max(1);
        let offset = (selected + 1).saturating_sub(rows);
        for (i, item) in items.iter().enumerate().skip(offset).take(rows) {
            let line = if i == selected {
                Span::styled(
                    format!("  › {item}"),
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(format!("    {item}"), Style::default().fg(Color::White))
            };
            lines.push(Line::from(line));
        }
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_hint(frame: &mut Frame, area: Rect, state: &HistoryModalState) {
    let hint = match state {
        HistoryModalState::Editing => {
            " Enter: search  |  Tab: provider  |  Shift+Tab: model  |  Esc: cancel "
        }
        HistoryModalState::InFlight(_) => " (Esc: cancel) ",
        HistoryModalState::Results { .. } => {
            " ↑↓: select  |  Enter: insert into pane  |  type to refine  |  Esc: cancel "
        }
        HistoryModalState::Error(_) => " Enter: retry  |  Tab: provider  |  Esc: cancel ",
    };
    let p = Paragraph::new(hint).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(p, area);
}
