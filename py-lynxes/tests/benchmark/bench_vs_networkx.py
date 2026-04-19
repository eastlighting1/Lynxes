"""
TST-012: Graphframe vs NetworkX performance benchmark.

Compares Graphframe (via Python API / PyO3) and NetworkX on:
  1. 2-hop BFS expand from a single seed node
  2. PageRank (damping=0.85, 100 iterations)

Also measures PyO3 serialisation overhead by running the expand with an
explicit graph-construction step separated from the query step.

Graph sizes: 1 000, 10 000, 100 000 nodes (scale-free via Barabasi-Albert)

Usage:
    uv run python python/benchmarks/bench_vs_networkx.py

Results are printed to stdout and optionally saved to
docs/benchmarks/bench_vs_networkx.md.
"""

import sys
import os
import time
import random
import statistics
import tempfile
import pathlib
from typing import Callable

# ── dependency check ──────────────────────────────────────────────────────────

try:
    import networkx as nx
except ImportError:
    print("networkx not installed — install with: uv sync --group benchmark")
    print("Skipping TST-012.")
    sys.exit(0)

try:
    import lynxes as gf
except ImportError:
    print("lynxes not installed — run: uv run maturin develop")
    sys.exit(1)

# ── graph generation helpers ──────────────────────────────────────────────────


def barabasi_albert_gf(n: int, m: int = 3) -> gf.GraphFrame:
    """Barabasi-Albert scale-free graph as a GraphFrame (same seed as TST-011)."""
    rng = random.Random(42)
    degree = [0] * n
    edges_src, edges_dst = [], []

    for new_node in range(m, n):
        total_deg = max(sum(degree[:new_node]), 1)
        chosen: set[int] = set()
        while len(chosen) < m:
            r = rng.random() * total_deg
            cum = 0.0
            for t in range(new_node):
                cum += max(degree[t], 1)
                if cum >= r:
                    chosen.add(t)
                    break
        for t in chosen:
            edges_src.append(str(new_node))
            edges_dst.append(str(t))
            degree[new_node] += 1
            degree[t] += 1

    lines = [f'(n{i}: Node)' for i in range(n)]
    lines += [f'n{s} -[EDGE]-> n{d}' for s, d in zip(edges_src, edges_dst)]
    content = "\n".join(lines) + "\n"

    with tempfile.NamedTemporaryFile(suffix=".gf", mode="w", delete=False, encoding="utf-8") as f:
        f.write(content)
        path = f.name
    try:
        return gf.read_gf(path)
    finally:
        os.unlink(path)


def barabasi_albert_nx(n: int, m: int = 3) -> nx.DiGraph:
    """Barabasi-Albert scale-free directed graph as a NetworkX DiGraph."""
    g_undirected = nx.barabasi_albert_graph(n, m, seed=42)
    # Convert to directed (both directions) to match Graphframe's Out semantics.
    return g_undirected.to_directed()


# ── timing ────────────────────────────────────────────────────────────────────

REPS = 3


def time_fn(fn: Callable, reps: int = REPS) -> float:
    times = []
    for _ in range(reps):
        t0 = time.perf_counter()
        fn()
        times.append(time.perf_counter() - t0)
    return statistics.median(times)


# ── benchmarks ────────────────────────────────────────────────────────────────


def bench_expand_gf(graph: gf.GraphFrame) -> float:
    return time_fn(
        lambda: (
            graph.lazy()
            .filter_nodes(gf.col("_id") == "n0")
            .expand(hops=2, direction="out")
            .collect()
        )
    )


def bench_expand_nx(g: nx.DiGraph) -> float:
    def _run():
        visited: set[int] = set()
        queue = [(0, 0)]
        while queue:
            v, depth = queue.pop(0)
            if v in visited or depth > 2:
                continue
            visited.add(v)
            if depth < 2:
                queue.extend((nb, depth + 1) for nb in g.successors(v))

    return time_fn(_run)


def bench_pagerank_gf(graph: gf.GraphFrame) -> float:
    return time_fn(lambda: graph.pagerank())


def bench_pagerank_nx(g: nx.DiGraph) -> float:
    return time_fn(lambda: nx.pagerank(g, alpha=0.85, max_iter=100))


def bench_pyo3_overhead(graph: gf.GraphFrame) -> dict[str, float]:
    """
    Measure PyO3 serialisation overhead by separating:
      - query only (collect() on a pre-built plan)
      - full Python call including Python-side plan construction
    """
    import pyarrow  # only for the conversion test

    # Time just the collect() → pyarrow conversion
    result = (
        graph.lazy()
        .filter_nodes(gf.col("_id") == "n0")
        .expand(hops=1, direction="out")
        .collect()
    )
    t_conversion = time_fn(lambda: result.nodes().to_pyarrow())
    t_full = bench_expand_gf(graph)
    return {"full_query_ms": t_full * 1e3, "pyarrow_conversion_ms": t_conversion * 1e3}


# ── main ──────────────────────────────────────────────────────────────────────

SIZES = [1_000, 10_000, 100_000]


def fmt(seconds: float) -> str:
    if seconds < 1e-3:
        return f"{seconds * 1e6:.1f} µs"
    if seconds < 1.0:
        return f"{seconds * 1e3:.1f} ms"
    return f"{seconds:.2f} s"


def speedup(gf_t: float, nx_t: float) -> str:
    if gf_t == 0:
        return "∞×"
    ratio = nx_t / gf_t
    return f"{ratio:.1f}×" if ratio >= 1 else f"1/{1/ratio:.1f}×"


def main():
    rows = []
    print()
    print(f"{'N':>8}  {'Operation':<28}  {'Graphframe':>12}  {'NetworkX':>12}  {'Speedup':>10}")
    print("-" * 80)

    for n in SIZES:
        print(f"  Building graphs n={n:,} ...", end="", flush=True)
        graph_gf = barabasi_albert_gf(n)
        graph_nx = barabasi_albert_nx(n)
        print(" done")

        ops = [
            ("2-hop BFS expand", bench_expand_gf,   bench_expand_nx),
            ("PageRank",         bench_pagerank_gf, bench_pagerank_nx),
        ]

        for op_name, gf_fn, nx_fn in ops:
            t_gf = gf_fn(graph_gf)
            t_nx = nx_fn(graph_nx)
            sp   = speedup(t_gf, t_nx)
            print(f"{n:>8,}  {op_name:<28}  {fmt(t_gf):>12}  {fmt(t_nx):>12}  {sp:>10}")
            rows.append((n, op_name, t_gf, t_nx))

    # ── PyO3 overhead measurement (on smallest graph only) ────────────────────
    print()
    print("PyO3 serialisation overhead (n=1,000):")
    overhead = bench_pyo3_overhead(barabasi_albert_gf(1_000))
    print(f"  Full query (filter→expand→collect): {overhead['full_query_ms']:.2f} ms")
    print(f"  PyArrow conversion (nodes.to_pyarrow): {overhead['pyarrow_conversion_ms']:.2f} ms")

    # ── optional: write to docs/benchmarks/ ──────────────────────────────────
    out_dir = pathlib.Path(__file__).parents[2] / "docs" / "benchmarks"
    if out_dir.exists():
        out_path = out_dir / "bench_vs_networkx.md"
        lines = [
            "# Graphframe vs NetworkX Benchmark (TST-012)\n",
            f"| {'N':>8} | {'Operation':<28} | {'Graphframe':>12} | {'NetworkX':>12} | {'Speedup':>10} |\n",
            f"|{'-'*10}|{'-'*30}|{'-'*14}|{'-'*14}|{'-'*12}|\n",
        ]
        for n, op, t_gf, t_nx in rows:
            lines.append(
                f"| {n:>8,} | {op:<28} | {fmt(t_gf):>12} | {fmt(t_nx):>12} | {speedup(t_gf, t_nx):>10} |\n"
            )
        lines += [
            "\n## PyO3 Overhead (n=1,000)\n",
            f"- Full query: {overhead['full_query_ms']:.2f} ms\n",
            f"- PyArrow conversion: {overhead['pyarrow_conversion_ms']:.2f} ms\n",
        ]
        out_path.write_text("".join(lines))
        print(f"\nResults written to {out_path}")


if __name__ == "__main__":
    main()
