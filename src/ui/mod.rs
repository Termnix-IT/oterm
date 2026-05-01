pub mod ai;
pub mod panes;
pub mod tabs;
pub mod terminal;
pub mod theme;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

use self::theme::Theme;

pub fn render(frame: &mut Frame, app: &mut App) {
    let theme = Theme::from_config(&app.config.theme);
    let chunks = layout(frame.area());
    let tab_bar = chunks[0];
    let main = chunks[1];
    let status = chunks[2];

    tabs::render(frame, tab_bar, &app.tabs, app.active_tab, &theme);

    if let Some(tab) = app.tabs.get_mut(app.active_tab) {
        panes::render(frame, main, tab, &theme);
    }

    render_status(frame, status, app, &theme);

    if let Some(modal) = &app.ai_modal {
        ai::render(frame, modal, &theme);
    }
}

pub fn pane_area(frame_area: Rect) -> Rect {
    layout(frame_area)[1]
}

fn layout(area: Rect) -> [Rect; 3] {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);
    [chunks[0], chunks[1], chunks[2]]
}

fn render_status(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let tab = app.tabs.get(app.active_tab);
    let pane_id = tab.map(|t| t.active).unwrap_or(0);
    let pane_count = tab.map(|t| t.panes.len()).unwrap_or(0);
    let ai_label = if app.ai.is_some() { "AI:on" } else { "AI:off" };
    let text = format!(
        " oterm  |  tab {}/{}  |  pane #{} ({} panes)  |  {}  |  Ctrl+Shift+T new  D|E split  Alt+arrow focus  Ctrl+Shift+W close  Ctrl+Space AI  Ctrl+Q quit ",
        app.active_tab + 1,
        app.tabs.len(),
        pane_id,
        pane_count,
        ai_label,
    );
    let widget = Paragraph::new(text).style(theme.status);
    frame.render_widget(widget, area);
}
