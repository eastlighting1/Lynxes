mod frame_builder;
mod gf_parser;
mod gf_writer;
mod gfb;
#[cfg(not(target_arch = "wasm32"))]
mod parquet;

pub use gf_parser::{parse_gf, ParsedEdgeDecl, ParsedGfDocument, ParsedNodeDecl};
pub use gf_writer::write_gf;
pub use gfb::{
    read_gfb_inspect, GfbCompression, GfbInspect, GfbReadOptions, GfbWriteOptions,
};
#[cfg(not(target_arch = "wasm32"))]
pub use gfb::{
    read_gfb, read_gfb_streaming, read_gfb_streaming_with_options, read_gfb_with_options,
    write_gfb, GfbGraphStream,
};
#[cfg(not(target_arch = "wasm32"))]
pub use parquet::{
    read_parquet_graph, read_parquet_graph_with_options, write_parquet_graph, ParquetReadOptions,
};
