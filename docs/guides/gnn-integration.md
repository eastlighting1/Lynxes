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
feature_batch = g.nodes().gather_rows(sampled.node_row_ids)
```

This gives you a pyarrow `RecordBatch` in the sampled node order. That order is what matters. You should not assume the sampled frontier is already aligned with the original node table.

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
table = feature_batch

# Example: use one numeric column
x = torch.tensor(table["age"].to_pylist(), dtype=torch.float32).unsqueeze(1)
```

For a real model you would usually convert several numeric columns together or move through NumPy first. The important point is that Lynxes has already done the graph-aware part:

- structure sampling
- sampled order tracking
- row gather

## What To Check

A minimal sanity check looks like this:

```python
print(sampled.node_indices)
print(sampled.edge_src)
print(sampled.edge_dst)
print(feature_batch.num_rows)
```

`feature_batch.num_rows` should match `len(sampled.node_row_ids)`.

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
