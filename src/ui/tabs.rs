use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Tabs;
use ratatui::Frame;

use crate::pane::Tab;

use super::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, tabs: &[Tab], active: usize, theme: &Theme) {
    let titles: Vec<Line> = tabs
        .iter()
        .enumerate()
        .map(|(i, t)| Line::from(format!(" {}: {} ", i + 1, t.title)))
        .collect();
    let widget = Tabs::new(titles)
        .select(active)
        .style(theme.tab_inactive)
        .highlight_style(theme.tab_active)
        .divider("│");
    frame.render_widget(widget, area);
}
