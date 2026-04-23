# Loading Data

This guide covers the practical ways to load graphs into Lynxes today.

## Python Entry Points

The main Python loading functions are:

- `lynxes.read_gf(path)`
- `lynxes.read_gfb(path)`
- `lynxes.read_parquet_graph(nodes_path, edges_path)`

If you are starting from the shared repository examples, the smallest input is:

`examples/data/example_simple.gf`

## Load From `.gf`

Use `.gf` when you want a human-readable source file.

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
print(g.node_count(), g.edge_count())
```

Use this format for:

- hand-authored examples
- debugging small graphs
- reviewing graph content in version control

## Load From `.gfb`

Use `.gfb` when you want a binary Lynxes-native file for faster reloads and compact storage.

```python
import lynxes as lx

g = lx.read_gfb("graph.gfb")
print(g.node_count(), g.edge_count())
```

Use this format for:

- local round-trips after preprocessing
- CLI conversion outputs
- repeated reload of the same graph

## Load From Parquet

Use parquet when your graph already lives in Arrow or parquet-oriented pipelines.

```python
import lynxes as lx

g = lx.read_parquet_graph("nodes.parquet", "edges.parquet")
print(g.node_count(), g.edge_count())
```

That example assumes you already have a parquet node/edge pair. If you are starting from a `.gf` graph inside Lynxes, generate the pair first with `write_parquet_graph(...)` and then reload it.

Lynxes expects a two-file graph shape:

- one parquet file for nodes
- one parquet file for edges

## Validate After Load

After loading, check the graph immediately:

```python
print(g.node_count())
print(g.edge_count())
print(g.nodes().column_names())
print(g.edges().column_names())
```

On the CLI, the equivalent first check is:

```bash
lynxes inspect examples/data/example_simple.gf
```

## Common Load Errors

Typical failures include:

- missing file paths
- malformed `.gf` syntax
- missing reserved columns in parquet input
- duplicate node ids
- edges that reference missing nodes

If loading fails, continue with [Errors and Debugging](errors-and-debugging.md).
