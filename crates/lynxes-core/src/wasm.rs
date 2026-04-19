#![cfg(target_arch = "wasm32")]

//! WebAssembly bindings for graphframe-core.
//!
//! Exposed symbols:
//! - `wasm_version()` — library version string
//! - `inspect_gfb_bytes(bytes)` — parse magic/header only, return JSON summary
//! - `read_gfb_bytes_summary(bytes)` — full parse, return node/edge counts as JSON
//! - `inspect_gfb_url(url)` — fetch .gfb from URL via `fetch()`, then inspect
//! - `filter_nodes_by_col_gt(bytes, col, threshold)` — parse .gfb, filter nodes
//!   where `col > threshold` (integer), return JSON `{node_count, edge_count}`
//! - `pagerank_summary(bytes, damping, max_iter)` — compute PageRank on in-memory
//!   graph, return top-10 node IDs by rank as JSON array
//! - `neighbors_of(bytes, node_id, direction)` — return neighbor IDs as JSON array

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::{
    io::{read_gfb_bytes_with_options, read_gfb_inspect_bytes},
    BinaryOp, Direction, Expr, GFError, GfbReadOptions, LazyGraphFrame, PageRankConfig,
    ScalarValue,
};

fn js_error(message: impl Into<String>) -> JsValue {
    js_sys::Error::new(&message.into()).into()
}

fn gf_err(e: GFError) -> JsValue {
    js_error(e.to_string())
}

// ── Version ──────────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_version() -> String {
    crate::version().to_owned()
}

// ── Byte-level I/O ───────────────────────────────────────────────────────────

/// Parse only the magic + header of a `.gfb` byte slice and return a JSON
/// summary (node count, edge count, labels, compression, etc.).
#[wasm_bindgen]
pub fn inspect_gfb_bytes(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let inspect = read_gfb_inspect_bytes(bytes).map_err(gf_err)?;
    serde_wasm_bindgen::to_value(&inspect).map_err(|e| js_error(e.to_string()))
}

/// Fully parse a `.gfb` byte slice and return a JSON summary:
/// `{node_count, edge_count, node_columns, edge_columns, density}`.
#[wasm_bindgen]
pub fn read_gfb_bytes_summary(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let graph = read_gfb_bytes_with_options(bytes, &GfbReadOptions::default()).map_err(gf_err)?;
    let summary = serde_json::json!({
        "node_count": graph.node_count(),
        "edge_count": graph.edge_count(),
        "node_columns": graph.nodes().column_names(),
        "edge_columns": graph.edges().column_names(),
        "density": graph.density(),
    });
    serde_wasm_bindgen::to_value(&summary).map_err(|e| js_error(e.to_string()))
}

/// Fetch a `.gfb` file from `url` using the browser `fetch()` API and return
/// the same JSON summary as `inspect_gfb_bytes`.
#[wasm_bindgen]
pub async fn inspect_gfb_url(url: String) -> Result<JsValue, JsValue> {
    let window = web_sys::window().ok_or_else(|| js_error("window is not available"))?;
    let response = JsFuture::from(window.fetch_with_str(&url)).await?;
    let response: web_sys::Response = response
        .dyn_into()
        .map_err(|_| js_error("failed to coerce fetch response"))?;
    let buffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|e| js_error(format!("failed to read response body: {e:?}")))?,
    )
    .await?;

    let array = Uint8Array::new(&buffer);
    let mut bytes = vec![0u8; array.length() as usize];
    array.copy_to(&mut bytes);
    inspect_gfb_bytes(&bytes)
}

// ── In-memory query operations ────────────────────────────────────────────────

/// Parse a `.gfb` byte slice and filter nodes where `col > threshold`.
/// Returns `{node_count, edge_count}` of the filtered result.
///
/// Example (JS):
/// ```js
/// const result = filter_nodes_by_col_gt(bytes, "age", 30);
/// console.log(result.node_count);
/// ```
#[wasm_bindgen]
pub fn filter_nodes_by_col_gt(
    bytes: &[u8],
    col: &str,
    threshold: i64,
) -> Result<JsValue, JsValue> {
    let graph = read_gfb_bytes_with_options(bytes, &GfbReadOptions::default()).map_err(gf_err)?;
    let lazy = LazyGraphFrame::from_graph(&graph);
    let filtered = lazy
        .filter_nodes(Expr::BinaryOp {
            left: Box::new(Expr::Col { name: col.to_owned() }),
            op: crate::BinaryOp::Gt,
            right: Box::new(Expr::Literal { value: ScalarValue::Int(threshold) }),
        })
        .collect()
        .map_err(gf_err)?;
    let result = serde_json::json!({
        "node_count": filtered.node_count(),
        "edge_count": filtered.edge_count(),
    });
    serde_wasm_bindgen::to_value(&result).map_err(|e| js_error(e.to_string()))
}

/// Parse a `.gfb` byte slice, compute PageRank, and return the top-`n` node
/// IDs sorted by descending rank as a JSON array of `{id, rank}` objects.
#[wasm_bindgen]
pub fn pagerank_top_n(
    bytes: &[u8],
    damping: f64,
    max_iter: usize,
    n: usize,
) -> Result<JsValue, JsValue> {
    use arrow_array::{cast::AsArray, types::Float64Type};

    let graph = read_gfb_bytes_with_options(bytes, &GfbReadOptions::default()).map_err(gf_err)?;
    let config = PageRankConfig { damping, max_iter, ..Default::default() };
    let nf = graph.pagerank(&config).map_err(gf_err)?;

    let batch = nf.to_record_batch();
    let id_col = batch
        .column_by_name("_id")
        .ok_or_else(|| js_error("missing _id column"))?;
    let rank_col = batch
        .column_by_name("pagerank")
        .ok_or_else(|| js_error("missing pagerank column"))?;

    let ids = id_col.as_string::<i32>();
    let ranks = rank_col.as_primitive::<Float64Type>();

    let mut pairs: Vec<(String, f64)> = ids
        .iter()
        .zip(ranks.values().iter())
        .filter_map(|(id, &rank)| Some((id?.to_owned(), rank)))
        .collect();
    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    pairs.truncate(n);

    let json: Vec<_> = pairs
        .into_iter()
        .map(|(id, rank)| serde_json::json!({"id": id, "rank": rank}))
        .collect();
    serde_wasm_bindgen::to_value(&json).map_err(|e| js_error(e.to_string()))
}

/// Parse a `.gfb` byte slice and return the direct neighbor IDs of `node_id`.
/// `direction`: `"out"` (default), `"in"`, or `"both"`.
///
/// Returns a JSON array of neighbor ID strings.
#[wasm_bindgen]
pub fn neighbors_of(bytes: &[u8], node_id: &str, direction: &str) -> Result<JsValue, JsValue> {
    let graph = read_gfb_bytes_with_options(bytes, &GfbReadOptions::default()).map_err(gf_err)?;
    let dir = match direction {
        "in" => Direction::In,
        "both" => Direction::Both,
        _ => Direction::Out,
    };
    let ids = graph.neighbors(node_id, dir).map_err(gf_err)?;
    serde_wasm_bindgen::to_value(&ids).map_err(|e| js_error(e.to_string()))
}
