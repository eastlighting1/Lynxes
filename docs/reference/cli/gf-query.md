# `lynxes query`

`lynxes query` is the most flexible CLI command in the current surface. It can load a graph, optionally seed a traversal, render the result in several terminal views, and optionally write the resulting subgraph back out to disk.

## Synopsis

```bash
lynxes query [OPTIONS] <INPUT>
```

## Arguments

| Name | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `INPUT` | `path` | Required | Input graph file. The current query path accepts `.gf`, `.gfb`, or graph-shaped parquet input. |

## Options

| Option | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `--from <FROM>` | `string` | No | - | Seed traversal from one node id. Conflicts with `--from-label`. |
| `--from-label <FROM_LABEL>` | `string` | No | - | Seed traversal from all nodes with one label. Conflicts with `--from`. |
| `--hops <HOPS>` | `u32` | No | `1` | BFS hop count when a seed is present. |
| `--edge-type <EDGE_TYPE>` | `string` | No | - | Restrict traversal to one edge type. |
| `--direction <DIRECTION>` | `out \| in \| both \| undirected` | No | `out` | Traversal direction. |
| `--output <OUTPUT>` | `path` | No | - | Write the resulting subgraph to disk. |
| `--view <VIEW>` | `summary \| table \| head \| tail \| info \| describe \| schema \| glimpse` | No | `summary` | Terminal rendering mode. |
| `--rows <ROWS>` | `usize` | No | `10` | Row count for preview-oriented views. |
| `--sort-by <SORT_BY>` | `string` | No | - | Sort visible rows by a column before rendering. |
| `--expand-attrs` | `flag` | No | `false` | Promote heuristic attribute columns in rendering. |
| `--attrs <ATTRS>` | `comma-separated string` | No | - | Explicit attribute keys to promote. |
| `--width <WIDTH>` | `usize` | No | - | Override terminal width used by the renderer. |
| `--ascii` | `flag` | No | `false` | Use ASCII borders instead of Unicode. |
| `--describe-mode <DESCRIBE_MODE>` | `all \| structure \| types \| attrs` | No | `all` | Sub-mode used when `--view describe` is selected. |
| `-h`, `--help` | `flag` | No | - | Print help text. |

## Interaction Rules

`--from` and `--from-label` are mutually exclusive. If neither is provided, `lynxes query` loads the full graph and renders it without a seeded traversal. `--hops` only matters when one of the seed options is present. `--describe-mode` only affects output when `--view describe` is selected.

The direction vocabulary for the CLI is not identical to the Python binding. In the CLI, one accepted value is `undirected`; in the Python API the comparable direction spelling is different. When you switch between the two surfaces, check the accepted value list instead of assuming they are byte-for-byte identical.

## Output Behavior

Without `--output`, the command renders only to the terminal. With `--output`, the resulting subgraph is also written to disk. The output format is inferred from the destination extension, so the path you provide controls whether the result is written as `.gf`, `.gfb`, or parquet graph output.

## Notes

Most rendering-related options make sense only for specific views. `--rows` matters for preview-oriented views such as `table`, `head`, `tail`, and `glimpse`. `--describe-mode` is relevant only for `describe`. For the view vocabulary itself, continue with [CLI output views](output-views.md).
