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


class TestIORoundTrip:
    def test_write_gfb_then_read_gfb(self, graph, tmp_dir):
        path = str(tmp_dir / "test.gfb")
        graph.write_gfb(path)
        restored = graph.read_gfb(path) if hasattr(graph, "read_gfb") else None
        if restored is None:
            import lynxes as gf

            restored = gf.read_gfb(path)
        assert restored.node_count() == graph.node_count()
        assert restored.edge_count() == graph.edge_count()

    def test_write_gf_then_read_gf(self, graph, tmp_dir):
        import lynxes as gf

        path = str(tmp_dir / "test_rt.gf")
        graph.write_gf(path)
        restored = gf.read_gf(path)
        assert restored.node_count() == graph.node_count()
        assert restored.edge_count() == graph.edge_count()

    def test_write_parquet_then_read(self, graph, tmp_dir):
        import lynxes as gf

        nodes_path = str(tmp_dir / "nodes.parquet")
        edges_path = str(tmp_dir / "edges.parquet")
        graph.write_parquet_graph(nodes_path, edges_path)
        restored = gf.read_parquet_graph(nodes_path, edges_path)
        assert restored.node_count() == graph.node_count()
        assert restored.edge_count() == graph.edge_count()
