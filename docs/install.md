# Install Graphframe

This page covers the practical ways to install and verify Graphframe from the current repository state.

## Naming Note

This repository currently uses two names in different places:

- Python import path: `graphframe`
- Project metadata name in `pyproject.toml`: `lynxes`

Until naming is unified, follow the command examples exactly and import the module as:

```python
import graphframe as gf
```

## Requirements

For local development and source builds, you should have:

- Python 3.10 or newer
- Rust toolchain with Cargo
- a working C/C++ build environment for native Python extensions

## Install for Python Use

### Recommended: build from source with `maturin`

From the repository root:

```bash
python -m venv .venv
.venv\Scripts\activate
pip install maturin pyarrow pytest
maturin develop --release -m crates/graphframe-py/Cargo.toml
```

If you use `uv`, you can also install the development dependencies first:

```bash
uv sync --group dev
maturin develop --release -m crates/graphframe-py/Cargo.toml
```

This builds the native extension and makes `graphframe` importable in the active environment.

## Verify the Python Install

Run:

```bash
python -c "import graphframe as gf; print(gf.__version__)"
```

If the import succeeds and a version prints, the Python package is ready to use.

## Use the CLI from Source

If you only want to run the CLI during development, you do not need a separate install step.
You can invoke it directly through Cargo:

```bash
cargo run -p graphframe-cli -- --help
```

## Install the CLI as a Standalone Command

If you want a persistent `gf` command on your machine, install it from the workspace:

```bash
cargo install --path crates/graphframe-cli
```

Then verify it with:

```bash
gf --help
```

## Quick Verification Checklist

Use this checklist after installation:

1. `python -c "import graphframe as gf; print(gf.__version__)"`
2. `cargo run -p graphframe-cli -- --help`
3. If you installed the CLI globally: `gf --help`

## Troubleshooting

### Python import fails

Make sure you ran `maturin develop` inside the same virtual environment that you are using to run Python.

### Native build fails

Check that:

- Rust is installed and available on `PATH`
- your Python headers and native build tools are installed
- you are running the command from the repository root

### `gf` command is not found

Either:

- use `cargo run -p graphframe-cli -- ...`, or
- install the CLI with `cargo install --path crates/graphframe-cli`

## Next Step

After installation, continue with:

- [Python Quickstart](quickstart/python.md)
- [CLI Quickstart](quickstart/cli.md)
