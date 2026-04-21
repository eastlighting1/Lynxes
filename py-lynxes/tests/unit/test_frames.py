import lynxes as gf


class TestNodeFrameSetOps:
    def test_concat_disjoint_frames(self, graph):
        persons = graph.lazy().filter_nodes(gf.col("_label").contains("Person")).collect_nodes()
        companies = graph.lazy().filter_nodes(gf.col("_label").contains("Company")).collect_nodes()
        merged = gf.NodeFrame.concat([persons, companies])
        assert merged.len() == persons.len() + companies.len()

    def test_concat_single_frame_is_identity(self, graph):
        nf = graph.nodes()
        merged = gf.NodeFrame.concat([nf])
        assert merged.len() == nf.len()

    def test_intersect_self_is_identity(self, graph):
        nf = graph.nodes()
        result = nf.intersect(nf)
        assert result.len() == nf.len()

    def test_intersect_with_subset(self, graph):
        all_nodes = graph.nodes()
        persons = graph.lazy().filter_nodes(gf.col("_label").contains("Person")).collect_nodes()
        intersection = all_nodes.intersect(persons)
        assert intersection.len() == persons.len()

    def test_difference_self_is_empty(self, graph):
        nf = graph.nodes()
        result = nf.difference(nf)
        assert result.len() == 0

    def test_difference_removes_subset(self, graph):
        all_nodes = graph.nodes()
        persons = graph.lazy().filter_nodes(gf.col("_label").contains("Person")).collect_nodes()
        diff = all_nodes.difference(persons)
        assert diff.len() == all_nodes.len() - persons.len()


class TestPartitionedGraph:
    def test_partition_returns_partitioned_graph_type(self, graph):
        pg = graph.partition(2)
        assert type(pg).__name__ == "PartitionedGraph"

    def test_partition_graph_function_alias(self, graph):
        pg = gf.partition_graph(graph, 2)
        assert type(pg).__name__ == "PartitionedGraph"

    def test_n_shards_matches_requested(self, graph):
        pg = graph.partition(3)
        assert pg.n_shards == 3

    def test_shards_list_length(self, graph):
        pg = graph.partition(2)
        assert len(pg.shards()) == 2

    def test_total_nodes_preserved(self, graph):
        pg = graph.partition(2)
        total = sum(s.node_count() for s in pg.shards())
        assert total == graph.node_count()

    def test_total_intra_edges_plus_boundary_covers_all_edges(self, graph):
        pg = graph.partition(2)
        intra = sum(s.edge_count() for s in pg.shards())
        boundary = pg.boundary_edge_count
        assert intra + boundary == graph.edge_count()

    def test_merge_round_trips_node_count(self, graph):
        pg = graph.partition(2)
        merged = pg.merge()
        assert merged.node_count() == graph.node_count()

    def test_merge_round_trips_edge_count(self, graph):
        pg = graph.partition(2)
        merged = pg.merge()
        assert merged.edge_count() == graph.edge_count()

    def test_stats_returns_dict_with_expected_keys(self, graph):
        pg = graph.partition(2)
        s = pg.stats()
        assert "n_shards" in s
        assert "nodes_per_shard" in s
        assert "edges_per_shard" in s
        assert "boundary_edge_count" in s
        assert "imbalance_ratio" in s

    def test_stats_n_shards_matches(self, graph):
        pg = graph.partition(3)
        assert pg.stats()["n_shards"] == 3

    def test_shard_of_known_node(self, graph):
        pg = graph.partition(2)
        idx = pg.shard_of("alice")
        assert idx is not None
        assert 0 <= idx < 2

    def test_shard_of_unknown_node_returns_none(self, graph):
        pg = graph.partition(2)
        assert pg.shard_of("nobody_xyz") is None

    def test_range_strategy(self, graph):
        pg = graph.partition(2, strategy="range")
        assert pg.n_shards == 2
        total = sum(s.node_count() for s in pg.shards())
        assert total == graph.node_count()

    def test_label_strategy(self, graph):
        pg = graph.partition(2, strategy="label")
        assert pg.n_shards == 2

    def test_repr_contains_n_shards(self, graph):
        pg = graph.partition(2)
        assert "2" in repr(pg)

    def test_distributed_expand_returns_tuple(self, graph):
        pg = graph.partition(2)
        result = pg.distributed_expand(["alice"], hops=1)
        assert isinstance(result, tuple)
        assert len(result) == 2

    def test_distributed_expand_reaches_direct_neighbors(self, graph):
        pg = graph.partition(2)
        node_frame, _ = pg.distributed_expand(["alice"], hops=1, direction="out")
        ids = {row for row in node_frame.to_pyarrow()["_id"].to_pylist() if row is not None}
        assert "bob" in ids or "diana" in ids

    def test_single_shard_partition(self, graph):
        pg = graph.partition(1)
        assert pg.n_shards == 1
        assert pg.boundary_edge_count == 0
        assert pg.shards()[0].node_count() == graph.node_count()
