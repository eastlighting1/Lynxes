# Concepts

This section is for the questions that usually show up before API details matter much. It is about what kind of engine Lynxes is trying to be, what assumptions it makes about graph workloads, and where those assumptions show up in the design.

These pages are not meant to teach installation or walk through a happy path. They are also not reference pages. The point is to give you a decent mental model of the engine before you start judging method names or file formats one by one.

If you are new to the project, reading them in order is probably the easiest way in. The flow is simple: why the project exists, how the data is laid out, what the execution model looks like, and where the design stops being a good fit.

## Read This First

- [Why Lynxes exists](why-lynxes.md)
- [Memory layout and CSR](memory-layout-and-csr.md)
- [Lazy engine](lazy-engine.md)
- [Trade-offs](trade-offs.md)
- [Mutation and preprocessing](mutation-and-preprocessing.md)
- [GNN feature store](gnn-feature-store.md)

## What These Pages Try To Answer

The questions here are mostly the ones people ask when they are still deciding whether the engine makes sense at all:

- Why did Lynxes choose Arrow-native storage instead of building on top of a dataframe layer?
- Why is adjacency treated as a first-class structure instead of something reconstructed when needed?
- What does `lazy()` mean beyond "it runs later"?
- What kinds of workloads fit this engine, and what kinds do not?

If you are looking for step-by-step usage, the quickstarts and guides are the better next stop. If you are looking for exact signatures or file format details, go to the reference pages.
