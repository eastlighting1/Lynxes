from pathlib import Path

import lynxes as lx


ROOT = Path(__file__).resolve().parents[3]
GRAPH_PATH = ROOT / "examples" / "data" / "example_simple.gf"


def main() -> None:
    # These examples assume a repository checkout. PyPI-first usage is covered in
    # docs/quickstart/python.md, where you provide your own graph path.
    graph = lx.read_gf(GRAPH_PATH)

    # GraphFrame keeps graph structure and Arrow-backed columns together, so simple
    # inspection can show both graph metrics and tabular schema from one object.
    print(f"graph: {GRAPH_PATH.name}")
    print(f"nodes: {graph.node_count()}")
    print(f"edges: {graph.edge_count()}")
    print(f"density: {graph.density()}")
    print(f"node columns: {graph.nodes().column_names()}")
    print(f"edge columns: {graph.edges().column_names()}")
    print()
    print(graph.nodes())
    print()
    print(graph.edges())


if __name__ == "__main__":
    main()
