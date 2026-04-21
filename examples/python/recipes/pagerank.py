from pathlib import Path

import lynxes as lx


ROOT = Path(__file__).resolve().parents[3]
GRAPH_PATH = ROOT / "examples" / "data" / "example_simple.gf"


def main() -> None:
    # These examples assume a repository checkout. PyPI-first usage is covered in
    # docs/quickstart/python.md, where you provide your own graph path.
    graph = lx.read_gf(GRAPH_PATH)

    # Eager algorithms execute immediately and return a frame-like result that can
    # be inspected with the same column-oriented methods as other Lynxes outputs.
    ranks = graph.pagerank()

    print(f"graph: {GRAPH_PATH.name}")
    print(f"columns: {ranks.column_names()}")
    print(f"rows: {ranks.len()}")
    print(ranks.to_pyarrow())


if __name__ == "__main__":
    main()
