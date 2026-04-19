use std::{fs, path::PathBuf, sync::Arc};

use arrow::{
    array::{make_array, ArrayData, BooleanArray, StringArray},
    datatypes::DataType,
    pyarrow::{PyArrowType, ToPyArrow},
    record_batch::RecordBatch,
};
use graphframe_core::{
    parse_gf, read_gfb, read_parquet_graph, write_gf as core_write_gf, AggExpr, ArangoConfig,
    ArangoConnector, BetweennessConfig, BinaryOp, Connector, Direction, EdgeFrame, EdgeTypeSpec,
    Expr, GFError, GraphFrame, GraphPartitionMethod, GraphPartitioner, LazyGraphFrame, Neo4jConfig,
    Neo4jConnector, NodeFrame, PageRankConfig, Pattern, PatternStep, PartitionedGraph, ScalarValue,
    ShortestPathConfig, SparqlConfig, SparqlConnector, StringOp, UnaryOp, COL_EDGE_DST,
    COL_EDGE_SRC,
};
use pyo3::{
    basic::CompareOp,
    exceptions::{
        PyKeyError, PyNotImplementedError, PyOSError, PyRuntimeError, PyTypeError, PyValueError,
    },
    prelude::*,
    types::{PyAny, PyList, PyTuple, PyType},
    wrap_pyfunction,
};

#[pyclass(name = "NodeFrame", module = "graphframe")]
#[derive(Clone)]
struct PyNodeFrame {
    inner: Arc<NodeFrame>,
}

#[pyclass(name = "EdgeFrame", module = "graphframe")]
#[derive(Clone)]
struct PyEdgeFrame {
    inner: Arc<EdgeFrame>,
    node_ids: Arc<Vec<String>>,
}

#[pyclass(name = "GraphFrame", module = "graphframe")]
#[derive(Clone)]
struct PyGraphFrame {
    inner: Arc<GraphFrame>,
}

#[pyclass(name = "LazyGraphFrame", module = "graphframe")]
#[derive(Clone)]
struct PyLazyGraphFrame {
    inner: LazyGraphFrame,
}

#[pyclass(name = "Expr", module = "graphframe")]
#[derive(Clone)]
struct PyExpr {
    inner: Expr,
}

#[pyclass(name = "AggExpr", module = "graphframe")]
#[derive(Clone)]
struct PyAggExpr {
    inner: AggExpr,
}

#[pyclass(name = "PartitionedGraph", module = "graphframe")]
#[derive(Clone)]
struct PyPartitionedGraph {
    inner: PartitionedGraph,
}

impl PyPartitionedGraph {
    fn new(inner: PartitionedGraph) -> Self {
        Self { inner }
    }
}

/// Namespace returned by `expr.str` — provides string predicate builders.
#[pyclass(name = "StringExprNamespace", module = "graphframe")]
#[derive(Clone)]
struct PyStrExprNamespace {
    inner: Expr,
}

#[pyclass(name = "PatternNode", module = "graphframe")]
#[derive(Clone)]
struct PyPatternNode {
    alias: String,
    label: Option<String>,
    props: Vec<String>,
}

#[pyclass(name = "PatternEdge", module = "graphframe")]
#[derive(Clone)]
struct PyPatternEdge {
    edge_type: Option<String>,
    optional: bool,
    min_hops: u32,
    max_hops: Option<u32>,
}

impl PyNodeFrame {
    fn new(inner: NodeFrame) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    fn from_arc(inner: Arc<NodeFrame>) -> Self {
        Self { inner }
    }

    fn to_arrow_impl(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.inner
            .to_record_batch()
            .clone()
            .to_pyarrow(py)
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))
    }
}

impl PyEdgeFrame {
    fn new(inner: EdgeFrame) -> Self {
        let node_ids = Arc::new(build_edge_node_ids(&inner));
        Self {
            inner: Arc::new(inner),
            node_ids,
        }
    }

    fn from_arc(inner: Arc<EdgeFrame>) -> Self {
        let node_ids = Arc::new(build_edge_node_ids(inner.as_ref()));
        Self { inner, node_ids }
    }

    fn to_arrow_impl(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.inner
            .to_record_batch()
            .clone()
            .to_pyarrow(py)
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))
    }
}

impl PyGraphFrame {
    fn new(inner: GraphFrame) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl PyLazyGraphFrame {
    fn new(inner: LazyGraphFrame) -> Self {
        Self { inner }
    }
}

impl PyExpr {
    fn new(inner: Expr) -> Self {
        Self { inner }
    }

    fn binary(&self, other: &Bound<'_, PyAny>, op: BinaryOp) -> PyResult<Self> {
        Ok(Self::new(Expr::BinaryOp {
            left: Box::new(self.inner.clone()),
            op,
            right: Box::new(expr_from_py_operand(other)?),
        }))
    }
}

impl PyAggExpr {
    fn new(inner: AggExpr) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeFrame {
    #[classmethod]
    fn from_arrow(_cls: &Bound<'_, PyType>, batch: PyArrowType<RecordBatch>) -> PyResult<Self> {
        let frame = NodeFrame::from_record_batch(batch.0).map_err(gf_error_to_py_err)?;
        Ok(Self::new(frame))
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn column_names(&self) -> Vec<String> {
        self.inner
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    fn filter(&self, mask: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mask = extract_boolean_mask(mask)?;
        let frame = self.inner.filter(&mask).map_err(gf_error_to_py_err)?;
        Ok(Self::new(frame))
    }

    fn select(&self, columns: Vec<String>) -> PyResult<Self> {
        let columns_ref: Vec<&str> = columns.iter().map(String::as_str).collect();
        let frame = self
            .inner
            .select(&columns_ref)
            .map_err(gf_error_to_py_err)?;
        Ok(Self::new(frame))
    }

    /// Concatenate multiple `NodeFrame`s into one (union of rows, schemas must be compatible).
    #[classmethod]
    fn concat(_cls: &Bound<'_, PyType>, frames: Vec<PyRef<'_, PyNodeFrame>>) -> PyResult<Self> {
        let inner_refs: Vec<&NodeFrame> = frames.iter().map(|f| f.inner.as_ref()).collect();
        let merged = NodeFrame::concat(&inner_refs).map_err(gf_error_to_py_err)?;
        Ok(Self::new(merged))
    }

    /// Return rows whose `_id` appears in *both* `self` and `other`.
    fn intersect(&self, other: PyRef<'_, PyNodeFrame>) -> PyResult<Self> {
        let result = self.inner.intersect(&other.inner).map_err(gf_error_to_py_err)?;
        Ok(Self::new(result))
    }

    /// Return rows whose `_id` is in `self` but **not** in `other`.
    fn difference(&self, other: PyRef<'_, PyNodeFrame>) -> PyResult<Self> {
        let result = self.inner.difference(&other.inner).map_err(gf_error_to_py_err)?;
        Ok(Self::new(result))
    }

    fn to_arrow(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.to_arrow_impl(py)
    }

    fn to_pyarrow(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.to_arrow_impl(py)
    }
}

#[pymethods]
impl PyEdgeFrame {
    #[classmethod]
    fn from_arrow(_cls: &Bound<'_, PyType>, batch: PyArrowType<RecordBatch>) -> PyResult<Self> {
        let frame = EdgeFrame::from_record_batch(batch.0).map_err(gf_error_to_py_err)?;
        Ok(Self::new(frame))
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn column_names(&self) -> Vec<String> {
        self.inner
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    fn edge_types(&self) -> Vec<String> {
        self.inner
            .edge_types()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    fn filter(&self, mask: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mask = extract_boolean_mask(mask)?;
        let frame = self.inner.filter(&mask).map_err(gf_error_to_py_err)?;
        Ok(Self::new(frame))
    }

    fn filter_by_type(&self, edge_type: &str) -> PyResult<Self> {
        let frame = self
            .inner
            .filter_by_type(edge_type)
            .map_err(gf_error_to_py_err)?;
        Ok(Self::new(frame))
    }

    fn filter_by_types(&self, edge_types: Vec<String>) -> PyResult<Self> {
        let edge_types_ref: Vec<&str> = edge_types.iter().map(String::as_str).collect();
        let frame = self
            .inner
            .filter_by_types(&edge_types_ref)
            .map_err(gf_error_to_py_err)?;
        Ok(Self::new(frame))
    }

    fn select(&self, columns: Vec<String>) -> PyResult<Self> {
        let columns_ref: Vec<&str> = columns.iter().map(String::as_str).collect();
        let frame = self
            .inner
            .select(&columns_ref)
            .map_err(gf_error_to_py_err)?;
        Ok(Self::new(frame))
    }

    fn out_neighbors(&self, node_id: &str) -> PyResult<Vec<String>> {
        let Some(node_idx) = self.inner.node_row_idx(node_id) else {
            return Err(PyKeyError::new_err(format!("node not found: {node_id}")));
        };

        self.inner
            .out_neighbors(node_idx)
            .iter()
            .map(|&idx| {
                self.node_ids.get(idx as usize).cloned().ok_or_else(|| {
                    PyRuntimeError::new_err(format!("invalid local node index: {idx}"))
                })
            })
            .collect()
    }

    fn to_arrow(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.to_arrow_impl(py)
    }

    fn to_pyarrow(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.to_arrow_impl(py)
    }
}

#[pymethods]
impl PyGraphFrame {
    #[classmethod]
    fn from_frames(
        _cls: &Bound<'_, PyType>,
        nodes: PyRef<'_, PyNodeFrame>,
        edges: PyRef<'_, PyEdgeFrame>,
    ) -> PyResult<Self> {
        let graph = GraphFrame::new((*nodes.inner).clone(), (*edges.inner).clone())
            .map_err(gf_error_to_py_err)?;
        Ok(Self::new(graph))
    }

    fn nodes(&self) -> PyNodeFrame {
        PyNodeFrame::from_arc(Arc::new(self.inner.nodes().clone()))
    }

    fn edges(&self) -> PyEdgeFrame {
        PyEdgeFrame::from_arc(Arc::new(self.inner.edges().clone()))
    }

    fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    fn density(&self) -> f64 {
        self.inner.density()
    }

    fn lazy(&self) -> PyLazyGraphFrame {
        PyLazyGraphFrame::new(self.inner.lazy())
    }

    fn subgraph(&self, node_ids: Vec<String>) -> PyResult<Self> {
        let node_ids_ref: Vec<&str> = node_ids.iter().map(String::as_str).collect();
        let graph = self
            .inner
            .subgraph(&node_ids_ref)
            .map_err(gf_error_to_py_err)?;
        Ok(Self::new(graph))
    }

    fn subgraph_by_label(&self, label: &str) -> PyResult<Self> {
        let graph = self
            .inner
            .subgraph_by_label(label)
            .map_err(gf_error_to_py_err)?;
        Ok(Self::new(graph))
    }

    fn subgraph_by_edge_type(&self, edge_type: &str) -> PyResult<Self> {
        let graph = self
            .inner
            .subgraph_by_edge_type(edge_type)
            .map_err(gf_error_to_py_err)?;
        Ok(Self::new(graph))
    }

    fn k_hop_subgraph(&self, root: &str, k: usize) -> PyResult<Self> {
        let graph = self
            .inner
            .k_hop_subgraph(root, k)
            .map_err(gf_error_to_py_err)?;
        Ok(Self::new(graph))
    }

    fn out_neighbors(&self, node_id: &str) -> PyResult<Vec<String>> {
        self.inner
            .out_neighbors(node_id)
            .map(|values| values.into_iter().map(str::to_owned).collect())
            .map_err(gf_error_to_py_err)
    }

    fn in_neighbors(&self, node_id: &str) -> PyResult<Vec<String>> {
        self.inner
            .in_neighbors(node_id)
            .map(|values| values.into_iter().map(str::to_owned).collect())
            .map_err(gf_error_to_py_err)
    }

    #[pyo3(signature = (node_id, direction="out"))]
    fn neighbors(&self, node_id: &str, direction: &str) -> PyResult<Vec<String>> {
        let direction = python_to_direction(direction)?;
        self.inner
            .neighbors(node_id, direction)
            .map(|values| values.into_iter().map(str::to_owned).collect())
            .map_err(gf_error_to_py_err)
    }

    fn out_degree(&self, node_id: &str) -> PyResult<usize> {
        self.inner.out_degree(node_id).map_err(gf_error_to_py_err)
    }

    fn in_degree(&self, node_id: &str) -> PyResult<usize> {
        self.inner.in_degree(node_id).map_err(gf_error_to_py_err)
    }

    #[pyo3(signature = (*, damping=0.85, max_iter=100, epsilon=1e-6, weight_col=None))]
    fn pagerank(
        &self,
        damping: f64,
        max_iter: usize,
        epsilon: f64,
        weight_col: Option<String>,
    ) -> PyResult<PyNodeFrame> {
        let config = PageRankConfig {
            damping,
            max_iter,
            epsilon,
            weight_col,
        };
        let nodes = self.inner.pagerank(&config).map_err(gf_error_to_py_err)?;
        Ok(PyNodeFrame::new(nodes))
    }

    fn connected_components(&self) -> PyResult<PyNodeFrame> {
        let nodes = self
            .inner
            .connected_components()
            .map_err(gf_error_to_py_err)?;
        Ok(PyNodeFrame::new(nodes))
    }

    fn largest_connected_component(&self) -> PyResult<Self> {
        let graph = self
            .inner
            .largest_connected_component()
            .map_err(gf_error_to_py_err)?;
        Ok(Self::new(graph))
    }

    #[pyo3(signature = (src, dst, *, weight_col=None, edge_type=None, direction="out"))]
    fn shortest_path(
        &self,
        src: &str,
        dst: &str,
        weight_col: Option<String>,
        edge_type: Option<String>,
        direction: &str,
    ) -> PyResult<Option<Vec<String>>> {
        let config = ShortestPathConfig {
            weight_col,
            edge_type: shortest_path_edge_type(edge_type),
            direction: python_to_direction(direction)?,
        };
        self.inner
            .shortest_path(src, dst, &config)
            .map_err(gf_error_to_py_err)
    }

    #[pyo3(signature = (src, dst, *, weight_col=None, edge_type=None, direction="out"))]
    fn all_shortest_paths(
        &self,
        src: &str,
        dst: &str,
        weight_col: Option<String>,
        edge_type: Option<String>,
        direction: &str,
    ) -> PyResult<Vec<Vec<String>>> {
        let config = ShortestPathConfig {
            weight_col,
            edge_type: shortest_path_edge_type(edge_type),
            direction: python_to_direction(direction)?,
        };
        self.inner
            .all_shortest_paths(src, dst, &config)
            .map_err(gf_error_to_py_err)
    }

    #[pyo3(signature = (*, weight_col=None))]
    fn betweenness_centrality(&self, weight_col: Option<String>) -> PyResult<PyNodeFrame> {
        let nodes = self
            .inner
            .betweenness_centrality_with_config(&BetweennessConfig { weight_col })
            .map_err(gf_error_to_py_err)?;
        Ok(PyNodeFrame::new(nodes))
    }

    #[pyo3(signature = (direction="out"))]
    fn degree_centrality(&self, direction: &str) -> PyResult<PyNodeFrame> {
        let nodes = self
            .inner
            .degree_centrality(python_to_direction(direction)?)
            .map_err(gf_error_to_py_err)?;
        Ok(PyNodeFrame::new(nodes))
    }

    #[pyo3(signature = (*, algorithm="louvain", resolution=1.0, seed=None))]
    fn community_detection(
        &self,
        algorithm: &str,
        resolution: f64,
        seed: Option<u64>,
    ) -> PyResult<PyNodeFrame> {
        let algorithm = match algorithm {
            "louvain" => graphframe_core::CommunityAlgorithm::Louvain,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unsupported community_detection algorithm: {other}"
                )))
            }
        };

        let nodes = self
            .inner
            .community_detection(graphframe_core::CommunityConfig {
                algorithm,
                resolution,
                seed,
            })
            .map_err(gf_error_to_py_err)?;
        Ok(PyNodeFrame::new(nodes))
    }

    #[pyo3(signature = (src, dst, *, max_hops=None))]
    fn has_path(&self, src: &str, dst: &str, max_hops: Option<usize>) -> PyResult<bool> {
        self.inner
            .has_path(src, dst, max_hops)
            .map_err(gf_error_to_py_err)
    }

    fn write_gf(&self, path: &Bound<'_, PyAny>) -> PyResult<()> {
        let path = path_from_py_any(path)?;
        write_gf_impl(self.inner.as_ref(), &path)
    }

    fn write_gfb(&self, path: &Bound<'_, PyAny>) -> PyResult<()> {
        let path = path_from_py_any(path)?;
        self.inner.write_gfb(path).map_err(gf_error_to_py_err)
    }

    fn write_parquet_graph(
        &self,
        nodes_path: &Bound<'_, PyAny>,
        edges_path: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let nodes_path = path_from_py_any(nodes_path)?;
        let edges_path = path_from_py_any(edges_path)?;
        self.inner
            .write_parquet_graph(nodes_path, edges_path)
            .map_err(gf_error_to_py_err)
    }

    fn write_rdf(&self, path: &Bound<'_, PyAny>) -> PyResult<()> {
        let path = path_from_py_any(path)?;
        unsupported_write_impl("write_rdf", &path)
    }

    fn write_owl(&self, path: &Bound<'_, PyAny>) -> PyResult<()> {
        let path = path_from_py_any(path)?;
        unsupported_write_impl("write_owl", &path)
    }

    /// Partition this graph into `n_shards` balanced shards.
    ///
    /// ```python
    /// pg = graph.partition(4, strategy="hash")
    /// pg = graph.partition(4, strategy="range")
    /// pg = graph.partition(4, strategy="label")
    /// ```
    #[pyo3(signature = (n_shards, strategy = "hash"))]
    fn partition(&self, n_shards: usize, strategy: &str) -> PyResult<PyPartitionedGraph> {
        let method = match strategy {
            "range" => GraphPartitionMethod::Range,
            "label" => GraphPartitionMethod::Label,
            _ => GraphPartitionMethod::Hash,
        };
        let pg = GraphPartitioner::partition(self.inner.as_ref(), n_shards, method)
            .map_err(gf_error_to_py_err)?;
        Ok(PyPartitionedGraph::new(pg))
    }
}

#[pymethods]
impl PyLazyGraphFrame {
    fn filter_nodes(&self, expr: PyRef<'_, PyExpr>) -> Self {
        Self::new(self.inner.clone().filter_nodes(expr.inner.clone()))
    }

    fn filter_edges(&self, expr: PyRef<'_, PyExpr>) -> Self {
        Self::new(self.inner.clone().filter_edges(expr.inner.clone()))
    }

    fn select_nodes(&self, columns: Vec<String>) -> Self {
        Self::new(self.inner.clone().select_nodes(columns))
    }

    fn select_edges(&self, columns: Vec<String>) -> Self {
        Self::new(self.inner.clone().select_edges(columns))
    }

    #[pyo3(signature = (edge_type=None, *, hops=1, direction="out"))]
    fn expand(
        &self,
        edge_type: Option<&Bound<'_, PyAny>>,
        hops: u32,
        direction: &str,
    ) -> PyResult<Self> {
        if hops == 0 {
            return Err(PyValueError::new_err("hops must be greater than zero"));
        }
        let direction = python_to_direction(direction)?;
        let edge_type = normalize_edge_type_spec(edge_type)?;
        Ok(Self::new(
            self.inner.clone().expand(edge_type, hops, direction),
        ))
    }

    fn aggregate_neighbors(&self, edge_type: &str, agg: PyRef<'_, PyAggExpr>) -> Self {
        Self::new(
            self.inner
                .clone()
                .aggregate_neighbors(edge_type.to_owned(), agg.inner.clone()),
        )
    }

    /// Declare a structural pattern to match.
    ///
    /// `steps` must be a list that alternates `PatternNode` / `PatternEdge` / `PatternNode` …
    /// with at least three items (one edge hop minimum).
    ///
    /// ```python
    /// lazy.match_pattern([
    ///     gf.node("a", "Person"),
    ///     gf.edge("KNOWS"),
    ///     gf.node("b", "Person"),
    /// ])
    /// ```
    ///
    /// Note: the PatternMatch executor is not yet implemented; calling `.collect()` on
    /// the result will raise `NotImplementedError`.
    #[pyo3(signature = (steps, where_=None))]
    fn match_pattern(
        &self,
        steps: &Bound<'_, PyAny>,
        where_: Option<PyRef<'_, PyExpr>>,
    ) -> PyResult<Self> {
        let pattern = pattern_from_py_steps(steps)?;
        let where_expr = where_.map(|e| e.inner.clone());
        Ok(Self::new(self.inner.clone().match_pattern(pattern, where_expr)))
    }

    #[pyo3(signature = (by, descending=false))]
    fn sort(&self, by: &str, descending: bool) -> Self {
        Self::new(self.inner.clone().sort(by.to_owned(), descending))
    }

    fn limit(&self, n: usize) -> Self {
        Self::new(self.inner.clone().limit(n))
    }

    fn explain(&self) -> String {
        self.inner.explain()
    }

    fn collect(&self) -> PyResult<PyGraphFrame> {
        let graph = self.inner.clone().collect().map_err(gf_error_to_py_err)?;
        Ok(PyGraphFrame::new(graph))
    }

    fn collect_nodes(&self) -> PyResult<PyNodeFrame> {
        let nodes = self
            .inner
            .clone()
            .collect_nodes()
            .map_err(gf_error_to_py_err)?;
        Ok(PyNodeFrame::new(nodes))
    }

    fn collect_edges(&self) -> PyResult<PyEdgeFrame> {
        let edges = self
            .inner
            .clone()
            .collect_edges()
            .map_err(gf_error_to_py_err)?;
        Ok(PyEdgeFrame::new(edges))
    }
}

#[pymethods]
impl PyExpr {
    fn __repr__(&self) -> String {
        format!("Expr({:?})", self.inner)
    }

    fn __bool__(&self) -> PyResult<bool> {
        Err(PyTypeError::new_err(
            "symbolic expressions do not support truth-value testing; use &, |, and ~",
        ))
    }

    fn contains(&self, item: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(Expr::ListContains {
            expr: Box::new(self.inner.clone()),
            item: Box::new(expr_from_py_operand(item)?),
        }))
    }

    fn cast(&self, dtype: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(Expr::Cast {
            expr: Box::new(self.inner.clone()),
            dtype: extract_dtype(dtype)?,
        }))
    }

    fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> PyResult<Self> {
        let op = match op {
            CompareOp::Eq => BinaryOp::Eq,
            CompareOp::Ne => BinaryOp::NotEq,
            CompareOp::Gt => BinaryOp::Gt,
            CompareOp::Ge => BinaryOp::GtEq,
            CompareOp::Lt => BinaryOp::Lt,
            CompareOp::Le => BinaryOp::LtEq,
        };
        self.binary(other, op)
    }

    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        self.binary(other, BinaryOp::Add)
    }

    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        self.binary(other, BinaryOp::Sub)
    }

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        self.binary(other, BinaryOp::Mul)
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        self.binary(other, BinaryOp::Div)
    }

    fn __and__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(Expr::And {
            left: Box::new(self.inner.clone()),
            right: Box::new(expr_from_py_operand(other)?),
        }))
    }

    fn __or__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(Expr::Or {
            left: Box::new(self.inner.clone()),
            right: Box::new(expr_from_py_operand(other)?),
        }))
    }

    fn __invert__(&self) -> Self {
        Self::new(Expr::Not {
            expr: Box::new(self.inner.clone()),
        })
    }

    fn __neg__(&self) -> Self {
        Self::new(Expr::UnaryOp {
            op: UnaryOp::Neg,
            expr: Box::new(self.inner.clone()),
        })
    }

    /// Return a string-operation namespace for this expression.
    ///
    /// ```python
    /// gf.col("name").str.contains("Alice")
    /// gf.col("name").str.startswith("Al")
    /// gf.col("name").str.endswith("ce")
    /// ```
    #[getter]
    fn str(&self) -> PyStrExprNamespace {
        PyStrExprNamespace { inner: self.inner.clone() }
    }
}

#[pymethods]
impl PyStrExprNamespace {
    fn __repr__(&self) -> String {
        format!("StringExprNamespace({:?})", self.inner)
    }

    /// `expr.str.contains(pat)` — true when the column value contains the substring.
    fn contains(&self, pat: &str) -> PyExpr {
        PyExpr::new(Expr::StringOp {
            op: StringOp::Contains,
            expr: Box::new(self.inner.clone()),
            pattern: Box::new(Expr::Literal {
                value: graphframe_core::ScalarValue::String(pat.to_owned()),
            }),
        })
    }

    /// `expr.str.startswith(pat)` — true when the column value starts with the prefix.
    fn startswith(&self, pat: &str) -> PyExpr {
        PyExpr::new(Expr::StringOp {
            op: StringOp::StartsWith,
            expr: Box::new(self.inner.clone()),
            pattern: Box::new(Expr::Literal {
                value: graphframe_core::ScalarValue::String(pat.to_owned()),
            }),
        })
    }

    /// `expr.str.endswith(pat)` — true when the column value ends with the suffix.
    fn endswith(&self, pat: &str) -> PyExpr {
        PyExpr::new(Expr::StringOp {
            op: StringOp::EndsWith,
            expr: Box::new(self.inner.clone()),
            pattern: Box::new(Expr::Literal {
                value: graphframe_core::ScalarValue::String(pat.to_owned()),
            }),
        })
    }
}

#[pymethods]
impl PyPartitionedGraph {
    fn __repr__(&self) -> String {
        format!(
            "PartitionedGraph(n_shards={}, boundary_edges={})",
            self.inner.n_shards,
            self.inner.boundary_edges.len()
        )
    }

    /// Number of shards.
    #[getter]
    fn n_shards(&self) -> usize {
        self.inner.n_shards
    }

    /// Number of boundary edges (cross-shard).
    #[getter]
    fn boundary_edge_count(&self) -> usize {
        self.inner.boundary_edges.len()
    }

    /// List of shard GraphFrames.
    fn shards(&self) -> Vec<PyGraphFrame> {
        self.inner
            .shards
            .iter()
            .map(|s| PyGraphFrame::new(s.clone()))
            .collect()
    }

    /// Merge all shards back into a single GraphFrame.
    fn merge(&self) -> PyResult<PyGraphFrame> {
        let g = self.inner.merge().map_err(gf_error_to_py_err)?;
        Ok(PyGraphFrame::new(g))
    }

    /// Partition statistics as a plain Python dict.
    /// Keys: `n_shards`, `nodes_per_shard`, `edges_per_shard`,
    /// `boundary_edge_count`, `imbalance_ratio`.
    fn stats<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyDict> {
        let s = self.inner.stats();
        let d = pyo3::types::PyDict::new_bound(py);
        d.set_item("n_shards", s.n_shards).unwrap();
        d.set_item("nodes_per_shard", s.nodes_per_shard).unwrap();
        d.set_item("edges_per_shard", s.edges_per_shard).unwrap();
        d.set_item("boundary_edge_count", s.boundary_edge_count).unwrap();
        d.set_item("imbalance_ratio", s.imbalance_ratio).unwrap();
        d
    }

    /// Which shard owns `node_id`?  Returns `None` if not found.
    fn shard_of(&self, node_id: &str) -> Option<usize> {
        self.inner.shard_of(node_id)
    }

    /// Distributed BFS expand.
    ///
    /// ```python
    /// nodes, edges = pg.distributed_expand(["alice"], edge_type="KNOWS", hops=2, direction="out")
    /// ```
    #[pyo3(signature = (seed_ids, edge_type=None, hops=1, direction="out"))]
    fn distributed_expand(
        &self,
        seed_ids: Vec<String>,
        edge_type: Option<&str>,
        hops: u32,
        direction: &str,
    ) -> PyResult<(PyNodeFrame, PyEdgeFrame)> {
        let et = match edge_type {
            Some(edge_type) => EdgeTypeSpec::Single(edge_type.to_owned()),
            None => EdgeTypeSpec::Any,
        };
        let dir = python_to_direction(direction)?;
        let seed_refs: Vec<&str> = seed_ids.iter().map(String::as_str).collect();
        let (nf, ef) = self
            .inner
            .distributed_expand(&seed_refs, &et, hops, dir)
            .map_err(gf_error_to_py_err)?;
        let ef_node_ids: Vec<String> = nf.id_column().iter().flatten().map(str::to_owned).collect();
        Ok((
            PyNodeFrame::new(nf),
            PyEdgeFrame { inner: std::sync::Arc::new(ef), node_ids: std::sync::Arc::new(ef_node_ids) },
        ))
    }
}

#[pymethods]
impl PyAggExpr {
    fn __repr__(&self) -> String {
        format!("AggExpr({:?})", self.inner)
    }

    fn __bool__(&self) -> PyResult<bool> {
        Err(PyTypeError::new_err(
            "symbolic aggregations do not support truth-value testing",
        ))
    }

    /// Override the output column name produced by this aggregation.
    ///
    /// ```python
    /// gf.count().alias("follower_count")
    /// gf.sum(gf.col("weight")).alias("total_weight")
    /// ```
    fn alias(&self, name: &str) -> Self {
        Self::new(AggExpr::Alias {
            expr: Box::new(self.inner.clone()),
            name: name.to_owned(),
        })
    }
}

#[pymethods]
impl PyPatternNode {
    #[getter]
    fn alias(&self) -> String {
        self.alias.clone()
    }

    #[getter]
    fn label(&self) -> Option<String> {
        self.label.clone()
    }

    #[getter]
    fn props(&self) -> Vec<String> {
        self.props.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "PatternNode(alias={:?}, label={:?}, props={:?})",
            self.alias, self.label, self.props
        )
    }
}

#[pymethods]
impl PyPatternEdge {
    #[getter]
    fn edge_type(&self) -> Option<String> {
        self.edge_type.clone()
    }

    #[getter]
    fn optional(&self) -> bool {
        self.optional
    }

    #[getter]
    fn min_hops(&self) -> u32 {
        self.min_hops
    }

    #[getter]
    fn max_hops(&self) -> Option<u32> {
        self.max_hops
    }

    fn __repr__(&self) -> String {
        format!(
            "PatternEdge(edge_type={:?}, optional={}, min_hops={}, max_hops={:?})",
            self.edge_type, self.optional, self.min_hops, self.max_hops
        )
    }
}

#[pyfunction]
fn col(name: &str) -> PyExpr {
    PyExpr::new(expr_from_col_name(name))
}

#[pyfunction]
fn count() -> PyAggExpr {
    PyAggExpr::new(AggExpr::Count)
}

#[pyfunction]
fn sum(expr: &Bound<'_, PyAny>) -> PyResult<PyAggExpr> {
    Ok(PyAggExpr::new(AggExpr::Sum {
        expr: normalize_agg_expr_input(expr)?,
    }))
}

#[pyfunction]
fn mean(expr: &Bound<'_, PyAny>) -> PyResult<PyAggExpr> {
    Ok(PyAggExpr::new(AggExpr::Mean {
        expr: normalize_agg_expr_input(expr)?,
    }))
}

#[pyfunction]
fn list(expr: &Bound<'_, PyAny>) -> PyResult<PyAggExpr> {
    Ok(PyAggExpr::new(AggExpr::List {
        expr: normalize_agg_expr_input(expr)?,
    }))
}

#[pyfunction]
fn first(expr: &Bound<'_, PyAny>) -> PyResult<PyAggExpr> {
    Ok(PyAggExpr::new(AggExpr::First {
        expr: normalize_agg_expr_input(expr)?,
    }))
}

#[pyfunction]
fn last(expr: &Bound<'_, PyAny>) -> PyResult<PyAggExpr> {
    Ok(PyAggExpr::new(AggExpr::Last {
        expr: normalize_agg_expr_input(expr)?,
    }))
}

#[pyfunction]
#[pyo3(signature = (alias, label=None, props=None))]
fn node(alias: &str, label: Option<&str>, props: Option<Vec<String>>) -> PyPatternNode {
    PyPatternNode {
        alias: alias.to_owned(),
        label: label.map(str::to_owned),
        props: props.unwrap_or_default(),
    }
}

#[pyfunction]
#[pyo3(signature = (edge_type=None, optional=false, min_hops=1, max_hops=None))]
fn edge(
    edge_type: Option<&str>,
    optional: bool,
    min_hops: u32,
    max_hops: Option<u32>,
) -> PyResult<PyPatternEdge> {
    if min_hops == 0 {
        return Err(PyValueError::new_err("min_hops must be greater than zero"));
    }
    if let Some(max_hops) = max_hops {
        if max_hops < min_hops {
            return Err(PyValueError::new_err(
                "max_hops must be greater than or equal to min_hops",
            ));
        }
    }

    Ok(PyPatternEdge {
        edge_type: edge_type.map(str::to_owned),
        optional,
        min_hops,
        max_hops,
    })
}

#[pyfunction]
fn read_gf(path: &Bound<'_, PyAny>) -> PyResult<PyGraphFrame> {
    let path = path_from_py_any(path)?;
    let source = fs::read_to_string(&path)
        .map_err(GFError::IoError)
        .map_err(gf_error_to_py_err)?;
    let graph = parse_gf(&source)
        .and_then(|document| document.to_graph_frame())
        .map_err(gf_error_to_py_err)?;
    Ok(PyGraphFrame::new(graph))
}

#[pyfunction]
fn read_gfb_py(path: &Bound<'_, PyAny>) -> PyResult<PyGraphFrame> {
    let path = path_from_py_any(path)?;
    let graph = read_gfb(path).map_err(gf_error_to_py_err)?;
    Ok(PyGraphFrame::new(graph))
}

#[pyfunction]
fn read_parquet_graph_py(
    nodes_path: &Bound<'_, PyAny>,
    edges_path: &Bound<'_, PyAny>,
) -> PyResult<PyGraphFrame> {
    let nodes_path = path_from_py_any(nodes_path)?;
    let edges_path = path_from_py_any(edges_path)?;
    let graph = read_parquet_graph(nodes_path, edges_path).map_err(gf_error_to_py_err)?;
    Ok(PyGraphFrame::new(graph))
}

#[pyfunction]
fn write_gf(graph: PyRef<'_, PyGraphFrame>, path: &Bound<'_, PyAny>) -> PyResult<()> {
    let path = path_from_py_any(path)?;
    write_gf_impl(graph.inner.as_ref(), &path)
}

#[pyfunction]
fn write_gfb_py(graph: PyRef<'_, PyGraphFrame>, path: &Bound<'_, PyAny>) -> PyResult<()> {
    let path = path_from_py_any(path)?;
    graph.inner.write_gfb(path).map_err(gf_error_to_py_err)
}

#[pyfunction]
fn write_parquet_graph_py(
    graph: PyRef<'_, PyGraphFrame>,
    nodes_path: &Bound<'_, PyAny>,
    edges_path: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let nodes_path = path_from_py_any(nodes_path)?;
    let edges_path = path_from_py_any(edges_path)?;
    graph
        .inner
        .write_parquet_graph(nodes_path, edges_path)
        .map_err(gf_error_to_py_err)
}

/// Create a lazy graph frame backed by a Neo4j database.
///
/// The frame is not executed until `.collect()` is called.
/// Requires the `neo4j` feature (currently uses an unsupported-backend stub
/// that raises a runtime error on `.collect()` unless a real backend is linked).
///
/// ```python
/// lazy = gf.read_neo4j("bolt://localhost:7687", "neo4j", "password")
/// result = lazy.filter_nodes(gf.col("age") > 30).collect()
/// ```
#[pyfunction]
#[pyo3(signature = (uri, user, password, database=None))]
fn read_neo4j(
    uri: &str,
    user: &str,
    password: &str,
    database: Option<&str>,
) -> PyLazyGraphFrame {
    let config = Neo4jConfig {
        uri: uri.to_owned(),
        user: user.to_owned(),
        password: password.to_owned(),
        database: database.map(str::to_owned),
    };
    let connector: std::sync::Arc<dyn Connector> =
        std::sync::Arc::new(Neo4jConnector::new(config));
    PyLazyGraphFrame {
        inner: LazyGraphFrame::from_connector(connector),
    }
}

/// Create a lazy graph frame backed by an ArangoDB graph.
///
/// ```python
/// lazy = gf.read_arangodb(
///     endpoint="http://localhost:8529",
///     database="mydb",
///     graph="social",
///     vertex_collection="persons",
///     edge_collection="knows",
///     username="root",
///     password="secret",
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (endpoint, database, graph, vertex_collection, edge_collection, username="", password=""))]
fn read_arangodb(
    endpoint: &str,
    database: &str,
    graph: &str,
    vertex_collection: &str,
    edge_collection: &str,
    username: &str,
    password: &str,
) -> PyLazyGraphFrame {
    let config = ArangoConfig {
        endpoint: endpoint.to_owned(),
        database: database.to_owned(),
        graph: graph.to_owned(),
        vertex_collection: vertex_collection.to_owned(),
        edge_collection: edge_collection.to_owned(),
        username: username.to_owned(),
        password: password.to_owned(),
    };
    let connector: std::sync::Arc<dyn Connector> =
        std::sync::Arc::new(ArangoConnector::new(config));
    PyLazyGraphFrame {
        inner: LazyGraphFrame::from_connector(connector),
    }
}

/// Create a lazy graph frame backed by a SPARQL endpoint.
///
/// ```python
/// lazy = gf.read_sparql(
///     endpoint="https://dbpedia.org/sparql",
///     node_template="SELECT ?id ?label WHERE { ?id rdfs:label ?label }",
///     edge_template="SELECT ?src ?dst WHERE { ?src ?edge ?dst }",
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (endpoint, node_template, edge_template, expand_template=None))]
fn read_sparql(
    endpoint: &str,
    node_template: &str,
    edge_template: &str,
    expand_template: Option<&str>,
) -> PyLazyGraphFrame {
    let config = SparqlConfig {
        endpoint: endpoint.to_owned(),
        node_template: node_template.to_owned(),
        edge_template: edge_template.to_owned(),
        expand_template: expand_template.map(str::to_owned),
    };
    let connector: std::sync::Arc<dyn Connector> =
        std::sync::Arc::new(SparqlConnector::new(config));
    PyLazyGraphFrame {
        inner: LazyGraphFrame::from_connector(connector),
    }
}

#[pyfunction]
fn write_rdf(graph: PyRef<'_, PyGraphFrame>, path: &Bound<'_, PyAny>) -> PyResult<()> {
    let _ = graph;
    let path = path_from_py_any(path)?;
    unsupported_write_impl("write_rdf", &path)
}

#[pyfunction]
fn write_owl(graph: PyRef<'_, PyGraphFrame>, path: &Bound<'_, PyAny>) -> PyResult<()> {
    let _ = graph;
    let path = path_from_py_any(path)?;
    unsupported_write_impl("write_owl", &path)
}

/// Partition a `GraphFrame` into `n_shards` shards.
///
/// Convenience top-level alias for `graph.partition(n_shards, strategy)`.
///
/// Parameters
/// ----------
/// graph : GraphFrame
/// n_shards : int
///     Number of partitions.
/// strategy : str, optional
///     ``"hash"`` (default), ``"range"``, or ``"label"``.
///
/// Returns
/// -------
/// PartitionedGraph
#[pyfunction]
#[pyo3(signature = (graph, n_shards, strategy = "hash"))]
fn partition_graph(
    graph: PyRef<'_, PyGraphFrame>,
    n_shards: usize,
    strategy: &str,
) -> PyResult<PyPartitionedGraph> {
    graph.partition(n_shards, strategy)
}

#[pymodule]
#[pyo3(name = "_graphframe_py")]
fn graphframe_py(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", graphframe_core::version())?;
    m.add("String", "String")?;
    m.add("Int", "Int")?;
    m.add("Float", "Float")?;
    m.add("Bool", "Bool")?;
    m.add("Date", "Date")?;
    m.add("DateTime", "DateTime")?;
    m.add("Duration", "Duration")?;
    m.add("Any", "Any")?;

    m.add_class::<PyNodeFrame>()?;
    m.add_class::<PyEdgeFrame>()?;
    m.add_class::<PyGraphFrame>()?;
    m.add_class::<PyLazyGraphFrame>()?;
    m.add_class::<PyExpr>()?;
    m.add_class::<PyAggExpr>()?;
    m.add_class::<PyStrExprNamespace>()?;
    m.add_class::<PyPatternNode>()?;
    m.add_class::<PyPatternEdge>()?;
    m.add_class::<PyPartitionedGraph>()?;

    m.add_function(wrap_pyfunction!(col, m)?)?;
    m.add_function(wrap_pyfunction!(node, m)?)?;
    m.add_function(wrap_pyfunction!(edge, m)?)?;
    m.add_function(wrap_pyfunction!(count, m)?)?;
    m.add_function(wrap_pyfunction!(sum, m)?)?;
    m.add_function(wrap_pyfunction!(mean, m)?)?;
    m.add_function(wrap_pyfunction!(list, m)?)?;
    m.add_function(wrap_pyfunction!(first, m)?)?;
    m.add_function(wrap_pyfunction!(last, m)?)?;
    m.add_function(wrap_pyfunction!(read_gf, m)?)?;
    m.add_function(wrap_pyfunction!(read_gfb_py, m)?)?;
    m.add_function(wrap_pyfunction!(read_parquet_graph_py, m)?)?;
    m.add_function(wrap_pyfunction!(write_gf, m)?)?;
    m.add_function(wrap_pyfunction!(write_gfb_py, m)?)?;
    m.add_function(wrap_pyfunction!(write_parquet_graph_py, m)?)?;
    m.add_function(wrap_pyfunction!(write_rdf, m)?)?;
    m.add_function(wrap_pyfunction!(write_owl, m)?)?;
    m.add_function(wrap_pyfunction!(read_neo4j, m)?)?;
    m.add_function(wrap_pyfunction!(read_arangodb, m)?)?;
    m.add_function(wrap_pyfunction!(read_sparql, m)?)?;
    m.add_function(wrap_pyfunction!(partition_graph, m)?)?;
    m.add("read_gfb", m.getattr("read_gfb_py")?)?;
    m.add("read_parquet_graph", m.getattr("read_parquet_graph_py")?)?;
    m.add("write_gfb", m.getattr("write_gfb_py")?)?;
    m.add("write_parquet_graph", m.getattr("write_parquet_graph_py")?)?;
    Ok(())
}

fn extract_boolean_mask(mask: &Bound<'_, PyAny>) -> PyResult<BooleanArray> {
    if let Ok(values) = mask.extract::<Vec<Option<bool>>>() {
        return Ok(BooleanArray::from(values));
    }

    if let Ok(values) = mask.extract::<Vec<bool>>() {
        return Ok(BooleanArray::from(values));
    }

    if let Ok(PyArrowType(array_data)) = mask.extract::<PyArrowType<ArrayData>>() {
        let array = make_array(array_data);
        let array = array
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| {
                PyTypeError::new_err(
                    "filter mask must be a boolean sequence or pyarrow.BooleanArray",
                )
            })?;
        return Ok(array.clone());
    }

    Err(PyTypeError::new_err(
        "filter mask must be a boolean sequence or pyarrow.BooleanArray",
    ))
}

fn gf_error_to_py_err(err: GFError) -> PyErr {
    let message = err.to_string();
    match err {
        GFError::NodeNotFound { .. }
        | GFError::EdgeNotFound { .. }
        | GFError::ColumnNotFound { .. }
        | GFError::InvalidPatternAlias { .. } => PyKeyError::new_err(message),

        GFError::ReservedColumnType { .. }
        | GFError::TypeMismatch { .. }
        | GFError::CannotInferType { .. }
        | GFError::TypeInferenceFailed { .. }
        | GFError::InvalidType { .. }
        | GFError::InvalidCast { .. }
        | GFError::DefaultTypeMismatch { .. } => PyTypeError::new_err(message),

        GFError::MissingReservedColumn { .. }
        | GFError::ReservedColumnName { .. }
        | GFError::DuplicateNodeId { .. }
        | GFError::DanglingEdge { .. }
        | GFError::InvalidDirection { .. }
        | GFError::SchemaMismatch { .. }
        | GFError::LengthMismatch { .. }
        | GFError::MissingRequiredField { .. }
        | GFError::UniqueViolation { .. }
        | GFError::CircularInheritance { .. }
        | GFError::SchemaValidation { .. }
        | GFError::ParseError { .. }
        | GFError::InvalidConfig { .. }
        | GFError::NegativeWeight { .. } => PyValueError::new_err(message),

        GFError::UnsupportedOperation { .. } => PyNotImplementedError::new_err(message),
        GFError::IoError(_) => PyOSError::new_err(message),
        GFError::ConnectorError { .. } => PyRuntimeError::new_err(message),
    }
}

fn build_edge_node_ids(frame: &EdgeFrame) -> Vec<String> {
    let batch = frame.to_record_batch();
    let src_col = batch
        .column_by_name(COL_EDGE_SRC)
        .expect("validated EdgeFrame has _src")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("validated EdgeFrame _src is Utf8");
    let dst_col = batch
        .column_by_name(COL_EDGE_DST)
        .expect("validated EdgeFrame has _dst")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("validated EdgeFrame _dst is Utf8");

    let mut node_ids = vec![String::new(); frame.node_count()];
    for row in 0..frame.len() {
        for id in [src_col.value(row), dst_col.value(row)] {
            if let Some(idx) = frame.node_row_idx(id) {
                if node_ids[idx as usize].is_empty() {
                    node_ids[idx as usize] = id.to_owned();
                }
            }
        }
    }

    node_ids
}

fn python_to_direction(value: &str) -> PyResult<Direction> {
    match value {
        "out" => Ok(Direction::Out),
        "in" => Ok(Direction::In),
        "both" => Ok(Direction::Both),
        "none" => Ok(Direction::None),
        other => Err(PyValueError::new_err(format!(
            "invalid direction: {other}; expected one of: out, in, both, none"
        ))),
    }
}

fn normalize_edge_type_spec(edge_type: Option<&Bound<'_, PyAny>>) -> PyResult<EdgeTypeSpec> {
    let Some(edge_type) = edge_type else {
        return Ok(EdgeTypeSpec::Any);
    };

    if edge_type.is_none() {
        return Ok(EdgeTypeSpec::Any);
    }

    if let Ok(value) = edge_type.extract::<String>() {
        return Ok(EdgeTypeSpec::Single(value));
    }

    if let Ok(values) = edge_type.extract::<Vec<String>>() {
        return Ok(match values.len() {
            0 => EdgeTypeSpec::Any,
            1 => EdgeTypeSpec::Single(values.into_iter().next().unwrap()),
            _ => EdgeTypeSpec::Multiple(values),
        });
    }

    Err(PyTypeError::new_err(
        "edge_type must be None, a string, or a sequence of strings",
    ))
}

fn shortest_path_edge_type(edge_type: Option<String>) -> EdgeTypeSpec {
    match edge_type {
        Some(edge_type) => EdgeTypeSpec::Single(edge_type),
        None => EdgeTypeSpec::Any,
    }
}

fn expr_from_col_name(name: &str) -> Expr {
    if let Some((alias, field)) = name.split_once('.') {
        if !alias.is_empty() && !field.is_empty() {
            return Expr::PatternCol {
                alias: alias.to_owned(),
                field: field.to_owned(),
            };
        }
    }

    Expr::Col {
        name: name.to_owned(),
    }
}

fn normalize_agg_expr_input(value: &Bound<'_, PyAny>) -> PyResult<Expr> {
    if let Ok(expr) = value.extract::<PyRef<'_, PyExpr>>() {
        return Ok(expr.inner.clone());
    }

    if let Ok(name) = value.extract::<String>() {
        return Ok(expr_from_col_name(&name));
    }

    Err(PyTypeError::new_err(
        "aggregate helpers expect an Expr or column name string",
    ))
}

fn expr_from_py_operand(value: &Bound<'_, PyAny>) -> PyResult<Expr> {
    if let Ok(expr) = value.extract::<PyRef<'_, PyExpr>>() {
        return Ok(expr.inner.clone());
    }

    Ok(Expr::Literal {
        value: scalar_from_py_any(value)?,
    })
}

fn scalar_from_py_any(value: &Bound<'_, PyAny>) -> PyResult<ScalarValue> {
    if value.is_none() {
        return Ok(ScalarValue::Null);
    }

    if let Ok(value) = value.extract::<bool>() {
        return Ok(ScalarValue::Bool(value));
    }

    if let Ok(value) = value.extract::<i64>() {
        return Ok(ScalarValue::Int(value));
    }

    if let Ok(value) = value.extract::<f64>() {
        return Ok(ScalarValue::Float(value));
    }

    if let Ok(value) = value.extract::<String>() {
        return Ok(ScalarValue::String(value));
    }

    if let Ok(values) = value.downcast::<PyList>() {
        return scalar_list_from_iter(values.iter());
    }

    if let Ok(values) = value.downcast::<PyTuple>() {
        return scalar_list_from_iter(values.iter());
    }

    Err(PyTypeError::new_err(
        "expected an Expr or a supported literal (None, bool, int, float, str, homogeneous list)",
    ))
}

fn scalar_list_from_iter<'py, I>(iter: I) -> PyResult<ScalarValue>
where
    I: IntoIterator<Item = Bound<'py, PyAny>>,
{
    let values: Vec<ScalarValue> = iter
        .into_iter()
        .map(|item| scalar_from_py_any(&item))
        .collect::<PyResult<_>>()?;

    ensure_homogeneous_scalar_list(&values)?;
    Ok(ScalarValue::List(values))
}

fn ensure_homogeneous_scalar_list(values: &[ScalarValue]) -> PyResult<()> {
    if let Some(first) = values.first() {
        let first_tag = scalar_type_tag(first);
        if values
            .iter()
            .any(|value| scalar_type_tag(value) != first_tag)
        {
            return Err(PyTypeError::new_err(
                "list literals must be homogeneous to lower into ScalarValue::List",
            ));
        }
    }

    Ok(())
}

fn scalar_type_tag(value: &ScalarValue) -> &'static str {
    match value {
        ScalarValue::Null => "null",
        ScalarValue::String(_) => "string",
        ScalarValue::Int(_) => "int",
        ScalarValue::Float(_) => "float",
        ScalarValue::Bool(_) => "bool",
        ScalarValue::List(_) => "list",
    }
}

fn extract_dtype(dtype: &Bound<'_, PyAny>) -> PyResult<DataType> {
    let dtype = dtype
        .extract::<String>()
        .map_err(|_| PyTypeError::new_err("dtype must be a Graphframe dtype marker or string"))?;

    match dtype.as_str() {
        "String" => Ok(DataType::Utf8),
        "Int" => Ok(DataType::Int64),
        "Float" => Ok(DataType::Float64),
        "Bool" => Ok(DataType::Boolean),
        "Null" => Ok(DataType::Null),
        other => Err(PyTypeError::new_err(format!(
            "unsupported dtype marker for cast(): {other}"
        ))),
    }
}

fn path_from_py_any(path: &Bound<'_, PyAny>) -> PyResult<PathBuf> {
    if let Ok(path) = path.extract::<PathBuf>() {
        return Ok(path);
    }

    if let Ok(path) = path.extract::<String>() {
        return Ok(PathBuf::from(path));
    }

    Err(PyTypeError::new_err(
        "path arguments must be str or os.PathLike[str]",
    ))
}

/// Convert a Python list of alternating `PatternNode`/`PatternEdge`/`PatternNode` …
/// into a `Pattern` for `LazyGraphFrame::match_pattern`.
///
/// Expects an odd-length list with at least 3 elements:
///   [node, edge, node, edge, node, …]
fn pattern_from_py_steps(steps: &Bound<'_, PyAny>) -> PyResult<Pattern> {
    let list = steps.downcast::<PyList>().map_err(|_| {
        PyTypeError::new_err("match_pattern: steps must be a list of alternating PatternNode / PatternEdge / PatternNode ...")
    })?;

    let len = list.len();
    if len < 3 || len % 2 == 0 {
        return Err(PyValueError::new_err(
            "match_pattern: steps must have an odd length ≥ 3 ([node, edge, node, ...])",
        ));
    }

    let mut pattern_steps: Vec<PatternStep> = Vec::with_capacity(len / 2);
    let mut i = 0usize;
    while i + 2 <= len - 1 {
        let from_node = list
            .get_item(i)?
            .extract::<PyRef<'_, PyPatternNode>>()
            .map_err(|_| {
                PyTypeError::new_err(format!(
                    "match_pattern: item {i} must be a PatternNode (got {:?})",
                    list.get_item(i).unwrap()
                ))
            })?;

        let pat_edge = list
            .get_item(i + 1)?
            .extract::<PyRef<'_, PyPatternEdge>>()
            .map_err(|_| {
                PyTypeError::new_err(format!(
                    "match_pattern: item {} must be a PatternEdge (got {:?})",
                    i + 1,
                    list.get_item(i + 1).unwrap()
                ))
            })?;

        let to_node = list
            .get_item(i + 2)?
            .extract::<PyRef<'_, PyPatternNode>>()
            .map_err(|_| {
                PyTypeError::new_err(format!(
                    "match_pattern: item {} must be a PatternNode (got {:?})",
                    i + 2,
                    list.get_item(i + 2).unwrap()
                ))
            })?;

        let edge_type = match &pat_edge.edge_type {
            Some(t) => EdgeTypeSpec::Single(t.clone()),
            None => EdgeTypeSpec::Any,
        };

        pattern_steps.push(PatternStep {
            from_alias: from_node.alias.clone(),
            edge_alias: None,
            edge_type,
            direction: Direction::Out,
            to_alias: to_node.alias.clone(),
        });

        i += 2;
    }

    Ok(Pattern::new(pattern_steps))
}

fn write_gf_impl(graph: &GraphFrame, path: &PathBuf) -> PyResult<()> {
    core_write_gf(graph, path).map_err(gf_error_to_py_err)
}

fn unsupported_write_impl(method: &str, path: &PathBuf) -> PyResult<()> {
    Err(PyNotImplementedError::new_err(format!(
        "{method} is not implemented in graphframe-core yet (requested path: {})",
        path.display()
    )))
}
