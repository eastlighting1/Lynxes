# Install Lynxes

This page covers the practical ways to install and verify Lynxes from the current repository state.

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

## Use the CLI from Source

If you only want to run the CLI during development, you do not need a separate install step.
You can invoke it directly through Cargo:

```bash
cargo run -p lynxes-cli -- --help
```

## Install the CLI as a Standalone Command

If you want a persistent `lynxes` command on your machine, install it from the workspace:

```bash
cargo install --path crates/lynxes-cli
```

Then verify it with:

```bash
lynxes --help
```

## Quick Verification Checklist

Use this checklist after installation:

1. `python -c "import lynxes as lx; print(lx.__version__)"`
2. `cargo run -p lynxes-cli -- --help`
3. If you installed the CLI globally: `lynxes --help`

## Troubleshooting

### Python import fails

Make sure you ran `maturin develop` inside the same virtual environment that you are using to run Python.

### Native build fails

Check that:

- Rust is installed and available on `PATH`
- your Python headers and native build tools are installed
- you are running the command from the repository root

### `lynxes` command is not found

Either:

- use `cargo run -p lynxes-cli -- ...`, or
- install the CLI with `cargo install --path crates/lynxes-cli`

## Next Step

After installation, continue with:

- [Python Quickstart](quickstart/python.md)
- [CLI Quickstart](quickstart/cli.md)
