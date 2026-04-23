# `Expr` Reference

`Expr` is the expression type used in lazy filters, projections, and pattern `where` clauses.

## Constructors

| Function | Description |
| :--- | :--- |
| `lynxes.col(name)` | Reference a column by name. |
| `lynxes.lit(value)` | A scalar literal — `int`, `float`, `str`, or `bool`. |

```python
import lynxes as lx

age_expr = lx.col("age")
thirty = lx.lit(30)
```

## Comparison Operators

All standard Python comparison operators return a Boolean `Expr`.

```python
lx.col("age") > 25
lx.col("age") >= 18
lx.col("age") == 42
lx.col("age") != 0
lx.col("score") < 0.5
lx.col("score") <= 1.0
```

## Arithmetic Operators

```python
lx.col("price") * lx.col("quantity")   # multiply
lx.col("revenue") - lx.col("cost")     # subtract
lx.col("a") + lx.col("b")             # add
lx.col("total") / lx.lit(100)          # divide
```

## Boolean Operators

Use `&` and `|` to combine Boolean expressions. Python's `and`/`or` do not work on `Expr`.

```python
(lx.col("age") > 18) & (lx.col("active") == True)
(lx.col("role") == "admin") | (lx.col("role") == "owner")
```

## String Namespace — `.str`

Access string methods through the `.str` accessor.

| Method | Description |
| :--- | :--- |
| `.str.contains(substr)` | True if the column value contains `substr`. |
| `.str.startswith(prefix)` | True if the value starts with `prefix`. |
| `.str.endswith(suffix)` | True if the value ends with `suffix`. |

```python
lx.col("_id").str.contains("alice")
lx.col("name").str.startswith("Dr.")
lx.col("email").str.endswith("@example.com")
```

## Pattern Column References

Inside a `filter_pattern` `where` clause, use dotted alias references to address fields on pattern-bound nodes and edges.

```python
# In a pattern context: a.age refers to the "age" column of alias "a"
lx.col("a.age") > 30
lx.col("rel.weight") > 0.5
```

## Type Casting — `.cast(dtype)`

Cast a column to a different Lynxes type.

```python
lx.col("score_str").cast("Float64")
lx.col("flag").cast("Bool")
```

Supported dtype strings: `"Int8"`, `"Int16"`, `"Int32"`, `"Int64"`, `"UInt8"`, `"UInt16"`, `"UInt32"`, `"UInt64"`, `"Float32"`, `"Float64"`, `"Bool"`, `"String"`.

## Aliasing — `.alias(name)`

Rename an expression output column.

```python
(lx.col("revenue") - lx.col("cost")).alias("profit")
```

## Example — filter and project

```python
import lynxes as lx

result = (
    lx.read_gf("examples/data/example_simple.gf")
    .lazy()
    .filter_nodes(
        (lx.col("age") >= 18) & lx.col("_id").str.startswith("u")
    )
    .collect_nodes()
)
```

## See Also

- [`AggExpr`](aggexpr.md) — aggregation expressions used in `aggregate_neighbors`
- [`LazyGraphFrame`](lazygraphframe.md) — where expressions are consumed
