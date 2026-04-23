# Build An Ego Network Around One Seed Node

## The Problem

Use this recipe when you want a small neighborhood around one node and you do not need a full graph algorithm yet. This is often the fastest way to answer questions like "what is immediately connected to this entity?" or "what does the local graph around this id look like before I export or inspect it further?"

It is a good recipe precisely because it stays local. Before you run PageRank, shortest path, community detection, or any kind of export pipeline, it is often useful to look at one bounded piece of the graph and confirm that the graph means what you think it means. An ego-network query is one of the cheapest ways to do that.

## Prerequisites

This recipe assumes:

- you are working from a repository checkout
- the shared example graph exists at `examples/data/example_simple.gf`
- the seed node id you want to use is present in the graph

It also assumes that a subgraph result is what you want. `expand(...).collect()` returns a `GraphFrame`, not just a list of node ids.

One more practical assumption is hidden in the example output below: the sample graph is intentionally small and directed. If you run the same recipe on a denser graph, the result can grow much more quickly than the numbers shown here suggest.

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
print("ids:", ego.nodes().ids())
```

On the shared example graph, this should produce a result shaped like:

```text
nodes: 3
edges: 2
ids: ['alice', 'bob', 'diana']
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
print("ids:", ego_2hop.nodes().ids())
```

On the same example graph, the two-hop version should bring in `charlie` while keeping the `KNOWS`-reachable neighborhood:

```text
nodes: 4
edges: 3
ids: ['alice', 'bob', 'charlie', 'diana']
```

That difference between one hop and two hops is the first real decision point in many graph workflows. One hop is often an inspection tool. Two hops is where you start drifting toward "small induced subgraph" rather than "local neighborhood." That can still be exactly what you want, but it is worth noticing.

CLI version:

```bash
cargo run -p lynxes-cli -- query examples/data/example_simple.gf --from alice --hops 1 --direction out --view info
```

If you expect to export or save the ego network afterward, do that only after this first shape check. A wrong seed id can still produce a perfectly well-formed subgraph, which makes it easy to save the wrong thing confidently.

## What To Check

Before treating the result as correct, check these things:

- the seed node id is present in the result
- the neighbor set is plausible by eye
- the edge count is not zero unless that is genuinely expected
- the result is a graph-shaped output rather than only a node list

If you use the one-hop version on the shared example and do not see both `bob` and `diana`, you are probably not on the expected input graph.

If you are doing this on your own graph, it is also worth comparing the result against one direct inspection query or one manual sample row. Ego-network recipes are often used early in analysis, and a quiet seed mismatch can lead you into the wrong part of the graph very quickly.

Another useful check is to inspect the returned edge types once before you move on. If your graph mixes relationship kinds heavily, an ego network can look structurally plausible while still including the wrong semantics.

## Side Effects And Limits

Ego-network queries expand quickly as hop count grows. A one-hop query is often easy to reason about; a two-hop query can already become noticeably larger; a three-hop query on a dense graph may stop being a "small local view" at all.

There is also a modeling choice hidden in the arguments. Direction and edge type matter. If you forget to restrict by edge type on a graph with multiple relationship kinds, you may collect a much broader subgraph than you intended. If you use `both` or `undirected`, you are explicitly broadening the neighborhood and should expect the result to grow faster.

One more practical limit is that this recipe assumes you know the seed id already. If the first problem is "which ids should I start from?", it is often better to run a small filter first, inspect the resulting node frame, and only then switch to ego-network extraction.

There is also a common workflow trap here. An ego network is excellent for inspection and for saving a bounded subgraph, but it is not automatically the right input to every downstream algorithm. If the next stage expects the global graph structure, do not silently replace the original graph with a local ego network just because it is easier to reason about.

## Related Recipes

If you want to turn this neighborhood into a more exact traversal query, continue with the query guides.
If you want to move from local exploration to a path answer, continue with [Find a shortest path between two nodes](shortest-path.md).
