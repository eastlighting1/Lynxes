# Install Lynxes

This page covers the practical ways to install and verify Lynxes from the current repository state.

## Supported Install Paths

Lynxes currently has two user-facing distribution paths:

- PyPI for the Python package
- the GitHub repository for source builds and CLI usage

If your goal is to use Lynxes from Python, start with PyPI.
If your goal is to use the CLI, start from a GitHub repository checkout.

## Requirements

For source builds and CLI work, you should have:

- Python 3.10 or newer
- Rust toolchain with Cargo
- a working C/C++ build environment for native Python extensions

For a PyPI-only Python install, you only need a supported Python environment.

## Install for Python Use

### Install from PyPI

```bash
pip install lynxes
```

Or with `uv`:

```bash
uv add lynxes
```

This is the simplest path if you only need the Python API.

### Build from source with `maturin`

Use this path when you are developing locally, testing unreleased changes, or working from a GitHub checkout.

From the repository root:

```bash
python -m venv .venv
.venv\Scripts\activate
pip install maturin pyarrow pytest
maturin develop --release -m crates/lynxes-python/Cargo.toml
```

If you use `uv`, you can also install the development dependencies first:

```bash
uv sync --group dev
maturin develop --release -m crates/lynxes-python/Cargo.toml
```

This builds the native extension and makes `lynxes` importable in the active environment.

## Verify the Python Install

Run:

```bash
python -c "import lynxes as lx; print(lx.__version__)"
```

If the import succeeds and a version prints, the Python package is ready to use.

## Use the CLI from a GitHub Checkout

The CLI docs assume you have cloned the GitHub repository.

Run it directly from the repo with:

```bash
cargo run -p lynxes-cli -- --help
```

This is the most direct way to use the CLI against the current repository state.

## Install the CLI from the Repository

If you want a persistent `lynxes` command on your machine, install it from the checked-out repository:

```bash
cargo install --path crates/lynxes-cli
```

Then verify it with:

```bash
lynxes --help
```

## Important CLI Note

This documentation does not assume a separate prebuilt CLI distribution.
For now, treat the CLI as a GitHub repository workflow:

- run it with `cargo run -p lynxes-cli -- ...`, or
- install it locally from the checked-out repository with `cargo install --path crates/lynxes-cli`

## Quick Verification Checklist

Use this checklist after installation:

1. `python -c "import lynxes as lx; print(lx.__version__)"`
2. If you are using the CLI from a repo checkout: `cargo run -p lynxes-cli -- --help`
3. If you installed the CLI from that checkout: `lynxes --help`

## Troubleshooting

### Python import fails

If you installed from source, make sure you ran `maturin develop` inside the same virtual environment that you are using to run Python.

### Native build fails

Check that:

- Rust is installed and available on `PATH`
- your Python headers and native build tools are installed
- you are running the command from the repository root

### `lynxes` command is not found

Either:

- use `cargo run -p lynxes-cli -- ...`, or
- install the CLI from the checked-out repository with `cargo install --path crates/lynxes-cli`

## Next Step

After installation, continue with:

- [Python Quickstart](quickstart/python.md)
- [CLI Quickstart](quickstart/cli.md)
