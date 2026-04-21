# Parquet Graph Shape

Lynxes supports graph-shaped parquet workflows through separate node and edge files. This is the right interop path when the graph already lives in a broader columnar data pipeline and you want to move it into Lynxes without flattening it into a text format first.

## Expected Shape

Parquet graph input uses two files:

- one parquet file for node rows
- one parquet file for edge rows

Python entry point:

```python
import lynxes as lx

g = lx.read_parquet_graph("nodes.parquet", "edges.parquet")
```

## Required Reserved Columns

The node file must preserve the reserved node columns:

- `_id`
- `_label`

The edge file must preserve the reserved edge columns:

- `_src`
- `_dst`
- `_type`
- `_direction`

For the meanings of those fields, continue with [Reserved graph columns](reserved-columns.md).

## Validation Notes

Typical parquet-load failures come from graph-shape problems rather than from parquet as a storage technology. Common causes include:

- node and edge rows mixed into one file
- reserved columns missing or misnamed
- duplicate node ids
- edge endpoints that do not refer to real nodes

## Notes

Parquet is a practical interop format when the graph is already embedded in a columnar workflow. It is less convenient than `.gf` for hand-authored graphs, and less compact than `.gfb` when what you want is a Lynxes-native binary artifact for repeated local reload.
