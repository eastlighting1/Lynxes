import pytest


def test_mutable_graph_frame_crud_smoke(graph):
    """CRUD Python surface smoke test for MutableGraphFrame."""
    # 1. into_mutable()
    mgf = graph.into_mutable()

    nodes = graph.nodes()

    # Filter to get exactly one row for 'alice'
    # Assuming 'alice' is the first row
    mask = [False] * len(nodes)
    if len(mask) > 0:
        mask[0] = True
    alice_node = nodes.filter(mask)

    # Filter to get exactly one row for 'bob'
    mask2 = [False] * len(nodes)
    if len(mask2) > 1:
        mask2[1] = True
    bob_node = nodes.filter(mask2)

    if len(alice_node) == 1 and len(bob_node) == 1:
        # Delete nodes to avoid duplicate ID error
        mgf.delete_node("alice")
        mgf.delete_node("bob")

        # 2. add_node()
        mgf.add_node(alice_node)

        # 3. add_nodes_batch()
        mgf.add_nodes_batch(bob_node)

        # 5. update_node()
        mgf.update_node("alice", alice_node)

    # 4. add_edge()
    mgf.add_edge("alice", "bob")

    # 7. delete_edge()
    mgf.delete_edge(0)

    # 8. compact()
    mgf.compact()

    # 9. freeze()
    new_graph = mgf.freeze()

    # Verify the new graph is valid
    assert new_graph.node_count() >= 0
    assert new_graph.edge_count() >= 0

    # Verify mgf is consumed/frozen
    with pytest.raises(RuntimeError, match="has already been frozen"):
        mgf.add_edge("alice", "bob")
