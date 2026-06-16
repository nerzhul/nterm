#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusAxis {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn cx(&self) -> i32 {
        self.x + self.w / 2
    }
    pub const fn cy(&self) -> i32 {
        self.y + self.h / 2
    }
    pub const fn right(&self) -> i32 {
        self.x + self.w
    }
    pub const fn bottom(&self) -> i32 {
        self.y + self.h
    }
}

/// Pick the best candidate to focus next, given the current rect and an axis.
/// Scoring (byte-for-byte ported from the original gtk-dependent version):
///   - Horizontal (Left/Right): (dx.abs() * 1000) + dy.abs(),
///     where dx = candidate.cx - from.cx. Candidates with dx >= 0 are ignored for Left,
///     and candidates with dx <= 0 are ignored for Right. Y-overlap is required.
///   - Vertical (Up/Down): (dy.abs() * 1000) + dx.abs(),
///     where dy = candidate.cy - from.cy. Candidates with dy >= 0 are ignored for Up,
///     and candidates with dy <= 0 are ignored for Down. X-overlap is required.
///   - In case of a tie, the first candidate encountered wins.
pub fn find_best_focus<T: Clone>(from: &Rect, candidates: &[(Rect, T)], axis: FocusAxis) -> Option<T> {
    let from_cx = from.cx();
    let from_cy = from.cy();

    let mut best: Option<(i64, T)> = None;
    for (rect, term) in candidates {
        let cx = rect.cx();
        let cy = rect.cy();
        let dx = cx - from_cx;
        let dy = cy - from_cy;
        let candidate = match axis {
            FocusAxis::Left => {
                if dx >= 0 {
                    continue;
                }
                let y_overlap = rect.y < from.bottom() && from.y < rect.bottom();
                if !y_overlap {
                    continue;
                }
                (dx.abs() as i64) * 1000 + (dy.abs() as i64)
            }
            FocusAxis::Right => {
                if dx <= 0 {
                    continue;
                }
                let y_overlap = rect.y < from.bottom() && from.y < rect.bottom();
                if !y_overlap {
                    continue;
                }
                (dx.abs() as i64) * 1000 + (dy.abs() as i64)
            }
            FocusAxis::Up => {
                if dy >= 0 {
                    continue;
                }
                let x_overlap = rect.x < from.right() && from.x < rect.right();
                if !x_overlap {
                    continue;
                }
                (dy.abs() as i64) * 1000 + (dx.abs() as i64)
            }
            FocusAxis::Down => {
                if dy <= 0 {
                    continue;
                }
                let x_overlap = rect.x < from.right() && from.x < rect.right();
                if !x_overlap {
                    continue;
                }
                (dy.abs() as i64) * 1000 + (dx.abs() as i64)
            }
        };
        let is_better = match &best {
            Some((s, _)) => candidate < *s,
            None => true,
        };
        if is_better {
            best = Some((candidate, term.clone()));
        }
    }
    best.map(|(_, t)| t)
}

/// Pure recursive leaf counter driven by closures. Mirrors the GTK-dependent
/// `count_leaves` function on `Rc<RefCell<Pane>>` so the logic can be unit-tested
/// without GTK in scope.
pub fn count_leaves_rec<L, F, S>(is_leaf: L, first: Option<F>, second: Option<S>, depth: usize) -> usize
where
    L: Fn() -> bool,
    F: FnOnce() -> usize,
    S: FnOnce() -> usize,
{
    if depth > 1024 {
        return 0;
    }
    if is_leaf() {
        return 1;
    }
    match (first, second) {
        (Some(f), Some(s)) => f() + s(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn find_best_focus_left_picks_candidate_with_overlap() {
        let from = rect(100, 0, 20, 20);
        let candidates = vec![(rect(0, 0, 20, 20), "left")];
        assert_eq!(find_best_focus(&from, &candidates, FocusAxis::Left), Some("left"));
    }

    #[test]
    fn find_best_focus_right_no_overlap_returns_none() {
        let from = rect(0, 0, 20, 20);
        let candidates = vec![(rect(100, 100, 20, 20), "right")];
        assert_eq!(find_best_focus(&from, &candidates, FocusAxis::Right), None);
    }

    #[test]
    fn find_best_focus_picks_closest_among_multiple() {
        let from = rect(100, 0, 20, 20);
        let candidates = vec![(rect(0, 0, 20, 20), "far"), (rect(50, 0, 20, 20), "near")];
        assert_eq!(find_best_focus(&from, &candidates, FocusAxis::Left), Some("near"));
    }

    #[test]
    fn find_best_focus_up_ignores_candidates_below() {
        let from = rect(0, 100, 20, 20);
        let candidates = vec![(rect(0, 200, 20, 20), "below")];
        assert_eq!(find_best_focus(&from, &candidates, FocusAxis::Up), None);
    }

    #[test]
    fn count_leaves_rec_single_leaf() {
        let n = count_leaves_rec(|| true, None::<fn() -> usize>, None::<fn() -> usize>, 0);
        assert_eq!(n, 1);
    }

    #[test]
    fn count_leaves_rec_split_two_leaves() {
        let left = count_leaves_rec(|| true, None::<fn() -> usize>, None::<fn() -> usize>, 0);
        let right = count_leaves_rec(|| true, None::<fn() -> usize>, None::<fn() -> usize>, 0);
        let n = count_leaves_rec(
            || false,
            Some(move || left),
            Some(move || right),
            0,
        );
        assert_eq!(n, 2);
    }

    #[test]
    fn count_leaves_rec_recursive_split_three_leaves() {
        let leaf = || count_leaves_rec(|| true, None::<fn() -> usize>, None::<fn() -> usize>, 0);
        let split_with_two = count_leaves_rec(
            || false,
            Some(leaf),
            Some(leaf),
            0,
        );
        let total = count_leaves_rec(
            || false,
            Some(move || split_with_two),
            Some(leaf),
            0,
        );
        assert_eq!(total, 3);
    }
}
