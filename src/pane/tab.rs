use std::collections::HashMap;

use anyhow::Result;
use ratatui::layout::Rect;

use crate::config::Profile;
use crate::pty::Pty;

use super::tree::{PaneId, PaneTree, RemoveOutcome, SplitOrientation};

#[derive(Debug, Clone, Copy)]
pub enum FocusDir {
    Left,
    Right,
    Up,
    Down,
}

pub struct Tab {
    pub title: String,
    pub root: PaneTree,
    pub panes: HashMap<PaneId, Pty>,
    pub rects: HashMap<PaneId, Rect>,
    pub active: PaneId,
    next_id: PaneId,
}

impl Tab {
    pub fn new(profile: &Profile, title: String, rows: u16, cols: u16) -> Result<Self> {
        let id: PaneId = 1;
        let pty = Pty::spawn(profile, rows.max(1), cols.max(1))?;
        let mut panes = HashMap::new();
        panes.insert(id, pty);
        Ok(Self {
            title,
            root: PaneTree::leaf(id),
            panes,
            rects: HashMap::new(),
            active: id,
            next_id: id + 1,
        })
    }

    pub fn split_active(
        &mut self,
        profile: &Profile,
        orientation: SplitOrientation,
    ) -> Result<()> {
        let active_rect = self.rects.get(&self.active).copied();
        let (rows, cols) = match (active_rect, orientation) {
            (Some(r), SplitOrientation::Horizontal) => ((r.height / 2).max(1), r.width.max(1)),
            (Some(r), SplitOrientation::Vertical) => (r.height.max(1), (r.width / 2).max(1)),
            (None, _) => (24, 80),
        };
        let new_id = self.next_id;
        self.next_id += 1;
        let pty = Pty::spawn(profile, rows, cols)?;
        if !self
            .root
            .split_leaf(self.active, new_id, orientation)
        {
            return Err(anyhow::anyhow!("active pane not found in tree"));
        }
        self.panes.insert(new_id, pty);
        self.active = new_id;
        Ok(())
    }

    /// アクティブペインを閉じる。タブが空になったら true。
    pub fn close_active(&mut self) -> bool {
        let target = self.active;
        match self.root.remove_leaf(target) {
            RemoveOutcome::Removed => {
                self.panes.remove(&target);
                self.rects.remove(&target);
                self.active = self.root.first_leaf();
                false
            }
            RemoveOutcome::WasRoot => {
                self.panes.remove(&target);
                self.rects.remove(&target);
                true
            }
            RemoveOutcome::NotFound => false,
        }
    }

    pub fn active_pane_mut(&mut self) -> Option<&mut Pty> {
        self.panes.get_mut(&self.active)
    }

    pub fn focus_pane(&mut self, dir: FocusDir) {
        let Some(current) = self.rects.get(&self.active).copied() else {
            return;
        };
        let cx = current.x as i32 + current.width as i32 / 2;
        let cy = current.y as i32 + current.height as i32 / 2;

        let mut best: Option<(PaneId, i32)> = None;
        for (&id, rect) in &self.rects {
            if id == self.active {
                continue;
            }
            let rx = rect.x as i32 + rect.width as i32 / 2;
            let ry = rect.y as i32 + rect.height as i32 / 2;
            let dx = rx - cx;
            let dy = ry - cy;
            let in_dir = match dir {
                FocusDir::Left => dx < 0 && dx.abs() >= dy.abs(),
                FocusDir::Right => dx > 0 && dx.abs() >= dy.abs(),
                FocusDir::Up => dy < 0 && dy.abs() >= dx.abs(),
                FocusDir::Down => dy > 0 && dy.abs() >= dx.abs(),
            };
            if !in_dir {
                continue;
            }
            let dist = dx * dx + dy * dy;
            if best.map_or(true, |(_, d)| dist < d) {
                best = Some((id, dist));
            }
        }
        if let Some((id, _)) = best {
            self.active = id;
        }
    }

    pub fn record_rect(&mut self, id: PaneId, rect: Rect) {
        self.rects.insert(id, rect);
    }

}
