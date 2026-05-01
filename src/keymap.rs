use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::pane::{FocusDir, SplitOrientation};

#[derive(Debug)]
pub enum Action {
    Quit,
    NewTab,
    ClosePane,
    NextTab,
    PrevTab,
    Split(SplitOrientation),
    Focus(FocusDir),
    SendInput(Vec<u8>),
    Paste(String),
    OpenAiModal,
}

pub fn map_key(key: &KeyEvent) -> Option<Action> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    let mods = key.modifiers;
    let ctrl = mods.contains(KeyModifiers::CONTROL);
    let shift = mods.contains(KeyModifiers::SHIFT);
    let alt = mods.contains(KeyModifiers::ALT);

    if ctrl && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
        return Some(Action::Quit);
    }

    if ctrl && key.code == KeyCode::Char(' ') {
        return Some(Action::OpenAiModal);
    }

    if ctrl && shift {
        match key.code {
            KeyCode::Char('t') | KeyCode::Char('T') => return Some(Action::NewTab),
            KeyCode::Char('w') | KeyCode::Char('W') => return Some(Action::ClosePane),
            KeyCode::Char('d') | KeyCode::Char('D') => {
                return Some(Action::Split(SplitOrientation::Vertical));
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                return Some(Action::Split(SplitOrientation::Horizontal));
            }
            _ => {}
        }
    }

    if ctrl && key.code == KeyCode::Tab {
        return Some(if shift { Action::PrevTab } else { Action::NextTab });
    }
    if ctrl && key.code == KeyCode::BackTab {
        return Some(Action::PrevTab);
    }

    if alt && !ctrl && !shift {
        match key.code {
            KeyCode::Left => return Some(Action::Focus(FocusDir::Left)),
            KeyCode::Right => return Some(Action::Focus(FocusDir::Right)),
            KeyCode::Up => return Some(Action::Focus(FocusDir::Up)),
            KeyCode::Down => return Some(Action::Focus(FocusDir::Down)),
            _ => {}
        }
    }

    key_to_bytes(key).map(Action::SendInput)
}

fn key_to_bytes(key: &KeyEvent) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let mut out: Vec<u8> = Vec::new();

    match key.code {
        KeyCode::Char(c) => {
            if ctrl {
                let lc = c.to_ascii_lowercase();
                if lc.is_ascii_lowercase() {
                    let code = (lc as u8) - b'a' + 1;
                    if alt {
                        out.push(0x1b);
                    }
                    out.push(code);
                    return Some(out);
                }
                if c == ' ' {
                    if alt {
                        out.push(0x1b);
                    }
                    out.push(0x00);
                    return Some(out);
                }
            }
            if alt {
                out.push(0x1b);
            }
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            out.extend_from_slice(s.as_bytes());
        }
        KeyCode::Enter => out.push(b'\r'),
        KeyCode::Backspace => out.push(0x7f),
        KeyCode::Tab => out.push(b'\t'),
        KeyCode::BackTab => out.extend_from_slice(b"\x1b[Z"),
        KeyCode::Esc => out.push(0x1b),
        KeyCode::Left => out.extend_from_slice(b"\x1b[D"),
        KeyCode::Right => out.extend_from_slice(b"\x1b[C"),
        KeyCode::Up => out.extend_from_slice(b"\x1b[A"),
        KeyCode::Down => out.extend_from_slice(b"\x1b[B"),
        KeyCode::Home => out.extend_from_slice(b"\x1b[H"),
        KeyCode::End => out.extend_from_slice(b"\x1b[F"),
        KeyCode::PageUp => out.extend_from_slice(b"\x1b[5~"),
        KeyCode::PageDown => out.extend_from_slice(b"\x1b[6~"),
        KeyCode::Insert => out.extend_from_slice(b"\x1b[2~"),
        KeyCode::Delete => out.extend_from_slice(b"\x1b[3~"),
        KeyCode::F(n) => {
            let seq: &[u8] = match n {
                1 => b"\x1bOP",
                2 => b"\x1bOQ",
                3 => b"\x1bOR",
                4 => b"\x1bOS",
                5 => b"\x1b[15~",
                6 => b"\x1b[17~",
                7 => b"\x1b[18~",
                8 => b"\x1b[19~",
                9 => b"\x1b[20~",
                10 => b"\x1b[21~",
                11 => b"\x1b[23~",
                12 => b"\x1b[24~",
                _ => return None,
            };
            out.extend_from_slice(seq);
        }
        _ => return None,
    }

    Some(out)
}
