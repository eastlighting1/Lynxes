# `GraphFrame`

`GraphFrame` is the eager graph object in the Python surface. Once a graph has already been loaded into memory, this is the type that holds it. Module-level loaders such as `lynxes.read_gf(...)`, `lynxes.read_gfb(...)`, and `lynxes.read_parquet_graph(...)` all return `GraphFrame`. The same type also comes back from `LazyGraphFrame.collect()` when a lazy plan is finally materialized.

In practice, `GraphFrame` is the object you reach for when you want to inspect a concrete graph, switch to node-side or edge-side frame views, run an eager algorithm immediately, or hand the graph off to an export path. If you want to keep composing work without materializing new results yet, the transition point is `graph.lazy()`.

## Construction

Most Python users do not instantiate `GraphFrame` directly. Common construction paths are:

- `lynxes.read_gf(path)`
- `lynxes.read_gfb(path)`
- `lynxes.read_parquet_graph(nodes_path, edges_path)`
- `lazy.collect()`
- `GraphFrame.from_frames(nodes, edges)`

`GraphFrame.from_frames(nodes, edges)` is the explicit constructor when you already have a `NodeFrame` and `EdgeFrame` and want to reassemble them into a graph. It validates that the reserved graph semantics still line up.

## Method Summary

### Structural access

| Method | Returns | Notes |
| :--- | :--- | :--- |
| `nodes()` | `NodeFrame` | Returns the node-side frame view. |
| `edges()` | `EdgeFrame` | Returns the edge-side frame view. |
| `lazy()` | `LazyGraphFrame` | Starts a lazy query from the current graph. |
| `node_count()` | `int` | Count of node rows in the current graph. |
| `edge_count()` | `int` | Count of edge rows in the current graph. |
| `density()` | `float` | Density summary computed from the current graph. |

### Neighborhood and degree

| Method | Returns | Notes |
| :--- | :--- | :--- |
| `neighbors(node_id, direction="out")` | `list[str]` | Returns neighboring node ids for one seed node. |
| `out_degree(node_id)` | `int` | Out-degree for one node id. |
| `in_degree(node_id)` | `int` | In-degree for one node id. |

### Eager algorithms

| Method | Returns | Notes |
| :--- | :--- | :--- |
| `pagerank(...)` | `NodeFrame` | Returns node-level PageRank scores. |
| `connected_components()` | `NodeFrame` | Returns node-level component assignments. |
| `largest_connected_component()` | `GraphFrame` | Returns the largest component as a graph. |
| `shortest_path(...)` | `list[str] \| None` | Returns one path as node ids, or `None`. |
| `all_shortest_paths(...)` | `list[list[str]]` | Returns all shortest paths that tie for best cost. |
| `betweenness_centrality(...)` | `NodeFrame` | Returns node-level centrality scores. |
| `degree_centrality(...)` | `NodeFrame` | Returns node-level degree-based scores. |
| `community_detection(...)` | `NodeFrame` | Returns node-level community assignments. |
| `has_path(src, dst, max_hops=None)` | `bool` | Returns whether a path exists. |

### Display helpers

| Method | Returns | Notes |
| :--- | :--- | :--- |
| `head(n=10, sort_by=None, ascii=False, width=None)` | `str` | Renders a preview from the front of the graph. |
| `tail(n=10, sort_by=None, ascii=False, width=None)` | `str` | Renders a preview from the end of the graph. |
| `info()` | `str` | Renders a structural summary. |
| `schema()` | `str` | Renders schema information. |
| `glimpse(n=10, sort_by=None, ascii=False, width=None)` | `str` | Renders a compact preview. |
| `describe(mode="all")` | `str` | Renders a textual description. |

## Selected Methods

### `GraphFrame.from_frames(nodes, edges) -> GraphFrame`

Create a graph from an existing `NodeFrame` and `EdgeFrame`.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `nodes` | `NodeFrame` | Required | - | Node-side frame with valid reserved node columns. |
| `edges` | `EdgeFrame` | Required | - | Edge-side frame with valid reserved edge columns. |

#### Returns

Returns a new eager `GraphFrame`.

#### Raises

- `ValueError` if the frames do not form a valid graph shape
- `TypeError` if either argument is not the expected frame wrapper

### `neighbors(node_id, direction="out") -> list[str]`

Return neighbor ids for a single node.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `node_id` | `str` | Required | - | Seed node id. |
| `direction` | `str` | Optional | `"out"` | Direction selector. Accepted values are `"out"`, `"in"`, `"both"`, and `"none"`. |

#### Returns

Returns a Python `list[str]` containing neighboring node ids in the requested direction mode.

#### Raises

- `KeyError` if `node_id` does not exist in the graph
- `ValueError` if `direction` is not one of the accepted values

### `pagerank(damping=0.85, max_iter=100, epsilon=1e-6, weight_col=None) -> NodeFrame`

Run PageRank eagerly.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `damping` | `float` | Optional | `0.85` | Damping factor used by the PageRank iteration. |
| `max_iter` | `int` | Optional | `100` | Maximum number of iterations. |
| `epsilon` | `float` | Optional | `1e-6` | Convergence threshold. |
| `weight_col` | `str \| None` | Optional | `None` | Optional edge-weight column. |

#### Returns

Returns a `NodeFrame`, not a Python list. Convert the result with `to_pyarrow()` if you want Arrow tables for downstream work.

#### Raises

- `TypeError` if an argument has the wrong Python type
- `ValueError` if configuration is invalid or the chosen weight column cannot be used

### `shortest_path(src, dst, weight_col=None, edge_type=None, direction="out") -> list[str] | None`

Compute one shortest path between two node ids.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `src` | `str` | Required | - | Source node id. |
| `dst` | `str` | Required | - | Destination node id. |
| `weight_col` | `str \| None` | Optional | `None` | Optional edge-weight column for weighted search. |
| `edge_type` | `str \| None` | Optional | `None` | Restrict traversal to one edge type. |
| `direction` | `str` | Optional | `"out"` | Traversal direction. Accepted values are `"out"`, `"in"`, `"both"`, and `"none"`. |

#### Returns

Returns one path as `list[str]`, or `None` when no path exists under the requested traversal constraints.

#### Raises

- `KeyError` if either endpoint does not exist
- `ValueError` if `direction` is invalid or the weighted search configuration cannot be satisfied

### `community_detection(algorithm="louvain", resolution=1.0, seed=None) -> NodeFrame`

Run community detection eagerly and return node-level assignments.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `algorithm` | `str` | Optional | `"louvain"` | Community algorithm name. |
| `resolution` | `float` | Optional | `1.0` | Resolution parameter passed to the algorithm. |
| `seed` | `int \| None` | Optional | `None` | Optional deterministic seed. |

#### Returns

Returns a `NodeFrame` with one row per node and community-related output columns.

#### Raises

- `ValueError` if the algorithm name is unsupported or the configuration is invalid

## Notes

`GraphFrame` is already materialized. Methods like `node_count()`, `neighbors(...)`, `pagerank(...)`, or `shortest_path(...)` run against the current in-memory graph immediately. If you want planning and deferred execution instead, call `graph.lazy()` and move into the `LazyGraphFrame` surface.

For graph export behavior, continue with [Graph export methods](graph-export.md). For how engine errors surface in Python, continue with [Python error mapping](errors.md).
