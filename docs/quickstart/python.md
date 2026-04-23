# Python Quickstart

This quickstart shows the smallest useful Python workflow in Lynxes:

1. pick a small `.gf` graph
2. load it into a `GraphFrame`
3. run a lazy traversal query
4. run one built-in algorithm
5. inspect the result as Arrow data

Source examples:

- [examples/python/tutorials/01_read_and_inspect.py](../../examples/python/tutorials/01_read_and_inspect.py)
- [examples/python/tutorials/02_lazy_expand.py](../../examples/python/tutorials/02_lazy_expand.py)
- [examples/python/tutorials/03_first_algorithm.py](../../examples/python/tutorials/03_first_algorithm.py)

## 1. Pick an Input Graph

If you are working from a GitHub checkout, reuse the shared example file:

`examples/data/example_simple.gf`

If you installed only from PyPI, create a local `social.gf` file with the same contents as that example.

## 2. Load the Graph

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

print("nodes:", g.node_count())
print("edges:", g.edge_count())
print("density:", g.density())
print("node columns:", g.nodes().column_names())
print("edge columns:", g.edges().column_names())
```

At this point you have an eager `GraphFrame`.
You can inspect counts immediately and call algorithms directly on it.

## 3. Build a Lazy Query

Now switch to the lazy API:

```python
result = (
    g.lazy()
    .filter_nodes(lx.col("_id") == "alice")
    .expand(edge_type="KNOWS", hops=2, direction="out")
    .collect()
)

print("expanded nodes:", result.node_count())
print("expanded edges:", result.edge_count())
```

This query does three things:

- starts from the node whose `_id` is `alice`
- follows only `KNOWS` edges
- expands two hops outward

No work happens until `.collect()` is called.

## 4. Run an Algorithm

You can also call eager graph algorithms directly on the loaded graph.
For example, shortest path:

```python
path = g.shortest_path("alice", "charlie")
print(path)
```

On this graph, the result should be the path through `bob`.

You can also run ranking-style algorithms:

```python
ranks = g.pagerank()
print(ranks.head(5, sort_by="pagerank", descending=True))
```

## 5. Inspect Results as Arrow

Lynxes frames can be inspected as PyArrow record batches:

```python
node_batch = result.nodes().to_pyarrow()
edge_batch = result.edges().to_pyarrow()

print(node_batch)
print(edge_batch)
```

This is useful when you want to hand results off to Arrow-based tooling without inventing a separate graph wrapper layer.

## Common Patterns

### Collect only nodes

```python
people = (
    g.lazy()
    .filter_nodes(lx.col("_label").contains("Person"))
    .collect_nodes()
)
```

### Collect only edges

```python
knows_edges = (
    g.lazy()
    .filter_edges(lx.col("_type") == "KNOWS")
    .collect_edges()
)
```

### Expand without restricting edge type

```python
subgraph = (
    g.lazy()
    .filter_nodes(lx.col("_id") == "alice")
    .expand(hops=1, direction="out")
    .collect()
)
```

## What to Learn Next

After this page, the next useful topics are:

- file format workflows with `.gf`, `.gfb`, and parquet
- expression building with `lx.col(...)`
- traversal semantics and direction handling
- graph algorithms such as PageRank, shortest path, and community detection

If you prefer a terminal-first workflow, continue with the [CLI Quickstart](cli.md).
