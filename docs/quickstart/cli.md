# CLI Quickstart

This quickstart shows how to use the Graphframe CLI to:

1. inspect a graph file
2. run a simple traversal query
3. convert between formats

If you have not set up the CLI yet, see [Install Graphframe](../install.md).

## Running the CLI

You can run the CLI in either of these ways:

### From source

```bash
cargo run -p graphframe-cli -- --help
```

### As an installed command

```bash
gf --help
```

The examples below use `gf ...` for readability.
If you have not installed the command globally, replace `gf` with:

```bash
cargo run -p graphframe-cli -- 
```

## 1. Create a Small Graph File

Save the following as `social.gf`:

```gf
(alice :Person { age: 30 })
(bob :Person { age: 25 })
(charlie :Person { age: 35 })
(diana :Person { age: 28 })
(acme :Company {})

alice -[KNOWS]-> bob {}
alice -[KNOWS]-> diana {}
bob -[KNOWS]-> charlie {}
diana -[WORKS_AT]-> acme {}
```

## 2. Inspect the Graph

Use `gf inspect` to print a quick summary:

```bash
gf inspect social.gf
```

This is the fastest way to confirm that:

- the file parses
- node and edge counts look correct
- labels and edge types are what you expect

## 3. Run a Simple Query

Without a seed, `gf query` loads the graph and prints a summary:

```bash
gf query social.gf
```

With a seed node, it runs a traversal from that node:

```bash
gf query social.gf --from alice --hops 2 --direction out
```

You can restrict traversal to one edge type:

```bash
gf query social.gf --from alice --hops 2 --edge-type KNOWS --direction out
```

You can also seed the traversal from all nodes with a given label:

```bash
gf query social.gf --from-label Person --hops 1 --direction out
```

Supported direction values are:

- `out`
- `in`
- `both`
- `undirected`

## 4. Write the Query Result

If you want to persist the result subgraph, use `--output`:

```bash
gf query social.gf --from alice --hops 2 --direction out --output alice_2hop.gfb
```

The output format is inferred from the file extension.

## 5. Convert Between Formats

Convert `.gf` to `.gfb`:

```bash
gf convert social.gf social.gfb --compression zstd
```

Convert `.gfb` back to `.gf`:

```bash
gf convert social.gfb social_roundtrip.gf
```

Graphframe also supports graph-shaped parquet workflows through the CLI.
For parquet output, the CLI uses a two-file convention for nodes and edges.

## Typical CLI Workflow

A common shell-based flow looks like this:

1. `gf inspect social.gf`
2. `gf query social.gf --from alice --hops 2 --direction out`
3. `gf convert social.gf social.gfb --compression zstd`
4. `gf inspect social.gfb`

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
