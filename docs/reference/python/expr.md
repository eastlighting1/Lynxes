# `Expr` Reference

`Expr` is the expression surface used in lazy filters and selections.

## Entry Point

Use `lynxes.col(name)` to start an expression.

```python
import lynxes as lx

expr = lx.col("age") > 25
```

## Common Operations

- comparisons such as `==`, `>`, `<`
- string namespace methods through `.str`
- alias-aware pattern references such as `a.age` in pattern workflows

## String Namespace

Current tests cover:

- `.str.contains(...)`
- `.str.startswith(...)`
- `.str.endswith(...)`

## Example

```python
import lynxes as lx

people = (
    lx.read_gf("examples/data/example_simple.gf")
    .lazy()
    .filter_nodes(lx.col("_id").str.contains("li"))
    .collect_nodes()
)
```
