# `NodeFrame` Reference

`NodeFrame` is the node-side tabular result surface used by Lynxes.

## How You Usually Get One

- `graph.nodes()`
- `lazy.collect_nodes()`
- algorithm outputs such as `pagerank()` or `community_detection()`
- `NodeFrame.from_dict({...})`
- `NodeFrame.from_arrow(batch_or_table)`

## Common Methods

- `NodeFrame.from_dict({...})`
- `len()`
- `node_count()`
- `is_empty()`
- `column_names()`
- `ids()`
- `column_values(name)`
- `feature_columns(include=None, exclude_reserved=True, numeric_only=True)`
- `to_numpy(columns=None, indices=None, dtype=None, contiguous=True)`
- `to_tensor(columns=None, indices=None, dtype="float32", device=None, contiguous=True)`
- `take(indices)`
- `head(...)` / `tail(...)`
- `glimpse(...)`
- `info()` / `schema()` / `describe(...)`
- `to_pyarrow()`
- `from_arrow(...)` accepts a `pyarrow.RecordBatch` or a chunked `pyarrow.Table`
- `intersect(other)`
- `difference(other)`
- `with_edges(edges)`
- `NodeFrame.concat([...])`

## Notes

- node results still preserve graph identity semantics through `_id`
- `ids()` is the shortest Python-native way to extract node ids without dropping to `pyarrow`
- `column_values(name)` is the general Python-native escape hatch when you want one column as a plain list
- display helpers such as `head(...)` are useful for algorithm outputs like `pagerank()`
- `with_edges(edges)` is the shortest way to rehydrate a `GraphFrame` when you already hold node results and a compatible edge frame
- `to_pyarrow()` is the main way to hand the result to Arrow-oriented tooling
- `from_dict({...})` is the shortest Python-native constructor when you want Lynxes to build the frame from plain column data
- `feature_columns()` excludes reserved graph metadata (`_id`, `_label`, and names starting with `_`) and non-numeric columns by default
- `to_numpy()` and `to_tensor()` use `feature_columns()` when `columns` is omitted; explicit `columns=[...]` preserves caller order and rejects non-numeric data
- `indices` for `take()`, `to_numpy()`, and `to_tensor()` may be a Python sequence, NumPy array, PyArrow array, or PyTorch tensor
- dense NumPy/tensor export materializes a 2D matrix and is contiguous by default; it should not be treated as zero-copy
- null numeric values follow PyArrow's `Array.to_numpy(zero_copy_only=False)` conversion behavior before optional dtype casting
- `NodeFrame` supports pickle round-trips by serializing through Arrow, which is the supported multiprocessing transfer policy for spawn workers

## Example

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
ranks = g.pagerank()
print(ranks.head(5, sort_by="pagerank", descending=True))
print(ranks.ids()[:3])
print(ranks.column_values("pagerank")[:3])
```

## GNN Feature Extraction

```python
import lynxes as lx

nodes = lx.NodeFrame.from_dict(
    {
        "_id": ["n0", "n1"],
        "_label": [["User"], ["User"]],
        "age": [31, 29],
        "score": [0.8, 0.4],
        "name": ["alice", "bob"],
    }
)

assert nodes.feature_columns() == ["age", "score"]

x_np = nodes.to_numpy(dtype="float32")
x_torch = nodes.to_tensor(indices=[1, 0], dtype="float32")
subset = nodes.take([1, 0])
```
