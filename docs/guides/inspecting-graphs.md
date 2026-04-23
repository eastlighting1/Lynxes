# Inspecting Graphs

This guide covers the fastest ways to sanity-check a graph after loading it.

## Inspect in Python

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

print(g.node_count())
print(g.edge_count())
print(g.density())
print(g.nodes().column_names())
print(g.edges().column_names())
```

Useful first checks are:

- node count
- edge count
- density
- node column names
- edge column names

## Inspect Result Frames

When you want to look past counts, inspect the frames directly:

```python
print(g.nodes())
print(g.edges())
```

This is the easiest way to verify:

- `_id` values
- labels
- edge endpoints
- attribute columns

## Inspect on the CLI

```bash
lynxes inspect examples/data/example_simple.gf
```

Use CLI inspection when you want:

- a fast format sanity check
- a quick summary without opening Python
- a first look at `.gfb` files

## Sanity Checks After Load

After loading a graph, confirm:

1. counts look plausible
2. reserved columns are present
3. labels and edge types look expected
4. the graph shape matches your intended workflow

If any of these are off, continue with [Errors and Debugging](errors-and-debugging.md).
