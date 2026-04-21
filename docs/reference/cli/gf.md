# `lynxes`

`lynxes` is the top-level CLI entry point. It dispatches to a small command tree rather than doing useful work on its own, so the main purpose of this page is to describe the command layout and the behavior that is shared across subcommands.

## Synopsis

```bash
lynxes <COMMAND> [OPTIONS]
```

## Commands

| Command | Purpose |
| :--- | :--- |
| `inspect` | Print quick statistics for a `.gf` or `.gfb` graph file. |
| `convert` | Convert a graph file between supported formats. |
| `query` | Load a graph, optionally traverse it, and render a chosen view. |

## Global Options

| Option | Required | Default | Description |
| :--- | :--- | :--- | :--- |
| `-h`, `--help` | No | - | Print help text. |
| `-V`, `--version` | No | - | Print the CLI version. |

## Exit Behavior

Successful commands exit with status code `0`. Failures exit non-zero and print a human-readable error to standard error.

## Input Support

At the top level, the CLI surface currently works with:

- `.gf`
- `.gfb`
- graph-shaped parquet input where the command explicitly supports it

The exact support matrix depends on the subcommand. `inspect` is narrower than `convert` and `query`, so use the subcommand pages when format support details matter.
