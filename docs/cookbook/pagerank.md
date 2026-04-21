# Rank Nodes With PageRank

## The Problem

Use this recipe when you want a simple importance ranking over the nodes in a graph. This is a good first ranking workflow when you do not need a path answer or a full collected traversal, but you do need a node-oriented score you can inspect, sort, or export.

## Prerequisites

This recipe assumes:

- the graph is already loaded successfully
- you want a node-level result, not another graph-shaped output
- if you use a weight column, it really means something for outgoing influence

The examples below use:

- `examples/data/example_simple.gf`
- `examples/data/example_weighted.gf`

## The Recipe

Basic PageRank run:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")
ranks = g.pagerank()

print("columns:", ranks.column_names())
print("rows:", ranks.len())
print(ranks.to_pyarrow())
```

The result should be a `NodeFrame` with one row per node and a score-bearing column for the PageRank result.

Weighted variant:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_weighted.gf")
ranks = g.pagerank(weight_col="weight")

print("columns:", ranks.column_names())
print("rows:", ranks.len())
```

Use the weighted form only when the weight column genuinely represents how rank should flow along outgoing edges. A generic numeric edge attribute is not automatically a good PageRank weight.

## What To Check

Before trusting the result, check:

- the number of result rows matches the number of nodes in the source graph
- the result really is a `NodeFrame`
- the score column is present
- the values are plausible for the graph size and structure

If you are running on the small shared example graph and get zero rows, that is almost certainly a setup or input problem rather than an interesting algorithmic edge case.

## Side Effects And Limits

PageRank is easy to call, but easy to misuse. The most common mistake is to treat any numeric edge attribute as a legitimate weight column. If the weight does not actually describe the influence you want to propagate, the weighted result may look more precise while being conceptually worse.

It is also worth remembering that the output is not a graph. It is a node-oriented result frame. That is useful, because you can inspect or export it directly, but it also means this recipe is not the right one if the next task is "show me the subgraph around the highest-ranked node." In that situation you usually run PageRank first, inspect the result, choose a node id, and then switch back to a query recipe.

## Related Recipes

If you want one route rather than a ranking, continue with [Find a shortest path between two nodes](shortest-path.md).
If you want group assignments instead of scores, continue with [Detect communities and inspect the result](community-detection.md).
