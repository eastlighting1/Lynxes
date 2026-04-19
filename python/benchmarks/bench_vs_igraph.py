"""
TST-011: Graphframe vs igraph performance benchmark.

Compares Graphframe and igraph on three operations:
  1. 2-hop BFS expand from a single seed node
  2. PageRank (damping=0.85, 100 iterations)
  3. Weakly Connected Components

Graph sizes: 1 000, 10 000, 100 000 nodes (scale-free via Barabasi-Albert)

Usage:
    uv run python python/benchmarks/bench_vs_igraph.py

Results are printed to stdout in a markdown table and appended to
docs/benchmarks/bench_vs_igraph.md if that directory exists.
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
    import igraph as ig
except ImportError:
    print("igraph not installed — install with: uv sync --group benchmark")
    print("Skipping TST-011.")
    sys.exit(0)

try:
    import graphframe as gf
except ImportError:
    print("graphframe not installed — run: uv run maturin develop")
    sys.exit(1)

# ── graph generation helpers ──────────────────────────────────────────────────


def barabasi_albert_gf(n: int, m: int = 3) -> gf.GraphFrame:
    """Generate a Barabasi-Albert scale-free graph as a GraphFrame."""
    rng = random.Random(42)
    targets = list(range(m))
    degree = [0] * n
    edges_src, edges_dst = [], []

    for new_node in range(m, n):
        total_deg = max(sum(degree[:new_node]), 1)
        chosen = set()
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

    gf_text_lines = [f'(n{i}: Node)' for i in range(n)]
    gf_text_lines += [
        f'n{s} -[EDGE]-> n{d}'
        for s, d in zip(edges_src, edges_dst)
    ]
    gf_text = "\n".join(gf_text_lines) + "\n"

    with tempfile.NamedTemporaryFile(suffix=".gf", mode="w", delete=False, encoding="utf-8") as f:
        f.write(gf_text)
        path = f.name
    try:
        return gf.read_gf(path)
    finally:
        os.unlink(path)


def barabasi_albert_igraph(n: int, m: int = 3) -> ig.Graph:
    """Generate a Barabasi-Albert scale-free graph as an igraph Graph."""
    return ig.Graph.Barabasi(n=n, m=m, directed=True, power=1.0, start_from=ig.Graph(m))


# ── timing helpers ────────────────────────────────────────────────────────────

REPS = 3  # repetitions per benchmark cell


def time_fn(fn: Callable, reps: int = REPS) -> float:
    """Return median wall-clock time (seconds) over `reps` calls."""
    times = []
    for _ in range(reps):
        t0 = time.perf_counter()
        fn()
        times.append(time.perf_counter() - t0)
    return statistics.median(times)


# ── benchmarks ────────────────────────────────────────────────────────────────


def bench_expand_gf(graph: gf.GraphFrame) -> float:
    seed_id = "n0"
    return time_fn(
        lambda: (
            graph.lazy()
            .filter_nodes(gf.col("_id") == seed_id)
            .expand(hops=2, direction="out")
            .collect()
        )
    )


def bench_expand_igraph(g: ig.Graph) -> float:
    def _run():
        g.bfsiter(0, mode="out", advanced=False)
        visited = set()
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


def bench_pagerank_igraph(g: ig.Graph) -> float:
    return time_fn(lambda: g.pagerank(damping=0.85, implementation="prpack"))


def bench_cc_gf(graph: gf.GraphFrame) -> float:
    return time_fn(lambda: graph.connected_components())


def bench_cc_igraph(g: ig.Graph) -> float:
    return time_fn(lambda: g.clusters(mode="weak"))


# ── main ──────────────────────────────────────────────────────────────────────

SIZES = [1_000, 10_000, 100_000]


def fmt(seconds: float) -> str:
    if seconds < 1e-3:
        return f"{seconds * 1e6:.1f} µs"
    if seconds < 1.0:
        return f"{seconds * 1e3:.1f} ms"
    return f"{seconds:.2f} s"


def speedup(gf_t: float, ig_t: float) -> str:
    if gf_t == 0:
        return "∞×"
    ratio = ig_t / gf_t
    return f"{ratio:.1f}×" if ratio >= 1 else f"1/{1/ratio:.1f}×"


def main():
    rows = []
    header = ("N", "Op", "Graphframe", "igraph", "Speedup (GF/IG)")
    print()
    print(f"{'N':>8}  {'Operation':<30}  {'Graphframe':>12}  {'igraph':>12}  {'Speedup':>10}")
    print("-" * 82)

    for n in SIZES:
        print(f"  Building graphs n={n:,} ...", end="", flush=True)
        graph_gf = barabasi_albert_gf(n)
        graph_ig  = barabasi_albert_igraph(n)
        print(" done")

        ops = [
            ("2-hop BFS expand", bench_expand_gf,    bench_expand_igraph),
            ("PageRank",         bench_pagerank_gf,  bench_pagerank_igraph),
            ("Connected Comps.", bench_cc_gf,        bench_cc_igraph),
        ]

        for op_name, gf_fn, ig_fn in ops:
            t_gf = gf_fn(graph_gf)
            t_ig = ig_fn(graph_ig)
            sp   = speedup(t_gf, t_ig)
            print(f"{n:>8,}  {op_name:<30}  {fmt(t_gf):>12}  {fmt(t_ig):>12}  {sp:>10}")
            rows.append((n, op_name, t_gf, t_ig))

    # ── optional: write to docs/benchmarks/ ──────────────────────────────────
    out_dir = pathlib.Path(__file__).parents[2] / "docs" / "benchmarks"
    if out_dir.exists():
        out_path = out_dir / "bench_vs_igraph.md"
        lines = [
            "# Graphframe vs igraph Benchmark (TST-011)\n",
            f"| {'N':>8} | {'Operation':<30} | {'Graphframe':>12} | {'igraph':>12} | {'Speedup':>10} |\n",
            f"|{'-'*10}|{'-'*32}|{'-'*14}|{'-'*14}|{'-'*12}|\n",
        ]
        for n, op, t_gf, t_ig in rows:
            lines.append(
                f"| {n:>8,} | {op:<30} | {fmt(t_gf):>12} | {fmt(t_ig):>12} | {speedup(t_gf, t_ig):>10} |\n"
            )
        out_path.write_text("".join(lines))
        print(f"\nResults written to {out_path}")


if __name__ == "__main__":
    main()
