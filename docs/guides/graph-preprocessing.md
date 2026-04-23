# Graph Preprocessing

This guide shows the intended shape of preprocessing work in Lynxes: start from a regular `GraphFrame`, move into `MutableGraphFrame` for a bounded rewrite phase, then `freeze()` back into a normal graph before continuing.

The point is not to keep a graph mutable forever. The point is to make one explicit cleanup or reshaping step part of the workflow instead of an external one-off script.

For simple preprocessing flows, the Python mutator methods are fluent. That means you can chain rewrites and end with `freeze()` directly. `freeze()` already performs the compaction it needs.

## When To Use This Guide

Use this guide when you need to do one of the following before running the rest of your workflow:

- remove known noisy nodes
- remove a set of edges you do not want to train or rank over
- append temporary nodes before an experiment
- rebuild a graph after a cleanup step

If your goal is continuous operational writes, this is still not the right engine path.

## Starting Point

This guide assumes you already have a `GraphFrame`.

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
```

## Move Into A Mutable Rewrite Phase

```python
mutable = g.into_mutable()
```

At this point you are no longer working with the ordinary eager graph wrapper. You are in a bounded preprocessing phase.

## Delete A Node

If you know one node should not survive into the final graph:

```python
mutable.delete_node("charlie")
```

This is a logical removal step. In practical terms, you should think of it as "this node should not appear in the next frozen graph."

## Delete A Stable Edge

If one known edge row should be removed:

```python
mutable.delete_edge(0)
```

This is useful when you already know the stable edge row you want to exclude.

## Append A New Edge

If you want to add one relationship before rebuilding:

```python
mutable.add_edge("alice", "diana")
```

This is the kind of thing you might do when preparing a training graph or building a temporary supervision signal.

## Append Nodes

Node appends use `NodeFrame`. If you want to stay inside Lynxes instead of hand-building Arrow batches, the easiest path is to describe the extra nodes as a tiny `.gf` snippet, load that graph, and reuse its node frame.

```python
from pathlib import Path
from tempfile import TemporaryDirectory

node_source = """
(erin: Person { age: 29, score: 0.6 })
""".strip()

with TemporaryDirectory() as tmp:
    path = Path(tmp) / "new_nodes.gf"
    path.write_text(node_source, encoding="utf-8")
    new_nodes = lx.read_gf(path).nodes()

mutable.add_node(new_nodes)
```

If you already have several rows ready, prefer `add_nodes_batch(...)` instead of repeated single-row calls.

The same pattern works for a batch:

```python
batch_source = """
(erin: Person { age: 29, score: 0.6 })
(frank: Person { age: 33, score: 0.5 })
""".strip()

with TemporaryDirectory() as tmp:
    path = Path(tmp) / "batch_nodes.gf"
    path.write_text(batch_source, encoding="utf-8")
    batch_nodes = lx.read_gf(path).nodes()

mutable.add_nodes_batch(batch_nodes)
```

## Compact Before You Freeze

```python
mutable.compact()
```

You do not need to call `compact()` before `freeze()` for the normal one-shot preprocessing path. Keep it for explicit checkpoints or for a repeated read-heavy mutable phase where you want to publish a new stable snapshot before continuing.

## Freeze Back Into A Regular Graph

```python
clean = mutable.freeze()
```

`clean` is now a plain `GraphFrame` again. From here you can go back to the rest of the engine:

- `clean.lazy()`
- `clean.pagerank()`
- `clean.sample_neighbors(...)`
- export paths such as `write_gfb(...)`

## What To Check

The easiest checks are:

```python
print(clean.node_count())
print(clean.edge_count())
print(clean.nodes().ids())
```

You should see the rewritten graph shape, not the original one.

## A Common Pattern

The clean mental model is:

```python
g = lx.read_gf("examples/data/example_simple.gf")
g2 = (
    g.into_mutable()
    .delete_node("charlie")
    .add_edge("alice", "diana")
    .freeze()
)
```

Treat that middle section as preprocessing. Do not treat it as a long-lived mutable graph session.

## Where To Go Next

If your next step is minibatch training, continue with [GNN integration](gnn-integration.md). If your next step is pattern extraction, continue with [KG pattern matching](kg-pattern-matching.md). If you want exact Python method signatures, use [the `MutableGraphFrame` reference](../reference/python/mutablegraphframe.md).
