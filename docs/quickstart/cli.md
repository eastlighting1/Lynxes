# CLI Quickstart

This quickstart shows how to use the Lynxes CLI to:

1. inspect a graph file
2. run a simple traversal query
3. convert between formats

Source examples:

- [examples/cli/inspect.md](../../examples/cli/inspect.md)
- [examples/cli/query.md](../../examples/cli/query.md)
- [examples/cli/convert.md](../../examples/cli/convert.md)

If you have not set up the CLI yet, see [Install Lynxes](../install.md).

This page assumes a GitHub repository checkout.
That is the current source-of-truth path for CLI usage and examples.

## Running the CLI

You can run the CLI in either of these ways:

### From a GitHub checkout

```bash
cargo run -p lynxes-cli -- --help
```

### As a command installed from that checkout

```bash
lynxes --help
```

The examples below use `lynxes ...` for readability.
If you have not installed the command from the repository, replace `lynxes` with:

```bash
cargo run -p lynxes-cli -- 
```

## 1. Pick an Input Graph

Reuse the shared example file from the repository:

`examples/data/example_simple.gf`

## 2. Inspect the Graph

Use `lynxes inspect` to print a quick summary:

```bash
lynxes inspect examples/data/example_simple.gf
```

This is the fastest way to confirm that:

- the file parses
- node and edge counts look correct
- labels and edge types are what you expect

## 3. Run a Simple Query

Without a seed, `lynxes query` loads the graph and prints a summary:

```bash
lynxes query examples/data/example_simple.gf
```

With a seed node, it runs a traversal from that node:

```bash
lynxes query examples/data/example_simple.gf --from alice --hops 2 --direction out
```

You can restrict traversal to one edge type:

```bash
lynxes query examples/data/example_simple.gf --from alice --hops 2 --edge-type KNOWS --direction out
```

You can also seed the traversal from all nodes with a given label:

```bash
lynxes query examples/data/example_simple.gf --from-label Person --hops 1 --direction out
```

Supported direction values are:

- `out`
- `in`
- `both`
- `undirected`

## 4. Write the Query Result

If you want to persist the result subgraph, use `--output`:

```bash
lynxes query examples/data/example_simple.gf --from alice --hops 2 --direction out --output alice_2hop.gfb
```

The output format is inferred from the file extension.

## 5. Convert Between Formats

Convert `.gf` to `.gfb`:

```bash
lynxes convert examples/data/example_simple.gf social.gfb --compression zstd
```

Convert `.gfb` back to `.gf`:

```bash
lynxes convert social.gfb social_roundtrip.gf
```

Lynxes also supports graph-shaped parquet workflows through the CLI.
For parquet output, the CLI uses a two-file convention for nodes and edges.

## Typical CLI Workflow

A common shell-based flow looks like this:

1. `lynxes inspect examples/data/example_simple.gf`
2. `lynxes query examples/data/example_simple.gf --from alice --hops 2 --direction out`
3. `lynxes convert examples/data/example_simple.gf social.gfb --compression zstd`
4. `lynxes inspect social.gfb`

## When to Switch to Python

The CLI is best for:

- quick file inspection
- format conversion
- simple traversal-driven summaries

Switch to Python when you want:

- lazy query composition
- richer result inspection
- direct access to NodeFrame and EdgeFrame
- built-in algorithms from scripts or notebooks

For that workflow, continue with the [Python Quickstart](python.md).
