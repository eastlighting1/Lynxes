# Graphframe Documentation

Graphframe is a graph analytics engine built natively on Apache Arrow.
It stores node and edge data in Arrow `RecordBatch` objects, keeps graph structure as a first-class concern, and exposes both eager graph algorithms and a lazy query API.

This documentation is the user-facing entry point for the project.
It focuses on how to install Graphframe, load data, run queries, and use the CLI.

## Start Here

- [Install Graphframe](install.md)
- [Python Quickstart](quickstart/python.md)
- [CLI Quickstart](quickstart/cli.md)

## What Graphframe Gives You

- Arrow-native node and edge storage
- CSR-backed neighbor lookups for graph traversal
- Lazy graph queries built around `.collect()`
- Eager algorithms such as shortest path, PageRank, connected components, and community detection
- Native file workflows for `.gf`, `.gfb`, and graph-shaped parquet data

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

## Current Documentation Set

The first user documentation pass currently includes:

- installation instructions
- a Python quickstart
- a CLI quickstart

More guides and API reference pages can be added on top of this structure without changing the entry points above.

## Engineering Specs

These are design references for features that are being specified or refined.
They are not a promise that the behavior is already shipped exactly as written.

- [Polars-Style Repository Restructure and Lynxes Rename Specification](spec/polars_style_repo_restructure.md)
- [Terminal Projection and CLI Rendering Specification](spec/terminal_projection.md)

## Project Status

Graphframe is still an actively evolving project.
The user docs here describe the current shipped behavior of the repository, not future design goals.

If a feature is not documented here, treat it as unstable until it has a dedicated guide or reference page.
