# `LazyGraphFrame`

`LazyGraphFrame` is the lazy query builder in the Python surface. It is returned by `GraphFrame.lazy()` and by connector entry points such as `lynxes.read_neo4j(...)`, `lynxes.read_arangodb(...)`, and `lynxes.read_sparql(...)`. The important contract is simple: the object describes graph work, but it does not materialize a result until one of the collection methods is called.

That means methods like `filter_nodes(...)`, `filter_edges(...)`, `select_nodes(...)`, and `expand(...)` return another `LazyGraphFrame`. They modify the logical plan rather than computing a new eager graph immediately.

## Construction

Typical construction starts from an eager graph:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
lazy = g.lazy()
```

Connector entry points also return `LazyGraphFrame` directly, because the graph source itself may be external and should stay lazy until collection.

## Method Summary

### Plan-building methods

| Method | Returns | Notes |
| :--- | :--- | :--- |
| `filter_nodes(expr)` | `LazyGraphFrame` | Adds a node predicate to the plan. |
| `filter_edges(expr)` | `LazyGraphFrame` | Adds an edge predicate to the plan. |
| `select_nodes(columns)` | `LazyGraphFrame` | Restricts visible node columns. |
| `select_edges(columns)` | `LazyGraphFrame` | Restricts visible edge columns. |
| `expand(edge_type=None, hops=1, direction="out")` | `LazyGraphFrame` | Adds a traversal expansion step. |
| `aggregate_neighbors(edge_type, agg)` | `LazyGraphFrame` | Adds a neighbor-aggregation step. |
| `match_pattern(steps, where_=None)` | `LazyGraphFrame` | Pattern-oriented surface that materializes tabular binding results. |
| `explain()` | `str` | Renders the current logical plan. |

### Materialization methods

| Method | Returns | Notes |
| :--- | :--- | :--- |
| `collect()` | `GraphFrame \| pyarrow.RecordBatch` | Materializes either a graph-shaped result or a pattern-row result. |
| `collect_nodes()` | `NodeFrame` | Materializes only the node-side result. |
| `collect_edges()` | `EdgeFrame` | Materializes only the edge-side result. |

## Selected Methods

### `filter_nodes(expr) -> LazyGraphFrame`

Add a node-side predicate.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `expr` | `Expr` | Required | - | Node predicate expression. |

#### Returns

Returns another `LazyGraphFrame`. The graph is still not materialized.

#### Raises

- `TypeError` if `expr` is not a valid Python expression wrapper

### `select_nodes(columns) -> LazyGraphFrame`

Restrict visible node columns in the lazy result.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `columns` | `list[str]` | Required | - | Node columns to retain. |

#### Returns

Returns another `LazyGraphFrame`.

#### Raises

- `TypeError` if `columns` is not a Python sequence of strings

### `expand(edge_type=None, hops=1, direction="out") -> LazyGraphFrame`

Add a traversal expansion step to the plan.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `edge_type` | `None \| str \| list[str]` | Optional | `None` | Optional edge-type restriction. `None` means any type. |
| `hops` | `int` | Optional | `1` | Traversal hop count. Must be greater than zero. |
| `direction` | `str` | Optional | `"out"` | Traversal direction. Accepted values are `"out"`, `"in"`, `"both"`, and `"none"`. |

#### Returns

Returns another `LazyGraphFrame`. No collection happens yet.

#### Raises

- `ValueError` if `hops` is zero
- `ValueError` if `direction` is invalid
- `TypeError` if `edge_type` is not `None`, a string, or a sequence of strings

### `aggregate_neighbors(edge_type, agg) -> LazyGraphFrame`

Add a neighbor aggregation step.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `edge_type` | `str` | Required | - | Edge type to aggregate over. |
| `agg` | `AggExpr` | Required | - | Aggregation expression. |

#### Returns

Returns another `LazyGraphFrame`.

### `match_pattern(steps, where_=None) -> LazyGraphFrame`

Add a pattern-style operation to the lazy plan.

#### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `steps` | `list[...]` | Required | - | Pattern step description. |
| `where_` | `Expr \| None` | Optional | `None` | Optional filter expression. |

### `collect() -> GraphFrame | pyarrow.RecordBatch`

Materialize the lazy plan.

#### Returns

Returns an eager `GraphFrame` for graph-domain plans. For `match_pattern(...)` plans, returns a pyarrow `RecordBatch` with alias-prefixed columns such as `a._id` or `e._type`.

#### Raises

- `ValueError`, `KeyError`, `OSError`, or `RuntimeError` depending on where plan execution fails

### `collect_nodes() -> NodeFrame`

Materialize only the node-side result.

### `collect_edges() -> EdgeFrame`

Materialize only the edge-side result.

## Notes

`LazyGraphFrame` does not hold a ready-made graph result. That is why plan-building methods keep returning the same lazy wrapper type, and it is why return-type boundaries matter here. If you need a real `GraphFrame`, `NodeFrame`, `EdgeFrame`, or pattern-row batch, collection is the point where the work actually happens.
