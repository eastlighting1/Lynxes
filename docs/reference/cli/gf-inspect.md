# `lynxes inspect`

`lynxes inspect` prints a quick summary for a graph file without going through the broader query-rendering surface.

## Synopsis

```bash
lynxes inspect <FILE>
```

## Arguments

| Name | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `FILE` | `path` | Required | Path to a `.gf` or `.gfb` graph file. |

## Options

| Option | Required | Default | Description |
| :--- | :--- | :--- | :--- |
| `-h`, `--help` | No | - | Print help text. |

## Accepted Formats

`lynxes inspect` currently accepts:

- `.gf`
- `.gfb`

It does not currently expose parquet inspection through this subcommand.

## Output

Depending on the format, inspection output can include:

- node count
- edge count
- label summary
- edge-type summary
- schema presence
- `.gfb` compression information

## Exit Behavior

Status code `0` means the file was parsed and summarized successfully. A non-zero exit means the file could not be read or was not valid for the selected inspection path.
