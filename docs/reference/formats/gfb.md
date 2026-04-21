# `.gfb` Format

`.gfb` is the Lynxes-native binary graph format. It is meant for compact local storage, repeated reload, and tool-to-tool reuse inside the Lynxes ecosystem. It is not a hand-authored format.

## Purpose

Use `.gfb` when:

- you want a binary artifact that loads faster than an authored text format
- you want to keep a graph around for repeated local reuse
- you want CLI or Python export paths that preserve Lynxes-native graph semantics cleanly

Use `.gf` instead when a person is meant to read or edit the file directly.

## Creation Paths

Python:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
g.write_gfb("example.gfb")
```

CLI:

```bash
lynxes convert examples/data/example_simple.gf example.gfb --compression zstd
```

## Compression

The current CLI exposes these compression choices for `.gfb` output:

- `none`
- `zstd`
- `lz4`

Compression is chosen when writing. It is surfaced again by `lynxes inspect` when the file is inspected later.

## Inspection

Use:

```bash
lynxes inspect example.gfb
```

That is the quickest way to confirm counts, label summaries, edge-type summaries, and compression mode.

## Notes

`.gfb` is intended for operational reuse rather than manual editing. A normal round trip is to load a graph, write `.gfb`, load the `.gfb` later, and confirm that graph counts and semantics still match what you expect.
