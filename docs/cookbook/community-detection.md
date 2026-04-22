# Detect Communities And Inspect The Result

## The Problem

Use this recipe when you want to partition the nodes of a graph into detected groups and then inspect or export those assignments. This is the right pattern when the output you need is "which nodes belong together?" rather than "what is the exact shortest path?" or "what is the immediate neighborhood around this seed?"

This recipe is often useful one step later than users expect. Community detection rarely answers the whole question by itself. What it gives you is a node-level assignment frame that makes the next question possible: which clusters are large, which ids ended up together, and what subgraph should I inspect next?

## Prerequisites

This recipe assumes:

- the graph is large enough that community structure is meaningful
- you want a node-level assignment result
- you are prepared to inspect the returned node frame rather than expecting another `GraphFrame`

The example below uses `examples/data/example_complex.gf`.

That example matters because community detection is easiest to misunderstand on tiny graphs. The algorithm can always return assignments, but not every graph has meaningful community structure.

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

If you need a fast first summary instead of the full assignment table, count rows per community immediately after the run:

```python
assignments = communities.to_pyarrow()
community_ids = assignments["community_id"].to_pylist()

print("distinct communities:", len(set(community_ids)))
```

For most practical workflows, this second summary is not optional. The raw assignment frame tells you where each node landed. The distinct-count or per-community-size view tells you whether the result is interesting enough to continue exploring.

## What To Check

The first checks are simple:

- the result contains one row per node
- a `community_id` column is present
- multiple community ids appear when the graph really does have more than one cluster

If the graph is tiny or structurally trivial, do not over-interpret the result. Community detection is most useful when there is actually something to partition.

One very practical check is to pick one returned community id and inspect a few `_id` values attached to it. That gives you a much faster sense of whether the grouping looks meaningful than staring at the full table.

If you are using the result downstream, also check that the row count still matches the input graph after any preprocessing step. Community assignment frames are only useful if you are clear about which graph snapshot they came from.

## Side Effects And Limits

The most common mistake here is to stop too early. Community detection rarely ends with "I have a `community_id` column." In practice, the next step is usually one of these:

- summarize the size of each community
- inspect the nodes assigned to one chosen group
- export the assignments for downstream analysis
- use the assignments to drive a follow-up traversal

This means the raw assignment frame is often only the middle of the workflow. It is the result you wanted from the algorithm, but not necessarily the last artifact you care about.

It is also worth being careful about expectations. Community detection is not a path algorithm and not a hard schema guarantee. The output is a grouping derived from graph structure, so the usefulness of the result depends on the shape of the graph you fed into it.

That means this recipe is often a poor fit for extremely small graphs, graphs dominated by one central hub, or graphs whose edge semantics do not actually imply community structure. In those cases the algorithm will still return assignments, but the result may be much less informative than the tidy output shape suggests.

Another common mistake is to stop at the assignments table and treat it like the end product. In practice, community detection is often a branching point. You may export the assignments, inspect one cluster, summarize group sizes, or use a chosen `community_id` to drive a follow-up graph query. The recipe is best understood as "get to the assignment frame cleanly and verify it," not "all downstream interpretation is now solved."

## Related Recipes

If you want node ranking instead of grouping, continue with [Rank nodes with PageRank](pagerank.md).
If you want to export the result and validate it after conversion, continue with [Save a graph and validate a round-trip](export-and-roundtrip.md).
