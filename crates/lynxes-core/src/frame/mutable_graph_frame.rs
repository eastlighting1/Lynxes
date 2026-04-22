//! Mutable graph wrapper skeleton.
//!
//! The concrete mutation architecture lands across the `MUT-*` tasks. On native
//! targets we fix the ownership model around `ArcSwap`, base CSR state, and
//! delta edge buffers before any public mutation methods are exposed. On wasm
//! we keep only placeholder type names for compile compatibility.

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    };

    use arc_swap::ArcSwap;
    use arrow_array::{
        new_null_array, Array, ArrayRef, BooleanArray, Int8Array, RecordBatch, StringArray,
    };
    use hashbrown::HashMap;
    use rayon::prelude::*;

    use super::super::{graph_frame::GraphFrame, CsrIndex, EdgeFrame, NodeFrame};
    use crate::{
        Direction, GFError, Result, COL_EDGE_DIRECTION, COL_EDGE_DST, COL_EDGE_SRC, COL_EDGE_TYPE,
    };

    #[cfg(test)]
    const FLUSH_THRESHOLD: usize = 2;
    #[cfg(not(test))]
    const FLUSH_THRESHOLD: usize = 1024;

    /// Append-only edge deltas that have not yet been merged into the base CSR.
    ///
    /// `pending` stores raw `(src_idx, dst_idx)` pairs for the cheapest
    /// possible edge append path. `frozen` stores compacted mini-CSR chunks
    /// that are ready to participate in read paths before a full `compact()`
    /// rebuild happens.
    pub struct DeltaEdges {
        pub(crate) pending: Mutex<Vec<(u32, u32)>>,
        pub(crate) frozen: RwLock<Vec<Arc<CsrIndex>>>,
    }

    impl DeltaEdges {
        pub(crate) fn new() -> Self {
            Self {
                pending: Mutex::new(Vec::new()),
                frozen: RwLock::new(Vec::new()),
            }
        }
    }

    impl Default for DeltaEdges {
        fn default() -> Self {
            Self::new()
        }
    }

    impl std::fmt::Debug for DeltaEdges {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let pending_len = self
                .pending
                .lock()
                .map(|pending| pending.len())
                .unwrap_or(0);
            let frozen_len = self.frozen.read().map(|frozen| frozen.len()).unwrap_or(0);

            f.debug_struct("DeltaEdges")
                .field("pending_len", &pending_len)
                .field("frozen_len", &frozen_len)
                .finish()
        }
    }

    /// Mutable graph state built around lock-free snapshot replacement.
    ///
    /// The stable eager graph stays immutable. Mutation work happens here by
    /// keeping node storage, edge storage, and the base CSR behind `ArcSwap`,
    /// while recently appended edges accumulate in [`DeltaEdges`] until later
    /// tasks add flush/compact/read-path logic.
    #[allow(dead_code)]
    pub struct MutableGraphFrame {
        pub(crate) node_frame: ArcSwap<NodeFrame>,
        pub(crate) base_csr: ArcSwap<CsrIndex>,
        pub(crate) edge_data: ArcSwap<EdgeFrame>,
        pub(crate) base_snapshot_has_stable_edge_rows: AtomicBool,
        pub(crate) delta: DeltaEdges,
        pub(crate) schema: Option<crate::Schema>,
        pub(crate) node_id_to_idx: HashMap<String, u32>,
        pub(crate) node_idx_to_id: Vec<String>,
        pub(crate) edge_node_id_to_idx: HashMap<String, u32>,
        pub(crate) edge_node_idx_to_id: Vec<String>,
        pub(crate) node_tombstones: Vec<bool>,
        pub(crate) edge_tombstones: Vec<bool>,
    }

    impl MutableGraphFrame {
        pub(crate) fn from_parts(
            node_frame: NodeFrame,
            edge_data: EdgeFrame,
            schema: Option<crate::Schema>,
            node_id_to_idx: HashMap<String, u32>,
            node_idx_to_id: Vec<String>,
            edge_node_id_to_idx: HashMap<String, u32>,
            edge_node_idx_to_id: Vec<String>,
        ) -> Self {
            let node_count = node_frame.len();
            let edge_count = edge_data.len();
            let base_csr = edge_data.out_csr_arc();

            Self {
                node_frame: ArcSwap::from_pointee(node_frame),
                base_csr: ArcSwap::from(base_csr),
                edge_data: ArcSwap::from_pointee(edge_data),
                base_snapshot_has_stable_edge_rows: AtomicBool::new(true),
                delta: DeltaEdges::new(),
                schema,
                node_id_to_idx,
                node_idx_to_id,
                edge_node_id_to_idx,
                edge_node_idx_to_id,
                node_tombstones: vec![true; node_count],
                edge_tombstones: vec![true; edge_count],
            }
        }

        pub fn from_graph_frame(graph: GraphFrame) -> Self {
            let (
                node_frame,
                edge_data,
                schema,
                node_id_to_idx,
                node_idx_to_id,
                edge_node_id_to_idx,
                edge_node_idx_to_id,
            ) = graph.into_mutable_parts();

            Self::from_parts(
                node_frame,
                edge_data,
                schema,
                node_id_to_idx,
                node_idx_to_id,
                edge_node_id_to_idx,
                edge_node_idx_to_id,
            )
        }

        /// Appends one node row to the mutable graph.
        ///
        /// This is a convenience wrapper over [`Self::add_nodes_batch`]. Repeated
        /// single-row appends rebuild and republish the node frame each time, so
        /// large inserts should prefer the batched API.
        pub fn add_node(&mut self, node: NodeFrame) -> Result<()> {
            if node.len() != 1 {
                return Err(GFError::InvalidConfig {
                    message: "add_node requires exactly one row".to_owned(),
                });
            }

            self.add_nodes_batch(node)
        }

        /// Appends a batch of nodes to the mutable graph and republishes the
        /// node-frame snapshot.
        ///
        /// The append path concatenates the current live node set with the new
        /// batch, then swaps in the rebuilt `NodeFrame` through `ArcSwap`.
        pub fn add_nodes_batch(&mut self, nodes: NodeFrame) -> Result<()> {
            if nodes.is_empty() {
                return Ok(());
            }

            let current_live = self.current_live_node_frame()?;
            let merged = NodeFrame::concat(&[&current_live, &nodes])?;

            self.replace_node_frame(merged);
            Ok(())
        }

        /// Updates one live node by tombstoning the old row and appending the
        /// replacement row.
        ///
        /// This method never edits Arrow buffers in place. The replacement row
        /// is validated against the current live snapshot first so an invalid
        /// replacement does not partially delete the original node.
        pub fn update_node(&mut self, old_id: &str, node: NodeFrame) -> Result<()> {
            if node.len() != 1 {
                return Err(GFError::InvalidConfig {
                    message: "update_node requires exactly one replacement row".to_owned(),
                });
            }

            let current_live = self.current_live_node_frame()?;
            let without_old = self.live_node_frame_without_id(&current_live, old_id)?;
            NodeFrame::concat(&[&without_old, &node])?;

            self.delete_node(old_id)?;
            self.add_nodes_batch(node)
        }

        /// Updates one stable edge row by tombstoning it and appending a new
        /// topology entry into the mutable delta buffer.
        ///
        /// The mutable edge delta currently tracks topology only, so this API
        /// rewrites `_src` / `_dst` and leaves edge payload re-materialization
        /// to later phases.
        pub fn update_edge(&mut self, edge_row: u32, src: &str, dst: &str) -> Result<()> {
            if !self.node_id_to_idx.contains_key(src) {
                return Err(GFError::NodeNotFound { id: src.to_owned() });
            }
            if !self.node_id_to_idx.contains_key(dst) {
                return Err(GFError::NodeNotFound { id: dst.to_owned() });
            }
            if edge_row as usize >= self.edge_tombstones.len() {
                return Err(GFError::EdgeNotFound {
                    id: edge_row.to_string(),
                });
            }

            self.delete_edge(edge_row)?;
            self.add_edge(src, dst)
        }

        /// Appends one pending edge in the mutable delta buffer.
        ///
        /// Node ids are resolved up front into the mutable graph's edge-local
        /// compact index space. If a node exists in the `NodeFrame` but has not
        /// yet appeared in any edge, this method assigns it a fresh edge-local
        /// index so later delta reads and flushes can address it consistently.
        pub fn add_edge(&mut self, src: &str, dst: &str) -> Result<()> {
            let src_idx = self.resolve_edge_node_idx(src)?;
            let dst_idx = self.resolve_edge_node_idx(dst)?;

            let mut pending = self
                .delta
                .pending
                .lock()
                .map_err(|_| GFError::InvalidConfig {
                    message: "mutable edge delta pending buffer is poisoned".to_owned(),
                })?;
            pending.push((src_idx, dst_idx));

            if pending.len() > FLUSH_THRESHOLD {
                self.flush_pending_locked(&mut pending)?;
            }

            Ok(())
        }

        /// Returns outgoing neighbors from the mutable graph's current edge view.
        ///
        /// The read path merges three sources in order:
        /// 1. the immutable base CSR snapshot
        /// 2. any frozen mini-CSR chunks flushed from pending deltas
        /// 3. the still-pending raw `(src, dst)` edge buffer
        ///
        /// This keeps mutation-local reads consistent before a later
        /// `compact()` folds all delta state back into the base snapshot.
        pub fn out_neighbors(&self, node_idx: u32) -> Result<std::vec::IntoIter<u32>> {
            if !self.edge_node_is_live(node_idx) {
                return Ok(Vec::new().into_iter());
            }

            let base_csr = self.base_csr.load_full();
            let frozen = self
                .delta
                .frozen
                .read()
                .map_err(|_| GFError::InvalidConfig {
                    message: "mutable edge delta frozen chunks are poisoned".to_owned(),
                })?;
            let pending = self
                .delta
                .pending
                .lock()
                .map_err(|_| GFError::InvalidConfig {
                    message: "mutable edge delta pending buffer is poisoned".to_owned(),
                })?;

            let neighbors = base_csr
                .neighbors(node_idx)
                .iter()
                .copied()
                .zip(base_csr.edge_ids(node_idx).iter().copied())
                .filter_map(|(dst_idx, edge_row)| {
                    (!self
                        .base_snapshot_has_stable_edge_rows
                        .load(Ordering::Relaxed)
                        || self.base_edge_is_live(edge_row))
                    .then_some(dst_idx)
                    .filter(|&dst_idx| self.edge_node_is_live(dst_idx))
                })
                .chain(frozen.iter().flat_map(|chunk| {
                    chunk
                        .neighbors(node_idx)
                        .iter()
                        .copied()
                        .filter(|&dst_idx| {
                            self.edge_node_is_live(node_idx) && self.edge_node_is_live(dst_idx)
                        })
                }))
                .chain(pending.iter().filter_map(|&(src_idx, dst_idx)| {
                    (src_idx == node_idx
                        && self.edge_node_is_live(src_idx)
                        && self.edge_node_is_live(dst_idx))
                    .then_some(dst_idx)
                }))
                .collect::<Vec<_>>();

            Ok(neighbors.into_iter())
        }

        /// Tombstones one node by `_id` and immediately removes it from the
        /// string-to-row lookup tables used by public graph APIs.
        ///
        /// The node payload remains in the underlying `NodeFrame` until a later
        /// `freeze()` materializes a physically compact graph. Any stable base
        /// edges touching this node are tombstoned as well.
        pub fn delete_node(&mut self, id: &str) -> Result<()> {
            let node_row = self
                .node_id_to_idx
                .remove(id)
                .ok_or_else(|| GFError::NodeNotFound { id: id.to_owned() })?;
            self.node_tombstones[node_row as usize] = false;

            if let Some(edge_idx) = self.edge_node_id_to_idx.remove(id) {
                let edge_data = self.edge_data.load();
                for &edge_row in edge_data.out_edge_ids(edge_idx) {
                    self.edge_tombstones[edge_row as usize] = false;
                }
                for &edge_row in edge_data.in_edge_ids(edge_idx) {
                    self.edge_tombstones[edge_row as usize] = false;
                }

                let mut pending =
                    self.delta
                        .pending
                        .lock()
                        .map_err(|_| GFError::InvalidConfig {
                            message: "mutable edge delta pending buffer is poisoned".to_owned(),
                        })?;
                pending.retain(|&(src_idx, dst_idx)| src_idx != edge_idx && dst_idx != edge_idx);
            }

            Ok(())
        }

        /// Tombstones one stable edge row from the immutable edge payload.
        ///
        /// Delta edges in `pending` / `frozen` do not yet carry stable row ids,
        /// so this API is intentionally scoped to the original `EdgeFrame` row
        /// space.
        pub fn delete_edge(&mut self, edge_row: u32) -> Result<()> {
            if edge_row as usize >= self.edge_tombstones.len() {
                return Err(GFError::EdgeNotFound {
                    id: edge_row.to_string(),
                });
            }

            self.edge_tombstones[edge_row as usize] = false;
            Ok(())
        }

        /// Merges the current base CSR snapshot with all frozen delta chunks and
        /// atomically publishes the rebuilt adjacency as the new base snapshot.
        ///
        /// Pending raw edges are flushed first so the rebuild sees a stable
        /// delta set. Readers that already hold the old `Arc<CsrIndex>` keep
        /// working during the swap; new readers observe the rebuilt snapshot.
        pub fn compact(&self) -> Result<()> {
            {
                let mut pending =
                    self.delta
                        .pending
                        .lock()
                        .map_err(|_| GFError::InvalidConfig {
                            message: "mutable edge delta pending buffer is poisoned".to_owned(),
                        })?;
                if !pending.is_empty() {
                    self.flush_pending_locked(&mut pending)?;
                }
            }

            let node_count = self.edge_node_idx_to_id.len();
            let base_csr = self.base_csr.load_full();
            let frozen_chunks = self
                .delta
                .frozen
                .read()
                .map_err(|_| GFError::InvalidConfig {
                    message: "mutable edge delta frozen chunks are poisoned".to_owned(),
                })?
                .clone();

            let merged_rows = (0..node_count as u32)
                .into_par_iter()
                .map(|src_idx| {
                    let base_iter = base_csr
                        .neighbors(src_idx)
                        .iter()
                        .copied()
                        .zip(base_csr.edge_ids(src_idx).iter().copied())
                        .filter_map(|(dst_idx, edge_row)| {
                            self.base_edge_is_live(edge_row)
                                .then_some(dst_idx)
                                .filter(|&dst_idx| {
                                    self.edge_node_is_live(src_idx)
                                        && self.edge_node_is_live(dst_idx)
                                })
                                .map(|dst_idx| (src_idx, dst_idx))
                        });
                    let frozen_iter = frozen_chunks.iter().flat_map(|chunk| {
                        chunk
                            .neighbors(src_idx)
                            .iter()
                            .copied()
                            .filter(move |&dst_idx| {
                                self.edge_node_is_live(src_idx) && self.edge_node_is_live(dst_idx)
                            })
                            .map(move |dst_idx| (src_idx, dst_idx))
                    });

                    base_iter.chain(frozen_iter).collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            let edge_count = merged_rows.iter().map(Vec::len).sum();
            let mut src_rows = Vec::with_capacity(edge_count);
            let mut dst_rows = Vec::with_capacity(edge_count);
            for per_node in merged_rows {
                for (src_idx, dst_idx) in per_node {
                    src_rows.push(src_idx);
                    dst_rows.push(dst_idx);
                }
            }

            let new_csr = Arc::new(CsrIndex::build(&src_rows, &dst_rows, node_count));
            let edge_data = self.edge_data.load_full();
            let rebuilt_edge_data = Arc::new(edge_data.with_out_csr(Arc::clone(&new_csr)));

            self.edge_data.store(rebuilt_edge_data);
            self.base_csr.store(Arc::clone(&new_csr));
            self.base_snapshot_has_stable_edge_rows
                .store(false, Ordering::Relaxed);

            self.delta
                .frozen
                .write()
                .map_err(|_| GFError::InvalidConfig {
                    message: "mutable edge delta frozen chunks are poisoned".to_owned(),
                })?
                .clear();

            Ok(())
        }

        /// Finalizes mutable state into a fresh immutable `GraphFrame`.
        ///
        /// The mutable wrapper is consumed. Live nodes are physically filtered,
        /// surviving stable edge rows keep their original payload columns, and
        /// topology-only delta edges are materialized with default `_type` /
        /// `_direction` values plus null-filled user columns.
        pub fn freeze(self) -> Result<GraphFrame> {
            {
                let mut pending =
                    self.delta
                        .pending
                        .lock()
                        .map_err(|_| GFError::InvalidConfig {
                            message: "mutable edge delta pending buffer is poisoned".to_owned(),
                        })?;
                if !pending.is_empty() {
                    self.flush_pending_locked(&mut pending)?;
                }
            }

            let frozen_chunks = self
                .delta
                .frozen
                .read()
                .map_err(|_| GFError::InvalidConfig {
                    message: "mutable edge delta frozen chunks are poisoned".to_owned(),
                })?
                .clone();

            self.compact()?;

            let nodes = self.current_live_node_frame()?;
            let base_edges = self.live_base_edge_frame()?;
            let delta_edges = self.materialize_delta_edge_frame(&frozen_chunks)?;
            let edges = if delta_edges.is_empty() {
                base_edges
            } else {
                EdgeFrame::concat(&[&base_edges, &delta_edges])?
            };

            GraphFrame::new_with_schema(nodes, edges, self.schema.clone(), false)
        }

        fn edge_node_is_live(&self, edge_node_idx: u32) -> bool {
            let Some(node_id) = self.edge_node_idx_to_id.get(edge_node_idx as usize) else {
                return false;
            };
            let Some(&node_row) = self.node_id_to_idx.get(node_id) else {
                return false;
            };
            self.node_tombstones
                .get(node_row as usize)
                .copied()
                .unwrap_or(false)
        }

        fn current_live_node_frame(&self) -> Result<NodeFrame> {
            let node_frame = self.node_frame.load_full();
            if self.node_tombstones.iter().all(|&live| live) {
                return Ok((*node_frame).clone());
            }

            let mask = BooleanArray::from(self.node_tombstones.clone());
            node_frame.filter(&mask)
        }

        fn live_node_frame_without_id(
            &self,
            current_live: &NodeFrame,
            old_id: &str,
        ) -> Result<NodeFrame> {
            let old_row = current_live
                .row_index(old_id)
                .ok_or_else(|| GFError::NodeNotFound {
                    id: old_id.to_owned(),
                })?;
            let mask = BooleanArray::from(
                (0..current_live.len())
                    .map(|row| row != old_row as usize)
                    .collect::<Vec<_>>(),
            );
            current_live.filter(&mask)
        }

        fn replace_node_frame(&mut self, node_frame: NodeFrame) {
            let (node_id_to_idx, node_idx_to_id) = Self::rebuild_node_mappings(&node_frame);
            let node_count = node_frame.len();

            self.node_frame.store(Arc::new(node_frame));
            self.node_id_to_idx = node_id_to_idx;
            self.node_idx_to_id = node_idx_to_id;
            self.node_tombstones = vec![true; node_count];
        }

        fn rebuild_node_mappings(node_frame: &NodeFrame) -> (HashMap<String, u32>, Vec<String>) {
            let id_column = node_frame.id_column();
            let mut node_id_to_idx = HashMap::with_capacity(id_column.len());
            let mut node_idx_to_id = Vec::with_capacity(id_column.len());

            for (row_idx, id) in id_column.iter().enumerate() {
                let id = id.expect("validated _id column is non-null");
                node_id_to_idx.insert(id.to_owned(), row_idx as u32);
                node_idx_to_id.push(id.to_owned());
            }

            (node_id_to_idx, node_idx_to_id)
        }

        fn base_edge_is_live(&self, edge_row: u32) -> bool {
            self.edge_tombstones
                .get(edge_row as usize)
                .copied()
                .unwrap_or(false)
        }

        fn live_base_edge_frame(&self) -> Result<EdgeFrame> {
            let edge_data = self.edge_data.load_full();
            let mask = BooleanArray::from(
                (0..edge_data.len())
                    .map(|row| self.base_edge_row_is_live(&edge_data, row as u32))
                    .collect::<Vec<_>>(),
            );
            edge_data.filter(&mask)
        }

        fn base_edge_row_is_live(&self, edge_data: &EdgeFrame, edge_row: u32) -> bool {
            if !self.base_edge_is_live(edge_row) {
                return false;
            }

            let batch = edge_data.to_record_batch();
            let src = batch
                .column_by_name(COL_EDGE_SRC)
                .expect("validated edge batch must have _src")
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("_src must be Utf8")
                .value(edge_row as usize);
            let dst = batch
                .column_by_name(COL_EDGE_DST)
                .expect("validated edge batch must have _dst")
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("_dst must be Utf8")
                .value(edge_row as usize);

            self.node_is_live_by_id(src) && self.node_is_live_by_id(dst)
        }

        fn materialize_delta_edge_frame(
            &self,
            frozen_chunks: &[Arc<CsrIndex>],
        ) -> Result<EdgeFrame> {
            let edge_data = self.edge_data.load_full();
            let schema = edge_data.schema().clone();

            let mut src_ids = Vec::new();
            let mut dst_ids = Vec::new();
            for chunk in frozen_chunks {
                for src_idx in 0..chunk.node_count() as u32 {
                    if !self.edge_node_is_live(src_idx) {
                        continue;
                    }
                    let Some(src_id) = self.edge_node_idx_to_id.get(src_idx as usize) else {
                        continue;
                    };
                    for &dst_idx in chunk.neighbors(src_idx) {
                        if !self.edge_node_is_live(dst_idx) {
                            continue;
                        }
                        let Some(dst_id) = self.edge_node_idx_to_id.get(dst_idx as usize) else {
                            continue;
                        };
                        src_ids.push(src_id.clone());
                        dst_ids.push(dst_id.clone());
                    }
                }
            }

            if src_ids.is_empty() {
                return Ok(EdgeFrame::empty(&schema));
            }

            let mut columns: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len());
            for field in schema.fields() {
                let name = field.name().as_str();
                let array: ArrayRef = match name {
                    COL_EDGE_SRC => Arc::new(StringArray::from(src_ids.clone())),
                    COL_EDGE_DST => Arc::new(StringArray::from(dst_ids.clone())),
                    COL_EDGE_TYPE => Arc::new(StringArray::from(vec!["__delta__"; src_ids.len()])),
                    COL_EDGE_DIRECTION => {
                        Arc::new(Int8Array::from(vec![Direction::Out.as_i8(); src_ids.len()]))
                    }
                    _ => new_null_array(field.data_type(), src_ids.len()),
                };
                columns.push(array);
            }

            let batch =
                RecordBatch::try_new(Arc::new(schema), columns).map_err(std::io::Error::other)?;
            EdgeFrame::from_record_batch(batch)
        }

        fn node_is_live_by_id(&self, id: &str) -> bool {
            let Some(&node_row) = self.node_id_to_idx.get(id) else {
                return false;
            };
            self.node_tombstones
                .get(node_row as usize)
                .copied()
                .unwrap_or(false)
        }

        fn resolve_edge_node_idx(&mut self, id: &str) -> Result<u32> {
            if let Some(&idx) = self.edge_node_id_to_idx.get(id) {
                return Ok(idx);
            }

            if !self.node_id_to_idx.contains_key(id) {
                return Err(GFError::NodeNotFound { id: id.to_owned() });
            }

            let idx = self.edge_node_idx_to_id.len() as u32;
            self.edge_node_id_to_idx.insert(id.to_owned(), idx);
            self.edge_node_idx_to_id.push(id.to_owned());
            Ok(idx)
        }

        fn flush_pending_locked(&self, pending: &mut Vec<(u32, u32)>) -> Result<()> {
            if pending.is_empty() {
                return Ok(());
            }

            let node_count = self.edge_node_idx_to_id.len();
            let (src_rows, dst_rows): (Vec<u32>, Vec<u32>) = pending.iter().copied().unzip();
            let mini_csr = Arc::new(CsrIndex::build(&src_rows, &dst_rows, node_count));

            self.delta
                .frozen
                .write()
                .map_err(|_| GFError::InvalidConfig {
                    message: "mutable edge delta frozen chunks are poisoned".to_owned(),
                })?
                .push(mini_csr);

            pending.clear();
            Ok(())
        }
    }

    impl std::fmt::Debug for MutableGraphFrame {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let node_count = self.node_frame.load().len();
            let edge_count = self.edge_data.load().len();
            let base_nodes = self.base_csr.load().node_count();
            let base_edges = self.base_csr.load().edge_count();

            f.debug_struct("MutableGraphFrame")
                .field("node_count", &node_count)
                .field("edge_count", &edge_count)
                .field("base_csr_node_count", &base_nodes)
                .field("base_csr_edge_count", &base_edges)
                .field("delta", &self.delta)
                .field("has_schema", &self.schema.is_some())
                .finish()
        }
    }

    #[cfg(test)]
    mod tests {
        use std::sync::{Arc, Barrier};
        use std::thread;

        use arrow_array::builder::{ListBuilder, StringBuilder};
        use arrow_array::{ArrayRef, Int8Array, ListArray, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema as ArrowSchema};

        use super::MutableGraphFrame;
        use crate::{
            EdgeFrame, GFError, GraphFrame, NodeFrame, COL_EDGE_DIRECTION, COL_EDGE_DST,
            COL_EDGE_SRC, COL_EDGE_TYPE, COL_NODE_ID, COL_NODE_LABEL,
        };

        fn labels_array(values: &[&[&str]]) -> ListArray {
            let value_builder = StringBuilder::new();
            let mut builder = ListBuilder::new(value_builder);
            for labels in values {
                for label in *labels {
                    builder.values().append_value(label);
                }
                builder.append(true);
            }
            builder.finish()
        }

        fn sample_graph() -> GraphFrame {
            let node_schema = Arc::new(ArrowSchema::new(vec![
                Field::new(COL_NODE_ID, DataType::Utf8, false),
                Field::new(
                    COL_NODE_LABEL,
                    DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
                    false,
                ),
            ]));
            let nodes = NodeFrame::from_record_batch(
                RecordBatch::try_new(
                    node_schema,
                    vec![
                        Arc::new(StringArray::from(vec!["alice", "bob", "charlie"])) as ArrayRef,
                        Arc::new(labels_array(&[&["Person"], &["Person"], &["Person"]]))
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
            ]));
            let edges = EdgeFrame::from_record_batch(
                RecordBatch::try_new(
                    edge_schema,
                    vec![
                        Arc::new(StringArray::from(vec!["alice", "bob"])) as ArrayRef,
                        Arc::new(StringArray::from(vec!["bob", "charlie"])) as ArrayRef,
                        Arc::new(StringArray::from(vec!["KNOWS", "KNOWS"])) as ArrayRef,
                        Arc::new(Int8Array::from(vec![0i8, 0])) as ArrayRef,
                    ],
                )
                .unwrap(),
            )
            .unwrap();

            GraphFrame::new(nodes, edges).unwrap()
        }

        #[test]
        fn from_graph_frame_shares_edge_csr_with_base_csr() {
            let mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            let base_csr = mutable.base_csr.load_full();
            let edge_data = mutable.edge_data.load();
            let edge_csr = edge_data.out_csr_arc();

            assert!(Arc::ptr_eq(&base_csr, &edge_csr));
            assert_eq!(mutable.node_frame.load().len(), 3);
            assert_eq!(mutable.edge_data.load().len(), 2);
            assert_eq!(mutable.node_id_to_idx.get("alice"), Some(&0));
        }

        #[test]
        fn add_edge_pushes_pending_with_existing_edge_local_indices() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            mutable.add_edge("alice", "charlie").unwrap();

            let pending = mutable.delta.pending.lock().unwrap();
            assert_eq!(pending.as_slice(), &[(0, 2)]);
        }

        #[test]
        fn add_edge_rejects_unknown_node_ids() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            let err = mutable.add_edge("alice", "ghost").unwrap_err();
            assert!(matches!(err, GFError::NodeNotFound { id } if id == "ghost"));
        }

        #[test]
        fn add_edge_assigns_edge_local_indices_for_isolated_nodes() {
            let node_schema = Arc::new(ArrowSchema::new(vec![
                Field::new(COL_NODE_ID, DataType::Utf8, false),
                Field::new(
                    COL_NODE_LABEL,
                    DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
                    false,
                ),
            ]));
            let nodes = NodeFrame::from_record_batch(
                RecordBatch::try_new(
                    node_schema,
                    vec![
                        Arc::new(StringArray::from(vec!["alice", "bob", "solo"])) as ArrayRef,
                        Arc::new(labels_array(&[&["Person"], &["Person"], &["Thing"]])) as ArrayRef,
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
            ]));
            let edges = EdgeFrame::from_record_batch(
                RecordBatch::try_new(
                    edge_schema,
                    vec![
                        Arc::new(StringArray::from(vec!["alice"])) as ArrayRef,
                        Arc::new(StringArray::from(vec!["bob"])) as ArrayRef,
                        Arc::new(StringArray::from(vec!["KNOWS"])) as ArrayRef,
                        Arc::new(Int8Array::from(vec![0i8])) as ArrayRef,
                    ],
                )
                .unwrap(),
            )
            .unwrap();
            let graph = GraphFrame::new(nodes, edges).unwrap();
            let mut mutable = MutableGraphFrame::from_graph_frame(graph);

            mutable.add_edge("solo", "alice").unwrap();

            let pending = mutable.delta.pending.lock().unwrap();
            assert_eq!(pending.as_slice(), &[(2, 0)]);
            drop(pending);
            assert_eq!(mutable.edge_node_id_to_idx.get("solo"), Some(&2));
            assert_eq!(mutable.edge_node_idx_to_id[2], "solo");
        }

        #[test]
        fn add_edge_keeps_pending_until_threshold_is_exceeded() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            mutable.add_edge("alice", "charlie").unwrap();
            mutable.add_edge("bob", "alice").unwrap();

            let pending = mutable.delta.pending.lock().unwrap();
            assert_eq!(pending.as_slice(), &[(0, 2), (1, 0)]);
            drop(pending);
            let frozen = mutable.delta.frozen.read().unwrap();
            assert!(frozen.is_empty());
        }

        #[test]
        fn add_edge_flushes_pending_into_frozen_mini_csr_after_threshold() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            mutable.add_edge("alice", "charlie").unwrap();
            mutable.add_edge("bob", "alice").unwrap();
            mutable.add_edge("charlie", "alice").unwrap();

            let pending = mutable.delta.pending.lock().unwrap();
            assert!(pending.is_empty());
            drop(pending);

            let frozen = mutable.delta.frozen.read().unwrap();
            assert_eq!(frozen.len(), 1);
            let chunk = &frozen[0];
            assert_eq!(chunk.node_count(), 3);
            assert_eq!(chunk.edge_count(), 3);
            assert_eq!(chunk.neighbors(0), &[2]);
            assert_eq!(chunk.neighbors(1), &[0]);
            assert_eq!(chunk.neighbors(2), &[0]);
        }

        #[test]
        fn out_neighbors_chains_base_frozen_and_pending_sources() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            mutable.add_edge("alice", "charlie").unwrap();
            let before_flush = mutable.out_neighbors(0).unwrap().collect::<Vec<_>>();
            assert_eq!(before_flush, vec![1, 2]);

            mutable.add_edge("bob", "alice").unwrap();
            mutable.add_edge("charlie", "alice").unwrap();

            let alice_neighbors = mutable.out_neighbors(0).unwrap().collect::<Vec<_>>();
            let bob_neighbors = mutable.out_neighbors(1).unwrap().collect::<Vec<_>>();
            let charlie_neighbors = mutable.out_neighbors(2).unwrap().collect::<Vec<_>>();

            assert_eq!(alice_neighbors, vec![1, 2]);
            assert_eq!(bob_neighbors, vec![2, 0]);
            assert_eq!(charlie_neighbors, vec![0]);
        }

        #[test]
        fn out_neighbors_ignores_unrelated_pending_edges() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            mutable.add_edge("bob", "alice").unwrap();

            let alice_neighbors = mutable.out_neighbors(0).unwrap().collect::<Vec<_>>();
            let bob_neighbors = mutable.out_neighbors(1).unwrap().collect::<Vec<_>>();

            assert_eq!(alice_neighbors, vec![1]);
            assert_eq!(bob_neighbors, vec![2, 0]);
        }

        #[test]
        fn compact_publishes_merged_base_and_clears_frozen_chunks() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            mutable.add_edge("alice", "charlie").unwrap();
            mutable.add_edge("bob", "alice").unwrap();
            mutable.add_edge("charlie", "alice").unwrap();

            {
                let frozen = mutable.delta.frozen.read().unwrap();
                assert_eq!(frozen.len(), 1);
            }

            mutable.compact().unwrap();

            let neighbors_0 = mutable.out_neighbors(0).unwrap().collect::<Vec<_>>();
            let neighbors_1 = mutable.out_neighbors(1).unwrap().collect::<Vec<_>>();
            let neighbors_2 = mutable.out_neighbors(2).unwrap().collect::<Vec<_>>();

            assert_eq!(neighbors_0, vec![1, 2]);
            assert_eq!(neighbors_1, vec![0, 2]);
            assert_eq!(neighbors_2, vec![0]);

            let frozen = mutable.delta.frozen.read().unwrap();
            assert!(frozen.is_empty());
            drop(frozen);

            let pending = mutable.delta.pending.lock().unwrap();
            assert!(pending.is_empty());
            drop(pending);

            let base_csr = mutable.base_csr.load_full();
            let edge_csr = mutable.edge_data.load().out_csr_arc();
            assert!(Arc::ptr_eq(&base_csr, &edge_csr));
            assert_eq!(base_csr.neighbors(1), &[0, 2]);
        }

        #[test]
        fn compact_keeps_old_base_snapshot_alive_for_existing_readers() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());
            let old_base = mutable.base_csr.load_full();

            mutable.add_edge("bob", "alice").unwrap();
            mutable.add_edge("charlie", "alice").unwrap();
            mutable.add_edge("alice", "charlie").unwrap();
            mutable.compact().unwrap();

            assert_eq!(old_base.neighbors(1), &[2]);

            let new_base = mutable.base_csr.load_full();
            assert_eq!(new_base.neighbors(1), &[0, 2]);
            assert!(!Arc::ptr_eq(&old_base, &new_base));
        }

        #[test]
        fn delete_edge_hides_tombstoned_base_edge_from_neighbors() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            mutable.delete_edge(0).unwrap();

            let alice_neighbors = mutable.out_neighbors(0).unwrap().collect::<Vec<_>>();
            assert!(alice_neighbors.is_empty());
            assert!(!mutable.edge_tombstones[0]);
        }

        #[test]
        fn delete_node_tombstones_incident_edges_and_removes_id_lookup() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            mutable.delete_node("bob").unwrap();

            assert!(!mutable.node_tombstones[1]);
            assert!(!mutable.edge_tombstones[0]);
            assert!(!mutable.edge_tombstones[1]);
            assert!(!mutable.node_id_to_idx.contains_key("bob"));

            let alice_neighbors = mutable.out_neighbors(0).unwrap().collect::<Vec<_>>();
            let bob_neighbors = mutable.out_neighbors(1).unwrap().collect::<Vec<_>>();
            assert!(alice_neighbors.is_empty());
            assert!(bob_neighbors.is_empty());
        }

        #[test]
        fn delete_node_prunes_pending_edges_for_deleted_node() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            mutable.add_edge("alice", "charlie").unwrap();
            mutable.add_edge("charlie", "alice").unwrap();
            mutable.delete_node("charlie").unwrap();

            let pending = mutable.delta.pending.lock().unwrap();
            assert!(pending.is_empty());
        }

        fn node_frame_from_ids(ids: &[&str]) -> NodeFrame {
            let labels: Vec<&[&str]> = (0..ids.len()).map(|_| &["Person"][..]).collect();
            let node_schema = Arc::new(ArrowSchema::new(vec![
                Field::new(COL_NODE_ID, DataType::Utf8, false),
                Field::new(
                    COL_NODE_LABEL,
                    DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
                    false,
                ),
            ]));

            NodeFrame::from_record_batch(
                RecordBatch::try_new(
                    node_schema,
                    vec![
                        Arc::new(StringArray::from(ids.to_vec())) as ArrayRef,
                        Arc::new(labels_array(&labels)) as ArrayRef,
                    ],
                )
                .unwrap(),
            )
            .unwrap()
        }

        fn edge_endpoints(graph: &GraphFrame, edge_row: usize) -> (String, String) {
            let batch = graph.edges().to_record_batch();
            let src = batch
                .column_by_name(COL_EDGE_SRC)
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(edge_row)
                .to_owned();
            let dst = batch
                .column_by_name(COL_EDGE_DST)
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(edge_row)
                .to_owned();
            (src, dst)
        }

        #[test]
        fn add_node_appends_single_row_and_updates_lookup() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());
            let delta = node_frame_from_ids(&["dora"]);

            mutable.add_node(delta).unwrap();

            assert_eq!(mutable.node_frame.load().len(), 4);
            assert_eq!(mutable.node_id_to_idx.get("dora"), Some(&3));
            assert_eq!(mutable.node_idx_to_id[3], "dora");
            assert_eq!(mutable.node_tombstones, vec![true, true, true, true]);
        }

        #[test]
        fn add_nodes_batch_appends_multiple_rows() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());
            let delta = node_frame_from_ids(&["dora", "erin"]);

            mutable.add_nodes_batch(delta).unwrap();

            assert_eq!(mutable.node_frame.load().len(), 5);
            assert_eq!(mutable.node_id_to_idx.get("dora"), Some(&3));
            assert_eq!(mutable.node_id_to_idx.get("erin"), Some(&4));
        }

        #[test]
        fn add_nodes_batch_reuses_deleted_id_after_live_snapshot_rebuild() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());
            mutable.delete_node("bob").unwrap();

            let delta = node_frame_from_ids(&["bob"]);
            mutable.add_nodes_batch(delta).unwrap();

            assert_eq!(mutable.node_frame.load().len(), 3);
            assert_eq!(mutable.node_id_to_idx.get("bob"), Some(&2));
            assert_eq!(mutable.node_idx_to_id[2], "bob");
            assert_eq!(mutable.node_tombstones, vec![true, true, true]);
        }

        #[test]
        fn update_node_tombstones_old_row_and_appends_replacement() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());
            let replacement = node_frame_from_ids(&["dora"]);

            mutable.update_node("bob", replacement).unwrap();

            assert_eq!(mutable.node_frame.load().len(), 3);
            assert!(!mutable.node_id_to_idx.contains_key("bob"));
            assert_eq!(mutable.node_id_to_idx.get("dora"), Some(&2));
            assert_eq!(mutable.node_idx_to_id[2], "dora");
            assert_eq!(mutable.node_tombstones, vec![true, true, true]);
        }

        #[test]
        fn update_edge_tombstones_old_edge_and_reinserts_new_topology() {
            let graph = sample_graph();
            let (old_src, old_dst) = edge_endpoints(&graph, 0);
            assert_eq!((old_src, old_dst), ("alice".to_owned(), "bob".to_owned()));

            let mut mutable = MutableGraphFrame::from_graph_frame(graph);
            mutable.update_edge(0, "alice", "charlie").unwrap();

            assert!(!mutable.edge_tombstones[0]);
            let pending = mutable.delta.pending.lock().unwrap();
            assert_eq!(pending.as_slice(), &[(0, 2)]);
            drop(pending);

            let alice_neighbors = mutable.out_neighbors(0).unwrap().collect::<Vec<_>>();
            assert_eq!(alice_neighbors, vec![2]);
        }

        #[test]
        fn update_node_precheck_preserves_original_when_replacement_conflicts() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());
            let conflicting = node_frame_from_ids(&["alice"]);

            let err = mutable.update_node("bob", conflicting).unwrap_err();
            assert!(matches!(err, GFError::DuplicateNodeId { id } if id == "alice"));
            assert_eq!(mutable.node_id_to_idx.get("bob"), Some(&1));
            assert!(mutable.node_tombstones[1]);
        }

        #[test]
        fn add_edge_keeps_base_snapshot_unchanged_until_compact() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());
            let original_base = mutable.base_csr.load_full();

            mutable.add_edge("alice", "charlie").unwrap();

            assert_eq!(original_base.edge_count(), 2);
            assert_eq!(mutable.base_csr.load().edge_count(), 2);
            assert_eq!(mutable.delta.frozen.read().unwrap().len(), 0);
            assert_eq!(mutable.delta.pending.lock().unwrap().as_slice(), &[(0, 2)]);
            assert_eq!(original_base.neighbors(0), &[1]);
            let mutable_neighbors = mutable.out_neighbors(0).unwrap().collect::<Vec<_>>();
            assert_eq!(mutable_neighbors, vec![1, 2]);
        }

        #[test]
        fn compact_physically_removes_tombstoned_edges_from_base_snapshot() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());

            mutable.add_edge("alice", "charlie").unwrap();
            mutable.add_edge("charlie", "alice").unwrap();
            mutable.delete_edge(0).unwrap();
            mutable.delete_node("bob").unwrap();
            mutable.compact().unwrap();

            let rebuilt = mutable.base_csr.load_full();
            assert_eq!(rebuilt.edge_count(), 2);
            assert_eq!(rebuilt.neighbors(0), &[2]);
            assert_eq!(rebuilt.neighbors(2), &[0]);
            assert!(rebuilt.neighbors(1).is_empty());

            let neighbors_after_compact = mutable.out_neighbors(0).unwrap().collect::<Vec<_>>();
            assert_eq!(neighbors_after_compact, vec![2]);
            assert!(mutable.delta.pending.lock().unwrap().is_empty());
            assert!(mutable.delta.frozen.read().unwrap().is_empty());
        }

        #[test]
        fn compact_keeps_reader_snapshot_valid_across_threads() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());
            mutable.add_edge("alice", "charlie").unwrap();
            mutable.add_edge("charlie", "alice").unwrap();

            let shared = Arc::new(mutable);
            let barrier = Arc::new(Barrier::new(2));
            let reader_graph = Arc::clone(&shared);
            let reader_barrier = Arc::clone(&barrier);

            let reader = thread::spawn(move || {
                let snapshot = reader_graph.base_csr.load_full();
                reader_barrier.wait();
                reader_barrier.wait();
                snapshot.neighbors(0).to_vec()
            });

            barrier.wait();
            shared.compact().unwrap();
            let fresh_neighbors = shared.out_neighbors(0).unwrap().collect::<Vec<_>>();
            barrier.wait();

            let stale_neighbors = reader.join().unwrap();
            assert_eq!(stale_neighbors, vec![1]);
            assert_eq!(fresh_neighbors, vec![1, 2]);
        }

        #[test]
        fn freeze_returns_graph_with_live_nodes_and_materialized_delta_edges() {
            let mut mutable = MutableGraphFrame::from_graph_frame(sample_graph());
            mutable.add_edge("charlie", "alice").unwrap();
            mutable.delete_edge(0).unwrap();
            mutable.delete_node("bob").unwrap();
            mutable.add_node(node_frame_from_ids(&["dora"])).unwrap();

            let frozen = mutable.freeze().unwrap();

            assert_eq!(frozen.node_count(), 3);
            assert_eq!(frozen.edge_count(), 1);
            assert!(frozen.nodes().row_index("alice").is_some());
            assert!(frozen.nodes().row_index("charlie").is_some());
            assert!(frozen.nodes().row_index("dora").is_some());
            assert!(frozen.nodes().row_index("bob").is_none());
            assert_eq!(frozen.out_neighbors("charlie").unwrap(), vec!["alice"]);
            assert!(frozen.out_neighbors("alice").unwrap().is_empty());
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    /// Wasm placeholder until mutation support is designed for non-threaded
    /// targets.
    #[allow(dead_code)]
    #[derive(Debug, Default)]
    pub struct DeltaEdges;

    /// Wasm placeholder until mutation support is designed for non-threaded
    /// targets.
    #[derive(Debug, Default)]
    pub struct MutableGraphFrame;

    impl MutableGraphFrame {
        pub fn from_graph_frame(_graph: super::super::graph_frame::GraphFrame) -> Self {
            Self
        }

        pub fn add_edge(&mut self, _src: &str, _dst: &str) -> crate::Result<()> {
            Err(crate::GFError::UnsupportedOperation {
                message: "mutable graph edges are not supported on wasm32".to_owned(),
            })
        }

        pub fn out_neighbors(&self, _node_idx: u32) -> crate::Result<std::vec::IntoIter<u32>> {
            Err(crate::GFError::UnsupportedOperation {
                message: "mutable graph reads are not supported on wasm32".to_owned(),
            })
        }

        pub fn compact(&self) -> crate::Result<()> {
            Err(crate::GFError::UnsupportedOperation {
                message: "mutable graph compaction is not supported on wasm32".to_owned(),
            })
        }

        pub fn delete_node(&mut self, _id: &str) -> crate::Result<()> {
            Err(crate::GFError::UnsupportedOperation {
                message: "mutable graph deletes are not supported on wasm32".to_owned(),
            })
        }

        pub fn delete_edge(&mut self, _edge_row: u32) -> crate::Result<()> {
            Err(crate::GFError::UnsupportedOperation {
                message: "mutable graph deletes are not supported on wasm32".to_owned(),
            })
        }

        pub fn add_node(&mut self, _node: super::super::NodeFrame) -> crate::Result<()> {
            Err(crate::GFError::UnsupportedOperation {
                message: "mutable graph node appends are not supported on wasm32".to_owned(),
            })
        }

        pub fn add_nodes_batch(&mut self, _nodes: super::super::NodeFrame) -> crate::Result<()> {
            Err(crate::GFError::UnsupportedOperation {
                message: "mutable graph node appends are not supported on wasm32".to_owned(),
            })
        }

        pub fn update_node(
            &mut self,
            _old_id: &str,
            _node: super::super::NodeFrame,
        ) -> crate::Result<()> {
            Err(crate::GFError::UnsupportedOperation {
                message: "mutable graph updates are not supported on wasm32".to_owned(),
            })
        }

        pub fn update_edge(&mut self, _edge_row: u32, _src: &str, _dst: &str) -> crate::Result<()> {
            Err(crate::GFError::UnsupportedOperation {
                message: "mutable graph updates are not supported on wasm32".to_owned(),
            })
        }

        pub fn freeze(self) -> crate::Result<super::super::graph_frame::GraphFrame> {
            Err(crate::GFError::UnsupportedOperation {
                message: "mutable graph freeze is not supported on wasm32".to_owned(),
            })
        }
    }
}

#[allow(unused_imports)]
pub use imp::{DeltaEdges, MutableGraphFrame};
