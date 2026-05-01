use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

use crate::pty::Pty;

pub fn render_with_cursor(frame: &mut Frame, area: Rect, pty: &Pty, show_cursor: bool) {
    let parser = pty.parser();
    let parser = parser.lock().expect("vt100 parser mutex poisoned");
    let screen = parser.screen();

    let (rows, cols) = screen.size();
    let max_row = rows.min(area.height);
    let max_col = cols.min(area.width);

    let buf = frame.buffer_mut();
    for row in 0..max_row {
        for col in 0..max_col {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            let x = area.x + col;
            let y = area.y + row;
            let Some(buf_cell) = buf.cell_mut((x, y)) else {
                continue;
            };
            let contents = cell.contents();
            if contents.is_empty() {
                buf_cell.set_symbol(" ");
            } else {
                buf_cell.set_symbol(&contents);
            }
            buf_cell.set_style(cell_style(cell));
        }
    }

    if show_cursor && !screen.hide_cursor() {
        let (cur_row, cur_col) = screen.cursor_position();
        let cx = area.x + cur_col.min(area.width.saturating_sub(1));
        let cy = area.y + cur_row.min(area.height.saturating_sub(1));
        frame.set_cursor_position(Position::new(cx, cy));
    }
}

fn cell_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default();
    if let Some(fg) = convert_color(cell.fgcolor()) {
        style = style.fg(fg);
    }
    if let Some(bg) = convert_color(cell.bgcolor()) {
        style = style.bg(bg);
    }
    let mut m = Modifier::empty();
    if cell.bold() {
        m |= Modifier::BOLD;
    }
    if cell.italic() {
        m |= Modifier::ITALIC;
    }
    if cell.underline() {
        m |= Modifier::UNDERLINED;
    }
    if cell.inverse() {
        m |= Modifier::REVERSED;
    }
    if cell.dim() {
        m |= Modifier::DIM;
    }
    style.add_modifier(m)
}

fn convert_color(c: vt100::Color) -> Option<Color> {
    match c {
        vt100::Color::Default => None,
        vt100::Color::Idx(i) => Some(Color::Indexed(i)),
        vt100::Color::Rgb(r, g, b) => Some(Color::Rgb(r, g, b)),
    }
}
