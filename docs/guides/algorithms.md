# Algorithms

This guide summarizes the eager graph algorithms currently exposed through `GraphFrame`.
Each section below includes a minimal runnable example so you can see the result shape before you dive into the cookbook pages.

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

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

paths = g.all_shortest_paths("alice", "charlie")

print(paths)
```

## Ranking Algorithms

### `pagerank(...)`

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

ranks = g.pagerank()

print(ranks.column_names())
```

Also supports a `weight_col` keyword when the edge weights are meaningful for rank propagation.

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_weighted.gf")

ranks = g.pagerank(weight_col="weight")

print(ranks.head(5, sort_by="pagerank", descending=True))
```

### `betweenness_centrality(...)`

Use when you want node importance based on shortest-path flow through the graph.

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

scores = g.betweenness_centrality()

print(scores.head(5, sort_by="betweenness", descending=True))
```

### `degree_centrality(...)`

Use when simple in/out/both degree prominence is enough.

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

scores = g.degree_centrality(direction="out")

print(scores.head(5, sort_by="degree_centrality", descending=True))
```

## Connectivity Algorithms

### `connected_components()`

Returns a node frame with component assignments.

```python
import lynxes as lx
from collections import Counter

g = lx.read_gf("examples/data/example_complex.gf")

components = g.connected_components()
component_ids = components.column_values("component_id")
component_sizes = Counter(component_ids)

print("columns:", components.column_names())
print("rows:", components.len())
print("distinct components:", len(component_sizes))
print("component sizes:", component_sizes)
```

If you want to inspect the members of one component, filter by a chosen `component_id` after that first summary.

### `largest_connected_component()`

Returns the largest connected subgraph as a `GraphFrame`.

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_complex.gf")

lcc = g.largest_connected_component()

print("nodes:", lcc.node_count())
print("edges:", lcc.edge_count())
```

## Community Detection

### `community_detection(...)`

```python
import lynxes as lx
from collections import Counter

g = lx.read_gf("examples/data/example_complex.gf")

communities = g.community_detection()
community_ids = communities.column_values("community_id")
community_sizes = Counter(community_ids)

print("columns:", communities.column_names())
print("rows:", communities.len())
print("distinct communities:", len(community_sizes))
print("community sizes:", community_sizes)
```

The current Python surface supports the default community workflow and exposes a `community_id` result column.
If you want to inspect the members of one detected community, filter by a chosen `community_id` after that first summary.

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
