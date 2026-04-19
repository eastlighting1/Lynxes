<h1 align="center">Lynxes</h1>

<p align="center">
  <strong>A Fast, Zero-Copy Graph Analytics Engine Built Natively on Apache Arrow.</strong>
</p>

<p align="center">
  <a href="https://pypi.org/project/lynxes/"><img src="https://img.shields.io/pypi/v/lynxes" alt="PyPI version"></a>
  <img src="https://img.shields.io/pypi/pyversions/lynxes" alt="Python versions">
  <img src="https://img.shields.io/badge/status-alpha-D97706" alt="alpha">
  <img src="https://img.shields.io/badge/engine-Rust-CE412B" alt="rust-engine">
</p>

<p align="center">
  <a href="#why-lynxes">Why Lynxes</a> |
  <a href="#quickstart">Quickstart</a> |
  <a href="#api-overview">API Overview</a> |
  <a href="#architecture">Architecture</a>
</p>

`Lynxes` is a blazingly fast, lazy-evaluated graph analytics engine. Unlike traditional Python libraries that wrap generic structures, **Lynxes builds a graph-native engine directly over Arrow**, completely bypassing the overhead of NetworkX or igraph.

## Why Lynxes

- **Zero-Copy Arrow Backing** — `NodeFrame` and `EdgeFrame` directly own Apache Arrow `RecordBatch`. No intermediate copies, no Pandas/Polars dependency.
- **Graph Structure as a First-Class Citizen** — `EdgeFrame` always maintains a Compressed Sparse Row (CSR) index. Neighbor lookups are O(degree) from day one — no full table scans.
- **Lazy by Default** — No computation happens until you call `.collect()`. The built-in optimizer runs Predicate Pushdown, Projection Pushdown, Traversal Pruning, and Subgraph Caching before execution.
- **Language-Agnostic Core** — The query engine, storage engine, and graph algorithms are written entirely in Rust. Python is a thin zero-overhead PyO3 wrapper.

## Quickstart

### Install

```bash
pip install lynxes
# or
uv add lynxes
```

### Build from source

```bash
git clone https://github.com/your-org/lynxes
cd lynxes/py-lynxes
uv run maturin develop --release
```

### Python API

```python
import lynxes as lx

# Load from .gf text, .gfb binary, or Parquet
g = lx.read_gf("graph.gf")
# g = lx.read_parquet_graph("nodes.parquet", "edges.parquet")
# g = lx.read_gfb("graph.gfb")

# Build a lazy plan — nothing executes yet
result = (
    g.lazy()
    .filter_nodes(lx.col("age") > 25)
    .expand("KNOWS", hops=2, direction="out")
    .aggregate_neighbors("KNOWS", lx.count().alias("friend_count"))
    .sort("friend_count", descending=True)
    .limit(10)
    .collect()
)

print(result)
```

### Pattern Matching

Cypher-like pattern matching over the lazy execution engine:

```python
result = (
    g.lazy()
    .match_pattern(
        [
            lx.node("person", "Person"),
            lx.edge("WORKS_AT"),
            lx.node("company", "Company"),
        ],
        where_=lx.col("person.age") > 25,
    )
    .collect()
)
```

### Graph Algorithms

```python
# PageRank
pr = g.pagerank()                          # → NodeFrame with 'pagerank' column

# Shortest path
path = g.shortest_path("alice", "charlie") # → ["alice", "bob", "charlie"]

# Connected components
cc = g.connected_components()              # → NodeFrame with 'component_id' column

# Betweenness centrality
bc = g.betweenness_centrality()

# Community detection (Louvain / Label Propagation)
cm = g.community_detection()
```

### Remote Connectors

```python
# Neo4j (Cypher)
g = lx.read_neo4j("bolt://localhost:7687", "neo4j", "password")

# ArangoDB (AQL)
g = lx.read_arangodb(
    endpoint="http://localhost:8529",
    database="mydb",
    graph="social",
    vertex_collection="persons",
    edge_collection="knows",
)

# SPARQL endpoint
g = lx.read_sparql(
    endpoint="https://dbpedia.org/sparql",
    node_template="SELECT ?id WHERE { ?id a <Thing> }",
    edge_template="SELECT ?s ?o WHERE { ?s ?p ?o }",
)
```

### Distributed Graph Partitioning

```python
# Partition a large graph across N shards
pg = g.partition(4, strategy="hash")   # or "range" / "label"
print(pg.n_shards)                     # 4
print(pg.stats())                      # imbalance ratio, boundary edges, …

# BFS across shard boundaries
nodes, edges = pg.distributed_expand(["alice"], hops=2, direction="out")

# Merge shards back into one GraphFrame
merged = pg.merge()
```

### CLI

```bash
# Inspect a .gfb file
lynxes inspect graph.gfb

# Convert formats
lynxes convert graph.gf graph.gfb

# Run a filter query
lynxes query graph.gfb --filter "age > 25" --limit 10
```

## API Overview

### Top-level functions

| Function | Description |
|---|---|
| `lx.read_gf(path)` | Load a `.gf` text graph |
| `lx.read_gfb(path)` | Load a `.gfb` binary graph |
| `lx.read_parquet_graph(nodes, edges)` | Load from Parquet files |
| `lx.read_neo4j(uri, user, password)` | Connect to Neo4j |
| `lx.read_arangodb(...)` | Connect to ArangoDB |
| `lx.read_sparql(endpoint, ...)` | Connect to SPARQL endpoint |
| `lx.col(name)` | Create a column expression |
| `lx.count()` / `lx.sum(e)` / `lx.mean(e)` | Aggregation expressions |
| `lx.node(alias, label?)` | Pattern node descriptor |
| `lx.edge(type?)` | Pattern edge descriptor |
| `lx.partition_graph(g, n)` | Partition a GraphFrame |

### `GraphFrame` methods

| Method | Returns |
|---|---|
| `.lazy()` | `LazyGraphFrame` |
| `.nodes()` / `.edges()` | `NodeFrame` / `EdgeFrame` |
| `.node_count()` / `.edge_count()` | `int` |
| `.subgraph(ids)` / `.subgraph_by_label(l)` | `GraphFrame` |
| `.pagerank(...)` | `NodeFrame` |
| `.shortest_path(src, dst)` | `list[str]` |
| `.connected_components()` | `NodeFrame` |
| `.betweenness_centrality()` | `NodeFrame` |
| `.community_detection()` | `NodeFrame` |
| `.partition(n, strategy)` | `PartitionedGraph` |
| `.write_gf(path)` / `.write_gfb(path)` | — |
| `.write_parquet_graph(nodes, edges)` | — |

### `LazyGraphFrame` methods

| Method | Description |
|---|---|
| `.filter_nodes(expr)` | Keep nodes matching expression |
| `.filter_edges(expr)` | Keep edges matching expression |
| `.select_nodes(cols)` / `.select_edges(cols)` | Project columns |
| `.expand(type?, hops, direction)` | BFS graph traversal |
| `.aggregate_neighbors(type, agg)` | Aggregate over neighbor edges |
| `.match_pattern(steps, where_?)` | Cypher-like pattern matching |
| `.sort(by, descending)` | Sort result |
| `.limit(n)` | Cap result size |
| `.explain()` | Print logical plan |
| `.collect()` | Execute → `GraphFrame` |
| `.collect_nodes()` | Execute → `NodeFrame` |
| `.collect_edges()` | Execute → `EdgeFrame` |

## Architecture

Lynxes is organized as a multi-crate Rust workspace with a thin Python layer on top:

```
py-lynxes/                ← Python package (maturin / PyO3)
  src/lynxes/             ← lynxes Python namespace
  tests/unit/             ← pytest integration tests
  tests/benchmark/        ← NetworkX / igraph comparisons

crates/
  lynxes/                 ← Umbrella re-export crate
  lynxes-core/            ← Arrow frames, CSR index, algorithms,
  │                           expression types, logical plan, optimizer
  lynxes-plan/            ← Logical plan re-exports (thin)
  lynxes-io/              ← File I/O (.gf parser, .gfb binary, Parquet)
  lynxes-connect/         ← Remote connectors (Neo4j, ArangoDB,
  │                           SPARQL, Arrow Flight, GFConnector)
  lynxes-lazy/            ← LazyGraphFrame + query executor
  lynxes-python/          ← PyO3 binding crate (_lynxes.so)
  lynxes-cli/             ← `lynxes` command-line tool
```

### Execution Pipeline

```
Python call
    │
    ▼
LazyGraphFrame (plan tree)
    │
    ▼
Optimizer ──── PredicatePushdown
            ── ProjectionPushdown
            ── TraversalPruning
            ── SubgraphCaching
            ── EarlyTermination
    │
    ▼
Executor ─────────────────────────────────────┐
    │                                         │
    ▼                                         ▼
NodeFrame / EdgeFrame                  CSR Index (O(degree))
(Arrow RecordBatch)                    BFS / Traversal / Algorithms
```

### Crate Dependency Graph

```
lynxes-python ──┐
lynxes-cli    ──┤
                ├──► lynxes-lazy ──► lynxes-connect ──┐
                │                                      ├──► lynxes-io ──┐
                │                                      └──► lynxes-plan ─┤
                │                                                        ├──► lynxes-core
                └───────────────────────────────────────────────────────►┘
```

## Documentation Map

- `DESIGN.md` — In-depth architectural design and engine principles
- `docs/spec/` — Feature and restructure specifications
- `py-lynxes/tests/benchmark/` — Performance benchmarks vs NetworkX / igraph

## Contributing

Please read `DESIGN.md` first. Core principles that are non-negotiable:

1. **Never wrap Polars** — `NodeFrame`/`EdgeFrame` own Arrow `RecordBatch` directly
2. **CSR is mandatory** — `EdgeFrame` always holds a CSR index; no linear scan fallbacks
3. **Lazy by default** — All operations build a `LogicalPlan`; execution only on `.collect()`
4. **No optimization without measurement** — Run `cargo bench` before claiming speedups
