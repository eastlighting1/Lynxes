# Algorithms

This guide summarizes the eager graph algorithms currently exposed through `GraphFrame`.

## Path Algorithms

### `shortest_path(...)`

Use for one source-to-destination path.

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_weighted.gf")
path = g.shortest_path(
    "seoul",
    "busan",
    weight_col="weight",
    edge_type="ROUTE",
    direction="out",
)
print(path)
```

### `all_shortest_paths(...)`

Use when you need all equally short paths between two nodes rather than only one chosen path.

## Ranking Algorithms

### `pagerank(...)`

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
ranks = g.pagerank()
print(ranks.column_names())
```

Also supports a `weight_col` keyword when the edge weights are meaningful for rank propagation.

### `betweenness_centrality(...)`

Use when you want node importance based on shortest-path flow through the graph.

### `degree_centrality(...)`

Use when simple in/out/both degree prominence is enough.

## Connectivity Algorithms

### `connected_components()`

Returns a node frame with component assignments.

### `largest_connected_component()`

Returns the largest connected subgraph as a `GraphFrame`.

## Community Detection

### `community_detection(...)`

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_complex.gf")
communities = g.community_detection()
print(communities.column_names())
```

The current Python surface supports the default community workflow and exposes a `community_id` result column.

## Choosing Query vs Algorithm APIs

Use lazy query operations when you want:

- filtering
- traversal
- subgraph construction

Use eager algorithms when you want:

- path answers
- ranking scores
- connectivity assignments
- community assignments

## Sampling For GNN Pipelines

Not every graph workflow ends in an eager algorithm result. If your next step is a minibatch training pipeline, the more relevant methods are the graph-side bridge helpers on `GraphFrame`:

- `sample_neighbors(...)`
- `random_walk(...)`
- `to_coo()`
- `nodes().gather_rows(...)`

Those are better understood as preprocessing and handoff utilities than as eager graph algorithms. If that is the workflow you are after, continue with [GNN integration](gnn-integration.md).
