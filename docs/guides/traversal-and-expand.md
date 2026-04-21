# Traversal and Expand

This guide explains the graph-native traversal flow exposed through `LazyGraphFrame.expand(...)`.

Source examples:

- [examples/python/tutorials/02_lazy_expand.py](../../examples/python/tutorials/02_lazy_expand.py)
- [examples/cli/query.md](../../examples/cli/query.md)

## Start With a Seed Set

In Lynxes, `expand(...)` is typically used after choosing a seed set with `filter_nodes(...)`.

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

seeded = g.lazy().filter_nodes(lx.col("_id") == "alice")
```

## Expand One Hop

```python
result = seeded.expand(hops=1, direction="out").collect()
```

This returns a subgraph, not only a node list.
The collected result includes:

- the reached nodes
- the traversed edges needed to represent that result subgraph

## Expand Multiple Hops

```python
result = seeded.expand(hops=2, direction="out").collect()
```

On `example_simple.gf`, a two-hop outward traversal from `alice` reaches `charlie`.

## Restrict by Edge Type

```python
result = (
    g.lazy()
    .filter_nodes(lx.col("_id") == "alice")
    .expand(edge_type="KNOWS", hops=2, direction="out")
    .collect()
)
```

This is useful when the graph mixes relationship types and you only want one traversal channel.

## Direction Options

Current Python direction strings are:

- `out`
- `in`
- `both`
- `none`

The CLI also accepts `undirected`, so when you move between the two surfaces it is worth checking the exact accepted values instead of assuming they are identical.

## CLI Equivalent

```bash
lynxes query examples/data/example_simple.gf --from alice --hops 2 --direction out
```

And with edge-type restriction:

```bash
lynxes query examples/data/example_simple.gf --from alice --hops 2 --edge-type KNOWS --direction out
```

## When to Use `expand(...)`

Use `expand(...)` when you want:

- BFS-style neighborhood exploration
- a subgraph result you can inspect or write out
- a lazy traversal that composes with filters

Use eager algorithms instead when the goal is ranking, shortest path, connectivity, or community assignment rather than neighborhood expansion itself.
