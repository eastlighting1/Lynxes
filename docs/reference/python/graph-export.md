# Graph Export Methods

This page covers the Python write paths for an already-materialized `GraphFrame`. All methods here are eager export operations. They do not create a lazy write plan, and they do not accept `LazyGraphFrame` directly.

Each export method also has a module-level helper with equivalent behavior. The instance method is usually the clearer form when you already have a `GraphFrame` in hand. The module-level function is useful when you want a functional style or are passing graphs around explicitly.

## Summary

| Method | Helper | Output |
| :--- | :--- | :--- |
| `graph.write_gf(path)` | `lynxes.write_gf(graph, path)` | `.gf` text file |
| `graph.write_gfb(path)` | `lynxes.write_gfb(graph, path)` | `.gfb` binary file |
| `graph.write_parquet_graph(nodes_path, edges_path)` | `lynxes.write_parquet_graph(graph, nodes_path, edges_path)` | two-file parquet graph |

## `graph.write_gf(path) -> None`

Write a `GraphFrame` to a `.gf` text file.

### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `path` | `str \| os.PathLike[str]` | Required | - | Destination path for the `.gf` file. |

### Raises

- `OSError` if the destination cannot be written
- `ValueError` if the graph cannot be serialized as valid `.gf`
- `TypeError` if `path` is not a string or path-like object

## `graph.write_gfb(path) -> None`

Write a `GraphFrame` to a `.gfb` binary file.

### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `path` | `str \| os.PathLike[str]` | Required | - | Destination path for the `.gfb` file. |

### Raises

- `OSError` if the destination cannot be written
- `ValueError` if the graph cannot be serialized successfully
- `TypeError` if `path` is not a string or path-like object

## `graph.write_parquet_graph(nodes_path, edges_path) -> None`

Write the graph as a node parquet file and an edge parquet file.

### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `nodes_path` | `str \| os.PathLike[str]` | Required | - | Destination path for node rows. |
| `edges_path` | `str \| os.PathLike[str]` | Required | - | Destination path for edge rows. |

### Raises

- `OSError` if either destination cannot be written
- `ValueError` if the graph cannot be serialized into the expected parquet graph shape
- `TypeError` if either argument is not a string or path-like object

## Module-Level Helpers

The module also exposes:

- `lynxes.write_gf(graph, path)`
- `lynxes.write_gfb(graph, path)`
- `lynxes.write_parquet_graph(graph, nodes_path, edges_path)`

These helpers use the same write paths as the instance methods above. They do not define a separate export model.

## Unsupported Export Names

The Python surface also exposes names such as `write_rdf(...)` and `write_owl(...)`. At the moment they should be treated as present-but-unsupported names rather than working export paths. Do not rely on them unless they later receive dedicated documentation and tests.
