# Fleet Coordinator

**A Rust library for fleet node coordination**: registration, health tracking, task assignment with tag-based scheduling, and least-loaded load balancing across a pool of worker nodes.

## Why It Matters

Distributed systems need a central authority to decide *which* node should run *which* task. This coordinator implements the core scheduling logic used in container orchestrators (like Kubernetes' scheduler), job queues (like Celery), and edge computing fleet managers. The least-loaded strategy with tag matching directly mirrors how production schedulers place workloads on GPU-enabled nodes, memory-optimized instances, or geo-pinned edge workers. In the SuperInstance ecosystem, this coordinates the fleet of edge Workers that serve vector search, authentication, and metrics collection.

## How It Works

The coordinator maintains two `HashMap`s: one for `FleetNode` records (with capacity, current load, tags, and health status) and one for `FleetTask` records (with required tags, weight, and assignment). All operations are **O(n)** where n is the node count — the `find_node` method iterates over healthy nodes, filters by capacity headroom (`load + weight ≤ capacity`) and tag containment, then selects the minimum-load candidate via `min_by_key`.

Task lifecycle follows a simple state machine: `assign_task` atomically finds a node, increments its load, and records the task. `complete_task` reverses this — looks up the task, decrements the assigned node's load using `saturating_sub` (preventing underflow), and removes the task. The `utilization()` method computes `Σ load / Σ capacity * 100` across all nodes, giving a fleet-wide utilization percentage. Health status transitions (`Healthy → Degraded → Offline`) exclude nodes from scheduling consideration, effectively draining them.

## Quick Start

```rust
use fleet_coordinator::{FleetCoordinator, NodeStatus};

fn main() {
    let mut fleet = FleetCoordinator::new();

    // Register nodes with capacity and tags
    fleet.register("gpu-1", "10.0.0.1", 100, vec!["gpu".into(), "cuda".into()]);
    fleet.register("gpu-2", "10.0.0.2", 100, vec!["gpu".into()]);
    fleet.register("cpu-1", "10.0.0.3", 50, vec!["cpu".into()]);

    // Assign tasks — scheduler picks the least-loaded matching node
    let node = fleet.assign_task("train-model", vec!["gpu".into()], 40);
    println!("Assigned to: {:?}", node); // Some("gpu-1")

    let node2 = fleet.assign_task("inference", vec!["gpu".into()], 30);
    println!("Assigned to: {:?}", node2); // Some("gpu-2") — gpu-1 now has higher load

    // Check fleet health
    let (healthy, degraded, offline) = fleet.status_summary();
    println!("Fleet: {} healthy, {} degraded, {} offline", healthy, degraded, offline);
    println!("Utilization: {:.1}%", fleet.utilization());

    // Complete a task to free resources
    fleet.complete_task("train-model");
}
```

## API

| Type / Function | Description |
|---|---|
| `FleetCoordinator::new()` | Create an empty fleet coordinator |
| `register(id, address, capacity, tags)` | Register a new node as `Healthy` |
| `set_status(node_id, status)` | Update a node's health status |
| `assign_task(task_id, required_tags, weight)` | Assign to least-loaded matching node — **O(n)** |
| `complete_task(task_id)` | Release task load from its node |
| `find_node(tags, weight)` | Find best node without assigning |
| `utilization()` | Fleet-wide utilization percentage |
| `status_summary()` | Counts of `(healthy, degraded, offline)` nodes |

## Architecture Notes

This is the coordination layer for the SuperInstance fleet of Cloudflare Workers. It interfaces with `fleet-auth` (authentication), `fleet-metrics-cron` (telemetry), and `fleet-vector-api` (vector search). See the [Architecture Guide](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## License

MIT
