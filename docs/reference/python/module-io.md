# Module I/O Functions

This page documents the Python module-level graph loading entry points exposed by `lynxes`. All functions on this page return eager `GraphFrame` objects. None of them creates a lazy scan. If you need a lazy graph after loading, call `.lazy()` on the returned graph.

All file-path parameters accept either a Python string or a path-like object. That behavior comes from the binding layer itself, so it applies consistently across `.gf`, `.gfb`, and parquet graph loaders.

## Summary

| Function | Returns | Purpose |
| :--- | :--- | :--- |
| `lynxes.read_gf(path)` | `GraphFrame` | Load a `.gf` text graph. |
| `lynxes.read_gfb(path)` | `GraphFrame` | Load a `.gfb` binary graph. |
| `lynxes.read_parquet_graph(nodes_path, edges_path)` | `GraphFrame` | Load a graph from node and edge parquet files. |

## `lynxes.read_gf(path) -> GraphFrame`

Load a graph from a `.gf` text file.

### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `path` | `str \| os.PathLike[str]` | Required | - | Path to a `.gf` file. |

### Returns

Returns an eager `GraphFrame`.

### Raises

- `OSError` if the file cannot be opened
- `ValueError` if the file fails parsing or graph validation
- `TypeError` if `path` is not a string or path-like object

### Example

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
print(g.node_count(), g.edge_count())
```

## `lynxes.read_gfb(path) -> GraphFrame`

Load a graph from a `.gfb` binary file.

### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `path` | `str \| os.PathLike[str]` | Required | - | Path to a `.gfb` file. |

### Returns

Returns an eager `GraphFrame`.

### Raises

- `OSError` if the file cannot be opened
- `ValueError` if the file content cannot be decoded as a valid Lynxes graph
- `TypeError` if `path` is not a string or path-like object

## `lynxes.read_parquet_graph(nodes_path, edges_path) -> GraphFrame`

Load a graph from two parquet files, one for nodes and one for edges.

### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `nodes_path` | `str \| os.PathLike[str]` | Required | - | Path to the node parquet file. |
| `edges_path` | `str \| os.PathLike[str]` | Required | - | Path to the edge parquet file. |

### Returns

Returns an eager `GraphFrame`.

### Raises

- `OSError` if either file cannot be opened
- `ValueError` if reserved columns are missing or the graph shape is invalid
- `TypeError` if either argument is not a string or path-like object

### Notes

The parquet loader expects a graph-shaped two-file layout. The node file and edge file are not interchangeable, and both must preserve the reserved graph columns Lynxes relies on. For the exact field-level expectations, continue with the format reference pages, especially [Reserved graph columns](../formats/reserved-columns.md) and [Parquet graph shape](../formats/parquet-interop.md).

## Related Pages

Use [Graph export methods](graph-export.md) for the write side of the same workflow. Use [Python error mapping](errors.md) when you need to understand why a load failure surfaced as `ValueError`, `TypeError`, or `OSError`.
