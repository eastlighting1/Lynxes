# `AggExpr` Reference

Aggregation helpers are used in workflows such as `aggregate_neighbors(...)`.

## Helper Constructors

- `lynxes.count()`
- `lynxes.sum(expr)`
- `lynxes.mean(expr)`
- `lynxes.list(expr)`
- `lynxes.first(expr)`
- `lynxes.last(expr)`

## Aliasing

Aggregation outputs can be renamed with `.alias(...)`.

```python
import lynxes as lx

agg = lx.count().alias("friend_count")
```

## Typical Use

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
result = (
    g.lazy()
    .aggregate_neighbors("KNOWS", lx.count().alias("friend_count"))
    .collect_nodes()
)
```
