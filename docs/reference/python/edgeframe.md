# `EdgeFrame` Reference

`EdgeFrame` is the edge-side tabular result surface used by Lynxes.

## How You Usually Get One

- `graph.edges()`
- `lazy.collect_edges()`

## Reserved Edge Columns

- `_src`
- `_dst`
- `_type`
- `_direction`

## Common Methods

- `len()`
- `is_empty()`
- `column_names()`
- `to_pyarrow()`

## Practical Note

Although an edge result looks tabular at the API boundary, Lynxes still treats graph structure as first-class internally.
Use `EdgeFrame` for inspection and export, not as a replacement for graph traversal semantics.

## Example

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
ef = g.lazy().filter_edges(lx.col("_type") == "KNOWS").collect_edges()
print(ef.column_names())
```
