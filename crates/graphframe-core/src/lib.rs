//! Core Rust engine for Graphframe.

mod algo;
#[cfg(not(target_arch = "wasm32"))]
mod connector;
mod error;
mod frame;
mod io;
mod query;
mod schema;
mod types;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use crate::algo::centrality::BetweennessConfig;
pub use crate::algo::community::{CommunityAlgorithm, CommunityConfig};
pub use crate::algo::pagerank::PageRankConfig;
pub use crate::algo::partition::{
    GraphPartitioner, PartitionMethod as GraphPartitionMethod, PartitionStats,
    PartitionedGraph,
};
pub use crate::algo::shortest_path::ShortestPathConfig;
pub use crate::algo::traversal::{bfs, BfsConfig};
#[cfg(not(target_arch = "wasm32"))]
pub use crate::connector::{
    AqlBindVars, AqlQuery, AqlValue, ArangoBackend, ArangoConfig, ArangoConnector, Connector,
    ConnectorFuture, CypherParams, CypherQuery, CypherValue, ExpandResult, FlightAuth,
    FlightConfig, FlightConnector, FlightGraphService, FlightServerConfig, FlightTlsConfig,
    GFConnector, GFConnectorFormat, Neo4jBackend, Neo4jConfig, Neo4jConnector, SparqlBackend,
    SparqlConfig, SparqlConnector, SparqlParams, SparqlQuery, SparqlValue,
};
pub use crate::error::{GFError, Result, SchemaValidationError};
pub use crate::frame::{CsrIndex, EdgeFrame, GraphFrame, NodeFrame};
pub use crate::io::{
    parse_gf, read_gfb_inspect, write_gf, GfbCompression, GfbInspect, GfbReadOptions,
    GfbWriteOptions, ParsedEdgeDecl, ParsedGfDocument, ParsedNodeDecl,
};
#[cfg(not(target_arch = "wasm32"))]
pub use crate::io::{
    read_gfb, read_gfb_streaming, read_gfb_streaming_with_options, read_gfb_with_options,
    write_gfb, GfbGraphStream,
};
#[cfg(not(target_arch = "wasm32"))]
pub use crate::io::{
    read_parquet_graph, read_parquet_graph_with_options, write_parquet_graph, ParquetReadOptions,
};
pub use crate::query::{
    AggExpr, BinaryOp, EarlyTermination, EdgeTypeSpec, ExecutionHint, Expr, LazyGraphFrame,
    LogicalPlan, Optimizer, OptimizerOptions, OptimizerPass, PartitionParallel, PartitionStrategy,
    Pattern, PatternStep, PlanDomain, PredicatePushdown, ProjectionPushdown, ScalarValue,
    StringOp, SubgraphCaching, TraversalPruning, UnaryOp,
};
pub use crate::schema::{EdgeSchema, FieldDef, GFType, GFValue, NodeSchema, Schema};
pub use crate::types::{
    Direction, EdgeId, NodeId, COL_EDGE_DIRECTION, COL_EDGE_DST, COL_EDGE_SRC, COL_EDGE_TYPE,
    COL_NODE_ID, COL_NODE_LABEL, EDGE_RESERVED_COLUMNS, NODE_RESERVED_COLUMNS,
};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
