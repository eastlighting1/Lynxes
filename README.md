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
  <a href="#architecture">Architecture</a>
</p>

`Lynxes` is a blazingly fast, lazy-evaluated graph analytics engine. Unlike traditional Python libraries that wrap generic structures, **Lynxes builds a graph-native engine directly over Arrow**, completely bypassing the overhead of traditional tools like NetworkX or igraph.

## Why Lynxes

- **Zero-Copy Arrow Backing**
  `NodeFrame` and `EdgeFrame` directly own Apache Arrow `RecordBatch`. 
- **Graph Structure as a First-Class Citizen**
  EdgeFrames maintain a Compressed Sparse Row (CSR) index internally. Neighbor lookups are $O(\text{degree})$ from day one—no full table scans.
- **Lazy by Default**
  Compute is delayed until you call `.collect()`. The built-in query optimizer pushes down predicates, prunes traversals, and caches subgraphs before execution.
- **Language Agnostic Core**
  The query engine, storage engine, and graph algorithms are written completely in Rust. Python is just a zero-overhead thin wrapper. 

## Quickstart

### Install

```bash
pip install lynxes
```

*Note: You can also use `uv add lynxes`.*

### Python API

The API is designed to feel highly familiar to modern DataFrame users, bringing robust lazy-evaluation semantics and native graph operations (like traversing and pattern matching) together.

```python
import lynxes as lx

# 1. Load data from native Arrow Parquet, Lynxes binary (.gfb), or Lynxes text (.gf)
# g = lx.read_gf("graph.gf")  # For human-readable text declarations
g = lx.read_parquet_graph("nodes.parquet", "edges.parquet")

# 2. Build a Lazy Plan
result = (
    g.lazy()
    # Apply standard column filters pushed down to scan
    .filter_nodes(lx.col("age") > 25)
    # Expand graph traversals natively
    .expand("KNOWS", hops=2, direction="out")
    # Native Neighbor Aggregation
    .aggregate_neighbors("KNOWS", lx.count().alias("friend_count"))
    .sort("friend_count", descending=True)
    .limit(10)
    .collect()
)

print(result)
```

### Pattern Matching

Lynxes also provides a Cypher-like pattern matching mechanism built over the lazy execution engine:

```python
result = (
    g.match(
        lx.node("person", label="Person"),
        lx.edge("WORKS_AT"),
        lx.node("company", label="Company"),
        lx.edge("LOCATED_IN"),
        lx.node("city", label="City"),
    )
    .where(lx.col("city.name") == "Seoul")
    .collect()
)
```

## Architecture

At its core, Lynxes relies on three main execution layers:

| Layer | Component | Description |
| --- | --- | --- |
| **Interface** | `lynxes-py` | PyO3 Python bindings. Thin wrapper delegating to Rust. |
| **Query Engine** | `LogicalPlan` | Constructs the query tree. Passes through the Optimizer (Predicate Pushdown, Early Termination). |
| **Storage Engine** | `NodeFrame` & `EdgeFrame` | Arrow RecordBatches combined with hash-based Node ID mapping and CSR (Compressed Sparse Row) indices for routing. |
| **Graph Engine** | Algorithm modules | Native BFS/DFS traversals, PageRank, Shortest Paths traversing the CSR structs without redundant copies. |

```text
LogicalPlan ──[Optimizer]──▶ PhysicalPlan ──▶ Executor ────┐
                                                           │
┌───────────────────────────┐  ┌───────────────────────────▼──┐
│      Storage Engine       │  │      Graph Engine (CSR)      │
│  Arrow RecordBatch        │  │  Traversal, Path Finding,    │
│  NodeIdIndex & CsrIndex   │  │  Centrality, Pattern Match   │
└───────────────────────────┘  └──────────────────────────────┘
```

## Documentation Map

- `docs/index.md` — Document hub
- `DESIGN.md` — In-depth architectural design docs and engine principles. 

## Contribution

We welcome contributions! Please verify `DESIGN.md` as our core philosophy requires strict adherence to Arrow-native memory layouts and absolute avoidance of Pandas/Polars dependencies within the core crate.
