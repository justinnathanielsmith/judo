use crate::domain::models::{CommitId, GraphRow};
use std::collections::HashMap;

pub fn calculate_graph_layout(rows: &mut [GraphRow]) {
    if rows.is_empty() {
        return;
    }

    let mut active_lanes: Vec<Option<CommitId>> = Vec::new();
    let mut commit_to_lane: HashMap<CommitId, usize> = HashMap::new();

    // Pass 1: Simple column assignment and active/connector lane tracking
    for row in rows.iter_mut() {
        let commit_id = row.commit_id.clone();

        // 1. Assign/Find lane for this commit
        let lane_idx = if let Some(&idx) = commit_to_lane.get(&commit_id) {
            idx
        } else {
            // New head (youngest commit of a new branch)
            // Find first empty slot or push
            let idx = if let Some(empty_idx) = active_lanes.iter().position(|l| l.is_none()) {
                active_lanes[empty_idx] = Some(commit_id.clone());
                empty_idx
            } else {
                active_lanes.push(Some(commit_id.clone()));
                active_lanes.len() - 1
            };
            commit_to_lane.insert(commit_id.clone(), idx);
            idx
        };

        row.visual.column = lane_idx;
        row.visual.active_lanes = active_lanes.iter().map(|l| l.is_some()).collect();

        // 2. Prepare for parents
        // Remove current commit from its lane
        active_lanes[lane_idx] = None;
        commit_to_lane.remove(&commit_id);

        // 3. Add parents to lanes
        let mut parent_columns = Vec::new();
        for parent_id in &row.parents {
            let p_lane = if let Some(&idx) = commit_to_lane.get(parent_id) {
                idx
            } else {
                // First time we see this parent (it's the next commit in a branch)
                // Try to put it in the same lane as the current commit if possible
                let idx = if active_lanes.get(lane_idx).is_none() {
                    active_lanes[lane_idx] = Some(parent_id.clone());
                    lane_idx
                } else if let Some(empty_idx) = active_lanes.iter().position(|l| l.is_none()) {
                    active_lanes[empty_idx] = Some(parent_id.clone());
                    empty_idx
                } else {
                    active_lanes.push(Some(parent_id.clone()));
                    active_lanes.len() - 1
                };
                commit_to_lane.insert(parent_id.clone(), idx);
                idx
            };
            parent_columns.push(p_lane);
        }

        row.visual.parent_columns = parent_columns.clone();
        if !parent_columns.is_empty() {
            row.visual.parent_min = *parent_columns.iter().min().unwrap_or(&lane_idx).min(&lane_idx);
            row.visual.parent_max = *parent_columns.iter().max().unwrap_or(&lane_idx).max(&lane_idx);
        } else {
            row.visual.parent_min = lane_idx;
            row.visual.parent_max = lane_idx;
        }

        row.visual.connector_lanes = active_lanes.iter().map(|l| l.is_some()).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{CommitId, GraphRow};

    #[test]
    fn test_simple_linear_layout() {
        let mut rows = vec![
            GraphRow {
                commit_id: CommitId("c2".to_string()),
                parents: vec![CommitId("c1".to_string())],
                ..Default::default()
            },
            GraphRow {
                commit_id: CommitId("c1".to_string()),
                parents: vec![CommitId("c0".to_string())],
                ..Default::default()
            },
            GraphRow {
                commit_id: CommitId("c0".to_string()),
                parents: vec![],
                ..Default::default()
            },
        ];

        calculate_graph_layout(&mut rows);

        assert_eq!(rows[0].visual.column, 0);
        assert_eq!(rows[0].visual.active_lanes, vec![true]);
        assert_eq!(rows[0].visual.connector_lanes, vec![true]);
        assert_eq!(rows[0].visual.parent_columns, vec![0]);

        assert_eq!(rows[1].visual.column, 0);
        assert_eq!(rows[1].visual.active_lanes, vec![true]);
        assert_eq!(rows[1].visual.connector_lanes, vec![true]);
        assert_eq!(rows[1].visual.parent_columns, vec![0]);

        assert_eq!(rows[2].visual.column, 0);
        assert_eq!(rows[2].visual.active_lanes, vec![true]);
        assert_eq!(rows[2].visual.connector_lanes, vec![false]);
        assert_eq!(rows[2].visual.parent_columns, vec![]);
    }

    #[test]
    fn test_branch_layout() {
        // c2 (parent c1)
        // c3 (parent c1)
        // c1 (parent c0)
        let mut rows = vec![
            GraphRow {
                commit_id: CommitId("c3".to_string()),
                parents: vec![CommitId("c1".to_string())],
                ..Default::default()
            },
            GraphRow {
                commit_id: CommitId("c2".to_string()),
                parents: vec![CommitId("c1".to_string())],
                ..Default::default()
            },
            GraphRow {
                commit_id: CommitId("c1".to_string()),
                parents: vec![CommitId("c0".to_string())],
                ..Default::default()
            },
        ];

        calculate_graph_layout(&mut rows);

        // c3 starts at lane 0
        assert_eq!(rows[0].visual.column, 0);
        assert_eq!(rows[0].visual.parent_columns, vec![0]);
        // c2 starts at lane 1
        assert_eq!(rows[1].visual.column, 1);
        // c2's parent c1 is already in lane 0 (from c3), so parent_columns should be [0]
        assert_eq!(rows[1].visual.parent_columns, vec![0]);
        
        // c1 is at lane 0
        assert_eq!(rows[2].visual.column, 0);
    }
}
