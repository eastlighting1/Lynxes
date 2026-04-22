# KG Pattern Matching

This guide walks through the current pattern-matching path in Lynxes from Python. The important change is that `match_pattern(...).collect()` now returns a real result instead of stopping at an unsupported executor path.

The result is not a `GraphFrame`. It is a pyarrow `RecordBatch` whose columns are prefixed with pattern aliases.

## A Minimal Pattern

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

result = (
    g.lazy()
    .match_pattern(
        [
            lx.node("a", "Person"),
            lx.edge("KNOWS"),
            lx.node("b", "Person"),
        ]
    )
    .collect()
)
```

## What Comes Back

`result` is a pyarrow `RecordBatch`.

Typical columns look like:

- `a._id`
- `a._label`
- `a.age`
- `b._id`
- `b._label`
- `b.age`

If you name an edge alias in the pattern, edge-prefixed columns appear too.

## Why The Result Is Tabular

Pattern match output is a binding table, not a graph-shaped result. Each row is one matched assignment of aliases. That is why `collect()` for pattern plans returns a batch rather than a `GraphFrame`.

This is the same reason the alias prefixes matter. They preserve which bound value came from which part of the pattern.

## Add A `where_` Predicate

```python
filtered = (
    g.lazy()
    .match_pattern(
        [
            lx.node("a", "Person"),
            lx.edge("KNOWS"),
            lx.node("b", "Person"),
        ],
        where_=lx.col("a.age") > 25,
    )
    .collect()
)
```

The filter is evaluated over the pattern bindings. In other words, `a.age` is not a plain column reference from a node frame. It is a column reference qualified by the pattern alias.

## A Two-Hop Pattern

```python
two_hop = (
    g.lazy()
    .match_pattern(
        [
            lx.node("a", "Person"),
            lx.edge("KNOWS"),
            lx.node("b", "Person"),
            lx.edge("WORKS_AT"),
            lx.node("c", "Company"),
        ]
    )
    .collect()
)
```

This returns one row per successful alias chain across both hops.

## What To Check

The easiest checks are:

```python
print(type(result))
print(result.schema.names)
print(result.num_rows)
```

If pattern execution succeeded, you should see a pyarrow `RecordBatch`, alias-prefixed column names, and a row count that matches the number of successful bindings.

## Current Shape Of The Feature

This path is now real and executable, but it is still best understood as a focused pattern-binding feature rather than a full declarative graph query language. The current strength is extracting typed path-shaped bindings into a tabular result that Python can immediately inspect or hand off downstream.

That is already useful for:

- KG slice extraction
- relation-oriented feature generation
- preparing batches for downstream dataframe or ML work

## Where To Go Next

If you want the exact Python surface, continue with [the `LazyGraphFrame` reference](../reference/python/lazygraphframe.md). If your next step is turning graph structure into GNN input, continue with [GNN integration](gnn-integration.md).
