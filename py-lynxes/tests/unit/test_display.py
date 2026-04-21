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
