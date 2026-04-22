# GNN Feature Store

Lynxes does not ship a full neural training framework. It is not trying to replace PyTorch Geometric, DGL, or the model code that already exists in that ecosystem. The part Lynxes cares about is the stage before training: getting graph structure and feature rows into a shape those libraries can consume without a detour through a slower, more ad hoc stack.

That is why the GNN-facing surface is built around a small set of bridge operations rather than a large training API.

## The Problem This Layer Solves

A graph training pipeline usually needs two things:

- graph structure in a tensor-friendly form such as COO
- feature rows gathered in exactly the sampled order the model expects

If you start from a graph library that is comfortable with topology but awkward with columnar payloads, you end up rebuilding feature tables somewhere else. If you start from a dataframe-like tool, you often have to reconstruct graph structure repeatedly. Lynxes already owns both pieces: Arrow batches for payloads and CSR for adjacency. The GNN layer is just the point where those two assets become exportable in the right shape.

## The Small Set Of Bridge Primitives

The current bridge revolves around four ideas.

### `to_coo()`

This exposes the graph topology as source and destination arrays in the same compact local index space used by the `EdgeFrame` adjacency structure. It is not meant to be the final tensor object by itself. It is the structural handoff point.

### `gather_rows()`

This takes an explicit list of row ids and returns the corresponding `RecordBatch`. In a training pipeline, that matters more than it may first appear. Sampled neighborhoods are rarely in the same order as the original node table. `gather_rows()` is the thing that lets you keep the sampled order instead of reconstructing it manually in Python.

### `sample_neighbors()`

This is the minibatch graph sampler. It gives you a sampled subgraph in the compact graph index space together with the `node_row_ids` you need for feature gather.

### `random_walk()`

This covers the walk-based side of graph preprocessing and representation learning. The return value is intentionally light: just the compact node index sequences. That is enough for downstream transition into Python-side training code.

## Why Arrow Matters Here

The feature side of this bridge is Arrow-native from the start. That means a node feature batch does not need to become a bespoke Python object before it can move into the next layer. The immediate bridge is usually PyArrow, and from there the pipeline can move into NumPy, Torch, or another consumer.

This is not magic zero-cost interoperability in every single case. Some downstream libraries still want ownership or a specific layout. But it is much closer to a direct handoff than rebuilding features from a row-oriented structure or repeatedly materializing Python lists.

## Why The Index Space Is Worth Calling Out

The bridge uses the `EdgeFrame` local compact node index space for topology-oriented operations. That is the right thing for graph kernels, but it is easy to misuse if you assume every integer is a `NodeFrame` row index.

The intended pattern is:

- graph structure helpers work in compact graph-local node indices
- feature gather uses `node_row_ids` when you need row-aligned payloads

That split may look slightly awkward at first, but it avoids conflating graph-local structure with table row position. Once the distinction is clear, the rest of the bridge is straightforward.

## What This Means For PyTorch Workflows

In practice, the flow looks like:

1. sample or walk over the graph
2. gather node feature rows in sampled order
3. convert the gathered Arrow output to the tensor representation your training stack wants

Lynxes is responsible for the graph-aware part of that flow. PyTorch or DGL still handles the model, optimizer, loss, and training loop. That is the intended division of labor.

## Why This Belongs In Lynxes At All

If the engine already owns Arrow-native payloads and CSR-backed structure, then exporting those two pieces in GNN-friendly form is not a side feature. It is one of the cleanest ways to make the existing architecture useful outside the engine's own query surface.

That is why this layer exists. It is not "Lynxes learns GNNs." It is "Lynxes can hand a graph and its features to a GNN stack without pretending graph structure and feature storage are separate problems."
