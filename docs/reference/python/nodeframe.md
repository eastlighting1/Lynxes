# `NodeFrame` Reference

`NodeFrame` is the node-side tabular result surface used by Lynxes.

## How You Usually Get One

- `graph.nodes()`
- `lazy.collect_nodes()`
- algorithm outputs such as `pagerank()` or `community_detection()`
- `NodeFrame.from_dict({...})`
- `NodeFrame.from_arrow(batch)`

## Common Methods

- `NodeFrame.from_dict({...})`
- `len()`
- `node_count()`
- `is_empty()`
- `column_names()`
- `head(...)` / `tail(...)`
- `glimpse(...)`
- `info()` / `schema()` / `describe(...)`
- `to_pyarrow()`
- `intersect(other)`
- `difference(other)`
- `with_edges(edges)`
- `NodeFrame.concat([...])`

## Notes

- node results still preserve graph identity semantics through `_id`
- display helpers such as `head(...)` are useful for algorithm outputs like `pagerank()`
- `with_edges(edges)` is the shortest way to rehydrate a `GraphFrame` when you already hold node results and a compatible edge frame
- `to_pyarrow()` is the main way to hand the result to Arrow-oriented tooling
- `from_dict({...})` is the shortest Python-native constructor when you want Lynxes to build the frame from plain column data

## Example

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
ranks = g.pagerank()
print(ranks.head(5, sort_by="pagerank", descending=True))
```
