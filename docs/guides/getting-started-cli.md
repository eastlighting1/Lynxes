# Getting Started On The CLI

This guide is the first CLI path for Lynxes.
It assumes you are working from a repository checkout, because today the CLI is documented from the perspective of the GitHub repository rather than as a separate standalone distribution.

The point of this guide is not to exhaust the CLI surface. It is to give you one clean path through inspection, querying, and conversion so that you can verify the tool behaves the way the rest of the docs claim it does.

## Before You Start

Make sure you are in the repository root.
The commands below assume the shared example file exists at:

`examples/data/example_simple.gf`

If that file is missing or you are in the wrong working directory, the rest of this guide will fail for reasons that have nothing to do with the CLI itself.

## Step 1: Inspect The Example Graph

Run:

```bash
cargo run -p lynxes-cli -- inspect examples/data/example_simple.gf
```

You should see a summary describing the file and the graph shape. The exact formatting may evolve, but the output should clearly indicate that the graph contains 3 nodes and 2 edges.

This is the fastest possible CLI sanity check. If this command fails, do not move on to querying or conversion yet.

## Step 2: Run A Simple Query

Now run:

```bash
cargo run -p lynxes-cli -- query examples/data/example_simple.gf --from alice --hops 2 --direction out
```

On the shared example graph, the result should represent a small outward neighborhood from `alice`. If you use the `--view info` form, the output should show a subgraph with 3 nodes and 2 edges.

That is the key learning moment for the CLI path. The command is not just scanning rows. It is materializing a graph result from a traversal.

## Step 3: Restrict By Edge Type

Add the edge-type constraint:

```bash
cargo run -p lynxes-cli -- query examples/data/example_simple.gf --from alice --hops 2 --edge-type KNOWS --direction out
```

This should still succeed on the shared example, and it makes the traversal intent more explicit. It is also a good reminder that the CLI is not a separate product with different semantics; it is another entrance to the same graph model.

## Step 4: Convert The Graph

Now try a format conversion:

```bash
cargo run -p lynxes-cli -- convert examples/data/example_simple.gf social.gfb --compression zstd
```

After that, inspect the converted file:

```bash
cargo run -p lynxes-cli -- inspect social.gfb
```

The converted file should describe the same logical graph shape as the original input. This is your first end-to-end CLI workflow: inspect, query, convert, inspect again.

## What To Confirm Before You Move On

At the end of this guide, you should be able to say all of the following are true:

- the CLI can inspect a `.gf` file from the repository examples
- a traversal query from `alice` succeeds
- the graph can be converted to `.gfb`
- the converted `.gfb` still inspects as the same graph

If any of those steps fail, that failure is already enough information to debug meaningfully. You do not need more complicated examples yet.

## Why This Guide Stays Narrow

The CLI has more than one command and more than one option surface, but the first guide should stay narrow on purpose. New users do not need five ways to run the same task. They need one path that works and clearly shows what the CLI is for.

Once that path is stable, it becomes much easier to read the more task-oriented CLI examples and references without guessing what "normal" output looks like.

## Where To Go Next

If the query step is what you want to understand more deeply, continue with [Your First Graph Query](first-graph-query.md).
If you want a broader overview of CLI flags and subcommands, continue with the CLI reference pages under `docs/reference/cli/`.
