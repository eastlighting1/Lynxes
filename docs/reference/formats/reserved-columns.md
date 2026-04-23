# Reserved Graph Columns

These columns carry graph semantics in Lynxes. They are not ordinary user attributes even when they live beside user-defined fields in the same Arrow batch or parquet file. If one of these columns is missing, malformed, or semantically inconsistent, Lynxes treats that as a graph-shape failure rather than as a harmless missing attribute.

## Node-Side Reserved Columns

| Column | Arrow type | Nullable | Meaning |
| :--- | :--- | :--- | :--- |
| `_id` | `Utf8` | No | Unique node identifier within the graph. Must be non-null and unique across all rows. Used as the primary key for CSR index construction and edge resolution. |
| `_label` | `Utf8` | No | Node label. Used for schema lookup and pattern matching constraints. A node may carry only one label per row in the current format. |

### `_id` constraints

- Values must be non-empty strings.
- Duplicate `_id` values within the same `NodeFrame` are a hard error (`DuplicateNodeId`).
- Edges reference nodes through `_src` and `_dst`, which must match values in `_id`. An edge that references a non-existent node id produces a `DanglingEdge` error.

### `_label` constraints

- Must be non-null.
- The label must match a declared node schema if a schema is present. Unlabeled or schema-less graphs can use any string.

## Edge-Side Reserved Columns

| Column | Arrow type | Nullable | Meaning |
| :--- | :--- | :--- | :--- |
| `_src` | `Utf8` | No | Source node id. Must match an `_id` in the associated `NodeFrame`. |
| `_dst` | `Utf8` | No | Destination node id. Must match an `_id` in the associated `NodeFrame`. |
| `_type` | `Utf8` | No | Edge type label. Used for `expand`, `traverse`, and `aggregate_neighbors` filters. |
| `_direction` | `Int8` | No | Direction encoding: `1` = outgoing, `-1` = incoming, `0` = undirected. |

### `_direction` values

| Value | Meaning | CSR behavior |
| :--- | :--- | :--- |
| `1` | Outgoing (directed) | Appears only in out-CSR from `_src`. |
| `-1` | Incoming (directed, stored reversed) | Appears only in in-CSR from `_dst`. |
| `0` | Undirected | Appears in both out-CSR and in-CSR. |

Any value other than `1`, `-1`, or `0` produces an `InvalidDirection` error at `EdgeFrame` construction time.

## Naming Restrictions

User-defined columns must not use the `_` prefix if their name matches a reserved column. Attempting to create a `NodeFrame` or `EdgeFrame` with a user column named `_id`, `_label`, `_src`, `_dst`, `_type`, or `_direction` returns a `ReservedColumnName` error.

Other `_`-prefixed names (e.g., `_score`, `_rank`) are allowed for user data.

## Validation Timing

Reserved column validation runs at frame construction time — `NodeFrame::from_record_batch` and `EdgeFrame::from_record_batch`. Validation is not deferred to query execution. A batch that fails validation never becomes a valid frame.

## `.gf` File Mapping

In `.gf` source files, reserved columns are not written explicitly. Instead:

- `_id` is the identifier on the left side of a node declaration.
- `_label` is derived from the schema type of the node.
- `_src`, `_dst`, `_type`, `_direction` are derived from the edge syntax.

The parser fills these columns when building `NodeFrame` and `EdgeFrame` from a parsed document.

## Parquet Mapping

When loading a parquet file as a graph, the same column names apply. Parquet files that omit `_id` or `_src`/`_dst`/`_type`/`_direction` fail with a `MissingReservedColumn` error. Column types that do not match the expected Arrow type fail with `ReservedColumnType`.

## See Also

- [`.gf` format](gf.md)
- [`.gfb` format](gfb.md)
- [Parquet graph shape](parquet-interop.md)
