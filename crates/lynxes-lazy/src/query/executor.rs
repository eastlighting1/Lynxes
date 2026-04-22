#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use std::{cmp::Ordering, sync::Arc};

use arrow_array::{
    builder::{BooleanBuilder, Float64Builder, Int64Builder, Int8Builder, ListBuilder, StringBuilder},
    Array, ArrayRef, BooleanArray, Float64Array, Int64Array, Int8Array, ListArray, RecordBatch,
    StringArray, UInt32Array,
};
use arrow_schema::{DataType, Field, Schema as ArrowSchema};
use hashbrown::{HashMap, HashSet};

use lynxes_core::{
    Direction, EdgeFrame, GFError, GraphFrame, NodeFrame, Result, COL_EDGE_DIRECTION, COL_EDGE_DST,
    COL_EDGE_SRC, COL_EDGE_TYPE,
};
use lynxes_plan::{
    AggExpr, BinaryOp, EdgeTypeSpec, ExecutionHint, Expr, LogicalPlan, Pattern, PatternStep,
    ScalarValue, StringOp, UnaryOp,
};

#[derive(Debug, Clone)]
#[allow(dead_code, clippy::large_enum_variant)]
pub(crate) enum ExecutionValue {
    Graph(GraphFrame),
    Nodes(NodeFrame),
    Edges(EdgeFrame),
    PatternRows(RecordBatch),
}

/// One row of alias bindings produced during `PatternMatch` execution.
///
/// The value space is intentionally fixed to `u32` so the executor can carry
/// lightweight graph-local identifiers while it is still in the step-expansion
/// phase. Node aliases use the `EdgeFrame` local compact node index. When edge
/// aliases start participating in execution, they will use edge row ids in the
/// same `u32` slot space.
///
/// Alias collision rule:
/// if an alias is encountered again with the same bound value, the binding row
/// remains valid and execution continues. If the same alias is encountered with
/// a different value, that row is rejected because the pattern would be asking
/// one alias to represent two different graph elements at once.
#[allow(dead_code)]
type PatternBindingRow = HashMap<String, u32>;

/// The full set of rows emitted by a `PatternMatch` executor pass.
#[allow(dead_code)]
type PatternBindings = Vec<PatternBindingRow>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatternAliasKind {
    Node,
    Edge,
}

#[allow(dead_code)]
fn bind_pattern_alias(row: &mut PatternBindingRow, alias: &str, value: u32) -> Result<()> {
    match row.get(alias).copied() {
        Some(bound) if bound == value => Ok(()),
        Some(bound) => Err(GFError::InvalidConfig {
            message: format!(
                "pattern alias '{alias}' is already bound to {bound}, cannot rebind to {value}"
            ),
        }),
        None => {
            row.insert(alias.to_owned(), value);
            Ok(())
        }
    }
}

pub(crate) fn execute(plan: &LogicalPlan, source_graph: Arc<GraphFrame>) -> Result<ExecutionValue> {
    match plan {
        LogicalPlan::Scan { .. } => Ok(ExecutionValue::Graph(source_graph.as_ref().clone())),
        LogicalPlan::Cache { input, .. } => execute(input, source_graph),
        LogicalPlan::Hint { hint, input } => execute_hint(hint, input, source_graph),
        LogicalPlan::FilterNodes { input, predicate } => {
            let input = execute(input, source_graph)?;
            let nodes = match input {
                ExecutionValue::Graph(graph) => graph.nodes().clone(),
                ExecutionValue::Nodes(nodes) => nodes,
                ExecutionValue::Edges(_) | ExecutionValue::PatternRows(_) => {
                    return Err(unsupported_plan(
                        "FilterNodes cannot consume an edge or pattern-row domain",
                    ));
                }
            };
            let mask = evaluate_node_predicate(&nodes, predicate)?;
            Ok(ExecutionValue::Nodes(nodes.filter(&mask)?))
        }
        LogicalPlan::FilterEdges { input, predicate } => {
            let input = execute(input, source_graph)?;
            let edges = match input {
                ExecutionValue::Graph(graph) => graph.edges().clone(),
                ExecutionValue::Edges(edges) => edges,
                ExecutionValue::Nodes(_) | ExecutionValue::PatternRows(_) => {
                    return Err(unsupported_plan(
                        "FilterEdges cannot consume a node or pattern-row domain",
                    ));
                }
            };
            let mask = evaluate_edge_predicate(&edges, predicate)?;
            Ok(ExecutionValue::Edges(edges.filter(&mask)?))
        }
        LogicalPlan::ProjectNodes { input, columns } => {
            let input = execute(input, source_graph)?;
            let nodes = match input {
                ExecutionValue::Graph(graph) => graph.nodes().clone(),
                ExecutionValue::Nodes(nodes) => nodes,
                ExecutionValue::Edges(_) | ExecutionValue::PatternRows(_) => {
                    return Err(unsupported_plan(
                        "ProjectNodes cannot consume an edge or pattern-row domain",
                    ));
                }
            };
            let columns: Vec<&str> = columns.iter().map(String::as_str).collect();
            Ok(ExecutionValue::Nodes(nodes.select(&columns)?))
        }
        LogicalPlan::ProjectEdges { input, columns } => {
            let input = execute(input, source_graph)?;
            let edges = match input {
                ExecutionValue::Graph(graph) => graph.edges().clone(),
                ExecutionValue::Edges(edges) => edges,
                ExecutionValue::Nodes(_) | ExecutionValue::PatternRows(_) => {
                    return Err(unsupported_plan(
                        "ProjectEdges cannot consume a node or pattern-row domain",
                    ));
                }
            };
            let columns: Vec<&str> = columns.iter().map(String::as_str).collect();
            Ok(ExecutionValue::Edges(edges.select(&columns)?))
        }
        LogicalPlan::Sort {
            input,
            by,
            descending,
        } => {
            let input = execute(input, source_graph)?;
            match input {
                ExecutionValue::Nodes(nodes) => {
                    Ok(ExecutionValue::Nodes(sort_nodes(&nodes, by, *descending)?))
                }
                ExecutionValue::Edges(edges) => {
                    Ok(ExecutionValue::Edges(sort_edges(&edges, by, *descending)?))
                }
                ExecutionValue::Graph(_) | ExecutionValue::PatternRows(_) => Err(unsupported_plan(
                    "Sort requires a node or edge domain, not a graph or pattern-row domain",
                )),
            }
        }
        LogicalPlan::Limit { input, n } => {
            let input = execute(input, source_graph)?;
            match input {
                ExecutionValue::Nodes(nodes) => {
                    Ok(ExecutionValue::Nodes(nodes.slice(0, (*n).min(nodes.len()))))
                }
                ExecutionValue::Edges(edges) => Ok(ExecutionValue::Edges(limit_edges(&edges, *n)?)),
                ExecutionValue::Graph(graph) => {
                    let node_ids: Vec<&str> = graph
                        .nodes()
                        .id_column()
                        .iter()
                        .take(*n)
                        .flatten()
                        .collect();
                    Ok(ExecutionValue::Graph(graph.subgraph(&node_ids)?))
                }
                ExecutionValue::PatternRows(_) => Err(unsupported_plan(
                    "Limit does not yet support pattern-row domains",
                )),
            }
        }
        LogicalPlan::Expand {
            input,
            edge_type,
            hops,
            direction,
            pre_filter,
        } => {
            let input = execute(input, source_graph.clone())?;
            let frontier = match input {
                ExecutionValue::Graph(graph) => graph.nodes().clone(),
                ExecutionValue::Nodes(nodes) => nodes,
                ExecutionValue::Edges(_) | ExecutionValue::PatternRows(_) => {
                    return Err(unsupported_plan(
                        "Expand cannot consume an edge or pattern-row domain",
                    ));
                }
            };
            Ok(ExecutionValue::Graph(expand_graph(
                source_graph.as_ref(),
                &frontier,
                edge_type,
                *hops as usize,
                *direction,
                pre_filter.as_ref(),
                None,
            )?))
        }
        LogicalPlan::Traverse { input, pattern } => {
            let input = execute(input, source_graph.clone())?;
            let frontier = match input {
                ExecutionValue::Graph(graph) => graph.nodes().clone(),
                ExecutionValue::Nodes(nodes) => nodes,
                ExecutionValue::Edges(_) | ExecutionValue::PatternRows(_) => {
                    return Err(unsupported_plan(
                        "Traverse cannot consume an edge or pattern-row domain",
                    ));
                }
            };
            Ok(ExecutionValue::Graph(traverse_graph(
                source_graph.as_ref(),
                &frontier,
                pattern,
                None,
            )?))
        }
        LogicalPlan::PatternMatch {
            input,
            pattern,
            where_,
        } => {
            let input = execute(input, source_graph.clone())?;
            let anchors = match input {
                ExecutionValue::Graph(graph) => graph.nodes().clone(),
                ExecutionValue::Nodes(nodes) => nodes,
                ExecutionValue::Edges(_) => {
                    return Err(unsupported_plan(
                        "PatternMatch cannot consume an edge domain",
                    ));
                }
                ExecutionValue::PatternRows(_) => {
                    return Err(unsupported_plan(
                        "PatternMatch cannot consume an existing pattern-row domain",
                    ));
                }
            };
            Ok(ExecutionValue::PatternRows(execute_pattern_match(
                source_graph.as_ref(),
                &anchors,
                pattern,
                where_.as_ref(),
            )?))
        }
        LogicalPlan::AggregateNeighbors {
            input,
            edge_type,
            agg,
        } => {
            let input = execute(input, source_graph.clone())?;
            let anchors = match input {
                ExecutionValue::Graph(graph) => graph.nodes().clone(),
                ExecutionValue::Nodes(nodes) => nodes,
                ExecutionValue::Edges(_) | ExecutionValue::PatternRows(_) => {
                    return Err(unsupported_plan(
                        "AggregateNeighbors cannot consume an edge or pattern-row domain",
                    ));
                }
            };
            Ok(ExecutionValue::Nodes(aggregate_neighbors(
                source_graph.as_ref(),
                &anchors,
                edge_type,
                agg,
            )?))
        }
    }
}

fn unsupported_plan(message: &str) -> GFError {
    GFError::UnsupportedOperation {
        message: message.to_owned(),
    }
}

// ── Hint dispatch ─────────────────────────────────────────────────────────────

fn execute_hint(
    hint: &ExecutionHint,
    input: &LogicalPlan,
    source_graph: Arc<GraphFrame>,
) -> Result<ExecutionValue> {
    match hint {
        ExecutionHint::LimitAware { n } => execute_limit_aware(input, source_graph, *n),
        ExecutionHint::TopK { n } => execute_top_k(input, source_graph, *n),
        ExecutionHint::PartitionParallel { .. } => execute_partition_parallel(input, source_graph),
    }
}

/// Executes `input` and, when it is an Expand or Traverse node, stops BFS
/// expansion once `n` nodes have been collected.
fn execute_limit_aware(
    input: &LogicalPlan,
    source_graph: Arc<GraphFrame>,
    n: usize,
) -> Result<ExecutionValue> {
    match input {
        LogicalPlan::Expand {
            input: inner,
            edge_type,
            hops,
            direction,
            pre_filter,
        } => {
            let frontier_val = execute(inner, source_graph.clone())?;
            let frontier = extract_node_frontier(frontier_val, "LimitAware Expand")?;
            Ok(ExecutionValue::Graph(expand_graph(
                source_graph.as_ref(),
                &frontier,
                edge_type,
                *hops as usize,
                *direction,
                pre_filter.as_ref(),
                Some(n),
            )?))
        }
        LogicalPlan::Traverse {
            input: inner,
            pattern,
        } => {
            let frontier_val = execute(inner, source_graph.clone())?;
            let frontier = extract_node_frontier(frontier_val, "LimitAware Traverse")?;
            Ok(ExecutionValue::Graph(traverse_graph(
                source_graph.as_ref(),
                &frontier,
                pattern,
                Some(n),
            )?))
        }
        // Hint not directly above an expansion node — fall through without limit.
        _ => execute(input, source_graph),
    }
}

/// Executes `input` and, when it is a Sort node, performs a partial top-K sort
/// rather than a full sort, yielding O(E log n) instead of O(E log E).
fn execute_top_k(
    input: &LogicalPlan,
    source_graph: Arc<GraphFrame>,
    n: usize,
) -> Result<ExecutionValue> {
    match input {
        LogicalPlan::Sort {
            input: inner,
            by,
            descending,
        } => {
            let inner_val = execute(inner, source_graph)?;
            match inner_val {
                ExecutionValue::Nodes(nodes) => {
                    let batch = top_k_batch(nodes.to_record_batch(), by, *descending, n)?;
                    Ok(ExecutionValue::Nodes(NodeFrame::from_record_batch(batch)?))
                }
                ExecutionValue::Edges(edges) => {
                    let batch = top_k_batch(edges.to_record_batch(), by, *descending, n)?;
                    Ok(ExecutionValue::Edges(EdgeFrame::from_record_batch(batch)?))
                }
                ExecutionValue::Graph(_) | ExecutionValue::PatternRows(_) => Err(
                    unsupported_plan(
                        "TopK Sort requires a node or edge domain, not a graph or pattern-row domain",
                    ),
                ),
            }
        }
        // Hint not directly above a Sort — fall through.
        _ => execute(input, source_graph),
    }
}

/// Extracts a `NodeFrame` frontier from an `ExecutionValue`.
fn extract_node_frontier(val: ExecutionValue, context: &str) -> Result<NodeFrame> {
    match val {
        ExecutionValue::Graph(graph) => Ok(graph.nodes().clone()),
        ExecutionValue::Nodes(nodes) => Ok(nodes),
        ExecutionValue::Edges(_) | ExecutionValue::PatternRows(_) => Err(unsupported_plan(
            &format!(
                "{context} cannot consume an edge or pattern-row domain"
            ),
        )),
    }
}

/// Partial top-K sort of `batch` by column `by`.
///
/// Returns a new `RecordBatch` containing the k rows with the largest (when
/// `descending`) or smallest (when `!descending`) values in the sort column,
/// themselves sorted.
///
/// Complexity: O(n + k log k) average via `select_nth_unstable_by`, versus
/// O(n log n) for a full sort.  Falls back to full sort when `k >= n`.
fn top_k_batch(batch: &RecordBatch, by: &str, descending: bool, k: usize) -> Result<RecordBatch> {
    let n = batch.num_rows();
    if k >= n {
        // Nothing to save — just do a regular sort.
        return reorder_batch(batch, by, descending);
    }

    let sort_column = batch
        .column_by_name(by)
        .ok_or_else(|| GFError::ColumnNotFound {
            column: by.to_owned(),
        })?;

    // Collect (row_index, sort_value) pairs.
    let mut indexed: Vec<(usize, Value)> = (0..n)
        .map(|row| {
            let val = read_array_value(sort_column.as_ref(), row, by)?;
            Ok((row, val))
        })
        .collect::<Result<_>>()?;

    // `select_nth_unstable_by` rearranges `indexed` so that elements at
    // positions 0..=k-1 are the k "best" entries (in some order) and the
    // element at position k-1 is in its final sorted position.
    // Average O(n); worst-case O(n²) but that is rare in practice.
    indexed.select_nth_unstable_by(k - 1, |a, b| {
        // Compare in the direction we want to KEEP (ascending for min-heap,
        // descending for max-heap).  We want the k entries that would appear
        // first in a full sort, so we use the same ordering as the full sort.
        compare_sort_values(&a.1, &b.1, descending).then_with(|| a.0.cmp(&b.0))
    });

    // Sort only the chosen k rows into final order.
    let mut top_k = indexed[..k].to_vec();
    top_k.sort_by(|a, b| compare_sort_values(&a.1, &b.1, descending).then_with(|| a.0.cmp(&b.0)));

    let indices: UInt32Array = top_k.iter().map(|(idx, _)| *idx as u32).collect();
    let reordered: Vec<ArrayRef> = batch
        .columns()
        .iter()
        .map(|col| arrow::compute::take(col.as_ref(), &indices, None))
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| GFError::IoError(std::io::Error::other(e)))?;

    RecordBatch::try_new(batch.schema_ref().clone(), reordered)
        .map_err(|e| GFError::IoError(std::io::Error::other(e)))
}

fn limit_edges(edges: &EdgeFrame, n: usize) -> Result<EdgeFrame> {
    let len = n.min(edges.len());
    let mask: BooleanArray = (0..edges.len()).map(|idx| Some(idx < len)).collect();
    edges.filter(&mask)
}

/// Raw CSR BFS: takes an owned slice of frontier node IDs and returns the
/// complete visited set and the set of retained edge-row indices.
///
/// This is the inner kernel shared by the serial wrapper (`expand_graph`) and
/// the parallel shard runner (`execute_partition_parallel`).
///
/// Complexity: O(hops × Σ degree(v) for v in frontier).
/// When `stop_at = Some(n)` halts once visited reaches n nodes.
fn expand_graph_raw(
    graph: &GraphFrame,
    frontier_ids: &[String],
    edge_type: &EdgeTypeSpec,
    hops: usize,
    direction: Direction,
    pre_filter: Option<&Expr>,
    stop_at: Option<usize>,
) -> Result<(HashSet<String>, HashSet<usize>)> {
    let edge_node_ids = build_edge_node_ids(graph.edges())?;
    let mut visited: HashSet<String> = frontier_ids.iter().cloned().collect();
    let mut current: HashSet<String> = visited.clone();
    // HashSet deduplicates retained edge rows naturally (same edge via two paths).
    let mut retained_rows: HashSet<usize> = HashSet::new();

    // Pre-extract the _direction column for O(1) per-edge direction checks.
    let edge_batch = graph.edges().to_record_batch();
    let dir_col = int8_array(edge_batch, COL_EDGE_DIRECTION)?;

    'expand: for _ in 0..hops {
        let mut next: HashSet<String> = HashSet::new();

        for frontier_id in &current {
            // Resolve EdgeFrame-local index; nodes absent from edge set have no edges.
            let Some(local_idx) = graph.edges().node_row_idx(frontier_id) else {
                continue;
            };

            match direction {
                Direction::Out => {
                    // Follow edges where this node is the _src.
                    let n_locals = graph.edges().out_neighbors(local_idx);
                    let e_rows = graph.edges().out_edge_ids(local_idx);
                    for (&n_local, &e_row) in n_locals.iter().zip(e_rows) {
                        let edge_dir = Direction::try_from(dir_col.value(e_row as usize))?;
                        // Semantic Out traversal: only Out and Both edges.
                        if !matches!(edge_dir, Direction::Out | Direction::Both) {
                            continue;
                        }
                        if !matches_edge_type(graph.edges().edge_type_at(e_row), edge_type) {
                            continue;
                        }
                        let Some(candidate) =
                            edge_node_ids.get(n_local as usize).map(String::as_str)
                        else {
                            continue;
                        };
                        if candidate_passes_pre_filter(graph.nodes(), candidate, pre_filter)? {
                            let is_new = visited.insert(candidate.to_owned());
                            if is_new {
                                next.insert(candidate.to_owned());
                            }
                            retained_rows.insert(e_row as usize);
                            if stop_at.is_some_and(|lim| visited.len() >= lim) {
                                break 'expand;
                            }
                        }
                    }
                }
                Direction::In => {
                    // Follow edges where this node is the _dst (reverse CSR).
                    let n_locals = graph.edges().in_neighbors(local_idx);
                    let e_rows = graph.edges().in_edge_ids(local_idx);
                    for (&n_local, &e_row) in n_locals.iter().zip(e_rows) {
                        let edge_dir = Direction::try_from(dir_col.value(e_row as usize))?;
                        // Semantic In traversal: Out, Both, and In edges.
                        if !matches!(edge_dir, Direction::Out | Direction::Both | Direction::In) {
                            continue;
                        }
                        if !matches_edge_type(graph.edges().edge_type_at(e_row), edge_type) {
                            continue;
                        }
                        let Some(candidate) =
                            edge_node_ids.get(n_local as usize).map(String::as_str)
                        else {
                            continue;
                        };
                        if candidate_passes_pre_filter(graph.nodes(), candidate, pre_filter)? {
                            let is_new = visited.insert(candidate.to_owned());
                            if is_new {
                                next.insert(candidate.to_owned());
                            }
                            retained_rows.insert(e_row as usize);
                            if stop_at.is_some_and(|lim| visited.len() >= lim) {
                                break 'expand;
                            }
                        }
                    }
                }
                Direction::Both | Direction::None => {
                    // Follow all edges regardless of semantic direction.
                    for (&n_local, &e_row) in graph
                        .edges()
                        .out_neighbors(local_idx)
                        .iter()
                        .zip(graph.edges().out_edge_ids(local_idx))
                    {
                        if !matches_edge_type(graph.edges().edge_type_at(e_row), edge_type) {
                            continue;
                        }
                        let Some(candidate) =
                            edge_node_ids.get(n_local as usize).map(String::as_str)
                        else {
                            continue;
                        };
                        if candidate_passes_pre_filter(graph.nodes(), candidate, pre_filter)? {
                            let is_new = visited.insert(candidate.to_owned());
                            if is_new {
                                next.insert(candidate.to_owned());
                            }
                            retained_rows.insert(e_row as usize);
                            if stop_at.is_some_and(|lim| visited.len() >= lim) {
                                break 'expand;
                            }
                        }
                    }
                    for (&n_local, &e_row) in graph
                        .edges()
                        .in_neighbors(local_idx)
                        .iter()
                        .zip(graph.edges().in_edge_ids(local_idx))
                    {
                        if !matches_edge_type(graph.edges().edge_type_at(e_row), edge_type) {
                            continue;
                        }
                        let Some(candidate) =
                            edge_node_ids.get(n_local as usize).map(String::as_str)
                        else {
                            continue;
                        };
                        if candidate_passes_pre_filter(graph.nodes(), candidate, pre_filter)? {
                            let is_new = visited.insert(candidate.to_owned());
                            if is_new {
                                next.insert(candidate.to_owned());
                            }
                            retained_rows.insert(e_row as usize);
                            if stop_at.is_some_and(|lim| visited.len() >= lim) {
                                break 'expand;
                            }
                        }
                    }
                }
            }
        }

        if next.is_empty() {
            break;
        }
        current = next;
    }

    Ok((visited, retained_rows))
}

/// Materialises the final `GraphFrame` from the raw BFS outputs.
///
/// Shared by the serial path and the parallel merge step.
fn build_expand_result(
    graph: &GraphFrame,
    visited: HashSet<String>,
    retained_rows: HashSet<usize>,
) -> Result<GraphFrame> {
    let retained_ids: Vec<&str> = visited.iter().map(String::as_str).collect();
    let nodes = graph.subgraph(&retained_ids)?.nodes().clone();

    let mask: BooleanArray = (0..graph.edges().len())
        .map(|row| Some(retained_rows.contains(&row)))
        .collect();
    let edges = graph.edges().filter(&mask)?;

    GraphFrame::new(nodes, edges)
}

/// Serial CSR multi-hop expansion.  Thin wrapper around `expand_graph_raw` +
/// `build_expand_result`.
fn expand_graph(
    graph: &GraphFrame,
    frontier: &NodeFrame,
    edge_type: &EdgeTypeSpec,
    hops: usize,
    direction: Direction,
    pre_filter: Option<&Expr>,
    stop_at: Option<usize>,
) -> Result<GraphFrame> {
    let frontier_ids: Vec<String> = frontier
        .id_column()
        .iter()
        .flatten()
        .map(str::to_owned)
        .collect();
    let (visited, retained_rows) = expand_graph_raw(
        graph,
        &frontier_ids,
        edge_type,
        hops,
        direction,
        pre_filter,
        stop_at,
    )?;
    build_expand_result(graph, visited, retained_rows)
}

/// Parallel frontier-partitioned `Expand` execution.
///
/// The frontier is split into `rayon::current_num_threads()` contiguous chunks.
/// Each chunk runs `expand_graph_raw` independently on a Rayon thread; the
/// per-shard `(visited, retained_rows)` sets are then unioned into a single
/// result.
///
/// Correctness: a node reachable from the full frontier within `hops` hops is
/// reachable from at least one shard's subset of the frontier → it appears in
/// at least one shard's visited set → it appears in the union.
///
/// Trade-off: nodes reachable from multiple shards are expanded redundantly.
/// This is acceptable when the frontier is large relative to the reachable set,
/// which is the case the `PartitionParallel` optimizer targets.
fn execute_partition_parallel(
    input: &LogicalPlan,
    source_graph: Arc<GraphFrame>,
) -> Result<ExecutionValue> {
    match input {
        LogicalPlan::Expand {
            input: inner,
            edge_type,
            hops,
            direction,
            pre_filter,
        } => {
            let frontier_val = execute(inner, source_graph.clone())?;
            let frontier = extract_node_frontier(frontier_val, "PartitionParallel Expand")?;

            let all_ids: Vec<String> = frontier
                .id_column()
                .iter()
                .flatten()
                .map(str::to_owned)
                .collect();

            #[cfg(not(target_arch = "wasm32"))]
            let n_threads = rayon::current_num_threads();
            #[cfg(target_arch = "wasm32")]
            let n_threads = 1usize;

            // Below this threshold the Rayon overhead outweighs the gain; fall
            // back to the serial path.
            if all_ids.len() < 2 * n_threads {
                return Ok(ExecutionValue::Graph(expand_graph(
                    source_graph.as_ref(),
                    &frontier,
                    edge_type,
                    *hops as usize,
                    *direction,
                    pre_filter.as_ref(),
                    None,
                )?));
            }

            let chunk_size = all_ids.len().div_ceil(n_threads);
            let graph_ref = source_graph.as_ref();
            let hops_u = *hops as usize;

            // Parallel shard execution (Rayon on native, serial on wasm).
            #[cfg(not(target_arch = "wasm32"))]
            let partial: Vec<Result<(HashSet<String>, HashSet<usize>)>> = all_ids
                .par_chunks(chunk_size)
                .map(|chunk| {
                    expand_graph_raw(
                        graph_ref,
                        chunk,
                        edge_type,
                        hops_u,
                        *direction,
                        pre_filter.as_ref(),
                        None,
                    )
                })
                .collect();
            #[cfg(target_arch = "wasm32")]
            let partial: Vec<Result<(HashSet<String>, HashSet<usize>)>> = all_ids
                .chunks(chunk_size)
                .map(|chunk| {
                    expand_graph_raw(
                        graph_ref,
                        chunk,
                        edge_type,
                        hops_u,
                        *direction,
                        pre_filter.as_ref(),
                        None,
                    )
                })
                .collect();

            // Sequentially merge partial results (the union is the correct answer).
            let mut visited: HashSet<String> = HashSet::new();
            let mut retained_rows: HashSet<usize> = HashSet::new();
            for result in partial {
                let (v, r) = result?;
                visited.extend(v);
                retained_rows.extend(r);
            }

            Ok(ExecutionValue::Graph(build_expand_result(
                source_graph.as_ref(),
                visited,
                retained_rows,
            )?))
        }
        // PatternRoots: PatternMatch executor is not yet implemented; fall through.
        _ => execute(input, source_graph),
    }
}

fn traverse_graph(
    graph: &GraphFrame,
    start: &NodeFrame,
    pattern: &[PatternStep],
    stop_at: Option<usize>,
) -> Result<GraphFrame> {
    let mut frontier: HashSet<String> = start
        .id_column()
        .iter()
        .flatten()
        .map(str::to_owned)
        .collect();
    let mut visited = frontier.clone();
    let mut retained_rows: HashSet<usize> = HashSet::new();

    for step in pattern {
        let (next, rows) = expand_frontier_csr(graph, &frontier, &step.edge_type, step.direction)?;
        if next.is_empty() {
            break;
        }
        retained_rows.extend(rows);
        visited.extend(next.iter().cloned());
        frontier = next;
        if stop_at.is_some_and(|lim| visited.len() >= lim) {
            break;
        }
    }

    let retained_ids: Vec<&str> = visited.iter().map(String::as_str).collect();
    let nodes = graph.subgraph(&retained_ids)?.nodes().clone();
    let mask: BooleanArray = (0..graph.edges().len())
        .map(|row| Some(retained_rows.contains(&row)))
        .collect();
    let edges = graph.edges().filter(&mask)?;

    GraphFrame::new(nodes, edges)
}

/// CSR-based single-hop frontier expansion used by `traverse_graph`.
///
/// Returns the set of newly discovered neighbour IDs and the set of edge row
/// indices that were traversed.  No visited-dedup is performed here; callers
/// accumulate `visited` themselves across steps.
fn expand_frontier_csr(
    graph: &GraphFrame,
    frontier: &HashSet<String>,
    edge_type: &EdgeTypeSpec,
    direction: Direction,
) -> Result<(HashSet<String>, HashSet<usize>)> {
    let edge_node_ids = build_edge_node_ids(graph.edges())?;
    let mut next: HashSet<String> = HashSet::new();
    let mut rows: HashSet<usize> = HashSet::new();

    for frontier_id in frontier {
        let Some(local_idx) = graph.edges().node_row_idx(frontier_id) else {
            continue;
        };

        match direction {
            Direction::Out => {
                for (&n_local, &e_row) in graph
                    .edges()
                    .out_neighbors(local_idx)
                    .iter()
                    .zip(graph.edges().out_edge_ids(local_idx))
                {
                    if !matches_edge_type(graph.edges().edge_type_at(e_row), edge_type) {
                        continue;
                    }
                    if let Some(candidate) = edge_node_ids.get(n_local as usize) {
                        next.insert(candidate.to_owned());
                        rows.insert(e_row as usize);
                    }
                }
            }
            Direction::In => {
                for (&n_local, &e_row) in graph
                    .edges()
                    .in_neighbors(local_idx)
                    .iter()
                    .zip(graph.edges().in_edge_ids(local_idx))
                {
                    if !matches_edge_type(graph.edges().edge_type_at(e_row), edge_type) {
                        continue;
                    }
                    if let Some(candidate) = edge_node_ids.get(n_local as usize) {
                        next.insert(candidate.to_owned());
                        rows.insert(e_row as usize);
                    }
                }
            }
            Direction::Both | Direction::None => {
                for (&n_local, &e_row) in graph
                    .edges()
                    .out_neighbors(local_idx)
                    .iter()
                    .zip(graph.edges().out_edge_ids(local_idx))
                {
                    if !matches_edge_type(graph.edges().edge_type_at(e_row), edge_type) {
                        continue;
                    }
                    if let Some(candidate) = edge_node_ids.get(n_local as usize) {
                        next.insert(candidate.to_owned());
                        rows.insert(e_row as usize);
                    }
                }
                for (&n_local, &e_row) in graph
                    .edges()
                    .in_neighbors(local_idx)
                    .iter()
                    .zip(graph.edges().in_edge_ids(local_idx))
                {
                    if !matches_edge_type(graph.edges().edge_type_at(e_row), edge_type) {
                        continue;
                    }
                    if let Some(candidate) = edge_node_ids.get(n_local as usize) {
                        next.insert(candidate.to_owned());
                        rows.insert(e_row as usize);
                    }
                }
            }
        }
    }

    Ok((next, rows))
}

fn aggregate_neighbors(
    graph: &GraphFrame,
    anchors: &NodeFrame,
    edge_type: &str,
    agg: &AggExpr,
) -> Result<NodeFrame> {
    let output_name = agg_output_name(agg);
    let output_type = agg_output_type(graph, agg)?;
    let values: Vec<Value> = anchors
        .id_column()
        .iter()
        .flatten()
        .map(|node_id| aggregate_for_node(graph, node_id, edge_type, agg))
        .collect::<Result<_>>()?;
    append_node_column(anchors, &output_name, &output_type, values)
}

fn build_edge_node_ids(edges: &EdgeFrame) -> Result<Vec<String>> {
    let batch = edges.to_record_batch();
    let src_col = string_array(batch, COL_EDGE_SRC)?;
    let dst_col = string_array(batch, COL_EDGE_DST)?;
    let mut node_ids = vec![String::new(); edges.node_count()];

    for row in 0..edges.len() {
        for id in [src_col.value(row), dst_col.value(row)] {
            if let Some(idx) = edges.node_row_idx(id) {
                if node_ids[idx as usize].is_empty() {
                    node_ids[idx as usize] = id.to_owned();
                }
            }
        }
    }

    Ok(node_ids)
}

/// Unwrap `AggExpr::Alias` to reach the inner expression for type-matching purposes.
fn agg_inner(agg: &AggExpr) -> &AggExpr {
    match agg {
        AggExpr::Alias { expr, .. } => agg_inner(expr),
        other => other,
    }
}

fn aggregate_for_node(
    graph: &GraphFrame,
    node_id: &str,
    edge_type: &str,
    agg: &AggExpr,
) -> Result<Value> {
    let edge_idx = match graph.edges().node_row_idx(node_id) {
        Some(idx) => idx,
        None => return empty_aggregate_value(agg),
    };
    let edge_rows = graph.edges().out_edge_ids(edge_idx);
    let edge_batch = graph.edges().to_record_batch();
    let type_col = string_array(edge_batch, COL_EDGE_TYPE)?;
    let dst_col = string_array(edge_batch, COL_EDGE_DST)?;

    if matches!(agg_inner(agg), AggExpr::Count) {
        let count = edge_rows
            .iter()
            .filter(|&&edge_row| type_col.value(edge_row as usize) == edge_type)
            .count();
        return Ok(Value::Int(count as i64));
    }

    let mut values = Vec::new();
    for &edge_row in edge_rows {
        let edge_row = edge_row as usize;
        if type_col.value(edge_row) != edge_type {
            continue;
        }
        let neighbor_id = dst_col.value(edge_row);
        let neighbor_row = graph
            .nodes()
            .row(neighbor_id)
            .ok_or_else(|| GFError::NodeNotFound {
                id: neighbor_id.to_owned(),
            })?;
        if let Some(value) = evaluate_neighbor_value(&neighbor_row, edge_batch, edge_row, agg)? {
            values.push(value);
        }
    }

    reduce_agg_values(agg, values)
}

fn evaluate_neighbor_value(
    neighbor_row: &RecordBatch,
    edge_batch: &RecordBatch,
    edge_row: usize,
    agg: &AggExpr,
) -> Result<Option<Value>> {
    match agg {
        AggExpr::Count => Ok(None),
        AggExpr::Sum { expr }
        | AggExpr::Mean { expr }
        | AggExpr::List { expr }
        | AggExpr::First { expr }
        | AggExpr::Last { expr } => {
            let value = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, expr)?;
            Ok((value != Value::Null).then_some(value))
        }
        AggExpr::Alias { expr, .. } => {
            evaluate_neighbor_value(neighbor_row, edge_batch, edge_row, expr)
        }
    }
}

fn evaluate_neighbor_expr(
    neighbor_row: &RecordBatch,
    edge_batch: &RecordBatch,
    edge_row: usize,
    expr: &Expr,
) -> Result<Value> {
    match expr {
        Expr::Col { name } => {
            if edge_batch.column_by_name(name).is_some() {
                read_column_value(edge_batch, edge_row, name)
            } else {
                read_column_value(neighbor_row, 0, name)
            }
        }
        Expr::Literal { value } => Ok(convert_scalar(value)),
        Expr::BinaryOp { left, op, right } => {
            let left = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, left)?;
            let right = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, right)?;
            evaluate_binary_values(left, op, right)
        }
        Expr::UnaryOp { op, expr } => {
            let value = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, expr)?;
            match (op, value) {
                (UnaryOp::Neg, Value::Int(value)) => Ok(Value::Int(-value)),
                (UnaryOp::Neg, Value::Float(value)) => Ok(Value::Float(-value)),
                (_, other) => Err(GFError::TypeMismatch {
                    message: format!("unsupported unary expression operand: {other:?}"),
                }),
            }
        }
        Expr::ListContains { expr, item } => {
            let list = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, expr)?;
            let item = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, item)?;
            match list {
                Value::List(values) => Ok(Value::Bool(values.iter().any(|value| value == &item))),
                other => Err(GFError::TypeMismatch {
                    message: format!("ListContains expects a list operand, got {other:?}"),
                }),
            }
        }
        Expr::Cast { expr, dtype } => cast_value(
            evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, expr)?,
            dtype,
        ),
        Expr::And { left, right } => {
            let left = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, left)?;
            let right = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, right)?;
            match (left, right) {
                (Value::Bool(left), Value::Bool(right)) => Ok(Value::Bool(left && right)),
                (left, right) => Err(GFError::TypeMismatch {
                    message: format!(
                        "boolean op expects bool operands, got {left:?} and {right:?}"
                    ),
                }),
            }
        }
        Expr::Or { left, right } => {
            let left = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, left)?;
            let right = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, right)?;
            match (left, right) {
                (Value::Bool(left), Value::Bool(right)) => Ok(Value::Bool(left || right)),
                (left, right) => Err(GFError::TypeMismatch {
                    message: format!(
                        "boolean op expects bool operands, got {left:?} and {right:?}"
                    ),
                }),
            }
        }
        Expr::Not { expr } => {
            match evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, expr)? {
                Value::Bool(value) => Ok(Value::Bool(!value)),
                other => Err(GFError::TypeMismatch {
                    message: format!("Not expects bool, got {other:?}"),
                }),
            }
        }
        Expr::PatternCol { alias, field } => Err(GFError::UnsupportedOperation {
            message: format!("PatternCol({alias}.{field}) requires PatternMatch execution"),
        }),
        Expr::StringOp { op, expr, pattern } => {
            let subject = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, expr)?;
            let pat = evaluate_neighbor_expr(neighbor_row, edge_batch, edge_row, pattern)?;
            match (subject, pat) {
                (Value::String(s), Value::String(p)) => Ok(Value::Bool(match op {
                    StringOp::Contains => s.contains(p.as_str()),
                    StringOp::StartsWith => s.starts_with(p.as_str()),
                    StringOp::EndsWith => s.ends_with(p.as_str()),
                })),
                (s, p) => Err(GFError::TypeMismatch {
                    message: format!("StringOp expects string operands, got {s:?} and {p:?}"),
                }),
            }
        }
    }
}

fn evaluate_binary_values(left: Value, op: &BinaryOp, right: Value) -> Result<Value> {
    match op {
        BinaryOp::Eq => Ok(Value::Bool(left == right)),
        BinaryOp::NotEq => Ok(Value::Bool(left != right)),
        BinaryOp::Gt => compare_values(left, right, Ordering::Greater),
        BinaryOp::GtEq => compare_values_inclusive(left, right, Ordering::Greater),
        BinaryOp::Lt => compare_values(left, right, Ordering::Less),
        BinaryOp::LtEq => compare_values_inclusive(left, right, Ordering::Less),
        BinaryOp::Add => arithmetic_values(left, right, |l, r| l + r, |l, r| l + r),
        BinaryOp::Sub => arithmetic_values(left, right, |l, r| l - r, |l, r| l - r),
        BinaryOp::Mul => arithmetic_values(left, right, |l, r| l * r, |l, r| l * r),
        BinaryOp::Div => arithmetic_values(left, right, |l, r| l / r, |l, r| l / r),
    }
}

fn reduce_agg_values(agg: &AggExpr, values: Vec<Value>) -> Result<Value> {
    match agg {
        AggExpr::Count => Ok(Value::Int(values.len() as i64)),
        AggExpr::Sum { .. } => {
            if values.iter().any(|value| matches!(value, Value::Float(_))) {
                Ok(Value::Float(
                    values
                        .into_iter()
                        .map(|value| match value {
                            Value::Int(value) => Ok(value as f64),
                            Value::Float(value) => Ok(value),
                            other => Err(GFError::TypeMismatch {
                                message: format!("sum expects numeric values, got {other:?}"),
                            }),
                        })
                        .collect::<Result<Vec<_>>>()?
                        .into_iter()
                        .sum(),
                ))
            } else {
                Ok(Value::Int(
                    values
                        .into_iter()
                        .map(|value| match value {
                            Value::Int(value) => Ok(value),
                            other => Err(GFError::TypeMismatch {
                                message: format!("sum expects int values, got {other:?}"),
                            }),
                        })
                        .collect::<Result<Vec<_>>>()?
                        .into_iter()
                        .sum(),
                ))
            }
        }
        AggExpr::Mean { .. } => {
            if values.is_empty() {
                return Ok(Value::Null);
            }
            let count = values.len();
            let total: f64 = values
                .into_iter()
                .map(|value| match value {
                    Value::Int(value) => Ok(value as f64),
                    Value::Float(value) => Ok(value),
                    other => Err(GFError::TypeMismatch {
                        message: format!("mean expects numeric values, got {other:?}"),
                    }),
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .sum();
            Ok(Value::Float(total / count as f64))
        }
        AggExpr::List { .. } => Ok(Value::List(values)),
        AggExpr::First { .. } => Ok(values.into_iter().next().unwrap_or(Value::Null)),
        AggExpr::Last { .. } => Ok(values.into_iter().last().unwrap_or(Value::Null)),
        AggExpr::Alias { expr, .. } => reduce_agg_values(expr, values),
    }
}

fn empty_aggregate_value(agg: &AggExpr) -> Result<Value> {
    match agg {
        AggExpr::Count => Ok(Value::Int(0)),
        AggExpr::Sum { .. } => Ok(Value::Int(0)),
        AggExpr::Mean { .. } | AggExpr::First { .. } | AggExpr::Last { .. } => Ok(Value::Null),
        AggExpr::List { .. } => Ok(Value::List(Vec::new())),
        AggExpr::Alias { expr, .. } => empty_aggregate_value(expr),
    }
}

fn agg_output_name(agg: &AggExpr) -> String {
    match agg {
        AggExpr::Count => "count".to_owned(),
        AggExpr::Sum { .. } => "sum".to_owned(),
        AggExpr::Mean { .. } => "mean".to_owned(),
        AggExpr::List { .. } => "list".to_owned(),
        AggExpr::First { .. } => "first".to_owned(),
        AggExpr::Last { .. } => "last".to_owned(),
        AggExpr::Alias { name, .. } => name.clone(),
    }
}

fn agg_output_type(graph: &GraphFrame, agg: &AggExpr) -> Result<DataType> {
    match agg {
        AggExpr::Count => Ok(DataType::Int64),
        AggExpr::Sum { expr } => infer_expr_type(graph, expr),
        AggExpr::Mean { .. } => Ok(DataType::Float64),
        AggExpr::List { expr } => Ok(DataType::List(Arc::new(Field::new(
            "item",
            infer_expr_type(graph, expr)?,
            true,
        )))),
        AggExpr::First { expr } | AggExpr::Last { expr } => infer_expr_type(graph, expr),
        AggExpr::Alias { expr, .. } => agg_output_type(graph, expr),
    }
}

fn infer_expr_type(graph: &GraphFrame, expr: &Expr) -> Result<DataType> {
    match expr {
        Expr::Col { name } => {
            if let Ok(field) = graph.edges().schema().field_with_name(name) {
                Ok(field.data_type().clone())
            } else if let Ok(field) = graph.nodes().schema().field_with_name(name) {
                Ok(field.data_type().clone())
            } else {
                Err(GFError::ColumnNotFound {
                    column: name.to_owned(),
                })
            }
        }
        Expr::Literal { value } => match value {
            ScalarValue::Null => Ok(DataType::Utf8),
            ScalarValue::String(_) => Ok(DataType::Utf8),
            ScalarValue::Int(_) => Ok(DataType::Int64),
            ScalarValue::Float(_) => Ok(DataType::Float64),
            ScalarValue::Bool(_) => Ok(DataType::Boolean),
            ScalarValue::List(values) => Ok(DataType::List(Arc::new(Field::new(
                "item",
                infer_expr_type(
                    graph,
                    &Expr::Literal {
                        value: values
                            .first()
                            .cloned()
                            .unwrap_or(ScalarValue::String(String::new())),
                    },
                )?,
                true,
            )))),
        },
        Expr::BinaryOp { left, op, right } => match op {
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::Gt
            | BinaryOp::GtEq
            | BinaryOp::Lt
            | BinaryOp::LtEq => Ok(DataType::Boolean),
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let left = infer_expr_type(graph, left)?;
                let right = infer_expr_type(graph, right)?;
                if left == DataType::Float64 || right == DataType::Float64 {
                    Ok(DataType::Float64)
                } else {
                    Ok(DataType::Int64)
                }
            }
        },
        Expr::UnaryOp { expr, .. } => infer_expr_type(graph, expr),
        Expr::ListContains { .. } => Ok(DataType::Boolean),
        Expr::Cast { dtype, .. } => Ok(dtype.clone()),
        Expr::And { .. } | Expr::Or { .. } | Expr::Not { .. } => Ok(DataType::Boolean),
        Expr::PatternCol { alias, field } => Err(GFError::UnsupportedOperation {
            message: format!("PatternCol({alias}.{field}) requires PatternMatch execution"),
        }),
        Expr::StringOp { .. } => Ok(DataType::Boolean),
    }
}

fn append_node_column(
    nodes: &NodeFrame,
    column_name: &str,
    data_type: &DataType,
    values: Vec<Value>,
) -> Result<NodeFrame> {
    let new_column = build_value_array(data_type, values)?;
    let mut fields: Vec<Field> = nodes
        .schema()
        .fields()
        .iter()
        .map(|field| field.as_ref().clone())
        .collect();
    fields.push(Field::new(column_name, data_type.clone(), true));
    let mut columns: Vec<ArrayRef> = nodes.to_record_batch().columns().to_vec();
    columns.push(new_column);

    let batch = RecordBatch::try_new(Arc::new(ArrowSchema::new(fields)), columns)
        .map_err(|error| GFError::IoError(std::io::Error::other(error)))?;
    NodeFrame::from_record_batch(batch)
}

fn build_value_array(data_type: &DataType, values: Vec<Value>) -> Result<ArrayRef> {
    match data_type {
        DataType::Int8 => {
            let mut builder = Int8Builder::new();
            for value in values {
                match value {
                    Value::Null => builder.append_null(),
                    Value::Int(value) => builder.append_value(value as i8),
                    other => {
                        return Err(GFError::TypeMismatch {
                            message: format!("expected Int8 aggregation result, got {other:?}"),
                        });
                    }
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Int64 => {
            let mut builder = Int64Builder::new();
            for value in values {
                match value {
                    Value::Null => builder.append_null(),
                    Value::Int(value) => builder.append_value(value),
                    other => {
                        return Err(GFError::TypeMismatch {
                            message: format!("expected Int64 aggregation result, got {other:?}"),
                        });
                    }
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Float64 => {
            let mut builder = Float64Builder::new();
            for value in values {
                match value {
                    Value::Null => builder.append_null(),
                    Value::Int(value) => builder.append_value(value as f64),
                    Value::Float(value) => builder.append_value(value),
                    other => {
                        return Err(GFError::TypeMismatch {
                            message: format!("expected Float64 aggregation result, got {other:?}"),
                        });
                    }
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Utf8 => {
            let mut builder = StringBuilder::new();
            for value in values {
                match value {
                    Value::Null => builder.append_null(),
                    Value::String(value) => builder.append_value(value),
                    other => {
                        return Err(GFError::TypeMismatch {
                            message: format!("expected Utf8 aggregation result, got {other:?}"),
                        });
                    }
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Boolean => {
            let mut builder = BooleanBuilder::new();
            for value in values {
                match value {
                    Value::Null => builder.append_null(),
                    Value::Bool(value) => builder.append_value(value),
                    other => {
                        return Err(GFError::TypeMismatch {
                            message: format!("expected Boolean aggregation result, got {other:?}"),
                        });
                    }
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::List(field) if field.data_type() == &DataType::Utf8 => {
            let mut builder = ListBuilder::new(StringBuilder::new());
            for value in values {
                match value {
                    Value::Null => builder.append(false),
                    Value::List(items) => {
                        for item in items {
                            match item {
                                Value::String(value) => builder.values().append_value(value),
                                Value::Null => builder.values().append_null(),
                                other => {
                                    return Err(GFError::TypeMismatch {
                                        message: format!("expected Utf8 list item, got {other:?}"),
                                    });
                                }
                            }
                        }
                        builder.append(true);
                    }
                    other => {
                        return Err(GFError::TypeMismatch {
                            message: format!(
                                "expected List<Utf8> aggregation result, got {other:?}"
                            ),
                        });
                    }
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::List(field) if field.data_type() == &DataType::Int64 => {
            let mut builder = ListBuilder::new(Int64Builder::new());
            for value in values {
                match value {
                    Value::Null => builder.append(false),
                    Value::List(items) => {
                        for item in items {
                            match item {
                                Value::Int(value) => builder.values().append_value(value),
                                Value::Null => builder.values().append_null(),
                                other => {
                                    return Err(GFError::TypeMismatch {
                                        message: format!("expected Int64 list item, got {other:?}"),
                                    });
                                }
                            }
                        }
                        builder.append(true);
                    }
                    other => {
                        return Err(GFError::TypeMismatch {
                            message: format!(
                                "expected List<Int64> aggregation result, got {other:?}"
                            ),
                        });
                    }
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::List(field) if field.data_type() == &DataType::Float64 => {
            let mut builder = ListBuilder::new(Float64Builder::new());
            for value in values {
                match value {
                    Value::Null => builder.append(false),
                    Value::List(items) => {
                        for item in items {
                            match item {
                                Value::Int(value) => builder.values().append_value(value as f64),
                                Value::Float(value) => builder.values().append_value(value),
                                Value::Null => builder.values().append_null(),
                                other => {
                                    return Err(GFError::TypeMismatch {
                                        message: format!(
                                            "expected Float64 list item, got {other:?}"
                                        ),
                                    });
                                }
                            }
                        }
                        builder.append(true);
                    }
                    other => {
                        return Err(GFError::TypeMismatch {
                            message: format!(
                                "expected List<Float64> aggregation result, got {other:?}"
                            ),
                        });
                    }
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        other => Err(GFError::UnsupportedOperation {
            message: format!("aggregation result type {other:?} is not implemented yet"),
        }),
    }
}

fn candidate_passes_pre_filter(
    nodes: &NodeFrame,
    id: &str,
    pre_filter: Option<&Expr>,
) -> Result<bool> {
    let Some(pre_filter) = pre_filter else {
        return Ok(true);
    };
    let row = nodes
        .row(id)
        .ok_or_else(|| GFError::NodeNotFound { id: id.to_owned() })?;
    match evaluate_expr(&row, 0, pre_filter)? {
        Value::Bool(value) => Ok(value),
        other => Err(GFError::TypeMismatch {
            message: format!("node pre_filter must evaluate to bool, got {other:?}"),
        }),
    }
}

fn evaluate_node_predicate(nodes: &NodeFrame, expr: &Expr) -> Result<BooleanArray> {
    evaluate_predicate(nodes.to_record_batch(), expr)
}

fn evaluate_edge_predicate(edges: &EdgeFrame, expr: &Expr) -> Result<BooleanArray> {
    evaluate_predicate(edges.to_record_batch(), expr)
}

fn evaluate_predicate(batch: &RecordBatch, expr: &Expr) -> Result<BooleanArray> {
    (0..batch.num_rows())
        .map(|row| match evaluate_expr(batch, row, expr)? {
            Value::Bool(value) => Ok(Some(value)),
            other => Err(GFError::TypeMismatch {
                message: format!("filter predicate must evaluate to bool, got {other:?}"),
            }),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Null,
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
}

impl Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::String(left), Self::String(right)) => Some(left.cmp(right)),
            (Self::Int(left), Self::Int(right)) => Some(left.cmp(right)),
            (Self::Float(left), Self::Float(right)) => left.partial_cmp(right),
            (Self::Bool(left), Self::Bool(right)) => Some(left.cmp(right)),
            (Self::Int(left), Self::Float(right)) => (*left as f64).partial_cmp(right),
            (Self::Float(left), Self::Int(right)) => left.partial_cmp(&(*right as f64)),
            _ => None,
        }
    }
}

fn evaluate_expr(batch: &RecordBatch, row: usize, expr: &Expr) -> Result<Value> {
    match expr {
        Expr::Col { name } => read_column_value(batch, row, name),
        Expr::Literal { value } => Ok(convert_scalar(value)),
        Expr::BinaryOp { left, op, right } => evaluate_binary(batch, row, left, op, right),
        Expr::UnaryOp { op, expr } => {
            let value = evaluate_expr(batch, row, expr)?;
            match (op, value) {
                (UnaryOp::Neg, Value::Int(value)) => Ok(Value::Int(-value)),
                (UnaryOp::Neg, Value::Float(value)) => Ok(Value::Float(-value)),
                (_, other) => Err(GFError::TypeMismatch {
                    message: format!("unsupported unary expression operand: {other:?}"),
                }),
            }
        }
        Expr::ListContains { expr, item } => {
            let list = evaluate_expr(batch, row, expr)?;
            let item = evaluate_expr(batch, row, item)?;
            match list {
                Value::List(values) => Ok(Value::Bool(values.iter().any(|value| value == &item))),
                other => Err(GFError::TypeMismatch {
                    message: format!("ListContains expects a list operand, got {other:?}"),
                }),
            }
        }
        Expr::Cast { expr, dtype } => cast_value(evaluate_expr(batch, row, expr)?, dtype),
        Expr::And { left, right } => boolean_op(batch, row, left, right, |l, r| l && r),
        Expr::Or { left, right } => boolean_op(batch, row, left, right, |l, r| l || r),
        Expr::Not { expr } => match evaluate_expr(batch, row, expr)? {
            Value::Bool(value) => Ok(Value::Bool(!value)),
            other => Err(GFError::TypeMismatch {
                message: format!("Not expects bool, got {other:?}"),
            }),
        },
        Expr::PatternCol { alias, field } => Err(GFError::UnsupportedOperation {
            message: format!("PatternCol({alias}.{field}) requires PatternMatch execution"),
        }),
        Expr::StringOp { op, expr, pattern } => {
            let subject = evaluate_expr(batch, row, expr)?;
            let pat = evaluate_expr(batch, row, pattern)?;
            match (subject, pat) {
                (Value::String(s), Value::String(p)) => Ok(Value::Bool(match op {
                    StringOp::Contains => s.contains(p.as_str()),
                    StringOp::StartsWith => s.starts_with(p.as_str()),
                    StringOp::EndsWith => s.ends_with(p.as_str()),
                })),
                (s, p) => Err(GFError::TypeMismatch {
                    message: format!("StringOp expects string operands, got {s:?} and {p:?}"),
                }),
            }
        }
    }
}

fn boolean_op(
    batch: &RecordBatch,
    row: usize,
    left: &Expr,
    right: &Expr,
    f: impl Fn(bool, bool) -> bool,
) -> Result<Value> {
    let left = evaluate_expr(batch, row, left)?;
    let right = evaluate_expr(batch, row, right)?;
    match (left, right) {
        (Value::Bool(left), Value::Bool(right)) => Ok(Value::Bool(f(left, right))),
        (left, right) => Err(GFError::TypeMismatch {
            message: format!("boolean op expects bool operands, got {left:?} and {right:?}"),
        }),
    }
}

fn evaluate_binary(
    batch: &RecordBatch,
    row: usize,
    left: &Expr,
    op: &BinaryOp,
    right: &Expr,
) -> Result<Value> {
    let left = evaluate_expr(batch, row, left)?;
    let right = evaluate_expr(batch, row, right)?;

    match op {
        BinaryOp::Eq => Ok(Value::Bool(left == right)),
        BinaryOp::NotEq => Ok(Value::Bool(left != right)),
        BinaryOp::Gt => compare_values(left, right, Ordering::Greater),
        BinaryOp::GtEq => compare_values_inclusive(left, right, Ordering::Greater),
        BinaryOp::Lt => compare_values(left, right, Ordering::Less),
        BinaryOp::LtEq => compare_values_inclusive(left, right, Ordering::Less),
        BinaryOp::Add => arithmetic_values(left, right, |l, r| l + r, |l, r| l + r),
        BinaryOp::Sub => arithmetic_values(left, right, |l, r| l - r, |l, r| l - r),
        BinaryOp::Mul => arithmetic_values(left, right, |l, r| l * r, |l, r| l * r),
        BinaryOp::Div => arithmetic_values(left, right, |l, r| l / r, |l, r| l / r),
    }
}

fn compare_values(left: Value, right: Value, expected: Ordering) -> Result<Value> {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return Ok(Value::Bool(false));
    }
    let ordering = left
        .partial_cmp(&right)
        .ok_or_else(|| GFError::TypeMismatch {
            message: format!("cannot compare {left:?} and {right:?}"),
        })?;
    Ok(Value::Bool(ordering == expected))
}

fn compare_values_inclusive(left: Value, right: Value, expected: Ordering) -> Result<Value> {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return Ok(Value::Bool(false));
    }
    let ordering = left
        .partial_cmp(&right)
        .ok_or_else(|| GFError::TypeMismatch {
            message: format!("cannot compare {left:?} and {right:?}"),
        })?;
    Ok(Value::Bool(
        ordering == expected || ordering == Ordering::Equal,
    ))
}

fn arithmetic_values(
    left: Value,
    right: Value,
    int_op: impl Fn(i64, i64) -> i64,
    float_op: impl Fn(f64, f64) -> f64,
) -> Result<Value> {
    match (left, right) {
        (Value::Int(left), Value::Int(right)) => Ok(Value::Int(int_op(left, right))),
        (Value::Int(left), Value::Float(right)) => Ok(Value::Float(float_op(left as f64, right))),
        (Value::Float(left), Value::Int(right)) => Ok(Value::Float(float_op(left, right as f64))),
        (Value::Float(left), Value::Float(right)) => Ok(Value::Float(float_op(left, right))),
        (left, right) => Err(GFError::TypeMismatch {
            message: format!("arithmetic expects numeric operands, got {left:?} and {right:?}"),
        }),
    }
}

fn cast_value(value: Value, dtype: &arrow_schema::DataType) -> Result<Value> {
    use arrow_schema::DataType;

    match (value, dtype) {
        (Value::Null, _) => Ok(Value::Null),
        (Value::Int(value), DataType::Float64) => Ok(Value::Float(value as f64)),
        (Value::Int(value), DataType::Int64) => Ok(Value::Int(value)),
        (Value::Float(value), DataType::Float64) => Ok(Value::Float(value)),
        (Value::Float(value), DataType::Int64) => Ok(Value::Int(value as i64)),
        (Value::String(value), DataType::Utf8) => Ok(Value::String(value)),
        (Value::Bool(value), DataType::Boolean) => Ok(Value::Bool(value)),
        (value, dtype) => Err(GFError::InvalidCast {
            from: format!("{value:?}"),
            to: format!("{dtype:?}"),
        }),
    }
}

fn convert_scalar(value: &ScalarValue) -> Value {
    match value {
        ScalarValue::Null => Value::Null,
        ScalarValue::String(value) => Value::String(value.clone()),
        ScalarValue::Int(value) => Value::Int(*value),
        ScalarValue::Float(value) => Value::Float(*value),
        ScalarValue::Bool(value) => Value::Bool(*value),
        ScalarValue::List(values) => Value::List(values.iter().map(convert_scalar).collect()),
    }
}

fn read_column_value(batch: &RecordBatch, row: usize, name: &str) -> Result<Value> {
    let column = batch
        .column_by_name(name)
        .ok_or_else(|| GFError::ColumnNotFound {
            column: name.to_owned(),
        })?;
    read_array_value(column.as_ref(), row, name)
}

fn read_array_value(array: &dyn Array, row: usize, name: &str) -> Result<Value> {
    if array.is_null(row) {
        return Ok(Value::Null);
    }

    if let Some(array) = array.as_any().downcast_ref::<StringArray>() {
        return Ok(Value::String(array.value(row).to_owned()));
    }
    if let Some(array) = array.as_any().downcast_ref::<Int8Array>() {
        return Ok(Value::Int(array.value(row) as i64));
    }
    if let Some(array) = array.as_any().downcast_ref::<Int64Array>() {
        return Ok(Value::Int(array.value(row)));
    }
    if let Some(array) = array.as_any().downcast_ref::<Float64Array>() {
        return Ok(Value::Float(array.value(row)));
    }
    if let Some(array) = array.as_any().downcast_ref::<BooleanArray>() {
        return Ok(Value::Bool(array.value(row)));
    }
    if let Some(array) = array.as_any().downcast_ref::<ListArray>() {
        let values = array.value(row);
        let values = values
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| GFError::TypeMismatch {
                message: format!("list column {name} currently supports Utf8 children only"),
            })?;
        return Ok(Value::List(
            values
                .iter()
                .flatten()
                .map(|value| Value::String(value.to_owned()))
                .collect(),
        ));
    }

    Err(GFError::UnsupportedOperation {
        message: format!(
            "executor cannot read column {name} with type {:?}",
            array.data_type()
        ),
    })
}

fn matches_edge_type(edge_type: &str, spec: &EdgeTypeSpec) -> bool {
    match spec {
        EdgeTypeSpec::Single(value) => edge_type == value,
        EdgeTypeSpec::Multiple(values) => values.iter().any(|value| value == edge_type),
        EdgeTypeSpec::Any => true,
    }
}

/// Expands one typed pattern step over an existing binding table.
///
/// Each input row must already bind `step.from_alias` to an `EdgeFrame` local
/// compact node index. The executor looks up matching edges from that node and
/// emits zero or more output rows. `step.to_alias` is bound to the neighboring
/// node index and `step.edge_alias`, when present, is bound to the traversed
/// edge row id.
///
/// Alias collisions are handled row-locally: if rebinding an alias would
/// assign a different value, that candidate row is dropped. This preserves the
/// "same alias means same graph element" rule without aborting the whole
/// pattern execution.
#[allow(dead_code)]
fn execute_pattern_step(
    graph: &GraphFrame,
    step: &PatternStep,
    input: &PatternBindings,
) -> Result<PatternBindings> {
    let edges = graph.edges();
    let mut output = Vec::new();

    for row in input {
        let from_idx = row
            .get(&step.from_alias)
            .copied()
            .ok_or_else(|| GFError::InvalidConfig {
                message: format!(
                    "pattern step requires alias '{}' to be bound before execution",
                    step.from_alias
                ),
            })?;

        for (neighbor_idx, edge_row) in pattern_candidates(edges, from_idx, step) {
            let mut next = row.clone();

            if bind_pattern_alias(&mut next, &step.to_alias, neighbor_idx).is_err() {
                continue;
            }
            if let Some(edge_alias) = step.edge_alias.as_deref() {
                if bind_pattern_alias(&mut next, edge_alias, edge_row).is_err() {
                    continue;
                }
            }

            output.push(next);
        }
    }

    Ok(output)
}

#[allow(dead_code)]
fn execute_pattern_steps(
    graph: &GraphFrame,
    steps: &[PatternStep],
    seed_bindings: &PatternBindings,
) -> Result<PatternBindings> {
    let mut current = seed_bindings.clone();

    for step in steps {
        if current.is_empty() {
            break;
        }
        current = execute_pattern_step(graph, step, &current)?;
    }

    Ok(current)
}

#[allow(dead_code)]
fn apply_pattern_where(
    graph: &GraphFrame,
    bindings: &PatternBindings,
    where_: Option<&Expr>,
) -> Result<PatternBindings> {
    let Some(where_) = where_ else {
        return Ok(bindings.clone());
    };

    let mut filtered = Vec::with_capacity(bindings.len());
    for row in bindings {
        match evaluate_pattern_expr(graph, row, where_)? {
            Value::Bool(true) => filtered.push(row.clone()),
            Value::Bool(false) => {}
            other => {
                return Err(GFError::TypeMismatch {
                    message: format!("pattern where predicate must evaluate to bool, got {other:?}"),
                });
            }
        }
    }

    Ok(filtered)
}

#[allow(dead_code)]
fn evaluate_pattern_expr(
    graph: &GraphFrame,
    binding: &PatternBindingRow,
    expr: &Expr,
) -> Result<Value> {
    match expr {
        Expr::Col { name } => Err(GFError::UnsupportedOperation {
            message: format!("plain column reference '{name}' is not supported in PatternMatch where clauses"),
        }),
        Expr::Literal { value } => Ok(convert_scalar(value)),
        Expr::BinaryOp { left, op, right } => evaluate_binary_values(
            evaluate_pattern_expr(graph, binding, left)?,
            op,
            evaluate_pattern_expr(graph, binding, right)?,
        ),
        Expr::UnaryOp { op, expr } => {
            let value = evaluate_pattern_expr(graph, binding, expr)?;
            match (op, value) {
                (UnaryOp::Neg, Value::Int(value)) => Ok(Value::Int(-value)),
                (UnaryOp::Neg, Value::Float(value)) => Ok(Value::Float(-value)),
                (_, other) => Err(GFError::TypeMismatch {
                    message: format!("unsupported unary expression operand: {other:?}"),
                }),
            }
        }
        Expr::ListContains { expr, item } => {
            let list = evaluate_pattern_expr(graph, binding, expr)?;
            let item = evaluate_pattern_expr(graph, binding, item)?;
            match list {
                Value::List(values) => Ok(Value::Bool(values.iter().any(|value| value == &item))),
                other => Err(GFError::TypeMismatch {
                    message: format!("ListContains expects a list operand, got {other:?}"),
                }),
            }
        }
        Expr::Cast { expr, dtype } => cast_value(evaluate_pattern_expr(graph, binding, expr)?, dtype),
        Expr::And { left, right } => {
            let left = evaluate_pattern_expr(graph, binding, left)?;
            let right = evaluate_pattern_expr(graph, binding, right)?;
            match (left, right) {
                (Value::Bool(left), Value::Bool(right)) => Ok(Value::Bool(left && right)),
                (left, right) => Err(GFError::TypeMismatch {
                    message: format!("boolean op expects bool operands, got {left:?} and {right:?}"),
                }),
            }
        }
        Expr::Or { left, right } => {
            let left = evaluate_pattern_expr(graph, binding, left)?;
            let right = evaluate_pattern_expr(graph, binding, right)?;
            match (left, right) {
                (Value::Bool(left), Value::Bool(right)) => Ok(Value::Bool(left || right)),
                (left, right) => Err(GFError::TypeMismatch {
                    message: format!("boolean op expects bool operands, got {left:?} and {right:?}"),
                }),
            }
        }
        Expr::Not { expr } => match evaluate_pattern_expr(graph, binding, expr)? {
            Value::Bool(value) => Ok(Value::Bool(!value)),
            other => Err(GFError::TypeMismatch {
                message: format!("Not expects bool, got {other:?}"),
            }),
        },
        Expr::PatternCol { alias, field } => read_pattern_field_value(graph, binding, alias, field),
        Expr::StringOp { op, expr, pattern } => {
            let subject = evaluate_pattern_expr(graph, binding, expr)?;
            let pat = evaluate_pattern_expr(graph, binding, pattern)?;
            match (subject, pat) {
                (Value::String(s), Value::String(p)) => Ok(Value::Bool(match op {
                    StringOp::Contains => s.contains(p.as_str()),
                    StringOp::StartsWith => s.starts_with(p.as_str()),
                    StringOp::EndsWith => s.ends_with(p.as_str()),
                })),
                (s, p) => Err(GFError::TypeMismatch {
                    message: format!("StringOp expects string operands, got {s:?} and {p:?}"),
                }),
            }
        }
    }
}

#[allow(dead_code)]
fn read_pattern_field_value(
    graph: &GraphFrame,
    binding: &PatternBindingRow,
    alias: &str,
    field: &str,
) -> Result<Value> {
    let bound = binding
        .get(alias)
        .copied()
        .ok_or_else(|| GFError::InvalidConfig {
            message: format!("pattern predicate requires alias '{alias}' to be bound"),
        })?;

    let node_has_field = graph.nodes().schema().field_with_name(field).is_ok();
    let edge_has_field = graph.edges().schema().field_with_name(field).is_ok();

    match (node_has_field, edge_has_field) {
        (true, false) => {
            let edge_node_ids = build_edge_node_ids(graph.edges())?;
            let node_id = edge_node_ids
                .get(bound as usize)
                .map(String::as_str)
                .ok_or_else(|| GFError::InvalidConfig {
                    message: format!(
                        "pattern alias '{alias}' is not a valid edge-local node index: {bound}"
                    ),
                })?;
            let node_row = graph.nodes().row_index(node_id).ok_or_else(|| GFError::NodeNotFound {
                id: node_id.to_owned(),
            })?;
            read_column_value(graph.nodes().to_record_batch(), node_row as usize, field)
        }
        (false, true) => {
            if bound as usize >= graph.edges().len() {
                return Err(GFError::InvalidConfig {
                    message: format!(
                        "pattern alias '{alias}' is not a valid edge row index: {bound}"
                    ),
                });
            }
            read_column_value(graph.edges().to_record_batch(), bound as usize, field)
        }
        (true, true) => Err(GFError::InvalidConfig {
            message: format!(
                "pattern field reference '{alias}.{field}' is ambiguous because '{field}' exists on both nodes and edges"
            ),
        }),
        (false, false) => Err(GFError::ColumnNotFound {
            column: field.to_owned(),
        }),
    }
}

#[allow(dead_code)]
fn collect_pattern_aliases(pattern: &[PatternStep]) -> Result<Vec<(String, PatternAliasKind)>> {
    let mut aliases = Vec::new();
    let mut kinds = HashMap::<String, PatternAliasKind>::new();

    let mut register = |alias: &str, kind: PatternAliasKind| -> Result<()> {
        match kinds.get(alias).copied() {
            Some(existing) if existing == kind => Ok(()),
            Some(existing) => Err(GFError::InvalidConfig {
                message: format!(
                    "pattern alias '{alias}' is used as both {:?} and {:?}",
                    existing, kind
                ),
            }),
            None => {
                kinds.insert(alias.to_owned(), kind);
                aliases.push((alias.to_owned(), kind));
                Ok(())
            }
        }
    };

    for step in pattern {
        register(&step.from_alias, PatternAliasKind::Node)?;
        if let Some(edge_alias) = step.edge_alias.as_deref() {
            register(edge_alias, PatternAliasKind::Edge)?;
        }
        register(&step.to_alias, PatternAliasKind::Node)?;
    }

    Ok(aliases)
}

#[allow(dead_code)]
fn materialize_pattern_bindings(
    graph: &GraphFrame,
    pattern: &[PatternStep],
    bindings: &PatternBindings,
) -> Result<RecordBatch> {
    let aliases = collect_pattern_aliases(pattern)?;
    let mut fields = Vec::new();
    let mut columns = Vec::new();

    for (alias, kind) in aliases {
        let schema = match kind {
            PatternAliasKind::Node => graph.nodes().schema(),
            PatternAliasKind::Edge => graph.edges().schema(),
        };

        for field in schema.fields() {
            let field = field.as_ref();
            let qualified_name = format!("{alias}.{}", field.name());
            let values = bindings
                .iter()
                .map(|row| read_pattern_field_value(graph, row, &alias, field.name()))
                .collect::<Result<Vec<_>>>()?;
            fields.push(Field::new(
                &qualified_name,
                field.data_type().clone(),
                field.is_nullable(),
            ));
            columns.push(build_value_array(field.data_type(), values)?);
        }
    }

    RecordBatch::try_new(Arc::new(ArrowSchema::new(fields)), columns)
        .map_err(|error| GFError::IoError(std::io::Error::other(error)))
}

#[allow(dead_code)]
fn execute_pattern_match(
    graph: &GraphFrame,
    anchors: &NodeFrame,
    pattern: &Pattern,
    where_: Option<&Expr>,
) -> Result<RecordBatch> {
    if pattern.is_empty() {
        return Err(GFError::InvalidConfig {
            message: "PatternMatch requires at least one step".to_owned(),
        });
    }

    let first_step = &pattern.steps[0];
    let mut seed_bindings = PatternBindings::new();
    for anchor_id in anchors.id_column().iter().flatten() {
        let Some(edge_local_idx) = graph.edges().node_row_idx(anchor_id) else {
            continue;
        };

        let mut row = PatternBindingRow::new();
        bind_pattern_alias(&mut row, &first_step.from_alias, edge_local_idx)?;
        seed_bindings.push(row);
    }

    let bindings = execute_pattern_steps(graph, &pattern.steps, &seed_bindings)?;
    let filtered = apply_pattern_where(graph, &bindings, where_)?;
    materialize_pattern_bindings(graph, &pattern.steps, &filtered)
}

#[allow(dead_code)]
fn pattern_candidates(edges: &EdgeFrame, from_idx: u32, step: &PatternStep) -> Vec<(u32, u32)> {
    let mut candidates = Vec::new();

    if matches!(step.direction, Direction::Out | Direction::Both) {
        for (&dst_idx, &edge_row) in edges
            .out_neighbors(from_idx)
            .iter()
            .zip(edges.out_edge_ids(from_idx).iter())
        {
            if matches_edge_type(edges.edge_type_at(edge_row), &step.edge_type) {
                candidates.push((dst_idx, edge_row));
            }
        }
    }

    if matches!(step.direction, Direction::In | Direction::Both) {
        for (&src_idx, &edge_row) in edges
            .in_neighbors(from_idx)
            .iter()
            .zip(edges.in_edge_ids(from_idx).iter())
        {
            if matches_edge_type(edges.edge_type_at(edge_row), &step.edge_type) {
                candidates.push((src_idx, edge_row));
            }
        }
    }

    candidates
}

fn sort_nodes(nodes: &NodeFrame, by: &str, descending: bool) -> Result<NodeFrame> {
    let batch = reorder_batch(nodes.to_record_batch(), by, descending)?;
    NodeFrame::from_record_batch(batch)
}

fn sort_edges(edges: &EdgeFrame, by: &str, descending: bool) -> Result<EdgeFrame> {
    let batch = reorder_batch(edges.to_record_batch(), by, descending)?;
    EdgeFrame::from_record_batch(batch)
}

fn reorder_batch(batch: &RecordBatch, by: &str, descending: bool) -> Result<RecordBatch> {
    let sort_column = batch
        .column_by_name(by)
        .ok_or_else(|| GFError::ColumnNotFound {
            column: by.to_owned(),
        })?;
    let mut row_indices: Vec<usize> = (0..batch.num_rows()).collect();
    row_indices.sort_by(|left, right| {
        let left_value = read_array_value(sort_column.as_ref(), *left, by).unwrap_or(Value::Null);
        let right_value = read_array_value(sort_column.as_ref(), *right, by).unwrap_or(Value::Null);
        compare_sort_values(&left_value, &right_value, descending).then_with(|| left.cmp(right))
    });

    let indices = UInt32Array::from(
        row_indices
            .into_iter()
            .map(|idx| idx as u32)
            .collect::<Vec<_>>(),
    );
    let reordered_columns: Vec<ArrayRef> = batch
        .columns()
        .iter()
        .map(|column| arrow::compute::take(column.as_ref(), &indices, None))
        .collect::<std::result::Result<_, _>>()
        .map_err(|error| GFError::IoError(std::io::Error::other(error)))?;

    RecordBatch::try_new(batch.schema_ref().clone(), reordered_columns)
        .map_err(|error| GFError::IoError(std::io::Error::other(error)))
}

fn compare_sort_values(left: &Value, right: &Value, descending: bool) -> Ordering {
    let ordering = match (left, right) {
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Null, _) => Ordering::Greater,
        (_, Value::Null) => Ordering::Less,
        _ => left.partial_cmp(right).unwrap_or(Ordering::Equal),
    };
    if descending {
        ordering.reverse()
    } else {
        ordering
    }
}

fn string_array<'a>(batch: &'a RecordBatch, name: &str) -> Result<&'a StringArray> {
    batch
        .column_by_name(name)
        .ok_or_else(|| GFError::MissingReservedColumn {
            column: name.to_owned(),
        })?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| GFError::ReservedColumnType {
            column: name.to_owned(),
            expected: "Utf8".to_owned(),
            actual: "non-Utf8 array".to_owned(),
        })
}

fn int8_array<'a>(batch: &'a RecordBatch, name: &str) -> Result<&'a arrow_array::Int8Array> {
    batch
        .column_by_name(name)
        .ok_or_else(|| GFError::MissingReservedColumn {
            column: name.to_owned(),
        })?
        .as_any()
        .downcast_ref::<arrow_array::Int8Array>()
        .ok_or_else(|| GFError::ReservedColumnType {
            column: name.to_owned(),
            expected: "Int8".to_owned(),
            actual: "non-Int8 array".to_owned(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{
        builder::{ListBuilder, StringBuilder},
        ArrayRef, Int64Array, Int8Array,
    };
    use arrow_schema::{DataType, Field, Schema as ArrowSchema};
    use lynxes_core::{
        Direction, EdgeTypeSpec, Optimizer, OptimizerOptions, Pattern, PatternStep, COL_EDGE_SRC,
        COL_NODE_ID, COL_NODE_LABEL,
    };
    use lynxes_plan::{Connector, PartitionStrategy};

    fn labels_array(values: &[&[&str]]) -> ListArray {
        let mut builder = ListBuilder::new(StringBuilder::new());
        for labels in values {
            for label in *labels {
                builder.values().append_value(label);
            }
            builder.append(true);
        }
        builder.finish()
    }

    fn demo_graph() -> GraphFrame {
        let node_schema = Arc::new(ArrowSchema::new(vec![
            Field::new(COL_NODE_ID, DataType::Utf8, false),
            Field::new(
                COL_NODE_LABEL,
                DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
                false,
            ),
            Field::new("age", DataType::Int64, true),
        ]));
        let nodes = NodeFrame::from_record_batch(
            RecordBatch::try_new(
                node_schema,
                vec![
                    Arc::new(StringArray::from(vec!["alice", "bob", "charlie", "acme"]))
                        as ArrayRef,
                    Arc::new(labels_array(&[
                        &["Person"],
                        &["Person"],
                        &["Person"],
                        &["Company"],
                    ])) as ArrayRef,
                    Arc::new(Int64Array::from(vec![Some(30), Some(40), Some(20), None]))
                        as ArrayRef,
                ],
            )
            .unwrap(),
        )
        .unwrap();

        let edge_schema = Arc::new(ArrowSchema::new(vec![
            Field::new(COL_EDGE_SRC, DataType::Utf8, false),
            Field::new(COL_EDGE_DST, DataType::Utf8, false),
            Field::new(COL_EDGE_TYPE, DataType::Utf8, false),
            Field::new(COL_EDGE_DIRECTION, DataType::Int8, false),
            Field::new("weight", DataType::Int64, true),
        ]));
        let edges = EdgeFrame::from_record_batch(
            RecordBatch::try_new(
                edge_schema,
                vec![
                    Arc::new(StringArray::from(vec!["alice", "alice", "bob"])) as ArrayRef,
                    Arc::new(StringArray::from(vec!["bob", "charlie", "acme"])) as ArrayRef,
                    Arc::new(StringArray::from(vec!["KNOWS", "KNOWS", "WORKS_AT"])) as ArrayRef,
                    Arc::new(Int8Array::from(vec![0i8, 0, 0])) as ArrayRef,
                    Arc::new(Int64Array::from(vec![Some(1), Some(2), Some(3)])) as ArrayRef,
                ],
            )
            .unwrap(),
        )
        .unwrap();

        GraphFrame::new(nodes, edges).unwrap()
    }

    fn scan(source: Arc<GraphFrame>) -> LogicalPlan {
        #[derive(Debug)]
        struct DummyConnector;
        impl Connector for DummyConnector {}

        let _ = source;
        LogicalPlan::Scan {
            source: Arc::new(DummyConnector),
            node_columns: None,
            edge_columns: None,
        }
    }

    #[test]
    fn filter_nodes_project_sort_limit_executes() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::Limit {
            input: Box::new(LogicalPlan::Sort {
                input: Box::new(LogicalPlan::ProjectNodes {
                    input: Box::new(LogicalPlan::FilterNodes {
                        input: Box::new(scan(graph.clone())),
                        predicate: Expr::BinaryOp {
                            left: Box::new(Expr::Col {
                                name: "age".to_owned(),
                            }),
                            op: BinaryOp::Gt,
                            right: Box::new(Expr::Literal {
                                value: ScalarValue::Int(25),
                            }),
                        },
                    }),
                    columns: vec!["age".to_owned()],
                }),
                by: "age".to_owned(),
                descending: true,
            }),
            n: 1,
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::Nodes(nodes) = result else {
            panic!("expected node result");
        };

        assert_eq!(nodes.len(), 1);
        assert_eq!(
            nodes.column_names(),
            vec![COL_NODE_ID, COL_NODE_LABEL, "age"]
        );
        assert_eq!(nodes.id_column().value(0), "bob");
    }

    #[test]
    fn filter_edges_project_sort_limit_executes() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::Limit {
            input: Box::new(LogicalPlan::Sort {
                input: Box::new(LogicalPlan::ProjectEdges {
                    input: Box::new(LogicalPlan::FilterEdges {
                        input: Box::new(scan(graph.clone())),
                        predicate: Expr::BinaryOp {
                            left: Box::new(Expr::Col {
                                name: COL_EDGE_TYPE.to_owned(),
                            }),
                            op: BinaryOp::Eq,
                            right: Box::new(Expr::Literal {
                                value: ScalarValue::String("KNOWS".to_owned()),
                            }),
                        },
                    }),
                    columns: vec!["weight".to_owned()],
                }),
                by: "weight".to_owned(),
                descending: true,
            }),
            n: 1,
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::Edges(edges) = result else {
            panic!("expected edge result");
        };

        assert_eq!(edges.len(), 1);
        let weight = edges
            .column("weight")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(weight.value(0), 2);
    }

    #[test]
    fn expand_from_filtered_nodes_returns_traversed_subgraph() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::Expand {
            input: Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: COL_NODE_ID.to_owned(),
                    }),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::String("alice".to_owned()),
                    }),
                },
            }),
            edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
            hops: 1,
            direction: Direction::Out,
            pre_filter: None,
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::Graph(graph) = result else {
            panic!("expected graph result");
        };

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
        assert!(graph.nodes().row_index("alice").is_some());
        assert!(graph.nodes().row_index("bob").is_some());
        assert!(graph.nodes().row_index("charlie").is_some());
    }

    #[test]
    fn traverse_executes_pattern_steps_in_order() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::Traverse {
            input: Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: COL_NODE_ID.to_owned(),
                    }),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::String("alice".to_owned()),
                    }),
                },
            }),
            pattern: vec![
                PatternStep {
                    from_alias: "a".to_owned(),
                    edge_alias: Some("e1".to_owned()),
                    edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                    direction: Direction::Out,
                    to_alias: "b".to_owned(),
                },
                PatternStep {
                    from_alias: "b".to_owned(),
                    edge_alias: Some("e2".to_owned()),
                    edge_type: EdgeTypeSpec::Single("WORKS_AT".to_owned()),
                    direction: Direction::Out,
                    to_alias: "c".to_owned(),
                },
            ],
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::Graph(graph) = result else {
            panic!("expected graph result");
        };

        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.edge_count(), 3);
        assert!(graph.nodes().row_index("alice").is_some());
        assert!(graph.nodes().row_index("bob").is_some());
        assert!(graph.nodes().row_index("charlie").is_some());
        assert!(graph.nodes().row_index("acme").is_some());
    }

    #[test]
    fn aggregate_neighbors_count_appends_node_column() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::AggregateNeighbors {
            input: Box::new(scan(graph.clone())),
            edge_type: "KNOWS".to_owned(),
            agg: AggExpr::Count,
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::Nodes(nodes) = result else {
            panic!("expected node result");
        };

        let counts = nodes
            .column("count")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(counts.value(0), 2);
        assert_eq!(counts.value(1), 0);
        assert_eq!(counts.value(2), 0);
        assert_eq!(counts.value(3), 0);
    }

    #[test]
    fn aggregate_neighbors_mean_can_read_edge_columns() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::AggregateNeighbors {
            input: Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: COL_NODE_ID.to_owned(),
                    }),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::String("alice".to_owned()),
                    }),
                },
            }),
            edge_type: "KNOWS".to_owned(),
            agg: AggExpr::Mean {
                expr: Expr::Col {
                    name: "weight".to_owned(),
                },
            },
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::Nodes(nodes) = result else {
            panic!("expected node result");
        };

        let mean = nodes
            .column("mean")
            .unwrap()
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert_eq!(nodes.len(), 1);
        assert!((mean.value(0) - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn aggregate_neighbors_alias_overrides_output_column_name() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::AggregateNeighbors {
            input: Box::new(scan(graph.clone())),
            edge_type: "KNOWS".to_owned(),
            agg: AggExpr::Alias {
                expr: Box::new(AggExpr::Count),
                name: "friend_count".to_owned(),
            },
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::Nodes(nodes) = result else {
            panic!("expected node result");
        };

        // Column must be named "friend_count", not "count".
        assert!(
            nodes.column("count").is_none(),
            "bare 'count' column should not exist"
        );
        let counts = nodes
            .column("friend_count")
            .expect("alias column 'friend_count' must exist")
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(counts.value(0), 2); // alice knows bob + charlie
    }

    // ── OPT-002: EarlyTermination hint tests ─────────────────────────────────

    /// `LimitAware { n=2 }` wrapping an Expand should stop BFS once the visited
    /// set reaches 2 nodes (the seed "alice" counts as 1 before any hops).
    #[test]
    fn limit_aware_expand_stops_early() {
        // demo_graph: alice→bob, alice→charlie, bob→acme
        // Without a limit all 4 nodes would be reachable from alice in 2 hops.
        // Seed the expansion from alice only (FilterNodes before Expand).
        let graph = Arc::new(demo_graph());
        let alice_only = Box::new(LogicalPlan::FilterNodes {
            input: Box::new(scan(graph.clone())),
            predicate: Expr::BinaryOp {
                left: Box::new(Expr::Col {
                    name: COL_NODE_ID.to_owned(),
                }),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal {
                    value: ScalarValue::String("alice".to_owned()),
                }),
            },
        });
        let hint_plan = LogicalPlan::Hint {
            hint: ExecutionHint::LimitAware { n: 2 },
            input: Box::new(LogicalPlan::Expand {
                input: alice_only,
                edge_type: EdgeTypeSpec::Any,
                hops: 3,
                direction: Direction::Out,
                pre_filter: None,
            }),
        };

        let result = execute(&hint_plan, graph).unwrap();
        let ExecutionValue::Graph(g) = result else {
            panic!("expected graph result");
        };
        // visited starts at {alice}; first out-neighbor admission makes len = 2,
        // triggering break 'expand immediately.
        assert_eq!(
            g.node_count(),
            2,
            "LimitAware(2) with 1-node seed must return exactly 2 nodes"
        );
        assert!(
            g.nodes().row_index("alice").is_some(),
            "alice must be in result"
        );
    }

    /// `LimitAware { n }` where `n` exceeds total reachable nodes should return
    /// the same graph as an unrestricted Expand.
    #[test]
    fn limit_aware_expand_no_stop_when_n_exceeds_reachable() {
        let graph = Arc::new(demo_graph());

        // Seed from alice so both plans share the same 1-node frontier.
        let alice_filter = || {
            Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: COL_NODE_ID.to_owned(),
                    }),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::String("alice".to_owned()),
                    }),
                },
            })
        };

        let unrestricted = LogicalPlan::Expand {
            input: alice_filter(),
            edge_type: EdgeTypeSpec::Any,
            hops: 3,
            direction: Direction::Out,
            pre_filter: None,
        };
        let hint_plan = LogicalPlan::Hint {
            hint: ExecutionHint::LimitAware { n: 100 },
            input: Box::new(LogicalPlan::Expand {
                input: alice_filter(),
                edge_type: EdgeTypeSpec::Any,
                hops: 3,
                direction: Direction::Out,
                pre_filter: None,
            }),
        };

        let base = match execute(&unrestricted, graph.clone()).unwrap() {
            ExecutionValue::Graph(g) => g,
            _ => panic!("expected graph"),
        };
        let limited = match execute(&hint_plan, graph).unwrap() {
            ExecutionValue::Graph(g) => g,
            _ => panic!("expected graph"),
        };

        assert_eq!(base.node_count(), limited.node_count());
        assert_eq!(base.edge_count(), limited.edge_count());
    }

    /// `LimitAware { n=2 }` wrapping a Traverse should stop before completing
    /// all pattern steps once the visited set grows past `n`.
    #[test]
    fn limit_aware_traverse_stops_early() {
        use lynxes_core::Direction as Dir;

        // demo_graph seeded from alice only.
        // Pattern: alice -KNOWS-> {bob, charlie} -WORKS_AT-> {acme}
        // Without limit: visited = {alice, bob, charlie, acme} (4 nodes).
        // With LimitAware n=2: after step 1 visited = {alice, bob, charlie} (3 ≥ 2),
        // so the step-level break fires and acme is never reached.
        let graph = Arc::new(demo_graph());
        let alice_only = Box::new(LogicalPlan::FilterNodes {
            input: Box::new(scan(graph.clone())),
            predicate: Expr::BinaryOp {
                left: Box::new(Expr::Col {
                    name: COL_NODE_ID.to_owned(),
                }),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal {
                    value: ScalarValue::String("alice".to_owned()),
                }),
            },
        });
        let hint_plan = LogicalPlan::Hint {
            hint: ExecutionHint::LimitAware { n: 2 },
            input: Box::new(LogicalPlan::Traverse {
                input: alice_only,
                pattern: vec![
                    PatternStep {
                        from_alias: "a".into(),
                        edge_alias: None,
                        edge_type: EdgeTypeSpec::Single("KNOWS".into()),
                        direction: Dir::Out,
                        to_alias: "b".into(),
                    },
                    PatternStep {
                        from_alias: "b".into(),
                        edge_alias: None,
                        edge_type: EdgeTypeSpec::Single("WORKS_AT".into()),
                        direction: Dir::Out,
                        to_alias: "c".into(),
                    },
                ],
            }),
        };

        let result = execute(&hint_plan, graph).unwrap();
        let ExecutionValue::Graph(g) = result else {
            panic!("expected graph result");
        };
        // Step-level stop fires after step 1; acme (reachable only at step 2)
        // must not appear in the result.
        assert!(
            g.nodes().row_index("acme").is_none(),
            "acme must not be reached under LimitAware(2)"
        );
        assert!(
            g.nodes().row_index("alice").is_some(),
            "alice must be in result"
        );
    }

    /// `TopK { n=2 }` over a Sort should return only the top 2 rows in correct order.
    #[test]
    fn top_k_sort_returns_k_rows_in_correct_order() {
        // demo_graph nodes have ages: alice=30, bob=40, charlie=20, acme=null
        // Sort descending by age: bob(40) > alice(30) > charlie(20) > acme(null)
        let graph = Arc::new(demo_graph());
        let hint_plan = LogicalPlan::Hint {
            hint: ExecutionHint::TopK { n: 2 },
            input: Box::new(LogicalPlan::Sort {
                input: Box::new(LogicalPlan::FilterNodes {
                    input: Box::new(scan(graph.clone())),
                    // Keep only nodes with a non-null age.
                    predicate: Expr::BinaryOp {
                        left: Box::new(Expr::Col {
                            name: "age".to_owned(),
                        }),
                        op: BinaryOp::Gt,
                        right: Box::new(Expr::Literal {
                            value: ScalarValue::Int(0),
                        }),
                    },
                }),
                by: "age".to_owned(),
                descending: true,
            }),
        };

        let result = execute(&hint_plan, graph).unwrap();
        let ExecutionValue::Nodes(nodes) = result else {
            panic!("expected node result");
        };

        assert_eq!(nodes.len(), 2, "TopK(2) must return exactly 2 rows");
        // Row 0 should be the highest age (bob=40), row 1 the next (alice=30).
        assert_eq!(nodes.id_column().value(0), "bob");
        assert_eq!(nodes.id_column().value(1), "alice");
    }

    /// `TopK { n }` where `n >= num_rows` must behave identically to a full Sort.
    #[test]
    fn top_k_sort_full_result_when_k_exceeds_rows() {
        let graph = Arc::new(demo_graph());
        let sort_plan = LogicalPlan::Sort {
            input: Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: "age".to_owned(),
                    }),
                    op: BinaryOp::Gt,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::Int(0),
                    }),
                },
            }),
            by: "age".to_owned(),
            descending: false,
        };
        let hint_plan = LogicalPlan::Hint {
            hint: ExecutionHint::TopK { n: 999 },
            input: Box::new(sort_plan.clone()),
        };

        let base = match execute(&sort_plan, graph.clone()).unwrap() {
            ExecutionValue::Nodes(n) => n,
            _ => panic!("expected nodes"),
        };
        let top = match execute(&hint_plan, graph).unwrap() {
            ExecutionValue::Nodes(n) => n,
            _ => panic!("expected nodes"),
        };

        assert_eq!(base.len(), top.len());
        for i in 0..base.len() {
            assert_eq!(base.id_column().value(i), top.id_column().value(i));
        }
    }

    /// `PartitionParallel` hint is a no-op at this stage — the plan beneath
    /// it should execute normally.
    #[test]
    fn partition_parallel_hint_falls_through() {
        let graph = Arc::new(demo_graph());
        let hint_plan = LogicalPlan::Hint {
            hint: ExecutionHint::PartitionParallel {
                strategy: PartitionStrategy::ExpandFrontier,
            },
            input: Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: COL_NODE_ID.to_owned(),
                    }),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::String("alice".to_owned()),
                    }),
                },
            }),
        };

        let result = execute(&hint_plan, graph).unwrap();
        let ExecutionValue::Nodes(nodes) = result else {
            panic!("expected node result");
        };
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes.id_column().value(0), "alice");
    }

    // ── OPT-003: PartitionParallel executor tests ─────────────────────────────

    /// `PartitionParallel` + `Expand` must produce the same result as a serial
    /// `Expand` — correctness is the baseline requirement for any parallelism.
    #[test]
    fn partition_parallel_expand_matches_serial() {
        let graph = Arc::new(demo_graph());

        let serial_plan = LogicalPlan::Expand {
            input: Box::new(scan(graph.clone())),
            edge_type: EdgeTypeSpec::Any,
            hops: 2,
            direction: Direction::Out,
            pre_filter: None,
        };
        let parallel_plan = LogicalPlan::Hint {
            hint: ExecutionHint::PartitionParallel {
                strategy: PartitionStrategy::ExpandFrontier,
            },
            input: Box::new(LogicalPlan::Expand {
                input: Box::new(scan(graph.clone())),
                edge_type: EdgeTypeSpec::Any,
                hops: 2,
                direction: Direction::Out,
                pre_filter: None,
            }),
        };

        let serial_g = match execute(&serial_plan, graph.clone()).unwrap() {
            ExecutionValue::Graph(g) => g,
            _ => panic!("expected graph"),
        };
        let parallel_g = match execute(&parallel_plan, graph).unwrap() {
            ExecutionValue::Graph(g) => g,
            _ => panic!("expected graph"),
        };

        // Same set of node IDs.
        let mut serial_ids: Vec<&str> = serial_g.nodes().id_column().iter().flatten().collect();
        let mut parallel_ids: Vec<&str> = parallel_g.nodes().id_column().iter().flatten().collect();
        serial_ids.sort_unstable();
        parallel_ids.sort_unstable();
        assert_eq!(serial_ids, parallel_ids, "node sets must match");

        // Same edge count (parallel may deduplicate the same way serial does).
        assert_eq!(
            serial_g.edge_count(),
            parallel_g.edge_count(),
            "edge counts must match"
        );
    }

    /// When the frontier is tiny (below the 2 × n_threads threshold) the
    /// parallel path falls back to serial — the result must still be correct.
    #[test]
    fn partition_parallel_expand_serial_fallback_is_correct() {
        // Single-node frontier: always below the threshold.
        let graph = Arc::new(demo_graph());
        let alice_only = Box::new(LogicalPlan::FilterNodes {
            input: Box::new(scan(graph.clone())),
            predicate: Expr::BinaryOp {
                left: Box::new(Expr::Col {
                    name: COL_NODE_ID.to_owned(),
                }),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal {
                    value: ScalarValue::String("alice".to_owned()),
                }),
            },
        });

        let plan = LogicalPlan::Hint {
            hint: ExecutionHint::PartitionParallel {
                strategy: PartitionStrategy::ExpandFrontier,
            },
            input: Box::new(LogicalPlan::Expand {
                input: alice_only,
                edge_type: EdgeTypeSpec::Any,
                hops: 2,
                direction: Direction::Out,
                pre_filter: None,
            }),
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::Graph(g) = result else {
            panic!("expected graph result");
        };

        // alice→bob→acme and alice→charlie; all 4 nodes reachable in 2 hops.
        assert_eq!(g.node_count(), 4);
        for id in ["alice", "bob", "charlie", "acme"] {
            assert!(
                g.nodes().row_index(id).is_some(),
                "{id} missing from result"
            );
        }
    }

    /// Expanding with `PartitionParallel` and a type filter must honour the
    /// edge-type filter — same as serial.  We seed from alice only so that the
    /// filter is exercised: alice→{bob,charlie} are KNOWS, bob→acme is WORKS_AT.
    #[test]
    fn partition_parallel_expand_respects_edge_type_filter() {
        let graph = Arc::new(demo_graph());

        let alice_seed = || {
            Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: COL_NODE_ID.to_owned(),
                    }),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::String("alice".to_owned()),
                    }),
                },
            })
        };

        let serial_plan = LogicalPlan::Expand {
            input: alice_seed(),
            edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
            hops: 2,
            direction: Direction::Out,
            pre_filter: None,
        };
        let parallel_plan = LogicalPlan::Hint {
            hint: ExecutionHint::PartitionParallel {
                strategy: PartitionStrategy::ExpandFrontier,
            },
            input: Box::new(LogicalPlan::Expand {
                input: alice_seed(),
                edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                hops: 2,
                direction: Direction::Out,
                pre_filter: None,
            }),
        };

        let serial_g = match execute(&serial_plan, graph.clone()).unwrap() {
            ExecutionValue::Graph(g) => g,
            _ => panic!(),
        };
        let parallel_g = match execute(&parallel_plan, graph).unwrap() {
            ExecutionValue::Graph(g) => g,
            _ => panic!(),
        };

        // alice -KNOWS-> bob, charlie; bob -KNOWS-> none; acme unreachable via KNOWS.
        let mut s: Vec<&str> = serial_g.nodes().id_column().iter().flatten().collect();
        let mut p: Vec<&str> = parallel_g.nodes().id_column().iter().flatten().collect();
        s.sort_unstable();
        p.sort_unstable();
        assert_eq!(
            s, p,
            "edge-type-filtered results must match between serial and parallel"
        );
        assert!(s.contains(&"alice") && s.contains(&"bob") && s.contains(&"charlie"));
        assert!(
            !s.contains(&"acme"),
            "acme must not appear (no KNOWS edge from alice's neighbourhood)"
        );
    }

    #[test]
    fn pattern_binding_row_accepts_fresh_aliases() {
        let mut row = PatternBindingRow::new();

        bind_pattern_alias(&mut row, "a", 0).unwrap();
        bind_pattern_alias(&mut row, "b", 3).unwrap();

        assert_eq!(row.get("a"), Some(&0));
        assert_eq!(row.get("b"), Some(&3));
    }

    #[test]
    fn pattern_binding_row_allows_same_alias_same_value() {
        let mut row = PatternBindingRow::new();

        bind_pattern_alias(&mut row, "a", 2).unwrap();
        bind_pattern_alias(&mut row, "a", 2).unwrap();

        assert_eq!(row.len(), 1);
        assert_eq!(row.get("a"), Some(&2));
    }

    #[test]
    fn pattern_binding_row_rejects_conflicting_alias_rebind() {
        let mut row = PatternBindingRow::new();
        bind_pattern_alias(&mut row, "a", 1).unwrap();

        let err = bind_pattern_alias(&mut row, "a", 4).unwrap_err();

        assert!(
            matches!(err, GFError::InvalidConfig { message } if message.contains("pattern alias 'a'"))
        );
        assert_eq!(row.get("a"), Some(&1));
    }

    #[test]
    fn pattern_bindings_is_vector_of_binding_rows() {
        let mut bindings: PatternBindings = Vec::new();
        let mut row = PatternBindingRow::new();
        bind_pattern_alias(&mut row, "seed", 7).unwrap();
        bindings.push(row);

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].get("seed"), Some(&7));
    }

    #[test]
    fn execute_pattern_step_builds_outbound_typed_bindings() {
        let graph = demo_graph();
        let step = PatternStep {
            from_alias: "a".to_owned(),
            edge_alias: Some("e".to_owned()),
            edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
            direction: Direction::Out,
            to_alias: "b".to_owned(),
        };

        let mut seed = PatternBindingRow::new();
        bind_pattern_alias(&mut seed, "a", 0).unwrap();
        let bindings = execute_pattern_step(&graph, &step, &vec![seed]).unwrap();

        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].get("a"), Some(&0));
        assert_eq!(bindings[0].get("e"), Some(&0));
        assert_eq!(bindings[0].get("b"), Some(&1));
        assert_eq!(bindings[1].get("a"), Some(&0));
        assert_eq!(bindings[1].get("e"), Some(&1));
        assert_eq!(bindings[1].get("b"), Some(&2));
    }

    #[test]
    fn execute_pattern_step_supports_inbound_typed_bindings() {
        let graph = demo_graph();
        let step = PatternStep {
            from_alias: "c".to_owned(),
            edge_alias: Some("e".to_owned()),
            edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
            direction: Direction::In,
            to_alias: "a".to_owned(),
        };

        let mut seed = PatternBindingRow::new();
        bind_pattern_alias(&mut seed, "c", 2).unwrap();
        let bindings = execute_pattern_step(&graph, &step, &vec![seed]).unwrap();

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].get("c"), Some(&2));
        assert_eq!(bindings[0].get("e"), Some(&1));
        assert_eq!(bindings[0].get("a"), Some(&0));
    }

    #[test]
    fn execute_pattern_step_drops_rows_on_alias_conflict() {
        let graph = demo_graph();
        let step = PatternStep {
            from_alias: "a".to_owned(),
            edge_alias: None,
            edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
            direction: Direction::Out,
            to_alias: "b".to_owned(),
        };

        let mut seed = PatternBindingRow::new();
        bind_pattern_alias(&mut seed, "a", 0).unwrap();
        bind_pattern_alias(&mut seed, "b", 9).unwrap();
        let bindings = execute_pattern_step(&graph, &step, &vec![seed]).unwrap();

        assert!(bindings.is_empty());
    }

    #[test]
    fn execute_pattern_steps_chains_two_hops_across_aliases() {
        let graph = demo_graph();
        let steps = vec![
            PatternStep {
                from_alias: "a".to_owned(),
                edge_alias: Some("e1".to_owned()),
                edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                direction: Direction::Out,
                to_alias: "b".to_owned(),
            },
            PatternStep {
                from_alias: "b".to_owned(),
                edge_alias: Some("e2".to_owned()),
                edge_type: EdgeTypeSpec::Single("WORKS_AT".to_owned()),
                direction: Direction::Out,
                to_alias: "c".to_owned(),
            },
        ];

        let mut seed = PatternBindingRow::new();
        bind_pattern_alias(&mut seed, "a", 0).unwrap();
        let bindings = execute_pattern_steps(&graph, &steps, &vec![seed]).unwrap();

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].get("a"), Some(&0));
        assert_eq!(bindings[0].get("b"), Some(&1));
        assert_eq!(bindings[0].get("c"), Some(&3));
        assert_eq!(bindings[0].get("e1"), Some(&0));
        assert_eq!(bindings[0].get("e2"), Some(&2));
    }

    #[test]
    fn execute_pattern_steps_returns_empty_when_later_hop_has_no_match() {
        let graph = demo_graph();
        let steps = vec![
            PatternStep {
                from_alias: "a".to_owned(),
                edge_alias: None,
                edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                direction: Direction::Out,
                to_alias: "b".to_owned(),
            },
            PatternStep {
                from_alias: "b".to_owned(),
                edge_alias: None,
                edge_type: EdgeTypeSpec::Single("WORKS_AT".to_owned()),
                direction: Direction::Out,
                to_alias: "c".to_owned(),
            },
            PatternStep {
                from_alias: "c".to_owned(),
                edge_alias: None,
                edge_type: EdgeTypeSpec::Single("WORKS_AT".to_owned()),
                direction: Direction::Out,
                to_alias: "d".to_owned(),
            },
        ];

        let mut seed = PatternBindingRow::new();
        bind_pattern_alias(&mut seed, "a", 0).unwrap();
        let bindings = execute_pattern_steps(&graph, &steps, &vec![seed]).unwrap();

        assert!(bindings.is_empty());
    }

    #[test]
    fn execute_pattern_steps_preserves_aliases_from_previous_hops() {
        let graph = demo_graph();
        let steps = vec![
            PatternStep {
                from_alias: "a".to_owned(),
                edge_alias: None,
                edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                direction: Direction::Out,
                to_alias: "b".to_owned(),
            },
            PatternStep {
                from_alias: "b".to_owned(),
                edge_alias: None,
                edge_type: EdgeTypeSpec::Single("WORKS_AT".to_owned()),
                direction: Direction::Out,
                to_alias: "c".to_owned(),
            },
        ];

        let mut seed = PatternBindingRow::new();
        bind_pattern_alias(&mut seed, "seed", 42).unwrap();
        bind_pattern_alias(&mut seed, "a", 0).unwrap();
        let bindings = execute_pattern_steps(&graph, &steps, &vec![seed]).unwrap();

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].get("seed"), Some(&42));
        assert_eq!(bindings[0].get("a"), Some(&0));
        assert_eq!(bindings[0].get("b"), Some(&1));
        assert_eq!(bindings[0].get("c"), Some(&3));
    }

    #[test]
    fn apply_pattern_where_filters_bindings_by_node_alias_field() {
        let graph = demo_graph();
        let steps = vec![PatternStep {
            from_alias: "a".to_owned(),
            edge_alias: None,
            edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
            direction: Direction::Out,
            to_alias: "b".to_owned(),
        }];

        let mut seed = PatternBindingRow::new();
        bind_pattern_alias(&mut seed, "a", 0).unwrap();
        let bindings = execute_pattern_steps(&graph, &steps, &vec![seed]).unwrap();
        let predicate = Expr::BinaryOp {
            left: Box::new(Expr::PatternCol {
                alias: "b".to_owned(),
                field: "age".to_owned(),
            }),
            op: BinaryOp::Gt,
            right: Box::new(Expr::Literal {
                value: ScalarValue::Int(30),
            }),
        };

        let filtered = apply_pattern_where(&graph, &bindings, Some(&predicate)).unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].get("a"), Some(&0));
        assert_eq!(filtered[0].get("b"), Some(&1));
    }

    #[test]
    fn apply_pattern_where_filters_bindings_by_edge_alias_field() {
        let graph = demo_graph();
        let steps = vec![
            PatternStep {
                from_alias: "a".to_owned(),
                edge_alias: Some("e1".to_owned()),
                edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                direction: Direction::Out,
                to_alias: "b".to_owned(),
            },
            PatternStep {
                from_alias: "b".to_owned(),
                edge_alias: Some("e2".to_owned()),
                edge_type: EdgeTypeSpec::Single("WORKS_AT".to_owned()),
                direction: Direction::Out,
                to_alias: "c".to_owned(),
            },
        ];

        let mut seed = PatternBindingRow::new();
        bind_pattern_alias(&mut seed, "a", 0).unwrap();
        let bindings = execute_pattern_steps(&graph, &steps, &vec![seed]).unwrap();
        let predicate = Expr::BinaryOp {
            left: Box::new(Expr::PatternCol {
                alias: "e2".to_owned(),
                field: COL_EDGE_TYPE.to_owned(),
            }),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal {
                value: ScalarValue::String("WORKS_AT".to_owned()),
            }),
        };

        let filtered = apply_pattern_where(&graph, &bindings, Some(&predicate)).unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].get("e2"), Some(&2));
        assert_eq!(filtered[0].get("c"), Some(&3));
    }

    #[test]
    fn apply_pattern_where_returns_all_bindings_when_predicate_is_none() {
        let graph = demo_graph();
        let steps = vec![PatternStep {
            from_alias: "a".to_owned(),
            edge_alias: None,
            edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
            direction: Direction::Out,
            to_alias: "b".to_owned(),
        }];

        let mut seed = PatternBindingRow::new();
        bind_pattern_alias(&mut seed, "a", 0).unwrap();
        let bindings = execute_pattern_steps(&graph, &steps, &vec![seed]).unwrap();

        let filtered = apply_pattern_where(&graph, &bindings, None).unwrap();

        assert_eq!(filtered, bindings);
    }

    #[test]
    fn materialize_pattern_bindings_emits_alias_prefixed_columns_in_pattern_order() {
        let graph = demo_graph();
        let pattern = vec![
            PatternStep {
                from_alias: "a".to_owned(),
                edge_alias: Some("e1".to_owned()),
                edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                direction: Direction::Out,
                to_alias: "b".to_owned(),
            },
            PatternStep {
                from_alias: "b".to_owned(),
                edge_alias: Some("e2".to_owned()),
                edge_type: EdgeTypeSpec::Single("WORKS_AT".to_owned()),
                direction: Direction::Out,
                to_alias: "c".to_owned(),
            },
        ];

        let mut seed = PatternBindingRow::new();
        bind_pattern_alias(&mut seed, "a", 0).unwrap();
        let bindings = execute_pattern_steps(&graph, &pattern, &vec![seed]).unwrap();
        let batch = materialize_pattern_bindings(&graph, &pattern, &bindings).unwrap();

        let schema = batch.schema();
        let column_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert_eq!(
            column_names,
            vec![
                "a._id",
                "a._label",
                "a.age",
                "e1._src",
                "e1._dst",
                "e1._type",
                "e1._direction",
                "e1.weight",
                "b._id",
                "b._label",
                "b.age",
                "e2._src",
                "e2._dst",
                "e2._type",
                "e2._direction",
                "e2.weight",
                "c._id",
                "c._label",
                "c.age",
            ]
        );
        assert_eq!(batch.num_rows(), 1);

        let a_id = batch
            .column_by_name("a._id")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let b_id = batch
            .column_by_name("b._id")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let c_id = batch
            .column_by_name("c._id")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let e1_type = batch
            .column_by_name("e1._type")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let e2_weight = batch
            .column_by_name("e2.weight")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();

        assert_eq!(a_id.value(0), "alice");
        assert_eq!(b_id.value(0), "bob");
        assert_eq!(c_id.value(0), "acme");
        assert_eq!(e1_type.value(0), "KNOWS");
        assert_eq!(e2_weight.value(0), 3);
    }

    #[test]
    fn materialize_pattern_bindings_supports_empty_result_with_full_schema() {
        let graph = demo_graph();
        let pattern = vec![
            PatternStep {
                from_alias: "a".to_owned(),
                edge_alias: Some("e1".to_owned()),
                edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                direction: Direction::Out,
                to_alias: "b".to_owned(),
            },
            PatternStep {
                from_alias: "b".to_owned(),
                edge_alias: Some("e2".to_owned()),
                edge_type: EdgeTypeSpec::Single("WORKS_AT".to_owned()),
                direction: Direction::Out,
                to_alias: "c".to_owned(),
            },
        ];

        let empty: PatternBindings = Vec::new();
        let batch = materialize_pattern_bindings(&graph, &pattern, &empty).unwrap();

        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 19);
        assert_eq!(batch.schema().field(0).name(), "a._id");
        assert_eq!(batch.schema().field(18).name(), "c.age");
    }

    #[test]
    fn materialize_pattern_bindings_rejects_alias_kind_conflicts() {
        let graph = demo_graph();
        let pattern = vec![PatternStep {
            from_alias: "x".to_owned(),
            edge_alias: Some("x".to_owned()),
            edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
            direction: Direction::Out,
            to_alias: "b".to_owned(),
        }];

        let empty: PatternBindings = Vec::new();
        let err = materialize_pattern_bindings(&graph, &pattern, &empty).unwrap_err();

        assert!(matches!(err, GFError::InvalidConfig { message } if message.contains("used as both")));
    }

    #[test]
    fn pattern_match_executes_collect_path_for_two_hop_pattern() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::PatternMatch {
            input: Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: COL_NODE_ID.to_owned(),
                    }),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::String("alice".to_owned()),
                    }),
                },
            }),
            pattern: Pattern::new(vec![
                PatternStep {
                    from_alias: "a".to_owned(),
                    edge_alias: Some("e1".to_owned()),
                    edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                    direction: Direction::Out,
                    to_alias: "b".to_owned(),
                },
                PatternStep {
                    from_alias: "b".to_owned(),
                    edge_alias: Some("e2".to_owned()),
                    edge_type: EdgeTypeSpec::Single("WORKS_AT".to_owned()),
                    direction: Direction::Out,
                    to_alias: "c".to_owned(),
                },
            ]),
            where_: None,
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::PatternRows(batch) = result else {
            panic!("expected pattern-row result");
        };

        assert_eq!(batch.num_rows(), 1);
        let schema = batch.schema();
        assert!(schema.column_with_name("a._id").is_some());
        assert!(schema.column_with_name("b._id").is_some());
        assert!(schema.column_with_name("c._id").is_some());
        assert!(schema.column_with_name("e1._type").is_some());
        assert!(schema.column_with_name("e2.weight").is_some());

        let a_ids = string_array(&batch, "a._id").unwrap();
        let b_ids = string_array(&batch, "b._id").unwrap();
        let c_ids = string_array(&batch, "c._id").unwrap();
        let e1_types = string_array(&batch, "e1._type").unwrap();
        let e2_weight = batch
            .column_by_name("e2.weight")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();

        assert_eq!(a_ids.value(0), "alice");
        assert_eq!(b_ids.value(0), "bob");
        assert_eq!(c_ids.value(0), "acme");
        assert_eq!(e1_types.value(0), "KNOWS");
        assert_eq!(e2_weight.value(0), 3);
    }

    #[test]
    fn pattern_match_executes_with_where_filter() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::PatternMatch {
            input: Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: COL_NODE_ID.to_owned(),
                    }),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::String("alice".to_owned()),
                    }),
                },
            }),
            pattern: Pattern::new(vec![PatternStep {
                from_alias: "a".to_owned(),
                edge_alias: Some("e".to_owned()),
                edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                direction: Direction::Out,
                to_alias: "b".to_owned(),
            }]),
            where_: Some(Expr::BinaryOp {
                left: Box::new(Expr::PatternCol {
                    alias: "b".to_owned(),
                    field: "age".to_owned(),
                }),
                op: BinaryOp::Gt,
                right: Box::new(Expr::Literal {
                    value: ScalarValue::Int(30),
                }),
            }),
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::PatternRows(batch) = result else {
            panic!("expected pattern-row result");
        };

        assert_eq!(batch.num_rows(), 1);
        let b_ids = string_array(&batch, "b._id").unwrap();
        assert_eq!(b_ids.value(0), "bob");
    }

    #[test]
    fn kg_typed_one_step_pattern_executes() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::PatternMatch {
            input: Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: COL_NODE_ID.to_owned(),
                    }),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::String("alice".to_owned()),
                    }),
                },
            }),
            pattern: Pattern::new(vec![PatternStep {
                from_alias: "a".to_owned(),
                edge_alias: Some("e".to_owned()),
                edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                direction: Direction::Out,
                to_alias: "b".to_owned(),
            }]),
            where_: None,
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::PatternRows(batch) = result else {
            panic!("expected pattern-row result");
        };

        assert_eq!(batch.num_rows(), 2);
        let a_ids = string_array(&batch, "a._id").unwrap();
        let b_ids = string_array(&batch, "b._id").unwrap();
        let e_types = string_array(&batch, "e._type").unwrap();
        assert_eq!(a_ids.value(0), "alice");
        assert_eq!(a_ids.value(1), "alice");
        assert_eq!(b_ids.value(0), "bob");
        assert_eq!(b_ids.value(1), "charlie");
        assert_eq!(e_types.value(0), "KNOWS");
        assert_eq!(e_types.value(1), "KNOWS");
    }

    #[test]
    fn kg_two_hop_multi_step_pattern_executes() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::PatternMatch {
            input: Box::new(LogicalPlan::FilterNodes {
                input: Box::new(scan(graph.clone())),
                predicate: Expr::BinaryOp {
                    left: Box::new(Expr::Col {
                        name: COL_NODE_ID.to_owned(),
                    }),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::String("alice".to_owned()),
                    }),
                },
            }),
            pattern: Pattern::new(vec![
                PatternStep {
                    from_alias: "a".to_owned(),
                    edge_alias: Some("e1".to_owned()),
                    edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                    direction: Direction::Out,
                    to_alias: "b".to_owned(),
                },
                PatternStep {
                    from_alias: "b".to_owned(),
                    edge_alias: Some("e2".to_owned()),
                    edge_type: EdgeTypeSpec::Single("WORKS_AT".to_owned()),
                    direction: Direction::Out,
                    to_alias: "c".to_owned(),
                },
            ]),
            where_: None,
        };

        let result = execute(&plan, graph).unwrap();
        let ExecutionValue::PatternRows(batch) = result else {
            panic!("expected pattern-row result");
        };

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(string_array(&batch, "a._id").unwrap().value(0), "alice");
        assert_eq!(string_array(&batch, "b._id").unwrap().value(0), "bob");
        assert_eq!(string_array(&batch, "c._id").unwrap().value(0), "acme");
    }

    #[test]
    fn kg_pattern_expansion_pushdown_preserves_result_set() {
        let graph = Arc::new(demo_graph());
        let plan = LogicalPlan::PatternMatch {
            input: Box::new(scan(graph.clone())),
            pattern: Pattern::new(vec![PatternStep {
                from_alias: "a".to_owned(),
                edge_alias: Some("e".to_owned()),
                edge_type: EdgeTypeSpec::Single("KNOWS".to_owned()),
                direction: Direction::Out,
                to_alias: "b".to_owned(),
            }]),
            where_: Some(Expr::And {
                left: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::PatternCol {
                        alias: "a".to_owned(),
                        field: "age".to_owned(),
                    }),
                    op: BinaryOp::Gt,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::Int(25),
                    }),
                }),
                right: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::PatternCol {
                        alias: "b".to_owned(),
                        field: "age".to_owned(),
                    }),
                    op: BinaryOp::Gt,
                    right: Box::new(Expr::Literal {
                        value: ScalarValue::Int(30),
                    }),
                }),
            }),
        };

        let baseline_plan = Optimizer::new(OptimizerOptions {
            pattern_expansion: false,
            ..OptimizerOptions::default()
        })
        .run(plan.clone());
        let optimized_plan = Optimizer::default().run(plan);

        let baseline = execute(&baseline_plan, graph.clone()).unwrap();
        let optimized = execute(&optimized_plan, graph).unwrap();

        let ExecutionValue::PatternRows(baseline_batch) = baseline else {
            panic!("expected pattern-row result");
        };
        let ExecutionValue::PatternRows(optimized_batch) = optimized else {
            panic!("expected pattern-row result");
        };

        assert_eq!(baseline_batch.schema(), optimized_batch.schema());
        assert_eq!(baseline_batch.num_rows(), optimized_batch.num_rows());
        assert_eq!(baseline_batch.num_columns(), optimized_batch.num_columns());
        for idx in 0..baseline_batch.num_columns() {
            assert_eq!(
                format!("{:?}", baseline_batch.column(idx)),
                format!("{:?}", optimized_batch.column(idx))
            );
        }
    }
}
