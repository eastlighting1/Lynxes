# Python Reference

The Python surface is where most users first touch Lynxes, but this section is not trying to be a first-time introduction. It is here for the moment when you already have the overall model in your head and need to check the exact public surface: which methods live on `GraphFrame`, which methods stay lazy, what shape comes back from an algorithm call, and what kind of exception to expect when a load or query goes wrong.

The pages below stay on the Python side of the boundary even though the implementation ultimately comes from Rust and PyO3. That means parameter names, return values, and error conditions are described in Python-facing terms. If a Python call raises `ValueError` or `KeyError`, this section should say so directly instead of making you reverse-engineer the Rust error enum.

## Core Objects

- [`GraphFrame`](graphframe.md)
- [`LazyGraphFrame`](lazygraphframe.md)
- [`NodeFrame`](nodeframe.md)
- [`EdgeFrame`](edgeframe.md)

## Expressions

- [`Expr`](expr.md)
- [`AggExpr`](aggexpr.md)

## I/O And Integration

- [Module I/O functions](module-io.md)
- [Graph export methods](graph-export.md)
- [Connectors](connectors.md)
- [Python error mapping](errors.md)

## Reading Order

If you are looking up a method on an object you already have in hand, start with that object page. If you are checking how a graph enters or leaves Python, go to the I/O pages. If you are debugging a failure and want to know why a call turned into `ValueError`, `TypeError`, or `RuntimeError`, use the error mapping page as the bridge back to the underlying engine behavior.
