from pathlib import Path

import lynxes as lx


ROOT = Path(__file__).resolve().parents[3]
GRAPH_PATH = ROOT / "examples" / "data" / "example_complex.gf"


def main() -> None:
    # These examples assume a repository checkout. PyPI-first usage is covered in
    # docs/quickstart/python.md, where you provide your own graph path.
    graph = lx.read_gf(GRAPH_PATH)

    # Community detection is exposed as a graph algorithm, but the result is still
    # a columnar frame that you can inspect or export downstream.
    communities = graph.community_detection()

    print(f"graph: {GRAPH_PATH.name}")
    print(f"columns: {communities.column_names()}")
    print(f"rows: {communities.len()}")
    print(communities.to_pyarrow())


if __name__ == "__main__":
    main()
