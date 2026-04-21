# Find A Shortest Path Between Two Nodes

## The Problem

Use this recipe when you already know the source and destination nodes and want a direct path answer instead of a full subgraph. This is the right pattern when the result you need is a route, not an ego network or a collected traversal frontier.

## Prerequisites

This recipe assumes:

- the graph contains both endpoint node ids
- you know whether the path should be weighted or unweighted
- for weighted pathfinding, the edge weight column is present and meaningful

The examples below use:

- `examples/data/example_simple.gf` for the unweighted case
- `examples/data/example_weighted.gf` for the weighted case

## The Recipe

Unweighted shortest path:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
path = g.shortest_path("alice", "charlie")
print(path)
```

On the shared simple graph, the returned path should clearly go through `bob`.

Weighted shortest path:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_weighted.gf")
path = g.shortest_path(
    "seoul",
    "busan",
    weight_col="weight",
    edge_type="ROUTE",
    direction="out",
)
print(path)
```

This version uses the `weight` edge column and restricts traversal to `ROUTE` edges. That combination matters. If the graph contains multiple edge types or irrelevant edges, leaving out the edge-type filter can silently produce a different route than the one you meant to ask for.

## What To Check

When the result comes back, check:

- the first node is the requested source
- the last node is the requested destination
- any intermediate node ids are plausible for the graph you loaded
- weighted and unweighted calls do not accidentally produce identical paths just because the weight column was ignored or misspelled

On the simple shared graph, the unweighted result should not jump directly from `alice` to `charlie`.

## Side Effects And Limits

Shortest path answers are direct, but they are easy to misread if the graph model is not clear. Direction matters. Edge type matters. Weight columns matter. If any of those are wrong, the path may still be valid for the query you actually ran while still being wrong for the problem you thought you were solving.

There is also an operational difference between pathfinding and lazy traversal. A shortest-path call gives you an answer, not a subgraph. If what you really need is the surrounding neighborhood or the traversed edges as a graph result, use a query-oriented recipe instead of forcing the path API into that role.

If no path exists, handle that as a legitimate graph outcome rather than treating it as a parser or runtime failure.

## Related Recipes

If you want a local neighborhood rather than one route, go back to [Build an ego network around one seed node](ego-network.md).
If you want scoring rather than path answers, continue with [Rank nodes with PageRank](pagerank.md).
