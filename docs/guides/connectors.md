# Connectors

This guide explains the current connector entry points exposed by the Python API.

## Connector Overview

The current Python entry points are:

- `lynxes.read_neo4j(...)`
- `lynxes.read_arangodb(...)`
- `lynxes.read_sparql(...)`

These functions return a `LazyGraphFrame`, not an eager `GraphFrame`.

That means the common flow is:

1. create a lazy scan
2. optionally add filters or traversal steps
3. materialize with `collect()`, `collect_nodes()`, or `collect_edges()`

## Important Expectation

Connectors are part of the public surface, but they should be treated as integration-oriented features.
They are not the best first stop when you are just learning the local graph workflow.

Start with local files first, then move to connectors once the query shape is clear.

## Neo4j

```python
import lynxes as lx

lazy = lx.read_neo4j("bolt://localhost:7687", "neo4j", "password")
print(lazy.explain())
```

Use this when your source of truth is already in Neo4j and you want to compose a Lynxes lazy plan over it.

## ArangoDB

```python
import lynxes as lx

lazy = lx.read_arangodb(
    endpoint="http://localhost:8529",
    database="mydb",
    graph="social",
    vertex_collection="persons",
    edge_collection="knows",
)
print(lazy.explain())
```

Use this when graph content is modeled as Arango collections and you want a lazy scan entry point.

## SPARQL

```python
import lynxes as lx

lazy = lx.read_sparql(
    endpoint="https://dbpedia.org/sparql",
    node_template="SELECT ?id WHERE { ?id a <Thing> }",
    edge_template="SELECT ?s ?o WHERE { ?s ?p ?o }",
)
print(lazy.explain())
```

Use this when graph edges and nodes are coming from query templates rather than local files.

## Practical Limits

A few things to keep in mind:

- connector flows are not a substitute for validating query logic on a tiny local graph first
- optimization should be treated as an implementation detail, not a hard pushdown guarantee
- if debugging is unclear, reduce the same idea to a local `.gf` repro

For troubleshooting connector-backed workflows, pair this guide with [Errors and Debugging](errors-and-debugging.md).
