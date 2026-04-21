# Verify Your Install

This guide is the quickest way to confirm that a Lynxes install is actually usable.
It is intentionally small, self-contained, and designed to avoid file-path problems. If this page works, you know that the Python package imports correctly and that you can build a tiny graph, inspect it, and run one simple graph operation.

That may not sound like much, but it is exactly the kind of first success signal a new user needs. Before reading about file formats, lazy queries, or connectors, it is worth getting one result on screen that is unambiguously correct.

## Before You Start

This page assumes that you have already installed the Python package and can open a Python REPL, notebook cell, or small script.

If you have not installed Lynxes yet, go back to `docs/install.md` first. Then return here and run the code below exactly as written.

## Build A Tiny Graph In Memory

Paste this into Python:

```python
import pyarrow as pa
import lynxes as lx

nodes = pa.record_batch(
    {
        "_id": ["alice", "bob", "carol"],
        "_label": ["Person", "Person", "Person"],
        "age": [31, 29, 35],
    }
)

edges = pa.record_batch(
    {
        "_src": ["alice", "bob"],
        "_dst": ["bob", "carol"],
        "_type": ["KNOWS", "KNOWS"],
        "_direction": ["out", "out"],
    }
)

g = lx.GraphFrame(nodes, edges)

print("nodes:", g.node_count())
print("edges:", g.edge_count())
print("node columns:", g.nodes().column_names())
print("edge columns:", g.edges().column_names())
```

If everything is working, you should see output with this shape:

```text
nodes: 3
edges: 2
node columns: ['_id', '_label', 'age']
edge columns: ['_src', '_dst', '_type', '_direction']
```

The exact quote style may differ slightly depending on the Python environment, but the counts and the column names should match.

## What This Step Proves

This first step is doing more work than it looks like.
It proves that:

- Python can import `lynxes`
- the Arrow dependency chain is working
- a `GraphFrame` can be constructed successfully
- Lynxes recognizes the reserved graph columns

If you do not get the counts shown above, stop here and fix this environment before moving on. Later guides assume this part already works.

## Run One Tiny Graph Operation

Now add this:

```python
result = (
    g.lazy()
    .filter_nodes(lx.col("_id") == "alice")
    .expand(edge_type="KNOWS", hops=1, direction="out")
    .collect()
)

print("expanded nodes:", result.node_count())
print("expanded edges:", result.edge_count())
print("expanded ids:", result.nodes().to_pyarrow()["_id"].to_pylist())
```

You should see something like:

```text
expanded nodes: 2
expanded edges: 1
expanded ids: ['alice', 'bob']
```

This is your first clear sign that the graph engine is not just importable, but usable. The query begins from `alice`, follows one `KNOWS` edge outward, and materializes the reachable subgraph.

## If It Fails

The two most common failure modes at this point are simpler than they look:

- `ModuleNotFoundError` or import failure: the package is not installed in the Python environment you are actually running
- Arrow or constructor-related exceptions: one of the reserved columns is missing or has the wrong shape

Do not try to debug more advanced docs until this page works. This page is intentionally free of file paths and external systems, so it is the best first place to confirm that the local environment is sound.

## Where To Go Next

If this guide worked, continue with [Getting Started in Python](getting-started-python.md). That guide keeps the same spirit of a controlled happy path, but moves from an in-memory graph to a more realistic workflow using a shared example file.
