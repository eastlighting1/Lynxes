# Save A Graph And Validate A Round-Trip

## The Problem

Use this recipe when you need to save a graph to another Lynxes-supported format and then confirm that the converted graph still looks like the one you started with. This is the recipe to use when the task is not just "write a file" but "write a file and make sure I did not silently break the graph."

That second part matters more than it sounds. File conversion is rarely the real goal. Usually the real goal is to trust the graph you are about to hand to another stage of the workflow, whether that means another Lynxes run, a Python pipeline, or an archived artifact you will reload later.

## Prerequisites

This recipe assumes:

- the source graph loads successfully before conversion
- you have write access to the destination path
- you are prepared to inspect counts and schema after the round-trip

The examples below use `examples/data/example_simple.gf`.

They stay intentionally small so that counts and a few representative rows are easy to compare by eye.

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

That is the minimum check, not the whole check.

CLI version:

```bash
cargo run -p lynxes-cli -- convert examples/data/example_simple.gf example.gfb --compression zstd
cargo run -p lynxes-cli -- inspect example.gfb
```

If you are round-tripping from parquet through a `GraphFrame`, the same pattern still applies:

```python
import lynxes as lx

source = lx.read_gf("examples/data/example_simple.gf")
source.write_parquet_graph("nodes.parquet", "edges.parquet")

g = lx.read_parquet_graph("nodes.parquet", "edges.parquet")
g.write_gfb("example.gfb")
restored = lx.read_gfb("example.gfb")

print(restored.node_count(), restored.edge_count())
```

If you already have a parquet node/edge pair, start from `read_parquet_graph(...)` directly and skip the `write_parquet_graph(...)` line above.

If the graph is important enough that counts alone are not reassuring, inspect one or two representative reserved columns after the round-trip:

```python
print(restored.nodes().ids()[:3])
print(restored.edges().to_pyarrow()["_type"].to_pylist()[:3])
```

For larger workflows, that second check is often the one that catches real mistakes. A graph can preserve counts while still losing the labels, types, or endpoint semantics you assumed were present.

## What To Check

After any round-trip, check at least these things:

- node count
- edge count
- node reserved columns
- edge reserved columns
- labels and edge types if they matter to the workflow

Counts are the fastest first pass, but they are not the whole story. A graph can preserve counts and still lose important schema details if the workflow is not what you thought it was.

For a more careful validation, inspect a few representative rows rather than all rows. Cookbook checks should stay cheap enough that you actually run them.

If the graph is going into a model pipeline or benchmark suite, record the before/after counts in the same script that performs the conversion. That makes the round-trip check repeatable instead of relying on manual inspection later.

## Side Effects And Limits

A successful conversion does not automatically mean the graph is semantically identical in every way that matters to your application. For small local validations, counts and reserved columns are often enough. For more important workflows, inspect representative rows as well.

It is also useful to separate "conversion succeeded" from "my downstream workflow is still correct." The first is a format question. The second is an application question. Cookbook recipes can help you validate the first one quickly, but they cannot replace checking the second in context.

If the graph will be re-used many times locally, `.gfb` is often a practical target. If the next system in the pipeline expects parquet-shaped data, the format choice may be different even when the round-trip itself works.

One common mistake is to validate only the final file and not the graph you actually exported. If the exported graph was already the wrong filtered subgraph, a perfect round-trip will still preserve the wrong thing faithfully. The validation question always starts one step earlier than the file format.

There is also a format-choice decision hidden inside this recipe. `.gfb` is usually the right target when the next consumer is Lynxes again and you want a compact reload path. Parquet interop is better when the next consumer is a table-oriented system. The conversion can be correct in both cases, but the "right" target still depends on the next stage.

## Related Recipes

If you want to build a graph result first and export it afterward, start from a query recipe such as [Build an ego network around one seed node](ego-network.md).
If you want a deeper beginner-oriented walkthrough before doing conversions, return to the guides section.
