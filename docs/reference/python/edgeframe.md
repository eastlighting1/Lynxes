# `EdgeFrame` Reference

`EdgeFrame` is the edge-side tabular result surface used by Lynxes.

## How You Usually Get One

- `graph.edges()`
- `lazy.collect_edges()`
- `EdgeFrame.from_dict({...})`
- `EdgeFrame.from_arrow(batch)`

## Reserved Edge Columns

- `_src`
- `_dst`
- `_type`
- `_direction`

## Common Methods

- `EdgeFrame.from_dict({...})`
- `len()`
- `edge_count()` / `node_count()`
- `is_empty()`
- `column_names()`
- `head(...)` / `tail(...)`
- `glimpse(...)`
- `info()` / `schema()` / `describe(...)`
- `out_neighbors(node_id)` / `in_neighbors(node_id)`
- `neighbors(node_id, direction=...)`
- `out_degree(node_id)` / `in_degree(node_id)`
- `with_nodes(nodes)`
- `to_pyarrow()`

## Practical Note

Although an edge result looks tabular at the API boundary, Lynxes still treats graph structure as first-class internally.
Use `EdgeFrame` for inspection, export, and CSR-backed local neighborhood lookups. If you need graph-global algorithms, rehydrate a `GraphFrame` with `with_nodes(nodes)`.

If you are creating an edge frame from Python data, `from_dict({...})` is the shortest constructor path. `from_arrow(...)` is still useful when Arrow batches already exist upstream.

## Example

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
ef = g.edges()
print(ef.head(5))
print(ef.out_neighbors("alice"))
```
