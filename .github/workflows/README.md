# CI/CD Workflows

## Overview

| Workflow | Trigger | Purpose |
|---|---|---|
| `ci.yml` | PR → main, push → main/dev | Rust lint + tests, Python tests |
| `release.yml` | push tag `v*` | Build 5-platform wheels → PyPI |
| `bench.yml` | push → main (engine files), manual | Criterion + Python benchmarks |

---

## `ci.yml` — Continuous Integration

Jobs run in parallel:

- **`rust-lint`** — `cargo fmt --check` + `cargo clippy -D warnings`
- **`rust-test`** — `cargo test --workspace --exclude lynxes-python`
- **`python-test`** — matrix [3.10, 3.11, 3.12, 3.13]: `maturin develop` → `pytest`
- **`python-lint`** — `ruff check` + `ruff format --check`
- **`ci-pass`** — single required status check for branch protection

---

## `release.yml` — Release & PyPI Publish

### How to release

```bash
# 1. Update version in Cargo.toml (workspace root)
#    The tag must match: v{major}.{minor}.{patch}
vim Cargo.toml

# 2. Commit and push
git add Cargo.toml
git commit -m "chore: bump version to 0.2.0"
git push

# 3. Tag and push tag — this triggers the workflow
git tag v0.2.0
git push origin v0.2.0
```

### Wheel build matrix

| Target | Runner | Notes |
|---|---|---|
| `x86_64-unknown-linux-gnu` | ubuntu-latest | manylinux_2_28 |
| `aarch64-unknown-linux-gnu` | ubuntu-latest | manylinux_2_28 via QEMU |
| `x86_64-apple-darwin` | macos-13 | Intel Mac |
| `aarch64-apple-darwin` | macos-14 | Apple Silicon |
| `x86_64-pc-windows-msvc` | windows-latest | |

Because `lynxes-python` uses `abi3-py310`, **one wheel per platform** covers all
Python ≥ 3.10. Total: 5 wheels + 1 sdist.

### PyPI Trusted Publishing setup (one-time)

1. Go to [pypi.org/manage/account/publishing](https://pypi.org/manage/account/publishing/)
2. Add a new **Trusted Publisher**:
   - **PyPI project name:** `lynxes`
   - **GitHub owner:** `<your-org>`
   - **Repository:** `<repo-name>`
   - **Workflow filename:** `release.yml`
   - **Environment name:** `pypi`
3. In the GitHub repo, create an environment named **`pypi`** under
   *Settings → Environments* with any required protection rules
   (e.g., only allow tags matching `v*`).

No API tokens needed — OIDC handles authentication automatically.

---

## `bench.yml` — Benchmarks

### Automatic (push to main, engine code only)
Runs Rust criterion benchmarks + Python benchmarks at sizes 1k and 10k.
Results are uploaded as artifacts (30-day retention).

### Manual
```
Actions → Benchmarks → Run workflow
  save-baseline: true   # optional, to save as new reference
```

The full 100k-node benchmark should be run locally:
```bash
cd py-lynxes
uv run python tests/benchmark/bench_vs_networkx.py --sizes 100000
```
