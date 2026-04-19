"""
TST-009: Python binding integration tests.

Covers:
  - read_gf() loading
  - filter_nodes / collect_nodes via Expr (gf.col())
  - filter_nodes label-contains
  - expand (BFS 1-hop and 2-hop)
  - lazy chain (filter → expand → collect)
  - gf.col() arithmetic / comparison expressions
  - PyArrow round-trip (NodeFrame.to_pyarrow)
  - write_gfb / read_gfb round-trip
  - pagerank / shortest_path algorithms
  - GFError → Python exception mapping
"""

import pytest

import lynxes as gf

# ── Loading ───────────────────────────────────────────────────────────────────


class TestReadGf:
    def test_node_count(self, graph):
        assert graph.node_count() == 5

    def test_edge_count(self, graph):
        assert graph.edge_count() == 4

    def test_missing_file_raises_os_error(self):
        with pytest.raises(OSError):
            gf.read_gf("/nonexistent/path/that/does/not/exist.gf")

    def test_returns_graph_frame_type(self, graph):
        assert type(graph).__name__ == "GraphFrame"


# ── Filter / Collect ──────────────────────────────────────────────────────────


class TestFilterNodes:
    def test_age_gt_filter(self, graph):
        nf = graph.lazy().filter_nodes(gf.col("age") > 25).collect_nodes()
        # alice(30), charlie(35), diana(28), acme(100) → 4 nodes
        assert nf.len() == 4

    def test_age_eq_filter(self, graph):
        nf = graph.lazy().filter_nodes(gf.col("age") == 30).collect_nodes()
        assert nf.len() == 1

    def test_chained_filter(self, graph):
        # age > 25 AND age < 35  → alice(30), diana(28)
        nf = (
            graph.lazy()
            .filter_nodes(gf.col("age") > 25)
            .filter_nodes(gf.col("age") < 35)
            .collect_nodes()
        )
        assert nf.len() == 2

    def test_filter_yields_empty_when_no_match(self, graph):
        nf = graph.lazy().filter_nodes(gf.col("age") > 9999).collect_nodes()
        assert nf.is_empty()

    def test_label_contains_filter(self, graph):
        # Only Person nodes (4 of them)
        nf = graph.lazy().filter_nodes(gf.col("_label").contains("Person")).collect_nodes()
        assert nf.len() == 4

    def test_label_contains_company(self, graph):
        nf = graph.lazy().filter_nodes(gf.col("_label").contains("Company")).collect_nodes()
        assert nf.len() == 1


# ── Expand (BFS) ──────────────────────────────────────────────────────────────


class TestExpand:
    def test_one_hop_out_from_alice(self, graph):
        # alice → bob, diana  (+ alice herself = seed)
        result = (
            graph.lazy()
            .filter_nodes(gf.col("_id") == "alice")
            .expand(hops=1, direction="out")
            .collect()
        )
        assert result.node_count() >= 2  # alice + at least bob or diana

    def test_two_hops_reaches_charlie(self, graph):
        # alice →(1) bob →(2) charlie
        result = (
            graph.lazy()
            .filter_nodes(gf.col("_id") == "alice")
            .expand(hops=2, direction="out")
            .collect()
        )
        node_ids = {row for row in result.nodes().to_pyarrow()["_id"].to_pylist()}
        assert "charlie" in node_ids

    def test_expand_with_edge_type_filter(self, graph):
        # KNOWS edges only — diana→acme is WORKS_AT, so acme unreachable
        result = (
            graph.lazy()
            .filter_nodes(gf.col("_id") == "alice")
            .expand(edge_type="KNOWS", hops=2, direction="out")
            .collect()
        )
        node_ids = {row for row in result.nodes().to_pyarrow()["_id"].to_pylist()}
        assert "acme" not in node_ids

    def test_expand_in_direction(self, graph):
        # charlie has no out-edges but is target of bob→charlie
        # In-direction from charlie should reach bob (1 hop in)
        result = (
            graph.lazy()
            .filter_nodes(gf.col("_id") == "charlie")
            .expand(hops=1, direction="in")
            .collect()
        )
        node_ids = {row for row in result.nodes().to_pyarrow()["_id"].to_pylist()}
        assert "bob" in node_ids


# ── Expr system ───────────────────────────────────────────────────────────────


class TestExpr:
    def test_col_returns_expr(self):
        expr = gf.col("age")
        assert type(expr).__name__ == "Expr"

    def test_comparison_gt(self):
        expr = gf.col("age") > 25
        assert type(expr).__name__ == "Expr"

    def test_comparison_eq(self):
        expr = gf.col("_id") == "alice"
        assert type(expr).__name__ == "Expr"

    def test_comparison_lt(self):
        expr = gf.col("age") < 30
        assert type(expr).__name__ == "Expr"


# ── PyArrow round-trip ────────────────────────────────────────────────────────


class TestPyArrow:
    def test_nodes_to_pyarrow_returns_record_batch(self, graph):
        import pyarrow as pa

        table = graph.nodes().to_pyarrow()
        assert isinstance(table, pa.RecordBatch)

    def test_nodes_pyarrow_has_id_column(self, graph):
        table = graph.nodes().to_pyarrow()
        assert "_id" in table.schema.names

    def test_nodes_pyarrow_row_count(self, graph):
        table = graph.nodes().to_pyarrow()
        assert table.num_rows == 5

    def test_edges_to_pyarrow_has_src_dst(self, graph):
        table = graph.edges().to_pyarrow()
        names = table.schema.names
        assert "_src" in names
        assert "_dst" in names


# ── I/O round-trips ───────────────────────────────────────────────────────────


class TestIORoundTrip:
    def test_write_gfb_then_read_gfb(self, graph, tmp_dir):
        path = str(tmp_dir / "test.gfb")
        graph.write_gfb(path)
        restored = gf.read_gfb(path)
        assert restored.node_count() == graph.node_count()
        assert restored.edge_count() == graph.edge_count()

    def test_write_gf_then_read_gf(self, graph, tmp_dir):
        path = str(tmp_dir / "test_rt.gf")
        graph.write_gf(path)
        restored = gf.read_gf(path)
        assert restored.node_count() == graph.node_count()
        assert restored.edge_count() == graph.edge_count()

    def test_write_parquet_then_read(self, graph, tmp_dir):
        nodes_path = str(tmp_dir / "nodes.parquet")
        edges_path = str(tmp_dir / "edges.parquet")
        graph.write_parquet_graph(nodes_path, edges_path)
        restored = gf.read_parquet_graph(nodes_path, edges_path)
        assert restored.node_count() == graph.node_count()
        assert restored.edge_count() == graph.edge_count()


# ── Algorithms ────────────────────────────────────────────────────────────────


class TestAlgorithms:
    def test_pagerank_returns_node_frame(self, graph):
        result = graph.pagerank()
        assert type(result).__name__ == "NodeFrame"

    def test_pagerank_has_pagerank_column(self, graph):
        result = graph.pagerank()
        assert "pagerank" in result.column_names()

    def test_pagerank_count_matches_graph(self, graph):
        result = graph.pagerank()
        assert result.len() == graph.node_count()

    def test_shortest_path_alice_to_charlie(self, graph):
        path = graph.shortest_path("alice", "charlie")
        assert isinstance(path, list)
        assert path[0] == "alice"
        assert path[-1] == "charlie"
        # alice→bob→charlie = 3 nodes
        assert len(path) == 3

    def test_shortest_path_to_self(self, graph):
        path = graph.shortest_path("alice", "alice")
        assert path == ["alice"]

    def test_connected_components_returns_node_frame(self, graph):
        result = graph.connected_components()
        assert type(result).__name__ == "NodeFrame"

    def test_connected_components_has_column(self, graph):
        result = graph.connected_components()
        assert "component_id" in result.column_names()


class TestDisplay:
    def test_graph_repr_contains_summary_and_rows(self, graph):
        rendered = repr(graph)
        assert "GraphFrame(rows=" in rendered
        assert "src" in rendered
        assert "alice" in rendered

    def test_head_renders_requested_slice(self, graph):
        rendered = graph.head(2, attrs=["age"])
        assert "age" in rendered
        assert "alice" in rendered

    def test_info_mentions_graph_stats(self, graph):
        rendered = graph.info()
        assert "Graph info" in rendered
        assert "self loops" in rendered
        assert "Node attrs" in rendered

    def test_schema_mentions_reserved_columns(self, graph):
        rendered = graph.schema()
        assert "Schema (" in rendered
        assert "_id" in rendered
        assert "_src" in rendered

    def test_glimpse_and_describe_structure_render(self, graph):
        glimpse = graph.glimpse(2)
        describe = graph.describe("structure")
        assert "Glimpse" in glimpse
        assert "rows sampled" in glimpse
        assert "Structure" in describe
        assert "connected components" in describe

    def test_describe_attrs_renders_stats(self, graph):
        rendered = graph.describe("attrs")
        assert "Attributes" in rendered
        assert "distinct=" in rendered
        assert "node.age" in rendered


# ── Exception mapping ─────────────────────────────────────────────────────────


class TestExceptions:
    def test_read_gf_missing_file_raises_os_error(self):
        with pytest.raises(OSError):
            gf.read_gf("/this/does/not/exist.gf")

    def test_read_gfb_missing_file_raises_os_error(self):
        with pytest.raises(OSError):
            gf.read_gfb("/this/does/not/exist.gfb")

    def test_shortest_path_missing_node_raises(self, graph):
        with pytest.raises((KeyError, RuntimeError, ValueError)):
            graph.shortest_path("alice", "does_not_exist")


# ── GF-600: NodeFrame set operations ─────────────────────────────────────────


class TestNodeFrameSetOps:
    """GF-600: concat / intersect / difference on NodeFrame."""

    def test_concat_disjoint_frames(self, graph):
        # Split into Persons and non-Persons, then concat should rebuild full set.
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
        # Filter to persons only (alice, bob, charlie, diana)
        persons = graph.lazy().filter_nodes(gf.col("_label").contains("Person")).collect_nodes()
        intersection = all_nodes.intersect(persons)
        # Result must be ≤ persons
        assert intersection.len() == persons.len()

    def test_difference_self_is_empty(self, graph):
        nf = graph.nodes()
        result = nf.difference(nf)
        assert result.len() == 0

    def test_difference_removes_subset(self, graph):
        all_nodes = graph.nodes()
        persons = graph.lazy().filter_nodes(gf.col("_label").contains("Person")).collect_nodes()
        diff = all_nodes.difference(persons)
        # Only Company nodes remain
        assert diff.len() == all_nodes.len() - persons.len()


# ── GF-500: AggExpr.alias ────────────────────────────────────────────────────


class TestAggExprAlias:
    """GF-500: AggExpr.alias() overrides the output column name."""

    def test_count_alias_changes_column_name(self, graph):
        result = (
            graph.lazy()
            .aggregate_neighbors("KNOWS", gf.count().alias("friend_count"))
            .collect_nodes()
        )
        cols = result.column_names()
        assert "friend_count" in cols, f"expected 'friend_count' in {cols}"
        assert "count" not in cols, "bare 'count' should not appear when aliased"

    def test_alias_preserves_values(self, graph):
        # alice has 2 outgoing KNOWS edges; bob, charlie, diana, acme have 0
        result = (
            graph.lazy()
            .filter_nodes(gf.col("_id") == "alice")
            .aggregate_neighbors("KNOWS", gf.count().alias("n_friends"))
            .collect_nodes()
        )
        rb = result.to_pyarrow()
        col = rb.column("n_friends")
        assert col[0].as_py() == 2

    def test_sum_alias(self, graph):
        result = (
            graph.lazy()
            .filter_nodes(gf.col("_id") == "alice")
            .aggregate_neighbors("KNOWS", gf.count().alias("c"))
            .collect_nodes()
        )
        assert "c" in result.column_names()


# ── GF-200: match_pattern API ─────────────────────────────────────────────────


class TestMatchPattern:
    """GF-200: match_pattern() wires the LazyGraphFrame plan node correctly."""

    def test_match_pattern_returns_lazy(self, graph):
        lazy = graph.lazy().match_pattern(
            [
                gf.node("a", "Person"),
                gf.edge("KNOWS"),
                gf.node("b", "Person"),
            ]
        )
        assert type(lazy).__name__ == "LazyGraphFrame"

    def test_match_pattern_explain_contains_pattern_match(self, graph):
        lazy = graph.lazy().match_pattern(
            [
                gf.node("a"),
                gf.edge(),
                gf.node("b"),
            ]
        )
        plan = lazy.explain()
        assert "PatternMatch" in plan

    def test_match_pattern_collect_raises_not_implemented(self, graph):
        """PatternMatch executor is not yet implemented — collect() must raise."""
        lazy = graph.lazy().match_pattern(
            [
                gf.node("a", "Person"),
                gf.edge("KNOWS"),
                gf.node("b", "Person"),
            ]
        )
        with pytest.raises((NotImplementedError, RuntimeError)):
            lazy.collect()

    def test_match_pattern_invalid_steps_raises(self, graph):
        """Even-length or <3-item lists must be rejected immediately."""
        with pytest.raises((TypeError, ValueError)):
            graph.lazy().match_pattern([gf.node("a"), gf.node("b")])

    def test_match_pattern_with_where_clause(self, graph):
        lazy = graph.lazy().match_pattern(
            [
                gf.node("a", "Person"),
                gf.edge("KNOWS"),
                gf.node("b", "Person"),
            ],
            where_=gf.col("a.age") > 25,
        )
        assert "PatternMatch" in lazy.explain()


# ── GF-str: StringExprNamespace ───────────────────────────────────────────────


class TestStrNamespace:
    """gf.col(x).str.contains/startswith/endswith filter tests."""

    def test_str_namespace_type(self):
        ns = gf.col("name").str
        assert type(ns).__name__ == "StringExprNamespace"

    def test_contains_returns_expr(self):
        expr = gf.col("_id").str.contains("ali")
        assert type(expr).__name__ == "Expr"

    def test_startswith_returns_expr(self):
        expr = gf.col("_id").str.startswith("al")
        assert type(expr).__name__ == "Expr"

    def test_endswith_returns_expr(self):
        expr = gf.col("_id").str.endswith("ce")
        assert type(expr).__name__ == "Expr"

    def test_contains_filters_nodes(self, graph):
        # "alice" and "charlie" both contain "li"
        nf = graph.lazy().filter_nodes(gf.col("_id").str.contains("li")).collect_nodes()
        ids = set(nf.to_pyarrow()["_id"].to_pylist())
        assert "alice" in ids
        assert "charlie" in ids
        assert "bob" not in ids

    def test_startswith_filters_nodes(self, graph):
        # "alice" starts with "al", "acme" starts with "ac" — only "alice" matches "al"
        nf = graph.lazy().filter_nodes(gf.col("_id").str.startswith("al")).collect_nodes()
        ids = set(nf.to_pyarrow()["_id"].to_pylist())
        assert "alice" in ids
        assert len(ids) == 1

    def test_endswith_filters_nodes(self, graph):
        # "alice" ends with "ice", no other node does
        nf = graph.lazy().filter_nodes(gf.col("_id").str.endswith("ice")).collect_nodes()
        ids = set(nf.to_pyarrow()["_id"].to_pylist())
        assert "alice" in ids
        assert len(ids) == 1


# ── GF-connectors: read_neo4j / read_arangodb / read_sparql ──────────────────


class TestConnectorAPI:
    """Verify connector read functions return a LazyGraphFrame with the right plan."""

    def test_read_neo4j_returns_lazy(self):
        lazy = gf.read_neo4j("bolt://localhost:7687", "neo4j", "password")
        assert type(lazy).__name__ == "LazyGraphFrame"

    def test_read_neo4j_explain_contains_scan(self):
        lazy = gf.read_neo4j("bolt://localhost:7687", "neo4j", "s3cr3t")
        plan = lazy.explain()
        assert "Scan" in plan

    def test_read_neo4j_with_database(self):
        lazy = gf.read_neo4j("bolt://localhost:7687", "neo4j", "pw", database="mydb")
        assert type(lazy).__name__ == "LazyGraphFrame"

    def test_read_arangodb_returns_lazy(self):
        lazy = gf.read_arangodb(
            endpoint="http://localhost:8529",
            database="mydb",
            graph="social",
            vertex_collection="persons",
            edge_collection="knows",
        )
        assert type(lazy).__name__ == "LazyGraphFrame"

    def test_read_arangodb_plan_contains_scan(self):
        lazy = gf.read_arangodb(
            endpoint="http://localhost:8529",
            database="mydb",
            graph="social",
            vertex_collection="persons",
            edge_collection="knows",
        )
        assert "Scan" in lazy.explain()

    def test_read_sparql_returns_lazy(self):
        lazy = gf.read_sparql(
            endpoint="https://dbpedia.org/sparql",
            node_template="SELECT ?id WHERE { ?id a <Thing> }",
            edge_template="SELECT ?s ?o WHERE { ?s ?p ?o }",
        )
        assert type(lazy).__name__ == "LazyGraphFrame"

    def test_read_sparql_with_expand_template(self):
        lazy = gf.read_sparql(
            endpoint="https://dbpedia.org/sparql",
            node_template="SELECT ?id WHERE { ?id a <Thing> }",
            edge_template="SELECT ?s ?o WHERE { ?s ?p ?o }",
            expand_template="SELECT ?s ?o WHERE { ?s ?p ?o FILTER(?s = $seed) }",
        )
        assert "Scan" in lazy.explain()


class TestPartitionedGraph:
    """graph.partition() / gf.partition_graph() — distributed processing tests."""

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
        shards = pg.shards()
        assert len(shards) == 2

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
        # alice's out-neighbors: bob, diana
        assert "bob" in ids or "diana" in ids

    def test_single_shard_partition(self, graph):
        pg = graph.partition(1)
        assert pg.n_shards == 1
        assert pg.boundary_edge_count == 0
        assert pg.shards()[0].node_count() == graph.node_count()
