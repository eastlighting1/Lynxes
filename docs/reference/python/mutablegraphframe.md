# `MutableGraphFrame`

`MutableGraphFrame` is the Python wrapper for the bounded preprocessing path in Lynxes. You do not load it directly from disk. You enter it from an existing eager graph by calling `graph.into_mutable()`, perform a finite set of graph rewrites, then return to an immutable `GraphFrame` with `freeze()`.

This object is for preprocessing, not for long-lived transactional graph editing.

## Construction

The normal construction path is:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
mutable = g.into_mutable()
```

## Method Summary

| Method | Returns | Notes |
| :--- | :--- | :--- |
| `add_node(node)` | `None` | Append one node row. |
| `add_nodes_batch(nodes)` | `None` | Append several node rows. |
| `add_edge(src, dst)` | `None` | Append one edge by node id. |
| `delete_node(id)` | `None` | Logically remove one node. |
| `delete_edge(edge_row)` | `None` | Logically remove one stable edge row. |
| `update_node(old_id, node)` | `None` | Replace one node via tombstone + append. |
| `compact()` | `None` | Publish a new compacted base snapshot. |
| `freeze()` | `GraphFrame` | Return to an immutable eager graph. |

## Selected Methods

### `add_node(node) -> None`

Append one node row.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `node` | `NodeFrame` | Required | - | A single-row node frame. |

#### Raises

- `ValueError` if the input frame does not contain exactly one row
- `TypeError` if the argument is not a `NodeFrame`

### `add_nodes_batch(nodes) -> None`

Append a multi-row batch of nodes.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `nodes` | `NodeFrame` | Required | - | One or more rows to append. |

### `add_edge(src, dst) -> None`

Append one edge by source and destination node id.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `src` | `str` | Required | - | Source node id. |
| `dst` | `str` | Required | - | Destination node id. |

#### Raises

- `KeyError` if either node id does not exist

### `delete_node(id) -> None`

Logically remove one node and hide its incident edges from the mutable view.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `id` | `str` | Required | - | Node id to delete. |

#### Raises

- `KeyError` if the node id does not exist

### `delete_edge(edge_row) -> None`

Logically remove one stable edge row.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `edge_row` | `int` | Required | - | Stable edge row id. |

#### Raises

- `KeyError` if the edge row does not exist

### `update_node(old_id, node) -> None`

Replace one node through the current tombstone-and-reinsert path.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `old_id` | `str` | Required | - | Existing node id to replace. |
| `node` | `NodeFrame` | Required | - | Single-row replacement node frame. |

#### Notes

This is not an in-place Arrow row mutation. The current implementation keeps the engine's immutable storage model and treats the operation as remove + append.

### `compact() -> None`

Compact delta state into a new base snapshot.

Use this when you want to force publication of a new stable adjacency snapshot before freezing or before a repeated read-heavy preprocessing phase.

### `freeze() -> GraphFrame`

Return a new eager `GraphFrame` from the current mutable state.

#### Returns

Returns a normal immutable `GraphFrame`.

#### Notes

`freeze()` consumes the current mutable state. In Python terms, you should treat the `MutableGraphFrame` instance as finished after this call.

## Usage Pattern

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
mutable = g.into_mutable()

mutable.delete_node("charlie")
mutable.add_edge("alice", "diana")
mutable.compact()

g2 = mutable.freeze()
```

## Related Pages

- [`GraphFrame`](graphframe.md)
- [Graph preprocessing guide](../../guides/graph-preprocessing.md)
