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


class TestCsvReader:
    def test_read_csv_builds_nodeframe_with_synthetic_ids(self, tmp_dir):
        import lynxes as gf

        path = tmp_dir / "movies.csv"
        path.write_text("title,year\nMoon,2009\nArrival,2016\n", encoding="utf-8")

        nodes = gf.read_csv(str(path), label="RawMovie", id_prefix="raw_movie")

        assert type(nodes).__name__ == "NodeFrame"
        assert nodes.ids() == ["raw_movie_0", "raw_movie_1"]
        assert nodes.column_values("_label") == [["RawMovie"], ["RawMovie"]]
        assert nodes.column_values("title") == ["Moon", "Arrival"]

    def test_nodeframe_read_csv_uses_id_col(self, tmp_dir):
        import lynxes as gf

        path = tmp_dir / "movies_with_id.csv"
        path.write_text("id,title\n10,Alien\n11,Aliens\n", encoding="utf-8")

        nodes = gf.NodeFrame.read_csv(str(path), label="RawMovie", id_col="id")

        assert nodes.ids() == ["10", "11"]
        assert nodes.column_values("id") == [10, 11]
        assert nodes.column_values("_label") == [["RawMovie"], ["RawMovie"]]

    def test_read_csv_uses_existing_label_column(self, tmp_dir):
        import lynxes as gf

        path = tmp_dir / "labeled.csv"
        path.write_text("_id,_label,title\nm1,Movie,Heat\nm2,Movie,Thief\n", encoding="utf-8")

        nodes = gf.read_csv(str(path))

        assert nodes.ids() == ["m1", "m2"]
        assert nodes.column_values("_label") == [["Movie"], ["Movie"]]
        assert nodes.column_values("title") == ["Heat", "Thief"]

    def test_read_csv_pyarrow_engine_matches_native(self, tmp_dir):
        import lynxes as gf

        path = tmp_dir / "engine_compare.csv"
        path.write_text("title,year\nMoon,2009\nArrival,2016\n", encoding="utf-8")

        native = gf.read_csv(str(path), label="RawMovie", id_prefix="raw_movie")
        pyarrow = gf.read_csv(
            str(path),
            label="RawMovie",
            id_prefix="raw_movie",
            engine="pyarrow",
        )

        assert pyarrow.ids() == native.ids()
        assert pyarrow.column_values("_label") == native.column_values("_label")
        assert pyarrow.column_values("title") == native.column_values("title")


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
