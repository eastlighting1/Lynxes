mod executor;
pub mod expr;
pub mod lazy_graph_frame;
pub mod logical_plan;
pub mod optimizer;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::Connector;
#[cfg(target_arch = "wasm32")]
pub trait Connector: Send + Sync + std::fmt::Debug {
    fn cache_source_key(&self) -> Option<String> {
        None
    }
}
pub use expr::{AggExpr, BinaryOp, EdgeTypeSpec, Expr, Pattern, PatternStep, ScalarValue, StringOp, UnaryOp};
pub use lazy_graph_frame::LazyGraphFrame;
pub use logical_plan::{ExecutionHint, LogicalPlan, PartitionStrategy, PlanDomain};
pub use optimizer::{
    EarlyTermination, Optimizer, OptimizerOptions, OptimizerPass, PartitionParallel,
    PredicatePushdown, ProjectionPushdown, SubgraphCaching, TraversalPruning,
};
