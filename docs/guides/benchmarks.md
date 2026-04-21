# Benchmarks

Lynxes is opinionated about performance claims: optimization work should be backed by measurement.
This repository already includes both engine-level and Python-surface benchmark entry points.

## What Exists Today

- Rust Criterion benchmarks in `crates/lynxes-core/benches`
- Python comparison scripts in `py-lynxes/tests/benchmark`
- CI automation in `.github/workflows/bench.yml`

The current benchmark set is meant to answer slightly different questions:

- Rust Criterion benches focus on engine internals such as traversal shape, CSR behavior, and executor performance.
- Python benchmark scripts compare Lynxes against common ecosystem baselines such as NetworkX and igraph.

## Run Rust Benchmarks

From the repository root:

```bash
cargo bench --workspace --exclude lynxes-python
```

This runs the Criterion benchmark targets under `crates/lynxes-core/benches`.
Use these when you want to verify a performance claim about the Rust engine itself.

## Run Python Benchmarks

From `py-lynxes` after the extension is built:

```bash
uv sync --group dev --group benchmark
uv run maturin develop --release
uv run python tests/benchmark/bench_vs_networkx.py --sizes 1000 10000 --reps 3
uv run python tests/benchmark/bench_vs_igraph.py --sizes 1000 10000 --reps 3
```

These scripts are designed to compare exposed Python workflows rather than isolated Rust internals.

## CI Coverage

The benchmark workflow is defined in `.github/workflows/bench.yml`.
It currently:

- runs Rust Criterion benches
- runs Python comparison benchmarks for selected graph sizes
- uploads benchmark artifacts so results are inspectable outside a local machine

## How To Read Results

When you add or change an optimization, tie the claim to the benchmark that best matches the change:

- adjacency or traversal changes: use the Rust benches
- Python-facing algorithm throughput claims: use the Python comparison scripts
- broad project-level messaging: cite benchmark output, not intuition

If a public-facing performance claim does not point back to one of these measurements, treat it as provisional.
