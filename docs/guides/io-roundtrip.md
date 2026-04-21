# I/O Round-Trip

This guide covers saving a graph and reading it back through the formats Lynxes supports today.

Source example:

- [examples/python/recipes/io_roundtrip.py](../../examples/python/recipes/io_roundtrip.py)

## Python Round-Trip With `.gfb`

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
g.write_gfb("example.gfb")

restored = lx.read_gfb("example.gfb")
print(restored.node_count(), restored.edge_count())
```

## Python Round-Trip With `.gf`

```python
g.write_gf("example_roundtrip.gf")
restored = lx.read_gf("example_roundtrip.gf")
```

## Python Round-Trip With Parquet

```python
g.write_parquet_graph("nodes.parquet", "edges.parquet")
restored = lx.read_parquet_graph("nodes.parquet", "edges.parquet")
```

## CLI Conversion Flow

```bash
lynxes convert examples/data/example_simple.gf example.gfb --compression zstd
lynxes inspect example.gfb
lynxes convert example.gfb example_roundtrip.gf
```

## What to Validate

After any round-trip, check:

- node count
- edge count
- labels and edge types
- expected reserved columns

Counts are the fastest first check, but for important workflows inspect columns and representative rows too.
