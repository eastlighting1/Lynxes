# Filtering and Selection

This guide covers the expression-based filtering flow used by `LazyGraphFrame`.

## Start With `lynxes.col(...)`

Use `lynxes.col(name)` to build expressions.

```python
import lynxes as lx

expr = lx.col("age") > 25
```

## Filter Nodes

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

people = (
    g.lazy()
    .filter_nodes(lx.col("_label").contains("Person"))
    .collect_nodes()
)
```

## Filter Edges

```python
knows_edges = (
    g.lazy()
    .filter_edges(lx.col("_type") == "KNOWS")
    .collect_edges()
)
```

## Chain Filters

```python
result = (
    g.lazy()
    .filter_nodes(lx.col("age") > 25)
    .filter_nodes(lx.col("age") < 35)
    .collect_nodes()
)
```

## String Filters

The current string namespace supports:

- `.str.contains(...)`
- `.str.startswith(...)`
- `.str.endswith(...)`

Example:

```python
result = (
    g.lazy()
    .filter_nodes(lx.col("_id").str.contains("li"))
    .collect_nodes()
)
```

## Selection

Lynxes also exposes node and edge selection methods on the lazy surface.
Use them when you want to narrow the visible columns before collecting.

## Collect Shapes

- `collect()` returns a `GraphFrame`
- `collect_nodes()` returns a `NodeFrame`
- `collect_edges()` returns an `EdgeFrame`

Choose the smallest result shape that matches the job you are doing.
