# Lynxes Documentation

Lynxes is a graph analytics engine built natively on Apache Arrow.
It stores node and edge data in Arrow `RecordBatch` objects, keeps graph structure as a first-class concern, and exposes both eager graph algorithms and a lazy query API.

This documentation is the user-facing entry point for the project.
It focuses on how to install Lynxes, load data, run queries, and use the CLI from the distribution paths that are supported today.

## Start Here

- [Install Lynxes](install.md)
- [Python Quickstart](quickstart/python.md)
- [CLI Quickstart](quickstart/cli.md)
- [`.gf` Format Authoring Guide](gf_authoring_guide.md)
- [Guide Index](guides/index.md)
- [Cookbook Index](cookbook/index.md)

## Common Tasks

- [Verify a fresh Python install](guides/verify-your-install.md)
- [Get started in Python](guides/getting-started-python.md)
- [Get started on the CLI](guides/getting-started-cli.md)
- [Run your first graph query](guides/first-graph-query.md)
- [Run your first algorithm](guides/first-algorithm-run.md)
- [Preprocess a graph before training or export](guides/graph-preprocessing.md)
- [Move sampled structure and features into a GNN pipeline](guides/gnn-integration.md)
- [Extract KG-style bindings with `match_pattern()`](guides/kg-pattern-matching.md)
- [Load data from `.gf`, `.gfb`, or parquet](guides/loading-data.md)
- [Traverse a graph with `expand(...)`](guides/traversal-and-expand.md)
- [Debug loading and query failures](guides/errors-and-debugging.md)
- [Use connector entry points](guides/connectors.md)
- [Understand benchmark coverage and run it locally](guides/benchmarks.md)
- [Build an ego network around one seed node](cookbook/ego-network.md)
- [Find a shortest path between two nodes](cookbook/shortest-path.md)
- [Rank nodes with PageRank](cookbook/pagerank.md)
- [Detect communities and inspect the result](cookbook/community-detection.md)
- [Save a graph and validate a round-trip](cookbook/export-and-roundtrip.md)
- [Python module I/O reference](reference/python/module-io.md)
- [`.gf` format reference](reference/formats/gf.md)
- [`.gfb` format reference](reference/formats/gfb.md)
- [Parquet interop reference](reference/formats/parquet-interop.md)

## API Reference

- [Reference Index](reference/index.md)
- [`GraphFrame`](reference/python/graphframe.md)
- [`LazyGraphFrame`](reference/python/lazygraphframe.md)
- [`NodeFrame`](reference/python/nodeframe.md)
- [`EdgeFrame`](reference/python/edgeframe.md)
- [`MutableGraphFrame`](reference/python/mutablegraphframe.md)
- [`Expr`](reference/python/expr.md)
- [`AggExpr`](reference/python/aggexpr.md)
- [Python connectors](reference/python/connectors.md)
- [Module I/O functions](reference/python/module-io.md)
- [Graph export methods](reference/python/graph-export.md)
- [Python error mapping](reference/python/errors.md)
- [`lynxes` CLI](reference/cli/gf.md)
- [`lynxes inspect`](reference/cli/gf-inspect.md)
- [`lynxes convert`](reference/cli/gf-convert.md)
- [`lynxes query`](reference/cli/gf-query.md)
- [CLI output views](reference/cli/output-views.md)

## Concepts

- [Concept overview](concepts/index.md)
- [Why Lynxes exists](concepts/why-lynxes.md)
- [Memory layout and CSR](concepts/memory-layout-and-csr.md)
- [Lazy engine](concepts/lazy-engine.md)
- [Trade-offs](concepts/trade-offs.md)
- [Mutation and preprocessing](concepts/mutation-and-preprocessing.md)
- [GNN feature store](concepts/gnn-feature-store.md)

## Distribution Paths

Today there are two practical ways users encounter Lynxes:

- PyPI for the Python package
- the GitHub repository for source builds, development, and CLI use

The Python docs assume either a PyPI install or a local source build.
The CLI docs assume you are working from a GitHub repository checkout unless you have explicitly installed the CLI from that checkout with Cargo.

## What Lynxes Gives You

- Arrow-native node and edge storage
- CSR-backed neighbor lookups for graph traversal
- lazy graph queries built around `.collect()`
- a bounded mutation path for graph preprocessing before freezing back to an eager snapshot
- eager algorithms such as shortest path, PageRank, connected components, and community detection
- graph-to-GNN bridge helpers such as neighborhood sampling, random walks, COO export, and row gather
- native file workflows for `.gf`, `.gfb`, and graph-shaped parquet data
- benchmark entry points for both Rust internals and Python-surface comparisons

## Choose Your Entry Point

### Use Python if you want to

- load a graph and inspect it interactively
- build lazy graph queries
- run built-in algorithms from notebooks or scripts
- move results into PyArrow for downstream processing

### Use the CLI if you want to

- inspect a graph file quickly
- convert between supported file formats
- run a simple traversal from the terminal

The CLI is documented from the perspective of a GitHub repository checkout.
That keeps the guidance aligned with the current repo state and avoids implying a separate standalone CLI distribution path.

## Current Documentation Set

The first user documentation pass currently includes:

- installation instructions
- a Python quickstart
- a CLI quickstart
- `.gf` authoring guidance

More guides and API reference pages can be added on top of this structure without changing the entry points above.

## Project Status

Lynxes is still an actively evolving project.
The user docs here describe the current shipped behavior of the repository, not future design goals.

If a feature is not documented here, treat it as unstable until it has a dedicated guide or reference page.
