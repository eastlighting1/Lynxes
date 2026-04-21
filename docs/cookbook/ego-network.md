# Build An Ego Network Around One Seed Node

## The Problem

Use this recipe when you want a small neighborhood around one node and you do not need a full graph algorithm yet. This is often the fastest way to answer questions like "what is immediately connected to this entity?" or "what does the local graph around this id look like before I export or inspect it further?"

## Prerequisites

This recipe assumes:

- you are working from a repository checkout
- the shared example graph exists at `examples/data/example_simple.gf`
- the seed node id you want to use is present in the graph

It also assumes that a subgraph result is what you want. `expand(...).collect()` returns a `GraphFrame`, not just a list of node ids.

## The Recipe

Python, one hop:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

ego = (
    g.lazy()
    .filter_nodes(lx.col("_id") == "alice")
    .expand(hops=1, direction="out")
    .collect()
)

print("nodes:", ego.node_count())
print("edges:", ego.edge_count())
print("ids:", ego.nodes().to_pyarrow()["_id"].to_pylist())
```

On the shared example graph, this should produce a result shaped like:

```text
nodes: 2
edges: 1
ids: ['alice', 'bob']
```

If you want a wider ego network, increase the hop count and, if needed, restrict by edge type:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

ego_2hop = (
    g.lazy()
    .filter_nodes(lx.col("_id") == "alice")
    .expand(edge_type="KNOWS", hops=2, direction="out")
    .collect()
)

print("nodes:", ego_2hop.node_count())
print("edges:", ego_2hop.edge_count())
print("ids:", ego_2hop.nodes().to_pyarrow()["_id"].to_pylist())
```

On the same example graph, the two-hop version should bring in `charlie` as well.

CLI version:

```bash
cargo run -p lynxes-cli -- query examples/data/example_simple.gf --from alice --hops 1 --direction out --view info
```

## What To Check

Before treating the result as correct, check these things:

- the seed node id is present in the result
- the neighbor set is plausible by eye
- the edge count is not zero unless that is genuinely expected
- the result is a graph-shaped output rather than only a node list

If you use the one-hop version on the shared example and get more than one outward edge, you are probably not on the expected input graph.

## Side Effects And Limits

Ego-network queries expand quickly as hop count grows. A one-hop query is often easy to reason about; a two-hop query can already become noticeably larger; a three-hop query on a dense graph may stop being a "small local view" at all.

There is also a modeling choice hidden in the arguments. Direction and edge type matter. If you forget to restrict by edge type on a graph with multiple relationship kinds, you may collect a much broader subgraph than you intended. If you use `both` or `undirected`, you are explicitly broadening the neighborhood and should expect the result to grow faster.

## Related Recipes

If you want to turn this neighborhood into a more exact traversal query, continue with the query guides.
If you want to move from local exploration to a path answer, continue with [Find a shortest path between two nodes](shortest-path.md).
