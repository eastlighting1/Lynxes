# Connectors

This page documents the Python connector entry points that create lazy graph sources backed by external systems. All connector functions return `LazyGraphFrame`, not eager `GraphFrame`. That matters because remote work is not forced immediately when you call the connector. In many cases the first real failure only shows up when the lazy plan is collected.

## Shared Behavior

Connector-backed objects behave like any other `LazyGraphFrame` after construction:

1. create the connector-backed lazy object
2. optionally inspect the plan with `explain()`
3. add lazy filters or traversal steps
4. materialize with `collect()`, `collect_nodes()`, or `collect_edges()`

## Summary

| Function | Returns | Backing system |
| :--- | :--- | :--- |
| `lynxes.read_neo4j(...)` | `LazyGraphFrame` | Neo4j |
| `lynxes.read_arangodb(...)` | `LazyGraphFrame` | ArangoDB |
| `lynxes.read_sparql(...)` | `LazyGraphFrame` | SPARQL endpoint |

## `lynxes.read_neo4j(uri, user, password, database=None) -> LazyGraphFrame`

### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `uri` | `str` | Required | - | Neo4j connection URI. |
| `user` | `str` | Required | - | Neo4j username. |
| `password` | `str` | Required | - | Neo4j password. |
| `database` | `str \| None` | Optional | `None` | Optional Neo4j database name. |

### Returns

Returns a `LazyGraphFrame`.

### Raises

- `TypeError` if arguments have invalid Python types
- `RuntimeError` if connector setup or later remote execution fails

## `lynxes.read_arangodb(endpoint, database, graph, vertex_collection, edge_collection, username, password) -> LazyGraphFrame`

### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `endpoint` | `str` | Required | - | ArangoDB HTTP endpoint. |
| `database` | `str` | Required | - | Database name. |
| `graph` | `str` | Required | - | Graph name. |
| `vertex_collection` | `str` | Required | - | Vertex collection name. |
| `edge_collection` | `str` | Required | - | Edge collection name. |
| `username` | `str` | Required | - | Username. |
| `password` | `str` | Required | - | Password. |

### Returns

Returns a `LazyGraphFrame`.

### Raises

- `TypeError` if arguments have invalid Python types
- `RuntimeError` if connector setup or later remote execution fails

## `lynxes.read_sparql(endpoint, node_template, edge_template, expand_template=None) -> LazyGraphFrame`

### Parameters

| Name | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `endpoint` | `str` | Required | - | SPARQL endpoint URL. |
| `node_template` | `str` | Required | - | Template used to load node rows. |
| `edge_template` | `str` | Required | - | Template used to load edge rows. |
| `expand_template` | `str \| None` | Optional | `None` | Optional template used for expansion. |

### Returns

Returns a `LazyGraphFrame`.

### Raises

- `TypeError` if arguments have invalid Python types
- `RuntimeError` if connector setup or later remote execution fails

## Notes

Connector failures are usually Python `RuntimeError` rather than `ValueError`. That is deliberate: most failures here come from the external system, credentials, endpoint availability, or connector execution itself rather than from local graph validation.
