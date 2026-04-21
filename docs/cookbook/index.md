# Cookbook

The cookbook is for the moment when a user already understands the basics and now wants to get a specific job done without reading a long introduction first.

That makes this section different from both guides and reference pages. A guide is supposed to teach. A reference page is supposed to answer exact interface questions. A cookbook page should do something narrower and more practical: name a concrete problem, state the conditions under which the recipe works, give a code path that can be used immediately, and warn about the places where that recipe stops being a good idea.

The best way to read this section is not front to back. It is to scan for the task you are actually trying to accomplish.

## Query-Oriented Recipes

- [Build an ego network around one seed node](ego-network.md)
- [Find a shortest path between two nodes](shortest-path.md)

## Algorithm Recipes

- [Rank nodes with PageRank](pagerank.md)
- [Detect communities and inspect the result](community-detection.md)

## Export And Interop Recipes

- [Save a graph and validate a round-trip](export-and-roundtrip.md)

## What To Expect From A Cookbook Page

Each recipe in this section should answer the same practical questions:

- what problem is this solving right now?
- what graph shape or environment does it assume?
- what is the shortest complete code path that works?
- what should I check before I trust the result?
- what are the common limits, side effects, or failure modes?

If you are looking for a first-time learning path, go back to the guides section.
If you are looking for exact argument names, return types, or full API surface coverage, use the reference pages.
