# `lynxes convert`

`lynxes convert` converts a graph file between the formats the CLI currently supports.

## Synopsis

```bash
lynxes convert [OPTIONS] <INPUT> <OUTPUT>
```

## Arguments

| Name | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `INPUT` | `path` | Required | Input graph path. |
| `OUTPUT` | `path` | Required | Output graph path. |

## Options

| Option | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `--compression` | `none \| zstd \| lz4` | No | `none` | Compression codec for `.gfb` output. Ignored for other output formats. |
| `-h`, `--help` | flag | No | - | Print help text. |

## Notes

Parquet output uses a dual-file convention. If you ask for an output stem like `graph.parquet`, Lynxes expands that into two physical files for nodes and edges rather than one monolithic parquet file.

`--compression` only matters when the target format is `.gfb`. Supplying it for other output formats is harmless but has no effect.

## Exit Behavior

Status code `0` means the input was read and the output was written successfully. A non-zero exit means either the input format could not be read, the output format could not be written, or the requested conversion path was invalid.
