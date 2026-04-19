use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};

use graphframe_core::{
    parse_gf, read_gfb, read_gfb_inspect, read_parquet_graph, write_gf, write_gfb,
    write_parquet_graph, BinaryOp, Direction, EdgeTypeSpec, Expr, GfbCompression, GfbInspect,
    GfbWriteOptions, GraphFrame, LazyGraphFrame, ScalarValue, COL_NODE_ID, COL_NODE_LABEL,
};

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(
    name = "gf",
    version = graphframe_core::version(),
    about = "Graphframe command-line tool",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print statistics for a .gf or .gfb graph file without loading the full graph.
    Inspect {
        /// Path to a `.gf` text file or `.gfb` binary file.
        file: String,
    },

    /// Convert a graph file between supported formats (.gf, .gfb, .parquet).
    ///
    /// Parquet uses a dual-file convention: `foo.parquet` expands to
    /// `foo-nodes.parquet` + `foo-edges.parquet`.
    Convert {
        /// Input file path.
        input: String,

        /// Output file path.
        output: String,

        /// Compression codec for .gfb output (ignored for other formats).
        #[arg(long, default_value = "none")]
        compression: CompressionArg,
    },

    /// Load a graph and optionally run a BFS traversal, then print a result summary.
    ///
    /// Without --from / --from-label the full graph is loaded and summarised.
    /// With a seed, BFS expansion is run via the lazy engine and the subgraph is
    /// summarised (and optionally written to --output).
    Query {
        /// Input graph file (.gf, .gfb, or .parquet).
        input: String,

        /// Seed the traversal from a single node ID.
        #[arg(long, conflicts_with = "from_label")]
        from: Option<String>,

        /// Seed the traversal from all nodes that carry this label.
        #[arg(long)]
        from_label: Option<String>,

        /// Number of BFS hops to expand from the seed (requires --from or --from-label).
        #[arg(long, default_value = "1")]
        hops: u32,

        /// Edge type to traverse (omit or pass "any" to traverse all types).
        #[arg(long)]
        edge_type: Option<String>,

        /// Traversal direction: out, in, both, or undirected.
        #[arg(long, default_value = "out")]
        direction: DirectionArg,

        /// Write the result subgraph to this file (format detected by extension).
        #[arg(long)]
        output: Option<String>,
    },
}

/// Compression options exposed through `--compression`.
#[derive(Debug, Clone, ValueEnum)]
enum CompressionArg {
    None,
    Zstd,
    Lz4,
}

/// Traversal direction options exposed through `--direction`.
#[derive(Debug, Clone, ValueEnum)]
enum DirectionArg {
    Out,
    In,
    Both,
    Undirected,
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Inspect { file } => cmd_inspect(&file),
        Command::Convert {
            input,
            output,
            compression,
        } => cmd_convert(&input, &output, compression),
        Command::Query {
            input,
            from,
            from_label,
            hops,
            edge_type,
            direction,
            output,
        } => cmd_query(
            &input,
            from.as_deref(),
            from_label.as_deref(),
            hops,
            edge_type.as_deref(),
            direction,
            output.as_deref(),
        ),
    }
}

// ── `gf inspect` ─────────────────────────────────────────────────────────────

fn cmd_inspect(file: &str) -> Result<()> {
    let path = Path::new(file);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "gfb" => inspect_gfb(path),
        "gf" => inspect_gf(path),
        other => bail!("unrecognised extension '.{other}'; expected .gf or .gfb"),
    }
}

// ── .gfb fast-path inspect ────────────────────────────────────────────────────

fn inspect_gfb(path: &Path) -> Result<()> {
    let info: GfbInspect = read_gfb_inspect(path)
        .with_context(|| format!("failed to read header of {}", path.display()))?;

    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
    let (major, minor) = info.version;

    println!("File:        {filename}");
    println!("Format:      .gfb v{major}.{minor}");
    if !info.created_at.is_empty() {
        println!("Created:     {}", info.created_at);
    }
    println!("Compression: {}", info.compression);
    println!();
    println!("Nodes:  {:>12}", fmt_count(info.node_count));
    println!("Edges:  {:>12}", fmt_count(info.edge_count));
    println!();
    print_tag_list("Labels", &info.node_labels);
    print_tag_list("Edge types", &info.edge_types);
    println!(
        "Schema:      {}",
        if info.has_schema { "embedded" } else { "none" }
    );

    Ok(())
}

// ── .gf text-format inspect ──────────────────────────────────────────────────

fn inspect_gf(path: &Path) -> Result<()> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let doc = parse_gf(&source).with_context(|| format!("failed to parse {}", path.display()))?;

    let node_labels: BTreeSet<String> = doc
        .nodes
        .iter()
        .flat_map(|n| n.labels.iter().cloned())
        .collect();

    let edge_types: BTreeSet<String> = doc.edges.iter().map(|e| e.edge_type.clone()).collect();

    let has_schema = !doc.schema.nodes.is_empty() || !doc.schema.edges.is_empty();

    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

    println!("File:   {filename}");
    println!("Format: .gf (text)");
    println!();
    println!("Nodes:  {:>12}", fmt_count(doc.nodes.len()));
    println!("Edges:  {:>12}", fmt_count(doc.edges.len()));
    println!();
    let labels_vec: Vec<String> = node_labels.into_iter().collect();
    let types_vec: Vec<String> = edge_types.into_iter().collect();
    print_tag_list("Labels", &labels_vec);
    print_tag_list("Edge types", &types_vec);
    println!("Schema: {}", if has_schema { "declared" } else { "none" });

    Ok(())
}

// ── `gf convert` ─────────────────────────────────────────────────────────────

/// Recognised input/output format, derived from the file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Gf,
    Gfb,
    Parquet,
}

fn detect_format(path: &Path) -> Result<Format> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "gf" => Ok(Format::Gf),
        "gfb" => Ok(Format::Gfb),
        "parquet" => Ok(Format::Parquet),
        other => bail!("unrecognised extension '.{other}'; expected .gf, .gfb, or .parquet"),
    }
}

/// For `foo.parquet` → `foo-nodes.parquet` + `foo-edges.parquet`.
fn parquet_stem_paths(path: &Path) -> (PathBuf, PathBuf) {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("graph");
    let dir = path.parent().unwrap_or(Path::new("."));
    (
        dir.join(format!("{stem}-nodes.parquet")),
        dir.join(format!("{stem}-edges.parquet")),
    )
}

fn spinner(msg: &'static str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(msg);
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

fn cmd_convert(input: &str, output: &str, compression: CompressionArg) -> Result<()> {
    let in_path = Path::new(input);
    let out_path = Path::new(output);

    let in_fmt = detect_format(in_path)?;
    let out_fmt = detect_format(out_path)?;

    if in_fmt == out_fmt {
        bail!(
            "input and output have the same format ({:?}); nothing to do",
            in_fmt
        );
    }

    // ── Load ────────────────────────────────────────────────────────────────
    let sp = spinner("Loading…");
    let graph: GraphFrame = match in_fmt {
        Format::Gf => {
            let source = std::fs::read_to_string(in_path)
                .with_context(|| format!("failed to read {}", in_path.display()))?;
            let doc = parse_gf(&source)
                .with_context(|| format!("failed to parse {}", in_path.display()))?;
            doc.to_graph_frame()
                .with_context(|| format!("failed to build graph from {}", in_path.display()))?
        }
        Format::Gfb => {
            read_gfb(in_path).with_context(|| format!("failed to read {}", in_path.display()))?
        }
        Format::Parquet => {
            let (nodes_path, edges_path) = parquet_stem_paths(in_path);
            read_parquet_graph(&nodes_path, &edges_path)
                .with_context(|| format!("failed to read parquet from {}", in_path.display()))?
        }
    };
    sp.finish_with_message(format!(
        "Loaded  {} nodes, {} edges",
        fmt_count(graph.node_count()),
        fmt_count(graph.edge_count()),
    ));

    // ── Write ────────────────────────────────────────────────────────────────
    let sp = spinner("Writing…");
    match out_fmt {
        Format::Gf => {
            write_gf(&graph, out_path)
                .with_context(|| format!("failed to write {}", out_path.display()))?;
        }
        Format::Gfb => {
            let codec = match compression {
                CompressionArg::None => GfbCompression::None,
                CompressionArg::Zstd => GfbCompression::Zstd,
                CompressionArg::Lz4 => GfbCompression::Lz4,
            };
            let opts = GfbWriteOptions {
                compression: codec,
                ..Default::default()
            };
            write_gfb(&graph, out_path, &opts)
                .with_context(|| format!("failed to write {}", out_path.display()))?;
        }
        Format::Parquet => {
            let (nodes_path, edges_path) = parquet_stem_paths(out_path);
            write_parquet_graph(&graph, &nodes_path, &edges_path)
                .with_context(|| format!("failed to write parquet to {}", out_path.display()))?;
        }
    }
    sp.finish_with_message(format!("Written → {output}"));

    Ok(())
}

// ── `gf query` ───────────────────────────────────────────────────────────────

fn cmd_query(
    input: &str,
    from: Option<&str>,
    from_label: Option<&str>,
    hops: u32,
    edge_type: Option<&str>,
    direction: DirectionArg,
    output: Option<&str>,
) -> Result<()> {
    let in_path = Path::new(input);
    let in_fmt = detect_format(in_path)?;

    // ── Validate args ────────────────────────────────────────────────────────
    let has_seed = from.is_some() || from_label.is_some();

    // ── Load ─────────────────────────────────────────────────────────────────
    let sp = spinner("Loading…");
    let graph: GraphFrame = match in_fmt {
        Format::Gf => {
            let source = std::fs::read_to_string(in_path)
                .with_context(|| format!("failed to read {}", in_path.display()))?;
            let doc = parse_gf(&source)
                .with_context(|| format!("failed to parse {}", in_path.display()))?;
            doc.to_graph_frame()
                .with_context(|| format!("failed to build graph from {}", in_path.display()))?
        }
        Format::Gfb => {
            read_gfb(in_path).with_context(|| format!("failed to read {}", in_path.display()))?
        }
        Format::Parquet => {
            let (nodes_path, edges_path) = parquet_stem_paths(in_path);
            read_parquet_graph(&nodes_path, &edges_path)
                .with_context(|| format!("failed to read parquet from {}", in_path.display()))?
        }
    };
    sp.finish_with_message(format!(
        "Loaded  {} nodes, {} edges",
        fmt_count(graph.node_count()),
        fmt_count(graph.edge_count()),
    ));

    // ── Query ─────────────────────────────────────────────────────────────────
    let result: GraphFrame = if !has_seed {
        // No traversal — return the full graph.
        graph
    } else {
        let sp = spinner("Querying…");

        // Build seed predicate.
        let seed_expr: Expr = if let Some(id) = from {
            Expr::BinaryOp {
                left: Box::new(Expr::Col {
                    name: COL_NODE_ID.to_owned(),
                }),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal {
                    value: ScalarValue::String(id.to_owned()),
                }),
            }
        } else {
            // from_label — ListContains on _label column.
            let label = from_label.unwrap();
            Expr::ListContains {
                expr: Box::new(Expr::Col {
                    name: COL_NODE_LABEL.to_owned(),
                }),
                item: Box::new(Expr::Literal {
                    value: ScalarValue::String(label.to_owned()),
                }),
            }
        };

        // Edge type spec.
        let etype_spec = match edge_type {
            None | Some("any") => EdgeTypeSpec::Any,
            Some(t) => EdgeTypeSpec::Single(t.to_owned()),
        };

        // Direction.
        let dir = match direction {
            DirectionArg::Out => Direction::Out,
            DirectionArg::In => Direction::In,
            DirectionArg::Both => Direction::Both,
            DirectionArg::Undirected => Direction::None,
        };

        let g = LazyGraphFrame::from_graph(&graph)
            .filter_nodes(seed_expr)
            .expand(etype_spec, hops, dir)
            .collect()
            .context("query execution failed")?;

        sp.finish_with_message(format!(
            "Result  {} nodes, {} edges",
            fmt_count(g.node_count()),
            fmt_count(g.edge_count()),
        ));
        g
    };

    // ── Print summary ────────────────────────────────────────────────────────
    print_graph_summary(&result);

    // ── Write output ─────────────────────────────────────────────────────────
    if let Some(out) = output {
        let out_path = Path::new(out);
        let out_fmt = detect_format(out_path)?;
        let sp = spinner("Writing…");
        match out_fmt {
            Format::Gf => {
                write_gf(&result, out_path)
                    .with_context(|| format!("failed to write {}", out_path.display()))?;
            }
            Format::Gfb => {
                let opts = GfbWriteOptions::default();
                write_gfb(&result, out_path, &opts)
                    .with_context(|| format!("failed to write {}", out_path.display()))?;
            }
            Format::Parquet => {
                let (nodes_path, edges_path) = parquet_stem_paths(out_path);
                write_parquet_graph(&result, &nodes_path, &edges_path).with_context(|| {
                    format!("failed to write parquet to {}", out_path.display())
                })?;
            }
        }
        sp.finish_with_message(format!("Written → {out}"));
    }

    Ok(())
}

/// Print a compact summary of a loaded/result graph to stdout.
fn print_graph_summary(graph: &GraphFrame) {
    println!();
    println!("Nodes:  {:>12}", fmt_count(graph.node_count()));
    println!("Edges:  {:>12}", fmt_count(graph.edge_count()));
    println!();

    // Collect distinct labels and edge types from the graph directly.
    use arrow_array::{Array, ListArray, StringArray};
    use graphframe_core::COL_EDGE_TYPE;

    // Node labels: flatten the List<Utf8> label column and collect distinct values.
    let node_batch = graph.nodes().to_record_batch();
    let label_strings: Vec<String> = if let Some(col) = node_batch.column_by_name(COL_NODE_LABEL) {
        if let Some(list) = col.as_any().downcast_ref::<ListArray>() {
            let values = list.values();
            if let Some(strings) = values.as_any().downcast_ref::<StringArray>() {
                let n = <StringArray as Array>::len(strings);
                let mut seen = std::collections::BTreeSet::new();
                for i in 0..n {
                    if !strings.is_null(i) {
                        seen.insert(strings.value(i).to_owned());
                    }
                }
                seen.into_iter().collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Edge types.
    let edge_batch = graph.edges().to_record_batch();
    let edge_type_strings: Vec<String> = if let Some(col) = edge_batch.column_by_name(COL_EDGE_TYPE)
    {
        if let Some(strings) = col.as_any().downcast_ref::<StringArray>() {
            let n = <StringArray as Array>::len(strings);
            let mut seen = std::collections::BTreeSet::new();
            for i in 0..n {
                if !strings.is_null(i) {
                    seen.insert(strings.value(i).to_owned());
                }
            }
            seen.into_iter().collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    print_tag_list("Labels", &label_strings);
    print_tag_list("Edge types", &edge_type_strings);
}

// ── Formatting helpers ────────────────────────────────────────────────────────

/// Format a count with thousands separators: `1234567` → `"1,234,567"`.
fn fmt_count(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Print a labelled list of tag strings, wrapping at 72 columns.
fn print_tag_list(heading: &str, items: &[String]) {
    if items.is_empty() {
        println!("{heading:<12} (none)");
        return;
    }

    let prefix = format!("{heading:<12} ");
    let indent = " ".repeat(prefix.len());
    let max_col = 72usize;

    let mut line = prefix;
    let mut first = true;
    for item in items {
        let sep = if first { "" } else { "  " };
        let candidate = format!("{sep}{item}");

        if !first && line.len() + candidate.len() > max_col {
            println!("{line}");
            line = format!("{indent}{item}");
            first = true;
        } else {
            line.push_str(&candidate);
            first = false;
        }
    }
    if !line.trim().is_empty() {
        println!("{line}");
    }
    println!("{indent}({} distinct)", fmt_count(items.len()));
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── fmt_count ─────────────────────────────────────────────────────────────

    #[test]
    fn fmt_count_zero() {
        assert_eq!(fmt_count(0), "0");
    }

    #[test]
    fn fmt_count_three_digits() {
        assert_eq!(fmt_count(999), "999");
    }

    #[test]
    fn fmt_count_four_digits() {
        assert_eq!(fmt_count(1_000), "1,000");
    }

    #[test]
    fn fmt_count_millions() {
        assert_eq!(fmt_count(1_234_567), "1,234,567");
    }

    #[test]
    fn fmt_count_exact_power() {
        assert_eq!(fmt_count(1_000_000), "1,000,000");
    }

    // ── inspect ───────────────────────────────────────────────────────────────

    /// `cmd_inspect` should return an error for an unrecognised extension.
    #[test]
    fn inspect_rejects_unknown_extension() {
        let err = cmd_inspect("graph.xyz").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unrecognised extension"), "got: {msg}");
    }

    /// `cmd_inspect` on a nonexistent .gf file should surface an IO error.
    #[test]
    fn inspect_gf_missing_file_is_error() {
        let err = cmd_inspect("/nonexistent/path/graph.gf").unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("failed to read") || msg.contains("No such file"),
            "got: {msg}"
        );
    }

    /// `cmd_inspect` on a nonexistent .gfb file should surface an IO error.
    #[test]
    fn inspect_gfb_missing_file_is_error() {
        let err = cmd_inspect("/nonexistent/path/graph.gfb").unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("failed to read") || msg.contains("No such file"),
            "got: {msg}"
        );
    }

    /// Round-trip: write a small graph to a temp .gfb, run inspect, check output
    /// doesn't panic and counts are echoed.
    #[test]
    fn inspect_gfb_round_trip_produces_output() {
        use arrow_array::builder::{ListBuilder, StringBuilder};
        use arrow_array::{Int8Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema as ArrowSchema};
        use graphframe_core::{
            EdgeFrame, GraphFrame, NodeFrame, COL_EDGE_DIRECTION, COL_EDGE_DST, COL_EDGE_SRC,
            COL_EDGE_TYPE, COL_NODE_ID, COL_NODE_LABEL,
        };
        use std::sync::Arc;

        // Minimal 2-node, 1-edge graph.
        let mut lb = ListBuilder::new(StringBuilder::new());
        lb.values().append_value("Person");
        lb.append(true);
        lb.values().append_value("Company");
        lb.append(true);

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
                    Arc::new(StringArray::from(vec!["alice", "acme"]))
                        as Arc<dyn arrow_array::Array>,
                    Arc::new(lb.finish()) as Arc<dyn arrow_array::Array>,
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
                    Arc::new(StringArray::from(vec!["alice"])) as Arc<dyn arrow_array::Array>,
                    Arc::new(StringArray::from(vec!["acme"])) as Arc<dyn arrow_array::Array>,
                    Arc::new(StringArray::from(vec!["WORKS_AT"])) as Arc<dyn arrow_array::Array>,
                    Arc::new(Int8Array::from(vec![0i8])) as Arc<dyn arrow_array::Array>,
                ],
            )
            .unwrap(),
        )
        .unwrap();

        let graph = GraphFrame::new(nodes, edges).unwrap();
        let path =
            std::env::temp_dir().join(format!("gf-cli-inspect-test-{}.gfb", std::process::id()));
        graph.write_gfb(&path).unwrap();

        // Must not panic or error.
        let result = cmd_inspect(path.to_str().unwrap());
        let _ = std::fs::remove_file(&path);
        assert!(result.is_ok(), "inspect should succeed, got: {:?}", result);
    }

    // ── detect_format ─────────────────────────────────────────────────────────

    #[test]
    fn detect_format_gf() {
        assert_eq!(detect_format(Path::new("graph.gf")).unwrap(), Format::Gf);
    }

    #[test]
    fn detect_format_gfb() {
        assert_eq!(detect_format(Path::new("graph.gfb")).unwrap(), Format::Gfb);
    }

    #[test]
    fn detect_format_parquet() {
        assert_eq!(
            detect_format(Path::new("graph.parquet")).unwrap(),
            Format::Parquet
        );
    }

    #[test]
    fn detect_format_unknown_is_error() {
        let err = detect_format(Path::new("graph.csv")).unwrap_err();
        assert!(format!("{err}").contains("unrecognised extension"));
    }

    // ── parquet_stem_paths ────────────────────────────────────────────────────

    #[test]
    fn parquet_stem_paths_splits_correctly() {
        let (nodes, edges) = parquet_stem_paths(Path::new("/tmp/mygraph.parquet"));
        assert_eq!(nodes, PathBuf::from("/tmp/mygraph-nodes.parquet"));
        assert_eq!(edges, PathBuf::from("/tmp/mygraph-edges.parquet"));
    }

    // ── convert round-trips ───────────────────────────────────────────────────

    fn make_test_graph() -> GraphFrame {
        use arrow_array::builder::{ListBuilder, StringBuilder};
        use arrow_array::{Int8Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema as ArrowSchema};
        use graphframe_core::{
            EdgeFrame, GraphFrame, NodeFrame, COL_EDGE_DIRECTION, COL_EDGE_DST, COL_EDGE_SRC,
            COL_EDGE_TYPE, COL_NODE_ID, COL_NODE_LABEL,
        };
        use std::sync::Arc;

        let mut lb = ListBuilder::new(StringBuilder::new());
        lb.values().append_value("Person");
        lb.append(true);
        lb.values().append_value("Company");
        lb.append(true);

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
                    Arc::new(StringArray::from(vec!["alice", "acme"]))
                        as Arc<dyn arrow_array::Array>,
                    Arc::new(lb.finish()) as Arc<dyn arrow_array::Array>,
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
                    Arc::new(StringArray::from(vec!["alice"])) as Arc<dyn arrow_array::Array>,
                    Arc::new(StringArray::from(vec!["acme"])) as Arc<dyn arrow_array::Array>,
                    Arc::new(StringArray::from(vec!["WORKS_AT"])) as Arc<dyn arrow_array::Array>,
                    Arc::new(Int8Array::from(vec![0i8])) as Arc<dyn arrow_array::Array>,
                ],
            )
            .unwrap(),
        )
        .unwrap();

        GraphFrame::new(nodes, edges).unwrap()
    }

    /// gfb → gf: converted file must parse and have correct counts.
    #[test]
    fn convert_gfb_to_gf_round_trip() {
        let pid = std::process::id();
        let tmp = std::env::temp_dir();
        let gfb_path = tmp.join(format!("gf-conv-test-{pid}.gfb"));
        let gf_path = tmp.join(format!("gf-conv-test-{pid}.gf"));

        make_test_graph().write_gfb(&gfb_path).unwrap();
        let result = cmd_convert(
            gfb_path.to_str().unwrap(),
            gf_path.to_str().unwrap(),
            CompressionArg::None,
        );
        let _ = std::fs::remove_file(&gfb_path);
        assert!(result.is_ok(), "convert gfb→gf failed: {result:?}");

        let source = std::fs::read_to_string(&gf_path).unwrap();
        let _ = std::fs::remove_file(&gf_path);
        let doc = parse_gf(&source).expect("converted .gf must parse");
        assert_eq!(doc.nodes.len(), 2);
        assert_eq!(doc.edges.len(), 1);
    }

    /// gf → gfb: converted file must be inspectable with correct counts.
    #[test]
    fn convert_gf_to_gfb_round_trip() {
        let pid = std::process::id();
        let tmp = std::env::temp_dir();
        let gf_path = tmp.join(format!("gf-conv2-test-{pid}.gf"));
        let gfb_path = tmp.join(format!("gf-conv2-test-{pid}.gfb"));

        // Write a .gf source manually.
        std::fs::write(
            &gf_path,
            "(alice: Person)\n(acme: Company)\nalice -[WORKS_AT]-> acme\n",
        )
        .unwrap();

        let result = cmd_convert(
            gf_path.to_str().unwrap(),
            gfb_path.to_str().unwrap(),
            CompressionArg::Zstd,
        );
        let _ = std::fs::remove_file(&gf_path);
        assert!(result.is_ok(), "convert gf→gfb failed: {result:?}");

        let info = read_gfb_inspect(&gfb_path).unwrap();
        let _ = std::fs::remove_file(&gfb_path);
        assert_eq!(info.node_count, 2);
        assert_eq!(info.edge_count, 1);
        assert_eq!(info.compression, "zstd");
    }

    /// Same format → error.
    #[test]
    fn convert_same_format_is_error() {
        let err = cmd_convert("a.gf", "b.gf", CompressionArg::None).unwrap_err();
        assert!(format!("{err}").contains("same format"));
    }

    /// Unknown extension on input → error.
    #[test]
    fn convert_unknown_input_extension_is_error() {
        let err = cmd_convert("a.csv", "b.gfb", CompressionArg::None).unwrap_err();
        assert!(format!("{err}").contains("unrecognised extension"));
    }

    // ── query ─────────────────────────────────────────────────────────────────

    /// Helper: write a small .gfb fixture and return its path.
    fn write_query_fixture(tag: &str) -> std::path::PathBuf {
        use arrow_array::builder::{ListBuilder, StringBuilder};
        use arrow_array::{Int8Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema as ArrowSchema};
        use graphframe_core::{
            EdgeFrame, GraphFrame, NodeFrame, COL_EDGE_DIRECTION, COL_EDGE_DST, COL_EDGE_SRC,
            COL_EDGE_TYPE, COL_NODE_ID, COL_NODE_LABEL,
        };
        use std::sync::Arc;

        // alice -[KNOWS]-> bob -[KNOWS]-> charlie   acme (isolated)
        let mut lb = ListBuilder::new(StringBuilder::new());
        for label in &["Person", "Person", "Person", "Company"] {
            lb.values().append_value(label);
            lb.append(true);
        }
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
                    Arc::new(StringArray::from(vec!["alice", "bob", "charlie", "acme"]))
                        as Arc<dyn arrow_array::Array>,
                    Arc::new(lb.finish()) as Arc<dyn arrow_array::Array>,
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
                    Arc::new(StringArray::from(vec!["alice", "bob"]))
                        as Arc<dyn arrow_array::Array>,
                    Arc::new(StringArray::from(vec!["bob", "charlie"]))
                        as Arc<dyn arrow_array::Array>,
                    Arc::new(StringArray::from(vec!["KNOWS", "KNOWS"]))
                        as Arc<dyn arrow_array::Array>,
                    Arc::new(Int8Array::from(vec![0i8, 0i8])) as Arc<dyn arrow_array::Array>,
                ],
            )
            .unwrap(),
        )
        .unwrap();

        let graph = GraphFrame::new(nodes, edges).unwrap();
        let path =
            std::env::temp_dir().join(format!("gf-query-fixture-{}-{tag}.gfb", std::process::id()));
        graph.write_gfb(&path).unwrap();
        path
    }

    /// No seed → full graph is returned (4 nodes, 2 edges).
    #[test]
    fn query_no_seed_returns_full_graph() {
        let path = write_query_fixture("no-seed");
        let result = cmd_query(
            path.to_str().unwrap(),
            None,
            None,
            1,
            None,
            DirectionArg::Out,
            None,
        );
        let _ = std::fs::remove_file(&path);
        assert!(result.is_ok(), "query failed: {result:?}");
    }

    /// --from alice --hops 1 should reach alice + bob (1 hop out).
    #[test]
    fn query_from_id_one_hop_out() {
        let path = write_query_fixture("from-id");

        let out_path = std::env::temp_dir().join(format!("gf-query-out-{}.gf", std::process::id()));

        let result = cmd_query(
            path.to_str().unwrap(),
            Some("alice"),
            None,
            1,
            None,
            DirectionArg::Out,
            Some(out_path.to_str().unwrap()),
        );
        let _ = std::fs::remove_file(&path);

        assert!(result.is_ok(), "query failed: {result:?}");

        // Output file must exist and parse with ≥2 nodes (alice + bob).
        let source = std::fs::read_to_string(&out_path).unwrap();
        let _ = std::fs::remove_file(&out_path);
        let doc = parse_gf(&source).expect("output .gf must parse");
        assert!(
            doc.nodes.len() >= 2,
            "expected ≥2 nodes, got {}",
            doc.nodes.len()
        );
        assert!(doc.nodes.iter().any(|n| n.id == "alice"), "alice missing");
        assert!(doc.nodes.iter().any(|n| n.id == "bob"), "bob missing");
    }

    /// --from-label Person --hops 1 seeds from all Person nodes.
    #[test]
    fn query_from_label_seeds_correctly() {
        let path = write_query_fixture("from-label");
        let result = cmd_query(
            path.to_str().unwrap(),
            None,
            Some("Person"),
            1,
            None,
            DirectionArg::Out,
            None,
        );
        let _ = std::fs::remove_file(&path);
        assert!(result.is_ok(), "query failed: {result:?}");
    }

    /// --from alice --hops 2 should reach alice, bob, charlie.
    #[test]
    fn query_two_hops_reaches_charlie() {
        let path = write_query_fixture("two-hops");

        let out_path =
            std::env::temp_dir().join(format!("gf-query-2hop-{}.gf", std::process::id()));

        let result = cmd_query(
            path.to_str().unwrap(),
            Some("alice"),
            None,
            2,
            None,
            DirectionArg::Out,
            Some(out_path.to_str().unwrap()),
        );
        let _ = std::fs::remove_file(&path);
        assert!(result.is_ok(), "query failed: {result:?}");

        let source = std::fs::read_to_string(&out_path).unwrap();
        let _ = std::fs::remove_file(&out_path);
        let doc = parse_gf(&source).expect("output .gf must parse");
        let ids: Vec<&str> = doc.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(
            ids.contains(&"charlie"),
            "charlie not reached at 2 hops; got: {ids:?}"
        );
    }

    /// --edge-type KNOWS --from alice should only traverse KNOWS edges.
    #[test]
    fn query_edge_type_filter() {
        let path = write_query_fixture("edge-type");
        // KNOWS edges exist, so result should be non-trivial.
        let result = cmd_query(
            path.to_str().unwrap(),
            Some("alice"),
            None,
            1,
            Some("KNOWS"),
            DirectionArg::Out,
            None,
        );
        let _ = std::fs::remove_file(&path);
        assert!(
            result.is_ok(),
            "query with edge-type filter failed: {result:?}"
        );
    }

    /// Unknown input extension → error even for query.
    #[test]
    fn query_unknown_extension_is_error() {
        let err = cmd_query("graph.csv", None, None, 1, None, DirectionArg::Out, None).unwrap_err();
        assert!(format!("{err}").contains("unrecognised extension"));
    }
}
