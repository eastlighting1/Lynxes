# Getting Started In Python

This guide is the first full Python walkthrough for Lynxes.
The goal is not to show every feature. The goal is to take a new user from loading a graph to inspecting it, running a small lazy traversal, and calling an eager algorithm, all while giving them concrete output to compare against.

If you have not already confirmed that the package imports and that Lynxes can load a tiny self-authored graph, start with [Verify Your Install](verify-your-install.md). That page is deliberately smaller and catches environment problems earlier.

## What You Will Do

By the end of this guide you will have:

- loaded a real graph file
- checked that the graph looks sane
- built one lazy traversal query
- run one eager algorithm

None of these steps is individually complicated. The value of the guide is that they happen in a sequence that makes the result easy to validate.

## Use The Shared Example Graph

If you are working from a repository checkout, use the shared example file:

`examples/data/example_simple.gf`

This guide assumes that path is available. If you installed only from PyPI and do not have the repository checkout, either clone the repository or adapt the same steps to one of your own local `.gf` files.

One design distinction is worth stating explicitly before you start. In Lynxes, "create a graph from Python data" and "load a graph from disk" are different workflows.

- if you want to construct a graph from Python data, use `graph(nodes=..., edges=...)`
- if you want to load an existing graph file, use `read_gf(...)`, `read_gfb(...)`, or another loader

This guide is about the second workflow: loading an existing graph file and then using it. The constructor workflow is covered first in [Verify Your Install](verify-your-install.md).

## Step 1: Load The Graph

Run this first:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

print("nodes:", g.node_count())
print("edges:", g.edge_count())
```

You should see:

```text
nodes: 5
edges: 4
```

If you get a file-not-found error here, do not skip ahead. The rest of the guide depends on this exact file loading correctly.

## Step 2: Inspect The Shape

Now ask the graph what columns it contains:

```python
print("node columns:", g.nodes().column_names())
print("edge columns:", g.edges().column_names())
```

You should see something with this shape:

```text
node columns: ['_id', '_label', ...]
edge columns: ['_src', '_dst', '_type', '_direction', ...]
```

The exact extra columns depend on the example data, but the reserved columns should be there. This step is simple, but it is important. A graph that loads without the expected reserved columns is not a graph you want to build further work on.

If you want a more direct look at the payload, add:

```python
print(g.nodes())
print(g.edges())
```

At this point you should be able to recognize node ids, labels, and edge endpoints by eye.

## Step 3: Build A Small Lazy Query

Now switch from eager inspection to a lazy traversal:

```python
result = (
    g.lazy()
    .filter_nodes(lx.col("_id") == "alice")
    .expand(edge_type="KNOWS", hops=2, direction="out")
    .collect()
)

print("expanded nodes:", result.node_count())
print("expanded edges:", result.edge_count())
print("expanded ids:", result.nodes().ids())
```

On the shared example graph, the result should look like this:

```text
expanded nodes: 4
expanded edges: 3
expanded ids: ['alice', 'bob', 'charlie', 'diana']
```

What just happened is worth saying plainly. The query did not start by traversing the whole graph. It started by choosing `alice` as a seed node, then expanded outward along `KNOWS` edges, then materialized the resulting subgraph at `collect()`.

This is one of the first places where Lynxes feels different from "just tables." The result is not a node list with some joins hidden in the background. It is a graph-shaped result.

## Step 4: Run One Eager Algorithm

Now try one eager call:

```python
path = g.shortest_path("alice", "charlie")
print("shortest path:", path)
```

The exact printed structure may vary slightly by environment, but it should clearly describe the path from `alice` to `charlie`.

The important point is not just the output. It is that eager algorithms and lazy queries coexist on the same `GraphFrame`. You do not have to change object models in order to switch from traversal-building to algorithm execution.

## What To Check Before You Move On

Before leaving this guide, make sure all four checkpoints are true:

1. the `.gf` file loaded without error
2. node and edge counts match the expected example
3. the lazy query returns the three expected node ids
4. `shortest_path("alice", "charlie")` returns a valid path

If one of these does not hold, that is more valuable than racing ahead. It means the environment or the file path is still not in a stable state.

## Why This Is The First Python Path

This guide is intentionally small, but it already covers the three interactions that matter most for a first Lynxes impression:

- load a graph
- inspect the graph
- do one structural query and one direct algorithm call

That is enough to answer the question most new users actually have: "Is this just another parser, or is there a usable graph engine here?" Once you have walked through this page successfully, the rest of the docs become much easier to place.

## Where To Go Next

If the lazy query was the most interesting part, continue with [Your First Graph Query](first-graph-query.md).
If the eager call was what you cared about, continue with [Your First Algorithm Run](first-algorithm-run.md).
