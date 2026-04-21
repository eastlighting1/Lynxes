# Save A Graph And Validate A Round-Trip

## The Problem

Use this recipe when you need to save a graph to another Lynxes-supported format and then confirm that the converted graph still looks like the one you started with. This is the recipe to use when the task is not just "write a file" but "write a file and make sure I did not silently break the graph."

## Prerequisites

This recipe assumes:

- the source graph loads successfully before conversion
- you have write access to the destination path
- you are prepared to inspect counts and schema after the round-trip

The examples below use `examples/data/example_simple.gf`.

## The Recipe

Python round-trip from `.gf` to `.gfb`:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
g.write_gfb("example.gfb")

restored = lx.read_gfb("example.gfb")
print("source:", g.node_count(), g.edge_count())
print("restored:", restored.node_count(), restored.edge_count())
print("restored node columns:", restored.nodes().column_names())
print("restored edge columns:", restored.edges().column_names())
```

The node and edge counts should match before and after the round-trip.

CLI version:

```bash
cargo run -p lynxes-cli -- convert examples/data/example_simple.gf example.gfb --compression zstd
cargo run -p lynxes-cli -- inspect example.gfb
```

If you are round-tripping from parquet through a `GraphFrame`, the same pattern still applies:

```python
import lynxes as lx

g = lx.read_parquet_graph("nodes.parquet", "edges.parquet")
g.write_gfb("example.gfb")
restored = lx.read_gfb("example.gfb")

print(restored.node_count(), restored.edge_count())
```

## What To Check

After any round-trip, check at least these things:

- node count
- edge count
- node reserved columns
- edge reserved columns
- labels and edge types if they matter to the workflow

Counts are the fastest first pass, but they are not the whole story. A graph can preserve counts and still lose important schema details if the workflow is not what you thought it was.

## Side Effects And Limits

A successful conversion does not automatically mean the graph is semantically identical in every way that matters to your application. For small local validations, counts and reserved columns are often enough. For more important workflows, inspect representative rows as well.

It is also useful to separate "conversion succeeded" from "my downstream workflow is still correct." The first is a format question. The second is an application question. Cookbook recipes can help you validate the first one quickly, but they cannot replace checking the second in context.

If the graph will be re-used many times locally, `.gfb` is often a practical target. If the next system in the pipeline expects parquet-shaped data, the format choice may be different even when the round-trip itself works.

## Related Recipes

If you want to build a graph result first and export it afterward, start from a query recipe such as [Build an ego network around one seed node](ego-network.md).
If you want a deeper beginner-oriented walkthrough before doing conversions, return to the guides section.
