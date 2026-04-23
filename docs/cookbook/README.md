# Cookbook

The cookbook is where Lynxes stops trying to teach the whole product and starts helping with concrete jobs. By the time a reader is here, they usually do not need another overview of `GraphFrame`, lazy execution, or reserved columns. They have a graph in hand and a narrower question:

- how do I pull out one neighborhood around this seed?
- how do I get one route instead of a whole collected subgraph?
- how do I rank or group nodes and then inspect the result quickly?
- how do I convert a graph and make sure the result is still trustworthy?

That is what these pages are for. A cookbook page should describe one recognizable task, make its assumptions explicit, show the shortest working path, and say where that path stops being a good fit.

This section is deliberately not organized like a tutorial. You are not expected to read it in order. The normal way to use the cookbook is to scan for the job you are trying to do right now, copy the recipe, and then adapt it carefully to your own graph.

If you are not sure which family of recipe you need yet, use this quick split:

- use a query-oriented recipe when the output should still be graph structure
- use an algorithm recipe when the output should be a score, route, or assignment result
- use an export recipe when the practical question is whether a converted graph is still safe to trust

## Before You Copy A Recipe

Three habits make these pages much more reliable in practice.

First, keep the example graph in mind. Most recipes here use one of the shared repository fixtures so that the result shape is easy to recognize. When you adapt a recipe to your own data, expect the exact ids, counts, and rankings to change even if the code path is still correct.

Second, check the result immediately. Cookbook pages are not the place to hide validation. Every page in this section includes a short "What To Check" section because a copied recipe is only useful if you can tell whether it actually worked on your graph.

Third, notice whether the result is still graph-shaped. Some Lynxes operations return another `GraphFrame`. Others return a `NodeFrame`, a path object, or an Arrow-facing result that is meant to leave the graph layer. That distinction is often more important than the algorithm name.

## Query-Oriented Recipes

- [Build an ego network around one seed node](ego-network.md)
- [Find a shortest path between two nodes](shortest-path.md)

## Algorithm Recipes

- [Rank nodes with PageRank](pagerank.md)
- [Detect communities and inspect the result](community-detection.md)

## Export And Interop Recipes

- [Save a graph and validate a round-trip](export-and-roundtrip.md)

## What To Expect From A Cookbook Page

Each recipe in this section should answer the same practical questions, in roughly the same order:

- what problem is this solving right now?
- what graph shape or environment does it assume?
- what is the shortest complete code path that works?
- what should I check before I trust the result?
- what are the common limits, side effects, or failure modes?

If a page starts spending more time introducing Lynxes than solving the task, it probably belongs in the guides section instead. If it starts listing every accepted keyword argument and return type, it probably belongs in the reference section instead.

If you are looking for a first-time learning path, go back to the guides section. If you are looking for exact argument names, return types, or full API surface coverage, use the reference pages.
