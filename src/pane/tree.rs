pub type PaneId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitOrientation {
    /// 上下に2分割 (Layout::Vertical を使う)
    Horizontal,
    /// 左右に2分割 (Layout::Horizontal を使う)
    Vertical,
}

#[derive(Debug)]
pub enum PaneTree {
    Leaf(PaneId),
    Split {
        orientation: SplitOrientation,
        /// first 側の割合 (1..=99)
        ratio: u16,
        first: Box<PaneTree>,
        second: Box<PaneTree>,
    },
}

impl PaneTree {
    pub fn leaf(id: PaneId) -> Self {
        PaneTree::Leaf(id)
    }

    pub fn first_leaf(&self) -> PaneId {
        match self {
            PaneTree::Leaf(id) => *id,
            PaneTree::Split { first, .. } => first.first_leaf(),
        }
    }

    #[allow(dead_code)]
    pub fn collect_leaves(&self, out: &mut Vec<PaneId>) {
        match self {
            PaneTree::Leaf(id) => out.push(*id),
            PaneTree::Split { first, second, .. } => {
                first.collect_leaves(out);
                second.collect_leaves(out);
            }
        }
    }

    /// `target` を見つけて、その葉を `Split{ leaf(target), leaf(new_id) }` で置き換える。
    pub fn split_leaf(
        &mut self,
        target: PaneId,
        new_id: PaneId,
        orientation: SplitOrientation,
    ) -> bool {
        match self {
            PaneTree::Leaf(id) if *id == target => {
                *self = PaneTree::Split {
                    orientation,
                    ratio: 50,
                    first: Box::new(PaneTree::Leaf(target)),
                    second: Box::new(PaneTree::Leaf(new_id)),
                };
                true
            }
            PaneTree::Leaf(_) => false,
            PaneTree::Split { first, second, .. } => {
                first.split_leaf(target, new_id, orientation)
                    || second.split_leaf(target, new_id, orientation)
            }
        }
    }

    /// `target` の葉を取り除き、Split を縮約する。
    /// 戻り値: 削除されたか (root が単葉で target と一致する場合は呼び出し側が処理する想定で false)
    pub fn remove_leaf(&mut self, target: PaneId) -> RemoveOutcome {
        match self {
            PaneTree::Leaf(id) if *id == target => RemoveOutcome::WasRoot,
            PaneTree::Leaf(_) => RemoveOutcome::NotFound,
            PaneTree::Split { first, second, .. } => {
                if matches!(first.as_ref(), PaneTree::Leaf(id) if *id == target) {
                    let surviving = std::mem::replace(second.as_mut(), PaneTree::Leaf(0));
                    *self = surviving;
                    return RemoveOutcome::Removed;
                }
                if matches!(second.as_ref(), PaneTree::Leaf(id) if *id == target) {
                    let surviving = std::mem::replace(first.as_mut(), PaneTree::Leaf(0));
                    *self = surviving;
                    return RemoveOutcome::Removed;
                }
                match first.remove_leaf(target) {
                    RemoveOutcome::NotFound => second.remove_leaf(target),
                    other => other,
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveOutcome {
    Removed,
    WasRoot,
    NotFound,
}
