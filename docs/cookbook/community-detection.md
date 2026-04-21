# Detect Communities And Inspect The Result

## The Problem

Use this recipe when you want to partition the nodes of a graph into detected groups and then inspect or export those assignments. This is the right pattern when the output you need is "which nodes belong together?" rather than "what is the exact shortest path?" or "what is the immediate neighborhood around this seed?"

## Prerequisites

This recipe assumes:

- the graph is large enough that community structure is meaningful
- you want a node-level assignment result
- you are prepared to inspect the returned node frame rather than expecting another `GraphFrame`

The example below uses `examples/data/example_complex.gf`.

## The Recipe

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_complex.gf")
communities = g.community_detection()

print("columns:", communities.column_names())
print("rows:", communities.len())
print(communities.to_pyarrow())
```

The returned node frame should include a `community_id` column. The row count should line up with the number of nodes in the source graph.

## What To Check

The first checks are simple:

- the result contains one row per node
- a `community_id` column is present
- multiple community ids appear when the graph really does have more than one cluster

If the graph is tiny or structurally trivial, do not over-interpret the result. Community detection is most useful when there is actually something to partition.

## Side Effects And Limits

The most common mistake here is to stop too early. Community detection rarely ends with "I have a `community_id` column." In practice, the next step is usually one of these:

- summarize the size of each community
- inspect the nodes assigned to one chosen group
- export the assignments for downstream analysis
- use the assignments to drive a follow-up traversal

This means the raw assignment frame is often only the middle of the workflow. It is the result you wanted from the algorithm, but not necessarily the last artifact you care about.

It is also worth being careful about expectations. Community detection is not a path algorithm and not a hard schema guarantee. The output is a grouping derived from graph structure, so the usefulness of the result depends on the shape of the graph you fed into it.

## Related Recipes

If you want node ranking instead of grouping, continue with [Rank nodes with PageRank](pagerank.md).
If you want to export the result and validate it after conversion, continue with [Save a graph and validate a round-trip](export-and-roundtrip.md).
