//! Incremental revision graph layout.
//!
//! Commits are fed in topological order (children before parents, as produced by
//! `git log --topo-order`). The layout keeps a list of *lanes*; every lane waits for
//! one specific commit (the parent that a previously seen child points at). The state
//! is small and serializable, so a page of rows can be laid out from the state returned
//! with the previous page - the backend stays stateless.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneState {
    /// Hash of the commit this lane is waiting for.
    pub hash: String,
    /// Palette index of the lane (stable for the lifetime of the lane).
    pub color: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphState {
    pub lanes: Vec<Option<LaneState>>,
    pub next_color: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineKind {
    /// A lane that continues straight through the row (`from == to`).
    Pass,
    /// From the top edge at lane `from` to the commit node at lane `to`.
    In,
    /// From the commit node at lane `from` to the bottom edge at lane `to`.
    Out,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphLine {
    pub kind: LineKind,
    pub from: usize,
    pub to: usize,
    pub color: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphRow {
    pub node_lane: usize,
    pub color: u32,
    pub lines: Vec<GraphLine>,
    /// Number of lanes needed to draw this row.
    pub width: usize,
}

fn first_free(lanes: &[Option<LaneState>], skip: Option<usize>) -> Option<usize> {
    lanes
        .iter()
        .enumerate()
        .find(|(i, l)| l.is_none() && Some(*i) != skip)
        .map(|(i, _)| i)
}

fn allocate(lanes: &mut Vec<Option<LaneState>>, skip: Option<usize>) -> usize {
    match first_free(lanes, skip) {
        Some(i) => i,
        None => {
            lanes.push(None);
            lanes.len() - 1
        }
    }
}

/// Lays out one commit and advances the state to the bottom edge of its row.
pub fn layout_row(state: &mut GraphState, hash: &str, parents: &[String]) -> GraphRow {
    let before = state.lanes.clone();
    let matching: Vec<usize> = before
        .iter()
        .enumerate()
        .filter(|(_, l)| l.as_ref().map_or(false, |l| l.hash == hash))
        .map(|(i, _)| i)
        .collect();

    let (node_lane, color) = match matching.first() {
        Some(&first) => (first, before[first].as_ref().map(|l| l.color).unwrap_or(0)),
        None => {
            // A branch tip: nothing above waits for this commit.
            let lane = allocate(&mut state.lanes, None);
            let color = state.next_color;
            state.next_color += 1;
            (lane, color)
        }
    };

    let mut lines = Vec::new();
    for (i, lane) in before.iter().enumerate() {
        if let Some(l) = lane {
            if matching.contains(&i) {
                lines.push(GraphLine { kind: LineKind::In, from: i, to: node_lane, color: l.color });
            } else {
                lines.push(GraphLine { kind: LineKind::Pass, from: i, to: i, color: l.color });
            }
        }
    }

    // Every lane that was waiting for this commit ends here.
    for &i in &matching {
        state.lanes[i] = None;
    }

    for (k, parent) in parents.iter().enumerate() {
        if k == 0 {
            state.lanes[node_lane] = Some(LaneState { hash: parent.clone(), color });
            lines.push(GraphLine { kind: LineKind::Out, from: node_lane, to: node_lane, color });
            continue;
        }
        // Additional parent (merge): reuse a lane that already waits for it, else open one.
        let existing = state
            .lanes
            .iter()
            .position(|l| l.as_ref().map_or(false, |l| &l.hash == parent));
        match existing {
            Some(q) => {
                let c = state.lanes[q].as_ref().map(|l| l.color).unwrap_or(color);
                lines.push(GraphLine { kind: LineKind::Out, from: node_lane, to: q, color: c });
            }
            None => {
                let q = allocate(&mut state.lanes, Some(node_lane));
                let c = state.next_color;
                state.next_color += 1;
                state.lanes[q] = Some(LaneState { hash: parent.clone(), color: c });
                lines.push(GraphLine { kind: LineKind::Out, from: node_lane, to: q, color: c });
            }
        }
    }

    while matches!(state.lanes.last(), Some(None)) {
        state.lanes.pop();
    }

    let width = before.len().max(state.lanes.len()).max(node_lane + 1);
    GraphRow { node_lane, color, lines, width }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn kinds(row: &GraphRow, kind: LineKind) -> Vec<(usize, usize)> {
        row.lines.iter().filter(|l| l.kind == kind).map(|l| (l.from, l.to)).collect()
    }

    #[test]
    fn linear_history_stays_in_lane_zero() {
        let mut s = GraphState::default();
        let r1 = layout_row(&mut s, "c", &p(&["b"]));
        let r2 = layout_row(&mut s, "b", &p(&["a"]));
        let r3 = layout_row(&mut s, "a", &[]);
        assert_eq!((r1.node_lane, r2.node_lane, r3.node_lane), (0, 0, 0));
        assert_eq!(kinds(&r1, LineKind::Out), vec![(0, 0)]);
        assert_eq!(kinds(&r2, LineKind::In), vec![(0, 0)]);
        assert!(kinds(&r3, LineKind::Out).is_empty());
        assert!(s.lanes.is_empty());
        // The whole history uses a single color.
        assert_eq!((r1.color, r2.color, r3.color), (0, 0, 0));
    }

    #[test]
    fn branch_and_merge() {
        // m (merge of b and d), b -> a, d -> a   (topo order: m, b, d, a)
        let mut s = GraphState::default();
        let m = layout_row(&mut s, "m", &p(&["b", "d"]));
        assert_eq!(m.node_lane, 0);
        assert_eq!(kinds(&m, LineKind::Out), vec![(0, 0), (0, 1)]);
        assert_eq!(s.lanes.len(), 2);

        let b = layout_row(&mut s, "b", &p(&["a"]));
        assert_eq!(b.node_lane, 0);
        assert_eq!(kinds(&b, LineKind::Pass), vec![(1, 1)]);

        let d = layout_row(&mut s, "d", &p(&["a"]));
        assert_eq!(d.node_lane, 1);
        assert_eq!(kinds(&d, LineKind::Pass), vec![(0, 0)]);

        // Both lanes wait for `a`: they converge on it.
        let a = layout_row(&mut s, "a", &[]);
        assert_eq!(a.node_lane, 0);
        assert_eq!(kinds(&a, LineKind::In), vec![(0, 0), (1, 0)]);
        assert!(s.lanes.is_empty());
        assert_eq!(a.width, 2);
    }

    #[test]
    fn second_branch_tip_opens_new_lane() {
        // Two tips x and y share the parent a.
        let mut s = GraphState::default();
        let x = layout_row(&mut s, "x", &p(&["a"]));
        let y = layout_row(&mut s, "y", &p(&["a"]));
        assert_eq!((x.node_lane, y.node_lane), (0, 1));
        assert_ne!(x.color, y.color);
        let a = layout_row(&mut s, "a", &[]);
        assert_eq!(kinds(&a, LineKind::In), vec![(0, 0), (1, 0)]);
    }

    #[test]
    fn merge_reuses_existing_lane_for_second_parent() {
        // t has parent a on lane 0; m merges (b, a): `a` is already awaited by lane 0.
        let mut s = GraphState::default();
        layout_row(&mut s, "t", &p(&["a"]));
        let m = layout_row(&mut s, "m", &p(&["b", "a"]));
        // m is a new tip on lane 1 (lane 0 waits for `a`), its second parent joins lane 0.
        assert_eq!(m.node_lane, 1);
        assert!(kinds(&m, LineKind::Out).contains(&(1, 0)));
        assert_eq!(s.lanes.len(), 2);
    }

    #[test]
    fn octopus_merge_opens_lane_per_parent() {
        let mut s = GraphState::default();
        let m = layout_row(&mut s, "m", &p(&["a", "b", "c"]));
        assert_eq!(kinds(&m, LineKind::Out), vec![(0, 0), (0, 1), (0, 2)]);
        assert_eq!(s.lanes.len(), 3);
    }

    #[test]
    fn state_roundtrips_across_pages() {
        let mut full = GraphState::default();
        let rows_full: Vec<_> = [("m", p(&["b", "d"])), ("b", p(&["a"])), ("d", p(&["a"])), ("a", vec![])]
            .iter()
            .map(|(h, ps)| layout_row(&mut full, h, ps))
            .collect();

        // Same layout in two pages, passing the serialized state through.
        let mut first = GraphState::default();
        layout_row(&mut first, "m", &p(&["b", "d"]));
        layout_row(&mut first, "b", &p(&["a"]));
        let json = serde_json::to_string(&first).unwrap();
        let mut second: GraphState = serde_json::from_str(&json).unwrap();
        let d = layout_row(&mut second, "d", &p(&["a"]));
        let a = layout_row(&mut second, "a", &[]);
        assert_eq!(d, rows_full[2]);
        assert_eq!(a, rows_full[3]);
    }
}
