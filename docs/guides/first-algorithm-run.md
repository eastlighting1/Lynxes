# Your First Algorithm Run

This guide is the first algorithm-oriented path for Lynxes.
The goal is not to survey every algorithm that exists on the surface. The goal is to show the difference between a lazy traversal workflow and a direct graph algorithm call, using small examples that are easy to validate.

If you have already worked through the first-query guide, this page should feel like a natural next step. You have already seen how to build a subgraph lazily. Now the contrast is that an eager algorithm call asks the engine for an answer directly.

## Start With The Smallest Useful Cases

This guide uses two shared example files:

- `examples/data/example_simple.gf`
- `examples/data/example_weighted.gf`

The first is useful for small structural checks. The second is useful because pathfinding becomes more interesting once edge weights are involved.

## Step 1: Run Shortest Path

Start with the weighted example:

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

On the shared weighted example, you should see:

```text
['seoul', 'daejeon', 'busan']
```

The important thing to notice is what the API feels like. This is not a plan waiting for `collect()`. It is an eager request: find the path now and return it now.

## Step 2: Run PageRank

Now switch to the simple example:

```python
g = lx.read_gf("examples/data/example_simple.gf")
ranked = g.pagerank()

print("columns:", ranked.column_names())
print("rows:", ranked.len())
print(ranked.head(5, sort_by="pagerank", descending=True))
```

You should see:

```text
columns: ['_id', '_label', 'pagerank']
rows: 5
```

The preview should show one row per node, sorted by score when you ask for it that way, and a score-bearing `pagerank` column. That is the key thing to verify. PageRank is an eager algorithm, but the result still comes back in a frame-like surface that can be inspected or exported.

## Step 3: Compare The Feel Of Querying And Algorithms

At this point it is worth pausing on the difference in interaction style.

When you use the lazy API, you usually:

- start with a graph
- build a query
- collect a result shape

When you use an eager algorithm, you usually:

- start with a graph
- call the algorithm directly
- inspect the returned answer or frame

Lynxes keeps both surfaces because these are genuinely different jobs. A shortest-path request is clearer as a direct call. A traversal that should remain inspectable and composable before materialization is better handled through the lazy layer.

## Step 4: Try Community Detection

Now use the larger example:

```python
g = lx.read_gf("examples/data/example_complex.gf")
communities = g.community_detection()

print("columns:", communities.column_names())
print("rows:", communities.len())
```

You should see output with one row per node and a column that identifies the assigned community:

```text
columns: ['_id', '_label', 'community_id']
rows: 106
```

The exact values are less important for a first walkthrough than the result shape. What you want to verify here is that community detection runs successfully and produces a node-oriented result frame rather than a graph or a raw Python list.

## How To Tell If The Algorithm Worked

For a beginner guide, success checks matter more than theoretical detail. A useful checklist is:

- shortest path returns a valid route object or path description
- PageRank returns one row per node
- community detection returns one row per node with community assignments

If those things are true, you already know the eager algorithm surface is functioning in the way the rest of the docs assume.

## What This Guide Is Not Trying To Do

This page is not the full algorithm reference. It is not trying to explain every keyword argument, every weighting option, or every implementation detail. It is doing something simpler and more important for a first encounter: showing that Lynxes has a direct algorithm surface and that the outputs remain usable in the same broader graph-and-frame workflow.

If you want exact API signatures after this, use the Python reference pages.
If you want task-focused examples such as PageRank or shortest-path recipes, the cookbook is the better next stop.

## Where To Go Next

If you want more detail about specific algorithms, continue with the cookbook pages for shortest path, PageRank, and community detection.
If you want to revisit query-building instead of direct algorithm calls, go back to [Your First Graph Query](first-graph-query.md).
