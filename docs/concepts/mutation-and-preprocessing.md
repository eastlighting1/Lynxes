# Mutation And Preprocessing

Lynxes is not a transactional graph database, and it is not trying to be one. The engine is still built around Arrow-owned batches, CSR-backed adjacency, and immutable snapshots that are easy to read quickly. That does not mean graph mutation is absent. It means mutation is treated as a preparation stage rather than as the center of the product.

That distinction matters. A lot of graph work begins with a graph that is technically valid but not yet shaped for the job you actually want to run. Recommendation pipelines often need to drop stale edges, remove low-value hubs, inject temporary super-nodes, or create negative edges for a training task. GNN pipelines often need a cleaned, sampled, or lightly rewritten graph before they hand anything to PyTorch. In those situations, "mutation" is really shorthand for graph preprocessing.

## Where `MutableGraphFrame` Fits

`GraphFrame` remains the stable eager graph object. It is the thing you inspect, query, export, and hand to eager algorithms. `MutableGraphFrame` is a side path you enter deliberately when you need to rewrite the graph before going back to a regular eager snapshot.

The intended workflow looks like this:

1. Load or build a `GraphFrame`.
2. Call `into_mutable()`.
3. Apply a bounded set of rewrites such as adding nodes, deleting edges, or replacing one node with another.
4. Call `freeze()` and return to an immutable `GraphFrame`.

That last step is not incidental. It is the boundary that turns a work-in-progress preprocessing graph back into a normal engine object.

## Why The Engine Uses A Mutable Wrapper Instead Of In-Place Graph Edits

The engine still wants fast, predictable reads. That means existing readers should not lose their snapshot just because a preprocessing step is in flight. The current mutation architecture keeps a base snapshot, accumulates changes in delta structures, and only publishes a new snapshot when compaction or freezing happens.

This is a very different model from "just update the graph in place." It keeps the read path close to the rest of Lynxes:

- a stable base CSR still exists
- appended edges can be buffered before compaction
- compaction publishes a new snapshot instead of mutating the old one out from under readers

The result is not free. Mutation has more machinery than a purely immutable graph, and repeated preprocessing work still benefits from batching. But the model lines up with the engine's existing layout instead of fighting it.

## What Mutation Is Good For In Practice

The strongest use cases are all preprocessing-flavored:

- dropping noisy or stale relationships before a ranking task
- removing structurally problematic nodes, such as known bots or dead accounts
- appending temporary supervision edges for link prediction experiments
- rebuilding a graph after one round of node cleanup, then passing the frozen result into a normal lazy or eager workflow

These are all finite rewrite phases. You do the rewrite, freeze, and continue with the rest of the pipeline.

## What Mutation Is Not Trying To Be

This architecture is not meant to compete with a graph store that handles continuous concurrent writes, transaction logs, and row-by-row operational updates all day long. If your workload is mostly "many sessions keep editing the graph forever," Lynxes is still not the natural fit.

What changed is narrower and more useful than that: Lynxes can now own a preprocessing step without forcing you to leave the engine just to clean up or reshape a graph.

## How This Connects To GNN And KG Workflows

The most immediate payoff is in machine-learning and extraction pipelines.

For GNN work, the graph you want to train on is often not the same graph you ingested. Maybe you want to remove edges outside a time window, prune high-degree noise, or add negative samples for a link task. `MutableGraphFrame` gives that stage a place to live before the graph turns back into an Arrow/CSR snapshot.

For KG work, the payoff is a little different. Pattern matching itself stays lazy, but a preprocessing stage can still make a knowledge graph easier to search or sample. If you need to drop irrelevant relation types or create a smaller training slice before repeated pattern queries, the mutation layer can sit in front of that workflow.

## The Important Boundary

The clean mental model is simple:

- `GraphFrame` is for stable graph work.
- `MutableGraphFrame` is for bounded preprocessing.
- `freeze()` is the handoff back to the rest of Lynxes.

If you keep that boundary in mind, the mutation story stays consistent with the rest of the engine rather than feeling like a different product bolted onto the side.
