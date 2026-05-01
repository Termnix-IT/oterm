use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::pane::{PaneId, PaneTree, SplitOrientation, Tab};

use super::terminal;
use super::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, tab: &mut Tab, theme: &Theme) {
    let mut layout = Vec::new();
    collect_layout(area, &tab.root, &mut layout);

    for (id, rect) in &layout {
        tab.record_rect(*id, *rect);
    }

    let active = tab.active;
    for (id, rect) in &layout {
        if let Some(pty) = tab.panes.get(id) {
            let inner = inner_area(*rect);
            ensure_pty_size(pty, inner);
            render_pane(frame, *rect, inner, *id, pty, theme, *id == active);
        }
    }
}

fn collect_layout(area: Rect, tree: &PaneTree, out: &mut Vec<(PaneId, Rect)>) {
    match tree {
        PaneTree::Leaf(id) => out.push((*id, area)),
        PaneTree::Split {
            orientation,
            ratio,
            first,
            second,
        } => {
            let direction = match orientation {
                SplitOrientation::Horizontal => Direction::Vertical,
                SplitOrientation::Vertical => Direction::Horizontal,
            };
            let first_pct = (*ratio).clamp(1, 99);
            let second_pct = 100 - first_pct;
            let chunks = Layout::default()
                .direction(direction)
                .constraints([
                    Constraint::Percentage(first_pct as u16),
                    Constraint::Percentage(second_pct as u16),
                ])
                .split(area);
            collect_layout(chunks[0], first, out);
            collect_layout(chunks[1], second, out);
        }
    }
}

fn inner_area(rect: Rect) -> Rect {
    Block::default().borders(Borders::ALL).inner(rect)
}

fn ensure_pty_size(pty: &crate::pty::Pty, inner: Rect) {
    let parser = pty.parser();
    let target_rows = inner.height.max(1);
    let target_cols = inner.width.max(1);
    let need_resize = {
        match parser.lock() {
            Ok(p) => {
                let (rows, cols) = p.screen().size();
                rows != target_rows || cols != target_cols
            }
            Err(_) => false,
        }
    };
    if need_resize {
        let _ = pty.resize(target_rows, target_cols);
    }
}

fn render_pane(
    frame: &mut Frame,
    rect: Rect,
    inner: Rect,
    id: PaneId,
    pty: &crate::pty::Pty,
    theme: &Theme,
    focused: bool,
) {
    let border_style = if focused {
        theme.focus_border
    } else {
        theme.inactive_border
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(" pane #{id} "));
    frame.render_widget(block, rect);
    terminal::render_with_cursor(frame, inner, pty, focused);
}
