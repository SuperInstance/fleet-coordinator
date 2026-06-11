//! # fleet-coordinator
//! Coordinate a fleet of nodes: registration, health checks, task assignment, and load balancing.

use std::collections::HashMap;

/// Status of a fleet node.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeStatus {
    Healthy,
    Degraded,
    Offline,
}

/// A node in the fleet.
#[derive(Clone, Debug)]
pub struct FleetNode {
    pub id: String,
    pub address: String,
    pub status: NodeStatus,
    pub capacity: u32,
    pub load: u32,
    pub tags: Vec<String>,
}

/// A task to be assigned.
#[derive(Clone, Debug)]
pub struct FleetTask {
    pub id: String,
    pub required_tags: Vec<String>,
    pub weight: u32,
    pub assigned_to: Option<String>,
}

/// The fleet coordinator.
pub struct FleetCoordinator {
    nodes: HashMap<String, FleetNode>,
    tasks: HashMap<String, FleetTask>,
}

impl FleetCoordinator {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            tasks: HashMap::new(),
        }
    }

    /// Register a new node.
    pub fn register(&mut self, id: &str, address: &str, capacity: u32, tags: Vec<String>) {
        self.nodes.insert(id.to_string(), FleetNode {
            id: id.to_string(),
            address: address.to_string(),
            status: NodeStatus::Healthy,
            capacity,
            load: 0,
            tags,
        });
    }

    /// Update a node's status.
    pub fn set_status(&mut self, node_id: &str, status: NodeStatus) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.status = status;
        }
    }

    /// Find the best healthy node for a task (least loaded, matching tags).
    pub fn find_node(&self, required_tags: &[String], weight: u32) -> Option<&FleetNode> {
        self.nodes.values()
            .filter(|n| n.status == NodeStatus::Healthy)
            .filter(|n| n.capacity >= n.load + weight)
            .filter(|n| required_tags.iter().all(|t| n.tags.contains(t)))
            .min_by_key(|n| n.load)
    }

    /// Assign a task to the best available node.
    pub fn assign_task(&mut self, task_id: &str, required_tags: Vec<String>, weight: u32) -> Option<String> {
        let node_id = {
            let node = self.find_node(&required_tags, weight)?;
            node.id.clone()
        };
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.load += weight;
        }
        self.tasks.insert(task_id.to_string(), FleetTask {
            id: task_id.to_string(),
            required_tags,
            weight,
            assigned_to: Some(node_id.clone()),
        });
        Some(node_id)
    }

    /// Complete a task and release its load from the assigned node.
    pub fn complete_task(&mut self, task_id: &str) -> bool {
        if let Some(task) = self.tasks.remove(task_id) {
            if let Some(ref node_id) = task.assigned_to {
                if let Some(node) = self.nodes.get_mut(node_id) {
                    node.load = node.load.saturating_sub(task.weight);
                }
            }
            true
        } else {
            false
        }
    }

    /// Fleet-wide utilization as a percentage (0.0 - 100.0).
    pub fn utilization(&self) -> f64 {
        let total_capacity: u32 = self.nodes.values().map(|n| n.capacity).sum();
        let total_load: u32 = self.nodes.values().map(|n| n.load).sum();
        if total_capacity == 0 { return 0.0; }
        (total_load as f64 / total_capacity as f64) * 100.0
    }

    /// Count nodes by status.
    pub fn status_summary(&self) -> (usize, usize, usize) {
        let healthy = self.nodes.values().filter(|n| n.status == NodeStatus::Healthy).count();
        let degraded = self.nodes.values().filter(|n| n.status == NodeStatus::Degraded).count();
        let offline = self.nodes.values().filter(|n| n.status == NodeStatus::Offline).count();
        (healthy, degraded, offline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_assign() {
        let mut fleet = FleetCoordinator::new();
        fleet.register("n1", "10.0.0.1", 100, vec!["gpu".into()]);
        let assigned = fleet.assign_task("t1", vec!["gpu".into()], 20);
        assert_eq!(assigned, Some("n1".to_string()));
        assert!((fleet.utilization() - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_complete_task() {
        let mut fleet = FleetCoordinator::new();
        fleet.register("n1", "10.0.0.1", 100, vec![]);
        fleet.assign_task("t1", vec![], 30);
        assert!(fleet.complete_task("t1"));
        assert!((fleet.utilization() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_status_summary() {
        let mut fleet = FleetCoordinator::new();
        fleet.register("n1", "a", 10, vec![]);
        fleet.register("n2", "b", 10, vec![]);
        fleet.set_status("n2", NodeStatus::Offline);
        let (h, _, o) = fleet.status_summary();
        assert_eq!(h, 1);
        assert_eq!(o, 1);
    }
}
