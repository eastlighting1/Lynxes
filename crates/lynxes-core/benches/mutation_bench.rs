use std::sync::Arc;
use std::time::Duration;

use arrow_array::builder::{ListBuilder, StringBuilder};
use arrow_array::{ArrayRef, Int8Array, ListArray, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema as ArrowSchema};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};

use lynxes_core::{
    Direction, EdgeFrame, GraphFrame, NodeFrame, COL_EDGE_DIRECTION, COL_EDGE_DST, COL_EDGE_SRC,
    COL_EDGE_TYPE, COL_NODE_ID, COL_NODE_LABEL,
};

fn node_id(idx: u32) -> String {
    format!("n{idx}")
}

fn labels_array(count: usize) -> ListArray {
    let mut builder = ListBuilder::new(StringBuilder::new());
    for _ in 0..count {
        builder.values().append_value("Person");
        builder.append(true);
    }
    builder.finish()
}

fn node_frame_range(start: u32, count: u32) -> NodeFrame {
    let ids = (start..start + count).map(node_id).collect::<Vec<_>>();
    let schema = Arc::new(ArrowSchema::new(vec![
        Field::new(COL_NODE_ID, DataType::Utf8, false),
        Field::new(
            COL_NODE_LABEL,
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            false,
        ),
    ]));

    NodeFrame::from_record_batch(
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(ids)) as ArrayRef,
                Arc::new(labels_array(count as usize)) as ArrayRef,
            ],
        )
        .unwrap(),
    )
    .unwrap()
}

fn edge_frame_from_pairs(pairs: &[(u32, u32)]) -> EdgeFrame {
    let srcs = pairs
        .iter()
        .map(|(src, _)| node_id(*src))
        .collect::<Vec<_>>();
    let dsts = pairs
        .iter()
        .map(|(_, dst)| node_id(*dst))
        .collect::<Vec<_>>();
    let len = pairs.len();
    let schema = Arc::new(ArrowSchema::new(vec![
        Field::new(COL_EDGE_SRC, DataType::Utf8, false),
        Field::new(COL_EDGE_DST, DataType::Utf8, false),
        Field::new(COL_EDGE_TYPE, DataType::Utf8, false),
        Field::new(COL_EDGE_DIRECTION, DataType::Int8, false),
    ]));

    EdgeFrame::from_record_batch(
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(srcs)) as ArrayRef,
                Arc::new(StringArray::from(dsts)) as ArrayRef,
                Arc::new(StringArray::from(vec!["E"; len])) as ArrayRef,
                Arc::new(Int8Array::from(vec![Direction::Out.as_i8(); len])) as ArrayRef,
            ],
        )
        .unwrap(),
    )
    .unwrap()
}

fn graph_with_pairs(node_count: u32, pairs: &[(u32, u32)]) -> GraphFrame {
    let nodes = node_frame_range(0, node_count);
    let edges = edge_frame_from_pairs(pairs);
    GraphFrame::new(nodes, edges).unwrap()
}

fn empty_graph(node_count: u32) -> GraphFrame {
    graph_with_pairs(node_count, &[])
}

fn hub_graph(node_count: u32, hub_degree: u32) -> GraphFrame {
    let mut pairs = Vec::with_capacity(hub_degree as usize);
    for dst in 1..=hub_degree.min(node_count.saturating_sub(1)) {
        pairs.push((0, dst));
    }
    graph_with_pairs(node_count, &pairs)
}

fn bench_single_edge_insert_100k(c: &mut Criterion) {
    let insert_count = 100_000u32;
    let ids = Arc::new((1..=insert_count).map(node_id).collect::<Vec<_>>());

    let mut group = c.benchmark_group("mutation_single_edge_insert");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(4));
    group.throughput(Throughput::Elements(insert_count as u64));
    group.bench_function("100k", |b| {
        let ids = Arc::clone(&ids);
        b.iter_batched(
            || empty_graph(insert_count + 1).into_mutable(),
            |mut graph| {
                for dst in ids.iter() {
                    graph.add_edge("n0", dst).unwrap();
                }
                black_box(graph.out_neighbors(0).unwrap().count())
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn bench_batch_node_insert_100k(c: &mut Criterion) {
    let batch_count = 100_000u32;
    let mut group = c.benchmark_group("mutation_batch_node_insert");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(4));
    group.throughput(Throughput::Elements(batch_count as u64));
    group.bench_function("100k", |b| {
        b.iter_batched(
            || {
                (
                    empty_graph(1).into_mutable(),
                    node_frame_range(1, batch_count),
                )
            },
            |(mut graph, batch)| {
                graph.add_nodes_batch(batch).unwrap();
                black_box(graph.freeze().unwrap().node_count())
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn bench_frozen_chunk_neighbor_lookup(c: &mut Criterion) {
    let chunk_count = 64u32;
    let chunk_width = 1024u32;
    let edge_count = chunk_count * chunk_width;
    let ids = Arc::new((1..=edge_count).map(node_id).collect::<Vec<_>>());

    let mut group = c.benchmark_group("mutation_frozen_chunk_neighbor_lookup");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(4));
    group.throughput(Throughput::Elements(edge_count as u64));
    group.bench_function("64x1024", |b| {
        let ids = Arc::clone(&ids);
        b.iter_batched(
            || {
                let mut graph = empty_graph(edge_count + 1).into_mutable();
                for dst in ids.iter() {
                    graph.add_edge("n0", dst).unwrap();
                }
                graph
            },
            |graph| black_box(graph.out_neighbors(0).unwrap().count()),
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn bench_compact_1m_edges(c: &mut Criterion) {
    let node_count = 100_000u32;
    let edge_count = 1_000_000u32;
    let edge_pairs = Arc::new(
        (0..edge_count)
            .map(|i| {
                (
                    i % node_count,
                    (i.wrapping_mul(31).wrapping_add(7)) % node_count,
                )
            })
            .collect::<Vec<_>>(),
    );
    let ids = Arc::new((0..node_count).map(node_id).collect::<Vec<_>>());

    let mut group = c.benchmark_group("mutation_compact");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Elements(edge_count as u64));
    group.bench_function("1m_edges", |b| {
        let edge_pairs = Arc::clone(&edge_pairs);
        let ids = Arc::clone(&ids);
        b.iter_batched(
            || {
                let mut graph = empty_graph(node_count).into_mutable();
                for &(src, dst) in edge_pairs.iter() {
                    graph
                        .add_edge(&ids[src as usize], &ids[dst as usize])
                        .unwrap();
                }
                graph
            },
            |graph| {
                graph.compact().unwrap();
                black_box(graph.out_neighbors(0).unwrap().count())
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn bench_delete_hub_node_degree_10000(c: &mut Criterion) {
    let degree = 10_000u32;
    let mut group = c.benchmark_group("mutation_delete_hub_node");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(4));
    group.throughput(Throughput::Elements(degree as u64));
    group.bench_function("degree_10000", |b| {
        b.iter_batched(
            || hub_graph(degree + 1, degree).into_mutable(),
            |mut graph| {
                graph.delete_node("n0").unwrap();
                black_box(graph.out_neighbors(0).unwrap().count())
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_single_edge_insert_100k,
    bench_batch_node_insert_100k,
    bench_frozen_chunk_neighbor_lookup,
    bench_compact_1m_edges,
    bench_delete_hub_node_degree_10000,
);
criterion_main!(benches);
