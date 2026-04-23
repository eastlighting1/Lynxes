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
| `add_node(node)` | `MutableGraphFrame` | Append one node row; fluent in Python. |
| `add_nodes_batch(nodes)` | `MutableGraphFrame` | Append several node rows; fluent in Python. |
| `add_edge(src, dst)` | `MutableGraphFrame` | Append one edge by node id; fluent in Python. |
| `delete_node(id)` | `MutableGraphFrame` | Logically remove one node; fluent in Python. |
| `delete_edge(edge_row)` | `MutableGraphFrame` | Logically remove one stable edge row; fluent in Python. |
| `update_node(old_id, node)` | `MutableGraphFrame` | Replace one node via tombstone + append; fluent in Python. |
| `compact()` | `MutableGraphFrame` | Publish a new compacted base snapshot and keep chaining. |
| `freeze()` | `GraphFrame` | Return to an immutable eager graph. |

## Selected Methods

### `add_node(node) -> MutableGraphFrame`

Append one node row.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `node` | `NodeFrame` | Required | - | A single-row node frame. |

#### Raises

- `ValueError` if the input frame does not contain exactly one row
- `TypeError` if the argument is not a `NodeFrame`

### `add_nodes_batch(nodes) -> MutableGraphFrame`

Append a multi-row batch of nodes.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `nodes` | `NodeFrame` | Required | - | One or more rows to append. |

### `add_edge(src, dst) -> MutableGraphFrame`

Append one edge by source and destination node id.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `src` | `str` | Required | - | Source node id. |
| `dst` | `str` | Required | - | Destination node id. |

#### Raises

- `KeyError` if either node id does not exist

### `delete_node(id) -> MutableGraphFrame`

Logically remove one node and hide its incident edges from the mutable view.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `id` | `str` | Required | - | Node id to delete. |

#### Raises

- `KeyError` if the node id does not exist

### `delete_edge(edge_row) -> MutableGraphFrame`

Logically remove one stable edge row.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `edge_row` | `int` | Required | - | Stable edge row id. |

#### Raises

- `KeyError` if the edge row does not exist

### `update_node(old_id, node) -> MutableGraphFrame`

Replace one node through the current tombstone-and-reinsert path.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `old_id` | `str` | Required | - | Existing node id to replace. |
| `node` | `NodeFrame` | Required | - | Single-row replacement node frame. |

#### Notes

This is not an in-place Arrow row mutation. The current implementation keeps the engine's immutable storage model and treats the operation as remove + append.

### `compact() -> MutableGraphFrame`

Compact delta state into a new base snapshot.

Use this when you want to force publication of a new stable adjacency snapshot before a repeated read-heavy preprocessing phase, or when you want an explicit checkpoint in the middle of a longer rewrite chain.

### `freeze() -> GraphFrame`

Return a new eager `GraphFrame` from the current mutable state.

#### Returns

Returns a normal immutable `GraphFrame`.

#### Notes

`freeze()` consumes the current mutable state. In Python terms, you should treat the `MutableGraphFrame` instance as finished after this call.

`freeze()` already performs the compaction it needs, so `compact()` is optional in the common one-shot preprocessing path.

## Usage Pattern

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
g2 = (
    g.into_mutable()
    .delete_node("charlie")
    .add_edge("alice", "diana")
    .freeze()
)
```

## Related Pages

- [`GraphFrame`](graphframe.md)
- [Graph preprocessing guide](../../guides/graph-preprocessing.md)
