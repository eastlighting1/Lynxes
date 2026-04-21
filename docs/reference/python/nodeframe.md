# `NodeFrame` Reference

`NodeFrame` is the node-side tabular result surface used by Lynxes.

## How You Usually Get One

- `graph.nodes()`
- `lazy.collect_nodes()`
- algorithm outputs such as `pagerank()` or `community_detection()`

## Common Methods

- `len()`
- `is_empty()`
- `column_names()`
- `to_pyarrow()`
- `intersect(other)`
- `difference(other)`
- `NodeFrame.concat([...])`

## Notes

- node results still preserve graph identity semantics through `_id`
- `to_pyarrow()` is the main way to hand the result to Arrow-oriented tooling

## Example

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
nf = g.lazy().filter_nodes(lx.col("_label").contains("Person")).collect_nodes()
print(nf.column_names())
print(nf.to_pyarrow())
```
