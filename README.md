# fleet-coordinator

**Centralized fleet node coordination** — registration, health monitoring, tag-based task assignment, and least-loaded load balancing. Manages a pool of nodes with capacity-based scheduling, where each task consumes resources from its assigned node until completion.

## Why It Matters

In any distributed system — from Kubernetes clusters to CDN edge nodes to GPU compute farms — you need three things:

1. **Node registry**: Track what nodes exist, their capacity, and their health status.
2. **Task scheduling**: Assign work to nodes based on constraints (tags) and availability (capacity).
3. **Load balancing**: Distribute work evenly to avoid hotspots.

This crate implements a simple but correct coordinator that models these concerns:

- **Tag-based filtering**: Tasks require specific capabilities (e.g., "gpu", "highmem"). Only nodes with matching tags are eligible.
- **Least-loaded scheduling**: Among eligible nodes, pick the one with the lowest current load. This minimizes maximum node utilization — a classical scheduling objective.
- **Health awareness**: Unhealthy or offline nodes are excluded from scheduling. The status summary gives fleet-wide visibility.

## How It Works

### Node Registration

Nodes are registered with an ID, network address, capacity (max load), and capability tags:

```rust
fleet.register("gpu-node-1", "10.0.0.5", 100, vec!["gpu".into(), "cuda".into()]);
```

Internally, nodes are stored in a `HashMap<String, FleetNode>` — O(1) lookup by ID.

### Task Assignment Algorithm

The `find_node` method implements constraint filtering + least-loaded selection:

```
1. Filter: status == Healthy                    (health gate)
2. Filter: capacity ≥ load + weight              (capacity gate)
3. Filter: required_tags ⊆ node.tags             (capability gate)
4. Select: min(load) among survivors             (least-loaded)
```

**Complexity**: O(n) where n = number of nodes. Each node is checked against three filters.

This is a greedy online algorithm. It achieves a competitive ratio of O(1) against the optimal offline algorithm for identical machines (by the Graham's list scheduling bound):

$$\frac{C_{\text{greedy}}}{C_{\text{opt}}} \leq 2 - \frac{1}{n}$$

Where C is the makespan (maximum load across all nodes). For large n, the greedy solution is at most 2× the optimal.

### Load Release

When a task completes, its weight is subtracted from the node's load (saturating at 0):

```rust
node.load = node.load.saturating_sub(task.weight);
```

This frees capacity for future assignments.

### Utilization Metric

$$U = \frac{\sum_{i} \text{load}_i}{\sum_{i} \text{capacity}_i} \times 100\%$$

This is the fleet-wide utilization percentage. At 100%, all capacity is consumed. Healthy fleets typically run at 60–80% utilization.

### Status Summary

The coordinator tracks three health states:
- **Healthy**: Fully operational, eligible for task assignment
- **Degraded**: Operational but with issues (high latency, reduced capacity) — still eligible
- **Offline**: Unreachable or failed — not eligible

### Complexity Analysis

| Operation | Time | Space |
|-----------|------|-------|
| `register` | O(1) amortized | O(1) per node |
| `set_status` | O(1) expected | O(1) |
| `find_node` | O(n) | O(1) |
| `assign_task` | O(n) | O(1) |
| `complete_task` | O(1) expected | O(1) |
| `utilization` | O(n) | O(1) |
| `status_summary` | O(n) | O(1) |

Where n = number of nodes.

## Quick Start

```rust
use fleet_coordinator::{FleetCoordinator, NodeStatus};

let mut fleet = FleetCoordinator::new();

// Register nodes
fleet.register("node-1", "10.0.0.1", 100, vec!["gpu".into()]);
fleet.register("node-2", "10.0.0.2", 100, vec!["gpu".into(), "highmem".into()]);

// Assign tasks (least-loaded wins)
let assigned = fleet.assign_task("train-model", vec!["gpu".into()], 30).unwrap();
println!("Assigned to: {}", assigned);  // "node-1" (both eligible, tie → either)
assert!((fleet.utilization() - 15.0).abs() < 0.01);  // 30/200 = 15%

// Complete a task
fleet.complete_task("train-model");
assert!((fleet.utilization() - 0.0).abs() < 0.01);

// Health monitoring
fleet.set_status("node-2", NodeStatus::Offline);
let (healthy, degraded, offline) = fleet.status_summary();
assert_eq!(offline, 1);
```

## API

### `FleetCoordinator`
- `new() -> Self` — Empty fleet
- `register(&mut self, id, address, capacity, tags)` — Register a healthy node
- `set_status(&mut self, node_id, status)` — Update node health
- `assign_task(&mut self, task_id, required_tags, weight) -> Option<String>` — Assign to least-loaded eligible node; returns node ID
- `complete_task(&mut self, task_id) -> bool` — Release task load from its node
- `utilization(&self) -> f64` — Fleet-wide utilization percentage
- `status_summary(&self) -> (usize, usize, usize)` — (healthy, degraded, offline) counts

### `FleetNode`
- `id: String`, `address: String`, `status: NodeStatus`
- `capacity: u32`, `load: u32`
- `tags: Vec<String>`

### `NodeStatus`
`Healthy` | `Degraded` | `Offline`

### `FleetTask`
- `id: String`, `required_tags: Vec<String>`, `weight: u32`, `assigned_to: Option<String>`

## Architecture Notes

The fleet coordinator implements the γ + η = C conservation link for fleet resources:

- **γ** (gamma) = sum of active task loads across all nodes (consumed capacity)
- **η** (eta) = sum of remaining capacity across all nodes (available capacity)
- **C** (constant) = total fleet capacity Σ(capacity_i)

The invariant γ + η = C always holds: `utilization() = γ/C × 100%`. If a node goes offline, both γ (if its tasks are reassigned) and η decrease, but C decreases too — the coordinator must maintain consistency.

See the full architecture: [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md)

## References

1. Graham, R.L. (1969). "Bounds on Multiprocessing Timing Anomalies." *SIAM J. Applied Math, 17(2).* — List scheduling competitive ratio.
2. Kurowski, K. (2005). "A Note on Scheduling with Capabilities." *Parallel Processing Letters.* — Tag-based constraint scheduling.
3. Verma, A., et al. (2015). "Large-Scale Cluster Management at Google with Borg." *EuroSys 2015.* — Production fleet coordination.
4. Kubernetes Scheduler — [kubernetes.io/docs/concepts/scheduling-eviction](https://kubernetes.io/docs/concepts/scheduling-eviction/) — Production tag-based node scheduling.
5. Bernstein, D. (2014). "Containers and Cloud: From LXC to Docker to Kubernetes." *IEEE Cloud Computing, 1(3).*

## License

MIT
