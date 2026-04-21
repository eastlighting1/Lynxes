# Errors and Debugging

This guide is for the common failure modes users hit when loading, querying, or exporting graphs.

## Common Error Categories

The most common problems are:

- malformed `.gf` syntax
- missing files
- duplicate node ids
- dangling edges that reference unknown nodes
- missing reserved columns in parquet input
- invalid direction strings
- unknown node ids in graph algorithms

## Read the Error Message First

Lynxes already surfaces many failures with direct messages such as:

- file not found
- parse error with line or token context
- node not found

In Python, these may surface as `OSError`, `ValueError`, `RuntimeError`, or `KeyError` depending on the path that failed.

## A Practical Debugging Workflow

When something breaks, use this order:

1. Confirm the input file path is correct.
2. Inspect the graph with the CLI if possible.
3. Reduce the failing graph to a tiny repro.
4. Re-run the same operation on `examples/data/example_simple.gf`.
5. Compare schema, reserved columns, and edge direction usage.

## Check the Graph Shape

In Python:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
print(g.nodes().column_names())
print(g.edges().column_names())
```

On the CLI:

```bash
lynxes inspect examples/data/example_simple.gf
```

## Reserved Columns to Watch

These columns have special meaning:

- node side: `_id`, `_label`
- edge side: `_src`, `_dst`, `_type`, `_direction`

If parquet input is missing required reserved columns, loading will fail.

## Minimal Repro Strategy

If a large graph fails:

- copy the failing pattern into a tiny `.gf`
- keep only the nodes and edges needed to reproduce the issue
- remove optional attributes until the problem becomes obvious

This is especially effective for parse errors and shortest-path edge cases.

## Format-Specific Tips

For `.gf`:

- keep edge properties after the full edge declaration
- prefer the shared examples as syntax references

For parquet:

- verify nodes and edges are in separate files
- verify reserved column names are present exactly as expected

For `.gfb`:

- use `lynxes inspect graph.gfb` first

## Unsupported or Not Yet Implemented Paths

Some API names exist but are not fully usable yet.
If a workflow raises an explicit unsupported or not-implemented error, treat that as current shipped behavior rather than a local setup problem.
