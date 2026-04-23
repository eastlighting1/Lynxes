# Your First Graph Query

This guide is about the first query shape that makes Lynxes feel like a graph engine rather than a file loader: choose a seed set, expand from it, and materialize the resulting subgraph.

The examples here intentionally stay small. The point is not to show a huge graph. The point is to make the query mechanics visible enough that you can tell what happened and whether it worked.

## Start From A Known Graph

This guide uses the shared example file:

`examples/data/example_simple.gf`

If you have already worked through the Python or CLI getting-started guide, keep using the same file. That continuity is helpful. You should not have to debug new data and a new query pattern at the same time.

## Python: Choose A Seed Node

Start with a seed:

```python
import lynxes as lx

g = lx.read_gf("examples/data/example_simple.gf")

seeded = g.lazy().filter_nodes(lx.col("_id") == "alice")
```

Nothing has been collected yet. At this point you have a lazy plan that says, in effect, "start from the node whose id is `alice`."

## Python: Expand One Hop

Now expand one hop outward:

```python
one_hop = seeded.expand(hops=1, direction="out").collect()

print("nodes:", one_hop.node_count())
print("edges:", one_hop.edge_count())
print("ids:", one_hop.nodes().ids())
```

You should see a result shaped like this:

```text
nodes: 2
edges: 1
ids: ['alice', 'bob']
```

This is a useful first check because it is easy to reason about by eye. Starting from `alice`, one outward hop along the graph should bring in `bob` and the edge that connects them.

## Python: Expand Two Hops

Now widen the query slightly:

```python
two_hop = seeded.expand(hops=2, direction="out").collect()

print("nodes:", two_hop.node_count())
print("edges:", two_hop.edge_count())
print("ids:", two_hop.nodes().ids())
```

On the shared example graph, this should look like:

```text
nodes: 3
edges: 2
ids: ['alice', 'bob', 'charlie']
```

This is the first moment where the shape of a graph result becomes clear. The result is not only "the nodes that matched." It is the subgraph needed to represent what the traversal reached.

## Python: Restrict By Edge Type

Now make the query more specific:

```python
typed = (
    g.lazy()
    .filter_nodes(lx.col("_id") == "alice")
    .expand(edge_type="KNOWS", hops=2, direction="out")
    .collect()
)

print("typed nodes:", typed.node_count())
print("typed edges:", typed.edge_count())
```

On this example graph, the output should still be consistent with the two-hop result above because the relevant edges are already of type `KNOWS`.

This step matters because it introduces a pattern that scales beyond toy graphs. Once multiple relationship types appear, edge-type restriction stops being optional and starts being one of the main ways users say what they really mean.

## CLI: Run The Same Query Shape

The CLI version of the same traversal looks like this:

```bash
cargo run -p lynxes-cli -- query examples/data/example_simple.gf --from alice --hops 2 --direction out --view info
```

If the CLI is behaving normally, the output should show a subgraph summary with 3 nodes and 2 edges.

Now add the type restriction:

```bash
cargo run -p lynxes-cli -- query examples/data/example_simple.gf --from alice --hops 2 --edge-type KNOWS --direction out --view info
```

The point of doing both is not just coverage. It shows that the Python lazy API and the CLI query flow are exposing the same model. They are not two different query languages with two different meanings.

## What Users Usually Learn Here

The first query guide tends to teach three things at once.

First, the seed set matters. Lynxes queries usually begin by deciding where traversal should start.

Second, `expand(...)` produces graph-shaped results, not just lists of matching rows.

Third, `collect()` is the moment where the plan becomes a concrete result. Until then, you are still describing the query.

That is a lot of value for one small walkthrough, which is why this page is a better early guide than a generic method catalog.

## Common Mistakes On The First Query

The first few failures are usually very ordinary:

- wrong file path
- a seed node id that is not present
- a direction string that does not match the intended traversal
- edge-type restriction that filters away more than expected

If you get a result that is smaller than expected, check the seed and the edge-type argument before assuming the traversal engine is wrong.

## Where To Go Next

If this query flow made sense, the next natural step is to compare it with direct algorithm calls. Continue with [Your First Algorithm Run](first-algorithm-run.md).
