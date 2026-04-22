import lynxes as gf


def _person_node_frame(ids, ages, scores):
    return gf.NodeFrame.from_dict(
        {
            "_id": ids,
            "_label": [["Person"] for _ in ids],
            "age": ages,
            "score": scores,
        }
    )


class TestNodeFrameSetOps:
    def test_nodeframe_from_dict_creates_frame_without_pyarrow_import(self):
        frame = gf.NodeFrame.from_dict(
            {
                "_id": ["alice", "bob"],
                "_label": [["Person"], ["Person"]],
                "age": [31, 29],
            }
        )

        assert frame.len() == 2
        assert frame.column_names() == ["_id", "_label", "age"]

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

    def test_gather_rows_returns_requested_rows_in_order(self, graph):
        import pyarrow as pa

        nodes = graph.nodes()
        base = nodes.to_pyarrow()
        gathered = nodes.gather_rows([1, 0, 1])
        expected_ids = [base["_id"][i].as_py() for i in [1, 0, 1]]

        assert isinstance(gathered, pa.RecordBatch)
        assert gathered["_id"].to_pylist() == expected_ids


class TestGraphFrameGnnBridge:
    def test_graph_from_dicts_creates_graph_in_one_call(self):
        graph = gf.graph(
            nodes={
                "_id": ["alice", "bob", "carol"],
                "_label": [["Person"], ["Person"], ["Person"]],
                "age": [31, 29, 35],
            },
            edges={
                "_src": ["alice", "bob"],
                "_dst": ["bob", "carol"],
                "_type": ["KNOWS", "KNOWS"],
                "_direction": [1, 1],
            },
        )

        assert type(graph).__name__ == "GraphFrame"
        assert graph.node_count() == 3
        assert graph.edge_count() == 2

    def test_to_coo_returns_pyarrow_arrays_with_expected_coordinates(self, graph):
        import pyarrow as pa

        src, dst = graph.to_coo()

        assert isinstance(src, pa.Array)
        assert isinstance(dst, pa.Array)
        assert src.to_pylist() == [0, 0, 1, 3]
        assert dst.to_pylist() == [1, 3, 2, 4]
        assert len(src) == graph.edge_count()
        assert len(dst) == graph.edge_count()

    def test_sample_neighbors_returns_python_wrapper_with_expected_fields(self, graph):
        sampled = graph.sample_neighbors(seed_nodes=["alice"], hops=1, fan_out=[8])

        assert type(sampled).__name__ == "SampledSubgraph"
        assert sampled.node_indices == [0, 1, 3]
        assert sampled.node_row_ids == [0, 1, 3]
        assert sampled.edge_src == [0, 0]
        assert sampled.edge_dst == [1, 3]
        assert sampled.edge_row_ids == [0, 2]

    def test_sample_neighbors_supports_edge_type_direction_and_replace(self, graph):
        sampled = graph.sample_neighbors(
            seed_nodes=["bob"],
            hops=1,
            fan_out=[3],
            direction="out",
            edge_type="KNOWS",
            replace=True,
        )

        assert sampled.node_indices == [1, 2]
        assert sampled.node_row_ids == [1, 2]
        assert sampled.edge_src == [1, 1, 1]
        assert sampled.edge_dst == [2, 2, 2]
        assert sampled.edge_row_ids == [1, 1, 1]

    def test_random_walk_returns_length_bounded_paths(self, graph):
        walks = graph.random_walk(start_nodes=["alice"], length=2, walks_per_node=2)

        assert isinstance(walks, list)
        assert len(walks) == 2
        assert all(isinstance(walk, list) for walk in walks)
        assert all(len(walk) <= 3 for walk in walks)
        assert all(walk[0] == 0 for walk in walks if walk)

    def test_random_walk_supports_direction_and_edge_type(self, graph):
        walks = graph.random_walk(
            start_nodes=["charlie"],
            length=1,
            walks_per_node=2,
            direction="in",
            edge_type="KNOWS",
        )

        assert len(walks) == 2
        assert all(walk == [2, 1] for walk in walks)


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


class TestMutableGraphFrame:
    def test_into_mutable_add_edge_compact_and_freeze(self, graph):
        mutable = graph.into_mutable()

        assert type(mutable).__name__ == "MutableGraphFrame"

        mutable.add_edge("charlie", "alice")
        mutable.compact()
        frozen = mutable.freeze()

        assert type(frozen).__name__ == "GraphFrame"
        assert frozen.node_count() == graph.node_count()
        src, dst = frozen.to_coo()
        assert len(src) == frozen.edge_count()
        assert len(dst) == frozen.edge_count()

    def test_mutable_node_methods_smoke(self, graph):
        mutable = graph.into_mutable()

        mutable.add_node(_person_node_frame(["dora"], [31], [0.7]))
        mutable.add_nodes_batch(_person_node_frame(["erin", "frank"], [29, 33], [0.6, 0.5]))
        mutable.update_node("bob", _person_node_frame(["robert"], [41], [0.95]))
        mutable.delete_node("charlie")

        frozen = mutable.freeze()
        ids = set(frozen.nodes().to_pyarrow()["_id"].to_pylist())

        assert "dora" in ids
        assert "erin" in ids
        assert "frank" in ids
        assert "robert" in ids
        assert "bob" not in ids
        assert "charlie" not in ids

    def test_mutable_delete_edge_smoke(self, graph):
        mutable = graph.into_mutable()
        mutable.delete_edge(0)
        frozen = mutable.freeze()

        assert frozen.edge_count() == graph.edge_count() - 1

    def test_single_shard_partition(self, graph):
        pg = graph.partition(1)
        assert pg.n_shards == 1
        assert pg.boundary_edge_count == 0
        assert pg.shards()[0].node_count() == graph.node_count()
