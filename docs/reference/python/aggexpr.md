# `AggExpr` Reference

`AggExpr` represents an aggregation over a set of neighbor values. It is consumed by `LazyGraphFrame.aggregate_neighbors(...)`.

## Constructors

| Function | Description |
| :--- | :--- |
| `lynxes.count()` | Count of neighbors (ignores column values). |
| `lynxes.sum(expr)` | Sum of `expr` across neighbors. |
| `lynxes.mean(expr)` | Arithmetic mean of `expr` across neighbors. |
| `lynxes.min(expr)` | Minimum value of `expr` across neighbors. |
| `lynxes.max(expr)` | Maximum value of `expr` across neighbors. |
| `lynxes.first(expr)` | First neighbor value of `expr` in traversal order. |
| `lynxes.last(expr)` | Last neighbor value of `expr` in traversal order. |
| `lynxes.list(expr)` | Collect all neighbor values of `expr` into a list. |

The `expr` argument is an [`Expr`](expr.md) — typically `lx.col("some_column")`.

```python
import lynxes as lx

lx.count()
lx.sum(lx.col("weight"))
lx.mean(lx.col("score"))
lx.min(lx.col("latency_ms"))
lx.max(lx.col("rating"))
lx.first(lx.col("timestamp"))
lx.list(lx.col("tag"))
```

## Aliasing — `.alias(name)`

Every `AggExpr` can be renamed with `.alias(name)`. The alias becomes the output column name on the result `NodeFrame`.

```python
lx.count().alias("friend_count")
lx.sum(lx.col("weight")).alias("total_weight")
lx.mean(lx.col("score")).alias("avg_score")
```

Without `.alias(...)`, the output column is named automatically (e.g., `count`, `sum(weight)`).

## Usage — `aggregate_neighbors`

`aggregate_neighbors(edge_type, *aggs)` groups outgoing edges of `edge_type` by source node and applies each aggregation.

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

result = (
    g.lazy()
    .aggregate_neighbors(
        "KNOWS",
        lx.count().alias("friend_count"),
        lx.mean(lx.col("strength")).alias("avg_strength"),
    )
    .collect_nodes()
)
```

The output is a `NodeFrame` containing the original node columns plus one new column per aggregation. Nodes with no outgoing edges of the given type get `null` for numeric aggregations and `0` for `count`.

## Multiple Aggregations

Multiple `AggExpr` arguments are evaluated in a single pass over the neighbor edges.

```python
result = (
    g.lazy()
    .aggregate_neighbors(
        "FOLLOWS",
        lx.count().alias("follower_count"),
        lx.max(lx.col("since_year")).alias("latest_follow"),
        lx.list(lx.col("_dst")).alias("followed_ids"),
    )
    .collect_nodes()
)
```

## Type Behavior

| Aggregation | Input type | Output type |
| :--- | :--- | :--- |
| `count` | any | `Int64` |
| `sum` | numeric | same as input |
| `mean` | numeric | `Float64` |
| `min` / `max` | numeric or string | same as input |
| `first` / `last` | any | same as input |
| `list` | any | `List(input type)` |

## See Also

- [`Expr`](expr.md) — expressions used as arguments to aggregation constructors
- [`LazyGraphFrame.aggregate_neighbors`](lazygraphframe.md)
