# GNN Integration

This guide shows the simplest Lynxes-to-PyTorch style flow: sample a graph neighborhood, gather feature rows in the sampled order, and turn the structure into COO-style arrays that a tensor stack can consume.

Lynxes is not training the model for you here. It is handling the graph-aware preprocessing step.

## What You Start With

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
```

## Sample A Neighborhood

```python
sampled = g.sample_neighbors(
    seed_nodes=["alice"],
    hops=2,
    fan_out=[25, 10],
    direction="out",
)
```

The returned object is `SampledSubgraph`. The key fields are:

- `node_indices`
- `edge_src`
- `edge_dst`
- `edge_row_ids`
- `node_row_ids`

`node_indices`, `edge_src`, and `edge_dst` live in the compact graph-local index space. `node_row_ids` is the bridge back to the original node table.

## Gather Feature Rows

```python
features = g.nodes().take(sampled.node_row_ids)
```

This gives you a `NodeFrame` in the sampled node order. That order is what matters. You should not assume the sampled frontier is already aligned with the original node table.

If you need the lower-level Arrow object, `g.nodes().gather_rows(sampled.node_row_ids)` still returns a pyarrow `RecordBatch`.

## Build A COO View

For the full graph:

```python
src, dst = g.to_coo()
```

For a sampled subgraph, the sampled structure is already split into source and destination vectors:

```python
sampled_src = sampled.edge_src
sampled_dst = sampled.edge_dst
```

## Move Into Torch

The exact conversion step depends on your tensor stack, but the usual shape is:

```python
import torch

edge_index = torch.tensor(
    [sampled.edge_src, sampled.edge_dst],
    dtype=torch.long,
)
```

And for features:

```python
x = features.to_tensor(dtype="float32")
```

By default, `to_tensor()` uses `feature_columns()`, which excludes reserved graph metadata like `_id` and `_label` plus non-numeric columns. If your model expects a specific feature order, pass it explicitly:

```python
x = features.to_tensor(columns=["age", "score"], dtype="float32")
```

Dense tensor export materializes a contiguous 2D matrix by default. It is designed for a clean ML boundary, not as a zero-copy promise. The important point is that Lynxes has already done the graph-aware part:

- structure sampling
- sampled order tracking
- row gather

## What To Check

A minimal sanity check looks like this:

```python
print(sampled.node_indices)
print(sampled.edge_src)
print(sampled.edge_dst)
print(len(features))
```

`len(features)` should match `len(sampled.node_row_ids)`.

## NumPy Export

Use `to_numpy()` when your training stack wants NumPy first:

```python
x_np = features.to_numpy(columns=["age", "score"], dtype="float32")
```

`to_numpy()` and `to_tensor()` share the same column and row selection rules. Single-column export has shape `(N, 1)`, empty row selections preserve the column count, and `columns=[]` returns shape `(N, 0)`.

Null numeric values follow PyArrow's `Array.to_numpy(zero_copy_only=False)` behavior before optional dtype casting. For training, it is usually better to impute or filter missing feature values before export so the model input policy is explicit.

## Multiprocessing

`NodeFrame` supports pickle round-trips by serializing through Arrow. That is the supported policy for Windows `spawn` workers and other multiprocessing environments:

```python
import multiprocessing as mp

def worker(nodes):
    return nodes.to_numpy(columns=["age"]).shape

with mp.get_context("spawn").Pool(1) as pool:
    shape = pool.apply(worker, (features,))
```

For very large frames, prefer writing Arrow/IPC-backed data once and passing file paths to workers when process startup memory matters.

## Why The Split Index Model Exists

It is easy to trip over the fact that topology and features do not use the same integer space.

- `edge_src` / `edge_dst` use compact graph-local indices
- `node_row_ids` points back to the node table

That is intentional. It keeps the graph structure side efficient without pretending that row order is the same thing as graph-local adjacency identity.

## Walk-Based Workflows

If your preprocessing step is walk-based rather than neighborhood-based, the shape is similar:

```python
walks = g.random_walk(
    start_nodes=["alice"],
    length=4,
    walks_per_node=2,
    direction="out",
)
```

The result is a `List[List[int]]` of compact node indices. You can use those sequences directly in a downstream representation-learning pipeline.

## Where To Go Next

If you need to reshape the graph before sampling, go to [Graph preprocessing](graph-preprocessing.md). If you need exact API signatures, continue with [the `GraphFrame` reference](../reference/python/graphframe.md) and [the `MutableGraphFrame` reference](../reference/python/mutablegraphframe.md).
