# Reference

The reference section is where Lynxes stops teaching and starts naming things precisely. When you already know the rough workflow and want to check an exact method name, accepted option value, return type, or file-shape rule, this is the part of the docs that should answer the question without making you read a tutorial again.

That distinction matters because the rest of the documentation has different jobs. Concepts explain why the engine looks the way it does. Guides walk a first-time user through a happy path. Cookbook pages solve specific tasks. Reference pages are narrower and drier on purpose. They are here so you can confirm behavior quickly while you are writing code, editing a command, or debugging a load failure.

## Python API

The Python reference is organized around the public object surfaces and the module-level entry points exposed by `lynxes`.

- [Python reference index](python/README.md)
- [`GraphFrame`](python/graphframe.md)
- [`LazyGraphFrame`](python/lazygraphframe.md)
- [`NodeFrame`](python/nodeframe.md)
- [`EdgeFrame`](python/edgeframe.md)
- [`Expr`](python/expr.md)
- [`AggExpr`](python/aggexpr.md)
- [Module I/O functions](python/module-io.md)
- [Graph export methods](python/graph-export.md)
- [Python connectors](python/connectors.md)
- [Python error mapping](python/errors.md)

## CLI

The CLI reference is command-oriented. It is meant to tell you what each command accepts, which defaults are in play, and how rendering-related options interact.

- [CLI reference index](cli/README.md)
- [`lynxes`](cli/gf.md)
- [`lynxes inspect`](cli/gf-inspect.md)
- [`lynxes convert`](cli/gf-convert.md)
- [`lynxes query`](cli/gf-query.md)
- [CLI output views](cli/output-views.md)

## Formats

The format reference covers the on-disk shapes and reserved graph semantics Lynxes expects to find when it loads a graph.

- [Format reference index](formats/README.md)
- [Reserved graph columns](formats/reserved-columns.md)
- [`.gf` format](formats/gf.md)
- [`.gf` authoring guide](../gf_authoring_guide.md)
- [`.gfb` format](formats/gfb.md)
- [Parquet graph shape](formats/parquet-interop.md)

## What To Expect From A Reference Page

A reference page in this section should stay narrow. It should tell you the accepted parameters, the output shape, the current support status, and the important constraints. It does not need to retell the motivation for the feature, and it does not need to hold your hand through a first run.

If a page starts reading like a tutorial, it probably belongs under `docs/guides`. If it starts arguing for a design decision, it probably belongs under `docs/concepts`. The pages here should feel closer to a technical manual than to a walkthrough.
